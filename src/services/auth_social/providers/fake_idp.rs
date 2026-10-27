use std::sync::{Arc, Mutex};

use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;

use crate::services::auth_social::loopback::{ParsedRequest, read_request};

#[derive(Clone, Debug)]
pub(crate) struct RecordedRequest {
    pub(crate) method: String,
    pub(crate) path: String,
    pub(crate) query: String,
    pub(crate) body: String,
    pub(crate) authorization: Option<String>,
    pub(crate) accept: Option<String>,
    pub(crate) content_type: Option<String>,
}

struct FakeState {
    origin: String,
    requests: Vec<RecordedRequest>,
    token_seq: u64,
    userinfo_enabled: bool,
    token_body_override: Option<String>,
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

fn route(request: &ParsedRequest, state: &Arc<Mutex<FakeState>>) -> (String, String) {
    let mut guard = state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let origin = guard.origin.clone();
    let (path, query) = request
        .target
        .split_once('?')
        .unwrap_or((request.target.as_str(), ""));
    guard.requests.push(RecordedRequest {
        method: request.method.clone(),
        path: path.to_string(),
        query: query.to_string(),
        body: String::from_utf8_lossy(&request.body).to_string(),
        authorization: request.header("authorization").map(str::to_string),
        accept: request.header("accept").map(str::to_string),
        content_type: request.header("content-type").map(str::to_string),
    });
    let mints_token = path == "/token" || path == "/login/oauth/access_token";
    if mints_token {
        guard.token_seq += 1;
    }
    let seq = guard.token_seq;
    match path {
        "/.well-known/openid-configuration" => (
            "200 OK".into(),
            format!(
                r#"{{"issuer": "{origin}", "authorization_endpoint": "{origin}/authorize", "token_endpoint": "{origin}/token", "userinfo_endpoint": "{origin}/userinfo", "jwks_uri": "{origin}/jwks", "response_types_supported": ["code"], "subject_types_supported": ["public"], "id_token_signing_alg_values_supported": ["RS256"]}}"#
            ),
        ),
        "/jwks" => ("200 OK".into(), r#"{"keys": []}"#.into()),
        "/token" => {
            if let Some(body) = guard.token_body_override.clone() {
                let status = if body.contains("\"error\"") {
                    "400 Bad Request"
                } else {
                    "200 OK"
                };
                return (status.into(), body);
            }
            (
                "200 OK".into(),
                format!(
                    r#"{{"access_token": "fake-access-{seq}", "token_type": "Bearer", "expires_in": 3600, "refresh_token": "fake-refresh-{seq}", "scope": "openid profile email"}}"#
                ),
            )
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

async fn write_response(stream: &mut tokio::net::TcpStream, status: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.shutdown().await;
}
