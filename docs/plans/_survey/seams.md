# Integration seams

Reference for the feature plans under `docs/plans/`. Every claim cites the
file and line read during this survey (2026-10-12). gpui/gpui-kit API claims
were verified against the crate sources on disk at
`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/{gpui-kit-0.7.1, gpui-pre-0.3.8, gpui-component-0.7.1, gpui-kit-assets-0.7.1}`
(shorthand below: `gpui-kit-0.7.1/...`, `gpui-pre-0.3.8/...`).

## 1. Page seam — adding a sidebar page

Removal-seam documentation is canonical: `src/gallery/mod.rs:2-7` names the
five seam touches. Full list for a new page `Foo`:

| # | File:line | Edit |
|---|-----------|------|
| 1 | `src/features/pages/foo.rs` | New module. Pattern: `src/features/pages/notifications.rs` — struct + `pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self` (`notifications.rs:12`); context-free pages may take no args (`AboutPage::new()`, `src/shell/root/app_root/state.rs:55`). |
| 2 | `src/features/pages/mod.rs:1-25` | `mod foo;` + `pub use foo::FooPage;`. Feature-gated module pattern: `#[cfg(feature = "ai-chat")] pub mod ai_chat;` (`mod.rs:2-3`, feature at `Cargo.toml:137-139`). |
| 3 | `src/shell/sidebar.rs:5-16` | `Page` variant. Derives include `Serialize, Deserialize` (`sidebar.rs:4`) — the variant lands in the persisted config (`AppConfig.active_route`, `src/state/config_store.rs:72`). |
| 4 | `src/shell/sidebar.rs:19-31` | `title()` arm (English label; logs and non-localized surfaces). |
| 5 | `src/shell/sidebar.rs:36-48` | `title_key()` arm — `nav_foo` i18n catalog key. |
| 6 | `src/shell/sidebar.rs:57-69` | `host()` arm — kebab-case deep-link host (`query-devtools-v2` style). Single source of truth for routing (`sidebar.rs:55-56`). |
| 7 | `src/shell/sidebar.rs:77-89` | `icon()` arm — `gpui_kit::component::IconName` variant (import `sidebar.rs:1`). |
| 8 | `src/shell/sidebar.rs:92-105` | `all()` entry. Order = sidebar order AND `secondary-1..9` binding order (`src/shell/root/app_root/state.rs:144-166` binds `crate::app::keys::page(i)` only for `i < 9`); also the `page as usize` context-menu cast (`src/shell/root/app_root/render.rs:105-116`). |
| 9 | `src/shell/route.rs:10-21` | `VALID_HOSTS` entry: `Page::Foo.host(),`. |
| 10 | `src/shell/root/app_root/state.rs:22-31` | Struct field `foo_page: Entity<FooPage>`. |
| 11 | `src/shell/root/app_root/state.rs:47-56` | Construct in `AppRoot::new` (`cx.new(|cx| FooPage::new(window, cx))`); import at `state.rs:6-9`. |
| 12 | `src/shell/root/app_root/state.rs:211-223` | `unchecked_active_page_view` arm: `Page::Foo => self.foo_page.clone().into()`. |
| 13 | `tests/e2e_navigation.rs:14-19` | Page-count assertion (`assert_eq!(all.len(), 10, ...)`) — bump it. |
| 14 | `locales/en.yml` + `locales/zh-CN.yml` | `nav_foo:` key in BOTH files (`en.yml:53-62` block). Diverging key sets fail `tests/qa_localization.rs:188-198`. |
| 15 | `src/lib.rs` | Nothing for a plain page (`pub use features::pages as views`, `lib.rs:45`, is automatic). Only new top-level modules need a `pub mod` line. |

Optional entries:

