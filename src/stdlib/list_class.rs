use crate::codegen::CodeGenerator;
use crate::types::WasmType;
use crate::error::CompilerError;
use wasm_encoder::{Instruction, MemArg};
use crate::stdlib::register_stdlib_function;

/// List class implementation for Clean Language
/// Provides comprehensive list manipulation capabilities as static methods
pub struct ListClass;

impl ListClass {
    pub fn new() -> Self {
        Self
    }

    /// Register all List class methods as static functions
    pub fn register_functions(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // Basic list operations
        self.register_basic_operations(codegen)?;
        
        // Search operations
        self.register_search_operations(codegen)?;
        
        // Modification operations
        self.register_modification_operations(codegen)?;
        
        // Transformation operations
        self.register_transformation_operations(codegen)?;
        
        // Utility operations
        self.register_utility_operations(codegen)?;
        
        Ok(())
    }
    
    fn register_basic_operations(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // List.length(list lst) -> integer
        register_stdlib_function(
            codegen,
            "List.length",
            &[WasmType::I32],
            Some(WasmType::I32),
            vec![
                // Get list pointer
                Instruction::LocalGet(0),
                // Load list length (first 4 bytes)
                Instruction::I32Load(MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                }),
            ]
        )?;
        
        // List.isEmpty(list lst) -> boolean
        register_stdlib_function(
            codegen,
            "List.isEmpty",
            &[WasmType::I32],
            Some(WasmType::I32),
            vec![
                // Get list pointer
                Instruction::LocalGet(0),
                // Load list length
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
        
        // List.get(list lst, integer index) -> any
        register_stdlib_function(
            codegen,
            "List.get",
            &[WasmType::I32, WasmType::I32],
            Some(WasmType::I32),
            self.generate_list_get()
        )?;
        
        // List.set(list lst, integer index, any value) -> void
        register_stdlib_function(
            codegen,
            "List.set",
            &[WasmType::I32, WasmType::I32, WasmType::I32],
            None,
            self.generate_list_set()
        )?;
        
        Ok(())
    }
    
    fn register_search_operations(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // List.indexOf(list lst, any value) -> integer
        register_stdlib_function(
            codegen,
            "List.indexOf",
            &[WasmType::I32, WasmType::I32],
            Some(WasmType::I32),
            self.generate_index_of()
        )?;
        
        // List.lastIndexOf(list lst, any value) -> integer
        register_stdlib_function(
            codegen,
            "List.lastIndexOf",
            &[WasmType::I32, WasmType::I32],
            Some(WasmType::I32),
            self.generate_last_index_of()
        )?;
        
        // List.contains(list lst, any value) -> boolean
        register_stdlib_function(
            codegen,
            "List.contains",
            &[WasmType::I32, WasmType::I32],
            Some(WasmType::I32),
            self.generate_contains()
        )?;
        
        // List.find(list lst, any value) -> any
        register_stdlib_function(
            codegen,
            "List.find",
            &[WasmType::I32, WasmType::I32],
            Some(WasmType::I32),
            self.generate_find()
        )?;
        
        Ok(())
    }
    
    fn register_modification_operations(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // List.push(list lst, any value) -> void
        register_stdlib_function(
            codegen,
            "List.push",
            &[WasmType::I32, WasmType::I32],
            None,
            self.generate_push()
        )?;
        
        // List.pop(list lst) -> any
        register_stdlib_function(
            codegen,
            "List.pop",
            &[WasmType::I32],
            Some(WasmType::I32),
            self.generate_pop()
        )?;
        
        // List.shift(list lst) -> any
        register_stdlib_function(
            codegen,
            "List.shift",
            &[WasmType::I32],
            Some(WasmType::I32),
            self.generate_shift()
        )?;
        
        // List.unshift(list lst, any value) -> void
        register_stdlib_function(
            codegen,
            "List.unshift",
            &[WasmType::I32, WasmType::I32],
            None,
            self.generate_unshift()
        )?;
        
        // List.insert(list lst, integer index, any value) -> void
        register_stdlib_function(
            codegen,
            "List.insert",
            &[WasmType::I32, WasmType::I32, WasmType::I32],
            None,
            self.generate_insert()
        )?;
        
        // List.remove(list lst, integer index) -> any
        register_stdlib_function(
            codegen,
            "List.remove",
            &[WasmType::I32, WasmType::I32],
            Some(WasmType::I32),
            self.generate_remove()
        )?;
        
        // List.clear(list lst) -> void
        register_stdlib_function(
            codegen,
            "List.clear",
            &[WasmType::I32],
            None,
            self.generate_clear()
        )?;
        
        Ok(())
    }
    
    fn register_transformation_operations(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // List.slice(list lst, integer start, integer end) -> list
        register_stdlib_function(
            codegen,
            "List.slice",
            &[WasmType::I32, WasmType::I32, WasmType::I32],
            Some(WasmType::I32),
            self.generate_slice()
        )?;
        
        // List.concat(list lst1, list lst2) -> list
        register_stdlib_function(
            codegen,
            "List.concat",
            &[WasmType::I32, WasmType::I32],
            Some(WasmType::I32),
            self.generate_concat()
        )?;
        
        // List.reverse(list lst) -> list
        register_stdlib_function(
            codegen,
            "List.reverse",
            &[WasmType::I32],
            Some(WasmType::I32),
            self.generate_reverse()
        )?;
        
        // List.sort(list lst) -> list
        register_stdlib_function(
            codegen,
            "List.sort",
            &[WasmType::I32],
            Some(WasmType::I32),
            self.generate_sort()
        )?;
        
        // List.join(list lst, string separator) -> string
        register_stdlib_function(
            codegen,
            "List.join",
            &[WasmType::I32, WasmType::I32],
            Some(WasmType::I32),
            self.generate_join()
        )?;
        
        Ok(())
    }
    
    fn register_utility_operations(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // List.copy(list lst) -> list
        register_stdlib_function(
            codegen,
            "List.copy",
            &[WasmType::I32],
            Some(WasmType::I32),
            self.generate_copy()
        )?;
        
        // List.equals(list lst1, list lst2) -> boolean
        register_stdlib_function(
            codegen,
            "List.equals",
            &[WasmType::I32, WasmType::I32],
            Some(WasmType::I32),
            self.generate_equals()
        )?;
        
        // List.fill(list lst, any value) -> void
        register_stdlib_function(
            codegen,
            "List.fill",
            &[WasmType::I32, WasmType::I32],
            None,
            self.generate_fill()
        )?;
        
        // List.toString(list lst) -> string
        register_stdlib_function(
            codegen,
            "List.toString",
            &[WasmType::I32],
            Some(WasmType::I32),
            self.generate_to_string()
        )?;
        
        Ok(())
    }

    // Implementation methods for list operations

    fn generate_list_get(&self) -> Vec<Instruction> {
        vec![
            // Basic list get - calculate element address and load
            // List structure: [length][element0][element1]...
            Instruction::LocalGet(0), // list pointer
            Instruction::LocalGet(1), // index
            Instruction::I32Const(4), // element size (4 bytes for i32 pointers)
            Instruction::I32Mul,      // index * element_size
            Instruction::I32Const(4), // add offset for length field
            Instruction::I32Add,      // list_ptr + 4 + (index * 4)
            Instruction::I32Load(MemArg {
                offset: 0,
                align: 2,
                memory_index: 0,
            }),                       // load element
        ]
    }

    fn generate_list_set(&self) -> Vec<Instruction> {
        vec![
            // Basic list set - calculate element address and store
            Instruction::LocalGet(0), // list pointer
            Instruction::LocalGet(1), // index
            Instruction::I32Const(4), // element size
            Instruction::I32Mul,      // index * element_size
            Instruction::I32Const(4), // add offset for length field
            Instruction::I32Add,      // list_ptr + 4 + (index * 4)
            Instruction::LocalGet(2), // value to store
            Instruction::I32Store(MemArg {
                offset: 0,
                align: 2,
                memory_index: 0,
            }),                       // store element
        ]
    }

    fn generate_index_of(&self) -> Vec<Instruction> {
        vec![
            // Full indexOf implementation - search for value in list
            // Parameters: list, value to find
            
            // Get list length
            Instruction::LocalGet(0), // list ptr
            Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }),
            Instruction::LocalSet(2), // save list length
            
            // Initialize loop counter
            Instruction::I32Const(0),
            Instruction::LocalSet(3), // i = 0
            
            // Search loop
            Instruction::Loop(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                // Check if we've reached the end
                Instruction::LocalGet(3), // i
                Instruction::LocalGet(2), // list length
                Instruction::I32GeU,     // i >= length
                Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                    Instruction::I32Const(-1), // return -1 - not found
                Instruction::Else,
                    // Load list element at index i
                    Instruction::LocalGet(0), // list ptr
                    Instruction::LocalGet(3), // i
                    Instruction::I32Const(4), // element size
                    Instruction::I32Mul,      // i * element_size
                    Instruction::I32Const(4), // add offset for length field
                    Instruction::I32Add,      // list_ptr + 4 + (i * 4)
                    Instruction::I32Load(MemArg {
                        offset: 0,
                        align: 2,
                        memory_index: 0,
                    }),                       // load element
                    
                    // Compare with search value (simple pointer comparison)
                    Instruction::LocalGet(1), // search value
                    Instruction::I32Eq,       // element == search value
                    Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                        Instruction::LocalGet(3), // return current index i
                    Instruction::Else,
                        // Increment counter and continue
                        Instruction::LocalGet(3), // i
                        Instruction::I32Const(1),
                        Instruction::I32Add,      // i + 1
                        Instruction::LocalSet(3), // i = i + 1
                        Instruction::Br(1),       // continue loop
                    Instruction::End,
                Instruction::End,
            Instruction::End,
        ]
    }

