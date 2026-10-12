# 21 Auth Foundation Crate (gpui-auth) Plan

## Goal

A new workspace member `crates/gpui-auth` provides the app's authentication
foundation: a `TokenSet` model, a `TokenStore` trait with an in-memory
implementation plus an app-side OS-keyring adapter, PKCE/state helpers, an
`AuthProvider` trait with a scripted mock implementation, and an
`AuthController` GPUI entity that drives flows and mirrors sign-in state into
the existing `SessionState`. No UI ships in this crate; plans 22 (email),
23 (social OIDC/OAuth), and 24 (account UI) build on the Interface Contract
below. Two-call app integration: one `install(cx)` at boot plus one
callback-routing line in the AppRoot event drain.

## Dependencies

No plan 01-20 items are consumed (verified: none of the 20 plans' Interface
Contracts are needed; `src/ui/canvas/`, `src/ui/floating/`, `src/ui/waveform/`,
`src/features/wgpu/` do not exist on disk, `ls src/ui/` → `widgets/` only).

Survey inputs, trusted over memory and re-verified where load-bearing:
`_survey/seams.md` (integration canon), `_survey/auth-zed.md` (Zed patterns,
gpui-pre 0.3.9 platform facts), `_survey/auth-ecosystem.md` (crate + protocol
facts, Part D decision inputs for 21). Registry sources located with
`ls ~/.cargo/registry/src/*/ | grep -E 'gpui-pre-0.3.9|gpui-kit-0.7.3'` (both
present) and read directly this session.

Repo services consumed (all read this session, cited in Research Digest):

- `src/services/session.rs` — `SessionState` enum (lines 3-12), `set_state`
  (31-36), `SessionSnapshot` global (14-29). The app adapter maps
  `AuthController` state into this global; the crate never imports it.
- `src/services/secure_storage.rs` — `set_secret`/`get_secret`/`delete_secret`
  (139-195 native; 197-234 wasm fail-closed stubs), `SecureStorageError`
  (6-66). The app-side `KeyringStore` adapter wraps these; the crate never
  imports them.
- `src/services/desktop_actions.rs` — `open_url` (147-191, native
  `open::that_detached`, http/https only; wasm returns `Unavailable`).
- `src/runtime/events.rs` — `AppEventKind::DeepLinkReceived(String)` (12),
  `RemoteCommand(ForwardedCommand)` (15); AppRoot drain consumes both
  (`src/shell/root/app_root/state.rs:63-114`).
- Deep links / single instance — `src/shell/route.rs` (`APP_URL_SCHEME` line 6,
  `parse_deep_link` 120-162 rejects unknown hosts and ignores query params,
  lines 135-141 + 152-154), `src/platform/ipc/command.rs:9-20`
  (`ForwardedCommand::DeepLink(String)` — the typed second-instance
  forward), `src/platform/process/single_instance.rs`
  (`SCHEME` line 50, `initial_deep_link` 138-140, IPC forward →
  `DeepLinkReceived` emit at 297 and 416), `src/web.rs:40` (startup link
  emit), `src/platform/deep_link_registration.rs` (Windows HKCU registration
  of the whole `gpui-starter://` scheme; the `auth` host needs no new
  registration).
- `src/services/tokio_runtime.rs` — `runtime_and_client(cx)` (80-83) supplies
  the reqwest client and the `Arc<tokio::runtime::Runtime>` the controller's
  `AuthIo` needs natively; the wasm shim is an undriven current-thread
  runtime (39-44), so wasm drives provider futures on the GPUI executor.
- `src/app/init.rs:202-218` — boot order: `TokioRuntimeGlobal` (207),
  `secure_storage::initialize` (216), `session::initialize` (217) all precede
  the adapter's `install` line.

## Research Digest

gpui-pre 0.3.9 / gpui-kit 0.7.3 facts, verified on disk this session (path
key `GPUI` = `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-pre-0.3.9/src`,
`KIT` = `.../gpui-kit-0.7.3`):

- `Context<T>::spawn<AsyncFn, R>(&self, f: AsyncFn) -> Task<R>` with
  `AsyncFn: AsyncFnOnce(WeakEntity<T>, &mut AsyncApp) -> R`
  (`GPUI/app/context.rs:230-238`).
- `WeakEntity<T>::update(&self, cx, f) -> Result<R>` errors once the entity is
  released (`GPUI/app/entity_map.rs:803-813`).
- `App::observe(&mut self, &Entity<W>, FnMut(Entity<W>, &mut App)) ->
  Subscription` (`GPUI/app.rs:1333-1343`); `App::subscribe` for
  `EventEmitter<Event>` entities (`GPUI/app.rs:1422-1436`).
- `pub trait EventEmitter<E: Any>: 'static {}` (`GPUI/gpui.rs:346`) — the
  empty-trait + `cx.emit` pattern the app already uses
  (`src/features/command_palette.rs:59`).
- `App::open_url(&self, url: &str)` (`GPUI/app.rs:1809`). Web backend opens a
  new browser context; `register_url_scheme` is a no-op on web and errors on
  Linux/Windows (auth-ecosystem Part C; `gpui-pre-web-0.3.9/src/platform.rs:468-470`,
  `gpui-pre-linux-0.3.9/src/linux/platform.rs:1102-1104`) — so the foundation
  never relies on scheme registration.
- `#[gpui::test]` proc macro re-exported at `GPUI/gpui.rs:116-120`, defined at
  `gpui-pre-macros-0.3.9/src/gpui_macros.rs:251`, gated on the crate's
  `test-support` feature (`gpui-pre-0.3.9/Cargo.toml:69`). The app reaches it
  as `#[gpui_kit::test]` through the kit glob `pub use ::gpui::*`
  (`KIT/src/lib.rs:101`); the new crate uses `#[gpui::test]` directly.
- Kit pins the gpui family: `[dependencies.gpui] version = "=0.3.9",
  package = "gpui-pre"` (`KIT/Cargo.toml`, `[dependencies.gpui]` block read
  this session), so the crate's `gpui.workspace = true` (workspace declares
  `version = "0.3.9"`, root `Cargo.toml:20`) resolves into the same single
  copy the lock already enforces (`tests/qa_migration.rs:131-146`).
- No auth surface exists in the kit: `grep -rn -ilE
  'keyring|keychain|credential|oauth' gpui-kit-0.7.3/src/` → no hits
  (re-run this session). Everything below is ours.
- Facade-scan scope: `tests/qa_migration.rs:41-80` scans the app package's
  `src/`, `build.rs`, `tests/` only. `crates/gpui-auth` is a separate package
  outside the scan; it depends on `gpui` (gpui-pre) directly and never pulls
  the UI stack. The app-side adapter in `src/services/` keeps kit-facade
  imports like its neighbors (`src/services/secure_storage.rs:1`).

Async shape, decided from the app's own patterns:

- HTTP on native runs reqwest futures inside `runtime.spawn(...)` and awaits
  the `JoinHandle` from a `cx.spawn` async block — the updater precedent
  (`src/services/updater/check.rs:72-77, 225-239`); the rule is stated at
  `src/services/tokio_runtime.rs:5-6` ("GPUI's executor is not a tokio
  runtime, so tokio-dependent code must run via `runtime.spawn(...)`").
- On wasm the tokio shim is deliberately undriven
  (`src/services/tokio_runtime.rs:39-44`); reqwest futures on wasm are fetch
  futures that run on any executor, so the same call drives them directly.
  reqwest 0.12.24 is an unconditional app dependency
  (`Cargo.toml:187-193`) and ships in the wasm build (survey Part C), so the
  crate's `reqwest = { default-features = false, features = ["json"] }`
  subset unions into the app's single copy.
- Send-ness split, verified on disk: reqwest-on-wasm bottoms out in
  `js_sys::futures::JsFuture`, whose inner value is `Rc<RefCell<...>>` and
  therefore `!Send` (`js-sys-0.3.102/src/futures/mod.rs:112-120`). GPUI's
  foreground executor takes non-Send futures — `App::spawn` boxes with
  `boxed_local()` and has no `Send` bound (`gpui-pre-0.3.9/src/app.rs:2297-2306`).
  Consequence, encoded in the contract: `AuthIo::drive` requires
  `Send + 'static` only on native (where `rt.spawn` needs it); on wasm it
  takes any `Future`. Matching that, `AuthProvider` uses default
  `#[async_trait]` (Send-boxed futures, required by the native spawn) and
  `#[async_trait(?Send)]` on wasm — the opt-out mode is documented in
  async-trait 0.1.92 itself (`async-trait-0.1.92/src/lib.rs:165`). Without
  this split, plan 22's wasm email provider (reqwest fetch inside an
  async_trait method) cannot compile. The trait object stays `Send + Sync`
  on both targets; only the returned futures relax on wasm.

Dependency picks for the crate (each with a one-line justification):