- Launcher/command: `src/services/commands.rs` — `CommandId` variant
  (`commands.rs:12-29`), `availability()` arm (`commands.rs:45-99`), registry
  entry via the `command(id, "command_foo", "command_foo_subtitle", icon)`
  helper (`commands.rs:102-200`, helper at `commands.rs:306-313`), `execute()`
  arm (`commands.rs:203-285`, navigate helper `commands.rs:287-289`), plus the
  two `command_foo*` i18n keys in both locales. The palette consumes
  `commands::registry()` automatically (`src/features/command_palette.rs:82`)
  — no palette edit needed.
- Sub-routes: `AppRoute::SettingsNotifications` is the pattern
  (`src/shell/route.rs:52-56`, parse match arm `route.rs:159`).

Page shortcut binding gotcha: `NavigateToPage(usize)` is positional into
`Page::all()` (`render.rs:234-239`); inserting a page mid-list silently
rebinds every later digit key.

## 2. Shared-widget seam — `src/ui/`

Pattern source: `src/ui/widgets/virtual_list.rs`.

| Concern | Where | Convention |
|---------|-------|------------|
| Mount | `src/ui/mod.rs:1` (`pub mod widgets;`) → `src/ui/widgets/mod.rs` `pub mod x;` + `pub use x::{...}` re-export (`mod.rs:1-6`). |
| Shape | Free functions, no struct wrapper (`virtual_list.rs:9-88`). Doc comment states preconditions on public fns (`virtual_list.rs:25-28`). |
| Imports | `use gpui_kit::component::{...}` named items + `use gpui_kit::{prelude::*, *};` (`virtual_list.rs:4-6`). Never `use gpui::` — see §9. |
| Tests | Sibling `x.test.rs` wired `#[cfg(test)] #[path = "x.test.rs"] mod x_test;` (`virtual_list.rs:90-92`). |
| Consumers | Cross-crate tests import the widget directly (`tests/qa_a11y.rs:7` uses `gpui_starter::ui::widgets::virtual_list_item`) — the `pub use` list is public API. |

Kit APIs this pattern sits on (verified): `v_virtual_list` /
`VirtualListScrollHandle` at `gpui-component-0.7.1/src/lib.rs:119`;
`ScrollableElement` trait with `fn scrollbar(...)` at
`gpui-component-0.7.1/src/scroll/scrollable.rs:16-19`.

## 3. Platform-service seam — `src/platform/`

Pattern source: `src/platform/desktop_shell/tray/`.

| Step | Where | Convention |
|------|-------|------------|
| Module decl | `src/platform/mod.rs:1-20` | One `pub mod x;` per service, cfg-gated per OS when needed (`#[cfg(target_os = "windows")] pub mod deep_link_registration;`, `mod.rs:2-3`; wasm-only gates `mod.rs:19-20`). |
| OS family gate | `src/platform/desktop_shell/mod.rs:1-2` | `#[cfg(any(target_os = "macos", target_os = "windows"))] pub mod tray;` + comment naming the excluded platform and why (`mod.rs:3-4`, Linux tray). |
| Per-OS split | `src/platform/desktop_shell/tray/mod.rs:1-9` | `mod macos;` / `mod windows;` each cfg'd; re-export a single `setup` symbol per OS; shared `const LOG` (`tray/mod.rs:11`). |
| Entry point | `src/platform/desktop_shell/tray/macos.rs:73` | `pub fn setup(cx: &mut App)`, called once from boot. |
| Threading rule | `tray/macos.rs:92-96` | OS threads park on blocking recvs and forward over `flume`; `AsyncApp` is `!Send` so threads never `cx.update` — a `cx.spawn` loop drains (`tray/macos.rs:144-153`). |
| Wiring | `src/web.rs` launch closure | `#[cfg(any(target_os = "macos", target_os = "windows"))] crate::tray::setup(cx);` (inside `launch`, after `app::init`). |
| Lib re-export | `src/lib.rs:50-51` | `#[cfg(any(target_os = "macos", target_os = "windows"))] pub use platform::desktop_shell::tray;`. |
| Capability | `src/platform/input/shortcuts.rs:290-305` | `set_capability` helper calling `crate::capabilities::set(...)` with supported/enabled/degraded/reason built from real state. |
| Deps | `Cargo.toml` per-OS tables | macOS `223-248`, Linux `250-266`, Windows `274-300`, Unix `302-303`; wasm table `356-417`. New OS-specific dep goes in its table, never the common one. |

