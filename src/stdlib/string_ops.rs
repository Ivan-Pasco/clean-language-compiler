use crate::error::{CompilerError, ErrorContext, ErrorType};
use wasm_encoder::{
    BlockType, Function, Instruction, MemArg, ValType,
};
use crate::codegen::CodeGenerator;
use crate::types::{WasmType, to_tuple, from_tuple, wasm_type_to_tuple, wasm_types_to_tuples};
use std::collections::HashMap;
use crate::stdlib::memory::MemoryManager;
use crate::stdlib::register_stdlib_function;

pub const STRING_TYPE_ID: u32 = 1;

pub struct StringManager {
    memory_manager: MemoryManager,
}

pub struct StringOperations {
    heap_start: usize,
    memory_manager: MemoryManager,
}

impl StringManager {
    pub fn new(memory_manager: MemoryManager) -> Self {
        Self { memory_manager }
    }

    pub fn register_functions(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        register_stdlib_function(
            codegen,
            "string_allocate",
            &[WasmType::I32], // length
            Some(WasmType::I32), // string pointer
            self.generate_string_allocate()
        )?;

        register_stdlib_function(
            codegen,
            "string_get",
            &[WasmType::I32, WasmType::I32], // string pointer, index
            Some(WasmType::I32), // character
            self.generate_string_get()
        )?;

        register_stdlib_function(
            codegen,
            "string_set",
            &[WasmType::I32, WasmType::I32, WasmType::I32], // string pointer, index, character
            Some(WasmType::I32), // string pointer
            self.generate_string_set()
        )?;

        Ok(())
    }

    fn generate_string_allocate(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        instructions.push(Instruction::I32Const(STRING_TYPE_ID.try_into().unwrap()));
        instructions.push(Instruction::Call(0)); // Call memory.allocate
        instructions
    }

    fn generate_string_get(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        instructions.push(Instruction::I32Add); // Add pointer and index
        instructions.push(Instruction::I32Load8U(MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        })); // Load byte
        instructions
    }

    fn generate_string_set(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        instructions.push(Instruction::I32Add); // Add pointer and index
        instructions.push(Instruction::I32Store8(MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        })); // Store byte
        instructions
    }

    pub fn allocate_string(&mut self, length: usize) -> Result<usize, CompilerError> {
        let ptr = self.memory_manager.allocate(length + 16, STRING_TYPE_ID)?;
        
        // Store length in header
        self.memory_manager.store_i32(ptr, length as i32)?;
        
        Ok(ptr)
    }

    pub fn get_string(&self, string_ptr: usize) -> Result<String, CompilerError> {
        // Check type
        if self.memory_manager.get_type_id(string_ptr)? != STRING_TYPE_ID {
            return Err(CompilerError::type_error(
                "Invalid string pointer",
                Some("Ensure the string pointer is valid".to_string()),
                None
            ));
        }
        
        // Get length from header
        let length = i32::from_le_bytes([
            self.memory_manager.data[string_ptr],
            self.memory_manager.data[string_ptr + 1],
            self.memory_manager.data[string_ptr + 2],
            self.memory_manager.data[string_ptr + 3],
        ]) as usize;
        
        // Get string data
        let data = self.memory_manager.data[string_ptr + 16..string_ptr + 16 + length].to_vec();
        
        // Convert to string
        String::from_utf8(data).map_err(|e| CompilerError::type_error(
            format!("Invalid UTF-8 string: {}", e),
            Some("String contains invalid UTF-8 sequences".to_string()),
            None
        ))
    }

    pub fn set_string(&mut self, string_ptr: usize, value: &str) -> Result<(), CompilerError> {
        // Check type
        if self.memory_manager.get_type_id(string_ptr)? != STRING_TYPE_ID {
            return Err(CompilerError::type_error(
                "Invalid string pointer",
                Some("Ensure the string pointer is valid".to_string()),
                None
            ));
        }
        
        // Get length from header
        let length = i32::from_le_bytes([
            self.memory_manager.data[string_ptr],
            self.memory_manager.data[string_ptr + 1],
            self.memory_manager.data[string_ptr + 2],
            self.memory_manager.data[string_ptr + 3],
        ]) as usize;
        
        // Check length
        if value.len() > length {
            return Err(CompilerError::type_error(
                format!("String too long: {} > {}", value.len(), length),
                Some("Ensure the string fits within allocated space".to_string()),
                None
            ));
        }
        
        // Copy string data
        self.memory_manager.data[string_ptr + 16..string_ptr + 16 + value.len()]
            .copy_from_slice(value.as_bytes());
        
        Ok(())
    }

    pub fn get_char(&self, string_ptr: usize, index: usize) -> Result<u8, CompilerError> {
        // Check type
        if self.memory_manager.get_type_id(string_ptr)? != STRING_TYPE_ID {
            return Err(CompilerError::type_error(
                "Invalid string pointer",
                Some("Ensure the string pointer is valid".to_string()),
                None
            ));
        }
        
        // Get length from header
        let length = i32::from_le_bytes([
            self.memory_manager.data[string_ptr],
            self.memory_manager.data[string_ptr + 1],
            self.memory_manager.data[string_ptr + 2],
            self.memory_manager.data[string_ptr + 3],
        ]) as usize;
        
        // Check bounds
        if index >= length {
            return Err(CompilerError::type_error(
                format!("String index out of bounds: {} >= {}", index, length),
                Some("Ensure index is within string bounds".to_string()),
                None
            ));
        }
        
        Ok(self.memory_manager.data[string_ptr + 16 + index])
    }

    pub fn set_char(&mut self, string_ptr: usize, index: usize, value: u8) -> Result<(), CompilerError> {
        // Check type
        if self.memory_manager.get_type_id(string_ptr)? != STRING_TYPE_ID {
            return Err(CompilerError::type_error(
                "Invalid string pointer",
                Some("Ensure the string pointer is valid".to_string()),
                None
            ));
        }
        
        // Get length from header
        let length = i32::from_le_bytes([
            self.memory_manager.data[string_ptr],
            self.memory_manager.data[string_ptr + 1],
            self.memory_manager.data[string_ptr + 2],
            self.memory_manager.data[string_ptr + 3],
        ]) as usize;
        
        // Check bounds
        if index >= length {
            return Err(CompilerError::type_error(
                format!("String index out of bounds: {} >= {}", index, length),
                Some("Ensure index is within string bounds".to_string()),
                None
            ));
        }
        
        self.memory_manager.data[string_ptr + 16 + index] = value;
        Ok(())
    }
}

