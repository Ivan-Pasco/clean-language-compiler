//! Module for memory operations during code generation.

use wasm_encoder::{DataSection, ConstExpr, Instruction, Function, ValType, MemArg, BlockType};
use crate::error::{CompilerError, ErrorContext, ErrorType};
use crate::ast::Value;
use crate::types::WasmType;

// Memory constants
pub const PAGE_SIZE: u32 = 65536;
pub const HEADER_SIZE: u32 = 16;  // 16-byte header for memory blocks
pub const MIN_ALLOCATION: u32 = 32;  // Increased for better alignment
pub const HEAP_START: usize = 65536;  // Start heap at 64KB
pub const DEFAULT_ALIGN: u32 = 2;
pub const DEFAULT_OFFSET: u32 = 0;
pub const ALIGNMENT: usize = 8;

// Memory type IDs
pub const INTEGER_TYPE_ID: u32 = 1;
pub const FLOAT_TYPE_ID: u32 = 2;
pub const STRING_TYPE_ID: u32 = 3;
pub const ARRAY_TYPE_ID: u32 = 4;
pub const MATRIX_TYPE_ID: u32 = 5;
pub const OBJECT_TYPE_ID: u32 = 6;
pub const FUNCTION_TYPE_ID: u32 = 7;

/// Represents a memory block
#[derive(Debug, Clone)]
pub struct MemoryBlock {
    pub address: usize,
    pub size: usize,
    pub is_free: bool,
    pub type_id: u32,
    pub ref_count: usize,
}

/// Memory management utilities
pub(crate) struct MemoryUtils {
    data_section: DataSection,
    heap_start: usize,
    current_address: usize,
    memory_blocks: Vec<MemoryBlock>,
    free_blocks: Vec<usize>, // Indices into memory_blocks for free blocks
}

impl MemoryUtils {
    /// Create a new MemoryUtils instance
    pub(crate) fn new(heap_start: usize) -> Self {
        Self {
            data_section: DataSection::new(),
            heap_start,
            current_address: heap_start,
            memory_blocks: Vec::new(),
            free_blocks: Vec::new(),
        }
    }

    /// Add a data segment
    pub(crate) fn add_data_segment(&mut self, offset: u32, data: &[u8]) {
        let offset_expr = ConstExpr::i32_const(offset as i32);
        let data_vec: Vec<u8> = data.iter().copied().collect();
        self.data_section.active(0, &offset_expr, data_vec);
    }

    /// Get the data section
    pub(crate) fn get_data_section(&self) -> &DataSection {
        &self.data_section
    }

    /// Align a size to the required alignment
    fn align_size(size: usize) -> usize {
        (size + ALIGNMENT - 1) & !(ALIGNMENT - 1)
    }

    /// Record memory allocation
    pub(crate) fn record_allocation(&mut self, size: usize, type_id: u32) -> usize {
        let address = self.current_address;
        self.memory_blocks.push(MemoryBlock {
            address,
            size,
            is_free: false,
            type_id,
            ref_count: 1,  // Start with ref count 1
        });
        self.current_address += size;
        address
    }

    /// Find a free block of sufficient size
    fn find_free_block(&self, size: usize) -> Option<usize> {
        for &block_idx in &self.free_blocks {
            let block = &self.memory_blocks[block_idx];
            if block.is_free && block.size >= size {
                return Some(block_idx);
            }
        }
        None
    }

