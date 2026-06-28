# Roadmap

SystemLab is a long-term systems programming learning project.

The goal is to understand how modern systems work by building small, focused implementations from scratch.

Rather than pursuing production-ready software, SystemLab emphasizes architectural understanding, incremental implementation, and engineering practice.

---

# Learning Domains

## Search Systems

Understand how search engines work.

Topics include:

- Document parsing
- Inverted indexes
- Ranking algorithms
- Query processing
- Index organization
- Distributed search

Current project:

- **SearchRS** (Rust)

---

## Storage Systems

Understand how data is stored locally and across machines.

Topics include:

- Storage engines
- Block storage
- Distributed file systems
- Object storage
- Metadata management
- Replication
- Fault tolerance

Current project:

- **GDFS** (Go)

---

## Cache Systems

Understand high-performance in-memory storage.

Topics include:

- Hash tables
- Memory allocators
- TTL
- Cache eviction
- Persistence (WAL / Snapshot)
- Networking

Current project:

- **ZigKV** (Zig)

---

## Distributed Systems

Understand how multiple machines coordinate.

Topics include:

- RPC
- Service discovery
- Heartbeats
- Replication
- Consensus
- Scheduling

Future projects may emerge naturally from SearchRS and GDFS.

---

## Systems Programming

Build software close to the operating system.

Topics include:

- Memory management
- Networking
- Concurrency
- Serialization
- Storage engines
- Performance optimization

This domain spans every project in SystemLab.

---

# Repository Vision

```text
                    SystemLab
                         │
      ┌──────────────────┼──────────────────┐
      │                  │                  │
  Search Systems     Storage Systems    Cache Systems
      │                  │                  │
  SearchRS            GDFS             ZigKV
     (Rust)            (Go)             (Zig)
```

Projects evolve independently.

When appropriate, they may collaborate through well-defined interfaces.

For example:

```text
           SearchRS
               │
      DocumentStore Interface
               │
    +----------+----------+
    |                     |
 LocalFS              Remote Storage
                         │
                        GDFS

           SearchRS
               │
       Document Cache
               │
             ZigKV
```

These integrations are optional rather than required.

---

# Design Principles

Every project should be:

- Learn by building
- Incremental
- Easy to understand
- Independently usable
- Well documented
- Cleanly architected

Complexity should be introduced only when it improves understanding.

---

# Long-term Roadmap

The repository will continue to grow by adding focused learning projects.

Current roadmap:

- ✅ SearchRS — Search engine fundamentals
- ✅ GDFS — Distributed storage fundamentals
- ✅ ZigKV — In-memory cache fundamentals

Potential future projects:

- TinyMQ — Message queue
- ToyDB — Storage engine / database
- TinyScheduler — Distributed scheduling
- TinyRPC — RPC framework

Each project explores one core systems topic while remaining small enough to understand from source code.
