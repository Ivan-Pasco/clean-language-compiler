use crate::stdlib::memory::{MemoryManager, MemoryBlock};
use crate::error::CompilerError;
use crate::codegen::{INTEGER_TYPE_ID, FLOAT_TYPE_ID, STRING_TYPE_ID, ARRAY_TYPE_ID, MATRIX_TYPE_ID};

#[test]
fn test_memory_manager_new() {
    let manager = MemoryManager::new(16, Some(1024));
    assert!(manager.size() > 0);
}

#[test]
fn test_memory_manager_allocate() {
    let mut manager = MemoryManager::new(16, Some(1024));
    
    // Allocate memory
    let ptr = manager.allocate(100, INTEGER_TYPE_ID).unwrap();
    assert!(ptr >= 1024); // Should be allocated beyond the heap start
}

#[test]
fn test_memory_block_clone() {
    let block = MemoryBlock {
        address: 1024,
        size: 100,
        is_free: false,
        type_id: INTEGER_TYPE_ID,
    };
    
    let cloned = block.clone();
    
    assert_eq!(block.address, cloned.address);
    assert_eq!(block.size, cloned.size);
    assert_eq!(block.is_free, cloned.is_free);
    assert_eq!(block.type_id, cloned.type_id);
}

#[test]
fn test_memory_manager_release() {
    let mut manager = MemoryManager::new(16, Some(1024));
    
    // Allocate memory
    let ptr = manager.allocate(100, INTEGER_TYPE_ID).unwrap();
    
    // Release memory
    manager.release(ptr).unwrap();
    
    // The memory should now be free, we can allocate at the same location
    let new_ptr = manager.allocate(100, INTEGER_TYPE_ID).unwrap();
    assert_eq!(ptr, new_ptr);
}

#[test]
fn test_allocate_with_types() -> Result<(), CompilerError> {
    let mut manager = MemoryManager::new(16, Some(1024));
    
    // Allocate with type IDs
    let int_ptr = manager.allocate(4, INTEGER_TYPE_ID)?;
    let float_ptr = manager.allocate(8, FLOAT_TYPE_ID)?;
    let string_ptr = manager.allocate(16, STRING_TYPE_ID)?;
    let array_ptr = manager.allocate(20, ARRAY_TYPE_ID)?;
    let matrix_ptr = manager.allocate(24, MATRIX_TYPE_ID)?;
    
    // Verify all allocations succeeded
    assert!(int_ptr > 0);
    assert!(float_ptr > 0);
    assert!(string_ptr > 0);
    assert!(array_ptr > 0);
    assert!(matrix_ptr > 0);
    
    Ok(())
} 