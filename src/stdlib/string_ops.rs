use crate::error::{CompilerError};
use wasm_encoder::{
    BlockType, Instruction, MemArg, ValType,
};
use crate::codegen::CodeGenerator;
use crate::types::{WasmType};

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
            memory_manager: MemoryManager::new(16, Some(heap_start as u32)),
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
            "string_length",
            &[WasmType::I32], // string pointer
            Some(WasmType::I32), // length
            self.generate_string_length()
        )?;

        // Register new string functions
        register_stdlib_function(
            codegen,
            "string_contains",
            &[WasmType::I32, WasmType::I32], // string, search
            Some(WasmType::I32), // boolean
            self.generate_string_contains()
        )?;

        register_stdlib_function(
            codegen,
            "string_index_of",
            &[WasmType::I32, WasmType::I32], // string, search
            Some(WasmType::I32), // index (-1 if not found)
            self.generate_string_index_of()
        )?;

        register_stdlib_function(
            codegen,
            "string_last_index_of",
            &[WasmType::I32, WasmType::I32], // string, search
            Some(WasmType::I32), // index (-1 if not found)
            self.generate_string_last_index_of()
        )?;

        register_stdlib_function(
            codegen,
            "string_starts_with",
            &[WasmType::I32, WasmType::I32], // string, prefix
            Some(WasmType::I32), // boolean
            self.generate_string_starts_with()
        )?;

        register_stdlib_function(
            codegen,
            "string_ends_with",
            &[WasmType::I32, WasmType::I32], // string, suffix
            Some(WasmType::I32), // boolean
            self.generate_string_ends_with()
        )?;

        register_stdlib_function(
            codegen,
            "string_to_upper",
            &[WasmType::I32], // string
            Some(WasmType::I32), // new string
            self.generate_string_to_upper()
        )?;

        register_stdlib_function(
            codegen,
            "string_to_lower",
            &[WasmType::I32], // string
            Some(WasmType::I32), // new string
            self.generate_string_to_lower()
        )?;

        register_stdlib_function(
            codegen,
            "string_trim",
            &[WasmType::I32], // string
            Some(WasmType::I32), // new string
            self.generate_string_trim()
        )?;

        register_stdlib_function(
            codegen,
            "string_trim_start",
            &[WasmType::I32], // string
            Some(WasmType::I32), // new string
            self.generate_string_trim_start()
        )?;

        register_stdlib_function(
            codegen,
            "string_trim_end",
            &[WasmType::I32], // string
            Some(WasmType::I32), // new string
            self.generate_string_trim_end()
        )?;

        register_stdlib_function(
            codegen,
            "string_substring",
            &[WasmType::I32, WasmType::I32, WasmType::I32], // string, start, end
            Some(WasmType::I32), // new string
            self.generate_string_substring()
        )?;

        register_stdlib_function(
            codegen,
            "string_replace",
            &[WasmType::I32, WasmType::I32, WasmType::I32], // string, old, new
            Some(WasmType::I32), // new string
            self.generate_string_replace()
        )?;

        register_stdlib_function(
            codegen,
            "string_replace_all",
            &[WasmType::I32, WasmType::I32, WasmType::I32], // string, old, new
            Some(WasmType::I32), // new string
            self.generate_string_replace_all()
        )?;

        register_stdlib_function(
            codegen,
            "string_char_at",
            &[WasmType::I32, WasmType::I32], // string, index
            Some(WasmType::I32), // character as string
            self.generate_string_char_at()
        )?;

        register_stdlib_function(
            codegen,
            "string_char_code_at",
            &[WasmType::I32, WasmType::I32], // string, index
            Some(WasmType::I32), // character code
            self.generate_string_char_code_at()
        )?;

        register_stdlib_function(
            codegen,
            "string_is_empty",
            &[WasmType::I32], // string
            Some(WasmType::I32), // boolean
            self.generate_string_is_empty()
        )?;

        register_stdlib_function(
            codegen,
            "string_is_blank",
            &[WasmType::I32], // string
            Some(WasmType::I32), // boolean
            self.generate_string_is_blank()
        )?;

        register_stdlib_function(
            codegen,
            "string_pad_start",
            &[WasmType::I32, WasmType::I32, WasmType::I32], // string, length, padString
            Some(WasmType::I32), // new string
            self.generate_string_pad_start()
        )?;

        register_stdlib_function(
            codegen,
            "string_pad_end",
            &[WasmType::I32, WasmType::I32, WasmType::I32], // string, length, padString
            Some(WasmType::I32), // new string
            self.generate_string_pad_end()
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
        
        // Allocate new string (simplified approach)
        // For now, use a fixed memory location for result
        // In a real implementation, this would call a proper memory allocator
        instructions.push(Instruction::I32Const(1024)); // Fixed memory location
        instructions.push(Instruction::LocalSet(7)); // store result pointer in local 7
        
        // Store total length in header
        instructions.push(Instruction::LocalGet(7));
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
        
        // Check if i >= length1 (exit condition)
        instructions.push(Instruction::LocalGet(8));
        instructions.push(Instruction::LocalGet(3));
        instructions.push(Instruction::I32GeU);
        instructions.push(Instruction::BrIf(1)); // Break outer block if true
        
        // Loop body: copy character
        // First, load the character from string1
        instructions.push(Instruction::LocalGet(2)); // string1 ptr
        instructions.push(Instruction::LocalGet(8)); // index
        instructions.push(Instruction::I32Add); // string1 ptr + index
        instructions.push(Instruction::I32Load8U(MemArg {
            offset: 16, // Skip header
            align: 0,
            memory_index: 0,
        }));
        instructions.push(Instruction::LocalSet(9)); // Store char in local 9
        
        // Then, store the character to result
        instructions.push(Instruction::LocalGet(7)); // result ptr
        instructions.push(Instruction::LocalGet(8)); // index
        instructions.push(Instruction::I32Add); // result ptr + index
        instructions.push(Instruction::LocalGet(9)); // char value
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
        
        // Check if i >= length2 (exit condition)
        instructions.push(Instruction::LocalGet(8));
        instructions.push(Instruction::LocalGet(5));
        instructions.push(Instruction::I32GeU);
        instructions.push(Instruction::BrIf(1)); // Break outer block if true
        
        // Loop body: copy character
        // First, load the character from string2
        instructions.push(Instruction::LocalGet(4)); // string2 ptr
        instructions.push(Instruction::LocalGet(8)); // index
        instructions.push(Instruction::I32Add); // string2 ptr + index
        instructions.push(Instruction::I32Load8U(MemArg {
            offset: 16, // Skip header
            align: 0,
            memory_index: 0,
        }));
        instructions.push(Instruction::LocalSet(9)); // Store char in local 9
        
        // Then, store the character to result
        instructions.push(Instruction::LocalGet(7)); // result ptr
        instructions.push(Instruction::LocalGet(3)); // length1 (offset for string2)
        instructions.push(Instruction::LocalGet(8)); // index
        instructions.push(Instruction::I32Add); // length1 + index
        instructions.push(Instruction::I32Add); // result ptr + length1 + index
        instructions.push(Instruction::LocalGet(9)); // char value
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

    // NEW STRING FUNCTIONS

    fn generate_string_contains(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // Real implementation: contains(haystack, needle) -> boolean
        // haystack is at local 0, needle is at local 1
        
        // Get haystack length
        instructions.push(Instruction::LocalGet(0));
        instructions.push(Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }));
        instructions.push(Instruction::LocalSet(2)); // haystack_len
        
        // Get needle length  
        instructions.push(Instruction::LocalGet(1));
        instructions.push(Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }));
        instructions.push(Instruction::LocalSet(3)); // needle_len
        
        // If needle is empty, return true
        instructions.push(Instruction::LocalGet(3));
        instructions.push(Instruction::I32Eqz);
        instructions.push(Instruction::If(BlockType::Result(ValType::I32)));
        instructions.push(Instruction::I32Const(1)); // true
        instructions.push(Instruction::Return);
        instructions.push(Instruction::End);
        
        // If needle is longer than haystack, return false
        instructions.push(Instruction::LocalGet(3));
        instructions.push(Instruction::LocalGet(2));
        instructions.push(Instruction::I32GtU);
        instructions.push(Instruction::If(BlockType::Result(ValType::I32)));
        instructions.push(Instruction::I32Const(0)); // false
        instructions.push(Instruction::Return);
        instructions.push(Instruction::End);
        
        // Search for needle in haystack
        instructions.push(Instruction::I32Const(0));
        instructions.push(Instruction::LocalSet(4)); // i = 0
        
        // Loop through possible positions
        instructions.push(Instruction::Block(BlockType::Result(ValType::I32)));
        instructions.push(Instruction::Loop(BlockType::Empty));
        
        // Check if we've exceeded search range
        instructions.push(Instruction::LocalGet(4)); // i
        instructions.push(Instruction::LocalGet(2)); // haystack_len
        instructions.push(Instruction::LocalGet(3)); // needle_len
        instructions.push(Instruction::I32Sub);
        instructions.push(Instruction::I32GtU);
        instructions.push(Instruction::BrIf(1)); // Break if i > haystack_len - needle_len
        
        // Compare substring at position i with needle
        // For now, we'll use a simplified comparison
        // In a full implementation, this would do byte-by-byte comparison
        
        // Get first character of needle
        instructions.push(Instruction::LocalGet(1));
        instructions.push(Instruction::I32Load8U(MemArg { offset: 4, align: 0, memory_index: 0 }));
        instructions.push(Instruction::LocalSet(5)); // needle_first_char
        
        // Get character at position i in haystack
        instructions.push(Instruction::LocalGet(0));
        instructions.push(Instruction::LocalGet(4));
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::I32Load8U(MemArg { offset: 4, align: 0, memory_index: 0 }));
        
        // Compare first characters
        instructions.push(Instruction::LocalGet(5));
        instructions.push(Instruction::I32Eq);
        instructions.push(Instruction::If(BlockType::Empty));
        
        // Found potential match - for simplified implementation, return true
        // In a full implementation, we would compare all characters
        instructions.push(Instruction::I32Const(1)); // true
        instructions.push(Instruction::Br(2)); // Return from outer block
        
        instructions.push(Instruction::End);
        
        // Increment i and continue
        instructions.push(Instruction::LocalGet(4));
        instructions.push(Instruction::I32Const(1));
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::LocalSet(4));
        instructions.push(Instruction::Br(0)); // Continue loop
        
        instructions.push(Instruction::End);
        instructions.push(Instruction::I32Const(0)); // Not found
        instructions.push(Instruction::End);
        
        instructions
    }

    pub fn generate_string_index_of(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // string_index_of(string_ptr: i32, search_ptr: i32) -> i32
        // Returns index of first occurrence of search string, or -1 if not found
        
        // Get string length
        instructions.push(Instruction::LocalGet(0)); // string_ptr
        instructions.push(Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }));
        instructions.push(Instruction::LocalSet(2)); // string_len
        
        // Get search string length
        instructions.push(Instruction::LocalGet(1)); // search_ptr
        instructions.push(Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }));
        instructions.push(Instruction::LocalSet(3)); // search_len
        
        // If search string is empty, return 0
        instructions.push(Instruction::LocalGet(3)); // search_len
        instructions.push(Instruction::I32Eqz);
        instructions.push(Instruction::If(BlockType::Result(ValType::I32)));
        instructions.push(Instruction::I32Const(0));
        instructions.push(Instruction::Return);
        instructions.push(Instruction::End);
        
        // If search string is longer than string, return -1
        instructions.push(Instruction::LocalGet(3)); // search_len
        instructions.push(Instruction::LocalGet(2)); // string_len
        instructions.push(Instruction::I32GtU);
        instructions.push(Instruction::If(BlockType::Result(ValType::I32)));
        instructions.push(Instruction::I32Const(-1));
        instructions.push(Instruction::Return);
        instructions.push(Instruction::End);
        
        // Initialize loop counter
        instructions.push(Instruction::I32Const(0));
        instructions.push(Instruction::LocalSet(4)); // i = 0
        
        // Calculate max search position
        instructions.push(Instruction::LocalGet(2)); // string_len
        instructions.push(Instruction::LocalGet(3)); // search_len
        instructions.push(Instruction::I32Sub);
        instructions.push(Instruction::I32Const(1));
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::LocalSet(5)); // max_pos = string_len - search_len + 1
        
        // Main search loop
        instructions.push(Instruction::Loop(BlockType::Empty));
        
        // Check if we've reached the end
        instructions.push(Instruction::LocalGet(4)); // i
        instructions.push(Instruction::LocalGet(5)); // max_pos
        instructions.push(Instruction::I32GeU);
        instructions.push(Instruction::If(BlockType::Empty));
        instructions.push(Instruction::I32Const(-1)); // Not found
        instructions.push(Instruction::Return);
        instructions.push(Instruction::End);
        
        // Check if substring matches at position i
        instructions.push(Instruction::I32Const(0));
        instructions.push(Instruction::LocalSet(6)); // j = 0 (inner loop counter)
        
        // Inner loop to compare characters
        instructions.push(Instruction::Loop(BlockType::Empty));
        
        // Check if we've compared all characters
        instructions.push(Instruction::LocalGet(6)); // j
        instructions.push(Instruction::LocalGet(3)); // search_len
        instructions.push(Instruction::I32GeU);
        instructions.push(Instruction::If(BlockType::Empty));
        // Match found, return current position i
        instructions.push(Instruction::LocalGet(4)); // i
        instructions.push(Instruction::Return);
        instructions.push(Instruction::End);
        
        // Load character from string at position i + j
        instructions.push(Instruction::LocalGet(0)); // string_ptr
        instructions.push(Instruction::I32Const(16)); // Skip header
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::LocalGet(4)); // i
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::LocalGet(6)); // j
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::I32Load8U(MemArg { offset: 0, align: 0, memory_index: 0 }));
        
        // Load character from search string at position j
        instructions.push(Instruction::LocalGet(1)); // search_ptr
        instructions.push(Instruction::I32Const(16)); // Skip header
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::LocalGet(6)); // j
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::I32Load8U(MemArg { offset: 0, align: 0, memory_index: 0 }));
        
        // Compare characters
        instructions.push(Instruction::I32Ne);
        instructions.push(Instruction::If(BlockType::Empty));
        // Characters don't match, break inner loop
        instructions.push(Instruction::Br(1));
        instructions.push(Instruction::End);
        
        // Increment inner loop counter
        instructions.push(Instruction::LocalGet(6)); // j
        instructions.push(Instruction::I32Const(1));
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::LocalSet(6)); // j++
        
        // Continue inner loop
        instructions.push(Instruction::Br(0));
        instructions.push(Instruction::End); // End inner loop
        
        // Increment outer loop counter
        instructions.push(Instruction::LocalGet(4)); // i
        instructions.push(Instruction::I32Const(1));
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::LocalSet(4)); // i++
        
        // Continue outer loop
        instructions.push(Instruction::Br(0));
        instructions.push(Instruction::End); // End outer loop
        
        // Should never reach here, but return -1 as fallback
        instructions.push(Instruction::I32Const(-1));
        
        instructions
    }

    pub fn generate_string_last_index_of(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // string_last_index_of(string_ptr: i32, search_ptr: i32) -> i32
        // Returns index of last occurrence of search string, or -1 if not found
        
        // Get string length
        instructions.push(Instruction::LocalGet(0)); // string_ptr
        instructions.push(Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }));
        instructions.push(Instruction::LocalSet(2)); // string_len
        
        // Get search string length
        instructions.push(Instruction::LocalGet(1)); // search_ptr
        instructions.push(Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }));
        instructions.push(Instruction::LocalSet(3)); // search_len
        
        // If search string is empty, return string length
        instructions.push(Instruction::LocalGet(3)); // search_len
        instructions.push(Instruction::I32Eqz);
        instructions.push(Instruction::If(BlockType::Result(ValType::I32)));
        instructions.push(Instruction::LocalGet(2)); // string_len
        instructions.push(Instruction::Return);
        instructions.push(Instruction::End);
        
        // If search string is longer than string, return -1
        instructions.push(Instruction::LocalGet(3)); // search_len
        instructions.push(Instruction::LocalGet(2)); // string_len
        instructions.push(Instruction::I32GtU);
        instructions.push(Instruction::If(BlockType::Result(ValType::I32)));
        instructions.push(Instruction::I32Const(-1));
        instructions.push(Instruction::Return);
        instructions.push(Instruction::End);
        
        // Initialize last found position to -1
        instructions.push(Instruction::I32Const(-1));
        instructions.push(Instruction::LocalSet(4)); // last_found = -1
        
        // Initialize loop counter to start from the end
        instructions.push(Instruction::LocalGet(2)); // string_len
        instructions.push(Instruction::LocalGet(3)); // search_len
        instructions.push(Instruction::I32Sub);
        instructions.push(Instruction::LocalSet(5)); // i = string_len - search_len
        
        // Main search loop (searching backwards)
        instructions.push(Instruction::Loop(BlockType::Empty));
        
        // Check if we've reached the beginning
        instructions.push(Instruction::LocalGet(5)); // i
        instructions.push(Instruction::I32Const(0));
        instructions.push(Instruction::I32LtS);
        instructions.push(Instruction::If(BlockType::Empty));
        // Return the last found position
        instructions.push(Instruction::LocalGet(4)); // last_found
        instructions.push(Instruction::Return);
        instructions.push(Instruction::End);
        
        // Check if substring matches at position i
        instructions.push(Instruction::I32Const(0));
        instructions.push(Instruction::LocalSet(6)); // j = 0 (inner loop counter)
        instructions.push(Instruction::I32Const(1)); // match_found = true initially
        instructions.push(Instruction::LocalSet(7));
        
        // Inner loop to compare characters
        instructions.push(Instruction::Loop(BlockType::Empty));
        
        // Check if we've compared all characters
        instructions.push(Instruction::LocalGet(6)); // j
        instructions.push(Instruction::LocalGet(3)); // search_len
        instructions.push(Instruction::I32GeU);
        instructions.push(Instruction::If(BlockType::Empty));
        // Finished comparing, check if match was found
        instructions.push(Instruction::LocalGet(7)); // match_found
        instructions.push(Instruction::If(BlockType::Empty));
        // Match found, update last_found and continue searching
        instructions.push(Instruction::LocalGet(5)); // i
        instructions.push(Instruction::LocalSet(4)); // last_found = i
        instructions.push(Instruction::End);
        instructions.push(Instruction::Br(1)); // Break inner loop
        instructions.push(Instruction::End);
        
        // Load character from string at position i + j
        instructions.push(Instruction::LocalGet(0)); // string_ptr
        instructions.push(Instruction::I32Const(16)); // Skip header
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::LocalGet(5)); // i
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::LocalGet(6)); // j
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::I32Load8U(MemArg { offset: 0, align: 0, memory_index: 0 }));
        
        // Load character from search string at position j
        instructions.push(Instruction::LocalGet(1)); // search_ptr
        instructions.push(Instruction::I32Const(16)); // Skip header
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::LocalGet(6)); // j
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::I32Load8U(MemArg { offset: 0, align: 0, memory_index: 0 }));
        
        // Compare characters
        instructions.push(Instruction::I32Ne);
        instructions.push(Instruction::If(BlockType::Empty));
        // Characters don't match, set match_found to false and break inner loop
        instructions.push(Instruction::I32Const(0));
        instructions.push(Instruction::LocalSet(7)); // match_found = false
        instructions.push(Instruction::Br(1)); // Break inner loop
        instructions.push(Instruction::End);
        
        // Increment inner loop counter
        instructions.push(Instruction::LocalGet(6)); // j
        instructions.push(Instruction::I32Const(1));
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::LocalSet(6)); // j++
        
        // Continue inner loop
        instructions.push(Instruction::Br(0));
        instructions.push(Instruction::End); // End inner loop
        
        // Decrement outer loop counter (search backwards)
        instructions.push(Instruction::LocalGet(5)); // i
        instructions.push(Instruction::I32Const(1));
        instructions.push(Instruction::I32Sub);
        instructions.push(Instruction::LocalSet(5)); // i--
        
        // Continue outer loop
        instructions.push(Instruction::Br(0));
        instructions.push(Instruction::End); // End outer loop
        
        // Return the last found position
        instructions.push(Instruction::LocalGet(4)); // last_found
        
        instructions
    }

    pub fn generate_string_starts_with(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // string_starts_with(string_ptr: i32, prefix_ptr: i32) -> i32
        // Returns 1 if string starts with prefix, 0 otherwise
        
        // Get string length
        instructions.push(Instruction::LocalGet(0)); // string_ptr
        instructions.push(Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }));
        instructions.push(Instruction::LocalSet(2)); // string_len
        
        // Get prefix length
        instructions.push(Instruction::LocalGet(1)); // prefix_ptr
        instructions.push(Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }));
        instructions.push(Instruction::LocalSet(3)); // prefix_len
        
        // If prefix is empty, return true
        instructions.push(Instruction::LocalGet(3)); // prefix_len
        instructions.push(Instruction::I32Eqz);
        instructions.push(Instruction::If(BlockType::Result(ValType::I32)));
        instructions.push(Instruction::I32Const(1)); // true
        instructions.push(Instruction::Return);
        instructions.push(Instruction::End);
        
        // If prefix is longer than string, return false
        instructions.push(Instruction::LocalGet(3)); // prefix_len
        instructions.push(Instruction::LocalGet(2)); // string_len
        instructions.push(Instruction::I32GtU);
        instructions.push(Instruction::If(BlockType::Result(ValType::I32)));
        instructions.push(Instruction::I32Const(0)); // false
        instructions.push(Instruction::Return);
        instructions.push(Instruction::End);
        
        // Compare characters from the beginning
        instructions.push(Instruction::I32Const(0));
        instructions.push(Instruction::LocalSet(4)); // i = 0
        
        // Loop to compare characters
        instructions.push(Instruction::Loop(BlockType::Empty));
        
        // Check if we've compared all prefix characters
        instructions.push(Instruction::LocalGet(4)); // i
        instructions.push(Instruction::LocalGet(3)); // prefix_len
        instructions.push(Instruction::I32GeU);
        instructions.push(Instruction::If(BlockType::Empty));
        // All characters matched, return true
        instructions.push(Instruction::I32Const(1)); // true
        instructions.push(Instruction::Return);
        instructions.push(Instruction::End);
        
        // Load character from string at position i
        instructions.push(Instruction::LocalGet(0)); // string_ptr
        instructions.push(Instruction::I32Const(16)); // Skip header
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::LocalGet(4)); // i
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::I32Load8U(MemArg { offset: 0, align: 0, memory_index: 0 }));
        
        // Load character from prefix at position i
        instructions.push(Instruction::LocalGet(1)); // prefix_ptr
        instructions.push(Instruction::I32Const(16)); // Skip header
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::LocalGet(4)); // i
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::I32Load8U(MemArg { offset: 0, align: 0, memory_index: 0 }));
        
        // Compare characters
        instructions.push(Instruction::I32Ne);
        instructions.push(Instruction::If(BlockType::Empty));
        // Characters don't match, return false
        instructions.push(Instruction::I32Const(0)); // false
        instructions.push(Instruction::Return);
        instructions.push(Instruction::End);
        
        // Increment loop counter
        instructions.push(Instruction::LocalGet(4)); // i
        instructions.push(Instruction::I32Const(1));
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::LocalSet(4)); // i++
        
        // Continue loop
        instructions.push(Instruction::Br(0));
        instructions.push(Instruction::End); // End loop
        
        // Should never reach here, but return false as fallback
        instructions.push(Instruction::I32Const(0)); // false
        
        instructions
    }

    pub fn generate_string_ends_with(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // string_ends_with(string_ptr: i32, suffix_ptr: i32) -> i32
        // Returns 1 if string ends with suffix, 0 otherwise
        
        // Get string length
        instructions.push(Instruction::LocalGet(0)); // string_ptr
        instructions.push(Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }));
        instructions.push(Instruction::LocalSet(2)); // string_len
        
        // Get suffix length
        instructions.push(Instruction::LocalGet(1)); // suffix_ptr
        instructions.push(Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }));
        instructions.push(Instruction::LocalSet(3)); // suffix_len
        
        // If suffix is empty, return true
        instructions.push(Instruction::LocalGet(3)); // suffix_len
        instructions.push(Instruction::I32Eqz);
        instructions.push(Instruction::If(BlockType::Result(ValType::I32)));
        instructions.push(Instruction::I32Const(1)); // true
        instructions.push(Instruction::Return);
        instructions.push(Instruction::End);
        
        // If suffix is longer than string, return false
        instructions.push(Instruction::LocalGet(3)); // suffix_len
        instructions.push(Instruction::LocalGet(2)); // string_len
        instructions.push(Instruction::I32GtU);
        instructions.push(Instruction::If(BlockType::Result(ValType::I32)));
        instructions.push(Instruction::I32Const(0)); // false
        instructions.push(Instruction::Return);
        instructions.push(Instruction::End);
        
        // Calculate starting position in string (string_len - suffix_len)
        instructions.push(Instruction::LocalGet(2)); // string_len
        instructions.push(Instruction::LocalGet(3)); // suffix_len
        instructions.push(Instruction::I32Sub);
        instructions.push(Instruction::LocalSet(4)); // start_pos
        
        // Compare characters from the end
        instructions.push(Instruction::I32Const(0));
        instructions.push(Instruction::LocalSet(5)); // i = 0
        
        // Loop to compare characters
        instructions.push(Instruction::Loop(BlockType::Empty));
        
        // Check if we've compared all suffix characters
        instructions.push(Instruction::LocalGet(5)); // i
        instructions.push(Instruction::LocalGet(3)); // suffix_len
        instructions.push(Instruction::I32GeU);
        instructions.push(Instruction::If(BlockType::Empty));
        // All characters matched, return true
        instructions.push(Instruction::I32Const(1)); // true
        instructions.push(Instruction::Return);
        instructions.push(Instruction::End);
        
        // Load character from string at position start_pos + i
        instructions.push(Instruction::LocalGet(0)); // string_ptr
        instructions.push(Instruction::I32Const(16)); // Skip header
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::LocalGet(4)); // start_pos
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::LocalGet(5)); // i
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::I32Load8U(MemArg { offset: 0, align: 0, memory_index: 0 }));
        
        // Load character from suffix at position i
        instructions.push(Instruction::LocalGet(1)); // suffix_ptr
        instructions.push(Instruction::I32Const(16)); // Skip header
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::LocalGet(5)); // i
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::I32Load8U(MemArg { offset: 0, align: 0, memory_index: 0 }));
        
        // Compare characters
        instructions.push(Instruction::I32Ne);
        instructions.push(Instruction::If(BlockType::Empty));
        // Characters don't match, return false
        instructions.push(Instruction::I32Const(0)); // false
        instructions.push(Instruction::Return);
        instructions.push(Instruction::End);
        
        // Increment loop counter
        instructions.push(Instruction::LocalGet(5)); // i
        instructions.push(Instruction::I32Const(1));
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::LocalSet(5)); // i++
        
        // Continue loop
        instructions.push(Instruction::Br(0));
        instructions.push(Instruction::End); // End loop
        
        // Should never reach here, but return false as fallback
        instructions.push(Instruction::I32Const(0)); // false
        
        instructions
    }

    pub fn generate_string_to_upper(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // string_to_upper(string_ptr: i32) -> i32
        // Returns pointer to new string with all characters converted to uppercase
        
        // Get string length
        instructions.push(Instruction::LocalGet(0)); // string_ptr
        instructions.push(Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }));
        instructions.push(Instruction::LocalSet(1)); // string_len
        
        // Allocate memory for new string (16 bytes header + string_len)
        instructions.push(Instruction::I32Const(16));
        instructions.push(Instruction::LocalGet(1)); // string_len
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::Call(0)); // Call memory allocation function
        instructions.push(Instruction::LocalSet(2)); // new_string_ptr
        
        // Copy string header (length and other metadata)
        instructions.push(Instruction::LocalGet(2)); // new_string_ptr
        instructions.push(Instruction::LocalGet(1)); // string_len
        instructions.push(Instruction::I32Store(MemArg { offset: 0, align: 2, memory_index: 0 }));
        
        // Initialize loop counter
        instructions.push(Instruction::I32Const(0));
        instructions.push(Instruction::LocalSet(3)); // i = 0
        
        // Loop through each character
        instructions.push(Instruction::Loop(BlockType::Empty));
        
        // Check if we've processed all characters
        instructions.push(Instruction::LocalGet(3)); // i
        instructions.push(Instruction::LocalGet(1)); // string_len
        instructions.push(Instruction::I32GeU);
        instructions.push(Instruction::If(BlockType::Empty));
        // Finished, return new string pointer
        instructions.push(Instruction::LocalGet(2)); // new_string_ptr
        instructions.push(Instruction::Return);
        instructions.push(Instruction::End);
        
        // Load character from original string
        instructions.push(Instruction::LocalGet(0)); // string_ptr
        instructions.push(Instruction::I32Const(16)); // Skip header
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::LocalGet(3)); // i
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::I32Load8U(MemArg { offset: 0, align: 0, memory_index: 0 }));
        instructions.push(Instruction::LocalSet(4)); // char
        
        // Check if character is lowercase letter (a-z: 97-122)
        instructions.push(Instruction::LocalGet(4)); // char
        instructions.push(Instruction::I32Const(97)); // 'a'
        instructions.push(Instruction::I32GeU);
        instructions.push(Instruction::LocalGet(4)); // char
        instructions.push(Instruction::I32Const(122)); // 'z'
        instructions.push(Instruction::I32LeU);
        instructions.push(Instruction::I32And);
        instructions.push(Instruction::If(BlockType::Empty));
        // Convert to uppercase by subtracting 32
        instructions.push(Instruction::LocalGet(4)); // char
        instructions.push(Instruction::I32Const(32));
        instructions.push(Instruction::I32Sub);
        instructions.push(Instruction::LocalSet(4)); // char = char - 32
        instructions.push(Instruction::End);
        
        // Store converted character in new string
        instructions.push(Instruction::LocalGet(2)); // new_string_ptr
        instructions.push(Instruction::I32Const(16)); // Skip header
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::LocalGet(3)); // i
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::LocalGet(4)); // char
        instructions.push(Instruction::I32Store8(MemArg { offset: 0, align: 0, memory_index: 0 }));
        
        // Increment loop counter
        instructions.push(Instruction::LocalGet(3)); // i
        instructions.push(Instruction::I32Const(1));
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::LocalSet(3)); // i++
        
        // Continue loop
        instructions.push(Instruction::Br(0));
        instructions.push(Instruction::End); // End loop
        
        // Should never reach here, but return new string pointer as fallback
        instructions.push(Instruction::LocalGet(2)); // new_string_ptr
        
        instructions
    }

    pub fn generate_string_to_lower(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // string_to_lower(string_ptr: i32) -> i32
        // Returns pointer to new string with all characters converted to lowercase
        
        // Get string length
        instructions.push(Instruction::LocalGet(0)); // string_ptr
        instructions.push(Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }));
        instructions.push(Instruction::LocalSet(1)); // string_len
        
        // Allocate memory for new string (16 bytes header + string_len)
        instructions.push(Instruction::I32Const(16));
        instructions.push(Instruction::LocalGet(1)); // string_len
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::Call(0)); // Call memory allocation function
        instructions.push(Instruction::LocalSet(2)); // new_string_ptr
        
        // Copy string header (length and other metadata)
        instructions.push(Instruction::LocalGet(2)); // new_string_ptr
        instructions.push(Instruction::LocalGet(1)); // string_len
        instructions.push(Instruction::I32Store(MemArg { offset: 0, align: 2, memory_index: 0 }));
        
        // Initialize loop counter
        instructions.push(Instruction::I32Const(0));
        instructions.push(Instruction::LocalSet(3)); // i = 0
        
        // Loop through each character
        instructions.push(Instruction::Loop(BlockType::Empty));
        
        // Check if we've processed all characters
        instructions.push(Instruction::LocalGet(3)); // i
        instructions.push(Instruction::LocalGet(1)); // string_len
        instructions.push(Instruction::I32GeU);
        instructions.push(Instruction::If(BlockType::Empty));
        // Finished, return new string pointer
        instructions.push(Instruction::LocalGet(2)); // new_string_ptr
        instructions.push(Instruction::Return);
        instructions.push(Instruction::End);
        
        // Load character from original string
        instructions.push(Instruction::LocalGet(0)); // string_ptr
        instructions.push(Instruction::I32Const(16)); // Skip header
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::LocalGet(3)); // i
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::I32Load8U(MemArg { offset: 0, align: 0, memory_index: 0 }));
        instructions.push(Instruction::LocalSet(4)); // char
        
        // Check if character is uppercase letter (A-Z: 65-90)
        instructions.push(Instruction::LocalGet(4)); // char
        instructions.push(Instruction::I32Const(65)); // 'A'
        instructions.push(Instruction::I32GeU);
        instructions.push(Instruction::LocalGet(4)); // char
        instructions.push(Instruction::I32Const(90)); // 'Z'
        instructions.push(Instruction::I32LeU);
        instructions.push(Instruction::I32And);
        instructions.push(Instruction::If(BlockType::Empty));
        // Convert to lowercase by adding 32
        instructions.push(Instruction::LocalGet(4)); // char
        instructions.push(Instruction::I32Const(32));
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::LocalSet(4)); // char = char + 32
        instructions.push(Instruction::End);
        
        // Store converted character in new string
        instructions.push(Instruction::LocalGet(2)); // new_string_ptr
        instructions.push(Instruction::I32Const(16)); // Skip header
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::LocalGet(3)); // i
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::LocalGet(4)); // char
        instructions.push(Instruction::I32Store8(MemArg { offset: 0, align: 0, memory_index: 0 }));
        
        // Increment loop counter
        instructions.push(Instruction::LocalGet(3)); // i
        instructions.push(Instruction::I32Const(1));
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::LocalSet(3)); // i++
        
        // Continue loop
        instructions.push(Instruction::Br(0));
        instructions.push(Instruction::End); // End loop
        
        // Should never reach here, but return new string pointer as fallback
        instructions.push(Instruction::LocalGet(2)); // new_string_ptr
        
        instructions
    }

    pub fn generate_string_trim(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // Placeholder implementation - return original string
        instructions.push(Instruction::LocalGet(0));
        
        instructions
    }

    pub fn generate_string_trim_start(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // Placeholder implementation - return original string
        instructions.push(Instruction::LocalGet(0));
        
        instructions
    }

    pub fn generate_string_trim_end(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // Placeholder implementation - return original string
        instructions.push(Instruction::LocalGet(0));
        
        instructions
    }

    pub fn generate_string_substring(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // Get string length
        instructions.push(Instruction::LocalGet(0));
        instructions.push(Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }));
        instructions.push(Instruction::LocalSet(3)); // string_len
        
        // Validate start index
        instructions.push(Instruction::LocalGet(1)); // start
        instructions.push(Instruction::I32Const(0));
        instructions.push(Instruction::I32LtS);
        instructions.push(Instruction::If(BlockType::Empty));
        instructions.push(Instruction::I32Const(0));
        instructions.push(Instruction::LocalSet(1)); // start = 0
        instructions.push(Instruction::End);
        
        // Handle end index (-1 means use string length)
        instructions.push(Instruction::LocalGet(2)); // end
        instructions.push(Instruction::I32Const(-1));
        instructions.push(Instruction::I32Eq);
        instructions.push(Instruction::If(BlockType::Empty));
        instructions.push(Instruction::LocalGet(3));
        instructions.push(Instruction::LocalSet(2)); // end = string_len
        instructions.push(Instruction::End);
        
        // Calculate substring length
        instructions.push(Instruction::LocalGet(2)); // end
        instructions.push(Instruction::LocalGet(1)); // start
        instructions.push(Instruction::I32Sub);
        instructions.push(Instruction::LocalSet(4)); // sub_len
        
        // Allocate new string
        instructions.push(Instruction::LocalGet(4));
        instructions.push(Instruction::I32Const(STRING_TYPE_ID as i32));
        instructions.push(Instruction::Call(0)); // Call memory.allocate
        instructions.push(Instruction::LocalTee(5)); // result_ptr
        
        // Store length in header
        instructions.push(Instruction::LocalGet(4));
        instructions.push(Instruction::I32Store(MemArg { offset: 0, align: 2, memory_index: 0 }));
        
        // Return result pointer (simplified implementation)
        instructions.push(Instruction::LocalGet(5));
        
        instructions
    }

    pub fn generate_string_replace(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // Placeholder implementation - return original string
        instructions.push(Instruction::LocalGet(0));
        
        instructions
    }

    fn generate_string_replace_all(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // Placeholder implementation - return original string
        instructions.push(Instruction::LocalGet(0));
        
        instructions
    }

    fn generate_string_char_at(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // Get character at index and create single-character string
        instructions.push(Instruction::LocalGet(0)); // string
        instructions.push(Instruction::LocalGet(1)); // index
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::I32Load8U(MemArg { offset: 16, align: 0, memory_index: 0 }));
        
        // Allocate single-character string
        instructions.push(Instruction::I32Const(1));
        instructions.push(Instruction::I32Const(STRING_TYPE_ID as i32));
        instructions.push(Instruction::Call(0)); // Call memory.allocate
        instructions.push(Instruction::LocalTee(2)); // result_ptr
        
        // Store length (1) in header
        instructions.push(Instruction::I32Const(1));
        instructions.push(Instruction::I32Store(MemArg { offset: 0, align: 2, memory_index: 0 }));
        
        // Store character
        instructions.push(Instruction::LocalGet(2));
        instructions.push(Instruction::I32Store8(MemArg { offset: 16, align: 0, memory_index: 0 }));
        
        instructions.push(Instruction::LocalGet(2)); // Return result
        
        instructions
    }

    fn generate_string_char_code_at(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // Get character at index and return its code
        instructions.push(Instruction::LocalGet(0)); // string
        instructions.push(Instruction::LocalGet(1)); // index
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::I32Load8U(MemArg { offset: 16, align: 0, memory_index: 0 }));
        
        instructions
    }

    fn generate_string_is_empty(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // Check if length == 0
        instructions.push(Instruction::LocalGet(0));
        instructions.push(Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }));
        instructions.push(Instruction::I32Eqz);
        
        instructions
    }

    fn generate_string_is_blank(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // Placeholder: check if empty or all whitespace
        instructions.push(Instruction::LocalGet(0));
        instructions.push(Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }));
        instructions.push(Instruction::I32Eqz);
        
        instructions
    }

    pub fn generate_string_pad_start(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // Placeholder implementation - return original string
        instructions.push(Instruction::LocalGet(0));
        
        instructions
    }

    fn generate_string_pad_end(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // Placeholder implementation - return original string
        instructions.push(Instruction::LocalGet(0));
        
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
        let wasm_bytes = codegen.generate_test_module().unwrap();
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
        let wasm_bytes = codegen.generate_test_module().unwrap();
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