    /// Allocate memory for a block
    pub(crate) fn allocate(&mut self, size: usize, type_id: u32) -> Result<usize, CompilerError> {
        let aligned_size = Self::align_size(size + HEADER_SIZE as usize);
        
        // First, try to find a free block of sufficient size
        if let Some(block_idx) = self.find_free_block(aligned_size) {
            let block = &mut self.memory_blocks[block_idx];
            block.is_free = false;
            block.type_id = type_id;
            block.ref_count = 1;
            
            // Remove from free list
            if let Some(pos) = self.free_blocks.iter().position(|&idx| idx == block_idx) {
                self.free_blocks.remove(pos);
            }
            
            return Ok(block.address + HEADER_SIZE as usize);
        }

        // Check if we have enough memory
        let total_memory = self.heap_start + 1024 * 1024; // Example memory limit of 1MB beyond heap start
        if self.current_address + aligned_size > total_memory {
            return Err(CompilerError::memory_allocation_error(
                "Memory allocation failed: not enough memory",
                aligned_size,
                Some(total_memory - self.current_address),
                None
            ));
        }

        // If no suitable free block was found, allocate new memory
        let address = self.record_allocation(aligned_size, type_id);
        Ok(address + HEADER_SIZE as usize)
    }

    /// Increase reference count for a block
    pub(crate) fn retain(&mut self, address: usize) -> Result<(), CompilerError> {
        let header_address = address - HEADER_SIZE as usize;
        
        // Find the block
        for block in &mut self.memory_blocks {
            if block.address == header_address {
                block.ref_count += 1;
                return Ok(());
            }
        }
        
        Err(CompilerError::runtime_error(
            format!("Attempt to retain invalid memory address: {}", address),
            None,
            None,
        ))
    }

    /// Decrease reference count for a block
    pub(crate) fn release(&mut self, address: usize) -> Result<(), CompilerError> {
        let header_address = address - HEADER_SIZE as usize;
        
        // Find the block
        for (i, block) in self.memory_blocks.iter_mut().enumerate() {
            if block.address == header_address {
                if block.ref_count > 0 {
                    block.ref_count -= 1;
                }
                
                // If reference count is zero, mark as free
                if block.ref_count == 0 {
                    block.is_free = true;
                    self.free_blocks.push(i);
                    
                    // TODO: Coalesce adjacent free blocks
                }
                
                return Ok(());
            }
        }
        
        Err(CompilerError::runtime_error(
            format!("Attempt to release invalid memory address: {}", address),
            None,
            None,
        ))
    }

    /// Generate memory initialization instructions
    pub(crate) fn generate_init_memory(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();

        // Initialize memory manager's heap pointer
        instructions.push(Instruction::I32Const(self.heap_start as i32));
        instructions.push(Instruction::GlobalSet(0)); // Assuming global 0 is the heap pointer

        instructions
    }

    /// Allocates memory for a string and adds a data segment for it
    pub(crate) fn allocate_string(&mut self, s: &str) -> Result<usize, CompilerError> {
        let bytes = s.as_bytes();
        let len = bytes.len();
        
        // Check for very large strings
        if len > 1024 * 1024 {  // 1MB limit for strings
            return Err(CompilerError::memory_allocation_error(
                "String allocation failed: string is too large",
                len + 4,
                None,
                None
            ));
        }
        
        // Allocate memory for the string (length + content)
        let ptr = self.allocate(len + 4, STRING_TYPE_ID)?;
        
        // Create data segment for length
        let len_bytes = (len as u32).to_le_bytes();
        self.add_data_segment((ptr - HEADER_SIZE as usize) as u32, &len_bytes);
        
        // Create data segment for the string content
        self.add_data_segment((ptr + 4 - HEADER_SIZE as usize) as u32, bytes);
        
        Ok(ptr)
    }

