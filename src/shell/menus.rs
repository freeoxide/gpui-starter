use gpui_kit::component::{
    ActiveTheme as _, GlobalState, Theme, ThemeMode, ThemeRegistry, menu::AppMenuBar,
};
use gpui_kit::{App, Entity, Menu, MenuItem, OwnedMenu, OwnedMenuItem, OwnedOsMenu, SharedString};

#[cfg(target_os = "macos")]
use gpui_kit::{SystemMenuType, actions};

use crate::{
    app::{About, ExecuteCommand, LocaleState, Quit, SelectLocale, SwitchTheme, SwitchThemeMode},
    commands::{self, CommandId},
};

#[cfg(target_os = "macos")]
actions!(menus, [Hide, HideOthers, ShowAll]);

pub fn init(title: impl Into<SharedString>, cx: &mut App) -> Entity<AppMenuBar> {
    let app_menu_bar = AppMenuBar::new(cx);
    let title: SharedString = title.into();
    update_app_menu(title.clone(), app_menu_bar.clone(), cx);

    cx.on_action({
        let title = title.clone();
        let app_menu_bar = app_menu_bar.clone();
        move |_: &SelectLocale, cx| {
            update_app_menu(title.clone(), app_menu_bar.clone(), cx);
        }
    });

    #[cfg(target_os = "macos")]
    {
        cx.on_action(|_: &Hide, cx| cx.hide());
        cx.on_action(|_: &HideOthers, cx| cx.hide_other_apps());
        cx.on_action(|_: &ShowAll, cx| cx.unhide_other_apps());
    }

    cx.observe_global::<Theme>({
        let title = title.clone();
        let app_menu_bar = app_menu_bar.clone();
        move |cx| {
            update_app_menu(title.clone(), app_menu_bar.clone(), cx);
        }
    })
    .detach();
    cx.observe_global::<LocaleState>({
        let title = title.clone();
        let app_menu_bar = app_menu_bar.clone();
        move |cx| {
            update_app_menu(title.clone(), app_menu_bar.clone(), cx);
        }
    })
    .detach();
    cx.observe_global::<crate::undo_stack::UndoState>({
        let title = title.clone();
        let app_menu_bar = app_menu_bar.clone();
        move |cx| {
            update_app_menu(title.clone(), app_menu_bar.clone(), cx);
        }
    })
    .detach();

    app_menu_bar
}

fn update_app_menu(title: impl Into<SharedString>, app_menu_bar: Entity<AppMenuBar>, cx: &mut App) {
    let title: SharedString = title.into();

    let menus = build_menus(title, cx);
    GlobalState::global_mut(cx).set_app_menus(owned_menus(&menus));
    cx.set_menus(menus);

    app_menu_bar.update(cx, |menu_bar, cx| {
        menu_bar.reload(cx);
    });
}

// Menu is not Clone (actions are boxed), so the second menu sink clones the
// owned form of a single build instead of rebuilding the whole tree.
fn owned_menus(menus: &[Menu]) -> Vec<OwnedMenu> {
    menus.iter().map(owned_menu).collect()
}

fn owned_menu(menu: &Menu) -> OwnedMenu {
    OwnedMenu {
        name: menu.name.clone(),
        items: menu.items.iter().map(owned_menu_item).collect(),
        disabled: menu.disabled,
    }
}

fn owned_menu_item(item: &MenuItem) -> OwnedMenuItem {
    match item {
        MenuItem::Separator => OwnedMenuItem::Separator,
        MenuItem::Submenu(submenu) => OwnedMenuItem::Submenu(owned_menu(submenu)),
        MenuItem::SystemMenu(os) => OwnedMenuItem::SystemMenu(OwnedOsMenu {
            name: os.name.clone(),
            menu_type: os.menu_type,
        }),
        MenuItem::Action {
            name,
            action,
            os_action,
            checked,
            disabled,
        } => OwnedMenuItem::Action {
            name: name.to_string(),
            action: action.boxed_clone(),
            os_action: *os_action,
            checked: *checked,
            disabled: *disabled,
        },
    }
}

