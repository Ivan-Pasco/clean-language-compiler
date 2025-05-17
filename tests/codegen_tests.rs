use clean_language_compiler::{
    ast::{Program, Statement, Expression, Function, Type, Operator},
    codegen::CodeGenerator,
    validation::WasmValidator,
};
use crate::test_utils;
use wasmtime::{Store, Module, Instance};

#[test]
fn test_empty_program() {
    let mut codegen = CodeGenerator::new();
    let program = Program {
        statements: vec![],
        functions: vec![],
        constants: vec![],
        classes: vec![],
    };
    
    let result = codegen.generate(&program);
    assert!(result.is_ok(), "Failed to generate WASM: {:?}", result.err());
    
    let wasm_binary = result.unwrap();
    assert!(!wasm_binary.is_empty(), "Generated WASM binary is empty");
    
    // Validate WASM binary
    let validator = WasmValidator::new();
    assert!(validator.validate_and_analyze(&wasm_binary).is_ok());
}

#[test]
fn test_basic_arithmetic() {
    let source = r#"
        number x = 10 + 20 * 2
        number y = (30 - 5) / 5
        number z = x + y
    "#;
    
    let program = test_utils::parse_source(source);
    let wasm_binary = test_utils::generate_wasm(&program);
    
    // Create WebAssembly instance
    let store = Store::default();
    let module = Module::new(&store, &wasm_binary).expect("Failed to create module");
    let instance = Instance::new(&store, &module, &[]).expect("Failed to instantiate module");
    
    // Get memory and exported functions
    let memory = instance.get_memory("memory").expect("Failed to get memory");
    let get_x = instance.get_func("get_x").expect("Failed to get x function");
    let get_y = instance.get_func("get_y").expect("Failed to get y function");
    let get_z = instance.get_func("get_z").expect("Failed to get z function");
    
    // Check results
    let x_result = get_x.call(&[]).expect("Failed to call get_x")[0].unwrap_f64();
    let y_result = get_y.call(&[]).expect("Failed to call get_y")[0].unwrap_f64();
    let z_result = get_z.call(&[]).expect("Failed to call get_z")[0].unwrap_f64();
    
    assert_eq!(x_result, 50.0); // 10 + (20 * 2)
    assert_eq!(y_result, 5.0);  // (30 - 5) / 5
    assert_eq!(z_result, 55.0); // 50 + 5
}

#[test]
fn test_function_generation() {
    let source = r#"
        functions:
            add() returns number
                input:
                    number:
                        - x
                        - y
                return x + y

            multiply() returns number
                input:
                    number:
                        - x
                        - y
                return x * y

        number result1 = add(10, 20)
        number result2 = multiply(5, 6)
    "#;
    
    let program = test_utils::parse_source(source);
    let wasm_binary = test_utils::generate_wasm(&program);
    
    // Create WebAssembly instance
    let store = Store::default();
    let module = Module::new(&store, &wasm_binary).expect("Failed to create module");
    let instance = Instance::new(&store, &module, &[]).expect("Failed to instantiate module");
    
    // Get exported functions
    let add_func = instance.get_func("add").expect("Failed to get add function");
    let multiply_func = instance.get_func("multiply").expect("Failed to get multiply function");
    let get_result1 = instance.get_func("get_result1").expect("Failed to get result1 function");
    let get_result2 = instance.get_func("get_result2").expect("Failed to get result2 function");
    
    // Test function calls
    let add_result = add_func.call(&[wasmtime::Val::F64(10.0), wasmtime::Val::F64(20.0)])
        .expect("Failed to call add")[0].unwrap_f64();
    let multiply_result = multiply_func.call(&[wasmtime::Val::F64(5.0), wasmtime::Val::F64(6.0)])
        .expect("Failed to call multiply")[0].unwrap_f64();
    
    assert_eq!(add_result, 30.0);
    assert_eq!(multiply_result, 30.0);
    
    // Test stored results
    let result1 = get_result1.call(&[]).expect("Failed to call get_result1")[0].unwrap_f64();
    let result2 = get_result2.call(&[]).expect("Failed to call get_result2")[0].unwrap_f64();
    
    assert_eq!(result1, 30.0);
    assert_eq!(result2, 30.0);
}

