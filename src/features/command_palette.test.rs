use super::page_binding;
use crate::commands::CommandId;
use crate::root::NavigateToPage;

/// NavigateToPage carries no Debug, so assertions compare its slot payload.
fn slot(id: CommandId) -> Option<usize> {
    page_binding(id).map(|NavigateToPage(slot)| slot)
}

#[test]
fn page_binding_uses_sidebar_slot_order() {
    assert_eq!(slot(CommandId::OpenHome), Some(0));
    assert_eq!(slot(CommandId::OpenForm), Some(1));
    assert_eq!(slot(CommandId::OpenSettings), Some(2));
    assert_eq!(slot(CommandId::OpenNotifications), Some(3));
    assert_eq!(slot(CommandId::OpenDiagnostics), Some(4));
    assert_eq!(slot(CommandId::OpenAbout), Some(8));
}

#[test]
fn page_binding_skips_slots_past_nine_and_non_page_commands() {
    // AppRoot binds NavigateToPage for the first nine Page::all slots only;
    // SystemControls sits at slot 9, SignIn and DesktopWidgets later still.
    assert_eq!(slot(CommandId::OpenSystemControls), None);
    assert_eq!(slot(CommandId::SignIn), None);
    assert_eq!(slot(CommandId::OpenDesktopWidgets), None);
    assert_eq!(slot(CommandId::Undo), None);
}
