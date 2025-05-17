# Clean Language Parser Tests

This directory contains tests for the Clean Language parser component.

## Standalone Files

- `minimal_parser_test.rs` - A minimal standalone test for the parser, focusing on simple grammar rules and method call parsing
- `minimal_parser_test_cargo.toml` - Cargo file for the minimal test

## Parser Test Project

The `parser_test` directory contains a more comprehensive test suite for the parser:

- Tests all major language constructs
- Validates the fixed left recursion in method calls
- Ensures correct parsing of expressions, statements, and declarations

### Running the Tests

Navigate to the `parser_test` directory and run:

```
cargo run
```

### Key Features Tested

1. Variable declarations and printing
2. Method calls with fixed left recursion
3. Function definitions with setup blocks
4. Complex expressions with operators
5. Array and matrix literals
6. Control flow constructs
7. Start function
8. Generic types
9. String interpolation 