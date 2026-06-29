# ADR 0001: GDFS Metadata Model

## Status

Accepted

## Context

GDFS v0.1 used `BlockInfo` directly in `FileMetadata`.

This was sufficient for a single DataNode, but it does not support distributed storage because a block's metadata did not record where replicas were stored.

As GDFS moves toward block placement and replication, metadata must describe both the logical file layout and the physical replica locations.

## Decision

GDFS file metadata is modeled as:

```text
FileMetadata
  └── ordered []BlockMetadata
          ├── BlockInfo
          └── []BlockReplica
```

Where:

- `FileMetadata` represents a logical file.
- `BlockMetadata` represents one ordered block of the file.
- `BlockInfo` stores block identity, size, and checksum.
- `BlockReplica` stores the DataNode location for a committed replica.

## Invariants

1. File block order is defined by the order of `FileMetadata.Blocks`.
2. A block may have one or more committed replicas.
3. Reads must be driven by replica metadata, not by a default DataNode address.
4. Placement produces candidate DataNodes.
5. Metadata records only successfully written replicas.
6. A file commit should only happen after required block writes succeed.
7. NameNode owns file metadata.
8. DataNode owns block bytes.

## Consequences

This enables:

- replica-aware reads
- multi-DataNode writes
- future replication
- future recovery
- future block rebalancing

This also means `DFSClient` becomes responsible for coordinating:

```text
split file
  -> allocate block
  -> write replicas
  -> commit file metadata
```

## Non-goals

This ADR does not define:

- replication pipeline strategy
- retry policy
- recovery mechanism
- metadata persistence
- consensus or multi-NameNode design