impl StringOperations {
    pub fn new(heap_start: usize) -> Self {
        Self {
            heap_start,
            memory_manager: MemoryManager::new(1, Some(10)),
        }
    }

    pub fn register_functions(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // Register string operations
        register_stdlib_function(
            codegen,
            "string.concat",
            &[WasmType::I32, WasmType::I32], // string1, string2
            Some(WasmType::I32), // result
            self.generate_string_concat()
        )?;

        // Register string comparison function
        register_stdlib_function(
            codegen,
            "string.compare",
            &[WasmType::I32, WasmType::I32], // string1, string2
            Some(WasmType::I32), // result (-1, 0, 1)
            self.generate_string_compare()
        )?;

        // Register string length function
        register_stdlib_function(
            codegen,
            "string.length",
            &[WasmType::I32], // string pointer
            Some(WasmType::I32), // length
            self.generate_string_length()
        )?;

        Ok(())
    }

    fn generate_string_concat(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // Store input parameters in locals for easier access
        instructions.push(Instruction::LocalGet(0)); // string1 ptr
        instructions.push(Instruction::LocalTee(2)); // store in local 2
        
        // Get string1 length at header
        instructions.push(Instruction::I32Load(MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));
        instructions.push(Instruction::LocalSet(3)); // store length1 in local 3
        
        instructions.push(Instruction::LocalGet(1)); // string2 ptr
        instructions.push(Instruction::LocalTee(4)); // store in local 4
        
        // Get string2 length at header
        instructions.push(Instruction::I32Load(MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));
        instructions.push(Instruction::LocalSet(5)); // store length2 in local 5
        
        // Calculate total length
        instructions.push(Instruction::LocalGet(3)); // length1
        instructions.push(Instruction::LocalGet(5)); // length2
        instructions.push(Instruction::I32Add); // length1 + length2
        instructions.push(Instruction::LocalTee(6)); // store total length in local 6
        
        // Allocate new string
        instructions.push(Instruction::I32Const(STRING_TYPE_ID as i32));
        instructions.push(Instruction::Call(0)); // Call memory.allocate
        instructions.push(Instruction::LocalTee(7)); // store result pointer in local 7
        
        // Store total length in header
        instructions.push(Instruction::LocalGet(6));
        instructions.push(Instruction::I32Store(MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));
        
        // Copy characters from string1
        // Loop index at local 8
        instructions.push(Instruction::I32Const(0));
        instructions.push(Instruction::LocalSet(8));
        
        // Loop for string1
        instructions.push(Instruction::Block(BlockType::Empty));
        instructions.push(Instruction::Loop(BlockType::Empty));
        
        // Check if i < length1
        instructions.push(Instruction::LocalGet(8));
        instructions.push(Instruction::LocalGet(3));
        instructions.push(Instruction::I32LtU);
        instructions.push(Instruction::BrIf(0)); // Continue loop if true
        
        // Break loop if condition is false
        instructions.push(Instruction::Br(1));
        
        // Loop body: copy character
        instructions.push(Instruction::LocalGet(7)); // result ptr
        instructions.push(Instruction::LocalGet(8)); // index
        instructions.push(Instruction::I32Add); // result ptr + index
        
        instructions.push(Instruction::LocalGet(2)); // string1 ptr
        instructions.push(Instruction::LocalGet(8)); // index
        instructions.push(Instruction::I32Add); // string1 ptr + index
        
        // Load char from string1
        instructions.push(Instruction::I32Load8U(MemArg {
            offset: 16, // Skip header
            align: 0,
            memory_index: 0,
        }));
        
        // Store char to result
        instructions.push(Instruction::I32Store8(MemArg {
            offset: 16, // Skip header
            align: 0,
            memory_index: 0,
        }));
        
        // Increment index
        instructions.push(Instruction::LocalGet(8));
        instructions.push(Instruction::I32Const(1));
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::LocalSet(8));
        
