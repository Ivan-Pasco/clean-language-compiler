use clean_language_compiler::compile;
use wasmtime::{Engine, Module, Store, Instance, Func};
use std::time::Duration;

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
    let instance = Instance::new(&store, &module, &[]).expect("Failed to instantiate module");
    
    // Get the result
    let get_result = instance.get_func(&store, "get_result")
        .expect("Failed to get result function");
    let result = get_result.call(&mut store, &[], &mut [])
        .expect("Failed to call function")[0]
        .unwrap_f64();
    
    assert_eq!(result, 42.0);
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
    let instance = Instance::new(&store, &module, &[]).expect("Failed to instantiate module");
    
    let get_message = instance.get_func(&store, "get_message")
        .expect("Failed to get message function");
    let memory = instance.get_memory(&store, "memory")
        .expect("Failed to get memory");
    
    let message_ptr = get_message.call(&mut store, &[], &mut [])
        .expect("Failed to call function")[0]
        .unwrap_i32() as u32;
    
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
    let instance = Instance::new(&store, &module, &[]).expect("Failed to instantiate module");
    
    let get_sum = instance.get_func(&store, "get_sum")
        .expect("Failed to get sum function");
    let sum = get_sum.call(&mut store, &[], &mut [])
        .expect("Failed to call function")[0]
        .unwrap_f64();
    
    // sum should be (3+4+5) + (4+5) = 21
    assert_eq!(sum, 21.0);
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
    let instance = Instance::new(&store, &module, &[]).expect("Failed to instantiate module");
    
    let get_area = instance.get_func(&store, "get_a")
        .expect("Failed to get area function");
    let get_perimeter = instance.get_func(&store, "get_p")
        .expect("Failed to get perimeter function");
    
    let area = get_area.call(&mut store, &[], &mut [])
        .expect("Failed to call area")[0]
        .unwrap_f64();
    let perimeter = get_perimeter.call(&mut store, &[], &mut [])
        .expect("Failed to call perimeter")[0]
        .unwrap_f64();
    
    assert_eq!(area, 15.0);      // 5 * 3
    assert_eq!(perimeter, 16.0);  // 2 * (5 + 3)
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
    let instance = Instance::new(&store, &module, &[]).expect("Failed to instantiate module");
    
    let get_result = instance.get_func(&store, "get_result")
        .expect("Failed to get result function");
    let result = get_result.call(&mut store, &[], &mut [])
        .expect("Failed to call function")[0]
        .unwrap_f64();
    
    assert_eq!(result, 42.0); // Error handler should have executed
}

#[test]
fn test_standard_library() {
    let source = r#"
        // String operations
        string str = "Hello, World!"
        number len = str.length()
        string upper = str.toUpper()
        
        // Math operations
        number pi = 3.14159
        number rounded = pi.round()
        number abs_val = (-42).abs()
        
        // Random operations
        number rand = random()
        number rand_range = random_range(1, 10)
        
        // DateTime operations
        number timestamp = time_now()
        number year = timestamp.year()
    "#;
    
    let wasm_binary = compile(source).expect("Failed to compile");
    
    let engine = Engine::default();
    let module = Module::new(&engine, &wasm_binary).expect("Failed to create module");
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&store, &module, &[]).expect("Failed to instantiate module");
    
    // Test string length
    let get_len = instance.get_func(&store, "get_len")
        .expect("Failed to get length function");
    let len = get_len.call(&mut store, &[], &mut [])
        .expect("Failed to call function")[0]
        .unwrap_f64();
    assert_eq!(len, 13.0);
    
    // Test math operations
    let get_rounded = instance.get_func(&store, "get_rounded")
        .expect("Failed to get rounded function");
    let rounded = get_rounded.call(&mut store, &[], &mut [])
        .expect("Failed to call function")[0]
        .unwrap_f64();
    assert_eq!(rounded, 3.0);
    
    let get_abs = instance.get_func(&store, "get_abs_val")
        .expect("Failed to get abs function");
    let abs_val = get_abs.call(&mut store, &[], &mut [])
        .expect("Failed to call function")[0]
        .unwrap_f64();
    assert_eq!(abs_val, 42.0);
    
    // Test random operations
    let get_rand = instance.get_func(&store, "get_rand")
        .expect("Failed to get random function");
    let rand = get_rand.call(&mut store, &[], &mut [])
        .expect("Failed to call function")[0]
        .unwrap_f64();
    assert!(rand >= 0.0 && rand < 1.0);
    
    let get_rand_range = instance.get_func(&store, "get_rand_range")
        .expect("Failed to get random range function");
    let rand_range = get_rand_range.call(&mut store, &[], &mut [])
        .expect("Failed to call function")[0]
        .unwrap_f64();
    assert!(rand_range >= 1.0 && rand_range < 10.0);
}

#[test]
fn test_complex_program() {
    let source = r#"
        class BankAccount
            public
                number balance = 0
                string owner = ""
            
            public
                number deposit(number amount) returns number
                    balance = balance + amount
                    return balance
                
                number withdraw(number amount) returns number
                    if amount <= balance
                        balance = balance - amount
                        return amount
                    return 0

        functions:
            processTransactions() returns number
                input:
                    number[]:
                        - amounts
                    string:
                        - type
                
                number total = 0
                iterate amount in amounts
                    if type == "deposit"
                        total = total + amount
                    if type == "withdraw"
                        total = total - amount
                return total

        object account = new BankAccount()
        account.owner = "John Doe"
        account.deposit(100)
        
        number[] transactions = [20, 30, 40]
        number result = processTransactions(transactions, "deposit")
        
        account.withdraw(50)
    "#;
    
    let wasm_binary = compile(source).expect("Failed to compile");
    
    let engine = Engine::default();
    let module = Module::new(&engine, &wasm_binary).expect("Failed to create module");
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&store, &module, &[]).expect("Failed to instantiate module");
    
    // Test final account balance
    let get_balance = instance.get_func(&store, "get_balance")
        .expect("Failed to get balance function");
    let balance = get_balance.call(&mut store, &[], &mut [])
        .expect("Failed to call function")[0]
        .unwrap_f64();
    
    // Initial deposit: 100
    // Process transactions: +90 (20+30+40)
    // Withdrawal: -50
    // Final balance should be 140
    assert_eq!(balance, 140.0);
} 