# quickmatch

**Lightning-fast fuzzy string matching for Rust.**

A high-performance string matching library optimized for interactive search experiences like autocomplete, command palettes, and search-as-you-type interfaces.

[![Crates.io](https://img.shields.io/crates/v/quickmatch.svg)](https://crates.io/crates/quickmatch)
[![Documentation](https://docs.rs/quickmatch/badge.svg)](https://docs.rs/quickmatch)

## Features

- **Blazing fast** - Optimized for sub-millisecond search times
- **Hybrid matching** - Word-level matching with trigram-based fuzzy fallback
- **Memory efficient** - Zero-copy string storage with pointer-based indexing
- **Ranked results** - Intelligent scoring based on match quality
- **Zero external dependencies** - Only uses `rustc-hash` for fast hashing

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
quickmatch = "0.1"
```
