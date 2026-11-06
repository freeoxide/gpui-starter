use super::*;

#[test]
fn portal_notification_carries_the_themed_icon_name() {
    let request = NotificationRequest::foreground("title", "body");

    let notification = portal_notification(&request);

    // ashpd serializes the portal vardict through serde; each field rides a
    // {signature, value} pair, so the strings sit under ["value"].
    let json = serde_json::to_value(&notification).expect("portal notification serializes");
    assert_eq!(json["title"]["value"], "title");
    assert_eq!(json["body"]["value"], "body");
    let icon = json.get("icon").expect("icon field set");
    assert_eq!(icon["value"][0], "themed");
    assert_eq!(icon["value"][1]["value"][0].as_str(), Some(APP_ICON_NAME));
}
