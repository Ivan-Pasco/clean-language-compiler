# Clean Language Specification

## Overview

Clean Language is a modern, type-safe programming language that compiles to WebAssembly. It combines the readability of JavaScript with the safety of Rust, providing a simple yet powerful syntax for developers.

## Core Features

- Strong static typing with type inference
- First-class functions and closures
- Object-oriented programming with classes and inheritance
- Built-in error handling and recovery
- WebAssembly compilation target
- Comprehensive standard library

## Data Types

### Primitive Types

- `integer` - 32-bit signed integers
- `number` - 64-bit floating point numbers
- `string` - UTF-8 encoded text
- `boolean` - true/false values

### Complex Types

- `list<T>` - Dynamic lists of type T
- `Matrix<T>` - 2D matrices of type T
- Custom classes and objects

## Standard Library Modules

Clean Language provides a comprehensive standard library organized into modules. All standard library functions are accessed using the `module.function()` syntax, except for console functions which are available directly.

### Module System

The standard library is organized into the following modules:

- **http** - HTTP client functionality
- **math** - Mathematical functions and constants
- **list** - List manipulation functions
- **string** - String processing functions
- **file** - File I/O operations
- **console** - Console I/O (accessible directly without module prefix)

## HTTP Module

The HTTP module provides easy-to-use functions for making HTTP requests. All HTTP functions return strings and handle errors gracefully.

### Basic HTTP Operations

These functions provide the core HTTP methods for common web operations:

#### http.get(url)
Performs an HTTP GET request to retrieve data from a server.

**Parameters:**
- `url` (string): The URL to request

**Returns:** 
- `string`: The response body from the server

**Example:**
```clean
start()
    string response = http.get("https://api.github.com/users/octocat")
    print("User data:")
    print(response)
```

#### http.post(url, data)
Performs an HTTP POST request to send data to a server.

**Parameters:**
- `url` (string): The URL to send the request to
- `data` (string): The data to send in the request body

**Returns:** 
- `string`: The response body from the server

**Example:**
```clean
start()
    string userData = "name=John&email=john@example.com"
    string response = http.post("https://api.example.com/users", userData)
    print("Server response:")
    print(response)
```

#### http.put(url, data)
Performs an HTTP PUT request to update data on a server.

**Parameters:**
- `url` (string): The URL to send the request to
- `data` (string): The data to send in the request body

**Returns:** 
- `string`: The response body from the server

**Example:**
```clean
start()
    string updateData = "name=Jane&email=jane@example.com"
    string response = http.put("https://api.example.com/users/123", updateData)
    print("Update response:")
    print(response)
```

#### http.delete(url)
Performs an HTTP DELETE request to remove data from a server.

**Parameters:**
- `url` (string): The URL to delete

**Returns:** 
- `string`: The response body from the server

**Example:**
```clean
start()
    string response = http.delete("https://api.example.com/users/123")
    print("Delete response:")
    print(response)
```

#### http.patch(url, data)
Performs an HTTP PATCH request to partially update data on a server.

**Parameters:**
- `url` (string): The URL to send the request to
- `data` (string): The data to send in the request body

**Returns:** 
- `string`: The response body from the server

**Example:**
```clean
start()
    string patchData = "email=newemail@example.com"
    string response = http.patch("https://api.example.com/users/123", patchData)
    print("Patch response:")
    print(response)
```

### Advanced HTTP Operations

#### http.postJson(url, jsonData)
Performs an HTTP POST request with JSON data, automatically setting the appropriate Content-Type header.

**Parameters:**
- `url` (string): The URL to send the request to
- `jsonData` (string): The JSON data to send in the request body

**Returns:** 
- `string`: The response body from the server

**Example:**
```clean
start()
    string jsonData = "{\"name\": \"John\", \"age\": 30}"
    string response = http.postJson("https://api.example.com/users", jsonData)
    print("JSON response:")
    print(response)
```

#### http.putJson(url, jsonData)
Performs an HTTP PUT request with JSON data.

**Parameters:**
- `url` (string): The URL to send the request to
- `jsonData` (string): The JSON data to send in the request body

**Returns:** 
- `string`: The response body from the server

