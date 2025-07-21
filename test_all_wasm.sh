#!/bin/bash

# Comprehensive test script for Clean Language WASM compilation and execution

CLEAN_DIR="tests/clean_files"
WASM_DIR="tests/wasm"
SUCCESSES=0
FAILURES=0

echo "=== Clean Language WASM Test Suite ==="
echo "Compiling and testing all .cln files..."
echo

# Create wasm directory if it doesn't exist
mkdir -p "$WASM_DIR"

# Find all .cln files and test them
for clean_file in "$CLEAN_DIR"/*.cln; do
    if [ -f "$clean_file" ]; then
        filename=$(basename "$clean_file" .cln)
        wasm_file="$WASM_DIR/${filename}_test.wasm"
        
        echo "=== Testing $filename ==="
        
        # Compile
        echo "  Compiling: $clean_file -> $wasm_file"
        if cargo run --bin clean-language-compiler compile -i "$clean_file" -o "$wasm_file" 2>/dev/null; then
            echo "  ✓ Compilation successful"
            
            # Test execution
            echo "  Testing execution..."
            if timeout 5s node test_runner.js "$wasm_file" 2>/dev/null | grep -q "PRINT\|Start function result"; then
                echo "  ✓ Execution successful"
                echo "  Output:"
                timeout 5s node test_runner.js "$wasm_file" 2>/dev/null | grep "PRINT\|Start function result" | sed 's/^/    /'
                SUCCESSES=$((SUCCESSES + 1))
            else
                echo "  ✗ Execution failed"
                FAILURES=$((FAILURES + 1))
            fi
        else
            echo "  ✗ Compilation failed"
            FAILURES=$((FAILURES + 1))
        fi
        echo
    fi
done

echo "=== Summary ==="
echo "Successful tests: $SUCCESSES"
echo "Failed tests: $FAILURES"
echo "Total tests: $((SUCCESSES + FAILURES))"

if [ $FAILURES -eq 0 ]; then
    echo "🎉 All tests passed!"
    exit 0
else
    echo "⚠️  Some tests failed"
    exit 1
fi