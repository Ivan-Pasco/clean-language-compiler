#!/bin/bash

# Make the script executable: chmod +x scripts/verify_fixes.sh
# Run with: ./scripts/verify_fixes.sh

echo "Running memory management tests..."
rustc memory_standalone_test.rs && ./memory_standalone_test

echo "Running type conversion tests..."
rustc type_test.rs && ./type_test

echo "Running parser tests..."
rustc parser_standalone_test.rs && ./parser_standalone_test

echo "All tests completed! 🎉" 