use gpui::{App, KeyBinding};
use gpui_component::{ActiveTheme as _, Root, WindowExt as _, text::markdown};

use crate::app::actions::*;
use crate::app::locale::set_locale;
use crate::app::theme::set_theme_mode;

// ---------------------------------------------------------------------------
// Init
// ---------------------------------------------------------------------------

/// Time and log a named startup step; the log name always matches the step.
macro_rules! startup_step {
    ($cx:expr, $name:expr, $body:block) => {{
        crate::lifecycle::set_startup_step($name, $cx);
        // clock (not std::time): Instant::now panics at runtime on wasm.
        let _t = crate::platform::clock::Instant::now();
        $body;
        tracing::info!(
            target: "gpui_starter::startup",
            elapsed_ms = _t.elapsed().as_millis() as u64,
            "{} done",
            $name
        );
    }};
}

pub fn init(cx: &mut App) {
    // clock (not std::time): Instant::now panics at runtime on wasm.
    let startup_start = crate::platform::clock::Instant::now();

    // Wasm: register embedded fonts before any text layout, or the first
    // render panics in `resolve_font` (browser tabs have no system fonts).
    #[cfg(target_family = "wasm")]
    {
        let fonts = crate::app::assets::embedded_font_bytes();
        let count = fonts.len();
        if let Err(err) = cx.text_system().add_fonts(fonts) {
            tracing::error!(
                target: "gpui_starter::startup",
                error = %err,
                "failed to register embedded fonts"
            );
        } else {
            tracing::info!(
                target: "gpui_starter::startup",
                count,
                "embedded fonts registered"
            );
        }
    }

    crate::lifecycle::install_panic_hook();

    // Must precede any gpui-component usage.
    startup_step!(cx, "component_init", {
        gpui_component::init(cx);
    });

    crate::lifecycle::set_stage(crate::lifecycle::LifecycleStage::Starting, cx);
    startup_step!(cx, "app_state_init", {
        crate::app_state::initialize(cx);
    });

    // Marker data must be user-owned, never $TMPDIR (symlink planting);
    // install it before the marker write so startup crashes stay detectable.
    crate::lifecycle::set_app_data_dir(crate::app_state::paths(cx).data_dir.clone());
    // Detect the previous run's marker BEFORE writing this launch's own.
    let previous_crash = crate::lifecycle::check_previous_crash();
    if let Some(marker) = &previous_crash {
        tracing::warn!(
            target: "gpui_starter::lifecycle",
            marker = %marker,
            "previous crash detected"
        );
    }
    crate::lifecycle::write_crash_marker();

    startup_step!(cx, "logging_init", {
        crate::logging::initialize(cx);
    });

    startup_step!(cx, "capabilities_init", {
        crate::capabilities::initialize(cx);
    });

    const DEFAULT_ENABLED_CAPABILITIES: &[&str] = &[
        "app_state",
        "deep_links",
        "diagnostics",
        "notification_inbox",
        "background_tasks",
        "status_bar",
        "connectivity",
        "session",
        "first_run",
        "launcher",
        "app_menu",
        "command_registry",
    ];
    for &name in DEFAULT_ENABLED_CAPABILITIES {
        crate::capabilities::set(
            name,
            crate::capabilities::CapabilityStatus::supported_enabled(),
            cx,
        );
    }

    // Initialize es-fluent i18n for app and form text
    let system_locale = crate::i18n::detect_system_locale();
    tracing::info!(
        target: "gpui_starter::startup",
        system_locale = %system_locale,
        "detected system locale"
    );
    if let Err(err) = crate::i18n::init_i18n(<_ as Into<
        es_fluent::unic_langid::LanguageIdentifier,
    >>::into(crate::app::Languages::default()))
    {
        tracing::error!("i18n initialization failed: {err}, using fallback locale");
    }

    let persisted = crate::app_state::config(cx);
    let locale_to_use = if persisted.locale.is_empty() {
        system_locale
    } else {
        persisted.locale.clone()
    };
    set_locale(&locale_to_use, cx);

    // Embedded themes are the bundle's only portable theme source; they
    // register before any registry lookup, on native and wasm alike.
    let persisted_theme = persisted.theme.clone();
    crate::app::theme::register_embedded_themes(cx);
    if let Some(theme) = gpui_component::ThemeRegistry::global(cx)
        .themes()
        .get(persisted_theme.as_str())
        .cloned()
    {
        gpui_component::Theme::global_mut(cx).apply_config(&theme);
    }

    // Hot reload of themes/ is dev-checkout-only: the watcher create_dir_all()s
    // missing dirs, so never point it at a path that should not exist.
    #[cfg(not(target_family = "wasm"))]
    {
        let themes_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("themes");
        if themes_dir.exists() {
            // watch_dir never returns Err (it logs internally); on_load
            // re-registers embedded themes after the initial reload.
            let _ = gpui_component::ThemeRegistry::watch_dir(
                themes_dir,
                cx,
                crate::app::theme::register_embedded_themes,
            );

            // Watcher reloads clear the registry without re-running on_load,
            // so a deleted dev theme file must not strand the embedded set.
            cx.observe_global::<gpui_component::ThemeRegistry>(|cx| {
                crate::app::theme::ensure_embedded_themes(cx);
            })
            .detach();
        }
    }

    if let Some(show) = persisted.scrollbar_show {
        gpui_component::Theme::global_mut(cx).scrollbar_mode = show;
    }
    cx.refresh_windows();

    cx.observe_global::<gpui_component::Theme>(move |cx| {
        let theme_name = cx.theme().theme_name().to_string();
        let scrollbar_show = cx.theme().scrollbar_mode;
        crate::app_state::update_config(cx, |config| {
            config.theme = theme_name;
            config.scrollbar_show = Some(scrollbar_show);
        });
    })
    .detach();

    // Theme switching actions
    cx.on_action(|switch: &SwitchTheme, cx| {
        if let Some(config) = gpui_component::ThemeRegistry::global(cx)
            .themes()
            .get(&switch.0)
            .cloned()
        {
            gpui_component::Theme::global_mut(cx).apply_config(&config);
        }
        cx.refresh_windows();
    });
    cx.on_action(|switch: &SwitchThemeMode, cx| {
        set_theme_mode(switch.0, cx);
    });
    cx.on_action(|locale: &SelectLocale, cx| {
        set_locale(&locale.0, cx);
    });

    crate::launcher::init(cx);
    cx.set_global(crate::events::AppEventQueue::default());
    cx.set_global(crate::launcher::LauncherOpen(false));
    startup_step!(cx, "runtime_services_init", {
        crate::tasks::initialize(cx);
        crate::error_surface::initialize(cx);
        crate::undo_stack::initialize(cx);
        crate::shortcuts::initialize(cx);
        cx.set_global(crate::services::tokio_runtime::TokioRuntimeGlobal(
            crate::services::tokio_runtime::TokioRuntime::new(),
        ));
        crate::connectivity::initialize(cx);
        crate::desktop_actions::initialize(cx);
        crate::accessibility::initialize(cx);
        crate::secure_storage::initialize(cx);
        crate::session::initialize(cx);
        crate::storage::initialize(cx);

        // SQLite is native-only; wasm storage is never available.
        #[cfg(not(target_family = "wasm"))]
        {
            crate::lifecycle::set_startup_step("db_migrations", cx);
            let migrations_t = std::time::Instant::now();
            if let Some(snapshot) = cx.try_global::<crate::storage::StorageSnapshot>()
                && snapshot.available
            {
                let db_path = std::path::PathBuf::from(snapshot.db_path.clone());
                match rusqlite::Connection::open(&db_path) {
                    Ok(conn) => match crate::db_migrations::run_migrations(&conn) {
                        Ok(version) => {
                            tracing::info!(
                                target: "gpui_starter::startup",
                                version,
                                elapsed_ms = migrations_t.elapsed().as_millis() as u64,
                                "db_migrations complete"
                            );
                        }
                        Err(err) => {
                            tracing::error!(
                                target: "gpui_starter::startup",
                                error = %err,
                                "db_migrations failed"
                            );
                            crate::lifecycle::set_startup_error(
                                format!("migration failed: {err}"),
                                cx,
                            );
                        }
                    },
                    Err(err) => {
                        tracing::error!(
                            target: "gpui_starter::startup",
                            error = %err,
                            "failed to open db for migrations"
                        );
                    }
                }
            }
        }

        crate::telemetry::initialize(cx);
    });
    // Window counts in the a11y snapshot go stale on close; refresh keeps the
    // diagnostics page honest.
    cx.on_window_closed(|cx, _| crate::accessibility::refresh(cx))
        .detach();
    crate::crash_report::initialize(cx);
    if previous_crash.is_some() {
        crate::crash_report::upload_pending_reports(cx);
    }
    crate::telemetry::record_event("app_runtime_initialized", cx);
    crate::notifications::inbox::initialize(cx);
    crate::notifications::initialize(cx);
    crate::notifications::set_native_notifications_enabled(
        persisted.native_notifications_enabled,
        cx,
    );
    crate::services::updater::initialize(cx);
    crate::services::updater::check_pending_swap(cx);

    // Wasm bridges (hash router, favicon/title, connectivity) install after
    // the globals they observe exist; native builds skip this.
    #[cfg(target_family = "wasm")]
    startup_step!(cx, "web_integrations", {
        crate::platform::web::install(cx);
    });

    // Key bindings
    cx.bind_keys([
        KeyBinding::new("cmd-k", ToggleSearch, None),
        KeyBinding::new("/", ToggleSearch, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-q", Quit, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("alt-f4", Quit, None),
        #[cfg(unix)]
        KeyBinding::new("ctrl-r", Restart, None),
    ]);

    cx.on_action(|_: &Quit, cx| {
        crate::lifecycle::set_shutdown_step("begin_shutdown", cx);
        crate::lifecycle::set_stage(crate::lifecycle::LifecycleStage::ShuttingDown, cx);

        // Persist the final window position even if the debounce has not fired.
        crate::lifecycle::set_shutdown_step("flush_window_bounds", cx);
        crate::root::flush_window_bounds(cx);

        crate::lifecycle::set_shutdown_step("drain_tasks", cx);

        let drain = crate::tasks::drain_with_timeout(std::time::Duration::from_secs(5), cx);

        cx.spawn(async move |cx| {
            drain.await;

            cx.update(|cx| {
                crate::lifecycle::set_shutdown_step("stop_ipc", cx);
                crate::single_instance::shutdown(cx);
                crate::lifecycle::set_shutdown_step("stop_watchers", cx);
                crate::desktop_actions::shutdown(cx);
                crate::lifecycle::set_shutdown_step("unregister_shortcuts", cx);
                crate::shortcuts::shutdown(cx);
                // Flush any debounced config changes before continuing shutdown.
                crate::lifecycle::set_shutdown_step("flush_config", cx);
                crate::app_state::force_save(cx);
                crate::lifecycle::set_shutdown_step("flush_storage", cx);
                crate::storage::shutdown(cx);
                crate::lifecycle::set_shutdown_step("flush_telemetry", cx);
                crate::telemetry::record_event("app_shutdown_requested", cx);
                crate::telemetry::shutdown(cx);
                crate::lifecycle::set_shutdown_step("flush_logs", cx);
                crate::logging::shutdown(cx);
                crate::lifecycle::set_shutdown_step("flush_crash_reports", cx);
                crate::crash_report::shutdown(cx);
                crate::lifecycle::set_shutdown_step("remove_crash_marker", cx);
                crate::lifecycle::remove_crash_marker();
                crate::lifecycle::set_shutdown_step("quit", cx);
                cx.quit();
            });
        })
        .detach();
    });

    cx.on_action(|_: &Restart, cx| {
        // Flag the re-exec, then reuse the full Quit shutdown path so every
        // flush runs before the process exits.
        #[cfg(unix)]
        {
            crate::app::request_reload();
            crate::lifecycle::set_shutdown_step("restart", cx);
        }
        #[cfg(not(unix))]
        {
            let _ = cx;
            tracing::warn!(
                target: "gpui_starter::reload",
                "restart requested on a platform without exec-reload support; ignoring"
            );
            return;
        }
        #[cfg(unix)]
        cx.dispatch_action(&Quit);
    });

    cx.on_action(|_: &About, cx| {
        if let Some(window) = cx.active_window().and_then(|w| w.downcast::<Root>()) {
            cx.defer(move |cx| {
                window
                    .update(cx, |_, window, cx| {
                        window.defer(cx, |window, cx| {
                            window.open_alert_dialog(cx, |alert, _, _| {
                                alert.title("About").description(markdown(
                                    "GPUI Starter\n\n\
                                    Version 0.1.0\n\n\
                                    A boilerplate for GPUI desktop apps.",
                                ))
                            });
                        });
                    })
                    .ok();
            });
        }
    });
    cx.on_action(|_: &OpenDiagnostics, cx| {
        crate::events::emit(
            crate::events::AppEventKind::Navigate(crate::routes::AppRoute::page(
                crate::sidebar::Page::Diagnostics,
            )),
            cx,
        );
    });
    #[cfg(debug_assertions)]
    cx.on_action(|_: &TriggerTestPanic, _cx| {
        panic!("gpui-starter test panic action");
    });
    cx.on_action(|command: &ExecuteCommand, cx| {
        let availability = crate::commands::availability(command.0, cx);
        if !availability.enabled {
            let reason = availability
                .disabled_reason
                .as_ref()
                .map(|value| value.to_string())
                .unwrap_or_else(|| "command disabled".to_string());
            tracing::warn!(
                target: "gpui_starter::commands",
                command = ?command.0,
                reason = %reason,
                "command ignored"
            );
            return;
        }
        crate::commands::execute(command.0, cx);
    });

    cx.activate(true);
    crate::lifecycle::set_startup_step("running", cx);
    crate::lifecycle::set_stage(crate::lifecycle::LifecycleStage::Running, cx);

    tracing::info!(
        target: "gpui_starter::startup",
        total_elapsed_ms = startup_start.elapsed().as_millis() as u64,
        "startup complete"
    );
}
