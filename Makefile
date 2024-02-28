SHELL := /bin/bash

FEATURES_INTEGRATION_TESTING="integration"
NODE_FEATURES_TESTING=--features testing
NODE_BINARY=--bin miden-node

integration-test:
	set -o pipefail; cargo run --release --bin="integration" --features "$(FEATURES_INTEGRATION_TESTING)"; pkill miden-node
	exit $$? || echo "integration test failed with exit code $$?"


node:
	if cd miden-node; then git pull; else git clone https://github.com/0xPolygonMiden/miden-node.git; fi
	rm -rf miden-node/miden-store.sqlite3 miden-node/miden-store.sqlite3-wal miden-node/miden-store.sqlite3-shm
	rm -rf miden-node/accounts
	rm -f miden-node/genesis.dat
	cd miden-node && cargo run --release $(NODE_BINARY) $(NODE_FEATURES_TESTING) -- make-genesis --inputs-path node/genesis.toml

start-node: node
	cd miden-node && cargo run --release $(NODE_BINARY) $(NODE_FEATURES_TESTING) -- start --config node/miden-node.toml

reset:
	rm -rf miden-node
	cargo clean

docs_deps:
	cd docs && pip3 install -r ../requirements.txt

build_docs: docs_deps
	cd docs && mkdocs build

serve_docs: docs_deps
	cd docs && mkdocs serve
  
fmt:
	cargo fix --allow-staged --allow-dirty --all-targets --all-features
	cargo fmt
	cargo clippy --all-targets --all-features -- -D clippy::all -D warnings
