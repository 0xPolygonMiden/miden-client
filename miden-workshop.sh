#!/bin/bash

# Function to check if Rust is installed
check_rust_installed() {
    if ! command -v cargo &> /dev/null
    then
        echo "Rust is not installed. Installing now..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
        source $HOME/.cargo/env
    else
        echo "Rust is already installed."
    fi
}

# Check if Rust is installed
check_rust_installed

# Default directory
default_dir="$HOME/Documents/miden-denver-workshop"

# Ask for target directory with a default value
echo "Enter the target directory: $default_dir"
read target_dir

# Use default directory if user just hits enter
if [ -z "$target_dir" ]
then
    target_dir="$default_dir"
fi

# Create the directory if it doesn't exist
mkdir -p "$target_dir"

# Clone the repository into the target directory
git clone https://github.com/0xPolygonMiden/miden-client "$target_dir/miden-denver-workshop"

# Change to the specified directory
cd "$target_dir/miden-denver-workshop"

# Replace the contents of miden-client.toml
cat << EOF > miden-client.toml
[rpc]
endpoint = { protocol = "http", host = "54.246.203.33", port = 57291 }

[store]
database_filepath = "store.sqlite3"
EOF

# Run Rust commands
cargo install --force --features testing,concurrent --path .
miden-client account new basic-immutable
miden-client account -l
