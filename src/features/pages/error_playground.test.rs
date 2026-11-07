// No `use super::*`: mod.rs globs `gpui_kit::*`, dragging the kit `test`
// macro in and making `#[test]` resolve to it recursively.
use gpui_kit::AppContext as _;
use gpui_kit::TestAppContext;

use super::ErrorPlaygroundPage;
use super::helpers::ResultMsg;
use super::sections::FS_CHECK_PATH;

#[test]
fn new_has_all_results_none() {
    let page = ErrorPlaygroundPage::new();
    assert!(page.http_result.is_none());
    assert!(page.fs_result.is_none());
    assert!(page.async_result.is_none());
    assert!(page.background_panic_result.is_none());
}

#[gpui_kit::test]
async fn fs_check_reports_the_read_error_and_clear_resets_every_result(cx: &mut TestAppContext) {
    let _ = std::fs::remove_file(FS_CHECK_PATH);
    let page = cx.new(|_| ErrorPlaygroundPage::new());

    cx.update(|cx| page.update(cx, |page, cx| page.run_fs_check(cx)));
    cx.run_until_parked();
    cx.update(|cx| {
        let fs = page.read(cx).fs_result.clone().expect("fs check lands");
        assert!(fs.is_error, "reading a missing path must read as an error");
        assert!(
            fs.text.contains(FS_CHECK_PATH),
            "the error text names the path: {}",
            fs.text
        );

        page.update(cx, |page, cx| {
            page.http_result = Some(ResultMsg::info("requesting".into()));
            page.async_result = Some(ResultMsg::error("timed out".into()));
            page.background_panic_result = Some(ResultMsg::error("spawned".into()));
            page.clear_results(cx);
        });
        let page = page.read(cx);
        assert!(page.http_result.is_none());
        assert!(page.fs_result.is_none());
        assert!(page.async_result.is_none());
        assert!(page.background_panic_result.is_none());
    });
}
