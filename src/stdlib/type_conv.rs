use crate::codegen::CodeGenerator;
use crate::types::WasmType;
use crate::error::CompilerError;
use wasm_encoder::{Instruction, MemArg, BlockType, ValType};
use crate::stdlib::MemoryManager;
use crate::stdlib::register_stdlib_function;

/// Type conversion operations implementation
pub struct TypeConvOperations {
    heap_start: usize,
}

impl TypeConvOperations {
    pub fn new(heap_start: usize) -> Self {
        Self { heap_start }
    }

    pub fn register_functions(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // Helper function to convert parameter types
        let params_to_types = |params: &[(WasmType, String)]| -> Vec<WasmType> {
            params.iter().map(|(t, _)| *t).collect()
        };

        // Register type conversion functions
        register_stdlib_function(
            codegen,
            "i32_to_i64",
            &params_to_types(&[(WasmType::I32, "value".to_string())]),
            Some(WasmType::I64),
            self.generate_i32_to_i64_function()
        )?;

        register_stdlib_function(
            codegen,
            "i64_to_i32",
            &params_to_types(&[(WasmType::I64, "value".to_string())]),
            Some(WasmType::I32),
            self.generate_i64_to_i32_function()
        )?;

        register_stdlib_function(
            codegen,
            "i32_to_f64",
            &params_to_types(&[(WasmType::I32, "value".to_string())]),
            Some(WasmType::F64),
            self.generate_i32_to_f64_function()
        )?;

        register_stdlib_function(
            codegen,
            "f64_to_i32",
            &params_to_types(&[(WasmType::F64, "value".to_string())]),
            Some(WasmType::I32),
            self.generate_f64_to_i32_function()
        )?;

        // Numeric conversions
        register_stdlib_function(
            codegen,
            "to_number",
            &params_to_types(&[(WasmType::I32, "str_ptr".to_string())]),
            Some(WasmType::F64),
            self.generate_to_number_function()
        )?;

        register_stdlib_function(
            codegen,
            "to_integer",
            &params_to_types(&[(WasmType::F64, "value".to_string())]),
            Some(WasmType::I32),
            self.generate_to_integer_function()
        )?;

        register_stdlib_function(
            codegen,
            "to_unsigned",
            &params_to_types(&[(WasmType::I32, "value".to_string())]),
            Some(WasmType::I32),
            self.generate_to_unsigned_function()
        )?;

        register_stdlib_function(
            codegen,
            "to_long",
            &params_to_types(&[(WasmType::F64, "value".to_string())]),
            Some(WasmType::I64),
            self.generate_to_long_function()
        )?;

        register_stdlib_function(
            codegen,
            "to_ulong",
            &params_to_types(&[(WasmType::I64, "value".to_string())]),
            Some(WasmType::I64),
            self.generate_to_ulong_function()
        )?;

        register_stdlib_function(
            codegen,
            "to_byte",
            &params_to_types(&[(WasmType::I32, "value".to_string())]),
            Some(WasmType::I32),
            self.generate_to_byte_function()
        )?;

        // String conversions
        register_stdlib_function(
            codegen,
            "to_string",
            &params_to_types(&[(WasmType::F64, "value".to_string())]),
            Some(WasmType::I32),
            self.generate_to_string_function()
        )?;

        // Boolean conversions
        register_stdlib_function(
            codegen,
            "parse_bool",
            &params_to_types(&[(WasmType::I32, "str_ptr".to_string())]),
            Some(WasmType::I32),
            self.generate_parse_bool_function()
        )?;

        register_stdlib_function(
            codegen,
            "bool_to_string",
            &params_to_types(&[(WasmType::I32, "value".to_string())]),
            Some(WasmType::I32),
            self.generate_bool_to_string_function()
        )?;

        register_stdlib_function(
            codegen,
            "int_to_string",
            &params_to_types(&[(WasmType::I32, "value".to_string())]),
            Some(WasmType::I32),
            self.generate_int_to_string_function()
        )?;

        register_stdlib_function(
            codegen,
            "float_to_string",
            &params_to_types(&[(WasmType::F64, "value".to_string())]),
            Some(WasmType::I32),
            self.generate_float_to_string_function()
        )?;

        register_stdlib_function(
            codegen,
            "string_to_int",
            &params_to_types(&[(WasmType::I32, "str_ptr".to_string())]),
            Some(WasmType::I32),
            self.generate_string_to_int_function()
        )?;

        register_stdlib_function(
            codegen,
            "string_to_float",
            &params_to_types(&[(WasmType::I32, "str_ptr".to_string())]),
            Some(WasmType::F64),
            self.generate_string_to_float_function()
        )?;

        // Register float_to_int function
        register_stdlib_function(
            codegen,
            "float_to_int",
            &params_to_types(&[(WasmType::F32, "value".to_string())]),
            Some(WasmType::I32),
            self.generate_float_to_int_function()
        )?;

        // Register int_to_float function
        register_stdlib_function(
            codegen,
            "int_to_float",
            &params_to_types(&[(WasmType::I32, "value".to_string())]),
            Some(WasmType::F32),
            self.generate_int_to_float_function()
        )?;

        // Register byte_to_int function
        register_stdlib_function(
            codegen,
            "byte_to_int",
            &params_to_types(&[(WasmType::I32, "value".to_string())]),
            Some(WasmType::I32),
            self.generate_byte_to_int_function()
        )?;

        // Register int_to_byte function
        register_stdlib_function(
            codegen,
            "int_to_byte",
            &params_to_types(&[(WasmType::I32, "value".to_string())]),
            Some(WasmType::I32),
            self.generate_int_to_byte_function()
        )?;

        Ok(())
    }

