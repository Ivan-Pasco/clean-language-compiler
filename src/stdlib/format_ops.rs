use crate::codegen::CodeGenerator;
use crate::types::WasmType;
use crate::error::CompilerError;
use wasm_encoder::Instruction;
use crate::stdlib::register_stdlib_function;

/// Format operations implementation for date and time
pub struct FormatOperations {}

impl FormatOperations {
    pub fn new() -> Self {
        Self {}
    }

    pub fn register_functions(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // Helper function to convert parameter types
        let params_to_types = |params: &[(WasmType, String)]| -> Vec<WasmType> {
            params.iter().map(|(t, _)| *t).collect()
        };

        // Date formatting functions
        register_stdlib_function(
            codegen,
            "format_date",
            &params_to_types(&[(WasmType::I64, "timestamp".to_string())]),
            Some(WasmType::I32),
            self.generate_format_date_function()
        )?;

        register_stdlib_function(
            codegen,
            "format_time",
            &params_to_types(&[(WasmType::I64, "timestamp".to_string())]),
            Some(WasmType::I32),
            self.generate_format_time_function()
        )?;

        register_stdlib_function(
            codegen,
            "format_iso_datetime",
            &params_to_types(&[(WasmType::I64, "timestamp".to_string())]),
            Some(WasmType::I32),
            self.generate_format_iso_datetime_function()
        )?;

        // Custom formatting
        register_stdlib_function(
            codegen,
            "format_custom",
            &params_to_types(&[
                (WasmType::I64, "timestamp".to_string()),
                (WasmType::I32, "format_str".to_string()),
            ]),
            Some(WasmType::I32),
            self.generate_format_custom_function()
        )?;

        // Standard format functions
        register_stdlib_function(
            codegen,
            "format_rfc3339",
            &params_to_types(&[(WasmType::I64, "timestamp".to_string())]),
            Some(WasmType::I32),
            self.generate_format_rfc3339_function()
        )?;

        register_stdlib_function(
            codegen,
            "format_rfc2822",
            &params_to_types(&[(WasmType::I64, "timestamp".to_string())]),
            Some(WasmType::I32),
            self.generate_format_rfc2822_function()
        )?;

        register_stdlib_function(
            codegen,
            "format_relative",
            &params_to_types(&[(WasmType::I64, "timestamp".to_string())]),
            Some(WasmType::I32),
            self.generate_format_relative_function()
        )?;

        Ok(())
    }

    fn generate_format_date_function(&self) -> Vec<Instruction> {
        vec![
            // Load timestamp
            Instruction::LocalGet(0),
            // Call helper to format date (YYYY-MM-DD)
            Instruction::Call(5), // format_date import
            // Result is string pointer (I32)
        ]
    }

    fn generate_format_time_function(&self) -> Vec<Instruction> {
        vec![
            // Load timestamp
            Instruction::LocalGet(0),
            // Call helper to format time (HH:mm:ss)
            Instruction::Call(6), // format_time import
            // Result is string pointer (I32)
        ]
    }

    fn generate_format_iso_datetime_function(&self) -> Vec<Instruction> {
        vec![
            // Load timestamp
            Instruction::LocalGet(0),
            // Call helper to format ISO datetime (YYYY-MM-DDTHH:mm:ssZ)
            Instruction::Call(7), // format_iso_datetime import
            // Result is string pointer (I32)
        ]
    }

    fn generate_format_custom_function(&self) -> Vec<Instruction> {
        vec![
            // Load timestamp and format string
            Instruction::LocalGet(0),
            Instruction::LocalGet(1),
            // Call helper to apply custom format
            Instruction::Call(8), // format_custom import
            // Result is string pointer (I32)
        ]
    }

    fn generate_format_rfc3339_function(&self) -> Vec<Instruction> {
        vec![
            // Load timestamp
            Instruction::LocalGet(0),
            // Call helper to format RFC3339 (YYYY-MM-DDTHH:mm:ss.sssZ)
            Instruction::Call(9), // format_rfc3339 import
            // Result is string pointer (I32)
        ]
    }

    fn generate_format_rfc2822_function(&self) -> Vec<Instruction> {
        vec![
            // Load timestamp
            Instruction::LocalGet(0),
            // Call helper to format RFC2822 (Thu, 13 Feb 2009 23:31:30 GMT)
            Instruction::Call(10), // format_rfc2822 import
            // Result is string pointer (I32)
        ]
    }

    fn generate_format_relative_function(&self) -> Vec<Instruction> {
        vec![
            // Load timestamp
            Instruction::LocalGet(0),
            // Call helper to format relative time (e.g., "2 hours ago")
            Instruction::Call(11), // format_relative import
            // Result is string pointer (I32)
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasmtime::{Store, Module, Instance, Val};

    struct TestFormat {
        store: Store<()>,
        instance: Instance,
        codegen: CodeGenerator,
    }

    impl TestFormat {
        fn new() -> Result<Self, CompilerError> {
            let mut codegen = CodeGenerator::new();
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
    }

    #[test]
    fn test_format_date() -> Result<(), CompilerError> {
        let mut format = TestFormat::new()?;
        let timestamp = 1234567890; // 2009-02-13 23:31:30 UTC
        let result = format.call_func("format_date", &[Val::I64(timestamp)])?;
        assert!(result.unwrap_i32() > 0, "Format date should return valid string pointer");
        Ok(())
    }

    #[test]
    fn test_format_iso_datetime() -> Result<(), CompilerError> {
        let mut format = TestFormat::new()?;
        let timestamp = 1234567890; // 2009-02-13 23:31:30 UTC
        let result = format.call_func("format_iso_datetime", &[Val::I64(timestamp)])?;
        assert!(result.unwrap_i32() > 0, "Format ISO datetime should return valid string pointer");
        Ok(())
    }

    #[test]
    fn test_format_rfc3339() -> Result<(), CompilerError> {
        let mut format = TestFormat::new()?;
        let timestamp = 1234567890; // 2009-02-13 23:31:30 UTC
        let result = format.call_func("format_rfc3339", &[Val::I64(timestamp)])?;
        assert!(result.unwrap_i32() > 0, "Format RFC3339 should return valid string pointer");
        Ok(())
    }

    #[test]
    fn test_format_relative() -> Result<(), CompilerError> {
        let mut format = TestFormat::new()?;
        let timestamp = 1234567890; // 2009-02-13 23:31:30 UTC
        let result = format.call_func("format_relative", &[Val::I64(timestamp)])?;
        assert!(result.unwrap_i32() > 0, "Format relative should return valid string pointer");
        Ok(())
    }
} 