## 4. Events / config / capability recipes

### Events (`src/runtime/events.rs`)

```rust
// emit (events.rs:48-57)
crate::events::emit(crate::events::AppEventKind::Navigate(route), cx);
// error helper (events.rs:59-69)
crate::events::emit_error(err, cx);
```

Kinds: `Navigate(AppRoute)`, `DeepLinkReceived(String)`,
`RemoteCommand(ForwardedCommand)`, `AppError { message, severity }`,
`DiagnosticsChanged`, `Test { message }` (`events.rs:10-24`). AppRoot drains
inside the observer: `cx.observe_global::<events::AppEventQueue>(|this, cx| {
for event in events::drain(cx) { ... } })` (`src/shell/root/app_root/state.rs:63-114`).
`drain` has a read-only early return so observer-driven drains cannot loop
(`events.rs:71-81`).

### Config (`src/state/config_store.rs`)

```rust
// debounced write (300 ms, coalesced) — config_store.rs:274-287
crate::app_state::update_config(cx, |config| { config.field = value; });
// borrowed read — config_store.rs:228-233 (prefer over config(), 214-218)
crate::app_state::with_config(cx, |c| c.field.clone());
```

- `update_config` runs the closure synchronously, then re-`normalized()`s
  (`config_store.rs:280-284`); never write from render — the
  `observe_window_bounds` handler is the model (`state.rs:132-138`).
- `force_save` is the shutdown path only (`config_store.rs:289-314`).
- New field: pub field + `#[serde(default ...)]`, never rename an existing one
  — `tests/snapshot_tests.rs:26,57` pin the serialized schema (comment at
  `config_store.rs:63-65`).

### Capabilities (`src/runtime/capabilities.rs`)

```rust
// registry entry (capabilities.rs:64-79)
crate::capabilities::set("my_feature", CapabilityStatus::supported_enabled(), cx);
// degraded / failed init (capabilities.rs:28-50)
CapabilityStatus::degraded(err) / CapabilityStatus::error(err)
// read (capabilities.rs:81-85) -> BTreeMap<String, CapabilityStatus>
let snap = crate::capabilities::snapshot(cx);
```

Initialize once at startup (`src/app/init.rs:91-93`); always-on features list
in `DEFAULT_ENABLED_CAPABILITIES` (`init.rs:95-108`). Stable snake_case name;
the diagnostics page renders whatever is registered.

## 5. i18n recipe

- Catalogs: `locales/en.yml`, `locales/zh-CN.yml`. Key sets must be identical
  (`tests/qa_localization.rs:188-198`).
- Key grammar (parser `tests/qa_localization.rs:20-71`): top-level
  `snake_case: "double-quoted scalar"`; one nesting level max
  (`parent:` + indented `child: "..."`, `locales/en.yml:35-52`). Names are
  `[a-z0-9_]` only; values must be quoted.
- Naming: `nav_<page>` (`sidebar.rs:36-48`), `command_<x>` +
  `command_<x>_subtitle` (`commands.rs:104-199`), page-prefix everything else
  (`notifications_*`, `notifications.rs:30-35`; `settings_*`, `en.yml:5-25`).
- Facade (`src/services/i18n.rs`):
  `crate::i18n::localize("key")` (`i18n.rs:3-5`);
  `crate::i18n::localize_with_args("key", &[("name", val)])` with `%{name}`
  placeholders (`i18n.rs:7-13`; call site `src/app/init.rs:379-383`).
