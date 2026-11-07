use gpui_kit::{App, Global};
use serde::{Deserialize, Serialize};

use crate::{
    app_state, ids::NotificationId, notifications::NotificationBackendKind, time::AppTimestamp,
};

const MAX_INBOX_ITEMS: usize = 200;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationInboxItem {
    pub id: NotificationId,
    pub created_at: AppTimestamp,
    pub title: String,
    pub body: String,
    pub read: bool,
    pub backend: String,
    pub error_summary: Option<String>,
}

impl NotificationInboxItem {
    pub fn summary_line(&self) -> String {
        let read_state = crate::i18n::localize(if self.read {
            "notifications_status_read"
        } else {
            "notifications_status_unread"
        });
        format!("{} • {read_state}", self.backend)
    }
}

#[derive(Clone, Debug, Default)]
pub struct NotificationInboxState {
    pub items: Vec<NotificationInboxItem>,
}

impl Global for NotificationInboxState {}

pub fn initialize(cx: &mut App) {
    let items = app_state::with_config(cx, |config| config.notification_inbox.clone());
    cx.set_global(NotificationInboxState { items });
}

/// Borrow the inbox without cloning it; prefer this in render paths.
pub fn with_items<R>(cx: &App, f: impl FnOnce(&[NotificationInboxItem]) -> R) -> R {
    match cx.try_global::<NotificationInboxState>() {
        Some(state) => f(&state.items),
        None => f(&[]),
    }
}

/// Returns the number of unread inbox items without cloning the inbox.
pub fn unread_count(cx: &App) -> usize {
    with_items(cx, |items| items.iter().filter(|item| !item.read).count())
}

/// Single write path: the config list owns the data and the render global is
/// rebuilt from it, so no mutator can let UI and disk diverge.
fn write_inbox(cx: &mut App, mutate: impl FnOnce(&mut Vec<NotificationInboxItem>)) {
    app_state::update_config(cx, |config| {
        mutate(&mut config.notification_inbox);
        config.notification_inbox.truncate(MAX_INBOX_ITEMS);
    });
    let items = app_state::with_config(cx, |config| config.notification_inbox.clone());
    cx.set_global(NotificationInboxState { items });
}

pub fn record(item: NotificationInboxItem, cx: &mut App) {
    write_inbox(cx, |items| items.insert(0, item));
}

pub fn mark_all_read(cx: &mut App) {
    write_inbox(cx, |items| {
        for item in items {
            item.read = true;
        }
    });
}

pub fn clear_all(cx: &mut App) {
    write_inbox(cx, |items| items.clear());
}

pub struct NotificationAttemptRecord {
    pub title: String,
    pub body: String,
    pub backend: NotificationBackendKind,
    pub error_summary: Option<String>,
}

pub fn record_attempt(record_input: NotificationAttemptRecord, cx: &mut App) {
    record(
        NotificationInboxItem {
            id: NotificationId::new(),
            created_at: AppTimestamp::now(),
            title: record_input.title,
            body: record_input.body,
            read: false,
            backend: record_input.backend.to_string(),
            error_summary: record_input.error_summary,
        },
        cx,
    );
}
