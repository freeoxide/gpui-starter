# gpui-agent integration status: blocked upstream

[gpui-agent](https://github.com/wsp-labs/gpui-agent) (wsp-labs) lets a CLI
or MCP client inspect and drive a GPUI app: element snapshots with refs,
clicks/typing/keys/scroll/drags through the app's own input handlers,
waits, screenshots, and state probes. We want it for agent-driven UI
verification. As of 2026-11 it cannot be integrated; this note records why
and what to do when upstream moves.

## Blocker

gpui-agent requires a **patched GPUI**. Its shipped patch set
(`patches/gpui-pre-0.3.9/` in their repo) and its own manifest pins both
target gpui-pre **0.3.9** exactly (`=0.3.9` for gpui-pre,
gpui-pre-wgpu, gpui-pre-linux). This app pins gpui-pre **=0.3.14**
(gpui-kit 0.11.0 pins the whole gpui-pre family to =0.3.14), and
`tests/qa_migration.rs` enforces exactly one gpui-pre family at 0.3.14 in
Cargo.lock. Consuming the git dep unpatched resolves a second 0.3.9 copy
and fails the QA gate on type identity grounds alone.

Porting the patches to 0.3.14 is not mechanical either — verified by
applying them against unpacked 0.3.14 sources (2026-11-23):

- `gpui-pre-0.3.9.patch`: two hunks fail. `src/app.rs` hunk 1 of 5 covers a
  `use crate::{...}` import block that upstream rewrapped and extended
  (`AppLifecyclePhase`, `PowerMode`, `WindowBounds`); only its three
  trailing context lines survive. `src/window.rs` hunk 1 of 12 adds the
  driver module beside lines where 0.3.14 inserted `mod native_view;` and
  its re-exports. `src/window.rs` hunk 12 applies only with fuzz 1.
- `gpui-pre-wgpu-0.3.9.patch`: hunk 2 (the `WgpuHeadlessRenderer`
  readback-to-`read_texture` refactor in `src/wgpu_renderer.rs`) fails on
  mid-hunk drift — since 0.3.10 the readback calls `get_mapped_range()?`,
  and that one changed line sits inside the block the hunk deletes.
- `gpui-pre-linux-0.3.9.patch`: applies with fuzz 1 via GNU `patch`
  (its first context line names `version = "=0.3.9"`, now `=0.3.14`);
  `git apply` refuses.

## Recipe when upstream ships 0.3.14 support

1. `Cargo.toml`:

   ```toml
   [features]
   agent = ["dep:gpui-agent", "gpui/test-support"]

   [dependencies]
   gpui-agent = { git = "https://github.com/wsp-labs/gpui-agent", optional = true, default-features = false, features = ["gpui-pre-0-3-14"] }
   ```

2. Patch the three gpui-pre crates (their `scripts/gpui-pre.sh` automates
   unpack + apply + checksums) and point `[patch.crates-io]` at the local
   copies. Vendored copies stay at version 0.3.14 so the lock keeps one
   family and qa_migration stays green.
3. `src/app/init.rs`, early in `App::init` (their README: call inside
   `application().run`):

   ```rust
   #[cfg(feature = "agent")]
   gpui_agent::init(cx);
   ```

4. Screenshots need `gpui/test-support` (and the platform crate's on
   Linux) — already covered by the feature line above.
5. Optional state exposure: `gpui_agent::probe(cx, "name", move |cx| ...)`
   for app-specific probes.

Until upstream publishes a 0.3.14-compatible patch set (or unpins its
`=0.3.9` requirements), do not vendor patched gpui-pre copies here: the
repo would carry a GPUI fork and re-port the patches on every gpui-kit
bump. The devtools lane (`--features devtools`, gpui-devtools 0.6.0)
covers element-tree inspection today and was accepted in the same scope.
