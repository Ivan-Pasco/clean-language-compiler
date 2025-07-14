use crate::codegen::CodeGenerator;
use crate::types::WasmType;
use crate::error::CompilerError;
use crate::ast::{ListBehavior};
use wasm_encoder::{Instruction, MemArg};
use crate::stdlib::register_stdlib_function;

/// List behavior implementation for Clean Language
/// Handles different list behaviors: line (FIFO), pile (LIFO), unique (set)
pub struct ListBehaviorManager {
    pub behaviors: std::collections::HashMap<i32, ListBehavior>,
    next_list_id: i32,
}

impl ListBehaviorManager {
    pub fn new() -> Self {
        Self {
            behaviors: std::collections::HashMap::new(),
            next_list_id: 1,
        }
    }

    /// Register a new list with default behavior
    pub fn register_list(&mut self) -> i32 {
        let list_id = self.next_list_id;
        self.behaviors.insert(list_id, ListBehavior::Default);
        self.next_list_id += 1;
        list_id
    }

    /// Set behavior for a specific list
    pub fn set_behavior(&mut self, list_id: i32, behavior: ListBehavior) {
        self.behaviors.insert(list_id, behavior);
    }

    /// Get behavior for a specific list
    pub fn get_behavior(&self, list_id: i32) -> ListBehavior {
        self.behaviors.get(&list_id).copied().unwrap_or(ListBehavior::Default)
    }

    /// Register all list behavior functions as stdlib functions
    pub fn register_functions(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        self.register_behavior_operations(codegen)?;
        self.register_property_operations(codegen)?;
        Ok(())
    }

    fn register_behavior_operations(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // List.add(list, value) - adds element according to behavior
        register_stdlib_function(
            codegen,
            "List.add",
            &[WasmType::I32, WasmType::I32],
            None,
            self.generate_list_add()
        )?;

        // List.remove(list) -> any - removes element according to behavior
        register_stdlib_function(
            codegen,
            "List.remove",
            &[WasmType::I32],
            Some(WasmType::I32),
            self.generate_list_remove()
        )?;

        // List.peek(list) -> any - views next element without removing
        register_stdlib_function(
            codegen,
            "List.peek",
            &[WasmType::I32],
            Some(WasmType::I32),
            self.generate_list_peek()
        )?;

        // List.setBehavior(list, behavior_string) - sets behavior for list
        register_stdlib_function(
            codegen,
            "List.setBehavior",
            &[WasmType::I32, WasmType::I32],
            None,
            self.generate_set_behavior()
        )?;

        // List.getBehavior(list) -> string - gets current behavior
        register_stdlib_function(
            codegen,
            "List.getBehavior",
            &[WasmType::I32],
            Some(WasmType::I32),
            self.generate_get_behavior()
        )?;

        Ok(())
    }

    fn register_property_operations(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // List.contains(list, value) -> boolean - checks if value exists (for unique behavior)
        register_stdlib_function(
            codegen,
            "List.contains",
            &[WasmType::I32, WasmType::I32],
            Some(WasmType::I32),
            self.generate_list_contains()
        )?;

        // List.size(list) -> integer - gets list size
        register_stdlib_function(
            codegen,
            "List.size",
            &[WasmType::I32],
            Some(WasmType::I32),
            self.generate_list_size()
        )?;

        Ok(())
    }

