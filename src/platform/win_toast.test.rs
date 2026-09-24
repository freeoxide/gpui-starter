use super::*;

#[test]
fn actions_category_adds_buttons() {
    let xml = toast_xml(&NotificationRequest::action_buttons("Title", "Body"));
    assert_eq!(
        actions_xml(&NotificationRequest::action_buttons("Title", "Body")),
        r#"<actions><action content="Open" arguments="settings.open" activationType="foreground"/><action content="Snooze" arguments="settings.snooze" activationType="foreground"/></actions>"#,
        "button ids must match the macOS category registrations"
    );
    assert!(xml.contains(r#"<text id="1">Title</text>"#));
    assert!(xml.contains(r#"<text id="3">Body</text>"#));
}

#[test]
fn reply_category_adds_input_bound_to_send() {
    let xml = actions_xml(&NotificationRequest::reply("Title", "Body"));
    assert_eq!(
        xml,
        r#"<actions><input id="reply" type="text" placeHolderContent="Type a reply"/><action content="Send" arguments="settings.reply" hint-inputId="reply"/></actions>"#,
        "the Send button must submit the reply input box"
    );
}

#[test]
fn plain_notification_has_no_actions_element() {
    let xml = toast_xml(&NotificationRequest::foreground("Title", "Body"));
    assert!(
        !xml.contains("<actions"),
        "uncategorized toasts carry no actions"
    );
}

#[test]
fn sound_flag_maps_onto_audio_element() {
    let mut quiet = NotificationRequest::foreground("Title", "Body");
    quiet.play_sound = false;
    assert!(toast_xml(&quiet).contains(r#"<audio silent="true" />"#));

    let loud = NotificationRequest::foreground("Title", "Body");
    let xml = toast_xml(&loud);
    assert!(xml.contains("ms-winsoundevent:Notification.Default"));
    assert!(!xml.contains("silent"));
}

#[test]
fn title_and_body_are_escaped() {
    let xml = toast_xml(&NotificationRequest::foreground(
        "A & <b> \"c\" 'd'",
        "5 > 4 & 3 < 4",
    ));
    assert!(xml.contains("<text id=\"1\">A &amp; &lt;b&gt; &quot;c&quot; &apos;d&apos;</text>"));
    assert!(xml.contains("<text id=\"3\">5 &gt; 4 &amp; 3 &lt; 4</text>"));
}

#[test]
fn thread_id_drives_tag_replacement() {
    let first = toast_tag(&NotificationRequest::reply("Title", "Body"));
    let second = toast_tag(&NotificationRequest::reply("Other", "Body"));
    assert_eq!(first, second, "same thread_id must reuse the toast slot");
    assert_eq!(first, "settings-reply");

    let actions = toast_tag(&NotificationRequest::action_buttons("Title", "Body"));
    assert_ne!(first, actions, "different threads must not collide");

    let long_thread = {
        let mut request = NotificationRequest::foreground("Title", "Body");
        request.thread_id = Some("settings-background-worthy-long".to_string());
        request
    };
    assert_eq!(toast_tag(&long_thread).len(), 16, "tag caps at 16 chars");
}

#[test]
fn unthreaded_requests_get_distinct_uuid_tags() {
    let request = NotificationRequest::foreground("Title", "Body");
    let first = toast_tag(&request);
    let second = toast_tag(&request);
    assert_ne!(first, second, "unthreaded toasts stack, never replace");
    for tag in [first, second] {
        // uuid simple format truncated to 16 chars: hex runs split at 8 and 13.
        assert_eq!(tag.len(), 16);
        assert!(tag.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-'));
        assert_eq!(tag.as_bytes()[8], b'-');
        assert_eq!(tag.as_bytes()[13], b'-');
    }
}
