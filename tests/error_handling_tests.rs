use clean_language_compiler::{
    parser::CleanParser,
    codegen::CodeGenerator,
};

#[test]
fn test_basic_error_handling() {
    let source = r#"
        start()
            number result = 10
            
            // Division by zero will trigger error
            result = 10 / 0
            onError:
                result = 42
            
            // Result should be 42 after error handling
            return result
    "#;
    
    // Parse and compile
    let program = CleanParser::parse_program(source).expect("Failed to parse");
    let mut code_gen = CodeGenerator::new();
    let wasm_binary = code_gen.generate(&program).expect("Failed to generate WASM");
    
    // Execute with wasmtime
    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, &wasm_binary).expect("Failed to create module");
    let mut store = wasmtime::Store::new(&engine, ());
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("Failed to instantiate module");
    
    // Get start function and execute
    let start = instance.get_func(&mut store, "start").expect("Failed to get start function");
    let mut results = vec![wasmtime::Val::I32(0)];
    start.call(&mut store, &[], &mut results).expect("Failed to execute");
    
    // Result should be 42 (error handler was executed)
    assert_eq!(results[0].unwrap_i32(), 42);
}

#[test]
fn test_error_variable_access() {
    let source = r#"
        start()
            // Call function that will throw error with code 123
            throwError()
            onError:
                // Error variable should contain error code 123
                return error
        
        throwError()
            throw 123
    "#;
    
    // Parse and compile
    let program = CleanParser::parse_program(source).expect("Failed to parse");
    let mut code_gen = CodeGenerator::new();
    let wasm_binary = code_gen.generate(&program).expect("Failed to generate WASM");
    
    // Execute with wasmtime
    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, &wasm_binary).expect("Failed to create module");
    let mut store = wasmtime::Store::new(&engine, ());
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("Failed to instantiate module");
    
    // Get start function and execute
    let start = instance.get_func(&mut store, "start").expect("Failed to get start function");
    let mut results = vec![wasmtime::Val::I32(0)];
    start.call(&mut store, &[], &mut results).expect("Failed to execute");
    
    // Result should be 123 (error code from throw)
    assert_eq!(results[0].unwrap_i32(), 123);
}

#[test]
fn test_nested_error_handling() {
    let source = r#"
        start()
            number result = 0
            
            // Outer error handler
            outer()
            onError:
                result = error * 10
            
            return result
        
        outer()
            // Inner error handler
            inner()
            onError:
                // Rethrow with modified error code
                throw error + 1
        
        inner()
            // Throw original error
            throw 5
    "#;
    
    // Parse and compile
    let program = CleanParser::parse_program(source).expect("Failed to parse");
    let mut code_gen = CodeGenerator::new();
    let wasm_binary = code_gen.generate(&program).expect("Failed to generate WASM");
    
    // Execute with wasmtime
    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, &wasm_binary).expect("Failed to create module");
    let mut store = wasmtime::Store::new(&engine, ());
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("Failed to instantiate module");
    
    // Get start function and execute
    let start = instance.get_func(&mut store, "start").expect("Failed to get start function");
    let mut results = vec![wasmtime::Val::I32(0)];
    start.call(&mut store, &[], &mut results).expect("Failed to execute");
    
    // Result should be 60 ((5+1)*10)
    assert_eq!(results[0].unwrap_i32(), 60);
} 