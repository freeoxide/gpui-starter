use std::sync::{Arc, Mutex};

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::Signer as _;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::net::TcpStream;

const MAX_REQUEST_BYTES: usize = 8 * 1024;

#[derive(Clone, Debug)]
pub(crate) struct RecordedRequest {
    pub(crate) method: String,
    pub(crate) path: String,
    pub(crate) query: String,
    pub(crate) body: String,
    pub(crate) authorization: Option<String>,
    pub(crate) accept: Option<String>,
}

pub(crate) struct FullRequest {
    method: String,
    target: String,
    headers: Vec<(String, String)>,
    pub(crate) body: Vec<u8>,
}

impl FullRequest {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

struct FakeState {
    origin: String,
    requests: Vec<RecordedRequest>,
    token_seq: u64,
    userinfo_enabled: bool,
    token_body_override: Option<String>,
    signing_key: Option<ed25519_dalek::SigningKey>,
}

pub(crate) struct FakeIdP {
    state: Arc<Mutex<FakeState>>,
}

impl FakeIdP {
    pub(crate) fn origin(&self) -> String {
        self.lock().origin.clone()
    }

    pub(crate) fn disable_userinfo(&self) {
        self.lock().userinfo_enabled = false;
    }

    pub(crate) fn enable_id_tokens(&self) {
        self.lock().signing_key = Some(ed25519_dalek::SigningKey::from_bytes(&[7u8; 32]));
    }

    pub(crate) fn override_token_body(&self, body: &str) {
        self.lock().token_body_override = Some(body.to_string());
    }

    pub(crate) fn requests(&self) -> Vec<RecordedRequest> {
        self.lock().requests.clone()
    }

