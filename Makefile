# ENV
FEATURES_INTEGRATION_TESTING="integration"
NODE_FEATURES_TESTING="testing"

# --- Testing ----------------------------------------------------------------------------------------

# Run tests
.PHONY: test
test:
	cargo nextest run --release --workspace

# Run tests with CI profile
.PHONY: ci-test
ci-test:
	cargo nextest run --profile ci-default --release --workspace

# --- Integration testing ----------------------------------------------------------------------------------------

# Run integration tests
.PHONY: integration-test
integration-test:
	cargo nextest run --release --test=integration --features $(FEATURES_INTEGRATION_TESTING)

# Run integration tests with CI profile
.PHONY: ci-integration-test
ci-integration-test:
	cargo nextest run --profile ci-default --release --test=integration --features $(FEATURES_INTEGRATION_TESTING)

# Kill node process
.PHONY: kill-node
kill-node:
	pkill miden-node || echo 'process not running'

# Clean node directory
.PHONY: clean-node
clean-node:
	rm -rf miden-node

# Setup node
.PHONY: node
node:
	if [ -d miden-node ]; then cd miden-node ; else git clone https://github.com/0xPolygonMiden/miden-node.git && cd miden-node; fi
	cd miden-node && git checkout main && git pull origin main && cargo update
	cd miden-node && rm -rf miden-store.sqlite3 miden-store.sqlite3-wal miden-store.sqlite3-shm
	cd miden-node && cargo run --bin miden-node --features $(NODE_FEATURES_TESTING) -- make-genesis --inputs-path ../tests/config/genesis.toml --force

# Run node
.PHONY: start-node
start-node:
	cd miden-node && cargo run --bin miden-node --features $(NODE_FEATURES_TESTING) -- start --config ../tests/config/miden-node.toml node

# --- Linting ----------------------------------------------------------------------------------------

# Runs format using nightly toolchain
.PHONY: format
format:
	cargo +nightly fmt --all

# Runs format using nightly toolchain but only in check mode
.PHONY: format-check
format-check:
	cargo +nightly fmt --all --check

# Runs clippy on all targets (except integration tests) with config
.PHONY: clippy
clippy:
	cargo clippy --workspace --all-targets -- -D clippy::all -D warnings

# Runs clippy integration tests with config
.PHONY: clippy-integration-tests
clippy-integration-tests:
	cargo clippy --workspace --tests --features integration -- -D clippy::all -D warnings

# Runs over all targets
.PHONY: clippy-all
clippy-all: clippy clippy-integration-tests

# Runs all linting tasks at once (clippy, formatting, doc)
.PHONY: lint
lint: check-format clippy-all doc

# --- Documentation site ----------------------------------------------------------------------------------------

# Install dependencies to build and serve documentation site
.PHONY: doc-deps
doc-deps:
	pip3 install -r scripts/docs_requirements.txt

# Build documentation site
.PHONY: doc-build
doc-build: doc-deps
	mkdocs build

# Serve documentation site
.PHONY: doc-serve
doc-serve: doc-deps
	mkdocs serve

# --- Rust documentation ----------------------------------------------------------------------------------------

# Generates & checks rust documentation
.PHONY: doc
doc:
	cargo doc --all--features --keep-going --release

