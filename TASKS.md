# Clean Language Compiler Tasks

## Critical Missing Features ❌

### 1. Apply-Blocks Implementation - NEEDS IMPLEMENTATION (HIGH PRIORITY)
**Current Status**: We use `:` syntax but with dashes, specification requires direct indentation

**What's Working (But Wrong):**
- `functions:` - function block declarations ✅ 
- `string:`, `integer:` - type declarations with `- item = value` syntax ❌ (uses dashes)
- `input:`, `description:` - function setup blocks ✅

**Specification Requirements (Need to Implement):**
- [ ] **Variable Apply-Blocks**: `integer: count = 0, maxSize = 100` (direct indentation, no dashes)
- [ ] **Function Apply-Blocks**: `println: "Hello", "World"` (direct indentation, no dashes)  
- [ ] **Constants Apply-Blocks**: `constant: integer MAX_SIZE = 100` (direct indentation, no dashes)
- [ ] **Method Apply-Blocks**: `array.push: item1, item2, item3` (direct indentation, no dashes)

**Required Work:**
- [ ] **Update Grammar**: Remove dash requirement, implement direct indentation parsing
- [ ] **Update Parser**: Handle direct indented items without dash prefix
- [ ] **Update Semantic Analysis**: Process apply-block expansion correctly
- [ ] **Update Examples**: Convert all `- item = value` to `item = value`
- [ ] **Backward Compatibility**: Decide whether to support both or migrate completely

**Migration Example:**
```clean
// Current (Wrong):
string:
    - message = "Hello"
    - name = "World"

// Specification (Correct):
string:
    message = "Hello"
    name = "World"
```

### 2. Multi-Line Expression Support (HIGH PRIORITY)
- [ ] **Parentheses Requirement**: Enforce parentheses for multi-line expressions
- [ ] **Balanced Parsing**: Track parentheses depth across lines
- [ ] **Error Messages**: Clear errors for missing parentheses in multi-line contexts
- [ ] **Grammar Updates**: Update parser to handle multi-line expression rules

### 3. Advanced Type System (MEDIUM PRIORITY)
- [ ] **Sized Types**: `integer:8`, `integer:16`, `integer:32`, `integer:64`, `float:32`, `float:64`
- [ ] **Type Conversions**: `.integer`, `.float`, `.string`, `.boolean` conversion methods
- [ ] **Generic Type Parameters**: `T` in class and function definitions
- [ ] **Composite Types**: Full `pairs<K,V>` implementation
- [ ] **Type Inference**: Improve type inference capabilities

### 4. Functions Block Syntax (MEDIUM PRIORITY)
- [x] **Functions Block Working**: `functions:` syntax is implemented and working
- [ ] **Deprecate Standalone Functions**: Remove individual function declarations completely
- [ ] **Migration Path**: Update existing code to use functions blocks exclusively
- [ ] **Error Messages**: Guide users to use functions blocks instead of standalone functions

### 5. Asynchronous Programming (LOW PRIORITY)
- [ ] **`run` Keyword**: Background operation execution
- [ ] **`later` Variables**: Deferred value assignment
- [ ] **Async Semantics**: Non-blocking execution model
- [ ] **WebAssembly Integration**: Async support in WASM output

## Major Gaps to Address 🔧

### 1. Grammar Specification Alignment
**Issue**: Current parser grammar doesn't fully match specification requirements

**Required Work**:
- [ ] **Apply-Block Decision**: Decide whether to change implementation or specification
- [ ] Update tab-based indentation enforcement
- [ ] Add multi-line expression parentheses rules
- [ ] Update function declaration grammar to require functions blocks
- [ ] Add sized type syntax support

### 2. Standard Library Completion
**Issue**: Built-in classes need full method implementation

**Required Work**:
- [ ] **StringUtils**: Complete all specification methods (split, trim, startsWith, endsWith, etc.)
- [ ] **ArrayUtils**: Complete all specification methods (slice, join, sort, reverse, etc.)
- [ ] **MathUtils**: Add missing methods (sin, cos, tan, log, exp, clamp, etc.)
- [ ] **Matrix Operations**: Complete matrix manipulation library
- [ ] **Type-based Operator Overloading**: Implement for matrix operations