    pub(crate) fn requests_with_path(&self, fragment: &str) -> Vec<RecordedRequest> {
        self.requests()
            .into_iter()
            .filter(|request| request.path.contains(fragment))
            .collect()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, FakeState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub(crate) fn spawn(rt: &tokio::runtime::Handle) -> Self {
        let std_listener = std::net::TcpListener::bind("127.0.0.1:0").expect("fake bind");
        let port = std_listener.local_addr().unwrap().port();
        std_listener.set_nonblocking(true).expect("nonblocking");
        let origin = format!("http://127.0.0.1:{port}");
        let state = Arc::new(Mutex::new(FakeState {
            origin: origin.clone(),
            requests: Vec::new(),
            token_seq: 0,
            userinfo_enabled: true,
            token_body_override: None,
            signing_key: None,
        }));
        let _guard = rt.enter();
        let listener = TcpListener::from_std(std_listener).expect("tokio listener");
        let task_state = state.clone();
        rt.spawn(async move {
            loop {
                let Ok((mut stream, _)) = listener.accept().await else {
                    continue;
                };
                let Some(request) = read_request(&mut stream).await else {
                    continue;
                };
                let (status, body) = route(&request, &task_state);
                write_response(&mut stream, &status, &body).await;
            }
        });
        Self { state }
    }
}

fn route(request: &FullRequest, state: &Arc<Mutex<FakeState>>) -> (String, String) {
    let mut guard = state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let origin = guard.origin.clone();
    let (path, query) = request
        .target
        .split_once('?')
        .unwrap_or((request.target.as_str(), ""));
    let body = String::from_utf8_lossy(&request.body).to_string();
    guard.requests.push(RecordedRequest {
        method: request.method.clone(),
        path: path.to_string(),
        query: query.to_string(),
        body: body.clone(),
        authorization: request.header("authorization").map(str::to_string),
        accept: request.header("accept").map(str::to_string),
    });
    let mints_token = path == "/token" || path == "/login/oauth/access_token";
    if mints_token {
        guard.token_seq += 1;
    }
    let seq = guard.token_seq;
    match path {
        "/.well-known/openid-configuration" => {
            let signing_alg = if guard.signing_key.is_some() {
                "EdDSA"
            } else {
                "RS256"
            };
            let doc = serde_json::json!({
                "issuer": origin,
                "authorization_endpoint": format!("{origin}/authorize"),
                "token_endpoint": format!("{origin}/token"),
                "userinfo_endpoint": format!("{origin}/userinfo"),
                "jwks_uri": format!("{origin}/jwks"),
                "response_types_supported": ["code"],
                "subject_types_supported": ["public"],
                "id_token_signing_alg_values_supported": [signing_alg],
            });
            ("200 OK".into(), doc.to_string())
        }
        "/jwks" => {
            let Some(key) = guard.signing_key.as_ref() else {
                return ("200 OK".into(), r#"{"keys": []}"#.into());
            };
            let jwks = serde_json::json!({
                "keys": [{
                    "kty": "OKP",
                    "use": "sig",
                    "alg": "EdDSA",
                    "crv": "Ed25519",
                    "x": URL_SAFE_NO_PAD.encode(key.verifying_key().to_bytes()),
                }]
            });
            ("200 OK".into(), jwks.to_string())
        }
        "/token" => {
            if let Some(body) = guard.token_body_override.clone() {
                let status = if body.contains("\"error\"") {
                    "400 Bad Request"
                } else {
                    "200 OK"
                };
                return (status.into(), body);
            }
            let mut response = serde_json::json!({
                "access_token": format!("fake-access-{seq}"),
                "token_type": "Bearer",
                "expires_in": 3600,
                "refresh_token": format!("fake-refresh-{seq}"),
                "scope": "openid profile email",
            });
            if let Some(key) = guard.signing_key.as_ref() {
                let nonce = guard
                    .requests
                    .iter()
                    .rev()
                    .find(|request| request.path == "/authorize")
                    .and_then(|request| pair_value(&request.query, "nonce"));
                let now = chrono::Utc::now().timestamp();
                let claims = serde_json::json!({
                    "iss": origin,
                    "sub": "fake-sub-2",
                    "aud": pair_value(&body, "client_id"),
                    "exp": now + 3600,
                    "iat": now,
                    "nonce": nonce,
                    "name": "ID Token User",
                    "preferred_username": "idtoken-user",
                    "email": "idtoken-user@example.test",
                    "email_verified": true,
                    "picture": "https://fake.example.test/idtoken.png",
                });
                response["id_token"] = serde_json::Value::String(mint_id_token(
                    key,
                    &claims.to_string(),
                ));
            }
            ("200 OK".into(), response.to_string())
        }
        "/userinfo" => {
            if !guard.userinfo_enabled {
                return ("404 Not Found".into(), "{}".into());
            }
            (
                "200 OK".into(),
                r#"{"sub": "fake-sub-1", "email": "fake-user@example.test", "email_verified": true, "name": "Fake User", "picture": "https://fake.example.test/avatar.png"}"#.into(),
            )
        }
        "/revoke" => ("200 OK".into(), "{}".into()),
        "/login/oauth/access_token" => {
            if request.header("accept") != Some("application/json") {
                return (
                    "406 Not Acceptable".into(),
                    "accept application/json required".into(),
                );
            }
            if let Some(body) = guard.token_body_override.clone() {
                return ("200 OK".into(), body);
            }
            (
                "200 OK".into(),
                format!(
                    r#"{{"access_token": "gh-access-{seq}", "token_type": "Bearer", "scope": "read:user user:email"}}"#
                ),
            )
        }
        "/user" => (
            "200 OK".into(),
            r#"{"login": "fake-login", "name": null, "email": null, "avatar_url": "https://fake.example.test/gh.png"}"#
                .into(),
        ),
        "/user/emails" => (
            "200 OK".into(),
            r#"[{"email": "secondary@example.test", "primary": false, "verified": true, "visibility": null}, {"email": "primary@example.test", "primary": true, "verified": true, "visibility": "private"}]"#
                .into(),
        ),
        path if path.starts_with("/applications/") && path.ends_with("/token") => {
            if request.method != "DELETE" || request.header("authorization").is_none() {
                return ("401 Unauthorized".into(), "{}".into());
            }
            ("204 No Content".into(), String::new())
        }
        _ => ("404 Not Found".into(), "{}".into()),
    }
}

fn pair_value(pairs: &str, key: &str) -> Option<String> {
    pairs.split('&').find_map(|pair| {
        let (name, value) = pair.split_once('=')?;
        (name == key).then(|| value.to_string())
    })
}

fn mint_id_token(key: &ed25519_dalek::SigningKey, claims: &str) -> String {
    let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"EdDSA"}"#);
    let claims = URL_SAFE_NO_PAD.encode(claims);
    let signature = key.sign(format!("{header}.{claims}").as_bytes());
    format!(
        "{header}.{claims}.{}",
        URL_SAFE_NO_PAD.encode(signature.to_bytes())
    )
}

/// Shared with the crash-report upload sink test: full header/body parse of
/// a single HTTP request.
pub(crate) async fn read_request(stream: &mut TcpStream) -> Option<FullRequest> {
    let mut buf = Vec::with_capacity(1024);
    let mut chunk = [0u8; 1024];
    let header_end = loop {
        if let Some(at) = find_header_end(&buf) {
            break at;
        }
        if buf.len() >= MAX_REQUEST_BYTES {
            return None;
        }
        let read = stream.read(&mut chunk).await.ok()?;
        if read == 0 {
            return None;
        }
        buf.extend_from_slice(&chunk[..read]);
    };
    let header_bytes = &buf[..header_end];
    let mut lines = header_bytes.split(|byte| *byte == b'\n');
    let request_line = lines.next()?;
    let mut parts = request_line.split(|byte: &u8| byte.is_ascii_whitespace());
    let method = std::str::from_utf8(parts.next()?).ok()?.to_string();
    let target = std::str::from_utf8(parts.next()?).ok()?.to_string();
    let mut headers = Vec::new();
    for line in lines {
        if line.len() <= 2 {
            continue;
        }
        let Some((name, value)) = std::str::from_utf8(line).ok()?.split_once(':') else {
            continue;
        };
        headers.push((name.trim().to_ascii_lowercase(), value.trim().to_string()));
    }
    let content_length = headers
        .iter()
        .find(|(name, _)| name == "content-length")
        .and_then(|(_, value)| value.parse::<usize>().ok())
        .unwrap_or(0);
    if header_end + content_length > MAX_REQUEST_BYTES {
        return None;
    }
    while buf.len() < header_end + content_length {
        let read = stream.read(&mut chunk).await.ok()?;
        if read == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..read]);
    }
    let body = buf[header_end..(header_end + content_length).min(buf.len())].to_vec();
    Some(FullRequest {
        method,
        target,
        headers,
        body,
    })
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|at| at + 4)
}

async fn write_response(stream: &mut TcpStream, status: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.shutdown().await;
}
