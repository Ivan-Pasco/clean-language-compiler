# Clean Language Standard Library

The Clean Language comes with a comprehensive standard library that provides essential functionality for common operations. This document describes the modules and functions available in the standard library.

## Table of Contents

1. [String Operations](#string-operations)
2. [Array Operations](#array-operations)
3. [Math Functions](#math-functions)
4. [Memory Management](#memory-management)
5. [Error Handling](#error-handling)
6. [I/O Operations](#io-operations)
7. [Type Conversion](#type-conversion)

## String Operations

The `string` module provides functions for working with strings.

### string.length

Returns the length of a string.

**Signature**
```
function string.length(s: string): int
```

**Parameters**
- `s`: The input string

**Returns**
- The number of characters in the string

**Example**
```
start()
    s = "Hello, World!"
    len = string.length(s)  // 13
    print(len)
```

### string.concat

Concatenates two strings.

**Signature**
```
function string.concat(s1: string, s2: string): string
```

**Parameters**
- `s1`: The first string
- `s2`: The second string

**Returns**
- A new string that is the concatenation of s1 and s2

**Example**
```
start()
    s1 = "Hello, "
    s2 = "World!"
    result = string.concat(s1, s2)  // "Hello, World!"
    print(result)
```

Note: Strings can also be concatenated using the `+` operator.

### string.compare

Compares two strings lexicographically.

**Signature**
```
function string.compare(s1: string, s2: string): int
```

**Parameters**
- `s1`: The first string
- `s2`: The second string

**Returns**
- A negative value if s1 < s2
- Zero if s1 = s2
- A positive value if s1 > s2

**Example**
```
start()
    s1 = "apple"
    s2 = "banana"
    result = string.compare(s1, s2)  // Negative value
    print(result)
```

### string.substr

Extracts a substring from a string.

**Signature**
```
function string.substr(s: string, start: int, length: int): string
```

**Parameters**
- `s`: The input string
- `start`: The starting index (0-based)
- `length`: The number of characters to extract

**Returns**
- A new string containing the specified substring

**Example**
```
start()
    s = "Hello, World!"
    sub = string.substr(s, 7, 5)  // "World"
    print(sub)
```

## Array Operations

The `array` module provides functions for working with arrays.

### array.length

Returns the length of an array.

**Signature**
```
function array.length(arr: array): int
```

**Parameters**
- `arr`: The input array

**Returns**
- The number of elements in the array

**Example**
```
start()
    arr = [1, 2, 3, 4, 5]
    len = array.length(arr)  // 5
    print(len)
```

### array.get

Gets the element at the specified index.

**Signature**
```
function array.get(arr: array, index: int): any
```

**Parameters**
- `arr`: The input array
- `index`: The index (0-based)

**Returns**
- The element at the specified index

**Example**
```
start()
    arr = [10, 20, 30, 40, 50]
    element = array.get(arr, 2)  // 30
    print(element)
```

### array.set

Sets the element at the specified index.

**Signature**
```
function array.set(arr: array, index: int, value: any): void
```

**Parameters**
- `arr`: The input array
- `index`: The index (0-based)
- `value`: The new value

**Example**
```
start()
    arr = [10, 20, 30, 40, 50]
    array.set(arr, 2, 99)  // arr becomes [10, 20, 99, 40, 50]
    print(arr)
```

### array.iterate

Iterates over an array, calling a function for each element.

**Signature**
```
function array.iterate(arr: array, callback: function): void
```

**Parameters**
- `arr`: The input array
- `callback`: A function that takes an element as its parameter

**Example**
```
start()
    arr = [1, 2, 3, 4, 5]
    array.iterate(arr, printElement)

function printElement(element)
    print(element)
```

### array.map

Creates a new array by calling a function on each element of the original array.

**Signature**
```
function array.map(arr: array, callback: function): array
```

**Parameters**
- `arr`: The input array
- `callback`: A function that takes an element as its parameter and returns a new value

**Returns**
- A new array containing the results of calling the callback function on each element

**Example**
```
start()
    arr = [1, 2, 3, 4, 5]
    doubled = array.map(arr, double)  // [2, 4, 6, 8, 10]
    print(doubled)

function double(x)
    return x * 2
```

### array.filter

Creates a new array with all elements that pass the test implemented by the provided function.

**Signature**
```
function array.filter(arr: array, callback: function): array
```

**Parameters**
- `arr`: The input array
- `callback`: A function that takes an element as its parameter and returns a boolean

**Returns**
- A new array containing the elements that pass the test

**Example**
```
start()
    arr = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
    evens = array.filter(arr, isEven)  // [2, 4, 6, 8, 10]
    print(evens)

function isEven(x)
    return x % 2 == 0
```

## Math Functions

The `math` module provides mathematical functions and constants.

### math.sqrt

Calculates the square root of a number.

**Signature**
```
function math.sqrt(x: float): float
```

**Parameters**
- `x`: The input number

**Returns**
- The square root of x

**Example**
```
start()
    x = 16
    result = math.sqrt(x)  // 4.0
    print(result)
```

### math.pow

Raises a number to the power of another number.

**Signature**
```
function math.pow(base: float, exponent: float): float
```

**Parameters**
- `base`: The base number
- `exponent`: The exponent

**Returns**
- base raised to the power of exponent

**Example**
```
start()
    base = 2
    exponent = 3
    result = math.pow(base, exponent)  // 8.0
    print(result)
```

### math.floor

Returns the largest integer less than or equal to a number.

**Signature**
```
function math.floor(x: float): int
```

**Parameters**
- `x`: The input number

**Returns**
- The largest integer less than or equal to x

**Example**
```
start()
    x = 3.7
    result = math.floor(x)  // 3
    print(result)
```

### math.ceil

Returns the smallest integer greater than or equal to a number.

**Signature**
```
function math.ceil(x: float): int
```

**Parameters**
- `x`: The input number

**Returns**
- The smallest integer greater than or equal to x

**Example**
```
start()
    x = 3.2
    result = math.ceil(x)  // 4
    print(result)
```

### math.round

Rounds a number to the nearest integer.

**Signature**
```
function math.round(x: float): int
```

**Parameters**
- `x`: The input number

**Returns**
- The rounded integer

**Example**
```
start()
    x = 3.5
    result = math.round(x)  // 4
    print(result)
```

### math.abs

Returns the absolute value of a number.

**Signature**
```
function math.abs(x: number): number
```

**Parameters**
- `x`: The input number

**Returns**
- The absolute value of x

**Example**
```
start()
    x = -5
    result = math.abs(x)  // 5
    print(result)
```

### Math Constants

The `math` module also provides several mathematical constants:

- `math.PI`: The ratio of a circle's circumference to its diameter (approximately 3.14159)
- `math.E`: The base of natural logarithms (approximately 2.71828)

**Example**
```
start()
    print(math.PI)  // 3.141592653589793
    print(math.E)   // 2.718281828459045
```

## Memory Management

The `memory` module provides functions for managing memory.

### memory.allocate

Allocates a block of memory of the specified size.

**Signature**
```
function memory.allocate(size: int, type_id: int): int
```

**Parameters**
- `size`: The size in bytes
- `type_id`: The type identifier

**Returns**
- A pointer to the allocated memory

**Example**
```
start()
    // Allocate memory for a string (type_id = 1)
    ptr = memory.allocate(10, 1)
    print(ptr)
```

### memory.release

Releases a previously allocated block of memory.

**Signature**
```
function memory.release(ptr: int): void
```

**Parameters**
- `ptr`: A pointer to the memory to release

**Example**
```
start()
    ptr = memory.allocate(10, 1)
    // Use the memory...
    memory.release(ptr)
```

## Error Handling

The `error` module provides functions for error handling.

### error.throw

Throws an error with the specified message.

**Signature**
```
function error.throw(message: string): void
```

**Parameters**
- `message`: The error message

**Example**
```
function divide(a, b)
    if b == 0
        error.throw("Division by zero")
    return a / b
```

### Using onError

The `onError` syntax provides a way to handle errors:

```
result = someFunction() onError:
    // Error handling code
    print("An error occurred")
    return fallbackValue
```

## I/O Operations

The `io` module provides input/output operations.

### io.print

Prints a value to the console.

**Signature**
```
function io.print(value: any): void
```

**Parameters**
- `value`: The value to print

**Example**
```
start()
    io.print("Hello, World!")
```

Note: The `print` function is a shorthand for `io.print`.

### io.printl

Prints a value to the console, followed by a newline.

**Signature**
```
function io.printl(value: any): void
```

**Parameters**
- `value`: The value to print

**Example**
```
start()
    io.printl("Hello, World!")
```

Note: The `printl` function is a shorthand for `io.printl`.

## Type Conversion

The `convert` module provides functions for converting between types.

### convert.toInt

Converts a value to an integer.

**Signature**
```
function convert.toInt(value: any): int
```

**Parameters**
- `value`: The value to convert

**Returns**
- The integer representation of the value

**Example**
```
start()
    s = "42"
    n = convert.toInt(s)  // 42
    print(n)
```

### convert.toFloat

Converts a value to a floating-point number.

**Signature**
```
function convert.toFloat(value: any): float
```

**Parameters**
- `value`: The value to convert

**Returns**
- The floating-point representation of the value

**Example**
```
start()
    s = "3.14"
    n = convert.toFloat(s)  // 3.14
    print(n)
```

### convert.toString

Converts a value to a string.

**Signature**
```
function convert.toString(value: any): string
```

**Parameters**
- `value`: The value to convert

**Returns**
- The string representation of the value

**Example**
```
start()
    n = 42
    s = convert.toString(n)  // "42"
    print(s)
``` 