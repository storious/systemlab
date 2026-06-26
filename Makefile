.PHONY: test build fmt clean searchfs-test gdfs-test

test: searchfs-test gdfs-test

build:
	$(MAKE) -C searchfs build
	$(MAKE) -C gdfs build

fmt:
	cd searchfs && cargo fmt --all
	cd gdfs && gofmt -w .

searchfs-test:
	cd searchfs && cargo test

gdfs-test:
	$(MAKE) -C gdfs test

clean:
	$(MAKE) -C searchfs clean
	$(MAKE) -C gdfs clean