        // Jump to loop start
        instructions.push(Instruction::Br(0));
        
        // End loop
        instructions.push(Instruction::End);
        instructions.push(Instruction::End);
        
        // Copy characters from string2
        // Reset loop index
        instructions.push(Instruction::I32Const(0));
        instructions.push(Instruction::LocalSet(8));
        
        // Loop for string2
        instructions.push(Instruction::Block(BlockType::Empty));
        instructions.push(Instruction::Loop(BlockType::Empty));
        
        // Check if i < length2
        instructions.push(Instruction::LocalGet(8));
        instructions.push(Instruction::LocalGet(5));
        instructions.push(Instruction::I32LtU);
        instructions.push(Instruction::BrIf(0)); // Continue loop if true
        
        // Break loop if condition is false
        instructions.push(Instruction::Br(1));
        
        // Loop body: copy character
        instructions.push(Instruction::LocalGet(7)); // result ptr
        instructions.push(Instruction::LocalGet(3)); // length1 (offset for string2)
        instructions.push(Instruction::LocalGet(8)); // index
        instructions.push(Instruction::I32Add); // length1 + index
        instructions.push(Instruction::I32Add); // result ptr + length1 + index
        
        instructions.push(Instruction::LocalGet(4)); // string2 ptr
        instructions.push(Instruction::LocalGet(8)); // index
        instructions.push(Instruction::I32Add); // string2 ptr + index
        
        // Load char from string2
        instructions.push(Instruction::I32Load8U(MemArg {
            offset: 16, // Skip header
            align: 0,
            memory_index: 0,
        }));
        
        // Store char to result
        instructions.push(Instruction::I32Store8(MemArg {
            offset: 16, // Skip header
            align: 0,
            memory_index: 0,
        }));
        
