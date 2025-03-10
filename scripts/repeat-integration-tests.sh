#!/bin/bash

mkdir -p logs
counter=1
while true; do
    CODEGEN=1 cargo test --workspace --exclude miden-client-web --release --test=integration --features "concurrent" -- --nocapture > "logs/output_file${counter}.log" 2>&1
    if [ $? -ne 0 ]; then
        echo "Command failed, stopping the loop."
        break
    fi
    counter=$((counter + 1))
done