- New locale FILE needs no registry — the crate-root
  `rust_i18n::i18n!("locales", fallback = "en")` embeds everything
  (`src/lib.rs:24-26`, `CONTRIBUTING.md:85`); a new locale also needs
  `src/app/locale.rs` + `KNOWN_LOCALES` + `normalized()` work
  (`CONTRIBUTING.md:86-87`, check at `config_store.rs:145-147`).
- Scanner blind spot: `qa_localization.rs:77-128` only resolves
  string-literal first arguments. `localize(self.title_key())`
  (`sidebar.rs:52`) is invisible to the guard — a missing `nav_*` key renders
  as the key itself at runtime (`i18n.rs:2`), so catalog edits for indirect
  calls are on you.

## 6. Wasm rules

- Dep tables (`Cargo.toml`): workspace tokio carries only `rt, sync, time`
  (`Cargo.toml:30-35`). Native-only table
  `[target.'cfg(not(target_family = "wasm"))'.dependencies]`
  (`Cargo.toml:310-353`): single-instance, interprocess, network-interface,
  keyring, rusqlite, notify-rust, open, arboard, tracing-appender, tokio
  `net/io-util/rt-multi-thread`, and the gpui-kit tree-sitter grammar
  features. Wasm table (`Cargo.toml:356-417`): `wasm-bindgen =0.2.125`
  (exact pin, CLI must match — `Cargo.toml:357-360`), `web-time`,
  `chrono` `wasmbind`, `web-sys =0.3.102` feature union, `js-sys =0.3.102`.
- `just wasm-check` = `cargo +nightly check --target wasm32-unknown-unknown
  -p gpui-starter --lib` (`justfile:24-25`). Lib only — no tests, no bin.
  Nightly required: wasm_thread 0.3.3 uses
  `#![feature(stdarch_wasm_atomic_wait)]` (`justfile:19-22`).
- CI has no wasm job (grep for "wasm" across `.github/workflows/*.yml`:
  zero hits). wasm-check is a local gate only.
- Clocks: never `std::time::Instant/SystemTime` in boot/render paths — they
  compile on wasm and panic at runtime. Use `crate::platform::clock::Instant`
  (`src/platform/clock.rs:1-8`; call sites `src/app/init.rs:12-13,25-26`,
  `src/shell/root/app_root/render.rs:32-33`).
- Known stubs/degradations: config save is a no-op on wasm
  (`config_store.rs:521-528`) with `AppPaths::fallback()`
  (`config_store.rs:169-181`); SQLite/migrations native-only
  (`src/lib.rs:47-49`, `src/app/init.rs:220-260`); web bridges (hash router,
  favicon/title, connectivity) install via
  `crate::platform::web::install(cx)` (`src/app/init.rs:284-287`); navigation
  pushes `location.hash` (`state.rs:246-247`, `route.rs:99-118`); embedded
  fonts must register before first render (`src/app/init.rs:29-47`); gallery
  editors render without grammars on wasm (`Cargo.toml:316-318`).
- Degradation convention: prefer a CapabilityRegistry entry with
  `supported: false`/`degraded: true` over a compile failure; only deps with
  no wasm32 story go behind the not-wasm table + cfg fences
  (`Cargo.toml:305-309`).

## 7. Test conventions

- Unit tests: sibling `<name>.test.rs` next to the module, wired
  `#[cfg(test)] #[path = "<name>.test.rs"] mod <name>_test;`
  (`src/shell/route.rs:171-173`, `src/state/config_store.rs:581-583`,
  `src/ui/widgets/virtual_list.rs:90-92`; `CONTRIBUTING.md:110`).
- `#[gpui_kit::test]` IS available for in-crate unit tests:
  `src/state/config_store.test.rs:280-281` (async fns, `cx: &TestAppContext`),
  `src/gallery/sections/theme/brush.test.rs:5-6` (`cx: &mut TestAppContext`).
  Enabled by the dev-dependency `gpui` (gpui-pre) with `test-support`
  (`Cargo.toml:212-216`); the macro and `TestAppContext` flow through the
  kit's `pub use ::gpui::*` glob (`gpui-kit-0.7.1/src/lib.rs:84-97`). Import
  test types explicitly — glob-importing `#[test]`-shadowing macros caused
  the old stack overflow (`src/lib.rs:1-2`, `.github/workflows/ci.yml:15-19`).
