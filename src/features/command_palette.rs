//! Command palette ("launcher"): a floating search window over the command
//! registry, rendered by the kit's [`Command`] component and filtered by
//! [`crate::features::palette`].

use gpui_kit::component::{
    ActiveTheme as _, FocusTrapElement as _, Icon, IconName, IndexPath, Root, Sizable as _,
    command::{Command, CommandItem, CommandState},
    h_flex, v_flex,
};
use gpui_kit::{prelude::*, *};

use crate::accessibility::A11yExt as _;
use crate::commands::{self, CommandId};
use crate::features::palette::{FuzzyMatchConfig, ItemFilter, PaletteEntry};

const LOG: &str = "gpui_starter::launcher";

// Prevents double-opening the launcher
pub struct LauncherOpen(pub bool);
impl Global for LauncherOpen {}

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
            filter: ItemFilter::new(FuzzyMatchConfig::default()),
        }
    }

    fn make_items() -> Vec<LauncherItem> {
        commands::registry()
            .into_iter()
            .map(|command| LauncherItem {
                title: command.title,
                subtitle: command.subtitle,
                icon: command.icon,
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
                let icon = row.icon.clone();
                let title = row.title;
                let subtitle = row.subtitle;
                CommandItem::new().label(title.clone()).child(
                    move |_: &mut Window, cx: &mut App| {
                        render_row(icon.clone(), title.clone(), subtitle.clone(), cx)
                    },
                )
            }))
            .placeholder("Search pages and commands…")
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
            .a11y(Role::Dialog, "Command palette")
            .size_full()
            .child(command);

        // The trap container has an id but no a11y role, so gpui drops the
        // wrapped element's node — trap a roleless wrapper, not the Dialog.
        let focus_handle = self.focus_handle(cx);
        div()
            .size_full()
            .focus_trap("launcher", &focus_handle)
            .child(surface)
    }
}

/// Row content (icon + two-line text); the palette component owns the row
/// chrome, selection highlight, and trailing slot.
fn render_row(icon: IconName, title: SharedString, subtitle: SharedString, cx: &App) -> Div {
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
}

fn render_no_results(cx: &App) -> Stateful<Div> {
    div()
        .id("launcher-no-results")
        .a11y(Role::Paragraph, "No results")
        .px_4()
        .py_8()
        .w_full()
        .flex()
        .items_center()
        .justify_center()
        .text_sm()
        .text_color(cx.theme().muted_foreground)
        .child("No results")
}

/// Palette footer: a live-region status announcing the selection and result
/// count (documented in docs/accessibility-checklist.md) plus key hints.
fn render_footer(state: &CommandState, titles: &[SharedString], cx: &App) -> Div {
    let count = state.matched_count();
    let status_text = if count > 0 {
        format!("{count} results")
    } else {
        "No results".to_string()
    };
    // The live-region label must change only when the selection or the
    // filtered set changes, or every keystroke would be announced.
    let status_label = match state.selected_index() {
        Some(path) => titles
            .get(path.row)
            .map(|title| format!("{title}, {} of {}", path.row + 1, count))
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
                    "Keyboard: up and down navigate, Enter opens, Escape closes",
                )
                .gap_4()
                .child("↑↓  navigate")
                .child("↵  open")
                .child("esc  close"),
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
        // Install Liquid Glass — creates NSGlassEffectView directly in the
        // native view hierarchy, no GPUI source patches needed.
        #[cfg(target_os = "macos")]
        crate::platform::liquid_glass::LiquidGlass::install(window, &Default::default());

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
                            tracing::info!(
                                target: LOG,
                                command = ?command_id,
                                "Executing command"
                            );
                            commands::execute(*command_id, cx);
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
                cx.set_global(LauncherOpen(false));
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
            cx.set_global(LauncherOpen(false));
            tracing::info!(target: LOG, "Removing launcher window (deferred)");
            #[cfg(target_os = "windows")]
            {
                // Hand OS focus to the main window before the popup leaves
                // gpui's map, or later key-ups log "window not found".
                let handle = window.window_handle();
                cx.spawn(async move |_this, cx| {
                    cx.update(|cx| {
                        if let Some(root) = crate::app::window::root_window(cx) {
                            let _ = root.update(cx, |_, window, _| window.activate_window());
                        }
                    });
                    cx.background_executor()
                        .timer(std::time::Duration::from_millis(50))
                        .await;
                    cx.update(|cx| {
                        let _ = handle.update(cx, |_, window, _| window.remove_window());
                    });
                })
                .detach();
            }
            #[cfg(not(target_os = "windows"))]
            window.defer(cx, |window, _cx| {
                window.remove_window();
            });
        }

        div().size_full().child(self.launcher.clone())
    }
}

pub fn open_launcher(cx: &mut App) {
    if cx.try_global::<LauncherOpen>().is_some_and(|g| g.0) {
        tracing::debug!(target: LOG, "Launcher already open — ignoring open request");
        return;
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
            return Ok::<_, anyhow::Error>(());
        };

        window
            .update(cx, |_, window, _| {
                window.activate_window();
                // WindowOptions has no title field; this is what names both
                // the WM window and the a11y root node.
                window.set_window_title("Command Palette");
            })
            .ok();

        Ok::<_, anyhow::Error>(())
    })
    .detach();
}
