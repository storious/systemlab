# GDFS

> A minimal distributed file system written in Go for learning distributed systems.

## Overview

GDFS is a learning-oriented distributed file system inspired by systems such as GFS and HDFS.

It is designed to be small enough to understand, but complete enough to be genuinely useful for personal experiments and small internal deployments.

## Quick Start

Build binaries:

```bash
make build
```

Start the NameNode:

```bash
./bin/namenode -addr :9000
```

Start the DataNode:

```bash
./bin/datanode -id node-1 -addr :9001 -root ./data/node-1
```

Upload a file:

```bash
./bin/gdfs put README.md /docs/readme.md
```

Inspect metadata:

```bash
./bin/gdfs stat /docs/readme.md
```

Download the file:

```bash
./bin/gdfs get /docs/readme.md out.md
```

Delete the file:

```bash
./bin/gdfs delete /docs/readme.md
```

## Features

Current capabilities:

- Local block storage
- DataNode block service
- NameNode metadata service
- HTTP-based APIs
- DFS client coordinator
- File upload and download
- End-to-end single-node integration tests

Planned capabilities:

- Replication
- Heartbeats
- Failure detection
- Metadata persistence
- Recovery
- Rebalancing
- Object storage interface

## Architecture

```text
              +-------------+
              |  GDFS CLI   |
              +------+------+ 
                     |
              +------v------+
              | DFS Client  |
              +---+-----+---+
                  |     |
          Metadata|     |Blocks
                  |     |
          +-------v-+ +-v--------+
          |NameNode | |DataNode  |
          +----+----+ +----+-----+
               |           |
     In-memory Metadata  LocalBlockStore
```

## Repository Layout

```text
gdfs/
├── cmd/
│   ├── gdfs/
│   ├── namenode/
│   └── datanode/
│
└── internal/
    ├── client/
    ├── datanode/
    ├── namenode/
    ├── protocol/
    └── storage/
```

## Current Implementation

GDFS v0.1 supports:

- Single NameNode
- Single DataNode
- In-memory metadata
- Local block storage
- HTTP communication
- File upload and download through CLI

Current limitations:

- No replication
- No fault tolerance
- No persistent metadata
- No block placement policy
- No authentication or access control

## Roadmap

See [ROADMAP.md](ROADMAP.md).

## Relationship with SearchFS

GDFS is **not** a storage plugin for SearchFS.

SearchFS and GDFS evolve independently and communicate through a storage abstraction.

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

## Project Philosophy

GDFS is intentionally designed to remain small, understandable, and useful.

Rather than competing with production distributed file systems, it focuses on implementing the core ideas behind systems such as GFS and HDFS while remaining suitable for learning, experimentation, and small-scale use
