FEATURES_CONCURRENT_TESTING=--features concurrent,testing
NODE_FEATURES_TESTING=--features testing
NODE_BINARY=--bin miden-node

client_test:
	# Wait for the node to be up
	while [[ "curl --http2-prior-knowledge -X POST -s -o /dev/null -w ''%{http_code}'' -H 'Content-Type: application/json' -d '{\"jsonrpc\":\"2.0\",\"method\":\"ping\",\"params\":[]}' http://localhost:57291" -eq 200 ]]; do sleep 2; done		
	cargo test $(FEATURES_CONCURRENT_TESTING) -- client_test_0

node:
	if cd miden-node; then git pull; else git clone https://github.com/0xPolygonMiden/miden-node.git; fi
	rm -rf miden-node/accounts
	rm -f miden-node/genesis.dat
	cd miden-node && cargo run $(NODE_BINARY) $(NODE_FEATURES_TESTING) -- make-genesis --inputs-path node/genesis.toml

start_node: node
	cd miden-node && cargo run $(NODE_BINARY) $(NODE_FEATURES_TESTING) -- start --config node/miden.toml

reset:
	rm -rf miden-node
