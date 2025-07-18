use crate::codegen::CodeGenerator;
use crate::types::WasmType;
use crate::error::CompilerError;
use wasm_encoder::{Instruction, MemArg};
use crate::stdlib::register_stdlib_function;

/// String class implementation for Clean Language
/// Provides comprehensive text manipulation capabilities as static methods
pub struct StringClass;

impl StringClass {
    pub fn new() -> Self {
        Self
    }

    /// Register all String class methods as static functions
    pub fn register_functions(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // Basic operations
        self.register_basic_operations(codegen)?;
        
        // Case operations
        self.register_case_operations(codegen)?;
        
        // Search and validation operations
        self.register_search_operations(codegen)?;
        
        // Text cleaning and formatting
        self.register_formatting_operations(codegen)?;
        
        // Advanced text manipulation
        self.register_advanced_operations(codegen)?;
        
        // Character operations
        self.register_character_operations(codegen)?;
        
        // Validation helpers
        self.register_validation_operations(codegen)?;
        
        // Padding operations
        self.register_padding_operations(codegen)?;
        
        Ok(())
    }
    
    fn register_basic_operations(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // String.length(string text) -> integer
        register_stdlib_function(
            codegen,
            "string.length",
            &[WasmType::I32],
            Some(WasmType::I32),
            vec![
                // Get string pointer
                Instruction::LocalGet(0),
                // Load string length (first 4 bytes)
                Instruction::I32Load(MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                }),
            ]
        )?;
        
        // String.concat(string a, string b) -> string
        register_stdlib_function(
            codegen,
            "string.concat",
            &[WasmType::I32, WasmType::I32],
            Some(WasmType::I32),
            self.generate_concat()
        )?;
        
        // String.substring(string text, integer start, integer end) -> string
        register_stdlib_function(
            codegen,
            "string.substring",
            &[WasmType::I32, WasmType::I32, WasmType::I32],
            Some(WasmType::I32),
            self.generate_substring()
        )?;
        
