#!/bin/bash

# Make the script executable: chmod +x scripts/fix_errors.sh
# Run with: ./scripts/fix_errors.sh

# Path to the error_fix script
cargo run --bin error_fix -- src/codegen/mod.rs
cargo run --bin error_fix -- src/stdlib/mod.rs
cargo run --bin error_fix -- src/stdlib/error.rs
cargo run --bin error_fix -- src/stdlib/string_ops.rs
cargo run --bin error_fix -- src/stdlib/array_ops.rs
cargo run --bin error_fix -- src/validation/mod.rs
cargo run --bin error_fix -- src/parser/mod.rs

echo "All error method calls fixed!" 