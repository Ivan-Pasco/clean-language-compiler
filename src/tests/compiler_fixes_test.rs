use crate::parser::CleanParser;
use crate::semantic::SemanticAnalyzer;
use crate::codegen::CodeGenerator;
use crate::ast::{Program, Function, Statement, Expression, Type, Value, SourceLocation};
use crate::stdlib::memory::{MemoryManager, MemoryBlock};
use crate::stdlib::error::StdlibError;
use crate::error::CompilerError;
use crate::types::WasmType;
use wasm_encoder::{FuncType, ValType};

#[test]
fn test_memory_manager_clone() {
    // Test the Clone implementation for MemoryManager
    let original = MemoryManager::new(16, Some(1024));
    let cloned = original.clone();
    
    // Verify they start with same state
    assert_eq!(original.allocate(100).unwrap(), cloned.allocate(100).unwrap());
}

#[test]
fn test_memory_block_clone() {
    // Test the Clone implementation for MemoryBlock
    let block = MemoryBlock {
        address: 1024,
        size: 100,
        is_free: false,
        type_id: 1,
    };
    
    let cloned = block.clone();
    
    // Verify clone has the same properties
    assert_eq!(block.address, cloned.address);
    assert_eq!(block.size, cloned.size);
    assert_eq!(block.is_free, cloned.is_free);
    assert_eq!(block.type_id, cloned.type_id);
}

#[test]
fn test_memory_manager_release() -> Result<(), CompilerError> {
    // Test the release method for proper memory cleanup
    let mut manager = MemoryManager::new(16, Some(1024));
    
    // Allocate some memory
    let ptr = manager.allocate(100).unwrap();
    
    // Release the memory
    manager.release(ptr)?;
    
    // The released block should now be free
    // Allocating the same size should reuse the same block
    let new_ptr = manager.allocate(100).unwrap();
    assert_eq!(ptr, new_ptr);
    
    Ok(())
}

#[test]
fn test_typed_allocation() -> Result<(), CompilerError> {
    // Test allocating with type IDs
    let mut manager = MemoryManager::new(16, Some(1024));
    
    // Allocate with different type IDs
    let int_ptr = manager.allocate(4, crate::codegen::INTEGER_TYPE_ID)?;
    let float_ptr = manager.allocate(8, crate::codegen::FLOAT_TYPE_ID)?;
    let string_ptr = manager.allocate(16, crate::codegen::STRING_TYPE_ID)?;
    let array_ptr = manager.allocate(20, crate::codegen::ARRAY_TYPE_ID)?;
    let matrix_ptr = manager.allocate(24, crate::codegen::MATRIX_TYPE_ID)?;
    
    // Ensure all allocations succeeded
    assert!(int_ptr > 0);
    assert!(float_ptr > 0);
    assert!(string_ptr > 0);
    assert!(array_ptr > 0);
    assert!(matrix_ptr > 0);
    
    // Each allocation should be at a different address
    let addresses = vec![int_ptr, float_ptr, string_ptr, array_ptr, matrix_ptr];
    let unique_addresses: std::collections::HashSet<_> = addresses.iter().collect();
    assert_eq!(addresses.len(), unique_addresses.len());
    
    Ok(())
}

#[test]
fn test_wasm_type_conversion() {
    // Test conversion between WasmType and (integer, ValType) tuples
    let wasm_types = vec![
        WasmType::I32,
        WasmType::I64,
        WasmType::F32,
        WasmType::F64,
    ];
    
    // Convert to tuples
    let tuples = crate::types::wasm_types_to_tuples(&wasm_types);
    
    // Convert back to WasmType
    let converted = crate::types::tuples_to_wasm_types(&tuples);
    
    // Check that the conversion preserves the types
    assert_eq!(wasm_types, converted);
}

#[test]
fn test_codegen_finish() -> Result<(), CompilerError> {
    // Test the finish method on CodeGenerator
    let mut codegen = CodeGenerator::new();
    
    // Add a simple dummy function
    let func_type = FuncType::new(vec![ValType::I32], vec![ValType::I32]);
    let func_id = codegen.module.funcs.len() as u32;
    codegen.module.add_func_type(func_type);
    
    // Call finish to generate the binary
    let wasm_binary = codegen.finish()?;
    
    // Verify that we got a non-empty WebAssembly binary
    assert!(!wasm_binary.is_empty());
    
    Ok(())
}

#[test]
fn test_stdlib_error_conversion() {
    // Test converting from MemoryAccessError to CompilerError
    use crate::stdlib::memory::MemoryAccessError;
    
    let error = MemoryAccessError::InvalidPointer { ptr: 123 };
    let compiler_error: CompilerError = error.into();
    
    // Verify the error was converted
    assert!(compiler_error.to_string().contains("Invalid pointer"));
}

#[test]
fn test_simple_program_compilation() -> Result<(), CompilerError> {
    // Test compiling a simple program using the fixed compiler components
    let source = r#"
        start() {
            let x = 5
            let y = 10
            let z = x + y
            printl z
        }
    "#;
    
    // Parse the program
    let program = CleanParser::parse_program(source)?;
    
    // Type check the program
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.check(&program)?;
    
    // Generate code
    let mut codegen = CodeGenerator::new();
    let wasm = codegen.generate(&program)?;
    
    // Verify we got a WebAssembly binary
    assert!(!wasm.is_empty());
    
    Ok(())
} 