| Dep | Version | Why |
| --- | --- | --- |
| `gpui` | workspace (gpui-pre 0.3.9) | entity/controller; same copy as the app via the kit's `=0.3.9` pin |
| `serde` / `serde_json` | workspace | `TokenSet` persistence format for the store |
| `thiserror` | 2 (app line `Cargo.toml:174`) | `AuthError`; same major as the app |
| `url` | 2.5 (app line 177) | callback-URL query parsing (`Url::query_pairs`) |
| `base64` | 0.22 (app line 182) | base64url for PKCE verifier/challenge and state |
| `sha2` | 0.10 (app line 183) | S256 challenge digest |
| `getrandom` | 0.4, features `["std", "wasm_js"]` | CSPRNG for verifier/state; 0.4.3 already in the lock; `wasm_js` is the wasm32-unknown-unknown backend (`getrandom-0.4.3/src/backends.rs:170-178`), and its wasm-bindgen dep is target-gated so the feature is inert natively (`getrandom-0.4.3/Cargo.toml:90`). API: `getrandom::fill(&mut [u8])` (`getrandom-0.4.3/src/lib.rs:87`) |
| `zeroize` | 1.9 | `SecretString` wipes on drop; 1.9.0 already transitive in the lock (survey A1), direct dep only upgrades semantics |
| `reqwest` | 0.12, `default-features = false`, `["json"]` | token-endpoint HTTP for plan 22/23 providers; subset of the app's feature set, one copy in the lock |
| `async-trait` | workspace | `AuthProvider` as a trait object |
| `tracing` | workspace | controller logging with the repo's target convention |
| `tokio` (native table only) | workspace (rt+sync+time) | `AuthIo::drive` needs `Runtime::spawn` on native; absent from wasm |

Deliberately NOT added: `oauth2` 5.0 / `openidconnect` 4.0 (plan 23's pick,
survey A2; the foundation's PKCE/state helpers make the browser-flow core
testable now without the heavier graph), `chrono` (the crate is clock-free:
`expires_at` is unix-seconds data and every freshness check takes `now_unix`
as a parameter, so no wasm clock hazard per seams §6 and no new dep),
`keyring` (stays app-side behind `secure_storage`), `rand` (getrandom
suffices; survey A1 notes it is not a direct dep today).

keyring 3.6.3 Linux gap (survey A3, verified in
`keyring-3.6.3/Cargo.toml` this session): the app enables only
`apple-native` + `windows-native` (`Cargo.toml:337`), so Linux compiles to
the mock store and writes vanish. Fix shipped by this plan: add
`linux-native` (kernel keyutils) to the app's keyring features. It is
target-gated inside keyring (`[target.'cfg(target_os = "linux")'.dependencies.linux-keyutils]`,
`keyring-3.6.3/Cargo.toml:167-170`), so the extra feature is inert on
macOS/Windows and adds no system libraries anywhere. Tradeoff recorded in
Risks: keyutils is session-scoped (tokens do not survive reboot; the user
re-authenticates). The persistent upgrade path
(`linux-native-sync-persistent` → `dbus-secret-service` → `libdbus-1-dev`
build requirement on every Linux dev/CI box,
`keyring-3.6.3/Cargo.toml:76-80`) is documented as the follow-up lever, not
taken by a foundation plan.

KeyringStore delete-miss handling, typed (verified on disk): keyring 3.6.3's
`Error::NoEntry` variant exists at `keyring-3.6.3/src/error.rs:38`, and its
`Display` renders "No matching entry found in secure storage"
(`error.rs:68`) — the substring `NoEntry` never appears in Display text, so
string matching cannot detect it. The native `SecureStorageError::DeleteFailed`
carries the typed `source: keyring::Error` field
(`src/services/secure_storage.rs:30-36`), and keyring is a direct app
dependency (`Cargo.toml:337`), so the adapter pattern-matches
`matches!(source, keyring::Error::NoEntry)`. The wasm twin variant carries a
`reason: String` (`secure_storage.rs:60-65`); wasm never selects
`KeyringStore`, so its delete arm maps every error without miss-detection.

`src/services/storage/` (SQLite/KVP runtime, read this session) is
deliberately NOT used: tokens stay in the OS credential store (or memory on
wasm), never in a database file readable at rest. Zed keeps credentials out
of its KVP store for the same reason (`auth-zed.md` §1/§4: keychain
provider, while `crates/session` — the KVP store — holds no credentials).

Protocol facts grounding the model (from `_survey/auth-ecosystem.md` Part B,
RFC texts fetched there 2026-10-12): PKCE S256 mandatory-when-capable (RFC
7636 §4.2), high-entropy `state` + exact-match on response (RFC 8252
§8, `rfc8252.txt:755-763`), redirect URI stored with the pending request and
re-sent at exchange (mix-up defense, `rfc8252.txt:768-779`), refresh rotation
is per-provider (RFC 9700: rotate-and-invalidate for public clients is AS
policy; Google reuses, GitHub defaults to no refresh, generic OIDC should
assume rotation) — hence `RefreshPolicy` on the provider and merge logic on
the model. Client secrets embedded in native apps are not confidential (RFC
8252 §8.5, `rfc8252.txt:665-667`): the foundation's contract carries no
client-secret field, and plans 22/23 must read secrets from config input.

No cargo gate was run during planning (doc-only scope, matching the surveys
and plans 01-20). Every Verification command is for the implementer.

## Architecture

### Crate vs in-app (decided)

Crate, not `src/services/auth/` in-app. Deciding factors:

1. Three consumers with different cadences share the surface: plan 22
   (email provider + TOTP secret storage), plan 23 (two social providers +
   loopback listener), plan 24 (account UI). In-app placement couples all
   three to the app crate's rebuild and review surface; a crate gives them a
   stable seam (coding guides, "Organize large applications by capability").
2. Extraction is a stated requirement: the crate has its own manifest, zero
   `crate::`/app imports, no i18n coupling, no UI, and a documented publish
   path. An in-app module cannot demonstrate that boundary.
3. Wasm discipline is cheaper at a crate boundary: tokio stays behind the
   crate's not-wasm table, mirroring the app's own split
   (`Cargo.toml:30-35` vs `310-353`).
4. Cost accepted: one more workspace member in every `--workspace` gate and
   one path dependency line. No CI changes needed (check/clippy lanes
   already build all members; test lanes compile but do not run crate tests
   in CI, seams §8).

AGENTS.md anti-speculation audit — every abstraction has at least two named
implementations: `AuthProvider` → `MockProvider` (this plan) +
`EmailPasswordProvider` (22) + `OidcProvider`/`GitHubProvider` (23);
`TokenStore` → `InMemoryStore` (this plan, crate tests + wasm) +
`KeyringStore` (this plan, app-side); `TokenSet`/`RefreshPolicy` → minted by
mock (21) and real providers (22, 23); PKCE/state helpers → mock flow (21) +
browser flows (23); `AuthController` → session mapping (21) + UI host (24).

### Entities, state ownership, events

- `AuthController` (GPUI entity, in-crate) owns: `provider: Arc<dyn
  AuthProvider>`, `store: Arc<dyn TokenStore>`, `io: AuthIo`, `state:
  AuthState`, `pending: Option<PendingFlow>` (single slot; a new
  `start_sign_in` replaces any previous flow). It is the only writer of the
  pending slot and of the store's row for its `provider_id`.
- Secrets live in exactly two places: the entity's transient `TokenSet`
  values (zeroized on drop) and the `TokenStore`. Never `AppConfig` (no
  config field is added; snapshot schema untouched), never logs
  (`SecretString`'s `Debug` is `[redacted]`; controller tracing logs state
  and provider ids only, the `session.rs:32` precedent).
- The app adapter owns: the `AuthHandle` global holding
  `Entity<AuthController>` plus the two subscriptions that keep it wired
  (observer → `session::set_state`; subscriber → browser open on
  `AuthorizationRequested`). Capability entries `auth_email`/`auth_social`
  are set by the adapter at install.
- Events: `AuthController` implements `EventEmitter<AuthEvent>`; `AuthEvent`
  carries transient flow outcomes (`AuthorizationRequested { url }` is not
  state, so it cannot ride `cx.notify`). Steady-state changes call
  `cx.notify()`, which is what the session-mapping observer listens to.

Data flow, sign-in:

```text
UI/command (22/24) ──start_sign_in──▶ AuthController
  cx.spawn: io.drive(provider.start_flow(http))        [tokio rt natively / GPUI executor on wasm]
  ◀─ FlowLaunch { authorize_url, state, pkce?, redirect_uri }
  pending = Some(...) ; state = SigningIn ; cx.notify ; emit AuthorizationRequested { url }
adapter subscriber ──▶ desktop_actions::open_url natively / cx.open_url on wasm
browser ▶─ gpui-starter://auth/callback?code&state ──▶ OS launches app (Windows: existing
  HKCU scheme registration) ──argv/IPC──▶ DeepLinkReceived / ForwardedCommand::DeepLink
AppRoot drain ──handle_callback_link──▶ controller.complete_callback(raw_url)
  parse query ◀▶ pending.state exact match (else StateMismatch, pending dropped)
  io.drive(provider.complete_callback(http, redirect, &pending.launch)) ─▶ TokenSet
  store.save ◀ provider_id ▸ state = SignedIn { account_label } ; notify ; emit FlowCompleted
adapter observer ──▶ session::set_state(SessionState::SignedIn { account_label })
```

