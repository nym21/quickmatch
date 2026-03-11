const DEFAULT_SEPARATORS = "_- :/";
const DEFAULT_TRIGRAM_BUDGET = 6;
const DEFAULT_LIMIT = 100;

/**
 * Configuration for QuickMatch.
 */
export class QuickMatchConfig {
  /** @type {string} Characters used to split items into words */
  separators = DEFAULT_SEPARATORS;

  /** @type {number} Maximum number of results to return */
  limit = DEFAULT_LIMIT;

  /** @type {number} Number of trigram lookups for fuzzy matching (0-20) */
  trigramBudget = DEFAULT_TRIGRAM_BUDGET;

  /** @param {number} n */
  withLimit(n) {
    this.limit = Math.max(1, n);
    return this;
  }

  /** @param {number} n - Budget (0-20, default: 6) */
  withTrigramBudget(n) {
    this.trigramBudget = Math.max(0, Math.min(20, n));
    return this;
  }

  /** @param {string} s - Separator characters (default: '_- ') */
  withSeparators(s) {
    this.separators = s;
    return this;
  }
}

/**
 * Fast fuzzy string matcher using word and trigram indexing.
 */
export class QuickMatch {
  /**
   * @param {string[]} items - Items to index (should be lowercase)
   * @param {QuickMatchConfig} [config]
   */
  constructor(items, config = new QuickMatchConfig()) {
    this.config = config;
    this.items = items;
    /** @type {Map<string, number[]>} */
    this.wordIndex = new Map();
    /** @type {Map<string, number[]>} */
    this.trigramIndex = new Map();
    this._sepLookup = sepLookup(config.separators);
    this._scores = new Uint32Array(items.length);
    /** @type {number[]} */
    this._dirty = [];

    let maxWordLen = 0;
    let maxQueryLen = 0;
    let maxWords = 0;
    const sep = this._sepLookup;

    for (let idx = 0; idx < items.length; idx++) {
      const item = items[idx];
      if (item.length > maxQueryLen) maxQueryLen = item.length;

      const words = [];
      let start = 0;

      for (let i = 0; i <= item.length; i++) {
        if (i < item.length && !sep[item.charCodeAt(i)]) continue;
        if (i > start) {
          const word = item.slice(start, i);
          words.push(word);
          if (word.length > maxWordLen) maxWordLen = word.length;
          addToIndex(this.wordIndex, word, idx);
          indexTrigrams(this.trigramIndex, word, idx);
        }
        start = i + 1;
      }

      for (let i = 0; i < words.length - 1; i++) {
        addToIndex(this.wordIndex, words[i] + words[i + 1], idx);
      }

      if (words.length > maxWords) maxWords = words.length;
    }

    this.maxWordLen = maxWordLen + 4;
    this.maxQueryLen = maxQueryLen + 6;
    this.maxWords = maxWords + 2;
  }

  /** @param {string} query */
  matches(query) {
    return this.matchesWith(query, this.config);
  }

  /**
   * @param {string} query
   * @param {QuickMatchConfig} config
   */
  matchesWith(query, config) {
    const { limit, trigramBudget } = config;
    const sep =
      config.separators === this.config.separators
        ? this._sepLookup
        : sepLookup(config.separators);

    const q = normalize(query);
    if (!q || q.length > this.maxQueryLen) return [];

    const qwords = splitWords(q, sep, this.maxWordLen);
    if (!qwords.length || qwords.length > this.maxWords) return [];

    const known = [];
    const unknown = [];

    for (const w of qwords) {
      const hits = this.wordIndex.get(w);
      if (hits) known.push(hits);
      else if (w.length >= 3 && unknown.length < trigramBudget) unknown.push(w);
    }

    const pool = intersect(known);
    const hasPool = pool.length > 0;

    if (!unknown.length || !trigramBudget) {
      if (!hasPool) return [];
      return this._rank(pool, null, qwords, sep, limit);
    }

    // Seed scores from exact-match pool
    const { _scores: scores, _dirty: dirty } = this;
    if (hasPool) {
      for (const i of pool) {
        scores[i] = 1;
        dirty.push(i);
      }
    }

    const hitCount = this._scoreTrigrams(
      unknown,
      trigramBudget,
      hasPool,
      Math.max(0, q.length - 3),
    );
    const minScore = Math.max(1, Math.ceil(hitCount / 2));
    const result = this._rank(dirty, minScore, qwords, sep, limit);

    for (const i of dirty) scores[i] = 0;
    dirty.length = 0;
    return result;
  }

  /**
   * @private
   * @param {string[]} unknown
   * @param {number} budget
   * @param {boolean} poolOnly
   * @param {number} minLen
   */
  _scoreTrigrams(unknown, budget, poolOnly, minLen) {
    const visited = new Set();
    const { _scores: scores, _dirty: dirty, items } = this;
    let remaining = budget;
    let hits = 0;

    outer: for (let round = 0; round < budget; round++) {
      for (const word of unknown) {
        if (remaining <= 0) break outer;

        const pos = trigramPosition(word.length, round);
        if (pos < 0) continue;

        const tri = word[pos] + word[pos + 1] + word[pos + 2];
        if (visited.has(tri)) continue;
        visited.add(tri);
        remaining--;

        const matched = this.trigramIndex.get(tri);
        if (!matched) continue;
        hits++;

        if (poolOnly) {
          for (let j = 0; j < matched.length; j++) {
            const i = matched[j];
            if (scores[i] > 0) scores[i]++;
          }
        } else {
          for (let j = 0; j < matched.length; j++) {
            const i = matched[j];
            if (items[i].length >= minLen) {
              if (scores[i] === 0) dirty.push(i);
              scores[i]++;
            }
          }
        }
      }
    }

    return hits;
  }

