#!/bin/bash

RUST_LOG=none cargo run --release --package node-builder & echo $! > .node.pid;
sleep 4;
if ! ps -p $(cat .node.pid) > /dev/null; then
    echo "Failed to start node server";
    rm -f .node.pid;
    exit 1;
fi;
rm -f .node.pid