        // Increment index
        instructions.push(Instruction::LocalGet(8));
        instructions.push(Instruction::I32Const(1));
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::LocalSet(8));
        
        // Jump to loop start
        instructions.push(Instruction::Br(0));
        
        // End loop
        instructions.push(Instruction::End);
        instructions.push(Instruction::End);
        
        // Return result pointer
        instructions.push(Instruction::LocalGet(7));
        
        instructions
    }

    fn generate_string_compare(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // Load string1 pointer and get length
        instructions.push(Instruction::LocalGet(0));
        instructions.push(Instruction::I32Load(MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));
        instructions.push(Instruction::LocalSet(2)); // length1
        
        // Load string2 pointer and get length
        instructions.push(Instruction::LocalGet(1));
        instructions.push(Instruction::I32Load(MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));
        instructions.push(Instruction::LocalSet(3)); // length2
        
        // Calculate min length for comparison
        instructions.push(Instruction::LocalGet(2)); // length1
        instructions.push(Instruction::LocalGet(3)); // length2
        instructions.push(Instruction::I32LtU); // length1 < length2
        
        // If length1 < length2, min = length1, else min = length2
        instructions.push(Instruction::If(BlockType::Result(ValType::I32)));
        instructions.push(Instruction::LocalGet(2)); // length1
        instructions.push(Instruction::Else);
        instructions.push(Instruction::LocalGet(3)); // length2
        instructions.push(Instruction::End);
        instructions.push(Instruction::LocalSet(4)); // min length
        
        // Initialize index to 0
        instructions.push(Instruction::I32Const(0));
        instructions.push(Instruction::LocalSet(5)); // index
        
        // Loop to compare characters
        instructions.push(Instruction::Block(BlockType::Result(ValType::I32)));
        instructions.push(Instruction::Loop(BlockType::Empty));
        
        // Check if index < min length
        instructions.push(Instruction::LocalGet(5)); // index
        instructions.push(Instruction::LocalGet(4)); // min length
        instructions.push(Instruction::I32GeU); // index >= min
        instructions.push(Instruction::BrIf(1)); // Break loop if condition is true
        
        // Load char from string1
        instructions.push(Instruction::LocalGet(0)); // string1 ptr
        instructions.push(Instruction::LocalGet(5)); // index
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::I32Load8U(MemArg {
            offset: 16, // Skip header
            align: 0,
            memory_index: 0,
        }));
        instructions.push(Instruction::LocalSet(6)); // char1
        
        // Load char from string2
        instructions.push(Instruction::LocalGet(1)); // string2 ptr
        instructions.push(Instruction::LocalGet(5)); // index
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::I32Load8U(MemArg {
            offset: 16, // Skip header
            align: 0,
            memory_index: 0,
        }));
        instructions.push(Instruction::LocalSet(7)); // char2
        
        // Compare characters
        instructions.push(Instruction::LocalGet(6)); // char1
        instructions.push(Instruction::LocalGet(7)); // char2
        instructions.push(Instruction::I32Ne); // char1 != char2
        
        // If chars are different, compare and return result
        instructions.push(Instruction::If(BlockType::Empty));
        
        // If char1 < char2, return -1, else return 1
        instructions.push(Instruction::LocalGet(6)); // char1
        instructions.push(Instruction::LocalGet(7)); // char2
        instructions.push(Instruction::I32LtU); // char1 < char2
        
        instructions.push(Instruction::If(BlockType::Result(ValType::I32)));
        instructions.push(Instruction::I32Const(-1)); // string1 < string2
        instructions.push(Instruction::Else);
        instructions.push(Instruction::I32Const(1)); // string1 > string2
        instructions.push(Instruction::End);
        
        instructions.push(Instruction::Br(2)); // Return from function
        instructions.push(Instruction::End);
        
        // Increment index
        instructions.push(Instruction::LocalGet(5)); // index
        instructions.push(Instruction::I32Const(1));
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::LocalSet(5));
        
        // Continue loop
        instructions.push(Instruction::Br(0));
        instructions.push(Instruction::End); // End loop
        
        // Strings are equal up to min length, now compare lengths
        instructions.push(Instruction::LocalGet(2)); // length1
        instructions.push(Instruction::LocalGet(3)); // length2
        instructions.push(Instruction::I32Eq); // length1 == length2
        
        // If lengths are equal, strings are equal
        instructions.push(Instruction::If(BlockType::Result(ValType::I32)));
        instructions.push(Instruction::I32Const(0)); // Strings are equal
        instructions.push(Instruction::Else);
        
        // If length1 < length2, return -1, else return 1
        instructions.push(Instruction::LocalGet(2)); // length1
        instructions.push(Instruction::LocalGet(3)); // length2
        instructions.push(Instruction::I32LtU); // length1 < length2
        
        instructions.push(Instruction::If(BlockType::Result(ValType::I32)));
        instructions.push(Instruction::I32Const(-1)); // string1 < string2
        instructions.push(Instruction::Else);
        instructions.push(Instruction::I32Const(1)); // string1 > string2
        instructions.push(Instruction::End);
        
        instructions.push(Instruction::End);
        instructions.push(Instruction::End); // End block
        
        instructions
    }

    fn generate_string_length(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // Get string pointer and load length from header
        instructions.push(Instruction::LocalGet(0));
        instructions.push(Instruction::I32Load(MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));
        
        instructions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_operations() {
        let mut string_manager = StringManager::new(MemoryManager::new(1, Some(10)));
        
        // Test string allocation
        let string_ptr = string_manager.allocate_string(5).unwrap();
        assert!(string_ptr >= 16); // Header size
        
        // Test string set/get
        string_manager.set_string(string_ptr, "Hello").unwrap();
        let value = string_manager.get_string(string_ptr).unwrap();
        assert_eq!(value, "Hello");
        
        // Test character access
        assert_eq!(string_manager.get_char(string_ptr, 0).unwrap(), b'H');
        string_manager.set_char(string_ptr, 0, b'h').unwrap();
        assert_eq!(string_manager.get_char(string_ptr, 0).unwrap(), b'h');
        
        // Test bounds checking
        assert!(string_manager.get_char(string_ptr, 5).is_err());
        assert!(string_manager.set_char(string_ptr, 5, b'!').is_err());
        
        // Test length checking
        assert!(string_manager.set_string(string_ptr, "Too Long String").is_err());
    }
    
    #[test]
    fn test_string_concat() {
        let engine = wasmtime::Engine::default();
        let memory_manager = MemoryManager::new(1, Some(10));
        let string_ops = StringOperations::new(1024);
        
        let mut codegen = CodeGenerator::new();
        string_ops.register_functions(&mut codegen).unwrap();
        
        // Generate WebAssembly module
        let wasm_bytes = codegen.finish();
        let module = wasmtime::Module::new(&engine, &wasm_bytes).unwrap();
        let mut store = wasmtime::Store::new(&engine, ());
        let instance = wasmtime::Instance::new(&mut store, &module, &[]).unwrap();
        
        // Create test string pointers
        let mut string_manager = StringManager::new(memory_manager);
        let s1_ptr = string_manager.allocate_string(5).unwrap();
        let s2_ptr = string_manager.allocate_string(6).unwrap();
        
        string_manager.set_string(s1_ptr, "Hello").unwrap();
        string_manager.set_string(s2_ptr, "World!").unwrap();
        
        // Call string.concat
        let concat = instance.get_func(&mut store, "string.concat").unwrap();
        let mut results = vec![wasmtime::Val::I32(0)];
        concat.call(&mut store, &[
            wasmtime::Val::I32(s1_ptr as i32),
            wasmtime::Val::I32(s2_ptr as i32),
        ], &mut results).unwrap();
        
        let result_ptr = results[0].unwrap_i32() as usize;
        assert!(result_ptr >= 16);
        
        // Verify the concatenated string
        let result = string_manager.get_string(result_ptr).unwrap();
        assert_eq!(result, "HelloWorld!");
    }
    
    #[test]
    fn test_string_compare() {
        let engine = wasmtime::Engine::default();
        let memory_manager = MemoryManager::new(1, Some(10));
        let string_ops = StringOperations::new(1024);
        
        let mut codegen = CodeGenerator::new();
        string_ops.register_functions(&mut codegen).unwrap();
        
        // Generate WebAssembly module
        let wasm_bytes = codegen.finish();
        let module = wasmtime::Module::new(&engine, &wasm_bytes).unwrap();
        let mut store = wasmtime::Store::new(&engine, ());
        let instance = wasmtime::Instance::new(&mut store, &module, &[]).unwrap();
        
        // Create test string pointers
        let mut string_manager = StringManager::new(memory_manager);
        let s1_ptr = string_manager.allocate_string(3).unwrap();
        let s2_ptr = string_manager.allocate_string(3).unwrap();
        let s3_ptr = string_manager.allocate_string(5).unwrap();
        
        string_manager.set_string(s1_ptr, "abc").unwrap();
        string_manager.set_string(s2_ptr, "abc").unwrap();
        string_manager.set_string(s3_ptr, "abcde").unwrap();
        
        // Call string.compare for equal strings
        let compare = instance.get_func(&mut store, "string.compare").unwrap();
        let mut results = vec![wasmtime::Val::I32(0)];
        compare.call(&mut store, &[
            wasmtime::Val::I32(s1_ptr as i32),
            wasmtime::Val::I32(s2_ptr as i32),
        ], &mut results).unwrap();
        
        assert_eq!(results[0].unwrap_i32(), 0); // Equal strings
        
        // Compare with different length
        compare.call(&mut store, &[
            wasmtime::Val::I32(s1_ptr as i32),
            wasmtime::Val::I32(s3_ptr as i32),
        ], &mut results).unwrap();
        
        assert_eq!(results[0].unwrap_i32(), -1); // s1 < s3
        
        compare.call(&mut store, &[
            wasmtime::Val::I32(s3_ptr as i32),
            wasmtime::Val::I32(s1_ptr as i32),
        ], &mut results).unwrap();
        
        assert_eq!(results[0].unwrap_i32(), 1); // s3 > s1
        
        // Compare with different content
        let s4_ptr = string_manager.allocate_string(3).unwrap();
        string_manager.set_string(s4_ptr, "abd").unwrap();
        
        compare.call(&mut store, &[
            wasmtime::Val::I32(s1_ptr as i32),
            wasmtime::Val::I32(s4_ptr as i32),
        ], &mut results).unwrap();
        
        assert_eq!(results[0].unwrap_i32(), -1); // abc < abd
    }
} 