# SearchFS

![CI](https://github.com/storious/searchfs/actions/workflows/ci.yml/badge.svg)

## Usage

Build an index:

```bash
make build
```
Search from a persisted index:

```bash
make search

```

Or directly:

```bash
cargo run -- build docs searchfs.idx
cargo run -- search searchfs.idx '"white whale"' 5
cargo run -- search searchfs.idx "rust memory" 10 and
cargo run -- search searchfs.idx "rust memory" 10 or
```

Development

```bash
make test
make clean
```
