# SystemLab

> Learn search engines, distributed storage systems, and systems programming by building them from scratch.

![CI](https://github.com/storious/systemlab/actions/workflows/ci.yaml/badge.svg)

## Overview

SystemLab is a monorepo containing a collection of independent but collaborative learning projects.

The goal is not to build production-ready software, but to understand how modern systems work through incremental implementation and experimentation.

## Projects

| Project | Language | Description |
|---------|----------|-------------|
| **SearchFS** | Rust | A toy search filesystem for learning inverted indexes, ranking, and search engine architecture. |
| **GDFS** | Go | A toy distributed file system for learning metadata management, block storage, replication, and distributed systems. |

## Repository Structure

```text
systemlab/
├── docs/          # Architecture, ADRs and design notes
├── searchfs/      # Rust search engine
└── gdfs/          # Go distributed file system
```

## Project Relationship

The projects evolve independently.

```
          SearchFS
              │
      DocumentStore Interface
              │
   +----------+----------+
   |                     |
 LocalFS              Remote Storage
                         │
                +--------+--------+
                |                 |
              GDFS            S3 / OSS
```

SearchFS depends only on an abstract storage interface.

GDFS is one possible implementation of that interface, rather than a plugin tightly coupled to SearchFS.

## Documentation

- `ROADMAP.md` — Overall learning roadmap
- `docs/` — Architecture, ADRs, and design documents
- `searchfs/README.md`
- `gdfs/README.md`

## Philosophy

This repository emphasizes:

- Learning by building
- Simple, incremental design
- Clean architecture
- Well-documented design decisions

Each project should remain useful and understandable on its own while being able to collaborate with other projects through well-defined interfaces.
