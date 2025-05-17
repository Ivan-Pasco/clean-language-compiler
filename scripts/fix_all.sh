#!/bin/bash

# Make the script executable: chmod +x scripts/fix_all.sh
# Run with: ./scripts/fix_all.sh

echo "Running CallIndirect fixes..."
./scripts/fix_call_indirect.sh

echo "Running WasmType fixes..."
./scripts/fix_wasm_types.sh

echo "Running error method call fixes..."
./scripts/fix_errors.sh

echo "All fixes applied! 🎉" 