- The kit's own `gpui_kit::test` module (TestWindowExt, ElementSnapshot —
  `gpui-kit-0.7.1/src/lib.rs:104-107`, `TESTING.md`) requires the kit's
  `test-support` feature (`gpui-kit-0.7.1/Cargo.toml [features]
  test-support`), which this repo does not enable. Unused here; see Open
  Questions.
- `tests/` (cross-crate, plain `gpui_starter::` API): `e2e_navigation.rs`
  (pages/routes, no GPUI context), `e2e_lifecycle.rs` (smoke, plain
  `#[test]`), `snapshot_tests.rs` (insta yaml; `Cargo.toml:80` insta with
  `yaml`), `qa_migration.rs`, `qa_localization.rs`, `qa_a11y.rs`,
  `qa_docs.rs`.
- CI runs `cargo test -p gpui-starter --lib` only
  (`.github/workflows/ci.yml:245,285`) — the `tests/` integration targets are
  compiled by the check job but NOT executed in CI; the full
  `cargo test --workspace --all-features` is the local/AGENTS.md gate
  (`AGENTS.md` Rust quality gates, `ci.yml:59-74` note).

## 8. Gates

Local (AGENTS.md, mandatory before "done"):

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-features
cargo test --workspace --all-features
```

just recipes (`justfile`): `just fmt` = write mode (cargo sort-derives, cargo
fmt, taplo fmt, rumdl — NOT a check, `justfile:7-11`); `just clippy` =
`cargo clippy --workspace --all-features` (`justfile:13-14`); `just test` =
`cargo test --workspace --all-features` (`justfile:44-45`); `just check` (`justfile:16-17`);
`just wasm-check` (`justfile:24-25`, see §6).

CI (`.github/workflows/ci.yml`, nightly toolchain everywhere): `cargo check`
+ `cargo check --release` (`ci.yml:47,56` — the release pass exists because
the dev profile is blind to `#[cfg(not(debug_assertions))]` code,
`ci.yml:49-55`); `cargo clippy -- -D warnings` (`ci.yml:91`); `cargo fmt
--check` (`ci.yml:104`); `cargo test -p gpui-starter --lib` (Linux
`ci.yml:245`, Windows `ci.yml:285`); Linux check lane repeats both checks
(`ci.yml:136,139`). Note CI clippy is narrower than the AGENTS.md gate (no
`--workspace --all-features`) — passing CI does not prove the repo gate.

## 9. Rules a big feature could accidentally break