### 3. Memory Management Implementation
**Issue**: Current memory management is basic, specification requires ARC

**Required Work**:
- [ ] **Automatic Reference Counting**: Implement ARC for object lifecycle
- [ ] **Cycle Detection**: Periodic sweep for circular references  
- [ ] **Memory Pools**: Size-segregated pools for allocation efficiency
- [ ] **Bounds Checking**: Comprehensive array/matrix bounds validation
- [ ] **Guard Pages**: Memory protection implementation

### 4. Error Handling Enhancement
**Issue**: Current onError is basic, needs comprehensive error model

**Required Work**:
- [ ] **Error Variable Access**: Implement `error` variable in onError blocks
- [ ] **Error Propagation**: Proper error bubbling through call stack
- [ ] **Error Types**: Structured error objects with codes and messages
- [ ] **Block Error Handlers**: `onError:` block syntax (not just expressions)
- [ ] **Exception Throwing**: `error("message")` statement implementation

## Immediate Action Items 🚀

### Phase 1: Critical Parsing Features (1-2 weeks)
1. **Implement Specification-Compliant Apply-Blocks**
   - Update grammar to parse direct indentation without dashes
   - Modify parser to handle `identifier: indented_item, indented_item` syntax
   - Update semantic analysis to expand apply-blocks to individual statements
   - Convert existing examples from dash syntax to direct indentation
   - Add comprehensive error messages for malformed apply-blocks

2. **Multi-Line Expression Support**
   - Enforce parentheses requirement for multi-line expressions
   - Update expression parser for balanced parentheses tracking
   - Add clear error messages for violations

### Phase 2: Type System Enhancement (2-3 weeks)
1. **Sized Types Implementation**
   - Add grammar support for `integer:32`, `float:64` syntax
   - Update type system to handle sized variants
   - Implement type conversion methods
   - Update code generation for sized types

2. **Functions Block Migration**
   - Ensure functions blocks work correctly
   - Deprecate standalone function syntax completely
   - Update all existing code examples
   - Add migration error messages

### Phase 3: Standard Library Completion (2-3 weeks)
1. **Built-in Class Methods**
   - Complete StringUtils, ArrayUtils, MathUtils implementations
   - Add comprehensive test coverage
   - Update semantic analysis for all methods
   - Update code generation for missing methods

2. **Advanced Features**
   - Implement memory management foundation
   - Enhanced error handling with error variables
   - Basic async programming support

## Testing Strategy 📋

### Specification Compliance Tests
- [ ] **Apply-Block Test Suite**: Test current working syntax thoroughly
- [ ] **Multi-Line Expression Tests**: Parentheses enforcement validation
- [ ] **Sized Type Tests**: All size variants and conversions
- [ ] **Standard Library Tests**: Every built-in method tested
- [ ] **Memory Management Tests**: ARC and cycle detection validation
- [ ] **Error Handling Tests**: Comprehensive error scenarios

### Integration Tests
- [ ] **Full Language Examples**: Complex programs using all features
- [ ] **Performance Tests**: Memory and execution performance validation
- [ ] **WebAssembly Output Tests**: Verify WASM compliance and execution

## Success Criteria 🎯

1. **Practical Specification Compliance**: All useful features implemented (may differ from written spec)
2. **Zero Compilation Failures**: All working examples continue to work
3. **Comprehensive Test Coverage**: >95% code coverage with working syntax tests
4. **Performance Targets**: Efficient memory usage and execution speed
5. **Clear Error Messages**: Helpful compilation errors guiding users to correct syntax

## Notes 📝

- **Apply-blocks are actually working** with `- item = value` syntax - specification may be outdated
- Functions blocks (`functions:`) are working well
- Current syntax is practical and readable
- **Recommendation**: Update specification to match working implementation rather than breaking working code 