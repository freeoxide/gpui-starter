//! Command palette ("launcher"): a floating search window over the command
//! registry, backed by [`crate::features::palette`].

use std::rc::Rc;

use gpui::{prelude::*, *};
use gpui_component::{
    ActiveTheme as _, FocusTrapElement as _, Icon, IconName, Root, Sizable as _, h_flex,
    input::{Input, InputEvent, InputState},
    scroll::ScrollableElement as _,
    v_flex,
};

use crate::accessibility::A11yExt as _;
use crate::commands::{self, CommandId};
use crate::features::palette::{BaseDelegate, FuzzyMatchConfig, ItemFilter, PaletteEntry};

const LOG: &str = "gpui_starter::launcher";
const CONTEXT: &str = "Launcher";

actions!(launcher, [SelectNext, SelectPrev, Dismiss]);

pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("down", SelectNext, Some(CONTEXT)),
        KeyBinding::new("up", SelectPrev, Some(CONTEXT)),
        KeyBinding::new("escape", Dismiss, Some(CONTEXT)),
    ]);
}

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

// Launcher view: pure search UI, emits LauncherEvent.
pub enum LauncherEvent {
    Act(LauncherActionKind),
    Dismiss,
}

pub struct Launcher {
    focus_handle: FocusHandle,
    pub input: Entity<InputState>,
    state: BaseDelegate<LauncherItem>,
    filter: ItemFilter,
}

impl EventEmitter<LauncherEvent> for Launcher {}

impl Focusable for Launcher {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Launcher {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Search pages and commands…"));

        cx.subscribe(&input, |this, _, ev: &InputEvent, cx| match ev {
            InputEvent::Change => this.refilter(cx),
            InputEvent::PressEnter { .. } => this.act(cx),
            _ => {}
        })
        .detach();

        let state = BaseDelegate::new(Self::make_items());
        let filter = ItemFilter::new(FuzzyMatchConfig::default());

        Self {
            focus_handle: cx.focus_handle(),
            input,
            state,
            filter,
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

    fn refilter(&mut self, cx: &mut Context<Self>) {
        // ItemFilter lowercases the query once per pass; handing it the raw
        // value keeps a single lowercase per keystroke.
        let q = self.input.read(cx).value();
        let indices = self.filter.filter_indices(self.state.items(), &q);
        self.state.apply_filtered_indices(indices);
        tracing::debug!(
            target: LOG,
            query = %q,
            results = self.state.filtered_count(),
            "Launcher filtered"
        );
        cx.notify();
    }

    fn selected_display(&self) -> usize {
        self.state.selected_index().unwrap_or(0)
    }

    fn set_selected_display(&mut self, display_ix: usize) {
        self.state.set_selected_unchecked(display_ix);
    }

    fn item(&self, display_ix: usize) -> Option<&LauncherItem> {
        self.state.get_filtered_item(display_ix)
    }

    fn act(&mut self, cx: &mut Context<Self>) {
        if let Some(item) = self.item(self.selected_display()) {
            let action = item.action; // LauncherActionKind: Copy
            tracing::info!(
                target: LOG,
                action = ?action,
                item = %item.title,
                "Launcher action triggered"
            );
            cx.emit(LauncherEvent::Act(action));
        } else {
            tracing::debug!(target: LOG, "Launcher dismissed with no selection");
        }
        cx.emit(LauncherEvent::Dismiss);
    }
}

impl Render for Launcher {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let selected = self.selected_display();
        let filtered_count = self.state.filtered_count();
        let has_results = filtered_count > 0;

        // Snapshot the visible rows so the render closure holds only owned data.
        let rows: Vec<(usize, IconName, SharedString, SharedString)> = (0..filtered_count)
            .filter_map(|di| {
                let item = self.item(di)?;
                Some((
                    di,
                    item.icon.clone(),
                    item.title.clone(),
                    item.subtitle.clone(),
                ))
            })
            .collect();

        let status_text = if has_results {
            format!("{filtered_count} results")
        } else {
            "No results".to_string()
        };
        // The live-region label must change only when the selection or the
        // filtered set changes, or every keystroke would be announced.
        let status_label = match rows.iter().find(|(ix, _, _, _)| *ix == selected) {
            Some((_, _, title, _)) => format!("{title}, {} of {}", selected + 1, filtered_count),
            None => status_text.clone(),
        };

