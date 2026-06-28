# GDFS Roadmap

GDFS is a long-term learning project that explores the architecture and implementation of distributed file systems.

The roadmap is organized around capabilities rather than release versions. Individual implementations may evolve over time while the overall direction remains stable.

---

## Stage 1 — Local Storage

Build a reliable local storage engine.

Focus areas:

- Block storage
- Local persistence
- Data integrity
- Storage abstraction
- Unit testing

Outcome:

A standalone storage layer capable of reading and writing blocks.

---

## Stage 2 — Distributed File System

Transform local storage into a distributed file system.

Focus areas:

- NameNode
- DataNode
- Metadata management
- Block placement
- Client protocol
- File upload and download

Outcome:

A minimal distributed file system supporting multiple storage nodes.

---

## Stage 3 — Reliability

Improve robustness under failures.

Focus areas:

- Replication
- Heartbeats
- Failure detection
- Recovery
- Metadata persistence
- Consistency

Outcome:

A distributed file system capable of tolerating node failures.

---

## Stage 4 — Scalability

Improve system scalability.

Focus areas:

- Rebalancing
- Snapshot
- Background maintenance
- Large file support
- Performance optimization

Outcome:

A storage system capable of supporting larger datasets and clusters.

---

## Stage 5 — Object Storage

Extend the file system into an object storage platform.

Focus areas:

- Object abstraction
- Bucket management
- Object metadata
- S3-compatible interfaces
- Access control

Outcome:

A lightweight object storage service built on the GDFS storage layer.

---

## Integration

GDFS is designed to evolve independently from SearchRS.

Rather than serving as a storage plugin, GDFS provides one possible implementation of a generic storage interface.

```text
SearchRS
      │
DocumentStore Interface
      │
+-----+--------+-----------+
|              |           |
LocalFS       GDFS      S3 / OSS
```

This separation allows each project to evolve independently while remaining interoperable.

---

## Guiding Principles

Throughout the project, GDFS follows several principles:

- Learn by building
- Keep the architecture understandable
- Prefer simple implementations first
- Add complexity only when necessary
- Document important design decisions
- Prioritize correctness over optimization
