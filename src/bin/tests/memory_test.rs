use std::cell::RefCell;
use std::rc::Rc;

// Constants for type IDs
const INTEGER_TYPE_ID: u32 = 1;
const FLOAT_TYPE_ID: u32 = 2;
const STRING_TYPE_ID: u32 = 3;
const ARRAY_TYPE_ID: u32 = 4;
const MATRIX_TYPE_ID: u32 = 5;

#[derive(Debug, Clone)]
struct MemoryBlock {
    address: usize,
    size: usize,
    is_free: bool,
    type_id: u32,
}

#[derive(Debug, Clone)]
struct MemoryManager {
    data: Vec<u8>,
    blocks: Vec<MemoryBlock>,
    heap_start: usize,
    max_size: Option<usize>,
    total_allocated: usize,
}

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
    
    pub fn store_i32(&mut self, address: usize, value: i32) -> Result<(), String> {
        if address + 4 > self.data.len() {
            return Err("Memory access out of bounds".to_string());
        }
        
        let bytes = value.to_le_bytes();
        self.data[address..address + 4].copy_from_slice(&bytes);
        
        Ok(())
    }
    
    pub fn store_u8(&mut self, address: usize, value: u8) -> Result<(), String> {
        if address >= self.data.len() {
            return Err("Memory access out of bounds".to_string());
        }
        
        self.data[address] = value;
        
        Ok(())
    }
    
    pub fn get_type_id(&self, address: usize) -> Result<u32, String> {
        let block = self.blocks.iter().find(|block| block.address == address);
        
        if let Some(block) = block {
            Ok(block.type_id)
        } else {
            Err("Invalid address for get_type_id".to_string())
        }
    }
}

fn main() {
    println!("Starting memory management test...");
    
    // Test 1: Basic memory allocation and deallocation
    let mut memory = MemoryManager::new(16, Some(1024));
    let ptr1 = memory.allocate(100, INTEGER_TYPE_ID).unwrap();
    println!("Allocated 100 bytes for INTEGER_TYPE_ID at address: {}", ptr1);
    assert!(ptr1 >= 1024);
    
    // Test 2: Check type ID
    let type_id = memory.get_type_id(ptr1).unwrap();
    println!("Type ID for ptr1: {}", type_id);
    assert_eq!(type_id, INTEGER_TYPE_ID);
    
    // Test 3: Release memory
    memory.release(ptr1).unwrap();
    println!("Released memory at address: {}", ptr1);
    
    // Test 4: Re-allocate and verify it uses the same location
    let ptr2 = memory.allocate(100, FLOAT_TYPE_ID).unwrap();
    println!("Re-allocated 100 bytes for FLOAT_TYPE_ID at address: {}", ptr2);
    assert_eq!(ptr1, ptr2);
    
    // Test 5: Test memory block clone
    let mut memory_clone = memory.clone();
    let ptr3 = memory_clone.allocate(50, STRING_TYPE_ID).unwrap();
    println!("Allocated 50 bytes for STRING_TYPE_ID in cloned memory at address: {}", ptr3);
    assert!(ptr3 > ptr2);
    
    // Test 6: Different type IDs
    let int_ptr = memory.allocate(4, INTEGER_TYPE_ID).unwrap();
    let float_ptr = memory.allocate(8, FLOAT_TYPE_ID).unwrap();
    let string_ptr = memory.allocate(16, STRING_TYPE_ID).unwrap();
    let array_ptr = memory.allocate(20, ARRAY_TYPE_ID).unwrap();
    let matrix_ptr = memory.allocate(24, MATRIX_TYPE_ID).unwrap();
    
    println!("Allocated memory for different types:");
    println!("INTEGER_TYPE_ID at: {}", int_ptr);
    println!("FLOAT_TYPE_ID at: {}", float_ptr);
    println!("STRING_TYPE_ID at: {}", string_ptr);
    println!("ARRAY_TYPE_ID at: {}", array_ptr);
    println!("MATRIX_TYPE_ID at: {}", matrix_ptr);
    
    // Verify all type IDs
    assert_eq!(memory.get_type_id(int_ptr).unwrap(), INTEGER_TYPE_ID);
    assert_eq!(memory.get_type_id(float_ptr).unwrap(), FLOAT_TYPE_ID);
    assert_eq!(memory.get_type_id(string_ptr).unwrap(), STRING_TYPE_ID);
    assert_eq!(memory.get_type_id(array_ptr).unwrap(), ARRAY_TYPE_ID);
    assert_eq!(memory.get_type_id(matrix_ptr).unwrap(), MATRIX_TYPE_ID);
    
    // Test 7: Release and allocate pattern
    memory.release(int_ptr).unwrap();
    memory.release(float_ptr).unwrap();
    let new_ptr = memory.allocate(10, INTEGER_TYPE_ID).unwrap();
    println!("Released int_ptr and float_ptr, then allocated 10 bytes at: {}", new_ptr);
    assert!(new_ptr == int_ptr || new_ptr == float_ptr);
    
    println!("All memory management tests passed!");
} 