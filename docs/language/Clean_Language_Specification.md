# Clean Language Comprehensive Specification

## Table of Contents

1. [Overview](#overview)
2. [Lexical Structure](#lexical-structure)
3. [Type System](#type-system)
4. [Apply-Blocks](#apply-blocks)
5. [Expressions](#expressions)
6. [Statements](#statements)
7. [Functions](#functions)
8. [Control Flow](#control-flow)
9. [Error Handling](#error-handling)
10. [Classes and Objects](#classes-and-objects)
11. [Modules and Imports](#modules-and-imports)
12. [Standard Library](#standard-library)
13. [Memory Management](#memory-management)
14. [Advanced Types](#advanced-types)
15. [Asynchronous Programming](#asynchronous-programming)

## Overview

Clean Language is a modern, type-safe programming language designed to compile to WebAssembly (WASM). It combines the readability of Python with the safety of Rust while being approachable for beginners. The language emphasizes strong static typing, first-class functions, matrix operations, and comprehensive error handling.

### Design Goals
- **Type Safety**: Strong static typing with type inference
- **Simplicity**: Clean, readable syntax without unnecessary complexity
- **Performance**: Efficient compilation to WebAssembly
- **Expressiveness**: First-class support for mathematical operations and data structures
- **Error Handling**: Comprehensive error handling and recovery mechanisms

### File Extension
Clean Language source files use the `.cln` extension.

## Compiler Instructions (Core Implementation Rules)

### 🛠 Clean Language Compiler Instructions (Core Fixes)

These are essential implementation rules that must be followed by the Clean Language compiler:

1. **Functions must be in a `functions:` block**
   - ❌ No standalone `function name(...)` allowed at top level
   - ✅ Use `functions:` for top-level and class functions
   ```clean
   // ❌ Invalid
   function myFunc()
       return 42
   
   // ✅ Valid
   functions:
       integer myFunc()
           return 42
   ```

2. **Helper methods require parentheses**
   - ✅ `x.toString()`
   - ❌ `x.toString`
   ```clean
   value = 42
   text = value.toString()  // ✅ Correct
   ```

3. **Use `Any` for generic types**
   - ✅ `Any identity(Any value) -> Any`
   - Treat any capitalized type name not declared as a concrete type as a generic
   ```clean
   functions:
       Any identity(Any value)
           return value
   ```

4. **Use `functions:` inside `class`**
   - All class methods go inside a `functions:` block
   ```clean
   class MyClass
       integer value
       
       functions:
           void setValue(integer newValue)
               value = newValue
   ```

5. **Drop `Utils` suffix from standard library classes**
   - ✅ Use `Math`, `String`, `Array`, `File` — not `MathUtils`, etc.

6. **Use natural generic container syntax**
   - ✅ `Array<Item>`, `Matrix<Type>`
   - ❌ No angle brackets in user code (`<>`) - these are internal representations

7. **Clean uses `Any` as the single generic placeholder type**
   - It represents a value of any type, determined when the function or class is used
   - No explicit type parameter declarations needed - `Any` is automatically generic

### Implementation Notes
- These rules ensure consistency with Clean's philosophy of simplicity and readability
- The compiler should enforce these patterns and provide helpful error messages when violated
- Generic type resolution happens at compile time based on usage context

## Lexical Structure

### Comments

```clean
// Single line comment

/* 
   Multi-line
   comment
*/
```

### Whitespace and Indentation

Clean Language uses **tab-based indentation** for code structure:

- **Indentation**: Uses tabs only. Each tab represents one block level
- **Spaces**: May be used within expressions for alignment and formatting, but not for indentation
- **Block Structure**: Indentation defines code blocks (no braces `{}`)
- **Whitespace**: Includes spaces, tabs, carriage returns, and newlines

**Example:**
```clean
start()
⇥⇥⇥⇥integer x = 5    // Tab indentation
⇥⇥⇥⇥if x > 0
⇥⇥⇥⇥⇥⇥⇥⇥print("positive")    // Nested tab indentation
⇥⇥⇥⇥else
⇥⇥⇥⇥⇥⇥⇥⇥print("zero or negative")
```

**Indentation Rules:**
- Each indentation level must use exactly one tab character
- Mixing tabs and spaces for indentation is not allowed
- Spaces within expressions are permitted for readability:
  ```clean
  result = function(arg1,  arg2,  arg3)    // Spaces for alignment
  value  = x + y                           // Spaces around operators
  ```

### Identifiers

Identifiers must:
- Start with a letter (`A-Z`, `a-z`)
- Contain only letters, digits, and underscores
- Follow camelCase conventions (e.g. `myVariable`, `calculateSum`)

**Valid Examples:**
```clean
x
count
myVariable
value1
calculateSum
```

**Invalid Examples:**
```clean
1value      // Cannot start with digit
my-var      // Hyphens not allowed
$name       // Special characters not allowed
```

### Keywords

Reserved keywords in Clean Language:

```
and        class       constructor  else        error       false      
for        from        function     if          import      in         
iterate    not         onError      or          print       println    
return     start       step         test        this        to         
true       while       is           returns     description input      
unit       private     constant     functions
```

### Literals

#### Numeric Literals

**Integers:**
```clean
42          // Decimal
-17         // Negative decimal
0xff        // Hexadecimal
0b1010      // Binary
0o777       // Octal
```

**Floating-Point:**
```clean
3.14        // Standard decimal
.5          // Leading zero optional
6.02e23     // Scientific notation
-2.5        // Negative float
```

#### String Literals

**Basic Strings:**
```clean
"Hello, World!"
"Line 1\nLine 2"
""          // Empty string
```

**String Interpolation:**
```clean
name = "World"
greeting = "Hello, {name}!"     // Results in "Hello, World!"

// Simple property access allowed
user = User("Alice", 25)
message = "User {user.name} is {user.age} years old"
```

#### Boolean Literals
```clean
true
false
```

#### Array Literals
```clean
[1, 2, 3, 4]           // Integer array
["a", "b", "c"]        // String array
[]                     // Empty array
[true, false, true]    // Boolean array
```

#### Matrix Literals
```clean
[[1, 2], [3, 4]]                    // 2x2 matrix
[[1, 2, 3], [4, 5, 6], [7, 8, 9]]   // 3x3 matrix
[[]]                                // Empty matrix
```

## Type System

### Core Types

| Type&nbsp;(keyword) | Description | Default Mapping | Literal Examples |
|---------------------|-------------|-----------------|------------------|
| `boolean`  | Logical value (`true` / `false`) | 1 bit | `true`, `false` |
| `integer`  | Whole numbers, signed | Platform optimal (≥32 bits) | `42`, `-17` |
| `float`    | Decimal numbers | Platform optimal (≥64 bits) | `3.14`, `6.02e23` |
| `string`   | UTF-8 text, dynamically sized | — | `"Hello"` |
| `void`     | No value / empty return type | 0 bytes | *(function return only)* |

### Precision Control

Clean Language supports explicit precision control for numeric types using type modifiers:

#### Integer Precision Modifiers

| Type Syntax | WebAssembly Type | Size | Range | Use Case |
|-------------|------------------|------|-------|----------|
| `integer:8`  | i32 (clamped) | 8-bit | -128 to 127 | Small values, memory optimization |
| `integer:16` | i32 (clamped) | 16-bit | -32,768 to 32,767 | Medium values, coordinates |
| `integer:32` | i32 | 32-bit | -2³¹ to 2³¹-1 | Default integer size |
| `integer:64` | i64 | 64-bit | -2⁶³ to 2⁶³-1 | Large numbers, timestamps |

#### Float Precision Modifiers

| Type Syntax | WebAssembly Type | Size | Precision | Use Case |
|-------------|------------------|------|-----------|----------|
| `float:32`  | f32 | 32-bit | IEEE 754 single | Default float, graphics |
| `float:64`  | f64 | 64-bit | IEEE 754 double | High precision, scientific computing |

#### Examples

```clean
// Integer precision examples
integer:8 red = 255              // Color component (0-255)
integer:16 coordinate = 1024     // Screen coordinate
integer:32 count = 1000000       // Default integer (can omit :32)
integer:64 timestamp = 1640995200000  // Unix timestamp in milliseconds

// Float precision examples  
float:32 temperature = 23.5      // Default float (can omit :32)
float:64 preciseValue = 3.141592653589793  // High precision calculation

// Apply-blocks with precision
integer:8:
    red = 255
    green = 128
    blue = 64

float:64:
    pi = 3.141592653589793
    e = 2.718281828459045
```

#### Default Behavior
- `integer` without modifier defaults to `integer:32`
- `float` without modifier defaults to `float:32`
- This maintains backward compatibility with existing code

### Composite & Generic Types

| Type syntax | What it is | Example |
|-------------|------------|---------|
| `Array<Any>`  | Homogeneous resizable list | `Array<integer>`, `[1, 2, 3]` |
| `List<Any>` | Flexible list with behavior properties | `List<string>`, see List Properties below |
| `Matrix<Any>` | 2-D array (array of arrays) | `Matrix<float>`, `[[1.0, 2.0], [3.0, 4.0]]` |
| `pairs<Any,Any>`  | Key-value associative container | `pairs<string, integer>` |
| `Any`         | Generic type parameter | Used in function definitions |

Arrays in Clean are zero-indexed by default (array[0] is the first element).
For readability, you can access elements starting from 1 using:

array.at(index)
This returns the element at position index - 1.

### List Properties - Collection Behavior Modifiers

Clean Language extends the core `List<Any>` type with **property modifiers** that change the list's behavior without requiring separate collection types. This provides a unified, consistent approach to different collection patterns while maintaining type safety and simplicity.

#### Property Syntax

```clean
List<Any> myList = List<Any>()
myList.type = behavior_type
```

Where `behavior_type` defines how the list handles insertions, removals, and access patterns.

#### Supported Properties

**`line` - Queue Behavior (FIFO)**

First-In-First-Out behavior. Elements are added to the back and removed from the front.

```clean
functions:
    void processTaskQueue()
        List<string> tasks = List<string>()
        tasks.type = line
        
        // Add tasks (to back)
        tasks.add("Task 1")
        tasks.add("Task 2") 
        tasks.add("Task 3")
        
        // Process tasks (from front)
        while tasks.size() > 0
            string currentTask = tasks.remove()  // Gets "Task 1", then "Task 2", etc.
            println("Processing: {currentTask}")
```

**Modified Operations**:
- `add(item)` → Adds to the **back** of the list
- `remove()` → Removes from the **front** of the list  
- `peek()` → Views the **front** element without removing
- Standard list operations (`get(index)`, `size()`) remain unchanged

**`pile` - Stack Behavior (LIFO)**

Last-In-First-Out behavior. Elements are added and removed from the same end (top).

```clean
functions:
    void undoSystem()
        List<string> actions = List<string>()
        actions.type = pile
        
        // Perform actions (add to top)
        actions.add("Create file")
        actions.add("Edit text")
        actions.add("Save file")
        
        // Undo actions (remove from top)
        while actions.size() > 0
            string lastAction = actions.remove()  // Gets "Save file", then "Edit text", etc.
            println("Undoing: {lastAction}")
```

**Modified Operations**:
- `add(item)` → Adds to the **top** of the list
- `remove()` → Removes from the **top** of the list
- `peek()` → Views the **top** element without removing
- Standard list operations (`get(index)`, `size()`) remain unchanged

**`unique` - Set Behavior (Uniqueness Constraint)**

Only allows unique elements. Duplicate additions are ignored.

```clean
functions:
    void trackUniqueVisitors()
        List<string> visitors = List<string>()
        visitors.type = unique
        
        // Add visitors (duplicates ignored)
        visitors.add("Alice")    // Added
        visitors.add("Bob")      // Added  
        visitors.add("Alice")    // Ignored (duplicate)
        visitors.add("Charlie")  // Added
        
        println("Unique visitors: {visitors.size()}")  // Prints: 3
        
        if visitors.contains("Alice")
            println("Alice has visited")
```

**Modified Operations**:
- `add(item)` → Adds only if `item` is not already present
- `remove(item)` → Removes the specified item (not index-based)
- `contains(item)` → Optimized for membership testing
- Standard list operations remain available

#### Property Combinations

Properties can be combined for specialized behavior:

```clean
// Unique queue - FIFO with no duplicates
List<string> uniqueQueue = List<string>()
uniqueQueue.type = line
uniqueQueue.type = unique

// Unique stack - LIFO with no duplicates  
List<integer> uniqueStack = List<integer>()
uniqueStack.type = pile
uniqueStack.type = unique
```

#### Performance Characteristics
- `line`: O(1) add, O(1) remove, O(1) peek
- `pile`: O(1) add, O(1) remove, O(1) peek  
- `unique`: O(1) add/contains (hash-based), O(1) remove

#### Advantages

1. **Unified Type System**: Single `List<Any>` type instead of multiple collection types
2. **Consistent API**: All lists share the same base methods
3. **Flexible Behavior**: Properties can be changed at runtime if needed
4. **Type Safety**: Full generic type support with compile-time validation
5. **Simplicity**: Easier to learn and remember than separate collection classes
6. **Interoperability**: All property-modified lists are still `List<Any>` types

### Type Annotations and Variable Declaration

Variables use **type-first** syntax:

```clean
// Basic variable declarations
integer count = 0
float temperature = 23.5
boolean isActive = true
string name = "Alice"

// Uninitialized variables
integer sum
string message
```

### Type Conversion

**Implicit conversions (safe widening):**
- `integer` → `float` (with precision loss warning)
- Same-sign, wider types → OK

**Explicit conversions:**
```clean
value.toInteger   // convert to integer
value.toFloat     // convert to floating-point
value.toString    // convert to string
value.toBoolean   // convert to boolean
```

**Implementation Status:**
- ✅ **Numeric Conversions**: `integer.toFloat`, `float.toInteger`, `integer.toBoolean` fully implemented
- ✅ **Boolean Conversions**: `integer.toBoolean` (0 = false, non-zero = true) implemented
- ⚠️ **String Conversions**: `value.toString` requires runtime functions (not yet implemented)

**Examples:**
```clean
integer num = 42
float numFloat = num.toFloat      // ✅ Works: converts 42 to 42.0
integer piInt = 3.14.toInteger    // ✅ Works: converts 3.14 to 3 (truncated)
boolean flag = 0.toBoolean        // ✅ Works: converts 0 to false
boolean nonZero = 5.toBoolean     // ✅ Works: converts 5 to true
```

## Apply-Blocks

Apply-blocks are a core language feature where `identifier:` applies that identifier to each indented item.

### Function Calls
```clean
println:
    "Hello"
    "World"
// Equivalent to: println("Hello"), println("World")

array.push:
    item1
    item2
    item3
// Equivalent to: array.push(item1), array.push(item2), array.push(item3)
```

### Variable Declarations
```clean
integer:
    count = 0
    maxSize = 100
    currentIndex = -1
// Equivalent to: integer count = 0, integer maxSize = 100, integer currentIndex = -1

string:
    name = "Alice"
    version = "1.0"
// Equivalent to: string name = "Alice", string version = "1.0"
```

### Constants
```clean
constant:
    integer MAX_SIZE = 100
    float PI = 3.14159
    string VERSION = "1.0.0"
```

## Expressions

### Operator Precedence

From highest to lowest precedence:

1. **Primary** - `()`, function calls, method calls, property access
2. **Unary** - `not`, `-` (unary minus)
3. **Multiplicative** - `*`, `/`, `%`
4. **Additive** - `+`, `-`
5. **Comparison** - `<`, `>`, `<=`, `>=`
6. **Equality** - `==`, `!=`, `is`, `not`
7. **Logical AND** - `and`
8. **Logical OR** - `or`
9. **Assignment** - `=`

### Multi-Line Expressions

**Rule**: If an expression spans multiple lines, it must be wrapped in parentheses.

**Parsing Logic**: The expression continues until all parentheses are properly balanced and closed. The parser will consume tokens across multiple lines until the opening parenthesis has its matching closing parenthesis.

**Syntax**:
```clean
// Single line expressions (no parentheses required)
result = a + b + c
value = functionCall(arg1, arg2)

// Multi-line expressions (parentheses required)
result = (a + b + c +
          d + e + f)

complex = (functionCall(arg1, arg2) +
           anotherFunction(arg3) *
           (nested + expression))

calculation = (matrix1 * matrix2 +
               matrix3.transpose() *
               scalar_value)
```

**Application Logic**:
1. **Single Line**: Expressions on a single line do not require parentheses
2. **Multi-Line Detection**: When the parser encounters an expression that continues to the next line, parentheses are mandatory
3. **Balanced Parsing**: The parser tracks parentheses depth and continues reading until:
   - All opening parentheses have matching closing parentheses
   - No unmatched parentheses remain
4. **Nested Support**: Multi-line expressions can contain nested parentheses for sub-expressions
5. **Error Handling**: Unmatched parentheses result in compilation errors with clear error messages

**Examples**:

```clean
// ✅ Valid: Single line, no parentheses needed
total = price + tax + shipping

// ✅ Valid: Multi-line with parentheses
total = (price + tax + 
         shipping + handling)

// ✅ Valid: Complex multi-line expression
result = (calculateBase(width, height) +
          calculateTax(subtotal) +
          (shippingCost * quantity))

// ✅ Valid: Multi-line function call
value = functionCall(
    (arg1 + arg2),
    (arg3 * arg4),
    defaultValue
)

// ❌ Invalid: Multi-line without parentheses
total = price + tax + 
        shipping         // Compilation error

// ❌ Invalid: Unmatched parentheses
result = (a + b + c      // Compilation error: missing closing parenthesis
```

**Benefits**:
- **Clarity**: Explicit parentheses make multi-line expressions unambiguous
- **Consistency**: Clear rules for when parentheses are required vs. optional
- **Readability**: Developers can format complex expressions across multiple lines
- **Error Prevention**: Prevents accidental statement termination in multi-line expressions

### Arithmetic Operators

```clean
a + b       // Addition
a - b       // Subtraction
a * b       // Multiplication
a / b       // Division
a % b       // Modulo
a ^ b       // Exponentiation
```

### Comparison Operators

```clean
a == b      // Equal
a != b      // Not equal
a < b       // Less than
a > b       // Greater than
a <= b      // Less than or equal
a >= b      // Greater than or equal
a is b      // Identity comparison
a not b     // Negated identity comparison
```

### Logical Operators

```clean
a and b     // Logical AND
a or b      // Logical OR
not a       // Logical NOT
```

### Matrix Operations

Clean Language uses **type-based operator overloading** for basic operations and **method calls** for advanced operations:

```clean
// Basic operations (type-based overloading)
A * B       // Matrix multiplication (when A, B are Matrix<T>)
A + B       // Matrix addition (when A, B are Matrix<T>)
A - B       // Matrix subtraction (when A, B are Matrix<T>)
a * b       // Scalar multiplication (when a, b are numbers)

// Advanced operations (methods)
A.transpose()    // Matrix transpose
A.inverse()      // Matrix inverse
A.determinant()  // Matrix determinant
```

### Method Calls and Property Access

```clean
obj.method()            // Method call
obj.property            // Property access
obj.method(arg1, arg2)  // Method with arguments
"string".length         // Property on literal
array.get(0)           // Built-in method
```

### Function Calls

```clean
functionName()                     // No arguments
functionName(arg1)                 // Single argument
functionName(arg1, arg2, arg3)     // Multiple arguments
```

## Statements

### Variable Declaration

```clean
// Type-first variable declarations
integer x = 10
float y = 3.14
string z
boolean flag = true
```

### Assignment

```clean
x = 42              // Simple assignment
arr[0] = value      // Array element assignment
obj.property = val  // Property assignment
```

### Print Statements

Clean Language supports two print syntaxes: simple inline syntax and block syntax with colon.

#### Simple Syntax
The print statement does not require parentheses. Write `print value` for simple cases. Parentheses are optional for grouping expressions.

```clean
print "Hello"           // Print without newline (preferred syntax)
println "Hello"         // Print with newline (preferred syntax)
print variable          // Print variable
println expression      // Print expression result

// Parentheses optional for expression grouping
print (a + b * c)
println (complex_expression)

// Function call syntax also supported (backwards compatibility)
print("Hello")          // Also valid
println("Hello")        // Also valid
print(variable)         // Also valid
```

#### Block Syntax
For multiple values or complex formatting, use the block syntax with colon (consistent with Clean Language's block patterns):

```clean
print:
    "First line"
    variable_name
    (complex + expression)
    result.toString()

println:
    "Header:"
    value1
    value2
    "Footer"
```

The block syntax allows for cleaner formatting when printing multiple values sequentially, maintaining consistency with other Clean Language block constructs like `functions:`, `string:`, etc.

### Return Statement

```clean
return              // Return void
return value        // Return a value
return expression   // Return expression result
```

## Functions

Clean Language uses **functions blocks** for all function declarations. This ensures consistency and organization in code structure.

### The Start Function

Every Clean program begins with a `start()` function within a `functions:` block:

```clean
functions:
    void start()
        print("Hello, World!")
        integer x = 42
        print(x)
```

### Functions Blocks (Required)

**All functions must be declared within a `functions:` block.** This is the only supported syntax for function declarations:

```clean
functions:
    integer add(integer a, integer b)
        return a + b

    integer multiply(integer a, integer b)
        description "Multiplies two integers"
        input
            integer a
            integer b
        return a * b
    
    integer square(integer x)
        return x * x
    
    void printMessage()
        print("Hello World")
```

### Generic Functions with `Any`

Clean Language uses `Any` as the universal generic type. No explicit type parameter declarations are needed:

```clean
functions:
    Any identity(Any value)
        return value
    
    Any getFirst(Array<Any> items)
        return items[0]
    
    void printAny(Any value)
        print(value.toString())

// Usage - type is inferred at compile time
string result = identity("hello")    // Any → string
integer number = identity(42)        // Any → integer
float decimal = identity(3.14)       // Any → float
```

### Function Features

Functions support optional documentation and input blocks:

```clean
functions:
    integer calculate(integer x, integer y)
        description "Calculates something important"
        input
            integer x
            integer y
        return x + y
```

### Method Calls (Require Parentheses)

All method calls must include parentheses, even when no arguments are provided:

```clean
functions:
    void demonstrateMethods()
        integer value = 42
        string text = value.toString()    // ✅ Correct - parentheses required
        integer length = text.length()   // ✅ Correct - parentheses required
        
        // ❌ Invalid - missing parentheses
        // string bad = value.toString
        // integer badLength = text.length
```

### Function Call Syntax

Functions are called using standard syntax:

```clean
functions:
    void start()
        integer result = add(5, 3)
        integer value = multiply(2, 4)
        integer squared = square(7)
        printMessage()
```

### Automatic Return

If a function doesn't use explicit `return`, Clean automatically returns the value of the last expression:

```clean
functions:
    integer addOne(integer x)
        x + 1    // Automatically returned
    
    string greet(string name)
        "Hello, " + name    // Automatically returned
```

## Control Flow

### Conditional Statements

```clean
// Basic if statement
if condition
    // statements

// If-else
if condition
    statements
else
    statements

// If-else if chain
if condition1
    statements
else if condition2
    statements
else
    statements
```

### Loops

#### Iterate Loop (for-each)

```clean
// Iterate over array elements
iterate item in array
    print(item)

// Iterate over string characters
iterate char in "hello"
    print(char)
```

#### Range-based Loops

```clean
iterate name in source [step n]
    // body

// Examples:
iterate i in 1 to 10
    print(i)

iterate k in 10 to 1 step -2
    print(k)                 // 10, 8, 6, 4, 2

iterate ch in "Clean"
    print(ch)

iterate row in matrix
    iterate value in row
        print(value)

iterate idx in 0 to 100 step 5
    print(idx)               // 0, 5, 10, …, 100
```

## Error Handling

### Raising Errors

```clean
functions:
    integer divide()
        input
            integer a
            integer b
        if b == 0
            error("Cannot divide by zero")
        return a / b
```

### Error Handling with onError

```clean
value = riskyCall() onError 0
data = readFile("file") onError print(error)

```

If an expression fails, onError runs the next line or block.
The error is available as error.


## Classes and Objects

### Class Definition

**All class methods must be declared within a `functions:` block:**

```clean
class Point
    integer x
    integer y

    constructor(integer x, integer y)        // Auto-stores matching parameter names

    functions:
    integer distanceFromOrigin()
        return sqrt(x * x + y * y)

        void move(integer dx, integer dy)
        x = x + dx
        y = y + dy
```

### Generic Classes with `Any`

Clean Language uses `Any` for generic class fields and methods:

```clean
class Container
    Any value                  // Any makes class generic

    constructor(Any value)     // Auto-stores to matching field

    functions:
        Any get()
        return value

        void set(Any newValue)
        value = newValue
```

### Inheritance

Clean Language supports single inheritance using the `is` keyword. Child classes inherit all public fields and methods from their parent class.

```clean
class Shape
    string color
    
    constructor(string colorParam)
        color = colorParam          // Implicit context - no 'this' needed
    
    functions:
        string getColor()
            return color            // Direct field access

class Circle is Shape
    float radius
    
    constructor(string colorParam, float radiusParam)
        base(colorParam)            // Call parent constructor with 'base'
        radius = radiusParam        // Implicit context
    
    functions:
        float area()
            return 3.14159 * radius * radius
        
        string getInfo()
            return color + " circle"    // Access inherited field directly
```

#### Inheritance Features

- **Syntax**: Use `class Child is Parent` to inherit from a parent class
- **Base Constructor**: Use `base(args...)` to call the parent constructor
- **Implicit Context**: No need for `this` or `self` - fields are directly accessible
- **Name Safety**: Parameters must have different names than fields to prevent conflicts
- **Method Inheritance**: Child classes inherit all public methods from parent classes
- **Field Inheritance**: Child classes inherit all public fields from parent classes
- **Method Overriding**: Child classes can override parent methods by defining methods with the same name

#### Implicit Context Rules

Clean Language uses implicit context for accessing class fields:

- ✅ `color = colorParam` (field assignment)
- ✅ `return color` (field access)  
- ✅ `radius = radiusParam` (works in child classes too)
- ❌ No `this.color` or `self.color` needed
- ❌ Parameter names cannot match field names (compiler enforced)

This makes code cleaner while maintaining type safety through name conflict prevention.

### Object Creation and Usage

```clean
functions:
    void start()
// Create objects
        Point point = Point(3, 4)
        Circle circle = Circle("red", 5.0)

        // Call methods (parentheses required)
        integer distance = point.distanceFromOrigin()
point.move(1, -2)

// Access properties
        integer xCoord = point.x
        string color = circle.color
```

### Static Methods

You can call class methods directly on the class name if they don't use instance fields:

```clean
class Math
functions:
        float add(float a, float b)
            return a + b
        
        float max(float a, float b)
            return if a > b then a else b

class DatabaseService
    functions:
        boolean connect(string url)
            // implementation that doesn't use instance fields
            return true
        
        User findUser(integer id)
            // implementation that doesn't use instance fields
            return User.loadFromDatabase(id)

// Static method calls - ClassName.method()
functions:
    void start()
        float result = Math.add(5.0, 3.0)
        float maximum = Math.max(10.0, 7.5)
        boolean connected = DatabaseService.connect("mysql://localhost")
        User user = DatabaseService.findUser(42)
```

**Rules for Static Methods:**
- Use `ClassName.method()` syntax for static calls
- Only allowed if the method doesn't access instance fields (`this.field`)
- All methods must be in `functions:` blocks
- Method calls require parentheses: `Math.add()` not `Math.add`
- Ideal for helpers, services, utilities, and database access functions

**Example - Mixed Static and Instance Methods:**
```clean
class User
    string name
    integer age
    
    constructor(string name, integer age)
    
    functions:
        // Instance method - accesses fields
        string getInfo()
            return "User: {name}, Age: {age}"
        
        // Static method - no field access
        boolean isValidAge(integer age)
            return age >= 0 and age <= 150

// Usage
functions:
    void start()
        User user = User("Alice", 25)
        string info = user.getInfo()                    // Instance method call
        boolean valid = User.isValidAge(30)             // Static method call
```

### Design Philosophy: Class-Based Organization

Clean Language encourages organizing all functionality into classes rather than standalone functions. This promotes:

- **Better code organization**: Related functionality is grouped together
- **Namespace management**: No global function name conflicts  
- **Consistent syntax**: All method calls use the same `Class.method()` or `object.method()` pattern
- **Extensibility**: Easy to add related methods to existing classes

**System provides built-in utility classes (without Utils suffix):**
```clean
functions:
    void start()
        // Built-in classes available automatically:
        float result = Math.add(5.0, 3.0)           // Math operations
        integer length = String.length("hello")     // String operations  
        integer size = Array.length([1, 2, 3])     // Array operations
        string data = File.readText("file.txt")    // File operations
        string response = Http.get("api/users")    // HTTP requests

// User code must use classes with functions blocks:
class Calculator
    functions:
        float calculateTax(float amount)
            return Math.multiply(amount, 0.15)
        
        string formatResult(float value)
            return String.concat("Result: ", value.toString())
```

**Exception:** The `start()` function remains as the program entry point within a `functions:` block.

## Standard Library

Clean Language provides built-in utility classes for common operations. All standard library classes follow the compiler instructions:

- All methods are in `functions:` blocks
- Method calls require parentheses
- No `Utils` suffix in class names
- Use `Any` for generic operations

### Math Class

```clean
class Math
    functions:
        // Basic arithmetic
        float add(float a, float b)
        float subtract(float a, float b)
        float multiply(float a, float b)
        float divide(float a, float b)
        
        // Advanced operations
        float sqrt(float x)
        float pow(float base, float exponent)
        float abs(float x)
        
        // Trigonometry
        float sin(float x)
        float cos(float x)
        float tan(float x)
        
        // Constants
        float pi()
        float e()

// Usage
functions:
    void start()
        float result = Math.add(5.0, 3.0)
        float hypotenuse = Math.sqrt(Math.add(Math.pow(3.0, 2.0), Math.pow(4.0, 2.0)))
```

### String Class

```clean
class String
    functions:
        // Basic operations
        integer length(string text)
        string concat(string a, string b)
        string substring(string text, integer start, integer end)
        
        // Case operations
        string toUpperCase(string text)
        string toLowerCase(string text)
        
        // Search operations
        boolean contains(string text, string search)
        integer indexOf(string text, string search)
        
        // Conversion
        string toString(Any value)

// Usage
functions:
    void start()
        integer len = String.length("hello")
        string upper = String.toUpperCase("world")
        string combined = String.concat("Hello, ", "World!")
```

### Array Class

```clean
class Array
    functions:
        // Basic operations
        integer length(Array<Any> array)
        Any get(Array<Any> array, integer index)
        void set(Array<Any> array, integer index, Any value)
        
        // Modification
        void push(Array<Any> array, Any item)
        Any pop(Array<Any> array)
        
        // Search
        boolean contains(Array<Any> array, Any item)
        integer indexOf(Array<Any> array, Any item)

// Usage
functions:
    void start()
        Array<integer> numbers = [1, 2, 3]
        integer size = Array.length(numbers)
        Array.push(numbers, 4)
        integer first = Array.get(numbers, 0)
```

### File Class

The File class makes working with files simple and straightforward. Whether you need to read configuration files, save user data, or process text documents, File has you covered with easy-to-use methods.

```clean
class File
    functions:
        // Reading files
        string read(string path)
            // Reads the entire file content as a single string
            // Perfect for small to medium-sized files
        
        List<string> lines(string path)
            // Reads the file and returns each line as a separate string
            // Great for processing text files line by line
        
        // Writing files
        void write(string path, string content)
            // Writes text to a file, replacing any existing content
            // Creates the file if it doesn't exist
        
        void append(string path, string content)
            // Adds text to the end of an existing file
            // Creates the file if it doesn't exist
        
        // File management
        boolean exists(string path)
            // Checks if a file exists at the given path
            // Returns true if found, false otherwise
        
        void delete(string path)
            // Removes a file from the filesystem
            // Does nothing if the file doesn't exist

// Usage Examples
functions:
    void start()
        // Read a configuration file
        string config = File.read("settings.txt")
        
        // Process a log file line by line
        List<string> logLines = File.lines("app.log")
        
        // Save user data
        File.write("user_data.txt", "John Doe, 25, Engineer")
        
        // Add to a log file
        File.append("activity.log", "User logged in at 2:30 PM")
        
        // Check if a file exists before reading
        if File.exists("backup.txt")
            string backup = File.read("backup.txt")
        
        // Clean up temporary files
        File.delete("temp_data.txt")
```

### Http Class

The Http class makes web requests simple and intuitive. Whether you're fetching data from APIs, submitting forms, or building web applications, Http provides all the essential HTTP methods you need.

```clean
class Http
    functions:
        // GET - Retrieve data from a server
        string get(string url)
            // Sends a GET request to fetch data
            // Returns the response body as a string
        
        // POST - Send new data to a server
        string post(string url, string body)
            // Sends a POST request with data in the body
            // Returns the server's response as a string
        
        // PUT - Update existing data on a server
        string put(string url, string body)
            // Sends a PUT request to update a resource
            // Returns the server's response as a string
        
        // PATCH - Partially update data on a server
        string patch(string url, string body)
            // Sends a PATCH request for partial updates
            // Returns the server's response as a string
        
        // DELETE - Remove data from a server
        string delete(string url)
            // Sends a DELETE request to remove a resource
            // Returns the server's response as a string

// Usage Examples
functions:
    void start()
        // Fetch user data from an API
        string users = Http.get("https://api.example.com/users")
        
        // Create a new user
        string newUser = "{\"name\": \"Alice\", \"email\": \"alice@example.com\"}"
        string response = Http.post("https://api.example.com/users", newUser)
        
        // Update user information
        string updatedUser = "{\"name\": \"Alice Smith\", \"email\": \"alice.smith@example.com\"}"
        Http.put("https://api.example.com/users/123", updatedUser)
        
        // Partially update user (just the email)
        string emailUpdate = "{\"email\": \"newemail@example.com\"}"
        Http.patch("https://api.example.com/users/123", emailUpdate)
        
        // Remove a user
        Http.delete("https://api.example.com/users/123")
        
        // Fetch weather data
        string weather = Http.get("https://api.weather.com/current?city=London")
```

## Modules and Imports

### Visibility Model

**Public by default** - functions and classes are exported unless marked private:

```clean
// All public by default
functions:
    calculateTotal()
        // implementation
    
    formatCurrency()
        // implementation
    
    // Mark private when needed
    private:
        internalHelper()
            // implementation
```

### Importing

```clean
import:
    Math                # whole module
    Math.sqrt           # single symbol
    Utils as U          # module alias
    Json.decode as jd   # symbol alias
```

## Asynchronous Programming
Clean uses start and later for simple asynchronous execution.
start begins a task in the background.
later declares that the result will be available in the future.
The value blocks only when accessed.
Use background to run a task without keeping the result.
You can also mark a function as background to always run it asynchronously and ignore its result.

later data = start fetchData("url")
print "Working..."
print data          # blocks here only

background logAction("login")    # runs and ignores result

function syncCache() background
    sendUpdateToServer()
    clearLocalTemp()
    
syncCache()    # runs in background automatically



## Memory Management
Clean uses Automatic Reference Counting (ARC) for memory management.
Each object tracks how many references point to it, and is automatically freed when no references remain.
A lightweight cycle detector runs periodically to prevent memory leaks in circular structures.
No manual memory handling is needed — memory is released as soon as it's no longer used.