#### http.patchJson(url, jsonData)
Performs an HTTP PATCH request with JSON data.

**Parameters:**
- `url` (string): The URL to send the request to
- `jsonData` (string): The JSON data to send in the request body

**Returns:** 
- `string`: The response body from the server

### HTTP Configuration

#### http.setTimeout(timeoutMs)
Sets the timeout for HTTP requests in milliseconds.

**Parameters:**
- `timeoutMs` (integer): The timeout in milliseconds

**Returns:** 
- `void`

**Example:**
```clean
start()
    http.setTimeout(5000)  // 5 second timeout
    string response = http.get("https://api.example.com/data")
```

#### http.setUserAgent(userAgent)
Sets the User-Agent header for HTTP requests.

**Parameters:**
- `userAgent` (string): The User-Agent string to use

**Returns:** 
- `void`

**Example:**
```clean
start()
    http.setUserAgent("Clean Language HTTP Client/1.0")
    string response = http.get("https://api.example.com/data")
```

#### http.enableCookies(enable)
Enables or disables cookie handling for HTTP requests.

**Parameters:**
- `enable` (boolean): Whether to enable cookie handling

**Returns:** 
- `void`

**Example:**
```clean
start()
    http.enableCookies(true)
    string response = http.get("https://api.example.com/data")
```

### Response Information

#### http.getResponseCode()
Gets the HTTP response code from the last request.

**Returns:** 
- `integer`: The HTTP response code (e.g., 200, 404, 500)

**Example:**
```clean
start()
    string response = http.get("https://api.example.com/data")
    integer code = http.getResponseCode()
    print("Response code: " + code)
```

#### http.getResponseHeaders()
Gets the HTTP response headers from the last request as a string.

**Returns:** 
- `string`: The response headers

**Example:**
```clean
start()
    string response = http.get("https://api.example.com/data")
    string headers = http.getResponseHeaders()
    print("Response headers:")
    print(headers)
```

### URL Utilities

#### http.encodeUrl(url)
URL-encodes a string to make it safe for use in URLs.

**Parameters:**
- `url` (string): The string to encode

**Returns:** 
- `string`: The URL-encoded string

**Example:**
```clean
start()
    string originalUrl = "https://example.com/path with spaces"
    string encodedUrl = http.encodeUrl(originalUrl)
    print("Encoded URL: " + encodedUrl)
```

#### http.decodeUrl(encodedUrl)
URL-decodes a string.

**Parameters:**
- `encodedUrl` (string): The URL-encoded string to decode

**Returns:** 
- `string`: The decoded string

**Example:**
```clean
start()
    string encodedUrl = "https%3A//example.com/path%20with%20spaces"
    string decodedUrl = http.decodeUrl(encodedUrl)
    print("Decoded URL: " + decodedUrl)
```

### Complete HTTP Example

```clean
start()
    // Configure HTTP client
    http.setTimeout(10000)  // 10 second timeout
    http.setUserAgent("Clean Language HTTP Client/1.0")
    http.enableCookies(true)
    
    // Make a simple GET request
    string apiUrl = "https://api.github.com/users/octocat"
    string userData = http.get(apiUrl)
    
    // Check response
    integer responseCode = http.getResponseCode()
    if responseCode == 200
        print("Success! User data received:")
        print(userData)
    else
        print("Error: HTTP " + responseCode)
    
    // Make a POST request with JSON data
    string jsonData = "{\"name\": \"Test User\", \"email\": \"test@example.com\"}"
    string createResponse = http.postJson("https://api.example.com/users", jsonData)
    print("User created:")
    print(createResponse)
    
    // URL encoding example
    string searchTerm = "clean language programming"
    string encodedTerm = http.encodeUrl(searchTerm)
    string searchUrl = "https://api.example.com/search?q=" + encodedTerm
    string searchResults = http.get(searchUrl)
    print("Search results:")
    print(searchResults)
```

## Error Handling

HTTP functions handle errors gracefully by returning error information in the response string. Applications should check the response code using `http.getResponseCode()` to determine if a request was successful.

## Best Practices

