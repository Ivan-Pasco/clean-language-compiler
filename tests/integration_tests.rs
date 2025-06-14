use clean_language_compiler::compile;
use wasmtime::{Engine, Module, Store, Instance, Func, Val};
use std::time::Duration;

const SIMPLE_FUNCTION_WAT: &str = r#"
(module
  (func $simple_function (result i32)
    i32.const 42
  )
  (export "simple_function" (func $simple_function))
)
"#;

const TEST_PROGRAM: &str = r#"
fn add(a: number, b: number) -> number {
    return a + b;
}

let result: number = add(40, 2);
print result;
"#;

#[test]
fn test_compile_and_run() {
    // Compile the program
    let wasm_binary = compile(TEST_PROGRAM).expect("Failed to compile");
    
    // Validate we can instantiate the WebAssembly module
    let engine = Engine::default();
    let module = Module::new(&engine, &wasm_binary).expect("Failed to create module");
    let mut store = Store::new(&engine, ());
    
    // TODO: Add more specific test cases
}

#[test]
fn test_empty_program() {
    let source = r#"
        // Empty program
    "#;
    
    let wasm_binary = compile(source).expect("Failed to compile program");
    assert!(!wasm_binary.is_empty(), "Generated WASM binary is empty");
    
    let engine = Engine::default();
    let _module = Module::new(&engine, &wasm_binary).expect("Failed to create module");
    let _store = Store::new(&engine, ());
}

#[test]
fn test_compile_and_run_arithmetic() {
    let source = r#"
        functions:
            calculate() returns number
                input:
                    number:
                        - x = 40
                        - y = 2
                return x + y

        number result = calculate()
        print result
    "#;
    
    // Compile the program
    let wasm_binary = compile(source).expect("Failed to compile");
    
    // Set up WebAssembly environment
    let engine = Engine::default();
    let module = Module::new(&engine, &wasm_binary).expect("Failed to create module");
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[]).expect("Failed to instantiate module");
    
    // Get the result
    let get_result = instance.get_func(&mut store, "get_result")
        .expect("Failed to get result function");
    let mut results = [Val::I32(0)];
    let result = get_result.call(&mut store, &[], &mut results)
        .expect("Failed to call function");
    
    assert_eq!(results[0].unwrap_i32(), 42);
}

#[test]
fn test_string_manipulation() {
    let source = r#"
        string greeting = "Hello"
        string name = "World"
        string message = greeting + ", " + name + "!"
        print message
    "#;
    
    let wasm_binary = compile(source).expect("Failed to compile");
    
    let engine = Engine::default();
    let module = Module::new(&engine, &wasm_binary).expect("Failed to create module");
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[]).expect("Failed to instantiate module");
    
    let get_message = instance.get_func(&mut store, "get_message")
        .expect("Failed to get message function");
    let memory = instance.get_memory(&mut store, "memory")
        .expect("Failed to get memory");
    
    let mut results = [Val::I32(0)];
    get_message.call(&mut store, &[], &mut results)
        .expect("Failed to call function");
    let message_ptr = results[0].unwrap_i32();
    // Read string from memory
    let message_len = memory.data(&store)[message_ptr as usize] as usize;
    let message_bytes = &memory.data(&store)[(message_ptr + 1) as usize..(message_ptr + 1 + message_len) as usize];
    let message = String::from_utf8_lossy(message_bytes);
    
    assert_eq!(message, "Hello, World!");
}

#[test]
fn test_control_flow_and_loops() {
    let source = r#"
        number sum = 0
        
        i = from 1 to 5
            if i > 2
                sum = sum + i
        
        number[] numbers = [1, 2, 3, 4, 5]
        iterate n in numbers
            if n > 3
                sum = sum + n
    "#;
    
    let wasm_binary = compile(source).expect("Failed to compile");
    
    let engine = Engine::default();
    let module = Module::new(&engine, &wasm_binary).expect("Failed to create module");
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[]).expect("Failed to instantiate module");
    
    let get_sum = instance.get_func(&mut store, "get_sum")
        .expect("Failed to get sum function");
    let mut results = [Val::I32(0)];
    let sum = get_sum.call(&mut store, &[], &mut results)
        .expect("Failed to call function");
    
    // sum should be (3+4+5) + (4+5) = 21
    assert_eq!(results[0].unwrap_i32(), 21);
}

#[test]
fn test_class_and_objects() {
    let source = r#"
        class Rectangle
            public
                number width = 0
                number height = 0
            
            public
                number area() returns number
                    return width * height
                
                number perimeter() returns number
                    return 2 * (width + height)

        object rect = new Rectangle()
        rect.width = 5
        rect.height = 3
        
        number a = rect.area()
        number p = rect.perimeter()
    "#;
    
    let wasm_binary = compile(source).expect("Failed to compile");
    
    let engine = Engine::default();
    let module = Module::new(&engine, &wasm_binary).expect("Failed to create module");
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[]).expect("Failed to instantiate module");
    
    let get_area = instance.get_func(&mut store, "get_a")
        .expect("Failed to get area function");
    let get_perimeter = instance.get_func(&mut store, "get_p")
        .expect("Failed to get perimeter function");
    
    let mut results = [Val::F64(0.0f64.to_bits())];
    let area = get_area.call(&mut store, &[], &mut results)
        .expect("Failed to call area");
    assert_eq!(results[0].unwrap_f64(), 15.0);      // 5 * 3
    
    let mut results = [Val::F64(0.0f64.to_bits())];
    let perimeter = get_perimeter.call(&mut store, &[], &mut results)
        .expect("Failed to call perimeter");
    assert_eq!(results[0].unwrap_f64(), 16.0);  // 2 * (5 + 3)
}

