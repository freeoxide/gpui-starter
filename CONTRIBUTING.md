# Contributing to GPUI Starter

Thanks for your interest in contributing. This guide covers everything you need to get started.

## Development Setup

**Requirements:**

- macOS (the app uses macOS-specific APIs for tray icons, hotkeys, and notifications)
- Rust nightly toolchain

**Install the nightly toolchain:**

```sh
rustup toolchain install nightly
```

**Clone and build:**

```sh
git clone https://github.com/freeoxide/gpui-starter.git
cd gpui-app
cargo build
```

The first build takes a while because it compiles GPUI and all dependencies. Subsequent builds are faster thanks to incremental compilation.

**Marketing site (optional):** the Astro docs site lives entirely in `web/` and is independent of the desktop app — `cd web && bun install && bun run docs:dev` to work on it. Boilerplate users can delete `web/` and `.github/workflows/deploy-docs.yml` without affecting the app.

## Development Workflow

Use the provided shell script for fast iteration. It builds the binary, wraps it in a `.app` bundle, and signs it locally:

```sh
bash scripts/macos-dev-app.sh
```

Open the printed `.app` path to run the app. Repeat after each code change.

## Code Style

- Run `cargo fmt` before committing. All formatting decisions go through rustfmt.
- Run `cargo clippy` and fix any warnings. The CI pipeline will reject code that triggers clippy lints.
- Match the existing patterns in the codebase. Look at nearby files for conventions on imports, module structure, error handling, and naming.

## Commit Messages

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add user preference for default locale
fix: resolve crash when sidebar is toggled rapidly
docs: update CONTRIBUTING.md with theme guide
refactor: extract notification logic into its own module
test: add unit tests for route matching
chore: bump gpui-component dependency
```

Keep the subject line under 72 characters. Use the body for anything that needs explanation beyond the diff.

## How to Add Things

### New Page

1. Create a new file in `src/features/pages/` (e.g. `src/features/pages/my_page.rs`).
2. Implement the page using the gpui-kit component patterns you see in existing pages like `home.rs` or `settings/mod.rs`.
3. Declare the module and re-export the page type in `src/features/pages/mod.rs`.
4. Add a `Page` variant in `src/shell/sidebar.rs`. The exhaustive `title`, `host`, and `icon` matches plus the `all()` list mark every spot to fill in, and `host()` feeds the deep-link hosts in `src/shell/route.rs`.
5. Construct the entity in `src/shell/root/app_root/state.rs` and return it from `unchecked_active_page_view`.

### New Command

1. Add a `CommandId` variant and its `CommandSpec` in `src/services/commands.rs`.
2. Handle it in the `ExecuteCommand` dispatch in `src/app/init.rs`.
3. Bind an in-app shortcut with `KeyBinding` in `src/app/init.rs` (shared keystroke constants live in `src/app/keys.rs`), or an OS-global hotkey in `src/platform/input/shortcuts.rs`.

### New Theme

1. Create a JSON file in `themes/` (e.g. `themes/my-theme.json`).
2. Follow the structure of an existing theme file like `themes/gruvbox.json` or `themes/tokyonight.json`.
3. The theme will be discoverable by filename at runtime.

### New Locale

1. Add a `locales/<code>.yml` file mirroring the keys of `locales/en.yml` (e.g. `locales/fr.yml`). The crate-root `rust_i18n::i18n!` macro in `src/lib.rs` embeds every file under `locales/` at compile time; there is no locale registry to edit.
2. Add a locale constant in `src/app/locale.rs` (alongside `LOCALE_EN`, `LOCALE_ZH_CN`), add it to `KNOWN_LOCALES` in `src/state/config_validation.rs`, and extend the locale check in `AppConfig::normalized()` (`src/state/config_store.rs`); unknown locales are reset to English on load.
3. Add the locale to the language menus; the settings and home pages enumerate the supported locales explicitly.

## Pull Request Process

1. **Fork** the repository and create a branch from `master`:
   ```sh
   git checkout -b feat/my-feature
   ```
2. **Commit** your changes with a conventional commit message.
3. **Push** to your fork and open a pull request against `master`.
4. Ensure `cargo fmt --check`, `cargo clippy`, and `cargo test` all pass locally before pushing.
5. Describe what the PR does and why in the description. Link any related issues.

Maintainers will review and merge. Small, focused PRs are easier to review and land faster.

## Testing

Run the full test suite:

```sh
cargo test
```

Add tests for any new functionality. Cross-crate tests live in `tests/` (see `qa_migration.rs` or `snapshot_tests.rs` for the patterns). Unit tests sit in a sibling `*.test.rs` file wired up with `#[cfg(test)] #[path = "..."] mod ...;` next to the code under test, following the existing modules.

## Architecture

For a high-level overview of the codebase structure, modules, and data flow, see [docs/gpui-architecture.md](docs/gpui-architecture.md).