Refresh: caller (plans 22/24, or a future timer) checks
`TokenSet::needs_refresh(now, margin)` and calls `controller.refresh(cx)`;
controller loads, drives `provider.refresh`, merges per the provider's
`RefreshPolicy`, saves, notifies. Sign-out: best-effort
`provider.revoke(access)`, then `store.delete`, `state = SignedOut`, notify,
emit.

### Crate boundary diagram

```text
crates/gpui-auth/                          src/services/auth.rs (app adapter)
├── token.rs   TokenSet, SecretString,     ├─ KeyringStore (impl TokenStore over
│             AccountInfo, RefreshPolicy   │  secure_storage::{set,get,delete}_secret)
├── pkce.rs    PkcePair, state helper      ├─ install(cx): builds AuthIo from
├── store.rs   TokenStore, InMemoryStore    │  tokio_runtime::runtime_and_client,
├── provider.rs AuthProvider, FlowLaunch,  │  picks store (keyring native /
│             RedirectParams, AuthIo        │  in-memory wasm), MockProvider,
├── controller.rs AuthController,          │  creates entity, sets capabilities,
│             AuthState, AuthEvent         │  wires session observer + browser
├── mock.rs    MockProvider (feature)      │  subscriber; AuthHandle global
├── error.rs   AuthError                   ├─ controller(cx) accessor for 22-24
└── lib.rs     re-exports, README          └─ handle_callback_link(link, cx)
   (no gpui_kit, no app imports,              (gpui-starter://auth/callback router
    publish = false)                           called from the AppRoot drain)
```

## Interface Contract

Load-bearing for plans 22-24. This block is line-stable: prose above and
below may extend, the code block is never reflowed (same discipline as plans
01/02/16 per INDEX §8). Consumers cite it as `21:Interface Contract`.
Amended post-review before any consumer drafted against it (2026-10-12):
`AuthError` gained `Clone` (an `AuthEvent` variant carries it), and
`AuthIo::drive` / `AuthProvider` acquired the native/wasm Send split — both
review findings, both fixing compile-blockers for plans 22-23.
Amended again post-review (2026-10-12, second round): `SecretString` gained
`Clone`, and `PkcePair` / `FlowLaunch` / `RefreshPolicy` gained the derives
their use sites require (`TokenSet` clones with SecretString fields;
Step 6.2 clones `FlowLaunch` into the async block; `refresh_policy(&self)`
returns by value) — review finding, compile-blocker at Phase 2.2, zero
design impact (all fields are `String`/`SecretString`).

```rust
// crates/gpui-auth/src/lib.rs
pub mod controller; pub mod error; pub mod mock; pub mod pkce;
pub mod provider; pub mod store; pub mod token;
pub use controller::{AuthController, AuthEvent, AuthState};
pub use error::AuthError;
pub use mock::MockProvider;            // feature "mock-provider"
pub use pkce::{PkcePair, authorize_challenge, random_state};
pub use provider::{AuthProvider, AuthIo, FlowLaunch, RedirectParams, RefreshPolicy};
pub use store::{InMemoryStore, TokenStore};
pub use token::{AccountInfo, SecretString, TokenSet};

// error.rs — Clone so AuthEvent (and plan-24 view state) can carry it.
#[derive(Clone, Debug, thiserror::Error)]
pub enum AuthError {
    #[error("no pending authorization flow")]
    NoPendingFlow,
    #[error("state mismatch on authorization response")]
    StateMismatch,
    #[error("authorization failed: {error}")]
    ProviderDenied { error: String, description: Option<String> },
    #[error("token response invalid: {0}")]
    InvalidTokenResponse(String),
    #[error("no refresh token available")]
    NoRefreshToken,
    #[error("token store failure: {0}")]
    Storage(String),
    #[error("network failure: {0}")]
    Network(String),
    #[error("randomness unavailable: {0}")]
    Entropy(String),
    #[error("invalid redirect url: {0}")]
    InvalidRedirect(String),
}

// token.rs — Debug prints [redacted] for every secret; serde is transparent
// because the store's backing (OS keyring row) is itself access-controlled.
pub struct SecretString(String);        // zeroized on drop
impl SecretString {
    pub fn new(value: impl Into<String>) -> Self;
    pub fn expose(&self) -> &str;       // only at the wire/store boundary
}
impl PartialEq for SecretString;        // state comparison; not constant-time — a single-use 256-bit value compared once
impl Clone for SecretString;            // String clone; the copy zeroizes on drop like the original

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AccountInfo {
    pub id: Option<String>,
    pub email: Option<String>,
    pub display_name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TokenSet {
    pub provider_id: String,            // must equal the store key it was saved under
    pub access_token: SecretString,
    pub refresh_token: Option<SecretString>,
    pub id_token: Option<SecretString>,
    pub token_type: String,             // "Bearer" unless the AS says otherwise
    pub expires_at: Option<i64>,        // unix seconds, UTC; None = no expiry advertised
    pub scope: Option<String>,
    pub account: Option<AccountInfo>,   // from id_token claims / userinfo; mock fills it
}
impl TokenSet {
    pub fn is_expired(&self, now_unix: i64) -> bool;              // false when expires_at is None
    pub fn needs_refresh(&self, now_unix: i64, margin_secs: i64) -> bool;
    pub fn redacted_summary(&self) -> String;                     // for logs: ids/scope/expiry only
}

/// Per-provider refresh behavior (RFC 9700 framing; see Research Digest).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefreshPolicy { Rotated, ReuseUntilDead, None }
impl RefreshPolicy {
    /// Fold a refresh response into the stored set: Rotated takes the new
    /// set verbatim; ReuseUntilDead keeps the old refresh token when the
    /// response omits one; None never reaches merge (refresh is refused).
    pub fn merge(self, old: &TokenSet, new: TokenSet) -> TokenSet;
}

// pkce.rs
#[derive(Clone)]                        // verifier clones via SecretString's Clone
pub struct PkcePair { /* verifier: SecretString, challenge: String */ }
impl PkcePair {
    /// 32 bytes of CSPRNG entropy -> 43-char base64url verifier + S256 challenge.
    pub fn generate() -> Result<Self, AuthError>;
    pub fn challenge(&self) -> &str;    // base64url(SHA-256(verifier)), no padding
    pub fn expose_verifier(&self) -> &str;  // token-exchange boundary only
}
/// Deterministic S256 for callers holding their own verifier bytes.
pub fn authorize_challenge(verifier: &str) -> String;
/// 32 bytes of CSPRNG entropy -> 43-char base64url string (state values).
pub fn random_state() -> Result<SecretString, AuthError>;

// store.rs — every method runs on the caller's App context so app-side
// implementations can update their own globals (secure_storage precedent).
pub trait TokenStore: Send + Sync {
    fn save(&self, provider_id: &str, tokens: &TokenSet, cx: &mut gpui::App) -> Result<(), AuthError>;
    fn load(&self, provider_id: &str, cx: &mut gpui::App) -> Result<Option<TokenSet>, AuthError>;
    fn delete(&self, provider_id: &str, cx: &mut gpui::App) -> Result<(), AuthError>;
}
pub struct InMemoryStore { /* Mutex<HashMap<String, TokenSet>> */ }
impl InMemoryStore { pub fn new() -> Self; }   // + Default

// provider.rs
#[derive(Clone)]
pub struct AuthIo {                     // Clone; shared by controller spawns
    pub http: reqwest::Client,
    #[cfg(not(target_family = "wasm"))]
    pub runtime: Option<std::sync::Arc<tokio::runtime::Runtime>>,
}
impl AuthIo {
    /// Native: run `fut` on the tokio runtime when present (Send + 'static
    /// required by rt.spawn); None falls through to a direct await (tests
    /// and mock-only setups). Wasm has no runtime field and no tokio.
    #[cfg(not(target_family = "wasm"))]
    pub async fn drive<F: Future + Send + 'static>(&self, fut: F) -> F::Output
    where F::Output: Send;
    /// Wasm: direct await, no Send bound — reqwest fetch futures bottom out
    /// in JsFuture (Rc-based, !Send; js-sys-0.3.102/src/futures/mod.rs:112-120)
    /// and GPUI's executor takes them (boxed_local, no Send bound).
    #[cfg(target_family = "wasm")]
    pub async fn drive<F: Future>(&self, fut: F) -> F::Output;
}

#[derive(Clone)]                        // Step 6.2 clones launch into the async block
pub struct FlowLaunch {
    pub authorize_url: String,          // the URL the app must open externally
    pub state: SecretString,            // controller matches this on callback
    pub pkce: Option<PkcePair>,         // required for browser flows (S256)
    pub redirect_uri: String,           // re-sent at exchange (RFC 8252 §8.1)
}
pub struct RedirectParams {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}
impl RedirectParams {
    /// Parse the query of any redirect URL (scheme-agnostic; the app decides
    /// which URLs are auth callbacks).
    pub fn from_url(raw: &str) -> Result<Self, AuthError>;
}

// Native: Send-boxed futures (rt.spawn requires them). Wasm: (?Send) mode,
// because JsFuture-backed provider futures are !Send (see Research Digest).
// The trait OBJECT stays Send + Sync on both targets.
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
pub trait AuthProvider: Send + Sync {
    fn id(&self) -> &str;                       // stable; the store key
    fn display_name(&self) -> &str;             // fallback account label
    fn refresh_policy(&self) -> RefreshPolicy;
    /// Build the authorization request (and any discovery) and return the
    /// launch. MUST set a fresh random state and, for browser flows, an
    /// S256 PkcePair from this crate's helpers.
    async fn start_flow(&self, http: reqwest::Client) -> Result<FlowLaunch, AuthError>;
    /// Validate the redirect against the launch (verifier, redirect_uri)
    /// and exchange the code for tokens. The controller has already
    /// exact-matched `redirect.state` against `launch.state`.
    async fn complete_callback(
        &self,
        http: reqwest::Client,
        redirect: RedirectParams,
        launch: &FlowLaunch,
    ) -> Result<TokenSet, AuthError>;
    async fn refresh(&self, http: reqwest::Client, refresh_token: &str)
        -> Result<TokenSet, AuthError>;
    async fn revoke(&self, http: reqwest::Client, access_token: &str) -> Result<(), AuthError>;
}

// controller.rs
#[derive(Clone, Debug, PartialEq)]
pub enum AuthState { SignedOut, SigningIn, SignedIn { account_label: String }, Error(String) }

#[derive(Clone, Debug)]
pub enum AuthEvent {
    AuthorizationRequested { url: String },   // app opens it externally
    FlowCompleted,
    FlowFailed { error: AuthError },
    SignedOut,
}
pub struct AuthController { /* provider, store, io, state, pending */ }
impl gpui::EventEmitter<AuthEvent> for AuthController {}
impl AuthController {
    pub fn new(
        provider: std::sync::Arc<dyn AuthProvider>,
        store: std::sync::Arc<dyn TokenStore>,
        io: AuthIo,
        cx: &mut gpui::Context<Self>,
    ) -> Self;                                 // starts SignedOut; no I/O
    pub fn state(&self) -> &AuthState;
    pub fn provider_id(&self) -> &str;
    /// Replace any pending flow and start a new one (SigningIn).
    pub fn start_sign_in(&mut self, cx: &mut gpui::Context<Self>);
    /// Route a raw redirect URL: parse, state-match, exchange, persist.
    /// Consumes the pending flow whether it succeeds or fails.
    pub fn complete_callback(&mut self, raw_redirect: &str, cx: &mut gpui::Context<Self>);
    /// Drop the pending flow without contacting the provider.
    pub fn cancel(&mut self, cx: &mut gpui::Context<Self>);
    /// Unconditional refresh; callers gate on TokenSet::needs_refresh.
    pub fn refresh(&mut self, cx: &mut gpui::Context<Self>);
    /// Load persisted tokens; SignedIn when the store has them.
    pub fn restore(&mut self, cx: &mut gpui::Context<Self>);
    /// Best-effort revoke, then delete stored tokens, then SignedOut.
    pub fn sign_out(&mut self, cx: &mut gpui::Context<Self>);
}

// mock.rs (feature "mock-provider") — scripted provider, id "mock"
pub struct MockProvider { /* AtomicU64 counter */ }
impl MockProvider {
    pub fn new() -> Self;                      // + Default
}
// Behavior: start_flow returns authorize_url "urn:gpui-auth:mock:{state}"
// (non-http on purpose: the adapter only opens http(s) URLs, so nothing
// touches the OS browser), a real random_state(), and a real PkcePair.
// complete_callback accepts any code, mints deterministic tokens
// (access "mock-access-{n}", refresh "mock-refresh-{n}",
// account.display_name "Mock account"), refresh_policy = ReuseUntilDead,
// refresh() mints access n+1 keeping the refresh token, revoke() = Ok(()).
```

