use gpui_kit::{prelude::*, *};

use crate::features::pages::{
    AboutPage, AnalyticsPage, AudioPage, CanvasShowcasePage, DesktopWidgetsPage, DiagnosticsPage,
    ErrorPlaygroundPage, FloatingUiPage, FormPage, HomePage, ImageEditorPage, InfiniteCanvasPage,
    MicrophonePage, ModelViewerPage, MultiuserCanvasPage, MusicPage, NotificationsPage,
    QueryDevToolsV2Page, QueryPlaygroundPage, RenderErrorPage, SettingsPage, ShaderCanvasPage,
    SignInPage, SystemControlsPage,
};
use crate::features::wgpu::WgpuPage;
use crate::gallery::GalleryPage;
use crate::page::Page;
use crate::title_bar::AppTitleBar;
use crate::{
    events::{self, AppEventKind},
    routes::AppRoute,
};

use super::super::actions::{NavigateToPage, ToggleSidebar};

pub struct AppRoot {
    pub(crate) focus_handle: FocusHandle,
    pub(crate) title_bar: Entity<AppTitleBar>,
    pub(crate) active_route: AppRoute,
    pub(crate) collapsed: bool,
    /// Last active-task count the TaskRegistry observer has seen.
    last_active_tasks: usize,
    pub(crate) home_page: Entity<HomePage>,
    pub(crate) form_page: Entity<FormPage>,
    pub(crate) settings_page: Entity<SettingsPage>,
    pub(crate) notifications_page: Entity<NotificationsPage>,
    pub(crate) diagnostics_page: Entity<DiagnosticsPage>,
    pub(crate) error_playground_page: Entity<ErrorPlaygroundPage>,
    pub(crate) query_playground_page: Entity<QueryPlaygroundPage>,
    pub(crate) query_devtools_v2_page: Entity<QueryDevToolsV2Page>,
    pub(crate) about_page: Entity<AboutPage>,
    pub(crate) system_controls_page: Entity<SystemControlsPage>,
    pub(crate) gallery_page: Entity<GalleryPage>,
    pub(crate) canvas_showcase_page: Entity<CanvasShowcasePage>,
    pub(crate) infinite_canvas_page: Entity<InfiniteCanvasPage>,
    pub(crate) multiuser_canvas_page: Entity<MultiuserCanvasPage>,
    pub(crate) floating_ui_page: Entity<FloatingUiPage>,
    pub(crate) image_editor_page: Entity<ImageEditorPage>,
    pub(crate) microphone_page: Entity<MicrophonePage>,
    pub(crate) audio_page: Entity<AudioPage>,
    pub(crate) analytics_page: Entity<AnalyticsPage>,
    pub(crate) music_page: Entity<MusicPage>,
    pub(crate) sign_in_page: Entity<SignInPage>,
    pub(crate) desktop_widgets_page: Entity<DesktopWidgetsPage>,
    pub(crate) now_playing_bar: Entity<crate::features::pages::music::NowPlayingBar>,
    pub(crate) wgpu_page: Entity<WgpuPage>,
    pub(crate) model_viewer_page: Entity<ModelViewerPage>,
    pub(crate) shader_canvas_page: Entity<ShaderCanvasPage>,
    pub(crate) floating_host: Entity<crate::ui::floating::OverlayFloatHost>,
    pub(crate) floating_ui_session: Entity<crate::features::pages::floating_ui::PaletteSession>,
    pub(crate) floating_ui_host: Entity<crate::features::pages::floating_ui::PalettePanelHost>,
    pub(crate) tutorial: Entity<crate::features::tutorial::TourController>,

    /// When true, the error fallback view replaces the active page; set on a
    /// render panic, cleared by "Reload Page" or navigation.
    pub(crate) render_error: bool,
    /// Cached error-boundary entity so we don't re-create it every frame.
    pub(crate) error_page: Option<Entity<RenderErrorPage>>,
}

