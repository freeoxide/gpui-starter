# 24 Auth UI and Account Shell Plan

## Goal

A "Sign In" sidebar page composes plan 21's `AuthController` into user-facing
auth: provider buttons from a registry (email always; Google/GitHub enabled
when their provider is registered), an email/password form built with
gpui-form + koruma, and loading/error/disabled states per the design guides.
The title bar grows an account menu whose signed-in trigger shows the
account initial, with an email header and sign out (the `Avatar` element
renders on the sign-in page and Settings account cards; the dropdown
trigger is Button-only in the kit and cannot host an Avatar — Research
Digest). Settings replaces its session demo buttons with a real account
section, Diagnostics shows auth state/provider/token expiry/store backend,
and every string lands in both locale catalogs with keyboard paths for
every control. Pure composition: no mutation path exists except
`crate::services::auth::controller(cx)`.

## Dependencies

Plan `21-auth-foundation.md` (read in full this session; its Interface
Contract items consumed):

- `AuthController` + `AuthState` + `AuthEvent` (contract lines 474-509):
  `state()`, `provider_id()`, `start_sign_in`, `complete_callback`, `cancel`,
  `refresh`, `restore`, `sign_out`, all through the entity.
- `AuthHandle`/`install`/`controller`/`handle_callback_link` app adapter
  surface (contract lines 525-541): title bar, settings, diagnostics, and the
  page mutate only via `controller(cx)`; the demo-completion control routes
  through `handle_callback_link`.
- `KeyringStore` (contract lines 538-540): this plan constructs the unit
  struct directly and calls `TokenStore::load(provider_id, cx)` read-only to
  fill the status snapshot (token expiry, account email). No writes.
- `MockProvider` behavior (contract lines 511-522): the foundation-only demo
  path — `start_sign_in` emits `AuthorizationRequested { url }` with
  `urn:gpui-auth:mock:{state}`, which this plan converts back into a
  `gpui-starter://auth/callback?code&state` link.
- Requirement on 21: `install_with(store, io, cx)` (21 Step 8.2 extracts it)
  must be `pub(crate)` — this plan's tests reuse it to install with
  `InMemoryStore`. If it shipped private, one-word widening is the only edit
  inside 21's files (Open Questions).

Repo services consumed (read this session): `src/services/session.rs`
(`SessionState` 3-12, `set_state` 31-36 — mirrored by 21's observer, this plan
never calls it), `src/services/secure_storage.rs` (read via `KeyringStore`
only), `src/services/desktop_actions.rs` (untouched; browser opening already
lives in 21's adapter), `src/runtime/events.rs` (`AppEventKind::Navigate`,
line 11 — command + menu navigation), deep links (`src/shell/route.rs`
`VALID_HOSTS` 10-21 — new page host; the `auth` callback host stays 21's,
consumed before route parsing). Plans 22 and 23 are on disk and consumed by
their Interface Contracts where noted: 22's `EMAIL_PROVIDER_ID == "email"`
(22:297, 564) and password-controller methods (the email path below routes
through `sign_in_with_password` once 22 lands, not `start_sign_in` — the
EmailPasswordProvider has no browser flow); 23's provider ids
`"google"`/`"github"`/`"oidc"` (23:390, 694) and its per-provider
controllers (23:26) — this plan degrades per provider when either is absent
and works with the foundation alone. Plan 15's `tutorial::target` contract (15-tutorial-mode.md
lines 119-127) is consumed in a conditional final phase (module absent on
disk today: `src/features/mod.rs` lists `command_palette`, `pages`,
`palette` only).

## Research Digest

Repo facts (all read this session, `path:line`):

- Title-bar seam: `AppTitleBar` struct at `src/shell/title_bar.rs:17-21`,
  construction at 24-38, right cluster (settings dropdown, search, bell) at
  63-111; `SettingsDropdown` (`title_bar.rs:116-205`) is the entity pattern
  this plan's account menu copies: `FocusHandle` + `track_focus` +
  `on_action` listeners + a `Button` with `.dropdown_menu(move |menu, ...|)`
  + `.anchor(Anchor::TopRight)` (title_bar.rs:167-203).
- Settings placeholders being replaced: session demo buttons at
  `src/features/pages/settings/dev_sections/runtime_sections.rs:144-176`
  (`session-sign-in`, `session-sign-out`, `session-error-demo` writing
  `SessionState` directly). Section helpers `settings_card_base`
  (`dev_sections/mod.rs:24`) and `section_heading` (35-50) are already
  `pub(crate)`, reachable as `super::dev_sections::…` from a sibling
  `settings` module; section call list at `settings/mod.rs:164-178`;
  subscription pattern `observe_global_in` at `settings/mod.rs:28-72`.
- Diagnostics: `row("diagnostics_session", &format!("{:?}", session.state))`
  at `src/features/pages/diagnostics/rows.rs:161`; `row`/`capability_row`
  helpers at `diagnostics/mod.rs:177-186`; the page's observer list
  (`observe_global_in` per snapshot global) at `diagnostics/mod.rs:20-79`.
  New rows need a matching observer for the new snapshot global.
- Form precedent: `src/features/pages/form_page.rs` — `GpuiForm` +
  `Koruma` derives with `#[gpui_form(component(input))]` and
  `#[koruma(EmailValidation::<_>::builder())]` / `NonEmptyValidation` (lines
  20-42); per-field subscription via `InputEvent::Change` + `set_value` with
  empty→`None` mapping (156-176); cached per-field error keys (208-270);
  `field().label(..).required(..).description_fn(..)` + `Input::new(&entity)`
  (274-305); submit gating + notifications (380-412).
- gpui-form codegen (verified in the pinned checkout
  `~/.cargo/git/checkouts/gpui-form-164d1babfe924b98/710b524`): label keys
  are generated as `"{snake_case_struct}.{field}_label"` with consts like
  `EMAIL_LABEL_KEY` on the `{}FormValueHolder` type
  (`crates/gpui-form-derive/src/derives/gpui_form/value_holder.rs:723-738`).
  For `struct SignInForm` that is `sign_in_form.email_label` /
  `sign_in_form.password_label`. The nested-block grammar already exists in
  the catalogs (`locales/en.yml:35-44` `registration_form:` block).
- i18n: facade `crate::i18n::localize` / `localize_with_args`
  (`src/services/i18n.rs:2-13`); key grammar + lockstep test
  (`tests/qa_localization.rs:188-198`, parser 20-71); scanner blind spot for
  indirect calls (survey §5) — `nav_sign_in` is added by hand to both files.
- Commands seam: `CommandId` enum `src/services/commands.rs:11-29`,
  `availability` match 45-99, `registry()` + `command(id, key, key, icon)`
  helper 102-198 + 306-313, `execute` match 203-283, `navigate` helper
  287-289. Palette consumes `registry()` automatically.
- Page seam (survey §1, re-verified): `Page` variant + `title`/`title_key`/
  `host`/`icon`/`all()` (`src/shell/sidebar.rs:19-105`), `VALID_HOSTS`
  (`src/shell/route.rs:10-21`), AppRoot field/construct/view arm
  (`src/shell/root/app_root/state.rs:17-38, 46-56, 211-223`), page count
  `tests/e2e_navigation.rs:14-33`. Digit keys bind only `i < 9`
  (`state.rs:147-158`), so an appended page at index >= 10 rebinds nothing.
- Boot order: plan 21 inserts `crate::services::auth::install(cx)` after
  `crate::storage::initialize(cx);` inside `runtime_services_init`
  (`src/app/init.rs:202-219`; 21 Seam Touches row 7). This plan's status
  service initializes immediately after that line.

gpui/gpui-kit facts, verified against the pinned registry sources this
session (kit 0.7.3 → locked gpui-component/gpui-base/gpui-kit-assets 0.7.3
per `Cargo.lock`; located with
`ls ~/.cargo/registry/src/*/ | grep -E 'gpui-pre-0.3.9|gpui-kit-0.7.3'`):

- `Button::loading(bool)` (`gpui-component-0.7.3/src/button/button.rs:445`)
  and `loading_icon` (474) — the sign-in submit's loading state. `Disableable`
  is `fn disabled(bool)` (`gpui-base-0.7.3/src/component_traits.rs:39-41`),
  implemented by Button at `button.rs:534`; `Selectable::selected` for the
  open-menu trigger state (`component_traits.rs:12-37`).
- `DropdownMenu` trait with `fn dropdown_menu(self, build)` at
  `gpui-component-0.7.3/src/menu/dropdown_menu.rs:12-22`, implemented for
  Button (`dropdown_menu.rs:33`), SidebarHeader (`sidebar/header.rs:77`), and
  SidebarFooter (`sidebar/footer.rs:67`) — no Avatar-capable generic-content
  trigger among them, so the account trigger is a Button carrying the user's
  initial as its label, exactly like `SettingsDropdown`. Menu builder:
  `.label(text)` header
  (`menu/popup_menu.rs:494`), `.menu(label, Box<dyn Action>)` (467),
  `.separator()` (682).
- Button content API is `label` + `icon` only: `pub fn label(impl Into<
  SharedString>)` (`button.rs:373`, no label = icon-button mode) and
  `pub fn icon(impl Into<ButtonIcon>)` (`button.rs:399`);
  `ButtonIcon` converts from `Icon`/`Spinner`/`ProgressCircle` only
  (`button/button_icon.rs:55-59`, blanket `From` at 19-23) — no `Avatar`
  conversion, and
  `DropdownMenu` has no non-Button implementor, so an `Avatar` element
  cannot sit inside a dropdown trigger. Consequence for the title bar: the
  signed-in trigger carries the account initial as its Button `label`
  (single glyph), while the `Avatar` element serves the sign-in page card
  and the settings section where composition is unconstrained.
  `accessibility_label` replaces the announced name for exactly this case —
  a Button whose visible text is not its real name (doc at
  `button.rs:381-393`).
- `Input::mask_toggle()` (`gpui-component-0.7.3/src/input/input.rs:322-325`)
  renders the eye/eye-off toggle itself; `Input::content_type(
  InputContentType::Password)` (input.rs:328-333, variant at
  `input/content_type.rs:75`) sets the a11y role to `Role::PasswordInput`
  (input.rs:42). Masked-by-default via
  `InputState::set_masked(true, window, cx)`
  (`gpui-base-0.7.3/src/input/base/state.rs:10952`). `InputState::new(window,
  cx)` (state.rs:10922), `.placeholder` (904), `set_value` (1038),
  `InputEvent::{Change, PressEnter { secondary, shift }, Focus, Blur}`
  (state.rs:127-132) — `PressEnter` is the keyboard submit path.
