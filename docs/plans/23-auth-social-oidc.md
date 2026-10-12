# 23 Social Login (OIDC/OAuth2 + PKCE) Plan

## Goal

Sign in with Google, GitHub, or any OIDC issuer from the boilerplate app: the
user picks a provider, the system browser opens through the existing
`desktop_actions::open_url` wrapper, and the redirect is captured either at
`gpui-starter://auth/callback` (the existing deep-link path, single-instance
forwarding included) or on an ephemeral `127.0.0.1` loopback listener. PKCE
S256 and a random `state` are enforced on every browser flow; tokens land in
plan 21's `TokenStore` and sign-in state reaches `SessionState`. Client
ids/secrets come from a user-owned config file that is never committed, and
diagnostics shows configured-not-connected states per provider. The wasm build
carries an explicit degraded capability entry, not a silent gap.

## Dependencies

Plan `21-auth-foundation.md` — must be implemented and green before this plan
starts (`crates/gpui-auth/` and `src/services/auth.rs` do not exist on disk
yet; `ls src/services/` shows no `auth.rs`, and `Cargo.toml:1-2` has an empty
`[workspace]` table with no `members` — verified this session). Interface
Contract items consumed (line refs into `docs/plans/21-auth-foundation.md`):

- `AuthController::new(provider, store, io, cx)`, `start_sign_in`,
  `complete_callback(raw_redirect, cx)`, `cancel`, `refresh`, `restore`,
  `sign_out`, `state`, `provider_id` (21.md:485-509) — one controller per
  social provider.
- `AuthProvider` trait with the native/wasm `async_trait` Send split
  (21.md:450-472, digest 21.md:115-128) — implemented for
  `OidcProvider` (Google + generic issuer) and `GitHubProvider`.
- `FlowLaunch { authorize_url, state, pkce, redirect_uri }` (21.md:429-434),
  `RedirectParams::from_url` scheme-agnostic parsing (21.md:435-445),
  `AuthIo` (21.md:409-427), `TokenSet`/`AccountInfo`/`SecretString`/
  `RefreshPolicy` (21.md:344-383), `PkcePair::generate`/`random_state`
  (21.md:386-396), `AuthError` (21.md:320-341).
- App adapter surface: `KeyringStore`, `install`, `controller`,
  `handle_callback_link` (21.md:527-541) — `handle_callback_link` gains one
  guard line (Seam Touches row 6), the `auth_social` capability setting moves
  here (row 7).

