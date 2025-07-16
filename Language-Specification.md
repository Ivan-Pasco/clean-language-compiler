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

- `Array<T>` - Dynamic arrays of type T
- `Matrix<T>` - 2D matrices of type T
- Custom classes and objects

## HTTP Client Library

Clean Language includes a built-in HTTP client library that provides easy-to-use functions for making HTTP requests. All HTTP functions return strings and handle errors gracefully.

### Basic HTTP Operations

These functions provide the core HTTP methods for common web operations:

#### httpGet(url)
Performs an HTTP GET request to retrieve data from a server.

**Parameters:**
- `url` (string): The URL to request

**Returns:** 
- `string`: The response body from the server

**Example:**
```clean
start()
    string response = httpGet("https://api.github.com/users/octocat")
    print("User data:")
    print(response)
```

#### httpPost(url, data)
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
    string response = httpPost("https://api.example.com/users", userData)
    print("Server response:")
    print(response)
```

#### httpPut(url, data)
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
    string response = httpPut("https://api.example.com/users/123", updateData)
    print("Update response:")
    print(response)
```

#### httpDelete(url)
Performs an HTTP DELETE request to remove data from a server.

**Parameters:**
- `url` (string): The URL to delete

**Returns:** 
- `string`: The response body from the server

**Example:**
```clean
start()
    string response = httpDelete("https://api.example.com/users/123")
    print("Delete response:")
    print(response)
```

#### httpPatch(url, data)
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
    string response = httpPatch("https://api.example.com/users/123", patchData)
    print("Patch response:")
    print(response)
```

### Advanced HTTP Operations

#### httpPostJson(url, jsonData)
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
    string response = httpPostJson("https://api.example.com/users", jsonData)
    print("JSON response:")
    print(response)
```

#### httpPutJson(url, jsonData)
Performs an HTTP PUT request with JSON data.

**Parameters:**
- `url` (string): The URL to send the request to
- `jsonData` (string): The JSON data to send in the request body

**Returns:** 
- `string`: The response body from the server

#### httpPatchJson(url, jsonData)
Performs an HTTP PATCH request with JSON data.

**Parameters:**
- `url` (string): The URL to send the request to
- `jsonData` (string): The JSON data to send in the request body

**Returns:** 
- `string`: The response body from the server

### HTTP Configuration

#### httpSetTimeout(timeoutMs)
Sets the timeout for HTTP requests in milliseconds.

**Parameters:**
- `timeoutMs` (integer): The timeout in milliseconds

**Returns:** 
- `void`

**Example:**
```clean
start()
    httpSetTimeout(5000)  // 5 second timeout
    string response = httpGet("https://api.example.com/data")
```

#### httpSetUserAgent(userAgent)
Sets the User-Agent header for HTTP requests.

**Parameters:**
- `userAgent` (string): The User-Agent string to use

**Returns:** 
- `void`

**Example:**
```clean
start()
    httpSetUserAgent("Clean Language HTTP Client/1.0")
    string response = httpGet("https://api.example.com/data")
```

#### httpEnableCookies(enable)
Enables or disables cookie handling for HTTP requests.

**Parameters:**
- `enable` (boolean): Whether to enable cookie handling

**Returns:** 
- `void`

**Example:**
```clean
start()
    httpEnableCookies(true)
    string response = httpGet("https://api.example.com/data")
```

### Response Information

#### httpGetResponseCode()
Gets the HTTP response code from the last request.

**Returns:** 
- `integer`: The HTTP response code (e.g., 200, 404, 500)

**Example:**
```clean
start()
    string response = httpGet("https://api.example.com/data")
    integer code = httpGetResponseCode()
    print("Response code: " + code)
```

#### httpGetResponseHeaders()
Gets the HTTP response headers from the last request as a string.

**Returns:** 
- `string`: The response headers

**Example:**
```clean
start()
    string response = httpGet("https://api.example.com/data")
    string headers = httpGetResponseHeaders()
    print("Response headers:")
    print(headers)
```

### URL Utilities

#### httpEncodeUrl(url)
URL-encodes a string to make it safe for use in URLs.

**Parameters:**
- `url` (string): The string to encode

**Returns:** 
- `string`: The URL-encoded string

**Example:**
```clean
start()
    string originalUrl = "https://example.com/path with spaces"
    string encodedUrl = httpEncodeUrl(originalUrl)
    print("Encoded URL: " + encodedUrl)
```

#### httpDecodeUrl(encodedUrl)
URL-decodes a string.

**Parameters:**
- `encodedUrl` (string): The URL-encoded string to decode

**Returns:** 
- `string`: The decoded string

**Example:**
```clean
start()
    string encodedUrl = "https%3A//example.com/path%20with%20spaces"
    string decodedUrl = httpDecodeUrl(encodedUrl)
    print("Decoded URL: " + decodedUrl)
```

### Complete HTTP Example

```clean
start()
    // Configure HTTP client
    httpSetTimeout(10000)  // 10 second timeout
    httpSetUserAgent("Clean Language HTTP Client/1.0")
    httpEnableCookies(true)
    
    // Make a simple GET request
    string apiUrl = "https://api.github.com/users/octocat"
    string userData = httpGet(apiUrl)
    
    // Check response
    integer responseCode = httpGetResponseCode()
    if responseCode == 200
        print("Success! User data received:")
        print(userData)
    else
        print("Error: HTTP " + responseCode)
    
    // Make a POST request with JSON data
    string jsonData = "{\"name\": \"Test User\", \"email\": \"test@example.com\"}"
    string createResponse = httpPostJson("https://api.example.com/users", jsonData)
    print("User created:")
    print(createResponse)
    
    // URL encoding example
    string searchTerm = "clean language programming"
    string encodedTerm = httpEncodeUrl(searchTerm)
    string searchUrl = "https://api.example.com/search?q=" + encodedTerm
    string searchResults = httpGet(searchUrl)
    print("Search results:")
    print(searchResults)
```

## Error Handling

HTTP functions handle errors gracefully by returning error information in the response string. Applications should check the response code using `httpGetResponseCode()` to determine if a request was successful.

## Best Practices

1. **Always set reasonable timeouts** using `httpSetTimeout()` to prevent hanging requests
2. **Check response codes** using `httpGetResponseCode()` to handle errors appropriately
3. **Use appropriate Content-Type headers** by using JSON-specific functions when sending JSON data
4. **URL-encode parameters** using `httpEncodeUrl()` when constructing URLs with user input
5. **Configure User-Agent** using `httpSetUserAgent()` to identify your application

## Security Considerations

- All HTTP requests are made over HTTPS when possible
- User input should be validated before being sent in HTTP requests
- Sensitive data should be handled carefully and not logged or exposed
- Consider rate limiting and proper error handling for production applications

## Language Syntax

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

In addition to HTTP functions, Clean Language provides:

- **String operations**: `length()`, `indexOf()`, `substring()`, `replace()`, etc.
- **Array operations**: `push()`, `pop()`, `join()`, `slice()`, etc.
- **Math functions**: `sin()`, `cos()`, `sqrt()`, `abs()`, etc.
- **Console I/O**: `print()`, `input()`, `inputInteger()`, etc.
- **File operations**: `file_read()`, `file_write()`, `file_exists()`, etc.

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