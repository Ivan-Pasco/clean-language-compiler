#!/bin/bash

# Test runner script for Clean Language compiler
set -e

COMPILER_DIR="/Users/earcandy/Documents/Dev/Clean Language/clean-language-compiler"
TEST_DIR="$COMPILER_DIR/tests"
CLEAN_FILES_DIR="$TEST_DIR/clean_files"
WASM_DIR="$TEST_DIR/wasm"

# Create wasm directory if it doesn't exist
mkdir -p "$WASM_DIR"

echo "🚀 Starting comprehensive Clean Language test compilation and execution"
echo "=================================================="

# Compile all .cln files to .wasm files
echo "📁 Compiling all Clean Language test files..."

success_count=0
fail_count=0
failed_files=()

for cln_file in "$CLEAN_FILES_DIR"/*.cln; do
    if [ -f "$cln_file" ]; then
        filename=$(basename "$cln_file" .cln)
        wasm_file="$WASM_DIR/${filename}.wasm"
        
        echo "  🔨 Compiling: $filename.cln"
        if cargo run --bin clean-language-compiler compile -i "$cln_file" -o "$wasm_file" 2>&1; then
            echo "    ✅ Compilation successful: $filename.cln"
            ((success_count++))
        else
            echo "    ❌ Compilation failed: $filename.cln"
            ((fail_count++))
            failed_files+=("$filename.cln")
        fi
    fi
done

echo ""
echo "📊 Compilation Summary:"
echo "  ✅ Successful: $success_count"
echo "  ❌ Failed: $fail_count"

if [ $fail_count -gt 0 ]; then
    echo "  Failed files:"
    for file in "${failed_files[@]}"; do
        echo "    - $file"
    done
fi

echo ""
echo "=================================================="
echo "🎯 Testing execution of compiled WASM files..."

exec_success_count=0
exec_fail_count=0
exec_failed_files=()

for wasm_file in "$WASM_DIR"/*.wasm; do
    if [ -f "$wasm_file" ]; then
        filename=$(basename "$wasm_file" .wasm)
        
        echo "  🎮 Executing: $filename.wasm"
        if ./target/debug/wasmtime_runner "$wasm_file" 2>&1; then
            echo "    ✅ Execution successful: $filename.wasm"
            ((exec_success_count++))
        else
            echo "    ❌ Execution failed: $filename.wasm"
            ((exec_fail_count++))
            exec_failed_files+=("$filename.wasm")
        fi
    fi
done

echo ""
echo "📊 Execution Summary:"
echo "  ✅ Successful: $exec_success_count"
echo "  ❌ Failed: $exec_fail_count"

if [ $exec_fail_count -gt 0 ]; then
    echo "  Failed files:"
    for file in "${exec_failed_files[@]}"; do
        echo "    - $file"
    done
fi

echo ""
echo "=================================================="
echo "🏁 Overall Summary:"
echo "  📁 Total files processed: $((success_count + fail_count))"
echo "  🔨 Compilation success rate: $success_count/$((success_count + fail_count))"
echo "  🎮 Execution success rate: $exec_success_count/$((exec_success_count + exec_fail_count))"

if [ $fail_count -eq 0 ] && [ $exec_fail_count -eq 0 ]; then
    echo "🎉 All tests passed successfully!"
    exit 0
else
    echo "⚠️  Some tests failed - see details above"
    exit 1
fi