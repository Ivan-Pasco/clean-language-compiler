use crate::codegen::CodeGenerator;
use crate::types::WasmType;
use crate::error::CompilerError;
use wasm_encoder::Instruction;
use crate::stdlib::register_stdlib_function;

/// Time operations implementation
pub struct TimeOperations {}

impl TimeOperations {
    pub fn new() -> Self {
        Self {}
    }

    pub fn register_functions(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // Helper function to convert parameter types
        let params_to_types = |params: &[(WasmType, String)]| -> Vec<WasmType> {
            params.iter().map(|(t, _)| *t).collect()
        };

        // Current time functions
        register_stdlib_function(
            codegen,
            "now",
            &[],
            Some(WasmType::I64),
            self.generate_now_function()
        )?;

        register_stdlib_function(
            codegen,
            "now_ms",
            &[],
            Some(WasmType::I64),
            self.generate_now_ms_function()
        )?;

        // Time component functions
        register_stdlib_function(
            codegen,
            "hour",
            &params_to_types(&[(WasmType::I64, "timestamp".to_string())]),
            Some(WasmType::I32),
            self.generate_hour_function()
        )?;

        register_stdlib_function(
            codegen,
            "minute",
            &params_to_types(&[(WasmType::I64, "timestamp".to_string())]),
            Some(WasmType::I32),
            self.generate_minute_function()
        )?;

        register_stdlib_function(
            codegen,
            "second",
            &params_to_types(&[(WasmType::I64, "timestamp".to_string())]),
            Some(WasmType::I32),
            self.generate_second_function()
        )?;

        register_stdlib_function(
            codegen,
            "millisecond",
            &params_to_types(&[(WasmType::I64, "timestamp".to_string())]),
            Some(WasmType::I32),
            self.generate_millisecond_function()
        )?;

        // Timezone functions
        register_stdlib_function(
            codegen,
            "hour_utc",
            &params_to_types(&[(WasmType::I64, "timestamp".to_string())]),
            Some(WasmType::I32),
            self.generate_hour_utc_function()
        )?;

        register_stdlib_function(
            codegen,
            "to_utc",
            &params_to_types(&[(WasmType::I64, "timestamp".to_string())]),
            Some(WasmType::I64),
            self.generate_to_utc_function()
        )?;

        register_stdlib_function(
            codegen,
            "from_utc",
            &params_to_types(&[(WasmType::I64, "timestamp".to_string())]),
            Some(WasmType::I64),
            self.generate_from_utc_function()
        )?;

        register_stdlib_function(
            codegen,
            "timezone_offset",
            &[],
            Some(WasmType::I32),
            self.generate_timezone_offset_function()
        )?;

        // Time arithmetic
        register_stdlib_function(
            codegen,
            "add_hours",
            &params_to_types(&[
                (WasmType::I64, "timestamp".to_string()),
                (WasmType::I32, "hours".to_string()),
            ]),
            Some(WasmType::I64),
            self.generate_add_hours_function()
        )?;

        register_stdlib_function(
            codegen,
            "add_minutes",
            &params_to_types(&[
                (WasmType::I64, "timestamp".to_string()),
                (WasmType::I32, "minutes".to_string()),
            ]),
            Some(WasmType::I64),
            self.generate_add_minutes_function()
        )?;

        register_stdlib_function(
            codegen,
            "add_seconds",
            &params_to_types(&[
                (WasmType::I64, "timestamp".to_string()),
                (WasmType::I32, "seconds".to_string()),
            ]),
            Some(WasmType::I64),
            self.generate_add_seconds_function()
        )?;

        Ok(())
    }

    fn generate_now_function(&self) -> Vec<Instruction> {
        vec![
            // Import current timestamp (seconds since epoch)
            Instruction::Call(0), // Assuming import index 0 is current_time
        ]
    }

    fn generate_now_ms_function(&self) -> Vec<Instruction> {
        vec![
            // Import current timestamp in milliseconds
            Instruction::Call(1), // Assuming import index 1 is current_time_ms
        ]
    }

    fn generate_hour_function(&self) -> Vec<Instruction> {
        vec![
            // Load timestamp
            Instruction::LocalGet(0),
            // Convert to hours (UTC)
            Instruction::I64Const(3600), // seconds per hour
            Instruction::I64DivU,
            Instruction::I64Const(24),
            Instruction::I64RemU,
            // Convert to I32
            Instruction::I32WrapI64,
        ]
    }

    fn generate_minute_function(&self) -> Vec<Instruction> {
        vec![
            // Load timestamp
            Instruction::LocalGet(0),
            // Convert to minutes
            Instruction::I64Const(60), // seconds per minute
            Instruction::I64DivU,
            Instruction::I64Const(60),
            Instruction::I64RemU,
            // Convert to I32
            Instruction::I32WrapI64,
        ]
    }

