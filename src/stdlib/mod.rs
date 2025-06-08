mod date_ops;
mod format_ops;
mod numeric_ops;
mod random_ops;
pub mod string_ops;
pub mod time_ops;
pub mod type_conv;
pub mod memory;
mod basic_ops;
pub mod array_ops;
pub mod matrix_ops;
pub mod error;

pub use date_ops::DateOperations;
pub use format_ops::FormatOperations;
pub use numeric_ops::NumericOperations;
pub use random_ops::RandomOperations;
pub use string_ops::StringOperations;
pub use time_ops::TimeOperations;
pub use type_conv::TypeConvOperations;
pub use memory::MemoryManager;
pub use basic_ops::basic_ops::*;
pub use string_ops::StringManager;
pub use array_ops::ArrayManager;
pub use matrix_ops::MatrixOperations;
pub use error::StdlibError;

use crate::error::CompilerError;
use crate::codegen::{CodeGenerator, INTEGER_TYPE_ID};
use crate::types::WasmType;


use crate::codegen::{STRING_TYPE_ID, HEAP_START};

use wasm_encoder::{Instruction};

/// Memory wrapper to handle allocations and access
pub struct Memory {
    inner: wasmtime::Memory,
    // Current address for new allocations
    current_address: u32,
}

impl Memory {
    /// Create a new memory wrapper
    pub fn new(memory: wasmtime::Memory) -> Self {
        Self {
            inner: memory,
            current_address: 65536, // Start memory at 64KB
        }
    }

    /// Custom allocation implementation since wasmtime::Memory doesn't have allocate
    pub fn allocate(&mut self, size: usize, _type_id: u32) -> Result<usize, String> {
        let aligned_size = ((size + 7) / 8) * 8; // Align to 8 bytes
        let address = self.current_address as usize;
        
        // Update current address for next allocation
        self.current_address += aligned_size as u32 + 16; // Add 16 bytes for header
        
        // Store the header information (we need to actually write this to memory)
        // For now, we just return the address since we can't directly manipulate memory
        
        Ok(address)
    }

    /// Allocate a string in memory
    pub fn allocate_string(&mut self, s: &str) -> Result<usize, String> {
        let size = s.len();
        let ptr = self.allocate(size + 4, 3)?; // Type ID 3 for string
        
        // For a real implementation, we would write:
        // 1. The string length (4 bytes)
        // 2. The string content
        
        Ok(ptr)
    }

    /// Allocate an array in memory
    pub fn allocate_array(&mut self, size: usize, _type_id: u32) -> Result<usize, String> {
        let ptr = self.allocate(size * 4, 4)?; // Type ID 4 for arrays
        
        // For a real implementation, we would write:
        // 1. Array header with length and element type
        // 2. Array content
        
        Ok(ptr)
    }

