#!/bin/bash

echo "Testing improved y/n prompt formatting in bxssh..."
source ~/.cargo/env
cargo build --quiet

echo ""
echo "🎯 IMPROVEMENT: Y/N prompt now appears on the same line"
echo ""
echo "Before: Prompt was on separate line"
echo "After:  Prompt with input on same line with colon"
echo ""
echo "Demo (will show improved prompt format):"
echo "Running: ./target/debug/bxssh udara@192.168.1.110"
echo ""
echo "Look for: '🔐 Key authentication failed. Try password authentication? (y/N): '"
echo "The cursor will be right after the colon, allowing inline input!"
echo ""
echo "Press Enter to see the demo..."
read
./target/debug/bxssh udara@192.168.1.110