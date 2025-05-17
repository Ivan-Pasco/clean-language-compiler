use clean_language::{
    parser::CleanParser,
    codegen::CodeGenerator,
    validation::WasmValidator,
};
use std::fs;

mod test_utils;

#[test]
fn test_arithmetic_program() {
    let source = fs::read_to_string("tests/test_inputs/arithmetic.cln").unwrap();
    let result = CleanParser::parse_program(&source);
    assert!(result.is_ok(), "Failed to parse arithmetic program: {:?}", result.err());
    
    let program = result.unwrap();
    let mut codegen = CodeGenerator::new();
    let wasm_result = codegen.generate(&program);
    assert!(wasm_result.is_ok(), "Failed to generate WASM: {:?}", wasm_result.err());
    
    let wasm_binary = wasm_result.unwrap();
    let validator = WasmValidator::new();
    assert!(validator.validate_and_analyze(&wasm_binary).is_ok());
    
    // Execute the WASM and verify output
    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, &wasm_binary).unwrap();
    let mut store = wasmtime::Store::new(&engine, ());
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).unwrap();
    
    let start = instance.get_func(&mut store, "start").unwrap();
    let mut results = vec![wasmtime::Val::I32(0)];
    start.call(&mut store, &[], &mut results).unwrap();
    
    assert_eq!(results[0].unwrap_i32(), 50); // 42 + 8 = 50
}

#[test]
fn test_matrix_program() {
    let source = fs::read_to_string("tests/test_inputs/matrix.cln").unwrap();
    let result = CleanParser::parse_program(&source);
    assert!(result.is_ok(), "Failed to parse matrix program: {:?}", result.err());
    
    let program = result.unwrap();
    let mut codegen = CodeGenerator::new();
    let wasm_result = codegen.generate(&program);
    assert!(wasm_result.is_ok(), "Failed to generate WASM: {:?}", wasm_result.err());
    
    let wasm_binary = wasm_result.unwrap();
    let validator = WasmValidator::new();
    assert!(validator.validate_and_analyze(&wasm_binary).is_ok());
    
    // Execute the WASM and verify output
    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, &wasm_binary).unwrap();
    let mut store = wasmtime::Store::new(&engine, ());
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).unwrap();
    
    let start = instance.get_func(&mut store, "start").unwrap();
    let mut results = vec![wasmtime::Val::I32(0)];
    start.call(&mut store, &[], &mut results).unwrap();
    
    // Get the matrix from memory and verify it's transposed correctly
    let memory = instance.get_memory(&mut store, "memory").unwrap();
    let ptr = results[0].unwrap_i32() as usize;
    let data = memory.data(&store);
    
    // Read matrix dimensions
    let rows = data[ptr] as usize;
    let cols = data[ptr + 4] as usize;
    assert_eq!(rows, 2);
    assert_eq!(cols, 2);
    
    // Verify transposed values
    let expected = vec![1.0, 3.0, 2.0, 4.0];
    for i in 0..4 {
        let value = f64::from_le_bytes(data[ptr + 8 + i * 8..ptr + 16 + i * 8].try_into().unwrap());
        assert_eq!(value, expected[i]);
    }
}

#[test]
fn test_function_program() {
    let source = fs::read_to_string("tests/test_inputs/function.cln").unwrap();
    let result = CleanParser::parse_program(&source);
    assert!(result.is_ok(), "Failed to parse function program: {:?}", result.err());
    
    let program = result.unwrap();
    let mut codegen = CodeGenerator::new();
    let wasm_result = codegen.generate(&program);
    assert!(wasm_result.is_ok(), "Failed to generate WASM: {:?}", wasm_result.err());
    
    let wasm_binary = wasm_result.unwrap();
    let validator = WasmValidator::new();
    assert!(validator.validate_and_analyze(&wasm_binary).is_ok());
    
    // Execute the WASM and verify output
    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, &wasm_binary).unwrap();
    let mut store = wasmtime::Store::new(&engine, ());
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).unwrap();
    
    let start = instance.get_func(&mut store, "start").unwrap();
    let mut results = vec![wasmtime::Val::I32(0)];
    start.call(&mut store, &[], &mut results).unwrap();
    
    assert_eq!(results[0].unwrap_i32(), 8); // 5 + 3 = 8
} 