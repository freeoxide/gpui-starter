use gpui_kit::AppContext as _;
use gpui_kit::component::input::InputEvent;
use gpui_kit::{TestAppContext, WindowOptions};

use super::InputSection;

#[gpui_kit::test]
fn typing_in_the_name_field_updates_the_greeting(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (window, section) = cx
        .update(|cx| {
            gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
                cx.new(|cx| InputSection::new(window, cx))
            })
        })
        .expect("test window must open");
    cx.run_until_parked();

    section.update(cx, |state, _| {
        assert_eq!(state.greeting, "", "the greeting starts empty");
    });

    // set_value alone does not emit Change, so the event is emitted the way
    // the app's own tests drive input subscriptions.
    window
        .update(cx, |_, window, cx| {
            section.update(cx, |state, cx| {
                state.name_input.update(cx, |input, cx| {
                    input.set_value("Ada", window, cx);
                    cx.emit(InputEvent::Change);
                });
            });
        })
        .expect("window stays open for the test");
    cx.run_until_parked();

    section.update(cx, |state, _| {
        assert_eq!(state.greeting, "Hello, Ada!");
    });
}
