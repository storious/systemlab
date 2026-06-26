.PHONY: all build index rebuild search test fmt lint clean

INDEX := searchfs.idx
DOCS  := docs

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
	@cargo fmt --all -- --check
	@cargo test
	@cargo clippy --all-targets --all-features -- -D warnings

fmt:
	@cargo fmt --all

lint:
	@cargo clippy --all-targets --all-features -- -D warnings

clean:
	@rm -rf *.idx *.bin tmp* searchfs_index target