App-side surface plans 22-24 consume (`src/services/auth.rs`, all `pub`):

```rust
pub struct AuthHandle { /* Entity<AuthController> + the two live Subscriptions */ }
impl gpui_kit::Global for AuthHandle {}
/// Boot wiring: globals must already be up (tokio runtime, secure storage,
/// session). Idempotent; second call replaces the handle.
pub fn install(cx: &mut gpui_kit::App);
pub fn controller(cx: &gpui_kit::App) -> Option<gpui_kit::Entity<AuthController>>;
/// Consume `gpui-starter://auth/callback` links. Returns true when the link
/// was routed to the controller (caller skips route parsing).
pub fn handle_callback_link(link: &str, cx: &mut gpui_kit::App) -> bool;
/// App-side TokenStore over secure_storage; service "gpui-starter.auth",
/// key = provider_id, value = serde_json(TokenSet). Native only in practice.
pub struct KeyringStore;
impl gpui_auth::TokenStore for KeyringStore {}
```

Contract notes that bind consumers: `complete_callback` takes the full raw
URL string, not pre-parsed params, so the app-side routing stays one call;
`restore()` is invoked once by `install` after the entity exists; the
controller performs exactly one `store.save` per successful exchange and one
`store.delete` on sign-out; providers must never embed client secrets in
`FlowLaunch` or `TokenSet` (secrets arrive through provider construction in
plans 22/23 from config input).

## Module Layout

| File | Purpose |
| --- | --- |
| `crates/gpui-auth/Cargo.toml` | crate manifest (deps per Research Digest; `mock-provider` feature; native tokio table; `publish = false`) |
| `crates/gpui-auth/src/lib.rs` | module decls + re-exports + crate docs (extraction checklist lives in README) |
| `crates/gpui-auth/src/error.rs` | `AuthError` |
| `crates/gpui-auth/src/token.rs` | `SecretString`, `TokenSet`, `AccountInfo`, `RefreshPolicy` |
| `crates/gpui-auth/src/pkce.rs` | `PkcePair`, `authorize_challenge`, `random_state` |
| `crates/gpui-auth/src/store.rs` | `TokenStore` trait, `InMemoryStore` |
| `crates/gpui-auth/src/provider.rs` | `AuthProvider`, `FlowLaunch`, `RedirectParams`, `AuthIo` |
| `crates/gpui-auth/src/controller.rs` | `AuthController` entity, `AuthState`, `AuthEvent` |
| `crates/gpui-auth/src/mock.rs` | `MockProvider` (cfg feature `mock-provider`) |
| `crates/gpui-auth/README.md` | usage, security rules, wasm posture, extraction/publish checklist |
| `crates/gpui-auth/src/*.test.rs` | sibling test modules per repo convention (`#[cfg(test)] #[path]`) |
| `src/services/auth.rs` | app adapter: `install`, `AuthHandle`, `KeyringStore`, `handle_callback_link`, capability entries, module header with removal recipe |
| `src/services/auth.test.rs` | adapter tests (in-lib, CI-executed) |

## Seam Touches

Every edit outside the new files:

| # | File | Change | Why minimal |
| --- | --- | --- | --- |
| 1 | `Cargo.toml:1-2` | `[workspace]` gains `members = ["crates/gpui-auth"]` | workspace-member registration (plan 08 precedent); root package stays an implicit member |
| 2 | `Cargo.toml [dependencies]` | `gpui-auth = { path = "crates/gpui-auth", features = ["mock-provider"] }` | single dependency line; mock stays dev/demo-reachable until plan 22 registers real providers |
| 3 | `Cargo.toml:337` | keyring features += `"linux-native"` | fixes the silent mock-store-on-Linux gap; target-gated inside keyring so other OSes are unaffected |
| 4 | `Cargo.lock` | regenerate + stage with the manifest edits | repo practice; `tests/qa_migration.rs:168-186` only rejects gpui-family removals, additions must still be staged for `--locked` builds |
| 5 | `src/services/mod.rs` | `pub mod auth;` | one module line, alphabetical position after `accessibility` |
| 6 | `src/lib.rs:57-61` | add `auth` to the `pub use services::{...}` list | same re-export style as every other service |
| 7 | `src/app/init.rs` (after line 218 `crate::storage::initialize(cx);`) | `crate::services::auth::install(cx);` | single boot hook; placed after the three globals it reads (tokio runtime 207, secure storage 216, session 217) |
| 8 | `src/shell/root/app_root/state.rs` — `DeepLinkReceived` arm (67-70) and `ForwardedCommand::DeepLink` arm (94-99) | first statement of each: `if crate::services::auth::handle_callback_link(&link, cx) { continue; }` | two-line-per-arm edit; `auth` is not a `VALID_HOSTS` entry so `parse_deep_link` would reject callback links; no route.rs change at all |
| 9 | `src/services/auth.test.rs` | new tests | CI runs `-p gpui-starter --lib`, so in-lib tests execute in CI (seams §8) |

Deliberate non-touches: no `src/shell/route.rs` edit (callback links are
consumed before route parsing; `auth` must NOT become a page host), no
sidebar/AppRoot page wiring (no UI), no `AppConfig` field (snapshot schema
pinned, `tests/snapshot_tests.rs:26,57`), no `DEFAULT_ENABLED_CAPABILITIES`
edit (entries set via `capabilities::set` at install, plan 08 precedent), no
locale keys, no justfile recipe, no `[patch.crates-io]` change, no edit to
`deep_link_registration.rs` (it registers the whole scheme already).

### Removal Recipe

`src/services/auth.rs` opens with (mirrors `src/gallery/mod.rs:1-7`):

```rust
//! Removable auth foundation wiring. `install`, `controller`,
//! `handle_callback_link`, and `KeyringStore` are the only public surface.
//! Removal = delete this file, delete crates/gpui-auth/, and revert the seam
//! touches: the members + gpui-auth + keyring-feature lines in Cargo.toml
//! with a regenerated Cargo.lock, the `pub mod auth;` line in
//! services/mod.rs, the `auth` re-export in lib.rs, the install line in
//! app/init.rs, and the two `handle_callback_link` guards in the AppRoot
//! event drain.
```

Ordered removal steps:

1. Revert the two AppRoot drain guards (`state.rs` arms back to bare
   `parse_deep_link`).
2. Remove the `install` line from `src/app/init.rs`.
3. `rm src/services/auth.rs src/services/auth.test.rs`; revert
   `src/services/mod.rs` and the `src/lib.rs` re-export.
4. `rm -r crates/gpui-auth/`; drop the `members` line, the `gpui-auth`
   dependency, and the `linux-native` keyring feature; regenerate
   `Cargo.lock` and stage it with the manifest.
5. Run the gates in Verification; `grep -rn "gpui_auth\|gpui-auth\|KeyringStore\|auth_email" src/ crates/ tests/ Cargo.toml`
   returns nothing (the `secure_storage` capability string `secure_storage`
   is unrelated and must remain).

## Implementation Phases

## Phase 1: Crate scaffold

### Step 1.1: Workspace member + manifest

- Files: `Cargo.toml` (workspace members), `crates/gpui-auth/Cargo.toml`,
  `crates/gpui-auth/src/lib.rs`, `crates/gpui-auth/README.md`, regenerated
  `Cargo.lock`.
- Build: `[workspace] members = ["crates/gpui-auth"]`. Crate manifest:
  `{ name = "gpui-auth", version = "0.1.0", edition = "2024", publish = false,
  description = "Auth foundation for GPUI apps: token model, PKCE, provider
  trait, controller entity", license = "MIT" }`;
  `[features] mock-provider = []`; dependencies exactly per the Research
  Digest table (`gpui.workspace = true`, serde/serde_json/async-trait/
  tracing workspace, thiserror/url/base64/sha2/zeroize/getrandom/reqwest
  crate-local); `[target.'cfg(not(target_family = "wasm")'.dependencies]
  tokio = { workspace = true }`;
  `[dev-dependencies] gpui = { package = "gpui-pre", version = "0.3.9",
  features = ["test-support"] }`. `lib.rs`: module decls with `pub mod error;`
  only (other modules land in their phases, declared as they land — an empty
  re-export list until then) plus a crate-level doc comment naming the
  security rules (secrets only in the store, redacted Debug, PKCE+state on
  every browser flow). README: usage sketch, security rules, wasm posture,
  and the extraction checklist (own manifest ✓, no app imports — enforce
  `grep -rn "gpui_starter\|crate::" src/` → zero in CI-unchecked form but
  listed as a done-check, feature flags ✓, README ✓, publish path: flip
  `publish`, set version, keep the gpui-pre pin aligned with the consumer's
  kit, `cargo publish` from the crate dir).
- Tests: none.
- Done-check: `cargo check -p gpui-auth` passes; `git diff --stat` shows
  Cargo.lock updated with a `gpui-auth` package block; stage it with the
  manifest.

## Phase 2: Token model

### Step 2.1: `SecretString`

- Files: `src/token.rs`, `src/token.test.rs` (wired `#[cfg(test)] #[path =
  "token.test.rs"] mod token_test;`), `src/lib.rs` (`pub mod token;` +
  re-export).
- Build: `SecretString(String)` with `new`, `expose`, manual `Debug`
  printing `SecretString("[redacted]")`, `PartialEq`/`Eq` comparing inner
  values, `Clone` (delegates to `String`'s; the copy zeroizes on drop like
  the original — required because `TokenSet` derives `Clone` over
  `SecretString` fields and Step 6.2 clones `FlowLaunch`), `ZeroizeOnDrop`
  via derive (`zeroize::ZeroizeOnDrop`), and transparent
  `Serialize`/`Deserialize` (serialize inner; no redaction in serde — the
  store's backing is access-controlled, comment states this).
- Tests: `format!("{:?}", SecretString::new("hunter2"))` contains
  `[redacted]` and not `hunter2`; serde round-trip through
  `serde_json::to_string`/`from_str` preserves the value; equality works.
- Done-check: `cargo test -p gpui-auth` passes.

### Step 2.2: `TokenSet`, `AccountInfo`, `RefreshPolicy`

- Files: same.
- Build: structs exactly per the Interface Contract; `is_expired` /
  `needs_refresh` (`expires_at.map_or(false, |t| now >= t)`; refresh when
  `expires_at.is_some() && now + margin >= t`); `redacted_summary` formats
  `provider_id`, token-type, scope, expires_at, and account id/email only;
  `RefreshPolicy::merge` per the contract doc comment (Rotated → `new` as-is;
  ReuseUntilDead → `new.refresh_token = new.refresh_token.or_else(|| old.refresh_token.clone())`;
  None → unreachable, returns `new`). All pub types carry doc comments
  stating what callers get (AGENTS.md doc-comment rule).
- Tests: expiry predicates with `expires_at: None` (never expired/refresh);
  `needs_refresh` true inside the margin; `merge` under ReuseUntilDead keeps
  the old refresh token when the response omits one and takes the new one
  when present; Rotated drops the old token; serde round-trip of a full
  `TokenSet` including `AccountInfo`; `redacted_summary` of a set with
  known secrets contains neither secret string.
- Done-check: `cargo test -p gpui-auth` passes.

## Phase 3: PKCE + state helpers

### Step 3.1: Randomness, S256, base64url

- Files: `src/pkce.rs`, `src/pkce.test.rs`, `src/lib.rs` (`pub mod pkce;` +
  re-export).
- Build: one private helper `fn random_urlsafe_32() -> Result<String,
  AuthError>` (`getrandom::fill(&mut [u8; 32])` then base64url encode);
  `PkcePair::generate` (verifier = helper output, 43 chars; challenge =
  `authorize_challenge(verifier)` = base64url-no-pad of SHA-256 digest);
  `random_state()` same helper, wrapped in `SecretString`; errors map to
  `AuthError::Entropy`. base64 URL_SAFE_NO_PAD via
  `base64::engine::general_purpose::URL_SAFE_NO_PAD` + `Engine` trait
  (base64 0.22 API).
- Tests: RFC 7636 Appendix B vector — verifier
  `dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk` → challenge
  `E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM` through
  `authorize_challenge`; generated verifier/challenge/state are 43 chars
  from the base64url alphabet `[A-Za-z0-9_-]`; two `generate()` calls
  produce different verifiers; `random_state` differs across 100 draws.
- Done-check: `cargo test -p gpui-auth` passes.

## Phase 4: Token store

### Step 4.1: Trait + `InMemoryStore`

- Files: `src/store.rs`, `src/store.test.rs`, `src/lib.rs` (`pub mod store;` +
  re-export).
- Build: `TokenStore` trait exactly per the contract (methods take
  `&mut gpui::App`; doc comment states why — app-side implementations update
  their own globals, `secure_storage.rs:236-240` precedent);
  `InMemoryStore(Mutex<HashMap<String, TokenSet>>)` with `new`/`Default`;
  save overwrites, load returns `Ok(None)` on miss, delete is idempotent
  (`Ok(())` on miss).
- Tests: save→load round-trip; overwrite replaces; delete then load →
  `None`; double delete → `Ok(())`; 8 threads × 50 saves on the same key →
  final load returns the last-written set and no panic ( Send+Sync proof).
- Done-check: `cargo test -p gpui-auth` passes.

## Phase 5: Provider trait, executor seam, mock

### Step 5.1: `AuthIo`, `FlowLaunch`, `RedirectParams`

- Files: `src/provider.rs`, `src/provider.test.rs`, `src/lib.rs`
  (`pub mod provider;` + re-export).
- Build: types per the contract. `AuthIo::drive` is a cfg pair: native
  method with `F: Future + Send + 'static` (`F::Output: Send`) — `Some(rt)`
  → `rt.spawn(fut).await` (JoinError propagates as a panic, the updater
  precedent), `None` → direct `fut.await`; wasm method with plain
  `F: Future` → `fut.await` (no tokio in the wasm build at all).
  `RedirectParams::from_url` parses with
  `url::Url::parse` (→ `InvalidRedirect`) and collects `code`, `state`,
  `error`, `error_description` from `query_pairs()`; unknown keys ignored.
- Tests: `from_url` on `https://c.b/x?code=a&state=b` yields both; on the
  spec URL `gpui-starter://auth/callback?code=k&state=s`; on an error
  response (`error=access_denied&error_description=nope`); on a non-URL
  string → `Err(InvalidRedirect)`; `drive` with `runtime: None` and a
  ready future returns the value (native host test).
- Done-check: `cargo test -p gpui-auth` passes.

### Step 5.2: `AuthProvider` trait + `MockProvider`

- Files: `src/provider.rs` (trait), `src/mock.rs` (cfg
  `#[cfg(feature = "mock-provider")]`), `src/mock.test.rs` (same cfg),
  `src/lib.rs` (cfg'd `pub mod mock;` + cfg'd re-export).
- Build: trait exactly per the contract — the cfg_attr'd async_trait pair
  (default `Send` boxing natively, `(?Send)` on wasm; sources in Research
  Digest), `&self` receivers, owned `reqwest::Client` params so returned
  futures can be moved onto the tokio runtime by `drive` after cloning the
  `Arc` provider (the pattern is spelled in Step 6.1). The impl side of
  `MockProvider` repeats the same cfg_attr pair (async-trait requires
  matching modes on trait and impl). `MockProvider` behavior
  exactly per the contract block: id `"mock"`, display `"Mock"`,
  `ReuseUntilDead`, atomic counter, `authorize_url` format
  `urn:gpui-auth:mock:{state}` (state exposed for URL construction),
  `complete_callback` builds a `TokenSet` with
  `provider_id "mock"`, access `mock-access-{n}`, refresh
  `mock-refresh-{n}`, `account.display_name Some("Mock account")`,
  `expires_at: None`; `refresh` mints `mock-access-{n+1}` keeping the given
  refresh token; `revoke` → `Ok(())`. No HTTP calls anywhere.
- Tests (all `#[cfg(feature = "mock-provider")]`): start_flow returns a
  launch whose state is 43-char base64url and whose pkce challenge is 43
  chars; complete_callback with the launch's own state round-trips a
  TokenSet whose secrets round-trip through serde; refresh keeps the
  refresh token and increments the access counter; revoke is Ok. The
  local gate runs these via `--all-features`; note in the module header
  that plain `cargo test -p gpui-auth` skips them.
- Done-check: `cargo test -p gpui-auth --all-features` passes;
  `cargo test -p gpui-auth` (no features) also passes (mock compiled out).

## Phase 6: AuthController

### Step 6.1: Entity, states, spawn plumbing

- Files: `src/controller.rs`, `src/controller.test.rs`, `src/lib.rs`
  (declare + re-export).
- Build: `AuthController` fields per the contract; `EventEmitter<AuthEvent>`
  empty impl; `new` (SignedOut, no I/O); `state`/`provider_id` readers.
  Spawn helper `fn launch_io<T, F>(provider: Arc<dyn AuthProvider>, io:
  AuthIo, make: F)` pattern written once and reused by all four async
  entry points: clone the `Arc` provider and the `AuthIo`, build the
  `'static` future, `io.drive(fut)` (the Step 5.2 note — futures borrowing
  `&self` cannot be spawned, so the provider Arc is moved into the async
  block). Send note: the wrapping async block must satisfy the native
  `drive` bound (`Send + 'static` — the moved Arc/Client/FlowLaunch are all
  Send, and the async_trait future inside is Send natively); on wasm no
  bound applies and `cx.spawn` itself is bound-free (`boxed_local`,
  `gpui-pre-0.3.9/src/app.rs:2297-2306`). `start_sign_in`: replace `pending` with `None`, set `SigningIn`,
  `cx.notify()`, `cx.spawn(async move |this, cx| { ... })`; on
  `FlowLaunch`: `cx.update(|cx| this.update(cx, |c, cx| {
  c.on_flow_prepared(launch, cx) }))` — store pending, keep `SigningIn`,
  `cx.notify()`, `cx.emit(AuthEvent::AuthorizationRequested { url })`; on
  `Err(e)`: `state = Error(e)`, notify, `emit(FlowFailed)`. Errors from the
  weak-handle update (entity released) end the task silently. Tracing
  target `"gpui_auth::controller"`; log provider_id and state only, never
  token or state-param values.
- Tests (`#[gpui::test]`, `cx: &mut gpui::TestAppContext`, mock +
  InMemoryStore + `AuthIo { http: reqwest::Client::new(), runtime: None }`):
  construct entity in `cx.new`, `start_sign_in`, `cx.run_until_parked()` →
  state `SigningIn`, `controller.read(cx).pending_is_some()` (expose a
  `#[cfg(test)]` reader) and a captured `AuthorizationRequested` event
  (subscribe before starting, store URLs in a `Rc<RefCell<Vec<String>>>`);
  the URL starts `urn:gpui-auth:mock:`.
- Done-check: `cargo test -p gpui-auth --all-features` passes.

### Step 6.2: `complete_callback`, `cancel`, `restore`, `refresh`, `sign_out`

- Files: same.
- Build: `complete_callback(raw_redirect, cx)` — take `pending` (else
  `Error(NoPendingFlow)` + notify + `FlowFailed`); parse
  `RedirectParams::from_url`; if `error` is set → `ProviderDenied` path;
  exact-compare `redirect.state` with `pending.launch.state` via
  `SecretString` equality (mismatch → `StateMismatch`, pending already
  consumed, state `Error`, emit `FlowFailed`); else spawn
  `provider.complete_callback(http, redirect, &launch)` through `drive` —
  note `launch` must be cloned into the async block (FlowLaunch owns
  `SecretString`s; clone is fine). On `TokenSet`: validate
  `tokens.provider_id == self.provider_id()` (mismatch →
  `InvalidTokenResponse`), `store.save` inside the entity update (the
  `&mut Context<Self>` derefs to `App`), state `SignedIn { account_label }`
  where the label is `account.display_name` → `account.email` →
  `provider.display_name()` fallback chain; notify, `emit(FlowCompleted)`.
  `cancel`: drop pending, state `SignedOut` (no provider call). `restore`:
  `store.load` → `Some` → `SignedIn` with the same label chain; `None` →
  stay. `refresh`: load; `refresh_token` absent → `Error(NoRefreshToken)`;
  provider policy `RefreshPolicy::None` → same error path (providers
  without refresh never reach merge); else drive `provider.refresh(http,
  expose(refresh))`, merge via `provider.refresh_policy()`, save, notify. `sign_out`: drive
  `provider.revoke` ignoring its result (log at warn on failure), then
  `store.delete`, state `SignedOut`, notify, `emit(SignedOut)`.
- Tests: happy path (start → complete with the mock's own redirect URL
  built as `https://cb?code=x&state={state}` → `SignedIn`, store populated,
  `FlowCompleted` captured); wrong-state redirect → `StateMismatch`, store
  empty, second callback attempt → `NoPendingFlow`; error-param redirect →
  `ProviderDenied`; `cancel` then callback → `NoPendingFlow`; `restore`
  with a pre-seeded InMemoryStore → `SignedIn`; `refresh` after sign-in →
  access token changed, refresh token unchanged (ReuseUntilDead);
  `refresh` while signed out → `NoRefreshToken` surfaced as `Error` state;
  `sign_out` → `SignedOut`, store row deleted, `SignedOut` event captured.
- Done-check: `cargo test -p gpui-auth --all-features` passes;
  `cargo clippy -p gpui-auth --all-features -- -D warnings` clean.

## Phase 7: App adapter

### Step 7.1: App dependency + module

- Files: root `Cargo.toml` (`gpui-auth` dep line, keyring
  `linux-native` feature), `src/services/mod.rs`, `src/lib.rs`,
  regenerated `Cargo.lock`.
- Build: `gpui-auth = { path = "crates/gpui-auth", features =
  ["mock-provider"] }` in `[dependencies]`; append `"linux-native"` to the
  keyring feature list at `Cargo.toml:337`; `pub mod auth;` in
  services/mod.rs; add `auth` to the `pub use services::{...}` list in
  `src/lib.rs`.
- Tests: none.
- Done-check: `cargo check -p gpui-starter` passes on the host;
  `git diff Cargo.lock` shows `gpui-auth` in the gpui-starter dep list and
  `linux-keyutils` added; stage the lock.

### Step 7.2: `KeyringStore` + `install` + capabilities + session mapping

- Files: `src/services/auth.rs` (with the removal-recipe header), no test
  file yet.
- Build: imports kit-facade style (`use gpui_kit::{App, Global,
  Subscription, ...};` matching `src/services/secure_storage.rs:1`) plus
  `use gpui_auth::{...}`. `KeyringStore` per the contract: save =
  `serde_json::to_string(tokens)` then
  `crate::services::secure_storage::set_secret("gpui-starter.auth",
  provider_id, &json, cx)` mapped to `AuthError::Storage`; load =
  `get_secret` → `Ok(None)` on miss, parse failure → log warn
  (`gpui_starter::auth` target, no token text) + `Ok(None)` (corrupt row =
  signed out, the Zed pattern-B behavior); delete =
  `delete_secret` with a cfg pair on the miss path: native arm matches
  `Err(SecureStorageError::DeleteFailed { source, .. }) if
  matches!(source, keyring::Error::NoEntry)` → `Ok(())` (typed source
  field, `secure_storage.rs:30-36`; keyring direct dep; Display never
  spells "NoEntry" — `keyring-3.6.3/src/error.rs:68` — so string matching
  is off the table, Research Digest); wasm arm maps every error to
  `Storage` without miss-detection (wasm never selects `KeyringStore`).
  Everything else maps to `AuthError::Storage`. `install(cx)`: build `AuthIo`
  from `crate::services::tokio_runtime::runtime_and_client(cx)` natively
  (`Some(rt)` + client; absent global → `reqwest::Client::new()` + `None`,
  logged); wasm arm constructs `AuthIo { http:
  reqwest::Client::new() }` (no runtime field on wasm). Store pick:
  native `Arc::new(KeyringStore)`, wasm `Arc::new(gpui_auth::InMemoryStore::new())`.
  Provider: `Arc::new(gpui_auth::MockProvider::new())`. Create the entity
  (`cx.new(|cx| AuthController::new(provider, store, io, cx))`), call
  `controller.update(cx, |c, cx| c.restore(cx))`, then wire: `cx.observe`
  → map `AuthState` into `crate::services::session::SessionState`
  (`SignedOut`/`SigningIn`/`SignedIn { account_label }`/`Error(msg)`) and
  `crate::session::set_state(mapped, cx)`; `cx.subscribe` → on
  `AuthorizationRequested { url }`: native `crate::services::desktop_actions::open_url(url, cx)`
  (log failures at warn), wasm `cx.open_url(url)` (opens a new browser
  context; mock's `urn:` URLs are rejected by desktop_actions'
  http/https guard and logged, which is the documented demo behavior);
  `FlowFailed` → log at warn. Both `Subscription`s stored in
  `AuthHandle` (impl `Global`), set as a global, replacing any previous
  handle (idempotent install). Capability entries:
  `crate::capabilities::set("auth_email", ...)` — native and wasm both
  `CapabilityStatus::supported_enabled()` (REST over reqwest works on both;
  plan 22 registers the real provider); `auth_social` — native
  `supported_enabled()`, wasm struct literal with all five pub fields
  (`src/runtime/capabilities.rs:6-13`): `supported: false, enabled: true,
  degraded: true, reason: Some("social login needs the web redirect path
  from plan 23; browser loopback flows do not run on wasm"), last_error:
  None`.
- Tests: none yet.
- Done-check: `cargo clippy -p gpui-starter --all-features -- -D warnings`
  clean; `just wasm-check` passes (adapter compiles on wasm with the
  in-memory store).

### Step 7.3: `handle_callback_link`

- Files: `src/services/auth.rs`.
- Build: parse with `url::Url::parse(link)`; return false on parse error
  (the route parser will emit its own error); return true only when
  `url.scheme() == crate::routes::APP_URL_SCHEME && url.host_str() == Some("auth")`
  and the first non-empty path segment is `"callback"` — then
  `controller(cx)` is `Some` → `entity.update(cx, |c, cx|
  c.complete_callback(link, cx))` and true; controller absent → log warn +
  false (pre-install deep links fall through to normal routing, which will
  reject them — acceptable, documented). Query params ride the raw string
  into the crate untouched.
- Tests: none yet (7.4 covers it end to end).
- Done-check: `cargo check -p gpui-starter` passes.

## Phase 8: Routing seam, adapter tests, gates

### Step 8.1: AppRoot drain guards

- Files: `src/shell/root/app_root/state.rs`.
- Build: in the `DeepLinkReceived` arm (line 67) and the
  `ForwardedCommand::DeepLink` arm (line 94), insert as the first statement
  of each link-handling block: `if
  crate::services::auth::handle_callback_link(&link, cx) { continue; }`
  (both sit inside the `for event in events::drain(cx)` loop, lines 64-113,
  so `continue` is valid). No other drain change.
- Tests: existing `tests/qa_migration.rs` facade scan still passes (no
  banned imports added).
- Done-check: `cargo test -p gpui-starter --lib` passes (drain compiles;
  arms stay exhaustive).

### Step 8.2: Adapter tests

- Files: `src/services/auth.test.rs`, wired from `src/services/auth.rs`
  (`#[cfg(test)] #[path = "auth.test.rs"] mod auth_test;`).
- Build (`#[gpui_kit::test]`, `cx: &mut TestAppContext`, pattern
  `src/state/config_store.test.rs:280-281`): a `test_install(cx)` helper
  mirrors `install` but injects `gpui_auth::InMemoryStore` + `AuthIo`
  with `runtime: None` (extract `install_with(store, io, cx)` as the
  internal fn both call; `install` stays the one-line public wrapper).
  Tests: (1) install sets both capability entries (snapshot contains
  `auth_email` enabled; `auth_social` matches the cfg'd expectation of the
  host — native host → supported); (2) `handle_callback_link` returns true
  for `gpui-starter://auth/callback?code=1&state=2`, false for
  `gpui-starter://settings`, `gpui-starter://auth/other`,
  `not-a-url://x`; (3) full flow: install → `start_sign_in` →
  `cx.run_until_parked()` → capture the `AuthorizationRequested` URL →
  `handle_callback_link(url_built_with_that_state)` → parked →
  `crate::session::snapshot(cx).state == SessionState::SignedIn` (the
  observer ran) and the store row exists (expose a test-only store handle
  or assert via a second `restore`); (4) sign_out clears session state to
  `SignedOut`.
- Done-check: `cargo test -p gpui-starter --lib services::auth` passes.

### Step 8.3: Full gates + manual QA

- Files: none (verification only).
- Build: run everything in Verification.
- Tests: as listed there.
- Done-check: all gates green; manual checklist executed or explicitly
  waived with notes.

## Platform & Wasm Gating

- Crate-level cfg: tokio appears only under
  `[target.'cfg(not(target_family = "wasm"))'.dependencies]` in
  `crates/gpui-auth/Cargo.toml`; `AuthIo`'s `runtime` field and the rt-spawn
  method of `drive` are `#[cfg(not(target_family = "wasm"))]`; the wasm
  `drive` method and the `(?Send)` async_trait mode apply only on wasm (a
  plan-22 reqwest provider must be able to return `!Send` JsFuture-backed
  futures there — Research Digest citations). Wasm compiles the crate with
  reqwest (fetch), getrandom (`wasm_js`), and no tokio.
- `just wasm-check` (`cargo +nightly check --target wasm32-unknown-unknown
  -p gpui-starter --lib`, `justfile:24-25`) transitively checks the crate:
  the app's path dependency is unconditional.
- Wasm posture, recorded not hand-waved (README repeats this table):
  store = `InMemoryStore` (secure_storage wasm stubs fail closed,
  `secure_storage.rs:197-234`), so tokens vanish on reload and the user
  re-authenticates — the zero-risk default per survey Part C; provider
  futures drive on the GPUI executor (undriven tokio shim,
  `tokio_runtime.rs:39-44`); no loopback listener and no scheme
  registration exist on web (`register_url_scheme` no-op,
  `gpui-pre-web-0.3.9/src/platform.rs:468-470`; `on_open_urls` never fires
  on web, survey Part C), so browser flows need plan 23's https-redirect +
  boot query parsing — `auth_social` is registered degraded-unsupported on
  wasm for exactly that reason; `auth_email` is supported (fetch-based REST
  works; plan 22 completes it). Deep-link callbacks cannot arrive on wasm in
  this plan (nothing emits them there); the drain guard is harmless.
- Capability-registry entries: `auth_email`, `auth_social` (set in
  `install`, Phase 7.2; not in `DEFAULT_ENABLED_CAPABILITIES` — that list is
  for boot-time always-on services, `src/app/init.rs:95-108`).
- Native store features: keyring gains `linux-native` (keyutils,
  session-scoped; see Risks). macOS/Windows unchanged (`apple-native`,
  `windows-native`).
- Clocks: the crate is clock-free by construction — no `Instant`/`SystemTime`
  anywhere, expiry comparisons take `now_unix` parameters (seams §6 rule
  respected without needing `platform::clock`, which is app-side anyway).
- `mock-provider` is a plain feature: app enables it (dev/demo until plan
  22); boilerplate users delete one word to ship without mock.

## i18n

No catalog keys. The crate has no UI and emits no user-facing copy; error
text is developer-facing (`AuthError` Display, consumed by plans 22-24 which
own localization). The two capability reason strings are English literals
inside the adapter, matching the repo precedent (`secure_storage`'s
`WASM_UNAVAILABLE` at `secure_storage.rs:124`, plan 08's wasm literal).
Plans 22-24 own their `auth_*` catalog keys and must keep both locale files
in lockstep (`tests/qa_localization.rs:188-198`).

| key | en | zh-CN |
| --- | --- | --- |
| — (none) | — | — |

## Design Compliance

Brief (non-UI plan; full weight lands in 24). State surfaced by this plan is
consumed by existing surfaces only: `SessionState` (title bar/session reads,
unchanged) and the diagnostics page rendering the two capability entries
(`src/runtime/capabilities.rs:81-85`; whatever registers renders). No layout,
density, token, motion, or a11y decisions are introduced. The one interface
copy rule observed: `AuthState::Error(String)` and `AuthError` messages are
treated as developer text, not localized product copy, so plan 24 must wrap
them rather than display them raw (noted in the contract).

## Verification

Repo gates, in order (AGENTS.md):

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-features
cargo test --workspace --all-features
just wasm-check
```

`--workspace` now includes `gpui-auth`; its tests run in the third gate
(`--all-features` enables `mock-provider`). CI's `cargo test -p
gpui-starter --lib` lane additionally runs the adapter tests (Phase 8.2)
but not crate tests (seams §8) — the local gate is the real one. Stage the
regenerated `Cargo.lock` with both manifest edits (Seam Touches rows 1-3).

Feature-specific automated coverage:

- `cargo test -p gpui-auth --all-features`: SecretString redaction + serde,
  expiry/refresh predicates, RefreshPolicy merge, RFC 7636 S256 vector,
  base64url shape/uniqueness, InMemoryStore concurrency, RedirectParams
  parsing, MockProvider flow, controller state machine (happy path, state
  mismatch, error param, cancel, restore, refresh merge, sign-out).
- `cargo test -p gpui-auth` (no features): proves mock compiles out.
- `cargo test -p gpui-starter --lib services::auth`: capability entries,
  link routing truth table, end-to-end mock sign-in including the
  `SessionState` mapping.

Manual QA (native, Linux dev box):

1. `cargo run`; open Diagnostics: `auth_email` and `auth_social` rows
   present, both supported/enabled.
2. From a terminal: `xdg-open 'gpui-starter://auth/callback?code=x&state=y'`
   → app window focuses, log shows the `NoPendingFlow` failure (cold link,
   no flow started; `FlowFailed` event, `Error` auth state per Step 6.2) and
   no navigation occurs (route stays; diagnostics unaffected). A
   `StateMismatch` line is only reachable with a live flow and a wrong
   state value, which manual QA reaches through the plan-24 UI later.
3. Launch a second instance while the first runs and repeat step 2 → the
   link forwards over single-instance IPC to the primary (same log line
   there, second instance exits) — proves the `ForwardedCommand::DeepLink`
   guard.
4. `just wasm-check` green; load the wasm build if available: diagnostics
   shows `auth_social` unsupported with the plan-23 reason and
   `auth_email` enabled.

Acceptance criteria: all four gates pass; the three automated suites pass;
manual items 1-3 confirmed (4 confirmed or waived with a note). No
`AuthConfig`-style secrets in `AppConfig` (diff shows no config_store.rs
change); `grep -rn "mock-access-" src/` outside tests returns nothing
(tokens never logged; the mock token strings appear only in test asserts).
Known adjacent exposure, pre-existing and deferred to plan 22: the
single-instance deep-link log lines carry the callback query verbatim
(`src/platform/process/single_instance.rs:296-297, 415-416`) — an
authorization `code`, single-use, not a token; see Open Questions.

## Risks & Open Questions

Ranked risks:

1. **Wasm compile of the new dep set is unproven until the gate runs.**
   getrandom 0.4's `wasm_js` union and reqwest json-only on wasm were chosen
   from registry sources (feature tables read this session) but not compiled
   (doc-only scope). Mitigation: Phase 7.2's done-check runs
   `just wasm-check`; fallback lever if getrandom's wasm-bindgen version
   requirement cannot resolve against the locked 0.2.125, generate
   randomness with `js-sys` directly behind a cfg (the survey's Part C
   fallback), or pin getrandom tighter.
2. **Linux keyutils store is session-scoped.** Tokens do not survive
   reboot/logout; behavior = signed out on next boot, user re-authenticates.
   Chosen over `sync-secret-service` because that path adds a
   `libdbus-1-dev` build requirement to every Linux dev/CI machine for a
   store nothing consumes until plan 22. Upgrade lever (one manifest line +
   `scripts/install-linux-deps.sh` entry): keyring
   `linux-native-sync-persistent`. Also untested at runtime here; if keyutils
   is unavailable in some sandbox, `secure_storage::initialize`'s
   availability probe reports it (capability `secure_storage` → error) and
   `KeyringStore` saves fail visibly rather than silently.
3. **Interface Contract stability for 22-24.** Every signature they build on
   is in one block; any change during implementation reflows it. Rule
   written into the contract header (append prose, never reflow). If
   implementation forces a signature change, plans 22-24 must be re-read
   before merge.
4. **Blocking keyring calls on the entity context.** `KeyringStore` methods
   run inside the controller's entity update (UI thread), same as every
   existing `secure_storage` call site today. Accepted for the foundation;
   if profiling later objects, move store calls into the spawned task and
   hop back via the weak entity (the spawn/update pattern is already in
   place from Step 6.1).
5. **Mock provider ships enabled.** Boilerplate-demo value (and plan 24's
   stand-in until 22/23 land); its `urn:` authorize URL cannot trip the OS
   browser (adapter's http/https guard, `desktop_actions.rs:150-153`).
   Removal is one feature word in the app manifest line.
6. **Plan 08 lands a `members` line first** (both plans edit `[workspace]`).
   Whoever lands second appends to the existing list rather than rewriting
   it (grep `members` before editing; same class as the §5 seam-collision
   entries in INDEX.md).

Open questions (each with what was tried):

- **oauth2/openidconnect deferred to plan 23** — decided, not open: the
  foundation's helpers cover PKCE/state deterministically without the heavier
  graph; the survey's A2 rows carry the 23 picks with their own caveats
  (oauth2 5.0 async-client wiring unverified until compiled there).
- **`AuthIo::drive` with `runtime: None` natively**: direct await of provider
  futures on the GPUI executor. Safe for the mock (no HTTP); real providers
  construct with `Some(rt)` via `install`. Not an open question, but the
  None-fallback is test-only by convention — documented in the contract and
  README rather than enforced by type.
- **KeyringStore delete-miss semantics — RESOLVED by review**: the draft
  string-matched `NoEntry` on the error Display, which never fires (Display
  renders "No matching entry found in secure storage",
  `keyring-3.6.3/src/error.rs:68`). Fix in place: typed
  `matches!(source, keyring::Error::NoEntry)` on the native
  `DeleteFailed { source, .. }` (`secure_storage.rs:30-36`; keyring is a
  direct app dep, `Cargo.toml:337`). The wasm twin variant has only a
  `reason: String` (`secure_storage.rs:60-65`); that arm maps all errors
  without miss-detection, which costs nothing because wasm never selects
  `KeyringStore`. If a typed miss variant is ever wanted on both targets,
  plan 22 can extend `SecureStorageError` — one small seam edit, noted so
  it is a decision, not a surprise.
- **Deep-link logging leaks the authorization `code` param (pre-existing)**:
  the single-instance forwarder info-logs the full link verbatim at both
  `DeepLinkReceived` emit sites (`src/platform/process/single_instance.rs:296-297`
  and `415-416`). An OAuth `code` is single-use and short-lived (not a
  token), but it is still a credential-shaped value in logs. This plan
  changes neither site (they serve every deep link, auth ones included);
  plan 22 should either redact query strings in those two log lines or
  accept and document the exposure. Recorded here so it is a visible
  decision for 22, not an oversight.
- **Second-instance forwarding of callback links on wasm**: N/A by
  construction (single-instance is native-only,
  `single_instance.rs:1-2`); recorded so plan 23's web path knows the wasm
  callback arrives only via page navigation.
- **No cargo gate was run during planning** (doc-only scope, matching the
  surveys and plans 01-20). All commands in Verification are for the
  implementer; the first `cargo check -p gpui-auth` (Step 1.1) and the first
  `just wasm-check` (Step 7.2) are the earliest compile evidence points.
