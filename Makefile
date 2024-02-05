FEATURES_CONCURRENT_TESTING=--features concurrent,testing
FEATURES_INTEGRATION_TESTING=$(FEATURES_CONCURRENT_TESTING),uuid
NODE_FEATURES_TESTING=--features testing
NODE_BINARY=--bin miden-node
HTTP_CODE := $(shell curl --http2-prior-knowledge -X POST -s -o /dev/null -w ''%{http_code}'' -H 'Content-Type: application/json' -d '{"jsonrpc":"2.0","method":"ping","params":[]}' http://localhost:57291)

integration_test:
	# Wait for the node to be up
	while [[ ${HTTP_CODE} != "200" ]]; do sleep 2; done		
	cargo run --release --bin="integration" $(FEATURES_INTEGRATION_TESTING)
	pkill miden-node

node:
	if cd miden-node; then git pull; else git clone https://github.com/0xPolygonMiden/miden-node.git; fi
	rm -rf miden-node/accounts
	rm -f miden-node/genesis.dat
	cd miden-node && cargo run --release $(NODE_BINARY) $(NODE_FEATURES_TESTING) -- make-genesis --inputs-path node/genesis.toml

start_node: node
	cd miden-node && cargo run --release $(NODE_BINARY) $(NODE_FEATURES_TESTING) -- start --config node/miden.toml

reset:
	rm -rf miden-node
	cargo clean
