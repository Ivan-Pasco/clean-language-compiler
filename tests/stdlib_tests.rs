use crate::stdlib::{
    StandardLibrary,
    StringOperations,
    NumericOperations,
    RandomOperations,
    TimeOperations,
    DateOperations,
    FormatOperations,
    TypeConversion
};
use crate::codegen::CodeGenerator;
use crate::error::CompilerError;
use wasmtime::{Store, Module, Instance};

#[test]
fn test_string_operations() -> Result<(), CompilerError> {
    let mut codegen = CodeGenerator::new();
    let string_ops = StringOperations::new();
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
    let numeric_ops = NumericOperations::new();
    numeric_ops.register_functions(&mut codegen)?;

    // Test absolute value
    let result = codegen.call_function("number_abs", vec![wasmtime::Val::F64(-42.5)])?;
    assert_eq!(result[0].unwrap_f64(), 42.5);

    // Test rounding
    let result = codegen.call_function("number_round", vec![wasmtime::Val::F64(3.7)])?;
    assert_eq!(result[0].unwrap_f64(), 4.0);

    // Test min/max
    let result = codegen.call_function("number_max", vec![
        wasmtime::Val::F64(10.5),
        wasmtime::Val::F64(20.7),
    ])?;
    assert_eq!(result[0].unwrap_f64(), 20.7);

    Ok(())
}

#[test]
fn test_random_operations() -> Result<(), CompilerError> {
    let mut codegen = CodeGenerator::new();
    let random_ops = RandomOperations::new();
    random_ops.register_functions(&mut codegen)?;

    // Test random number generation
    let result = codegen.call_function("random", vec![])?;
    let random_value = result[0].unwrap_f64();
    assert!(random_value >= 0.0 && random_value < 1.0);

    // Test random range
    let result = codegen.call_function("random_range", vec![
        wasmtime::Val::F64(10.0),
        wasmtime::Val::F64(20.0),
    ])?;
    let random_value = result[0].unwrap_f64();
    assert!(random_value >= 10.0 && random_value < 20.0);

    Ok(())
}

#[test]
fn test_time_operations() -> Result<(), CompilerError> {
    let mut codegen = CodeGenerator::new();
    let time_ops = TimeOperations::new();
    time_ops.register_functions(&mut codegen)?;

    // Test current time
    let result = codegen.call_function("now", vec![])?;
    let timestamp = result[0].unwrap_i64();
    assert!(timestamp > 0);

    // Test time components
    let test_timestamp = 1609459200; // 2021-01-01 00:00:00
    
    let result = codegen.call_function("hour", vec![wasmtime::Val::I64(test_timestamp)])?;
    let hour = result[0].unwrap_i32();
    assert!(hour >= 0 && hour < 24);

    let result = codegen.call_function("minute", vec![wasmtime::Val::I64(test_timestamp)])?;
    let minute = result[0].unwrap_i32();
    assert!(minute >= 0 && minute < 60);

    let result = codegen.call_function("second", vec![wasmtime::Val::I64(test_timestamp)])?;
    let second = result[0].unwrap_i32();
    assert!(second >= 0 && second < 60);

    Ok(())
}

#[test]
fn test_date_operations() -> Result<(), CompilerError> {
    let mut codegen = CodeGenerator::new();
    let date_ops = DateOperations::new();
    date_ops.register_functions(&mut codegen)?;

    let test_timestamp = 1609459200; // 2021-01-01 00:00:00

    // Test date components
    let result = codegen.call_function("year", vec![wasmtime::Val::I64(test_timestamp)])?;
    assert_eq!(result[0].unwrap_i32(), 2021);

    let result = codegen.call_function("month", vec![wasmtime::Val::I64(test_timestamp)])?;
    let month = result[0].unwrap_i32();
    assert!(month >= 1 && month <= 12);

    let result = codegen.call_function("day", vec![wasmtime::Val::I64(test_timestamp)])?;
    let day = result[0].unwrap_i32();
    assert!(day >= 1 && day <= 31);

    // Test date arithmetic
    let result = codegen.call_function("add_days", vec![
        wasmtime::Val::I64(test_timestamp),
        wasmtime::Val::I32(1),
    ])?;
    assert_eq!(result[0].unwrap_i64() - test_timestamp, 86400);

    Ok(())
}

#[test]
fn test_format_operations() -> Result<(), CompilerError> {
    let mut codegen = CodeGenerator::new();
    let format_ops = FormatOperations::new();
    format_ops.register_functions(&mut codegen)?;

    let test_timestamp = 1609459200; // 2021-01-01 00:00:00
    let format_str = "YYYY-MM-DD HH:mm:ss";
    let format_ptr = codegen.add_string_to_pool(format_str);

    let result = codegen.call_function("format", vec![
        wasmtime::Val::I64(test_timestamp),
        wasmtime::Val::I32(format_ptr as i32),
    ])?;
    let formatted = codegen.get_string_from_memory(result[0].unwrap_i32() as u64)?;
    assert_eq!(formatted, "2021-01-01 00:00:00");

    Ok(())
}

#[test]
fn test_type_conversion() -> Result<(), CompilerError> {
    let mut codegen = CodeGenerator::new();
    let type_conv = TypeConversion::new();
    type_conv.register_functions(&mut codegen)?;

    // Test number to integer conversion
    let result = codegen.call_function("to_integer", vec![wasmtime::Val::F64(42.7)])?;
    assert_eq!(result[0].unwrap_i32(), 42);

    // Test number to string conversion
    let result = codegen.call_function("to_string", vec![wasmtime::Val::F64(42.5)])?;
    let result_str = codegen.get_string_from_memory(result[0].unwrap_i32() as u64)?;
    assert_eq!(result_str, "42.5");

    Ok(())
}

#[test]
fn test_stdlib_integration() -> Result<(), CompilerError> {
    let mut codegen = CodeGenerator::new();
    let stdlib = StandardLibrary::new();
    stdlib.register_functions(&mut codegen)?;

    // Test combination of operations
    let test_str = "42.5";
    let str_ptr = codegen.add_string_to_pool(test_str);
    
    // Convert string to number
    let result = codegen.call_function("to_number", vec![wasmtime::Val::I32(str_ptr as i32)])?;
    let number = result[0].unwrap_f64();
    assert_eq!(number, 42.5);
    
    // Round the number
    let result = codegen.call_function("number_round", vec![wasmtime::Val::F64(number)])?;
    let rounded = result[0].unwrap_f64();
    assert_eq!(rounded, 43.0);
    
    // Convert back to string
    let result = codegen.call_function("to_string", vec![wasmtime::Val::F64(rounded)])?;
    let result_str = codegen.get_string_from_memory(result[0].unwrap_i32() as u64)?;
    assert_eq!(result_str, "43");

    Ok(())
} 