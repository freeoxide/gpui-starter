use gpui_kit::TestAppContext;

use super::*;

const ALL_COMMAND_IDS: &[CommandId] = &[
    CommandId::OpenHome,
    CommandId::OpenForm,
    CommandId::OpenSettings,
    CommandId::OpenNotifications,
    CommandId::OpenDiagnostics,
    CommandId::OpenAbout,
    CommandId::OpenSystemControls,
    CommandId::ThemeLight,
    CommandId::ThemeDark,
    CommandId::StartDemoTask,
    CommandId::CheckConnectivity,
    CommandId::CopyDiagnostics,
    CommandId::OpenLogsFolder,
    CommandId::OpenConfigFolder,
    CommandId::Undo,
    CommandId::Redo,
    CommandId::Restart,
    CommandId::OpenMenubarCharts,
    CommandId::StartTour,
    CommandId::SignIn,
    CommandId::SignOut,
    CommandId::OpenDesktopWidgets,
];

#[test]
fn registry_covers_every_command_exactly_once() {
    assert_eq!(REGISTRY.len(), ALL_COMMAND_IDS.len());
    for id in ALL_COMMAND_IDS {
        let listings = REGISTRY.iter().filter(|spec| spec.id == *id).count();
        assert_eq!(listings, 1, "{id:?} must appear exactly once");
        assert!(spec(*id).is_some(), "{id:?} must be reachable via spec()");
    }
}

#[test]
fn registry_keys_resolve_in_every_locale() {
    for locale in ["en", "zh-CN"] {
        rust_i18n::set_locale(locale);
        for spec in REGISTRY {
            assert_ne!(
                crate::i18n::localize(spec.title_key),
                spec.title_key,
                "title key {} has no {locale} translation",
                spec.title_key
            );
            assert_ne!(
                crate::i18n::localize(spec.subtitle_key),
                spec.subtitle_key,
                "subtitle key {} has no {locale} translation",
                spec.subtitle_key
            );
        }
    }
    rust_i18n::set_locale("en");
}

// With no clipboard backend the gate refuses inside execute() before the
// desktop action runs; with one the copy succeeds. Neither path may leave a
// failure record on the error surface.
#[gpui_kit::test]
async fn execute_refuses_commands_the_runtime_cannot_run(cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::desktop_actions::initialize(cx);
        crate::error_surface::initialize(cx);
    });
    cx.update(|cx| execute(CommandId::CopyDiagnostics, cx));
    let latest = cx.update(|cx| crate::error_surface::latest(cx));
    assert!(
        latest.is_none(),
        "an unavailable command must not report a failure: {latest:?}"
    );
}
