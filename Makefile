NODE_FEATURES_TESTING=--features testing
NODE_BINARY=--bin miden-node

HTTP_CODE = "200"

integration-test:
	./tests/run_integration_test.sh || echo "integration test failed with exit code $$?"

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

fmt:
	cargo fix --allow-staged --allow-dirty --all-targets --all-features
	cargo fmt
	cargo clippy --all-targets --all-features -- -D clippy::all -D warnings
