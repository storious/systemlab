# ADR 0002: GDFS Registry Backend

## Status

Accepted

## Context

GDFS v0.2 introduces DataNode registry, heartbeat, health tracking, and placement.

The registry currently lives in the NameNode memory space.

An external registry backend such as ZigKV may be useful in the future, but it would introduce a separate storage dependency and require a more explicit registry abstraction.

## Decision

GDFS v0.2 uses an in-memory registry inside the NameNode.

ZigKV-backed registry is deferred to a future experiment.

## Consequences

- GDFS v0.2 remains self-contained.
- Heartbeat-driven placement can be released without external dependencies.
- Future registry backends can be introduced behind a Registry interface.
