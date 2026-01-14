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
