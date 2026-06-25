# SearchFS

> A toy search engine written in Rust, evolving from an in-memory inverted index
> to a segment-based search engine inspired by Lucene and Tantivy.

![CI](https://github.com/storious/searchfs/actions/workflows/ci.yaml/badge.svg)

## Features

Current capabilities include:

- In-memory inverted index
- Snapshot persistence
- Incremental indexing
- Immutable segment storage
- Multi-segment search
- Segment merge / compaction
- BM25 ranking
- Interactive search REPL
- Segment inspection utilities

---

## Build

Build a snapshot index:

```bash
cargo run -- build docs searchfs.idx
```

Build an immutable segment index:

```bash
cargo run -- build-segment docs searchfs_index
```

---

## Search

Search a snapshot index:

```bash
cargo run -- search searchfs.idx "white whale" 5 phrase
cargo run -- search searchfs.idx "rust memory" 10 and
cargo run -- search searchfs.idx "rust memory" 10 or
```

Search a segment index:

```bash
cargo run -- search-segments searchfs_index "white whale" 5
cargo run -- search-segments searchfs_index "rust memory" 10 and
cargo run -- search-segments searchfs_index "rust memory" 10 or
```

---

## REPL

Start an interactive search session:

```bash
cargo run -- repl searchfs_index
```

Available commands:

```
:q
:mode and
:mode or
:mode phrase
:limit 20
:stats
:help
```

---

## Segment Management

Append a new immutable segment:

```bash
cargo run -- update-segment searchfs_index new_docs
```

Merge all existing segments:

```bash
cargo run -- merge-segments searchfs_index
```

Inspect segment metadata:

```bash
cargo run -- inspect-segments searchfs_index
```

---

## Development

Run tests:

```bash
make test
```

Clean build artifacts:

```bash
make clean
```

Run Clippy:

```bash
cargo clippy -- -D warnings
```

Format source:

```bash
cargo fmt
```

---

## Project Structure

```
index/
    tokenizer
    cleaner
    crawler
    inverted index

segment/
    immutable segment format
    segment reader
    segment store
    BM25 scorer
    merge / compaction

cmd/
    CLI
    REPL

snapshot/
    legacy snapshot persistence
```

---

## Roadmap

See [ROADMAP.md](ROADMAP.md).