  /**
   * Rank candidates by prefix match, then score, then length.
   * @private
   * @param {number[]} indices
   * @param {number|null} minScore - null = no score filtering (exact-match path)
   * @param {string[]} qwords
   * @param {Uint8Array} sep
   * @param {number} limit
   */
  _rank(indices, minScore, qwords, sep, limit) {
    const { items, _scores: scores } = this;
    const results = [];

    for (let i = 0; i < indices.length; i++) {
      const idx = indices[i];
      if (minScore !== null && scores[idx] < minScore) continue;
      results.push(idx);
    }

    const pscores = new Uint8Array(items.length);
    for (let i = 0; i < results.length; i++) {
      pscores[results[i]] = prefixScore(items[results[i]], qwords, sep);
    }

    results.sort(
      (a, b) =>
        pscores[b] - pscores[a] ||
        scores[b] - scores[a] ||
        items[a].length - items[b].length,
    );

    if (results.length > limit) results.length = limit;
    return results.map((i) => items[i]);
  }
}

// --- Helpers ---

/** @param {string} query */
function normalize(query) {
  let out = "";
  let start = 0;
  let end = query.length;
  while (start < end && query.charCodeAt(start) <= 32) start++;
  while (end > start && query.charCodeAt(end - 1) <= 32) end--;
  for (let i = start; i < end; i++) {
    const c = query.charCodeAt(i);
    if (c >= 128) continue;
    out += c >= 65 && c <= 90 ? String.fromCharCode(c + 32) : query[i];
  }
  return out;
}

/** @param {string} separators */
function sepLookup(separators) {
  const t = new Uint8Array(128);
  for (let i = 0; i < separators.length; i++) {
    const c = separators.charCodeAt(i);
    if (c < 128) t[c] = 1;
  }
  return t;
}

/**
 * @param {string} text
 * @param {Uint8Array} sep
 * @param {number} maxLen
 */
function splitWords(text, sep, maxLen) {
  /** @type {string[]} */
  const words = [];
  let start = 0;
  for (let i = 0; i <= text.length; i++) {
    if (i < text.length && !sep[text.charCodeAt(i)]) continue;
    if (i > start) {
      const w = text.slice(start, i);
      if (w.length <= maxLen && !words.includes(w)) words.push(w);
    }
    start = i + 1;
  }
  return words;
}

/**
 * @param {Map<string, number[]>} index
 * @param {string} key
 * @param {number} value
 */
function addToIndex(index, key, value) {
  const arr = index.get(key);
  if (arr) arr.push(value);
  else index.set(key, [value]);
}

/**
 * @param {Map<string, number[]>} index
 * @param {string} word
 * @param {number} idx
 */
function indexTrigrams(index, word, idx) {
  if (word.length < 3) return;
  for (let i = 0; i <= word.length - 3; i++) {
    const tri = word[i] + word[i + 1] + word[i + 2];
    const arr = index.get(tri);
    if (!arr) index.set(tri, [idx]);
    else if (arr[arr.length - 1] !== idx) arr.push(idx);
  }
}

/** @param {number[][]} arrays */
function intersect(arrays) {
  if (!arrays.length) return [];

  let si = 0;
  for (let i = 1; i < arrays.length; i++) {
    if (arrays[i].length < arrays[si].length) si = i;
  }

  const result = arrays[si].slice();
  for (let i = 0; i < arrays.length && result.length > 0; i++) {
    if (i === si) continue;
    let w = 0;
    for (let j = 0; j < result.length; j++) {
      if (bsearch(arrays[i], result[j])) result[w++] = result[j];
    }
    result.length = w;
  }
  return result;
}

/**
 * @param {number[]} arr
 * @param {number} val
 */
function bsearch(arr, val) {
  let lo = 0,
    hi = arr.length - 1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    if (arr[mid] === val) return true;
    if (arr[mid] < val) lo = mid + 1;
    else hi = mid - 1;
  }
  return false;
}

/**
 * 2 = exact match, 1 = prefix match, 0 = no match
 * @param {string} item
 * @param {string[]} qwords
 * @param {Uint8Array} sep
 */
function prefixScore(item, qwords, sep) {
  let qi = 0,
    pos = 0;
  const len = item.length;

  while (qi < qwords.length) {
    while (pos < len && sep[item.charCodeAt(pos)]) pos++;
    if (pos >= len) return 0;

    const ws = pos;
    while (pos < len && !sep[item.charCodeAt(pos)]) pos++;

    const qw = qwords[qi];
    if (pos - ws !== qw.length) return 0;
    for (let j = 0; j < qw.length; j++) {
      if (item.charCodeAt(ws + j) !== qw.charCodeAt(j)) return 0;
    }
    qi++;
  }

  while (pos < len && sep[item.charCodeAt(pos)]) pos++;
  return pos >= len ? 2 : 1;
}

/** @param {number} len @param {number} round */
function trigramPosition(len, round) {
  const max = len - 3;
  if (max < 0) return -1;
  if (round === 0) return 0;
  if (round === 1 && max > 0) return max;
  if (round === 2 && max > 1) return max >> 1;
  if (max <= 2) return -1;

  const mid = max >> 1;
  const off = (round - 2) >> 1;
  const pos = round & 1 ? Math.max(0, mid - off) : mid + off;
  return pos === 0 || pos >= max || pos === mid ? -1 : pos;
}
