use gpui_kit::{Background, Hsla, linear_color_stop, linear_gradient};

pub(crate) fn compact(value: f64) -> String {
    let magnitude = value.abs();
    if magnitude >= 1_000_000. {
        format!("{:.1}M", value / 1_000_000.)
    } else if magnitude >= 10_000. {
        format!("{:.0}K", value / 1_000.)
    } else if magnitude >= 1_000. {
        format!("{:.1}K", value / 1_000.)
    } else {
        format!("{:.0}", value)
    }
}

pub(crate) fn money(value: f64) -> String {
    let sign = if value < 0. { "-" } else { "" };
    let magnitude = value.abs();
    // Small amounts keep their cents: compact would flatten $9.20 to $9.2.
    if magnitude < 1_000. {
        format!("{sign}${magnitude:.2}")
    } else {
        format!("{sign}${}", compact(magnitude))
    }
}

pub(crate) fn percent(value: f64, digits: usize) -> String {
    format!("{:.*}%", digits, value)
}

pub(crate) fn area_gradient(color: Hsla) -> Background {
    linear_gradient(
        0.,
        linear_color_stop(color.opacity(0.45), 1.),
        linear_color_stop(color.opacity(0.), 0.),
    )
}

#[cfg(test)]
#[path = "format.test.rs"]
mod format_test;
