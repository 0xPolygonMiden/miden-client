FEATURES_INTEGRATION_TESTING="integration"

# compile before waiting for the node to be up
cargo build --release --bin="integration" --features "$FEATURES_INTEGRATION_TESTING"

# # Wait for the node to be up
# http_code="000"
# while true; do
#   if [ "$http_code" = "200" ]; then
#     break;
#   fi
#   sleep 2
#   http_code=$(curl --http2-prior-knowledge -X POST -s -o /dev/null -w ''%{http_code}'' -H 'Content-Type: application/json' -d '{"jsonrpc":"2.0","method":"ping","params":[]}' http://localhost:57291)
# done;
cargo run --release --bin="integration" --features "$FEATURES_INTEGRATION_TESTING"
pkill miden-node
