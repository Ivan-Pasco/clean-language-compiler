// Core standard library modules
pub mod numeric_ops;
pub mod string_ops;
pub mod array_ops;
pub mod matrix_ops;
pub mod type_conv;
pub mod memory;
pub mod basic_ops;
pub mod error;

// Re-exports for convenience
pub use numeric_ops::NumericOperations;
pub use string_ops::{StringOperations, StringManager};
pub use array_ops::ArrayManager;
pub use matrix_ops::MatrixOperations;
pub use type_conv::TypeConvOperations;
pub use memory::MemoryManager;
pub use basic_ops::basic_ops::*;
pub use error::StdlibError;

use crate::error::CompilerError;
use crate::codegen::{CodeGenerator, HEAP_START};
use crate::types::WasmType;
use wasm_encoder::Instruction;

/// Standard library implementation for the Clean Language
pub struct StandardLibrary {
    string_ops: StringOperations,
    numeric_ops: NumericOperations,
    matrix_ops: MatrixOperations,
    type_conv: TypeConvOperations,
}

impl StandardLibrary {
    pub fn new() -> Self {
        Self {
            string_ops: StringOperations::new(HEAP_START),
            numeric_ops: NumericOperations::new(),
            matrix_ops: MatrixOperations::new(),
            type_conv: TypeConvOperations::new(HEAP_START),
        }
    }

    pub fn register_functions(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        self.string_ops.register_functions(codegen)?;
        self.numeric_ops.register_functions(codegen)?;
        self.matrix_ops.register_functions(codegen)?;
        self.type_conv.register_functions(codegen)?;
        Ok(())
    }
}

/// Runtime environment for Clean Language execution
pub struct Runtime {
    pub memory: MemoryManager,
    pub strings: StringManager,
    pub arrays: ArrayManager,
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            memory: MemoryManager::new(16, Some(HEAP_START as u32)),
            strings: StringManager::new(MemoryManager::new(16, Some(HEAP_START as u32))),
            arrays: ArrayManager::new(MemoryManager::new(16, Some(HEAP_START as u32))),
        }
    }

    pub fn reset(&mut self) {
        self.memory = MemoryManager::new(16, Some(HEAP_START as u32));
        self.strings = StringManager::new(MemoryManager::new(16, Some(HEAP_START as u32)));
        self.arrays = ArrayManager::new(MemoryManager::new(16, Some(HEAP_START as u32)));
    }
}

/// Simplified standard library interface
pub struct StdLib {
    pub runtime: Runtime,
    pub numeric_ops: NumericOperations,
    pub type_conv: TypeConvOperations,
}

impl StdLib {
    pub fn new() -> Self {
        Self {
            runtime: Runtime::new(),
            numeric_ops: NumericOperations::new(),
            type_conv: TypeConvOperations::new(HEAP_START),
        }
    }

    pub fn reset(&mut self) {
        self.runtime.reset();
    }

    pub fn register_functions(&mut self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        self.runtime.strings.register_functions(codegen)?;
        self.runtime.arrays.register_functions(codegen)?;
        self.numeric_ops.register_functions(codegen)?;
        self.type_conv.register_functions(codegen)?;
        Ok(())
    }

    pub fn allocate_string(&mut self, data: &[u8]) -> Result<usize, CompilerError> {
        // Simplified string allocation
        self.runtime.memory.allocate(data.len() + 4, 3)
            .map_err(|e| CompilerError::runtime_error(&format!("String allocation failed: {}", e), None, None))
    }

    pub fn get_string_from_memory(&self, _ptr: usize) -> Result<String, CompilerError> {
        // Simplified string retrieval - return empty string for now
        Ok(String::new())
    }
}

/// Helper function to register stdlib functions
pub(crate) fn register_stdlib_function(
    codegen: &mut CodeGenerator,
    name: &str, 
    params: &[WasmType], 
    return_type: Option<WasmType>, 
    instructions: Vec<Instruction>
) -> Result<u32, CompilerError> {
    codegen.register_function(name, params, return_type, &instructions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stdlib_creation() {
        let _stdlib = StdLib::new();
        assert!(true); // Basic test to ensure creation works
    }

    #[test]
    fn test_runtime_creation() {
        let _runtime = Runtime::new();
        assert!(true); // Basic test to ensure creation works
    }

    #[test]
    fn test_memory_allocation() -> Result<(), CompilerError> {
        let mut memory = MemoryManager::new(16, Some(1024));
        assert!(memory.allocate(100, 1).is_ok());
        Ok(())
    }
} 