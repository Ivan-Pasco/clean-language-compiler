# Compiler Fix Utilities

This directory contains utility scripts used to automatically fix common issues in the codebase.

## Available Utilities

### Error Method Call Fixer (`error_fix.rs`)
- **Purpose**: Updates error method calls to use the new three-parameter signatures
- **Usage**: `cargo run --bin utils/error_fix -- <filepath>`
- **What it does**:
  - Fixes `runtime_error`, `validation_error`, `parse_error`, and `codegen_error` calls
  - Updates `with_help` and `with_location` calls to use option-based methods

### CallIndirect Fixer (`call_indirect_fix.rs`)
- **Purpose**: Updates CallIndirect instructions to use the correct field names
- **Usage**: `cargo run --bin utils/call_indirect_fix`
- **What it does**:
  - Automatically fixes CallIndirect field names (changes `ty_idx` to `ty` and `table_idx` to `table`)

### WasmType Usage Fixer (`wasm_type_fix.rs`)
- **Purpose**: Updates direct tuple usage to use the type conversion helpers
- **Usage**: `cargo run --bin utils/wasm_type_fix -- <filepath>`
- **What it does**:
  - Adds the necessary import for type conversion helpers if needed
  - Replaces tuple patterns like `(0, ValType::I32)` with `to_tuple(WasmType::I32)`

## How to Use

These utilities are designed to be run from the command line using Cargo. To fix multiple files, you can use shell loops or scripts.

Example of fixing multiple files with the error fixer:

```sh
for file in src/codegen/mod.rs src/validation/mod.rs src/stdlib/mod.rs; do
  cargo run --bin utils/error_fix -- $file
done
```

## Notes

- These utilities modify files in-place, so make sure to have backups or use version control.
- Some complex patterns might need manual fixing if the utilities don't catch them.
- For more details on the fix utilities, see `docs/tools/implementation-tools.md`. 