# Auth ecosystem survey: verified crate + protocol facts for plans 21–23

Inputs for the auth feature plans (21 foundation auth crate, 22 email/password, 23
social login via OIDC/OAuth). None of the three plans exist on disk yet. Every
gpui/gpui-kit claim below was verified against the pinned crate sources
(gpui-pre 0.3.9 / gpui-kit 0.7.3) in the local registry this session; registry
crates were located with `ls ~/.cargo/registry/src/*/ | grep -E 'gpui-pre-0.3.9|gpui-kit-0.7.3'`
and read directly. Third-party crate facts come from the crates.io API and
docs.rs feature pages (`curl -A "gpui-starter-survey" https://crates.io/api/v1/crates/<name>[/<version>[/dependencies]]`,
WebFetch on docs.rs), all fetched 2026-10-12. Protocol facts cite RFC text
downloaded from rfc-editor.org this session and provider docs pages fetched
2026-10-12. Doc-only survey: no cargo gate was run (repo convention, see
Verification). Anything not verifiable is under Open Questions, not guessed.

## Path key

| Short | Path |
| --- | --- |
| `GPUI` | `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-pre-0.3.9/src` |
| `LINUX` | `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-pre-linux-0.3.9/src` |
| `WEB` | `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-pre-web-0.3.9` |
| `WEB38` | `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-pre-web-0.3.9/src` |
| `MAC38` | `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-pre-macos-0.3.8/src` |
| `WIN38` | `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-pre-windows-0.3.8/src` |
| `KEY3` | `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/keyring-3.6.3` |
| `REPO` | `/home/thragg/fo/gpui-starter` |
| `CARGO` | `REPO/Cargo.toml` |
| `SECURE` | `REPO/src/services/secure_storage.rs` |
| `ROUTER` | `REPO/src/platform/web/router.rs` |
| `DEEPLINK` | `REPO/src/platform/deep_link_registration.rs` |

Pins: `gpui = { package = "gpui-pre", version = "0.3.9" }`, `gpui-kit = "0.7.3"`
(`CARGO:20-21`). The older `_survey/ecosystem.md` cites 0.3.8/0.7.1 for plans
01–20; those crates remain in the registry. This survey uses 0.3.9/0.7.3 only.
`WEB` (gpui-pre-web-0.3.9) exists on disk; `gpui-pre-macos`/`-windows`/`-apple`
do NOT exist at 0.3.9 (Linux host never fetched them), so macOS/Windows
platform behavior is cited from their 0.3.8 sources and flagged in Open
Questions.

## Part A — crates

### A1. Auth-adjacent deps already in the workspace (do not re-add)

Read from `CARGO` this session. Direct deps:

| Dep | Version / features | Line | Auth relevance |
| --- | --- | --- | --- |
| `url` | 2.5.8 | `CARGO:177` | redirect/authorization URL construction |
| `base64` | 0.22 | `CARGO:182` | base64url for PKCE verifier, JWT b64 parts, TOTP secrets |
| `sha2` | 0.10 | `CARGO:183` | S256 code challenge digest; token hashing |
| `reqwest` | 0.12.24, rustls-tls, json, cookies, multipart, stream, default-features off | `CARGO:187-193` | all token/userinfo HTTP; same reqwest major oauth2 5.0 defaults to |
| `tokio` | base rt+sync+time; native adds net/io-util/rt-multi-thread | `CARGO:33,353` | async runtime for token exchange |
| `serde` / `serde_json` | 1 / 1 | `CARGO:25-26` | TokenSet, discovery, provider payloads |
| `thiserror` | 2.0.18 | `CARGO:174` | auth error enums |
| `uuid` | 1.23.1, v4+serde | `CARGO:175` | session/request IDs |
| `chrono` | 0.4.44 (+wasmbind on wasm) | `CARGO:176,371` | token expiry timestamps |
| `ed25519-dalek` | 2, std | `CARGO:181` | already pulled by openidconnect 4 too (`ed25519-dalek ^2`) |
| `keyring` | 3.6.3, features apple-native + windows-native | `CARGO:337` | OS credential store (see A3 for the Linux gap) |
| `rusqlite` | 0.32.1 bundled | `CARGO:338` | local persistence if tokens go to SQLite |
| `koruma` / `koruma-collection` | 0.9, email+phone-number+url features | `CARGO:52-53` | email validation for plan 22 forms |
| `web-sys` (wasm) | =0.3.102, features include `Location`, `History`, `Storage`, `Navigator` | `CARGO:388-414` | browser redirect + storage surface (Part C) |
| `js-sys` / `wasm-bindgen` | =0.3.102 / =0.2.125 | `CARGO:417,360` | JS interop on web |

