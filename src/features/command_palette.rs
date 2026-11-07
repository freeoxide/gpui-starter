//! Command palette ("launcher"): a floating search window over the command
//! registry, rendered by the kit's [`Command`] component and filtered by
//! [`crate::features::palette`].

use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, FocusTrapElement as _, Icon, IconName, IndexPath, Root,
    Sizable as _,
    command::{Command, CommandItem, CommandState},
    h_flex,
    kbd::Kbd,
    v_flex,
};
use gpui_kit::{prelude::*, *};

use crate::accessibility::A11yExt as _;
use crate::commands::{self, CommandId};
use crate::features::palette::{ItemFilter, PaletteEntry};
use crate::page::Page;
use crate::root::NavigateToPage;

const LOG: &str = "gpui_starter::launcher";

/// Prevents double-opening the launcher.
pub struct LauncherOpen(pub bool);
impl Global for LauncherOpen {}

/// The open launcher window; a dead handle (destroyed elsewhere) must read
/// as closed, so toggle paths verify it against the live window list.
pub struct LauncherWindow(pub AnyWindowHandle);
impl Global for LauncherWindow {}

#[derive(Clone, Copy, Debug)]
pub enum LauncherActionKind {
    Execute(CommandId),
}

#[derive(Clone)]
pub struct LauncherItem {
    pub title: SharedString,
    pub subtitle: SharedString,
    pub icon: IconName,
    pub action: LauncherActionKind,
}

impl PaletteEntry for LauncherItem {
    fn name(&self) -> &str {
        &self.title
    }
    fn description(&self) -> Option<&str> {
        Some(&self.subtitle)
    }
}

pub enum LauncherEvent {
    Act(LauncherActionKind),
    Dismiss,
}

pub struct Launcher {
    state: Entity<CommandState>,
    /// Registry snapshot the fuzzy filter scores.
    items: Vec<LauncherItem>,
    /// Indices into `items` behind the rows the last render supplied; the
    /// palette's confirm IndexPath row indexes this list.
    matches: Vec<usize>,
    filter: ItemFilter,
}

impl EventEmitter<LauncherEvent> for Launcher {}

impl Focusable for Launcher {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.state.read(cx).focus_handle(cx)
    }
}

impl Launcher {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let state = cx.new(|cx| CommandState::new(window, cx));
        let items = Self::make_items();
        let matches = (0..items.len()).collect();

        Self {
            state,
            items,
            matches,
            filter: ItemFilter::default(),
        }
    }

    fn make_items() -> Vec<LauncherItem> {
        commands::REGISTRY
            .iter()
            .map(|command| LauncherItem {
                title: crate::i18n::localize(command.title_key).into(),
                subtitle: crate::i18n::localize(command.subtitle_key).into(),
                icon: command.icon.clone(),
                action: LauncherActionKind::Execute(command.id),
            })
            .collect()
    }

    /// Local filtering stays in [`crate::features::palette`]: its fuzzy scorer
    /// ranks description hits below name hits, which the component's own
    /// substring match does not do, so the palette runs `filterable(false)`
    /// and this callback supplies the rows each query.
    fn refilter(&mut self, query: &str, cx: &mut Context<Self>) {
        // ItemFilter lowercases the query once per pass; handing it the raw
        // value keeps a single lowercase per keystroke.
        self.matches = self.filter.filter_indices(&self.items, query);
        tracing::debug!(
            target: LOG,
            query = %query,
            results = self.matches.len(),
            "Launcher filtered"
        );
        cx.notify();
    }

    fn confirm(&mut self, path: IndexPath, cx: &mut Context<Self>) {
        if let Some(item) = self.matches.get(path.row).map(|&ix| &self.items[ix]) {
            tracing::info!(
                target: LOG,
                action = ?item.action,
                item = %item.title,
                "Launcher action triggered"
            );
            cx.emit(LauncherEvent::Act(item.action));
        } else {
            tracing::debug!(target: LOG, "Launcher dismissed with no selection");
        }
        cx.emit(LauncherEvent::Dismiss);
    }
}

impl Render for Launcher {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let entity = cx.entity();

        let rows: Vec<LauncherItem> = self
            .matches
            .iter()
            .map(|&ix| self.items[ix].clone())
            .collect();
        let titles: Vec<SharedString> = rows.iter().map(|row| row.title.clone()).collect();

