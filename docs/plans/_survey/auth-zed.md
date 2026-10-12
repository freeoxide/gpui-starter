# Zed auth survey: pattern reference

How Zed (zed-industries/zed) does authentication, as input for our auth plans.
Zed sources were fetched 2026-10-12 from `raw.githubusercontent.com` at main HEAD
`4905e67333956e0d351a14884213aec45d9c4303` (permalinks below use that SHA);
line numbers match the fetched files. Registry sources were located with
`ls ~/.cargo/registry/src/*/ | grep -E 'gpui-pre-0.3.9|gpui-kit-0.7.3'` and read
directly at `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/` (shorthand
`gpui-pre-0.3.9/...`, `gpui-kit-0.7.3/...`, plus backend crates
`gpui-pre-linux-0.3.9/...`, `gpui-pre-windows-0.3.8/...`, `gpui-pre-macos-0.3.8/...`).
Anything not verifiable is under Open Questions.

Zed has four distinct auth patterns. Keep them separate when citing:

| Pattern | Used by | Flow |
| --- | --- | --- |
| A. Zed account | collab, cloud, plans | browser → loopback HTTP → encrypted token handoff |
| B. Standard OAuth2 + PKCE | ChatGPT Subscription, MCP servers | browser → loopback HTTP → `code` + `state` |
| C. Agent-driven device flow | GitHub Copilot | LSP request → `user_code` → browser |
| D. API key in settings | Google AI provider | no OAuth at all |

## 1. Credential storage

Trait (provider-agnostic, no keyring types leak through):
`CredentialsProvider::{read,write,delete}_credentials(url) -> (username: String, password: Vec<u8>)`
— crates/credentials_provider/src/credentials_provider.rs:11-34.

Two implementations, chosen at startup:

| Impl | Store | Where | Gate |
| --- | --- | --- | --- |
| `KeychainCredentialsProvider` | OS keychain, via GPUI platform API | crates/zed_credentials_provider/src/zed_credentials_provider.rs:69-101 | default for nightly/preview/stable |
| `DevelopmentCredentialsProvider` | JSON file `paths::config_dir()/development_credentials`, `HashMap<String,(String,Vec<u8>)>` keyed by URL | same file:110-134 | dev release channel only, unless `ZED_DEVELOPMENT_USE_KEYCHAIN` set (same file:22-24, 45-66) |

The provider is a GPUI global: `ZedCredentialsProvider(pub Arc<dyn CredentialsProvider>)`,
`impl Global`, `init_global(cx)` / `global(cx)` with lazy fallback (zed_credentials_provider.rs:26-43).

Zed account credential shape: `Credentials { user_id: u64, access_token: String }`,
keychain username = `user_id.to_string()`, password = access token bytes, key =
`credentials_url` setting or `server_url` (crates/client/src/client.rs:343-353, 403-421, 370-374).
The `credentials_url` setting exists so side-by-side instances don't overwrite each
other's keychain entries (client.rs:106-115). Authorization header is
`format!("{} {}", user_id, access_token)` (client.rs:349-353).

Refresh: the Zed account has **no refresh token**. `sign_in` validates the stored
access token against the server (`validate_credentials`, client.rs:961-977);
on failure it deletes the keychain entry and re-runs the interactive flow
(client.rs:902-941). Short-lived LLM API tokens are separate: cached, refreshed on
`EXPIRED_LLM_TOKEN_HEADER_NAME`/`OUTDATED_LLM_TOKEN_HEADER_NAME` response headers,
refreshed+cleared on org switch, cleared on sign-out
(crates/client/src/llm_token.rs:14-20, 66-79; client.rs:1628-1647).

Pattern B stores a JSON blob instead of a bare token: `CodexCredentials
{ access_token, refresh_token, expires_at_ms, account_id, email }` under keychain
key `https://chatgpt.com/backend-api/codex`, username literal `"Bearer"`
(crates/openai_subscribed/src/openai_subscribed.rs:55-61, 40, 317).
`refresh_token` POSTs `grant_type=refresh_token&client_id=...` (openai_subscribed.rs:1192+);
sign-out deletes the entry (openai_subscribed.rs:379-396).

