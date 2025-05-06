# Miden Node Builder (Testing Only)

A minimal node implementation used exclusively for running integration tests of the Miden client. This crate is NOT intended for production use.

## Purpose

This crate provides a simplified node implementation that is used to run integration tests for:
- The Miden client library
- The Miden web client
- Other client-related integration tests

## Features

- Minimal node implementation with essential components
- Configurable block and batch intervals
- Support for both local and remote provers
- Simple setup for testing scenarios

## Usage

```rust
use miden_node_builder::NodeBuilder;
use std::path::PathBuf;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create a new node builder for testing
    let builder = NodeBuilder::new(PathBuf::from("./data"))
        .with_block_interval(Duration::from_millis(1000))
        .with_batch_interval(Duration::from_millis(1000));

    // Start the node
    let node_handle = builder.start().await?;
    println!("RPC server listening at: {}", node_handle.rpc_url());

    // ... run tests ...

    // Stop the node when done
    node_handle.stop().await?;

    Ok(())
}
```

For a complete working example, see the `simple.rs` example in the crate's source code.

## Components

The builder initializes and manages the following components:

1. **Store**: Manages the node's state and data persistence
2. **Block Producer**: Handles block production and validation
3. **RPC Server**: Provides an interface for interacting with the node

## Configuration Options

- `data_directory`: Path to store node data
- `block_interval`: Duration between block production attempts
- `batch_interval`: Duration between batch production attempts

## Note

This implementation is intentionally simplified and may not include all features of a production node. It is designed to be:
- Easy to maintain
- Quick to start up
- Sufficient for running integration tests
- NOT suitable for production use

## License
This project is [MIT licensed](../../LICENSE).