    /// Generate WebAssembly instructions for List.add operation
    fn generate_list_add(&self) -> Vec<Instruction> {
        vec![
            // Get list pointer (arg 0)
            Instruction::LocalGet(0),
            // Get value to add (arg 1)
            Instruction::LocalGet(1),
            
            // Load list behavior (stored at offset 8 in list structure)
            Instruction::LocalGet(0),
            Instruction::I32Load(MemArg {
                offset: 8, // behavior field offset
                align: 2,
                memory_index: 0,
            }),
            
            // Switch on behavior type
            // 0 = Default, 1 = Line, 2 = Pile, 3 = Unique, etc.
            Instruction::LocalSet(2), // Store behavior in local 2
            
            // Check if unique behavior and value already exists
            Instruction::LocalGet(2),
            Instruction::I32Const(3), // Unique behavior
            Instruction::I32Eq,
            Instruction::If(wasm_encoder::BlockType::Empty),
                // Check if value already exists for unique behavior
                Instruction::LocalGet(0),
                Instruction::LocalGet(1),
                Instruction::Call(0), // Call List.contains function
                Instruction::If(wasm_encoder::BlockType::Empty),
                    // Value exists, don't add (return early)
                    Instruction::Return,
                Instruction::End,
            Instruction::End,
            
            // Get current size
            Instruction::LocalGet(0),
            Instruction::I32Load(MemArg {
                offset: 0, // size field
                align: 2,
                memory_index: 0,
            }),
            Instruction::LocalSet(3), // Store size in local 3
            
            // Get capacity
            Instruction::LocalGet(0),
            Instruction::I32Load(MemArg {
                offset: 4, // capacity field
                align: 2,
                memory_index: 0,
            }),
            Instruction::LocalSet(4), // Store capacity in local 4
            
            // Check if resize needed
            Instruction::LocalGet(3), // size
            Instruction::LocalGet(4), // capacity
            Instruction::I32GeU,
            Instruction::If(wasm_encoder::BlockType::Empty),
                // Resize list (double capacity)
                // This is a simplified implementation - would need proper memory management
                Instruction::LocalGet(0),
                Instruction::LocalGet(4),
                Instruction::I32Const(2),
                Instruction::I32Mul,
                Instruction::I32Store(MemArg {
                    offset: 4, // capacity field
                    align: 2,
                    memory_index: 0,
                }),
            Instruction::End,
            
            // Add element based on behavior
            Instruction::LocalGet(2), // behavior
            Instruction::I32Const(2), // Pile behavior
            Instruction::I32Eq,
            Instruction::If(wasm_encoder::BlockType::Empty),
                // Pile behavior: add to front (LIFO)
                // Shift existing elements
                // Add new element at position 0
                Instruction::LocalGet(0),
                Instruction::I32Const(12), // data offset
                Instruction::LocalGet(1),
                Instruction::I32Store(MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                }),
            Instruction::Else,
                // Default/Line behavior: add to end (FIFO)
                Instruction::LocalGet(0),
                Instruction::I32Const(12), // data offset
                Instruction::LocalGet(3), // size
                Instruction::I32Const(4), // element size
                Instruction::I32Mul,
                Instruction::I32Add,
                Instruction::LocalGet(1),
                Instruction::I32Store(MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                }),
            Instruction::End,
            
