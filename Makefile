.PHONY: all

INDEX := searchfs.idx
DOCS  := docs

all: test build

build:
	cargo run -- build $(DOCS) $(INDEX)

rebuild:
	rm -f $(INDEX)
	cargo run -- build $(DOCS) $(INDEX)

search:
	cargo run -- search $(INDEX) "rust" 10 and

test:
	cargo fmt --check
	cargo test
	cargo clippy --all-targets -- -D warnings

fmt:
	cargo fmt

lint:
	cargo clippy --all-targets -- -D warnings

clean:
	rm -f *.idx *.bin
