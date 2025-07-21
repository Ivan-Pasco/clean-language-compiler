# Clean Language Test Suite

This directory contains a comprehensive set of test files (`.cln`) that validate all aspects of the Clean Language compiler. These tests are organized sequentially and cover the complete language specification.

## Test File Structure

The test files are numbered and organized by feature category:

### Basic Syntax Tests (01-05)
- **01_hello_world.cln** - Basic program structure and print statements
- **02_variables_basic.cln** - Variable declarations for all primitive types
- **03_arithmetic_operations.cln** - Mathematical operations (+, -, *, /, %, ^)
- **04_comparison_operations.cln** - Comparison operators (==, !=, <, >, <=, >=, is, not)
- **05_logical_operations.cln** - Logical operators (and, or, !)

### Type System Tests (06-09)
- **06_type_conversions.cln** - Explicit type conversions between primitives
- **07_lists_basic.cln** - List creation, access, modification, and methods
- **08_matrices.cln** - Matrix operations and multi-dimensional arrays
- **09_type_inference.cln** - Automatic type inference with `auto` keyword

### Function Tests (10-13)
- **10_functions_basic.cln** - Function definitions, parameters, and return values
- **11_functions_overloading.cln** - Function overloading with different parameter types
- **12_functions_recursion.cln** - Recursive function calls (factorial, fibonacci)
- **13_functions_generics.cln** - Generic functions with type parameters

### Class and Inheritance Tests (14-16)
- **14_classes_basic.cln** - Class definitions, constructors, and methods
- **15_classes_inheritance.cln** - Class inheritance with `base()` constructor calls
- **16_classes_polymorphism.cln** - Method overriding and polymorphic behavior

### Control Flow and Async Tests (17-20)
- **17_control_flow_if.cln** - If-else statements and nested conditions
- **18_control_flow_loops.cln** - For loops, while loops, break, and continue
- **19_async_basic.cln** - Basic async/await operations with functions
- **20_async_parallel.cln** - Parallel async execution with Promise.all

### Error Handling Tests (21-23)
- **21_error_handling_try_catch.cln** - Try-catch blocks and custom errors
- **22_error_handling_onerror.cln** - OnError syntax with fallback values
- **23_error_handling_async.cln** - Error handling in async contexts

### Advanced Features Tests (24-28)
- **24_memory_management.cln** - Memory allocation, usage tracking, and cleanup
- **25_stdlib_functions.cln** - Standard library math, string, and list functions
- **26_io_operations.cln** - File I/O and console input/output operations
- **27_http_networking.cln** - HTTP requests and networking capabilities
- **28_complex_example.cln** - Comprehensive example combining multiple features

## Usage

These test files serve multiple purposes:

1. **Compiler Testing** - Validate that the Clean Language compiler correctly parses and compiles all language features
2. **Regression Testing** - Ensure that changes to the compiler don't break existing functionality
3. **Documentation** - Serve as examples of proper Clean Language syntax and usage
4. **Feature Validation** - Verify that all features described in the Language Specification are implemented

## Running Tests

To use these test files with the Clean Language compiler:

```bash
# Compile individual test
cargo run --bin clean-language-compiler compile -i tests/clean_files/01_hello_world.cln -o output.wasm

# Run parser tests
cargo run --bin clean-language-compiler parse -i tests/clean_files/

# Debug with AST display
cargo run --bin clean-language-compiler debug -i tests/clean_files/28_complex_example.cln --show-ast
```

## Test Categories Coverage

- ✅ **Basic Syntax** - Variables, operators, expressions
- ✅ **Type System** - All primitive types, collections, inference
- ✅ **Functions** - Declaration, overloading, recursion, generics
- ✅ **Classes** - OOP features, inheritance, polymorphism
- ✅ **Control Flow** - Conditionals, loops, async operations
- ✅ **Error Handling** - Try-catch, onError, async errors
- ✅ **Standard Library** - Built-in functions and utilities
- ✅ **I/O Operations** - File and console operations
- ✅ **Networking** - HTTP client functionality
- ✅ **Memory Management** - Allocation and cleanup

## Maintenance

When adding new language features:

1. Create a new test file following the numbering convention
2. Include comprehensive examples of the new feature
3. Add error cases and edge conditions
4. Update this README with the new test file
5. Ensure the test covers all aspects mentioned in the Language Specification

## Notes

- All test files use the `.cln` extension as specified in the language standard
- Tests are designed to be self-contained and runnable independently
- Comments in test files explain expected outputs and behavior
- Test files follow Clean Language coding conventions and style guidelines