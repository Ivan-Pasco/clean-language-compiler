# Clean Language Implementation Tasks

## Remaining Work to Complete Implementation

### 1. Update Codegen Modules for Simplified Type System

**Priority: High**

Update all codegen modules to remove references to old Value and Type variants and use only the core types from the updated specification.

#### 1.1 Update `src/codegen/memory.rs`
- [ ] Remove references to `Value::Number` (replace with `Value::Float`)
- [ ] Remove all sized type variants (`Value::Byte`, `Value::Unsigned`, `Value::Long`, `Value::ULong`, `Value::Big`, `Value::UBig`)
- [ ] Remove `Value::Null` and `Value::Unit` (replace with appropriate defaults)
- [ ] Update memory allocation functions to handle only core types

#### 1.2 Update `src/codegen/type_manager.rs`
- [ ] Remove references to `Expression::StringConcat` (now handled as `Expression::StringInterpolation`)
- [ ] Remove all old Value variants from type mapping functions
- [ ] Remove `Type::Unit`, `Type::Number`, and other non-core types
- [ ] Update type conversion functions for core types only

#### 1.3 Update `src/codegen/instruction_generator.rs`
- [ ] Remove `Statement::FromTo` and `Statement::ErrorHandler` references
- [ ] Remove `Expression::MatrixOperation` and `Expression::StringConcat` references
- [ ] Remove old Value variant handling in `generate_value()`
- [ ] Fix type casting issues (i64 to i32 conversions)
- [ ] Update `StringPart::Expression` to `StringPart::Interpolation`

#### 1.4 Update `src/codegen/mod.rs`
- [ ] Remove `Type::Unit` references (replace with `Type::Void`)
- [ ] Remove old Statement and Expression variant handling
- [ ] Fix type mapping functions for core types only
- [ ] Update WasmType conversion for simplified type system

#### 1.5 Update `src/types/mod.rs`
- [ ] Remove sized type variants from AST type mapping
- [ ] Update type conversion functions for core types only
- [ ] Remove `Type::Number`, `Type::Byte`, etc.

### 2. Fix Parser Implementation Issues

**Priority: High**

#### 2.1 Add Missing `parse_parameter` Function
- [ ] Add `parse_parameter` function to `src/parser/class_parser.rs`
- [ ] Implement parameter parsing for class constructors
- [ ] Ensure parameter parsing matches function parameter syntax

#### 2.2 Update Grammar Rules
- [ ] Ensure all Rule variants match the grammar file
- [ ] Add missing rules like `function_def`, `block`, etc.
- [ ] Verify indented_block vs block rule usage

### 3. Update Semantic Analyzer

**Priority: Medium**

#### 3.1 Add Support for New Constructs
- [ ] Add semantic analysis for `ApplyBlock` statements
- [ ] Add semantic analysis for `OnError` expressions
- [ ] Add semantic analysis for `StringInterpolation` expressions
- [ ] Update type checking for simplified type system

#### 3.2 Add Asynchronous Programming Support
- [ ] Add semantic analysis for `run` expressions
- [ ] Add semantic analysis for `later` variable declarations
- [ ] Implement async type checking rules

### 4. Add Missing AST Support

**Priority: Medium**

#### 4.1 Implement Missing Expression Types
- [ ] Add proper `StringInterpolation` code generation
- [ ] Add `OnError` expression code generation
- [ ] Add support for async expressions (`run`, `later`)

#### 4.2 Update Statement Handling
- [ ] Complete `ApplyBlock` statement implementation
- [ ] Add proper scoping for apply blocks
- [ ] Implement constant declarations in apply blocks

### 5. Testing and Validation

**Priority: Medium**

#### 5.1 Parser Testing
- [ ] Test parser with `examples/simple_test.cln`
- [ ] Test parser with `examples/hello_world.cln`
- [ ] Test parser with `examples/complex_test.cln`
- [ ] Create additional test files for new syntax

#### 5.2 End-to-End Testing
- [ ] Test complete compilation pipeline
- [ ] Test WASM output generation
- [ ] Test error handling and reporting
- [ ] Test with larger example programs

#### 5.3 Syntax Checker Testing
- [ ] Test syntax-only checking without codegen
- [ ] Validate all new syntax features
- [ ] Test error recovery mechanisms

### 6. Documentation and Examples

**Priority: Low**

#### 6.1 Update Examples
- [ ] Update existing examples to use new syntax
- [ ] Create examples demonstrating apply-blocks
- [ ] Create examples demonstrating string interpolation
- [ ] Create examples demonstrating async programming

#### 6.2 Update Documentation
- [ ] Verify specification matches implementation
- [ ] Update README with new features
- [ ] Add compilation instructions
- [ ] Add usage examples

### 7. Performance and Optimization

**Priority: Low**

#### 7.1 Code Generation Optimization
- [ ] Optimize WASM output for core types
- [ ] Implement efficient string interpolation
- [ ] Optimize matrix operations
- [ ] Implement memory pool management

#### 7.2 Parser Optimization
- [ ] Optimize parsing performance
- [ ] Improve error messages
- [ ] Add better error recovery

## Notes

- Focus on high-priority tasks first to get a working implementation
- The parser and AST changes are mostly complete
- Main work is in updating codegen to match the simplified specification
- Testing should be done incrementally as features are completed

## Current Status

✅ **Completed:**
- Updated grammar to match specification
- Updated AST to use core types only
- Fixed parser implementation for new syntax
- Updated example files to new syntax
- Resolved SourceLocation type conflicts

🔄 **In Progress:**
- Updating codegen modules (many compilation errors remain)

⏳ **Pending:**
- Complete codegen updates
- Add missing parse_parameter function
- End-to-end testing 