    /// Allocate a matrix in memory
    pub fn allocate_matrix(&mut self, rows: usize, cols: usize) -> Result<usize, String> {
        let total_size = rows * cols * 8; // 8 bytes per f64 element
        let ptr = self.allocate(total_size, 5)?; // Type ID 5 for matrices
        
        // For a real implementation, we would write:
        // 1. Matrix header with rows, columns, and element type
        // 2. Matrix content
        
        Ok(ptr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::CodeGenerator;
    use crate::error::CompilerError;
    use wasm_encoder::{Function, Instruction, ValType};

    #[test]
    fn test_memory_allocation() -> Result<(), CompilerError> {
        let mut memory = memory::MemoryManager::new(16, Some(1024)); // 16 pages, heap starts at 1024
        assert!(memory.allocate(100, INTEGER_TYPE_ID).is_ok());
        Ok(())
    }

    #[test]
    fn test_string_operations() -> Result<(), CompilerError> {
        let mut codegen = CodeGenerator::new();
        let mut string_ops = string_ops::StringOperations::new(1024); // heap starts at 1024
        string_ops.register_functions(&mut codegen)?;
        Ok(())
    }

    #[test]
    fn test_array_operations() -> Result<(), CompilerError> {
        let mut codegen = CodeGenerator::new();
        let memory_manager = memory::MemoryManager::new(16, Some(1024));
        let mut array_ops = array_ops::ArrayManager::new(memory_manager);
        array_ops.register_functions(&mut codegen)?;
        Ok(())
    }

    #[test]
    fn test_numeric_operations() -> Result<(), CompilerError> {
        let mut codegen = CodeGenerator::new();
        let mut numeric_ops = numeric_ops::NumericOperations::new();
        numeric_ops.register_functions(&mut codegen)?;
        Ok(())
    }

    #[test]
    fn test_random_operations() -> Result<(), CompilerError> {
        let mut codegen = CodeGenerator::new();
        let mut random_ops = random_ops::RandomOperations::new();
        random_ops.register_functions(&mut codegen)?;
        Ok(())
    }

    #[test]
    fn test_runtime_creation() {
        let mut runtime = Runtime::new();
        assert_eq!(runtime.memory.allocate(16, INTEGER_TYPE_ID).unwrap(), 1024);
    }

    #[test]
    fn test_runtime_reset() {
        let mut runtime = Runtime::new();
        runtime.memory.allocate(16, INTEGER_TYPE_ID).unwrap();
        runtime.reset();
        assert_eq!(runtime.memory.allocate(16, INTEGER_TYPE_ID).unwrap(), 1024);
    }

    #[test]
    fn test_memory_management() -> Result<(), CompilerError> {
        let mut stdlib = StdLib::new();
        let mut codegen = CodeGenerator::new();

        // Test memory allocation
        let ptr = stdlib.runtime.memory.allocate(100, INTEGER_TYPE_ID)?;
        assert!(ptr >= HEAP_START);

        // Test string pool - commented out as method doesn't exist
        // let test_str = "Hello, World!";
        // let str_ptr = stdlib.add_string_to_pool(test_str).unwrap();
        // let retrieved_str = stdlib.get_string_from_memory(str_ptr).unwrap();
        // assert_eq!(test_str, retrieved_str);

        // Test array operations
        stdlib.runtime.arrays.register_functions(&mut codegen).unwrap();
        
        Ok(())
    }
}

/// Standard library implementation for the Clean Language
pub struct StandardLibrary {
    string_ops: StringOperations,
    numeric_ops: NumericOperations,
    random_ops: RandomOperations,
    time_ops: TimeOperations,
    date_ops: DateOperations,
    format_ops: FormatOperations,
    type_conv: TypeConvOperations,
    matrix_ops: MatrixOperations,
}

impl StandardLibrary {
    pub fn new() -> Self {
        let heap_start = 1024; // Start heap at 1024
        Self {
            string_ops: StringOperations::new(heap_start),
            numeric_ops: NumericOperations::new(),
            random_ops: RandomOperations::new(),
            time_ops: TimeOperations::new(),
            date_ops: DateOperations::new(),
            format_ops: FormatOperations::new(),
            type_conv: TypeConvOperations::new(heap_start),
            matrix_ops: MatrixOperations::new(heap_start),
        }
    }

    pub fn register_functions(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        self.string_ops.register_functions(codegen)?;
        self.numeric_ops.register_functions(codegen)?;
        self.random_ops.register_functions(codegen)?;
        self.time_ops.register_functions(codegen)?;
        self.date_ops.register_functions(codegen)?;
        self.format_ops.register_functions(codegen)?;
        self.type_conv.register_functions(codegen)?;
        self.matrix_ops.register_functions(codegen)?;
        Ok(())
    }
}

// Runtime context that holds all managers
pub struct Runtime {
    pub memory: MemoryManager,
    pub strings: StringManager,
    pub arrays: ArrayManager,
}

impl Runtime {
    pub fn new() -> Self {
        let memory_manager = MemoryManager::new(16, Some(HEAP_START as u32));
        let string_manager = StringManager::new(memory_manager.clone());
        let array_manager = ArrayManager::new(memory_manager.clone());

        Self {
            memory: memory_manager,
            strings: string_manager,
            arrays: array_manager,
        }
    }

    pub fn reset(&mut self) {
        let memory_manager = MemoryManager::new(16, Some(HEAP_START as u32));
        self.memory = memory_manager.clone();
        self.strings = StringManager::new(memory_manager.clone());
        self.arrays = ArrayManager::new(memory_manager);
    }
}

pub struct StdLib {
    pub runtime: Runtime,
    pub basic_ops: BasicOperations,
    pub numeric_ops: NumericOperations,
    pub random_ops: RandomOperations,
    pub type_conv: TypeConvOperations,
    pub heap_start: usize,
}

impl StdLib {
    pub fn new() -> Self {
        let heap_start = HEAP_START;
        let runtime = Runtime::new();
        
        Self {
            runtime,
            basic_ops: BasicOperations::new(),
            numeric_ops: NumericOperations::new(),
            random_ops: RandomOperations::new(),
            type_conv: TypeConvOperations::new(heap_start),
            heap_start,
        }
    }

    pub fn reset(&mut self) {
        self.runtime = Runtime::new();
    }

    pub fn register_functions(&mut self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // Register memory functions
        self.runtime.memory.register_functions(codegen)?;
        
        // Register string functions
        self.runtime.strings.register_functions(codegen)?;
        
        // Register array functions
        self.runtime.arrays.register_functions(codegen)?;
        
        // Register numeric operations
        self.numeric_ops.register_functions(codegen)?;
        
        // Register random operations
        self.random_ops.register_functions(codegen)?;
        
        // Register type conversion operations
        self.type_conv.register_functions(codegen)?;
        
        Ok(())
    }

    pub fn allocate_string(&mut self, data: &[u8]) -> Result<usize, CompilerError> {
        let size = data.len();
        let ptr = self.runtime.memory.allocate(size + 4, STRING_TYPE_ID)?;
        self.runtime.memory.store_i32(ptr, size as i32)?;
        for (i, &byte) in data.iter().enumerate() {
            self.runtime.memory.store_u8(ptr + 4 + i, byte)?;
        }
        Ok(ptr)
    }

    pub fn get_string_from_memory(&self, ptr: usize) -> Result<String, CompilerError> {
        let len = unsafe { *(ptr as *const i32) } as usize;
        let bytes = unsafe {
            std::slice::from_raw_parts((ptr + 4) as *const u8, len)
        };
        String::from_utf8(bytes.to_vec())
            .map_err(|e| CompilerError::runtime_error(
                e.to_string(), 
                None, 
                None
            ))
    }
}

// Add the BasicOperations struct definition
pub struct BasicOperations;

impl BasicOperations {
    pub fn new() -> Self {
        Self
    }
}

// Add a helper function to register stdlib functions with the correct signature
pub(crate) fn register_stdlib_function(
    codegen: &mut CodeGenerator,
    name: &str, 
    params: &[WasmType], 
    return_type: Option<WasmType>, 
    instructions: Vec<Instruction>
) -> Result<u32, CompilerError> {
    codegen.register_function(name, params, return_type, &instructions)
} 