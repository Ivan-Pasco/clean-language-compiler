use crate::codegen::CodeGenerator;
use crate::error::CompilerError;
use crate::stdlib::TimeOperations;
use wasmtime::{Store, Module, Instance, Val};

struct TestTime {
    store: Store<()>,
    instance: Instance,
}

impl TestTime {
    fn new() -> Result<Self, CompilerError> {
        let mut codegen = CodeGenerator::new();
        
        // Register time operations
        let time_ops = TimeOperations::new();
        time_ops.register_functions(&mut codegen)?;

        let wasm = codegen.build();
        let engine = wasmtime::Engine::default();
        let module = Module::new(&engine, &wasm)?;
        let mut store = Store::new(&engine, ());
        let instance = Instance::new(&mut store, &module, &[])?;

        Ok(Self { store, instance })
    }

    fn call_func(&mut self, name: &str, params: &[Val]) -> Result<Vec<Val>, CompilerError> {
        let mut results = vec![Val::I32(0)];
        self.instance
            .get_func(&mut self.store, name)
            .ok_or_else(|| CompilerError::RuntimeError(format!("Function {} not found", name)))?
            .call(&mut self.store, params, &mut results)
            .map_err(|e| CompilerError::RuntimeError(e.to_string()))?;
        Ok(results)
    }
}

#[test]
fn test_current_time() -> Result<(), CompilerError> {
    let mut time = TestTime::new()?;
    
    // Test now()
    let result = time.call_func("now", &[])?;
    let timestamp = result[0].unwrap_i64();
    assert!(timestamp > 0, "Current timestamp should be positive");
    
    // Test now_ms()
    let result = time.call_func("now_ms", &[])?;
    let timestamp_ms = result[0].unwrap_i64();
    assert!(timestamp_ms > timestamp * 1000, "Millisecond timestamp should be larger");

    Ok(())
}

#[test]
fn test_time_components() -> Result<(), CompilerError> {
    let mut time = TestTime::new()?;
    let timestamp = 1234567890; // 2009-02-13 23:31:30 UTC

    // Test hour
    let result = time.call_func("hour", &[Val::I64(timestamp)])?;
    let hour = result[0].unwrap_i32();
    assert!(hour >= 0 && hour < 24, "Hour should be between 0 and 23");

    // Test minute
    let result = time.call_func("minute", &[Val::I64(timestamp)])?;
    let minute = result[0].unwrap_i32();
    assert!(minute >= 0 && minute < 60, "Minute should be between 0 and 59");

    // Test second
    let result = time.call_func("second", &[Val::I64(timestamp)])?;
    let second = result[0].unwrap_i32();
    assert!(second >= 0 && second < 60, "Second should be between 0 and 59");

    // Test millisecond
    let result = time.call_func("millisecond", &[Val::I64(timestamp)])?;
    let ms = result[0].unwrap_i32();
    assert!(ms >= 0 && ms < 1000, "Millisecond should be between 0 and 999");

    Ok(())
} 