1. **Always set reasonable timeouts** using `http.setTimeout()` to prevent hanging requests
2. **Check response codes** using `http.getResponseCode()` to handle errors appropriately
3. **Use appropriate Content-Type headers** by using JSON-specific functions when sending JSON data
4. **URL-encode parameters** using `http.encodeUrl()` when constructing URLs with user input
5. **Configure User-Agent** using `http.setUserAgent()` to identify your application

## Security Considerations

- All HTTP requests are made over HTTPS when possible
- User input should be validated before being sent in HTTP requests
- Sensitive data should be handled carefully and not logged or exposed
- Consider rate limiting and proper error handling for production applications

## Math Module

The Math module provides mathematical functions and constants for numerical computations.

### Mathematical Functions

#### math.sin(x), math.cos(x), math.tan(x)
Trigonometric functions for angle calculations.

**Parameters:**
- `x` (number): The angle in radians

**Returns:** 
- `number`: The trigonometric result

**Example:**
```clean
start()
    number angle = 1.5708  // π/2 radians
    number sine = math.sin(angle)
    number cosine = math.cos(angle)
    print("sin(π/2) = " + sine)
    print("cos(π/2) = " + cosine)
```

#### math.asin(x), math.acos(x), math.atan(x)
Inverse trigonometric functions.

**Parameters:**
- `x` (number): The input value

**Returns:** 
- `number`: The angle in radians

#### math.atan2(y, x)
Two-argument arctangent function.

**Parameters:**
- `y` (number): The y-coordinate
- `x` (number): The x-coordinate

**Returns:** 
- `number`: The angle in radians

#### math.sqrt(x)
Square root function.

**Parameters:**
- `x` (number): The input value

**Returns:** 
- `number`: The square root of x

#### math.pow(base, exponent)
Power function.

**Parameters:**
- `base` (number): The base value
- `exponent` (number): The exponent

**Returns:** 
- `number`: base raised to the power of exponent

#### math.log(x), math.log10(x), math.log2(x)
Logarithmic functions.

**Parameters:**
- `x` (number): The input value

**Returns:** 
- `number`: The logarithm of x

#### math.exp(x), math.exp2(x)
Exponential functions.

**Parameters:**
- `x` (number): The exponent

**Returns:** 
- `number`: e^x or 2^x

#### math.abs(x)
Absolute value function.

**Parameters:**
- `x` (number): The input value

**Returns:** 
- `number`: The absolute value of x

#### math.floor(x), math.ceil(x), math.round(x)
Rounding functions.

**Parameters:**
- `x` (number): The input value

**Returns:** 
- `number`: The rounded value

#### math.min(a, b), math.max(a, b)
Minimum and maximum functions.

**Parameters:**
- `a` (number): First value
- `b` (number): Second value

**Returns:** 
- `number`: The minimum or maximum value

### Mathematical Constants

#### math.pi()
Returns the mathematical constant π (pi).

**Returns:** 
- `number`: 3.141592653589793

