# Clean Language Comprehensive Specification

## Table of Contents

1. [Overview](#overview)
2. [Lexical Structure](#lexical-structure)
3. [Type System](#type-system)
4. [Apply-Blocks](#apply-blocks)
5. [Expressions](#expressions)
6. [Statements](#statements)
7. [Functions](#functions)
8. [Testing](#testing)
9. [Control Flow](#control-flow)
10. [Error Handling](#error-handling)
11. [Classes and Objects](#classes-and-objects)
12. [Modules and Imports](#modules-and-imports)
13. [Package Management](#package-management)
14. [Standard Library](#standard-library)
15. [Memory Management](#memory-management)
16. [Advanced Types](#advanced-types)
17. [Asynchronous Programming](#asynchronous-programming)

## Overview

Clean Language is a modern, type-safe programming language designed to compile to WebAssembly (WASM). It combines the readability of Python with the safety of Rust while being approachable for beginners. The language emphasizes strong static typing, first-class functions, matrix operations, and comprehensive error handling.

### Design Goals
- **Type Safety**: Strong static typing with type inference
- **Simplicity**: Clean, readable syntax without unnecessary complexity
- **Performance**: Efficient compilation to WebAssembly
- **Expressiveness**: First-class support for mathematical operations and data structures
- **Error Handling**: Comprehensive error handling and recovery mechanisms
- **Developer Experience**: Default parameter values for cleaner APIs and optional configuration

### File Extension
Clean Language source files use the `.cln` extension.

## Compiler Instructions (Core Implementation Rules)

### 🛠 Clean Language Compiler Instructions (Core Fixes)

These are essential implementation rules that must be followed by the Clean Language compiler:

1. **Functions must be in a `functions:` block (except start())**
   - ❌ No standalone `function name(...)` allowed at top level
   - ✅ Use `functions:` for top-level and class functions
   - ✅ Exception: `start()` can be standalone
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

3. **Use `any` for generic types**
   - ✅ `any identity(any value) -> any`
   - Treat any capitalized type name not declared as a concrete type as a generic
   ```clean
   functions:
       any identity(any value)
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
   - ✅ Use `Math`, `String`, `List`, `File` — not `MathUtils`, etc.

6. **Use natural generic container syntax**
   - ✅ `list<item>`, `matrix<type>`
   - ❌ No angle brackets in user code (`<>`) - these are internal representations

7. **Clean uses `any` as the single generic placeholder type**
   - It represents a value of any type, determined when the function or class is used
   - No explicit type parameter declarations needed - `any` is automatically generic

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
return     start       step         test        tests       this        
to         true        while        is          returns     description 
input      unit        private      constant    functions
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
-2.5        // Negative number
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

#### List Literals
```clean
[1, 2, 3, 4]           // Integer list
["a", "b", "c"]        // String list
[]                     // Empty list
[true, false, true]    // Boolean list
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
| `number`    | Decimal numbers | Platform optimal (≥64 bits) | `3.14`, `6.02e23` |
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
| `number:32`  | f32 | 32-bit | IEEE 754 single | Default number, graphics |
| `number:64`  | f64 | 64-bit | IEEE 754 double | High precision, scientific computing |

#### Examples

```clean
// Integer precision examples
integer:8 red = 255              // Color component (0-255)
integer:16 coordinate = 1024     // Screen coordinate
integer:32 count = 1000000       // Default integer (can omit :32)
integer:64 timestamp = 1640995200000  // Unix timestamp in milliseconds

// Float precision examples  
number:32 temperature = 23.5      // Default number (can omit :32)
number:64 preciseValue = 3.141592653589793  // High precision calculation

// Apply-blocks with precision
integer:8:
    red = 255
    green = 128
    blue = 64

number:64:
    pi = 3.141592653589793
    e = 2.718281828459045
```

#### Default Behavior
- `integer` without modifier defaults to `integer:32`
- `number` without modifier defaults to `number:32`
- This maintains backward compatibility with existing code

### Composite & Generic Types

| Type syntax | What it is | Example |
|-------------|------------|---------|
| `list<any>`  | Homogeneous resizable list | `list<integer>`, `[1, 2, 3]` |
| `list<any>` | Flexible list with behavior properties | `list<string>`, `[]`, behavior via `.type` property |
| `matrix<any>` | 2-D list (list of lists) | `matrix<number>`, `[[1.0, 2.0], [3.0, 4.0]]` |
| `pairs<any,any>`  | Key-value associative container | `pairs<string, integer>` |
| `any`         | Generic type parameter | Used in function definitions |

Lists in Clean are zero-indexed by default (list[0] is the first element).
For readability, you can access elements starting from 1 using:

list.at(index)
This returns the element at position index - 1.

### List Properties - Collection Behavior Modifiers

Clean Language extends the core `list<any>` type with **property modifiers** that change the list's behavior without requiring separate collection types. This provides a unified, consistent approach to different collection patterns while maintaining type safety and simplicity.

#### Property Syntax

```clean
list<any> myList = []                    // Create empty list
myList.type = "behavior_type"            // Set behavior using string
```

Where `behavior_type` is a string that defines how the list handles insertions, removals, and access patterns.

**Supported behavior strings:**
- `"default"` - Standard list behavior
- `"line"` - FIFO queue behavior  
- `"pile"` - LIFO stack behavior
- `"unique"` - Set behavior (no duplicates)
- `"line-unique"` - FIFO queue with uniqueness
- `"pile-unique"` - LIFO stack with uniqueness

#### Supported Properties

**`"line"` - Queue Behavior (FIFO)**

First-In-First-Out behavior. Elements are added to the back and removed from the front.

```clean
functions:
    void processTaskQueue()
        list<string> tasks = []
        tasks.type = "line"
        
        // Add tasks (to back)
        tasks.add("Task 1")
        tasks.add("Task 2") 
        tasks.add("Task 3")
        
        // Process tasks (from front)
        while tasks.size() > 0
            string currentTask = tasks.remove()  // Gets "Task 1", then "Task 2", etc.
            println("Processing: " + currentTask)
```

**Modified Operations**:
- `add(item)` → Adds to the **back** of the list
- `remove()` → Removes from the **front** of the list  
- `peek()` → Views the **front** element without removing
- Standard list operations (`get(index)`, `size()`) remain unchanged

**`"pile"` - Stack Behavior (LIFO)**

Last-In-First-Out behavior. Elements are added and removed from the same end (top).

```clean
functions:
    void undoSystem()
        list<string> actions = []
        actions.type = "pile"
        
        // Perform actions (add to top)
        actions.add("Create file")
        actions.add("Edit text")
        actions.add("Save file")
        
        // Undo actions (remove from top)
        while actions.size() > 0
            string lastAction = actions.remove()  // Gets "Save file", then "Edit text", etc.
            println("Undoing: " + lastAction)
```

**Modified Operations**:
- `add(item)` → Adds to the **top** of the list
- `remove()` → Removes from the **top** of the list
- `peek()` → Views the **top** element without removing
- Standard list operations (`get(index)`, `size()`) remain unchanged

**`"unique"` - Set Behavior (Uniqueness Constraint)**

Only allows unique elements. Duplicate additions are ignored.

```clean
functions:
    void trackUniqueVisitors()
        list<string> visitors = []
        visitors.type = "unique"
        
        // Add visitors (duplicates ignored)
        visitors.add("Alice")    // Added
        visitors.add("Bob")      // Added  
        visitors.add("Alice")    // Ignored (duplicate)
        visitors.add("Charlie")  // Added
        
        println("Unique visitors: " + visitors.size().toString())  // Prints: 3
        
        if visitors.contains("Alice")
            println("Alice has visited")
```

**Modified Operations**:
- `add(item)` → Adds only if `item` is not already present
- `remove()` → Removes from default position (implementation-dependent)
- `contains(item)` → Optimized for membership testing
- Standard list operations remain available

#### Property Combinations

Properties can be combined by setting the type to a combined behavior string:

```clean
// Unique queue - FIFO with no duplicates
list<string> uniqueQueue = []
uniqueQueue.type = "line-unique"

// Unique stack - LIFO with no duplicates  
list<integer> uniqueStack = []
uniqueStack.type = "pile-unique"

// All combinations are supported
list<integer> allFeatures = []
allFeatures.type = "line-unique-pile"  // Advanced combination
```

#### Available Methods

All list types support these methods regardless of behavior:

**Core Methods:**
- `add(item)` → Adds an item to the list (behavior determines position)
- `remove()` → Removes and returns an item (behavior determines which item)
- `peek()` → Views the next item to be removed without removing it
- `contains(item)` → Returns `true` if the item exists in the list
- `size()` → Returns the number of items in the list

**Standard List Methods:**
- `get(index)` → Gets item at specific index (0-based)
- `set(index, item)` → Sets item at specific index
- `isEmpty()` → Returns `true` if list is empty
- `isNotEmpty()` → Returns `true` if list contains items

**Behavior Management:**
- Setting `myList.type = "behavior"` changes the list's behavior at runtime

#### Performance Characteristics
- `"line"`: O(1) add, O(1) remove, O(1) peek
- `"pile"`: O(1) add, O(1) remove, O(1) peek  
- `"unique"`: O(1) add/contains (hash-based), O(1) remove

#### Advantages

1. **Unified Type System**: Single `list<any>` type instead of multiple collection types
2. **Consistent API**: All lists share the same base methods
3. **Flexible Behavior**: Properties can be changed at runtime if needed
4. **Type Safety**: Full generic type support with compile-time validation
5. **Simplicity**: Easier to learn and remember than separate collection classes
6. **Interoperability**: All property-modified lists are still `list<any>` types

#### Complete Example

```clean
start()
    // Test different list behaviors
    list<integer> myList = []
    
    // Test line behavior (FIFO queue)
    myList.type = "line"
    myList.add(1)
    myList.add(2)
    myList.add(3)
    
    integer first = myList.remove()   // Returns 1 (first in, first out)
    integer second = myList.remove()  // Returns 2
    
    // Switch to pile behavior (LIFO stack)
    myList.type = "pile"
    myList.add(10)
    myList.add(20)
    myList.add(30)
    
    integer top = myList.remove()     // Returns 30 (last in, first out)
    
    // Switch to unique behavior (set)
    myList.type = "unique"
    myList.add(100)
    myList.add(200)
    myList.add(100)  // Ignored (duplicate)
    
    boolean hasHundred = myList.contains(100)  // Returns true
    integer listSize = myList.size()           // Returns 2 (no duplicates)
    
    print("List demonstrates flexible behavior at runtime")
```

### Type Annotations and Variable Declaration

Variables use **type-first** syntax:

```clean
// Basic variable declarations
integer count = 0
number temperature = 23.5
boolean isActive = true
string name = "Alice"

// Uninitialized variables
integer sum
string message
```

### Type Conversion

**Implicit conversions (safe widening):**
- `integer` → `number` (with precision loss warning)
- Same-sign, wider types → OK

**Explicit conversions:**
```clean
value.toInteger   // convert to integer
value.toNumber     // convert to floating-point
value.toString    // convert to string
value.toBoolean   // convert to boolean
```

**Implementation Status:**
- ✅ **Numeric Conversions**: `integer.toNumber`, `number.toInteger`, `integer.toBoolean` fully implemented
- ✅ **Boolean Conversions**: `integer.toBoolean` (0 = false, non-zero = true) implemented
- ⚠️ **String Conversions**: `value.toString` requires runtime functions (not yet implemented)

**Examples:**
```clean
integer num = 42
number numFloat = num.toNumber      // ✅ Works: converts 42 to 42.0
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

list.push:
    item1
    item2
    item3
// Equivalent to: list.push(item1), list.push(item2), list.push(item3)
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
    number PI = 3.14159
    string VERSION = "1.0.0"
```

## Expressions

### Operator Precedence

From highest to lowest precedence:

1. **Primary** - `()`, function calls, method calls, property access
2. **Unary** - `not`, `-` (unary minus)
3. **Exponentiation** - `^` (right-associative)
4. **Multiplicative** - `*`, `/`, `%`
5. **Additive** - `+`, `-`
6. **Comparison** - `<`, `>`, `<=`, `>=`
7. **Equality** - `==`, `!=`, `is`, `not`
8. **Logical AND** - `and`
9. **Logical OR** - `or`
10. **Assignment** - `=`

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
a not b     // Logical NOT (binary, equivalent to !=)
// Note: Unary not operator not yet implemented
```

### Matrix Operations

Clean Language uses **type-based operator overloading** for basic operations and **method calls** for advanced operations:

```clean
// Basic operations (type-based overloading)
A * B       // Matrix multiplication (when A, B are matrix<T>)
A + B       // Matrix addition (when A, B are matrix<T>)
A - B       // Matrix subtraction (when A, B are matrix<T>)
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
list.get(0)           // Built-in method
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
number y = 3.14
string z
boolean flag = true
```

### Assignment

```clean
x = 42              // Simple assignment
arr[0] = value      // List element assignment
obj.property = val  // Property assignment
```

### Print Statements

Clean Language supports two print syntaxes: simple inline syntax and block syntax with colon. Print functions automatically convert any value to a string representation, making output simple and intuitive.

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

#### Automatic String Conversion

**Print functions work seamlessly with all data types through the toString() method system**. The compiler automatically handles string conversion when needed:

```clean
// toString() method calls work perfectly
integer age = 25
number price = 19.99
boolean isValid = true

print(age.toString())       // Prints: 25
print(price.toString())     // Prints: 19.99  
print(isValid.toString())   // Prints: true

// String variables and literals work directly
string name = "Alice"
print(name)                 // Prints: Alice
print("Hello World")        // Prints: Hello World

// Mixed usage in the same program
print("Age:")
print(age.toString())
print("Price:")
print(price.toString())
```

**Implementation Status:**
- ✅ **toString() method calls**: `print(value.toString())` works perfectly
- ✅ **String variables**: `print(string_var)` works perfectly  
- ✅ **String literals**: `print("text")` works perfectly
- ✅ **Variable assignment**: `string result = value.toString()` works perfectly

#### Default toString() Behavior

Every type in Clean Language has a built-in `toString()` method with sensible defaults:

**Built-in Types:**
- **Integers**: `42` → `"42"`
- **Floats**: `3.14` → `"3.14"`
- **Booleans**: `true` → `"true"`, `false` → `"false"`
- **Strings**: `"hello"` → `"hello"` (no change)
- **Lists**: `[1, 2, 3]` → `"[1, 2, 3]"`
- **Objects**: `MyClass` instance → `"MyClass"` (default) or custom representation

**Custom Classes:**
```clean
class Person
    string name
    integer age
    
    // Optional: Override default toString() for custom output
    functions:
        string toString()
            return name + " (" + age.toString() + " years old)"

// Usage
Person user = Person("Alice", 30)
print(user)             // Prints: Alice (30 years old)

// Without custom toString(), would print: Person
```

**Default Class Behavior:**
- Classes without custom `toString()` method print their class name
- You can override `toString()` in any class for custom string representation
- The custom `toString()` method is automatically used by print functions

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

### Console Input

Console input in Clean lets you ask the user for a value with a simple prompt. Use `input()` for text, `input.integer()` and `input.number()` for numbers, and `input.yesNo()` for true/false — all with safe defaults and clear syntax.

```clean
// Get text input from user
string name = input("What's your name? ")
string message = input()  // Simple prompt with no text

// Get numeric input with automatic conversion
integer age = input.integer("How old are you? ")
number height = input.number("Your height in meters: ")

// Get yes/no input as boolean
boolean confirmed = input.yesNo("Are you sure? ")
boolean subscribe = input.yesNo("Subscribe to newsletter? ")
```

#### Input Features

- **Safe defaults**: Invalid input automatically retries with helpful messages
- **Type conversion**: `input.integer()` and `input.number()` handle numeric conversion safely
- **Boolean parsing**: `input.yesNo()` accepts "yes"/"no", "y"/"n", "true"/"false", "1"/"0"
- **Clean prompts**: Prompts are displayed clearly and wait for user input
- **Error handling**: Invalid input shows friendly error messages and asks again

#### Usage Examples

```clean
functions:
    void start()
        // Basic user interaction
        string userName = input("Enter your name: ")
        println("Hello, " + userName + "!")
        
        // Numeric calculations
        integer num1 = input.integer("First number: ")
        integer num2 = input.integer("Second number: ")
        integer sum = num1 + num2
        println("Sum: " + sum.toString())
        
        // Decision making
        boolean wantsCoffee = input.yesNo("Would you like coffee? ")
        if wantsCoffee
            println("Great! Coffee coming right up.")
        else
            println("No problem, maybe next time.")
```

### Return Statement

```clean
return              // Return void
return value        // Return a value
return expression   // Return expression result
```

## Functions

Clean Language uses **functions blocks** for all function declarations. This ensures consistency and organization in code structure.

### The Start Function

Every Clean program begins with a `start()` function. The start function is **special** and can be declared standalone (outside of functions: blocks):

```clean
start()
    print("Hello, World!")
    integer x = 42
    print(x)
```

Alternatively, it can be declared within a `functions:` block:

```clean
functions:
    void start()
        print("Hello, World!")
        integer x = 42
        print(x)
```

### Functions Blocks (Required)

**All functions except `start()` must be declared within a `functions:` block.** This is the only supported syntax for function declarations:

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

### Generic Functions with `any`

Clean Language uses `any` as the universal generic type. No explicit type parameter declarations are needed:

```clean
functions:
    any identity(any value)
        return value
    
    any getFirst(list<any> items)
        return items[0]
    
    void printAny(any value)
        print(value.toString())

// Usage - type is inferred at compile time
string result = identity("hello")    // any → string
integer number = identity(42)        // any → integer
number decimal = identity(3.14)       // any → number
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

### Default Parameter Values

Clean Language supports default parameter values in both function declarations and input blocks. This feature enhances code readability and provides sensible defaults for optional parameters.

#### Input Block Default Values

Default values are particularly useful in input blocks, allowing functions to work with sensible defaults when parameters are not provided:

```clean
functions:
    integer calculateArea()
        description "Calculate area with default dimensions"
        input
            integer width = 10      // Default width
            integer height = 5      // Default height
        return width * height

    string formatMessage()
        description "Format a message with optional parameters"
        input
            string text = "Hello"   // Default message
            string prefix = ">> "   // Default prefix
            boolean uppercase = false  // Default formatting
        if uppercase
            return prefix + text.toUpperCase()
        else
            return prefix + text
```

#### Function Parameter Default Values

Default values can also be used in regular function parameters:

```clean
functions:
    string greet(string name = "World")
        return "Hello, " + name
    
    integer power(integer base, integer exponent = 2)
        // Default exponent of 2 for squaring
        return base ^ exponent
    
    void logMessage(string message, string level = "INFO")
        print("[" + level + "] " + message)
```

#### Usage Examples

```clean
functions:
    void start()
        // Using functions with default values
        print(greet())              // "Hello, World" (uses default)
        print(greet("Alice"))       // "Hello, Alice" (overrides default)
        
        integer squared = power(5)  // 25 (uses default exponent=2)
        integer cubed = power(5, 3) // 125 (overrides exponent)
        
        logMessage("System started")           // [INFO] System started
        logMessage("Error occurred", "ERROR")  // [ERROR] Error occurred
        
        // Input blocks with defaults work seamlessly
        integer area1 = calculateArea()        // Uses defaults: 10 * 5 = 50
        // When calling functions with input blocks, defaults are applied automatically
```

#### Default Value Rules

1. **Expression Support**: Default values can be any valid Clean Language expression
2. **Type Compatibility**: Default values must match the parameter's declared type
3. **Evaluation Time**: Default values are evaluated at function call time
4. **Optional Nature**: Parameters with default values become optional in function calls

**Examples of Valid Default Values:**
```clean
functions:
    void examples()
        input
            integer count = 42                    // Literal value
            string message = "Default text"       // String literal
            boolean flag = true                   // Boolean literal
            number ratio = 3.14                    // Number literal
            integer calculated = 10 + 5           // Expression
            string formatted = "Value: " + "test" // String concatenation
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

## Testing

Clean Language includes a built-in testing framework with a simple and readable syntax. Tests can be embedded directly in your source code using the `tests:` block.

### Test Block Syntax

Tests are defined within a `tests:` block and can be either named or anonymous:

```clean
tests:
    // Named tests with descriptions
    "adds numbers": add(2, 3) = 5
    "squares a number": square(4) = 16
    "detects empty string": String.isEmpty("") = true
    
    // Anonymous tests (no description)
    String.toUpperCase("hi") = "HI"
    Math.abs(-42) = 42
    [1, 2, 3].length() = 3
```

### Test Syntax Rules

1. **Named Tests**: `"description": expression = expected`
   - The description is a string literal that will be used as a label in test output
   - The colon (`:`) separates the description from the test expression
   - Useful for documenting what the test is verifying

2. **Anonymous Tests**: `expression = expected`
   - No description provided - the expression itself serves as documentation
   - Simpler syntax for obvious test cases

3. **Test Expressions**: Can be any valid Clean Language expression
   - Function calls: `add(2, 3)`
   - Method calls: `String.isEmpty("")`
   - Complex expressions: `(x + y) * 2`
   - Object creation and method chaining: `Point(3, 4).distanceFromOrigin()`

4. **Expected Values**: The right side of `=` is the expected result
   - Must be a compile-time evaluable expression or literal
   - Type must match the test expression's return type

### Test Execution

When a Clean program contains a `tests:` block, the compiler can run tests in several ways:

```bash
# Run tests during compilation
cleanc --test myprogram.cln

# Compile and run tests separately
cleanc myprogram.cln --include-tests
./myprogram --run-tests
```

### Test Output Format

The test runner provides clear, readable output:

```
Running tests for myprogram.cln...

✅ adds numbers: add(2, 3) = 5 (PASS)
✅ squares a number: square(4) = 16 (PASS) 
❌ detects empty string: String.isEmpty("") = true (FAIL: expected true, got false)
✅ String.toUpperCase("hi") = "HI" (PASS)

Test Results: 3 passed, 1 failed, 4 total
```

### Advanced Testing Features

#### Testing Functions with Error Handling

```clean
functions:
    integer safeDivide(integer a, integer b)
        if b == 0
            error("Division by zero")
        return a / b

tests:
    "normal division": safeDivide(10, 2) = 5
    "division by zero throws error": safeDivide(10, 0) = error("Division by zero")
```

#### Testing Object Methods

```clean
class Calculator
    integer value
    
    constructor(integer initialValue)
        value = initialValue
    
    functions:
        integer add(integer x)
            value = value + x
            return value

tests:
    "calculator addition": Calculator(10).add(5) = 15
    "calculator chaining": Calculator(0).add(3).add(7) = 10
```

#### Testing List and String Operations

```clean
tests:
    "list operations": [1, 2, 3].length() = 3
    "list contains": [1, 2, 3].contains(2) = true
    "string operations": "hello".toUpperCase() = "HELLO"
    "string indexing": "world".indexOf("r") = 2
```

### Best Practices

1. **Descriptive Test Names**: Use clear, descriptive names for complex tests
   ```clean
   tests:
       "calculates compound interest correctly": calculateCompoundInterest(1000, 0.05, 2) = 1102.5
   ```

2. **Test Edge Cases**: Include tests for boundary conditions
   ```clean
   tests:
       "handles empty list": [].length() = 0
       "handles single character": "a".toUpperCase() = "A"
       "handles zero input": factorial(0) = 1
   ```

3. **Group Related Tests**: Organize tests logically within the `tests:` block
   ```clean
   tests:
       // Basic arithmetic
       "addition": add(2, 3) = 5
       "subtraction": subtract(5, 2) = 3
       
       // String operations  
       "uppercase conversion": "hello".toUpperCase() = "HELLO"
       "lowercase conversion": "WORLD".toLowerCase() = "world"
   ```

4. **Test Both Success and Failure Cases**: Include tests for error conditions
   ```clean
   tests:
       "valid input": processInput("valid") = "processed: valid"
       "invalid input": processInput("") = error("Input cannot be empty")
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
// Iterate over list elements
iterate item in list
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

### Generic Classes with `any`

Clean Language uses `any` for generic class fields and methods:

```clean
class Container
    any value                  // any makes class generic

    constructor(any value)     // Auto-stores to matching field

    functions:
        any get()
        return value

        void set(any newValue)
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
    number radius
    
    constructor(string colorParam, number radiusParam)
        base(colorParam)            // Call parent constructor with 'base'
        radius = radiusParam        // Implicit context
    
    functions:
        number area()
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
        number add(number a, number b)
            return a + b
        
        number max(number a, number b)
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
        number result = Math.add(5.0, 3.0)
        number maximum = Math.max(10.0, 7.5)
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

### Design Philosophy: Flexible Organization

Clean Language supports both class-based organization and top-level functions, providing flexibility for different coding styles and project needs:

#### Class-Based Organization (Recommended for complex projects)
- **Better code organization**: Related functionality is grouped together
- **Namespace management**: No global function name conflicts  
- **Consistent syntax**: All method calls use the same `Class.method()` or `object.method()` pattern
- **Extensibility**: Easy to add related methods to existing classes

```clean
class Calculator
    functions:
        number calculateTax(number amount)
            return amount * 0.15
        
        string formatResult(number value)
            return "Result: " + value.toString()
```

#### Top-Level Functions (Suitable for simpler projects)
- **Direct approach**: Functions can be declared directly in `functions:` blocks
- **Simplicity**: No need for class wrapper when functionality is standalone
- **Scripting style**: Perfect for utility scripts and simple programs

```clean
functions:
    number calculateTax(number amount)
        return amount * 0.15
    
    string formatResult(number value)
        return "Result: " + value.toString()
    
    void start()
        number tax = calculateTax(100.0)
        string result = formatResult(tax)
        print(result)
```

**Both approaches are valid and can be mixed within the same program.** The choice depends on project complexity and developer preference.

## Standard Library

Clean Language provides built-in utility classes for common operations. All standard library classes follow the compiler instructions:

- All methods are in `functions:` blocks
- Method calls require parentheses
- No `Utils` suffix in class names
- Use `any` for generic operations

### Math Class

```clean
class Math
    functions:
        // Basic arithmetic
        number add(number a, number b)
        number subtract(number a, number b)
        number multiply(number a, number b)
        number divide(number a, number b)
        
        // Core mathematical operations
        number sqrt(number x)
        number abs(number x)          // Absolute value for numbers
        integer abs(integer x)      // Absolute value for integers
        number max(number a, number b)
        number min(number a, number b)
        
        // Rounding and precision functions
        number floor(number x)    // Round down to nearest integer
        number ceil(number x)     // Round up to nearest integer  
        number round(number x)    // Round to nearest integer
        number trunc(number x)    // Remove decimal part
        number sign(number x)     // Returns -1, 0, or 1
        
        // Trigonometric functions - work with radians
        number sin(number x)      // Sine
        number cos(number x)      // Cosine
        number tan(number x)      // Tangent
        number asin(number x)     // Arc sine (inverse sine)
        number acos(number x)     // Arc cosine (inverse cosine)
        number atan(number x)     // Arc tangent (inverse tangent)
        number atan2(number y, number x)  // Two-argument arc tangent
        
        // Logarithmic and exponential functions
        number ln(number x)       // Natural logarithm (base e)
        number log10(number x)    // Base-10 logarithm
        number log2(number x)     // Base-2 logarithm
        number exp(number x)      // e raised to the power of x
        number exp2(number x)     // 2 raised to the power of x
        
        // Hyperbolic functions - useful for advanced calculations
        number sinh(number x)     // Hyperbolic sine
        number cosh(number x)     // Hyperbolic cosine
        number tanh(number x)     // Hyperbolic tangent
        
        // Mathematical constants
        number pi()              // π ≈ 3.14159
        number e()               // Euler's number ≈ 2.71828
        number tau()             // τ = 2π ≈ 6.28318

// Usage Examples
functions:
    void start()
        // Basic calculations
        number result = Math.add(5.0, 3.0)
        float maximum = Math.max(10.5, 7.2)
        
        // Geometry - calculate circle area
        number radius = 5.0
        number area = Math.multiply(Math.pi(), radius ^ 2.0)
        
        // Trigonometry - find triangle sides
        number angle = Math.divide(Math.pi(), 4.0)  // 45 degrees in radians
        number opposite = Math.multiply(10.0, Math.sin(angle))
        number adjacent = Math.multiply(10.0, Math.cos(angle))
        
        // Rounding numbers for display
        number price = 19.99567
        number rounded = Math.round(price)  // 20.0
        number floored = Math.floor(price)  // 19.0
        
        // Logarithmic calculations
        number growth = Math.exp(0.05)      // e^0.05 for 5% growth
        number halfLife = Math.log2(100.0)  // How many times to halve 100 to get 1
        
        // Distance calculations using Pythagorean theorem
        number dx = 3.0
        number dy = 4.0
        number distance = Math.sqrt(Math.add(dx ^ 2.0, dy ^ 2.0))
        
        // Absolute values for different types
        number numberAbs = Math.abs(-5.7)    // 5.7
        integer intAbs = Math.abs(-42)     // 42
```

### String Class

The String class provides powerful text manipulation capabilities. Whether you're processing user input, formatting output, or analyzing text data, String has all the tools you need for effective text handling.

```clean
class String
    functions:
        // Basic operations
        integer length(string text)
            // Returns the number of characters in the string
            // Perfect for validation and loop bounds
        
        string concat(string a, string b)
            // Joins two strings together
            // Creates a new string without modifying the originals
        
        string substring(string text, integer start, integer end)
            // Extracts a portion of the string from start to end position
            // Great for parsing and text extraction
        
        // Case operations - useful for user input normalization
        string toUpperCase(string text)
            // Converts all letters to uppercase
            // Perfect for case-insensitive comparisons
        
        string toLowerCase(string text)
            // Converts all letters to lowercase
            // Ideal for standardizing user input
        
        // Search and validation operations
        boolean contains(string text, string search)
            // Checks if the text contains the search string
            // Returns true if found, false otherwise
        
        integer indexOf(string text, string search)
            // Finds the first position of search string in text
            // Returns -1 if not found, position index if found
        
        integer lastIndexOf(string text, string search)
            // Finds the last position of search string in text
            // Useful for finding file extensions or repeated patterns
        
        boolean startsWith(string text, string prefix)
            // Checks if text begins with the given prefix
            // Great for URL validation or command parsing
        
        boolean endsWith(string text, string suffix)
            // Checks if text ends with the given suffix
            // Perfect for file type checking
        
        // Text cleaning and formatting
        string trim(string text)
            // Removes whitespace from both ends of the string
            // Essential for cleaning user input
        
        string trimStart(string text)
            // Removes whitespace from the beginning only
            // Useful for preserving trailing spaces
        
        string trimEnd(string text)
            // Removes whitespace from the end only
            // Helpful for cleaning line endings
        
        // Advanced text manipulation - powerful tools for text transformation
        string replace(string text, string oldValue, string newValue)
            // Replaces the first occurrence of oldValue with newValue
            // Like find-and-replace in a word processor, but only changes the first match
            // Example: replace("Hello Hello", "Hello", "Hi") → "Hi Hello"
        
        string replaceAll(string text, string oldValue, string newValue)
            // Replaces ALL occurrences of oldValue with newValue
            // Like find-and-replace-all - changes every match in the text
            // Example: replaceAll("Hello Hello", "Hello", "Hi") → "Hi Hi"
        
        list<string> split(string text, string delimiter)
            // Breaks a string into pieces using a separator character
            // Like cutting a rope at specific points - very useful for data processing
            // Example: split("apple,banana,orange", ",") → ["apple", "banana", "orange"]
        
        string join(list<string> parts, string separator)
            // Combines an array of strings into one string with separators
            // The opposite of split - like gluing pieces back together
            // Example: join(["apple", "banana", "orange"], ", ") → "apple, banana, orange"
        
        // Character operations - work with individual letters and symbols
        string charAt(string text, integer index)
            // Gets the character (letter/symbol) at a specific position
            // Like picking out the 3rd letter from a word
            // Example: charAt("Hello", 1) → "e" (positions start at 0)
        
        integer charCodeAt(string text, integer index)
            // Gets the numeric code of a character (useful for sorting or encoding)
            // Every character has a number - 'A' is 65, 'a' is 97, etc.
            // Example: charCodeAt("Hello", 0) → 72 (the code for 'H')
        
        // Validation helpers - check if text meets certain conditions
        boolean isEmpty(string text)
            // Checks if a string has no characters at all
            // Like checking if a box is completely empty
            // Example: isEmpty("") → true, isEmpty("Hi") → false
        
        boolean isBlank(string text)
            // Checks if a string is empty OR contains only spaces/tabs
            // More thorough than isEmpty - catches "invisible" content too
            // Example: isBlank("   ") → true, isBlank("Hi") → false
        
        // Padding operations - add characters to make text a specific length
        string padStart(string text, integer length, string padString)
            // Adds characters to the beginning until the text reaches desired length
            // Like adding zeros before a number: "42" becomes "00042"
            // Example: padStart("42", 5, "0") → "00042"
        
        string padEnd(string text, integer length, string padString)
            // Adds characters to the end until the text reaches desired length
            // Like adding spaces after text to align it in columns
            // Example: padEnd("Name", 10, " ") → "Name      "
        
        // Conversion utilities
        string toString(any value)
            // Converts any value to its string representation
            // Universal conversion for display purposes

// Usage Examples - Real-world string processing scenarios
functions:
    void start()
        // Basic text processing
        string userInput = "  Hello World!  "
        string cleaned = String.trim(userInput)        // "Hello World!"
        integer length = String.length(cleaned)        // 12
        
        // Case normalization for comparisons
        string email1 = "USER@EXAMPLE.COM"
        string email2 = "user@example.com"
        boolean same = String.toLowerCase(email1) == String.toLowerCase(email2)  // true
        
        // Text searching and validation
        string filename = "document.pdf"
        boolean isPdf = String.endsWith(filename, ".pdf")     // true
        integer dotPos = String.lastIndexOf(filename, ".")    // 8
        
        // URL processing
        string url = "https://api.example.com/users"
        boolean isHttps = String.startsWith(url, "https://")  // true
        boolean hasApi = String.contains(url, "api")          // true
        
        // Text parsing and reconstruction
        string csvLine = "John,Doe,25,Engineer"
        list<string> fields = String.split(csvLine, ",")     // ["John", "Doe", "25", "Engineer"]
        string fullName = String.join([fields[0], fields[1]], " ")  // "John Doe"
        
        // Text replacement and cleaning
        string messyText = "Hello    World"
        string cleaned = String.replaceAll(messyText, "    ", " ")  // "Hello World"
        
        // Formatting and padding
        string number = "42"
        string padded = String.padStart(number, 5, "0")       // "00042"
        
        // Character-level operations
        string word = "Hello"
        string firstChar = String.charAt(word, 0)             // "H"
        integer charCode = String.charCodeAt(word, 0)         // 72 (ASCII for 'H')
        
        // Input validation
        string userField = "   "
        boolean isValid = !String.isBlank(userField)          // false
```

### List Class

The List class provides powerful data collection capabilities. Whether you're managing lists of items, processing data sets, or organizing information, List has all the tools you need for effective data manipulation.

```clean
class List
    functions:
        // Basic operations - fundamental list access
        integer length(list<any> array)
            // Returns the number of elements in the list
            // Like counting how many items are in a box
            // Example: length([1, 2, 3]) → 3
        
        any get(list<any> array, integer index)
            // Gets the element at the specified position
            // Like picking out the 3rd item from a list
            // Example: get([10, 20, 30], 1) → 20 (positions start at 0)
        
        void set(list<any> array, integer index, any value)
            // Updates the element at the specified position
            // Like replacing an item in a specific slot
            // Example: set([1, 2, 3], 1, 99) → [1, 99, 3]
        
        // Modification operations - changing array contents
        list<any> push(list<any> array, any item)
            // Adds an element to the end of the list
            // Like adding a new item to the end of a list
            // Example: push([1, 2], 3) → [1, 2, 3]
        
        any pop(list<any> array)
            // Removes and returns the last element from the list
            // Like taking the top item off a stack
            // Example: pop([1, 2, 3]) → 3, array becomes [1, 2]
        
        list<any> insert(list<any> array, integer index, any item)
            // Inserts an element at a specific position
            // Like squeezing a new item into the middle of a line
            // Example: insert([1, 3], 1, 2) → [1, 2, 3]
        
        any remove(list<any> array, integer index)
            // Removes and returns the element at the specified position
            // Like taking out a specific item and closing the gap
            // Example: remove([1, 2, 3], 1) → 2, array becomes [1, 3]
        
        // Search operations - finding elements in lists
        boolean contains(list<any> array, any item)
            // Checks if the list contains the specified item
            // Like looking through a box to see if something is there
            // Example: contains([1, 2, 3], 2) → true
        
        integer indexOf(list<any> array, any item)
            // Finds the first position of the item in the list
            // Like finding where something is located in a list
            // Example: indexOf([10, 20, 30], 20) → 1
        
        integer lastIndexOf(list<any> array, any item)
            // Finds the last position of the item in the list
            // Useful when the same item appears multiple times
            // Example: lastIndexOf([1, 2, 1, 3], 1) → 2
        
        // List transformation operations - creating new lists
        list<any> slice(list<any> array, integer start, integer end)
            // Creates a new array containing elements from start to end position
            // Like cutting out a section of the original array
            // Example: slice([1, 2, 3, 4, 5], 1, 4) → [2, 3, 4]
        
        list<any> concat(list<any> array1, list<any> array2)
            // Combines two lists into a single new array
            // Like joining two lists together
            // Example: concat([1, 2], [3, 4]) → [1, 2, 3, 4]
        
        list<any> reverse(list<any> array)
            // Creates a new array with elements in reverse order
            // Like flipping the list upside down
            // Example: reverse([1, 2, 3]) → [3, 2, 1]
        
        list<any> sort(list<any> array)
            // Creates a new array with elements sorted in ascending order
            // Like organizing items from smallest to largest
            // Example: sort([3, 1, 4, 2]) → [1, 2, 3, 4]
        
        // Functional programming operations - advanced array processing
        list<any> map(list<any> array, function callback)
            // Creates a new array by applying a function to each element
            // Like transforming every item in the list using a rule
            // Example: map([1, 2, 3], x => x * 2) → [2, 4, 6]
        
        list<any> filter(list<any> array, function callback)
            // Creates a new array containing only elements that pass a test
            // Like keeping only the items that meet certain criteria
            // Example: filter([1, 2, 3, 4], x => x > 2) → [3, 4]
        
        any reduce(list<any> array, function callback, any initialValue)
            // Reduces the list to a single value by applying a function
            // Like combining all elements into one result
            // Example: reduce([1, 2, 3, 4], (sum, x) => sum + x, 0) → 10
        
        void forEach(list<any> array, function callback)
            // Executes a function for each element in the list
            // Like doing something with every item in the list
            // Example: forEach([1, 2, 3], x => print(x)) → prints 1, 2, 3
        
        // Utility operations - helpful array functions
        boolean isEmpty(list<any> array)
            // Checks if the list has no elements
            // Like checking if a box is completely empty
            // Example: isEmpty([]) → true, isEmpty([1]) → false
        
        boolean isNotEmpty(list<any> array)
            // Checks if the list has at least one element
            // Opposite of isEmpty - checks if there's something there
            // Example: isNotEmpty([1, 2]) → true
        
        any first(list<any> array)
            // Gets the first element of the list
            // Like looking at the item at the front of the line
            // Example: first([10, 20, 30]) → 10
        
        any last(list<any> array)
            // Gets the last element of the list
            // Like looking at the item at the back of the line
            // Example: last([10, 20, 30]) → 30
        
        string join(list<string> array, string separator)
            // Combines all array elements into a single string with separators
            // Like gluing text pieces together with a connector
            // Example: join(["apple", "banana", "orange"], ", ") → "apple, banana, orange"
        
        // List creation helpers - building new lists
        list<any> fill(integer size, any value)
            // Creates a new array of specified size filled with the same value
            // Like making multiple copies of the same item
            // Example: fill(3, "hello") → ["hello", "hello", "hello"]
        
        list<integer> range(integer start, integer end)
            // Creates an array of numbers from start to end
            // Like counting from one number to another
            // Example: range(1, 5) → [1, 2, 3, 4, 5]

// Usage Examples - Real-world array processing scenarios
functions:
    void start()
        // Basic array operations
        list<integer> numbers = [1, 2, 3]
        integer size = List.length(numbers)           // 3
        integer first = List.get(numbers, 0)          // 1
        List.set(numbers, 1, 99)                      // [1, 99, 3]
        
        // Building and modifying lists
        list<string> fruits = ["apple", "banana"]
        fruits = List.push(fruits, "orange")          // ["apple", "banana", "orange"]
        string lastFruit = List.pop(fruits)           // "orange", fruits becomes ["apple", "banana"]
        
        // Searching through data
        list<integer> scores = [85, 92, 78, 96, 88]
        boolean hasHighScore = List.contains(scores, 96)     // true
        integer position = List.indexOf(scores, 92)          // 1
        
        // Data processing and transformation
        list<integer> data = [1, 2, 3, 4, 5]
        list<integer> doubled = List.map(data, x => x * 2)  // [2, 4, 6, 8, 10]
        list<integer> evens = List.filter(data, x => x % 2 == 0)  // [2, 4]
        integer sum = List.reduce(data, (total, x) => total + x, 0)  // 15
        
        // List manipulation
        list<string> names1 = ["Alice", "Bob"]
        list<string> names2 = ["Charlie", "Diana"]
        list<string> allNames = List.concat(names1, names2)  // ["Alice", "Bob", "Charlie", "Diana"]
        list<string> reversed = List.reverse(allNames)       // ["Diana", "Charlie", "Bob", "Alice"]
        
        // Working with sections of lists
        list<integer> bigList = [10, 20, 30, 40, 50]
        list<integer> middle = List.slice(bigList, 1, 4)     // [20, 30, 40]
        
        // Text processing with lists
        list<string> words = ["hello", "world", "from", "Clean"]
        string sentence = List.join(words, " ")               // "hello world from Clean"
        
        // Creating lists programmatically
        list<string> greetings = List.fill(3, "Hello")       // ["Hello", "Hello", "Hello"]
        list<integer> countdown = List.range(5, 1)           // [5, 4, 3, 2, 1]
        
        // Validation and utility
        boolean isEmpty = List.isEmpty([])                    // true
        string firstWord = List.first(words)                  // "hello"
        string lastWord = List.last(words)                    // "Clean"
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
        
        list<string> lines(string path)
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
        list<string> logLines = File.lines("app.log")
        
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

## Method-Style Syntax

Clean Language supports both traditional function calls and modern method-style syntax. This makes your code more readable and intuitive by allowing you to call functions directly on values.

### How It Works

Instead of writing `function(value, parameters)`, you can write `value.function(parameters)`. This feels more natural and reads like English!

**Traditional Style:**
```clean
integer textLength = length(myText)
string upperText = toUpperCase(myText)
```

**Method Style (Same Result):**
```clean
integer textLength = myText.length()
string upperText = myText.toUpperCase()
```

### Available Method-Style Functions

#### Utility Functions
These work on any value and help with common tasks:

**Length and Size Functions:**

```clean
// Get the length of text, lists, or collections
integer size = myText.length()
integer count = myList.length()

// Check if something is empty or has content
boolean empty = myText.isEmpty()
boolean hasContent = myList.isNotEmpty()

// Check if a value exists (not null/undefined)
boolean exists = myValue.isDefined()
boolean missing = myValue.isNotDefined()

// Keep numbers within bounds (like volume controls)
integer volume = userInput.keepBetween(0, 100)
number temperature = reading.keepBetween(-10.0, 50.0)

// Get default values for types
integer defaultNumber = defaultInt()        // Returns 0
number defaultDecimal = defaultNumber()       // Returns 0.0
boolean defaultFlag = defaultBool()         // Returns false
```

**Validation and Checking Functions:**

```clean
// Check if something is empty or has content
boolean empty = myText.isEmpty()
boolean hasContent = myList.isNotEmpty()

// Check if a value exists (not null/undefined)
boolean exists = myValue.isDefined()
boolean missing = myValue.isNotDefined()
```

**Boundary and Range Functions:**

```clean
// Keep numbers within bounds (like volume controls)
integer volume = userInput.keepBetween(0, 100)
number temperature = reading.keepBetween(-10.0, 50.0)
```

**Default Value Pattern:**

```clean
// Use 'or' for elegant default values - much cleaner!
integer count = userInput or 0              // If userInput is null/undefined, use 0
string name = userName or "Anonymous"       // If userName is empty, use "Anonymous"
number rate = configRate or 1.0             // If configRate is missing, use 1.0
boolean enabled = setting or true          // If setting is undefined, use true
```

#### Type Conversion Functions
Convert values from one type to another - perfect for user input and data processing:

```clean
// Convert numbers to different types
string numberText = age.toString()          // 25 → "25"
number decimal = wholeNumber.toNumber()       // 42 → 42.0
integer rounded = price.toInteger()         // 19.99 → 19

// Convert text to numbers
integer userAge = ageInput.toInteger()      // "25" → 25
number userHeight = heightInput.toNumber()    // "5.8" → 5.8

// Convert to true/false values
boolean isValid = userChoice.toBoolean()    // "true" → true

// Chain conversions together!
string result = temperature.toFloat().toString()  // "98.6" → 98.6 → "98.6"
```

#### Validation Functions
Make sure your data is correct with friendly assertion methods:

```clean
// Check that conditions are true
userAge.mustBeTrue(userAge > 0)           // Ensures age is positive
password.mustBeTrue(password.length() >= 8)  // Ensures strong password

// Check that conditions are false  
email.mustBeFalse(email.isEmpty())        // Ensures email isn't empty

// Check that two values match
confirmPassword.mustBeEqual(originalPassword)  // Password confirmation

// Check that two values are different
newPassword.mustNotBeEqual(oldPassword)   // Ensures password was changed
```

### Method Chaining

One of the best features is **method chaining** - you can call multiple methods in a row:

```clean
// Clean up and validate user input in one line
string cleanEmail = userInput.trim().toLowerCase().toString()

// Process numbers with multiple steps
integer finalScore = rawScore.keepBetween(0, 100).toInteger()

// Complex text processing
string result = messyText
    .trim()                    // Remove extra spaces
    .toLowerCase()             // Make lowercase  
    .toString()                // Ensure it's text
```

### When to Use Each Style

**Use Method Style When:**
- Working with a specific value (like `text.length()`)
- Chaining multiple operations together
- The code reads more naturally

**Use Traditional Style When:**
- Calling utility functions like `Math.sqrt()`
- Working with multiple parameters of equal importance
- Following existing code patterns

### Real-World Examples

```clean
functions:
    void processUserData()
        // User registration form processing with elegant defaults
        string email = (userEmail or "").trim().toLowerCase()
        boolean validEmail = email.length().keepBetween(5, 100)
        
        // Age validation with method chaining and defaults
        integer age = (ageInput or "18").toInteger().keepBetween(13, 120)
        age.mustBeTrue(age >= 18)  // Must be adult
        
        // Password strength checking
        password.mustBeTrue(password.length() >= 8)
        confirmPassword.mustBeEqual(password)
        
        // Format display text with defaults
        string firstName = userFirstName or "User"
        string welcome = "Welcome, ".concat(firstName.trim())
        string ageText = "Age: ".concat(age.toString())
        
    void dataProcessing()
        // List processing with methods
        list<string> names = ["Alice", "Bob", "Charlie"]
        integer count = names.length()
        boolean hasData = names.isNotEmpty()
        
        // Number processing
        list<number> scores = [85.5, 92.3, 78.9]
        number average = calculateAverage(scores).keepBetween(0.0, 100.0)
        string displayScore = average.toString().concat("%")
```

This method-style syntax makes Clean Language feel modern and intuitive while keeping all the power of traditional function calls!

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

## Package Management (Future Feature)

**Note:** Package management is planned for future releases of Clean Language. Currently, Clean Language focuses on core language features and WebAssembly compilation. Package management capabilities will be added in subsequent versions to enable code sharing and dependency management.

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