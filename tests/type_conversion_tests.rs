use clean_language_compiler::{
    parser::CleanParser,
    codegen::CodeGenerator,
};

#[test]
fn test_any_type_conversion() {
    let source = r#"
        start()
            // Test Any type with different values
            Any value = 42
            assert(value == 42)
            
            value = 3.14
            assert(value == 3.14)
            
            value = "hello"
            assert(value == "hello")
            
            value = true
            assert(value == true)
            
            // Test Any type in function parameters
            assert(identity(42) == 42)
            assert(identity(3.14) == 3.14)
            assert(identity("hello") == "hello")
            assert(identity(true) == true)
            
            // Test Any type in function return values
            assert(add(1, 2) == 3)
            assert(add(1.5, 2.5) == 4.0)
            
            // Test Any type in arrays
            Array<Any> arr = [1, 2.5, "hello", true]
            assert(arr[0] == 1)
            assert(arr[1] == 2.5)
            assert(arr[2] == "hello")
            assert(arr[3] == true)
            
            return 0
        
        Any identity(Any value)
            return value
        
        Any add(Any a, Any b)
            return a + b
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
    
    // Result should be 0 (all assertions passed)
    assert_eq!(results[0].unwrap_i32(), 0);
}

#[test]
fn test_any_type_error_handling() {
    let source = r#"
        start()
            // Test Any type with error handling
            Any value = 42
            
            value = 10.0 / 0.0
            onError:
                value = "error occurred"
            
            assert(value == "error occurred")
            
            // Test Any type with nested error handling
            value = 0.0
            
            outer()
            onError:
                value = error
            
            assert(value == 123.0)
            
            return 0
        
        outer()
            inner()
            onError:
                throw error + 1.0
        
        inner()
            throw 122.0
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
    
    // Result should be 0 (all assertions passed)
    assert_eq!(results[0].unwrap_i32(), 0);
} 