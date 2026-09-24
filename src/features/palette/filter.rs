//! Fuzzy scoring and filtering for palette items, wrapping `SkimMatcherV2`.

use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

use crate::features::palette::items::PaletteEntry;

/// A scored filter hit: the index into the original item slice plus its score.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FilteredItem {
    index: usize,
    score: i64,
}

#[derive(Clone, Debug)]
pub struct FuzzyMatchConfig {
    pub exact_match_bonus: i64,
    pub prefix_match_bonus: i64,
    /// Bonus added when the query matches the start of any whitespace-delimited
    /// word in the name.
    pub word_prefix_bonus: i64,
    /// Maximum bonus for contiguous (adjacent) matched characters; scaled by the
    /// observed contiguity ratio.
    pub contiguity_bonus: i64,
    /// Multiplier applied to a match found only in the description (0.0–1.0
    /// demotes description-only hits below name hits).
    pub description_penalty: f64,
}

impl Default for FuzzyMatchConfig {
    fn default() -> Self {
        Self {
            exact_match_bonus: 1000,
            prefix_match_bonus: 500,
            word_prefix_bonus: 250,
            contiguity_bonus: 100,
            description_penalty: 0.6,
        }
    }
}

pub struct ItemFilter {
    matcher: SkimMatcherV2,
    config: FuzzyMatchConfig,
}

impl Default for ItemFilter {
    fn default() -> Self {
        Self::new(FuzzyMatchConfig::default())
    }
}

impl ItemFilter {
    pub fn new(config: FuzzyMatchConfig) -> Self {
        Self {
            // Skim is smart-case by default; callers hand over the raw
            // query, and uppercase input must stay case-insensitive.
            matcher: SkimMatcherV2::default().ignore_case(),
            config,
        }
    }

    /// Return just the matching indices, sorted by descending score.
    pub fn filter_indices<E: PaletteEntry>(&self, items: &[E], query: &str) -> Vec<usize> {
        self.filter_with_scores(items, query)
            .into_iter()
            .map(|f| f.index)
            .collect()
    }