        let dialog = v_flex()
            .id("launcher-surface")
            .a11y(Role::Dialog, "Command palette")
            .size_full()
            .bg(theme.background.opacity(0.0))
            .border_1()
            .border_color(theme.border.opacity(0.5))
            .rounded(theme.radius_lg)
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(|this, _: &SelectNext, _, cx| {
                this.state.select_down();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &SelectPrev, _, cx| {
                this.state.select_up();
                cx.notify();
            }))
            .on_action(cx.listener(|_, _: &Dismiss, _, cx| {
                cx.emit(LauncherEvent::Dismiss);
            }))
            .child(
                h_flex()
                    .px_4()
                    .py(px(12.))
                    .gap_3()
                    .border_b_1()
                    .border_color(theme.border)
                    .items_center()
                    .child(
                        Icon::new(IconName::Search)
                            .size_5()
                            .text_color(theme.muted_foreground),
                    )
                    .child(
                        Input::new(&self.input)
                            .appearance(false)
                            .bordered(false)
                            .focus_bordered(false)
                            .aria_label("Search commands")
                            .flex_1(),
                    ),
            )
            .child(
                v_flex()
                    .id("launcher-results")
                    .a11y(Role::ListBox, "Results")
                    .aria_orientation(Orientation::Vertical)
                    // AT-SPI derives each item's setsize from the nearest
                    // ancestor that declares one; item-level values are ignored.
                    .aria_size_of_set(rows.len())
                    .flex_1()
                    .overflow_y_scrollbar()
                    .py_1()
                    .children(rows.iter().enumerate().map(|(display_ix, row)| {
                        let is_selected = display_ix == selected;
                        let icon = row.1.clone();
                        let title = row.2.clone();
                        let subtitle = row.3.clone();
                        let on_activate: Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)> =
                            Rc::new(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                this.set_selected_display(display_ix);
                                this.act(cx);
                            }));
                        let on_a11y_activate = on_activate.clone();

                        h_flex()
                            .id(display_ix)
                            .a11y(Role::ListBoxOption, title.clone())
                            .aria_description(subtitle.clone())
                            .aria_selected(is_selected)
                            // accesskit stores position_in_set 0-based; AT
                            // bridges report the stored value +1.
                            .aria_position_in_set(display_ix)
                            .aria_size_of_set(rows.len())
                            .px_3()
                            .py_2()
                            .mx_1()
                            .gap_3()
                            .items_center()
                            .rounded(theme.radius)
                            .cursor_pointer()
                            .when(is_selected, |el| el.bg(theme.list_active))
                            .when(!is_selected, |el| el.hover(|el| el.bg(theme.list_hover)))
                            .on_mouse_move(cx.listener(move |this, _, _, cx| {
                                if this.selected_display() != display_ix {
                                    this.set_selected_display(display_ix);
                                    cx.notify();
                                }
                            }))
                            .on_click(move |event, window, cx| {
                                on_activate(event, window, cx);
                            })
                            .on_a11y_action(AccessibleAction::Click, move |_, window, cx| {
                                on_a11y_activate(&ClickEvent::default(), window, cx);
                            })
                            .child(
                                div()
                                    .flex_shrink_0()
                                    .size_8()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(theme.radius)
                                    .bg(theme.secondary)
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
                                            .text_color(theme.muted_foreground)
                                            .truncate()
                                            .child(subtitle),
                                    ),
                            )
                    }))
                    .when(!has_results, |el| {
                        el.child(
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
                                .text_color(theme.muted_foreground)
                                .child("No results"),
                        )
                    }),
            )
            .child(
                h_flex()
                    .px_4()
                    .py(px(8.))
                    .gap_4()
                    .flex_shrink_0()
                    .border_t_1()
                    .border_color(theme.border)
                    .text_xs()
                    .text_color(theme.muted_foreground)
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
                    ),
            );

        // The trap container has an id but no a11y role, so gpui drops the
        // wrapped element's node — trap a roleless wrapper, not the Dialog.
        div()
            .size_full()
            .focus_trap("launcher", &self.focus_handle)
            .child(dialog)
    }
}

// LauncherRoot: standalone window root, handles events and closes the window.
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

        // Focus the search input after the first layout pass
        let input_fh = launcher.read(cx).input.read(cx).focus_handle(cx);
        window.defer(cx, move |window, cx| {
            input_fh.focus(window, cx);
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
            window.defer(cx, |window, _cx| {
                window.remove_window();
            });
        }

        let dialog_layer = Root::render_dialog_layer(window, cx);

        div()
            .size_full()
            .child(self.launcher.clone())
            .children(dialog_layer)
    }
}

// Open the launcher as a floating PopUp window.
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
            window_min_size: Some(gpui::Size {
                width: window_w,
                height: window_h,
            }),
            ..Default::default()
        };

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