NOT direct deps: `rand`, `ring`, `zeroize`, `oauth2`, `openidconnect`,
`argon2`, `totp-rs`, `hmac`, any Supabase/Auth0/Firebase crate. Transitive
copies already in `Cargo.lock` (grep `name = "..."` this session): `rand`
0.8.6 + 0.9.4, `ring` 0.17.14, `zeroize` 1.9.0, `getrandom` 0.2.17 / 0.3.4 /
0.4.3, `sha2` 0.10.9 + 0.11.0, `subtle` 2.6.1. (`argon2-0.5.3` sits in the
registry cache but is not in the lock; registry presence ≠ dependency.)

### A2. Candidate crates (versions + metadata from crates.io API, 2026-10-12)

| Crate | Version (date) | License | Fit | Risk | Wasm |
| --- | --- | --- | --- | --- | --- |
| `oauth2` | 5.0.0 (2025-01-21) | MIT OR Apache-2.0 | core OAuth2 flows; default features are reqwest 0.12 + rustls-tls, matching the app's reqwest 0.12.24 line; PKCE + state types built in | API reworked in 5.0: async token calls go through the `AsyncHttpClient` trait (auto-impl for `Fn(HttpRequest) -> Future`) rather than a 4.x-style convenience fn; exact re-export names must be confirmed at compile time | untested. Deps are pure Rust (getrandom 0.2, rand 0.8, chrono, sha2 0.10, hmac, serde). PKCE/state randomness rides getrandom 0.2, which needs its `js` feature on wasm32-unknown-unknown — see Open Questions |
| `openidconnect` | 4.0.1 (2025-07-06) | MIT | OIDC on top of oauth2 ^5.0.0: discovery, id_token validation (ed25519-dalek 2, rsa 0.9, p256/p384), userinfo. Feature flags forward to oauth2; default = async reqwest + rustls | heavier dep tree; requires an IdP with real OIDC discovery (Google yes, GitHub no — Part B) | untested. Pure-Rust deps; same getrandom/rand caveat as oauth2 |
| `argon2` | 0.6.0 (2026-08-27) | MIT OR Apache-2.0 | password hashing (RFC 9106) for a self-hosted backend in plan 22; RustCrypto `password-hash` traits, opt `zeroize` | client-side hashing of a password that is then sent over TLS is wrong design — argon2 belongs server-side; also multi-100ms KDF cost is painful under wasm | expected OK: deps base64ct, blake2, cpufeatures, password-hash are pure Rust; not compiled here |
| `totp-rs` | 6.0.0 (2026-08-06) | MIT | RFC 6238 TOTP for plan 22 MFA; features `otpauth` (otpauth:// URLs), `qr` (qrcodegen-image), `gen_secret` (rand 0.10), `serde`, `zeroize`, `steam`; default = std+migration | pulls sha1 0.11 + sha2 0.11 (app pins sha2 0.10 — two majors coexist); `std` default enables rand `thread_rng`, which has no wasm story | expected OK for the math with `default-features = false` + needed features; secret generation on wasm should use a web-random source; not compiled here |
| `keyring` (upgrade path) | 4.2.0 (2026-08-29) | MIT OR Apache-2.0 | 4.x splits stores into crates: default = apple-native-keyring-store + windows-native-keyring-store + zbus-secret-service-keyring-store | feature names change from 3.x; app currently pins 3.6.3 (`CARGO:337`) — upgrade is optional, not required by auth plans | n/a: keyring is native-only; app cfg-fences it out of wasm already (`SECURE:39-66`) |
| `zeroize` | 1.9.1 (2026-10-06) | Apache-2.0 OR MIT | zeroing token/secret buffers before drop; already transitive (1.9.0 in lock), adding it direct only upgrades semantics | minimal; derive/`Zeroizing` wrapper discipline needed on TokenSet | pure Rust, no platform code; not compiled here |
| `supabase_rs` | 0.8.0 (2026-10-02) | MIT | NOT an auth client: docs.rs module list covers PostgREST select/insert/update/delete only; no sign_in/sign_up/session API | reqwest ^0.13.1 dep (newer major than the app's 0.12) would duplicate the HTTP stack; Supabase auth (GoTrue) would be hand-rolled REST anyway | unknown; docs.rs lists only x86_64-unknown-linux-gnu; skip unless plan 22 wants its PostgREST half |
| `auth0` | 0.3.0 (2020-10-08) | MIT | dead: last release 2020, repository field empty (crates.io API) | abandonware; Auth0 tenants are standard OIDC, so `openidconnect` covers them | n/a, do not use |
| `firebase-auth` | 0.5.1 (2025-06-23) | MIT | server-side Firebase ID-token verifier (jsonwebtoken + Google public keys, axum/actix extractors) per docs.rs | wrong shape for a desktop client: no email/password sign-in API; client sign-in against Firebase = Identity Toolkit REST over reqwest | server crate; not relevant client-side |

### A3. Existing storage layer (what plans must build on, not duplicate)

`src/services/secure_storage.rs` wraps keyring behind `set_secret` /
`get_secret` / `delete_secret` with a `SecureStorageSnapshot` global and a
`secure_storage` capability (`SECURE:68-99`). Native paths use
`keyring::Entry::new` + `set_password` / `get_password` / `delete_credential`
(`SECURE:127-195`); `NoEntry` maps to `Ok(None)` (`SECURE:169-172`). Wasm
stubs hard-fail every operation (`SECURE:197-234`).

Linux gap: `keyring` 3.6.3 has no default features (`grep 'default' KEY3/Cargo.toml`
matches only `default-target`), the workspace enables only `apple-native` +
`windows-native` (`CARGO:337`), so Linux compiles to the mock store —
"fallback to mock if neither keyutils nor secret service is available"
(`KEY3/src/lib.rs:255-262`). Writes vanish, reads miss. Fix options for plan 21:
`linux-native` (keyutils, session-scoped), `sync-secret-service` /
`async-secret-service` (Secret Service/persistence), or the combined
`linux-native-async-persistent` (`KEY3/Cargo.toml` features block from line 56).
Note gpui's own platform trait also has `write_credentials` /
`read_credentials` / `delete_credentials` backed by oo7 on Linux
(`GPUI/src/platform.rs:597-599`, `LINUX/linux/platform.rs:1028-1070`) — a
second, unused credential API. Plans should stay on the app's `secure_storage`
(keyring) rather than fork onto the platform one.

## Part B — protocol facts for desktop apps

### B1. RFC 8252 (OAuth 2.0 for Native Apps, BCP 212)

Text downloaded from rfc-editor.org this session (`rfc8252.txt`, 1179 lines);
cites are file lines.

| Topic | Requirement | Cite |
| --- | --- | --- |
| PKCE support | for native apps using the auth-code flow, "authorization servers MUST support PKCE"; ASs "SHOULD reject authorization requests from native apps that don't use PKCE" | rfc8252.txt:360-361, 577-580 |
| PKCE method | clients capable of S256 MUST use S256; plain only if S256 impossible | RFC 7636 §4.2 (rfc7636.txt:433-438) |
| Loopback redirect | `http://127.0.0.1:{port}/{path}` or `http://[::1]:{port}/...`, random ephemeral port; AS "MUST allow any port" at request time; clients should bind both v4 and v6 | rfc8252.txt:511-529 |
| Loopback hygiene | plain http acceptable (never leaves device); open the port only for the duration of the request; listen on loopback interface only; `localhost` NOT RECOMMENDED vs the IP literal | rfc8252.txt:598-616 |
| Custom scheme | MUST be reverse-domain under the app publisher's control (`com.example.app`), not bare `myapp` | rfc8252.txt:411-435 |
| Client secret | secrets statically embedded in distributed apps are not confidential; an AS accepting a secret from a native app MUST treat that client as public and not accept the secret as identity proof | rfc8252.txt:665-667, 684-686 |
| CSRF / state | RECOMMENDED: high-entropy random `state`, reject responses whose state matches no pending request | rfc8252.txt:755-763 |
| Mix-up defense | REQUIRED: unique redirect URI per authorization server; store the redirect URI with the session (state) data and exact-match it on response | rfc8252.txt:768-779 |
| Embedded user-agents | native apps MUST NOT use embedded user-agents (webviews) for OAuth | rfc8252.txt:802-806 |

Practical read for plan 23: system browser + loopback redirect is the
first-choice flow (all three desktop OSes, no scheme registration);
`cx.open_url` is the launch mechanism (Part C).

### B2. Provider specifics (docs pages fetched 2026-10-12)

Google (`developers.google.com/identity/protocols/oauth2/native-app` and
`.../loopback-migration`, `.../web-server#offline`, `.../oauth2`):

| Topic | Fact |
| --- | --- |
| Redirect for Desktop client type | loopback `http://127.0.0.1:PORT` / `http://[::1]:PORT`, random port; recommended for macOS/Linux/Windows desktop apps; `localhost` accepted but can trip client firewalls; loopback deprecation applies to iOS/Android/Chrome client types only, desktop continues |
| Custom scheme | documented for installed apps (reverse-DNS, must contain a dot), but "no longer supported" for Android and Chrome clients due to app-impersonation risk; desktop availability ambiguous — treat loopback as the safe default |
| client_secret | optional for installed apps at code exchange and refresh (installed apps are treated as unable to keep secrets) |
| redirect_uri matching | must match a registered URI exactly, else `redirect_uri_mismatch`; out-of-band copy/pose flow is dead |
| PKCE / DPoP | S256 supported and recommended; DPoP optional |
| Offline access | `access_type=offline` on the authorization request; refresh token returned only on FIRST consent; `prompt=consent` forces re-issue |
| Refresh token lifetime | no rotation on use ("continue to use them as long as they remain valid"); dies on user revocation, 6 months unused, password change when Gmail scopes present, admin policy, or the 100-refresh-tokens-per-account-per-client limit (oldest silently invalidated); Testing-status projects: 7-day expiry |
| OIDC discovery | live check this session: `GET https://accounts.google.com/.well-known/openid-configuration` → 200 with issuer/auth/token/userinfo endpoints (curl, 2026-10-12); `openidconnect` crate usable as-is |

GitHub OAuth Apps (`docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps`):

| Topic | Fact |
| --- | --- |
| Endpoints | authorize `GET https://github.com/login/oauth/authorize`; token `POST https://github.com/login/oauth/access_token`. Token response is urlencoded text by default; send `Accept: application/json` for JSON. Live probe: bogus client_id → HTTP 404 `{"error":"Not Found"}` — 404 means unknown client, not wrong endpoint |
| OIDC | none: `GET https://github.com/.well-known/openid-configuration` → 404 (curl, 2026-10-12). No discovery, no id_token, so `openidconnect` is unusable; use `oauth2` directly + GitHub REST for identity (`GET https://api.github.com/user` exists — unauthenticated probe returned 401 this session — but its authed response shape was not verified; plan 23 must pin it) |
| Callback URL | exact match by default; wildcard matching opt-in (host-minus-subdomains + path prefix fixed, port exact). Loopback: `http://127.0.0.1:1234/path` accepted even when the port differs from the registered callback (RFC 8252 §7.3 behavior). Non-HTTP (custom-scheme) redirect URIs are accepted and trigger the account picker |
| Token lifetime | default: non-expiring, no refresh token. Opt-in "expiring tokens": access token 8 h + refresh token, enabled per-app or per-sign-in via `offline_access` scope; refresh expires after 6 months unused; refreshing ROTATES both tokens; stale refresh → `bad_refresh_token`. Cap: 10 tokens per user/app/scope. The premise "GitHub has no refresh tokens" is outdated — but the default remains no expiry, no rotation |
| state / PKCE | both "strongly recommended"; PKCE `S256` only (plain unsupported) |
| Scopes | space-delimited; omitted `scope` reuses previously granted scopes without re-prompt |
| Device flow | exists, must be enabled in app settings; polling honors `interval`, user codes expire in 15 min |

Refresh-rotation background (RFC 9700, OAuth Security BCP, text downloaded
this session): rotation = AS issues a new refresh token on every refresh and
invalidates the previous, so replay by an attacker forces revocation of the
whole grant (rfc9700.txt:1990-1999); ASs "MUST utilize one of these methods to
detect refresh token replay ... for public clients" (rfc9700.txt:1983-1985).
TokenSet model must therefore treat refresh-token presence, expiry, and
rotation-vs-reuse as per-provider properties: Google = reuse-until-dead,
GitHub default = none, OIDC-generic = assume rotation.

## Part C — wasm posture

What a browser build does differently (facts, then gaps):

- No loopback listener, no OS custom-scheme registration. The only redirect
  target is an https URL of the web app itself; the code/state arrive as query
  params parsed at boot. The app currently routes on `location.hash` only
  (`ROUTER:1`, `current_hash` + `window.location()` at `ROUTER:145-165`) —
  query-string code capture is net-new work for plan 23.
- `cx.open_url` exists on web and opens a new browser context:
  `WEB38/platform.rs:459-462` (`browser_window.open_with_url`). App-level API:
  `GPUI/src/app.rs:1809-1811` (`App::open_url`).
- Inbound URLs: `on_open_urls` on web only stores the callback
  (`WEB38/platform.rs:136,465-466`) and nothing in the web crate ever fires it
  (grep over `WEB/src` — sole references are the two assignments). So no
  scheme-launch story; everything must come through page navigation.
- `register_url_scheme` on web is a no-op returning `Task::ready(Ok(()))`
  (`WEB38/platform.rs:468-470`); on Linux it errors "unimplemented"
  (`LINUX/linux/platform.rs:1102-1104`); on Windows 0.3.8 also "unimplemented"
  (`WIN38/platform.rs:1113-1115`); on macOS 0.3.8 it calls NSWorkspace
  setDefaultApplication, macOS 12+ only (`MAC38/platform.rs:841-849`). Net:
  a custom-scheme strategy cannot rely on this API outside macOS.
- Storage: `SECURE` wasm stubs fail closed (`SECURE:197-234`). Options already
  wired in the web build: web-sys `Storage` feature is enabled
  (`CARGO:408`) for localStorage/sessionStorage (XSS-readable — a real
  tradeoff for refresh tokens), plus the existing OPFS/SQLite-worker path
  (rationale comment at `CARGO:383-385`). Memory-only TokenSet with
  re-auth-on-reload is the zero-risk default.
- gpui-pre-web-0.3.9 itself enables web-sys features including `Location`,
  `Storage`, `Navigator`, `Window` (`WEB/Cargo.toml`
  `[target.'cfg(target_family = "wasm")'.dependencies.web-sys]`, features list
  starting line 125), but its own source never touches `window.location` —
  grep for location over `WEB/src` matches only the word "allocations" in a
  test name (`WEB/src/run_replacements.rs:490`). The Location feature is dep
  union, not an API the kit exposes. Browser location work is the app's own
  (`ROUTER`, plus hash deep links noted at `CARGO:372-374`).
- reqwest 0.12.24 is an unconditional dependency (`CARGO:187`) and the app's
  wasm build ships it (`just wasm-check` is the standing gate; not run this
  session), so token-endpoint HTTP on wasm reuses the existing client.
- oauth2/openidconnect/totp-rs/argon2 have no wasm compile evidence in this
  repo. Chief hazard: oauth2 5.0's `PkceCodeChallenge::new_random_sha256()`
  and state generation pull rand 0.8 / getrandom 0.2 (crates.io dependency
  list, 2026-10-12), and getrandom 0.2 on wasm32-unknown-unknown needs its
  `js` feature. Fallback: generate verifier/state with the web Crypto RNG and
  construct the challenge via the crate's non-random constructors. Unverified —
  see Open Questions.

## Part D — decision inputs per plan

| Plan | Read first | Key decisions this survey settles / frames |
| --- | --- | --- |
| 21 (foundation auth crate) | A1 (add nothing duplicate), A3 (storage layer + Linux keyring gap), Part C storage bullets, B2 TokenSet paragraph | TokenSet must model: optional refresh token, provider identity (mix-up defense, `rfc8252.txt:768-779`), expires_at, and per-provider rotation policy. Secrets through `SECURE` (extend, do not bypass). Linux store feature choice (keyutils vs secret-service) is the one storage decision to make. Add `zeroize`; keep keyring at 3.6.3 unless the store-feature fix is easier at 4.x. Provide a wasm TokenSet store trait with memory + Storage backends |
| 22 (email/password) | A1 (koruma already validates emails), A2 rows argon2 / totp-rs / supabase_rs / firebase-auth, B2 refresh table (hosted backends) | Backend shape: self-hosted (argon2 server-side; client never hashes the password it sends over TLS) vs hosted (Supabase GoTrue or Firebase Identity Toolkit via plain reqwest — both surveyed crates are the wrong tools, A2). MFA: totp-rs 6 with `otpauth` + `qr` + `gen_secret` + `zeroize` features, `default-features = false` if wasm is in scope; secret storage through `SECURE`. Duplicate-major note: totp-rs pulls sha2 0.11 alongside the app's 0.10 |
| 23 (social OIDC) | B1 (whole table), B2 both provider tables, Part C (web flow), A2 rows oauth2 / openidconnect | Stack: `openidconnect` 4.0.1 for discovery-capable IdPs (Google verified live; Auth0 tenants), `oauth2` 5.0.0 raw + REST for GitHub. Flow: system browser via `cx.open_url` (`GPUI/src/app.rs:1809`), loopback 127.0.0.1 random port as primary redirect on all desktops (B1 + B2), custom scheme only as a macOS/Windows extra (registration code already exists for Windows at `DEEPLINK:20`; Linux `register_url_scheme` errors). Mandatory: PKCE S256, high-entropy state, per-provider redirect URIs. Web build: https-page redirect + boot-time query parsing (net-new), storage per Part C |

## Open Questions

- oauth2 5.0.0, openidconnect 4.0.1, totp-rs 6.0.0, argon2 0.6.0, zeroize
  1.9.1 were never compiled this session (doc-only survey, repo convention).
  In particular the oauth2 5.0 reqwest-module convenience names (async client
  wiring) and wasm32-unknown-unknown compile (getrandom 0.2 `js` feature union)
  are unproven. First plan-21 session must run `cargo check` + `just wasm-check`
  with the candidate deps before citing any API beyond the types named on
  docs.rs.
- gpui-pre-macos / -windows / -apple 0.3.9 sources are absent from this host's
  registry (verified by directory listing). macOS/Windows platform behavior
  above cites the 0.3.8 sources. Re-verify `register_url_scheme` /
  `on_open_urls` shapes at 0.3.9 during plan 23's per-OS pass.
- GitHub loopback any-port allowance and the expiring-token refresh response
  shape come from one docs page fetch (2026-10-12); the docs did not show the
  refresh response body. Plan 23 should confirm both with a throwaway OAuth
  app before freezing TokenSet handling for GitHub.
- Firebase Identity Toolkit and Supabase GoTrue REST endpoints were not
  fetched this session (the crates turned out to be non-clients). Plan 22 must
  pin exact endpoint docs before writing the client.
- The Google native-app page was served in a non-English locale by the fetch
  tool; facts were cross-checked against the loopback-migration page and are
  consistent, but the custom-scheme-for-desktop ambiguity (B2) should be
  re-read in the English page before relying on custom schemes with Google.
- Whether the 100-refresh-token Google limit and 6-month inactivity expiry
  behave identically for Desktop client type vs web client type is not
  distinguished by the fetched docs; plan 23 should not depend on the
  distinction (handle `invalid_grant` uniformly).

## Verification (not run this session — commands for plan authors)

```sh
# crate presence / duplicate majors after adding auth deps
cargo tree -i rand && cargo tree -i getrandom && cargo tree -i reqwest
# wasm compile of the auth graph (getrandom js feature is the risk)
just wasm-check
# full gates per AGENTS.md
cargo fmt --all -- --check && cargo clippy --workspace --all-features && cargo test --workspace --all-features
# OIDC discovery probes used in this survey
curl -s https://accounts.google.com/.well-known/openid-configuration | head -c 300
curl -s -o /dev/null -w '%{http_code}\n' https://github.com/.well-known/openid-configuration
```