    /// Score and sort all items against `query`; empty query returns every item
    /// in original order, otherwise matching items by descending score.
    fn filter_with_scores<E: PaletteEntry>(&self, items: &[E], query: &str) -> Vec<FilteredItem> {
        if query.is_empty() {
            return (0..items.len())
                .map(|index| FilteredItem { index, score: 0 })
                .collect();
        }

        // Lowercase the query once per pass instead of once per item.
        let query_lower = query.to_lowercase();

        let mut scored: Vec<FilteredItem> = items
            .iter()
            .enumerate()
            .filter_map(|(idx, item)| {
                let score = self.score_entry(item, query, &query_lower)?;
                Some(FilteredItem { index: idx, score })
            })
            .collect();

        scored.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.index.cmp(&b.index)));
        scored
    }

    /// Score a single entry, preferring name matches and falling back to the
    /// description with a penalty. `query_lower` is pre-lowercased by the caller.
    fn score_entry<E: PaletteEntry>(
        &self,
        item: &E,
        query: &str,
        query_lower: &str,
    ) -> Option<i64> {
        let name = item.name();
        let name_lower = name.to_lowercase();

        if let Some(score) = self.score_text(name, &name_lower, query, query_lower, false) {
            return Some(score);
        }

        let desc = item.description()?;
        let desc_lower = desc.to_lowercase();
        self.score_text(desc, &desc_lower, query, query_lower, true)
    }

    /// `text_lower`/`query_lower` are pre-lowercased by the
    /// caller; returns `None` when the matcher reports no hit.
    fn score_text(
        &self,
        text: &str,
        text_lower: &str,
        query: &str,
        query_lower: &str,
        is_description: bool,
    ) -> Option<i64> {
        let matched = self.matcher.fuzzy_indices(text, query).or_else(|| {
            // Normalize whitespace: "foo bar" -> "foobar" and "foo-bar", which
            // lets "counter strike" match "Counter-Strike".
            if !query.contains(' ') {
                return None;
            }
            let no_spaces: String = query.chars().filter(|c| *c != ' ').collect();
            if let Some(hit) = self.matcher.fuzzy_indices(text, &no_spaces) {
                return Some(hit);
            }
            let with_hyphens = query.replace(' ', "-");
            self.matcher.fuzzy_indices(text, &with_hyphens)
        });

        let (mut score, indices) = matched?;

        if !is_description {
            if text_lower == query_lower {
                score += self.config.exact_match_bonus;
            } else if text_lower.starts_with(query_lower) {
                score += self.config.prefix_match_bonus;
            } else if Self::matches_word_start(text_lower, query_lower) {
                score += self.config.word_prefix_bonus;
            }
        }

        score += self.contiguity_bonus(&indices);

        if is_description {
            score = (score as f64 * self.config.description_penalty) as i64;
        }

        Some(score)
    }

    fn contiguity_bonus(&self, indices: &[usize]) -> i64 {
        if indices.len() <= 1 {
            return self.config.contiguity_bonus;
        }
        let adjacent = indices.windows(2).filter(|w| w[1] == w[0] + 1).count();
        let ratio = adjacent as f64 / (indices.len() - 1) as f64;
        (ratio * self.config.contiguity_bonus as f64) as i64
    }

    /// Both arguments must already be lowercased.
    fn matches_word_start(text_lower: &str, query_lower: &str) -> bool {
        text_lower
            .split_whitespace()
            .any(|word| word.starts_with(query_lower))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::palette::items::PaletteEntry;

    struct Entry {
        name: &'static str,
        desc: Option<&'static str>,
    }
    impl PaletteEntry for Entry {
        fn name(&self) -> &str {
            self.name
        }
        fn description(&self) -> Option<&str> {
            self.desc
        }
    }

    #[test]
    fn empty_query_returns_all() {
        let f = ItemFilter::default();
        let items = [
            Entry {
                name: "Firefox",
                desc: None,
            },
            Entry {
                name: "Chrome",
                desc: None,
            },
        ];
        let r = f.filter_indices(&items, "");
        assert_eq!(r, vec![0, 1]);
    }

    #[test]
    fn filters_by_name_substring() {
        let f = ItemFilter::default();
        let items = [
            Entry {
                name: "Firefox",
                desc: None,
            },
            Entry {
                name: "Chrome",
                desc: None,
            },
        ];
        let r = f.filter_indices(&items, "fire");
        assert_eq!(r, vec![0]);
    }

    #[test]
    fn exact_name_beats_description_only_match() {
        let f = ItemFilter::default();
        let items = [
            Entry {
                name: "Browser",
                desc: None,
            },
            Entry {
                name: "Editor",
                desc: Some("a browser for files"),
            },
        ];
        let scored = f.filter_with_scores(&items, "browser");
        assert_eq!(scored[0].index, 0);
        assert!(scored[0].score > scored[1].score);
    }

    #[test]
    fn prefix_beats_infix() {
        let f = ItemFilter::default();
        let items = [
            Entry {
                name: "Firefox",
                desc: None,
            },
            Entry {
                name: "Waterfox",
                desc: None,
            },
        ];
        let r = f.filter_indices(&items, "fire");
        assert_eq!(r[0], 0);
    }

    #[test]
    fn space_query_matches_hyphenated_name() {
        let f = ItemFilter::default();
        let items = [Entry {
            name: "Counter-Strike",
            desc: None,
        }];
        let r = f.filter_indices(&items, "counter strike");
        assert_eq!(r, vec![0]);
    }

    #[test]
    fn no_match_returns_empty() {
        let f = ItemFilter::default();
        let items = [Entry {
            name: "Firefox",
            desc: None,
        }];
        let r = f.filter_indices(&items, "zzz");
        assert!(r.is_empty());
    }

    #[test]
    fn uppercase_query_matches_lowercase_items() {
        // Regression: skim is smart-case, so without ignore_case the raw
        // query "Set" would turn matching case-sensitive and miss
        // all-lowercase names entirely.
        let f = ItemFilter::default();
        let items = [
            Entry {
                name: "settings",
                desc: None,
            },
            Entry {
                name: "notifications",
                desc: None,
            },
        ];
        let r = f.filter_indices(&items, "Set");
        assert_eq!(r, vec![0]);
    }
}