    fn generate_i32_to_i64_function(&self) -> Vec<Instruction> {
        vec![
            // Load i32 value
            Instruction::LocalGet(0),
            // Convert to i64
            Instruction::I64ExtendI32S,
        ]
    }

    fn generate_i64_to_i32_function(&self) -> Vec<Instruction> {
        vec![
            // Load i64 value
            Instruction::LocalGet(0),
            // Convert to i32
            Instruction::I32WrapI64,
        ]
    }

    fn generate_i32_to_f64_function(&self) -> Vec<Instruction> {
        vec![
            // Load i32 value
            Instruction::LocalGet(0),
            // Convert to f64
            Instruction::F64ConvertI32S,
        ]
    }

    fn generate_f64_to_i32_function(&self) -> Vec<Instruction> {
        vec![
            // Load f64 value
            Instruction::LocalGet(0),
            // Convert to i32
            Instruction::I32TruncF64S,
        ]
    }

    fn generate_to_number_function(&self) -> Vec<Instruction> {
        vec![
            // Get string pointer
            Instruction::LocalGet(0),
            
            // Call string to number conversion helper
            Instruction::Call(6), // Assuming import index 6 is string_to_number
            
            // Result is already F64
        ]
    }

    fn generate_to_integer_function(&self) -> Vec<Instruction> {
        vec![
            // Get float value
            Instruction::LocalGet(0),
            
            // Convert to integer (truncate)
            Instruction::I32TruncF64S,
        ]
    }

    fn generate_to_unsigned_function(&self) -> Vec<Instruction> {
        vec![
            // Get signed integer
            Instruction::LocalGet(0),
            
            // Convert to unsigned by masking
            Instruction::I32Const(-1), // All bits set (0xFFFFFFFF as i32)
            Instruction::I32And,
        ]
    }

    fn generate_to_long_function(&self) -> Vec<Instruction> {
        vec![
            // Get float value
            Instruction::LocalGet(0),
            
            // Convert to long integer (truncate)
            Instruction::I64TruncF64S,
        ]
    }

    fn generate_to_ulong_function(&self) -> Vec<Instruction> {
        vec![
            // Get signed long
            Instruction::LocalGet(0),
            
            // Convert to unsigned by masking
            Instruction::I64Const(-1), // All bits set
            Instruction::I64And,
        ]
    }

    fn generate_to_byte_function(&self) -> Vec<Instruction> {
        vec![
            // Get integer value
            Instruction::LocalGet(0),
            
            // Mask to byte range (0-255)
            Instruction::I32Const(0xFF),
            Instruction::I32And,
        ]
    }

    fn generate_to_string_function(&self) -> Vec<Instruction> {
        vec![
            // Allocate memory for result string (max 32 chars)
            Instruction::I32Const(32),
            Instruction::Call(3), // Call memory allocator
            
            // Store result pointer
            Instruction::LocalTee(1),
            
            // Get number to convert
            Instruction::LocalGet(0),
            
            // Call number to string conversion helper
            Instruction::Call(7), // Assuming import index 7 is number_to_string
            
            // Return string pointer
            Instruction::LocalGet(1),
        ]
    }