fn build_menus(title: impl Into<SharedString>, cx: &App) -> Vec<Menu> {
    let title: SharedString = title.into();
    vec![
        Menu {
            name: title.clone(),
            items: app_menu_items(&title, cx),
            disabled: false,
        },
        Menu {
            name: crate::i18n::localize("menu_edit").into(),
            items: vec![
                MenuItem::action(
                    crate::i18n::localize("menu_edit_undo"),
                    ExecuteCommand(CommandId::Undo),
                )
                .disabled(!commands::availability(CommandId::Undo, cx).enabled),
                MenuItem::action(
                    crate::i18n::localize("menu_edit_redo"),
                    ExecuteCommand(CommandId::Redo),
                )
                .disabled(!commands::availability(CommandId::Redo, cx).enabled),
                MenuItem::separator(),
                MenuItem::action(
                    crate::i18n::localize("menu_edit_cut"),
                    gpui_kit::component::input::Cut,
                ),
                MenuItem::action(
                    crate::i18n::localize("menu_edit_copy"),
                    gpui_kit::component::input::Copy,
                ),
                MenuItem::action(
                    crate::i18n::localize("menu_edit_paste"),
                    gpui_kit::component::input::Paste,
                ),
                MenuItem::separator(),
                MenuItem::action(
                    crate::i18n::localize("menu_edit_select_all"),
                    gpui_kit::component::input::SelectAll,
                ),
            ],
            disabled: false,
        },
        Menu {
            name: crate::i18n::localize("menu_window").into(),
            items: vec![
                MenuItem::action(
                    crate::i18n::localize("menu_window_toggle_search"),
                    crate::app::ToggleSearch,
                ),
                MenuItem::action(
                    crate::i18n::localize("menu_window_diagnostics"),
                    ExecuteCommand(CommandId::OpenDiagnostics),
                ),
            ],
            disabled: false,
        },
        Menu {
            name: crate::i18n::localize("menu_go").into(),
            items: command_menu_items(
                &[
                    CommandId::OpenHome,
                    CommandId::OpenForm,
                    CommandId::OpenSettings,
                    CommandId::OpenNotifications,
                    CommandId::OpenDiagnostics,
                    CommandId::OpenAbout,
                    CommandId::CopyDiagnostics,
                    CommandId::OpenLogsFolder,
                    CommandId::OpenConfigFolder,
                ],
                cx,
            ),
            disabled: false,
        },
        Menu {
            name: crate::i18n::localize("menu_tools").into(),
            items: command_menu_items(
                &[CommandId::StartDemoTask, CommandId::CheckConnectivity],
                cx,
            ),
            disabled: false,
        },
    ]
}

#[cfg_attr(not(target_os = "macos"), allow(unused_variables))]
fn app_menu_items(title: &SharedString, cx: &App) -> Vec<MenuItem> {
    let mut items = vec![
        MenuItem::action(crate::i18n::localize("menu_about"), About),
        MenuItem::Separator,
        MenuItem::Submenu(Menu {
            name: crate::i18n::localize("menu_appearance").into(),
            items: vec![
                MenuItem::action(
                    crate::i18n::localize("menu_appearance_light"),
                    SwitchThemeMode(ThemeMode::Light),
                )
                .checked(!cx.theme().mode.is_dark()),
                MenuItem::action(
                    crate::i18n::localize("menu_appearance_dark"),
                    SwitchThemeMode(ThemeMode::Dark),
                )
                .checked(cx.theme().mode.is_dark()),
            ],
            disabled: false,
        }),
        theme_menu(cx),
        language_menu(),
    ];
    // Windows' hide_other_apps() is unimplemented!(), so the NSApp
    // conventions ship on macOS only.
    #[cfg(target_os = "macos")]
    items.extend([
        MenuItem::Separator,
        MenuItem::os_submenu("Services", SystemMenuType::Services),
        MenuItem::Separator,
        MenuItem::action(format!("Hide {title}"), Hide),
        MenuItem::action("Hide Others", HideOthers),
        MenuItem::action("Show All", ShowAll),
    ]);
    items.push(MenuItem::Separator);
    items.push(MenuItem::action(crate::i18n::localize("menu_quit"), Quit));
    items
}

fn language_menu() -> MenuItem {
    let locale = rust_i18n::locale().to_string();
    MenuItem::Submenu(Menu {
        name: crate::i18n::localize("menu_language").into(),
        items: vec![
            MenuItem::action("English", SelectLocale("en".into())).checked(locale == "en"),
            MenuItem::action("简体中文", SelectLocale("zh-CN".into())).checked(locale == "zh-CN"),
        ],
        disabled: false,
    })
}

fn theme_menu(cx: &App) -> MenuItem {
    let themes = ThemeRegistry::global(cx).sorted_themes();
    let current_name = cx.theme().theme_name();
    MenuItem::Submenu(Menu {
        name: crate::i18n::localize("menu_theme").into(),
        items: themes
            .iter()
            .map(|theme| {
                let checked = current_name == &theme.name;
                MenuItem::action(theme.name.clone(), SwitchTheme(theme.name.clone()))
                    .checked(checked)
            })
            .collect(),
        disabled: false,
    })
}

fn command_menu_items(command_ids: &[CommandId], cx: &App) -> Vec<MenuItem> {
    command_ids
        .iter()
        .filter_map(|id| {
            commands::spec(*id).map(|spec| {
                let availability = commands::availability(*id, cx);
                let title = crate::i18n::localize(spec.title_key);
                let title = match availability.disabled_reason {
                    Some(reason) => format!("{title} ({reason})"),
                    None => title,
                };
                MenuItem::action(title, ExecuteCommand(*id)).disabled(!availability.enabled)
            })
        })
        .collect()
}
