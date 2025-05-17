// A standalone test file for memory management that can be run independently
// To run: rustc memory_standalone_test.rs && ./memory_standalone_test

#[derive(Debug, Clone)]
struct MemoryBlock {
    address: usize,
    size: usize,
    is_free: bool,
    type_id: u32,
}

#[derive(Debug)]
struct MemoryManager {
    data: Vec<u8>,
    blocks: Vec<MemoryBlock>,
    heap_start: usize,
    max_size: Option<usize>,
    total_allocated: usize,
}

// Type ID constants
const INTEGER_TYPE_ID: u32 = 1;
const FLOAT_TYPE_ID: u32 = 2;
const STRING_TYPE_ID: u32 = 3;
const ARRAY_TYPE_ID: u32 = 4;
const MATRIX_TYPE_ID: u32 = 5;

impl MemoryManager {
    pub fn new(initial_pages: u32, max_pages: Option<u32>) -> Self {
        let page_size = 65536;
        let heap_start = 1024; // Start heap at 1024 bytes
        let size = initial_pages as usize * page_size;
        let max_size = max_pages.map(|pages| pages as usize * page_size);
        
        Self {
            data: vec![0; size],
            blocks: Vec::new(),
            heap_start,
            max_size,
            total_allocated: 0,
        }
    }
    
    pub fn size(&self) -> usize {
        self.data.len()
    }
    
    pub fn allocate(&mut self, size: usize, type_id: u32) -> Result<usize, String> {
        if size == 0 {
            return Err("Cannot allocate 0 bytes".to_string());
        }
        
        // Align size to 8 bytes
        let aligned_size = (size + 7) & !7;
        
        // Check if we have a free block that's large enough
        for i in 0..self.blocks.len() {
            let block = &self.blocks[i];
            if block.is_free && block.size >= aligned_size {
                let address = block.address;
                let old_size = block.size;
                
                // Update block info
                self.blocks[i].is_free = false;
                self.blocks[i].size = aligned_size;
                self.blocks[i].type_id = type_id;
                
                // If the remaining space is large enough for a new block, split it
                if old_size >= aligned_size + 16 {
                    let new_block = MemoryBlock {
                        address: address + aligned_size,
                        size: old_size - aligned_size,
                        is_free: true,
                        type_id: 0,
                    };
                    self.blocks.push(new_block);
                }
                
                self.total_allocated += aligned_size;
                return Ok(address);
            }
        }
        
        // No suitable free block found, allocate at the end
        let address = if self.blocks.is_empty() {
            self.heap_start
        } else {
            let last_block = &self.blocks[self.blocks.len() - 1];
            last_block.address + last_block.size
        };
        
        // Check if we have enough memory
        if address + aligned_size > self.data.len() {
            if let Some(max_size) = self.max_size {
                if address + aligned_size > max_size {
                    return Err("Memory limit exceeded".to_string());
                }
            }
            
            // Grow memory
            let new_size = (address + aligned_size).max(self.data.len() * 2);
            self.data.resize(new_size, 0);
        }
        
        // Create new block
        let block = MemoryBlock {
            address,
            size: aligned_size,
            is_free: false,
            type_id,
        };
        
        self.blocks.push(block);
        self.total_allocated += aligned_size;
        
        Ok(address)
    }
    
    pub fn release(&mut self, address: usize) -> Result<(), String> {
        let block_index = self.blocks.iter().position(|block| block.address == address);
        
        if let Some(index) = block_index {
            if self.blocks[index].is_free {
                return Err("Double free detected".to_string());
            }
            
            // Mark block as free
            self.blocks[index].is_free = true;
            self.total_allocated -= self.blocks[index].size;
            
            // Merge adjacent free blocks
            self.merge_free_blocks();
            
            Ok(())
        } else {
            Err("Invalid address for release".to_string())
        }
    }
    
    fn merge_free_blocks(&mut self) {
        // Sort blocks by address
        self.blocks.sort_by_key(|block| block.address);
        
        let mut i = 0;
        while i < self.blocks.len() - 1 {
            if self.blocks[i].is_free && self.blocks[i + 1].is_free {
                // Merge two adjacent free blocks
                self.blocks[i].size += self.blocks[i + 1].size;
                self.blocks.remove(i + 1);
            } else {
                i += 1;
            }
        }
    }
}

impl Clone for MemoryManager {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            blocks: self.blocks.clone(),
            heap_start: self.heap_start,
            max_size: self.max_size,
            total_allocated: self.total_allocated,
        }
    }
}

// Test functions
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
    
    println!("Memory block clone test passed!");
}

fn test_memory_manager_new() {
    let manager = MemoryManager::new(16, Some(1024));
    assert!(manager.size() > 0);
    
    println!("Memory manager new test passed!");
}

fn test_memory_manager_allocate() {
    let mut manager = MemoryManager::new(16, Some(1024));
    
    // Allocate memory
    let ptr = manager.allocate(100, INTEGER_TYPE_ID).unwrap();
    assert!(ptr >= 1024); // Should be allocated beyond the heap start
    
    println!("Memory manager allocate test passed!");
}

fn test_memory_manager_release() {
    let mut manager = MemoryManager::new(16, Some(1024));
    
    // Allocate memory
    let ptr = manager.allocate(100, INTEGER_TYPE_ID).unwrap();
    
    // Release memory
    manager.release(ptr).unwrap();
    
    // The memory should now be free, we can allocate at the same location
    let new_ptr = manager.allocate(100, INTEGER_TYPE_ID).unwrap();
    assert_eq!(ptr, new_ptr);
    
    println!("Memory manager release test passed!");
}

fn test_allocate_with_types() {
    let mut manager = MemoryManager::new(16, Some(1024));
    
    // Allocate with type IDs
    let int_ptr = manager.allocate(4, INTEGER_TYPE_ID).unwrap();
    let float_ptr = manager.allocate(8, FLOAT_TYPE_ID).unwrap();
    let string_ptr = manager.allocate(16, STRING_TYPE_ID).unwrap();
    let array_ptr = manager.allocate(20, ARRAY_TYPE_ID).unwrap();
    let matrix_ptr = manager.allocate(24, MATRIX_TYPE_ID).unwrap();
    
    // Verify all allocations succeeded
    assert!(int_ptr > 0);
    assert!(float_ptr > 0);
    assert!(string_ptr > 0);
    assert!(array_ptr > 0);
    assert!(matrix_ptr > 0);
    
    println!("Allocate with types test passed!");
}

fn main() {
    test_memory_block_clone();
    test_memory_manager_new();
    test_memory_manager_allocate();
    test_memory_manager_release();
    test_allocate_with_types();
    
    println!("All memory management tests passed!");
} 