impl AppRoot {
    pub fn new(
        title: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let title_bar = cx.new(|cx| AppTitleBar::new(title, cx));
        let home_page = cx.new(|_| HomePage::new());
        let form_page = cx.new(|cx| FormPage::new(window, cx));
        let settings_page = cx.new(|cx| SettingsPage::new(window, cx));
        let notifications_page = cx.new(|cx| NotificationsPage::new(window, cx));
        let diagnostics_page = cx.new(|cx| DiagnosticsPage::new(window, cx));
        let error_playground_page = cx.new(|_| ErrorPlaygroundPage::new());
        let query_playground_page = cx.new(|cx| QueryPlaygroundPage::new(window, cx));
        let query_devtools_v2_page = cx.new(|cx| QueryDevToolsV2Page::new(window, cx));
        let about_page = cx.new(|_| AboutPage::new());
        let system_controls_page = cx.new(|cx| SystemControlsPage::new(window, cx));
        let gallery_page = cx.new(|cx| GalleryPage::new(window, cx));
        let canvas_showcase_page = cx.new(CanvasShowcasePage::new);
        let infinite_canvas_page = cx.new(|cx| InfiniteCanvasPage::new(window, cx));
        let multiuser_canvas_page = cx.new(|cx| MultiuserCanvasPage::new(window, cx));
        let wgpu_page = cx.new(|cx| WgpuPage::new(window, cx));
        let model_viewer_page = cx.new(|cx| ModelViewerPage::new(window, cx));
        let shader_canvas_page = cx.new(|cx| ShaderCanvasPage::new(window, cx));
        let floating_host = cx.new(|cx| crate::ui::floating::OverlayFloatHost::new(window, cx));
        let floating_ui_session = cx.new(crate::features::pages::floating_ui::PaletteSession::new);
        let floating_ui_host = cx.new(|cx| {
            crate::features::pages::floating_ui::PalettePanelHost::new(
                floating_ui_session.clone(),
                window,
                cx,
            )
        });
        let floating_ui_page = cx.new(|cx| FloatingUiPage::new(floating_ui_session.clone(), cx));
        let image_editor_page = cx.new(|cx| ImageEditorPage::new(window, cx));
        let microphone_page = cx.new(|cx| MicrophonePage::new(window, cx));
        let audio_page = cx.new(|cx| AudioPage::new(window, cx));
        let analytics_page = cx.new(|cx| AnalyticsPage::new(window, cx));
        let (music_page, now_playing_bar) = crate::features::pages::music::init(window, cx);
        let sign_in_page = cx.new(|cx| SignInPage::new(window, cx));
        let desktop_widgets_page = cx.new(|cx| DesktopWidgetsPage::new(window, cx));
        let tutorial = cx.new(|cx| crate::features::tutorial::TourController::new(window, cx));
        cx.set_global(crate::features::tutorial::TourHandle(tutorial.clone()));
        cx.observe_global::<crate::app::window::RootWindow>(|this, cx| {
            this.floating_ui_session
                .update(cx, |session, cx| session.restore_windows(cx));
        })
        .detach();

        if !cx.has_global::<gpui_query::client::QueryClient>() {
            cx.set_global(gpui_query::client::QueryClient::new());
        }

        // SettingsPage's event log peeks this queue, so the page construction
        // above must stay first: peek observers must register before the drain.
        cx.observe_global::<events::AppEventQueue>(|_, cx| {
            for event in events::drain(cx) {
                match event.kind {
                    AppEventKind::Navigate(route) => {
                        defer_to_root_window(cx, move |this, window, cx| {
                            this.set_route(route, window, cx)
                        });
                    }
                    AppEventKind::DeepLinkReceived(link) => {
                        defer_to_root_window(cx, move |this, window, cx| {
                            handle_deep_link(this, &link, window, cx)
                        });
                    }
                    AppEventKind::AppError { message, severity } => {
                        tracing::warn!(target: "gpui_starter::root", error = %message, ?severity, "app error event received");
                        crate::error_surface::report(
                            message,
                            severity,
                            crate::error_surface::ErrorCategory::System,
                            vec![crate::error_surface::ErrorAction::Dismiss],
                            cx,
                        );
                        cx.notify();
                    }
                    AppEventKind::DiagnosticsChanged => {}
                    AppEventKind::PaletteChanged { palette, value } => {
                        crate::features::pages::floating_ui::apply_change(&palette, &value, cx);
                    }
                    AppEventKind::Test { message } => {
                        tracing::info!(target: "gpui_starter::root", message, "test event received");
                    }
                    AppEventKind::RemoteCommand(cmd) => {
                        // Forwarded commands arrive from a second instance over
                        // IPC. Translate each variant into its in-process action.
                        use crate::platform::ipc::command::ForwardedCommand;
                        match cmd {
                            ForwardedCommand::TogglePalette => {
                                crate::features::command_palette::open_launcher(cx);
                            }
                            ForwardedCommand::DeepLink(link) => {
                                defer_to_root_window(cx, move |this, window, cx| {
                                    handle_deep_link(this, &link, window, cx)
                                });
                            }
                            ForwardedCommand::ShowWindow => {
                                // A second launch raises the running app like
                                // tray Show; the recorded root avoids active_window().
                                if let Some(root) = crate::app::window::root_window(cx)
                                    && let Err(err) =
                                        root.update(cx, |_, window, _| window.activate_window())
                                {
                                    tracing::warn!(
                                        target: "gpui_starter::root",
                                        error = %err,
                                        "failed to raise the root window"
                                    );
                                }
                            }
                            ForwardedCommand::HideWindow
                            | ForwardedCommand::Quit
                            | ForwardedCommand::ReloadConfig => {
                                tracing::debug!(
                                    target: "gpui_starter::root",
                                    cmd = %cmd.label(),
                                    "remote command forwarded (no-op at boilerplate level)"
                                );
                            }
                        }
                    }
                }
            }
        })
        .detach();
        cx.observe_global::<crate::tasks::TaskRegistry>(|this, cx| {
            // The status bar's active count is the only rendered value; demo
            // ticks mutate the registry every 350ms without always moving it.
            let active = crate::tasks::active_count(cx);
            if this.last_active_tasks == active {
                return;
            }
            this.last_active_tasks = active;
            tracing::debug!(
                target: "gpui_starter::root::render",
                active_tasks = active,
                "TaskRegistry active count changed; notifying root"
            );
            cx.notify();
        })
        .detach();
        // The remaining globals only need a `cx.notify()` on change; the
        // `notify_on` helper collapses that duplicated closure.
        notify_on::<crate::notifications::NativeNotificationState>(cx).detach();
        notify_on::<crate::notifications::inbox::NotificationInboxState>(cx).detach();
        notify_on::<crate::connectivity::ConnectivitySnapshot>(cx).detach();
        notify_on::<crate::session::SessionSnapshot>(cx).detach();
        // The status bar reads both, so AppRoot must repaint when either
        // changes or its rows go stale until an unrelated notify.
        notify_on::<crate::updater::UpdateSnapshot>(cx).detach();
        notify_on::<crate::error_surface::ErrorSurfaceState>(cx).detach();
        // update_config already debounces; hand it the latest bounds directly.
        cx.observe_window_bounds(window, |_, window, cx| {
            let persisted =
                super::persisted_bounds(&window.window_bounds(), super::display_uuid(window, cx));
            crate::app_state::update_config(cx, |config| {
                config.window_bounds = Some(persisted);
            });
        })
        .detach();
        // gpui clears focus when the focused element leaves the tree (a
        // dismissed overlay); park the keyboard so it never goes dead.
        cx.on_focus_lost(window, |this, window, cx| {
            window
                .focus_lost_restore_target(cx)
                .unwrap_or_else(|| this.focus_handle.clone())
                .focus(window, cx);
        })
        .detach();

        // Borrow instead of cloning the whole AppConfig.
        let (active_route, collapsed) =
            crate::app_state::with_config(cx, |c| (c.active_route.clone(), c.sidebar_collapsed));
        let last_active_tasks = crate::tasks::active_count(cx);

        let pages = Page::all();
        let bindings = pages
            .iter()
            .enumerate()
            .take(9)
            .map(|(i, _)| {
                crate::app::keys::binding(&crate::app::keys::page(i), NavigateToPage(i), None, cx)
            })
            .chain(std::iter::once(crate::app::keys::binding(
                crate::app::keys::TOGGLE_SIDEBAR,
                ToggleSidebar,
                None,
                cx,
            )))
            .collect::<Vec<_>>();
        cx.bind_keys(bindings);

        Self {
            focus_handle: cx.focus_handle(),
            title_bar,
            active_route,
            collapsed,
            last_active_tasks,
            home_page,
            form_page,
            settings_page,
            notifications_page,
            diagnostics_page,
            error_playground_page,
            query_playground_page,
            query_devtools_v2_page,
            about_page,
            system_controls_page,
            gallery_page,
            canvas_showcase_page,
            infinite_canvas_page,
            multiuser_canvas_page,
            wgpu_page,
            model_viewer_page,
            shader_canvas_page,
            floating_host,
            floating_ui_session,
            floating_ui_host,
            floating_ui_page,
            image_editor_page,
            microphone_page,
            audio_page,
            analytics_page,
            music_page,
            sign_in_page,
            desktop_widgets_page,
            now_playing_bar,
            tutorial,
            render_error: false,
            error_page: None,
        }
    }

