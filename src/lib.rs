use std::{marker::PhantomData, ptr};

use rustc_hash::{FxHashMap, FxHashSet};

mod config;

pub use config::*;

/// Instant search over a list of strings.
///
/// Supports exact words, prefixes ("dom" → "dominance"), joined words
/// ("hashrate" → "hash_rate"), and typo tolerance ("suply" → "supply").
/// Results are ranked: exact matches first, then by specificity.
pub struct QuickMatch<'a> {
    config: QuickMatchConfig,
    max_word_count: usize,
    max_word_len: usize,
    max_query_len: usize,
    word_index: FxHashMap<String, FxHashSet<*const str>>,
    trigram_index: FxHashMap<[char; 3], FxHashSet<*const str>>,
    _phantom: PhantomData<&'a str>,
}

unsafe impl Send for QuickMatch<'_> {}
unsafe impl Sync for QuickMatch<'_> {}

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
            let item_words: Vec<&str> = item.split(separators).filter(|w| !w.is_empty()).collect();
            max_words = max_words.max(item_words.len());

            for word in &item_words {
                max_word_len = max_word_len.max(word.len());

                for len in 1..=word.len() {
                    word_index
                        .entry(word[..len].to_string())
                        .or_default()
                        .insert(item);
                }

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

            for pair in item_words.windows(2) {
                let compound = format!("{}{}", pair[0], pair[1]);
                let from = pair[0].len() + 1;
                for len in from..=compound.len() {
                    word_index
                        .entry(compound[..len].to_string())
                        .or_default()
                        .insert(item);
                }
            }
        }

        Self {
            max_query_len: max_query_len + 6,
            max_word_len: max_word_len + 4,
            max_word_count: max_words + 2,
            word_index,
            trigram_index,
            config,
            _phantom: PhantomData,
        }
    }

    pub fn matches(&self, query: &str) -> Vec<&'a str> {
        self.matches_with(query, &self.config)
    }

    pub fn matches_with(&self, query: &str, config: &QuickMatchConfig) -> Vec<&'a str> {
        let limit = config.limit();
        let trigram_budget = config.trigram_budget();

        if query.is_empty() {
            return vec![];
        }

        let query = query
            .trim()
            .chars()
            .filter(|c| c.is_ascii())
            .collect::<String>()
            .to_ascii_lowercase();

        if query.is_empty() || query.len() > self.max_query_len {
            return vec![];
        }

        let separators = config.separators();

        let mut seen = FxHashSet::default();
        let query_words: Vec<&str> = query
            .split(separators)
            .filter(|w| !w.is_empty() && w.len() <= self.max_word_len)
            .filter(|w| seen.insert(*w))
            .collect();
        drop(seen);

        if query_words.is_empty() || query_words.len() > self.max_word_count {
            return vec![];
        }

        let min_len = query.len().saturating_sub(3);

        let mut unknown_words = Vec::new();
        let mut known_sets: Vec<&FxHashSet<*const str>> = vec![];

        for &word in &query_words {
            if let Some(items) = self.word_index.get(word) {
                known_sets.push(items)
            } else if word.len() >= 3 && unknown_words.len() < trigram_budget {
                unknown_words.push(word.chars().collect::<Vec<_>>())
            }
        }

        let pool = Self::intersect_sets(&known_sets);

        // Try typo matching for unknown words
        if !unknown_words.is_empty() && trigram_budget > 0 {
            let has_pool = pool.is_some();
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

                    if has_pool {
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

            let min_score = hit_count.div_ceil(2).max(config.min_score());
            let results = Self::rank(
                scores
                    .into_iter()
                    .filter(|(_, s)| *s >= min_score),
                &query_words,
                separators,
                limit,
            );

            if !results.is_empty() {
                return results;
            }
        }

        // Rank known candidates (intersection, or union as fallback)
        let candidates = pool.unwrap_or_else(|| Self::union_sets(&known_sets));
        Self::rank(
            candidates.into_iter().map(|p| (p, 0)),
            &query_words,
            separators,
            limit,
        )
    }

    fn intersect_sets(sets: &[&FxHashSet<*const str>]) -> Option<FxHashSet<*const str>> {
        if sets.is_empty() {
            return None;
        }

        let mut sorted: Vec<_> = sets.to_vec();
        sorted.sort_unstable_by_key(|s| s.len());

        let mut result = (*sorted[0]).clone();

        for set in &sorted[1..] {
            result.retain(|ptr| set.contains(ptr));
            if result.is_empty() {
                return None;
            }
        }

        Some(result)
    }

    fn union_sets(sets: &[&FxHashSet<*const str>]) -> FxHashSet<*const str> {
        let mut result = FxHashSet::default();
        for set in sets {
            result.extend(set.iter());
        }
        result
    }

    /// Bucket by prefix score, sort only needed buckets by score then length.
    fn rank(
        candidates: impl IntoIterator<Item = (*const str, usize)>,
        query_words: &[&str],
        separators: &[char],
        limit: usize,
    ) -> Vec<&'a str> {
        let mut buckets: [Vec<(&str, usize)>; 3] = [vec![], vec![], vec![]];

        for (item, score) in candidates {
            let s = unsafe { &*item as &'a str };
            let ps = prefix_score(s, query_words, separators);
            buckets[ps as usize].push((s, score));
        }

        let mut results = Vec::with_capacity(limit);
        for ps in (0..3).rev() {
            let bucket = &mut buckets[ps];
            if bucket.is_empty() {
                continue;
            }
            bucket.sort_unstable_by(|a, b| {
                b.1.cmp(&a.1).then_with(|| a.0.len().cmp(&b.0.len()))
            });
            let take = (limit - results.len()).min(bucket.len());
            results.extend(bucket[..take].iter().map(|(s, _)| *s));
            if results.len() >= limit {
                break;
            }
        }

        results
    }
}

fn prefix_score(item: &str, query_words: &[&str], separators: &[char]) -> u8 {
    let mut item_words = item.split(separators).filter(|w| !w.is_empty());
    for &qw in query_words {
        match item_words.next() {
            Some(iw) if iw.starts_with(qw) => continue,
            _ => return 0,
        }
    }
    if item_words.next().is_none() {
        2
    } else {
        1
    }
}
