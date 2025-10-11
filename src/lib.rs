use std::{marker::PhantomData, ptr};

use rustc_hash::{FxHashMap, FxHashSet};

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

        for &item in items {
            max_query_len = max_query_len.max(item.len());
            let mut word_count = 0;
            for word in item.split(config.separators) {
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
        let limit = config.limit;
        let trigram_budget = config.trigram_budget;
        let query_len = query.len();

        if limit == 0 || query.is_empty() || query_len > self.max_query_len {
            return vec![];
        }

        let query = query
            .trim()
            .chars()
            .filter(|c| c.is_ascii())
            .collect::<String>()
            .to_ascii_lowercase();
        let words: FxHashSet<&str> = query
            .split(config.separators)
            .filter(|w| !w.is_empty() && w.len() <= self.max_word_len)
            .collect();

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

        if unknown_words.is_empty() {
            if !some_pool {
                return vec![];
            }

            let mut results: Vec<_> = pool
                .unwrap()
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

        let mut trigram_count = 0;
        let mut visited: FxHashSet<[char; 3]> = FxHashSet::default();

        'outer: for round in 0..trigram_budget {
            let mut processed_trigrams = false;

            for chars in &unknown_words {
                if trigram_count >= trigram_budget {
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
                    // Alternate left and right of middle
                    let mid = max_pos / 2;
                    let offset = (round - 2) >> 1; // Faster than / 2
                    let p = if (round & 1) == 1 {
                        // Faster than (r - 3) % 2 == 0
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

                let Some(items) = self.trigram_index.get(&trigram) else {
                    continue;
                };

                processed_trigrams = true;
                trigram_count += 1;

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

            if !processed_trigrams {
                break 'outer;
            }
        }

        let min_score = trigram_count.div_ceil(2).max(1);
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

const DEFAULT_SEPARATORS: &[char] = &['_', '-', ' '];
const DEFAULT_TRIGRAM_BUDGET: usize = 6;
const DEFAULT_LIMIT: usize = 100;

pub struct QuickMatchConfig {
    /// Separators used to split words.
    ///
    /// Default: ['_', '-', ' ']
    separators: &'static [char],
    /// Maximum number of results to return.
    ///
    /// Default: 100
    /// - Min: 1
    /// - Max: No hard limit (but large values may impact performance)
    limit: usize,
    /// Budget of trigrams to process from unknown words.
    /// This budget is distributed fairly across all unknown words.
    ///
    /// Default: 6 (recommended: 3-9)
    /// - 0: Disable trigram matching (only exact word matches)
    /// - Low (3-6): Faster, less accurate fuzzy matching
    /// - High (9-15): Slower, more accurate fuzzy matching
    /// - Max: 20
    trigram_budget: usize,
}

impl Default for QuickMatchConfig {
    fn default() -> Self {
        Self {
            separators: DEFAULT_SEPARATORS,
            limit: DEFAULT_LIMIT,
            trigram_budget: DEFAULT_TRIGRAM_BUDGET,
        }
    }
}

impl QuickMatchConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit.max(1);
        self
    }

    pub fn with_trigram_budget(mut self, trigram_budget: usize) -> Self {
        self.trigram_budget = trigram_budget.clamp(0, 20);
        self
    }

    pub fn with_separators(mut self, separators: &'static [char]) -> Self {
        self.separators = separators;
        self
    }

    pub fn limit(&self) -> usize {
        self.limit
    }

    pub fn trigram_budget(&self) -> usize {
        self.trigram_budget
    }

    pub fn separators(&self) -> &[char] {
        self.separators
    }
}
