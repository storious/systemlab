# SearchRS

> A modular search engine written in Rust, built from first principles and
> inspired by Lucene and Tantivy.


SearchRS is a learning project that incrementally implements the core building
blocks of a modern search engine, including immutable segments, BM25 ranking,
posting list compression, query planning, and parallel segment search.

---

## Features

Current capabilities include:

- In-memory inverted index
- Snapshot persistence
- Incremental indexing
- Immutable segment architecture
- Multi-segment search
- Parallel segment search
- Automatic segment merge scheduling
- Segment merge / compaction
- BM25 ranking
- Posting list compression
- Query planner
- Top-K collector
- Interactive search REPL
- Segment inspection utilities
- Pluggable storage backend
- Pluggable posting codec
- Memory-mapped local storage
---

## Build

Build a snapshot index:

```bash
cargo run -- build docs searchrs.idx
```

Build an immutable segment index:

```bash
cargo run -- build-segment docs searchrs_index
```

---

## Search

Search a snapshot index:

```bash
cargo run -- search searchrs.idx "white whale" 5 phrase
cargo run -- search searchrs.idx "rust memory" 10 and
cargo run -- search searchrs.idx "rust memory" 10 or
```

Search a segment index:

```bash
cargo run -- search-segments searchrs_index "white whale" 5
cargo run -- search-segments searchrs_index "rust memory" 10 and
cargo run -- search-segments searchrs_index "rust memory" 10 or
```

---

## Interactive REPL

Start an interactive search session:

```bash
cargo run -- repl searchrs_index
```

Built-in commands:

```text
:q
:help
:stats

:mode and
:mode or
:mode phrase

:limit 20
```

---

## Segment Management

Append a new immutable segment:

```bash
cargo run -- update-segment searchrs_index new_docs
```

Merge all segments:

```bash
cargo run -- merge-segments searchrs_index
```

Inspect segment metadata:

```bash
cargo run -- inspect-segments searchrs_index
```

---

## Development

Run tests:

```bash
make test
```

Run Clippy:

```bash
cargo clippy -- -D warnings
```

Format source:

```bash
cargo fmt
```

Clean build artifacts:

```bash
make clean
```

---

## Architecture

```text
                     CLI / REPL

                         │

                  Query Planner

                         │

                 Segment Searcher

                         │

               Segment Reader Cache

                         │

                  Segment Reader

                         │

                   Segment Store

                  ┌──────┴──────┐

               Storage        Codec

          ┌────────────┐    ┌────────────────────┐

      LocalStorage      PostingCodec

      MemoryStorage     CompressedPostingCodec
```

---

## Project Structure

```text
src/

├── cmd/          CLI and REPL
├── index/        Tokenization and in-memory indexing
├── query/        Query planner and collectors
├── segment/      Immutable segment engine
├── snapshot/     Snapshot persistence
└── storage/      Storage abstraction
```

---

## Roadmap

See [ROADMAP.md](ROADMAP.md).


## Release History

| Version | Highlights |
|---------|------------|
| v0.1 | In-memory inverted index |
| v0.2 | Snapshot persistence |
| v0.3 | Immutable segment architecture |
| v0.4 | Query engine and posting compression |
| v0.5 | Storage abstraction, parallel search, merge scheduler |
