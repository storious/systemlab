.PHONY: test build fmt clean searchfs-test gdfs-test

test: searchfs-test gdfs-test

build:
	$(MAKE) -C searchfs build
	$(MAKE) -C gdfs build

lint:
	$(MAKE) -C searchfs lint
	$(MAKE) -C gdfs lint

searchfs-test:
	$(MAKE) -C searchfs test

gdfs-test:
	$(MAKE) -C gdfs test

clean:
	$(MAKE) -C searchfs clean
	$(MAKE) -C gdfs clean