Corruption behavior:

| Store | On corruption |
| --- | --- |
| Zed account keychain entry | `String::from_utf8`/`parse()` failure → `read_credentials` returns `None` → treated as signed out (client.rs:394-397) |
| Dev JSON file | `load_credentials().unwrap_or_default()` — unreadable file reads as empty map (zed_credentials_provider.rs:143-147, 160) |
| Pattern B JSON blob | deserialize failure → `log::warn!`, state stays unauthenticated (openai_subscribed.rs:131-147) |

## 2. Sign-in flow shape (Zed account, pattern A)

`Client::authenticate_with_browser` (client.rs:1430-1559). No webview, no device
code, no custom scheme:

1. Generate asymmetric keypair (`rpc::auth::keypair()`); the browser-side server
   encrypts the access token to the app's public key so other local processes
   can't read it off the loopback wire (client.rs:1449-1455).
2. Bind loopback HTTP server `127.0.0.1:0` via `tiny_http` (ephemeral port,
   client.rs:1470-1476).
3. Build `{server_url}/native_app_signin?native_app_port&native_app_public_key&system_id`
   and open the default browser with GPUI's `cx.open_url` (client.rs:1478-1496,
   1436-1444).
4. Identity-provider choice (Google / GitHub / email) happens on that web page,
   not in the editor — the editor binary contains no Google/GitHub client ids for
   the Zed account (verified: no `google|github` hits in crates/zed/src/zed.rs or
   title_bar; see Open Questions for the page itself).
5. Browser redirects to `http://127.0.0.1:{port}/?user_id&access_token`; the app
   polls `server.recv_timeout(1s)` up to 100 times, responds HTTP 302 to
   `{server_url}/native_app_signin_succeeded`, decrypts the token with the private
   key, `cx.activate(true)` to raise the window (client.rs:1509-1557).
   In-tree TODO: switch to a custom URL scheme instead of the local HTTP server
   (client.rs:1507-1508).
6. Credentials persisted (unless impersonating), `Status::Authenticated` set;
   cancellation is via the status watch stream (client.rs:916-956, 938-941).

Staff/impersonation escape hatch: `ZED_IMPERSONATE` + `ZED_ADMIN_API_TOKEN` env
vars call `/internal/users/impersonate` directly, skipping the browser
(client.rs:67-79, 1457-1467, 1561-1606).

Pattern B replaces steps 1/5 with standard OAuth2: shared loopback server crate
`oauth_callback_server` binds `127.0.0.1` (ephemeral or fixed port with fallback),
parses `code`/`state`/`error` from the query, replies with a themed HTML card
("Authorization Successful — You can close this tab and return to Zed"), 2-minute
timeout, and a `GET /cancel` handshake to evict a stale listener on a fixed port
(crates/oauth_callback_server/src/oauth_callback_server.rs:146-168, 170-219, 223,
284-302, 369-454).

## 3. Social specifics

