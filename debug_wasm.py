#!/usr/bin/env python3
import subprocess
import sys

# Run the numeric operations test and capture the generated WASM
result = subprocess.run([
    'cargo', 'test', 'stdlib::numeric_ops::tests::test_add', '--', '--nocapture'
], capture_output=True, text=True, cwd='/Users/earcandy/Documents/Dev/Clean Language')

print("STDOUT:")
print(result.stdout)
print("\nSTDERR:")
print(result.stderr)
print(f"\nReturn code: {result.returncode}")