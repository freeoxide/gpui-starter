use gpui_kit::SharedString;
use gpui_kit::component::Size;
use serde::Deserialize;

use super::format::{compact, money};

#[derive(Clone, Deserialize)]
pub(crate) struct DailyRow {
    pub(crate) date: SharedString,
    pub(crate) organic: f64,
    pub(crate) paid: f64,
    pub(crate) social: f64,
    pub(crate) direct: f64,
    pub(crate) sessions: f64,
    pub(crate) revenue: f64,
}

#[derive(Clone, PartialEq, Deserialize)]
pub(crate) struct ChannelRow {
    pub(crate) page: SharedString,
    pub(crate) channel: SharedString,
    pub(crate) visitors: f64,
    pub(crate) sessions: f64,
    pub(crate) revenue: f64,
    pub(crate) conversion: f64,
    pub(crate) avg_session_secs: f64,
}

/// Sessions summed over every channel of one page: the unit a per-page chart
/// bands on, where a raw row would collide with its sibling channels.
#[derive(Clone, Deserialize)]
pub(crate) struct PageSessions {
    pub(crate) page: SharedString,
    pub(crate) sessions: f64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Segment {
    All,
    Organic,
    Paid,
    Social,
    Direct,
}

impl Segment {
    pub(crate) const ALL: [Segment; 5] = [
        Segment::All,
        Segment::Organic,
        Segment::Paid,
        Segment::Social,
        Segment::Direct,
    ];

    pub(crate) fn from_index(ix: usize) -> Self {
        match ix {
            1 => Segment::Organic,
            2 => Segment::Paid,
            3 => Segment::Social,
            4 => Segment::Direct,
            _ => Segment::All,
        }
    }

    pub(crate) fn label_key(self) -> &'static str {
        match self {
            Segment::All => "analytics_segment_all",
            Segment::Organic => "analytics_segment_organic",
            Segment::Paid => "analytics_segment_paid",
            Segment::Social => "analytics_segment_social",
            Segment::Direct => "analytics_segment_direct",
        }
    }

