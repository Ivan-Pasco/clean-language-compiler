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
function start()
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

### Composite & Generic Types

| Type syntax | What it is | Example |
|-------------|------------|---------|
| `Array<T>`  | Homogeneous resizable list | `Array<integer>`, `[1, 2, 3]` |
| `Matrix<T>` | 2-D array (array of arrays) | `Matrix<float>`, `[[1.0, 2.0], [3.0, 4.0]]` |
| `pairs<K,V>`  | Key-value associative container | `pairs<string, integer>` |
| `T`         | Generic type parameter | Used in function definitions |

Arrays in Clean are zero-indexed by default (array[0] is the first element).
For readability, you can access elements starting from 1 using:

array.at(index)
This returns the element at position index - 1.

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
value.integer   // convert to integer
value.float     // convert to floating-point
value.string    // convert to string
value.boolean   // convert to boolean
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

> **Note:** Clean Language encourages organizing functionality into classes with static methods rather than standalone functions. See [Class-Based Organization](#design-philosophy-class-based-organization) for the recommended approach.

Clean Language uses a **functions block syntax** for all function declarations. Functions must be declared within a `functions:` block and cannot be declared as standalone statements.

### Function Declaration Syntax

```clean
functions:
    integer add()
        input
            integer a
            integer b
        return a + b

    integer multiply()
        description "Multiplies two integers"
        input
            integer a
            integer b
        return a * b
    
    integer square()
        input integer x
        return x * x
    
    printMessage()
        println("Hello World")
```

**Key Rules:**
- All functions (except `start()`) must be declared within a `functions:` block
- Each function follows standard signature and body format within the block
- Functions can have optional `description` and `input` blocks
- Clean does not support standalone function declarations outside the `functions:` block

**Standalone functions are deprecated:**
- Use built-in system classes (MathUtils, StringUtils, etc.) for utilities
- Organize your application logic in classes with static/instance methods
- Standalone functions are only for internal compiler use

**System provides built-in utility classes:**
```clean
// Built-in classes - available automatically:
result = MathUtils.add(5, 3)
text = StringUtils.toUpper("hello")
count = ArrayUtils.length([1, 2, 3])

// Your application code - must use classes:
class BusinessLogic
    functions:
        integer calculateTotal(integer items)
            return MathUtils.multiply(items, 10)
```

### Function Calls

```clean
result = add(5, 3)
value = multiply(2, 4)
message = square(7)
```

If a function does not use return, Clean automatically returns the value of the last expression in the function body.
Use return for clarity in multi-step or branching logic.
Implicit return is best for short, single-expression functions.

### Generic Functions

```clean
functions:
    T identity()
        input T value
        return value

// Usage
string result = identity("hello")
integer number = identity(42)
```

### Start Function

Every Clean Language program must have a `start()` function:

```clean
function start()
    println("Hello, World!")
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

```clean
class Point
    integer x
    integer y

    constructor(x, y)        // Auto-stores matching parameter names

    integer distanceFromOrigin()
        return sqrt(x * x + y * y)

    move()
        input
            integer dx
            integer dy
        x = x + dx
        y = y + dy
```

### Generic Classes

```clean
class Container
    T value                  // First mention of T makes class generic

    constructor(value)       // Auto-stores to matching field

    T get()
        return value

    set()
        input T newValue
        value = newValue
```

### Inheritance

```clean
class Shape
    string color
    
    constructor(color)
    
    string getColor()
        return color

class Circle is Shape
    float radius
    
    constructor(color, radius)
        super(color)
    
    float area()
        return pi * radius * radius
```

### Object Creation and Usage

```clean
// Create objects
point = Point(3, 4)
circle = Circle("red", 5.0)

// Call methods
distance = point.distanceFromOrigin()
point.move(1, -2)

// Access properties
xCoord = point.x
color = circle.color
```

### Static Methods

You can call class methods directly on the class name if they don't use instance fields:

```clean
class MathUtils
    float add(float a, float b)
        return a + b
    
    float max(float a, float b)
        return if a > b then a else b

class DatabaseService
    boolean connect(string url)
        // implementation that doesn't use instance fields
        return true
    
    User findUser(integer id)
        // implementation that doesn't use instance fields
        return User.loadFromDatabase(id)

// Static method calls - ClassName.method()
result = MathUtils.add(5.0, 3.0)
maximum = MathUtils.max(10.0, 7.5)
connected = DatabaseService.connect("mysql://localhost")
user = DatabaseService.findUser(42)
```

**Rules for Static Methods:**
- Use `ClassName.method()` syntax for static calls
- Only allowed if the method doesn't access instance fields (`this.field`)
- Ideal for helpers, services, utilities, and database access functions
- To access instance fields, call methods on object instances instead

**Example - Mixed Static and Instance Methods:**
```clean
class User
    string name
    integer age
    
    constructor(name, age)
    
    // Instance method - accesses fields
    string getInfo()
        return "User: {name}, Age: {age}"
    
    // Static method - no field access
    boolean isValidAge(integer age)
        return age >= 0 and age <= 150

// Usage
user = User("Alice", 25)
info = user.getInfo()                    // Instance method call
valid = User.isValidAge(30)              // Static method call
```

### Design Philosophy: Class-Based Organization

Clean Language encourages organizing all functionality into classes rather than standalone functions. This promotes:

- **Better code organization**: Related functionality is grouped together
- **Namespace management**: No global function name conflicts  
- **Consistent syntax**: All method calls use the same `Class.method()` or `object.method()` pattern
- **Extensibility**: Easy to add related methods to existing classes

**System provides built-in utility classes:**
```clean
// Built-in classes available automatically:
result = MathUtils.add(5.0, 3.0)           // Math operations
length = StringUtils.length("hello")        // String operations  
size = ArrayUtils.length([1, 2, 3])        // Array operations
data = FileUtils.readText("file.txt")      // File operations
response = HttpUtils.get("api/users")      // HTTP requests

// User code must use classes:
class Calculator
    functions:
        float calculateTax(float amount)
            return MathUtils.multiply(amount, 0.15)
        
        string formatResult(float value)
            return StringUtils.concat("Result: ", value)
```

**Exception:** The `start()` function remains as the program entry point.

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

## Standard Library

### StringUtils Class

```clean
StringUtils.length(str)                        // Get string length
StringUtils.compare(s1, s2)                    // Compare strings (-1, 0, 1)
StringUtils.substring(str, start, len)         // Extract substring
StringUtils.toUpper(str)                       // Convert to uppercase
StringUtils.toLower(str)                       // Convert to lowercase
StringUtils.trim(str)                          // Remove whitespace
StringUtils.split(str, delimiter)              // Split into array
StringUtils.concat(s1, s2)                     // Concatenate strings
StringUtils.contains(str, substring)           // Check if contains substring
StringUtils.startsWith(str, prefix)            // Check if starts with prefix
StringUtils.endsWith(str, suffix)              // Check if ends with suffix
```

**Note:** String concatenation uses `+` only when both operands are strings.

### ArrayUtils Class

```clean
ArrayUtils.length(arr)                       // Get array length
ArrayUtils.get(arr, index)                   // Get element at index
ArrayUtils.set(arr, index, value)            // Set element at index

ArrayUtils.push(arr, value)                  // Add element to end
ArrayUtils.pop(arr)                          // Remove and return last element
ArrayUtils.insert(arr, index, value)         // Insert element at index
ArrayUtils.remove(arr, index)                // Remove element at index

ArrayUtils.contains(arr, value)              // Check if array contains value
ArrayUtils.indexOf(arr, value)               // Get index of first occurrence
ArrayUtils.lastIndexOf(arr, value)           // Get index of last occurrence

ArrayUtils.sort(arr)                         // Sort array in place
ArrayUtils.reverse(arr)                      // Reverse array in place
ArrayUtils.slice(arr, start, end)            // Get subarray
ArrayUtils.join(arr, separator)              // Join elements into string
```

### MathUtils Class

```clean
MathUtils.add(a, b)                // Addition
MathUtils.subtract(a, b)           // Subtraction  
MathUtils.multiply(a, b)           // Multiplication
MathUtils.divide(a, b)             // Division
MathUtils.modulo(a, b)             // Modulo

MathUtils.sqrt(x)                  // Square root
MathUtils.pow(x, y)                // Power
MathUtils.abs(x)                   // Absolute value
MathUtils.floor(x)                 // Floor
MathUtils.ceil(x)                  // Ceiling
MathUtils.round(x)                 // Round to nearest integer

MathUtils.sin(x)                   // Sine
MathUtils.cos(x)                   // Cosine
MathUtils.tan(x)                   // Tangent
MathUtils.log(x)                   // Natural logarithm
MathUtils.exp(x)                   // e^x

MathUtils.min(a, b)                // Minimum of two numbers
MathUtils.max(a, b)                // Maximum of two numbers
MathUtils.clamp(value, min, max)   // Clamp value between min and max

MathUtils.PI                       // Pi constant
MathUtils.E                        // Euler's number
```

### Matrix Module

```clean
matrix.create(rows, cols, value)  // Create matrix filled with value
matrix.identity(size)             // Create identity matrix

// Basic operations (type-based overloading)
A * B          // Matrix multiplication
A + B          // Matrix addition
A - B          // Matrix subtraction

// Advanced operations (methods)
A.transpose()  // Matrix transpose
A.inverse()    // Matrix inverse
A.determinant() // Matrix determinant
A.get(row, col) // Get element
A.set(row, col, value) // Set element
A.rows         // Number of rows
A.cols         // Number of columns
A.size         // Number of elements
```

### Memory Module

```clean
allocate(bytes)                    // Allocate memory block
release(pointer)                   // Deallocate memory block
copyBytes(from, to, bytes)         // Copy memory
fillBytes(pointer, value, bytes)   // Fill memory with value
memoryStats()                      // Get memory usage statistics
```

## Memory Management

### Allocation Strategy

Clean Language uses **automatic reference counting (ARC)** with cycle detection:

- **Reference Counting**: Objects automatically deallocated when reference count reaches zero
- **Cycle Detection**: Periodic sweep to handle circular references
- **Memory Pools**: Size-segregated pools (8B, 16B, 32B, ...) to minimize fragmentation
- **Bounds Checking**: All array and matrix accesses are bounds-checked
- **Guard Pages**: Memory protection with <15% overhead on 64-bit systems

### Memory Layout

```
WebAssembly Linear Memory Layout:
┌─────────────────┬─────────────────┬─────────────────┬─────────────────┐
│  Stack Space    │   Heap Space    │  String Pool    │  Static Data    │
│  (grows down)   │  (grows up)     │                 │                 │
└─────────────────┴─────────────────┴─────────────────┴─────────────────┘
```

### Memory Safety Features

- **Bounds Checking**: All array and matrix accesses are bounds-checked
- **Type Safety**: Strong typing prevents memory corruption
- **Null Safety**: Nullable references must be declared with `?T`
- **Automatic Cleanup**: Resources are automatically cleaned up
- **Leak Detection**: Debug builds track and report memory leaks

## Advanced Types

For cases requiring specific memory layouts or performance characteristics:

### Sized Integer Types
```clean
integer:8     // 8-bit signed integer (-128 to 127)
integer:8u    // 8-bit unsigned integer (0 to 255)
integer:16    // 16-bit signed integer
integer:16u   // 16-bit unsigned integer
integer:32    // 32-bit signed integer
integer:64    // 64-bit signed integer
```

### Sized Float Types
```clean
float:32      // 32-bit IEEE-754 floating point
float:64      // 64-bit IEEE-754 floating point (default)
```

### Usage Examples
```clean
// Graphics/byte manipulation
integer:8u pixelValue = 255

// Large numbers
integer:64 bigNumber = 123456789123456789

// Memory-constrained 3D graphics
float:32 position = 1.5
```

The compiler maps default `integer` and `float` types to optimal sizes for the target platform.

## Asynchronous Programming

Clean uses two keywords for simple asynchronous operations:

### Keywords

* **`run`** — starts an operation in the background
* **`later`** — declares a variable that will be filled when ready

### Basic Usage

```clean
later result = run fetchData("url")
print result   // blocks if not ready
```

### Behavior

* The `run` function begins immediately.
* Accessing the `later` variable blocks until complete.
* No `await` or callbacks are needed.
* Functions do not need to be marked `async`.

### Optional Short Form

```clean
result = run compute()   // treated as later result
```

Use `later` to make it explicit. Clean waits only when you read the value.

##END OF FUNCTIONAL SPECIFICATION