#[test]
fn test_error_handling() {
    let source = r#"
        number result = 0
        
        onError:
            result = 42
            print "Error caught"
        
        // Trigger division by zero
        number x = 10 / 0
    "#;
    
    let wasm_binary = compile(source).expect("Failed to compile");
    
    let engine = Engine::default();
    let module = Module::new(&engine, &wasm_binary).expect("Failed to create module");
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[]).expect("Failed to instantiate module");
    
    let get_result = instance.get_func(&mut store, "get_result")
        .expect("Failed to get result function");
    let mut results = [Val::I32(0)];
    let result = get_result.call(&mut store, &[], &mut results)
        .expect("Failed to call function");
    
    assert_eq!(results[0].unwrap_i32(), 42);
}

#[test]
fn test_standard_library() {
    let source = r#"
        // Test string operations
        string text = "Hello, World!"
        number len = string_length(text)
        
        // Test math operations
        number x = -5
        number abs_val = abs(x)
        
        // Test random numbers
        number rand = random()
        number rand_range = random_range(1, 10)
    "#;
    
    let wasm_binary = compile(source).expect("Failed to compile");
    
    let engine = Engine::default();
    let module = Module::new(&engine, &wasm_binary).expect("Failed to create module");
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[]).expect("Failed to instantiate module");
    
    let get_len = instance.get_func(&mut store, "get_len")
        .expect("Failed to get length function");
    let mut results = [Val::I32(0)];
    let len = get_len.call(&mut store, &[], &mut results)
        .expect("Failed to call function");
    assert_eq!(results[0].unwrap_i32(), 13);
    
    let get_rounded = instance.get_func(&mut store, "get_rounded")
        .expect("Failed to get rounded function");
    let mut results = [Val::F64(0.0f64.to_bits())];
    let rounded = get_rounded.call(&mut store, &[], &mut results)
        .expect("Failed to call function");
    assert_eq!(results[0].unwrap_f64(), 3.0);
    
    let get_abs = instance.get_func(&mut store, "get_abs_val")
        .expect("Failed to get abs function");
    let mut results = [Val::F64(0.0f64.to_bits())];
    let abs_val = get_abs.call(&mut store, &[], &mut results)
        .expect("Failed to call function");
    assert_eq!(results[0].unwrap_f64(), 5.0);
    
    let get_rand = instance.get_func(&mut store, "get_rand")
        .expect("Failed to get random function");
    let mut results = [Val::F64(0.0f64.to_bits())];
    let rand = get_rand.call(&mut store, &[], &mut results)
        .expect("Failed to call function");
    let rand_val = results[0].unwrap_f64();
    assert!(rand_val >= 0.0 && rand_val < 1.0);
    
    let get_rand_range = instance.get_func(&mut store, "get_rand_range")
        .expect("Failed to get random range function");
    let mut results = [Val::F64(0.0f64.to_bits())];
    let rand_range = get_rand_range.call(&mut store, &[], &mut results)
        .expect("Failed to call function");
    let rand_val = results[0].unwrap_f64();
    assert!(rand_val >= 1.0 && rand_val <= 10.0);
}

#[test]
fn test_complex_program() {
    let source = r#"
        class BankAccount
            private
                number balance = 0
            
            public
                deposit(number amount) returns void
                    balance = balance + amount
                
                withdraw(number amount) returns void
                    if amount > balance
                        throw "Insufficient funds"
                    balance = balance - amount
                
                get_balance() returns number
                    return balance

        object account = new BankAccount()
        account.deposit(100)
        account.withdraw(30)
        number balance = account.get_balance()
    "#;
    
    let wasm_binary = compile(source).expect("Failed to compile");
    
    let engine = Engine::default();
    let module = Module::new(&engine, &wasm_binary).expect("Failed to create module");
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[]).expect("Failed to instantiate module");
    
    let get_balance = instance.get_func(&mut store, "get_balance")
        .expect("Failed to get balance function");
    let mut results = [Val::F64(0.0f64.to_bits())];
    let balance = get_balance.call(&mut store, &[], &mut results)
        .expect("Failed to call function");
    
    assert_eq!(results[0].unwrap_f64(), 70.0);
}

#[test]
fn test_simple_function() {
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());
    let module = Module::new(&engine, SIMPLE_FUNCTION_WAT).expect("Failed to create module");
    let instance = Instance::new(&mut store, &module, &[]).expect("Failed to instantiate module");
    let get_result = instance.get_func(&mut store, "get_result")
        .expect("Failed to get function");
    let mut results = [Val::I32(0)];
    let result = get_result.call(&mut store, &[], &mut results)
        .expect("Failed to call function");
    assert_eq!(results[0].unwrap_i32(), 42);
} 