            // Increment size
            Instruction::LocalGet(0),
            Instruction::LocalGet(3),
            Instruction::I32Const(1),
            Instruction::I32Add,
            Instruction::I32Store(MemArg {
                offset: 0, // size field
                align: 2,
                memory_index: 0,
            }),
        ]
    }

    /// Generate WebAssembly instructions for List.remove operation
    fn generate_list_remove(&self) -> Vec<Instruction> {
        vec![
            // Get list pointer (arg 0)
            Instruction::LocalGet(0),
            
            // Check if list is empty
            Instruction::LocalGet(0),
            Instruction::I32Load(MemArg {
                offset: 0, // size field
                align: 2,
                memory_index: 0,
            }),
            Instruction::I32Const(0),
            Instruction::I32Eq,
            Instruction::If(wasm_encoder::BlockType::Result(WasmType::I32.into())),
                // Empty list, return 0 (null/error value)
                Instruction::I32Const(0),
            Instruction::Else,
                // Load behavior
                Instruction::LocalGet(0),
                Instruction::I32Load(MemArg {
                    offset: 8, // behavior field
                    align: 2,
                    memory_index: 0,
                }),
                Instruction::LocalSet(1), // Store behavior in local 1
                
                // Get current size
                Instruction::LocalGet(0),
                Instruction::I32Load(MemArg {
                    offset: 0, // size field
                    align: 2,
                    memory_index: 0,
                }),
                Instruction::LocalSet(2), // Store size in local 2
                
                // Remove based on behavior
                Instruction::LocalGet(1), // behavior
                Instruction::I32Const(2), // Pile behavior
                Instruction::I32Eq,
                Instruction::If(wasm_encoder::BlockType::Result(WasmType::I32.into())),
                    // Pile behavior: remove from front (LIFO)
                    Instruction::LocalGet(0),
                    Instruction::I32Const(12), // data offset
                    Instruction::I32Load(MemArg {
                        offset: 0,
                        align: 2,
                        memory_index: 0,
                    }),
                Instruction::Else,
                    // Default/Line behavior: remove from front (FIFO)
                    Instruction::LocalGet(0),
                    Instruction::I32Const(12), // data offset
                    Instruction::I32Load(MemArg {
                        offset: 0,
                        align: 2,
                        memory_index: 0,
                    }),
                Instruction::End,
                
                // Decrement size
                Instruction::LocalGet(0),
                Instruction::LocalGet(2),
                Instruction::I32Const(1),
                Instruction::I32Sub,
                Instruction::I32Store(MemArg {
                    offset: 0, // size field
                    align: 2,
                    memory_index: 0,
                }),
            Instruction::End,
        ]
    }

    /// Generate WebAssembly instructions for List.peek operation
    fn generate_list_peek(&self) -> Vec<Instruction> {
        vec![
            // Get list pointer (arg 0)
            Instruction::LocalGet(0),
            
            // Check if list is empty
            Instruction::LocalGet(0),
            Instruction::I32Load(MemArg {
                offset: 0, // size field
                align: 2,
                memory_index: 0,
            }),
            Instruction::I32Const(0),
            Instruction::I32Eq,
            Instruction::If(wasm_encoder::BlockType::Result(WasmType::I32.into())),
                // Empty list, return 0
                Instruction::I32Const(0),
            Instruction::Else,
                // Load behavior to determine peek position
                Instruction::LocalGet(0),
                Instruction::I32Load(MemArg {
                    offset: 8, // behavior field
                    align: 2,
                    memory_index: 0,
                }),
                Instruction::LocalSet(1), // Store behavior in local 1
                
                Instruction::LocalGet(1), // behavior
                Instruction::I32Const(2), // Pile behavior
                Instruction::I32Eq,
                Instruction::If(wasm_encoder::BlockType::Result(WasmType::I32.into())),
                    // Pile behavior: peek at front
                    Instruction::LocalGet(0),
                    Instruction::I32Const(12), // data offset
                    Instruction::I32Load(MemArg {
                        offset: 0,
                        align: 2,
                        memory_index: 0,
                    }),
                Instruction::Else,
                    // Default/Line behavior: peek at front
                    Instruction::LocalGet(0),
                    Instruction::I32Const(12), // data offset
                    Instruction::I32Load(MemArg {
                        offset: 0,
                        align: 2,
                        memory_index: 0,
                    }),
                Instruction::End,
            Instruction::End,
        ]
    }

    /// Generate WebAssembly instructions for List.contains operation
    fn generate_list_contains(&self) -> Vec<Instruction> {
        vec![
            // Get list pointer (arg 0) and value to find (arg 1)
            Instruction::LocalGet(0),
            Instruction::LocalGet(1),
            
            // Get list size
            Instruction::LocalGet(0),
            Instruction::I32Load(MemArg {
                offset: 0, // size field
                align: 2,
                memory_index: 0,
            }),
            Instruction::LocalSet(2), // Store size in local 2
            
            // Initialize index counter
            Instruction::I32Const(0),
            Instruction::LocalSet(3), // Store index in local 3
            
            // Loop through list elements
            Instruction::Loop(wasm_encoder::BlockType::Empty),
                // Check if index >= size
                Instruction::LocalGet(3),
                Instruction::LocalGet(2),
                Instruction::I32GeU,
                Instruction::If(wasm_encoder::BlockType::Empty),
                    // Not found, return false
                    Instruction::I32Const(0),
                    Instruction::Return,
                Instruction::End,
                
                // Load element at current index
                Instruction::LocalGet(0),
                Instruction::I32Const(12), // data offset
                Instruction::LocalGet(3),
                Instruction::I32Const(4), // element size
                Instruction::I32Mul,
                Instruction::I32Add,
                Instruction::I32Load(MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                }),
                
                // Compare with target value
                Instruction::LocalGet(1),
                Instruction::I32Eq,
                Instruction::If(wasm_encoder::BlockType::Empty),
                    // Found, return true
                    Instruction::I32Const(1),
                    Instruction::Return,
                Instruction::End,
                
                // Increment index
                Instruction::LocalGet(3),
                Instruction::I32Const(1),
                Instruction::I32Add,
                Instruction::LocalSet(3),
                
                // Continue loop
                Instruction::Br(0),
            Instruction::End,
            
            // Not found
            Instruction::I32Const(0),
        ]
    }

    /// Generate WebAssembly instructions for List.size operation
    fn generate_list_size(&self) -> Vec<Instruction> {
        vec![
            // Get list pointer (arg 0)
            Instruction::LocalGet(0),
            // Load size field
            Instruction::I32Load(MemArg {
                offset: 0, // size field
                align: 2,
                memory_index: 0,
            }),
        ]
    }

    /// Generate WebAssembly instructions for List.setBehavior operation
    fn generate_set_behavior(&self) -> Vec<Instruction> {
        vec![
            // Get list pointer (arg 0) and behavior string (arg 1)
            Instruction::LocalGet(0),
            Instruction::LocalGet(1),
            
            // Parse behavior string and convert to behavior enum value
            // This is a simplified implementation - would need proper string comparison
            Instruction::LocalGet(1),
            Instruction::Call(1), // Call helper function to parse behavior string
            
            // Store behavior in list structure
            Instruction::LocalGet(0),
            Instruction::I32Store(MemArg {
                offset: 8, // behavior field offset
                align: 2,
                memory_index: 0,
            }),
        ]
    }

    /// Generate WebAssembly instructions for List.getBehavior operation
    fn generate_get_behavior(&self) -> Vec<Instruction> {
        vec![
            // Get list pointer (arg 0)
            Instruction::LocalGet(0),
            // Load behavior field
            Instruction::I32Load(MemArg {
                offset: 8, // behavior field
                align: 2,
                memory_index: 0,
            }),
            // Convert behavior enum to string (simplified)
            Instruction::Call(2), // Call helper function to convert behavior to string
        ]
    }
}

/// Helper function to parse behavior string
pub fn parse_behavior_string(behavior_str: &str) -> ListBehavior {
    match behavior_str {
        "line" => ListBehavior::Line,
        "pile" => ListBehavior::Pile,
        "unique" => ListBehavior::Unique,
        "line-pile" | "linepile" => ListBehavior::LinePile,
        "line-unique" | "lineunique" => ListBehavior::LineUnique,
        "pile-unique" | "pileunique" => ListBehavior::PileUnique,
        "line-unique-pile" | "lineuniuepile" => ListBehavior::LineUniquePile,
        _ => ListBehavior::Default,
    }
}

/// Helper function to convert behavior to string
pub fn behavior_to_string(behavior: ListBehavior) -> &'static str {
    match behavior {
        ListBehavior::Default => "default",
        ListBehavior::Line => "line",
        ListBehavior::Pile => "pile",
        ListBehavior::Unique => "unique",
        ListBehavior::LinePile => "line-pile",
        ListBehavior::LineUnique => "line-unique",
        ListBehavior::PileUnique => "pile-unique",
        ListBehavior::LineUniquePile => "line-unique-pile",
    }
}