#[test]
fn test_memory_operations() {
    let source = r#"
        string message = "Hello, World!"
        number[] numbers = [1, 2, 3, 4, 5]
        
        print message
        iterate n in numbers
            print n
    "#;
    
    let program = test_utils::parse_source(source);
    let wasm_binary = test_utils::generate_wasm(&program);
    
    // Create WebAssembly instance
    let store = Store::default();
    let module = Module::new(&store, &wasm_binary).expect("Failed to create module");
    let instance = Instance::new(&store, &module, &[]).expect("Failed to instantiate module");
    
    // Get memory and functions
    let memory = instance.get_memory("memory").expect("Failed to get memory");
    let get_message = instance.get_func("get_message").expect("Failed to get message function");
    let get_numbers = instance.get_func("get_numbers").expect("Failed to get numbers function");
    
    // Check string in memory
    let message_ptr = get_message.call(&[]).expect("Failed to call get_message")[0].unwrap_i32() as u32;
    let message_len = memory.data(&store)[message_ptr as usize] as usize;
    let message_bytes = &memory.data(&store)[(message_ptr + 1) as usize..(message_ptr + 1 + message_len) as usize];
    let message = String::from_utf8_lossy(message_bytes);
    assert_eq!(message, "Hello, World!");
    
    // Check array in memory
    let numbers_ptr = get_numbers.call(&[]).expect("Failed to call get_numbers")[0].unwrap_i32() as u32;
    let numbers_len = memory.data(&store)[numbers_ptr as usize] as usize;
    let numbers_data = &memory.data(&store)[(numbers_ptr + 1) as usize..(numbers_ptr + 1 + numbers_len * 8) as usize];
    let numbers: Vec<f64> = numbers_data.chunks(8)
        .map(|chunk| f64::from_le_bytes(chunk.try_into().unwrap()))
        .collect();
    assert_eq!(numbers, vec![1.0, 2.0, 3.0, 4.0, 5.0]);
}

#[test]
fn test_control_flow() {
    let source = r#"
        number x = 10
        number y = 0
        
        if x > 5
            y = 1
        
        iterate i in [1, 2, 3]
            y = y + i
        
        i = from 1 to 3
            y = y + i
    "#;
    
    let program = test_utils::parse_source(source);
    let wasm_binary = test_utils::generate_wasm(&program);
    
    // Create WebAssembly instance
    let store = Store::default();
    let module = Module::new(&store, &wasm_binary).expect("Failed to create module");
    let instance = Instance::new(&store, &module, &[]).expect("Failed to instantiate module");
    
    // Get y value after all operations
    let get_y = instance.get_func("get_y").expect("Failed to get y function");
    let final_y = get_y.call(&[]).expect("Failed to call get_y")[0].unwrap_f64();
    
    // y should be: 1 (from if) + (1+2+3) (from iterate) + (1+2+3) (from from-to) = 13
    assert_eq!(final_y, 13.0);
}

#[test]
fn test_class_generation() {
    let source = r#"
        class Point
            public
                number x = 0
                number y = 0
            
            public
                number distance() returns number
                    return x * x + y * y

        object p = new Point()
        p.x = 3
        p.y = 4
        number d = p.distance()
    "#;
    
    let program = test_utils::parse_source(source);
    let wasm_binary = test_utils::generate_wasm(&program);
    
    // Create WebAssembly instance
    let store = Store::default();
    let module = Module::new(&store, &wasm_binary).expect("Failed to create module");
    let instance = Instance::new(&store, &module, &[]).expect("Failed to instantiate module");
    
    // Get distance result
    let get_d = instance.get_func("get_d").expect("Failed to get d function");
    let distance = get_d.call(&[]).expect("Failed to call get_d")[0].unwrap_f64();
    
    assert_eq!(distance, 25.0); // 3*3 + 4*4 = 25
}

#[test]
fn test_error_handling() {
    let source = r#"
        number x = 0
        
        onError:
            x = 42
            print "Error occurred"
        
        // Trigger error (e.g., division by zero)
        number y = 10 / x
    "#;
    
    let program = test_utils::parse_source(source);
    let wasm_binary = test_utils::generate_wasm(&program);
    
    // Create WebAssembly instance
    let store = Store::default();
    let module = Module::new(&store, &wasm_binary).expect("Failed to create module");
    let instance = Instance::new(&store, &module, &[]).expect("Failed to instantiate module");
    
    // Get x value after error
    let get_x = instance.get_func("get_x").expect("Failed to get x function");
    let x_after_error = get_x.call(&[]).expect("Failed to call get_x")[0].unwrap_f64();
    
    assert_eq!(x_after_error, 42.0); // Error handler should have executed
} 