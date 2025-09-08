#!/bin/bash

echo "Building bxssh..."
source ~/.cargo/env
cargo build --quiet

echo "Testing simple command execution..."
RUST_LOG=debug ./target/debug/bxssh -u udara -c "echo 'Hello from remote server'" 192.168.1.110

echo ""
echo "Test completed. If this worked, the basic SSH connection is fine."
echo "Now try interactive mode with:"
echo "RUST_LOG=debug ./target/debug/bxssh -u udara 192.168.1.110"