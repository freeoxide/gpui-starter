use gpui_kit::component::{ActiveTheme as _, button::Button, h_flex, v_flex};
use gpui_kit::{prelude::*, *};

use crate::capabilities::CapabilityStatus;
use crate::features::notch;
use crate::gallery::registry::GallerySection;
use crate::ui::floating::{FloatingPlatformInfo, probe};

use super::demo::section;

pub struct NotchSection;

impl NotchSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            cx.observe_global::<notch::controller::NotchFloaterWindow>(|_, cx| cx.notify())
                .detach();
            Self
        })
    }
}

impl Render for NotchSection {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let info = probe(cx);
        let capability = crate::capabilities::snapshot(cx)
            .get(notch::CAPABILITY_NOTCH_FLOATER)
            .cloned();
        let label = if notch::is_open(cx) {
            "Close notch floater"
        } else {
            "Open notch floater"
        };
        v_flex()
            .w_full()
            .items_center()
            .gap_6()
            .p_4()
            .child(
                section("notch-toggle", "Floater")
                    .description("Toggle the notch floater window.")
                    .w(rems(32.5))
                    .child(
                        Button::new("notch-floater-toggle")
                            .label(label)
                            .on_click(cx.listener(|_, _, _, cx| {
                                notch::toggle(cx);
                                cx.notify();
                            })),
                    ),
            )
            .child(
                section("notch-status", "Platform status")
                    .description("Live facts from the floating platform probe.")
                    .w(rems(32.5))
                    .child(probe_line(&info, cx))
                    .child(capability_line(capability.as_ref(), cx)),
            )
    }
}

fn probe_line(info: &FloatingPlatformInfo, cx: &App) -> Div {
    let notch = info.notch.as_ref();
    let safe_top = notch
        .map(|n| n.safe_area.top)
        .or(info.menu_bar)
        .map(|top| top.to_string())
        .unwrap_or_else(|| "n/a".to_string());
    let housing = notch
        .and_then(|n| n.camera_housing)
        .map(|h| format!("{} px", h.size.width))
        .unwrap_or_else(|| "no camera housing".to_string());
    let menu_bar = info
        .menu_bar
        .map(|height| height.to_string())
        .unwrap_or_else(|| "n/a".to_string());
    h_flex()
        .gap_2()
        .flex_wrap()
        .text_sm()
        .text_color(cx.theme().muted_foreground)
        .child(format!(
            "Safe-area top: {safe_top}, housing: {housing}, menu bar: {menu_bar}"
        ))
}

fn capability_line(status: Option<&CapabilityStatus>, cx: &App) -> Div {
    let state = |status: &CapabilityStatus| {
        if status.degraded {
            "degraded"
        } else if status.supported && status.enabled {
            "supported, enabled"
        } else {
            "unavailable"
        }
    };
    let line = match status {
        Some(status) => match &status.reason {
            Some(reason) => format!("notch_floater: {}. {reason}", state(status)),
            None => format!("notch_floater: {}", state(status)),
        },
        None => "notch_floater: not registered".to_string(),
    };
    h_flex()
        .gap_2()
        .flex_wrap()
        .text_sm()
        .text_color(cx.theme().muted_foreground)
        .child(line)
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "notch-floater",
        "Notch Floater",
        "macOS notch-area floater with hover expansion; inset-anchored demo on other platforms.",
        NotchSection::view(window, cx),
    ));
}
