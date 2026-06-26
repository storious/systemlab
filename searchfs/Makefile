.PHONY: all build index rebuild search test fmt lint clean

INDEX := searchfs.idx
DOCS  := testdata

all: test build

build:
	@cargo build

release:
	@cargo build --release

index:
	@cargo run -- build $(DOCS) $(INDEX)

rebuild:
	@rm -f $(INDEX)
	@cargo run -- build $(DOCS) $(INDEX)

search:
	@cargo run -- search $(INDEX) "rust" 10 and

test:
	@cargo test

fmt:
	@cargo fmt --all

clippy:
	@cargo clippy --all-targets --all-features -- -D warnings

lint: fmt clippy

clean:
	@rm -rf *.idx *.bin tmp* searchfs_index target
