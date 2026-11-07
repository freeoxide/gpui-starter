//! The kit-panel plumbing shared by the dock sections: one demo panel over a
//! spec, with the per-demo differences as fields.

use gpui_kit::component::{
    ActiveTheme as _, IconName, WindowExt as _,
    button::Button,
    dock::{BasePanel, Panel, PanelEvent},
    menu::PopupMenu,
};
use gpui_kit::*;

actions!(gallery_dock, [PanelInfo]);

/// One demo panel's identity and body copy.
pub(super) struct PanelSpec {
    pub(super) name: &'static str,
    pub(super) title: &'static str,
    pub(super) body: &'static str,
}

pub(super) struct DemoPanel {
    spec: &'static PanelSpec,
    focus_handle: FocusHandle,
    /// Shared hide-list; a panel on it reports itself invisible.
    hidden: Option<Entity<Vec<SharedString>>>,
    /// Whether the panel carries the info toolbar and menu entry.
    toolbar: bool,
    closable: bool,
}

impl DemoPanel {
    /// A plain body panel, as the DockStory arranges.
    pub(super) fn new(spec: &'static PanelSpec, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self {
            spec,
            focus_handle: cx.focus_handle(),
            hidden: None,
            toolbar: false,
            closable: true,
        })
    }

    /// A panel that hides with `hidden` and carries the info toolbar.
    pub(super) fn with_toolbar(
        spec: &'static PanelSpec,
        hidden: Entity<Vec<SharedString>>,
        cx: &mut App,
    ) -> Entity<Self> {
        cx.new(|cx| Self {
            spec,
            focus_handle: cx.focus_handle(),
            hidden: Some(hidden),
            toolbar: true,
            closable: true,
        })
    }

    pub(super) fn set_closable(&mut self, closable: bool) {
        self.closable = closable;
    }
}

impl EventEmitter<PanelEvent> for DemoPanel {}

impl Focusable for DemoPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl BasePanel for DemoPanel {
    fn panel_name(&self) -> &'static str {
        self.spec.name
    }

    fn closable(&self, _: &App) -> bool {
        self.closable
    }

    fn visible(&self, cx: &App) -> bool {
        let Some(hidden) = self.hidden.as_ref() else {
            return true;
        };
        !hidden
            .read(cx)
            .iter()
            .any(|name| name.as_ref() == self.spec.name)
    }
}

impl Panel for DemoPanel {
    fn title(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.spec.title
    }

    fn dropdown_menu(
        &mut self,
        menu: PopupMenu,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> PopupMenu {
        if self.toolbar {
            menu.menu("Info", Box::new(PanelInfo))
        } else {
            menu
        }
    }

    fn toolbar_buttons(&mut self, _: &mut Window, _: &mut Context<Self>) -> Option<Vec<Button>> {
        self.toolbar.then(|| {
            vec![
                Button::new(SharedString::from(format!("{}-info", self.spec.name)))
                    .icon(IconName::Info)
                    .on_click(|_, window, cx| {
                        window.push_notification("You clicked the info button.", cx);
                    }),
                Button::new(SharedString::from(format!("{}-search", self.spec.name)))
                    .icon(IconName::Search)
                    .on_click(|_, window, cx| {
                        window.push_notification("You clicked the search button.", cx);
                    }),
            ]
        })
    }
}

impl Render for DemoPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p_4()
            .text_color(cx.theme().foreground)
            .child(self.spec.body)
    }
}