    fn generate_parse_bool_function(&self) -> Vec<Instruction> {
        vec![
            // Get string pointer
            Instruction::LocalGet(0),
            
            // Load first character
            Instruction::I32Load8U(MemArg {
                offset: 0,
                align: 0,
                memory_index: 0,
            }),
            
            // Compare with 't' or 'T'
            Instruction::I32Const(116), // ASCII 't'
            Instruction::I32Eq,
            
            Instruction::LocalGet(0),
            Instruction::I32Load8U(MemArg {
                offset: 0,
                align: 0,
                memory_index: 0,
            }),
            Instruction::I32Const(84), // ASCII 'T'
            Instruction::I32Eq,
            
            // Combine conditions with OR
            Instruction::I32Or,
            
            // Result is already a boolean (0 or 1)
        ]
    }

    fn generate_bool_to_string_function(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // Get boolean value
        instructions.push(Instruction::LocalGet(0));
        
        // Call host function for bool to string conversion
        instructions.push(Instruction::Call(18)); // Import index for bool_to_string
        
        instructions
    }

    fn generate_int_to_string_function(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // Get integer value
        instructions.push(Instruction::LocalGet(0));
        
        // Call host function for int to string conversion
        instructions.push(Instruction::Call(17)); // Import index for int_to_string
        
        instructions
    }

    fn generate_float_to_string_function(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // Get float value
        instructions.push(Instruction::LocalGet(0));
        
        // Call host function for float to string conversion
        instructions.push(Instruction::Call(16)); // Import index for float_to_string
        
        instructions
    }

    fn generate_float_to_int_function(&self) -> Vec<Instruction> {
        vec![
            // Load the float argument
            Instruction::LocalGet(0),

            // Convert to int using trunc instruction
            Instruction::I32TruncF32S,
        ]
    }

    fn generate_int_to_float_function(&self) -> Vec<Instruction> {
        vec![
            // Load the int argument
            Instruction::LocalGet(0),

            // Convert to float using convert instruction
            Instruction::F32ConvertI32S,
        ]
    }