    /// Page view with error-boundary handling: a render panic since the last
    /// frame swaps in [`RenderErrorPage`]; navigation or "Reload Page" clears it.
    pub(crate) fn active_page_view(&mut self, cx: &mut Context<Self>) -> AnyView {
        if crate::lifecycle::take_render_panic() {
            tracing::warn!(
                target: "gpui_starter::root",
                "render panic detected – activating error boundary"
            );
            self.render_error = true;
            let summary = crate::lifecycle::last_panic_summary()
                .unwrap_or_else(|| crate::i18n::localize("render_error_unknown"));
            self.error_page = Some(cx.new(|_| RenderErrorPage::new(summary)));
        }

        if self.render_error
            && let Some(error_page) = &self.error_page
        {
            return error_page.clone().into();
        }

        self.unchecked_active_page_view()
    }

    pub(crate) fn unchecked_active_page_view(&self) -> AnyView {
        match self.active_route.page_for_render() {
            Page::Home => self.home_page.clone().into(),
            Page::Form => self.form_page.clone().into(),
            Page::Settings => self.settings_page.clone().into(),
            Page::Notifications => self.notifications_page.clone().into(),
            Page::Diagnostics => self.diagnostics_page.clone().into(),
            Page::ErrorPlayground => self.error_playground_page.clone().into(),
            Page::QueryPlayground => self.query_playground_page.clone().into(),
            Page::QueryDevToolsV2 => self.query_devtools_v2_page.clone().into(),
            Page::About => self.about_page.clone().into(),
            Page::SystemControls => self.system_controls_page.clone().into(),
            Page::Gallery => self.gallery_page.clone().into(),
            Page::CanvasShowcase => self.canvas_showcase_page.clone().into(),
            Page::InfiniteCanvas => self.infinite_canvas_page.clone().into(),
            Page::MultiuserCanvas => self.multiuser_canvas_page.clone().into(),
            Page::FloatingUi => self.floating_ui_page.clone().into(),
            Page::ImageEditor => self.image_editor_page.clone().into(),
            Page::Microphone => self.microphone_page.clone().into(),
            Page::Audio => self.audio_page.clone().into(),
            Page::Analytics => self.analytics_page.clone().into(),
            Page::Music => self.music_page.clone().into(),
            Page::SignIn => self.sign_in_page.clone().into(),
            Page::DesktopWidgets => self.desktop_widgets_page.clone().into(),
            Page::Wgpu => self.wgpu_page.clone().into(),
            Page::ModelViewer => self.model_viewer_page.clone().into(),
            Page::ShaderCanvas => self.shader_canvas_page.clone().into(),
        }
    }

