use gpui_kit::actions;
use gpui_kit::component::ThemeMode;
use gpui_kit::{Action, SharedString};

actions!(
    app,
    [
        About,
        Quit,
        ToggleSearch,
        OpenDiagnostics,
        TriggerTestPanic,
        Restart
    ]
);

#[derive(Action, Clone, PartialEq, Eq, serde::Deserialize)]
#[action(namespace = app, no_json)]
pub struct SelectLocale(pub SharedString);

#[derive(Action, Clone, PartialEq, Eq, serde::Deserialize)]
#[action(namespace = app, no_json)]
pub struct SelectFont(pub usize);

#[derive(Action, Clone, PartialEq, Eq, serde::Deserialize)]
#[action(namespace = app, no_json)]
pub struct SelectRadius(pub usize);

#[derive(Action, Clone, PartialEq, Eq, serde::Deserialize)]
#[action(namespace = app, no_json)]
pub struct ExecuteCommand(pub crate::commands::CommandId);

#[derive(Action, Clone, PartialEq, Eq, serde::Deserialize)]
#[action(namespace = app, no_json)]
pub struct SwitchTheme(pub SharedString);

#[derive(Action, Clone, PartialEq, Eq, serde::Deserialize)]
#[action(namespace = app, no_json)]
pub struct SwitchThemeMode(pub ThemeMode);
