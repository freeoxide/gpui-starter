use gpui_kit::{App, BorrowAppContext as _, Global};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum SessionState {
    #[default]
    SignedOut,
    SigningIn,
    SignedIn {
        account_label: String,
    },
    Error(String),
}

#[derive(Clone, Debug, Default)]
pub struct SessionSnapshot {
    pub state: SessionState,
}

impl Global for SessionSnapshot {}

pub fn initialize(cx: &mut App) {
    cx.set_global(SessionSnapshot::default());
}

pub fn snapshot(cx: &App) -> SessionSnapshot {
    cx.try_global::<SessionSnapshot>()
        .cloned()
        .unwrap_or_default()
}

pub fn set_state(state: SessionState, cx: &mut App) {
    tracing::info!(target: "gpui_starter::session", state = ?state, "session state updated");
    cx.update_global::<SessionSnapshot, _>(|snapshot, _cx| {
        snapshot.state = state;
    });
}
