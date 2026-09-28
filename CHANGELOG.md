# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `tests/qa_migration.rs` locks the migration's dependency-graph invariants: kit 0.7.0 + `gpui-pre` 0.3.7 manifest pins, exactly one gpui-kit family and one unified `gpui-pre` snapshot in the lockfile, and the vendored `gpui-form` path patch

### Changed

- Migrated the GPUI foundation to **gpui-kit 0.6.0** from crates.io, built on `gpui-pre` (the published snapshot of Zed's gpui) — the whole graph resolves to exactly one gpui copy and the zed git `[patch]` entries are gone
- Bumped the gpui-kit ecosystem (`gpui-kit`, `gpui-component`, `gpui-kit-assets`) to **0.6.1**; the pinned `gpui-form` rev's caret requirement unifies on the same checkout, and 0.6.1's slimmed optional deps drop syntect/jni/rustls-platform-verifier from the lockfile
- `gpui-form` now comes from canonical upstream `stayhydated/gpui-form` master @`216af496` (git pin); `koruma` 0.11 and `es-fluent` 0.18.1 follow upstream's stayhydated pins
- `gpui-query` resolves through a `[patch.crates-io]` override to our `gpui-pre-0.6` fork branch (a one-line manifest swap onto `gpui-pre`); no library sources are vendored
- Ported the form page to the gpui-form 0.6 API: component paths via `gpui_form_collection`, `value_change` subscriptions, koruma bare-path validators; generated form members now use raw field names (`name` instead of `name_input`)
- `i18n::localize` now takes a `&'static str` message id, and es-fluent 0.18 renames the variants-enum label message suffix `_this` to `_label`
- Migrated the gpui-kit ecosystem (`gpui-kit`, `gpui-component`, `gpui-kit-assets`) 0.6.4 → **0.7.0**, with `gpui` + `gpui_platform` (`gpui-pre` family) 0.3.5 → **0.3.7**; kit 0.7 pins the whole `gpui-pre` snapshot family at `=0.3.7`
- Vendored `stayhydated/gpui-form` @`216af496` into `vendor/gpui-form` and wired it through a `[patch]` to the local paths, with the vendored kit pins bumped to 0.7.0 — no upstream rev targets kit 0.7 (master 0.7.1 still pins `gpui-kit = "0.6.6"`), and the git pin under kit 0.7.0 pulls a second `gpui-kit-assets` into the graph, which cargo's `links` one-copy rule rejects; drop the vendor tree and patch table and re-pin the git rev once upstream ships a kit-0.7 rev
- The main window opens through `gpui_kit::open_window` after `gpui_kit::init`; the launcher keeps the manual `cx.open_window` + `Root::new().bg(transparent_black())` path, because the kit's root plugin paints an opaque background that only an instance style refinement overrides
- Deleted the manual sheet/dialog/notification layer rendering (`Root::render_*_layer` is gone in 0.7); the root plugin auto-hosts every overlay layer above app content
- Theme mutations go through `Theme::update` / `Theme::set_scrollbar_mode`, which reconcile tokens, rebuild the Base projection, and refresh every window — no `Theme::global_mut` mutation sites remain

## [0.3.0] - 2026-06-05

### Added

- Auto-updater with signed manifests and Ed25519 verification
- Crash report generation and storage for application panics
- Frame-time debugger for performance profiling
- Background WebSocket support (optional feature)
- gpui-query async data fetching library (TanStack Query-inspired)
- gpui-query-v2 next-gen async state management
- Vendored gpui-component library from longbridge
- Vendored gpui-form for declarative form handling
- Error boundary with action-based triggers
- Release workflow with codesigning and notarization
- Deploy workflow for Cloudflare Pages

### Changed

- 60 new blog posts covering GPUI development topics
- Improved documentation coverage across all pages
- Updated FAQ entries with better SEO targeting
- Enhanced structured data and meta optimization
- Internal linking improvements across all pages

## [0.2.0] - 2025-06-15

### Added

- Desktop notification system with native OS backends, in-app toast fallback, and persistent inbox
- Diagnostics page showing live app state, subsystem status, and debug actions
- Telemetry module with disabled/local/remote modes and consent gate
- Secure storage module with OS keyring integration (macOS Keychain, Windows Credential Manager, Linux Secret Service)
- Single-instance guard with IPC forwarding for deep links
- First-run detection and setup experience
- Undo/redo stack with command pattern and keyboard shortcuts (Cmd+Z/Cmd+Y)
- Custom title bar replacing native window chrome with drag regions and traffic light support
- Global keyboard shortcuts (Alt+Space hotkey, Cmd+K launcher)
- Background task manager for async operations
- Connectivity state monitoring with network probes
- File logging with tracing-appender
- Capabilities registry for runtime feature detection
- Lifecycle state machine for app startup/shutdown/crash handling
- Configuration migration system for schema changes

### Changed

- Expanded documentation site with 5 new reference pages (command launcher, notifications, secure storage, routing, testing)
- Added 12 new blog posts covering GPUI development topics
- Added 8 new FAQ entries across Features and Advanced categories
- Updated llms.txt with comprehensive feature coverage

## [0.1.0] - 2025-05-15

### Added

- **Multi-page architecture** with sidebar navigation and page routing via GPUI
- **21 built-in themes** with live hot-reloading and custom theme support
- **Internationalization (i18n)** supporting English and Chinese (zh-CN) via `es-fluent`
- **Form validation** with `gpui-form` derive macros and `koruma` validation rules
- **Command launcher** (Cmd+K) with fuzzy search across all app actions
- **macOS system tray** integration with app icon and quick-access menu
- **Secure credential storage** via OS keychain (`keyring` crate)
- **SQLite database** integration with `rusqlite` for local data persistence
- **Animated app preview** component with Three.js wireframe scenes
- **Documentation site** built with Astro Starlight
- **Custom marketing landing page** with 3D animations and glassmorphism design
- **Privacy policy** and **terms of use** pages

### Changed

- Moved web sources from `src/web` to `web` directory for cleaner project structure
- Enabled `apple-native` feature for keyring on macOS
- Switched to blocking `reqwest` for connectivity checks

### Fixed

- Animation now pauses on hover for better UX
- Added logging to secure storage operations for debugging