    /// Allocates memory for an array and adds a data segment for it
    pub(crate) fn allocate_array(&mut self, elements: &[Value]) -> Result<usize, CompilerError> {
        let element_type = if elements.is_empty() {
            WasmType::I32
        } else {
            match &elements[0] {
                Value::Number(_) => WasmType::F64,
                Value::Integer(_) => WasmType::I32,
                Value::Boolean(_) => WasmType::I32,
                Value::String(_) => WasmType::I32,
                _ => WasmType::I32,
            }
        };
        
        let element_size = element_type.size_in_bytes(); 
        let num_elements = elements.len();
        let total_data_size = num_elements * element_size;

        // Allocate memory for array
        let ptr = self.allocate(total_data_size + 8, ARRAY_TYPE_ID)?;

        // Create header (num_elements + element_type)
        let header_bytes = [
            (num_elements as u32).to_le_bytes(), 
            (element_type.to_id()).to_le_bytes()
        ].concat();
        
        // Add header data segment
        self.add_data_segment((ptr - HEADER_SIZE as usize) as u32, &header_bytes);

        // Create element data
        let mut data_bytes = Vec::with_capacity(total_data_size);
        for element in elements {
            match (element, element_type) {
                (Value::Integer(i), WasmType::I32) => 
                    data_bytes.extend_from_slice(&i.to_le_bytes()),
                (Value::Number(n), WasmType::F64) => 
                    data_bytes.extend_from_slice(&n.to_le_bytes()),
                (Value::Boolean(b), WasmType::I32) => 
                    data_bytes.extend_from_slice(&(if *b { 1i32 } else { 0i32 }).to_le_bytes()),
                _ => return Err(CompilerError::codegen_error(
                    format!("Cannot store value {:?} in array of type {:?}", element, element_type),
                    None, None
                )),
            }
        }
        
        // Add element data segment
        self.add_data_segment((ptr + 8 - HEADER_SIZE as usize) as u32, &data_bytes);

        Ok(ptr)
    }

    /// Allocates memory for a matrix and adds a data segment for it
    pub(crate) fn allocate_matrix(&mut self, rows: &[Vec<f64>]) -> Result<usize, CompilerError> {
        if rows.is_empty() {
            // Allocate minimal matrix
            return self.allocate(12, MATRIX_TYPE_ID);
        }

        let num_rows = rows.len();
        let num_cols = rows[0].len();

        if rows.iter().any(|row| row.len() != num_cols) {
            return Err(CompilerError::codegen_error(
                "All matrix rows must have the same length", 
                None, None
            ));
        }

        let element_type = WasmType::F64;
        let element_size = element_type.size_in_bytes();
        let total_data_size = num_rows * num_cols * element_size;

        // Allocate memory for matrix
        let ptr = self.allocate(total_data_size + 12, MATRIX_TYPE_ID)?;

        // Create header (num_rows + num_cols + element_type)
        let header_bytes = [
            (num_rows as u32).to_le_bytes(), 
            (num_cols as u32).to_le_bytes(), 
            (element_type.to_id()).to_le_bytes()
        ].concat();
        
        // Add header data segment
        self.add_data_segment((ptr - HEADER_SIZE as usize) as u32, &header_bytes);

        // Create element data
        let mut data_bytes = Vec::with_capacity(total_data_size);
        for row in rows {
            for val in row {
                data_bytes.extend_from_slice(&val.to_le_bytes());
            }
        }

        // Add element data segment
        self.add_data_segment((ptr + 12 - HEADER_SIZE as usize) as u32, &data_bytes);

        Ok(ptr)
    }

