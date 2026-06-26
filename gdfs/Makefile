.PHONY: all build test fmt vet lint clean \
        namenode datanode client

BIN_DIR := bin

all: test build

build: namenode datanode client

namenode:
	@mkdir -p $(BIN_DIR)
	@go build -o $(BIN_DIR)/namenode ./cmd/namenode

datanode:
	@mkdir -p $(BIN_DIR)
	@go build -o $(BIN_DIR)/datanode ./cmd/datanode

client:
	@mkdir -p $(BIN_DIR)
	@go build -o $(BIN_DIR)/gdfs ./cmd/gdfs

test:
	@go test ./...

fmt:
	@go fmt ./...

vet:
	@go vet ./...

lint: fmt vet test

clean:
	@find . -type d -name "*_network" -exec rm -rf {} + 2>/dev/null || true
