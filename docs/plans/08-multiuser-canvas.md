# 08 Multiuser Canvas Plan

## Goal

A "Shared Canvas" sidebar page mounts plan 01's engine and syncs it: named colored collaborator cursors, live shape add/update/delete relayed through a socket.io dev server (`just collab-server`), a visible connection status with automatic reconnect, and an offline demo mode with simulated collaborators that runs with no server (default on open, and the only mode on wasm). Live sync is native-only; the wasm build keeps the demo peers and hides the live controls with a stated reason.

## Dependencies

Plan `01-canvas-foundation.md`. `src/ui/canvas/` does not exist on disk yet (`ls src/ui/` shows only `widgets/`), so this plan starts only after plan 01 is implemented and green. Interface Contract items consumed (line refs into `docs/plans/01-canvas-foundation.md`):

- `Canvas::new(id, Entity<CanvasState>)` mount + the documented observe obligation in the mounting view (`cx.observe(&state, |_, _, cx| cx.notify())`, contract lines 320-323, 336).
- `CanvasState`: `new` (218), `edit_document` (240), `add_layer` (241), `add_shape` (242), `remove_shape` (243), `top_layer` via `CanvasDocument::top_layer` (187), `update_shape` via `CanvasDocument::update_shape` (195), `document` reader (237), `transform` reader (223), `selected_shape_ids` (248), `hit_test` (249), `select_shape` (250), `clear_selection` (251), `set_tool` (254), `active_tool_id` (256), `set_tool_overlay` (260), `tool_overlay` (261), `zoom_to_fit` (229), `reset_view` (230).
- `CanvasShape::{rect, ellipse, path, text}` constructors (158-162), `doc_bounds` (165), `CanvasLayer::shapes` reader (177), `ShapeStyle` constructors (140-142), `StrokeStyle` (128-133), `LayerId`/`ShapeId` (103-107), `HIT_TOLERANCE` (101).
- `CanvasTool` trait with default no-op bodies and the `&mut CanvasState` + `&mut Context<CanvasState>` dispatch shape (294-304), `CanvasPointerEvent` (266-274), `ToolContext` (286-288), the swap-based `pub(crate) dispatch_tool_event` recipe (337, step 3.1), and the router rules consumed as-is (338).
- Plan 01's `Image` variant is not syncable (`RenderImage` has no serde story, 01 Research Digest); this plan never publishes or accepts `Image` shapes.

Survey input: `docs/plans/_survey/seams.md` sections 1, 3, 4, 5, 6, 7, 8, 9 and `docs/plans/_survey/ecosystem.md` sections a, d, f, Step 3; everything load-bearing re-verified this session (sources cited below).

## Research Digest

Versions checked 2026-10-12 with `curl -s -A "gpui-starter-planner" https://crates.io/api/v1/crates/<name>`: `rust_socketio` 0.6.0 (2024-04-16), `socketioxide` 0.18.7 (2026-08-24), `axum` 0.8.9 (2026-04-14). Neither socket crate is in the local registry cache (`ls ~/.cargo/registry/src/*/ | grep -iE 'socketio'` → no hits); both .crate tarballs were downloaded from `static.crates.io` to /tmp and read directly this session, plus docs.rs pages fetched. Path keys below: `RSIO` = `/tmp/rust_socketio-0.6.0/src`, `SOX` = `/tmp/socketioxide-0.18.7/src`, `GPUI` = `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-pre-0.3.8/src`, repo files by `path:line`.

rust_socketio 0.6.0 (client, native-only per survey Step 3):

- Async API: `rust_socketio::asynchronous::{Client, ClientBuilder, ReconnectSettings}` re-exported at `RSIO/asynchronous/mod.rs:6-7`; `Payload` and `Event` at crate root (`RSIO/lib.rs:192-196`).
- `ClientBuilder::new(url)`, `.namespace(ns)`, `.auth(serde_json::Value)` (handshake auth), `.transport_type(TransportType)` (`TransportType` from the shared `client` module, imported by the async builder at `RSIO/asynchronous/client/builder.rs:15`), `.reconnect(bool)` (default true), `.reconnect_on_disconnect(bool)` (default false), `.reconnect_delay(min: u64, max: u64)`, `.max_reconnect_attempts(u8)` (default infinite), `.on(event, cb)`, `.on_reconnect(cb)`, `.on_any(cb)`; `.connect() -> Result<Client>`, all verified in the tarball (`RSIO/asynchronous/client/builder.rs:380-420` and docs.rs).
- Callback shape: `F: FnMut(Payload, Client) -> BoxFuture<'static, ()> + 'static + Send + Sync`; docs recommend `futures_util::FutureExt::boxed` on the async block (docs.rs ClientBuilder page; `RSIO/asynchronous/client/builder.rs:186-195`).
- `Payload` variants: `Binary(bytes::Bytes)`, `Text(Vec<serde_json::Value>)`, `String(String)` (deprecated); `From<serde_json::Value> -> Text` (docs.rs Payload page). Receiving a JSON object from socketioxide arrives as `Payload::Text(vec![value])`.
- `Client::emit<E: Into<Event>, D: Into<Payload>>(&self, event, data) -> Result<(), Error>` and `Client::disconnect()` (both async); `Client: Clone` (docs.rs Client page).
- Drive model: `connect()` = `connect_manual()` + `poll_stream()` (`RSIO/asynchronous/client/builder.rs:415-419`); `poll_stream` spawns the receive/reconnect loop via `tokio::runtime::Handle::current().spawn` (`RSIO/asynchronous/client/client.rs:151`), so the client must be created inside a tokio runtime. The reconnect loop retries with exponential backoff and consults `reconnect`/`reconnect_on_disconnect` after the stream drops (`client.rs:140-175`). An initial connect against a down server fails in `connect_manual` with no auto-retry; the app thread owns initial-connect retry (design below).
- Features: `default = []`, `async = ["async-callbacks", "rust_engineio/async", "tokio", "futures-util", "async-stream"]` (`RSIO/Cargo.toml [features]`). `native-tls 0.2.11` is an unconditional dependency (`RSIO/Cargo.toml [dependencies.native-tls]`), which pulls `openssl-sys` on Linux (see Risks).

socketioxide 0.18.7 (server, own crate):

- Axum wiring, verbatim from the docs.rs crate page example: `let (layer, io) = SocketIo::new_layer(); io.ns("/", async |s: SocketRef| { s.on("message", async |s: SocketRef| { s.emit("message-back", "Hello World!").ok(); }); }); axum::Router::new().route("/", get(...)).layer(layer); axum::serve(TcpListener, app).await;` Also `SocketIo::new_svc()` / `SocketIo::builder()` exist (`SOX/src/io.rs`, docs.rs SocketIo page).
- Features: no `default` key in `[features]` → default is empty; `v4 = ["engineioxide/v3"]` opts into socket.io protocol v4 / engine.io v3 compatibility (`SOX/Cargo.toml [features]`). socketioxide's default is socket.io protocol v5 / engine.io v4 (`SOX/README.md:43` "protocol v5 ... enabled by default"; `SOX/src/lib.rs:371-387` maps engineio V4 ↔ socket.io V5), so the server ships with default features (no `v4`). Which engine.io version rust_socketio actually speaks is NOT verifiable from its own sources (`grep -rn EIO /tmp/rust_socketio-0.6.0/src/` → nothing; `inner_create` at `RSIO/asynchronous/client/builder.rs:433-458` only builds the engine client, negotiation lives in rust_engineio, whose tarball was not downloaded) — end-to-end handshake compatibility is proven by the Phase 2 loopback test, not by this planning pass (Open Questions).
- Handlers: `Socket::on(event, H: MessageHandler<A, T>)`; async closures taking extractor args implement it (`SOX/src/handler/message.rs:175-215`); handlers are `FnOnce(...) -> Fut + Clone` so closures capturing `Arc` state work.
- Extractors: `extract::SocketRef` (wraps `Arc<Socket>`, `Deref<Target = Socket>`, `FromMessageParts`, `SOX/src/extract/socket.rs:26`); `extract::Data<T>(pub T)` with `T: DeserializeOwned`, `FromConnectParts` decodes the handshake `auth` value (`SOX/src/extract/data.rs:33-45`); this is how the client's `.auth(hello)` reaches the connect handler.
- `Socket` surface used: `pub id: Sid` field (`SOX/src/socket.rs:284-294`), `emit<T: ?Sized + Serialize>(&self, event, &T) -> Result<(), SendError>` — synchronous, the `Result` is `#[must_use]` (`SOX/src/socket.rs:485-487`), `on_disconnect(C: DisconnectHandler)` (`SOX/src/socket.rs:475-479`), `broadcast() -> BroadcastOperators` whose `emit<T: ?Sized + Serialize>(self, event, &T) -> impl Future<Output = Result<(), BroadcastError>>` must be awaited (`SOX/src/socket.rs:638-640`, `SOX/src/operators.rs:259-264`, `SOX/docs/operators/broadcast.md`: broadcast filters out the current socket). Handlers _must_ be async: `MessageHandler` requires `F: FnOnce(..) -> Fut` with `Fut: Future<Output = ()>` (`SOX/src/handler/message.rs:193-198`), `DisconnectHandler` likewise ("Handlers _must_ be async", `SOX/src/handler/disconnect.rs:7, 151-155`); sync closures do not compile.
- axum 0.8.9 pairs with socketioxide 0.18 through the tower layer above (both hyper 1.x era; socketioxide carries its own hyper dep, `SOX/Cargo.toml [dependencies.hyper]`).

App-side async/threading facts (gpui-pre 0.3.8 + repo):

