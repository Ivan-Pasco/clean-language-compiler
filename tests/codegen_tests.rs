use clean_language_compiler::{
    ast::Program,
    codegen::CodeGenerator,
    validation::Validator,
};
mod test_utils;
use wasmtime::{Store, Module, Instance, Engine};

#[test]
fn test_empty_program() {
    let mut codegen = CodeGenerator::new();
    
    // Create a basic start function  
    use clean_language_compiler::ast::{Function, FunctionSyntax, Visibility, Type, SourceLocation};
    let start_function = Function {
        name: "start".to_string(),
        type_parameters: vec![],
        parameters: vec![],
        return_type: Type::Void,
        body: vec![],
        description: None,
        syntax: FunctionSyntax::Simple,
        visibility: Visibility::Public,
        location: Some(SourceLocation {
            line: 1,
            column: 1,
            file: "test".to_string(),
        }),
    };
    
    let program = Program {
        start_function: Some(start_function),
        functions: vec![],
        classes: vec![],
    };
    
    let result = codegen.generate(&program);
    assert!(result.is_ok(), "Failed to generate WASM: {:?}", result.err());
    
    let wasm_binary = result.unwrap();
    assert!(!wasm_binary.is_empty(), "Generated WASM binary is empty");
    
    // Validate WASM binary
    assert!(Validator::validate_wasm(&wasm_binary).is_ok());
    
    // Test basic WebAssembly functionality
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());
    let module = Module::new(&engine, &wasm_binary).expect("Failed to create module");
    let _instance = Instance::new(&mut store, &module, &[]).expect("Failed to instantiate module");
}

#[test]
fn test_basic_arithmetic() {
    let source = r#"
float x = 10 + 20 * 2
float y = (30 - 5) / 5
float z = x + y

function start()
	print(x)
    "#;
    
    let program = test_utils::parse_source(source);
    let wasm_binary = test_utils::generate_wasm(&program);
    
    // Create WebAssembly instance
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());
    let module = Module::new(&engine, &wasm_binary).expect("Failed to create module");
    let instance = Instance::new(&mut store, &module, &[]).expect("Failed to instantiate module");
    
    // Get memory and exported functions
    let _memory = instance.get_memory(&mut store, "memory").expect("Failed to get memory");
    let get_x = instance.get_func(&mut store, "get_x").expect("Failed to get x function");
    let get_y = instance.get_func(&mut store, "get_y").expect("Failed to get y function");
    let get_z = instance.get_func(&mut store, "get_z").expect("Failed to get z function");
    
    // Check results
    let mut results = vec![wasmtime::Val::I32(0)];
    get_x.call(&mut store, &[], &mut results).expect("Failed to call get_x");
    let x_result = match results[0] {
        wasmtime::Val::F64(bits) => f64::from_bits(bits),
        _ => panic!("Expected F64 result"),
    };
    get_y.call(&mut store, &[], &mut results).expect("Failed to call get_y");
    let y_result = match results[0] {
        wasmtime::Val::F64(bits) => f64::from_bits(bits),
        _ => panic!("Expected F64 result"),
    };
    get_z.call(&mut store, &[], &mut results).expect("Failed to call get_z");
    let z_result = match results[0] {
        wasmtime::Val::F64(bits) => f64::from_bits(bits),
        _ => panic!("Expected F64 result"),
    };
    
    assert_eq!(x_result, 50.0); // 10 + (20 * 2)
    assert_eq!(y_result, 5.0);  // (30 - 5) / 5
    assert_eq!(z_result, 55.0); // 50 + 5
}