    /// Persisted so restarts keep the choice.
    pub(crate) fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.collapsed = !self.collapsed;
        crate::app_state::update_config(cx, |config| {
            config.sidebar_collapsed = self.collapsed;
        });
        cx.notify();
    }

    pub(crate) fn set_route(
        &mut self,
        route: AppRoute,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.active_route == route {
            return;
        }
        let route_url = route.to_url();
        tracing::info!(target: "gpui_starter::root", route = ?route, route_url, "navigating");
        self.active_route = route.clone();
        self.render_error = false;
        self.error_page = None;
        // Wasm: push navigation into location.hash (bookmarkable, Back works);
        // the router bridge handles boot and back/forward via replaceState.
        #[cfg(target_family = "wasm")]
        crate::platform::web::router::push_route_hash(&route);
        crate::app_state::update_config(cx, |config| {
            config.active_route = route;
        });
        // The outgoing page's focus handle leaves the tree; give the keyboard
        // a home on the content area so Tab starts from a real element.
        self.focus_handle.focus(window, cx);
        cx.notify();
    }
}

/// Deep links arrive from both the event queue and IPC; auth callbacks are
/// consumed first, parse failures surface as app errors.
fn handle_deep_link(
    this: &mut AppRoot,
    link: &str,
    window: &mut Window,
    cx: &mut Context<AppRoot>,
) {
    if crate::services::auth::handle_callback_link(link, cx) {
        return;
    }
    match AppRoute::parse_deep_link(link) {
        Ok(route) => this.set_route(route, window, cx),
        Err(err) => events::emit_error(err, cx),
    }
}

/// Event observers run inside AppRoot's entity lease with no window in
/// scope; defer so the update runs outside the lease and gains the window.
fn defer_to_root_window<F>(cx: &mut Context<AppRoot>, f: F)
where
    F: FnOnce(&mut AppRoot, &mut Window, &mut Context<AppRoot>) + 'static,
{
    let Some(handle) = crate::app::window::root_window(cx).and_then(|h| h.downcast::<AppRoot>())
    else {
        return;
    };
    cx.defer(move |cx| {
        if let Err(err) = handle.update(cx, f) {
            tracing::warn!(
                target: "gpui_starter::root",
                error = %err,
                "root window unavailable for deferred navigation"
            );
        }
    });
}

/// Observe global `T` and notify `AppRoot` on change; caller detaches.
fn notify_on<T: Global>(cx: &mut Context<AppRoot>) -> Subscription {
    cx.observe_global::<T>(|_, cx| {
        cx.notify();
    })
}
