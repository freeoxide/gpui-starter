//! Alert section, ported from the upstream `AlertStory`.

use gpui_kit::component::{IconName, Sizable as _, Size, alert::Alert, text::markdown, v_flex};
use gpui_kit::*;

use crate::gallery::registry::GallerySection;

use super::demo::{DemoToggle, demo_toolbar, section, size_dropdown};

pub struct AlertSection {
    size: Size,
    banner_visible: bool,
}

impl AlertSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self {
            size: Size::default(),
            banner_visible: true,
        })
    }
}

impl Render for AlertSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let size = self.size;

        v_flex()
            .w_full()
            .gap_6()
            .p_4()
            .on_action(cx.listener(|this, action: &DemoToggle, _, cx| {
                if let Some(size) = action.size() {
                    this.size = size;
                    cx.notify();
                }
            }))
            .child(demo_toolbar(vec![
                size_dropdown("alert-size", size).into_any_element(),
            ]))
            .child(
                section("alert-default", "Default")
                    .description("Title, icon, and rich text content.")
                    .w_2_3()
                    .child(
                        Alert::new(
                            "alert-default",
                            markdown(
                                "Your workspace is ready for the team.\n\
                                - **12 members** have access\n\
                                - Billing remains with the workspace owner",
                            ),
                        )
                        .with_size(size)
                        .title("Workspace settings saved"),
                    ),
            )
            .child(
                section("alert-variants", "Variants")
                    .description("Info, success, warning, and error states.")
                    .w_2_3()
                    .child(
                        v_flex()
                            .w_full()
                            .gap_3()
                            .child(
                                Alert::info(
                                    "alert-info",
                                    "Maintenance starts Friday at 22:00 UTC.",
                                )
                                .with_size(size)
                                .title("Scheduled maintenance"),
                            )
                            .child(
                                Alert::success(
                                    "alert-success",
                                    "The transfer is queued and usually settles within one business day.",
                                )
                                .with_size(size)
                                .title("Transfer submitted"),
                            )
                            .child(
                                Alert::warning(
                                    "alert-warning",
                                    "Two teammates still use recovery codes generated more than a year ago.\n\
                                    Ask them to generate a fresh set in Security settings.",
                                )
                                .with_size(size),
                            )
                            .child(
                                Alert::error(
                                    "alert-error",
                                    markdown(
                                        "Please verify your billing information and try again.\n\
                                    - Check your card details\n\
                                    - Ensure sufficient funds\n\
                                    - Verify billing address",
                                    ),
                                )
                                .with_size(size)
                                .title("Unable to process your payment."),
                            ),
                    ),
            )
            .child(
                section("alert-banner", "Banner")
                    .description("Full-width and closable alerts.")
                    .w_2_3()
                    .child(
                        v_flex()
                            .w_full()
                            .gap_2()
                            .child(
                                Alert::new(
                                    "alert-banner",
                                    "Reporting is read-only while the nightly ledger closes.",
                                )
                                .banner()
                                .on_close(cx.listener(|this, _, _, cx| {
                                    this.banner_visible = false;
                                    cx.notify();
                                }))
                                .visible(self.banner_visible)
                                .with_size(size),
                            )
                            .child(
                                Alert::info(
                                    "alert-banner-info",
                                    "A new desktop update will install after you restart.",
                                )
                                .banner()
                                .with_size(size),
                            )
                            .child(
                                Alert::success(
                                    "alert-banner-success",
                                    "All 1,284 records finished importing.",
                                )
                                .banner()
                                .with_size(size),
                            )
                            .child(
                                Alert::warning(
                                    "alert-banner-warning",
                                    "Your API key expires in 6 days. Rotate it before August 19.",
                                )
                                .banner()
                                .with_size(size),
                            )
                            .child(
                                Alert::error(
                                    "alert-banner-error",
                                    "Live updates are disconnected. Changes may be delayed.",
                                )
                                .banner()
                                .with_size(size),
                            ),
                    ),
            )
            .child(
                section("alert-custom-icon", "Custom icon")
                    .description("Custom icon and long content.")
                    .w_2_3()
                    .child(
                        Alert::new(
                            "alert-custom-icon",
                            "The quarterly planning review overlaps with the APAC operations call. \
                            Move one event or invite another owner before sending the agenda.",
                        )
                        .title("Two events overlap by 30 minutes")
                        .with_size(size)
                        .icon(IconName::Calendar),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "alert",
        "Alert",
        "Communicate important status changes without interrupting the workflow.",
        AlertSection::view(window, cx),
    ));
}
