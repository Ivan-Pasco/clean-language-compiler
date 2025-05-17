use crate::codegen::CodeGenerator;
use crate::error::CompilerError;
use crate::stdlib::FormatOperations;
use wasmtime::{Store, Module, Instance, Val};

struct TestFormat {
    store: Store<()>,
    instance: Instance,
    codegen: CodeGenerator,
}

impl TestFormat {
    fn new() -> Result<Self, CompilerError> {
        let mut codegen = CodeGenerator::new();
        
        // Register format operations
        let format_ops = FormatOperations::new();
        format_ops.register_functions(&mut codegen)?;

        let wasm = codegen.build();
        let engine = wasmtime::Engine::default();
        let module = Module::new(&engine, &wasm)?;
        let mut store = Store::new(&engine, ());
        let instance = Instance::new(&mut store, &module, &[])?;

        Ok(Self { 
            store, 
            instance,
            codegen,
        })
    }

    fn call_func(&mut self, name: &str, params: &[Val]) -> Result<Val, CompilerError> {
        let mut results = vec![Val::I32(0)];
        self.instance
            .get_func(&mut self.store, name)
            .ok_or_else(|| CompilerError::RuntimeError(format!("Function {} not found", name)))?
            .call(&mut self.store, params, &mut results)
            .map_err(|e| CompilerError::RuntimeError(e.to_string()))?;
        Ok(results[0])
    }

    fn add_string_to_pool(&mut self, s: &str) -> u32 {
        self.codegen.add_string_to_pool(s)
    }

    fn get_string_from_memory(&self, ptr: u32) -> Result<String, CompilerError> {
        self.codegen.get_string_from_memory(ptr as u64)
    }
}

#[test]
fn test_basic_formatting() -> Result<(), CompilerError> {
    let mut format = TestFormat::new()?;
    let timestamp = 1234567890; // 2009-02-13 23:31:30 UTC

    // Test date formatting
    let date_ptr = format.call_func("format_date", &[Val::I64(timestamp)])?.unwrap_i32() as u32;
    let date_str = format.get_string_from_memory(date_ptr)?;
    assert!(!date_str.is_empty(), "Date format should return non-empty string");
    assert!(date_str.contains("2009"), "Date should contain year");

    // Test time formatting
    let time_ptr = format.call_func("format_time", &[Val::I64(timestamp)])?.unwrap_i32() as u32;
    let time_str = format.get_string_from_memory(time_ptr)?;
    assert!(!time_str.is_empty(), "Time format should return non-empty string");
    assert!(time_str.contains("23:31"), "Time should contain hour and minute");

    // Test ISO datetime formatting
    let iso_ptr = format.call_func("format_iso_datetime", &[Val::I64(timestamp)])?.unwrap_i32() as u32;
    let iso_str = format.get_string_from_memory(iso_ptr)?;
    assert!(!iso_str.is_empty(), "ISO datetime format should return non-empty string");
    assert!(iso_str.contains("2009-02-13T23:31:30"), "ISO datetime should be properly formatted");

    Ok(())
}

#[test]
fn test_custom_formatting() -> Result<(), CompilerError> {
    let mut format = TestFormat::new()?;
    let timestamp = 1234567890; // 2009-02-13 23:31:30 UTC

    // Test custom format
    let format_str = format.add_string_to_pool("%Y-%m-%d %H:%M:%S");
    let result_ptr = format.call_func(
        "format_custom",
        &[Val::I64(timestamp), Val::I32(format_str as i32)],
    )?.unwrap_i32() as u32;
    
    let result_str = format.get_string_from_memory(result_ptr)?;
    assert!(!result_str.is_empty(), "Custom format should return non-empty string");
    assert_eq!(result_str, "2009-02-13 23:31:30", "Custom format should match expected output");

    Ok(())
} 