    fn generate_string_to_int_function(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // Get string pointer
        instructions.push(Instruction::LocalGet(0));
        
        // Load string length
        instructions.push(Instruction::I32Load(MemArg {
            offset: 0,
            align: 2,
            memory_index: 0
        }));
        instructions.push(Instruction::LocalSet(1)); // Store length
        
        // Initialize result
        instructions.push(Instruction::I32Const(0));
        instructions.push(Instruction::LocalSet(2)); // Store result
        
        // Initialize sign (1 for positive, -1 for negative)
        instructions.push(Instruction::I32Const(1));
        instructions.push(Instruction::LocalSet(3)); // Store sign
        
        // Initialize index
        instructions.push(Instruction::I32Const(0));
        instructions.push(Instruction::LocalSet(4)); // Store index
        
        // Check if first character is '-'
        instructions.push(Instruction::LocalGet(0));
        instructions.push(Instruction::I32Const(4)); // Skip length header
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::I32Load8U(MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        }));
        instructions.push(Instruction::I32Const(45)); // ASCII '-'
        instructions.push(Instruction::I32Eq);
        
        // If negative, set sign to -1 and increment index
        instructions.push(Instruction::If(BlockType::Result(ValType::I32)));
        instructions.push(Instruction::I32Const(-1));
        instructions.push(Instruction::LocalSet(3));
        instructions.push(Instruction::LocalGet(4));
        instructions.push(Instruction::I32Const(1));
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::LocalSet(4));
        instructions.push(Instruction::End);
        
        // Main loop
        instructions.push(Instruction::Loop(BlockType::Result(ValType::I32)));
        
        // Check if we've reached the end
        instructions.push(Instruction::LocalGet(4));
        instructions.push(Instruction::LocalGet(1));
        instructions.push(Instruction::I32LtU);
        
        instructions.push(Instruction::If(BlockType::Result(ValType::I32)));
        
        // Load current character
        instructions.push(Instruction::LocalGet(0));
        instructions.push(Instruction::I32Const(4)); // Skip length header
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::LocalGet(4));
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::I32Load8U(MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        }));
        instructions.push(Instruction::LocalSet(5)); // Store current char
        
        // Check if character is a digit
        instructions.push(Instruction::LocalGet(5));
        instructions.push(Instruction::I32Const(48)); // ASCII '0'
        instructions.push(Instruction::I32GeU);
        instructions.push(Instruction::LocalGet(5));
        instructions.push(Instruction::I32Const(57)); // ASCII '9'
        instructions.push(Instruction::I32LeU);
        instructions.push(Instruction::I32And);
        
        // If digit, update result
        instructions.push(Instruction::If(BlockType::Result(ValType::I32)));
        instructions.push(Instruction::LocalGet(2));
        instructions.push(Instruction::I32Const(10));
        instructions.push(Instruction::I32Mul);
        instructions.push(Instruction::LocalGet(5));
        instructions.push(Instruction::I32Const(48)); // ASCII '0'
        instructions.push(Instruction::I32Sub);
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::LocalSet(2));
        
        // Increment index
        instructions.push(Instruction::LocalGet(4));
        instructions.push(Instruction::I32Const(1));
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::LocalSet(4));
        
        // Continue loop
        instructions.push(Instruction::Br(1));
        
        instructions.push(Instruction::End);
        instructions.push(Instruction::End);
        
        // Return result * sign
        instructions.push(Instruction::LocalGet(2));
        instructions.push(Instruction::LocalGet(3));
        instructions.push(Instruction::I32Mul);
        
        instructions
    }

    fn generate_string_to_float_function(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // Get string pointer
        instructions.push(Instruction::LocalGet(0));
        
        // Call host function for string to float conversion
        instructions.push(Instruction::Call(15)); // Import index for string_to_float
        
        instructions
    }

    fn generate_byte_to_int_function(&self) -> Vec<Instruction> {
        vec![
            Instruction::LocalGet(0),
            Instruction::I32Load8U(MemArg {
                offset: 0,
                align: 0,
                memory_index: 0,
            }),
        ]
    }

    fn generate_int_to_byte_function(&self) -> Vec<Instruction> {
        vec![
            Instruction::LocalGet(0),
            Instruction::I32Store8(MemArg {
                offset: 0,
                align: 0,
                memory_index: 0
            }),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::CodeGenerator;
    use wasmtime::{Engine, Instance, Module, Store, Val};
    use std::cell::RefCell;

    fn setup_test_environment() -> (Store<()>, Instance) {
            let mut codegen = CodeGenerator::new();
        let memory = RefCell::new(MemoryManager::new(16, Some(1024))); // 16 pages, heap starts at 1024
        let type_conv = TypeConvOperations::new(1024);
        type_conv.register_functions(&mut codegen).unwrap();

        let engine = Engine::default();
        let wasm_bytes = codegen.finish();
        let module = Module::new(&engine, &wasm_bytes).unwrap();
            let mut store = Store::new(&engine, ());
        let instance = Instance::new(&mut store, &module, &[]).unwrap();
        (store, instance)
    }

    #[test]
    fn test_i32_to_f64() {
        let (mut store, instance) = setup_test_environment();
        let conv = instance.get_func(&mut store, "i32_to_f64").unwrap();
        
        let value = 42;
        let mut results = vec![Val::F64(0)];
        conv.call(&mut store, &[Val::I32(value)], &mut results).unwrap();
        
        let result = f64::from_bits(results[0].unwrap_f64());
        assert!((result - value as f64).abs() < f64::EPSILON);
    }

    #[test]
    fn test_f64_to_i32() {
        let (mut store, instance) = setup_test_environment();
        let conv = instance.get_func(&mut store, "f64_to_i32").unwrap();
        
        let value = 42.0;
        let mut results = vec![Val::I32(0)];
        conv.call(&mut store, &[Val::F64(f64::to_bits(value))], &mut results).unwrap();
        
        assert_eq!(results[0].unwrap_i32(), value as i32);
    }

    #[test]
    fn test_bool_to_i32() {
        let (mut store, instance) = setup_test_environment();
        let conv = instance.get_func(&mut store, "bool_to_i32").unwrap();
        
        let mut results = vec![Val::I32(0)];
        conv.call(&mut store, &[Val::I32(1)], &mut results).unwrap();
        assert_eq!(results[0].unwrap_i32(), 1);

        conv.call(&mut store, &[Val::I32(0)], &mut results).unwrap();
        assert_eq!(results[0].unwrap_i32(), 0);
    }

    #[test]
    fn test_i32_to_bool() {
        let (mut store, instance) = setup_test_environment();
        let conv = instance.get_func(&mut store, "i32_to_bool").unwrap();
        
        let mut results = vec![Val::I32(0)];
        conv.call(&mut store, &[Val::I32(42)], &mut results).unwrap();
        assert_eq!(results[0].unwrap_i32(), 1);

        conv.call(&mut store, &[Val::I32(0)], &mut results).unwrap();
        assert_eq!(results[0].unwrap_i32(), 0);
    }
} 