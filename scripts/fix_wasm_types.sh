#!/bin/bash

# Make the script executable: chmod +x scripts/fix_wasm_types.sh
# Run with: ./scripts/fix_wasm_types.sh

# Run wasm_type_fix on relevant files
cargo run --bin wasm_type_fix -- src/stdlib/string_ops.rs
cargo run --bin wasm_type_fix -- src/stdlib/array_ops.rs
cargo run --bin wasm_type_fix -- src/stdlib/matrix_ops.rs
cargo run --bin wasm_type_fix -- src/codegen/mod.rs

echo "Fixed WasmType usage in all relevant files!" 