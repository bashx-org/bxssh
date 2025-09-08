#!/bin/bash

echo "Testing bxssh with vim character fix..."
echo "Building latest version..."
source ~/.cargo/env
cargo build --quiet

echo ""
echo "Instructions for testing:"
echo "1. Run: RUST_LOG=debug ./target/debug/bxssh -u udara 192.168.1.110"
echo "2. Enter your password when prompted"
echo "3. Try opening vim: vim test.txt"
echo "4. Look for debug output about escape sequences"
echo "5. Check if the 'lots of characters' issue is resolved"
echo ""
echo "The debug log will show:"
echo "- High volume of escape sequences (if vim sends many ESC chars)"
echo "- Vim alternate screen buffer commands"
echo "- Vim cursor visibility commands"  
echo "- Filtering of problematic mouse/terminal sequences"
echo ""
echo "Press Enter to start the test..."
read
RUST_LOG=debug ./target/debug/bxssh -u udara 192.168.1.110