    fn generate_last_index_of(&self) -> Vec<Instruction> {
        vec![
            // Full lastIndexOf implementation - search backwards for value in list
            // Parameters: list, value to find
            
            // Get list length
            Instruction::LocalGet(0), // list ptr
            Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }),
            Instruction::LocalSet(2), // save list length
            
            // If list is empty, return -1
            Instruction::LocalGet(2), // list length
            Instruction::I32Const(0),
            Instruction::I32Eq,       // length == 0
            Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                Instruction::I32Const(-1), // return -1
            Instruction::Else,
                // Initialize loop counter (start from last index)
                Instruction::LocalGet(2), // list length
                Instruction::I32Const(1),
                Instruction::I32Sub,      // length - 1
                Instruction::LocalSet(3), // i = length - 1
                
                // Search loop (backwards)
                Instruction::Loop(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                    // Check if we've gone past the beginning
                    Instruction::LocalGet(3), // i
                    Instruction::I32Const(0),
                    Instruction::I32LtS,      // i < 0
                    Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                        Instruction::I32Const(-1), // return -1 - not found
                    Instruction::Else,
                        // Load list element at index i
                        Instruction::LocalGet(0), // list ptr
                        Instruction::LocalGet(3), // i
                        Instruction::I32Const(4), // element size
                        Instruction::I32Mul,      // i * element_size
                        Instruction::I32Const(4), // add offset for length field
                        Instruction::I32Add,      // list_ptr + 4 + (i * 4)
                        Instruction::I32Load(MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        }),                       // load element
                        
