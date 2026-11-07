use std::rc::Rc;

use gpui_kit::TestAppContext;

use super::super::model::{ChannelRow, Segment, WorkspaceData};
use super::ChannelDetail;

fn first_social_row(data: &WorkspaceData) -> ChannelRow {
    data.channels(Segment::Social)
        .into_iter()
        .next()
        .expect("social segment has rows")
}

#[gpui_kit::test]
async fn detail_starts_loading_and_finishes_ready(cx: &mut TestAppContext) {
    let data = Rc::new(WorkspaceData::load());
    let row = first_social_row(&data);
    let detail = cx.update(|cx| ChannelDetail::view(row, data, Default::default(), cx));
    cx.read(|cx| {
        let detail = detail.read(cx);
        assert_eq!(detail.phase, super::DetailPhase::Loading);
        assert_eq!(detail.series.len(), 90);
    });
    cx.executor().advance_clock(super::DETAIL_DELAY);
    cx.run_until_parked();
    cx.read(|cx| {
        assert_eq!(detail.read(cx).phase, super::DetailPhase::Ready);
    });
}
