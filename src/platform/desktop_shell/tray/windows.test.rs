use super::*;
use tray_icon::{
    TrayIconId,
    dpi::{PhysicalPosition, PhysicalSize},
    menu::MenuId,
};

fn tray_click(button: MouseButton, state: MouseButtonState) -> TrayIconEvent {
    TrayIconEvent::Click {
        id: TrayIconId::new("test-tray"),
        rect: tray_icon::Rect {
            size: PhysicalSize::new(16, 16),
            position: PhysicalPosition::new(0.0, 0.0),
        },
        position: PhysicalPosition::new(1.0, 2.0),
        button,
        button_state: state,
    }
}

#[test]
fn menu_lists_show_settings_and_quit_with_a_separator_before_quit() {
    let items = menu_items();
    let ids: Vec<&str> = items
        .iter()
        .filter_map(|item| match item {
            MenuItemSpec::Action { id, .. } => Some(*id),
            MenuItemSpec::Separator => None,
        })
        .collect();
    assert_eq!(ids, vec![SHOW_MENU_ID, SETTINGS_MENU_ID, QUIT_MENU_ID]);

    assert!(matches!(items[items.len() - 2], MenuItemSpec::Separator));
    assert_eq!(
        items
            .iter()
            .filter(|item| matches!(item, MenuItemSpec::Separator))
            .count(),
        1
    );

    for item in &items {
        if let MenuItemSpec::Action { label, .. } = item {
            assert!(!label.is_empty());
        }
    }
}

#[test]
fn every_menu_action_id_maps_to_an_action() {
    for item in menu_items() {
        if let MenuItemSpec::Action { id, .. } = item {
            assert!(action_for_menu_id(id).is_some(), "unmapped menu id: {id}");
        }
    }
}

#[test]
fn menu_id_mapping_is_exact() {
    assert!(matches!(
        action_for_menu_id(SHOW_MENU_ID),
        Some(TrayAction::ShowWindow)
    ));
    assert!(matches!(
        action_for_menu_id(SETTINGS_MENU_ID),
        Some(TrayAction::OpenSettings)
    ));
    assert!(matches!(
        action_for_menu_id(QUIT_MENU_ID),
        Some(TrayAction::Quit)
    ));
    assert!(action_for_menu_id("gpui-starter-show ").is_none());
    assert!(action_for_menu_id("GPUI-STARTER-QUIT").is_none());
    assert!(action_for_menu_id("").is_none());
}

#[test]
fn left_button_release_maps_to_open_launcher() {
    assert!(matches!(
        action_from_tray_event(&tray_click(MouseButton::Left, MouseButtonState::Up)),
        Some(TrayAction::OpenLauncher)
    ));
}

#[test]
fn other_tray_clicks_map_to_nothing() {
    assert!(
        action_from_tray_event(&tray_click(MouseButton::Left, MouseButtonState::Down)).is_none()
    );
    assert!(
        action_from_tray_event(&tray_click(MouseButton::Right, MouseButtonState::Up)).is_none()
    );
    assert!(
        action_from_tray_event(&tray_click(MouseButton::Middle, MouseButtonState::Up)).is_none()
    );
}

#[test]
fn menu_events_map_by_id() {
    let quit = MenuEvent {
        id: MenuId::new(QUIT_MENU_ID),
    };
    assert!(matches!(
        action_from_menu_event(&quit),
        Some(TrayAction::Quit)
    ));

    let unknown = MenuEvent {
        id: MenuId::new("not-a-tray-item"),
    };
    assert!(action_from_menu_event(&unknown).is_none());
}