                        // Compare with search value
                        Instruction::LocalGet(1), // search value
                        Instruction::I32Eq,       // element == search value
                        Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                            Instruction::LocalGet(3), // return current index i
                        Instruction::Else,
                            // Decrement counter and continue
                            Instruction::LocalGet(3), // i
                            Instruction::I32Const(1),
                            Instruction::I32Sub,      // i - 1
                            Instruction::LocalSet(3), // i = i - 1
                            Instruction::Br(1),       // continue loop
                        Instruction::End,
                    Instruction::End,
                Instruction::End,
            Instruction::End,
        ]
    }

    fn generate_contains(&self) -> Vec<Instruction> {
        vec![
            // Full contains implementation - check if value exists in list
            // Parameters: list, value to find
            
            // Get list length
            Instruction::LocalGet(0), // list ptr
            Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }),
            Instruction::LocalSet(2), // save list length
            
            // Initialize loop counter
            Instruction::I32Const(0),
            Instruction::LocalSet(3), // i = 0
            
            // Search loop
            Instruction::Loop(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                // Check if we've reached the end
                Instruction::LocalGet(3), // i
                Instruction::LocalGet(2), // list length
                Instruction::I32GeU,     // i >= length
                Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                    Instruction::I32Const(0), // return false - not found
                Instruction::Else,
                    // Load list element at index i
                    Instruction::LocalGet(0), // list ptr
                    Instruction::LocalGet(3), // i
                    Instruction::I32Const(4), // element size
                    Instruction::I32Mul,      // i * element_size
                    Instruction::I32Const(4), // add offset for length field
                    Instruction::I32Add,      // list_ptr + 4 + (i * 4)
                    Instruction::I32Load(MemArg {
                        offset: 0,
                        align: 2,
                        memory_index: 0,
                    }),                       // load element
                    
                    // Compare with search value (simple pointer comparison)
                    Instruction::LocalGet(1), // search value
                    Instruction::I32Eq,       // element == search value
                    Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                        Instruction::I32Const(1), // return true - found
                    Instruction::Else,
                        // Increment counter and continue
                        Instruction::LocalGet(3), // i
                        Instruction::I32Const(1),
                        Instruction::I32Add,      // i + 1
                        Instruction::LocalSet(3), // i = i + 1
                        Instruction::Br(1),       // continue loop
                    Instruction::End,
                Instruction::End,
            Instruction::End,
        ]
    }

    fn generate_find(&self) -> Vec<Instruction> {
        vec![
            // Full find implementation - return first matching element or null
            // Parameters: list, value to find
            
            // Get list length
            Instruction::LocalGet(0), // list ptr
            Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }),
            Instruction::LocalSet(2), // save list length
            
            // Initialize loop counter
            Instruction::I32Const(0),
            Instruction::LocalSet(3), // i = 0
            
            // Search loop
            Instruction::Loop(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                // Check if we've reached the end
                Instruction::LocalGet(3), // i
                Instruction::LocalGet(2), // list length
                Instruction::I32GeU,     // i >= length
                Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                    Instruction::I32Const(0), // return null - not found
                Instruction::Else,
                    // Load list element at index i
                    Instruction::LocalGet(0), // list ptr
                    Instruction::LocalGet(3), // i
                    Instruction::I32Const(4), // element size
                    Instruction::I32Mul,      // i * element_size
                    Instruction::I32Const(4), // add offset for length field
                    Instruction::I32Add,      // list_ptr + 4 + (i * 4)
                    Instruction::I32Load(MemArg {
                        offset: 0,
                        align: 2,
                        memory_index: 0,
                    }),                       // load element
                    Instruction::LocalSet(4), // save element
                    
                    // Compare with search value
                    Instruction::LocalGet(4), // element
                    Instruction::LocalGet(1), // search value
                    Instruction::I32Eq,       // element == search value
                    Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                        Instruction::LocalGet(4), // return found element
                    Instruction::Else,
                        // Increment counter and continue
                        Instruction::LocalGet(3), // i
                        Instruction::I32Const(1),
                        Instruction::I32Add,      // i + 1
                        Instruction::LocalSet(3), // i = i + 1
                        Instruction::Br(1),       // continue loop
                    Instruction::End,
                Instruction::End,
            Instruction::End,
        ]
    }

    fn generate_push(&self) -> Vec<Instruction> {
        vec![
            // Basic push - no-op for now
            // Full implementation would reallocate list with increased size
        ]
    }

    fn generate_pop(&self) -> Vec<Instruction> {
        vec![
            // Basic pop - return 0 for now
            // Full implementation would return and remove last element
            Instruction::I32Const(0),
        ]
    }

    fn generate_shift(&self) -> Vec<Instruction> {
        vec![
            // Basic shift - return 0 for now
            // Full implementation would return and remove first element
            Instruction::I32Const(0),
        ]
    }

    fn generate_unshift(&self) -> Vec<Instruction> {
        vec![
            // Basic unshift - no-op for now
            // Full implementation would insert element at beginning
        ]
    }

    fn generate_insert(&self) -> Vec<Instruction> {
        vec![
            // Basic insert - no-op for now
            // Full implementation would insert element at specified index
        ]
    }

    fn generate_remove(&self) -> Vec<Instruction> {
        vec![
            // Basic remove - return 0 for now
            // Full implementation would remove element at index
            Instruction::I32Const(0),
        ]
    }

    fn generate_clear(&self) -> Vec<Instruction> {
        vec![
            // Full clear implementation - reset list length to 0
            // Parameters: list
            
            // Set list length to 0
            Instruction::LocalGet(0), // list ptr
            Instruction::I32Const(0), // new length = 0
            Instruction::I32Store(MemArg {
                offset: 0,
                align: 2,
                memory_index: 0,
            }),                       // store new length
            // Note: This doesn't deallocate memory, just sets length to 0
        ]
    }

    fn generate_slice(&self) -> Vec<Instruction> {
        vec![
            // Full slice implementation - create new list with subset of elements
            // Parameters: list, start index, end index
            
            // Get list length
            Instruction::LocalGet(0), // list ptr
            Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }),
            Instruction::LocalSet(3), // save original list length
            
            // Calculate slice length (end - start)
            Instruction::LocalGet(2), // end index
            Instruction::LocalGet(1), // start index
            Instruction::I32Sub,      // end - start
            Instruction::LocalSet(4), // save slice length
            
            // Bounds check: if slice length <= 0, return empty list
            Instruction::LocalGet(4), // slice length
            Instruction::I32Const(0),
            Instruction::I32LeS,      // slice length <= 0
            Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                // Return empty list
                Instruction::I32Const(4), // allocate 4 bytes for length only
                Instruction::Call(0),     // allocate memory
                Instruction::LocalTee(5), // save and keep on stack
                Instruction::I32Const(0), // length = 0
                Instruction::I32Store(MemArg { offset: 0, align: 2, memory_index: 0 }),
                Instruction::LocalGet(5), // return empty list ptr
            Instruction::Else,
                // Allocate memory for new list
                Instruction::LocalGet(4), // slice length
                Instruction::I32Const(4), // element size
                Instruction::I32Mul,      // slice length * element_size
                Instruction::I32Const(4), // add 4 bytes for length field
                Instruction::I32Add,      // total allocation size
                Instruction::Call(0),     // allocate memory
                Instruction::LocalSet(5), // save new list ptr
                
                // Store length in new list
                Instruction::LocalGet(5), // new list ptr
                Instruction::LocalGet(4), // slice length
                Instruction::I32Store(MemArg { offset: 0, align: 2, memory_index: 0 }),
                
                // Copy slice elements
                Instruction::LocalGet(5), // new list ptr
                Instruction::I32Const(4), // offset past length field
                Instruction::I32Add,      // new list data ptr
                Instruction::LocalGet(0), // original list ptr
                Instruction::I32Const(4), // offset past length field
                Instruction::I32Add,      // original list data ptr
                Instruction::LocalGet(1), // start index
                Instruction::I32Const(4), // element size
                Instruction::I32Mul,      // start * element_size
                Instruction::I32Add,      // original data + start offset
                Instruction::LocalGet(4), // slice length
                Instruction::I32Const(4), // element size
                Instruction::I32Mul,      // slice length * element_size (bytes to copy)
                Instruction::MemoryCopy { src_mem: 0, dst_mem: 0 },  // copy slice data
                
                // Return new list pointer
                Instruction::LocalGet(5),
            Instruction::End,
        ]
    }

    fn generate_concat(&self) -> Vec<Instruction> {
        vec![
            // Full concat implementation - combine two lists into new list
            // Parameters: list1, list2
            
            // Get length of first list
            Instruction::LocalGet(0), // list1 ptr
            Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }),
            Instruction::LocalSet(2), // save list1 length
            
            // Get length of second list
            Instruction::LocalGet(1), // list2 ptr
            Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }),
            Instruction::LocalSet(3), // save list2 length
            
            // Calculate total length
            Instruction::LocalGet(2), // list1 length
            Instruction::LocalGet(3), // list2 length
            Instruction::I32Add,      // total length
            Instruction::LocalSet(4), // save total length
            
            // Allocate memory for new list
            Instruction::LocalGet(4), // total length
            Instruction::I32Const(4), // element size
            Instruction::I32Mul,      // total length * element_size
            Instruction::I32Const(4), // add 4 bytes for length field
            Instruction::I32Add,      // total allocation size
            Instruction::Call(0),     // allocate memory
            Instruction::LocalSet(5), // save new list ptr
            
            // Store total length in new list
            Instruction::LocalGet(5), // new list ptr
            Instruction::LocalGet(4), // total length
            Instruction::I32Store(MemArg { offset: 0, align: 2, memory_index: 0 }),
            
            // Copy first list elements
            Instruction::LocalGet(5), // new list ptr
            Instruction::I32Const(4), // offset past length field
            Instruction::I32Add,      // new list data ptr
            Instruction::LocalGet(0), // list1 ptr
            Instruction::I32Const(4), // offset past length field
            Instruction::I32Add,      // list1 data ptr
            Instruction::LocalGet(2), // list1 length
            Instruction::I32Const(4), // element size
            Instruction::I32Mul,      // list1 length * element_size (bytes to copy)
            Instruction::MemoryCopy { src_mem: 0, dst_mem: 0 },  // copy list1 data
            
            // Copy second list elements
            Instruction::LocalGet(5), // new list ptr
            Instruction::I32Const(4), // offset past length field
            Instruction::I32Add,      // new list data ptr
            Instruction::LocalGet(2), // list1 length
            Instruction::I32Const(4), // element size
            Instruction::I32Mul,      // list1 length * element_size (offset for list2)
            Instruction::I32Add,      // position after list1 data
            Instruction::LocalGet(1), // list2 ptr
            Instruction::I32Const(4), // offset past length field
            Instruction::I32Add,      // list2 data ptr
            Instruction::LocalGet(3), // list2 length
            Instruction::I32Const(4), // element size
            Instruction::I32Mul,      // list2 length * element_size (bytes to copy)
            Instruction::MemoryCopy { src_mem: 0, dst_mem: 0 },  // copy list2 data
            
            // Return new list pointer
            Instruction::LocalGet(5),
        ]
    }

    fn generate_reverse(&self) -> Vec<Instruction> {
        vec![
            // Full reverse implementation - reverse list in place
            // Parameters: list
            
            // Get list length
            Instruction::LocalGet(0), // list ptr
            Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }),
            Instruction::LocalSet(1), // save list length
            
            // If length <= 1, no reversal needed
            Instruction::LocalGet(1), // list length
            Instruction::I32Const(1),
            Instruction::I32LeU,      // length <= 1
            Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                Instruction::LocalGet(0), // return original list
            Instruction::Else,
                // Initialize indices for swapping
                Instruction::I32Const(0),
                Instruction::LocalSet(2), // left = 0
                
                Instruction::LocalGet(1), // list length
                Instruction::I32Const(1),
                Instruction::I32Sub,      // length - 1
                Instruction::LocalSet(3), // right = length - 1
                
                // Swap loop
                Instruction::Loop(wasm_encoder::BlockType::Empty),
                    // Check if done (left >= right)
                    Instruction::LocalGet(2), // left
                    Instruction::LocalGet(3), // right
                    Instruction::I32GeU,     // left >= right
                    Instruction::BrIf(1),    // exit loop if done
                    
                    // Load left element
                    Instruction::LocalGet(0), // list ptr
                    Instruction::LocalGet(2), // left
                    Instruction::I32Const(4), // element size
                    Instruction::I32Mul,      // left * element_size
                    Instruction::I32Const(4), // add offset for length field
                    Instruction::I32Add,      // list_ptr + 4 + (left * 4)
                    Instruction::I32Load(MemArg {
                        offset: 0,
                        align: 2,
                        memory_index: 0,
                    }),                       // load left element
                    Instruction::LocalSet(4), // save left element
                    
                    // Load right element
                    Instruction::LocalGet(0), // list ptr
                    Instruction::LocalGet(3), // right
                    Instruction::I32Const(4), // element size
                    Instruction::I32Mul,      // right * element_size
                    Instruction::I32Const(4), // add offset for length field
                    Instruction::I32Add,      // list_ptr + 4 + (right * 4)
                    Instruction::I32Load(MemArg {
                        offset: 0,
                        align: 2,
                        memory_index: 0,
                    }),                       // load right element
                    Instruction::LocalSet(5), // save right element
                    
                    // Store right element at left position
                    Instruction::LocalGet(0), // list ptr
                    Instruction::LocalGet(2), // left
                    Instruction::I32Const(4), // element size
                    Instruction::I32Mul,      // left * element_size
                    Instruction::I32Const(4), // add offset for length field
                    Instruction::I32Add,      // list_ptr + 4 + (left * 4)
                    Instruction::LocalGet(5), // right element
                    Instruction::I32Store(MemArg {
                        offset: 0,
                        align: 2,
                        memory_index: 0,
                    }),                       // store right element at left
                    
                    // Store left element at right position
                    Instruction::LocalGet(0), // list ptr
                    Instruction::LocalGet(3), // right
                    Instruction::I32Const(4), // element size
                    Instruction::I32Mul,      // right * element_size
                    Instruction::I32Const(4), // add offset for length field
                    Instruction::I32Add,      // list_ptr + 4 + (right * 4)
                    Instruction::LocalGet(4), // left element
                    Instruction::I32Store(MemArg {
                        offset: 0,
                        align: 2,
                        memory_index: 0,
                    }),                       // store left element at right
                    
                    // Move indices toward center
                    Instruction::LocalGet(2), // left
                    Instruction::I32Const(1),
                    Instruction::I32Add,      // left + 1
                    Instruction::LocalSet(2), // left = left + 1
                    
                    Instruction::LocalGet(3), // right
                    Instruction::I32Const(1),
                    Instruction::I32Sub,      // right - 1
                    Instruction::LocalSet(3), // right = right - 1
                    
                    Instruction::Br(0),       // continue loop
                Instruction::End,
                
                // Return modified list
                Instruction::LocalGet(0),
            Instruction::End,
        ]
    }

    fn generate_sort(&self) -> Vec<Instruction> {
        vec![
            // Basic sort - return original list for now
            // Full implementation would sort elements in place or create sorted copy
            Instruction::LocalGet(0),
        ]
    }

    fn generate_join(&self) -> Vec<Instruction> {
        vec![
            // Basic join - return empty string for now
            // Full implementation would concatenate elements with separator
            Instruction::I32Const(0), // Empty string pointer
        ]
    }

    fn generate_copy(&self) -> Vec<Instruction> {
        vec![
            // Full copy implementation - create shallow copy of list
            // Parameters: list
            
            // Get list length
            Instruction::LocalGet(0), // list ptr
            Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }),
            Instruction::LocalSet(1), // save list length
            
            // Calculate total size needed (length field + elements)
            Instruction::LocalGet(1), // list length
            Instruction::I32Const(4), // element size (4 bytes each)
            Instruction::I32Mul,      // length * element_size
            Instruction::I32Const(4), // add 4 bytes for length field
            Instruction::I32Add,      // total size
            Instruction::LocalSet(2), // save total size
            
            // Allocate memory for new list
            Instruction::LocalGet(2), // total size
            Instruction::Call(0),     // allocate memory
            Instruction::LocalSet(3), // save new list ptr
            
            // Copy entire list (length + all elements)
            Instruction::LocalGet(3), // new list ptr (destination)
            Instruction::LocalGet(0), // original list ptr (source)
            Instruction::LocalGet(2), // total size
            Instruction::MemoryCopy { src_mem: 0, dst_mem: 0 },  // copy all data
            
            // Return new list pointer
            Instruction::LocalGet(3),
        ]
    }

    fn generate_equals(&self) -> Vec<Instruction> {
        vec![
            // Full equals implementation - compare lists element by element
            // Parameters: list1, list2
            
            // Get length of first list
            Instruction::LocalGet(0), // list1 ptr
            Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }),
            Instruction::LocalSet(2), // save list1 length
            
            // Get length of second list
            Instruction::LocalGet(1), // list2 ptr
            Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }),
            Instruction::LocalSet(3), // save list2 length
            
            // If lengths are different, return false
            Instruction::LocalGet(2), // list1 length
            Instruction::LocalGet(3), // list2 length
            Instruction::I32Ne,       // lengths != equal
            Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                Instruction::I32Const(0), // return false
            Instruction::Else,
                // If both lists are empty, return true
                Instruction::LocalGet(2), // list1 length
                Instruction::I32Const(0),
                Instruction::I32Eq,       // length == 0
                Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                    Instruction::I32Const(1), // return true
                Instruction::Else,
                    // Compare elements one by one
                    Instruction::I32Const(0),
                    Instruction::LocalSet(4), // i = 0
                    
                    // Comparison loop
                    Instruction::Loop(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                        // Check if we've compared all elements
                        Instruction::LocalGet(4), // i
                        Instruction::LocalGet(2), // list length
                        Instruction::I32GeU,     // i >= length
                        Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                            Instruction::I32Const(1), // return true - all elements matched
                        Instruction::Else,
                            // Load element from list1
                            Instruction::LocalGet(0), // list1 ptr
                            Instruction::LocalGet(4), // i
                            Instruction::I32Const(4), // element size
                            Instruction::I32Mul,      // i * element_size
                            Instruction::I32Const(4), // add offset for length field
                            Instruction::I32Add,      // list1_ptr + 4 + (i * 4)
                            Instruction::I32Load(MemArg {
                                offset: 0,
                                align: 2,
                                memory_index: 0,
                            }),                       // load list1[i]
                            
                            // Load element from list2
                            Instruction::LocalGet(1), // list2 ptr
                            Instruction::LocalGet(4), // i
                            Instruction::I32Const(4), // element size
                            Instruction::I32Mul,      // i * element_size
                            Instruction::I32Const(4), // add offset for length field
                            Instruction::I32Add,      // list2_ptr + 4 + (i * 4)
                            Instruction::I32Load(MemArg {
                                offset: 0,
                                align: 2,
                                memory_index: 0,
                            }),                       // load list2[i]
                            
                            // Compare elements
                            Instruction::I32Eq,       // list1[i] == list2[i]
                            Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
                                // Elements match, continue to next
                                Instruction::LocalGet(4), // i
                                Instruction::I32Const(1),
                                Instruction::I32Add,      // i + 1
                                Instruction::LocalSet(4), // i = i + 1
                                Instruction::Br(1),       // continue loop
                            Instruction::Else,
                                Instruction::I32Const(0), // return false - elements don't match
                            Instruction::End,
                        Instruction::End,
                    Instruction::End,
                Instruction::End,
            Instruction::End,
        ]
    }

    fn generate_fill(&self) -> Vec<Instruction> {
        vec![
            // Full fill implementation - set all elements to specified value
            // Parameters: list, value
            
            // Get list length
            Instruction::LocalGet(0), // list ptr
            Instruction::I32Load(MemArg { offset: 0, align: 2, memory_index: 0 }),
            Instruction::LocalSet(2), // save list length
            
            // Initialize loop counter
            Instruction::I32Const(0),
            Instruction::LocalSet(3), // i = 0
            
            // Fill loop
            Instruction::Loop(wasm_encoder::BlockType::Empty),
                // Check if done
                Instruction::LocalGet(3), // i
                Instruction::LocalGet(2), // list length
                Instruction::I32GeU,     // i >= length
                Instruction::BrIf(1),    // exit loop if done
                
                // Store value at current index
                Instruction::LocalGet(0), // list ptr
                Instruction::LocalGet(3), // i
                Instruction::I32Const(4), // element size
                Instruction::I32Mul,      // i * element_size
                Instruction::I32Const(4), // add offset for length field
                Instruction::I32Add,      // list_ptr + 4 + (i * 4)
                Instruction::LocalGet(1), // value to store
                Instruction::I32Store(MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                }),                       // store value
                
                // Increment counter
                Instruction::LocalGet(3), // i
                Instruction::I32Const(1),
                Instruction::I32Add,      // i + 1
                Instruction::LocalSet(3), // i = i + 1
                Instruction::Br(0),       // continue loop
            Instruction::End,
        ]
    }

    fn generate_to_string(&self) -> Vec<Instruction> {
        vec![
            // Basic toString - return empty string for now
            // Full implementation would convert list to string representation
            Instruction::I32Const(0), // Empty string pointer
        ]
    }
}