- `Context<T>::spawn(f: AsyncFnOnce(WeakEntity<T>, &mut AsyncApp) -> R)` (`GPUI/app/context.rs:230-238`); `WeakEntity<T>::update(&self, cx, f) -> Result<R>` returns `Err` once the entity is released (`GPUI/app/entity_map.rs:803-813`); `AsyncApp` implements `update_entity` (`GPUI/app/async_context.rs:60-67`). OS threads never touch `cx`; they forward over flume and a `cx.spawn` loop drains (seams survey section 3; tray precedent `src/platform/desktop_shell/tray/macos.rs:146-153`).
- `flume 0.11` is already a dependency (`Cargo.toml:208`); tokio gains `rt-multi-thread`, `net`, `io-util` natively (`Cargo.toml:350-353`), so a dedicated socket thread can build `tokio::runtime::Runtime::new()` without new tokio features.
- Timers for the demo-peer loop: `cx.background_executor().timer(Duration)` has in-repo precedent (`src/gallery/sections/tables/data_table.rs:803`, `src/gallery/sections/charts/system_monitor.rs:219`) and the gallery ships on wasm, so the demo loop is wasm-safe. Throttling uses `crate::platform::clock::Instant` (`src/platform/clock.rs:1-8`; `std::time::Instant` panics on wasm, seams section 6).
- Kit facade: no `use gpui::` anywhere in `src/`, enforced by `tests/qa_migration.rs:386-405`. `use gpui_kit::{prelude::*, *};` + named `gpui_kit::component::` imports (`src/features/pages/notifications.rs:1-2`).
- Component facts: `InputState::new(window, cx).placeholder(...)`, `Input::new(&entity)` (`src/gallery/page.rs:23, 81`); `Badge::new().with_size(...).child(...)` (`src/gallery/sections/buttons/badge.rs:44-50`); `Toggle::new(id)` (`src/gallery/sections/buttons/toolbar.rs:191`); a11y helpers `.a11y(Role, label)`, `.a11y_live(accesskit::Live::Polite)`, list pattern `Role::List` + `aria_size_of_set` / `Role::ListItem` + `aria_position_in_set` (`src/features/pages/notifications.rs:50-53, 84-124`).
- Icons: the component enum the app registers is generated ONLY from the default 106 (`gpui-kit-assets-0.7.1/default-icons.txt`); the app's native asset source embeds exactly that set, so full-catalog names (`users-round`, `wifi`, `wifi-off`, `pen-tool`, `mouse-pointer-2`, `cloud-off`) compile via the assets enum but render blank on native. Verified defaults used: `User` (peer panel), `Network` (connected), `Ban` (offline), `RefreshCw` (reconnecting), `Bot` (demo peers), `Undo2`, `Plus`, `Minus`, `Square`, `Circle`. No wifi glyph exists in the defaults — `Network`/`Ban` carry the online/offline states.
- Pages: capability registry API (`src/runtime/capabilities.rs:6-50`; fields are pub so a wasm "unsupported" status is a struct literal), init list pattern (`src/app/init.rs:95-115`), i18n facade `crate::i18n::localize` / `localize_with_args` (`src/services/i18n.rs:1-13`), locale key grammar and lockstep test (seams section 5; `tests/qa_localization.rs:188-198`).
- Lock discipline: dep changes stage the regenerated `Cargo.lock` together with the manifest — repo practice for reproducible builds. The named gate is narrower than it sounds: `tests/qa_migration.rs:168-186` only rejects HEAD's gpui-starter lock block listing removed gpui-family deps (`gpui-pre-platform`, `gpui-component`, `gpui-kit-assets`), and no CI lane runs `cargo build --locked` on the app (grep of `justfile` + `.github/workflows/*.yml` for `--locked` → zero hits this session), so un-staged additions pass CI. The one-gpui-family lock rule (`tests/qa_migration.rs:131-146`) is untouched: neither new crate depends on gpui.

Crate picks:

- `rust_socketio 0.6` (`default-features = false, features = ["async"]`): the only maintained Rust socket.io client; native-only (survey Step 3), which the spec already accepts. Stale ~2.5 years and hard-wires native-tls; accepted for a dev-demo feature, tracked in Risks.
- `socketioxide 0.18` + `axum 0.8`: server-side pair from the spec. The socketioxide layer API is tarball-verified; the axum wiring is docs.rs-only — axum is not in the local registry cache (`ls ~/.cargo/registry/src/*/ | grep -c '^axum-0.8'` → 0 this session) — so `Router::layer`/`axum::serve` spellings carry docs.rs as their evidence.
- `collab-protocol` (new path crate, serde only): wire types + LWW board shared by app and server; avoids the server depending on the app crate (which cannot happen: the app is a wasm-building cdylib with the gpui graph). Two real consumers exist, so the shared crate is not speculative (AGENTS.md).

No CRDT: shape sync is last-write-wins per shape on a `(lamport, origin)` version tuple: deterministic total order, no merge machinery, cheap. A CRDT would buy concurrent-edit intent preservation the demo does not need.

## Architecture

Entities and ownership (page owns everything; nothing is global):

- `MultiuserCanvasPage` (view entity) owns `Entity<CanvasState>` (plan 01 engine state), `Entity<CollabSession>` (sync state), two `Entity<InputState>` (name, server URL), a `FocusHandle`, and subscriptions. Render: toolbar, status chip, peer panel, canvas + cursor overlay.
- `CollabSession` owns: identity (`Peer { id, name, color }`), link `Status`, `Mode` (Demo | Live), peer map `BTreeMap<ClientId, PeerCursor>` (cursor doc position included), a `collab_protocol::Board` mirror (wire shapes, versions, tombstones, lamport clock), `HashMap<ShapeKey, ShapeId>` + reverse mapping, `link: Option<Arc<dyn NetLink>>` (`NetLink` is declared in session.rs and is cfg-neutral; native net.rs implements it, so no session field or method needs a cfg), an optional `FakePeerDriver`, and a 50 ms cursor-throttle stamp. It holds `Entity<CanvasState>` and is the only writer of synced document content besides the page's tools.
- `net.rs` (native only) owns a dedicated `std::thread` running a private multi-thread tokio runtime and the `rust_socketio` client. It exposes exactly one entry, `connect_link(url, hello) -> (flume::Receiver<InboundEvent>, Arc<dyn NetLink>)`: the receiver carries the cfg-neutral `InboundEvent` (net.rs imports it from session.rs), and the returned trait object wraps the native handle. Thread ↔ session communication: unbounded flume channels both ways. The thread never touches GPUI contexts (seams section 3 rule). Because session.rs touches the network only through `Option<Arc<dyn NetLink>>` and the `InboundEvent` receiver, its single wasm-specific code is the `#[cfg]`-fenced connect body (native arm calls `net::connect_link`, wasm arm logs and stays in Demo); every field and every other method compiles unchanged on both targets.
- `collab-server` (separate binary) owns the truth-ish board: it integrates every op with the same LWW rule and answers `snapshot` on join/reconnect. Clients do not talk to each other.