- `Avatar` (`gpui-component-0.7.3/src/avatar/avatar.rs:26-66`):
  `Avatar::new().name(impl Into<SharedString>)` derives initials
  automatically (`extract_text_initials`, avatar.rs:48-52) — the
  avatar-or-initial requirement with no image asset. `Sizable` (avatar.rs:62).
- Default icon set (`gpui-kit-assets-0.7.3/default-icons.txt`, 106 entries —
  INDEX §4 pin-update note says 0.7.1→0.7.3 diff is identical): contains
  `github`, `user`, `circle-user`, `globe`, `eye`, `eye-off`, `loader-circle`,
  `loader`, `triangle-alert`, `circle-alert`, `external-link`. Provider icons:
  GitHub → `IconName::Github` (real glyph); Google → `IconName::Globe`
  stand-in (no brand glyph in the default set; INDEX stand-in precedent);
  Email → `IconName::User`. No `log-out`/`mail`/`lock` exists — menu items
  and text buttons carry no icon instead of a wrong one.
- `Avatar` is at `gpui_kit::component::avatar::Avatar` (module list,
  `gpui-component-0.7.3/src/lib.rs:32`); title_bar.rs already imports
  component submodules the same way (title_bar.rs:1-6).

No third-party crates are added: gpui-form, koruma, koruma-collection,
chrono (RFC3339 expiry formatting), gpui-auth are all existing dependencies
(`Cargo.toml:6, 52-53, 176, and 21's Seam Touches row 2`). `DateTime::
from_timestamp(secs, 0).to_rfc3339()` is pure computation (no clock), so the
seams §6 no-`std::time` rule is untouched.

No cargo gate was run during planning (doc-only scope, matching the surveys
and plans 01-23). Every Verification command is for the implementer.

## Architecture

State ownership — this plan owns views and one read-side mirror; all auth
mutations stay in 21's controller:

- `AuthUiState` (new GPUI global, `src/services/auth_ui.rs`) holds
  `snapshot: AuthUiSnapshot` plus the two live subscriptions (observer on the
  controller entity, subscriber on `AuthEvent`). It re-mirrors
  `Entity<AuthController>` → `AuthUiSnapshot` the same way 21's adapter
  mirrors `AuthState` → `SessionSnapshot` (`session.rs` pattern). It is the
  single read surface for title bar, settings, diagnostics, and the page.
- `SignInPage` (`src/features/pages/sign_in/`) owns only presentation state:
  `Mode { Choose, Email }`, the gpui-form value holder + input entities,
  cached validation keys, focus handle, subscriptions. It reads
  `auth_ui::snapshot(cx)` and mutates through `auth::controller(cx)`.
- `AccountMenu` (new entity in `src/shell/account_menu.rs`) owns a
  `FocusHandle` + an `AuthUiState` observer; renders one of four states
  (signed out / signing in / signed in / error) around a Button +
  `dropdown_menu`.

Data flow, sign-in (foundation build, mock provider):

```text
SignInPage submit ──controller(cx)?.update(start_sign_in)──▶ AuthController
  (21) state=SigningIn, emit AuthorizationRequested{ urn:gpui-auth:mock:{state} }
auth_ui subscriber captures demo_authorize_url ──▶ AuthUiState refresh ── notify
page shows "Complete demo sign-in" (mock + SigningIn + url present)
click ──▶ auth_ui::complete_demo_sign_in(cx)
  strips prefix, builds gpui-starter://auth/callback?code=demo&state={state}
  ──▶ auth::handle_callback_link ──▶ controller.complete_callback
  (21) state-match, mint TokenSet, store.save, SignedIn{label}, emit
auth_ui observer ──▶ snapshot refresh (store.load: email, expires_at) ── notify
title bar / settings / diagnostics / status bar re-render from snapshots
```

With plan 22 registered, the email form's submit routes to
`sign_in_with_password` (no browser flow — `start_sign_in` would fail with
`NoBrowserFlow` on the email provider); with plan 23 registered, the
Google/GitHub rows dispatch to 23's per-provider controllers, and the real
browser flow replaces the demo step on the mock row; with the foundation
alone the urn:
URL is rejected by `desktop_actions::open_url`'s http/https guard
(`src/services/desktop_actions.rs:150-153`) and logged by 21's adapter — the
demo-complete control is the visible completion path.

Module tree:

```text
src/services/auth_ui.rs            read-side status service (snapshot global,
                                   controller observer, store read, demo
                                   completion, expiry formatting)
src/services/auth_ui.test.rs       sibling tests
src/features/pages/sign_in/
├── mod.rs                         SignInPage entity, Mode machine, actions,
│                                  module header with removal recipe
├── form.rs                        SignInForm (GpuiForm+Koruma), FieldKind
│                                  enum, validation-key mapping
├── providers.rs                   ProviderSpec registry + availability rule
├── mod.test.rs / form.test.rs / providers.test.rs
src/shell/account_menu.rs          AccountMenu entity + AccountSignIn/
                                   AccountSignOut actions
src/shell/account_menu.test.rs
src/features/pages/settings/
└── session_section.rs             render_session_section (replaces demo
                                   buttons)
```

## Interface Contract

Public surface this plan adds. Consumers today: `title_bar.rs`,
`settings/mod.rs`, `diagnostics/rows.rs`, `commands.rs` (all seam edits in
this plan). Plans 22/23 consume the registry rule and the snapshot when they
land. Code block is line-stable (INDEX §8 discipline).

```rust
// src/services/auth_ui.rs — read-side status mirror. Mutations never happen
// here; callers go through crate::services::auth::controller(cx).
pub struct AuthUiSnapshot {
    pub state: gpui_auth::AuthState,        // Clone + PartialEq (21 contract)
    pub provider_id: String,                // controller's provider, e.g. "mock"
    pub account_label: Option<String>,      // SignedIn { account_label }
    pub account_email: Option<String>,      // TokenSet.account.email, store read
    pub expires_at: Option<i64>,            // unix seconds, UTC; None = unknown/none
    pub store_backend: &'static str,        // STORE_BACKEND_KEYRING | _MEMORY
    pub demo_authorize_url: Option<String>, // mock flow only; NEVER logged/traced
}
impl gpui_kit::Global for AuthUiState {}   // global holds snapshot + subscriptions

pub const STORE_BACKEND_KEYRING: &str;     // "OS keyring (gpui-starter.auth)"
pub const STORE_BACKEND_MEMORY: &str;      // "in-memory (session only)"

/// Boot wiring. Must run after crate::services::auth::install (needs the
/// controller entity). Idempotent; safe when the controller is absent
/// (default snapshot, warn log).
pub fn initialize(cx: &mut gpui_kit::App);
pub fn snapshot(cx: &gpui_kit::App) -> AuthUiSnapshot;
/// RFC3339 (UTC) for an expiry, or None; callers localize the None case.
pub fn format_expiry(expires_at: Option<i64>) -> Option<String>;
/// Foundation demo: route the captured mock authorize URL back through
/// auth::handle_callback_link. Returns false when no mock flow is pending.
pub fn complete_demo_sign_in(cx: &mut gpui_kit::App) -> bool;

// src/features/pages/sign_in/providers.rs
pub(crate) enum ProviderKind { Email, Browser }
pub(crate) struct ProviderSpec {
    pub id: &'static str,        // "email" | "google" | "github"
    pub kind: ProviderKind,
    pub label_key: &'static str,
    pub icon: gpui_kit::component::IconName,
    pub capability: Option<&'static str>,   // "auth_email" | "auth_social" (21)
}
pub(crate) fn providers() -> &'static [ProviderSpec];   // email, google, github
/// Enabled rule: Email — capability enabled; Browser — capability supported
/// AND snapshot.provider_id == spec.id (single-provider controller, 21).

// src/features/pages/sign_in/mod.rs — page type is the only pub item.
pub struct SignInPage { /* private */ }
impl SignInPage {
    /// Constructed once by AppRoot.
    pub fn new(window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<Self>) -> Self;
}
impl gpui_kit::Render for SignInPage {}

// src/shell/account_menu.rs
pub struct AccountMenu { /* private */ }
impl AccountMenu {
    pub fn new(window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<Self>) -> Self;
}
impl gpui_kit::Render for AccountMenu {}
#[derive(gpui_kit::Action, Clone, PartialEq, Eq, serde::Deserialize)]
#[action(namespace = account, no_json)]
pub struct AccountSignIn;   // navigate to Page::SignIn
pub struct AccountSignOut;  // controller sign_out (same derives)

// src/features/pages/settings/session_section.rs
pub(super) fn render_session_section(
    cx: &mut gpui_kit::Context<crate::views::SettingsPage>,
) -> impl gpui_kit::IntoElement;
```

Binding notes: the snapshot is a plain clone-out mirror (no `pub` fields
crossing a crate seam — same crate, `session.rs` precedent of pub snapshot
fields, `src/services/session.rs:14-17`); `demo_authorize_url` carries the
pending `state` value, so it is memory-only and excluded from every tracing
call (the plan-wide no-tokens-in-logs rule).

## Module Layout

| File | Purpose |
| --- | --- |
| `src/services/auth_ui.rs` | `AuthUiSnapshot`/`AuthUiState` global, `initialize`, `snapshot`, `format_expiry`, `complete_demo_sign_in`, controller observer + `AuthEvent` subscriber, store read; module header with removal recipe |
| `src/services/auth_ui.test.rs` | Snapshot lifecycle tests over 21's mock controller (`install_with`) |
| `src/features/pages/sign_in/mod.rs` | `SignInPage` entity: mode machine, states, submit, demo control, signed-in card, actions + key bindings; removal-recipe header |
| `src/features/pages/sign_in/form.rs` | `SignInForm` (`GpuiForm` + `Koruma`), generated-component wiring, `SignInField` enum, error-key mapping |
| `src/features/pages/sign_in/providers.rs` | `ProviderSpec` registry + availability rule |
| `src/features/pages/sign_in/*.test.rs` | Sibling test modules (`#[cfg(test)] #[path]`) |
| `src/shell/account_menu.rs` | `AccountMenu` entity + `AccountSignIn`/`AccountSignOut` actions; removal-recipe header |
| `src/shell/account_menu.test.rs` | State-branch render + action tests |
| `src/features/pages/settings/session_section.rs` | Account section (state, provider, store, expiry, sign in/out) |

