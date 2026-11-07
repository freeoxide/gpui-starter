# Security and privacy

Where credentials live, what telemetry does, how updates are trusted, and which
sockets the app opens. Each claim names the source that enforces it.

## Sign-in tokens live in the OS keyring

Desktop builds store sign-in tokens in the operating system's credential store
through the `keyring` crate with its per-platform native backends
(`Cargo.toml`): the macOS Keychain, the Windows Credential Store, and on Linux
the kernel's keyutils store. One row per provider: service `gpui-starter.auth`,
key the provider id, value the token set serialized as JSON
(`src/services/auth.rs`).

Keyutils is in-memory only: on Linux a saved sign-in lives in the kernel's
per-user keyrings and does not survive a reboot, so expect to sign in again
after restarting the machine.

A corrupt row reads as signed out, never a hard failure. Sign-out deletes the
row. There is no plaintext-file fallback: if the keyring is unavailable, the
secure-storage capability reports an error instead
(`src/services/secure_storage.rs`).

`state.json` in the config directory holds settings, not credentials. On wasm
there is no keyring; tokens stay in memory and are lost on reload.

The built-in email/password backend is an in-process demo
(`crates/gpui-auth/src/demo_backend.rs`): accounts live in a map in memory,
there is no server, and nothing persists across restarts (wasm ships a mock
provider). Replace it before shipping anything real.

## auth-providers.json

Social login (Google, GitHub, OIDC) reads `auth-providers.json` from the config
directory, or from `$GPUI_AUTH_PROVIDERS` when set. You author the file; the app
only reads it (`src/services/auth_social/config.rs`). It holds client ids and
client secrets.

- `chmod 600` the file. On Unix the app warns when it is group- or
  world-readable (it accepts exactly `0600`); Windows has no equivalent check.
- A file larger than 64 KiB is ignored and social login stays unconfigured.
- The default redirect mode is loopback, `http://127.0.0.1:{port}/`, and the
  URI must be registered with the provider verbatim. `"redirect": "scheme"`
  opts into `gpui-starter://auth/callback` and requires a registered OS
  handler for the scheme.

## Telemetry is opt-in

Telemetry is disabled by default. Settings offers three modes: disabled,
local-only, and remote; picking local-only or remote is the consent. The choice
persists in `state.json` and is re-applied on startup
(`src/services/telemetry/mod.rs`).

- Local-only logs event names to the app log. Nothing leaves the machine.
- Remote exports OTLP traces over HTTP and requires a build with the `otlp`
  feature. Without it, remote mode reports an error and events stay local.
- The endpoint is `OTEL_EXPORTER_OTLP_ENDPOINT` when set, else the built-in
  `http://localhost:4318`; the Settings buttons configure the mode only, not
  an endpoint. Traces go to `{endpoint}/v1/traces`.
- What is recorded: event names, error strings, and key/value properties the
  app passes explicitly. No file contents, page text, or free-form input is
  collected.

## Updates are Ed25519-pinned

The updater's Ed25519 public key is compiled into the binary
(`src/services/updater_public_key.bin`); the matching private key is
`UPDATE_SIGNING_KEY` in CI. A signature is base64 Ed25519 over the artifact's
SHA-256 (`src/services/updater/apply.rs`).

- Enforcement happens at swap time. A pending-swap marker must carry a
  signature, and the file it names is verified against the pinned key before
  anything replaces the running binary. An unsigned or tampered marker is
  deleted and the swap refused; a marker for a version that is not strictly
  newer is refused as well.
- The manifest itself is not signed. It is fetched over HTTPS and parsed as
  JSON, so a compromised manifest host can claim any version. The pinned key is
  what stops an untrusted binary from installing.
- Automatic checks need two opt-ins: the `update_check_on_startup` setting
  (default off) and a manifest URL baked at compile time
  (`GPUI_UPDATE_MANIFEST_URL`). The default URL is a placeholder, and
  placeholder builds never poll. With both set, the app checks 5 seconds after
  startup and then every 4 hours.
- This tree stops at detecting and reporting an available update: the
  manifest's asset fields are parsed but never read, and no code downloads an
  artifact or writes the swap marker. Swap staging lives in an app-owned
  `updates/` directory under the user's data dir, never a shared temp path.

The release pipeline that produces manifests and signatures is covered in
[releasing.md](releasing.md).

## Local listeners

The app itself opens two local sockets, both scoped:

- OAuth redirect capture. During social sign-in a single-shot listener binds
  `127.0.0.1` on a random port, reads one request line, answers a static page,
  and exits; it gives up after 300 seconds
  (`src/services/auth_social/loopback.rs`).
- Single-instance forwarding. A local socket carries deep links and commands
  from a second launch to the running instance: a namespaced socket (abstract
  on Linux, a named pipe on Windows), or a file socket in the user's cache
  directory where those do not exist. The name is world-bindable, so both ends
  check the peer's uid on Unix and reject a squatter from another account
  (`src/platform/process/single_instance.rs`).

The app opens no other listening sockets.

`collab-server` is a separate demo binary, not part of the app: a socket.io
relay behind the shared-canvas page. It binds `127.0.0.1:3001` by default,
which is also the address the canvas page dials; `COLLAB_HOST=0.0.0.0` exposes
it to the network. It has no authentication. Anyone who can reach the port can
join the board and broadcast ops and cursors, so leave it on loopback unless
the network is trusted.
