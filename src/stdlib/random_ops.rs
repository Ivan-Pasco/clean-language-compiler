use crate::codegen::CodeGenerator;
use crate::types::WasmType;
use crate::error::CompilerError;

use wasm_encoder::Instruction;
use crate::stdlib::register_stdlib_function;

/// Random operations implementation
pub struct RandomOperations {}

impl RandomOperations {
    pub fn new() -> Self {
        Self {}
    }

    pub fn register_functions(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // Helper function to convert parameter types
        let params_to_types = |params: &[(WasmType, String)]| -> Vec<WasmType> {
            params.iter().map(|(t, _)| *t).collect()
        };

        // Basic random functions
        register_stdlib_function(
            codegen,
            "random",
            &[],
            Some(WasmType::F64),
            self.generate_random_function()
        )?;

        register_stdlib_function(
            codegen,
            "random_range",
            &params_to_types(&[
                (WasmType::F64, "min".to_string()),
                (WasmType::F64, "max".to_string()),
            ]),
            Some(WasmType::F64),
            self.generate_random_range_function()
        )?;

        register_stdlib_function(
            codegen,
            "random_int",
            &params_to_types(&[
                (WasmType::I32, "min".to_string()),
                (WasmType::I32, "max".to_string()),
            ]),
            Some(WasmType::I32),
            self.generate_random_int_function()
        )?;

        register_stdlib_function(
            codegen,
            "random_bool",
            &[],
            Some(WasmType::I32),
            self.generate_random_bool_function()
        )?;

        Ok(())
    }

    fn generate_random_function(&self) -> Vec<Instruction> {
        // Linear Congruential Generator parameters
        const A: i64 = 1664525;
        const C: i64 = 1013904223;
        const M: i64 = 1 << 32;

        vec![
            // Load current seed from memory (global)
            Instruction::GlobalGet(0),
            
            // Multiply by A
            Instruction::I64Const(A),
            Instruction::I64Mul,
            
            // Add C
            Instruction::I64Const(C),
            Instruction::I64Add,
            
            // Modulo M (using bitwise AND since M is a power of 2)
            Instruction::I64Const(M - 1),
            Instruction::I64And,
            
            // Store new seed back to memory
            Instruction::GlobalSet(0),
            
            // Convert to float between 0 and 1
            Instruction::F64ConvertI64U,
            Instruction::F64Const(1.0 / (M as f64)),
            Instruction::F64Mul,
        ]
    }

    fn generate_random_range_function(&self) -> Vec<Instruction> {
        vec![
            // Generate random number between 0 and 1
            Instruction::Call(0), // Call random()
            
            // Calculate range size (max - min)
            Instruction::LocalGet(1), // max
            Instruction::LocalGet(0), // min
            Instruction::F64Sub,
            
            // Multiply random by range size
            Instruction::F64Mul,
            
            // Add minimum value
            Instruction::LocalGet(0),
            Instruction::F64Add,
        ]
    }

    fn generate_random_int_function(&self) -> Vec<Instruction> {
        vec![
            // Generate random float
            Instruction::Call(0), // Call random()
            
            // Calculate range size (max - min)
            Instruction::LocalGet(1), // max
            Instruction::LocalGet(0), // min
            Instruction::I32Sub,
            Instruction::I32Const(1),
            Instruction::I32Add, // Add 1 to make max inclusive
            
            // Convert range to float and multiply
            Instruction::F64ConvertI32S,
            Instruction::F64Mul,
            
            // Add minimum value as float
            Instruction::LocalGet(0),
            Instruction::F64ConvertI32S,
            Instruction::F64Add,
            
            // Convert back to integer (floor)
            Instruction::I32TruncF64S,
        ]
    }

    fn generate_random_bool_function(&self) -> Vec<Instruction> {
        vec![
            // Generate random float
            Instruction::Call(0), // Call random()
            
            // Compare with 0.5
            Instruction::F64Const(0.5),
            Instruction::F64Lt,
            
            // Convert comparison result to i32 (0 or 1)
            Instruction::I32WrapI64,
        ]
    }
} 