        let command = Command::new(&self.state)
            .items(rows.into_iter().map(|row| {
                let LauncherActionKind::Execute(command_id) = row.action;
                let LauncherItem {
                    title,
                    subtitle,
                    icon,
                    ..
                } = row;
                // Availability is read per render so rows track the same
                // state the menus see (undo stack, sign-in, platform).
                let availability = commands::availability(command_id, cx);
                let subtitle = match &availability.disabled_reason {
                    Some(reason) => format!("{subtitle} ({reason})").into(),
                    None => subtitle,
                };
                let binding = page_binding(command_id);
                CommandItem::new()
                    .label(title.clone())
                    .disabled(!availability.enabled)
                    .child(move |window: &mut Window, cx: &mut App| {
                        render_row(
                            icon.clone(),
                            title.clone(),
                            subtitle.clone(),
                            binding.clone(),
                            window,
                            cx,
                        )
                    })
            }))
            .placeholder(crate::i18n::localize("palette_placeholder"))
            .filterable(false)
            .empty(|_: &CommandState, _: &mut Window, cx: &mut App| render_no_results(cx))
            .footer(move |state: &CommandState, _: &mut Window, cx: &mut App| {
                render_footer(state, &titles, cx)
            })
            .on_query({
                let entity = entity.clone();
                move |query, _window, cx| {
                    entity.update(cx, |launcher, cx| launcher.refilter(query, cx));
                }
            })
            .on_confirm({
                let entity = entity.clone();
                move |path, _window, cx| {
                    entity.update(cx, |launcher, cx| launcher.confirm(path, cx));
                }
            })
            .on_cancel(move |_window, cx| {
                entity.update(cx, |_, cx| cx.emit(LauncherEvent::Dismiss));
            })
            // The launcher window is a transparent borderless popup; the
            // palette surface and the window must stay see-through together.
            .bordered(false)
            .size_full()
            .max_h(DefiniteLength::Fraction(1.))
            .bg(transparent_black())
            .border_1()
            .border_color(theme.border.opacity(0.5))
            .rounded(theme.radius_lg);

        let surface = div()
            .id("launcher-surface")
            .a11y(Role::Dialog, crate::i18n::localize("palette_dialog"))
            .size_full()
            .child(command);

        // The trap container has an id but no a11y role, so gpui drops the
        // wrapped element's node; trap a roleless wrapper, not the Dialog.
        let focus_handle = self.focus_handle(cx);
        div()
            .size_full()
            .focus_trap("launcher", &focus_handle)
            .child(surface)
    }
}

/// Binding hint: pages reachable from the sidebar answer to NavigateToPage,
/// bound to secondary-1..9 over the first nine [`Page::all`] slots (AppRoot).
fn page_binding(id: CommandId) -> Option<NavigateToPage> {
    let page = match id {
        CommandId::OpenHome => Page::Home,
        CommandId::OpenForm => Page::Form,
        CommandId::OpenSettings => Page::Settings,
        CommandId::OpenNotifications => Page::Notifications,
        CommandId::OpenDiagnostics => Page::Diagnostics,
        CommandId::OpenAbout => Page::About,
        CommandId::OpenSystemControls => Page::SystemControls,
        CommandId::SignIn => Page::SignIn,
        CommandId::OpenDesktopWidgets => Page::DesktopWidgets,
        _ => return None,
    };
    let slot = Page::all()
        .iter()
        .position(|candidate| *candidate == page)?;
    (slot < 9).then_some(NavigateToPage(slot))
}

/// Row content (icon + two-line text + binding hint); the palette component
/// owns the row chrome and selection highlight.
fn render_row(
    icon: IconName,
    title: SharedString,
    subtitle: SharedString,
    binding: Option<NavigateToPage>,
    window: &mut Window,
    cx: &App,
) -> Div {
    let kbd = binding
        .as_ref()
        .and_then(|action| Kbd::binding_for_action(action, None, window));
    h_flex()
        .w_full()
        .gap_3()
        .items_center()
        .child(
            div()
                .flex_shrink_0()
                .size_8()
                .flex()
                .items_center()
                .justify_center()
                .rounded(cx.theme().radius)
                .bg(cx.theme().secondary)
                .child(Icon::new(icon).small()),
        )
        .child(
            v_flex()
                .flex_1()
                .overflow_hidden()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .truncate()
                        .child(title),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .truncate()
                        .child(subtitle),
                ),
        )
        .when_some(kbd, |row, kbd| row.child(kbd))
}

fn render_no_results(cx: &App) -> Stateful<Div> {
    let label = crate::i18n::localize("palette_no_results");
    div()
        .id("launcher-no-results")
        .a11y(Role::Paragraph, label.clone())
        .px_4()
        .py_8()
        .w_full()
        .flex()
        .items_center()
        .justify_center()
        .text_sm()
        .text_color(cx.theme().muted_foreground)
        .child(label)
}

