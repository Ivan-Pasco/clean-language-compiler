# Clean Language Compiler Scripts

This directory contains utility scripts for fixing and testing the Clean Language compiler.

## Fix Scripts

- **fix_all.sh**: Master script that runs all fix scripts in the correct order
- **fix_call_indirect.sh**: Fixes CallIndirect field names in array operations
- **fix_errors.sh**: Updates error method calls to use the new signatures with all required parameters
- **fix_wasm_types.sh**: Updates tuple usage to use the type conversion helpers

## Test Scripts

- **verify_fixes.sh**: Runs standalone tests to verify that fixes are working correctly

## Usage

All scripts should be made executable before use:

```bash
chmod +x scripts/*.sh
```

### Running All Fixes

```bash
./scripts/fix_all.sh
```

### Verifying Fixes

```bash
./scripts/verify_fixes.sh
```

### Running Individual Fixes

```bash
# Fix CallIndirect field names
./scripts/fix_call_indirect.sh

# Fix error method calls
./scripts/fix_errors.sh

# Fix WasmType usage
./scripts/fix_wasm_types.sh
```

## Note

These scripts interact with the source code directly, making automated changes. Always ensure you have a backup or are using version control before running these scripts to avoid losing important changes. 