#[test]
fn test_function_generation() {
    let source = r#"
functions:
	float add()
		input
			float x
			float y
		return x + y

	float multiply()
		input
			float x
			float y
		return x * y

float result1 = add(10, 20)
float result2 = multiply(5, 6)

function start()
	print(result1)
    "#;
    
    let program = test_utils::parse_source(source);
    let wasm_binary = test_utils::generate_wasm(&program);
    
    // Create WebAssembly instance
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());
    let module = Module::new(&engine, &wasm_binary).expect("Failed to create module");
    let instance = Instance::new(&mut store, &module, &[]).expect("Failed to instantiate module");
    
    // Get exported functions
    let add_func = instance.get_func(&mut store, "add").expect("Failed to get add function");
    let multiply_func = instance.get_func(&mut store, "multiply").expect("Failed to get multiply function");
    let get_result1 = instance.get_func(&mut store, "get_result1").expect("Failed to get result1 function");
    let get_result2 = instance.get_func(&mut store, "get_result2").expect("Failed to get result2 function");
    
    // Test function calls
    let mut results = vec![wasmtime::Val::I32(0)];
    add_func.call(&mut store, &[wasmtime::Val::F64(10.0_f64.to_bits()), wasmtime::Val::F64(20.0_f64.to_bits())], &mut results)
        .expect("Failed to call add");
    let add_result = match results[0] {
        wasmtime::Val::F64(bits) => f64::from_bits(bits),
        _ => panic!("Expected F64 result"),
    };
    multiply_func.call(&mut store, &[wasmtime::Val::F64(5.0_f64.to_bits()), wasmtime::Val::F64(6.0_f64.to_bits())], &mut results)
        .expect("Failed to call multiply");
    let multiply_result = match results[0] {
        wasmtime::Val::F64(bits) => f64::from_bits(bits),
        _ => panic!("Expected F64 result"),
    };
    
    assert_eq!(add_result, 30.0);
    assert_eq!(multiply_result, 30.0);
    
    // Test stored results
    get_result1.call(&mut store, &[], &mut results).expect("Failed to call get_result1");
    let result1 = match results[0] {
        wasmtime::Val::F64(bits) => f64::from_bits(bits),
        _ => panic!("Expected F64 result"),
    };
    get_result2.call(&mut store, &[], &mut results).expect("Failed to call get_result2");
    let result2 = match results[0] {
        wasmtime::Val::F64(bits) => f64::from_bits(bits),
        _ => panic!("Expected F64 result"),
    };
    
    assert_eq!(result1, 30.0);
    assert_eq!(result2, 30.0);
}

#[test]
fn test_memory_operations() {
    let source = r#"
string message = "Hello, World!"
Array<float> numbers = [1, 2, 3, 4, 5]

function start()
	print(message)
	iterate n in numbers
		print(n)
    "#;
    
    let program = test_utils::parse_source(source);
    let wasm_binary = test_utils::generate_wasm(&program);
    
    // Create WebAssembly instance
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());
    let module = Module::new(&engine, &wasm_binary).expect("Failed to create module");
    let instance = Instance::new(&mut store, &module, &[]).expect("Failed to instantiate module");
    
    // Get memory and functions
    let memory = instance.get_memory(&mut store, "memory").expect("Failed to get memory");
    let get_message = instance.get_func(&mut store, "get_message").expect("Failed to get message function");
    let get_numbers = instance.get_func(&mut store, "get_numbers").expect("Failed to get numbers function");
    
    // Check string in memory
    let mut results = vec![wasmtime::Val::I32(0)];
    get_message.call(&mut store, &[], &mut results).expect("Failed to call get_message");
    let message_ptr = match results[0] {
        wasmtime::Val::I32(ptr) => ptr as u32,
        _ => panic!("Expected I32 result"),
    };
    let message_len = memory.data(&store)[message_ptr as usize] as usize;
    let message_bytes = &memory.data(&store)[(message_ptr + 1) as usize..(message_ptr + 1) as usize + message_len];
    let message = String::from_utf8_lossy(message_bytes);
    assert_eq!(message, "Hello, World!");
    
    // Check array in memory
    get_numbers.call(&mut store, &[], &mut results).expect("Failed to call get_numbers");
    let numbers_ptr = match results[0] {
        wasmtime::Val::I32(ptr) => ptr as u32,
        _ => panic!("Expected I32 result"),
    };
    let numbers_len = memory.data(&store)[numbers_ptr as usize] as usize;
    let numbers_data = &memory.data(&store)[(numbers_ptr + 1) as usize..(numbers_ptr + 1) as usize + numbers_len * 8];
    let numbers: Vec<f64> = numbers_data.chunks(8)
        .map(|chunk| f64::from_le_bytes(chunk.try_into().unwrap()))
        .collect();
    assert_eq!(numbers, vec![1.0, 2.0, 3.0, 4.0, 5.0]);
}

