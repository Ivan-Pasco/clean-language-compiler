use crate::codegen::CodeGenerator;
use crate::types::WasmType;
use crate::error::CompilerError;
use wasm_encoder::Instruction;
use crate::stdlib::register_stdlib_function;

/// Utility operations implementation for common built-in functions
pub struct UtilityOperations {}

impl UtilityOperations {
    pub fn new() -> Self {
        Self {}
    }

    pub fn register_functions(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // Register length function for strings and arrays
        self.register_length_functions(codegen)?;
        
        // Register assertion functions
        self.register_assertion_functions(codegen)?;
        
        // Register type checking functions
        self.register_type_checking_functions(codegen)?;
        
        // Register utility functions
        self.register_utility_functions(codegen)?;
        
        Ok(())
    }

    fn register_length_functions(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // Generic length function for strings
        register_stdlib_function(
            codegen,
            "length",
            &[WasmType::I32], // String pointer
            Some(WasmType::I32), // Length
            vec![
                // Load string length from memory (first 4 bytes)
                Instruction::LocalGet(0),
                Instruction::I32Load(wasm_encoder::MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                }),
            ]
        )?;



        Ok(())
    }

    fn register_assertion_functions(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // Enhanced mustBe function with message
        register_stdlib_function(
            codegen,
            "mustBeWithMessage",
            &[WasmType::I32, WasmType::I32], // condition, message
            None, // void
            self.generate_assert_with_message()
        )?;

        // MustBeEqual function
        register_stdlib_function(
            codegen,
            "mustBeEqual",
            &[WasmType::I32, WasmType::I32], // expected, actual
            None, // void
            self.generate_assert_equals()
        )?;

        // MustNotBeEqual function
        register_stdlib_function(
            codegen,
            "mustNotBeEqual",
            &[WasmType::I32, WasmType::I32], // expected, actual
            None, // void
            self.generate_assert_not_equals()
        )?;



        // MustBeFalse function
        register_stdlib_function(
            codegen,
            "mustBeFalse",
            &[WasmType::I32], // condition
            None, // void
            vec![
                Instruction::LocalGet(0),
                Instruction::I32Const(0),
                Instruction::I32Ne,
                Instruction::If(wasm_encoder::BlockType::Empty),
                    // If condition is true, trap
                    Instruction::Unreachable,
                Instruction::End,
            ]
        )?;

        Ok(())
    }

    fn register_type_checking_functions(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // IsDefined function - returns true if value exists (not null/zero)
        register_stdlib_function(
            codegen,
            "isDefined",
            &[WasmType::I32], // pointer
            Some(WasmType::I32), // boolean
            vec![
                Instruction::LocalGet(0),
                Instruction::I32Const(0),
                Instruction::I32Ne,  // Returns true if NOT zero (i.e., defined)
            ]
        )?;

        // IsNotDefined function - returns true if value doesn't exist (null/zero)
        register_stdlib_function(
            codegen,
            "isNotDefined",
            &[WasmType::I32], // pointer
            Some(WasmType::I32), // boolean
            vec![
                Instruction::LocalGet(0),
                Instruction::I32Const(0),
                Instruction::I32Eq,  // Returns true if zero (i.e., not defined)
            ]
        )?;

        // IsEmpty function (for strings and arrays)
        register_stdlib_function(
            codegen,
            "isEmpty",
            &[WasmType::I32], // pointer
            Some(WasmType::I32), // boolean
            vec![
                // Check if length is 0
                Instruction::LocalGet(0),
                Instruction::I32Load(wasm_encoder::MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                }),
                Instruction::I32Const(0),
                Instruction::I32Eq,
            ]
        )?;

        // IsNotEmpty function
        register_stdlib_function(
            codegen,
            "isNotEmpty",
            &[WasmType::I32], // pointer
            Some(WasmType::I32), // boolean
            vec![
                // Check if length is not 0
                Instruction::LocalGet(0),
                Instruction::I32Load(wasm_encoder::MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                }),
                Instruction::I32Const(0),
                Instruction::I32Ne,
            ]
        )?;

        Ok(())
    }

    fn register_utility_functions(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // Default value function for integers
        register_stdlib_function(
            codegen,
            "defaultInt",
            &[],
            Some(WasmType::I32),
            vec![
                Instruction::I32Const(0),
            ]
        )?;

        // Default value function for floats
        register_stdlib_function(
            codegen,
            "defaultFloat",
            &[],
            Some(WasmType::F64),
            vec![
                Instruction::F64Const(0.0),
            ]
        )?;

        // Default value function for booleans
        register_stdlib_function(
            codegen,
            "defaultBool",
            &[],
            Some(WasmType::I32),
            vec![
                Instruction::I32Const(0), // false
            ]
        )?;

        // Swap function for integers
        register_stdlib_function(
            codegen,
            "swap_int",
            &[WasmType::I32, WasmType::I32], // a, b
            Some(WasmType::I64), // packed result (high 32 bits = b, low 32 bits = a)
            vec![
                // Pack b in high 32 bits, a in low 32 bits
                Instruction::LocalGet(1), // b
                Instruction::I64ExtendI32U,
                Instruction::I64Const(32),
                Instruction::I64Shl,
                Instruction::LocalGet(0), // a
                Instruction::I64ExtendI32U,
                Instruction::I64Or,
            ]
        )?;

        // KeepBetween function for integers
        register_stdlib_function(
            codegen,
            "keepBetween",
            &[WasmType::I32, WasmType::I32, WasmType::I32], // value, min, max
            Some(WasmType::I32), // value kept between bounds
            self.generate_keep_between_int()
        )?;

        // KeepBetween function for floats
        register_stdlib_function(
            codegen,
            "keepBetweenFloat",
            &[WasmType::F64, WasmType::F64, WasmType::F64], // value, min, max
            Some(WasmType::F64), // value kept between bounds
            self.generate_keep_between_float()
        )?;

        Ok(())
    }

    fn generate_assert_with_message(&self) -> Vec<Instruction> {
        vec![
            // Check condition
            Instruction::LocalGet(0),
            Instruction::I32Const(0),
            Instruction::I32Eq,
            Instruction::If(wasm_encoder::BlockType::Empty),
                // If condition is false, we could print the message here
                // For now, just trap
                Instruction::Unreachable,
            Instruction::End,
        ]
    }

    fn generate_assert_equals(&self) -> Vec<Instruction> {
        vec![
            // Compare expected and actual
            Instruction::LocalGet(0), // expected
            Instruction::LocalGet(1), // actual
            Instruction::I32Ne,
            Instruction::If(wasm_encoder::BlockType::Empty),
                // If not equal, trap
                Instruction::Unreachable,
            Instruction::End,
        ]
    }

    fn generate_assert_not_equals(&self) -> Vec<Instruction> {
        vec![
            // Compare expected and actual
            Instruction::LocalGet(0), // expected
            Instruction::LocalGet(1), // actual
            Instruction::I32Eq,
            Instruction::If(wasm_encoder::BlockType::Empty),
                // If equal, trap
                Instruction::Unreachable,
            Instruction::End,
        ]
    }

    fn generate_keep_between_int(&self) -> Vec<Instruction> {
        vec![
            // max(min, min(value, max))
            // First: min(value, max)
            Instruction::LocalGet(0), // value
            Instruction::LocalGet(2), // max
            Instruction::LocalGet(0), // value
            Instruction::LocalGet(2), // max
            Instruction::I32LtS,
            Instruction::Select,
            
            // Then: max(min_result, min)
            Instruction::LocalGet(1), // min
            Instruction::LocalGet(1), // min
            Instruction::LocalGet(0), // value (from previous min operation)
            Instruction::LocalGet(2), // max
            Instruction::LocalGet(0), // value
            Instruction::LocalGet(2), // max
            Instruction::I32LtS,
            Instruction::Select,
            Instruction::I32GtS,
            Instruction::Select,
        ]
    }

    fn generate_keep_between_float(&self) -> Vec<Instruction> {
        vec![
            // max(min, min(value, max))
            Instruction::LocalGet(0), // value
            Instruction::LocalGet(2), // max
            Instruction::F64Min,
            Instruction::LocalGet(1), // min
            Instruction::F64Max,
        ]
    }
} 