Repo services consumed (all read this session, cited in Research Digest):
`src/services/session.rs` (`SessionState` 3-12, `set_state` 31-36),
`src/services/secure_storage.rs` (via plan 21's `KeyringStore`),
`src/services/desktop_actions.rs` (`open_url` 147-191, http/https guard
150-153, wasm `Unavailable`), `src/services/tokio_runtime.rs`
(`runtime_and_client` 80-83), `src/runtime/events.rs` (`emit` 48-57,
`AppEventKind::DeepLinkReceived` 12), deep-link delivery
(`src/platform/process/single_instance.rs`: `SCHEME` 50,
`initial_deep_link` 138-140, IPC forward at 415-416; `src/web.rs:40`;
`src/platform/deep_link_registration.rs` whole-scheme HKCU registration
20-53, called at boot `src/app/init.rs:78-89`),
`src/runtime/capabilities.rs` (`CapabilityStatus` five pub fields 6-13,
`set` 64-79), `src/state/config_store.rs` (`AppPaths` config_dir via
`paths(cx)` 254-270).

Surveys trusted over memory and re-verified where load-bearing:
`_survey/seams.md` (integration canon; 0.7.1/0.3.8 citations kept as
historical record per INDEX §4 pin-update note — nothing kit-API-load-bearing
in this plan), `_survey/auth-ecosystem.md` (crate picks, RFC 8252/7636/9700
facts, Google/GitHub provider tables, wasm posture),
`_survey/auth-zed.md` (loopback callback-server pattern, refresh-shape
precedent).

## Research Digest

Path keys: `O2` = `/tmp/oauth2-5.0.0` and `OIDC` =
`/tmp/openidconnect-4.0.1` (tarballs downloaded from static.crates.io this
session — neither crate is in the local registry, `ls ~/.cargo/registry/src/*/
| grep -iE '^(oauth2|openidconnect)'` → no hits — so sources were read from
the extracted tarballs, plan 08's method), `GPUI` =
`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-pre-0.3.9/src`
(present, verified with `ls ~/.cargo/registry/src/*/ | grep -E
'gpui-pre-0.3.9|gpui-kit-0.7.3'`).

Crate picks (survey A2 rows, versions from the crates.io API 2026-10-12; API
surface verified in the tarballs above):

| Dep | Version | Why |
| --- | --- | --- |
| `oauth2` | 5.0.0, `default-features = false`, `features = ["reqwest", "rustls-tls"]` | GitHub flow (no OIDC discovery there — `GET https://github.com/.well-known/openid-configuration` → 404, surveyed live 2026-10-12). Default features are exactly reqwest 0.12 + rustls-tls (`O2/Cargo.toml:123-126`), matching the app's reqwest line (`Cargo.toml:187-193`); one reqwest copy in the lock |
| `openidconnect` | 4.0.1, same feature set | Google + generic issuer: discovery, id_token validation, userinfo. Forwards its reqwest/rustls features onto oauth2 ^5.0.0 (`OIDC/Cargo.toml [dependencies.oauth2]`, `[features]` 147-161) |
| `chrono` | workspace (already direct, `Cargo.toml:176`) | `expires_at` = `Utc::now().timestamp() + expires_in` (native-only module; no wasm clock hazard) |

Both go in the app's `[target.'cfg(not(target_family = "wasm"))'.dependencies]`
table (`Cargo.toml:310`): oauth2 5.0's wasm randomness rides getrandom 0.2 +
rand 0.8 (`O2/Cargo.toml [dependencies.rand]` version 0.8; wasm32 getrandom
`js` feature at `O2/Cargo.toml:137-139`), and openidconnect adds rsa/p256/p384
(`OIDC/Cargo.toml [dependencies.*]`) — untested on wasm32-unknown-unknown and
unneeded there; the native-only table sidesteps the whole question. Cost
accepted: `thiserror` 1.x (both crates) coexists with the app's 2.0.18
(`O2/Cargo.toml [dependencies.thiserror]` version 1.0) — majors coexist in one
lock without conflict.

Verified API surface used by this plan (tarball path:line):

- `oauth2::basic::BasicClient` type alias (`O2/src/basic.rs:11`);
  `Client::new(ClientId)` (`O2/src/client.rs:190`) plus
  `.set_auth_uri(AuthUrl)` / `.set_token_uri(TokenUrl)` /
  `.set_client_secret(ClientSecret)` / `.set_redirect_uri(RedirectUrl)`
  builders (`O2/src/client.rs:261,545,330,468`).
- oauth2's `Client::authorize_url(state_fn) -> AuthorizationRequest` where
  `state_fn: FnOnce() -> CsrfToken` (`O2/src/client.rs:686`) — the ONE-ARG
  form the GitHub module uses (via `BasicClient`);
  `CsrfToken::new(String)` / `.secret()` generated by the crate's
  new_secret_type macro (`O2/src/types.rs:159-166`, `CsrfToken` at 559) — so
  plan 21's `random_state()` value feeds it verbatim and the controller's
  state match stays authoritative.
- `PkceCodeVerifier::new(String)` (macro, `O2/src/types.rs:417-424` region),
  `PkceCodeChallenge::from_code_verifier_sha256(&PkceCodeVerifier)` —
  deterministic S256 from OUR verifier (`O2/src/types.rs:476-490`),
  `AuthorizationRequest::set_pkce_challenge` (`O2/src/code.rs:135`),
  `add_scope`/`add_extra_param` (`O2/src/code.rs:71,89`),
  `url(self) -> (Url, CsrfToken)` (`O2/src/code.rs:148`). Plan 21's
  `PkcePair` remains the entropy source; the crate's
  `new_random_sha256()` (which pulls rand 0.8 `thread_rng`,
  `O2/src/types.rs:451-468`) is never called.
- Token exchange: `exchange_code(AuthorizationCode) -> CodeTokenRequest`
  (`O2/src/client.rs:808`), `.set_pkce_verifier(PkceCodeVerifier)`
  (`O2/src/token/mod.rs:179`), `.set_redirect_uri` (`O2/src/token/mod.rs:185`),
  `.request_async(self, &C) -> impl Future` where `C: AsyncHttpClient<'c>`
  (`O2/src/token/mod.rs:227-236`); `impl AsyncHttpClient for reqwest::Client`
  ships behind the `reqwest` feature (`O2/src/reqwest_client.rs:6-37`), and
  natively its boxed future is `Send + Sync` (reqwest_client.rs:9-12) — fits
  plan 21's native `AuthIo::drive` `Send + 'static` bound.
- `TokenResponse` accessors: `access_token()`, `expires_in() ->
  Option<Duration>`, `refresh_token()` (`O2/src/token/mod.rs:570-583`);
  `exchange_refresh_token` (`O2/src/client.rs:849`).
- Discovery: `CoreProviderMetadata::discover_async(IssuerUrl, &http_client)`
  (`OIDC/src/discovery/mod.rs:308-314`) + `CoreClient::
  from_provider_metadata(metadata, ClientId, Option<ClientSecret>)`
  (`OIDC/src/client.rs:242-261`); Google's discovery document verified live
  (200, survey B2). openidconnect's `Client::authorize_url` takes THREE
  arguments — `authorize_url<NF, RS, SF>(&self, authentication_flow:
  AuthenticationFlow<RS>, state_fn: SF, nonce_fn: NF)`
  (`OIDC/src/client.rs:760-768`, verified this session) — unlike oauth2's
  one-arg form above; the flow is `CoreAuthenticationFlow::AuthorizationCode`
  (alias `AuthenticationFlow<CoreResponseType>` at `OIDC/src/core/mod.rs:50-51`,
  `AuthorizationCode` variant at `OIDC/src/authorization.rs:23`; `Core*`
  types import from `openidconnect::core::{...}`, the crate's own doc
  example at `OIDC/src/lib.rs:152-158`, `pub mod core` at
  `OIDC/src/lib.rs:637`), and `Nonce::new_random()` / `Nonce::new(String)`
  exist (`OIDC/src/types/mod.rs:341-345` + the new_secret_type macro). Its
  request type carries `add_scope`/`set_pkce_challenge`/`set_redirect_uri`
  (`OIDC/src/authorization.rs:67,109,197`) and
  `url(self) -> (Url, CsrfToken, Nonce)` (`OIDC/src/authorization.rs:204`).
- `exchange_code` on the OIDC client (`OIDC/src/client.rs:868`) returns
  `CodeTokenRequest` with the same `request_async`; `openidconnect::
  TokenResponse` adds `id_token()` (`OIDC/src/token.rs:19`);
  `IdToken::claims(&IdTokenVerifier, &Nonce)` (`OIDC/src/id_token/mod.rs:113`,
  verifier from `client.id_token_verifier()`, `OIDC/src/client.rs:642`).
- `UserInfoClaims` getters via the crate's field_getters_setters macro with
  `pub self` mode: `name()`, `picture()`, `email()`, `email_verified()`,
  `preferred_username()` (`OIDC/src/user_info.rs:358-376`, macro generates
  the getter named after each field); `user_info(AccessToken,
  Option<SubjectIdentifier>)` (`OIDC/src/client.rs:1362-1368`).
- The raw id_token JWT string is recoverable: `Serialize` for
  `JsonWebToken` emits `"{signing_input}.{signature_base64}"`
  (`OIDC/src/jwt/mod.rs:527`), so `serde_json::to_value(&id_token)` yields the
  full JWT as a JSON string.
- Revocation, verified by absence this session: openidconnect 4.0.1 does NOT
  model the RFC 8414 `revocation_endpoint` discovery field —
  `grep -rn "revocation_endpoint" /tmp/openidconnect-4.0.1/src/` returns
  zero hits, and the `ProviderMetadata` struct
  (`OIDC/src/discovery/mod.rs:43-100`) has no revocation field with no such
  accessor in its generated getter block. The crate's `Client::revoke_token`
  is real but returns `Err(ConfigurationError)` unless `set_revocation_url`
  ran first (`OIDC/src/client.rs:1273-1282`; setter at `OIDC/src/client.rs:511+`).
  Consequence: revocation URLs come from config/fixed endpoints and go out
  as plain reqwest POSTs (Implementation Phases Step 3.2), not through the
  crate's revocation plumbing.
- openidconnect re-exports every oauth2 type this plan touches
  (`OIDC/src/lib.rs:662-677`) — the OIDC modules import from `openidconnect`
  only; the GitHub module imports `oauth2` directly. Two dep lines, no
  version skew possible (openidconnect pins `oauth2 = "5.0.0"`).

Protocol facts (survey Part B, RFC texts fetched there 2026-10-12): PKCE S256
mandatory-when-capable (RFC 7636 §4.2), high-entropy `state` + exact match
(RFC 8252 §8, rfc8252.txt:755-763), redirect URI stored with the pending
request and re-sent at exchange (rfc8252.txt:768-779) — plan 21's controller
already implements all three; this plan only feeds it real providers.
Loopback interface rules (rfc8252.txt:511-529, 598-616): bind 127.0.0.1 only,
ephemeral port, plain http acceptable, close the port when the request
completes. Embedded user-agents forbidden (rfc8252.txt:802-806) — the flow
always uses the system browser. Client secrets in native apps are not
confidential (rfc8252.txt:665-667): Google treats the desktop client secret
as optional at exchange; GitHub's token endpoints accept public-client
exchange with just client_id + code when no secret is configured — but the
config surface carries secrets anyway because some generic issuers require
them, and they are config inputs, never source constants.

Provider specifics (survey B2, docs fetched 2026-10-12; GitHub REST shapes
re-fetched and pinned this session):

- Google (developers.google.com native-app + loopback-migration): loopback
  `http://127.0.0.1:{port}` documented and recommended for Desktop client
  type; redirect_uri must match exactly; refresh token only on first consent
  (`prompt=consent` + `access_type=offline` re-issue levers); no rotation on
  use → `RefreshPolicy::ReuseUntilDead`; Testing-status projects expire
  refresh tokens in 7 days.
- GitHub (docs.github.com authorizing-oauth-apps + REST): authorize
  `https://github.com/login/oauth/authorize`, token
  `https://github.com/login/oauth/access_token` with
  `Accept: application/json` (default response is urlencoded text); PKCE S256
  supported (plain not); custom-scheme redirect URIs accepted and trigger the
  account picker; loopback callbacks accepted with any port (RFC 8252 §7.3);
  default tokens are non-expiring with no refresh → `RefreshPolicy::None`;
  expiring-token opt-in rotates both tokens — not assumed. REST: `GET /user`
  returns `login` (string), `name` (string or null), `email` (string or
  null), `avatar_url` (uri) with `Authorization: Bearer` +
  `Accept: application/vnd.github+json` (docs re-fetched this session); when
  `email` is null, `GET /user/emails` (needs the `user:email` scope) returns
  `[{email, primary, verified, visibility}]` — take the `primary` entry.
  Token revocation: `DELETE /applications/{client_id}/token` with the
  `access_token` body, 204 on success; the docs page does not spell out the
  auth scheme for this endpoint family (Basic client_id:client_secret is the
  convention) — confirmed during manual QA, flagged in Open Questions.
- Generic OIDC: assume refresh rotation (RFC 9700 public-client framing,
  survey B2) → `RefreshPolicy::Rotated`; `invalid_grant` on refresh is
  handled uniformly regardless of provider (survey Open Questions: the Google
  Desktop-vs-web expiry distinction is not documented — controller-level
  `Error` state + sign-out is the uniform response).

Deep-link delivery, verified in-repo this session: Windows HKCU registration
of the whole `gpui-starter://` scheme at boot
(`deep_link_registration.rs:20-53`, call site `src/app/init.rs:78-89`); the
Linux desktop entry ships `MimeType=x-scheme-handler/gpui-starter;`
(`resources/com.gpui-starter.app.desktop`, read this session) so packaged
deb/rpm installs get scheme delivery, but a `cargo run` dev process has no
registered handler; macOS needs `CFBundleURLTypes` in the bundle (not
verifiable here — 0.3.9 macOS sources absent from this host's registry,
survey Open Questions; the dev-app script's plist was not read). Consequence,
encoded in the config surface: **default redirect mode is `loopback` on every
native platform** (RFC 8252 first choice, zero registration dependency);
`scheme` is an opt-in per-provider config value documented as requiring a
registered handler (Windows always; Linux packaged installs; macOS bundled).
This inverts the spec's shorthand ordering ("capture at the deep link, with a
loopback fallback") for a concrete reason: in every context this repo can
actually verify (Linux dev, macOS dev), the custom scheme is the unusable
one. Both paths are implemented, tested, and config-selectable.

Diagnostics surface, verified: the diagnostics page renders the capability
registry generically — `for (name, status) in capabilities` over
`capabilities::snapshot(cx)` (`src/features/pages/diagnostics/rows.rs:387-396`)
— so per-provider capability entries appear with zero page edits, and the
session row already reads `session::snapshot(cx)` (`rows.rs:20`) for the
signed-in label.

No cargo gate was run during planning (doc-only scope, matching the surveys,
plans 01-20, and plan 21). Every Verification command is for the implementer;
the first compile evidence points are Step 1.2 (`cargo check`) and Step 7.2.

## Architecture

### Where the code lives (decided)

App-side `src/services/auth_social/`, native-only — not inside
`crates/gpui-auth`. Deciding factors:

1. Wasm discipline: oauth2/openidconnect have no wasm32-unknown-unknown
   compile evidence anywhere in this repo (survey Open Questions; getrandom
   0.2/rand 0.8/rsa graph). Keeping them behind the app's not-wasm table
   leaves the foundation crate's wasm story (plan 21's) untouched and
   `just wasm-check` unexposed to the risk.
2. The providers are configuration-bound app code (client ids, redirect
   modes, REST endpoints), not reusable framework. Plan 21's crate stays
   protocol-generic; AGENTS.md's second-implementation rule is satisfied by
   the trait having three impls total (mock in 21, two here).
3. The module implements plan 21's `AuthProvider` — a trait object seam — so
   app-side placement costs nothing in coupling, and removal is one services
   module (Seam Touches) instead of a crate-graph change.

Anti-speculation audit: `AuthProvider` impls — `MockProvider` (21),
`OidcProvider` used for both Google and generic issuer (two constructors,
one impl — the second impl justifying the abstraction already exists),
`GitHubProvider`. `LoopbackServer` is the only new concrete service; it has
one production consumer and one test consumer (the fake IdP harness reuses
its HTTP parsing). No other new abstractions.

### Entities, state ownership, events

- `SocialFlows` (GPUI entity, `flows.rs`) owns everything: `controllers:
  BTreeMap<ProviderKind, Entity<AuthController>>`, `providers:
  BTreeMap<ProviderKind, Arc<dyn AuthProvider>>` plus the concrete handles
  (`Arc<OidcProvider>` ×2, `Arc<GitHubProvider>` — needed to reach the
  loopback receiver, see below), `config: SocialConfig`, `active_flow:
  Option<ActiveFlow { kind, _loopback_task: Option<Task<()>>,
  _abort: Option<tokio::task::AbortHandle> }>`, `session_owner:
  Option<ProviderKind>`, and the observer/subscriptions. Single writer for
  all of it; created once by `install(cx)` after plan 21's `auth::install`.
- `OidcProvider` / `GitHubProvider` own per-flow redirect state in
  `Mutex<Option<PendingRedirect>>` (single pending slot, same semantics as
  the controller's): for loopback mode this holds the `oneshot::Receiver`
  end of the captured redirect plus the bound port; `start_flow` binds the
  listener and replaces any previous pending (a stale listener is aborted by
  `SocialFlows` when it replaces the flow). Secrets (`client_secret`,
  PKCE verifier, state) live in plan 21's `SecretString`/`FlowLaunch`
  types — zeroized, redacted Debug, never logged.
- Tokens live only in the `TokenStore` (plan 21's `KeyringStore` under
  provider ids `"google"`, `"github"`, `"oidc"`), never in `AppConfig`, never
  in `auth-providers.json` (that file holds client credentials only, is
  user-authored, and is never written by the app).
- Events: providers emit nothing; `AuthController` per provider emits plan
  21's `AuthEvent`s. `SocialFlows` subscribes to each controller: on
  `AuthorizationRequested { url }` it (a) opens the system browser via
  `desktop_actions::open_url` (http/https guard passes — authorize URLs are
  https) and (b) for loopback mode spawns the drain task below. On
  `FlowCompleted`/state change it refreshes capability entries and syncs the
  session mapping.
- Session mapping: `session_owner` is the provider whose controller most
  recently reached `SignedIn` (or the first restored one at boot, in
  `ProviderKind` order). `sync_session(cx)` copies the owner controller's
  `AuthState` into `session::set_state` (`SessionState` shapes are
  1:1 with `AuthState`, session.rs:3-12 vs 21.md:475-476); a sign-out from
  the owner clears `session_owner` first (another signed-in provider can
  claim ownership on its next state change; without one, session goes
  `SignedOut`). Other providers' sign-ins do not touch the session global —
  documented, and plan 24's UI reads the full snapshot anyway.

Data flow, loopback sign-in (Google configured, default mode):

```text
SocialFlows::start(Google, cx)
  controller(google).start_sign_in  ──io.drive──▶ OidcProvider::start_flow
    discover_async(issuer, http)  →  CoreClient
    tokio TcpListener::bind("127.0.0.1:0")  →  port
    redirect_uri = http://127.0.0.1:{port}/   (stashed with the receiver)
    PkcePair::generate (S256)  +  random_state()
    authorize_url(...CsrfToken::new(state)...).set_pkce_challenge(...)
  ◀─ FlowLaunch
  controller: pending = launch, SigningIn, emit AuthorizationRequested { url }
SocialFlows subscriber:
  desktop_actions::open_url(url)                          [system browser]
  drain task: rt.spawn(server.wait()) → cx.spawn await → on raw redirect URL:
    controller.complete_callback(raw_url, cx)             [same call as deep links]
      RedirectParams::from_url → state exact-match → exchange (PKCE verifier,
      redirect_uri re-sent) → userinfo → TokenSet{account} → store.save
  timeout 300 s (no browser return): controller.cancel(cx)
browser GET http://127.0.0.1:port/?code&state → server task responds 200 HTML,
  sends raw URL through the oneshot, exits (port closed — RFC 8252 hygiene)
```

Data flow, scheme mode (opt-in): identical until `start_flow`, except
`redirect_uri = gpui-starter://auth/callback`, no listener. The OS launches
the app (Windows HKCU / packaged Linux .desktop), the link arrives as
`DeepLinkReceived` (cold launch: `single_instance.rs:138-140` + `web.rs:40`;
second instance: IPC forward `single_instance.rs:415-416` → `RemoteCommand`),
plan 21's drain guard calls `auth::handle_callback_link`, whose new first
line routes to `auth_social::route_callback(link, cx)` → the active-flow
controller's `complete_callback`. Cold links with no flow running fall
through to plan 21's mock controller (`NoPendingFlow`, documented behavior).

### Module tree / crate boundary

```text
src/services/auth_social/                  (all #[cfg(not(target_family = "wasm"))]
├── mod.rs          install, ProviderKind, SocialAuthSnapshot,             except the tiny wasm
│                   route_callback, module header w/ removal recipe        shim noted in mod.rs
├── config.rs       SocialConfig schema + loader (auth-providers.json)
├── loopback.rs     LoopbackServer: 127.0.0.1:0 single-shot HTTP capture
├── flows.rs        SocialFlows entity: controllers, browser open, drain
│                   tasks, session sync, capability entries
├── providers/
│   ├── mod.rs      shared helpers: TokenSet assembly, now_unix, error maps
│   ├── oidc.rs     OidcProvider (google() + generic() ctors, openidconnect)
│   └── github.rs   GitHubProvider (oauth2 + REST profile)
├── *.test.rs       sibling test modules per repo convention
└── providers/fake_idp.rs  test-only mini IdP/GitHub server (tokio, in-process)

crates/gpui-auth (plan 21, one amendment): AccountInfo gains
  avatar_url: Option<String> with #[serde(default)]  ← Interface Contract note
src/services/auth.rs (plan 21 adapter, two edits): handle_callback_link
  guard + auth_social capability ownership moved to auth_social::install
```

## Interface Contract

Load-bearing for plan 24 (account UI) and any boilerplate consumer. Line
stable; extend prose, never reflow the block. Consumers cite it as
`23:Interface Contract`.

```rust
// src/services/auth_social/mod.rs — native-only module (cfg not-wasm).
// Mirrors 21's adapter discipline: install once at boot, then imperative
// calls from UI/commands.

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProviderKind { Google, GitHub, Oidc }

impl ProviderKind {
    pub fn id(&self) -> &'static str;          // "google" | "github" | "oidc" (store key)
    pub fn display_name(&self) -> &'static str; // "Google" | "GitHub" | "OIDC"
    pub fn parse(s: &str) -> Option<Self>;     // config-file/URL-safe round-trip
}

/// Per-provider state for diagnostics and plan 24's UI.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SocialProviderStatus {
    pub configured: bool,        // client_id present in auth-providers.json
    pub redirect: RedirectMode,  // Loopback | Scheme (config value)
    pub state: Option<gpui_auth::AuthState>, // None = controller not installed
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SocialAuthSnapshot {
    pub providers: Vec<(ProviderKind, SocialProviderStatus)>, // ProviderKind order
}
impl SocialAuthSnapshot {
    pub fn any_configured(&self) -> bool;
    pub fn session_owner(&self) -> Option<ProviderKind>;     // from SocialFlows
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RedirectMode { Loopback, Scheme }

/// Boot wiring. Reads auth-providers.json, builds one AuthController per
/// configured provider over plan 21's KeyringStore + AuthIo, restores tokens,
/// registers capability entries, subscribes browser-open + session sync.
/// Idempotent: second call replaces the flows entity. Call after
/// services::auth::install (needs its globals).
pub fn install(cx: &mut gpui_kit::App);
/// Begin a browser flow for one provider; no-op (logged) when unconfigured.
pub fn start(kind: ProviderKind, cx: &mut gpui_kit::App);
/// Best-effort revoke + token delete for one provider (controller.sign_out).
pub fn sign_out(kind: ProviderKind, cx: &mut gpui_kit::App);
pub fn controller(kind: ProviderKind, cx: &gpui_kit::App)
    -> Option<gpui_kit::Entity<gpui_auth::AuthController>>;
pub fn snapshot(cx: &gpui_kit::App) -> SocialAuthSnapshot;
/// gpui-starter://auth/callback router. True when the link was routed to the
/// active-flow controller (caller skips plan 21's mock fallback).
pub fn route_callback(link: &str, cx: &mut gpui_kit::App) -> bool;
```

gpui-auth contract amendment (applies plan 21's amendment rule). Plans 22
and 24 exist on disk (`docs/plans/22-auth-email.md`, `docs/plans/
24-auth-ui.md`, both written before this plan — verified by directory
listing this session). 22 is a live consumer of the amended type: it
constructs `AccountInfo` exhaustively at 22.md:630-632, and adding
`avatar_url` breaks that literal unless 22 adds `avatar_url: None` — a
one-line doc edit listed as Seam Touches row 10. 24 reads `AccountInfo`
through accessors only (avatar for the account menu), so it needs no edit.

```rust
// crates/gpui-auth/src/token.rs — AccountInfo gains one field:
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AccountInfo {
    pub id: Option<String>,
    pub email: Option<String>,
    pub display_name: Option<String>,
    #[serde(default)]                     // old keyring rows parse unchanged
    pub avatar_url: Option<String>,       // userinfo `picture` / GitHub avatar_url
}
```

Notes that bind consumers: `start` replaces any in-flight flow across ALL
providers (the previous controller gets `cancel`, its loopback listener is
aborted); `route_callback` is called by `services::auth::handle_callback_link`
before its own routing (Seam Touches row 6) and must stay cheap (URL parse +
map lookup, no I/O); capability names `auth_social`, `auth_social_google`,
`auth_social_github`, `auth_social_oidc` are set here and rendered by the
diagnostics page without edits; secrets from `auth-providers.json` are read
once at install and held in provider structs behind `SecretString` —
`SocialProviderStatus` never carries them.

## Module Layout

| File | Purpose |
| --- | --- |
| `src/services/auth_social/mod.rs` | `install`/`start`/`sign_out`/`controller`/`snapshot`/`route_callback`, `ProviderKind`, `RedirectMode`, snapshot types, module header with removal recipe |
| `src/services/auth_social/config.rs` | `SocialConfig`/`ProviderConfigFile` serde schema, `load(cx)` (path resolution + env override + 0600 perms lint) |
| `src/services/auth_social/loopback.rs` | `LoopbackServer::spawn(rt) -> LoopbackHandle`: 127.0.0.1:0 bind, single-shot capture, HTML response, timeout |
| `src/services/auth_social/flows.rs` | `SocialFlows` entity: controller map, provider map, browser-open subscriber, loopback drain spawn/abort, session sync, capability refresh |
| `src/services/auth_social/providers/mod.rs` | shared `now_unix()`, `TokenSet` assembly helper, error mapping (`RequestTokenError` → `AuthError`) |
| `src/services/auth_social/providers/oidc.rs` | `OidcProvider` (google/generic constructors, discovery, PKCE+state, exchange, userinfo, id_token fallback, revoke) |
| `src/services/auth_social/providers/github.rs` | `GitHubProvider` (oauth2 flow, `Accept: application/json` exchange, `/user` + `/user/emails` profile, DELETE token revoke) |
| `src/services/auth_social/providers/fake_idp.rs` | test-only in-process IdP/GitHub endpoints over tokio TcpListener (discovery/JWKS/token/userinfo + GitHub variants) |
| `src/services/auth_social/{config,loopback,flows}.test.rs`, `providers/{oidc,github}.test.rs` | sibling test modules (`#[cfg(test)] #[path]`) |

## Seam Touches

Every edit outside the new module:

| # | File | Change | Why minimal |
| --- | --- | --- | --- |
| 1 | `Cargo.toml:310` (not-wasm table) | `oauth2 = { version = "5.0", default-features = false, features = ["reqwest", "rustls-tls"] }` and `openidconnect = { version = "4.0", same features }` | two dep lines, native-only; reqwest feature set subsets the app's existing line (`Cargo.toml:187-193`) so one copy resolves |
| 2 | `Cargo.lock` | regenerate + stage with the manifest edits | repo practice (plan 21 seam row 4 rationale; `tests/qa_migration.rs:168-186` scope) |
| 3 | `src/services/mod.rs` | `#[cfg(not(target_family = "wasm"))] pub mod auth_social;` after `pub mod auth;` (plan 21's line) | one cfg'd module line; the `db_migrations` cfg-re-export pattern exists at `src/lib.rs:47-49` |
| 4 | `src/lib.rs` services re-export (~line 57-61) | `#[cfg(not(target_family = "wasm"))] auth_social,` inside the existing `pub use services::{...}` | same style as the list's neighbors; no new top-level export |
| 5 | `src/app/init.rs` (after plan 21's `crate::services::auth::install(cx);`) | `crate::services::auth_social::install(cx);` | one boot hook; must follow auth::install (KeyringStore/AuthIo precedent) — itself after tokio runtime 207 / secure storage 216 / session 217 per plan 21 seam 7 |
| 6 | `src/services/auth.rs` (plan 21 file) — `handle_callback_link` body | first statement: `#[cfg(not(target_family = "wasm"))] if crate::services::auth_social::route_callback(link, cx) { return true; }` (wasm: no line at all) | one guard line; social routing precedes the mock fallback; on wasm the module does not exist so the cfg line keeps the file compiling |
| 7 | `src/services/auth.rs` — capability block | delete the `auth_social` entry from `install` (keep `auth_email`); on wasm keep a cfg'd `auth_social` set with the final reason string (see Platform & Wasm Gating) | ownership moves to `auth_social::install` natively; the wasm entry cannot live there (module absent), so it stays in the adapter behind cfg |
| 8 | `crates/gpui-auth/src/token.rs` + its tests | `AccountInfo.avatar_url` field per the contract amendment | additive serde field with `#[serde(default)]`; `tests/snapshot_tests.rs` does not cover the crate; plan 21's token round-trip test extends |
| 9 | `docs/plans/21-auth-foundation.md` | append an amendment note in its contract header (avatar_url, mirror of its own 2026-10-12 amendment precedent) | keeps the dependency plan honest for any 22/24 author |
| 10 | `docs/plans/22-auth-email.md` (22.md:630-632) | add `avatar_url: None` to the `AccountInfo` literal in the mock provider's `TokenSet` mint | one token in an existing struct literal; 22 constructs `AccountInfo` exhaustively, so the amendment does not compile without it. Whoever implements 22 after 23 takes this from 22's own text; if 22 lands first, its implementer adds the field when pulling the amended crate — either order is one line |
| 11 | `src/platform/process/single_instance.rs` (`:296` and `:415`) | both forwarded-link log lines currently log `link` verbatim; this plan is the first to put a single-use authorization code in that query (`gpui-starter://auth/callback?code=...&state=...`), so the fix lands with it: in both `tracing::info!` calls replace the `link` field with `path = link.split('?').next().unwrap_or(&link)` (route stays visible for debugging; the query never carries anything but credentials today) | closes the security tripwire 21 flagged and 22 recorded (22's Open Questions, 2026-10-12 fix round); two one-line edits, no behavior change — the full link still flows to `events::emit` untouched, only the log field is sanitized |

Deliberate non-touches: no `src/shell/route.rs` edit (`auth` must NOT become a
page host — plan 21's rule; callback links never reach `parse_deep_link`),
no sidebar/page wiring (UI is plan 24), no `AppConfig` field (snapshot schema
pinned, `tests/snapshot_tests.rs:26,57`; provider config is a separate
never-committed file), no `DEFAULT_ENABLED_CAPABILITIES` edit, no
`[patch.crates-io]` change, no justfile recipe (no server to run), no
deep-link registration edits (whole-scheme registration already exists and
`auth` needs none), no diagnostics page edit (registry renders generically,
`rows.rs:387-396`).

### Removal Recipe

`src/services/auth_social/mod.rs` opens with (mirrors
`src/gallery/mod.rs:1-7`):

```rust
//! Removable social-login wiring. `install`, `start`, `sign_out`,
//! `controller`, `snapshot`, and `route_callback` are the only public
//! surface. Removal = delete this directory and revert the seam touches:
//! the `route_callback` guard and the auth_social capability move in
//! services/auth.rs (restore plan 21's original entry), the install line in
//! app/init.rs, the cfg'd module + re-export lines in services/mod.rs and
//! lib.rs, the oauth2/openidconnect lines in Cargo.toml's not-wasm table
//! with a regenerated Cargo.lock, and the AccountInfo.avatar_url field in
//! crates/gpui-auth (keyring rows keep parsing either way).
```

Ordered removal steps:

1. Revert the two edits in `src/services/auth.rs` (guard line, capability
   block back to plan 21's original `auth_social` entry).
2. Remove the `install` line from `src/app/init.rs`.
3. `rm -r src/services/auth_social/`; revert `src/services/mod.rs` and the
   `src/lib.rs` re-export.
4. Drop `avatar_url` from `AccountInfo` (and its test lines) in
   `crates/gpui-auth`; drop the plan-22 doc note (Seam Touches row 10) and
   the plan-21 amendment sentence; drop the oauth2/openidconnect manifest
   lines; regenerate `Cargo.lock` and stage it.
5. Revert the two log-field edits in `src/platform/process/single_instance.rs`
   (seam row 11) OR keep them — the redaction is a strict security
   improvement with no coupling to this feature; keeping it is the
   recommended choice and costs nothing.
6. Run the gates in Verification; `grep -rn "auth_social\|openidconnect\|oauth2::" src/ crates/ tests/ Cargo.toml`
   returns nothing (`oauth2::` with the trailing colons — the bare word
   `oauth2` otherwise appears in unrelated URLs inside plan docs only).

## Implementation Phases

## Phase 1: Dependencies + config surface

### Step 1.1: Manifest lines

- Files: `Cargo.toml` (not-wasm table), regenerated `Cargo.lock`.
- Build: append the two dependency lines exactly per Seam Touches row 1.
  Confirm no `oauth2`/`openidconnect` line exists elsewhere first
  (`grep -n 'oauth2\|openidconnect' Cargo.toml` — clean today, verified this
  session).
- Tests: none.
- Done-check: `cargo check -p gpui-starter` passes on the host;
  `git diff Cargo.lock` shows `oauth2`, `openidconnect`, and their
  native-only transitives added; stage the lock with the manifest.

### Step 1.2: Config schema + loader

- Files: `src/services/auth_social/mod.rs` (scaffold: cfg'd module with the
  removal header, `ProviderKind`, `RedirectMode`), `config.rs`,
  `config.test.rs`, `src/services/mod.rs` (module line), `src/lib.rs`
  (re-export).
- Build: `ProviderKind::{id, display_name, parse}` per the contract
  (`parse` accepts the lowercase ids). Schema:

  ```rust
  #[derive(Clone, Debug, Default, PartialEq, serde::Deserialize)]
  pub struct SocialConfig {
      #[serde(default)] pub google: Option<ProviderConfig>,
      #[serde(default)] pub github: Option<ProviderConfig>,
      #[serde(default)] pub oidc: Option<OidcConfig>,
  }
  #[derive(Clone, Debug, PartialEq, serde::Deserialize)]
  pub struct ProviderConfig {
      pub client_id: String,
      #[serde(default)] pub client_secret: Option<String>, // public clients omit
      #[serde(default)] pub redirect: Option<String>,       // "loopback"|"scheme"
      #[serde(default)] pub scopes: Option<Vec<String>>,    // overrides defaults
  }
  #[derive(Clone, Debug, PartialEq, serde::Deserialize)]
  pub struct OidcConfig {
      pub issuer: String,               // https:// issuer URL, no trailing slash
      #[serde(default)] pub revoke_url: Option<String>, // RFC 7009 endpoint;
      //                                   optional — sign-out skips network revocation
      //                                   when absent (openidconnect cannot discover
      //                                   it; see Research Digest)
      #[serde(flatten)] pub base: ProviderConfig,
  }
  ```

  `load(cx) -> (SocialConfig, Vec<String> /*warnings*/)`: path =
  `std::env::var_os("GPUI_AUTH_PROVIDERS")` or
  `crate::app_state::paths(cx).config_dir.join("auth-providers.json")`;
  missing file → `(Default::default(), [])` (unconfigured is normal);
  read cap 64 KiB (same bounding discipline as `config_store.rs:32-33`, whose
  own cap is 1 MiB for a different file; the value here is deliberately
  smaller);
  parse failure → default + a warning string carried to the capability
  reason. `#[cfg(unix)]` perms lint: `std::fs::metadata` mode `& 0o777 !=
  0o600` → warning "auth-providers.json is group/world readable; chmod 600".
  Resolution helper `effective_redirect(kind, cfg) -> RedirectMode`:
  `Some("scheme")` → Scheme, `Some("loopback")`/None → Loopback, anything
  else → Loopback + warning. The app never writes this file; a template
  lands in the module docs (doc comment with a JSON example including all
  three providers). That doc comment also tells users to register redirect
  URIs verbatim, trailing slash included — the loopback redirect this plan
  sends is `http://127.0.0.1:{port}/` (slash before the query), and Google
  exact-matches registered URIs, so the Google console entry must read
  `http://127.0.0.1` with the port field handled per its loopback docs
  (survey B2) and any path-registered provider must match `/` exactly.
- Tests: parse a full three-provider JSON; defaults with missing keys;
  empty file → default config; malformed JSON → default + warning;
  `GPUI_AUTH_PROVIDERS` env override (create a temp file via
  `std::env::temp_dir` + unique name, set/restore the env var around the
  call); perms warning triggers on a 0644 temp file (unix host);
  `effective_redirect` truth table; `ProviderKind::parse` round-trip.
- Done-check: `cargo test -p gpui-starter --lib services::auth_social`
  passes.

## Phase 2: Loopback server

### Step 2.1: `LoopbackServer`

- Files: `loopback.rs`, `loopback.test.rs`, `mod.rs` (`pub(crate) mod
  loopback;`).
- Build: `LoopbackServer::spawn(rt: &tokio::runtime::Handle) ->
  LoopbackHandle` where `LoopbackHandle { port: u16, wait:
  tokio::sync::oneshot::Receiver<String>, abort: tokio::task::AbortHandle }`.
  Shape: `LoopbackServer::spawn` binds the listener itself (std bind is
  sync/cheap — happens on the caller's thread), reads the port, then
  `rt.spawn`s the accept loop and returns the handle; the listener moves
  into the task, accepting runs on tokio (the runtime is the auth `AuthIo`
  one from `runtime_and_client`, `tokio_runtime.rs:80-83`). Accept loop:
  `timeout(Duration::from_secs(
  300), async { loop { let (mut stream, _) = listener.accept().await?;
  read until "\r\n\r\n" or 8 KiB cap; parse the request line
  (`GET <path> HTTP/1.x`); rebuild the raw URL as
  `format!("http://127.0.0.1:{port}{path}")`; if the query contains `code=`
  or `error=`: write the response, send the URL, return Ok; else respond
  404 and keep accepting (favicon probes) } })`. Response: minimal
  `HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\
  Content-Length: {n}\r\nConnection: close\r\n\r\n` + a plain English
  literal page "Sign-in captured. You can close this tab and return to the
  app." (browser-side copy, outside the app locale catalogs — Zed's pattern
  B precedent, auth-zed.md §2). On timeout/oneshot-dropped: exit silently
  (listener drops → port closed, RFC 8252 loopback hygiene). Errors map to
  task exit; `LoopbackHandle::wait()` is consumed by Phase 5's drain task.
  Tracing target `"gpui_starter::auth_social::loopback"`, logs port and
  completion only — never the query (it carries the code).
- Tests (`#[gpui_kit::test]` with a scratch multi-thread tokio runtime built
  in the test, since the app global is absent there): spawn server; `reqwest`
  GET `http://127.0.0.1:{port}/?code=x&state=y` → response status 200, body
  contains "close this tab", `wait()` (await with a 1 s timeout) resolves to
  the exact raw URL; a `GET /favicon.ico` first does NOT resolve the
  receiver and the connection gets 404 while the server keeps accepting;
  dropped receiver (a second flow's server replaced) → task exits within the
  test's tolerance after a capture (assert `abort.is_finished()` after
  driving it with another request); port is non-zero and two spawned servers
  get different ports.
- Done-check: `cargo test -p gpui-starter --lib services::auth_social`
  passes.

## Phase 3: gpui-auth amendment + OIDC provider

### Step 3.1: `AccountInfo.avatar_url`

- Files: `crates/gpui-auth/src/token.rs`, `crates/gpui-auth/src/token.test.rs`,
  `docs/plans/21-auth-foundation.md` (amendment note),
  `docs/plans/22-auth-email.md` (Seam Touches row 10).
- Build: field per the contract block (`#[serde(default)]`); extend the
  existing serde round-trip test to include it; add one test that a JSON
  value WITHOUT the field deserializes (`#[serde(default)]` proof — old
  keyring rows). Append the amendment sentence to plan 21's contract header
  prose (not the code block — plan 21's own rule is prose-append; the block
  is regenerated by this plan's contract section instead), and add
  `avatar_url: None` to plan 22's `AccountInfo` literal (22.md:630-632).
- Tests: as above.
- Done-check: `cargo test -p gpui-auth --all-features` passes.

### Step 3.2: `OidcProvider` flow core

- Files: `providers/mod.rs` (helpers), `providers/oidc.rs`,
  `providers/oidc.test.rs`, `providers/fake_idp.rs`, `providers/mod.rs`
  declare in `mod.rs`.
- Build: helpers in `providers/mod.rs`: `pub(crate) fn now_unix() -> i64`
  (`chrono::Utc::now().timestamp()`), `pub(crate) fn token_set(provider_id,
  access: String, refresh: Option<String>, id_token: Option<String>,
  expires_in: Option<std::time::Duration>, scope: Option<String>, account:
  Option<AccountInfo>) -> TokenSet` (SecretString-wraps each; `expires_at =
  expires_in.map(|d| now_unix() + d.as_secs() as i64)`), and
  `pub(crate) fn map_token_error(e: impl std::fmt::Display) -> AuthError`
  (→ `InvalidTokenResponse(e.to_string())`; no token text is ever in the
  Display of oauth2 errors — they carry error/error_description fields
  only, but the map does not log regardless). `OidcProvider`:

  ```rust
  pub struct OidcProvider {
      id: &'static str,                    // "google" | "oidc"
      display: &'static str,
      issuer: String,                      // https://accounts.google.com for google()
      client_id: String,
      client_secret: Option<SecretString>,
      scopes: Vec<String>,                 // google default: ["openid","email","profile"]
      redirect: RedirectMode,
      refresh_policy: RefreshPolicy,       // google: ReuseUntilDead; oidc: Rotated
      extra_auth_params: Vec<(String, String)>,  // google: access_type=offline
      revoke_url: Option<String>,         // generic only (OidcConfig field);
                                          // google() hardcodes its endpoint.
                                          // Cannot come from discovery —
                                          // openidconnect does not model
                                          // revocation_endpoint (Research Digest)
      pending: std::sync::Mutex<Option<PendingRedirect>>,
  }
  impl OidcProvider {
      pub fn google(cfg: &ProviderConfig) -> Self;
      pub fn generic(cfg: &OidcConfig) -> Self;   // issuer from config, default
  }                                                // scopes openid/profile/email
  ```

  `start_flow(http)`: `IssuerUrl::new(issuer)` (→ `AuthError::
  InvalidTokenResponse` on bad issuer), `CoreProviderMetadata::
  discover_async(issuer_url, &http).await` (→ `AuthError::Network`),
  `CoreClient::from_provider_metadata(metadata, ClientId::new(client_id),
  client_secret.map(|s| ClientSecret::new(s.expose().to_string())))`;
  redirect handling: Loopback → `LoopbackServer::spawn(&
  tokio::runtime::Handle::current())` — `start_flow` runs inside
  `AuthIo::drive` on the tokio runtime, so `Handle::current()` is valid
  there; this is the one place the provider assumes it executes on tokio,
  stated in its doc comment (the controller always drives it, and the app's
  `AuthIo` carries `Some(rt)`); stash `PendingRedirect { port, receiver }`
  in `pending`, `RedirectUrl::new(format!("http://127.0.0.1:{port}/"))`;
  Scheme → `RedirectUrl::new("gpui-starter://auth/callback")`, pending =
  the scheme marker (no listener). Then `let pkce =
  gpui_auth::PkcePair::generate()?; let state =
  gpui_auth::random_state()?; let nonce = Nonce::new_random();` (nonce
  generated by us, stashed, and supplied to the builder — see below).
  Build the request (openidconnect's `authorize_url` takes THREE args —
  flow, state_fn, nonce_fn, `OIDC/src/client.rs:760-768`):
  `client.authorize_url(CoreAuthenticationFlow::AuthorizationCode,
  || CsrfToken::new(state.expose().to_string()),
  || nonce.clone()).add_scopes(scopes.map(Scope::new))` +
  `.set_pkce_challenge(
  PkceCodeChallenge::from_code_verifier_sha256(&PkceCodeVerifier::new(
  pkce.expose_verifier().to_string())))` + `.set_redirect_uri(
  Cow::Owned(redirect_url))` + `.add_extra_param(k, v)` for extras; `.url().0`
  → the
  authorize URL string (`.url()` returns `(Url, CsrfToken, Nonce)`
  — a three-tuple on this type, `OIDC/src/authorization.rs:204`); stash
  `nonce: SecretString(nonce.secret().to_string())` in `pending` for
  id_token validation (the FlowLaunch state remains the CSRF check, the
  nonce is the id_token binding). Return `FlowLaunch { authorize_url,
  state, pkce: Some(pkce), redirect_uri }`.

  `complete_callback(http, redirect, launch)`: `redirect.code` required
  (else `ProviderDenied` when `error` set, `InvalidTokenResponse`
  otherwise); `exchange_code(AuthorizationCode::new(code))
  .set_pkce_verifier(PkceCodeVerifier::new(launch.pkce verifier —
  launch.pkce.as_ref().expect("browser flows carry PKCE").
  expose_verifier().to_string()))` — the launch's own verifier, honoring
  the contract's "launch carries what exchange needs"; google (public
  client): when no client_secret and the provider is Google, send the
  client_id explicitly via `.add_extra_param("client_id", ...)`
  (RFC 8252 §8.5 posture; openidconnect's builder includes client_id in
  the body for `AuthType::RequestBody` — verify the default auth type at
  compile, noted in the step's done-check). `.request_async(&http).await`
  → `CoreTokenResponse`; build `TokenSet` via the helper (`access_token()
  .secret()`, `refresh_token()`, `id_token()` serialized to its JWT string
  via `serde_json::to_value(&id_token).as_str()` — `OIDC/src/jwt/mod.rs:527`,
  `expires_in()`, scopes joined). Profile: `client.user_info(
  token_response.access_token().clone(), None).request_async(&http).await`
  → `AccountInfo { id: Some(subject().as_str()), email: email() map +
  `email_verified` gate (unverified email still stored, flag ignored —
  documented), display_name: name() or preferred_username(), avatar_url:
  picture() }`; on `UserInfoError` (endpoint absent on some IdPs) fall
  back to `id_token.claims(&client.id_token_verifier(), &nonce)` with the
  stashed nonce (claims accessors same shape, `OIDC/src/claims.rs`
  standard claims) — both paths produce the same `AccountInfo` shape; on
  both failing, `AccountInfo: None` (sign-in still succeeds; label falls
  back to `provider.display_name()` per plan 21's label chain).
  `refresh(http, refresh_token)`: `exchange_refresh_token(
  RefreshToken::new(...)).request_async(&http)` → TokenSet via helper
  (merge policy applied by the controller). `revoke(http, access)`: one
  plain-reqwest RFC 7009 POST per provider, no crate plumbing — openidconnect
  4.0.1 does not model the discovery `revocation_endpoint` field at all
  (`grep -rn "revocation_endpoint" /tmp/openidconnect-4.0.1/src/` → zero
  hits, re-run this session), and its `revoke_token` errors with
  `ConfigurationError` unless `set_revocation_url` was called first
  (`OIDC/src/client.rs:1273-1282`, setter at `OIDC/src/client.rs:511+`) —
  the crate has no source for that URL, so the config does instead.
  Google → `POST https://oauth2.googleapis.com/revoke` form body `token=...`
  (endpoint from Google's OAuth docs — flagged for manual-QA confirmation,
  Open Questions). Generic → `POST {revoke_url}` (the `OidcConfig` field,
  form body `token=...`), `Ok(())` when the field is absent (best-effort
  by contract). All Send bounds satisfied natively (the
  reqwest-backed futures are `Send + Sync`, `O2/src/reqwest_client.rs:9-12`).
- Tests (`fake_idp.rs` first): in-process server on
  `TcpListener::bind("127.0.0.1:0")` (own scratch runtime), serving on any
  path: `/.well-known/openid-configuration` (issuer = the bound origin,
  authorization_endpoint/token_endpoint/userinfo_endpoint =
  same-origin paths, `jwks_uri` pointing at `/jwks` with an empty-ish
  valid JWKS `{"keys":[]}` — enough for discovery; id_token validation is
  NOT exercised by the fake since a signed JWT would need key material;
  the fake's token response omits `id_token`, exercising the userinfo
  path), `/token` (JSON with access_token/refresh_token/expires_in/scope),
  `/userinfo` (`{sub, email, email_verified, name, picture}`). Tests:
  construct `OidcProvider::generic` with issuer = fake origin; call
  `start_flow` inside `Runtime::block_on`; assert the authorize URL
  contains the fake authorization endpoint, `code_challenge`,
  `code_challenge_method=S256`, `client_id`, the 43-char state, and
  `redirect_uri=http%3A%2F%2F127.0.0.1%3A{port}%2F`; the flow bound a
  listener (pending is Loopback); complete_callback with
  `RedirectParams::from_url(&format!("http://127.0.0.1:{p}/?code=c&state={}",
  state_exposed))` (state injected by building the URL from the launch)
  returns a TokenSet whose `account.email/name/avatar_url` match the fake,
  `expires_at` ≈ now + expires_in (±2 s), secrets round-trip through
  serde; wrong-state redirect is rejected by the CONTROLLER test in Phase
  5 (provider contract says the controller state-matches first); refresh
  mints a new access token and the provider's policy is Rotated for
  generic / ReuseUntilDead for google (assert `refresh_policy()`); revoke
  with `revoke_url` set to a fake `POST /revoke` endpoint returns Ok (fake
  asserts the `token=` form body, no Authorization header leaks the
  client secret), and revoke with `revoke_url: None` returns Ok without
  any HTTP call (assert the fake saw zero requests).
- Done-check: `cargo test -p gpui-starter --lib services::auth_social`
  passes; `cargo clippy -p gpui-starter --all-features -- -D warnings`
  clean.

## Phase 4: GitHub provider

### Step 4.1: `GitHubProvider`

- Files: `providers/github.rs`, `providers/github.test.rs`, fake_idp.rs
  extends.
- Build: struct mirrors `OidcProvider` minus discovery:

  ```rust
  pub struct GitHubProvider {
      client_id: String,
      client_secret: Option<SecretString>,
      scopes: Vec<String>,          // default: ["read:user", "user:email"]
      redirect: RedirectMode,
      pending: Mutex<Option<PendingRedirect>>,
  }
  ```

  `id() = "github"`, `refresh_policy() = RefreshPolicy::None` (default
  non-expiring tokens, survey B2). `start_flow`: `BasicClient::new(
  ClientId::new(client_id)).set_auth_uri(AuthUrl::new(
  "https://github.com/login/oauth/authorize")?).set_token_uri(TokenUrl::new(
  "https://github.com/login/oauth/access_token")?)` (+ secret when
  present); redirect/loopback identical to Step 3.2; PKCE/state identical
  (`PkcePair::generate`, `random_state`, `set_pkce_challenge`); `.url().0`
  → FlowLaunch. `complete_callback`: `exchange_code(...).
  set_pkce_verifier(...)` + when no client_secret,
  `.add_extra_param("client_id", ...)` (public-client exchange), then
  **`.request_async` is unusable for the JSON response** because GitHub's
  default token response is urlencoded text and oauth2 parses JSON —
  instead set the Accept header via a one-off `reqwest` call mirroring
  what `CodeTokenRequest::prepare_request` builds: use the oauth2 request
  for form encoding and send with `http` client directly:
  `http.post(token_url).header(ACCEPT, "application/json").
  header(CONTENT_TYPE, "application/x-www-form-urlencoded").
  body(prepared_form).send().await` where `prepared_form` is built from
  the same params (`grant_type=authorization_code`, `code`, `client_id`,
  secret?, `redirect_uri`, `code_verifier`) — urlencoded via `serde_urlencoded`
  or manual `form_urlencoded` from the `url` crate (`url::form_urlencoded::
  Serializer`, url 2.5 is already a dep, `Cargo.toml:177`). Parse the JSON
  body with a local `#[derive(Deserialize)] struct GhTokenResponse
  { access_token, refresh_token: Option, expires_in: Option<i64>,
  scope: Option<String>, error: Option<String>, error_description:
  Option<String> }`; `error` present → `AuthError::ProviderDenied`.
  Profile: `GET {api}/user` with `Authorization: Bearer {access}` +
  `Accept: application/vnd.github+json` (`{api}` =
  `https://api.github.com`, overridable in the config's scopes-free way
  only via a `pub(crate)` field set by tests — tests point it at the
  fake); response `{login, name, email, avatar_url}` (pinned this
  session, Research Digest); `email == null` → `GET /user/emails` (same
  auth), array `{email, primary, verified, visibility}` — take the
  `primary == true` entry (first if none marked). `AccountInfo { id:
  Some(login), email, display_name: name.or(login), avatar_url }`.
  `refresh`: `Err(AuthError::NoRefreshToken)` (policy None — controller
  refuses before calling; defensive). `revoke`: `DELETE
  {api}/applications/{client_id}/token` with HTTP Basic
  (client_id, client_secret-or-empty) + JSON body `{access_token}` (204 =
  Ok; docs fetched this session; Basic-auth convention flagged for
  manual QA). No secret ever enters a log line or an error Display.
- Tests (fake GitHub): fake serves `POST /login/oauth/access_token`
  returning JSON only when `Accept: application/json` (else urlencoded —
  the test asserts both branches), `GET /user` with
  `{login, name: null, email: null, avatar_url}`, `GET /user/emails`
  with two entries one primary. Flow: start (authorize URL contains
  `github.com/login/oauth/authorize`, S256 challenge, state 43 chars),
  complete with a code → TokenSet with account from `/user` +
  `/user/emails` primary; `Accept` missing → provider still succeeds via
  its own header (the fake's 406 otherwise proves the header was sent);
  revoke → fake asserts DELETE method + path + Basic header, returns
  204 → Ok; token-response `{"error":"incorrect_client_credentials"}`
  → `AuthError::ProviderDenied`.
- Done-check: `cargo test -p gpui-starter --lib services::auth_social`
  passes.

## Phase 5: Orchestration (`SocialFlows`)

### Step 5.1: Entity + install

- Files: `flows.rs`, `flows.test.rs`, `mod.rs` surface fns.
- Build: `SocialFlows` fields per Architecture. `install(cx)` (internal
  `install_with(config, cx)` for tests): `runtime_and_client(cx)` →
  `AuthIo { http, runtime: Some(rt) }` (absent global → `Client::new()` +
  `None`, logged, matching plan 21's adapter fallback); store =
  `Arc::new(crate::services::auth::KeyringStore)`; load config; for each
  configured kind build the provider (`OidcProvider::google`,
  `GitHubProvider`, `OidcProvider::generic`), keep the concrete `Arc` AND
  an `Arc<dyn AuthProvider>` clone, `cx.new(|cx| AuthController::new(
  dyn_provider, store.clone(), io.clone(), cx))`; call `restore` on each;
  session_owner = first `SignedIn` in `ProviderKind` order; `cx.new` the
  `SocialFlows` entity; subscribe each controller:
  `cx.subscribe(&controller, |this, _entity, event, cx| ...)` on
  `AuthorizationRequested` → `desktop_actions::open_url(&url, cx)` (warn
  on failure) + spawn the loopback drain when the active flow is loopback
  (Step 5.2); on `FlowCompleted`/`FlowFailed`/`SignedOut` →
  `refresh_capabilities(cx)` + `sync_session(cx)`. Also `cx.observe` each
  controller → `refresh_capabilities` (state changes ride notify).
  Store the entity in a `pub(crate) struct SocialFlowsHandle(
  Entity<SocialFlows>)` global (`impl gpui_kit::Global`). Capability
  refresh (native): `auth_social` aggregate — any configured →
  `supported_enabled()` + per-provider entries: not configured →
  `CapabilityStatus::error("not configured: set client_id in
  auth-providers.json")`; configured + not SignedIn →
  `CapabilityStatus::degraded("configured — not connected")`; SignedIn →
  `supported_enabled()`; zero configured → aggregate
  `CapabilityStatus::error("no providers configured; see
  auth-providers.json in the config directory")`. Reason strings are
  English literals (plan 21 precedent, `secure_storage.rs:124`); plan 24
  localizes around them.
- Tests: `install_with` on an empty config → snapshot shows all three
  unconfigured, `auth_social` capability error-state with the reason;
  with a config (temp file via env override) → two controllers exist,
  `controller(Google, cx)` is Some, capabilities degraded "configured —
  not connected"; `start` on an unconfigured kind is a logged no-op (no
  controller, no panic).
- Done-check: `cargo test -p gpui-starter --lib services::auth_social`
  passes.

### Step 5.2: `start`, drain task, `route_callback`, session sync

- Files: same + `mod.rs`.
- Build: `start(kind, cx)`: controller must exist; cancel any previous
  active flow's controller (`update(|c, cx| c.cancel(cx))` — drops its
  pending) and abort its loopback task (`_abort`); set `active_flow`;
  `controller.update(|c, cx| c.start_sign_in(cx))`. The subscriber (5.1)
  handles browser open; for loopback flows it also grabs the concrete
  provider's pending redirect (a `pub(crate) fn take_pending_redirect()
  -> Option<(tokio::sync::oneshot::Receiver<String>,
  tokio::task::AbortHandle)>` on the concrete types — NOT on the trait;
  `SocialFlows` holds the concrete `Arc`s, which is why both maps exist)
  and spawns:
  `cx.spawn(async move |this, cx| { let joined = rt.spawn(async move {
  receiver.await }); match joined.await { Ok(raw_url) =>
  this.update(cx, |flows, cx| flows.deliver_redirect(kind, &raw_url,
  cx)), Err(_join) => /* aborted: ignore */ } })` where
  `deliver_redirect` calls the controller's `complete_callback(raw_url,
  cx)` and clears `active_flow`. Timeout arm: wrap the receiver await in
  `tokio::time::timeout(Duration::from_secs(300), ...)` (rt-side) → on
  timeout, `controller.cancel` + `active_flow = None`. Store the `Task`
  and the tokio `AbortHandle` in `active_flow` (dropping the Task on
  replace cancels the GPUI side; abort kills the tokio side).
  `route_callback(link, cx) -> bool`: `Url::parse(link)` ok, scheme ==
  `crate::routes::APP_URL_SCHEME` (`route.rs:6`), host == `"auth"`,
  first path segment `"callback"` (same shape plan 21's
  `handle_callback_link` checks), AND `active_flow.is_some()` → route to
  that controller's `complete_callback(link, cx)`, clear active_flow,
  true; else false (falls to plan 21's mock: cold links → NoPendingFlow,
  the documented behavior). `sign_out(kind, cx)`:
  `controller.update(|c, cx| c.sign_out(cx))`; if kind was session_owner
  → owner cleared, `sync_session` picks another SignedIn provider or
  sets `SignedOut`. `sync_session`: owner's `AuthState` →
  `session::set_state` mapping (`SignedIn{account_label}` 1:1).
  `snapshot(cx)` assembles `SocialAuthSnapshot` from config + controller
  states + owner.
- Tests: full loopback flow against the fake IdP through the REAL
  controller: `install_with(generic-oidc config pointing at fake)` →
  `start(Oidc)` → run_until_parked → assert `AuthorizationRequested`
  captured (subscribe before), browser-open attempted (desktop_actions
  failure is the native-host evidence — in tests `open_url` actually
  fires xdg-open; instead assert on the loopback path): drive the fake's
  authorize URL? No — simulate the browser: read the captured authorize
  URL, extract state, then hit `reqwest GET http://127.0.0.1:{port}/?
  code=fake-code&state={state}` (port discovered by re-parsing the
  provider's pending — expose `#[cfg(test)] fn pending_port(&self) ->
  Option<u16>` on the concrete provider types), then
  run_until_parked → controller state `SignedIn`, `TokenSet` in an
  injected `InMemoryStore` (test install uses the in-memory store,
  plan 21's `install_with` pattern), `session::snapshot` SignedIn with
  the fake's name, capability `auth_social_oidc` supported_enabled;
  wrong-state GET → controller state Error(StateMismatch) + store empty;
  scheme-mode flow: config `redirect: "scheme"` → captured URL contains
  `redirect_uri=gpui-starter%3A%2F%2Fauth%2Fcallback`;
  `route_callback("gpui-starter://auth/callback?code=x&state=y")` with an
  active flow → true and complete_callback ran; with no active flow →
  false; non-auth links (`gpui-starter://settings`, garbage) → false;
  `start(Google)` while Oidc flow pending → oidc controller state leaves
  SigningIn (cancel) and only one active flow remains; timeout path:
  fake scratch-runtime test with a 50 ms timeout override (`pub(crate)
  test const`) → controller cancelled.
- Done-check: `cargo test -p gpui-starter --lib services::auth_social`
  passes.

## Phase 6: Adapter + boot seams

### Step 6.0: Log redaction (seam row 11, security)

- Files: `src/platform/process/single_instance.rs`.
- Build: in BOTH `tracing::info!` call sites that log a forwarded link
  (`:296` queue-file poller, `:415` IPC forward), replace the `link` field
  with `path = link.split('?').next().unwrap_or(&link)`. The full link
  still flows to `events::emit` untouched — only the log field changes.
  Reason: this plan is the first to put a single-use authorization code in
  that query; 21 flagged the pattern and 22 recorded the tripwire.
- Tests: extend this file's existing test (or add one) asserting the
  sanitize helper's behavior on `gpui-starter://auth/callback?code=x` →
  `gpui-starter://auth/callback`; plain links unchanged.
- Done-check: `grep -n "received forwarded" src/platform/process/
  single_instance.rs` shows `path =` in both sites and no bare `link,`
  field; `cargo test -p gpui-starter --lib platform` passes.

### Step 6.1: `auth.rs` guard, capability move, init line

- Files: `src/services/auth.rs` (plan 21 file), `src/app/init.rs`,
  wasm reason update.
- Build: per Seam Touches rows 5-7. In `handle_callback_link`, insert the
  cfg'd guard as the first statement. In `install`'s capability block,
  remove the native `auth_social` entry (plan 21 Phase 7.2 wrote
  `supported_enabled` natively — that line moves out); the wasm arm keeps
  an `auth_social` entry with the final reason: "social login needs the
  native browser + loopback/scheme redirect path; a web build would use an
  https redirect + boot query parse (not implemented)". Add
  `crate::services::auth_social::install(cx);` in `init.rs` directly after
  `crate::services::auth::install(cx);` (plan 21's seam row 7 position —
  after `storage::initialize`, i.e. inside/after the runtime-services
  block per wherever plan 21 landed it; grep `auth::install` to find the
  exact line, append after).
- Tests: existing plan 21 adapter tests still pass (mock flow unaffected —
  `handle_callback_link` falls through to the mock when social returns
  false: unconfigured social always returns false).
- Done-check: `cargo test -p gpui-starter --lib` passes;
  `cargo clippy --workspace --all-features` clean.

## Phase 7: Gates + manual QA

### Step 7.1: Full gates

- Files: none (verification only).
- Build: run every command in Verification.
- Tests: as listed there.
- Done-check: all gates green; manual checklist executed or explicitly
  waived with notes.

### Step 7.2: Wasm gate

- Files: none.
- Build: `just wasm-check` — the module is cfg'd out; the only wasm-facing
  edits are the cfg'd guard absence and the capability reason string in
  `auth.rs`'s wasm arm.
- Tests: none (check only, per `justfile:24-25`).
- Done-check: `just wasm-check` passes.

## Platform & Wasm Gating

- Native-only dependency table: `oauth2` and `openidconnect` under
  `[target.'cfg(not(target_family = "wasm"))'.dependencies]`
  (`Cargo.toml:310`). The whole `src/services/auth_social/` module is
  declared `#[cfg(not(target_family = "wasm"))]` in `src/services/mod.rs`;
  the lib re-export carries the same cfg (`src/lib.rs:47-49` pattern).
  `src/services/auth.rs`'s new guard line is itself cfg'd so the adapter
  compiles on both targets.
- Wasm posture, explicit, not silent: the `auth_social` capability entry is
  set by the adapter's wasm arm (`CapabilityStatus` struct literal with all
  five pub fields, `src/runtime/capabilities.rs:6-13`):
  `supported: false, enabled: true, degraded: true, reason: Some("social
  login needs the native browser + loopback/scheme redirect path; a web
  build would use an https redirect + boot query parse (not implemented)"),
  last_error: None`. What a web build WOULD need (documented in the module
  docs, not implemented): authorize URL redirecting to the app's own https
  origin, `code`/`state` captured from `location.search` at boot (net-new:
  the wasm router reads `location.hash` only, `_survey/auth-ecosystem.md`
  Part C), completion through the same `complete_callback` seam, and a
  storage decision (in-memory vs localStorage — Part C storage tradeoff).
  `cx.open_url` exists on web (`gpui-pre-web-0.3.9/src/platform.rs:459-462`
  via survey) so the launch half is portable; the capture half is not.
- Capability-registry entries: `auth_social` (aggregate),
  `auth_social_google`, `auth_social_github`, `auth_social_oidc` — set in
  `SocialFlows::refresh_capabilities` natively, in `auth.rs`'s wasm arm for
  the aggregate only. Not in `DEFAULT_ENABLED_CAPABILITIES` (that list is
  boot-time always-on services, plan 21 precedent).
- Loopback mode binds `127.0.0.1` only, ephemeral port, single-shot,
  300 s timeout, port closed on capture — RFC 8252 §7.3 hygiene
  (rfc8252.txt:598-616). Scheme mode depends on OS registration: Windows
  HKCU (boot registration, `init.rs:78-89`), Linux packaged installs
  (`resources/com.gpui-starter.app.desktop` MimeType line), macOS bundles
  need `CFBundleURLTypes` (unverified — 0.3.9 macOS sources absent from
  this host; loopback is the default everywhere).
- Clocks: the module uses `chrono::Utc::now()` (native-only code; the
  seams §6 ban targets wasm boot/render panics — `chrono`'s default clock
  is correct natively, and wasm compiles neither this module nor the call).
  `std::time::Duration` for timeouts is not a clock read.
- Tokio: the loopback listener + receiver await run on the shared auth
  runtime from `runtime_and_client(cx)` (`tokio_runtime.rs:80-83`); the
  crate's `net` feature is already enabled natively
  (`Cargo.toml:350-353`). GPUI-side awaits use `cx.spawn` +
  JoinHandle (updater precedent, `src/services/updater/check.rs:72-77`).
  OS threads never touch `cx` (the listener task lives on tokio, forwards
  through the oneshot into a `cx.spawn` drain — seams §3 rule).

## i18n

No catalog keys. The module ships no UI; every user-visible surface it
touches already exists: capability reason strings are English literals
inside the adapter/module (repo precedent: `secure_storage.rs:124`
`WASM_UNAVAILABLE`, plan 21's capability reasons), and the loopback
success page is browser-side copy outside the app (literal English, Zed
pattern-B precedent, `_survey/auth-zed.md` §2). Plan 24 owns the `auth_*`
catalog keys for the account UI and must keep both locale files in
lockstep (`tests/qa_localization.rs:188-198`); the diagnostics page
renders capability `reason` strings verbatim today (rows.rs:387-396) —
localizing that surface is 24's decision, and this plan's reasons are
written as full sentences so they wrap cleanly when that happens.

| key | en | zh-CN |
| --- | --- | --- |
| — (none) | — | — |

## Design Compliance

Brief (non-UI plan; full weight lands in 24). No layout, density, token,
motion, or component decisions are introduced. The surfaces this plan
feeds: (1) the diagnostics capability rows — plain `CapabilityStatus`
data through the existing generic renderer (rows.rs:387-396), where the
guides' "status is not communicated by color alone" rule is inherited from
the page's existing row pattern, and this plan's contribution is honest
reason text for every state (not-configured / configured-not-connected /
connected) rather than a bare boolean; (2) the session row's signed-in
label — `account_label` from plan 21's fallback chain (display_name →
email → provider display name), one line, no truncation decision needed.
Interface copy rule observed: all reason strings are developer-facing
English sentences; plan 24 must wrap, not re-display, `AuthError`/reason
text (same note plan 21's Design Compliance records). The loopback browser
page is a terminal-state message ("close this tab"), which the design
guides' feedback section would classify as an acknowledgement, not an
in-app surface — exempt from the token system by being outside the app.

## Verification

Repo gates, in order (AGENTS.md):

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-features
cargo test --workspace --all-features
just wasm-check
```

`--workspace` includes `gpui-auth` (Phase 3.1 amendment). Stage the
regenerated `Cargo.lock` with the manifest edits (Seam Touches rows 1-2).

Feature-specific automated coverage:

- `cargo test -p gpui-starter --lib services::auth_social`: config
  parsing/defaults/env-override/perms lint; loopback capture + favicon
  tolerance + replacement abort + distinct ports; OIDC flow against the
  in-process fake IdP (authorize URL shape incl. S256 + 43-char state,
  exchange + userinfo TokenSet incl. avatar_url, refresh policy per
  constructor); GitHub flow against the fake (Accept: application/json
  exchange, /user + /user/emails profile with null email, revoke DELETE,
  ProviderDenied on error body); orchestration (install states, start
  replace-cancel, route_callback truth table, full loopback round trip
  through the real controller + InMemoryStore + session mapping,
  StateMismatch, timeout cancel).
- `cargo test -p gpui-auth --all-features`: AccountInfo serde
  backward-compat (field-absent JSON parses).
- `cargo test -p gpui-starter --lib`: plan 21's adapter tests still pass
  (mock fallback routing unaffected).

Manual QA (Linux dev box; needs throwaway provider registrations —
documented, not scripted):

1. Create a GitHub OAuth App (Settings → Developer settings): callback
   `http://127.0.0.1:1/callback` (any port — GitHub loopback any-port);
   create a Google OAuth Desktop client. Write
   `~/.config/gpui-starter/auth-providers.json` (exact path from the
   diagnostics/log line) with both client ids; `chmod 600` it.
2. `cargo run`; Diagnostics: `auth_social` error-state with the
   no-providers reason BEFORE step 1's file exists; after: per-provider
   rows degraded "configured — not connected".
3. Trigger `start(Google)` (temporary debug command or plan 24 UI; during
   23's QA, a `#[cfg(debug_assertions)]` test action or the existing
   command palette is acceptable — QA may also drive it via
   `cargo test -- --ignored` harness marked `#[ignore]`): browser opens at
   Google, consent, tab shows the capture page, app reaches SignedIn,
   session row shows the account label, capability flips to
   supported_enabled, keyring row exists under service
   `gpui-starter.auth` key `google` (verify via
   `secret-tool lookup service gpui-starter.auth key google` style probe
   or the app's secure_storage path). Repeat for GitHub (email fallback
   path if the test account hides email).
4. Wrong-state probe: while a flow is pending, `xdg-open
   'gpui-starter://auth/callback?code=x&state=bogus'` → log shows
   StateMismatch, no navigation, second-instance forwarding still works
   (launch a second app instance with the URL as argv).
5. Scheme mode: set `"redirect": "scheme"` on the GitHub provider
   (GitHub accepts custom-scheme redirects), repeat sign-in → link
   arrives via deep-link path (window focuses; on Windows this is the
   default path — QA there if a box exists, else record Linux-packaged
   or skip with note).
6. Sign-out: token revoked (GitHub app's token page shows it gone or the
   API 401s), keyring row deleted, session SignedOut, capability back to
   degraded.
7. `just wasm-check` green; wasm build if available: diagnostics shows
   `auth_social` unsupported with the final reason; no other auth rows
   changed.
8. Logs audit: `grep -R "mock-access-\|access_token\|code_verifier" \
   target/debug/*.log ~/.local/share/gpui-starter/logs/` (or the
   diagnostics "open logs folder" path) returns no token/verifier
   values — codes appear only in the two pre-existing single-instance
   forward log lines (plan 21's documented exposure, unchanged here).

Acceptance criteria: four gates green; the three automated suites pass;
manual items 2, 3, 4, 8 confirmed (5-7 or waived with notes, 7 is the
hard gate); diff shows no `AppConfig` field, no secret in any committed
file (`git grep -l "client_secret" -- ':!docs/plans'` → only the config
schema source), `auth-providers.json` absent from the repo tree.

## Risks & Open Questions

Ranked risks:

1. **oauth2 5.0 / openidconnect 4.0.1 never compiled in this repo.** API
   claims are tarball-verified (path:line in Research Digest) but the
   feature-graph resolution against the lock (reqwest union, thiserror 1/2
   coexistence, rsa/p256 native-only) proves out only at Step 1.1's
   `cargo check`. Fallback levers if resolution fails: pin
   `oauth2 = "=5.0.0"`, drop `rustls-tls` from the oauth2 line (the app's
   reqwest already carries it — feature union), or exchange GitHub codes
   with plain reqwest (the form body is 6 params) and keep openidconnect
   only.
2. **AuthType/default body encoding for public clients.** Google's
   public-client exchange must carry `client_id` in the body; oauth2's
   `AuthType` default and whether `from_provider_metadata` clients include
   it without a secret is compile-time behavior, not fully readable from
   the tarball (auth-type plumbing spans client.rs/token/mod.rs). Mitigation:
   the plan sends `client_id` explicitly via `add_extra_param` on both
   providers when no secret is configured, which is legal under both
   auth types; Step 3.2's done-check is the fake-IdP exchange test, which
   fails loudly if the server never sees the id.
3. **GitHub specifics from one docs fetch.** Loopback any-port, DELETE
   /applications/{client_id}/token Basic auth, and the expiring-token
   refresh shape are documented-but-unexercised (survey Open Questions +
   this session's fetches). Mitigation: manual QA items 3/6 against a
   throwaway app before merge; `RefreshPolicy::None` means the unverified
   refresh shape is unreachable code.
4. **Contract amendment ripple.** `AccountInfo.avatar_url` reflows plan
   21's block and breaks plan 22's exhaustive literal at 22.md:630-632
   until it gains `avatar_url: None` (Seam Touches rows 9-10; both plans
   are on disk, verified this session). Plan 24 reads the field through
   accessors only (its `AuthUiSnapshot.account_email` derives from
   `TokenSet.account`, 24.md:238-239), so it needs no edit. Whoever lands
   second owns the one-line sync. No other signature changes.
5. **Discovery latency on every flow.** `start_flow` does discovery +
   JWKS fetch before the browser opens (hundreds of ms typical). Accepted:
   no cache (no speculative abstraction); the SigningIn state covers the
   wait; a discovery cache is a one-field follow-up if it bothers anyone.
6. **Client secret at rest in a config file.** `auth-providers.json` is
   plaintext in the config dir (0600 lint, never committed, never
   app-written). Tradeoff vs keyring: bootstrap problem — the secret is
   an input to auth, not a product of it; RFC 8252 §8.5 caps the stakes
   for native-app secrets anyway. Documented in the module docs; moving
   secrets into `secure_storage` is a 24-lifecycle option.
7. **Plan 21 not yet implemented.** Every seam row touching
   `src/services/auth.rs` / `crates/gpui-auth` presumes plan 21's tree.
   If 21 lands with deviations from its Interface Contract, re-read both
   before executing Phase 3 onward (plan 21's own risk 3 rule).

Open questions (each with what was tried):

- **Google desktop custom-scheme support** — the survey's Google page was
  served non-English and the desktop custom-scheme status stayed ambiguous
  (survey Open Questions). Tried: survey Part B2 + loopback-migration
  cross-check. Resolution here: loopback is the default and Google's
  documented desktop redirect; scheme mode is documented
  GitHub-or-generic-only in the config docs. Re-read the English page
  before ever shipping scheme mode with Google.
- **GitHub DELETE-token Basic auth** — docs page doesn't spell out the
  auth scheme (fetch this session); the endpoint family's convention is
  Basic client_id:client_secret. Tried: docs fetch. Confirmation is
  manual QA item 6; failure mode is a 401/422 on a best-effort call
  (sign-out still deletes the local token).
- **Google revoke endpoint** (`https://oauth2.googleapis.com/revoke`) —
  from Google's OAuth docs knowledge, not fetched this session (the
  survey's Google table has no revoke row). Manual QA item 6 confirms;
  best-effort failure is acceptable by contract.
- **macOS 0.3.9 platform sources absent from this host** (survey Open
  Questions; directory listing). Consequence confined to scheme-mode
  claims; loopback default avoids depending on any macOS behavior. QA on
  a Mac before advertising scheme mode there.
- **`route_callback` cold-link UX**: a scheme-mode callback arriving with
  no pending flow falls through to plan 21's mock controller and logs
  NoPendingFlow. Tried: routing table alternatives (state→controller
  index) rejected — the state values are secrets inside controllers by
  design, and cold links carry no provider identity. Documented behavior;
  24 can add a toast if users hit it.
- **No cargo gate was run during planning** (doc-only scope, matching the
  surveys, plans 01-20, and plan 21). Every Verification command is for
  the implementer; first evidence points are Step 1.1 and Step 7.2.
- **Sibling-plan staleness, not fixed here (scope is this file)**: plan
  24's Dependencies section claims "Plans 22/23 do not exist yet"
  (24.md:46-47) and its contract notes "Plans 22/23 consume ... when they
  land" (24.md:251-252) — both predate 22/23 landing on disk (directory
  listing this session). Tried: nothing to try beyond the listing; the
  sentences are 24's own. Whoever executes or next edits 24 should drop
  them; 23's coordination with 24 lives in this plan's risk 4.
- **Em-dash density**: this plan is the family ceiling (~115 lines carry
  an em-dash, vs 56/32/75 in 21/22/24 by the reviewer's count). Reviewer
  note, not a correctness item. Skipped a wholesale rewrite: mechanical
  replacement would churn every section without changing any claim;
  trimmed nothing beyond the sentences already touched in this revision.
  Revisit if the doc is edited again.