/// Palette footer: a live-region status announcing the selection and result
/// count (documented in docs/accessibility-checklist.md) plus key hints.
fn render_footer(state: &CommandState, titles: &[SharedString], cx: &App) -> Div {
    let count = state.matched_count();
    let status_text = if count > 0 {
        format!("{count} {}", crate::i18n::localize("palette_results"))
    } else {
        crate::i18n::localize("palette_no_results")
    };
    // The live-region label must change only when the selection or the
    // filtered set changes, or every keystroke would be announced.
    let status_label = match state.selected_index() {
        Some(path) => titles
            .get(path.row)
            .map(|title| {
                format!(
                    "{title}, {} {} {count}",
                    path.row + 1,
                    crate::i18n::localize("palette_of"),
                )
            })
            .unwrap_or_else(|| status_text.clone()),
        None => status_text.clone(),
    };

    h_flex()
        .px_4()
        .py_2()
        .gap_4()
        .flex_shrink_0()
        .border_t_1()
        .border_color(cx.theme().border)
        .text_xs()
        .text_color(cx.theme().muted_foreground)
        .child(
            div()
                .id("launcher-status")
                .a11y(Role::Status, status_label)
                .a11y_live(accesskit::Live::Polite)
                .child(status_text),
        )
        .child(
            h_flex()
                .id("launcher-hints")
                .a11y(
                    Role::Paragraph,
                    crate::i18n::localize("palette_keyboard_help"),
                )
                .gap_4()
                .child(format!(
                    "↑↓  {}",
                    crate::i18n::localize("palette_hint_navigate")
                ))
                .child(format!("↵  {}", crate::i18n::localize("palette_hint_open")))
                .child(format!(
                    "esc  {}",
                    crate::i18n::localize("palette_hint_close")
                )),
        )
}

pub struct LauncherRoot {
    launcher: Entity<Launcher>,
    should_close: bool,
}

impl Focusable for LauncherRoot {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.launcher.focus_handle(cx)
    }
}

impl LauncherRoot {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Attaches an NSGlassEffectView to the native view tree; no GPUI patch.
        #[cfg(target_os = "macos")]
        crate::platform::liquid_glass::LiquidGlass::install(window);

        let launcher = cx.new(|cx| Launcher::new(window, cx));

        // Deferred until after the first layout pass
        let focus_handle = launcher.read(cx).focus_handle(cx);
        window.defer(cx, move |window, cx| {
            focus_handle.focus(window, cx);
        });

        cx.subscribe(&launcher, |this, _, ev: &LauncherEvent, cx| {
            match ev {
                LauncherEvent::Act(action) => {
                    tracing::info!(target: LOG, action = ?action, "LauncherRoot handling action");
                    match action {
                        LauncherActionKind::Execute(command_id) => {
                            // Rows render disabled, but availability can move
                            // while open; gate like the ExecuteCommand path.
                            let availability = commands::availability(*command_id, cx);
                            if availability.enabled {
                                tracing::info!(
                                    target: LOG,
                                    command = ?command_id,
                                    "Executing command"
                                );
                                commands::execute(*command_id, cx);
                            } else {
                                tracing::warn!(
                                    target: LOG,
                                    command = ?command_id,
                                    reason = ?availability.disabled_reason,
                                    "Command unavailable"
                                );
                            }
                        }
                    }
                }
                LauncherEvent::Dismiss => {
                    tracing::debug!(target: LOG, "LauncherRoot received Dismiss event");
                }
            }
            tracing::debug!(target: LOG, "Scheduling launcher window close");
            this.should_close = true;
            cx.notify();
        })
        .detach();

        // remove_window directly on blur: macOS changes the blur treatment on
        // deactivation, and a deferred close would flash.
        cx.observe_window_activation(window, |_, window, cx| {
            if !window.is_window_active() {
                tracing::debug!(target: LOG, "Launcher window deactivated — closing");
                forget_launcher(cx);
                window.remove_window();
            }
        })
        .detach();

        Self {
            launcher,
            should_close: false,
        }
    }
}

impl Render for LauncherRoot {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.should_close {
            self.should_close = false;
            forget_launcher(cx);
            tracing::info!(target: LOG, "Removing launcher window (deferred)");
            window.defer(cx, |window, cx| {
                // Hand OS focus to the main window before the popup leaves
                // gpui's map, or in-flight key-ups log "window not found".
                if let Some(root) = crate::app::window::root_window(cx) {
                    let _ = root.update(cx, |_, window, _| window.activate_window());
                }
                window.remove_window();
            });
        }

