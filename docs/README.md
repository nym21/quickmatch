# quickmatch

Fast fuzzy string matching for Rust and JavaScript.

Built for autocomplete, command palettes, and search-as-you-type interfaces.

[![Crates.io](https://img.shields.io/crates/v/quickmatch.svg)](https://crates.io/crates/quickmatch)
[![npm](https://img.shields.io/npm/v/quickmatch-js.svg)](https://www.npmjs.com/package/quickmatch-js)
[![Documentation](https://docs.rs/quickmatch/badge.svg)](https://docs.rs/quickmatch)

## Install

```bash
# rust
cargo add quickmatch

# js
npm install quickmatch-js
```

## Usage

**Rust**

```rust
use quickmatch::{QuickMatch, QuickMatchConfig};

let items = vec!["file_name", "file_size", "created_at", "updated_at"];
let qm = QuickMatch::new(&items);

qm.matches("file name");  // ["file_name", "file_size"]
qm.matches("filename");   // ["file_name", "file_size"]  (compound match)
qm.matches("filenme");    // ["file_name", "file_size"]  (trigram fuzzy)

// Custom config
let config = QuickMatchConfig::new()
    .with_limit(5)
    .with_trigrams(10)
    .with_separators(&['_', '-', ' ']);
let qm = QuickMatch::new_with(&items, config);
```

**JavaScript**

```js
import { QuickMatch, QuickMatchConfig } from "quickmatch-js";

const items = ["file_name", "file_size", "created_at", "updated_at"];
const qm = new QuickMatch(items);

qm.matches("file name");  // ["file_name", "file_size"]
qm.matches("filename");   // ["file_name", "file_size"]  (compound match)
qm.matches("filenme");    // ["file_name", "file_size"]  (trigram fuzzy)

// Custom config
const config = new QuickMatchConfig()
  .withLimit(5)
  .withTrigramBudget(10)
  .withSeparators("_- ");
const qm2 = new QuickMatch(items, config);
```

## How it works

Queries go through three matching stages:

1. **Word match** — query is split by separators and looked up in a word index
2. **Compound match** — adjacent words are indexed as compounds, so `hashrate` finds `hash_rate`
3. **Trigram fallback** — unknown words are matched via character trigrams for fuzzy/typo tolerance

Results are ranked by prefix score (exact > prefix > unordered), then by trigram score, then by length.

## Config

| Option | Rust | JS | Default | Description |
|--------|------|-----|---------|-------------|
| Limit | `with_limit(n)` | `withLimit(n)` | 100 | Max results returned |
| Trigrams | `with_trigrams(n)` | `withTrigramBudget(n)` | 6 | Trigram budget for fuzzy matching (0-20). Higher = more fuzzy, slower |
| Separators | `with_separators(&[..])` | `withSeparators(s)` | `_`, `-`, ` `, `:`, `/` | Characters that split words |

Setting trigrams to `0` disables fuzzy matching entirely (exact word matches only).

## Performance

Benchmarked against ~5,000 metric names, 83 queries, averaged over 10K iterations:

| | Avg/query | Build time |
|---|-----------|------------|
| **Rust** | ~26 us | ~40 ms |
| **JS** | ~29 us | ~30 ms |

## License

MIT