    pub(crate) fn matches(self, channel: &str) -> bool {
        match self {
            Segment::All => true,
            Segment::Organic => channel == "organic",
            Segment::Paid => channel == "paid",
            Segment::Social => channel == "social",
            Segment::Direct => channel == "direct",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum DateRange {
    Last7,
    Last30,
    Last90,
}

impl DateRange {
    pub(crate) const ALL: [DateRange; 3] = [DateRange::Last7, DateRange::Last30, DateRange::Last90];

    pub(crate) fn from_index(ix: usize) -> Self {
        match ix {
            0 => DateRange::Last7,
            2 => DateRange::Last90,
            _ => DateRange::Last30,
        }
    }

    pub(crate) fn days(self) -> usize {
        match self {
            DateRange::Last7 => 7,
            DateRange::Last30 => 30,
            DateRange::Last90 => 90,
        }
    }

    pub(crate) fn label_key(self) -> &'static str {
        match self {
            DateRange::Last7 => "analytics_range_7",
            DateRange::Last30 => "analytics_range_30",
            DateRange::Last90 => "analytics_range_90",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Density {
    Compact,
    Comfortable,
}

impl Density {
    pub(crate) fn size(self) -> Size {
        match self {
            Density::Compact => Size::Small,
            Density::Comfortable => Size::Medium,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct ChannelKey {
    pub(crate) page: SharedString,
    pub(crate) channel: SharedString,
}

impl ChannelKey {
    pub(crate) fn of(row: &ChannelRow) -> Self {
        Self {
            page: row.page.clone(),
            channel: row.channel.clone(),
        }
    }
}

pub(crate) struct Kpi {
    pub(crate) label_key: &'static str,
    pub(crate) value: String,
    pub(crate) delta: Option<f64>,
}

pub(crate) struct WorkspaceData {
    daily: Vec<DailyRow>,
    channels: Vec<ChannelRow>,
}

impl WorkspaceData {
    pub(crate) fn load() -> Self {
        let daily: Vec<DailyRow> = serde_json::from_str(include_str!("fixtures/daily-series.json"))
            .expect("daily-series.json fixture must parse");
        let channels: Vec<ChannelRow> =
            serde_json::from_str(include_str!("fixtures/channels.json"))
                .expect("channels.json fixture must parse");
        Self { daily, channels }
    }

    pub(crate) fn window(&self, range: DateRange) -> &[DailyRow] {
        &self.daily[self.daily.len().saturating_sub(range.days())..]
    }

    pub(crate) fn previous_window(&self, range: DateRange) -> &[DailyRow] {
        let days = range.days();
        let end = self.daily.len().saturating_sub(days);
        &self.daily[end.saturating_sub(days)..end]
    }

    pub(crate) fn kpis(&self, range: DateRange) -> [Kpi; 4] {
        let window = self.window(range);
        let previous = self.previous_window(range);
        let has_previous = !previous.is_empty();

        let visitors = |rows: &[DailyRow]| {
            rows.iter()
                .map(|row| row.organic + row.paid + row.social + row.direct)
                .sum::<f64>()
        };
        let sessions = |rows: &[DailyRow]| rows.iter().map(|row| row.sessions).sum::<f64>();
        let revenue = |rows: &[DailyRow]| rows.iter().map(|row| row.revenue).sum::<f64>();

        let delta =
            |latest: f64, previous: f64| has_previous.then(|| change_percent(latest, previous));

        [
            Kpi {
                label_key: "analytics_kpi_visitors",
                value: compact(visitors(window)),
                delta: delta(visitors(window), visitors(previous)),
            },
            Kpi {
                label_key: "analytics_kpi_sessions",
                value: compact(sessions(window)),
                delta: delta(sessions(window), sessions(previous)),
            },
            Kpi {
                label_key: "analytics_kpi_revenue",
                value: money(revenue(window)),
                delta: delta(revenue(window), revenue(previous)),
            },
            Kpi {
                label_key: "analytics_kpi_revenue_per_visitor",
                value: money(revenue(window) / visitors(window).max(1.)),
                delta: delta(
                    revenue(window) / visitors(window).max(1.),
                    revenue(previous) / visitors(previous).max(1.),
                ),
            },
        ]
    }

    pub(crate) fn channels(&self, segment: Segment) -> Vec<ChannelRow> {
        self.channels
            .iter()
            .filter(|row| segment.matches(&row.channel))
            .cloned()
            .collect()
    }

    /// The `count` pages with the most sessions in `segment`, one aggregated
    /// entry per page so every band key is unique.
    pub(crate) fn top_pages(&self, segment: Segment, count: usize) -> Vec<PageSessions> {
        let mut totals: Vec<PageSessions> = Vec::new();
        for row in self.channels(segment) {
            match totals.iter_mut().find(|total| total.page == row.page) {
                Some(total) => total.sessions += row.sessions,
                None => totals.push(PageSessions {
                    page: row.page.clone(),
                    sessions: row.sessions,
                }),
            }
        }
        totals.sort_by(|a, b| b.sessions.total_cmp(&a.sessions));
        totals.truncate(count);
        totals
    }

    pub(crate) fn channel_row(&self, key: &ChannelKey) -> Option<ChannelRow> {
        self.channels
            .iter()
            .find(|row| row.page == key.page && row.channel == key.channel)
            .cloned()
    }

    pub(crate) fn channel_series(&self, channel: &str) -> Vec<(SharedString, f64)> {
        let pick = match channel {
            "organic" => |row: &DailyRow| row.organic,
            "paid" => |row: &DailyRow| row.paid,
            "social" => |row: &DailyRow| row.social,
            "direct" => |row: &DailyRow| row.direct,
            _ => return Vec::new(),
        };
        self.daily
            .iter()
            .map(|row| (row.date.clone(), pick(row)))
            .collect()
    }
}

fn change_percent(latest: f64, previous: f64) -> f64 {
    if previous.abs() < f64::EPSILON {
        0.
    } else {
        (latest - previous) / previous.abs() * 100.
    }
}

#[cfg(test)]
#[path = "model.test.rs"]
mod model_test;