#[test]
fn test_control_flow() {
    let source = r#"
float x = 10
float y = 0

function start()
	if x > 5
		y = 1
	
	iterate i in [1, 2, 3]
		y = y + i
	
	iterate i in 1 to 3
		y = y + i
    "#;
    
    let program = test_utils::parse_source(source);
    let wasm_binary = test_utils::generate_wasm(&program);
    
    // Create WebAssembly instance
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());
    let module = Module::new(&engine, &wasm_binary).expect("Failed to create module");
    let instance = Instance::new(&mut store, &module, &[]).expect("Failed to instantiate module");
    
    // Get y value after all operations
    let get_y = instance.get_func(&mut store, "get_y").expect("Failed to get y function");
    let mut results = vec![wasmtime::Val::I32(0)];
    get_y.call(&mut store, &[], &mut results).expect("Failed to call get_y");
    let final_y = match results[0] {
        wasmtime::Val::F64(bits) => f64::from_bits(bits),
        _ => panic!("Expected F64 result"),
    };
    
    // y should be: 1 (from if) + (1+2+3) (from iterate) + (1+2+3) (from from-to) = 13
    assert_eq!(final_y, 13.0);
}

#[test]
fn test_class_generation() {
    let source = r#"
class Point
	float x = 0
	float y = 0
	
	constructor(x, y)
	
	float distance()
		return sqrt(x * x + y * y)

function start()
	Point p = Point(3, 4)
	float d = p.distance()
    "#;
    
    let program = test_utils::parse_source(source);
    let wasm_binary = test_utils::generate_wasm(&program);
    
    // Create WebAssembly instance
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());
    let module = Module::new(&engine, &wasm_binary).expect("Failed to create module");
    let instance = Instance::new(&mut store, &module, &[]).expect("Failed to instantiate module");
    
    // Get distance result
    let get_d = instance.get_func(&mut store, "get_d").expect("Failed to get d function");
    let mut results = vec![wasmtime::Val::I32(0)];
    get_d.call(&mut store, &[], &mut results).expect("Failed to call get_d");
    let distance = match results[0] {
        wasmtime::Val::F64(bits) => f64::from_bits(bits),
        _ => panic!("Expected F64 result"),
    };
    
    assert_eq!(distance, 25.0); // 3*3 + 4*4 = 25
}

#[test]
fn test_error_handling() {
    let source = r#"
float x = 0

function start()
	// This should trigger an error and be handled
	float y = (10 / x) onError 42
	x = y
    "#;
    
    let program = test_utils::parse_source(source);
    let wasm_binary = test_utils::generate_wasm(&program);
    
    // Create WebAssembly instance
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());
    let module = Module::new(&engine, &wasm_binary).expect("Failed to create module");
    let instance = Instance::new(&mut store, &module, &[]).expect("Failed to instantiate module");
    
    // Get x value after error
    let get_x = instance.get_func(&mut store, "get_x").expect("Failed to get x function");
    let mut results = vec![wasmtime::Val::I32(0)];
    get_x.call(&mut store, &[], &mut results).expect("Failed to call get_x");
    let x_after_error = match results[0] {
        wasmtime::Val::F64(bits) => f64::from_bits(bits),
        _ => panic!("Expected F64 result"),
    };
    
    assert_eq!(x_after_error, 42.0); // Error handler should have executed
} 