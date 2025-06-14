use clean_language_compiler::{
    stdlib::{
        string_ops::StringOperations,
        type_conv::TypeConvOperations,
    },
    codegen::CodeGenerator,
    error::CompilerError,
};
use wasmtime::{Store, Module, Instance, Val};

#[test]
fn test_string_operations() -> Result<(), CompilerError> {
    let mut codegen = CodeGenerator::new();
    let string_ops = StringOperations::new(1024);
    string_ops.register_functions(&mut codegen)?;

    // Test string length
    let test_str = "Hello, World!";
    let str_ptr = codegen.add_string_to_pool(test_str);
    let result = codegen.call_function("string_length", vec![wasmtime::Val::I32(str_ptr as i32)])?;
    assert_eq!(result[0].unwrap_i32(), test_str.len() as i32);

    // Test string concatenation
    let str1 = "Hello, ";
    let str2 = "World!";
    let str1_ptr = codegen.add_string_to_pool(str1);
    let str2_ptr = codegen.add_string_to_pool(str2);
    let result = codegen.call_function("string_concat", vec![
        wasmtime::Val::I32(str1_ptr as i32),
        wasmtime::Val::I32(str2_ptr as i32),
    ])?;
    let result_str = codegen.get_string_from_memory(result[0].unwrap_i32() as u64)?;
    assert_eq!(result_str, format!("{}{}", str1, str2));

    Ok(())
}

#[test]
fn test_numeric_operations() -> Result<(), CompilerError> {
    let mut codegen = CodeGenerator::new();
    
    // Test abs
    let result = codegen.call_function("number_abs", vec![Val::F64((-42.5f64).to_bits())])?;
    let abs_result = f64::from_bits(result[0].unwrap_i64() as u64);
    assert!((abs_result - 42.5).abs() < f64::EPSILON);
    
    // Test round
    let result = codegen.call_function("number_round", vec![Val::F64(3.7f64.to_bits())])?;
    let round_result = f64::from_bits(result[0].unwrap_i64() as u64);
    assert!((round_result - 4.0).abs() < f64::EPSILON);
    
    // Test min/max
    let result = codegen.call_function("number_min", vec![
        Val::F64(10.5f64.to_bits()),
        Val::F64(20.7f64.to_bits()),
    ])?;
    let min_result = f64::from_bits(result[0].unwrap_i64() as u64);
    assert!((min_result - 10.5).abs() < f64::EPSILON);
    
    // Test add
    let result = codegen.call_function("add", vec![
        Val::F64(10.0f64.to_bits()),
        Val::F64(20.0f64.to_bits()),
    ])?;
    let add_result = f64::from_bits(result[0].unwrap_i64() as u64);
    assert!((add_result - 30.0).abs() < f64::EPSILON);

    Ok(())
}

#[test]
fn test_type_conversion() -> Result<(), CompilerError> {
    let mut codegen = CodeGenerator::new();
    
    // Test integer conversion
    let result = codegen.call_function("to_integer", vec![Val::F64(42.7f64.to_bits())])?;
    let int_result = f64::from_bits(result[0].unwrap_i64() as u64);
    assert!((int_result - 42.0).abs() < f64::EPSILON);
    
    // Test string conversion
    let result = codegen.call_function("to_string", vec![Val::F64(42.5f64.to_bits())])?;
    let str_result = f64::from_bits(result[0].unwrap_i64() as u64);
    assert!((str_result - 42.5).abs() < f64::EPSILON);

    Ok(())
} 