    /// Generates memory management functions (malloc, retain, release)
    pub(crate) fn generate_memory_functions(&self) -> Vec<(Vec<ValType>, Option<ValType>, Vec<Instruction<'static>>)> {
        let mut functions = Vec::new();
        
        // Generate malloc function
        let malloc_params = vec![ValType::I32, ValType::I32]; // size, type_id
        let malloc_result = Some(ValType::I32); // pointer
        let malloc_body = vec![
            // Parameters:
            // - Local 0: size (i32)
            // - Local 1: type_id (i32)
            
            // Align to 8 bytes
            Instruction::LocalGet(0), // size
            Instruction::I32Const(ALIGNMENT as i32 - 1),
            Instruction::I32Add, 
            Instruction::I32Const(-(ALIGNMENT as i32)),
            Instruction::I32And, // (size + ALIGNMENT - 1) & ~(ALIGNMENT - 1)
            Instruction::LocalSet(0), // aligned_size = ...
            
            // Add header size
            Instruction::LocalGet(0),
            Instruction::I32Const(HEADER_SIZE as i32),
            Instruction::I32Add,
            Instruction::LocalSet(0), // size += HEADER_SIZE
            
            // Get current heap pointer
            Instruction::GlobalGet(0),
            Instruction::LocalSet(2), // ptr = heap_ptr
            
            // Update heap pointer
            Instruction::GlobalGet(0),
            Instruction::LocalGet(0),
            Instruction::I32Add,
            Instruction::GlobalSet(0), // heap_ptr += size
            
            // Store type ID at the beginning of the allocated memory
            Instruction::LocalGet(2), // ptr
            Instruction::LocalGet(1), // type_id
            Instruction::I32Store(MemArg {
                offset: 0,
                align: 2,
                memory_index: 0,
            }),
            
            // Store reference count (1) at offset 4
            Instruction::LocalGet(2), // ptr
            Instruction::I32Const(1), // ref_count = 1
            Instruction::I32Store(MemArg {
                offset: 4,
                align: 2,
                memory_index: 0,
            }),
            
            // Return the pointer (after header)
            Instruction::LocalGet(2),
            Instruction::I32Const(HEADER_SIZE as i32),
            Instruction::I32Add,
        ];
        
        functions.push((malloc_params, malloc_result, malloc_body));
        
        // Generate retain function
        let retain_params = vec![ValType::I32]; // pointer
        let retain_result = None;
        let retain_body = vec![
            // Parameters:
            // - Local 0: pointer (i32)
            
            // Get header pointer
            Instruction::LocalGet(0),
            Instruction::I32Const(HEADER_SIZE as i32),
            Instruction::I32Sub,
            Instruction::LocalSet(1), // header_ptr = ptr - HEADER_SIZE
            
            // Load current reference count
            Instruction::LocalGet(1),
            Instruction::I32Load(MemArg {
                offset: 4,
                align: 2,
                memory_index: 0,
            }),
            Instruction::LocalSet(2), // ref_count = load(header_ptr + 4)
            
            // Increment reference count
            Instruction::LocalGet(2),
            Instruction::I32Const(1),
            Instruction::I32Add,
            Instruction::LocalSet(2), // ref_count++
            
            // Store updated reference count
            Instruction::LocalGet(1),
            Instruction::LocalGet(2),
            Instruction::I32Store(MemArg {
                offset: 4,
                align: 2,
                memory_index: 0,
            }),
        ];
        
        functions.push((retain_params, retain_result, retain_body));
        
        // Generate release function
        let release_params = vec![ValType::I32]; // pointer
        let release_result = None;
        let release_body = vec![
            // Parameters:
            // - Local 0: pointer (i32)
            
            // Get header pointer
            Instruction::LocalGet(0),
            Instruction::I32Const(HEADER_SIZE as i32),
            Instruction::I32Sub,
            Instruction::LocalSet(1), // header_ptr = ptr - HEADER_SIZE
            
            // Load current reference count
            Instruction::LocalGet(1),
            Instruction::I32Load(MemArg {
                offset: 4,
                align: 2,
                memory_index: 0,
            }),
            Instruction::LocalSet(2), // ref_count = load(header_ptr + 4)
            
            // Check if reference count is already 0
            Instruction::LocalGet(2),
            Instruction::I32Const(0),
            Instruction::I32Eq,
            Instruction::If(BlockType::Empty),
            Instruction::Return, // If ref_count is 0, just return
            Instruction::End,
            
            // Decrement reference count
            Instruction::LocalGet(2),
            Instruction::I32Const(1),
            Instruction::I32Sub,
            Instruction::LocalSet(2), // ref_count--
            
            // Store updated reference count
            Instruction::LocalGet(1),
            Instruction::LocalGet(2),
            Instruction::I32Store(MemArg {
                offset: 4,
                align: 2,
                memory_index: 0,
            }),
            
            // For now, we don't implement actual memory freeing in WASM
            // Future: If ref_count becomes 0, add logic to mark block as free
        ];
        
        functions.push((release_params, release_result, release_body));
        
        functions
    }
}

/// Aligns a value up to the nearest multiple of alignment
fn align_up(value: usize, alignment: usize) -> usize {
    (value + alignment - 1) & !(alignment - 1)
} 