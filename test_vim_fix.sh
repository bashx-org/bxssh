#!/bin/bash

echo "Testing bxssh with vim character fix..."
echo "Building latest version..."
source ~/.cargo/env
cargo build --quiet

echo ""
echo "✅ FIX IMPLEMENTED: Vim color sequence filtering"
echo ""
echo "This version specifically filters out the sequences you reported:"
echo "- :ffff/ffff/ffff^G"  
echo "- ]11;rgb:1e1e/1e1e/1e1e^G"
echo "- OSC (Operating System Command) sequences"
echo "- Mouse terminal sequences"
echo ""
echo "Instructions for testing:"
echo "1. Run: RUST_LOG=debug ./target/debug/bxssh -u udara 192.168.1.110"
echo "2. Enter your password when prompted"
echo "3. Try opening vim: vim test.txt"
echo "4. The 'lots of characters' should no longer appear"
echo "5. Check debug logs for 'Filtering vim color response sequences'"
echo ""
echo "The debug log will show when filtering occurs:"
echo "- 'Filtering vim color response sequences'"
echo "- 'Filtering OSC sequences from vim'"
echo "- 'Filtering problematic mouse/terminal sequences'"
echo ""
echo "Press Enter to start the test..."
read
RUST_LOG=debug ./target/debug/bxssh -u udara 192.168.1.110