    fn generate_second_function(&self) -> Vec<Instruction> {
        vec![
            // Load timestamp
            Instruction::LocalGet(0),
            // Get seconds component
            Instruction::I64Const(60),
            Instruction::I64RemU,
            // Convert to I32
            Instruction::I32WrapI64,
        ]
    }

    fn generate_millisecond_function(&self) -> Vec<Instruction> {
        vec![
            // Load timestamp
            Instruction::LocalGet(0),
            // Get milliseconds component
            Instruction::I64Const(1000),
            Instruction::I64RemU,
            // Convert to I32
            Instruction::I32WrapI64,
        ]
    }

    fn generate_hour_utc_function(&self) -> Vec<Instruction> {
        vec![
            // Get timestamp
            Instruction::LocalGet(0),
            // Call host function for UTC hour
            Instruction::Call(14), // Import index for UTC hour
            // Convert to i32
            Instruction::I32WrapI64,
        ]
    }

    fn generate_to_utc_function(&self) -> Vec<Instruction> {
        vec![
            // Get timestamp
            Instruction::LocalGet(0),
            // Call host function to convert to UTC
            Instruction::Call(15), // Import index for to UTC conversion
        ]
    }

    fn generate_from_utc_function(&self) -> Vec<Instruction> {
        vec![
            // Get timestamp
            Instruction::LocalGet(0),
            // Call host function to convert from UTC
            Instruction::Call(16), // Import index for from UTC conversion
        ]
    }

    fn generate_timezone_offset_function(&self) -> Vec<Instruction> {
        vec![
            // Call host function for timezone offset
            Instruction::Call(17), // Import index for timezone offset
            // Convert to i32
            Instruction::I32WrapI64,
        ]
    }

    fn generate_add_hours_function(&self) -> Vec<Instruction> {
        vec![
            // Get timestamp and hours
            Instruction::LocalGet(0),
            Instruction::LocalGet(1),
            // Convert hours to seconds
            Instruction::I64ExtendI32S,
            Instruction::I64Const(3600), // seconds per hour
            Instruction::I64Mul,
            // Add to timestamp
            Instruction::I64Add,
        ]
    }

    fn generate_add_minutes_function(&self) -> Vec<Instruction> {
        vec![
            // Get timestamp and minutes
            Instruction::LocalGet(0),
            Instruction::LocalGet(1),
            // Convert minutes to seconds
            Instruction::I64ExtendI32S,
            Instruction::I64Const(60), // seconds per minute
            Instruction::I64Mul,
            // Add to timestamp
            Instruction::I64Add,
        ]
    }

    fn generate_add_seconds_function(&self) -> Vec<Instruction> {
        vec![
            // Get timestamp and seconds
            Instruction::LocalGet(0),
            Instruction::LocalGet(1),
            // Convert to i64 and add
            Instruction::I64ExtendI32S,
            Instruction::I64Add,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasmtime::{Store, Module, Instance};

    #[test]
    fn test_now() -> Result<(), CompilerError> {
        let mut codegen = CodeGenerator::new();
        let time_ops = TimeOperations::new();
        time_ops.register_functions(&mut codegen)?;

        let wasm = codegen.build();
        let engine = wasmtime::Engine::default();
        let module = Module::new(&engine, &wasm)?;
        let mut store = Store::new(&engine, ());
        let instance = Instance::new(&mut store, &module, &[])?;

        let now = instance.get_func(&mut store, "now").unwrap();
        let result = now.call(&mut store, &[], &mut [])?;
        
        // The timestamp should be greater than 0
        assert!(result[0].unwrap_i64() > 0);

        Ok(())
    }

    #[test]
    fn test_add_hours() -> Result<(), CompilerError> {
        let mut codegen = CodeGenerator::new();
        let time_ops = TimeOperations::new();
        time_ops.register_functions(&mut codegen)?;

        let wasm = codegen.build();
        let engine = wasmtime::Engine::default();
        let module = Module::new(&engine, &wasm)?;
        let mut store = Store::new(&engine, ());
        let instance = Instance::new(&mut store, &module, &[])?;

        let add_hours = instance.get_func(&mut store, "add_hours").unwrap();
        let base_time = 1234567890;
        let hours_to_add = 24;
        let result = add_hours.call(
            &mut store,
            &[
                wasmtime::Val::I64(base_time),
                wasmtime::Val::I32(hours_to_add),
            ],
            &mut [],
        )?;
        
        // Check if the result is correct (base_time + hours_to_add * 3600)
        assert_eq!(
            result[0].unwrap_i64(),
            base_time + (hours_to_add as i64 * 3600)
        );

        Ok(())
    }
} 