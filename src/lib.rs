use std::{marker::PhantomData, ptr};

use rustc_hash::{FxHashMap, FxHashSet};

mod config;

pub use config::*;

pub struct QuickMatch<'a> {
    config: QuickMatchConfig,
    max_word_count: usize,
    max_word_len: usize,
    max_query_len: usize,
    word_index: FxHashMap<String, FxHashSet<*const str>>,
    trigram_index: FxHashMap<[char; 3], FxHashSet<*const str>>,
    _phantom: PhantomData<&'a str>,
}

unsafe impl<'a> Send for QuickMatch<'a> {}
unsafe impl<'a> Sync for QuickMatch<'a> {}

impl<'a> QuickMatch<'a> {
    /// Expect the items to be pre-formatted (lowercase)
    pub fn new(items: &[&'a str]) -> Self {
        Self::new_with(items, QuickMatchConfig::default())
    }

    /// Expect the items to be pre-formatted (lowercase)
    pub fn new_with(items: &[&'a str], config: QuickMatchConfig) -> Self {
        let mut word_index: FxHashMap<String, FxHashSet<*const str>> = FxHashMap::default();
        let mut trigram_index: FxHashMap<[char; 3], FxHashSet<*const str>> = FxHashMap::default();
        let mut max_word_len = 0;
        let mut max_query_len = 0;
        let mut max_words = 0;
        let separators = config.separators();

        for &item in items {
            max_query_len = max_query_len.max(item.len());
            let mut word_count = 0;
            for word in item.split(separators) {
                word_count += 1;
                if word.is_empty() {
                    continue;
                }

                max_word_len = max_word_len.max(item.len());

                word_index.entry(word.to_string()).or_default().insert(item);

                if word.len() >= 3 {
                    let chars = word.chars().collect::<Vec<_>>();
                    for window in chars.windows(3) {
                        trigram_index
                            .entry(unsafe { ptr::read(window.as_ptr() as *const [char; 3]) })
                            .or_default()
                            .insert(item);
                    }
                }
            }
            max_words = max_words.max(word_count);
        }

        Self {
            max_query_len: max_query_len + 6,
            max_word_len: max_word_len + 4,
            max_word_count: max_word_len + 2,
            word_index,
            trigram_index,
            config,
            _phantom: PhantomData,
        }
    }

    ///
    /// `limit`: max number of returned matches
    ///
    /// `max_trigrams`: max number of processed trigrams in unknown words (0-10 recommended)
    ///
    pub fn matches(&self, query: &str) -> Vec<&'a str> {
        self.matches_with(query, &self.config)
    }

    ///
    /// `limit`: max number of returned matches
    ///
    /// `max_trigrams`: max number of processed trigrams in unknown words (0-10 recommended)
    ///
    pub fn matches_with(&self, query: &str, config: &QuickMatchConfig) -> Vec<&'a str> {
        let limit = config.limit();
        let trigram_budget = config.trigram_budget();
        let query_len = query.len();

        if query.is_empty() || query_len > self.max_query_len {
            return vec![];
        }

        let query = query
            .trim()
            .chars()
            .filter(|c| c.is_ascii())
            .collect::<String>()
            .to_ascii_lowercase();

        let words = query
            .split(config.separators())
            .filter(|w| !w.is_empty() && w.len() <= self.max_word_len)
            .collect::<FxHashSet<_>>();

        if words.is_empty() || words.len() > self.max_word_count {
            return vec![];
        }

        let min_len = query_len.saturating_sub(3);

        let mut pool: Option<FxHashSet<*const str>> = None;
        let mut unknown_words = Vec::new();

        let mut words_to_intersect = vec![];
        for word in words {
            if let Some(items) = self.word_index.get(word) {
                words_to_intersect.push(items)
            } else if word.len() >= 3 && unknown_words.len() < trigram_budget {
                unknown_words.push(word.chars().collect::<Vec<_>>())
            }
        }

        if !words_to_intersect.is_empty() {
            words_to_intersect.sort_unstable_by_key(|set| -(set.len() as i64));

            let mut intersect = words_to_intersect.pop().cloned().unwrap();

            for other_set in words_to_intersect.iter().rev() {
                intersect.retain(|ptr| other_set.contains(ptr));
                if intersect.is_empty() {
                    break;
                }
            }

            pool = Some(intersect);
        }
        let some_pool = pool.is_some();

        if unknown_words.is_empty() || trigram_budget == 0 {
            let mut results: Vec<_> = pool
                .unwrap_or_default()
                .into_iter()
                .map(|item| unsafe { &*item as &str })
                .collect();

            if results.len() > limit {
                results.select_nth_unstable_by_key(limit, |item| item.len());
                results.truncate(limit);
            }

            results.sort_unstable_by_key(|item| item.len());

            return results;
        }

        let mut scores: FxHashMap<*const str, usize> = FxHashMap::default();
        scores.reserve(256);
        if let Some(pool) = &pool {
            for &item in pool {
                scores.insert(item, 1);
            }
        }

        let mut budget = trigram_budget;
        let mut hit_count: usize = 0;
        let mut visited: FxHashSet<[char; 3]> = FxHashSet::default();

        'outer: for round in 0..trigram_budget {
            for chars in &unknown_words {
                if budget == 0 {
                    break 'outer;
                }

                let len = chars.len();
                let max_pos = len - 3;

                let pos = if round == 0 {
                    0
                } else if round == 1 && max_pos > 0 {
                    max_pos
                } else if round == 2 && max_pos > 1 {
                    max_pos / 2
                } else if max_pos > 2 {
                    let mid = max_pos / 2;
                    let offset = (round - 2) >> 1;
                    let p = if (round & 1) == 1 {
                        mid.saturating_sub(offset)
                    } else {
                        mid + offset
                    };

                    if p == 0 || p >= max_pos || p == mid {
                        continue;
                    }
                    p
                } else {
                    continue;
                };

                let trigram = [chars[pos], chars[pos + 1], chars[pos + 2]];

                if !visited.insert(trigram) {
                    continue;
                }

                budget -= 1;

                let Some(items) = self.trigram_index.get(&trigram) else {
                    continue;
                };

                hit_count += 1;

                if some_pool {
                    for &item in items {
                        if let Some(score) = scores.get_mut(&item) {
                            *score += 1;
                        }
                    }
                } else {
                    for &item in items {
                        let len = unsafe { &*item }.len();
                        if len >= min_len {
                            *scores.entry(item).or_default() += 1;
                        }
                    }
                }
            }
        }

        let min_score = hit_count.div_ceil(2).max(1);
        let mut results: Vec<_> = scores
            .into_iter()
            .filter(|(_, s)| *s >= min_score)
            .map(|(item, score)| (unsafe { &*item as &str }, score))
            .collect();

        if results.len() > limit {
            results.select_nth_unstable_by(limit, |a, b| {
                b.1.cmp(&a.1).then_with(|| a.0.len().cmp(&b.0.len()))
            });
            results.truncate(limit);
        }

        results.sort_unstable_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.len().cmp(&b.0.len())));

        results
            .into_iter()
            .take(limit)
            .map(|(item, _)| item)
            .collect()
    }
}
