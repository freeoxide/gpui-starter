# gpui-auth

Auth foundation for GPUI apps: a `TokenSet` model, a `TokenStore` trait with an
in-memory implementation, PKCE/state helpers, an `AuthProvider` trait with a
scripted mock, and an `AuthController` GPUI entity that drives flows. No UI and
no app imports — the host app adapts it in one file (`src/services/auth.rs` in
gpui-starter).

## Usage

```rust
use std::sync::Arc;
use gpui_auth::{AuthController, AuthIo, InMemoryStore, MockProvider};

let io = AuthIo {
    http: reqwest::Client::new(),
    runtime: None, // native: Some(rt) for real providers; None = direct await
};
let controller = cx.new(|cx| {
    AuthController::new(
        Arc::new(MockProvider::new()),
        Arc::new(InMemoryStore::new()),
        io,
        cx,
    )
});
controller.update(cx, |c, cx| c.start_sign_in(cx));
```

The host routes `gpui-starter://auth/callback` links to
`complete_callback(link, cx)`; everything else (state matching, exchange,
persistence, sign-out) is the controller's job.

## Email + password (plan 22)

`email.rs` adds the credential seams: `AuthBackend` (app-owned backend, every
method takes `now_unix`), `PasswordAuth` (what the controller calls), and
`EmailPasswordProvider` (provider id `"email"`, `ReuseUntilDead`, no browser
flow — `start_flow` returns `NoBrowserFlow`). The controller gains
`sign_up_with_password`, `sign_in_with_password`, `submit_totp`,
`cancel_challenge`, `enroll_totp`, `confirm_totp_enrollment`,
`refresh_if_needed`, and the `enable_password` / `set_clock` registrations;
wire them before first use:

```rust
let provider = Arc::new(EmailPasswordProvider::new(backend, clock));
controller.update(cx, |c, _| {
    c.enable_password(provider.clone() as Arc<dyn PasswordAuth>);
    c.set_clock(clock);
});
```

`DemoBackend` (native only, id `"demo"`) is the reference backend: argon2id
hashing (`"v1$b64url(salt)$b64url(hash)"`, 8 MiB / 2 passes by default,
`with_params` for cheaper test costs), deterministic tokens
(`demo-access-{seq}` / `demo-refresh-{seq}`, 900 s access TTL), refresh that
keeps the refresh token, idempotent revoke, and RFC 6238 TOTP with a
hand-rolled HMAC-SHA1 verifier (totp-rs 6.0 does not carry its documented
feature set, so the ~30-line fallback from the plan landed instead).

Demo caveats: users and sessions live in process memory — after an app
restart the persisted `TokenSet` restores `SignedIn` until the first
`refresh_if_needed` surfaces `RefreshRejected` (sign in again to recover);
TOTP secrets never leave the backend's process memory and never enter the
`TokenStore`; `verified` is always `false` (nothing is sent, the state exists
so UI can badge it).

## Security rules

- Secrets live in exactly two places: the controller's transient `TokenSet`
  values (zeroized on drop) and the `TokenStore`. Never in config files, never
  in logs (`SecretString`'s `Debug` prints `[redacted]`; use
  `TokenSet::redacted_summary` for logging).
- Every browser flow carries PKCE S256 and a fresh high-entropy `state`; the
  controller exact-matches the state on callback and drops the flow on
  mismatch (RFC 7636, RFC 8252).
- Client secrets never enter `FlowLaunch` or `TokenSet`; providers receive
  them at construction from host config input (RFC 8252 §8.5: embedded
  secrets in native apps are not confidential).
- The crate is clock-free: expiry checks take `now_unix` parameters, so no
  wasm clock hazard exists.

## Wasm posture

| Concern | Behavior |
| --- | --- |
| Store | `InMemoryStore` — tokens vanish on reload; the user re-authenticates |
| Executor | Provider futures run on the GPUI executor (`AuthIo::drive` has no tokio on wasm) |
| Randomness | `getrandom` `wasm_js` backend (browser crypto) |
| HTTP | `reqwest` fetch futures (json-only feature set) |
| Browser flows | Not available until the host ships an https-redirect path; scheme registration is a no-op on web |

Native hosts pass `Some(tokio_runtime)` to `AuthIo` so reqwest futures run on
tokio (`rt.spawn`); `None` falls through to a direct await, which is the
documented test/mock-only convention.

## Extraction checklist

- Own manifest: yes (`publish = false`).
- Zero app imports: `grep -rn "gpui_starter\|crate::" src/` returns nothing.
- Feature flags: `mock-provider` gates the scripted provider; delete one word
  in the host manifest to ship without it.
- README: this file.
- Publish path: flip `publish`, set a version, keep the gpui-pre pin aligned
  with the consumer's kit (`gpui-kit` pins the gpui-pre family to `=0.3.9`),
  then `cargo publish` from this directory.
