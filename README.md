# Clean Language Compiler

A modern, type-safe programming language that compiles to WebAssembly.

**Author:** Ivan Pasco Lizarraga  
**Website:** https://www.cleanlanguage.dev  
**Date:** 17-07-2025

## Overview

Clean Language is designed to be a simple, expressive, and type-safe programming language that compiles to WebAssembly. It combines the readability of JavaScript with the safety of Rust, while being more approachable for beginners.

## Features

- Strong static typing with type inference
- First-class functions
- Matrix operations
- String interpolation
- Error handling
- Class inheritance with base constructor calls
- WebAssembly compilation target

## Project Status

Clean Language is currently in active development. We have successfully implemented several critical fixes:

- ✅ **Memory Management System**: Complete and verified
- ✅ **Error Handling Framework**: Complete and verified
- ✅ **Parser Implementation**: Complete and verified
- ✅ **Type System**: Complete and verified
- ✅ **Code Generation**: Complete and verified with integration tests

See the [tasks.md](docs/tasks/tasks.md) file for detailed information on project status and upcoming features.

## Verification Status

| Component | Fix Status | Verification Status |
|-----------|------------|---------------------|
| Memory Management | ✅ Complete | ✅ Verified with standalone test |
| Parser Grammar | ✅ Complete | ✅ Verified with standalone test |
| Error Handling | ✅ Complete | ✅ Verified in parser test |
| Type Conversion | ✅ Complete | ✅ Verified with standalone test |
| CallIndirect Fixes | ✅ Complete | ✅ Verified manually |

## Installation

### Prerequisites

- Rust 1.70.0 or later
- Cargo package manager

### Building from Source

```bash
# Clone the repository
git clone https://github.com/your-username/clean-language.git
cd clean-language

# Build the compiler
cargo build --release

# Run the tests
cargo test
```

## Usage

### Compiling a Clean Language Program

```bash
# Compile a Clean Language file to WebAssembly
cargo run -- compile examples/hello_world.clean -o hello_world.wasm

# Run a compiled WebAssembly file
cargo run -- run hello_world.wasm
```

### Using the REPL

```bash
# Start the Clean Language REPL
cargo run -- repl
```

## Examples

### Hello World

```clean
start()
    println("Hello, World!")
```

### Fibonacci Sequence

```clean
function integer fibonacci()
    input integer n
    if n <= 1
        return n
    return fibonacci(n - 1) + fibonacci(n - 2)

start()
    integer n = 10
    integer result = fibonacci(n)
    println("Fibonacci of {n} is {result}")
```

### Class Inheritance

```clean
class Shape
    string color
    
    constructor(string colorParam)
        color = colorParam  // Implicit context - no 'this' needed
    
    functions:
        string getColor()
            return color

class Circle is Shape
    float radius
    
    constructor(string colorParam, float radiusParam)
        base(colorParam)  // Call parent constructor
        radius = radiusParam  // Implicit context
    
    functions:
        float getArea()
            return 3.14159 * radius * radius
```

#### Inheritance Features

- **Class Inheritance**: Use `class Child is Parent` syntax to inherit from a parent class
- **Base Constructor Calls**: Use `base(args...)` in child constructors to call the parent constructor
- **Method Inheritance**: Child classes automatically inherit all public methods from parent classes
- **Field Access**: Child classes can access public fields from parent classes
- **Method Overriding**: Child classes can override parent methods by defining methods with the same name
- **Implicit Context**: No need for `this` or `self` - class fields are directly accessible when no name conflicts exist

## Documentation Structure

The project documentation is organized as follows:

- **Language Reference** (`docs/language/`): Language syntax and semantics
- **Compiler Documentation** (`docs/compiler/`): Compiler architecture and implementation details
- **Tasks and Planning** (`docs/tasks/`): Current tasks, implementation plans, and critical fixes
- **Tools Documentation** (`docs/tools/`): Guide to using the implementation tools

## Running Integration Tests

We have comprehensive integration tests to ensure all components work together correctly:

```bash
# Run all integration tests
cargo test --test integration
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. 