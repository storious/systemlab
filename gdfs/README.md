# GDFS

> A toy distributed file system written in Go.

## Overview

GDFS is a learning project that explores the core ideas behind distributed file systems.

Rather than pursuing production performance, the project focuses on understanding the architecture and implementation of systems such as HDFS, GFS, and modern object storage systems.

## Goals

- Understand distributed file system architecture
- Implement block-based storage
- Learn metadata management
- Implement replication and recovery
- Explore distributed system fundamentals

## Architecture

```text
                 +-------------+
                 |   Client    |
                 +------+------+
                        |
                 Metadata Request
                        |
                 +------+------+
                 |  NameNode   |
                 +------+------+
                        |
              Block Placement Metadata
                        |
        +---------------+---------------+
        |                               |
 +------+-------+               +-------+------+
 | DataNode #1  |               | DataNode #2  |
 | Block Store  |               | Block Store  |
 +--------------+               +--------------+
```

## Repository Layout

```text
gdfs/
├── cmd/
│   ├── gdfs/
│   ├── namenode/
│   └── datanode/
│
├── internal/
│   ├── client/
│   ├── datanode/
│   ├── namenode/
│   ├── protocol/
│   └── storage/
│
└── legacy/
```

## Development Roadmap

### Phase 1

- Local Block Store
- DataNode
- NameNode
- Client
- File Upload / Download

### Phase 2

- Block Replication
- Heartbeat
- Failure Detection
- Metadata Persistence

### Phase 3

- Rebalancing
- Snapshot
- Object Storage Interface

See `ROADMAP.md` for the complete roadmap.

## Relationship with SearchFS

GDFS is **not** a storage plugin for SearchFS.

Instead, both projects evolve independently and communicate through a storage abstraction.

```text
SearchFS
     │
DocumentStore Interface
     │
+----+---------+-----------+
|              |           |
LocalFS       GDFS      S3 / OSS
```

This design keeps SearchFS independent of any particular storage implementation while allowing GDFS to serve as one possible backend.

## Status

🚧 This project is under active development.

The architecture may change as new concepts are explored and implemented.
