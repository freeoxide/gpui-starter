# 22 Email + Password Auth Plan

## Goal

Email+password authentication runs end to end against an app-owned `AuthBackend`
trait: sign up, sign in, refresh before expiry, and sign out, with tokens
persisted through plan 21's `TokenStore`. A demo backend (`DemoBackend`, an
in-process user store with argon2 hashing, deterministic expiring tokens, and
working refresh) makes the whole flow run offline and testable headlessly; a
TOTP phase adds enrollment and challenge/verify; email verification is a
surfaced state, nothing sent. Every flow state (`SigningUp`, `AwaitingChallenge`,
account status) is exposed on plan 21's `AuthController` so plan 24 renders it.
An optional axum dev-server backend mirrors plan 08's collab-server decision.

## Dependencies

Plan `21-auth-foundation.md`, implemented and green. Contract items consumed
(cited as `21:Interface Contract`; that block is line-stable, this plan appends
to it, see Interface Contract below):

- `AuthController` + `AuthState` + `AuthEvent` (entity, states, events), and
  its `refresh` / `restore` / `sign_out` methods.
- `TokenStore` + `InMemoryStore` (wasm store, test injection), `KeyringStore`
  (app adapter, 21 Phase 7.2).
- `AuthIo` (cfg'd `drive`, native `runtime` field), `AuthProvider` trait
  (cfg_attr Send split), `FlowLaunch` (not used by the email flow; the provider
  returns `NoBrowserFlow`), `RedirectParams` (unused, no browser redirect).
- `SecretString` (password transport, zeroized), `TokenSet` + `AccountInfo` +
  `RefreshPolicy` (demo mints `ReuseUntilDead` sets), `AuthError`.
- `MockProvider` (wasm fallback provider, unchanged).
- App-side adapter surface from 21: `install` / `install_with`,
  `controller(cx)`, `AuthHandle`, `handle_callback_link`, `KeyringStore`
  (`src/services/auth.rs` as 21 builds it).

Repo services: 22 changes none of their code. `session.rs` (observer mapping
wired by 21), `secure_storage.rs` (behind `KeyringStore`), `tokio_runtime.rs`
(`runtime_and_client` consumed inside 21's `install`), `desktop_actions.rs` and
the deep-link/event drain (`DeepLinkReceived`, `ForwardedCommand::DeepLink`
guards from 21 Phase 8.1) are all consumed as-is. The email flow has no browser
step: no `AuthorizationRequested` is ever emitted, no callback link exists, so
`open_url` and the drain guards are dormant for this provider. Boot order
dependency: 21's `install` call sits after the tokio, secure-storage, and
session globals (`src/app/init.rs:207-217`, verified this session).

Plans 01-20: no items consumed (no canvas/floating/waveform surface). Plan 08's
*decision* (separate workspace member for a dev server, axum 0.8, justfile
recipe, loopback integration test) is mirrored by the optional Phase 6.

## Research Digest

Path keys: `GPUI` = `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-pre-0.3.9/src`,
`ARGON2` = `.../argon2-0.5.3`, repo files by `path:line`. gpui-pre 0.3.9 /
gpui-kit 0.7.3 located on disk per survey key (`ls ~/.cargo/registry/src/*/ |
grep -E 'gpui-pre-0.3.9|gpui-kit-0.7.3'` → both present). Registry has
`argon2-0.5.3` (no 0.6.x); `totp-rs` is NOT in the registry cache, its 6.0.0 API
was read on docs.rs (fetched 2026-10-12); `axum-0.7.9` is on disk but plan 08
standardized on axum 0.8.9, and this plan follows 08 for the optional server.

Crate facts, verified:

- argon2 0.5.3 (`ARGON2/src`): `Argon2::new(Algorithm, Version, Params)` at
  `lib.rs:194`; `hash_password_into(pwd, salt, out)` at `lib.rs:229`, gated
  `#[cfg(feature = "alloc")]`; `Params::new(m_cost, t_cost, p_cost,
  output_len: Option<usize>) -> Result<Self>` is `pub const fn` at
  `params.rs:107`, cost floors `MIN_M_COST = 2 × SYNC_POINTS` (8 KiB),
  `MIN_T_COST = 1`, `MIN_P_COST = 1` (`params.rs:43-67`), defaults m=19 KiB
  blocks / t=2 / p=1 (`params.rs:43-45,54,61`). `Algorithm` derives `Default`
  with `#[default] Argon2id` (`algorithm.rs:28-52`); `Version::default()` is
  `V0x13` (`version.rs:8-15`). Features (`ARGON2/Cargo.toml [features]`, read
  this session): `default = ["alloc", "password-hash", "rand"]`, `alloc =
  ["password-hash/alloc"]`, so `default-features = false, features = ["alloc"]`
  turns off argon2's PHC-string methods (gated on argon2's own
  `password-hash` feature) and the randCore salt helper, and keeps rand out of
  the graph — but it leaves the optional `password-hash` crate in the dep
  graph with its `alloc` feature on (cost framing only; the API claim below
  is unaffected because `hash_password_into` at `lib.rs:229` is gated only on
  argon2's `alloc`). Salt and hash are
  plain bytes in/out of `hash_password_into`; no `SaltString` needed, so the
  crate's existing `getrandom` + `base64` deps (21 manifest) cover salt
  generation and storage encoding.
- totp-rs 6.0.0 (docs.rs, fetched 2026-10-12; not on disk, not compiled): the
  6.0 API is builder-based. `Builder::new()`, `with_secret(impl Into<Secret>)`,
  `with_step_duration(u64)`, `with_account_name(...)`, `with_issuer(Option<..>)`
  (both otpauth-gated), `build() -> Result<Totp, TotpError>`; defaults SHA1, 6
  digits, skew 1, 30 s. `Totp::generate(time: u64) -> Token`,
  `Totp::check(token: &str, time: u64) -> Option<u64>` (time is unix seconds;
  the crate divides by step), `Totp::to_url() -> Result<String, TotpError>`
  (otpauth), `Totp::secret() -> &Secret`. `Secret::try_from_base32(...)`,
  `Secret::to_base32() -> String`, `Secret::as_bytes()`, `From<[u8; N]>`
  (alloc-gated methods; `Secret` is a struct in 6.0, not the 5.x enum).
  Dependencies: sha1 0.11, sha2 0.11, hmac 0.13 unconditional; base32, url,
  percent-encoding, rand, serde, zeroize optional behind features. Feature
  table (docs.rs `/crate/totp-rs/6.0.0/features`, fetched 2026-10-12):
  `default = ["migration", "std"]`; `std` = `alloc` + rand's `thread_rng`
  (defaults DO pull rand); `alloc` pulls base32; `otpauth` = `alloc` +
  percent-encoding + url; `zeroize` adds the zeroize dep; `qr` adds
  qrcodegen-image and implies `otpauth`. Our minimal set
  `default-features = false, features = ["alloc", "otpauth", "zeroize"]`
  therefore excludes rand, serde, and qr. `check`
  panics at digits >= 10; defaults are safe.
- Wasm clock rule honored by construction: the crate stays clock-free (21
  Research Digest); the only clock is an `Arc<dyn Fn() -> i64 + Send + Sync>`
  closure injected app-side, where `chrono::Utc::now()` already has a call-site
  precedent (`src/app/lifecycle.rs:218`) and chrono's `wasmbind` feature makes
  it wasm-correct (`Cargo.toml:371`).
- Refresh loop primitives: `AsyncApp::background_executor(&self) ->
  &BackgroundExecutor` (`GPUI/app/async_context.rs:159`),
  `BackgroundExecutor::timer(Duration) -> Task<()>` (`GPUI/executor.rs:183`),
  in-repo timer-loop precedent `src/gallery/sections/tables/data_table.rs:803`;
  `WeakEntity::update` errors after release (`GPUI/app/entity_map.rs:803-813`,
  cited by 21 this run), which terminates the loop's `break` path.
- Capability surface: `CapabilityStatus` has five pub fields
  (`src/runtime/capabilities.rs:6-13`); the prebuilt `degraded()`/`error()`
  constructors keep `supported: true`, so the wasm "unsupported" arm needs a
  struct literal, exactly like 21's `auth_social` wasm entry.
- No auth surface exists in gpui-kit 0.7.3 (`grep -rn -ilE 'keyring|oauth|
  credential' gpui-kit-0.7.3/src/` → no hits, survey Part 6): everything here is
  ours on top of 21.
- Survey Part D decision inputs consumed: A2 argon2 row (correctly placed
  server-side; the demo backend IS the server), A2 totp-rs row (duplicate sha2
  majors noted), A1 (koruma stays app-side; the crate does minimal syntactic
  email checks), D22 MFA row (secret storage: demo backend keeps secrets in its
  in-process store; the OS `TokenStore` never holds TOTP secrets).
- Design-guider discipline (coding guides, "Organize large applications by
  capability"; public-API naming table): trait named `AuthBackend` per spec,
  controller-facing trait `PasswordAuth` (`<Role>Provider`-style seam), readers
  `is_`/plain-noun, no `pub` fields on behavioral types.

Crate picks:

| Dep | Version | Why |
| --- | --- | --- |
| `argon2` | 0.5, `default-features = false`, `["alloc"]` | password hashing (RFC 9106 Argon2id); 0.5.3 sources on disk, API line-verified above; 0.6.0 exists (survey A2) but is not in the registry cache, see Open Questions |
| `totp-rs` | 6.0, `default-features = false`, `["alloc", "otpauth", "zeroize"]` | RFC 6238 with explicit-time `check(token, unix)` (fits the clock-free crate); `otpauth` for the enrollment URL; no `gen_secret` (secret bytes come from the crate's `getrandom`), no `qr` (plan 24 decides), no `rand` |
| (server phase only) `axum` | 0.8 | mirrors plan 08's collab-server pick; wiring cited there (docs.rs evidence, 08 Research Digest) |

Both crypto deps go under the crate's
`[target.'cfg(not(target_family = "wasm"))'.dependencies]` table (21 already
has that table for tokio), so the wasm build never sees them.

No cargo gate was run during planning (doc-only scope, matching 21 and the
surveys). Every Verification command is for the implementer; Step 2.1's
`cargo check -p gpui-auth` is the first compile evidence for argon2, Step 4.1's
for totp-rs.

## Architecture

Two mock-ish things exist after this plan; keep them distinct:

| Thing | Plan | Role | id |
| --- | --- | --- | --- |
| `MockProvider` | 21 | scripted browser-flow provider; wasm fallback, tests | `"mock"` |
| `DemoBackend` | 22 | in-process `AuthBackend`: real argon2, real token lifecycle, no UI, no network | `"demo"` |

Entities and ownership:

- `AuthBackend` (crate trait, `email.rs`): the spec's app-owned backend seam.
  Pure data-in/data-out async methods, all taking `now_unix` so backends stay
  deterministic and the crate stays clock-free. Three implementations: the
  native `DemoBackend`, the optional native `RemoteBackend` (Phase 6), and test
  backends in both crate and app tests.
- `EmailPasswordProvider` (crate, `email.rs`): implements 21's `AuthProvider`
  (id `"email"`, `ReuseUntilDead`, `refresh`/`revoke` delegate to the backend,
  `start_flow`/`complete_callback` return `AuthError::NoBrowserFlow`) and the
  new controller-facing `PasswordAuth` trait (`sign_up`, `sign_in`,
  `submit_totp`, `begin_totp_enrollment`, `confirm_totp_enrollment`). It holds
  the backend `Arc` and the injected clock closure; every backend call passes
  `clock()`.
- `AuthController` (21, amended): gains `password: Option<Arc<dyn PasswordAuth>>`,
  `clock: Option<Arc<dyn Fn() -> i64 + Send + Sync>>`, `pending_challenge`,
  `email_status`, `pending_enrollment`, `enrollment_error`. Same single-writer
  rules as 21: only the controller writes the store row and the pending slots.
  The browser-flow methods are untouched; email methods are additive.
- Secrets: passwords live only in `SecretString` arguments (zeroized on drop)
  and the demo backend's argon2 digest (salt+hash, never the password). Tokens
  live in `TokenSet`/`TokenStore` per 21. TOTP secrets live in the demo user
  store (process memory), never in the `TokenStore`, never in `AppConfig`, never
  logged. No `AppConfig` field is added (snapshot schema pinned,
  `tests/snapshot_tests.rs:26,57`).

State machine additions (`SignedOut` / `SigningIn` / `SignedIn` / `Error` from
21): `SigningUp` (in-flight sign-up), `AwaitingChallenge { email, error }`
(pending TOTP challenge; `error` set on a rejected code so the challenge
survives failures). Transitions: sign-up `SignedOut → SigningUp → SignedOut`
(plus `AuthEvent::AccountCreated`); sign-in `SignedOut|Error → SigningIn →
SignedIn` (tokens persisted) or `→ AwaitingChallenge`; `submit_totp`
`AwaitingChallenge → SignedIn` or stays with `error` set; `cancel_challenge →
SignedOut`. Email verification is a state, not a gate: `EmailAccountStatus.
verified` is `false` forever in the demo (nothing is sent), surfaced through
`controller.email_status()` for plan 24's badge; sign-in never blocks on it.

Data flow, sign in (native):

```text
UI/command (24) ──sign_in_with_password(email, pwd)──▶ AuthController
  guard: password auth enabled (else Error(PasswordAuthUnavailable))
  state = SigningIn ; cx.notify ; cx.spawn
  io.drive(password.sign_in(http, &email, &pwd))          [tokio rt / direct await on wasm]
  ◀─ SignInOutcome
     Authenticated(session): on_tokens(session.tokens)   [shared with 21's complete_callback:
       provider_id check → store.save → SignedIn{label} → email_status → notify → FlowCompleted]
     TotpRequired(challenge): pending_challenge = challenge ; state = AwaitingChallenge ; notify
adapter observer ──▶ session::set_state(SessionState::…)   [21 wiring, unchanged]
```

Refresh before expiry: the adapter spawns one 60 s timer loop; each tick calls
`controller.refresh_if_needed(120, cx)` which loads the store row, computes
`TokenSet::needs_refresh(clock(), 120)`, and only then re-runs 21's refresh body
(refactored into a private `do_refresh`). Demo access tokens expire after 900 s,
so the refresh lands between 60 and 120 s before expiry (first tick past
`expires_at - margin`). The loop is a
`WeakEntity` task: it exits when the controller is released.

DemoBackend model (native, process-lifetime):

```text
users:     Mutex<HashMap<email, DemoUser { password: "v1$b64(salt)$b64(hash)",
             verified: false, totp: Option<Totp>, created_at }>>
sessions:  Mutex<HashMap<refresh_token, DemoSession { email, access, access_exp }>>
by_access: Mutex<HashMap<access_token, refresh_token>>
challenges: Mutex<HashMap<challenge_id, { email, expires_at }>>        (TOTP phase)
pending_totp: Mutex<HashMap<email, Totp>>                              (TOTP phase, unconfirmed enrollments)
seq: AtomicU64                                                        (token + challenge ids)
```

Deterministic tokens: `demo-access-{seq}` / `demo-refresh-{seq}`, minted in call
order; `expires_at = now + 900`. Refresh keeps the refresh token (matching the
provider's `ReuseUntilDead` policy) and mints the next access seq; revoke(access)
removes the session and both index entries; refresh with an unknown or deleted
token → `AuthError::RefreshRejected`. Users are ephemeral by design (spec:
HashMap store): after an app restart the persisted `TokenSet` restores
`SignedIn`, and the first `refresh_if_needed` surfaces `Error(RefreshRejected)`
until the user signs in again. Documented, not fixed.

Module tree:

```text
crates/gpui-auth/src/
├── email.rs            PasswordAuth, AuthBackend, SignInOutcome, EmailAccountStatus,
│                       TotpChallenge, TotpEnrollment, EmailPasswordProvider  (all targets)
├── email.test.rs
├── demo_backend.rs     DemoBackend: argon2 store, sessions, tokens  (cfg not-wasm)
├── demo_backend.test.rs
├── remote.rs           RemoteBackend over the dev-server REST API   (cfg not-wasm, Phase 6)
├── remote.test.rs
└── error.rs / controller.rs / lib.rs   amended (see Interface Contract)

auth-dev-server/                     optional workspace member (Phase 6)
├── Cargo.toml                       axum 0.8 + argon2 + tokio
└── src/main.rs                      POST /auth/{sign-up,sign-in,refresh,revoke}, GET /healthz

src/services/auth.rs                 amended adapter: email provider native, MockProvider wasm,
                                     enable_password + set_clock + refresh loop, capability tweak
src/services/auth.test.rs            extended: headless e2e suites
```

Crate boundary (unchanged from 21, extended): `crates/gpui-auth` owns every auth
type and flow; `src/services/auth.rs` stays the only app-side adapter, importing
`gpui_auth::email::{...}` alongside the kit facade. No app module outside
`services/auth.rs` gains an auth import.

## Interface Contract

Amendment to `21:Interface Contract`. 21's block is line-stable; this plan
appends a pointer note under it (prose only, never reflowed) and lands the
additions below. All additions are purely additive (new enum variants appended,
new methods, new error variants appended at the end of `AuthError`); plans 23/24
must re-read this block before building. Consumers cite it as
`22:Interface Contract`.

```rust
// crates/gpui-auth/src/error.rs — variants APPENDED to AuthError
#[error("invalid email address")]
InvalidEmail,
#[error("password does not meet the minimum policy")]
WeakPassword,
#[error("email already registered")]
EmailTaken,
#[error("invalid email or password")]
InvalidCredentials,
#[error("backend rejected the token")]
RefreshRejected,
#[error("provider has no browser authorization flow")]
NoBrowserFlow,
#[error("password auth is not available on this controller")]
PasswordAuthUnavailable,
#[error("no challenge is pending")]
NoPendingChallenge,
#[error("challenge unknown or expired")]
InvalidChallenge,
#[error("TOTP code rejected")]
TotpFailed,
#[error("TOTP not enrolled for this account")]
TotpNotEnrolled,

// crates/gpui-auth/src/email.rs
pub const EMAIL_PROVIDER_ID: &str = "email";

/// Account state as the backend reports it. `verified` is a modeled state in
/// the demo (nothing is sent), surfaced for UI badges; it never gates sign-in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmailAccountStatus { pub email: String, pub verified: bool }

#[derive(Clone, Debug)]
pub struct AuthenticatedSession { pub tokens: TokenSet, pub status: EmailAccountStatus }

#[derive(Clone, Debug)]
pub enum SignInOutcome {
    Authenticated(AuthenticatedSession),
    TotpRequired(TotpChallenge),
}

/// Opaque pending-challenge handle; `email` is carried for UI copy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TotpChallenge { pub id: String, pub email: String }

/// One enrollment round: secret shown for manual entry + otpauth:// URL.
/// Manual `Debug`: `secret_base32` AND `otpauth_url` print `[redacted]`
/// (the URL embeds the secret) — same discipline as `SecretString`.
#[derive(Clone)]
pub struct TotpEnrollment { pub email: String, pub secret_base32: String, pub otpauth_url: String }
impl std::fmt::Debug for TotpEnrollment { /* email clear, both secret-bearing fields "[redacted]" */ }

/// Backend seam (spec's app-owned AuthBackend). All methods take `now_unix`
/// so implementations stay deterministic; none touch GPUI contexts.
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
pub trait AuthBackend: Send + Sync {
    fn id(&self) -> &str;
    async fn sign_up(&self, http: reqwest::Client, email: &str, password: &SecretString,
                     now_unix: i64) -> Result<EmailAccountStatus, AuthError>;
    async fn sign_in(&self, http: reqwest::Client, email: &str, password: &SecretString,
                     now_unix: i64) -> Result<SignInOutcome, AuthError>;
    async fn verify_totp(&self, http: reqwest::Client, challenge: &TotpChallenge, code: &str,
                         now_unix: i64) -> Result<AuthenticatedSession, AuthError>;
    async fn begin_totp_enrollment(&self, http: reqwest::Client, email: &str)
        -> Result<TotpEnrollment, AuthError>;
    async fn confirm_totp_enrollment(&self, http: reqwest::Client, email: &str, code: &str,
                                     now_unix: i64) -> Result<(), AuthError>;
    async fn refresh(&self, http: reqwest::Client, refresh_token: &str,
                     now_unix: i64) -> Result<TokenSet, AuthError>;
    async fn revoke(&self, http: reqwest::Client, token: &str) -> Result<(), AuthError>;
}

/// Controller-facing credential seam. Same Send split as 21's AuthProvider.
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
pub trait PasswordAuth: Send + Sync {
    async fn sign_up(&self, http: reqwest::Client, email: &str, password: &SecretString)
        -> Result<EmailAccountStatus, AuthError>;
    async fn sign_in(&self, http: reqwest::Client, email: &str, password: &SecretString)
        -> Result<SignInOutcome, AuthError>;
    async fn submit_totp(&self, http: reqwest::Client, challenge: &TotpChallenge, code: &str)
        -> Result<AuthenticatedSession, AuthError>;
    async fn begin_totp_enrollment(&self, http: reqwest::Client, email: &str)
        -> Result<TotpEnrollment, AuthError>;
    async fn confirm_totp_enrollment(&self, http: reqwest::Client, email: &str, code: &str)
        -> Result<(), AuthError>;
}

pub struct EmailPasswordProvider { /* backend, clock */ }
impl EmailPasswordProvider {
    /// `clock` supplies unix seconds; injected app-side (chrono Utc::now).
    pub fn new(backend: std::sync::Arc<dyn AuthBackend>,
               clock: std::sync::Arc<dyn Fn() -> i64 + Send + Sync>) -> Self;
}
impl AuthProvider for EmailPasswordProvider {
    // id() = EMAIL_PROVIDER_ID, display_name() = "Email",
    // refresh_policy() = RefreshPolicy::ReuseUntilDead,
    // start_flow()/complete_callback() = Err(AuthError::NoBrowserFlow),
    // refresh()/revoke() delegate to the backend with clock().
}

// crates/gpui-auth/src/demo_backend.rs — cfg(not(target_family = "wasm"))
pub struct DemoBackend { /* users, sessions, indexes, challenges, seq, argon2 */ }
impl DemoBackend {
    /// 8 MiB / 2 passes / p=1 Argon2id: real hash, tolerable in dev profiles.
    pub fn new() -> Self;                                          // + Default
    /// Custom cost parameters (tests use Params::new(8, 1, 1, None)).
    pub fn with_params(params: argon2::Params) -> Self;
}
impl AuthBackend for DemoBackend { /* id "demo" */ }

// crates/gpui-auth/src/controller.rs — amendments to AuthController
#[derive(Clone, Debug, PartialEq)]
pub enum AuthState {
    SignedOut,
    SigningIn,
    SigningUp,                                        // 22
    AwaitingChallenge { email: String, error: Option<String> },  // 22
    SignedIn { account_label: String },
    Error(String),
}

#[derive(Clone, Debug)]
pub enum AuthEvent {
    AuthorizationRequested { url: String },
    FlowCompleted,
    FlowFailed { error: AuthError },
    SignedOut,
    AccountCreated { email: String },                 // 22
}

impl AuthController {
    /// Register the credential seam. Call once after `new`; None (default)
    /// makes every password method fail with PasswordAuthUnavailable.
    pub fn enable_password(&mut self, auth: std::sync::Arc<dyn PasswordAuth>);
    /// Unix-seconds clock for expiry checks; without it refresh_if_needed
    /// logs once and no-ops.
    pub fn set_clock(&mut self, clock: std::sync::Arc<dyn Fn() -> i64 + Send + Sync>);
    pub fn email_status(&self) -> Option<&EmailAccountStatus>;
    pub fn totp_enrollment(&self) -> Option<&TotpEnrollment>;
    pub fn totp_enrollment_error(&self) -> Option<&str>;
    /// No-op (warn log) while SignedIn; otherwise SigningUp → backend
    /// sign_up → AccountCreated event, back to SignedOut.
    pub fn sign_up_with_password(&mut self, email: String, password: SecretString,
                                 cx: &mut gpui::Context<Self>);
    /// SigningIn → backend sign_in → Authenticated (on_tokens: persist,
    /// SignedIn, FlowCompleted) or TotpRequired (AwaitingChallenge).
    pub fn sign_in_with_password(&mut self, email: String, password: SecretString,
                                 cx: &mut gpui::Context<Self>);
    /// AwaitingChallenge only: wrong code keeps the challenge, sets
    /// `error`; success runs on_tokens. Requires a pending challenge.
    pub fn submit_totp(&mut self, code: String, cx: &mut gpui::Context<Self>);
    /// Drop the pending challenge → SignedOut.
    pub fn cancel_challenge(&mut self, cx: &mut gpui::Context<Self>);
    /// SignedIn only: fetches a new enrollment round for the signed-in
    /// account (begin_totp_enrollment).
    pub fn enroll_totp(&mut self, cx: &mut gpui::Context<Self>);
    /// Verifies the code against the pending enrollment and enables TOTP
    /// for the account; failure keeps the enrollment and records the error.
    pub fn confirm_totp_enrollment(&mut self, code: String, cx: &mut gpui::Context<Self>);
    /// Loads the store row, refreshes only when
    /// TokenSet::needs_refresh(clock(), margin_secs); no row → no-op.
    pub fn refresh_if_needed(&mut self, margin_secs: i64, cx: &mut gpui::Context<Self>);
}
// sign_out (21) additionally clears pending_challenge, email_status,
// pending_enrollment, and enrollment_error before SignedOut.
```

App-side surface (inside `src/services/auth.rs`, all `pub`):

```rust
/// install_with as 21 defined it, plus the provider and clock seams.
/// `install` stays the one-line public wrapper: both arms build the clock
/// closure (`chrono::Utc::now().timestamp()`); native additionally builds
/// Arc<EmailPasswordProvider> over DemoBackend (Phase 6 adds the
/// GPUI_AUTH_DEV_SERVER_URL override) and passes it as both provider and
/// Some(password); wasm passes MockProvider and None. install_with applies
/// enable_password + set_clock on the entity itself, so test installs
/// (Phase 5.3) inject the fixed clock through this same parameter.
fn install_with(store: std::sync::Arc<dyn TokenStore>, io: AuthIo,
                provider: std::sync::Arc<dyn AuthProvider>,
                password: Option<std::sync::Arc<dyn PasswordAuth>>,
                clock: std::sync::Arc<dyn Fn() -> i64 + Send + Sync>,
                cx: &mut gpui_kit::App);
```

Binding notes for dependents: `sign_up`/`sign_in`/`submit_totp` may be called
from any state (in-flight slots are replaced, one flow at a time, same rule as
21's `start_sign_in`); the controller performs exactly one `store.save` per
successful authentication and one `store.delete` on sign-out; the label chain
stays 21's (`display_name` → `email` → provider `display_name()`), and the demo
backend fills both `display_name` and `email` with the address so labels read as
emails. Error/state strings are developer-facing; plan 24 wraps them, never
displays raw (21 Design Compliance rule carried forward).

## Module Layout

| File | Purpose |
| --- | --- |
| `crates/gpui-auth/src/email.rs` | `AuthBackend`, `PasswordAuth`, outcome/status/challenge types, `EmailPasswordProvider`; compiles on all targets |
| `crates/gpui-auth/src/email.test.rs` | provider tests against a scripted `TestBackend` |
| `crates/gpui-auth/src/demo_backend.rs` | `DemoBackend`: argon2 user store, sessions, deterministic tokens, TOTP store (cfg not-wasm) |
| `crates/gpui-auth/src/demo_backend.test.rs` | backend unit tests (pure sync core, plain `#[test]`) |
| `crates/gpui-auth/src/remote.rs` | `RemoteBackend`: dev-server REST client (cfg not-wasm, Phase 6) |
| `crates/gpui-auth/src/remote.test.rs` | request/response mapping tests (Phase 6) |
| `crates/gpui-auth/Cargo.toml` | argon2 + totp-rs under the not-wasm table |
| `crates/gpui-auth/README.md` | email-flow usage, demo-backend caveats, wasm posture |
| `src/services/auth.rs` | amended adapter (provider pair, clock, refresh loop, capability) |
| `src/services/auth.test.rs` | extended headless e2e suites |
| `auth-dev-server/Cargo.toml` + `src/main.rs` | optional dev server (Phase 6) |

## Seam Touches

Every edit outside the new files:

| # | File | Change | Why minimal |
| --- | --- | --- | --- |
| 1 | `crates/gpui-auth/src/error.rs` | append 11 variants to `AuthError` | additive; no existing variant touched |
| 2 | `crates/gpui-auth/src/controller.rs` | 5 new fields, 10 new methods, 2 new `AuthState` variants, 1 new `AuthEvent` variant, sign_out clears email state, private `do_refresh` refactor shared by `refresh`/`refresh_if_needed`, private `on_tokens` refactor shared with `complete_callback` | additive methods; the two refactors move 21's existing bodies verbatim into named private fns so the browser path and the password path share one persistence path |
| 3 | `crates/gpui-auth/src/lib.rs` | `pub mod email;` + re-exports; `#[cfg(not(target_family = "wasm"))] pub mod demo_backend;` (+ `pub mod remote;` in Phase 6); re-export `DemoBackend` behind the same cfg | module seam, 21's pattern |
| 4 | `crates/gpui-auth/Cargo.toml` | not-wasm table gains argon2 + totp-rs lines | keeps both out of the wasm graph |
| 5 | root `Cargo.toml` | NEW table `[target.'cfg(not(target_family = "wasm"))'.dev-dependencies]` with `argon2` (same 0.5.3 pin as the crate) + `totp-rs` (same feature set as the crate) — Step 5.3's app tests construct `argon2::Params` and generate TOTP codes, and the app crate itself never depends on either outside tests | dev-deps never reach production or wasm builds; target-gated so the wasm lane never resolves them (same shape as the existing gpui `test-support` dev-dep, `Cargo.toml:216`) |
| 6 | `Cargo.lock` | regenerate + stage with each manifest edit | repo practice (21 seam row 4) |
| 7 | `src/services/auth.rs` | `install_with` gains provider/password/clock params; native arm constructs `EmailPasswordProvider` over `DemoBackend`; both arms pass the chrono clock and `install_with` applies `enable_password` + `set_clock` on the entity; wasm arm keeps MockProvider/None; wasm `auth_email` capability switches to an unsupported struct literal; spawn the 60 s refresh loop | the adapter is the single app-side seam 21 established |
| 8 | `src/services/auth.test.rs` | new suites (Phase 5.3) | in-lib, CI-executed (seams §7/§8) |
| 9 | `docs/plans/21-auth-foundation.md` | append one prose paragraph under the Interface Contract pointing at `22:Interface Contract` for the amendments | 21's own rule: append prose, never reflow the block |
| 10 | (Phase 6 only) root `Cargo.toml` `[workspace]` members += `"auth-dev-server"` (grep-first: 08 adds `collab-protocol`/`collab-server`, 21 adds `crates/gpui-auth`; append, never rewrite) + `justfile` recipe | mirrors 08 seam rows 1/5 |

Deliberate non-touches: no `src/app/init.rs` edit (21's install line stands),
no route/VALID_HOSTS change (no new host; `auth` host semantics from 21 are
unchanged and dormant for this provider), no deep-link/drain changes, no
`desktop_actions.rs` change, no `AppConfig` field, no locale keys (crate and
adapter emit no user-facing copy; capability reason strings are English
literals per 21 precedent), no `DEFAULT_ENABLED_CAPABILITIES` edit, no new
capability id (`auth_email` from 21 is amended in place, one owning feature per
INDEX §4), no `[patch.crates-io]` change.

### Removal Recipe

`crates/gpui-auth/src/email.rs` opens with (mirrors `src/gallery/mod.rs:1-7`):

```rust
//! Removable email+password auth. `AuthBackend`, `PasswordAuth`, and
//! `EmailPasswordProvider` are the crate surface; `DemoBackend` is the native
//! demo implementation. Removal = delete email.rs/demo_backend.rs
//! (+remote.rs), revert the AuthError/AuthState/AuthEvent additions and the
//! controller's password methods in this crate, restore 21's three-arg
//! install_with in src/services/auth.rs, drop the argon2/totp-rs manifest
//! lines, and (if Phase 6 landed) remove the auth-dev-server member and
//! justfile recipe.
```

Ordered removal steps:

1. Revert `src/services/auth.rs` to 21's shape: `install_with(store, io, cx)`
   with MockProvider (native too), delete the refresh-loop spawn, restore the
   21 wasm `auth_email` capability entry, revert the new tests.
2. Delete `crates/gpui-auth/src/{email,email.test,demo_backend,demo_backend.test}.rs`
   (+ `remote*.rs` if Phase 6 landed); revert `lib.rs` decls/re-exports.
3. Revert `controller.rs`: drop the 5 fields, 10 methods, `SigningUp`,
   `AwaitingChallenge`, `AccountCreated`; inline `do_refresh` back into
   `refresh` and `on_tokens` into `complete_callback`; remove the email-state
   clears from `sign_out`.
4. Revert `error.rs` variant additions and the crate manifest's argon2/totp-rs
   lines; regenerate and stage `Cargo.lock`.
5. If Phase 6 landed: `rm -r auth-dev-server/`, drop the members entry and the
   justfile recipe.
6. Remove the amendment paragraph from `docs/plans/21-auth-foundation.md`.
7. Run the Verification gates; `grep -rn "PasswordAuth\|AuthBackend\|DemoBackend\|EmailPasswordProvider\|enable_password\|refresh_if_needed" src/ crates/ tests/` returns nothing.

## Implementation Phases

Precondition for Phase 1: plan 21 is implemented and its gates pass
(`crates/gpui-auth/` exists; `cargo test -p gpui-auth --all-features` green).
`cargo check -p gpui-auth` first to confirm the baseline.

## Phase 1: Crate email core (traits + provider)

### Step 1.1: Error variants, types, traits

- Files: `crates/gpui-auth/src/error.rs`, `src/email.rs`, `src/email.test.rs`,
  `src/lib.rs`.
- Build: append the 11 `AuthError` variants exactly per the contract. In
  `email.rs`: `EMAIL_PROVIDER_ID`, `EmailAccountStatus`, `AuthenticatedSession`,
  `SignInOutcome`, `TotpChallenge`, `TotpEnrollment` (all `Clone + Debug`,
  statuses also `PartialEq, Eq`), and the `AuthBackend` + `PasswordAuth` traits
  with 21's cfg_attr async_trait pair (verbatim pattern from
  `21:Interface Contract` provider block). `lib.rs`: `pub mod email;` +
  re-exports. Doc comments on every pub item (what callers get, `now_unix`
  convention). No new deps yet.
- Tests: compile-level (`email.test.rs` declares a `TestBackend` implementing
  `AuthBackend` with a `Mutex<HashMap>` of users and fixed-token responses —
  it becomes the Phase 1.2 fixture; assert `EMAIL_PROVIDER_ID == "email"`).
- Done-check: `cargo test -p gpui-auth --all-features` passes.

### Step 1.2: `EmailPasswordProvider`

- Files: `src/email.rs`, `src/email.test.rs`.
- Build: struct `{ backend: Arc<dyn AuthBackend>, clock: Arc<dyn Fn() -> i64 +
  Send + Sync> }`, `new` per contract. `impl AuthProvider`: `id` →
  `EMAIL_PROVIDER_ID`, `display_name` → `"Email"`, `refresh_policy` →
  `RefreshPolicy::ReuseUntilDead`, `start_flow`/`complete_callback` →
  `Err(AuthError::NoBrowserFlow)`, `refresh(http, rt)` →
  `backend.refresh(http, rt, clock())`, `revoke` likewise. `impl PasswordAuth`:
  each method delegates with `clock()` for the time-taking backend calls.
  Futures own cloned `Arc`s (21's Step 6.1 move-into-async-block pattern).
- Tests (with `TestBackend`, `#[gpui::test]` where contexts are needed, plain
  async via the crate's test executor pattern from 21 Step 6.1):
  `start_flow` errors `NoBrowserFlow`; `refresh` forwards the token string the
  backend received; `sign_in` returns the scripted outcome; a fixed clock
  closure (`Arc<AtomicI64>` capture) makes the observed `now_unix` assertable.
- Done-check: `cargo test -p gpui-auth --all-features` passes;
  `cargo clippy -p gpui-auth --all-features -- -D warnings` clean.

## Phase 2: DemoBackend (argon2, sessions, tokens)

### Step 2.1: Hashing, validation, sign_up

- Files: `crates/gpui-auth/src/demo_backend.rs`, `demo_backend.test.rs`,
  `src/lib.rs` (cfg'd `pub mod demo_backend;` + re-export),
  `crates/gpui-auth/Cargo.toml`, `Cargo.lock`.
- Build: manifest not-wasm table gains `argon2 = { version = "0.5",
  default-features = false, features = ["alloc"] }`. Internals as pure sync
  fns (`sign_up_impl`, `verify_password`, `hash_password`) wrapped by the
  async trait methods, so tests are plain `#[test]`. `hash_password`:
  `getrandom::fill(&mut [u8; 16])` salt, `Argon2::new(Algorithm::default(),
  Version::default(), params)`, `hash_password_into(pwd, &salt, &mut [u8; 32])`
  (API cites in Research Digest), stored `"v1$" + b64url(salt) + "$" +
  b64url(hash)`; `verify_password` recomputes and compares with a manual
  constant-time fold (no new dep). Params: `new()` uses
  `Params::new(8 * 1024, 2, 1, None)`, `with_params` stores caller's (tests:
  `Params::new(8, 1, 1, None)` — the m_cost floor is 8, `params.rs:46`).
  Validation: email trimmed + lowercased, 3..=254 chars, exactly one `@`,
  non-empty local and domain, domain contains `.` (violations →
  `InvalidEmail`); password 8..=128 bytes (`WeakPassword`). `sign_up_impl`:
  validate → `users` write-lock, `EmailTaken` on existing → hash → insert
  `DemoUser { password, verified: false, totp: None, created_at: now }` →
  return status. `verify_totp`/`begin_totp_enrollment`/`confirm_totp_enrollment`
  return `Err(AuthError::TotpNotEnrolled)` this phase (Phase 4 implements).
- Tests: round-trip hash/verify (correct + wrong password); malformed emails
  and short passwords rejected; duplicate signup → `EmailTaken`; PHC-lite
  string never contains the password; two users with the same password get
  different salts; `with_params(8,1,1)` path used by every test for speed.
- Done-check: `cargo test -p gpui-auth --all-features` passes; `git diff
  Cargo.lock` shows argon2 (+ base64ct/blake2 etc.), stage it.

### Step 2.2: sign_in, refresh, revoke (sessions + deterministic tokens)

- Files: same.
- Build: `sign_in_impl`: user lookup → `verify_password` (fail →
  `InvalidCredentials`) → totp enrolled? → challenge (Phase 4; this phase
  always proceeds) → `create_session(email, now)`: `seq.fetch_add(1)`,
  `access = format!("demo-access-{seq}")`, `refresh = format!("demo-refresh-
  {seq}")`, `access_exp = now + ACCESS_TTL_SECS` (pub const 900), insert both
  maps, build `TokenSet { provider_id: EMAIL_PROVIDER_ID, access_token, 
  refresh_token: Some, id_token: None, token_type: "Bearer", expires_at:
  Some(access_exp), scope: None, account: Some(AccountInfo { id:
  Some(seq.to_string()), email: Some(email), display_name: Some(email) }) }`.
  `refresh_impl(refresh_token, now)`: session lookup (miss → `RefreshRejected`)
  → mint next access seq for the same session, keep the refresh token,
  `access_exp = now + 900`, update `by_access`, return the fresh `TokenSet`.
  `revoke_impl(token)`: `by_access` → remove session + index; unknown token →
  `Ok(())` (idempotent, mirrors 21's store semantics). Sessions keyed by
  refresh token; one session per sign-in.
- Tests: exact token strings for a fresh backend (`demo-access-1` etc.),
  asserting determinism across two backends driven identically; wrong password
  → `InvalidCredentials`; `needs_refresh` true past the margin using the
  returned `expires_at`; refresh keeps the refresh token, changes the access,
  advances `expires_at`; revoke then refresh → `RefreshRejected`; double
  revoke → `Ok(())`; 8 threads × 20 sign-ins → no panic, seq unique
  (`Send + Sync` proof).
- Done-check: `cargo test -p gpui-auth --all-features` passes; clippy clean.

## Phase 3: Controller email flows

### Step 3.1: Fields, states, events, readers

- Files: `crates/gpui-auth/src/controller.rs`, `controller.test.rs`, `src/lib.rs`
  (re-exports of `SignInOutcome` etc. from the crate root).
- Build: append `SigningUp` and `AwaitingChallenge { email: String, error:
  Option<String> }` to `AuthState` (derives already cover them) and
  `AccountCreated { email }` to `AuthEvent`. Add fields `password`, `clock`,
  `pending_challenge: Option<TotpChallenge>`, `email_status:
  Option<EmailAccountStatus>`, `pending_enrollment: Option<TotpEnrollment>`,
  `enrollment_error: Option<String>`, `warned_no_clock: bool`; `new`
  initializes the five optional slots to `None` and `warned_no_clock` to
  `false` (signature unchanged, 21's tests keep passing). Methods
  `enable_password`,
  `set_clock`, `email_status`, `totp_enrollment`, `totp_enrollment_error`
  per contract. Amend `sign_out` to clear the four email-state slots (before
  setting `SignedOut`). Import `crate::email::{PasswordAuth, ...}` — note for
  the implementer: the controller must compile on wasm, so it references only
  the cross-target `email` items, never `demo_backend`.
- Tests: `new` → `email_status() == None`, `totp_enrollment() == None`;
  `enable_password` twice replaces; sign_out clears a seeded
  `AwaitingChallenge` + `email_status` (via a `#[cfg(test)]` setter or by
  driving a real flow from Step 3.2 onward — order the asserts after 3.2 if a
  setter is unwanted; do NOT add a test-only setter, drive the flow).
- Done-check: `cargo test -p gpui-auth --all-features` passes (21's suite
  still green proves the amendments are additive).

### Step 3.2: sign_up_with_password, sign_in_with_password, on_tokens

- Files: same.
- Build: extract 21's post-exchange block of `complete_callback` (provider_id
  check → `store.save` → `SignedIn` label chain → notify → `FlowCompleted`)
  into `fn on_tokens(&mut self, tokens: TokenSet, cx)` and call it from both
  paths. `sign_up_with_password`: `SignedIn` → warn log + return;
  `password` None → `Error(PasswordAuthUnavailable.to_string())` + notify;
  else state `SigningUp`, notify, `cx.spawn` with the moved
  `Arc<dyn PasswordAuth>` + `AuthIo` clone + owned email/password, drive
  `sign_up`, then `this.update`: `Ok(status)` → `email_status = Some`,
  state `SignedOut`, notify, `emit(AccountCreated { email })`; `Err(e)` →
  `Error(e.to_string())`, notify, `emit(FlowFailed)`. `sign_in_with_password`:
  same spawn shape with `SigningIn`; `Ok(Authenticated(session))` →
  `email_status = Some(session.status)`, `on_tokens(session.tokens, cx)`;
  `Ok(TotpRequired(challenge))` → `pending_challenge = Some`,
  `AwaitingChallenge { email: challenge.email.clone(), error: None }`, notify;
  `Err(e)` → Error path. `cancel_challenge`: `pending_challenge.take()`,
  state `SignedOut`, notify. Weak-handle update errors end the task silently
  (21 pattern). Tracing target `"gpui_auth::controller"`; log method name +
  state, never the password or token strings.
- Tests (`#[gpui::test]`, `TestBackend` from Step 1.1 + `InMemoryStore` +
  `AuthIo { http: reqwest::Client::new(), runtime: None }`): happy sign-in →
  `SignedIn` with the email label, store row saved, `FlowCompleted` captured;
  sign-up happy → `AccountCreated` captured, state back to `SignedOut`,
  `email_status` set; duplicate sign-up → `Error` state, `FlowFailed`;
  invalid credentials → `Error`; sign-up while `SignedIn` is a no-op (state
  unchanged); sign-in replaces an existing session (second `on_tokens`
  overwrites the store row — assert single row, new label); `TotpRequired`
  outcome → `AwaitingChallenge` (scripted via `TestBackend`); sign_out clears
  everything and deletes the row.
- Done-check: `cargo test -p gpui-auth --all-features` passes; clippy clean.

### Step 3.3: set_clock + refresh_if_needed (do_refresh refactor)

- Files: same.
- Build: extract 21's `refresh` body into `fn do_refresh(&mut self, cx: &mut
  Context<Self>)` (load row; `NoRefreshToken` when absent; policy `None` →
  error; drive `provider.refresh`; merge via `refresh_policy()`; save; notify;
  errors → `Error` state) and call it from `refresh()`.
  `refresh_if_needed(margin_secs, cx)`: `clock` None → warn exactly once via
  the `warned_no_clock` field (set it, never reset it; a per-call warn would
  fire every 60 s forever on a clock-less install, e.g. a controller nobody
  wired) + return; `store.load(provider_id)` → `None` → return;
  `tokens.needs_refresh((clock)(), margin_secs)` false → return; else
  `do_refresh(cx)`.
- Tests: fixed clock at `iat`; `refresh_if_needed(120)` before the margin →
  store row unchanged (access token same); advance the clock past
  `expires_at - 120` → access token changes, refresh token unchanged
  (`ReuseUntilDead` merge, 21 Step 6.2 behavior); with the clock unset →
  no-op; while signed out (no row) → no-op; dead refresh token (swap the row
  for one the `TestBackend` rejects) → `Error` state.
- Done-check: `cargo test -p gpui-auth --all-features` passes.

## Phase 4: TOTP end to end

### Step 4.1: DemoBackend TOTP (totp-rs)

- Files: `crates/gpui-auth/src/demo_backend.rs`, `demo_backend.test.rs`,
  `crates/gpui-auth/Cargo.toml`, `Cargo.lock`.
- Build: manifest not-wasm table gains `totp-rs = { version = "6.0",
  default-features = false, features = ["alloc", "otpauth", "zeroize"] }`.
  `DemoUser.totp: Option<Totp>`. `begin_totp_enrollment`: user must exist
  (else `InvalidCredentials`); already enrolled → `Err(AuthError::
  InvalidChallenge)` (the enrollment secret is shown once, never re-shown;
  re-enrollment requires a backend reset); the signed-in check is the
  controller's job, the backend only knows the email. Fresh enrollment:
  `getrandom::fill(&mut [u8; 20])`, `Totp` via `Builder::new().with_secret(
  Secret::from(bytes)).with_account_name(email).with_issuer(Some(
  "gpui-starter")).build()`, store in a `pending_totp: Mutex<HashMap<email,
  Totp>>` (NOT yet on the user), return `TotpEnrollment { email,
  secret_base32: totp.secret().to_base32(), otpauth_url: totp.to_url()? }`
  (map the `TotpError` to `AuthError::InvalidTokenResponse`). 
  `confirm_totp_enrollment(email, code, now)`: pending entry required
  (`InvalidChallenge`); `totp.check(code, now)` → `None` → `TotpFailed`;
  `Some(_)` → move the `Totp` onto the user, drop pending, `Ok(())`.
  `sign_in_impl`: when `user.totp.is_some()`, mint
  `challenge = format!("demo-ch-{seq}")`, store `{ email, expires_at: now +
  CHALLENGE_TTL_SECS (300) }`, return `TotpRequired(TotpChallenge { id,
  email })` (no session yet). `verify_totp(challenge, code, now)`:
  challenge lookup (`InvalidChallenge` on miss/expiry — expiry check
  `now >= expires_at`), `user.totp.check(code, now)` (`TotpFailed`),
  remove the challenge, `create_session` → `AuthenticatedSession`. Wrap in
  the same sync-core pattern.
- Tests: enrollment round-trip with a code generated by the returned secret
  (`Secret::try_from_base32` + `Builder` + `generate(now)` — the API cites in
  Research Digest); wrong code → `TotpFailed`, pending kept, retry with the
  right code succeeds; second enrollment while enrolled → `InvalidChallenge`;
  sign-in after enrollment → `TotpRequired`; verify with an expired challenge
  → `InvalidChallenge`; wrong-code verify keeps the challenge (retryable);
  codes are single-use per RFC 6238 note: after successful verify the
  challenge is consumed (second verify → `InvalidChallenge`).
- Done-check: `cargo test -p gpui-auth --all-features` passes; `git diff
  Cargo.lock` shows totp-rs (+ sha1 0.11, sha2 0.11, hmac 0.13 — duplicate
  sha2 major expected, see Risks), stage it.

### Step 4.2: Controller TOTP methods

- Files: `crates/gpui-auth/src/controller.rs`, `controller.test.rs`.
- Build: `submit_totp(code, cx)`: `pending_challenge` required (else
  `Error(NoPendingChallenge)` + notify); spawn `password.submit_totp(http,
  &challenge, &code)`; `Ok(session)` → clear challenge, `email_status =
  Some`, `on_tokens`; `Err(AuthError::TotpFailed | InvalidChallenge)` →
  keep the challenge (drop it on `InvalidChallenge` — expired: back to
  `SignedOut`), set `AwaitingChallenge { error: Some(e.to_string()) }`,
  notify; other errors → Error path + drop the challenge.
  `enroll_totp(cx)`: requires `SignedIn` + `email_status`/`pending_challenge`
  email (the account's email; if `email_status` is None — mock-restored
  session — read the email from `store.load`'s `TokenSet.account`; absent →
  warn + return); spawn `begin_totp_enrollment`; `Ok(enrollment)` →
  `pending_enrollment = Some`, clear `enrollment_error`, notify; `Err` →
  `enrollment_error = Some`, notify. `confirm_totp_enrollment(code, cx)`:
  pending enrollment required; spawn `confirm_totp_enrollment`; `Ok(())` →
  drop the enrollment, notify; `Err` → keep it, `enrollment_error = Some`,
  notify.
- Tests: scripted `TestBackend` flows: sign-in → `TotpRequired` → wrong code
  keeps `AwaitingChallenge` with `error: Some` → right code → `SignedIn` +
  store row; `cancel_challenge` mid-flow → `SignedOut`; `submit_totp` with no
  challenge → `Error(NoPendingChallenge)`; enrollment requires `SignedIn`;
  confirm failure keeps the enrollment and sets `enrollment_error`, confirm
  success clears it.
- Done-check: `cargo test -p gpui-auth --all-features` passes; clippy clean.

## Phase 5: App adapter

### Step 5.1: Provider construction, capability, install_with

- Files: `src/services/auth.rs`, `crates/gpui-auth/README.md`.
- Build: extend `install_with` to
  `(store, io, provider: Arc<dyn AuthProvider>, password: Option<Arc<dyn
  PasswordAuth>>, clock: Arc<dyn Fn() -> i64 + Send + Sync>, cx)`; all of 21's
  wiring (entity, restore, observer, browser-open subscriber, `AuthHandle`)
  is unchanged and now uses the parameters instead of hardcoding
  `MockProvider`. Inside `install_with`, after entity creation and before
  `restore`: `entity.update(cx, |c, _| { if let Some(p) = &password {
  c.enable_password(p.clone()); } c.set_clock(clock); })` — both values are
  parameters, so this compiles in every caller. Native `install` arm:
  `let clock: Arc<dyn Fn() -> i64 + Send + Sync> = Arc::new(|| chrono::Utc::now().timestamp());`
  (`src/app/lifecycle.rs:218` precedent), `let provider =
  Arc::new(EmailPasswordProvider::new(Arc::new(DemoBackend::new()),
  clock.clone()));` then `install_with(store, io, provider.clone() as Arc<dyn
  AuthProvider>, Some(provider as Arc<dyn PasswordAuth>), clock, cx)`. Wasm
  arm: `install_with(store, io, Arc::new(MockProvider::new()), None, clock,
  cx)` — same chrono closure (wasmbind keeps it wasm-correct, `Cargo.toml:371`),
  so the refresh loop never runs clock-less. Capability: native keeps
  `auth_email` = `supported_enabled()`; wasm arm becomes the five-field struct
  literal (capabilities.rs:6-13): `supported: false, enabled: true, degraded:
  true, reason: Some("password auth needs native crypto crates; the scripted
  mock provider remains"), last_error: None`. README: email flow section
  (provider ids, demo caveats: ephemeral users, restart → refresh error,
  TOTP secrets in process memory).
- Tests: none yet (5.3).
- Done-check: `cargo clippy -p gpui-starter --all-features -- -D warnings`
  clean; `just wasm-check` passes (email module + provider compile on wasm;
  DemoBackend compiled out).

### Step 5.2: Refresh loop

- Files: `src/services/auth.rs`.
- Build: in `install_with`, after the subscriptions are stored:
  `let weak = entity.downgrade(); cx.spawn(async move |cx: &mut AsyncApp| {
  loop { cx.background_executor().timer(std::time::Duration::from_secs(60))
  .await; if weak.update(cx, |c, cx| c.refresh_if_needed(120, cx)).is_err()
  { break; } } }).detach();` (API cites: `GPUI/app/async_context.rs:159`,
  `GPUI/executor.rs:183`, `GPUI/app/entity_map.rs:803-813`; timer-loop
  precedent `src/gallery/sections/tables/data_table.rs:803`). Wasm keeps the
  loop: the mock's `expires_at: None` makes every tick a no-op, and
  `refresh_if_needed` early-returns without a store row. Interval 60 s,
  margin 120 s, demo TTL 900 s: refresh fires 60-120 s before expiry.
- Tests: none (crate tests cover the decision logic; the loop is three lines
  of plumbing).
- Done-check: `cargo check -p gpui-starter` passes; app boots without a
  refresh-related panic in the log.

### Step 5.3: Adapter headless e2e suites

- Files: `src/services/auth.test.rs`, root `Cargo.toml`, `Cargo.lock`.
- Build: first the manifest edit — root `Cargo.toml` gains
  `[target.'cfg(not(target_family = "wasm"))'.dev-dependencies]` with
  `argon2` (0.5.3, matching seam row 5) and `totp-rs` (crate-identical
  feature set); regenerate `Cargo.lock` and stage both (qa_migration
  discipline). Without this the suite below does not compile: the app
  crate, not gpui-auth, is what `cargo test -p gpui-starter` builds.
  Then extend 21's `test_install` into `test_install_email(cx)`:
  `InMemoryStore` (kept as an `Arc` for direct row asserts), `AuthIo { http:
  reqwest::Client::new(), runtime: None }`, `DemoBackend::with_params(
  Params::new(8, 1, 1, None))` (fast argon2), and a fixed clock over an
  `Arc<AtomicI64>` the test advances, passed through `install_with`'s clock
  parameter (so `set_clock` runs through the same path production uses).
  Suites: (1) full roundtrip — sign_up →
  `AccountCreated`, session snapshot `SignedOut`; sign_in → `SessionState::
  SignedIn` (the 21 observer ran), store row exists with `provider_id
  "email"`; advance clock past expiry-minus-margin → `refresh_if_needed(120)`
  → row's access token changed, refresh token unchanged; sign_out → row
  `None`, session `SignedOut`. (2) bad credentials → controller `Error`,
  session `Error`, no store row. (3) duplicate sign-up → error surfaced.
  (4) TOTP: sign_in → enroll_totp → enrollment has 32-char-ish base32 secret
  and an `otpauth://` URL; generate the code in-test via totp-rs from the
  returned secret; confirm; sign_out; sign_in → `AwaitingChallenge`; wrong
  code keeps the challenge with error; right code → `SignedIn`. (5)
  `handle_callback_link` still routes and returns false for non-auth links
  (21's test, re-run to prove no regression). Keep 21's mock-provider tests
  (wasm parity + browser-flow coverage) intact alongside.
- Done-check: `cargo test -p gpui-starter --lib services::auth` passes.

## Phase 6: Optional dev server + RemoteBackend

Explicitly optional: skip the whole phase and everything above stands
(`GPUI_AUTH_DEV_SERVER` unset → DemoBackend). Mirrors plan 08's collab-server
decision (separate member, axum 0.8, justfile recipe, loopback test).

### Step 6.1: auth-dev-server crate

- Files: `auth-dev-server/Cargo.toml`, `auth-dev-server/src/main.rs`, root
  `Cargo.toml` (members append, grep-first), `Cargo.lock`.
- Build: manifest `{ name = "auth-dev-server", version = "0.1.0", edition =
  "2024", publish = false }`; deps `axum = "0.8"`, `tokio = { version = "1",
  features = ["macros", "rt-multi-thread", "net"] }`, `serde`, `serde_json`,
  `argon2 = { version = "0.5", default-features = false, features = ["alloc"] }`,
  `tracing` + `tracing-subscriber` (fmt, env-filter). Server: same in-process
  model as `DemoBackend` (users HashMap + sessions + argon2, no TOTP — the
  dev server covers the core endpoints; TOTP stays a demo-backend feature,
  documented). JSON endpoints (error body `{"error": "<code>"}`):
  `POST /auth/sign-up {email, password}` → 200 `{email, verified: false}` |
  409 `email_taken` | 400 `invalid_email`/`weak_password`; `POST /auth/sign-in`
  → 200 `{access_token, refresh_token, expires_at, token_type, account: {id,
  email, display_name}}` | 401 `invalid_credentials`; `POST /auth/refresh
  {refresh_token}` → 200 tokens | 401 `refresh_rejected`; `POST /auth/revoke
  {token}` → 204; `GET /healthz` → `"ok"`. Bind `127.0.0.1:{AUTH_DEV_PORT or
  3999}` (loopback only; passwords cross plain HTTP by design — dev tool,
  stated in the module header). Wiring style per 08 Research Digest
  (`Router::new().route(...)` + `axum::serve(TcpListener, app)`).
- Tests: none here (6.2 loopback covers it).
- Done-check: `cargo run -p auth-dev-server` boots; `curl -s
  localhost:3999/healthz` prints `ok` (manual, once).

### Step 6.2: RemoteBackend + env switch

- Files: `crates/gpui-auth/src/remote.rs`, `remote.test.rs`, `src/lib.rs`,
  `crates/gpui-auth/Cargo.toml` (nothing new — reqwest already a dep),
  `src/services/auth.rs`.
- Build: `remote.rs` cfg native: `pub struct RemoteBackend { base: url::Url }`,
  `new(base: impl Into<String>) -> Result<Self, AuthError>` (parse →
  `InvalidRedirect`-style mapping: reuse `AuthError::InvalidRedirect`? No —
  add nothing; map to `AuthError::Network` with the parse message).
  `impl AuthBackend` (id `"dev-server"`): each method `http.post(base.join(
  "/auth/...")).json(&body).send().await` (reqwest error → `AuthError::
  Network`), status 2xx → serde-decode (decode error →
  `InvalidTokenResponse`), error status → map the `error` code to the
  matching `AuthError` variant (`email_taken` → `EmailTaken`,
  `invalid_credentials` → `InvalidCredentials`, `refresh_rejected` →
  `RefreshRejected`, `invalid_email`/`weak_password` likewise, unknown →
  `Network` with the body text). TOTP methods → `Err(TotpNotEnrolled)`
  (server has no TOTP; documented). `src/services/auth.rs` native arm gains
  the switch (no `?`: `install` returns `()`, so failures fall back, never
  propagate): `let backend: Arc<dyn AuthBackend> = match
  std::env::var("GPUI_AUTH_DEV_SERVER_URL") { Ok(u) => match
  RemoteBackend::new(u) { Ok(b) => Arc::new(b), Err(e) => { tracing::warn!
  (target: "gpui_starter::auth", error = %e, "invalid GPUI_AUTH_DEV_SERVER_URL;
  falling back to DemoBackend"); Arc::new(DemoBackend::new()) } }, Err(_) =>
  Arc::new(DemoBackend::new()) };`.
- Tests: `remote.test.rs` maps fixture JSON bodies → outcomes (pure fn tests:
  factor the response mapping into `fn map_error(code: &str) -> AuthError`
  and test it; no HTTP server needed in the crate). Loopback integration
  test in `auth-dev-server` (08 Step 2.3 pattern): bind `127.0.0.1:0`,
  sign-up + sign-in + refresh + revoke through a real reqwest client against
  the served router (dev-dep on the app's reqwest is unnecessary — the
  server crate declares its own dev-dep `reqwest = { version = "0.12",
  default-features = false, features = ["json"] }`).
- Done-check: `cargo test -p gpui-auth -p auth-dev-server --all-features`
  passes; `cargo check -p gpui-starter` still clean.

### Step 6.3: justfile recipe

- Files: `justfile`.
- Build: recipe `# Run the local auth backend the email provider can target
  (GPUI_AUTH_DEV_SERVER_URL=http://127.0.0.1:3999).` / `auth-dev-server:` /
  `    cargo run -p auth-dev-server` (grep `auth-dev-server` first; 08's
  `collab-server` recipe is the format precedent).
- Tests: none.
- Done-check: `just --list` shows `auth-dev-server`.

## Phase 7: Gates + manual QA

### Step 7.1: Full gates + manual QA

- Files: none (verification only).
- Build: run everything in Verification.
- Tests: as listed there.
- Done-check: all gates green; manual checklist executed or explicitly waived
  with notes.

## Platform & Wasm Gating

- Native-only deps, `crates/gpui-auth/Cargo.toml`
  `[target.'cfg(not(target_family = "wasm"))'.dependencies]` (alongside 21's
  tokio entry):

  | dep | line |
  | --- | --- |
  | `argon2 = { version = "0.5", default-features = false, features = ["alloc"] }` | Phase 2.1 |
  | `totp-rs = { version = "6.0", default-features = false, features = ["alloc", "otpauth", "zeroize"] }` | Phase 4.1 |

- Same two crates, same target gate, as ROOT `[target.'cfg(not(target_family
  = "wasm"))'.dev-dependencies]` (seam row 5 / Step 5.3) — the app's own
  test suite constructs `argon2::Params` and generates TOTP codes, so the
  app crate needs test-only access; dev-deps never reach production or wasm
  builds.

- `demo_backend.rs` and `remote.rs` are `#[cfg(not(target_family = "wasm"))]`
  (declared via cfg'd `mod` lines in `lib.rs`). `email.rs`, `PasswordAuth`,
  the outcome types, and `EmailPasswordProvider` compile on every target:
  the controller's password methods exist on wasm and fail with
  `PasswordAuthUnavailable` (provider never registered there).
- Wasm degradation: the adapter keeps 21's `MockProvider` + `InMemoryStore`
  (tokens vanish on reload; 21's recorded posture, unchanged). The
  `auth_email` capability flips to the unsupported/degraded struct literal
  with the reason string (Phase 5.1); the scripted mock sign-in from 21 still
  works, so the surface degrades rather than disappearing (seams §6
  convention). `auth_social` (21) is untouched.
- Clocks: the crate stays clock-free; the only time source is the injected
  closure built app-side from `chrono::Utc::now()` (`wasmbind` covers wasm,
  `Cargo.toml:371`), so no `std::time` hazard enters the crate (seams §6).
- `just wasm-check` (`cargo +nightly check --target wasm32-unknown-unknown -p
  gpui-starter --lib`, `justfile:24-25`) transitively checks the crate; the
  email module must compile there without argon2/totp-rs (Phase 5.1
  done-check).
- Optional Phase 6 member is native-only by construction (axum/tokio server);
  it never enters the app's wasm graph because the app does not depend on it.

## i18n

No catalog keys. The crate and adapter emit no user-facing copy: `AuthState` /
`AuthError` strings are developer-facing (24 wraps them; 21 Design Compliance
rule). The one new capability reason string is an English literal inside the
adapter, matching 21's precedent (`secure_storage.rs:124`, 21's `auth_social`
wasm reason). Plan 24 owns the `auth_*` catalog keys and must keep both locale
files in lockstep (`tests/qa_localization.rs:188-198`).

| key | en | zh-CN |
| --- | --- | --- |
| — (none) | — | — |

## Design Compliance

Brief (non-UI plan; full weight lands in 24). No layout, density, token,
motion, or component decisions are introduced here. What this plan owes 24:
every flow state is observable (`SigningUp`, `SigningIn`, `AwaitingChallenge
{ email, error }`, `SignedIn`, `Error`, plus `email_status`,
`totp_enrollment`, `totp_enrollment_error`), outcomes arrive as data not
strings, and error text is marked developer-facing so 24 localizes instead of
displaying raw (coding-guides interface-language discipline carried from 21).
Interaction-state completeness (loading/empty/error per the design guides'
form rules) is deferred to 24's forms; the state machine above is the
checklist 24 renders against.

## Verification

Repo gates, in order (AGENTS.md):

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-features
cargo test --workspace --all-features
just wasm-check
```

`--workspace` includes `gpui-auth` and, if Phase 6 landed, `auth-dev-server`.
Stage the regenerated `Cargo.lock` with the Phase 2.1, 4.1, and 6.1 manifest
edits. CI's `cargo test -p gpui-starter --lib` lane runs the adapter suites;
crate suites run in the local gate only (seams §8).

Feature-specific automated coverage:

- `cargo test -p gpui-auth --all-features`: provider delegation + NoBrowserFlow,
  DemoBackend (hash round-trip, validation, EmailTaken, exact deterministic
  token strings, refresh keeps refresh/replaces access, revoke + RefreshRejected,
  concurrency, TOTP enrollment/verify/expiry/single-use), controller email
  flows (sign-up, sign-in, challenge keep-on-wrong-code, refresh_if_needed
  margins, clock-unset no-op, sign_out clears email state), and 21's suites
  still green (additivity proof).
- `cargo test -p gpui-starter --lib services::auth`: headless e2e roundtrip
  against `DemoBackend::with_params` + `InMemoryStore` + fixed clock
  (Phase 5.3 list), including the `SessionState` mapping and TOTP path.
- `cargo test -p gpui-auth` (no features): mock compiles out (21 check).

Manual QA (native, Linux dev box; the feature has no UI until 24, so QA is
headless-first):

1. `cargo run`; Diagnostics shows `auth_email` supported + enabled
   (unchanged from 21 natively) and `auth_social` unchanged.
2. Run `cargo test -p gpui-starter --lib services::auth -- --nocapture` once
   and confirm the log lines during the roundtrip contain provider ids and
   states only: `grep -E "gpui_auth|gpui_starter::auth"` output must not
   contain `demo-access-`, `demo-refresh-`, `v1$`, or any password text.
3. Restart check: sign in (test 1's path or a 24 preview build), quit, run
   again — the app boots, the stale token surfaces `RefreshRejected` on the
   first refresh window (up to 60 s), and a fresh sign-in recovers; no panic.
4. `just wasm-check` green.
5. Phase 6 only: `just auth-dev-server` in one terminal; with
   `GPUI_AUTH_DEV_SERVER_URL=http://127.0.0.1:3999 cargo test -p
   auth-dev-server` and the loopback test green.

Acceptance criteria: all four gates pass; the three automated suites pass;
manual items 1-4 confirmed (5 if Phase 6 taken). `grep -rn "demo-access-\|
demo-refresh-" src/ --include="*.rs" | grep -v test` returns nothing (app
code never touches the demo token literals; the minting `format!` calls live
in the crate's `demo_backend.rs` by design — same scoping 21 uses for
`mock-access-` — and the crate's log lines never print them, which manual QA
item 2 checks against real output); `git diff src/state/
config_store.rs` is empty (no config schema change); `grep -rn "PasswordAuth\|
AuthBackend" src/ | grep -v "services/auth"` returns nothing (single app-side
seam).

## Risks & Open Questions

Ranked risks:

1. **totp-rs 6.0.0 is docs.rs-verified but never compiled here** (not in the
   registry cache). The Builder/Totp/Secret surface and the feature names
   `alloc`/`otpauth`/`zeroize` come from docs.rs pages fetched 2026-10-12;
   the feature table is now known (docs.rs `/crate/totp-rs/6.0.0/features`,
   fetched 2026-10-12: default = `migration` + `std`; `std` enables rand's
   `thread_rng`, so the defaults DO pull rand), so the minimal set
   `["alloc", "otpauth", "zeroize"]` is chosen precisely to keep rand out.
   Mitigation: Step 4.1's `cargo check` is the first evidence point; if the
   minimal set fails, add the specific missing feature by name — enabling
   defaults is not a fallback because `std` re-adds rand; the hard fallback
   is a hand-rolled RFC 6238 verify (~30 lines over `hmac` +
   `sha1`, both pure Rust, replacing the dep entirely).
2. **Contract amendments touch 21's line-stable surface.** Additive only
   (variants appended, new methods, new fields private to the controller);
   the additivity proof is 21's own suite passing unmodified after Phase 3.
   23/24 must read `22:Interface Contract` before building; the pointer note
   appended to 21's doc is the tripwire.
3. **argon2 latency in dev profiles.** 8 MiB / 2 passes is a compromise; if
   debug-profile sign-up feels slow on low-end hardware, drop `new()` to
   4096 KiB (one constant) — the hash format and tests are cost-agnostic.
   Tests already use the 8 KiB floor.
4. **Demo ephemerality vs persisted tokens.** Users live in a HashMap (spec);
   an app restart keeps the keyring `TokenSet` but loses the sessions, so
   `restore` shows `SignedIn` until the first refresh surfaces
   `Error(RefreshRejected)`. Documented in the README and manual QA 3; the
   alternative (validate on restore) adds a backend round-trip to the boot
   path and was rejected.
5. **UI-thread keyring reads every 60 s.** `refresh_if_needed` loads the
   store row inside the entity update, the accepted 21 risk #4 class; a
   keyutils read is cheap and the loop no-ops without a row. If profiling
   ever objects, move the load into the spawned task and hop back via the
   weak entity (pattern already in place).
6. **Duplicate sha1/sha2 majors.** totp-rs pulls sha2 0.11 + sha1 0.11
   alongside the crate's sha2 0.10 (survey A2); cargo permits it and
   `qa_migration` guards only the gpui family. Cost is lock size and compile
   time, not correctness.
7. **Workspace members / justfile collisions.** 08 and 21 both add `members`
   lines and 08 adds a recipe; Phase 6 appends and greps first (INDEX §5
   discipline). Whoever lands last merges the list.
8. **Phase 6 plaintext passwords on loopback HTTP.** Dev tool on
   127.0.0.1 by design; module header states it; not a production surface.

Open questions (each with what was tried):

- **totp-rs 6.0 compile behavior** — the feature table is now verified
  (docs.rs `/crate/totp-rs/6.0.0/features`, fetched this session after the
  crates.io `/features` page rendered nothing); what remains unproven is only
  that the minimal set `["alloc", "otpauth", "zeroize"]` compiles and that
  the documented signatures match the published source. Resolution deferred
  to the Step 4.1 compile; fallbacks in Risk 1.
- **argon2 0.6.0 upgrade** — survey A2 dates it 2026-08-27; not in the local
  registry cache (`ls ~/.cargo/registry/src/*/ | grep argon2` → 0.5.3 only),
  so its API could not be line-verified. 0.5.3 pinned deliberately (every
  load-bearing call cited from disk). Upgrading later is a manifest bump
  plus re-reading `Params`/`hash_password_into` signatures.
- **Deep-link auth-code logging (21 flagged for 22)** — verified the two
  verbatim log lines (`src/platform/process/single_instance.rs:296,415`).
  Decision: 22 introduces no browser flow, so no authorization codes transit
  deep links in this plan; the redact-or-accept call belongs to plan 23,
  which is the first plan to put codes in URLs. 23 now exists on disk —
  the post-review audit found it did NOT carry this item (its citations
  name `single_instance.rs:415-416` only as a delivery path), so the
  2026-10-12 fix round added the redaction seam touch to 23 directly
  (23 Seam Touches: sanitize the query of both log lines). Tripwire closed;
  re-check the two lines when either plan lands.
- **QR rendering for TOTP enrollment** — enrollment returns the otpauth URL
  and base32 secret; the `qr` feature (pulls `qrcodegen-image`) is not
  enabled. Whether 24 renders a QR or manual-entry text is 24's call; the
  enrollment data supports both.
- **`Secret` serialization of TOTP secrets** — demo secrets live only in
   process memory, so totp-rs's `serde` feature is unused; if a future
   backend persists secrets, decide storage then (the OS `TokenStore` is the
   only sanctioned secret store per 21).
