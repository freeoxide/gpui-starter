# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `tests/qa_migration.rs` locks the migration's dependency-graph invariants: kit 0.7.0 + `gpui-pre` 0.3.7 manifest pins, exactly one gpui-kit family and one unified `gpui-pre` snapshot in the lockfile, the freeoxide `gpui-form` rev pin, single-copy `koruma` 0.9 / `es-fluent` 0.16 registry satellites, and the temporary one-entry `[patch]` override whose post-merge removal flips exactly one test

### Changed

- Migrated the GPUI foundation to **gpui-kit 0.6.0** from crates.io, built on `gpui-pre` (the published snapshot of Zed's gpui) — the whole graph resolves to exactly one gpui copy and the zed git `[patch]` entries are gone
- Bumped the gpui-kit ecosystem (`gpui-kit`, `gpui-component`, `gpui-kit-assets`) to **0.6.1**; the pinned `gpui-form` rev's caret requirement unifies on the same checkout, and 0.6.1's slimmed optional deps drop syntect/jni/rustls-platform-verifier from the lockfile
- `gpui-form` is re-pinned to the freeoxide fork @`f7e2fb0b`, the `sync/gpui-kit-0.7.0` migration branch HEAD (no stayhydated rev targets kit 0.7; PR merge pending); the run-1 vendoring is retired and a temporary `[patch]` resolves the unpushed rev from the local sibling checkout — delete the table after the PR merges. `koruma` 0.9 and `es-fluent` 0.16 follow the migrated graph's single-copy registry lines
- `gpui-query` resolves through a `[patch.crates-io]` override to our `gpui-pre-0.6` fork branch (a one-line manifest swap onto `gpui-pre`); no sources are vendored for this override
- Ported the form page to the gpui-form 0.5.2 API: `#[gpui_form(component(input))]` declarations, koruma 0.9 builder validators, `{field}_input` generated members, `InputEvent::Change` subscriptions; two adaptations are intentional — empty fields now fail the codegen-added required validator, and `website` is required-enforced at the holder level
- The i18n service is ported to es-fluent 0.16 (`localize` keeps its `&'static str` id and resolves through `localize_in_domain`): an invalid message id now falls back to the id string instead of failing the retired 0.18 static registry's eager key check
- Migrated the gpui-kit ecosystem (`gpui-kit`, `gpui-component`, `gpui-kit-assets`) 0.6.4 → **0.7.0**, with `gpui` + `gpui_platform` (`gpui-pre` family) 0.3.5 → **0.3.7**; kit 0.7 pins the whole `gpui-pre` snapshot family at `=0.3.7`
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
