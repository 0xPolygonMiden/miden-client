.DEFAULT_GOAL := help

.PHONY: help
help: ## Show description of all commands
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

# --- Variables ----------------------------------------------------------------------------------------

FEATURES_INTEGRATION_TESTING="integration"
NODE_FEATURES_TESTING="testing"
WARNINGS=RUSTDOCFLAGS="-D warnings"

# --- Linting ----------------------------------------------------------------------------------------

.PHONY: clippy
clippy: ## Runs clippy on all targets with config
	cargo +nightly clippy --workspace --tests --all-targets --all-features -- -D clippy::all -D warnings

.PHONY: fix
fix: ## Runs Fix with configs
	cargo +nightly fix --allow-staged --allow-dirty --all-targets --all-features

.PHONY: format
format: ## Runs format using nightly toolchain
	cargo +nightly fmt --all

.PHONY: format-check
format-check: ## Runs format using nightly toolchain but only in check mode
	cargo +nightly fmt --all --check

.PHONY: lint
lint: format fix clippy ## Runs all linting tasks at once (clippy, fixing, formatting)

# --- Documentation site ----------------------------------------------------------------------------------------

.PHONY: doc-deps
doc-deps: ## Install dependencies to build and serve documentation site
	pip3 install -r scripts/docs_requirements.txt

.PHONY: doc-build
doc-build: doc-deps ## Build documentation site
	mkdocs build

.PHONY: doc-serve
doc-serve: doc-deps ## Serve documentation site
	mkdocs serve

# --- Rust documentation ----------------------------------------------------------------------------------------

.PHONY: doc
doc: ## Generates & checks rust documentation
	$(WARNINGS) cargo doc --all-features --keep-going --release

# --- Testing ----------------------------------------------------------------------------------------

.PHONY: test
test: ## Run tests
	cargo nextest run --release --workspace

# --- Integration testing ----------------------------------------------------------------------------------------

.PHONY: integration-test
integration-test: ## Run integration tests
	cargo nextest run --release --test=integration --features $(FEATURES_INTEGRATION_TESTING)

.PHONY: kill-node
kill-node: ## Kill node process
	pkill miden-node || echo 'process not running'

.PHONY: clean-node
clean-node: ## Clean node directory
	rm -rf miden-node

.PHONY: node
node: ## Setup node
	if [ -d miden-node ]; then cd miden-node ; else git clone https://github.com/0xPolygonMiden/miden-node.git && cd miden-node; fi
	cd miden-node && git checkout main && git pull origin main && cargo update
	cd miden-node && rm -rf miden-store.sqlite3 miden-store.sqlite3-wal miden-store.sqlite3-shm
	cd miden-node && cargo run --bin miden-node --features $(NODE_FEATURES_TESTING) -- make-genesis --inputs-path ../tests/config/genesis.toml --force

.PHONY: start-node
start-node: ## Run node
	cd miden-node && cargo run --bin miden-node --features $(NODE_FEATURES_TESTING) -- start --config ../tests/config/miden-node.toml node
