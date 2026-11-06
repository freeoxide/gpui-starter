use gpui_kit::TestAppContext;
use gpui_kit::component::input::InputEvent;

use super::InputSection;

#[gpui_kit::test]
fn typing_in_the_name_field_updates_the_greeting(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let (section, cx) = cx.add_window_view(|window, cx| InputSection::view(window, cx));
    cx.run_until_parked();

    cx.update(|_, cx| {
        section.update(cx, |state, _| {
            assert_eq!(state.greeting, "", "the greeting starts empty");
        });
    });

    // set_value alone does not emit Change, so the event is emitted the way
    // the app's own tests drive input subscriptions.
    cx.update(|window, cx| {
        section.update(cx, |state, cx| {
            state.name_input.update(cx, |input, cx| {
                input.set_value("Ada".to_string(), window, cx);
                cx.emit(InputEvent::Change);
            });
        });
    });
    cx.run_until_parked();

    cx.update(|_, cx| {
        section.update(cx, |state, _| {
            assert_eq!(state.greeting, "Hello, Ada!");
        });
    });
}