#### math.e()
Returns the mathematical constant e (Euler's number).

**Returns:** 
- `number`: 2.718281828459045

#### math.tau()
Returns the mathematical constant τ (tau = 2π).

**Returns:** 
- `number`: 6.283185307179586

## list Module

The list module provides functions for manipulating lists and collections.

### list Operations

#### list.push(arr, element)
Adds an element to the end of a list.

**Parameters:**
- `arr` (list<T>): The list to modify
- `element` (T): The element to add

**Returns:** 
- `list<T>`: The modified list

#### list.pop(arr)
Removes and returns the last element of a list.

**Parameters:**
- `arr` (list<T>): The list to modify

**Returns:** 
- `T`: The removed element

#### list.get(arr, index)
Returns the element at the specified index.

**Parameters:**
- `arr` (list<T>): The list to access
- `index` (integer): The index to access

**Returns:** 
- `T`: The element at the specified index

#### list.set(arr, index, value)
Sets the element at the specified index.

**Parameters:**
- `arr` (list<T>): The list to modify
- `index` (integer): The index to set
- `value` (T): The value to set

**Returns:** 
- `list<T>`: The modified list

#### list.length(arr)
Returns the number of elements in a list.

**Parameters:**
- `arr` (list<T>): The list to measure

**Returns:** 
- `integer`: The number of elements

#### list.slice(arr, start, end)
Extracts a section of a list.

**Parameters:**
- `arr` (list<T>): The source list
- `start` (integer): The start index
- `end` (integer): The end index (exclusive)

**Returns:** 
- `list<T>`: A new list containing the extracted elements

#### list.concat(arr1, arr2)
Concatenates two lists.

**Parameters:**
- `arr1` (list<T>): The first list
- `arr2` (list<T>): The second list

**Returns:** 
- `list<T>`: A new list containing elements from both lists

#### list.join(arr, separator)
Joins list elements into a string.

**Parameters:**
- `arr` (list<T>): The list to join
- `separator` (string): The separator string

**Returns:** 
- `string`: The joined string

#### list.reverse(arr)
Reverses the order of elements in a list.

**Parameters:**
- `arr` (list<T>): The list to reverse

**Returns:** 
- `list<T>`: The reversed list

#### list.indexOf(arr, element)
Finds the index of an element in a list.

**Parameters:**
- `arr` (list<T>): The list to search
- `element` (T): The element to find

**Returns:** 
- `integer`: The index of the element, or -1 if not found

#### list.contains(arr, element)
Checks if a list contains a specific element.

**Parameters:**
- `arr` (list<T>): The list to search
- `element` (T): The element to find

**Returns:** 
- `boolean`: True if the element is found, false otherwise

## String Module

The String module provides functions for string manipulation and processing.

### String Operations

#### string.length(str)
Returns the length of a string.

**Parameters:**
- `str` (string): The string to measure

**Returns:** 
- `integer`: The length of the string

#### string.charAt(str, index)
Returns the character at the specified index.

**Parameters:**
- `str` (string): The source string
- `index` (integer): The character index

**Returns:** 
- `string`: The character at the specified index

#### string.substring(str, start, end)
Extracts a substring from a string.

**Parameters:**
- `str` (string): The source string
- `start` (integer): The start index
- `end` (integer): The end index (exclusive)

**Returns:** 
- `string`: The extracted substring

#### string.indexOf(str, searchString)
Finds the index of a substring.

**Parameters:**
- `str` (string): The source string
- `searchString` (string): The substring to find

**Returns:** 
- `integer`: The index of the substring, or -1 if not found

#### string.lastIndexOf(str, searchString)
Finds the last index of a substring.

**Parameters:**
- `str` (string): The source string
- `searchString` (string): The substring to find

**Returns:** 
- `integer`: The last index of the substring, or -1 if not found

#### string.replace(str, searchString, replaceString)
Replaces the first occurrence of a substring.

**Parameters:**
- `str` (string): The source string
- `searchString` (string): The substring to replace
- `replaceString` (string): The replacement string

**Returns:** 
- `string`: The modified string

#### string.replaceAll(str, searchString, replaceString)
Replaces all occurrences of a substring.

**Parameters:**
- `str` (string): The source string
- `searchString` (string): The substring to replace
- `replaceString` (string): The replacement string

**Returns:** 
- `string`: The modified string

#### string.toUpperCase(str)
Converts a string to uppercase.

**Parameters:**
- `str` (string): The source string

**Returns:** 
- `string`: The uppercase string

#### string.toLowerCase(str)
Converts a string to lowercase.

**Parameters:**
- `str` (string): The source string

**Returns:** 
- `string`: The lowercase string

#### string.trim(str)
Removes whitespace from both ends of a string.

**Parameters:**
- `str` (string): The source string

**Returns:** 
- `string`: The trimmed string

#### string.startsWith(str, prefix)
Checks if a string starts with a specific prefix.

**Parameters:**
- `str` (string): The source string
- `prefix` (string): The prefix to check

**Returns:** 
- `boolean`: True if the string starts with the prefix, false otherwise

#### string.endsWith(str, suffix)
Checks if a string ends with a specific suffix.

**Parameters:**
- `str` (string): The source string
- `suffix` (string): The suffix to check

**Returns:** 
- `boolean`: True if the string ends with the suffix, false otherwise

#### string.contains(str, searchString)
Checks if a string contains a specific substring.

**Parameters:**
- `str` (string): The source string
- `searchString` (string): The substring to find

**Returns:** 
- `boolean`: True if the substring is found, false otherwise

#### string.isEmpty(str)
Checks if a string is empty.

**Parameters:**
- `str` (string): The string to check

**Returns:** 
- `boolean`: True if the string is empty, false otherwise

## File Module

The File module provides functions for file I/O operations.

### File Operations

#### file.read(filename)
Reads the contents of a file.

**Parameters:**
- `filename` (string): The path to the file to read

**Returns:** 
- `string`: The contents of the file

#### file.write(filename, content)
Writes content to a file.

**Parameters:**
- `filename` (string): The path to the file to write
- `content` (string): The content to write

**Returns:** 
- `boolean`: True if successful, false otherwise

#### file.append(filename, content)
Appends content to a file.

**Parameters:**
- `filename` (string): The path to the file to append to
- `content` (string): The content to append

**Returns:** 
- `boolean`: True if successful, false otherwise

#### file.exists(filename)
Checks if a file exists.

**Parameters:**
- `filename` (string): The path to the file to check

**Returns:** 
- `boolean`: True if the file exists, false otherwise

#### file.delete(filename)
Deletes a file.

**Parameters:**
- `filename` (string): The path to the file to delete

**Returns:** 
- `boolean`: True if successful, false otherwise

## Console Module

The Console module provides functions for console I/O operations. **Note:** Console functions are accessible directly without the module prefix.

### Console Functions

#### print(message)
Prints a message to the console.

**Parameters:**
- `message` (string): The message to print

**Returns:** 
- `void`

#### input()
Reads a line of input from the console.

**Returns:** 
- `string`: The input string

#### inputInteger()
Reads an integer from the console.

**Returns:** 
- `integer`: The input integer

#### inputNumber()
Reads a number from the console.

**Returns:** 
- `number`: The input number

#### inputYesNo()
Reads a yes/no response from the console.

**Returns:** 
- `boolean`: True for yes, false for no

## Language Syntax

### Operators

#### Arithmetic Operators
```clean
a + b       // Addition
a - b       // Subtraction
a * b       // Multiplication
a / b       // Division
a % b       // Modulo
a ^ b       // Exponentiation (power)
```

#### Comparison Operators
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

#### Logical Operators
```clean
a and b     // Logical AND
a or b      // Logical OR
a not b     // Logical NOT (binary, equivalent to !=)
// Note: Unary not operator not yet implemented
```

#### Operator Precedence
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

### Functions

```clean
function returnType functionName(paramType paramName)
    // Function body
    return value
```

### Classes

```clean
class ClassName
    // Fields
    type fieldName
    
    // Constructor
    constructor(paramType paramName)
        // Initialize fields
    
    // Methods
    functions:
        returnType methodName(paramType paramName)
            // Method body
```

### Control Flow

```clean
// If statements
if condition
    // statements
else
    // statements

// For loops
for integer i = 0; i < 10; i++
    // statements

// While loops
while condition
    // statements
```

### Variable Declarations

```clean
// Explicit typing
integer x = 5
string name = "John"
boolean flag = true

// Type inference
let y = 10  // inferred as integer
let message = "Hello"  // inferred as string
```

## Standard Library

In addition to HTTP functions, Clean Language provides a comprehensive standard library organized into modules:

- **Math Module**: `math.sin()`, `math.cos()`, `math.sqrt()`, `math.abs()`, etc.
- **list Module**: `list.push()`, `list.pop()`, `list.join()`, `list.slice()`, etc.
- **String Module**: `string.length()`, `string.indexOf()`, `string.substring()`, `string.replace()`, etc.
- **File Module**: `file.read()`, `file.write()`, `file.exists()`, `file.delete()`, etc.
- **Console Functions**: `print()`, `input()`, `inputInteger()`, `inputNumber()`, etc. (accessible directly)

## Compilation

Clean Language compiles to WebAssembly, providing:

- **Fast execution**: Near-native performance
- **Portability**: Runs on any WebAssembly-compatible environment
- **Security**: Sandboxed execution environment
- **Interoperability**: Easy integration with existing systems

## Development Tools

- **Compiler**: `cargo run --bin clean-language-compiler`
- **REPL**: Interactive development environment
- **Testing**: Built-in testing framework
- **Debugging**: Source-level debugging support