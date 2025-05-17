use clean_language::{compile, error::CompilerError};

#[test]
fn test_simple_program() -> Result<(), CompilerError> {
    let source = r#"
        // Simple program to test basic functionality
        number x = 42
        number y = 10
        
        // Test arithmetic
        number result = x + y
        
        // Test print
        print "Result is: "
        printl result
        
        // Test error handling
        onError:
            printl "An error occurred"
            x = 0
    "#;

    let wasm_binary = compile(source)?;
    assert!(!wasm_binary.is_empty());
    Ok(())
}

#[test]
fn test_error_handling() -> Result<(), CompilerError> {
    let source = r#"
        number x = 10
        number y = 0
        
        onError:
            printl "Division by zero error"
            x = 42
        
        // This should trigger the error handler
        number result = x / y
    "#;

    let wasm_binary = compile(source)?;
    assert!(!wasm_binary.is_empty());
    Ok(())
}

#[test]
fn test_string_operations() -> Result<(), CompilerError> {
    let source = r#"
        string name = "World"
        string message = "Hello, " + name + "!"
        printl message
    "#;

    let wasm_binary = compile(source)?;
    assert!(!wasm_binary.is_empty());
    Ok(())
} 