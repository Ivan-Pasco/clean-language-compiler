#!/bin/bash

# Make the script executable: chmod +x scripts/run_integration_tests.sh
# Run with: ./scripts/run_integration_tests.sh

echo "Running Clean Language Compiler Integration Tests..."

# Build the compiler if needed
echo "Building compiler..."
cargo build --release

# Run the integration tests
echo "Running integration tests..."
cargo test --test integration -- --nocapture

# Test compiler CLI with example program
echo "Testing compiler CLI with example program..."

# First compile the hello world example
cargo run --bin cleanc compile examples/hello_world.cl examples/hello_world.wasm

# Then run the compiled WebAssembly file
if [ -f examples/hello_world.wasm ]; then
    echo "Compiled successfully. Running the program:"
    cargo run --bin cleanc run examples/hello_world.cl
else
    echo "Compilation failed, WebAssembly file not found."
    exit 1
fi

echo "All integration tests completed! 🎉" 