        div()
            .size_full()
            // The launcher window has no AppRoot; without its own handler
            // the ToggleSearch binding cannot reach open_launcher to close.
            .on_action(|_: &crate::app::ToggleSearch, _, cx| open_launcher(cx))
            .child(self.launcher.clone())
    }
}

pub fn open_launcher(cx: &mut App) {
    if cx.try_global::<LauncherOpen>().is_some_and(|g| g.0) {
        match live_launcher_window(cx) {
            // Toggle: a second request from any entry point (shortcut, IPC,
            // hotkey) closes the live launcher instead of being swallowed.
            Some(handle) => {
                tracing::debug!(target: LOG, "Launcher open — closing (toggle)");
                close_launcher(handle, cx);
                return;
            }
            // Flag set but the tracked window is gone: destroyed without our
            // close paths. Reset and fall through to open again.
            None if cx.has_global::<LauncherWindow>() => {
                cx.set_global(LauncherOpen(false));
            }
            None => {
                tracing::debug!(target: LOG, "Launcher opening — ignoring open request");
                return;
            }
        }
    }
    tracing::info!(target: LOG, "Opening launcher window");
    cx.set_global(LauncherOpen(true));

    let window_w = px(620.);
    let window_h = px(460.);

    let bounds = if let Some(display) = cx.primary_display() {
        let display_bounds = display.bounds();
        let x = display_bounds.origin.x + (display_bounds.size.width - window_w) / 2.;
        let y = display_bounds.origin.y + display_bounds.size.height * 0.12;
        Bounds {
            origin: point(x, y),
            size: size(window_w, window_h),
        }
    } else {
        Bounds {
            origin: point(px(200.), px(120.)),
            size: size(window_w, window_h),
        }
    };

    cx.spawn(async move |cx| {
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: None,
            focus: true,
            show: true,
            kind: WindowKind::PopUp,
            is_movable: true,
            is_resizable: false,
            // Same id as the main window (Wayland app_id / X11 WM_CLASS) so
            // both windows group under the .desktop StartupWMClass entry.
            app_id: Some("gpui-starter".to_string()),
            window_background: WindowBackgroundAppearance::Blurred,
            window_min_size: Some(Size {
                width: window_w,
                height: window_h,
            }),
            ..Default::default()
        };

        // Manual Root, not gpui_kit::open_window: the WindowState plugin
        // paints an opaque root surface; only a post-plugin bg stays clear.
        let Some(window) = cx
            .open_window(options, |window, cx| {
                let launcher_root = cx.new(|cx| LauncherRoot::new(window, cx));
                cx.new(|cx| Root::new(launcher_root, window, cx).bg(transparent_black()))
            })
            .ok()
        else {
            tracing::error!("failed to open launcher window");
            // The guard above would otherwise refuse every later open.
            cx.update(|cx| {
                cx.set_global(LauncherOpen(false));
            });
            return;
        };

        // Recorded so toggle paths can find the live window.
        cx.update(|cx| {
            cx.set_global(LauncherWindow(window.into()));
        });

        window
            .update(cx, |_, window, _| {
                window.activate_window();
                // WindowOptions has no title field; this is what names both
                // the WM window and the a11y root node.
                window.set_window_title(&crate::i18n::localize("palette_window_title"));
            })
            .ok();
    })
    .detach();
}

/// The tracked launcher window, but only while it is still listed as open;
/// a dead handle reads as "no launcher".
fn live_launcher_window(cx: &App) -> Option<AnyWindowHandle> {
    let handle = cx.try_global::<LauncherWindow>()?.0;
    cx.windows()
        .iter()
        .any(|open| open.window_id() == handle.window_id())
        .then_some(handle)
}

/// Reset both open-state globals; every close path funnels through here.
fn forget_launcher(cx: &mut App) {
    cx.set_global(LauncherOpen(false));
    if cx.has_global::<LauncherWindow>() {
        cx.remove_global::<LauncherWindow>();
    }
}

/// Clear open state and remove the launcher window, returning OS focus to
/// the root window first so in-flight key-ups are not lost with the popup.
fn close_launcher(handle: AnyWindowHandle, cx: &mut App) {
    forget_launcher(cx);
    if let Some(root) = crate::app::window::root_window(cx) {
        let _ = root.update(cx, |_, window, _| window.activate_window());
    }
    let _ = handle.update(cx, |_, window, _| window.remove_window());
}

#[cfg(test)]
#[path = "command_palette.test.rs"]
mod command_palette_test;