Data flow, local edit: page tool (inside plan 01's `dispatch_tool_event`) mutates `CanvasState` with plan 01 mutators, then calls `session.update(cx, |s, _| s.publish_*(...))` (a different entity; no double lease). `publish_*` assigns `(ShapeKey, Version)`, integrates into the local `Board` mirror, records the ShapeId mapping, and pushes the `WireOp` through `link.send("op", ...)` when a live link exists (net thread command channel in Live mode; nothing in Demo mode, where local shapes stay local).

Data flow, remote op (Live): socket callback → `InboundEvent::ServerMessage` → session drain task (`cx.spawn` loop on the session entity) → `session.update` → `Board::integrate` (LWW gate) → on accept, `edit_document` on `CanvasState` (add / replace / remove via the mapping) → `cx.notify()` → page redraw. Demo mode: the fake-peer timer task fabricates `WireOp`s with fake `ClientId` origins and pushes them through the identical accept path, so LWW, mapping, and rendering are one code path for remote and simulated peers.

Presence: handshake `auth` carries the `Peer` hello (verified `Data<T>` FromConnectParts path); server registers, sends `snapshot` (peers + board) to the joiner, broadcasts `peer_join`/`peer_leave`. Reconnect re-runs the handshake, so the fresh `snapshot` both restores missed state and flips status back to Connected.

Cursors: tools forward `doc_position` on every forwarded pointer event → `session::set_own_cursor` throttled to 20 Hz → Live: `cursor` event to server → server relays; Demo: fake peers ignore it. Peer cursors render as an absolutely-positioned div overlay above the `Canvas` element inside one `relative()` wrapper; screen position = `CanvasState::transform().to_screen(peer.cursor_doc)` (contract line 223; `to_screen` line 116).

LWW rule (identical in `collab_protocol::Board`, used by client and server): reject an op when `version <=` the shape's stored version or its tombstone version; `Add`/`Update` store `(version, shape)` and clear a tombstone, `Delete` stores the tombstone and drops the shape. Version = `(lamport: u64, origin: u64)` with derived lexicographic order; lamport clocks bump by one on send and to `max(local, received)+1` on receive.

Module tree:

```text
collab-protocol/                  new workspace member (serde-only)
├── Cargo.toml
└── src/lib.rs                    Peer, ClientId, ShapeKey, Version, WireColor, WireStroke,
                                   WireStyle, WireShape, WireOp, CursorMsg, PeerHello,
                                   SnapshotMsg, Board (LWW integrate/tombstones/lamport)
collab-server/                    new workspace member (native binary)
├── Cargo.toml
└── src/main.rs                   axum + socketioxide relay: ns handler, presence map,
                                   snapshot on join, op/cursor relay, /healthz
src/features/pages/multiuser_canvas/
├── mod.rs                        page seam + module header (removal recipe) + re-exports
├── session.rs                    CollabSession entity: status/mode/peers/mappings/publish_*,
                                   accept path, drain + demo tasks, capability updates
├── net.rs                        #[cfg(not(target_family = "wasm"))] socket thread + channels
├── fake.rs                       FakePeerDriver: deterministic simulated collaborators
├── tools.rs                      PenTool + SelectTool (CanvasTool impls that publish ops)
├── view.rs                       render helpers: toolbar, status chip, peer panel, overlay
├── session.test.rs / fake.test.rs / tools.test.rs / view.test.rs (sibling test modules)
```

## Interface Contract

Public surface consumers get. Nothing outside `src/features/pages/multiuser_canvas/` imports the session internals; the page type is the only pub item re-exported through `crate::views`.

```rust
// src/features/pages/multiuser_canvas/mod.rs — only the page type is public.
pub struct MultiuserCanvasPage { /* private */ }
impl MultiuserCanvasPage {
    /// Constructed once by AppRoot. Mounts the canvas, starts demo peers.
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self;
}
impl Render for MultiuserCanvasPage { .. }
```

`collab_protocol` (crate `collab-protocol`, consumed by app + server):

```rust
pub type ClientId = u64;   // type alias, not newtype: keeps JSON a plain number

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Version { pub lamport: u64, pub origin: ClientId }   // lexicographic order = LWW order

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ShapeKey { pub origin: ClientId, pub seq: u32 }

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WireColor(pub [f32; 4]);   // H S L A; h in degrees, rest 0..=1

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WireStroke { pub width: f32, pub color: WireColor, pub dash: Option<Vec<f32>> }

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WireStyle { pub fill: Option<WireColor>, pub stroke: Option<WireStroke> }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WireShape {
    Rect { bounds: [f32; 4], style: WireStyle },           // x, y, w, h
    Ellipse { bounds: [f32; 4], style: WireStyle },
    Path { points: Vec<[f32; 2]>, stroke: WireStroke },
    Text { position: [f32; 2], text: String, font_size: f32, color: WireColor },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WireOp {
    Add { key: ShapeKey, version: Version, shape: WireShape },
    Update { key: ShapeKey, version: Version, shape: WireShape }, // full replacement
    Delete { key: ShapeKey, version: Version },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Peer { pub id: ClientId, pub name: String, pub color: WireColor }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CursorMsg { pub origin: ClientId, pub doc: [f32; 2] }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShapeEntry { pub key: ShapeKey, pub version: Version, pub shape: WireShape }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SnapshotMsg { pub peers: Vec<Peer>, pub shapes: Vec<ShapeEntry> }

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PeerLeaveMsg { pub id: ClientId }

/// LWW board: shared integrate rules for client and server.
#[derive(Clone, Debug, Default)]
pub struct Board { /* private: shapes, tombstones, lamport */ }
impl Board {
    pub fn new() -> Self;
    pub fn lamport(&self) -> u64;
    pub fn next_version(&mut self, origin: ClientId) -> Version; // lamport += 1
    pub fn observe(&mut self, version: Version);                 // lamport = max(lamport, v)+1
    /// Pure LWW gate; true when the op was integrated.
    pub fn integrate(&mut self, key: ShapeKey, version: Version, change: Change) -> bool;
    pub fn shape(&self, key: &ShapeKey) -> Option<&ShapeEntry>;
    pub fn shapes(&self) -> impl Iterator<Item = &ShapeEntry> + '_;
    /// Swap the whole board for a snapshot's contents; lamport becomes
    /// max(own, every seen lamport), so the next `next_version` outranks
    /// everything in the snapshot and post-snapshot local ops win.
    pub fn replace_all(&mut self, shapes: Vec<ShapeEntry>);
    pub fn clear(&mut self);
}
pub enum Change { Put(WireShape), Remove }
```

Session surface (`pub(crate)`: nothing outside the page module consumes it):

```rust
pub(crate) enum Status { Demo, Offline, Connecting, Connected, Reconnecting }
pub(crate) enum Mode { Demo, Live }

/// One remote (or simulated) collaborator as the view reads it.
pub(crate) struct PeerCursor { pub peer: Peer, pub doc: Option<Point<Pixels>> }

/// Sole event type crossing the net boundary. Declared here so the native
/// net thread and the drain task share one type; net.rs (cfg'd out on wasm)
/// builds these directly — nothing in session.rs mirrors them.
pub(crate) enum InboundEvent {
    Connected,
    Reconnecting,
    ServerMessage { event: String, payload: serde_json::Value },
    Error { message: String },
}

/// Cfg-neutral link seam: declared here, implemented by native net.rs
/// (`NetHandle`); wasm sessions hold `None`, which makes every send a no-op.
pub(crate) trait NetLink: Send + Sync {
    fn send(&self, event: &'static str, payload: serde_json::Value);
    fn shutdown(&self);
}

impl CollabSession {
    pub(crate) fn new(canvas: Entity<CanvasState>, cx: &mut Context<Self>) -> Self;
    // Rendering reads:
    pub(crate) fn status(&self) -> Status;
    pub(crate) fn mode(&self) -> Mode;
    pub(crate) fn identity(&self) -> &Peer;
    pub(crate) fn peers(&self) -> impl Iterator<Item = &PeerCursor> + '_;
    // Control (page calls through entity::update):
    pub(crate) fn set_mode(&mut self, mode: Mode, cx: &mut Context<Self>);
    pub(crate) fn set_name(&mut self, name: String, cx: &mut Context<Self>); // effective next connect
    pub(crate) fn connect(&mut self, url: String, cx: &mut Context<Self>);   // no-op on wasm (logged)
    pub(crate) fn disconnect(&mut self, cx: &mut Context<Self>);
    // Edit publishing (tools + page actions):
    pub(crate) fn publish_add(&mut self, shape: &CanvasShape, local_id: ShapeId, cx: &mut Context<Self>);
    pub(crate) fn publish_update(&mut self, local_id: ShapeId, shape: &CanvasShape, cx: &mut Context<Self>);
    pub(crate) fn publish_delete(&mut self, local_id: ShapeId, cx: &mut Context<Self>);
    pub(crate) fn wire_key(&self, local_id: ShapeId) -> Option<ShapeKey>;
    // Intake (drain task + fake loop + tests):
    pub(crate) fn handle_net_event(&mut self, event: InboundEvent, cx: &mut Context<Self>);
    pub(crate) fn apply_op(&mut self, op: WireOp, cx: &mut Context<Self>);   // shared by live + fake
    // Cursor:
    pub(crate) fn set_own_cursor(&mut self, doc: Point<Pixels>, cx: &mut Context<Self>); // 20 Hz throttle
    // Board ops (page actions):
    pub(crate) fn delete_selected(&mut self, cx: &mut Context<Self>);
    pub(crate) fn clear_board(&mut self, cx: &mut Context<Self>);
}
```

Socket wire protocol (both sides, event names stable): handshake `auth` = `Peer`; events `op` (`WireOp`), `cursor` (`CursorMsg`), `peer_join` (`Peer`), `peer_leave` (`{ "id": ClientId }` as `PeerLeaveMsg`), `snapshot` (`SnapshotMsg`). Server binds `0.0.0.0:3001` by default (`COLLAB_PORT` env override) and serves `GET /healthz`.

Consumption notes that are part of this contract: the page must `cx.observe(&session, |_, _, cx| cx.notify())` (same pattern as the canvas mount obligation), and `CanvasState` must be observed the same way per plan 01. `publish_*` with an `Image` shape is a debug_assert + no-op (not syncable).

## Module Layout

| File | Purpose |
|---|---|
| `collab-protocol/Cargo.toml` | serde-only crate manifest, `publish = false`, edition 2024 |
| `collab-protocol/src/lib.rs` | Wire types, `Board` LWW engine, lamport clock (all pure, no gpui) |
| `collab-server/Cargo.toml` | socketioxide + axum + tokio binary manifest |
| `collab-server/src/main.rs` | Relay server: presence, snapshot, op/cursor broadcast, /healthz |
| `src/features/pages/multiuser_canvas/mod.rs` | `MultiuserCanvasPage`, module header with removal recipe |
| `src/features/pages/multiuser_canvas/session.rs` | `CollabSession` entity, accept path, tasks, capability updates |
| `src/features/pages/multiuser_canvas/net.rs` | Native socket thread, `NetCommand`, `connect_link` + `impl NetLink for NetHandle`, flume bridge |
| `src/features/pages/multiuser_canvas/fake.rs` | Deterministic fake peers (cursor walks, draws, edits, deletes) |
| `src/features/pages/multiuser_canvas/tools.rs` | `PenTool`, `SelectTool`: `CanvasTool` impls that publish |
| `src/features/pages/multiuser_canvas/view.rs` | Toolbar, status chip, peer panel, cursor overlay render helpers |
| `src/features/pages/multiuser_canvas/*.test.rs` | Sibling test modules per repo convention |

## Seam Touches

Every edit outside the new modules:

| # | File | Change | Why minimal |
|---|---|---|---|
| 1 | `Cargo.toml:1-2` | `[workspace]` gains `members = ["collab-protocol", "collab-server"]` | Server/protocol crates must not sit in the app's dep graph (wasm build + lock bloat) |
| 2 | `Cargo.toml [dependencies]` | `collab-protocol = { path = "collab-protocol" }` | Single shared wire crate; serde-only, compiles on wasm (fake mode uses it) |
| 3 | `Cargo.toml` native-only table (`Cargo.toml:310`) | `rust_socketio = { version = "0.6", default-features = false, features = ["async"] }` | Spec's platform gate; keeps rust_socketio + native-tls out of wasm |
| 4 | `Cargo.lock` | Regenerate + commit with the manifest edits | `tests/qa_migration.rs:168-186` reads the HEAD blob but only rejects removed gpui-family deps; staging additions is repo practice, not CI-enforced (no `cargo build --locked` lane) |
| 5 | `justfile` | `collab-server` recipe: `cargo run -p collab-server` | Spec asks for a justfile entry point |
| 6 | `src/features/pages/mod.rs` | `pub mod multiuser_canvas;` + `pub use multiuser_canvas::MultiuserCanvasPage;` | Page registration seam (survey section 1, row 2) |
| 7 | `src/shell/sidebar.rs` | `Page::MultiuserCanvas` variant + the five match sites: `title` (19-31), `title_key` (36-49), `host` (57-70), `icon` (77-90), `all()` entry appended last (92-105) | Append-last avoids rebinding digit keys (`state.rs:148-152` binds only `i < 9`) |
| 8 | `src/shell/route.rs:10-21` | `Page::MultiuserCanvas.host(),` in `VALID_HOSTS` | Deep-link routing (survey row 9) |
| 9 | `src/shell/root/app_root/state.rs:22-31` | Field `multiuser_canvas_page: Entity<MultiuserCanvasPage>` | AppRoot wiring (survey row 10) |
| 10 | `src/shell/root/app_root/state.rs:41-56` | Construct `cx.new(|cx| MultiuserCanvasPage::new(window, cx))` + import | Survey row 11 |
| 11 | `src/shell/root/app_root/state.rs:211-223` | `unchecked_active_page_view` arm | Survey row 12 |
| 12 | `tests/e2e_navigation.rs:14-19` | Page count `10` → current literal + 1; add `Page::MultiuserCanvas` to the `expected` array | Count gate stays honest; +1 keeps it mechanical regardless of which sibling plans (05) landed first |
| 13 | `locales/en.yml` + `locales/zh-CN.yml` | `nav_multiuser_canvas` + `multiuser_*` keys (i18n section) in BOTH files | Key lockstep test (`tests/qa_localization.rs:188-198`) |

Deliberate non-touches: no `src/services/commands.rs` entry (launcher integration is optional per survey section 1 and not in the spec), no `AppConfig` field (display name is per-run; the config schema is snapshot-pinned, `tests/snapshot_tests.rs:26,57`), no `[patch.crates-io]` change, no gallery section.

### Removal Recipe

`src/features/pages/multiuser_canvas/mod.rs` opens with (mirrors `src/gallery/mod.rs:1-7`):

```rust
//! Removable collaborative-canvas page. `MultiuserCanvasPage` is the only
//! public surface; the session, tools, net thread, and fake peers stay
//! private. Removal = delete this directory, the collab-protocol and
//! collab-server workspace members, and revert the seam touches: the
//! Page::MultiuserCanvas arms and all() entry in shell/sidebar.rs, the
//! VALID_HOSTS entry in shell/route.rs, the AppRoot wiring in
//! shell/root/app_root/state.rs, the mod/pub use lines in
//! features/pages/mod.rs, the nav/multiuser locale keys, the page count in
//! tests/e2e_navigation.rs, the workspace members + two dependency lines +
//! the justfile collab-server recipe in Cargo.toml/justfile, and the
//! regenerated Cargo.lock.
```

Ordered removal steps:

1. Revert `src/features/pages/mod.rs` to the current list.
2. `rm -r src/features/pages/multiuser_canvas/`.
3. Revert the five `sidebar.rs` arms + `all()` entry, the `route.rs` host, and the `state.rs` field/construct/arm; `cargo check` proves the arms are exhaustive again.
4. Revert `tests/e2e_navigation.rs` count (-1) and drop the `expected` entry.
5. Remove the `nav_multiuser_canvas`/`multiuser_*` keys from both locale files.
6. `rm -r collab-protocol/ collab-server/`; remove the `members` line, the `collab-protocol` dependency, the `rust_socketio` line, and the justfile recipe; regenerate `Cargo.lock` and commit both together.
7. Run the gates in Verification; grep `multiuser_canvas\|collab_protocol\|rust_socketio` across the repo returns nothing.

## Implementation Phases

## Phase 1: `collab-protocol` crate

### Step 1.1: Workspace member + crate scaffold

- Files: `Cargo.toml` (workspace `members`), `collab-protocol/Cargo.toml`, `collab-protocol/src/lib.rs`, regenerated `Cargo.lock`.
- Build: crate `{ name = "collab-protocol", version = "0.1.0", edition = "2024", publish = false }`, deps `serde = { version = "1", features = ["derive"] }` only. `lib.rs` empty placeholder. Root `Cargo.toml [workspace]` gains `members = ["collab-protocol"]`; `"collab-server"` is appended in Step 2.1 (a listed-but-missing member is a cargo error).
- Tests: none.
- Done-check: `cargo check -p collab-protocol` passes; `git diff --stat` shows Cargo.lock updated (stage it with the manifest — repo practice; see Research Digest for the gate's actual scope).

### Step 1.2: Wire types + serde

- Files: `collab-protocol/src/lib.rs`, `collab-protocol/src/lib.test.rs` wired with `#[cfg(test)] #[path = "lib.test.rs"] mod lib_test;`.
- Build: every type from the Interface Contract exactly (`ClientId`, `Version` with derived `PartialOrd/Ord`, `ShapeKey`, `WireColor([f32;4])`, `WireStroke`, `WireStyle`, `WireShape` `#[serde(tag = "kind", rename_all = "snake_case")]`, `WireOp` same tag style, `Peer`, `CursorMsg`, `ShapeEntry`, `SnapshotMsg`, `PeerLeaveMsg { id: ClientId }`). Doc comments on each pub type: what it carries and its direction on the wire.
- Tests: JSON round-trip per `WireOp` variant and `SnapshotMsg` (`serde_json::to_value` → `from_value` → assert_eq); assert the serialized `WireOp::Add` discriminant is exactly `{"kind":"add",...}` (pins the tag spelling both sides parse).
- Done-check: `cargo test -p collab-protocol` passes.

### Step 1.3: `Board` LWW engine

- Files: same.
- Build: `Board { shapes: HashMap<ShapeKey, ShapeEntry>, tombstones: HashMap<ShapeKey, Version>, lamport: u64 }`, `Change::{Put, Remove}`; `integrate` per the Architecture rule (reject `version <=` stored or tombstone; Put stores + clears tombstone; Remove stores tombstone + drops shape; returns the accept decision); `next_version` (lamport += 1, `Version { lamport, origin }`), `observe` (lamport = max + 1), `replace_all` (swap shapes/tombstones wholesale, lamport = max(own, max seen lamport) — matches the contract comment), `clear`, readers. Pure: no gpui, no clock.
- Tests: Add accepted then stale Update with lower version rejected; tie on lamport broken by origin (both directions); Delete tombstones and a lower-version Add is rejected while a higher-version Add resurrects; `next_version`/`observe` produce monotonic lamports across interleaved remote versions; `replace_all` drops state not in the snapshot; `clear` keeps lamport monotonic.
- Done-check: `cargo test -p collab-protocol` passes.

## Phase 2: `collab-server` crate

### Step 2.1: Crate scaffold + axum/socketioxide wiring

- Files: `collab-server/Cargo.toml`, `collab-server/src/main.rs`, root `Cargo.toml` (append `"collab-server"` to members), `Cargo.lock` regenerated.
- Build: deps `socketioxide = "0.18"` (default features), `axum = "0.8"`, `tokio = { version = "1", features = ["macros", "rt-multi-thread", "net"] }`, `serde`/`serde_json`, `collab-protocol = { path = "../collab-protocol" }`, `tracing` + `tracing-subscriber` (fmt, env-filter) for request logs; dev-deps `rust_socketio = { version = "0.6", default-features = false, features = ["async"] }`, `futures-util`. `main.rs`: bind `TcpListener` on `format!("0.0.0.0:{}", std::env::var("COLLAB_PORT").unwrap_or_else(|_| "3001".into()))`, `let (layer, io) = SocketIo::new_layer();` router `Router::new().route("/healthz", get(|| async { "ok" })).layer(layer)`, `axum::serve(listener, app).await`: the exact docs.rs wiring from the Research Digest.
- Tests: none yet.
- Done-check: `cargo check -p collab-server` passes.

### Step 2.2: Namespace handler, presence, relay

- Files: `collab-server/src/main.rs` (+ a `#[cfg(test)] #[path = "main.test.rs"] mod main_test;` sibling in Phase 2.3).
- Build: `Arc<Mutex<ServerState>>` where `ServerState { board: collab_protocol::Board, peers: HashMap<ClientId, Peer> }`. `io.ns("/", move |s: SocketRef, Data(hello): Data<Peer>| async move { ... })` with `Data` over handshake auth (verified `SOX/src/extract/data.rs:33-45`); every handler closure is `async move` (sync closures do not satisfy `MessageHandler`'s `Fut: Future` bound, Research Digest). Inside: insert peer; `s.emit("snapshot", &SnapshotMsg { peers: all peers, shapes: board.shapes().cloned().collect() }).ok();` (sync `Result`, Research Digest); `s.broadcast().emit("peer_join", &hello).await.ok();`; register `s.on("op", |s: SocketRef, Data(op): Data<WireOp>| async move { lock; board.observe(op.version); per-op-key integrate; drop lock before any await (a std guard is `!Send`); s.broadcast().emit("op", &op).await.ok(); })`; `s.on("cursor", |s: SocketRef, Data(c): Data<CursorMsg>| async move { s.broadcast().emit("cursor", &c).await.ok(); });` `s.on_disconnect(move |s: SocketRef| async move { remove peer by scanning for the id; broadcast "peer_leave" with PeerLeaveMsg .await.ok(); })`. Handler closures capture a `ServerState` clone; `Clone` closures over `Arc` satisfy `MessageHandler` (verified bounds in Research Digest). Note `socket.id` exists as a `Sid` field but presence is keyed by `ClientId` from the hello so reconnects replace the same peer.
- Tests: none yet (loopback in 2.3).
- Done-check: `cargo run -p collab-server` boots and `curl -s localhost:3001/healthz` prints `ok` (manual, run once); `cargo clippy -p collab-server -- -D warnings` clean.

### Step 2.3: Loopback integration test

- Files: `collab-server/src/main.test.rs`; extract the state/handler construction in `main.rs` into `pub(crate) fn build_router(state: ServerState) -> (Router, SocketIo)` so the test binds `TcpListener::bind("127.0.0.1:0")`, reads the port, spawns `axum::serve`, and runs against it.
- Build (`#[tokio::test]`, two `rust_socketio` clients): client A connects with `.auth(peer_a)` + `transport_type(TransportType::Websocket)`; assert A receives `snapshot` (peers contains A); client B connects; assert A receives `peer_join(B)`; B emits `op` Add; assert A receives the op and B does NOT (broadcast excludes sender); A emits a stale-version Update; assert B receives it but a second snapshot shows the board kept the newer shape (server LWW); B disconnects; assert A receives `peer_leave`. Payload parse: `Payload::Text(v)` → `v.first().cloned()` → `serde_json::from_value`.
- Done-check: `cargo test -p collab-server` passes; this test is the protocol-compatibility proof between rust_socketio 0.6 and socketioxide 0.18 (see Open Questions).

### Step 2.4: justfile recipe

- Files: `justfile`.
- Build: recipe `# Run the socket.io relay the Shared Canvas page connects to.` / `collab-server:` / `    cargo run -p collab-server`.
- Tests: none.
- Done-check: `just --list` shows `collab-server`.

## Phase 3: App dependency + native socket thread

### Step 3.1: App dependencies (`collab-protocol`, `rust_socketio`)

- Files: `Cargo.toml` `[dependencies]` and native-only table (`[target.'cfg(not(target_family = "wasm"))'.dependencies]`), `Cargo.lock`.
- Build: `collab-protocol = { path = "collab-protocol" }` in `[dependencies]` (Seam Touches row 2; serde-only so it compiles on wasm — Steps 4.x need `Board`/`WireOp` on every target, and Step 3.2's `net.rs` needs `Peer`); plus `rust_socketio = { version = "0.6", default-features = false, features = ["async"] }` in the native-only table (default features are empty; `async` is the full client set per the tarball `[features]`).
- Tests: none.
- Done-check: `cargo check -p gpui-starter` passes on the host; `git diff Cargo.lock` shows collab-protocol, rust_socketio, rust_engineio, and native-tls added; stage the lock with the manifest (see Research Digest for what actually enforces it).

### Step 3.2: Socket thread

- Files: `src/features/pages/multiuser_canvas/mod.rs` (created here with the module header comment and `#[cfg(not(target_family = "wasm"))] pub(crate) mod net;`), `src/features/pages/mod.rs` (add `pub mod multiuser_canvas;` now; the `pub use ... MultiuserCanvasPage;` re-export lands in Phase 7 when the type exists), `net.rs`.
- Build: `use super::session::{InboundEvent, NetLink};` — net.rs defines no event enum of its own. `pub(crate) enum NetCommand { Emit { event: &'static str, payload: serde_json::Value }, Shutdown }` (thread-internal); `pub(crate) struct NetHandle { cmd_tx: flume::Sender<NetCommand>, thread: std::thread::JoinHandle<()> }` with `impl NetLink for NetHandle`: `send` does `cmd_tx.try_send(NetCommand::Emit { .. })` (log on full/closed), `shutdown` sends `Shutdown` then waits on `done_rx: flume::Receiver<()>` with `recv_timeout(Duration::from_secs(2))` — `JoinHandle::join` has no timeout variant in std, so the thread signals exit through this second channel; the handle itself is detached (on timeout, log a warn and leave the thread to exit on its own: the Shutdown command and `cmd_rx` disconnect both terminate the loop). Entry point `pub(crate) fn connect_link(url: String, hello: Peer) -> (flume::Receiver<InboundEvent>, std::sync::Arc<dyn NetLink>)`: creates the unbounded `InboundEvent` channel and the `NetCommand` channel, spawns the thread below holding `event_tx: flume::Sender<InboundEvent>` and `cmd_rx: flume::Receiver<NetCommand>`, returns `(event_rx, Arc::new(handle))`. The thread is named "collab-socket", builds `tokio::runtime::Builder::new_multi_thread().enable_all().build()`, then `rt.block_on`:
  - Connect loop: `loop { match ClientBuilder::new(url).namespace("/").auth(serde_json::to_value(hello)).transport_type(TransportType::Websocket).reconnect(true).reconnect_on_disconnect(true).reconnect_delay(300, 5000).on("op", cb).on("cursor", cb).on("peer_join", cb).on("peer_leave", cb).on("snapshot", cb).on("error", err_cb).on_reconnect(reconnect_cb).connect().await { Ok(client) => break Some(client), Err(e) => { event_tx Reconnecting; if cmd_rx.try_recv() is Shutdown → return; sleep backoff (500 ms * attempt, capped 5 s, via tokio sleep) } } }`. Initial-connect retry is ours because rust_socketio only retries established connections (Research Digest).
  - Callbacks: clone `event_tx`, match `Payload::Text(v) => v.first().cloned()`, `Payload::String(s) => serde_json::from_str(&s).ok()`, forward `ServerMessage`; futures boxed with `.boxed()` per docs. `reconnect_cb` sends `Reconnecting` and returns default `ReconnectSettings::new()`.
  - After connect: `event_tx Connected`; then `while let Ok(cmd) = cmd_rx.recv_async().await { match cmd { Emit { event, payload } => if let Err(e) = client.emit(event, Payload::from(payload)).await { event_tx Error } , Shutdown => { let _ = client.disconnect().await; break; } } }`.
  - On `cmd_rx` disconnect (all senders dropped): disconnect + exit. Thread logs through `tracing` with `target: "gpui_starter::multiuser_canvas::net"`; never touches a GPUI context.
- Tests: none in-app (the loopback test in Step 2.3 covers both halves of the protocol against the real server implementation).
- Done-check: `cargo clippy -p gpui-starter --all-features` clean; `just wasm-check` still passes (`net.rs` is cfg'd out on wasm).

## Phase 4: `CollabSession` core

### Step 4.1: Entity, status, identity, capability

- Files: `src/features/pages/multiuser_canvas/session.rs`, `session.test.rs`, `mod.rs` (declare `pub(crate) mod session;`).
- Build: `CollabSession` fields per Architecture; `Status`, `Mode`; peer map `BTreeMap<ClientId, PeerCursor>`. Identity: `id = uuid::Uuid::new_v4().as_u64_pair().0 as ClientId` (uuid 1.23.1 already a dep, `Cargo.toml:175`); default `name = format!("{} #{:04x}", crate::i18n::localize("multiuser_you"), (id & 0xffff) as u16)` (localized prefix plus a stable discriminator so two instances are distinguishable); color from a fixed 8-entry `WireColor` palette in `session.rs` indexed by `id % 8`; peer colors are protocol data, the same exemption plan 01 gives document stroke colors, and the palette lives as data constants, not in render code. Constructors/readers from the Interface Contract; `new` registers the capability entry `"collab_canvas"` (native: `CapabilityStatus::supported_enabled()`; wasm: struct literal `supported: false, enabled: true, degraded: true, reason: Some("live sync needs native networking; demo peers only")`; fields pub at `src/runtime/capabilities.rs:6-13`). `EventEmitter` not needed: the page uses `cx.observe`.
- Tests (`#[gpui_kit::test]`, `cx: &TestAppContext`, pattern `src/state/config_store.test.rs:280-281`): new session has status Demo, mode Demo, identity with non-empty name, peer map empty; capability snapshot (`crate::capabilities::snapshot`) contains `collab_canvas` with `supported == !cfg!(target_family = "wasm")` asserted via the runtime cfg of the host running the test (native host → true).
- Done-check: `cargo test -p gpui-starter --lib multiuser_canvas::session` passes.

### Step 4.2: Board ↔ document mapping and the accept path

- Files: same.
- Build: mappings `by_wire`/`by_local`; wire⇄canvas conversions (`WireShape -> CanvasShape` via plan 01 constructors with `px()`; `CanvasShape -> Option<WireShape>` reading `bounds().origin/size`, points, and lengths through `as_f32()`; `None` for `Image`). `apply_op(op)`: `board.observe(op.version)` always, then `board.integrate(key, version, change)`; if accepted, mutate the canvas through plan 01 mutators inside `canvas.update(cx, |state, cx| ...)` — Add: `let layer = state.document().top_layer().unwrap_or_else(|| state.add_layer(cx)); let local = state.edit_document(cx, |doc| doc.add_shape(layer, shape)).flatten();` record mapping on `Some(local_id)`; Update: `state.edit_document(cx, |doc| doc.update_shape(local_id, |s| *s = new_shape))` (whole-variant replacement; mapping unchanged); Delete: `state.remove_shape(local_id, cx)` then drop the mapping; finish with one `cx.notify()` on the session. `apply_snapshot(msg)`: `board.replace_all(msg.shapes)`, then rebuild the document (clear + re-add every entry + rebuild both maps) in one `edit_document`; set peers from `msg.peers` minus own id; status Connected. Presence events insert/remove `PeerCursor`. `publish_add/update/delete`: `let version = board.next_version(id); let key = ShapeKey { origin, seq: next local seq }; board.integrate(key, version, ...)`; record mapping; `if let Some(link) = &self.link { link.send("op", serde_json::to_value(op).unwrap_or_default()); }` (Demo or offline: `link` is `None`, no-op, the shape stays local). Every document mutation goes through a plan 01 mutator exactly once per op.
- Tests: applying Add from a fake origin creates one shape and maps ids both ways; stale Update rejected leaves the document untouched (revision unchanged); Delete removes the shape and a later stale Add is dropped; `publish_add` in Demo mode adds locally with no link installed and a subsequent higher remote Update for the same key replaces geometry; `apply_snapshot` replaces the document wholesale (old shapes gone, mapping rebuilt); tombstoned key resurrects on higher-version Add.
- Done-check: `cargo test -p gpui-starter --lib multiuser_canvas::session` passes.

### Step 4.3: Net plumbing, connect/disconnect, cursor throttle, tasks

- Files: same.
- Build: `session.rs` declares both `InboundEvent` and the `NetLink` trait (Interface Contract) and stores `link: Option<std::sync::Arc<dyn NetLink>>` — cfg-free on both targets; net.rs implements the trait and builds `InboundEvent` values, so nothing in session.rs mirrors native types. `connect(url, cx)` holds the module's only `#[cfg]` pair: native arm → status Connecting, `cx.notify()`, `let (rx, link) = net::connect_link(url, hello);` store `Some(link)`, and spawn the drain task `cx.spawn(async move |this: WeakEntity<CollabSession>, cx: &mut AsyncApp| { while let Ok(ev) = rx.recv_async().await { if this.update(cx, |s, cx| s.handle_net_event(ev, cx)).is_err() { break; } } })` (signatures verified: `GPUI/app/context.rs:230-238`, `GPUI/app/entity_map.rs:803-813`); wasm arm → log once and keep status Demo. On wasm, `set_mode(Live)` also logs and keeps Demo so the demo peers never silently stop (the Live toggle is hidden there anyway). `disconnect`: `if let Some(link) = self.link.take() { link.shutdown(); }`, peers cleared, status Offline — no cfg. `set_own_cursor`: `clock::Instant` 50 ms gate (`src/platform/clock.rs:1-8`); then `if let Some(link) = &self.link { link.send("cursor", ...); }`; notify. Status transitions call `crate::capabilities::set("collab_canvas", ...)` (Connected → supported_enabled; Reconnecting → degraded("reconnecting"); Error → error(msg)).
- Tests: `handle_net_event(InboundEvent::Connected)` on a session with no peers stays Demo-only bookkeeping (status Connected requires mode Live; assert mode gating); injecting a full `ServerMessage{snapshot}` through `handle_net_event` flips status to Connected and populates peers; cursor throttle: install a test-only `RecordingLink` (implements `NetLink`, records sends in a `std::sync::Mutex<Vec<(String, serde_json::Value)>>`) via a `#[cfg(test)] fn install_link_for_test(&mut self, link)`, then two immediate `set_own_cursor` calls produce at most one `cursor` send; `disconnect` on the recording link is observable as a shutdown flag.
- Done-check: `cargo test -p gpui-starter --lib multiuser_canvas::session` passes; `just wasm-check` passes (session.rs's only cfg is the connect body's native/wasm pair; the `link` field, drain task, `publish_*`, `set_own_cursor`, and `disconnect` are cfg-free because they go through `Arc<dyn NetLink>` and `InboundEvent`).

## Phase 5: Fake peers

### Step 5.1: Driver model

- Files: `src/features/pages/multiuser_canvas/fake.rs`, `fake.test.rs`, `mod.rs` declare.
- Build: `FakePeerDriver { peers: Vec<FakePeer>, rng_state: u64, tick: u64 }` with a xorshift64* PRNG implemented inline (no `rand` dependency: rust_socketio brings one transitively but it is not a direct dep and cannot be imported). `FakePeer { peer: Peer, phase: f32, cursor: [f32; 2], strokes: Vec<ShapeKey>, next_op_tick: u64 }`. `FakePeerDriver::new(seed)` builds exactly `PEER_COUNT = 2` peers (`pub(crate) const`, phase-offset per index; Step 7.3's smoke test asserts the peer map holds 2). `advance(&mut self) -> Vec<PeerEvent>` where `pub(crate) enum PeerEvent { Cursor(CursorMsg), Op(WireOp) }`: every tick the cursor follows a lissajous path over a fixed doc area (x ∈ [0, 800], y ∈ [0, 600] doc px, per-peer phase offset); on `next_op_tick` (rng-derived 40-90 ticks, ~3-7 s at 80 ms) the rng picks add (path of 10-16 points around the cursor, stroke in the peer's color) / update (translate one own stroke by a small rng delta, emitted as a full-shape Update) / delete (drop the oldest own stroke beyond 2); each peer carries its own `lamport: u64` bumped once per emitted op, so `WireOp.version = Version { lamport, origin: peer.id }` stays deterministic and per-origin increasing (the session's board merges the clock through `observe`). Names via `localize_with_args("multiuser_demo_peer", &[("n", &index.to_string())])`; ids from the fixed range `0xFABE_0000_0000_0000 + i` so they never collide with uuid client ids. Deterministic: same seed → same event sequence.
- Tests: two drivers with the same seed produce identical event streams for the first 500 ticks; every emitted Op's version is strictly increasing per origin; cursor positions stay inside the constant viewport; add/update/delete mix occurs within 200 ticks for a 2-peer driver (rng coverage smoke).
- Done-check: `cargo test -p gpui-starter --lib multiuser_canvas::fake` passes.

### Step 5.2: Demo loop in session

- Files: `session.rs`.
- Build: `start_demo(cx)`: builds driver (seed from own client id), `cx.spawn` loop `loop { timer 80ms (`cx.background_executor().timer(Duration::from_millis(80))`, precedent `src/gallery/sections/tables/data_table.rs:803`); driver.advance(tick) → for each Cursor update peer map; for each Op `apply_op` through the same path as remote ops; `cx.notify()` }` with WeakEntity guard identical to the drain task; exits when the session is released or mode leaves Demo. `set_mode` starts/stops the loop (stop = drop the driver; task observes mode via a check each tick and returns when `mode != Demo`). Called from `new` (demo peers run from first open, per Goal).
- Tests: with `#[gpui_kit::test]`, advance the deterministic clock in step with the parked executor — timers on the test clock do not fire under `run_until_parked` alone, so interleave `cx.executor().advance_clock(Duration::from_millis(80))` with `cx.run_until_parked` (precedent pairing: `src/state/config_store.test.rs:294-295, 317-318, 334-335`). After ~120 such ticks (9.6 simulated seconds), the session has ≥1 peer cursor and ≥1 shape; switching to Live stops emissions (advance a further simulated second, shape count stable).
- Done-check: `cargo test -p gpui-starter --lib multiuser_canvas` passes.

## Phase 6: Sync tools

### Step 6.1: `PenTool` and `SelectTool`

- Files: `src/features/pages/multiuser_canvas/tools.rs`, `tools.test.rs`, `mod.rs` declare.
- Build: both hold `session: Entity<CollabSession>` + drag state. `PenTool` = plan 01 `BrushTool` behavior (Step 4.2 of plan 01) with: `pointer_move` also calls `session.update(cx, |s, cx| s.set_own_cursor(ev.doc_position, cx))`; commit on `pointer_up` first does `state.add_shape(...)` (returns `Option<ShapeId>`), then `session.update(cx, |s, cx| if let Some(id) = local_id { s.publish_add(&shape, id, cx) })`. `SelectTool` = plan 01 `SelectTool` behavior (snapshot + translate via `edit_document`) with: cursor reporting on move; on `pointer_up`, if the drag moved anything, for each selected id with a geometry snapshot different from the original, `s.publish_update(id, &current_shape, cx)` (read `state.document().shape(id)`); a click that only selects publishes nothing. Both cursors: Pen `Crosshair`, Select `Arrow`. Never re-enter the CanvasState lease: session updates happen with the `&mut Context<CanvasState>` deref target, i.e. `session.update(cx, ...)` where `cx: &mut Context<CanvasState>`: `Entity::<CollabSession>::update` takes `&mut impl AppContext`, which `Context<CanvasState>` is via its `AppContext` impl (`GPUI/app/context.rs:777`, cited in plan 01 Research Digest).
- Tests (mirror plan 01's tool test recipe, driving via `dispatch_tool_event` inside `entity.update` on a `CanvasState` wired to a session): pen down/move/up leaves one Path in the document, one `publish_add` (assert via `session.read(cx).wire_key(shape_id).is_some()`), overlay cleared; select-drag on a seeded shape moves it locally during the drag (no publish) and publishes exactly one Update on release; select-click only selects (no version bump; assert board lamport unchanged); pointer_move updates own cursor (throttled).
- Done-check: `cargo test -p gpui-starter --lib multiuser_canvas::tools` passes.

## Phase 7: Page view

### Step 7.1: Page struct, focus, actions

- Files: `src/features/pages/multiuser_canvas/mod.rs`; `src/features/pages/mod.rs` (add `pub use multiuser_canvas::MultiuserCanvasPage;`; the `pub mod` line landed in Step 3.2). `view.rs`/`view.test.rs` first appear in Steps 7.2/7.3.
- Build: two actions defined in `mod.rs` with the exact derive style of `src/shell/root/actions.rs:3-15`: `#[derive(Action, Clone, PartialEq, Eq, serde::Deserialize)] #[action(namespace = multiuser, no_json)] pub struct DeleteSelection;` and the same for `ClearBoard`. `MultiuserCanvasPage { session, canvas, name_input, url_input, focus_handle, _subscriptions }`; `new`: create `Entity<CanvasState>` with the `cx.observe` wiring (plan 01 mount obligation), create the session entity + `cx.observe(&session, |_, _, cx| cx.notify())`, `InputState`s (name seeded from the identity, url default `http://127.0.0.1:3001`), and `cx.bind_keys([KeyBinding::new("delete", DeleteSelection, Some("MultiuserCanvas"))])` (KeyBinding construction pattern at `src/shell/root/app_root/state.rs:145-166`). Render root: `v_flex().id("multiuser-canvas-page").track_focus(&self.focus_handle).key_context("MultiuserCanvas").size_full()` with `.on_action(cx.listener(...))` handlers routing to `session.update` (`delete_selected` / `clear_board`) and `canvas.update` (`clear_selection`).
- Tests: page constructs in a `TestAppContext` window without panicking; dispatching `DeleteSelection` on a session with one selected shape removes it (drive via `cx.dispatch_action` on the window or call the listener path through the action handler by simulating focus; assert through the canvas document).
- Done-check: `cargo test -p gpui-starter --lib multiuser_canvas` passes.

### Step 7.2: View helpers

- Files: `view.rs`.
- Build (all `RenderOnce`-style free fns taking the data they need, imports per `src/features/pages/notifications.rs:1-2` plus `use gpui_kit::component::button::{Button, ButtonVariants as _};` — `.ghost()` is a `ButtonVariants` trait method (`gpui-component-0.7.1/src/button/button.rs:48, 82`), imported as `_` per the repo precedent `src/gallery/sections/editor/workspace.rs:7`):
  - `render_header(status: Status, cx)`: title (`multiuser_title`, `Role::Heading` + `aria_level(1)`) + status chip: `Badge::new().child(Icon)` + localized label inside a live container: `div().id("mc-status").a11y(Role::Status, label).a11y_live(accesskit::Live::Polite)` (`Role::Status` variant verified in the kit's accesskit 0.24.1, `accesskit-0.24.1/src/lib.rs` Role enum) (announce transitions, pattern `notifications.rs:50-53`); icon by status: Demo `Bot`, Connected `Network`, Reconnecting `RefreshCw`, Connecting/Offline `Ban` (all verified defaults — no wifi glyph exists in the default 106; Research Digest icons bullet).
  - `render_toolbar(mode: Mode, status: Status, is_live_available: bool, page-listener-routed actions, inputs, cx)`: mode segmented pair of `Toggle`s (Demo/Live). The live cluster (the Live toggle arm, url Input, connect Button) renders only when `is_live_available` (a `const bool` = `!cfg!(target_family = "wasm")`); on wasm its place shows the `multiuser_wasm_note` muted paragraph. The layout code itself compiles once, gated at runtime. url `Input` + connect/disconnect `Button` (label swaps by status; disabled while Connecting); name `Input` (commits on change → `set_name`); tool buttons `Button::ghost` with `.label(multiuser_tool_pen/select)` toggling `canvas.set_tool`; `Delete` (`multiuser_delete`), `Clear board` (`multiuser_clear`), `Reset view` (`multiuser_reset_view`, ghost, no icon). Buttons carry the Delete/Clear actions through the page listener, not per-button closures duplicating logic (one command, one handler, per the coding guides' "Model one logical desktop command once").
  - `render_peer_panel(peers: &[PeerCursor], own: &Peer, cx)`: `v_flex().w_56().border_l_1().border_color(cx.theme().border)` with `div().a11y(Role::List, localize("multiuser_peer_list_label")).aria_size_of_set(len)` + rows `Role::ListItem` with `aria_position_in_set`, each row: color dot (`div().size_2().rounded_full().bg(peer_color_hsla)`), name; own row labeled `multiuser_you`; empty state `multiuser_peer_list_empty` muted text (data-heavy + a11y list rules, design guides).
  - `render_cursor_overlay(peers: &[PeerCursor], transform: CanvasTransform, cx)`: peers' cursors as `div().absolute().left(px(x)).top(px(y))` marker (color dot `size_2` + `text_xs` name chip with `bg(peer_color).opacity(0.9)` text in `foreground`), inside the same `relative()` wrapper that hosts `Canvas::new("mc-canvas", canvas)`. `px()` here is measured runtime geometry (doc cursor × transform), the audited exception class in the design guides' zoom section.
  - Layout: `v_flex().p_6().gap_4()` header + toolbar, then `h_flex().flex_1().min_h_0().items_stretch()` with `div().relative().flex_1().min_w_0()` (canvas + overlay) + peer panel (h_flex items_stretch per coding guides' column-in-row rule).
- Tests: none new (rendered assertions are the Step 7.1 + 7.3 scope).
- Done-check: `cargo clippy -p gpui-starter --all-features` clean for the module.

### Step 7.3: Page smoke test

- Files: `view.test.rs`.
- Build (`#[gpui_kit::test]`): `cx.add_window` with the page; the demo loop's first op lands after 3–7 simulated seconds at the 80 ms tick, so interleave `cx.executor().advance_clock(Duration::from_millis(80))` with `cx.run_until_parked` for ~150 ticks (12 simulated seconds) before asserting through entity reads (status is `Status::Demo`, peer map holds 2 entries, canvas document gained ≥1 shape from the fake drawer), then switch to the Pen tool and drive a draw through the `VisualTestContext` mouse path (plan 01 Step 7.2 recipe): the local path shape exists and `session.read(cx).wire_key(id)` is `Some`.
- Done-check: `cargo test -p gpui-starter --lib multiuser_canvas` passes.

## Phase 8: Shell seams, i18n, gates

### Step 8.1: Sidebar, route, AppRoot seams

- Files: `src/shell/sidebar.rs`, `src/shell/route.rs`, `src/shell/root/app_root/state.rs`, `tests/e2e_navigation.rs`.
- Build: per Seam Touches rows 7-12: `Page::MultiuserCanvas` variant (enum at `sidebar.rs:4-16`), `title() => "Shared Canvas"`, `title_key() => "nav_multiuser_canvas"`, `host() => "multiuser-canvas"`, `icon() => IconName::User` (verified default; `UsersRound` is full-catalog
   and renders blank on native — Research Digest icons bullet), `all()` append `Page::MultiuserCanvas,` last (`sidebar.rs:92-105`); `VALID_HOSTS` + `Page::MultiuserCanvas.host(),` (`route.rs:10-21`); `state.rs` field + construct + `unchecked_active_page_view` arm + import; e2e: bump the count literal (`tests/e2e_navigation.rs:17`), add `Page::MultiuserCanvas` to `expected`, AND rewrite the now-stale `"exactly ten pages"` comment (`tests/e2e_navigation.rs:14`) to the new number so the assertion message and comment agree.
- Tests: existing `tests/e2e_navigation.rs` now covers the new page (count + membership + title non-empty); add the case `("gpui-starter://multiuser-canvas", AppRoute::Page(Page::MultiuserCanvas))` to the `test_route_parsing` `cases` array; the existing round-trip and membership checks then cover it.
- Done-check: `cargo test -p gpui-starter --test e2e_navigation` passes.

### Step 8.2: Locale keys

- Files: `locales/en.yml`, `locales/zh-CN.yml`.
- Build: every key from the i18n section below, top-level `snake_case: "quoted"` grammar (parser `tests/qa_localization.rs:20-71`), both files.
- Tests: existing `qa_localization` lockstep + grammar checks.
- Done-check: `cargo test -p gpui-starter --test qa_localization` passes.

### Step 8.3: Full gates + manual QA

- Files: none (verification only).
- Build: run the four repo gates plus feature QA (Verification section).
- Tests: as listed.
- Done-check: all gates green; manual checklist executed or explicitly waived with notes.

## Platform & Wasm Gating

- Native-only dependency: `rust_socketio 0.6` under `[target.'cfg(not(target_family = "wasm"))'.dependencies]` (`Cargo.toml:310` table), alongside the existing native tokio features (`Cargo.toml:350-353`) its runtime needs. `src/features/pages/multiuser_canvas/net.rs` is wholly `#[cfg(not(target_family = "wasm"))]` (declared via a cfg'd `mod` line in the module's `mod.rs`, the same pattern as `src/lib.rs:47-49`).
- Wasm behavior, fully specified: the page exists and renders; demo peers run (the timer loop uses the background executor, wasm-safe per `src/gallery/sections/tables/data_table.rs:803` precedent); `set_mode(Live)`/`connect` log once and keep status Demo; the toolbar hides the Live toggle, URL field, and connect button behind `is_live_available = !cfg!(target_family = "wasm")` and shows `multiuser_wasm_note` instead. The page is never "hidden": every platform gets a working surface, live sync degrades with a stated reason (the repo's preferred degradation convention, seams section 6).
- Capability registry: `"collab_canvas"` set at session construction: native `CapabilityStatus::supported_enabled()`; wasm struct literal `{ supported: false, enabled: true, degraded: true, reason: Some(...) }` (fields pub, `src/runtime/capabilities.rs:6-13`); live transitions update it (Phase 4.3). No `DEFAULT_ENABLED_CAPABILITIES` edit (that list is for boot-time always-on services, `src/app/init.rs:95-108`; this feature reports its own state when its page exists).
- `collab-protocol` compiles on wasm (serde only) and is a plain `[dependencies]` entry; the demo mode needs it. `collab-server` is never built for wasm: `just wasm-check` checks `-p gpui-starter --lib` only (`justfile:24-25`), and the workspace's wasm-relevant surface is unchanged.
- Clock discipline: the cursor throttle uses `crate::platform::clock::Instant`, never `std::time::Instant` (`src/platform/clock.rs:1-8`; seams section 6 rule).
- CI note: the Linux/Windows check lanes build the two new crates (root-level `cargo check` covers all members); socketioxide/axum/rust_socketio are pure-Rust except rust_socketio's unconditional `native-tls` → `openssl-sys` on Linux (needs system OpenSSL headers or the `vendored` feature; Risks).

## i18n

Keys added to BOTH `locales/en.yml` and `locales/zh-CN.yml` (lockstep enforced by `tests/qa_localization.rs:188-198`); facade calls `crate::i18n::localize` / `localize_with_args` (`src/services/i18n.rs:3-13`). `%{n}` placeholder per the facade contract.

| key | en | zh-CN |
|---|---|---|
| `nav_multiuser_canvas` | "Shared Canvas" | "共享画布" |
| `multiuser_title` | "Shared whiteboard" | "共享白板" |
| `multiuser_status_demo` | "Demo peers" | "演示协作者" |
| `multiuser_status_offline` | "Offline" | "离线" |
| `multiuser_status_connecting` | "Connecting" | "连接中" |
| `multiuser_status_connected` | "Connected" | "已连接" |
| `multiuser_status_reconnecting` | "Reconnecting" | "重连中" |
| `multiuser_mode_demo` | "Demo" | "演示" |
| `multiuser_mode_live` | "Live" | "实时" |
| `multiuser_connect` | "Connect" | "连接" |
| `multiuser_disconnect` | "Disconnect" | "断开" |
| `multiuser_server_label` | "Server" | "服务器" |
| `multiuser_name_label` | "Name" | "名称" |
| `multiuser_peer_list_label` | "Collaborators" | "协作者" |
| `multiuser_peer_list_empty` | "No one else is here yet" | "还没有其他协作者" |
| `multiuser_tool_pen` | "Pen" | "画笔" |
| `multiuser_tool_select` | "Select" | "选择" |
| `multiuser_delete` | "Delete" | "删除" |
| `multiuser_clear` | "Clear board" | "清空画板" |
| `multiuser_reset_view` | "Reset view" | "重置视图" |
| `multiuser_demo_peer` | "Demo %{n}" | "演示 %{n}" |
| `multiuser_you` | "You" | "我" |
| `multiuser_cursors_label` | "Collaborator cursors" | "协作者光标" |
| `multiuser_wasm_note` | "Live sync needs the desktop app; the web build runs demo peers." | "实时同步需要桌面版；网页版运行演示协作者。" |

The status chip element carries `a11y_live(Polite)` so each transition is announced without a focus jump. Fake-peer names go through `multiuser_demo_peer`; the own-identity default name composes `multiuser_you` plus a hex discriminator (Phase 4.1) so two instances on one board are distinguishable.

## Design Compliance

- Primary task and layout (design guides "Start from the task", "Choose a stable shell"): the shared canvas is the primary object and takes `flex_1().min_w_0()`; controls sit in one toolbar row above it; the collaborator panel is a fixed-width trailing column (`w_56`) with a hairline `border_l_1` in `cx.theme().border`: secondary, scannable, subordinate in width. Peer cursors are content, not chrome, and render inside the canvas wrapper only.
- Tokens: every surface color from `cx.theme()` (`background`, `foreground`, `border`, `muted_foreground` for the empty state and wasm note, `primary` for the selected tool state via the kit's own selected styling). No raw hex/`rgb()`/`hsla()` in view code: peer and stroke colors are protocol data (`WireColor` from the wire), the same exemption class plan 01 gives document stroke colors; the local palette constants live in `session.rs` as data, not in render code.
- `px()` exceptions, each audited per the zoom section: cursor overlay positions (measured runtime geometry = doc × transform, must track zoom), cursor-throttle window (timing, not layout). All other layout uses rem helpers (`p_6`, `gap_4`, `w_56`, `size_2`, `text_xs`) per the toolbar/panel density guidance.
- Density and components: toolbar uses `Button::ghost`/outline in default medium size; mode picker uses kit `Toggle` (checked = active mode, persistent selected state, not hover-dependent); status uses `Badge` + `Icon`; inputs use kit `Input` with visible labels. No custom clickable divs.
- Interaction states: every button is a kit `Button` (hover/pressed/focus/disabled owned by the kit); Connect disables while `Connecting` (loading state prevents duplicate submission, coding guides async section); tool buttons show persistent selected state through `Toggle`-style checked or the kit's selected variant; status is text + icon, never color alone (icon shape differs per status); Demo/Live mode is a persistent visible selection.
- Keyboard path: page root is a focus region (`track_focus` + `key_context("MultiuserCanvas")`) with `delete` bound to `DeleteSelection`; every toolbar action is a labeled button reachable by Tab; clear/delete confirm nothing (reversible in demo scope: deleted shapes can be redrawn; no destructive AlertDialog needed for a whiteboard, per the reversible-action guidance).
- A11y: status chip `a11y_live(Polite)`; peer list follows the repo's list pattern (`Role::List` + `aria_size_of_set`, items `Role::ListItem` + `aria_position_in_set`, 0-based; `src/features/pages/notifications.rs:84-124`), cursor overlay container gets `a11y(Role::Label, multiuser_cursors_label)` as one labeled region rather than per-cursor noise (`Role::Label` verified in the kit's accesskit 0.24.1; there is no `Role::Text`); headings get `aria_level(1)`. The canvas surface itself has no a11y node in plan 01 v1, documented there and inherited here; the toolbar and peer panel are the accessible surface.
- Feedback surfaces (guides "Feedback and overlays"): connection transitions are inline status (chip), not dialogs or notifications; errors from the socket appear in the chip + tracing, not as alerts. Empty peer list shows one muted sentence naming the next action.
- Motion: none added; peer cursors jump per event (no interpolation): motion is not required to understand state, and smoothing is deferred.
- Interface language: sentence-case labels, verbs for commands (`Connect`, `Delete`, `Clear board`), nouns for destinations (`Shared Canvas`), no trailing periods on control labels, zh-CN full-width punctuation in sentences only, ellipsis `…` not used (no dialog-opening commands).

## Verification

Repo gates, in order (AGENTS.md):

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-features
cargo test --workspace --all-features
just wasm-check
```

`--workspace` now includes `collab-protocol` and `collab-server` (their tests run in `cargo test --workspace --all-features`). Stage the regenerated `Cargo.lock` with the manifest changes: `tests/qa_migration.rs:168-186` only rejects gpui-family removals from HEAD's lock and no CI lane runs `cargo build --locked` on the app (grep of `justfile` + `.github/workflows/*.yml` → zero `--locked` hits), so staging additions is repo practice for reproducible builds rather than an enforced gate.

Feature-specific automated coverage (wired in the phases):

- `cargo test -p collab-protocol`: serde round-trips, LWW accept/reject/tombstone/resurrect, lamport monotonicity.
- `cargo test -p collab-server`: live loopback: two real rust_socketio clients against the real axum/socketioxide server; snapshot, peer_join/leave, op relay (sender excluded), server-side LWW. This is the protocol-compatibility proof.
- `cargo test -p gpui-starter --lib multiuser_canvas`: session accept path and mapping, publish flow, cursor throttle, fake-peer determinism and demo loop, tool publish behavior through `dispatch_tool_event`, page smoke incl. focus/action dispatch.

Manual QA (native): `just collab-server` in one terminal, `cargo run` (or `bash scripts/macos-dev-app.sh`) twice in two others:

1. Open the Shared Canvas page in both instances: both show Demo status with moving fake peers drawing shapes immediately, no server needed.
2. Switch to Live, `Connect` in both: status goes Connecting → Connected; each sees the other's name + colored cursor in the peer panel and on the canvas; the second joiner receives the first's board via snapshot.
3. Draw in A while B watches: the stroke appears in B live. Drag-move it in B: it moves in A. Delete it in A: it disappears in B. Concurrent edits to the same shape resolve to the later version in both.
4. Kill the server: both chips flip to Reconnecting within seconds. Restart it: both reconnect (status Connected, fresh snapshot) without app restart.
5. Start with the server down and press Connect: chip shows Reconnecting (retry), then start the server: recovery. Press Disconnect: Offline, no further reconnects.
6. Set a name, reconnect: the peer list shows it in the other instance.
7. Pen and Select tools both work locally and sync; `delete` key removes the selection in both; Clear board clears both; Reset view recenters.
8. Wasm (`just wasm-serve`): page renders, Demo peers work, Live controls replaced by the note; diagnostics page shows `collab_canvas` unsupported with the reason.

Acceptance criteria: all four gates pass; the loopback and lib test suites pass; manual checklist items 1-3 and 8 confirmed (4-7 confirmed or waived with a note naming what was checked).

## Risks & Open Questions

Ranked risks:

1. **Socket.IO protocol compatibility is proven only by the Phase 2 test.** rust_socketio 0.6 is 2.5 years stale; socketioxide 0.18 defaults to socket.io protocol v5 / engine.io v4 (`SOX/README.md:43`, `SOX/src/lib.rs:371-387`) while the `v4` feature maps to engineio v3 (tarball `[features]`). Planning evidence is feature-flag analysis plus verified APIs, not a live handshake. Fallback lever if the loopback test fails: enable socketioxide's `v4` feature and/or switch the client to `TransportType::Any` (polling fallback); both single-line changes.
2. **rust_socketio's unconditional `native-tls`** (`RSIO/Cargo.toml [dependencies.native-tls]`) adds `openssl-sys` on Linux: CI runners and dev machines need OpenSSL headers; a pinned deployment without them fails to build the app crate. Mitigation options if it bites: `openssl-sys` `vendored` feature via a new dependency line (adds build time), or feature-gating the whole page behind a cargo feature like `ai-chat` (survey section 1). Chosen default: accept the dep; it is dev-demo scope.
3. **Plan 01 is not implemented yet** (`ls src/ui/` → `widgets/` only). Phases 4-7 consume its Interface Contract verbatim; any plan 01 contract change (renames, tool-dispatch reshape) breaks this plan's steps. Mitigation: this plan starts only against a green plan 01; every consumed item is listed in Dependencies with line refs.
4. **LWW loses concurrent edits to one shape** by design (a simultaneous move/draw race resolves to one winner). Accepted per spec ("last-write-wins unless a cheap CRDT is justified"; not justified: per-shape full-payload LWW is the cheap correct-enough register). Whole-stroke granularity means two people drawing never conflict; only same-shape edits can lose.
5. **Cursor and op rates on one shared namespace**: cursors are 20 Hz per client, unsynced ops are per-edit; a relay to everyone (single namespace) is fine at demo scale (a handful of peers). No rate limiting, no rooms; the server is explicitly a dev server. If more than ~10 clients matter later, add socket.io rooms per board id (the `BroadcastOperators` API already supports it).
6. **`replace_all` snapshot on reconnect discards local unpublished edits made while disconnected** (there is no offline queue). Drawing during a reconnect gap keeps shapes locally; the snapshot on rejoin overwrites the board and those local shapes lose their wire keys (the rebuild drops shapes without wire keys, so they vanish on rejoin). Accepted for v1: the visible status makes the gap explicit; an offline outbox is the follow-up if it matters.
7. **Delete key needs page-region focus** (the canvas element owns pointer focus contexts, plan 01 has none). Documented keyboard path: Tab into the page or click any toolbar control, then `delete`. The Delete toolbar button is always available.

Open questions (each with what was tried):

- **Live handshake rust_socketio ↔ socketioxide**: not run during planning (no server was built; socket crates absent from the registry cache, `ls ~/.cargo/registry/src/*/ | grep -i socketio` returned nothing). Evidence gathered: feature maps of both crates from their published tarballs and docs.rs wiring examples, plus socketioxide's documented default (socket.io v5 / engine.io v4, `SOX/README.md:43`, `SOX/src/lib.rs:371-387`). What rust_socketio speaks at the engine.io layer could NOT be verified: `grep -rn EIO /tmp/rust_socketio-0.6.0/src/` returns nothing and `inner_create` (`RSIO/asynchronous/client/builder.rs:433-458`) only builds the engine client — negotiation lives in rust_engineio, whose tarball was not downloaded, so no EIO claim is made here. The Phase 2.3 loopback test is the designed closure; if it fails, the lever in Risk 1.
- **`collab-protocol` on the app's wasm build**: assumed fine (serde-only). Not compiled for wasm during planning (doc-only scope; `just wasm-check` not run). The plan's Step 3.1/4.x done-checks include `just wasm-check`.
- **Page-count literal drift with sibling plans**: plan 05 (`docs/plans/05-canvas-showcase.md:143`) also bumps the count 10 → 11 and both plans append to `Page::all()`. Whichever lands second must use "current literal + 1", which is how Step 8.1 is written; if both land in one branch the literal is 12. Tried: reading plan 05's Seam Touches (cited above): no conflict in ordering as long as append-last holds for both.
- **No cargo gate was run during planning** (doc-only scope, matching the survey and plan 01). All commands in Verification are for the implementer.
