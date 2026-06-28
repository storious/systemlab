# SearchRS Roadmap

SearchRS is a learning project that incrementally implements the core building blocks of a modern search engine.

---

## v0.1.x — In-Memory Search

* In-memory inverted index
* Tokenization
* AND / OR / Phrase search
* TF-IDF ranking
* CLI

---

## v0.2.x — Persistence

* Snapshot serialization
* Snapshot loading
* Incremental indexing
* Persistent search

---

## v0.3.x — Segment Architecture

* Immutable segments
* Multi-segment indexing
* Segment merge / compaction
* Segment metadata
* Document metadata
* BM25 ranking
* Segment inspection
* Interactive REPL

---

## v0.4.x — Query Engine

* Query planner
* Skip lists
* Posting list compression
* Binary term dictionary
* Top-K collector
* Reader / scorer refactoring

---

## v0.5.x — Storage & Execution

- ✓ Storage abstraction
- ✓ Local filesystem backend
- ✓ In-memory storage backend
- ✓ Segment reader cache
- ✓ Parallel segment search
- ✓ REPL improvements
- ✓ Memory-mapped local storage
- ✓ Background merge scheduler

---

## v0.6.x — Search Engine

- Global search statistics
- Global BM25 scoring
- Query planner cache
- Parallel Top-K reduction
- Segment cache
- Query profiling
- Search metrics
- Benchmark suite

---

## v0.7.x — Search Service

- HTTP API
- REST search service
- JSON responses
- Concurrent query execution
- Configuration management
- Request tracing

---

## v0.8.x — Distributed Search

- Metadata service
- Sharding
- Replication
- Distributed query execution
- Cluster management

