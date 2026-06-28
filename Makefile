.PHONY: \
	all check \
	build test fmt lint clean \
	searchrs-build searchrs-test searchrs-fmt searchrs-lint \
	gdfs-build gdfs-test gdfs-fmt gdfs-lint \
	zigkv-build zigkv-test zigkv-fmt zigkv-lint

all: build

check: fmt lint test

#
# All projects
#

build: searchrs-build gdfs-build zigkv-build

test: searchrs-test gdfs-test zigkv-test

fmt: searchrs-fmt gdfs-fmt zigkv-fmt

lint: searchrs-lint gdfs-lint zigkv-lint

clean:
	$(MAKE) -C searchrs clean
	$(MAKE) -C gdfs clean
	$(MAKE) -C zigkv clean

#
# searchrs
#

searchrs-build:
	$(MAKE) -C searchrs build

searchrs-test:
	$(MAKE) -C searchrs test

searchrs-fmt:
	$(MAKE) -C searchrs fmt

searchrs-lint:
	$(MAKE) -C searchrs lint

#
# GDFS
#

gdfs-build:
	$(MAKE) -C gdfs build

gdfs-test:
	$(MAKE) -C gdfs test

gdfs-fmt:
	$(MAKE) -C gdfs fmt

gdfs-lint:
	$(MAKE) -C gdfs lint

#
# ZigKV
#

zigkv-build:
	$(MAKE) -C zigkv build

zigkv-test:
	$(MAKE) -C zigkv test

zigkv-fmt:
	$(MAKE) -C zigkv fmt

zigkv-lint:
	$(MAKE) -C zigkv lint

