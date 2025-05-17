#!/bin/bash

# Make the script executable: chmod +x scripts/fix_call_indirect.sh
# Run with: ./scripts/fix_call_indirect.sh

# Run call_indirect_fix on array_ops.rs
cargo run --bin call_indirect_fix -- src/stdlib/array_ops.rs

echo "Fixed CallIndirect field names in array_ops.rs!" 