## Seam Touches

Every edit outside the new modules:

| # | File | Change | Why minimal |
| --- | --- | --- | --- |
| 1 | `src/services/mod.rs` | `pub mod auth_ui;` (alphabetical, after `auth` from 21) | one module line |
| 2 | `src/lib.rs:57-61` | add `auth_ui` to the `pub use services::{...}` list | same re-export style as every service |
| 3 | `src/app/init.rs` | `crate::services::auth_ui::initialize(cx);` directly after 21's `auth::install` line inside `runtime_services_init` (`init.rs:202-219`) | single boot hook; must follow the controller's existence |
| 4 | `src/features/pages/mod.rs` | `pub mod sign_in;` + `pub use sign_in::SignInPage;` | survey §1 row 2 |
| 5 | `src/shell/sidebar.rs` | `Page::SignIn` variant + `title()` "Sign In" + `title_key()` `nav_sign_in` + `host()` `sign-in` + `icon()` `IconName::User` + `all()` entry appended last | survey §1 rows 3-8; append-last keeps digit bindings (`state.rs:147-158`) |
| 6 | `src/shell/route.rs:10-21` | `Page::SignIn.host(),` in `VALID_HOSTS` | survey §1 row 9; host `sign-in` does not collide with 21's `auth` callback host (consumed earlier by `handle_callback_link`) |
| 7 | `src/shell/root/app_root/state.rs` | field `sign_in_page: Entity<SignInPage>` + construct in `AppRoot::new` + `unchecked_active_page_view` arm + import | survey §1 rows 10-12 |
| 8 | `src/shell/mod.rs` | `pub mod account_menu;` | one module line |
| 9 | `src/shell/title_bar.rs` | `AppTitleBar` gains `account: Entity<AccountMenu>`; construct in `new` (title_bar.rs:24-38); render as the last child of the right cluster (after the bell button, title_bar.rs:91-110) | one field, one construct, one child |
| 10 | `src/features/pages/settings/mod.rs` | `mod session_section;` + `.child(session_section::render_session_section(cx))` after the language row (mod.rs:127-163) + one `observe_global_in::<crate::services::auth_ui::AuthUiState>` subscription in `new` | section registration + re-render trigger |
| 11 | `src/features/pages/settings/dev_sections/runtime_sections.rs:144-176` | delete the three session demo buttons (`session-sign-in`, `session-sign-out`, `session-error-demo`) and the now-unused `use crate::session::{self, SessionState};` (line 7) | the placeholder this plan replaces; connectivity + secure-storage demos in the same section stay |
| 12 | `src/features/pages/diagnostics/mod.rs` | one `observe_global_in::<crate::services::auth_ui::AuthUiState>` subscription in `DiagnosticsPage::new` (pattern mod.rs:20-79) | rows re-render on auth change |
| 13 | `src/features/pages/diagnostics/rows.rs` | after the `diagnostics_session` row (line 161): four rows — `diagnostics_auth_state`, `diagnostics_auth_provider`, `diagnostics_auth_token_expiry`, `diagnostics_auth_store` — from `auth_ui::snapshot(cx)` | additive rows; existing row untouched |
| 14 | `src/services/commands.rs` | `CommandId::SignIn` + `CommandId::SignOut`; availability arms (SignIn always enabled; SignOut enabled iff `auth_ui::snapshot(cx).state` is `SignedIn`, reason `command_reason_signed_out`); registry entries via `command(..)` with `IconName::User` / `IconName::CircleUser`; `execute` arms (navigate / controller sign_out) | keyboard/palette path, survey §1 optional entries |
| 15 | `locales/en.yml` + `locales/zh-CN.yml` | add the i18n table's keys (both files); delete `settings_session_sign_in_demo`, `settings_session_sign_out`, `settings_session_error_demo` from both (dead after row 11) | lockstep test (`tests/qa_localization.rs:188-198`) |
| 16 | `tests/e2e_navigation.rs:14-33` | page count literal +1 (read the current literal first — sibling plans 05-19 may have landed), comment rewritten to match, `Page::SignIn` in `expected`; route case `("gpui-starter://sign-in", AppRoute::Page(Page::SignIn))` in `test_route_parsing` | INDEX §3 count rule |
| 17 (conditional) | `src/features/pages/sign_in/mod.rs`, `src/shell/title_bar.rs`, `src/features/pages/settings/session_section.rs` | `tutorial::target(id, …)` wraps — only when plan 15 has landed (`grep -q "pub mod tutorial" src/features/mod.rs`) | ids reserved below; defer the whole row if 15 is absent |