| Rule | Enforced by | What breaks it |
|------|-------------|----------------|
| Kit facade only: no `use gpui::`, `gpui_platform::`, `gpui_component::`, `gpui_kit_assets::` in src/, build.rs, tests/ | `tests/qa_migration.rs:386-405` | Any hand-written import bypassing `gpui_kit` / `gpui_kit::component` / `gpui_kit::base` / `gpui_kit::assets` / `gpui_kit::platform`. |
| No direct `accesskit` dependency | `Cargo.toml:18-19` | Kit re-exports gpui's accesskit 0.24; a second copy is type-incompatible. Use `gpui_kit::accesskit` (`tests/qa_a11y.rs:6`). |
| One kit/gpui family in the lock | `tests/qa_migration.rs:131-146,188-213` | Any new dep pulling a second `gpui-pre*`/`gpui-component`/`gpui-kit-assets` copy (links conflict). |
| Layer crates absent from Cargo.toml | `tests/qa_migration.rs:103-125` | Declaring `gpui_platform`/`gpui-component`/`gpui-kit-assets` directly. |
| Cargo.lock committed with manifest changes | `tests/qa_migration.rs:167-186` (checks HEAD blob) | Editing deps without staging the regenerated lock. |
| gpui-query patch line untouched | `tests/qa_migration.rs:238-267`, `Cargo.toml:436-449` | Moving the `[patch.crates-io]` rev or activating two lines. |
| `AppConfig` serde schema stable | `tests/snapshot_tests.rs:26,57`; comment `config_store.rs:63-65` | Renaming/repurposing fields; adds need `#[serde(default)]`. |
| Locale key lockstep | `tests/qa_localization.rs:145-198` | Adding a key to one catalog, or a quoted-scalar/nesting violation. |
| Sidebar page count | `tests/e2e_navigation.rs:14-19` | Any page add/remove without updating the count. |
| Docs sentinels | `tests/qa_docs.rs:1-15` | Deleting/renaming "second-instance forwarding", "open logs folder", "secure storage unavailable path" from `docs/qa-matrix.md`, or "Keyboard And Focus"/"Done Criteria" from `docs/accessibility-checklist.md`. |
| A11y list pattern | `src/shell/root/app_root/render.rs:210-213` comment; `src/ui/widgets/virtual_list.rs:54-88`; `notifications.rs:84-124` | Lists need `Role::List` + `aria_size_of_set` on the nearest ancestor; items `Role::ListItem` + `aria_position_in_set`/`aria_size_of_set` (0-based; AT reports +1). Headings that announce changes need `a11y_live` (`notifications.rs:50-53`, `render.rs:149-158`). |
| No `std::time` clocks in boot/render | `src/platform/clock.rs:1-8` | Direct `Instant::now()` — runtime panic on wasm. |
| Tracing targets `gpui_starter::<module>` | `src/runtime/events.rs:51`, `config_store.rs:66` | Inconsistent target strings. |
| Off-input-thread window access | `src/app/window.rs:7-38` | Tray/shutdown code must use the `RootWindow` global (`root_window(cx)`), not `active_window()` (Windows GetActiveWindow thread-local). |
| OS threads never touch `cx` | `tray/macos.rs:92-96` | `AsyncApp` is `!Send`; forward via flume into `cx.spawn`. |
| No `recursion_limit` restoration | `src/lib.rs:1-2` | Fix glob-shadowed test imports at call sites instead. |
| Comment/style/commit rules | `AGENTS.md` | Comments >2 lines, diff-narrating comments, non-conventional commits, banned words. |

## Open Questions

- `gpui_kit::test` module tools (TestWindowExt, ElementSnapshot,
  `gpui_kit::test` namespace): the kit's `test-support` feature exists
  (`gpui-kit-0.7.1/Cargo.toml [features]`) but this repo enables only the
  gpui crate's `test-support` via the dev-dep (`Cargo.toml:216`), so the
  module is cfg'd out. Not verified runnable here (no build was run); plans
  needing element snapshots should budget enabling the kit feature or use
  `#[gpui_kit::test]` + `TestAppContext` as `src/state/config_store.test.rs`
  does.
- Novel `IconName` variants: the enum is generated at build time from
  `assets/icons/*.svg` filenames (kebab-case → PascalCase;
  `gpui-kit-assets-0.7.1/src/icon.rs:3` `include!`s the generated file,
  `build.rs` reads `assets/icons`, lucide 1.43.0, 1818 icons). Variants used
  by this repo were confirmed by their svg files (`inbox.svg`,
  `layout-dashboard.svg`, `gallery-vertical-end.svg`, `settings-2.svg`,
  `triangle-alert.svg`, `redo-2.svg`). An unused-variant name in a plan is
  verified only by compiling; check the svg filename exists under
  `gpui-kit-assets-0.7.1/assets/icons/` first.
- No `cargo` gate was run during this survey (doc-only scope); all
  command listings are quoted from `AGENTS.md`, `justfile`, and
  `.github/workflows/ci.yml`, not executed here.
