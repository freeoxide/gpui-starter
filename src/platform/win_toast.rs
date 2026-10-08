//! Interactive Windows toasts. user-notify 0.4's Windows backend emits
//! action-less toast XML, so this module hand-rolls the send with the
//! windows crate: action buttons, a quick-reply input, and the
//! Activated/Dismissed plumbing that reports through a shared sink.

use std::sync::{Arc, OnceLock};

use windows::Data::Xml::Dom::XmlDocument;
use windows::Foundation::{IReference, TypedEventHandler};
use windows::UI::Notifications::{
    ToastActivatedEventArgs, ToastDismissalReason, ToastDismissedEventArgs, ToastNotification,
    ToastNotificationManager,
};
use windows::core::{HSTRING, IInspectable, Interface, Ref};

use crate::notifications::{
    ACTION_OPEN, ACTION_REPLY, ACTION_SNOOZE, CATEGORY_ACTIONS, CATEGORY_REPLY, DESKTOP_ENTRY_ID,
    NotificationRequest,
};

const LOG: &str = "gpui_starter::win_toast";
const REPLY_INPUT_ID: &str = "reply";
// Toast identity is the (group, tag) pair; the constant group scopes the
// thread-derived tags the way the web backend's `tag` field does.
const MESSAGE_GROUP: &str = "gpui-starter";

/// One callback shared by every toast, mirroring user-notify's single
/// manager callback; the `tag` field tells responses apart.
pub type ResponseSink = Arc<OnceLock<Box<dyn Fn(ToastResponse) + Send + Sync + 'static>>>;

#[derive(Debug)]
pub struct ToastResponse {
    pub tag: String,
    pub action: ResponseAction,
    pub user_text: Option<String>,
}

#[derive(Debug)]
pub enum ResponseAction {
    Default,
    Dismiss,
    Other(String),
}

fn toast_xml(request: &NotificationRequest) -> String {
    let actions = actions_xml(request);
    let audio = if request.play_sound {
        r#"<audio src="ms-winsoundevent:Notification.Default" />"#
    } else {
        r#"<audio silent="true" />"#
    };
    format!(
        r#"<toast duration="short"><visual><binding template="ToastGeneric"><text id="1">{}</text><text id="3">{}</text></binding></visual>{actions}{audio}</toast>"#,
        escaped(&request.title),
        escaped(&request.body),
    )
}

fn actions_xml(request: &NotificationRequest) -> String {
    match request.category.as_deref() {
        Some(CATEGORY_ACTIONS) => format!(
            r#"<actions><action content="{}" arguments="{ACTION_OPEN}" activationType="foreground"/><action content="{}" arguments="{ACTION_SNOOZE}" activationType="foreground"/></actions>"#,
            escaped(&crate::i18n::localize("toast_open")),
            escaped(&crate::i18n::localize("toast_snooze")),
        ),
        Some(CATEGORY_REPLY) => format!(
            r#"<actions><input id="{REPLY_INPUT_ID}" type="text" placeHolderContent="{}"/><action content="{}" arguments="{ACTION_REPLY}" hint-inputId="{REPLY_INPUT_ID}"/></actions>"#,
            escaped(&crate::i18n::localize("toast_reply_placeholder")),
            escaped(&crate::i18n::localize("toast_send")),
        ),
        _ => String::new(),
    }
}

fn escaped(text: &str) -> std::borrow::Cow<'_, str> {
    quick_xml::escape::escape(text)
}

/// Windows replaces a toast only on a matching (group, tag) pair, so the tag
/// carries the thread id — capped at the pre-Creators-Update limit of 16
/// characters, the same cap user-notify's uuid tags use. Unthreaded sends
/// get a fresh uuid prefix and stack instead of replacing.
fn toast_tag(request: &NotificationRequest) -> String {
    match &request.thread_id {
        Some(thread_id) => thread_id.chars().take(16).collect(),
        None => uuid::Uuid::new_v4().to_string()[..16].to_owned(),
    }
}

pub fn send(request: &NotificationRequest, sink: &ResponseSink) -> anyhow::Result<()> {
    let tag = toast_tag(request);

    let xml = XmlDocument::new()?;
    xml.LoadXml(&HSTRING::from(toast_xml(request)))?;

    let toast = ToastNotification::CreateToastNotification(&xml)?;
    toast.SetTag(&HSTRING::from(&tag))?;
    toast.SetGroup(&HSTRING::from(MESSAGE_GROUP))?;

    // WinRT interface types are !Send, so every WinRT object stays local to
    // this frame; the closures below capture only Send types (sink + tag).
    let activated = {
        let sink = Arc::clone(sink);
        let tag = tag.clone();
        TypedEventHandler::new(move |_, args: Ref<'_, IInspectable>| {
            let response = ToastResponse {
                tag: tag.clone(),
                action: activated_action(&args),
                user_text: reply_text(&args),
            };
            if let Some(handler) = sink.get() {
                handler(response);
            }
            Ok(())
        })
    };
    let dismissed = {
        let sink = Arc::clone(sink);
        let tag = tag.clone();
        TypedEventHandler::new(move |_, args: Ref<'_, ToastDismissedEventArgs>| {
            let user_canceled = args
                .as_ref()
                .and_then(|args| args.Reason().ok())
                .is_some_and(|reason| reason == ToastDismissalReason::UserCanceled);
            if user_canceled && let Some(handler) = sink.get() {
                handler(ToastResponse {
                    tag: tag.clone(),
                    action: ResponseAction::Dismiss,
                    user_text: None,
                });
            }
            Ok(())
        })
    };

    toast.Activated(&activated)?;
    toast.Dismissed(&dismissed)?;

    let notifier =
        ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(DESKTOP_ENTRY_ID))?;
    notifier.Show(&toast)?;
    tracing::debug!(target: LOG, tag = %tag, "interactive toast shown");
    Ok(())
}

fn activated_action(args: &Ref<'_, IInspectable>) -> ResponseAction {
    let arguments = args
        .as_ref()
        .and_then(|insp| insp.cast::<ToastActivatedEventArgs>().ok())
        .and_then(|event_args| event_args.Arguments().ok())
        .filter(|arguments| !arguments.is_empty());
    match arguments {
        Some(arguments) => ResponseAction::Other(arguments.to_string()),
        None => ResponseAction::Default,
    }
}

/// Quick-reply text: the `<input id="reply">` box surfaces in
/// `ToastActivatedEventArgs::UserInput` as an `IReference<HSTRING>` box.
fn reply_text(args: &Ref<'_, IInspectable>) -> Option<String> {
    let event_args = args.as_ref()?.cast::<ToastActivatedEventArgs>().ok()?;
    let input = event_args.UserInput().ok()?;
    let value = input.Lookup(&HSTRING::from(REPLY_INPUT_ID)).ok()?;
    let text = value.cast::<IReference<HSTRING>>().ok()?.Value().ok()?;
    Some(text.to_string())
}

#[cfg(test)]
#[path = "win_toast.test.rs"]
mod win_toast_test;
