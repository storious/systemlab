# SystemLab

> Learn systems programming by building search engines, storage systems, caches, and distributed infrastructure from scratch.

![Purpose](https://img.shields.io/badge/purpose-learning-blue)
![CI](https://github.com/storious/systemlab/actions/workflows/ci.yaml/badge.svg)
![Rust](https://img.shields.io/badge/Rust-1.96+-orange)
![Go](https://img.shields.io/badge/Go-1.26+-00ADD8)
![Zig](https://img.shields.io/badge/Zig-0.16+-F7A41D)
![License](https://img.shields.io/github/license/storious/systemlab)

## Overview

SystemLab is a monorepo of independent systems programming projects.

Each project explores a core area of modern system software through incremental implementation, clean architecture, and practical experimentation.

The goal is **not** to build production-ready software, but to understand how real systems work by building them from scratch.

## Projects

| Project | Language | Focus |
|---------|----------|-------|
| **SearchFS** | Rust | Search engine, inverted index, ranking, query processing |
| **GDFS** | Go | Distributed file system, metadata management, replication |
| **ZigKV** | Zig | In-memory key-value cache, memory management, TTL |

## Repository Structure

```text
systemlab/
├── docs/
│   ├── architecture/
│   ├── adr/
│   └── ...
├── searchfs/
├── gdfs/
├── zigkv/
├── Makefile
├── ROADMAP.md
└── README.md
```

## Design Principles

SystemLab follows a few simple principles:

- **Learn by building**
- **Incremental implementation**
- **Small and understandable codebases**
- **Clean architecture**
- **Well-documented design decisions**
- **Independent but composable projects**

Each project is designed to be useful on its own while remaining easy to integrate with other projects through well-defined interfaces.

## Project Relationships

Projects evolve independently and may collaborate when appropriate.

```text
                SystemLab
                    │
    ┌───────────────┼────────────────┐
    │               │                │
 SearchFS         GDFS            ZigKV
  (Rust)          (Go)            (Zig)
    │               │                │
 Search         Storage          Cache
```

One possible integration looks like:

```text
          SearchFS
              │
     DocumentStore Interface
              │
   +----------+----------+
   |                     |
 LocalFS             Remote Storage
                        │
                       GDFS

          SearchFS
              │
      Document Cache
              │
            ZigKV
```

These integrations are optional rather than required. The projects remain independent learning projects with clearly defined responsibilities.

## Documentation

- `ROADMAP.md` — Overall learning roadmap
- `docs/` — Architecture, ADRs, and design notes
- `searchfs/README.md`
- `gdfs/README.md`
- `zigkv/README.md`

## Philosophy

> Learn by building.

Instead of reproducing production systems feature by feature, SystemLab focuses on understanding fundamental ideas through clean implementations, comprehensive tests, and continuous refinement.