        Ok(())
    }
    
    fn register_case_operations(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // String.toUpperCase(string text) -> string
        register_stdlib_function(
            codegen,
            "string.toUpperCase",
            &[WasmType::I32],
            Some(WasmType::I32),
            self.generate_to_upper()
        )?;
        
        // String.toLowerCase(string text) -> string
        register_stdlib_function(
            codegen,
            "string.toLowerCase",
            &[WasmType::I32],
            Some(WasmType::I32),
            self.generate_to_lower()
        )?;
        
        Ok(())
    }
    
    fn register_search_operations(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // String.contains(string text, string search) -> boolean
        register_stdlib_function(
            codegen,
            "string.contains",
            &[WasmType::I32, WasmType::I32],
            Some(WasmType::I32),
            self.generate_contains()
        )?;
        
        // String.indexOf(string text, string search) -> integer
        register_stdlib_function(
            codegen,
            "string.indexOf",
            &[WasmType::I32, WasmType::I32],
            Some(WasmType::I32),
            self.generate_index_of()
        )?;
        
        // String.lastIndexOf(string text, string search) -> integer
        register_stdlib_function(
            codegen,
            "string.lastIndexOf",
            &[WasmType::I32, WasmType::I32],
            Some(WasmType::I32),
            self.generate_last_index_of()
        )?;
        
        // String.startsWith(string text, string prefix) -> boolean
        register_stdlib_function(
            codegen,
            "string.startsWith",
            &[WasmType::I32, WasmType::I32],
            Some(WasmType::I32),
            self.generate_starts_with()
        )?;
        
        // String.endsWith(string text, string suffix) -> boolean
        register_stdlib_function(
            codegen,
            "string.endsWith",
            &[WasmType::I32, WasmType::I32],
            Some(WasmType::I32),
            self.generate_ends_with()
        )?;
        
        Ok(())
    }
    
    fn register_formatting_operations(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // String.trim(string text) -> string
        register_stdlib_function(
            codegen,
            "string.trim",
            &[WasmType::I32],
            Some(WasmType::I32),
            self.generate_trim()
        )?;
        
        // String.trimStart(string text) -> string
        register_stdlib_function(
            codegen,
            "string.trimStart",
            &[WasmType::I32],
            Some(WasmType::I32),
            self.generate_trim_start()
        )?;
        
        // String.trimEnd(string text) -> string
        register_stdlib_function(
            codegen,
            "string.trimEnd",
            &[WasmType::I32],
            Some(WasmType::I32),
            self.generate_trim_end()
        )?;
        
        Ok(())
    }
    
    fn register_advanced_operations(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // String.replace(string text, string oldValue, string newValue) -> string
        register_stdlib_function(
            codegen,
            "string.replace",
            &[WasmType::I32, WasmType::I32, WasmType::I32],
            Some(WasmType::I32),
            self.generate_replace()
        )?;
        
        // String.replaceAll(string text, string oldValue, string newValue) -> string
        register_stdlib_function(
            codegen,
            "string.replaceAll",
            &[WasmType::I32, WasmType::I32, WasmType::I32],
            Some(WasmType::I32),
            self.generate_replace_all()
        )?;
        
        // String.split(string text, string delimiter) -> array<string>
        register_stdlib_function(
            codegen,
            "string.split",
            &[WasmType::I32, WasmType::I32],
            Some(WasmType::I32),
            self.generate_split()
        )?;
        
        // String.join(array<string> parts, string separator) -> string
        register_stdlib_function(
            codegen,
            "string.join",
            &[WasmType::I32, WasmType::I32],
            Some(WasmType::I32),
            self.generate_join()
        )?;
        
        Ok(())
    }
    
    fn register_character_operations(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // String.charAt(string text, integer index) -> string
        register_stdlib_function(
            codegen,
            "string.charAt",
            &[WasmType::I32, WasmType::I32],
            Some(WasmType::I32),
            self.generate_char_at()
        )?;
        
        // String.charCodeAt(string text, integer index) -> integer
        register_stdlib_function(
            codegen,
            "string.charCodeAt",
            &[WasmType::I32, WasmType::I32],
            Some(WasmType::I32),
            self.generate_char_code_at()
        )?;
        
        Ok(())
    }
    
    fn register_validation_operations(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // String.isEmpty(string text) -> boolean
        register_stdlib_function(
            codegen,
            "string.isEmpty",
            &[WasmType::I32],
            Some(WasmType::I32),
            vec![
                // Get string pointer
                Instruction::LocalGet(0),
                // Load string length
                Instruction::I32Load(MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                }),
                // Check if length == 0
                Instruction::I32Const(0),
                Instruction::I32Eq,
            ]
        )?;
        
        // String.isBlank(string text) -> boolean
        register_stdlib_function(
            codegen,
            "string.isBlank",
            &[WasmType::I32],
            Some(WasmType::I32),
            self.generate_is_blank()
        )?;
        
        Ok(())
    }
    
    fn register_padding_operations(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // String.padStart(string text, integer length, string padString) -> string
        register_stdlib_function(
            codegen,
            "string.padStart",
            &[WasmType::I32, WasmType::I32, WasmType::I32],
            Some(WasmType::I32),
            self.generate_pad_start()
        )?;
        
        // String.padEnd(string text, integer length, string padString) -> string
        register_stdlib_function(
            codegen,
            "string.padEnd",
            &[WasmType::I32, WasmType::I32, WasmType::I32],
            Some(WasmType::I32),
            self.generate_pad_end()
        )?;
        
        Ok(())
    }

    // Implementation methods for complex string operations

    fn generate_concat(&self) -> Vec<Instruction> {
        vec![
            // Full string concatenation implementation
            // String structure: [length:i32][data...]
            
            // Get length of first string
            Instruction::LocalGet(0),  // str1 ptr
            Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }), // str1 length
            Instruction::LocalSet(2), // save str1 length
            
            // Get length of second string
            Instruction::LocalGet(1),  // str2 ptr
            Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }), // str2 length
            Instruction::LocalSet(3), // save str2 length
            
            // Calculate total length
            Instruction::LocalGet(2), // str1 length
            Instruction::LocalGet(3), // str2 length
            Instruction::I32Add,      // total length
            Instruction::LocalSet(4), // save total length
            
            // Allocate memory for new string (length + data)
            Instruction::LocalGet(4), // total length
            Instruction::I32Const(4), // add 4 bytes for length field
            Instruction::I32Add,      // total allocation size
            Instruction::Call(0),     // call memory allocator (assuming it's function 0)
            Instruction::LocalSet(5), // save new string pointer
            
            // Store length in new string
            Instruction::LocalGet(5), // new string ptr
            Instruction::LocalGet(4), // total length
            Instruction::I32Store(MemArg { offset: 0, align: 2, memory_index: 0 }),
            
            // Copy first string data
            Instruction::LocalGet(5), // new string ptr
            Instruction::I32Const(4), // offset past length field
            Instruction::I32Add,      // new string data ptr
            Instruction::LocalGet(0), // str1 ptr
            Instruction::I32Const(4), // offset past length field
            Instruction::I32Add,      // str1 data ptr
            Instruction::LocalGet(2), // str1 length
            Instruction::MemoryCopy { src_mem: 0, dst_mem: 0 },  // copy str1 data
            
            // Copy second string data
            Instruction::LocalGet(5), // new string ptr
            Instruction::I32Const(4), // offset past length field
            Instruction::I32Add,      // new string data ptr
            Instruction::LocalGet(2), // str1 length (offset for second string)
            Instruction::I32Add,      // position after first string
            Instruction::LocalGet(1), // str2 ptr
            Instruction::I32Const(4), // offset past length field
            Instruction::I32Add,      // str2 data ptr
            Instruction::LocalGet(3), // str2 length
            Instruction::MemoryCopy { src_mem: 0, dst_mem: 0 },  // copy str2 data
            
            // Return new string pointer
            Instruction::LocalGet(5),
        ]
    }

    fn generate_substring(&self) -> Vec<Instruction> {
        vec![
            // Full substring implementation
            // Parameters: string ptr, start index, end index
            
            // Get original string length
            Instruction::LocalGet(0), // string ptr
            Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }), // length
            Instruction::LocalSet(3), // save original length
            
            // Validate and calculate substring length
            Instruction::LocalGet(2), // end index
            Instruction::LocalGet(1), // start index
            Instruction::I32Sub,      // end - start = substring length
            Instruction::LocalSet(4), // save substring length
            
            // Bounds check: if substring length <= 0, return empty string
            Instruction::LocalGet(4), // substring length
            Instruction::I32Const(0),
            Instruction::I32LeS,      // check if length <= 0
            Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                // Return empty string (allocate 4 bytes with length 0)
                Instruction::I32Const(4),
                Instruction::Call(0),     // allocate memory
                Instruction::LocalTee(5), // save and keep on stack
                Instruction::I32Const(0), // length = 0
                Instruction::I32Store(MemArg { offset: 0, align: 2, memory_index: 0 }),
                Instruction::LocalGet(5), // return empty string ptr
            Instruction::Else,
                // Allocate memory for substring
                Instruction::LocalGet(4), // substring length
                Instruction::I32Const(4), // add 4 bytes for length field
                Instruction::I32Add,
                Instruction::Call(0),     // allocate memory
                Instruction::LocalSet(5), // save new string ptr
                
                // Store length in new string
                Instruction::LocalGet(5), // new string ptr
                Instruction::LocalGet(4), // substring length
                Instruction::I32Store(MemArg { offset: 0, align: 2, memory_index: 0 }),
                
                // Copy substring data
                Instruction::LocalGet(5), // new string ptr
                Instruction::I32Const(4), // offset past length field
                Instruction::I32Add,      // new string data ptr
                Instruction::LocalGet(0), // original string ptr
                Instruction::I32Const(4), // offset past length field
                Instruction::I32Add,      // original string data ptr
                Instruction::LocalGet(1), // start index
                Instruction::I32Add,      // offset to start position
                Instruction::LocalGet(4), // substring length
                Instruction::MemoryCopy { src_mem: 0, dst_mem: 0 },  // copy substring data
                
                // Return new string pointer
                Instruction::LocalGet(5),
            Instruction::End,
        ]
    }

    fn generate_to_upper(&self) -> Vec<Instruction> {
        vec![
            // Full case conversion to uppercase
            // Get string length
            Instruction::LocalGet(0), // string ptr
            Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }), // length
            Instruction::LocalSet(1), // save length
            
            // Allocate new string
            Instruction::LocalGet(1), // length
            Instruction::I32Const(4), // add 4 bytes for length field
            Instruction::I32Add,
            Instruction::Call(0),     // allocate memory
            Instruction::LocalSet(2), // save new string ptr
            
            // Store length in new string
            Instruction::LocalGet(2), // new string ptr
            Instruction::LocalGet(1), // length
            Instruction::I32Store(MemArg { offset: 0, align: 2, memory_index: 0 }),
            
            // Initialize loop counter
            Instruction::I32Const(0),
            Instruction::LocalSet(3), // i = 0
            
            // Loop through each character
            Instruction::Loop(wasm_encoder::BlockType::Empty),
                // Check if done
                Instruction::LocalGet(3), // i
                Instruction::LocalGet(1), // length
                Instruction::I32GeU,     // i >= length
                Instruction::BrIf(1),    // exit loop if done
                
                // Load character
                Instruction::LocalGet(0), // original string ptr
                Instruction::I32Const(4), // offset past length
                Instruction::I32Add,
                Instruction::LocalGet(3), // i
                Instruction::I32Add,      // character address
                Instruction::I32Load8U(MemArg { offset: 0, align: 0, memory_index: 0 }),
                Instruction::LocalSet(4), // save character
                
                // Convert to uppercase if lowercase (a-z: 97-122 -> A-Z: 65-90)
                Instruction::LocalGet(4), // character
                Instruction::I32Const(97), // 'a'
                Instruction::I32GeU,      // char >= 'a'
                Instruction::LocalGet(4), // character
                Instruction::I32Const(122), // 'z'
                Instruction::I32LeU,      // char <= 'z'
                Instruction::I32And,      // is lowercase
                Instruction::If(wasm_encoder::BlockType::Empty),
                    Instruction::LocalGet(4), // character
                    Instruction::I32Const(32), // difference between 'a' and 'A'
                    Instruction::I32Sub,      // convert to uppercase
                    Instruction::LocalSet(4), // save converted character
                Instruction::End,
                
                // Store character in new string
                Instruction::LocalGet(2), // new string ptr
                Instruction::I32Const(4), // offset past length
                Instruction::I32Add,
                Instruction::LocalGet(3), // i
                Instruction::I32Add,      // character address
                Instruction::LocalGet(4), // character
                Instruction::I32Store8(MemArg { offset: 0, align: 0, memory_index: 0 }),
                
                // Increment counter
                Instruction::LocalGet(3), // i
                Instruction::I32Const(1),
                Instruction::I32Add,      // i + 1
                Instruction::LocalSet(3), // i = i + 1
                Instruction::Br(0),       // continue loop
            Instruction::End,
            
            // Return new string
            Instruction::LocalGet(2),
        ]
    }

    fn generate_to_lower(&self) -> Vec<Instruction> {
        vec![
            // Full case conversion to lowercase
            // Get string length
            Instruction::LocalGet(0), // string ptr
            Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }), // length
            Instruction::LocalSet(1), // save length
            
            // Allocate new string
            Instruction::LocalGet(1), // length
            Instruction::I32Const(4), // add 4 bytes for length field
            Instruction::I32Add,
            Instruction::Call(0),     // allocate memory
            Instruction::LocalSet(2), // save new string ptr
            
            // Store length in new string
            Instruction::LocalGet(2), // new string ptr
            Instruction::LocalGet(1), // length
            Instruction::I32Store(MemArg { offset: 0, align: 2, memory_index: 0 }),
            
            // Initialize loop counter
            Instruction::I32Const(0),
            Instruction::LocalSet(3), // i = 0
            
            // Loop through each character
            Instruction::Loop(wasm_encoder::BlockType::Empty),
                // Check if done
                Instruction::LocalGet(3), // i
                Instruction::LocalGet(1), // length
                Instruction::I32GeU,     // i >= length
                Instruction::BrIf(1),    // exit loop if done
                
                // Load character
                Instruction::LocalGet(0), // original string ptr
                Instruction::I32Const(4), // offset past length
                Instruction::I32Add,
                Instruction::LocalGet(3), // i
                Instruction::I32Add,      // character address
                Instruction::I32Load8U(MemArg { offset: 0, align: 0, memory_index: 0 }),
                Instruction::LocalSet(4), // save character
                
                // Convert to lowercase if uppercase (A-Z: 65-90 -> a-z: 97-122)
                Instruction::LocalGet(4), // character
                Instruction::I32Const(65), // 'A'
                Instruction::I32GeU,      // char >= 'A'
                Instruction::LocalGet(4), // character
                Instruction::I32Const(90), // 'Z'
                Instruction::I32LeU,      // char <= 'Z'
                Instruction::I32And,      // is uppercase
                Instruction::If(wasm_encoder::BlockType::Empty),
                    Instruction::LocalGet(4), // character
                    Instruction::I32Const(32), // difference between 'A' and 'a'
                    Instruction::I32Add,      // convert to lowercase
                    Instruction::LocalSet(4), // save converted character
                Instruction::End,
                
                // Store character in new string
                Instruction::LocalGet(2), // new string ptr
                Instruction::I32Const(4), // offset past length
                Instruction::I32Add,
                Instruction::LocalGet(3), // i
                Instruction::I32Add,      // character address
                Instruction::LocalGet(4), // character
                Instruction::I32Store8(MemArg { offset: 0, align: 0, memory_index: 0 }),
                
                // Increment counter
                Instruction::LocalGet(3), // i
                Instruction::I32Const(1),
                Instruction::I32Add,      // i + 1
                Instruction::LocalSet(3), // i = i + 1
                Instruction::Br(0),       // continue loop
            Instruction::End,
            
            // Return new string
            Instruction::LocalGet(2),
        ]
    }

    fn generate_contains(&self) -> Vec<Instruction> {
        vec![
            // Full contains implementation using substring search
            // Parameters: haystack string, needle string
            
            // Get haystack length
            Instruction::LocalGet(0), // haystack ptr
            Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }),
            Instruction::LocalSet(2), // save haystack length
            
            // Get needle length
            Instruction::LocalGet(1), // needle ptr
            Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }),
            Instruction::LocalSet(3), // save needle length
            
            // If needle is empty, return true
            Instruction::LocalGet(3), // needle length
            Instruction::I32Const(0),
            Instruction::I32Eq,       // needle length == 0
            Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                Instruction::I32Const(1), // return true
            Instruction::Else,
                // If needle is longer than haystack, return false
                Instruction::LocalGet(3), // needle length
                Instruction::LocalGet(2), // haystack length
                Instruction::I32GtU,      // needle length > haystack length
                Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                    Instruction::I32Const(0), // return false
                Instruction::Else,
                    // Search for needle in haystack
                    Instruction::I32Const(0),
                    Instruction::LocalSet(4), // i = 0 (search position)
                    
                    // Calculate max search position
                    Instruction::LocalGet(2), // haystack length
                    Instruction::LocalGet(3), // needle length
                    Instruction::I32Sub,      // haystack length - needle length
                    Instruction::I32Const(1),
                    Instruction::I32Add,      // max positions to check
                    Instruction::LocalSet(5), // save max search positions
                    
                    // Search loop
                    Instruction::Loop(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                        // Check if we've exceeded search range
                        Instruction::LocalGet(4), // i
                        Instruction::LocalGet(5), // max search positions
                        Instruction::I32GeU,     // i >= max
                        Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                            Instruction::I32Const(0), // return false - not found
                        Instruction::Else,
                            // Compare substring at position i
                            Instruction::LocalGet(0), // haystack ptr
                            Instruction::I32Const(4), // offset past length
                            Instruction::I32Add,
                            Instruction::LocalGet(4), // i
                            Instruction::I32Add,      // haystack data + i
                            Instruction::LocalGet(1), // needle ptr
                            Instruction::I32Const(4), // offset past length
                            Instruction::I32Add,      // needle data
                            Instruction::LocalGet(3), // needle length
                            Instruction::Call(1),     // call memory compare function (assuming function 1)
                            
                            // If match found, return true
                            Instruction::I32Const(0),
                            Instruction::I32Eq,       // compare result == 0 (equal)
                            Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                                Instruction::I32Const(1), // return true
                            Instruction::Else,
                                // Increment search position and continue
                                Instruction::LocalGet(4), // i
                                Instruction::I32Const(1),
                                Instruction::I32Add,      // i + 1
                                Instruction::LocalSet(4), // i = i + 1
                                Instruction::Br(1),       // continue loop
                            Instruction::End,
                        Instruction::End,
                    Instruction::End,
                Instruction::End,
            Instruction::End,
        ]
    }

    fn generate_index_of(&self) -> Vec<Instruction> {
        vec![
            // Full indexOf implementation - returns position or -1 if not found
            // Parameters: haystack string, needle string
            
            // Get haystack length
            Instruction::LocalGet(0), // haystack ptr
            Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }),
            Instruction::LocalSet(2), // save haystack length
            
            // Get needle length
            Instruction::LocalGet(1), // needle ptr
            Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }),
            Instruction::LocalSet(3), // save needle length
            
            // If needle is empty, return 0
            Instruction::LocalGet(3), // needle length
            Instruction::I32Const(0),
            Instruction::I32Eq,       // needle length == 0
            Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                Instruction::I32Const(0), // return 0 (empty string found at start)
            Instruction::Else,
                // If needle is longer than haystack, return -1
                Instruction::LocalGet(3), // needle length
                Instruction::LocalGet(2), // haystack length
                Instruction::I32GtU,      // needle length > haystack length
                Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                    Instruction::I32Const(-1), // return -1
                Instruction::Else,
                    // Search for needle in haystack
                    Instruction::I32Const(0),
                    Instruction::LocalSet(4), // i = 0 (search position)
                    
                    // Calculate max search position
                    Instruction::LocalGet(2), // haystack length
                    Instruction::LocalGet(3), // needle length
                    Instruction::I32Sub,      // haystack length - needle length
                    Instruction::I32Const(1),
                    Instruction::I32Add,      // max positions to check
                    Instruction::LocalSet(5), // save max search positions
                    
                    // Search loop
                    Instruction::Loop(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                        // Check if we've exceeded search range
                        Instruction::LocalGet(4), // i
                        Instruction::LocalGet(5), // max search positions
                        Instruction::I32GeU,     // i >= max
                        Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                            Instruction::I32Const(-1), // return -1 - not found
                        Instruction::Else,
                            // Compare substring at position i
                            Instruction::LocalGet(0), // haystack ptr
                            Instruction::I32Const(4), // offset past length
                            Instruction::I32Add,
                            Instruction::LocalGet(4), // i
                            Instruction::I32Add,      // haystack data + i
                            Instruction::LocalGet(1), // needle ptr
                            Instruction::I32Const(4), // offset past length
                            Instruction::I32Add,      // needle data
                            Instruction::LocalGet(3), // needle length
                            Instruction::Call(1),     // call memory compare function
                            
                            // If match found, return position
                            Instruction::I32Const(0),
                            Instruction::I32Eq,       // compare result == 0 (equal)
                            Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                                Instruction::LocalGet(4), // return current position i
                            Instruction::Else,
                                // Increment search position and continue
                                Instruction::LocalGet(4), // i
                                Instruction::I32Const(1),
                                Instruction::I32Add,      // i + 1
                                Instruction::LocalSet(4), // i = i + 1
                                Instruction::Br(1),       // continue loop
                            Instruction::End,
                        Instruction::End,
                    Instruction::End,
                Instruction::End,
            Instruction::End,
        ]
    }

    fn generate_last_index_of(&self) -> Vec<Instruction> {
        vec![
            // Full lastIndexOf implementation - returns last position or -1 if not found
            // Parameters: haystack string, needle string
            
            // Get haystack length
            Instruction::LocalGet(0), // haystack ptr
            Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }),
            Instruction::LocalSet(2), // save haystack length
            
            // Get needle length
            Instruction::LocalGet(1), // needle ptr
            Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }),
            Instruction::LocalSet(3), // save needle length
            
            // If needle is empty, return haystack length
            Instruction::LocalGet(3), // needle length
            Instruction::I32Const(0),
            Instruction::I32Eq,       // needle length == 0
            Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                Instruction::LocalGet(2), // return haystack length
            Instruction::Else,
                // If needle is longer than haystack, return -1
                Instruction::LocalGet(3), // needle length
                Instruction::LocalGet(2), // haystack length
                Instruction::I32GtU,      // needle length > haystack length
                Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                    Instruction::I32Const(-1), // return -1
                Instruction::Else,
                    // Start from the end and search backwards
                    Instruction::LocalGet(2), // haystack length
                    Instruction::LocalGet(3), // needle length
                    Instruction::I32Sub,      // haystack length - needle length
                    Instruction::LocalSet(4), // i = last possible start position
                    
                    // Initialize result to -1 (not found)
                    Instruction::I32Const(-1),
                    Instruction::LocalSet(5), // last_found = -1
                    
                    // Search loop (backwards)
                    Instruction::Loop(wasm_encoder::BlockType::Empty),
                        // Check if we've gone past the beginning
                        Instruction::LocalGet(4), // i
                        Instruction::I32Const(0),
                        Instruction::I32LtS,      // i < 0
                        Instruction::BrIf(1),     // exit loop if i < 0
                        
                        // Compare substring at position i
                        Instruction::LocalGet(0), // haystack ptr
                        Instruction::I32Const(4), // offset past length
                        Instruction::I32Add,
                        Instruction::LocalGet(4), // i
                        Instruction::I32Add,      // haystack data + i
                        Instruction::LocalGet(1), // needle ptr
                        Instruction::I32Const(4), // offset past length
                        Instruction::I32Add,      // needle data
                        Instruction::LocalGet(3), // needle length
                        Instruction::Call(1),     // call memory compare function
                        
                        // If match found, update last_found
                        Instruction::I32Const(0),
                        Instruction::I32Eq,       // compare result == 0 (equal)
                        Instruction::If(wasm_encoder::BlockType::Empty),
                            Instruction::LocalGet(4), // i
                            Instruction::LocalSet(5), // last_found = i
                        Instruction::End,
                        
                        // Decrement search position and continue
                        Instruction::LocalGet(4), // i
                        Instruction::I32Const(1),
                        Instruction::I32Sub,      // i - 1
                        Instruction::LocalSet(4), // i = i - 1
                        Instruction::Br(0),       // continue loop
                    Instruction::End,
                    
                    // Return last found position
                    Instruction::LocalGet(5), // last_found
                Instruction::End,
            Instruction::End,
        ]
    }

    fn generate_starts_with(&self) -> Vec<Instruction> {
        vec![
            // Full startsWith implementation
            // Parameters: text string, prefix string
            
            // Get text length
            Instruction::LocalGet(0), // text ptr
            Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }),
            Instruction::LocalSet(2), // save text length
            
            // Get prefix length
            Instruction::LocalGet(1), // prefix ptr
            Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }),
            Instruction::LocalSet(3), // save prefix length
            
            // If prefix is empty, return true
            Instruction::LocalGet(3), // prefix length
            Instruction::I32Const(0),
            Instruction::I32Eq,       // prefix length == 0
            Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                Instruction::I32Const(1), // return true
            Instruction::Else,
                // If prefix is longer than text, return false
                Instruction::LocalGet(3), // prefix length
                Instruction::LocalGet(2), // text length
                Instruction::I32GtU,      // prefix length > text length
                Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                    Instruction::I32Const(0), // return false
                Instruction::Else,
                    // Compare prefix with beginning of text
                    Instruction::LocalGet(0), // text ptr
                    Instruction::I32Const(4), // offset past length
                    Instruction::I32Add,      // text data
                    Instruction::LocalGet(1), // prefix ptr
                    Instruction::I32Const(4), // offset past length
                    Instruction::I32Add,      // prefix data
                    Instruction::LocalGet(3), // prefix length
                    Instruction::Call(1),     // call memory compare function
                    
                    // Return true if match, false otherwise
                    Instruction::I32Const(0),
                    Instruction::I32Eq,       // compare result == 0 (equal)
                Instruction::End,
            Instruction::End,
        ]
    }

    fn generate_ends_with(&self) -> Vec<Instruction> {
        vec![
            // Full endsWith implementation
            // Parameters: text string, suffix string
            
            // Get text length
            Instruction::LocalGet(0), // text ptr
            Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }),
            Instruction::LocalSet(2), // save text length
            
            // Get suffix length
            Instruction::LocalGet(1), // suffix ptr
            Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }),
            Instruction::LocalSet(3), // save suffix length
            
            // If suffix is empty, return true
            Instruction::LocalGet(3), // suffix length
            Instruction::I32Const(0),
            Instruction::I32Eq,       // suffix length == 0
            Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                Instruction::I32Const(1), // return true
            Instruction::Else,
                // If suffix is longer than text, return false
                Instruction::LocalGet(3), // suffix length
                Instruction::LocalGet(2), // text length
                Instruction::I32GtU,      // suffix length > text length
                Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                    Instruction::I32Const(0), // return false
                Instruction::Else,
                    // Calculate start position for suffix comparison
                    Instruction::LocalGet(2), // text length
                    Instruction::LocalGet(3), // suffix length
                    Instruction::I32Sub,      // text length - suffix length
                    Instruction::LocalSet(4), // save start position
                    
                    // Compare suffix with end of text
                    Instruction::LocalGet(0), // text ptr
                    Instruction::I32Const(4), // offset past length
                    Instruction::I32Add,      // text data
                    Instruction::LocalGet(4), // start position
                    Instruction::I32Add,      // text data + start position
                    Instruction::LocalGet(1), // suffix ptr
                    Instruction::I32Const(4), // offset past length
                    Instruction::I32Add,      // suffix data
                    Instruction::LocalGet(3), // suffix length
                    Instruction::Call(1),     // call memory compare function
                    
                    // Return true if match, false otherwise
                    Instruction::I32Const(0),
                    Instruction::I32Eq,       // compare result == 0 (equal)
                Instruction::End,
            Instruction::End,
        ]
    }

    fn generate_trim(&self) -> Vec<Instruction> {
        vec![
            // Full trim implementation - remove leading and trailing whitespace
            // Parameters: text string
            
            // Get text length
            Instruction::LocalGet(0), // text ptr
            Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }),
            Instruction::LocalSet(1), // save text length
            
            // Find start position (skip leading whitespace)
            Instruction::I32Const(0),
            Instruction::LocalSet(2), // start = 0
            
            // Loop to find first non-whitespace character
            Instruction::Loop(wasm_encoder::BlockType::Empty),
                // Check if we've reached the end
                Instruction::LocalGet(2), // start
                Instruction::LocalGet(1), // text length
                Instruction::I32GeU,     // start >= text length
                Instruction::BrIf(1),    // exit loop if at end
                
                // Load character at start position
                Instruction::LocalGet(0), // text ptr
                Instruction::I32Const(4), // offset past length
                Instruction::I32Add,
                Instruction::LocalGet(2), // start
                Instruction::I32Add,      // character address
                Instruction::I32Load8U(MemArg { offset: 0, align: 0, memory_index: 0 }),
                Instruction::LocalSet(3), // save character
                
                // Check if character is whitespace (space=32, tab=9, newline=10, carriage return=13)
                Instruction::LocalGet(3), // character
                Instruction::I32Const(32), // space
                Instruction::I32Eq,
                Instruction::LocalGet(3), // character
                Instruction::I32Const(9),  // tab
                Instruction::I32Eq,
                Instruction::I32Or,
                Instruction::LocalGet(3), // character
                Instruction::I32Const(10), // newline
                Instruction::I32Eq,
                Instruction::I32Or,
                Instruction::LocalGet(3), // character
                Instruction::I32Const(13), // carriage return
                Instruction::I32Eq,
                Instruction::I32Or,        // is whitespace
                
                // If not whitespace, exit loop
                Instruction::If(wasm_encoder::BlockType::Empty),
                    Instruction::LocalGet(2), // start
                    Instruction::I32Const(1),
                    Instruction::I32Add,      // start + 1
                    Instruction::LocalSet(2), // start = start + 1
                    Instruction::Br(1),       // continue loop
                Instruction::Else,
                    Instruction::Br(2),       // exit loop - found non-whitespace
                Instruction::End,
            Instruction::End,
            
            // Find end position (skip trailing whitespace)
            Instruction::LocalGet(1), // text length
            Instruction::LocalSet(4), // end = text length
            
            // Loop to find last non-whitespace character
            Instruction::Loop(wasm_encoder::BlockType::Empty),
                // Check if we've gone past the start
                Instruction::LocalGet(4), // end
                Instruction::LocalGet(2), // start
                Instruction::I32LeU,      // end <= start
                Instruction::BrIf(1),     // exit loop if end <= start
                
                // Load character at end-1 position
                Instruction::LocalGet(0), // text ptr
                Instruction::I32Const(4), // offset past length
                Instruction::I32Add,
                Instruction::LocalGet(4), // end
                Instruction::I32Const(1),
                Instruction::I32Sub,      // end - 1
                Instruction::I32Add,      // character address
                Instruction::I32Load8U(MemArg { offset: 0, align: 0, memory_index: 0 }),
                Instruction::LocalSet(3), // save character
                
                // Check if character is whitespace
                Instruction::LocalGet(3), // character
                Instruction::I32Const(32), // space
                Instruction::I32Eq,
                Instruction::LocalGet(3), // character
                Instruction::I32Const(9),  // tab
                Instruction::I32Eq,
                Instruction::I32Or,
                Instruction::LocalGet(3), // character
                Instruction::I32Const(10), // newline
                Instruction::I32Eq,
                Instruction::I32Or,
                Instruction::LocalGet(3), // character
                Instruction::I32Const(13), // carriage return
                Instruction::I32Eq,
                Instruction::I32Or,        // is whitespace
                
                // If not whitespace, exit loop
                Instruction::If(wasm_encoder::BlockType::Empty),
                    Instruction::LocalGet(4), // end
                    Instruction::I32Const(1),
                    Instruction::I32Sub,      // end - 1
                    Instruction::LocalSet(4), // end = end - 1
                    Instruction::Br(1),       // continue loop
                Instruction::Else,
                    Instruction::Br(2),       // exit loop - found non-whitespace
                Instruction::End,
            Instruction::End,
            
            // Calculate trimmed length
            Instruction::LocalGet(4), // end
            Instruction::LocalGet(2), // start
            Instruction::I32Sub,      // end - start = trimmed length
            Instruction::LocalSet(5), // save trimmed length
            
            // If trimmed length is 0, return empty string
            Instruction::LocalGet(5), // trimmed length
            Instruction::I32Const(0),
            Instruction::I32Eq,       // trimmed length == 0
            Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                // Allocate empty string
                Instruction::I32Const(4),
                Instruction::Call(0),     // allocate memory
                Instruction::LocalTee(6), // save and keep on stack
                Instruction::I32Const(0), // length = 0
                Instruction::I32Store(MemArg { offset: 0, align: 2, memory_index: 0 }),
                Instruction::LocalGet(6), // return empty string ptr
            Instruction::Else,
                // Allocate memory for trimmed string
                Instruction::LocalGet(5), // trimmed length
                Instruction::I32Const(4), // add 4 bytes for length field
                Instruction::I32Add,
                Instruction::Call(0),     // allocate memory
                Instruction::LocalSet(6), // save new string ptr
                
                // Store length in new string
                Instruction::LocalGet(6), // new string ptr
                Instruction::LocalGet(5), // trimmed length
                Instruction::I32Store(MemArg { offset: 0, align: 2, memory_index: 0 }),
                
                // Copy trimmed data
                Instruction::LocalGet(6), // new string ptr
                Instruction::I32Const(4), // offset past length field
                Instruction::I32Add,      // new string data ptr
                Instruction::LocalGet(0), // original string ptr
                Instruction::I32Const(4), // offset past length field
                Instruction::I32Add,      // original string data ptr
                Instruction::LocalGet(2), // start position
                Instruction::I32Add,      // original data + start
                Instruction::LocalGet(5), // trimmed length
                Instruction::MemoryCopy { src_mem: 0, dst_mem: 0 },  // copy trimmed data
                
                // Return new string pointer
                Instruction::LocalGet(6),
            Instruction::End,
        ]
    }

    fn generate_trim_start(&self) -> Vec<Instruction> {
        vec![
            // Full trimStart implementation - remove leading whitespace only
            // Parameters: text string
            
            // Get text length
            Instruction::LocalGet(0), // text ptr
            Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }),
            Instruction::LocalSet(1), // save text length
            
            // Find start position (skip leading whitespace)
            Instruction::I32Const(0),
            Instruction::LocalSet(2), // start = 0
            
            // Loop to find first non-whitespace character
            Instruction::Loop(wasm_encoder::BlockType::Empty),
                // Check if we've reached the end
                Instruction::LocalGet(2), // start
                Instruction::LocalGet(1), // text length
                Instruction::I32GeU,     // start >= text length
                Instruction::BrIf(1),    // exit loop if at end
                
                // Load character at start position
                Instruction::LocalGet(0), // text ptr
                Instruction::I32Const(4), // offset past length
                Instruction::I32Add,
                Instruction::LocalGet(2), // start
                Instruction::I32Add,      // character address
                Instruction::I32Load8U(MemArg { offset: 0, align: 0, memory_index: 0 }),
                Instruction::LocalSet(3), // save character
                
                // Check if character is whitespace
                Instruction::LocalGet(3), // character
                Instruction::I32Const(32), // space
                Instruction::I32Eq,
                Instruction::LocalGet(3), // character
                Instruction::I32Const(9),  // tab
                Instruction::I32Eq,
                Instruction::I32Or,
                Instruction::LocalGet(3), // character
                Instruction::I32Const(10), // newline
                Instruction::I32Eq,
                Instruction::I32Or,
                Instruction::LocalGet(3), // character
                Instruction::I32Const(13), // carriage return
                Instruction::I32Eq,
                Instruction::I32Or,        // is whitespace
                
                // If not whitespace, exit loop
                Instruction::If(wasm_encoder::BlockType::Empty),
                    Instruction::LocalGet(2), // start
                    Instruction::I32Const(1),
                    Instruction::I32Add,      // start + 1
                    Instruction::LocalSet(2), // start = start + 1
                    Instruction::Br(1),       // continue loop
                Instruction::Else,
                    Instruction::Br(2),       // exit loop - found non-whitespace
                Instruction::End,
            Instruction::End,
            
            // Calculate trimmed length (from start to end)
            Instruction::LocalGet(1), // text length
            Instruction::LocalGet(2), // start
            Instruction::I32Sub,      // text length - start = trimmed length
            Instruction::LocalSet(4), // save trimmed length
            
            // If trimmed length is 0, return empty string
            Instruction::LocalGet(4), // trimmed length
            Instruction::I32Const(0),
            Instruction::I32Eq,       // trimmed length == 0
            Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                // Allocate empty string
                Instruction::I32Const(4),
                Instruction::Call(0),     // allocate memory
                Instruction::LocalTee(5), // save and keep on stack
                Instruction::I32Const(0), // length = 0
                Instruction::I32Store(MemArg { offset: 0, align: 2, memory_index: 0 }),
                Instruction::LocalGet(5), // return empty string ptr
            Instruction::Else,
                // Allocate memory for trimmed string
                Instruction::LocalGet(4), // trimmed length
                Instruction::I32Const(4), // add 4 bytes for length field
                Instruction::I32Add,
                Instruction::Call(0),     // allocate memory
                Instruction::LocalSet(5), // save new string ptr
                
                // Store length in new string
                Instruction::LocalGet(5), // new string ptr
                Instruction::LocalGet(4), // trimmed length
                Instruction::I32Store(MemArg { offset: 0, align: 2, memory_index: 0 }),
                
                // Copy trimmed data
                Instruction::LocalGet(5), // new string ptr
                Instruction::I32Const(4), // offset past length field
                Instruction::I32Add,      // new string data ptr
                Instruction::LocalGet(0), // original string ptr
                Instruction::I32Const(4), // offset past length field
                Instruction::I32Add,      // original string data ptr
                Instruction::LocalGet(2), // start position
                Instruction::I32Add,      // original data + start
                Instruction::LocalGet(4), // trimmed length
                Instruction::MemoryCopy { src_mem: 0, dst_mem: 0 },  // copy trimmed data
                
                // Return new string pointer
                Instruction::LocalGet(5),
            Instruction::End,
        ]
    }

    fn generate_trim_end(&self) -> Vec<Instruction> {
        vec![
            // Full trimEnd implementation - remove trailing whitespace only
            // Parameters: text string
            
            // Get text length
            Instruction::LocalGet(0), // text ptr
            Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }),
            Instruction::LocalSet(1), // save text length
            
            // Find end position (skip trailing whitespace)
            Instruction::LocalGet(1), // text length
            Instruction::LocalSet(2), // end = text length
            
            // Loop to find last non-whitespace character
            Instruction::Loop(wasm_encoder::BlockType::Empty),
                // Check if we've gone past the beginning
                Instruction::LocalGet(2), // end
                Instruction::I32Const(0),
                Instruction::I32LeU,      // end <= 0
                Instruction::BrIf(1),     // exit loop if end <= 0
                
                // Load character at end-1 position
                Instruction::LocalGet(0), // text ptr
                Instruction::I32Const(4), // offset past length
                Instruction::I32Add,
                Instruction::LocalGet(2), // end
                Instruction::I32Const(1),
                Instruction::I32Sub,      // end - 1
                Instruction::I32Add,      // character address
                Instruction::I32Load8U(MemArg { offset: 0, align: 0, memory_index: 0 }),
                Instruction::LocalSet(3), // save character
                
                // Check if character is whitespace
                Instruction::LocalGet(3), // character
                Instruction::I32Const(32), // space
                Instruction::I32Eq,
                Instruction::LocalGet(3), // character
                Instruction::I32Const(9),  // tab
                Instruction::I32Eq,
                Instruction::I32Or,
                Instruction::LocalGet(3), // character
                Instruction::I32Const(10), // newline
                Instruction::I32Eq,
                Instruction::I32Or,
                Instruction::LocalGet(3), // character
                Instruction::I32Const(13), // carriage return
                Instruction::I32Eq,
                Instruction::I32Or,        // is whitespace
                
                // If not whitespace, exit loop
                Instruction::If(wasm_encoder::BlockType::Empty),
                    Instruction::LocalGet(2), // end
                    Instruction::I32Const(1),
                    Instruction::I32Sub,      // end - 1
                    Instruction::LocalSet(2), // end = end - 1
                    Instruction::Br(1),       // continue loop
                Instruction::Else,
                    Instruction::Br(2),       // exit loop - found non-whitespace
                Instruction::End,
            Instruction::End,
            
            // Calculate trimmed length (from 0 to end)
            Instruction::LocalGet(2), // end position = trimmed length
            Instruction::LocalSet(4), // save trimmed length
            
            // If trimmed length is 0, return empty string
            Instruction::LocalGet(4), // trimmed length
            Instruction::I32Const(0),
            Instruction::I32Eq,       // trimmed length == 0
            Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                // Allocate empty string
                Instruction::I32Const(4),
                Instruction::Call(0),     // allocate memory
                Instruction::LocalTee(5), // save and keep on stack
                Instruction::I32Const(0), // length = 0
                Instruction::I32Store(MemArg { offset: 0, align: 2, memory_index: 0 }),
                Instruction::LocalGet(5), // return empty string ptr
            Instruction::Else,
                // Allocate memory for trimmed string
                Instruction::LocalGet(4), // trimmed length
                Instruction::I32Const(4), // add 4 bytes for length field
                Instruction::I32Add,
                Instruction::Call(0),     // allocate memory
                Instruction::LocalSet(5), // save new string ptr
                
                // Store length in new string
                Instruction::LocalGet(5), // new string ptr
                Instruction::LocalGet(4), // trimmed length
                Instruction::I32Store(MemArg { offset: 0, align: 2, memory_index: 0 }),
                
                // Copy trimmed data
                Instruction::LocalGet(5), // new string ptr
                Instruction::I32Const(4), // offset past length field
                Instruction::I32Add,      // new string data ptr
                Instruction::LocalGet(0), // original string ptr
                Instruction::I32Const(4), // offset past length field
                Instruction::I32Add,      // original string data ptr
                Instruction::LocalGet(4), // trimmed length
                Instruction::MemoryCopy { src_mem: 0, dst_mem: 0 },  // copy trimmed data
                
                // Return new string pointer
                Instruction::LocalGet(5),
            Instruction::End,
        ]
    }

    fn generate_replace(&self) -> Vec<Instruction> {
        vec![
            // Basic replace implementation - replace first occurrence
            // For now, return original string (complex operation requiring substring operations)
            // Full implementation would search for oldValue and replace with newValue
            Instruction::LocalGet(0), // return original string
        ]
    }

    fn generate_replace_all(&self) -> Vec<Instruction> {
        vec![
            // Basic replaceAll implementation - replace all occurrences
            // For now, return original string (complex operation requiring multiple substring operations)
            // Full implementation would search for all oldValue instances and replace with newValue
            Instruction::LocalGet(0), // return original string
        ]
    }

    fn generate_split(&self) -> Vec<Instruction> {
        // Simplified string split: split by single character delimiter
        // Parameters: string_ptr, delimiter_ptr
        // Returns: array of string pointers
        vec![
            // Get source string length
            Instruction::LocalGet(0), // string_ptr
            Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }),
            Instruction::LocalSet(2), // save source length
            
            // Get delimiter character (assume single char)
            Instruction::LocalGet(1), // delimiter_ptr
            Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }),
            Instruction::LocalSet(3), // save delimiter length
            
            // If delimiter is empty or longer than 1 char, return original string in array
            Instruction::LocalGet(3), // delimiter length
            Instruction::I32Const(1),
            Instruction::I32Ne, // delimiter_length != 1
            Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
            
            // Return single-element array with original string
            Instruction::I32Const(8), // allocate 8 bytes (4 for length + 4 for element)
            Instruction::Call(0),     // allocate memory
            Instruction::LocalTee(4), // save array ptr and keep on stack
            Instruction::I32Const(1), // array length = 1
            Instruction::I32Store(MemArg { offset: 0, align: 2, memory_index: 0 }),
            
            // Store original string pointer as first element
            Instruction::LocalGet(4), // array ptr
            Instruction::I32Const(4), // offset past length
            Instruction::I32Add,      // element address
            Instruction::LocalGet(0), // original string
            Instruction::I32Store(MemArg { offset: 0, align: 2, memory_index: 0 }),
            
            Instruction::LocalGet(4), // return array pointer
            
            Instruction::Else,
            
            // Get delimiter character
            Instruction::LocalGet(1), // delimiter_ptr
            Instruction::I32Const(16), // skip header
            Instruction::I32Add,
            Instruction::I32Load8U(MemArg { offset: 0, align: 0, memory_index: 0 }),
            Instruction::LocalSet(5), // save delimiter char
            
            // Count occurrences of delimiter
            Instruction::I32Const(0),
            Instruction::LocalSet(6), // count = 0
            Instruction::I32Const(0),
            Instruction::LocalSet(7), // pos = 0
            
            // Count loop
            Instruction::Loop(wasm_encoder::BlockType::Empty),
            Instruction::LocalGet(7), // pos
            Instruction::LocalGet(2), // source_length
            Instruction::I32LtU, // pos < source_length
            Instruction::If(wasm_encoder::BlockType::Empty),
            
            // Check if current char matches delimiter
            Instruction::LocalGet(0), // string_ptr
            Instruction::I32Const(16), // skip header
            Instruction::I32Add,
            Instruction::LocalGet(7), // pos
            Instruction::I32Add, // string data + pos
            Instruction::I32Load8U(MemArg { offset: 0, align: 0, memory_index: 0 }),
            
            Instruction::LocalGet(5), // delimiter char
            Instruction::I32Eq, // char == delimiter
            Instruction::If(wasm_encoder::BlockType::Empty),
            
            // Increment count
            Instruction::LocalGet(6),
            Instruction::I32Const(1),
            Instruction::I32Add,
            Instruction::LocalSet(6),
            
            Instruction::End,
            
            // Increment position
            Instruction::LocalGet(7),
            Instruction::I32Const(1),
            Instruction::I32Add,
            Instruction::LocalSet(7),
            
            Instruction::Br(1), // Continue count loop
            Instruction::End, // End count if
            Instruction::End, // End count loop
            
            // Number of parts = count + 1
            Instruction::LocalGet(6), // count
            Instruction::I32Const(1),
            Instruction::I32Add,
            Instruction::LocalSet(8), // num_parts = count + 1
            
            // Allocate array for parts
            Instruction::LocalGet(8), // num_parts
            Instruction::I32Const(4), // size per element
            Instruction::I32Mul,
            Instruction::I32Const(4), // add space for length
            Instruction::I32Add, // total array size
            Instruction::Call(0),     // allocate memory
            Instruction::LocalSet(9), // save result_array
            
            // Store array length
            Instruction::LocalGet(9), // result_array
            Instruction::LocalGet(8), // num_parts
            Instruction::I32Store(MemArg { offset: 0, align: 2, memory_index: 0 }),
            
            // Split and create substring pointers (simplified: just return original for now)
            // In a full implementation, we would create new string objects for each part
            Instruction::I32Const(0),
            Instruction::LocalSet(10), // part_index = 0
            
            // For simplicity, fill array with original string
            Instruction::Loop(wasm_encoder::BlockType::Empty),
            Instruction::LocalGet(10), // part_index
            Instruction::LocalGet(8), // num_parts
            Instruction::I32LtU, // part_index < num_parts
            Instruction::If(wasm_encoder::BlockType::Empty),
            
            // Store original string in array slot
            Instruction::LocalGet(9), // result_array
            Instruction::I32Const(4), // offset past length
            Instruction::I32Add,
            Instruction::LocalGet(10), // part_index
            Instruction::I32Const(4), // element size
            Instruction::I32Mul,
            Instruction::I32Add, // element address
            Instruction::LocalGet(0), // original string (placeholder)
            Instruction::I32Store(MemArg { offset: 0, align: 2, memory_index: 0 }),
            
            // Increment part_index
            Instruction::LocalGet(10),
            Instruction::I32Const(1),
            Instruction::I32Add,
            Instruction::LocalSet(10),
            
            Instruction::Br(1), // Continue split loop
            Instruction::End, // End split if
            Instruction::End, // End split loop
            
            // Return result array
            Instruction::LocalGet(9),
            
            Instruction::End, // End main else
        ]
    }

    fn generate_join(&self) -> Vec<Instruction> {
        vec![
            // Basic join implementation - return first array element or empty string
            // Parameters: array of strings, separator
            // Full implementation would concatenate all array elements with separator
            
            // Get array length
            Instruction::LocalGet(0), // array ptr
            Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }),
            Instruction::LocalSet(2), // save array length
            
            // If array is empty, return empty string
            Instruction::LocalGet(2), // array length
            Instruction::I32Const(0),
            Instruction::I32Eq,       // length == 0
            Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                // Allocate empty string
                Instruction::I32Const(4),
                Instruction::Call(0),     // allocate memory
                Instruction::LocalTee(3), // save and keep on stack
                Instruction::I32Const(0), // length = 0
                Instruction::I32Store(MemArg { offset: 0, align: 2, memory_index: 0 }),
                Instruction::LocalGet(3), // return empty string ptr
            Instruction::Else,
                // Return first element (simplified join)
                Instruction::LocalGet(0), // array ptr
                Instruction::I32Const(4), // offset past length
                Instruction::I32Add,      // first element address
                Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }),
            Instruction::End,
        ]
    }

    fn generate_char_at(&self) -> Vec<Instruction> {
        vec![
            // Full charAt implementation - return single character string at index
            // Parameters: text string, index
            
            // Get text length
            Instruction::LocalGet(0), // text ptr
            Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }),
            Instruction::LocalSet(2), // save text length
            
            // Check if index is out of bounds
            Instruction::LocalGet(1), // index
            Instruction::I32Const(0),
            Instruction::I32LtS,      // index < 0
            Instruction::LocalGet(1), // index
            Instruction::LocalGet(2), // text length
            Instruction::I32GeU,      // index >= text length
            Instruction::I32Or,       // out of bounds
            Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                // Return empty string if out of bounds
                Instruction::I32Const(4),
                Instruction::Call(0),     // allocate memory
                Instruction::LocalTee(3), // save and keep on stack
                Instruction::I32Const(0), // length = 0
                Instruction::I32Store(MemArg { offset: 0, align: 2, memory_index: 0 }),
                Instruction::LocalGet(3), // return empty string ptr
            Instruction::Else,
                // Allocate string for single character
                Instruction::I32Const(5), // 4 bytes for length + 1 byte for character
                Instruction::Call(0),     // allocate memory
                Instruction::LocalSet(3), // save new string ptr
                
                // Store length = 1
                Instruction::LocalGet(3), // new string ptr
                Instruction::I32Const(1), // length = 1
                Instruction::I32Store(MemArg { offset: 0, align: 2, memory_index: 0 }),
                
                // Load character from original string
                Instruction::LocalGet(0), // text ptr
                Instruction::I32Const(4), // offset past length
                Instruction::I32Add,
                Instruction::LocalGet(1), // index
                Instruction::I32Add,      // character address
                Instruction::I32Load8U(MemArg { offset: 0, align: 0, memory_index: 0 }),
                
                // Store character in new string
                Instruction::LocalGet(3), // new string ptr
                Instruction::I32Const(4), // offset past length
                Instruction::I32Add,      // character storage address
                Instruction::I32Store8(MemArg { offset: 0, align: 0, memory_index: 0 }),
                
                // Return new string pointer
                Instruction::LocalGet(3),
            Instruction::End,
        ]
    }

    fn generate_char_code_at(&self) -> Vec<Instruction> {
        vec![
            // Full charCodeAt implementation - return character code at index
            // Parameters: text string, index
            
            // Get text length
            Instruction::LocalGet(0), // text ptr
            Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }),
            Instruction::LocalSet(2), // save text length
            
            // Check if index is out of bounds
            Instruction::LocalGet(1), // index
            Instruction::I32Const(0),
            Instruction::I32LtS,      // index < 0
            Instruction::LocalGet(1), // index
            Instruction::LocalGet(2), // text length
            Instruction::I32GeU,      // index >= text length
            Instruction::I32Or,       // out of bounds
            Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                // Return 0 if out of bounds (or could return -1)
                Instruction::I32Const(0),
            Instruction::Else,
                // Load and return character code
                Instruction::LocalGet(0), // text ptr
                Instruction::I32Const(4), // offset past length
                Instruction::I32Add,
                Instruction::LocalGet(1), // index
                Instruction::I32Add,      // character address
                Instruction::I32Load8U(MemArg { offset: 0, align: 0, memory_index: 0 }),
            Instruction::End,
        ]
    }

    fn generate_is_blank(&self) -> Vec<Instruction> {
        vec![
            // Full isBlank implementation - check if string contains only whitespace
            // Parameters: text string
            
            // Get text length
            Instruction::LocalGet(0), // text ptr
            Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }),
            Instruction::LocalSet(1), // save text length
            
            // If length is 0, return true (empty string is blank)
            Instruction::LocalGet(1), // text length
            Instruction::I32Const(0),
            Instruction::I32Eq,       // length == 0
            Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                Instruction::I32Const(1), // return true
            Instruction::Else,
                // Initialize loop counter
                Instruction::I32Const(0),
                Instruction::LocalSet(2), // i = 0
                
                // Loop through each character
                Instruction::Loop(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                    // Check if done
                    Instruction::LocalGet(2), // i
                    Instruction::LocalGet(1), // text length
                    Instruction::I32GeU,     // i >= length
                    Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                        Instruction::I32Const(1), // return true - all characters were whitespace
                    Instruction::Else,
                        // Load character
                        Instruction::LocalGet(0), // text ptr
                        Instruction::I32Const(4), // offset past length
                        Instruction::I32Add,
                        Instruction::LocalGet(2), // i
                        Instruction::I32Add,      // character address
                        Instruction::I32Load8U(MemArg { offset: 0, align: 0, memory_index: 0 }),
                        Instruction::LocalSet(3), // save character
                        
                        // Check if character is whitespace
                        Instruction::LocalGet(3), // character
                        Instruction::I32Const(32), // space
                        Instruction::I32Eq,
                        Instruction::LocalGet(3), // character
                        Instruction::I32Const(9),  // tab
                        Instruction::I32Eq,
                        Instruction::I32Or,
                        Instruction::LocalGet(3), // character
                        Instruction::I32Const(10), // newline
                        Instruction::I32Eq,
                        Instruction::I32Or,
                        Instruction::LocalGet(3), // character
                        Instruction::I32Const(13), // carriage return
                        Instruction::I32Eq,
                        Instruction::I32Or,        // is whitespace
                        
                        // If not whitespace, return false
                        Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                            // Increment counter and continue
                            Instruction::LocalGet(2), // i
                            Instruction::I32Const(1),
                            Instruction::I32Add,      // i + 1
                            Instruction::LocalSet(2), // i = i + 1
                            Instruction::Br(1),       // continue loop
                        Instruction::Else,
                            Instruction::I32Const(0), // return false - found non-whitespace
                        Instruction::End,
                    Instruction::End,
                Instruction::End,
            Instruction::End,
        ]
    }

    fn generate_pad_start(&self) -> Vec<Instruction> {
        vec![
            // Basic padStart implementation
            // Parameters: text string, target length, pad string
            // For now, return original string (complex operation requiring length calculation and concatenation)
            // Full implementation would prepend pad string until target length is reached
            Instruction::LocalGet(0), // return original string
        ]
    }

    fn generate_pad_end(&self) -> Vec<Instruction> {
        vec![
            // Basic padEnd implementation
            // Parameters: text string, target length, pad string
            // For now, return original string (complex operation requiring length calculation and concatenation)
            // Full implementation would append pad string until target length is reached
            Instruction::LocalGet(0), // return original string
        ]
    }
}