Deliberate non-touches: no `AppConfig` field (snapshot schema pinned,
`tests/snapshot_tests.rs:26,57`; `active_route` gains a variant value only),
no `src/services/session.rs` edit (its states stay; the replaced placeholder
is the settings UI), no `src/services/auth.rs` edit (21's adapter is consumed
as-is), no `AppEventKind` variant (navigation reuses `Navigate`), no new
capability entries (21's `auth_email`/`auth_social` are the gates), no
`[patch.crates-io]`/lockfile change, no status_bar edit (it already renders
`SessionSnapshot`, `src/shell/status_bar.rs:64-79`).

### Removal Recipe

`src/features/pages/sign_in/mod.rs` opens with (mirrors
`src/gallery/mod.rs:1-7`):

```rust
//! Removable sign-in page over the plan-21 auth controller. `SignInPage` is
//! the only public surface; the provider registry and form stay private.
//! Removal = delete this directory and revert the seam touches: the
//! Page::SignIn arms and all() entry in shell/sidebar.rs, the VALID_HOSTS
//! entry in shell/route.rs, the AppRoot wiring in shell/root/app_root/
//! state.rs, the mod/pub use lines in features/pages/mod.rs, the
//! auth_ui service (services/mod.rs, lib.rs, app/init.rs install line),
//! the account menu (shell/mod.rs, title_bar field/child), the settings
//! session section + the restored demo buttons, the diagnostics observer
//! and rows, the command arms, the locale keys, and the page count in
//! tests/e2e_navigation.rs.
```

Ordered removal steps:

1. Revert `src/features/pages/mod.rs` and `rm -r src/features/pages/sign_in/`.
2. Revert the five `sidebar.rs` arms + `all()` entry, the `route.rs` host,
   and the `state.rs` field/construct/view arm; `cargo check` proves the
   matches exhaustive again.
3. Remove the diagnostics observer (`diagnostics/mod.rs`) and the four rows
   (`rows.rs`).
4. Restore the session demo buttons in `runtime_sections.rs` (git revert of
   seam 11) and drop `session_section.rs` + its `mod` line, child call, and
   observer from `settings/mod.rs` — or drop the section entirely if the
   demo buttons are not wanted back.
5. Remove the account menu: `title_bar.rs` field/construct/child, `rm
   src/shell/account_menu.rs src/shell/account_menu.test.rs`, revert
   `shell/mod.rs`.
6. Remove `commands.rs` SignIn/SignOut arms (enum, availability, registry,
   execute) and the `command_reason_signed_out` key.
7. `rm src/services/auth_ui.rs src/services/auth_ui.test.rs`; revert
   `services/mod.rs`, the `lib.rs` re-export, and the `init.rs` line.
8. Remove every key from the i18n table (and `command_reason_signed_out`,
   `auth_no_expiry`, the `sign_in_form:` block, `nav_sign_in`) from both
   catalogs; optionally restore the three demo keys.
9. Decrement `tests/e2e_navigation.rs` (count -1, drop the expected entry and
   route case). Note: a user config persisted with `active_route: SignIn`
   fails deserialization after removal and falls into the config-store load
   error path (`last_load_error`, `src/state/config_store.rs`) — same
   behavior as removing any page variant; acceptable for a boilerplate.
10. Run the gates in Verification;
    `grep -rn "auth_ui\|SignInPage\|AccountMenu\|sign_in_form\|diagnostics_auth" src/ tests/ locales/`
    returns nothing.

## Implementation Phases

## Phase 1: Auth status service (`src/services/auth_ui.rs`)

### Step 1.1: Snapshot type + globals

- Files: `src/services/auth_ui.rs` (new, with the removal-recipe header),
  `src/services/mod.rs` (`pub mod auth_ui;`, alphabetical after 21's
  `pub mod auth;`).
- Build: `AuthUiSnapshot` exactly per the Interface Contract (all fields
  `pub`, `Clone + Debug + Default`; `Default` = `state:
  gpui_auth::AuthState::SignedOut`, empty strings, `None`s, backend chosen
  by `cfg!(target_family = "wasm")`). `AuthUiState { snapshot:
  AuthUiSnapshot, _subscriptions: Vec<Subscription> }` + `impl gpui_kit::Global
  for AuthUiState {}` (struct mirrors `src/services/session.rs:14-29`).
  `STORE_BACKEND_KEYRING = "OS keyring (gpui-starter.auth)"`,
  `STORE_BACKEND_MEMORY = "in-memory (session only)"`. Imports kit-facade
  style (`use gpui_kit::{App, ...}` matching `secure_storage.rs:1`).
- Tests: none.
- Done-check: `cargo check -p gpui-starter` passes (module declared and
  compiling; `lib.rs` re-export and the `init.rs` hook land in Step 1.4).

### Step 1.2: Observer, store read, refresh

- Files: `src/services/auth_ui.rs`.
- Build: `initialize(cx)` — build the default snapshot; `if let Some(entity)
  = crate::services::auth::controller(cx)` → `let observer =
  cx.observe(&entity, |_, cx| refresh(cx))` and `let subscriber =
  cx.subscribe(&entity, |_, _, event: &gpui_auth::AuthEvent, cx| { if let
  AuthorizationRequested { url } = event { remember url; refresh(cx) } })`
  (observe/subscribe signatures verified in 21's Research Digest:
  `gpui-pre-0.3.9/src/app.rs:1333-1343, 1422-1436`); else warn log
  (`target: "gpui_starter::auth_ui"`, controller absent). `refresh(cx:
  &mut App)` reads `entity.read(cx)` → `state().clone()`, `provider_id()`;
  when `SignedIn { account_label }` also reads tokens: `let store =
  crate::services::auth::KeyringStore; use gpui_auth::TokenStore as _;
  store.load(&provider_id, cx)` → `Ok(Some(tokens))` fills `account_email =
  tokens.account.as_ref().and_then(|a| a.email.clone())` and `expires_at =
  tokens.expires_at`; `Ok(None)`/`Err` → leave `None` (corrupt row reads as
  signed-in-unknown-expiry, mirroring 21's load behavior; error logged at
  debug without token text). `SigningIn` keeps the previous
  `demo_authorize_url` only when it starts with `urn:gpui-auth:mock:`,
  else clears it; every other state clears it. Then
  `cx.update_global::<AuthUiState, _>(|s, _| s.snapshot = next)` (the
  update notifies every observer — title bar, settings, diagnostics).
  Tracing logs state transitions only: `tracing::info!(target:
  "gpui_starter::auth_ui", state = ?next.state, provider_id, "auth snapshot
  refreshed")` — `demo_authorize_url` and any token-derived value never
  appear in log macros.
- Tests: none yet.
- Done-check: `cargo clippy -p gpui-starter --all-features -- -D warnings`
  clean for the file (after 1.4 wires the module).

### Step 1.3: Helpers — `format_expiry`, `complete_demo_sign_in`

- Files: `src/services/auth_ui.rs`.
- Build: `format_expiry(Option<i64>) -> Option<String>` =
  `expires_at.and_then(|secs| chrono::DateTime::from_timestamp(secs,
  0).map(|dt| dt.to_rfc3339())` (pure, no clock). `complete_demo_sign_in(cx)
  -> bool`: read snapshot; `demo_authorize_url` must be
  `Some(url.starts_with("urn:gpui-auth:mock:"))`; strip the prefix → state;
  build `format!("gpui-starter://auth/callback?code=demo&state={state}")`;
  `crate::services::auth::handle_callback_link(&link, cx)` return value
  passthrough. Timing note: the controller's exchange runs through its
  `AuthIo::drive` spawn (21 contract lines 415-427), and entity-notify
  observers are deferred effects flushed at the end of the outermost update
  (`finish_update` → `flush_effects`, `gpui-pre-0.3.9/src/app.rs:1325-1331,
  1997`) — so the snapshot refresh lands after this call returns,
  not during it. Every caller parks first: tests via
  `cx.run_until_parked()` (Steps 1.5, 4.3), the UI via the observer-driven
  re-render.
- Tests: none yet (1.5 covers).
- Done-check: `cargo check -p gpui-starter` clean.

### Step 1.4: Service seams

- Files: `src/lib.rs` (`auth_ui` in the services re-export list,
  `lib.rs:57-61`), `src/app/init.rs` (`crate::services::auth_ui::
  initialize(cx);` immediately after 21's `auth::install` line in
  `runtime_services_init`).
- Build: as stated (`services/mod.rs` line landed in Step 1.1).
- Tests: none.
- Done-check: `cargo test -p gpui-starter --lib` passes (boot compiles;
  `initialize` is not exercised without a window).

### Step 1.5: Service tests

- Files: `src/services/auth_ui.test.rs` wired `#[cfg(test)] #[path =
  "auth_ui.test.rs"] mod auth_ui_test;`.
- Build (`#[gpui_kit::test]`, `cx: &mut TestAppContext`, pattern
  `src/state/config_store.test.rs:280-281`): a `setup(cx)` helper —
  `crate::services::auth::install_with(std::sync::Arc::new(
  gpui_auth::InMemoryStore::new()), gpui_auth::AuthIo { http:
  reqwest::Client::new(), runtime: None }, cx)` then
  `crate::services::auth_ui::initialize(cx)`. If `install_with` is not
  `pub(crate)` (21 Open-Question dependency), widen it — one word, no logic
  change. Tests: (1) `snapshot` default SignedOut, backend matches
  `cfg!(target_family = "wasm")`; (2) `controller(cx).update(..start_sign_in)
  ` + `cx.run_until_parked()` → snapshot `SigningIn`,
  `demo_authorize_url` starts `urn:gpui-auth:mock:`; (3)
  `complete_demo_sign_in` → parked → snapshot `SignedIn` with
  `account_label == Some("Mock account")` (21 mock contract),
  `demo_authorize_url` cleared; (4) `complete_demo_sign_in` with no pending
  mock flow returns false; (5) sign-out via controller → snapshot
  `SignedOut`; (6) `format_expiry(Some(0))` contains "1970", `None` → None.
  Test-environment note: on SignedIn the refresh's token read goes through
  `KeyringStore::load` (the read path is constructed by this service, not
  the controller's `InMemoryStore`), so native test runs touch the real OS
  keyring row `gpui-starter.auth/mock`; errors map to `None` and no
  assertion reads `account_email`, so a present or absent row cannot fail
  these tests — only a stale real row could populate `account_email`, which
  nothing asserts.
- Done-check: `cargo test -p gpui-starter --lib services::auth_ui` passes.

## Phase 2: Sign-in page scaffold + page seams

### Step 2.1: Minimal page module

- Files: `src/features/pages/sign_in/mod.rs`, `src/features/pages/mod.rs`
  (`pub mod sign_in;` now; `pub use` lands with the type — add both here
  since the type exists in this step).
- Build: `SignInPage { mode: Mode, focus_handle: FocusHandle,
  _subscriptions: Vec<Subscription> }`, `enum Mode { Choose, Email }` (private).
  `new(window, cx)` — `focus_handle: cx.focus_handle().tab_stop(true)`
  (coding guides focus rule), one `observe_global_in`-style subscription:
  `cx.observe_global::<crate::services::auth_ui::AuthUiState>(|_, cx|
  cx.notify())`. Render skeleton: `v_flex().id("sign-in-page").
  track_focus(&self.focus_handle).key_context("SignIn").size_full().
  min_h_full().p_6().gap_4()` with the level-1 heading (title +
  `Role::Heading` + `aria_level(1)`, form_page.rs:323-330 pattern) and the
  muted subtitle (form_page.rs:331-338 pattern). Module header = removal
  recipe.
- Tests: `mod.test.rs` — page constructs and renders in a `TestAppContext`
  window without panicking (`cx.add_window`, e2e pattern).
- Done-check: `cargo test -p gpui-starter --lib sign_in` passes.

### Step 2.2: Sidebar, route, AppRoot, e2e count

- Files: `src/shell/sidebar.rs`, `src/shell/route.rs`,
  `src/shell/root/app_root/state.rs`, `tests/e2e_navigation.rs`.
- Build: exactly Seam Touches rows 5-7 + 16: `Page::SignIn` variant
  (append to the enum, `sidebar.rs:5-16`), `title() => "Sign In"`,
  `title_key() => "nav_sign_in"`, `host() => "sign-in"`, `icon() =>
  IconName::User` (verified default), `all()` append last
  (`sidebar.rs:92-105`); `VALID_HOSTS` += `Page::SignIn.host(),`; AppRoot
  field `sign_in_page` + `cx.new(|cx| SignInPage::new(window, cx))` +
  `unchecked_active_page_view` arm + import; e2e: read the count literal,
  +1, rewrite the "exactly ten pages" comment to the new number, add
  `Page::SignIn` to `expected`, add the route-parse case
  `("gpui-starter://sign-in", AppRoute::Page(Page::SignIn))`.
- Tests: existing e2e suite now covers the page.
- Done-check: `cargo test -p gpui-starter --test e2e_navigation` passes.

### Step 2.3: Scaffold keys

- Files: `locales/en.yml`, `locales/zh-CN.yml`.
- Build: `nav_sign_in` inside the nav block + `sign_in_title` +
  `sign_in_subtitle` (values from the i18n table) in BOTH files.
- Tests: `cargo test -p gpui-starter --test qa_localization`.
- Done-check: that command passes.

## Phase 3: Provider registry + chooser

### Step 3.1: Registry

- Files: `src/features/pages/sign_in/providers.rs`, `mod.rs` declares
  `pub(crate) mod providers;`.
- Build: `ProviderKind`, `ProviderSpec`, `providers()` returning
  `&[ProviderSpec { id: "email", kind: Email, label_key:
  "sign_in_provider_email", icon: IconName::User, capability: Some(
  "auth_email") }, ProviderSpec { id: "google", kind: Browser, label_key:
  "sign_in_provider_google", icon: IconName::Globe, capability: Some(
  "auth_social") }, ProviderSpec { id: "github", kind: Browser, label_key:
  "sign_in_provider_github", icon: IconName::Github, capability: Some(
  "auth_social") }]`. Availability fn `pub(crate) fn is_available(spec:
  &ProviderSpec, snapshot: &AuthUiSnapshot, cx: &App) -> bool`: Email →
  `crate::capabilities::snapshot(cx).get(spec.capability).is_some_and(|s|
  s.enabled)`; Browser → same capability check `s.supported && s.enabled`
  AND `snapshot.provider_id == spec.id`. Doc comment states the 23 contract:
  a browser button lights up when its provider is the controller's.
- Tests: `providers.test.rs` — registry is static (3 entries, stable order,
  email first); with a capability snapshot carrying enabled `auth_email` and
  degraded `auth_social` (21's wasm entries) + `provider_id "mock"` → email
  available, google/github not; with `provider_id "google"` + supported
  `auth_social` → google available, github not.
- Done-check: `cargo test -p gpui-starter --lib sign_in::providers` passes.

### Step 3.2: Chooser UI

- Files: `src/features/pages/sign_in/mod.rs`.
- Build: `render_chooser(&self, cx) -> impl IntoElement` shown when `mode ==
  Mode::Choose` and snapshot state is not `SignedIn`: a level-2 heading
  (`section_heading` pattern — inline: `div().id(..).a11y(Role::Heading,
  ..).aria_level(2)`) labeled `sign_in_provider_heading`, then a `v_flex` of
  provider rows wrapped as an a11y list: container `div().a11y(Role::List,
  localize("sign_in_provider_heading")).aria_size_of_set(providers().len())`;
  each row a `Role::ListItem` wrapper with `aria_position_in_set(ix)` +
  `aria_size_of_set(providers().len())` (0-based,
  `notifications.rs:84-124` pattern; size derives from the registry so
  plan-23 extensions cannot leave the a11y set size stale)
  containing one `Button::new(("auth-provider", ix)).outline().
  label(localize(spec.label_key)).icon(spec.icon).
  disabled(!available).loading(state == SigningIn && available)`.
  Element id is the enumerate index — `(&'static str, &str)` has no
  `Into<ElementId>` impl (gpui-pre-0.3.9/src/window.rs:7706-7739; integer
  seconds are the repo pattern, e.g. `("motion-spring-option", ix)` at
  `src/gallery/sections/motion/demos.rs:235`); the a11y list metadata
  (above) already carries the domain identity.
  Disabled browser buttons get a trailing muted reason line
  `sign_in_provider_unavailable` under the list (design guides: disabled
  states state why; a per-button tooltip needs the tooltip API — the shared
  line under the list covers all disabled entries without new API surface).
  Click behavior: Email → `self.mode = Mode::Email; cx.notify()` (the email
  form then calls 22's `sign_in_with_password` when 22 is registered —
  `start_sign_in` would fail with `NoBrowserFlow` on the email provider,
  22:367-372); Browser →
  `crate::services::auth::controller(cx).map(|c| c.update(cx, |ctl, cx| {
  ctl.start_sign_in(cx); }));` — the mock provider path; once 23 lands,
  Google/GitHub rows dispatch to 23's per-provider controllers (23:26,
  281-291), which this page reads through the same provider registry seam.
  When `snapshot.provider_id == "mock"`, a muted note `sign_in_demo_note`
  renders under the list (honest demo labeling).
- Tests: extend `mod.test.rs` — in a window: three buttons render; email
  click flips mode (assert via internal `mode` read after
  `cx.read`-driven click through the listener path; if the harness cannot
  click, drive via the action/listener equivalent — assert
  `is_available` gating instead and leave click to manual QA, per harness
  limits in INDEX §8).
- Done-check: `cargo test -p gpui-starter --lib sign_in` passes.

### Step 3.3: Chooser keys

- Files: `locales/en.yml`, `zh-CN.yml`.
- Build: `sign_in_provider_heading`, `sign_in_provider_email`,
  `sign_in_provider_google`, `sign_in_provider_github`,
  `sign_in_provider_unavailable`, `sign_in_demo_note` in both files.
- Done-check: `cargo test -p gpui-starter --test qa_localization` passes.

## Phase 4: Email/password form (gpui-form)

### Step 4.1: Form derive + field plumbing

- Files: `src/features/pages/sign_in/form.rs`, `mod.rs` declares
  `pub(crate) mod form;`.
- Build: mirror form_page.rs exactly, two fields:
  `#[derive(Clone, Debug, Default, GpuiForm, Koruma)] #[gpui_form(koruma(fluent))]
  pub(crate) struct SignInForm {
  #[gpui_form(component(input))] #[koruma(EmailValidation::<_>::builder())] pub email: String,
  #[gpui_form(component(input))] #[koruma(NonEmptyValidation::<_>::builder())] pub password: String }`.
  `enum SignInField { Email, Password }` with `ALL`, `set_on(&mut
  SignInFormFormValueHolder, Option<String>)`, `label_key()` →
  `SignInFormFormValueHolder::EMAIL_LABEL_KEY` / `PASSWORD_LABEL_KEY`
  (generated consts, Research Digest), `description_key()` →
  `sign_in_email_description` / `sign_in_password_description`.
  Error-key mapping fn per field mirroring form_page.rs:219-265:
  email → `RequiredValidation => "validation.required"`, `EmailValidation =>
  "validation.email"`; password → `RequiredValidation =>
  "validation.required"`, `NonEmptyValidation => "validation.non_empty"`
  (all four keys already exist, `locales/en.yml:46-52`). Validator enum
  names: `SignInFormFormValueHolderEmailKorumaValidator::*` /
  `...PasswordKorumaValidator::*` (codegen naming from form_page.rs:223-247).
- Tests: `form.test.rs` — holder validation: empty → email errors include
  required, password errors include required; `"not-an-email"` → email
  error key `validation.email`, password ok; label keys are exactly
  `"sign_in_form.email_label"` / `"sign_in_form.password_label"` (pins the
  codegen prefix).
- Done-check: `cargo test -p gpui-starter --lib sign_in::form` passes.

### Step 4.2: Page form state + render

- Files: `src/features/pages/sign_in/mod.rs`.
- Build: extend `SignInPage` — `current_data:
  SignInFormFormValueHolder`, `fields: SignInFormFormFields`, `touched:
  bool`, `dirty: bool`, `cached_errors: [Option<Vec<&'static str>>; 2]`.
  `new` builds the two input entities
  (`SignInFormFormComponents::email_input(window, cx)` /
  `password_input(window, cx)`) and, for the password input only,
  `input.update(cx, |s, cx| s.set_masked(true, window, cx))` after
  construction (state.rs:10952; masked-by-default). Subscriptions:
  `subscribe_field` per field (form_page.rs:156-176 shape) +
  password-only `cx.subscribe(.., |this, _, event, cx| if let
  InputEvent::PressEnter { .. } = event { this.try_submit(window, cx) })`
  (keyboard submit). `render_form(&self, cx)`: `v_form().label_width(px(160.))`
  with two `field()` rows via a `render_field(&self, kind, danger)` helper
  copying form_page.rs:274-305 (label, required, description_fn with error
  block `Role::Alert` + `a11y_live(accesskit::Live::Polite)` + danger
  color), the password row's input rendered as
  `Input::new(input).mask_toggle().content_type(
  gpui_kit::component::input::InputContentType::Password)` (verified
  builders, Research Digest). Back button: ghost `Button::new(
  "auth-use-another").label(localize("sign_in_back"))` → `mode = Choose`.
  Validation recompute: same `touched && dirty` gate as form_page.rs:310-312,
  recompute in render.
- Tests: extend `mod.test.rs` — mode Email renders two fields; seeding
  `current_data.email = Some("a@b.c".into())`, `password = Some("x".into())`
  then `recompute_validation` leaves `cached_errors` all-`None`; empty
  seeds produce error keys per field.
- Done-check: `cargo test -p gpui-starter --lib sign_in` passes.

### Step 4.3: Submit flow + states

- Files: `src/features/pages/sign_in/mod.rs`.
- Build: `fn try_submit(&mut self, window, cx)` — `touched = true`;
  recompute; valid = cached_errors all-`None`; if !valid → `cx.notify()`
  and return (errors already visible per field); if valid →
  `crate::services::auth::controller(cx)` present → dispatch by provider:
  email provider registered (22) → `ctl.sign_in_with_password(email,
  password, cx)` (`start_sign_in` on the EmailPasswordProvider fails with
  `NoBrowserFlow`, 22:367-372); mock provider → `ctl.start_sign_in(cx)`;
  absent controller → log warn + notify. Submit button:
  `Button::new("auth-submit").primary().label(localize("sign_in_submit")).
  loading(state == SigningIn).disabled(state == SigningIn)` (primary = the
  form's Enter/default commit, design guides). Error banner: when snapshot
  state is `Error(_)`, render the form_page success-banner shape inverted
  (danger token): `div().id("auth-error").a11y(Role::Alert,
  title).a11y_live(..).text_color(cx.theme().danger).border_1().
  border_color(danger).rounded(cx.theme().radius).p_3()` with
  `sign_in_error_heading` as the localized title and the detail line
  localized by variant: map the known `AuthError` kinds to
  `sign_in_err_invalid_credentials` / `sign_in_err_network` /
  `sign_in_err_challenge` / `sign_in_err_state_mismatch` /
  `sign_in_err_unknown` keys; only unmapped/foreign variants fall back to
  the raw Display string as the secondary line (21's Design Compliance note
  requires the wrap — developer text must not be the primary copy a zh-CN
  user reads), and the detail stays out of logs. Demo control: when `state == SigningIn &&
  provider_id == "mock" && demo_authorize_url.is_some()` render ghost
  `Button::new("auth-demo-complete").label(localize(
  "sign_in_demo_complete")).on_click(|_, _, cx| {
  crate::services::auth_ui::complete_demo_sign_in(cx); })` directly under
  the submit button.
- Tests: extend `mod.test.rs` with the 21-mock `setup` from Step 1.5:
  valid seeds + `try_submit` → `cx.run_until_parked()` →
  `auth_ui::snapshot(cx).state == SigningIn`; then
  `complete_demo_sign_in` → parked → `SignedIn`.
- Done-check: `cargo test -p gpui-starter --lib sign_in` passes.

### Step 4.4: Escape binding + form keys

- Files: `src/features/pages/sign_in/mod.rs`, `locales/en.yml`,
  `zh-CN.yml`.
- Build: action `#[derive(Action, Clone, PartialEq, Eq,
  serde::Deserialize)] #[action(namespace = sign_in, no_json)] pub(crate)
  struct BackToProviders;` (derive style `src/shell/root/actions.rs:3-15`);
  `new` binds `cx.bind_keys([KeyBinding::new("escape", BackToProviders,
  Some("SignIn"))])` (binding pattern `state.rs:145-166`); root carries
  `.key_context("SignIn")` (Step 2.1) + `.on_action(cx.listener(|this, _,
  _, cx| { this.mode = Mode::Choose; cx.notify(); }))`. Keys:
  `sign_in_submit`, `sign_in_back`, `sign_in_error_heading`,
  `sign_in_demo_complete`, `sign_in_email_description`,
  `sign_in_password_description`, the five error-detail keys
  `sign_in_err_invalid_credentials` ("Email or password is incorrect" /
  "邮箱或密码不正确"), `sign_in_err_network` ("Network error during
  sign-in" / "登录时网络错误"), `sign_in_err_challenge` ("Verification
  failed; check the code and retry" / "验证失败；请检查验证码后重试"),
  `sign_in_err_state_mismatch` ("Sign-in session expired; try again" /
  "登录会话已过期；请重试"), `sign_in_err_unknown` ("Sign-in failed" /
  "登录失败"), and the `sign_in_form:` nested block
  (`email_label`, `password_label`) in both catalogs.
- Tests: existing suites + qa_localization.
- Done-check: `cargo test -p gpui-starter --lib sign_in` and
  `--test qa_localization` pass.

## Phase 5: Signed-in account view on the page

### Step 5.1: Account card

- Files: `src/features/pages/sign_in/mod.rs`, `locales/en.yml`,
  `zh-CN.yml`.
- Build: when `snapshot.state` is `SignedIn { .. }` the page body (instead
  of chooser/form) renders an account card on `settings_card_base`-shaped
  chrome (inline: `v_flex().gap_3().p_4().rounded(cx.theme().radius).
  border_1().border_color(cx.theme().border)`): `Avatar::new().
  name(label.clone()).with_size(Small)` (initials derived automatically,
  Research Digest), `account_label` (+ `account_email` as secondary line
  when present), a status row pair for provider and expiry
  (`format_expiry` → RFC3339, else `localize("auth_no_expiry")`), muted
  hint `sign_in_signed_in_hint`, and `Button::new("auth-page-sign-out").
  outline().label(localize("settings_auth_sign_out")).on_click(..controller
  sign_out..)`. Add keys `sign_in_signed_in_hint` + `auth_no_expiry` to
  both catalogs.
- Tests: extend `mod.test.rs` — after the Step 4.3 sign-in, the page (read
  through entity state / re-render) is in the signed-in branch; driving
  sign-out via the controller flips `auth_ui::snapshot` back to SignedOut.
- Done-check: `cargo test -p gpui-starter --lib sign_in` passes.

## Phase 6: Title-bar account menu

### Step 6.1: `AccountMenu` entity

- Files: `src/shell/account_menu.rs` (new, removal-recipe header),
  `src/shell/account_menu.test.rs`, `src/shell/mod.rs` (`pub mod
  account_menu;`).
- Build: copy the `SettingsDropdown` shape (`title_bar.rs:116-205`):
  `AccountMenu { focus_handle: FocusHandle, _subscriptions:
  Vec<Subscription> }`; `new` — `focus_handle: cx.focus_handle()`, one
  `cx.observe_global::<crate::services::auth_ui::AuthUiState>(|_, cx|
  cx.notify())`. Actions `AccountSignIn` / `AccountSignOut` per the
  Interface Contract (derive style above). Render root:
  `div().id("account-menu").a11y(Role::Group,
  localize("titlebar_account")).track_focus(&focus_handle).
  on_action(cx.listener(|_, _: &AccountSignIn, _, cx| { navigate to
  Page::SignIn via events::emit(Navigate) })).on_action(cx.listener(..
  AccountSignOut → controller sign_out))` wrapping one Button whose content
  switches on `auth_ui::snapshot(cx).state`. Private helper `fn initial_of(
  label: &str) -> SharedString`: the first alphanumeric char of the label,
  uppercased (`label.chars().find(char::is_alphanumeric).map(|c|
  c.to_uppercase().next().unwrap_or(c)) …`, `SharedString::from` fallback
  `"?"`) — local derivation because Avatar's `extract_text_initials` is
  private to its module (avatar.rs); the Avatar element itself renders the
  same account on the page card (Step 5.1) and settings (Step 7.1).
  - `SignedOut`: `Button::new("account-btn").small().ghost().compact().
    icon(IconName::CircleUser).accessibility_label(localize(
    "titlebar_account")).dropdown_menu(move |menu, _, _| menu.
    menu(localize("titlebar_account_sign_in"), Box::new(AccountSignIn)))
    .anchor(Anchor::TopRight)` — menu keyboard nav, Enter/Escape are the
    menu system's own.
  - `SigningIn`: no dropdown; `Button::new("account-btn").small().ghost().
    compact().icon(IconName::LoaderCircle).disabled(true).
    accessibility_label(localize("titlebar_account_signing_in"))`.
  - `SignedIn { account_label }`: `Button::new("account-btn").small().
    ghost().compact().label(initial_of(&account_label)).
    accessibility_label(account_label.clone()).dropdown_menu(move |menu,
    _, _| menu.label(account_email_or_label).separator().menu(localize(
    "titlebar_account_sign_out"), Box::new(AccountSignOut))).
    anchor(Anchor::TopRight)`. The visible trigger is the initial (the
    avatar-or-initial requirement; `DropdownMenu` is Button-only and
    Button's content API is label/icon, so no Avatar inside the trigger —
    Research Digest), and `accessibility_label` replaces the announced name
    so screen readers hear the full account label, not the glyph (doc at
    button.rs:381-393).
  - `Error(_)`: `SignedOut` button + icon `IconName::TriangleAlert` +
    accessibility label `titlebar_account_error`; menu label carries the
    error title and the sign-in item.
  Menu closures capture clones of the strings read before the closure
  (menu builders run later; no `cx` borrow escapes — same as
  title_bar.rs:173-201 capturing `font_size`).
- Tests: `account_menu.test.rs` (`#[gpui_kit::test]`) — entity constructs
  in a window and renders without panic under a default snapshot; with the
  1.5 mock setup after sign-in, rendering reads the SignedIn branch
  (assert via a re-render + no panic; branch selection itself is the
  snapshot match already tested in 1.5); pure unit tests for `initial_of`:
  `"Mock account"` → `"M"`, `"zed dev"` → `"Z"`, `""` and `" — "` → `"?"`.
- Done-check: `cargo test -p gpui-starter --lib account_menu` passes.

### Step 6.2: Title-bar seam

- Files: `src/shell/title_bar.rs`.
- Build: `AppTitleBar` gains `account: Entity<AccountMenu>` (struct
  title_bar.rs:17-21); construct `let account = cx.new(|cx|
  AccountMenu::new(window, cx));` in `new` (after `settings`, title_bar.rs:31);
  render `.child(self.account.clone())` as the LAST child of the right
  cluster (after the bell button, title_bar.rs:91-110). No other edit.
- Tests: none new (render covered by 6.1 + manual QA).
- Done-check: `cargo test -p gpui-starter --lib` passes; keys
  `titlebar_account`, `titlebar_account_sign_in`,
  `titlebar_account_sign_out`, `titlebar_account_signing_in`,
  `titlebar_account_error` added to both catalogs and qa_localization
  passes.

## Phase 7: Settings session section

### Step 7.1: Section module

- Files: `src/features/pages/settings/session_section.rs` (new),
  `src/features/pages/settings/mod.rs`.
- Build: `pub(super) fn render_session_section(cx: &mut
  Context<SettingsPage>) -> impl IntoElement` —
  `settings_card_base(cx).child(section_heading("settings-account-title",
  localize("settings_auth_title")))` (both helpers are already `pub(crate)`
  at `dev_sections/mod.rs:24, 35-50`; import via `super::dev_sections::`
  from the sibling `session_section` module). Body from
  `auth_ui::snapshot(cx)`: state row (signed out →
  `settings_auth_signed_out` + outline `Button "settings-auth-sign-in"` →
  `events::emit(Navigate(Page::SignIn))`; signing in →
  `settings_auth_signing_in` muted; signed in → `Avatar` + label + email);
  provider row (`settings_auth_provider` + `provider_id`); store row
  (`settings_auth_store` + `store_backend`); expiry row
  (`settings_auth_expires` + `format_expiry(..)` or `auth_no_expiry`);
  signed-in → sign-out button (`Button::new("settings-auth-sign-out").
  outline()` → controller `sign_out`). Status rows use the
  `status_row(id, label, value)` pattern from
  `notifications_section.rs:22-38` (id-keyed, combined-text a11y label).
- Tests: none new (state branches exercised through 1.5 + manual QA).
- Done-check: `cargo check -p gpui-starter` passes.

### Step 7.2: Wire + replace placeholders

- Files: `src/features/pages/settings/mod.rs`,
  `src/features/pages/settings/dev_sections/runtime_sections.rs`,
  `locales/en.yml`, `locales/zh-CN.yml`.
- Build: `mod session_section;` + `.child(session_section::
  render_session_section(cx))` right after the language row
  (settings/mod.rs:127-163) + `cx.observe_global_in::<crate::services::
  auth_ui::AuthUiState>(window, |_, _, cx| cx.notify())` in `new`'s
  subscription vec (pattern mod.rs:28-72). Delete the three session demo
  buttons (runtime_sections.rs:144-176) and the `use crate::session::{self,
  SessionState};` import (line 7; verify `session` is not referenced
  elsewhere in the file first — grep shows it is not,
  runtime_sections.rs:7, 149-172 only). Locales: add
  `settings_auth_title`, `settings_auth_signed_out`,
  `settings_auth_signing_in`, `settings_auth_sign_in`,
  `settings_auth_sign_out`, `settings_auth_provider`,
  `settings_auth_store`, `settings_auth_expires`; DELETE
  `settings_session_sign_in_demo`, `settings_session_sign_out`,
  `settings_session_error_demo` from BOTH files.
- Tests: `cargo test -p gpui-starter --lib` + `--test qa_localization`
  (lockstep also proves the three deletions are symmetric).
- Done-check: both pass; `grep -rn "settings_session_" src/ locales/`
  returns nothing.

## Phase 8: Diagnostics auth rows

### Step 8.1: Observer + rows

- Files: `src/features/pages/diagnostics/mod.rs`,
  `src/features/pages/diagnostics/rows.rs`.
- Build: one subscription in `DiagnosticsPage::new`:
  `cx.observe_global_in::<crate::services::auth_ui::AuthUiState>(window,
  |_, _, cx| cx.notify())` (pattern mod.rs:20-79). In
  `build_diagnostic_rows`, after the `diagnostics_session` row (rows.rs:161):
  `let auth = crate::services::auth_ui::snapshot(cx);` then `row(
  "diagnostics_auth_state", &format!("{:?}", auth.state))`, `row(
  "diagnostics_auth_provider", &auth.provider_id)`, `row(
  "diagnostics_auth_token_expiry", &crate::services::auth_ui::format_expiry(
  auth.expires_at).unwrap_or_else(|| crate::i18n::localize("auth_no_expiry")))
  `, `row("diagnostics_auth_store", auth.store_backend)`. Values are
  developer-facing diagnostics text (the page's existing convention prints
  `Debug`, rows.rs:161), so `AuthState`'s English Debug shape is correct
  here and nowhere else.
- Tests: none new (page renders rows; state proven in 1.5).
- Done-check: `cargo test -p gpui-starter --lib diagnostics` passes; keys
  `diagnostics_auth_state`, `diagnostics_auth_provider`,
  `diagnostics_auth_token_expiry`, `diagnostics_auth_store` in both
  catalogs, qa_localization green.

## Phase 9: Launcher commands

### Step 9.1: `CommandId::SignIn` / `CommandId::SignOut`

- Files: `src/services/commands.rs`, `locales/en.yml`, `zh-CN.yml`.
- Build: enum variants (commands.rs:11-29); availability arms — SignIn:
  always enabled; SignOut: `let signed_in =
  matches!(crate::services::auth_ui::snapshot(cx).state,
  gpui_auth::AuthState::SignedIn { .. })` → `enabled: signed_in,
  disabled_reason: disabled_reason("command_reason_signed_out", !
  signed_in)`; registry entries
  `command(CommandId::SignIn, "command_sign_in",
  "command_sign_in_subtitle", IconName::User)` and
  `command(CommandId::SignOut, "command_sign_out",
  "command_sign_out_subtitle", IconName::CircleUser)`; execute arms —
  `SignIn => navigate(Page::SignIn, cx)`, `SignOut =>` controller
  `sign_out` (absent → warn log). Keys in both catalogs:
  `command_sign_in`, `command_sign_in_subtitle`, `command_sign_out`,
  `command_sign_out_subtitle`, `command_reason_signed_out`.
- Tests: none new (palette pickup is automatic, survey §1).
- Done-check: `cargo test -p gpui-starter --lib` passes; palette lists both
  commands in manual QA.

## Phase 10: Tutorial-target ids (conditional on plan 15)

### Step 10.1: Wraps

- Files: `src/features/pages/sign_in/mod.rs`, `src/shell/title_bar.rs`,
  `src/features/pages/settings/session_section.rs`.
- Build: ONLY if `grep -q "pub mod tutorial" src/features/mod.rs` (plan 15
  landed). Wrap: page heading → `tutorial::target("signin-title", ..)`,
  chooser list container → `target("signin-providers", ..)`, the submit
  button's field row → `target("signin-submit", ..)`, the account-menu
  group div → `target("titlebar-account", ..)`, the settings card →
  `target("settings-account", ..)` (`target(id, child)` signature, plan 15
  contract lines 119-127; ids are unique among siblings, layout- and
  a11y-transparent per 15's contract). If 15 is absent, skip this step
  entirely — the ids above are the reserved contract; plan 15 or a follow-up
  applies the wraps.
- Tests: none (15's registry tests own capture behavior).
- Done-check: `cargo test -p gpui-starter --lib` passes with the wraps in.

## Phase 11: Gates + manual QA

### Step 11.1: Full gates

- Files: none (verification only).
- Build: run everything in Verification.
- Done-check: all gates green; manual checklist executed or explicitly
  waived with notes.

## Platform & Wasm Gating

- No new dependencies, no cfg-gated dep tables, no new Cargo.toml or
  Cargo.lock changes at all (everything consumed is already unconditional
  or 21's).
- cfg in this plan's code: exactly one runtime branch —
  `AuthUiSnapshot::default()`/refresh picks `STORE_BACKEND_MEMORY` vs
  `STORE_BACKEND_KEYRING` via `cfg!(target_family = "wasm")` (mirrors what
  21's `install` selected). `KeyringStore::load` compiles on wasm and fails
  closed through `secure_storage`'s stubs (`secure_storage.rs:212-224`), so
  the read path needs no cfg pair; errors map to `None`.
- Wasm posture: the page renders and is fully routable (`sign-in` host
  routes through the hash router, `route.rs:110-118`); provider gating turns
  off Google/GitHub because 21 registers `auth_social` degraded-unsupported
  on wasm; email submit starts the controller flow (fetch-based REST is the
  22 path; with the mock it completes in-memory); the demo-complete button
  works because `complete_demo_sign_in` → `handle_callback_link` →
  `controller.complete_callback` is all in-process — no OS deep link needed
  on wasm; tokens live only in `InMemoryStore` and vanish on reload
  (21's posture, unchanged). Status bar, title bar, settings, diagnostics
  all render from snapshots with the memory backend named.
- Clocks: `chrono::DateTime::from_timestamp` is pure conversion; nothing in
  this plan reads a clock, so the seams §6 rule is respected without
  `platform::clock`.
- Capability registry: no new entries. Reads `auth_email`/`auth_social`
  (set by 21's `install`) for provider gating and the SignOut command
  availability is snapshot-driven instead.
- `just wasm-check` covers the new modules transitively (app lib build).

## i18n

Facade `crate::i18n::localize` (survey §5). Both catalogs in lockstep; the
`sign_in_form:` block uses the one nesting level the parser allows
(`locales/en.yml:35-44` precedent). Scanner blind spot: `title_key()` is an
indirect call — `nav_sign_in` must land in both files by hand.

| key | en | zh-CN |
| --- | --- | --- |
| `nav_sign_in` | "Sign In" | "登录" |
| `sign_in_title` | "Sign In" | "登录" |
| `sign_in_subtitle` | "Use email or a provider to continue." | "使用邮箱或第三方服务继续。" |
| `sign_in_provider_heading` | "Continue with a provider" | "选择登录方式" |
| `sign_in_provider_email` | "Continue with email" | "通过邮箱继续" |
| `sign_in_provider_google` | "Continue with Google" | "通过 Google 继续" |
| `sign_in_provider_github` | "Continue with GitHub" | "通过 GitHub 继续" |
| `sign_in_provider_unavailable` | "Not configured in this build." | "当前构建未配置。" |
| `sign_in_demo_note` | "Demo build: the active provider is the scripted mock." | "演示构建：当前提供方为脚本化的模拟服务。" |
| `sign_in_demo_complete` | "Complete demo sign-in" | "完成演示登录" |
| `sign_in_back` | "Use another method" | "使用其他方式" |
| `sign_in_submit` | "Sign in" | "登录" |
| `sign_in_error_heading` | "Sign-in failed" | "登录失败" |
| `sign_in_err_invalid_credentials` | "Email or password is incorrect" | "邮箱或密码不正确" |
| `sign_in_err_network` | "Network error during sign-in" | "登录时网络错误" |
| `sign_in_err_challenge` | "Verification failed; check the code and retry" | "验证失败；请检查验证码后重试" |
| `sign_in_err_state_mismatch` | "Sign-in session expired; try again" | "登录会话已过期；请重试" |
| `sign_in_err_unknown` | "Sign-in failed" | "登录失败" |
| `sign_in_email_description` | "The address associated with your account." | "账号绑定的邮箱地址。" |
| `sign_in_password_description` | "Enter your account password." | "输入账号密码。" |
| `sign_in_signed_in_hint` | "You are signed in. Sign out to switch accounts." | "已登录。退出后可切换账号。" |
| `sign_in_form.email_label` | "Email" | "邮箱" |
| `sign_in_form.password_label` | "Password" | "密码" |
| `auth_no_expiry` | "No expiry advertised" | "无过期时间" |
| `settings_auth_title` | "Account" | "账号" |
| `settings_auth_signed_out` | "Not signed in" | "未登录" |
| `settings_auth_signing_in` | "Signing in…" | "登录中…" |
| `settings_auth_sign_in` | "Sign In…" | "登录…" |
| `settings_auth_sign_out` | "Sign out" | "退出登录" |
| `settings_auth_provider` | "Provider" | "登录提供方" |
| `settings_auth_store` | "Token store" | "令牌存储" |
| `settings_auth_expires` | "Access token expires" | "访问令牌过期时间" |
| `titlebar_account` | "Account" | "账号" |
| `titlebar_account_sign_in` | "Sign in" | "登录" |
| `titlebar_account_sign_out` | "Sign out" | "退出登录" |
| `titlebar_account_signing_in` | "Signing in…" | "登录中…" |
| `titlebar_account_error` | "Sign-in error" | "登录出错" |
| `diagnostics_auth_state` | "Auth state" | "认证状态" |
| `diagnostics_auth_provider` | "Auth provider" | "认证提供方" |
| `diagnostics_auth_token_expiry` | "Token expiry" | "令牌过期" |
| `diagnostics_auth_store` | "Token store" | "令牌存储" |
| `command_sign_in` | "Sign In" | "登录" |
| `command_sign_in_subtitle` | "Open the sign-in page" | "打开登录页面" |
| `command_sign_out` | "Sign Out" | "退出登录" |
| `command_sign_out_subtitle` | "End the current session" | "结束当前会话" |
| `command_reason_signed_out` | "You are not signed in" | "当前未登录" |

Reused existing keys (no re-adding): `validation.required`,
`validation.non_empty`, `validation.email` (`locales/en.yml:46-52`),
`status_session_*` (status bar, untouched). Deleted: 
`settings_session_sign_in_demo`, `settings_session_sign_out`,
`settings_session_error_demo` (both files, Phase 7). Copy rules applied:
sentence-case buttons, destination noun "Sign In" for nav/title matching the
repo's Title Case nav labels, single `…` character on input-requiring
commands and the indeterminate state, no terminal periods on short labels,
full-width punctuation in zh-CN sentences.

## Design Compliance

Full weight for this UI plan; mapped to the design-guides sections:

- **Start from the task / primary action.** The page's one job is "get
  signed in": chooser first (three provider buttons, email first), email
  path reveals the form, submit is the single primary Button on the page —
  the only primary in the whole feature (title bar, settings, menu items
  use ghost/outline/menu items). Signed-in users get the account card, not
  a disabled form.
- **Interaction states** (guides table): submit carries rest → hover →
  pressed via Button, `loading(true)` during `SigningIn` (verified
  `Button::loading`, button.rs:445) which also prevents duplicate dispatch
  via `disabled`; provider buttons disabled get the shared reason line
  (disabled must not be the only cue — text carries why); error state is
  the `Role::Alert` live banner naming what happened (localized heading +
  raw detail as muted secondary text); the account trigger stays visibly
  `selected`/open while its dropdown is open (DropdownMenu owns that,
  component_traits.rs:12-37).
- **Keyboard paths** (a11y checklist): page root `track_focus` + tab stops;
  Tab order = chooser buttons → (email mode) email input → password input →
  submit → back; Enter submits from the password field (`PressEnter`
  subscription) and activates any focused Button; Escape returns to the
  chooser (context-scoped binding); account menu: Tab to trigger, Enter to
  open, arrows/Enter/Escape inside the menu are the menu system's contract
  (design guides "Prefer desktop command surfaces"); palette commands
  (`SignIn`/`SignOut`) give global keyboard reach; icon-only buttons
  (signed-out account trigger, signing-in indicator) carry
  `accessibility_label`s, and the signed-in trigger sets
  `accessibility_label(account_label)` so the initial glyph is not the
  announced name (Button doc, button.rs:381-393).
- **Components and composition.** No custom popup: the account menu is a
  Button + `dropdown_menu` (no Avatar-capable trigger implementor exists —
  see Research Digest) with a `.label()` header + separator + one or two
  menu items — the system menu's geometry, focus transfer, and dismissal
  are reused, not imitated. Avatar-or-initial split, forced by the
  Button-only trigger (Button content is `label`/`icon`, no Avatar
  conversion — Research Digest): the signed-in title-bar trigger shows the
  derived initial as its Button `label`; the `Avatar` element (initials
  from `name()`, avatar.rs:48-52) renders the same account on the sign-in
  page card and the Settings section. `Input::mask_toggle` +
  `content_type(Password)` gives the eye toggle and `Role::PasswordInput`
  without hand-rolled masking. Provider buttons keep icon+label pairs
  (icons supplement, never replace: all three carry text).
- **Color/tokens/spacing.** All colors from `cx.theme()` (`danger`,
  `muted_foreground`, `border`, `success` untouched); zero raw colors, zero
  `px()` outside `v_form().label_width(px(160.))` which is the established
  form_page value (form_page.rs:357) — kept identical so the two forms
  share one visual grid. Spacing `p_6`/`gap_4` page chrome matches
  form_page and settings pages. Density: `.small().ghost().compact()`
  trigger matches the neighboring search/bell buttons in the title bar's
  one band (alignment-spine rule).
- **Interface language.** Destination noun "Sign In" (nav + H1); commands
  are verbs ("Sign out"); states are short phrases ("Signing in…", "Not
  signed in"); zh-CN composed natively, not translated word-by-word;
  ellipsis rules per the Capitalization/punctuation section; "Account"
  inside Settings is not "Account Settings" (context carries context).
- **Icons.** Only default-set variants (`github`, `user`, `circle-user`,
  `globe`, `loader-circle`, `triangle-alert` all verified in
  default-icons.txt); Google uses the `Globe` stand-in — documented here,
  same convention as INDEX §4's stand-in table; no icon where the default
  set has no honest glyph (sign-out items).
- **A11y beyond keyboard.** Provider chooser is a `Role::List` with
  `aria_size_of_set`/`aria_position_in_set` (seams §9 list rule); error and
  state changes announce through `a11y_live(Polite)` on `Role::Alert` /
  status nodes; status not communicated by color alone (loading is an icon
  + label, errors are text); the account group exposes `Role::Group` with
  a label.

## Verification

Repo gates, in order (AGENTS.md):

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-features
cargo test --workspace --all-features
just wasm-check
```

`--workspace` includes `crates/gpui-auth` (21); this plan adds no manifest
changes, so no lockfile staging is needed. CI additionally runs
`cargo test -p gpui-starter --lib`, which executes every in-lib test this
plan adds (Phase 1-9 test files).

Feature-specific automated coverage (all runnable via
`cargo test -p gpui-starter --lib`):

- `services::auth_ui`: snapshot lifecycle over the mock controller
  (default → SigningIn with captured demo URL → complete → SignedIn with
  label → sign-out), `complete_demo_sign_in` false path, expiry
  formatting.
- `sign_in::form`: koruma validation key mapping; generated label keys pin
  `sign_in_form.*`.
- `sign_in::providers`: registry shape + availability truth table under
  capability snapshots.
- `sign_in`: page construct/render, mode flip, validation recompute,
  try_submit → SigningIn → complete → SignedIn over the mock.
- `account_menu`: entity render under default snapshot.
- `qa_localization` (cross-crate): lockstep + grammar for ~40 added keys
  and the 3 deletions.

Manual QA (native, Linux dev box — the only host this planning session can
assume; macOS/Windows visual passes are on the implementer):

1. `cargo run`: sidebar shows "Sign In" (nav + zh-CN under locale switch);
   page opens; chooser shows email + Google + GitHub, with Google/GitHub
   disabled and the reason line + demo note visible (foundation build).
2. Email → form appears; submit empty → per-field errors; valid input →
   submit shows loading; "Complete demo sign-in" appears; click → page
   flips to the account card ("Mock account"), the title-bar trigger now
   shows the "M" initial and opens the account menu with the label header
   and Sign out, settings Account section shows provider/store/expiry
   rows, diagnostics shows the four auth rows, status bar shows
   SignedIn(Mock account).
3. Sign out from the title-bar menu → all surfaces return to signed-out;
   `secret-tool search service gpui-starter.auth` (or OS equivalent) shows
   the `mock` row gone.
4. Keyboard-only pass: Tab through the page, Enter from the password field
   submits, Escape returns to the chooser, palette → "Sign In"/"Sign Out"
   both work; "Sign Out" is disabled in the palette while signed out.
5. `xdg-open 'gpui-starter://sign-in'` routes to the page (and a second
   instance forwards it over single-instance IPC).
6. `just wasm-check` green; if a web build is available: page routable via
   `#/sign-in`, social buttons disabled with 21's reason, email demo flow
   completes in-memory, reload returns to signed-out.

Acceptance criteria: all four repo gates pass; the automated suites above
pass; manual items 1-5 executed (6 executed or waived with a note);
`grep -rn "mock-access-\|access_token\|refresh_token" src/features/pages/sign_in/
src/shell/account_menu.rs src/services/auth_ui.rs src/features/pages/settings/
session_section.rs` returns no hits (no token text anywhere in the UI layer);
`grep -rn "demo_authorize_url" src/services/auth_ui.rs` shows the value only
in struct fields and comparisons, never in a `tracing` macro.

Security review (plan-wide rules): tokens/secrets never logged — the UI layer
never touches `TokenSet` secret fields at all (only `expires_at`,
`account.email`, labels); nothing added to `AppConfig`; the store is read
through 21's `KeyringStore` only; PKCE/state remain entirely inside 21's
controller; `demo_authorize_url` (the mock `state`) is memory-only and
excluded from logs; client secrets are nowhere (providers 22/23 own that
rule).

## Risks & Open Questions

Ranked risks:

1. **`install_with` visibility on plan 21.** This plan's tests need
   `pub(crate) fn install_with(store, io, cx)` (21 Step 8.2 extracts it as
   "the internal fn"). If 21 shipped it module-private, Phase 1.5 fails to
   compile; fix is widening to `pub(crate)` — one word inside 21's
   `src/services/auth.rs`, no behavior change. Tried: 21's Phase 8.2 text
   says "extract `install_with(store, io, cx)` as the internal fn both
   call" without stating visibility — hence this risk entry, not a guess.
2. **Blocking keyring read in the snapshot refresh.** `store.load` runs on
   the UI thread inside the controller observer, same acceptance as 21's
   risk 4 (keyring on entity context). Only on `SignedIn` transitions, one
   row. If it janks, move the read into a `cx.spawn` + weak-global hop
   (pattern already in 21's controller spawns).
3. **Single-provider controller vs multi-provider UI.** 21's controller
   holds one provider; this plan's registry enables a browser button only
   when `provider_id == spec.id`, so at most one social button lights up
   until plan 23 introduces provider selection or per-provider controllers.
   That is 23's design space; the registry rule is the documented contract
   (Step 3.1) so 23 changes one function.
4. **Demo-completion UX leaks the mock into product UI.** The "Complete
   demo sign-in" button renders only when `provider_id == "mock"` and a
   mock flow is pending; plans 22/23 make it unreachable without touching
   this plan (condition never true). The demo note likewise. Accepted for a
   boilerplate showcase.
5. **Plan-order collisions.** Seam files shared with plans 05-19
   (`sidebar.rs`, `route.rs`, `state.rs`, `pages/mod.rs`, e2e count,
   locale files) are additive per INDEX §5; the e2e count rule is "read the
   literal, +1" (Step 2.2). `title_bar.rs` is touched by 13's
   `now_playing_bar`? No — 13 touches render.rs, not title_bar (INDEX §5);
   this plan is alone in `title_bar.rs` among the 20 showcase plans.
6. **`Button::loading` + `disabled` interplay.** `loading` swaps the icon
   (button.rs:445-474) but I did not verify it blocks clicks by itself, so
   the submit sets `disabled` too (Step 4.3) — belt and braces, no reliance
   on unverified behavior. Visual confirmation is manual QA item 2.

Open questions (each with what was tried):

- **Plan 15 tutorial wraps cannot land until 15 exists.**
  `src/features/mod.rs` currently lists `command_palette`, `pages`,
  `palette` (read this session) — no `tutorial` module. Phase 10 is
  conditional with a grep guard; the five target ids are the reserved
  contract. Not blocking anything else.
- **Whether `Button::loading` implies disabled** — not read from source
  (only the setter at button.rs:445 was verified); worked around by setting
  `disabled` explicitly. If a reviewer wants single-flag styling, verify in
  `button.rs` render before removing the `disabled` call.
- **macOS/Windows behavior of the account menu and menu keyboard nav** is
  the kit's own surface; only Linux manual QA is executable from this
  planning host. Flagged for the implementer's platform pass (CONTRIBUTING
  targets macOS).
- **zh-CN copy review** — values in the i18n table were composed for this
  plan, not reviewed by a native speaker; the repo has no zh-CN reviewer
  convention documented (checked CONTRIBUTING.md + AGENTS.md — silent).
  Treat as draft-quality like the existing catalog's zh-CN entries.
- **`auth_no_expiry` breaks the page-prefix convention deliberately**
  (survey §5 says page-prefix everything else). Three surfaces share the
  string (sign-in card, settings row, diagnostics row) and no page owns it;
  alternatives (`sign_in_no_expiry` consumed by settings/diagnostics) read
  worse than the shared `auth_` feature prefix every other cross-surface
  key here follows. Kept; noted so it is a decision, not an oversight.
- **No cargo gate was run during planning** (doc-only scope, matching the
  surveys and plans 01-23). Earliest compile evidence points: Step 1.4
  (`cargo check -p gpui-starter`) and Step 2.2
  (`cargo test -p gpui-starter --test e2e_navigation`).