| Flow | Grant | Client id | Scopes | Redirect |
| --- | --- | --- | --- | --- |
| Zed account | proprietary (server-mediated) | none in editor; zed.dev owns IdP clients | n/a from editor | loopback, app-specific query params |
| ChatGPT Subscription | OAuth2 authorization code + PKCE S256, CSRF `state` | hardcoded in source: `app_EMoamEEZ73f0CkXaXp7hrann` (Codex CLI's public client) | `openid profile email offline_access` — deliberately minimal so the token JWT fits Windows Credential Manager's 2560-byte blob | `http://localhost:1455/auth/callback`, fallback 1457 (fixed, matches OpenAI's allow-list) |
| GitHub Copilot | device flow, delegated to Copilot LSP server | inside GitHub's bundled agent | n/a | none; `PromptUserDeviceFlow { user_code, command: "github.copilot.finishDeviceFlow" }` |
| Google AI | none | — | — | API key typed into settings, sent as `?key=` query param |

Citations: openai_subscribed.rs:36-40, 1040-1048 (redirect allow-list comment,
RFC 6749 §3.1.2 exact-match note), 1063-1073 (PKCE verifier/challenge), 1085-1101
(authorize URL: `client_id`, `redirect_uri`, `scope`, `response_type=code`,
`code_challenge_method=S256`, `state`, browser opened only after listener bound),
1103-1111 (state check), 1133+ (`exchange_code` form POST with `code_verifier`),
1257 (`extract_jwt_claims` pulls `account_id`/`email` from the id_token).
Copilot: crates/copilot/src/copilot.rs:94-119 (`SignInStatus` states incl.
`SigningIn { prompt }`), 755-828 (`sign_in` = LSP `SignIn` request; the returned
`user_code` is shown to the user, completion observed via status), test fixture
showing the device-flow payload at copilot.rs:1646-1655. Google AI:
crates/google_ai/src/google_ai.rs:19-30.

## 4. Session surface in the editor UI

State lives in two places, UI reads both:

- `Client::status()` — a `postage::watch` stream over
  `Status::{SignedOut, Authenticating, Authenticated, AuthenticationError,
  Connected, Reauthenticating, Reauthenticated, ...}` (client.rs:280-331, 335).
- `UserStore` — entity holding `current_user: watch::Receiver<Option<Arc<User>>>`
  plus organizations/plans/usage; a spawned task maps client status → user fetch
  (`cloud_client().get_authenticated_user`) → `Event::PrivateUserInfoUpdated`;
  `Status::SignedOut` clears orgs, plan, usage, contacts
  (crates/client/src/user.rs:115, 215-294, 272-282, 881-929; UI subscribes via
  `watch_current_user`, user.rs:961-963).

Title bar render (crates/title_bar/src/title_bar.rs:363-411):

| State | UI |
| --- | --- |
| signed out / auth error, `show_sign_in` setting on | "Sign In" button → `client.sign_in_with_optional_connect(true)` (title_bar.rs:387-392, 1199-1217) |
| signing in | pulsing "Signing in…" label, 2s repeat animation (title_bar.rs:393-406) |
| signed in, `show_user_menu` on | `PopoverMenu` triggered by `Avatar::new(user.avatar_uri)` (+org name when business); menu: username row → opens `{server}/account` in browser, plan/upgrade entries, "Sign Out" → `client::SignOut` action (title_bar.rs:1219-1312, 1433-1435) |

Actions `SignIn`/`SignOut`/`Reconnect` are defined in the client crate and wired
in `client::init` (client.rs:91-101, 166-182). Account/billing/trial pages are
always browser URLs built from `server_url` (crates/client/src/zed_urls.rs:32-48);
the editor never renders an account panel itself.

Sign-out clearing: `Client::sign_out` drops in-memory credentials, clears cloud
client credentials, disconnects, deletes the keychain entry; `request_sign_out()`
pushes an async message consumed by `UserStore`'s sign-out task (client.rs:1681-1699;
user.rs:295-307). Pattern B mirrors this: clear entity state + notify first, then
delete persisted credentials in a spawned task (openai_subscribed.rs:379-396).

Note: crate `session` is not auth — it persists an app session id + window stack
in the KVP store for restore (crates/session/src/session.rs:5-38, 63-133).

## 5. Reusable vs zed-specific

| Piece | Verdict | Why / how to reuse |
| --- | --- | --- |
| `CredentialsProvider` trait (url → username/password bytes) | reuse pattern | three-method async trait + GPUI global; provider-agnostic, exactly what a multi-backend store needs |
| GPUI `App::{read,write,delete}_credentials` keychain API | reuse directly | already in our pinned gpui-pre 0.3.9, see §6 |
| Loopback OAuth callback server (`oauth_callback_server` crate) | reuse pattern | crate is ~490 lines, deps: `tiny_http`, `url`, `futures`; ephemeral-port default + fixed-port `/cancel` eviction + HTML result page are the parts worth copying |
| OAuth2 code + PKCE + state flow (`do_oauth_flow`) | reuse pattern | works against any standards-compliant IdP (Google/GitHub fit directly); client id becomes config |
| Dev-channel file-backed provider | reuse pattern | avoids keychain prompts during development; keep it dev-only the way Zed gates it |
| Browser-first, IdP-on-server sign-in (`/native_app_signin`) | zed-specific | assumes zed.dev; but the shape (editor → web page owns provider choice → loopback return) is the provider-agnostic option if we ever add our own backend |
| Keypair-encrypted token handoff | zed-specific, optional | defense against local eavesdroppers on the loopback port; standard `code`+PKCE already prevents token exposure, so only the proprietary handoff needs it |
| `user_id u64` + opaque access token, `Authorization: "{user_id} {token}"` | zed-specific | their server's private format; use JWT/OIDC instead |
| watch-stream status enum + status→UI-state mapping | reuse pattern | `Status` watch + `current_user` watch is a clean two-layer model for signed-in UI |
| Title-bar sign-in button / signing-in label / avatar menu | reuse pattern | gpui code, directly portable to gpui-kit components |
| LLM-token cache/refresh-on-header | zed-specific | tied to their response headers; the "retry once after refresh" shape (client.rs:1628-1647) is the reusable part |
| Copilot device flow | zed-specific | only relevant if we embed an agent that owns its own auth |

## 6. gpui-pre 0.3.9 / gpui-kit 0.7.3: what we already have

Verified against the registry sources on disk (the ask expected nothing; there is
something):

- `gpui-pre-0.3.9/src/app.rs:1789-1811` — `App::write_credentials(url, username, password)`,
  `App::read_credentials(url) -> Task<Result<Option<(String, Vec<u8>)>>>`,
  `App::delete_credentials(url)`, all delegating to the platform
  (`src/platform.rs:597-599` declares the `Platform` methods).
- `gpui-pre-0.3.9/src/app.rs:1809-1810` — `App::open_url` (default browser).
- `gpui-pre-0.3.9/src/app.rs:1819-1820` + `platform.rs:442` —
  `App::register_url_scheme(scheme)` and `on_open_urls` callback (`app.rs:292-296`):
  the custom-URL-scheme alternative to a loopback server is available if we want it.
- Backends are separate crates; on disk at 0.3.9: `gpui-pre-linux-0.3.9`,
  `gpui-pre-web-0.3.9` (macos/windows/apple latest present are 0.3.8):
  - Linux: Secret Service via `oo7::Keyring`, item label constant
    `KEYRING_LABEL = "zed-github-account"`, attributes `url`/`username`
    (`gpui-pre-linux-0.3.9/src/linux/platform.rs:57, 1026-1046, 1047-1063`).
  - Windows: `CredWriteW` with explicit `CRED_MAX_CREDENTIAL_BLOB_SIZE` (2560-byte)
    guard (`gpui-pre-windows-0.3.8/src/platform.rs:1020-1028`).
  - macOS: Security framework keychain (`gpui-pre-macos-0.3.8/src/platform.rs:1258+`).
  - Test/visual-test platforms implement them as no-ops
    (`gpui-pre-0.3.9/src/platform/test/platform.rs:768-778`).
- gpui-kit 0.7.3: `grep -rn -iE 'keyring|keychain|credential|oauth|\bauth\b' gpui-kit-0.7.3/src -l`
  returned nothing — no auth surface in the kit; auth is entirely ours to build on
  the gpui-pre primitives above.

## Open Questions

- zed.dev sign-in page provider buttons (Google vs GitHub vs email) could not be
  verified: `curl https://zed.dev/native_app_signin` returns a client-rendered
  shell (title only, no buttons in HTML), and `zed-industries/zed.dev` is not a
  public repo (GitHub API: "Not Found"). The editor source proves the editor is
  IdP-blind (no provider names, no client ids); the web-side provider list is
  their closed surface.
- Exact current line numbers beyond the pinned SHA: Zed moves fast; citations
  above are exact for `4905e673` fetched 2026-10-12.
- `gpui-pre-macos`/`gpui-pre-windows` at 0.3.9 are not on disk (0.3.8 is the
  latest present); if our app pins those backends elsewhere, re-check their
  credential impls at the pinned versions.
