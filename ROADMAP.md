# Roadmap

SystemLab is a long-term learning project focused on understanding modern systems by building them from scratch.

Rather than pursuing production-ready software, the repository emphasizes architectural understanding, incremental implementation, and engineering practice.

## Learning Directions

### Search Systems

Build a search engine from first principles.

Topics include:

- Document parsing
- Inverted indexes
- Ranking algorithms
- Query processing
- Index organization
- Distributed search

Current project:

- SearchFS

---

### Storage Systems

Build storage systems from local storage to distributed storage.

Topics include:

- Local storage engines
- Block storage
- Distributed file systems
- Object storage
- Replication
- Metadata management
- Fault tolerance

Current project:

- GDFS

---

### Distributed Systems

Understand how distributed systems coordinate and scale.

Topics include:

- RPC
- Service discovery
- Heartbeats
- Consensus
- Replication
- Scheduling

---

### Systems Programming

Learn low-level system implementation through practical projects.

Topics include:

- Memory management
- Concurrency
- Networking
- Serialization
- Storage engines
- Performance optimization

---

## Project Philosophy

Each project in this repository should satisfy the following principles:

- Learn by building
- Keep implementations understandable
- Evolve incrementally
- Document important design decisions
- Prefer clear architecture over unnecessary complexity

Projects should remain independently useful while being able to collaborate through well-defined interfaces.

---

## Repository Vision

```
                SystemLab
                     │
     ┌───────────────┴───────────────┐
     │                               │
 Search Systems                Storage Systems
     │                               │
 SearchFS                       GDFS
     │                               │
     └────────── DocumentStore ──────┘
```

Future projects may be added as the repository grows, while preserving the independence of each project.
