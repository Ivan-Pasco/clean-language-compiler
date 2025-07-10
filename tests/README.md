# Clean Language Tests Organization

This directory contains all test files for the Clean Language compiler, organized for better maintainability and clarity.

## Directory Structure

### Test Source Files (Rust)
- `*.rs` - Main test files containing test cases
- `test_utils.rs` - Shared utilities for testing
- `parser_tests/` - Parser-specific tests

### Test Input Files
- `test_inputs/` - **Official test inputs** used by the test suite
  - `arithmetic.cln` - Arithmetic operations test
  - `function.cln` - Function definition test
  - `hello_world.cln` - Basic hello world test
  - `matrix.cln` - Matrix operations test
  - `test_multiple_functions.cln` - Multiple function definitions
  - `test_return_types.cln` - Return type validation
  - `test_simple.cln` - Simple program test

### Test Artifacts
- `clean_files/` - **Moved test files** (.clean and .cln files previously scattered in root)
- `wasm/` - **Generated WebAssembly files** (.wasm files for testing output)

## Test Categories

### Library Tests (68/68 passing)
- Core functionality tests located in `src/stdlib/*/tests.rs`
- String operations, numeric operations, memory management
- Array operations, type conversions, error handling

### Integration Tests
- `integration_tests.rs` - End-to-end compilation tests
- `basic_examples_test.rs` - Basic program compilation tests
- `compiler_tests.rs` - Compiler functionality tests

### Component Tests
- `parser_tests/` - Parser and grammar tests
- `semantic_tests.rs` - Semantic analysis tests
- `codegen_tests.rs` - Code generation tests
- `type_conversion_tests.rs` - Type system tests

## Usage

### Running Tests
```bash
# Run all tests
cargo test

# Run specific test categories
cargo test --test integration_tests
cargo test --test basic_examples_test
cargo test --lib  # Library tests only
```

### Test File Organization
- **Keep** official test inputs in `test_inputs/`
- **Archive** development test files in `clean_files/`
- **Store** generated WASM files in `wasm/`
- **Remove** temporary/debug files regularly

## Test Status
- **Library Tests**: 68/68 passing (100% success rate)
- **Integration Tests**: Some advanced features pending
- **Basic Examples**: 3/3 passing (arithmetic, functions, matrix)
- **WASM Output**: Valid, parseable modules generated

## File Naming Convention
- `.cln` files - Clean Language source files
- `.clean` files - Alternative Clean Language source files
- `.wasm` files - WebAssembly output files
- `*_test.rs` - Rust test files