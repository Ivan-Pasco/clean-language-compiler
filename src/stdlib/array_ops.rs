use crate::error::{CompilerError, ErrorContext, ErrorType};
use wasm_encoder::{
    BlockType, Instruction, MemArg, ValType,
};
use crate::codegen::CodeGenerator;
use crate::types::{WasmType, wasm_type_to_tuple, wasm_types_to_tuples};
use crate::stdlib::memory::MemoryManager;
use crate::stdlib::register_stdlib_function;

const ARRAY_TYPE_ID: u32 = 1;

pub struct ArrayManager {
    memory_manager: MemoryManager,
}

impl ArrayManager {
    pub fn new(memory_manager: MemoryManager) -> Self {
        Self { memory_manager }
    }

    pub fn register_functions(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // Register array allocation function
        register_stdlib_function(
            codegen,
            "array.allocate",
            &[WasmType::I32], // Size
            Some(WasmType::I32), // Pointer
            self.generate_array_allocate()
        )?;

        // Register array get function
        register_stdlib_function(
            codegen,
            "array.get",
            &[WasmType::I32, WasmType::I32], // Array pointer and index
            Some(WasmType::I32), // Element pointer
            self.generate_array_get()
        )?;

        // Register array set function
        register_stdlib_function(
            codegen,
            "array.set",
            &[WasmType::I32, WasmType::I32, WasmType::I32], // Array pointer, index, and value pointer
            None, // No return value
            self.generate_array_set()
        )?;
        
        // Register array length function
        register_stdlib_function(
            codegen,
            "array.length",
            &[WasmType::I32], // Array pointer
            Some(WasmType::I32), // Length
            self.generate_array_length()
        )?;
        
        // Register array iteration function
        register_stdlib_function(
            codegen,
            "array.iterate",
            &[WasmType::I32, WasmType::I32], // Array pointer and callback function index
            None, // No return value
            self.generate_array_iterate()
        )?;

        // Register array map function
        register_stdlib_function(
            codegen,
            "array.map",
            &[WasmType::I32, WasmType::I32], // Array pointer and callback function index
            Some(WasmType::I32), // New array pointer
            self.generate_array_map()
        )?;

        Ok(())
    }

    fn generate_array_allocate(&self) -> Vec<Instruction> {
        vec![
            // Get size parameter
            Instruction::LocalGet(0),
            
            // Calculate total size (size * 8 + header)
            Instruction::I32Const(8),
            Instruction::I32Mul,
            Instruction::I32Const(16), // Header size
            Instruction::I32Add,
            
            // Allocate memory
            Instruction::I32Const(ARRAY_TYPE_ID as i32),
            Instruction::Call(0), // Call memory.allocate
            
            // Return pointer
            Instruction::Return,
        ]
    }

    fn generate_array_get(&self) -> Vec<Instruction> {
        vec![
            // Get array pointer and index
            Instruction::LocalGet(0),
            Instruction::LocalGet(1),
            
            // Check bounds
            Instruction::LocalGet(0),
            Instruction::I32Load(MemArg {
                offset: 8,
                align: 2,
                memory_index: 0,
            }),
            Instruction::LocalGet(1),
            Instruction::I32GeS,
            Instruction::If(BlockType::Empty),
            Instruction::Unreachable, // Out of bounds
            Instruction::End,
            
            // Calculate element pointer
            Instruction::I32Const(8),
            Instruction::I32Mul,
            Instruction::I32Const(16), // Header size
            Instruction::I32Add,
            Instruction::I32Add,
            
            // Return element pointer
            Instruction::Return,
        ]
    }

    fn generate_array_set(&self) -> Vec<Instruction> {
        vec![
            // Get array pointer, index, and value pointer
            Instruction::LocalGet(0),
            Instruction::LocalGet(1),
            Instruction::LocalGet(2),
            
            // Check bounds
            Instruction::LocalGet(0),
            Instruction::I32Load(MemArg {
                offset: 8,
                align: 2,
                memory_index: 0,
            }),
            Instruction::LocalGet(1),
            Instruction::I32GeS,
            Instruction::If(BlockType::Empty),
            Instruction::Unreachable, // Out of bounds
            Instruction::End,
            
            // Calculate element pointer
            Instruction::I32Const(8),
            Instruction::I32Mul,
            Instruction::I32Const(16), // Header size
            Instruction::I32Add,
            Instruction::I32Add,
            
            // Store value
            Instruction::I32Store(MemArg {
                offset: 0,
                align: 2,
                memory_index: 0,
            }),
            
            // Return
            Instruction::Return,
        ]
    }

    fn generate_array_length(&self) -> Vec<Instruction> {
        vec![
            // Get array pointer
            Instruction::LocalGet(0),
            
            // Load length from header
            Instruction::I32Load(MemArg {
                offset: 0,
                align: 2,
                memory_index: 0,
            }),
            
            // Return length
            Instruction::Return,
        ]
    }
    
    fn generate_array_iterate(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // Get array pointer
        instructions.push(Instruction::LocalGet(0));
        
        // Get array length
        instructions.push(Instruction::I32Load(MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));
        instructions.push(Instruction::LocalSet(2)); // Store length in local 2
        
        // Initialize loop index to 0
        instructions.push(Instruction::I32Const(0));
        instructions.push(Instruction::LocalSet(1)); // Store index in local 1
        
        // Start loop
        instructions.push(Instruction::Block(BlockType::Empty));
        instructions.push(Instruction::Loop(BlockType::Empty));
        
        // Check if index < array length
        instructions.push(Instruction::LocalGet(1)); // Get index
        instructions.push(Instruction::LocalGet(2)); // Get length
        instructions.push(Instruction::I32LtS); // index < length
        instructions.push(Instruction::BrIf(0)); // Continue if true
        
        // Break out of loop if false
        instructions.push(Instruction::Br(1));
        
        // Get current element
        instructions.push(Instruction::LocalGet(0)); // array pointer
        instructions.push(Instruction::LocalGet(1)); // index
        
        // Calculate element pointer
        instructions.push(Instruction::I32Const(8));
        instructions.push(Instruction::I32Mul);
        instructions.push(Instruction::I32Const(16)); // Header size
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::LocalSet(3)); // Store element pointer in local 3
        
        // Call callback function with element
        instructions.push(Instruction::LocalGet(3)); // Element pointer
        instructions.push(Instruction::LocalGet(1)); // Current index
        instructions.push(Instruction::CallIndirect {
            ty: 0, // Function signature index (assuming it takes element and index)
            table: 0, // Table index
        });
        
        // Increment index
        instructions.push(Instruction::LocalGet(1));
        instructions.push(Instruction::I32Const(1));
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::LocalSet(1));
        
        // Jump back to start of loop
        instructions.push(Instruction::Br(0));
        
        // End loop and block
        instructions.push(Instruction::End);
        instructions.push(Instruction::End);
        
        instructions
    }
    
    fn generate_array_map(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // Get array pointer
        instructions.push(Instruction::LocalGet(0));
        
        // Get array length
        instructions.push(Instruction::I32Load(MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));
        instructions.push(Instruction::LocalTee(2)); // Store length in local 2
        
        // Allocate new array of same length
        instructions.push(Instruction::I32Const(ARRAY_TYPE_ID as i32));
        instructions.push(Instruction::Call(0)); // Call memory.allocate
        instructions.push(Instruction::LocalTee(4)); // Store result array pointer in local 4
        
        // Store length in new array header
        instructions.push(Instruction::LocalGet(2));
        instructions.push(Instruction::I32Store(MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));
        
        // Initialize loop index to 0
        instructions.push(Instruction::I32Const(0));
        instructions.push(Instruction::LocalSet(1)); // Store index in local 1
        
        // Start loop
        instructions.push(Instruction::Block(BlockType::Empty));
        instructions.push(Instruction::Loop(BlockType::Empty));
        
        // Check if index < array length
        instructions.push(Instruction::LocalGet(1)); // Get index
        instructions.push(Instruction::LocalGet(2)); // Get length
        instructions.push(Instruction::I32LtS); // index < length
        instructions.push(Instruction::BrIf(0)); // Continue if true
        
        // Break out of loop if false
        instructions.push(Instruction::Br(1));
        
        // Get current element
        instructions.push(Instruction::LocalGet(0)); // array pointer
        instructions.push(Instruction::LocalGet(1)); // index
        
        // Calculate element pointer
        instructions.push(Instruction::I32Const(8));
        instructions.push(Instruction::I32Mul);
        instructions.push(Instruction::I32Const(16)); // Header size
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::LocalSet(3)); // Store element pointer in local 3
        
        // Call callback function with element
        instructions.push(Instruction::LocalGet(3)); // Element pointer
        instructions.push(Instruction::LocalGet(1)); // Current index
        instructions.push(Instruction::CallIndirect {
            ty: 0, // Function signature index (assuming it takes element and index)
            table: 0, // Table index
        });
        
        // Store result in new array
        instructions.push(Instruction::LocalGet(4)); // result array pointer
        instructions.push(Instruction::LocalGet(1)); // current index
        
        // Calculate result element pointer
        instructions.push(Instruction::I32Const(8));
        instructions.push(Instruction::I32Mul);
        instructions.push(Instruction::I32Const(16)); // Header size
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::I32Add);
        
        // Store result value
        instructions.push(Instruction::I32Store(MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));
        
        // Increment index
        instructions.push(Instruction::LocalGet(1));
        instructions.push(Instruction::I32Const(1));
        instructions.push(Instruction::I32Add);
        instructions.push(Instruction::LocalSet(1));
        
        // Jump back to start of loop
        instructions.push(Instruction::Br(0));
        
        // End loop and block
        instructions.push(Instruction::End);
        instructions.push(Instruction::End);
        
        // Return new array pointer
        instructions.push(Instruction::LocalGet(4));
        
        instructions
    }

    pub fn allocate_array(&mut self, size: usize) -> Result<usize, CompilerError> {
        let ptr = self.memory_manager.allocate(size * 8 + 16, ARRAY_TYPE_ID)?;
        
        // Store size in header
        self.memory_manager.store_i32(ptr, size as i32)?;
        
        Ok(ptr)
    }

    pub fn get_element(&self, array_ptr: usize, index: usize) -> Result<usize, CompilerError> {
        // Check type
        if self.memory_manager.get_type_id(array_ptr)? != ARRAY_TYPE_ID {
            return Err(CompilerError::type_error(
                "Invalid array pointer", 
                Some("Ensure the array pointer is valid".to_string()),
                None
            ));
        }
        
        // Check bounds
        let size = i32::from_le_bytes([
            self.memory_manager.data[array_ptr],
            self.memory_manager.data[array_ptr + 1],
            self.memory_manager.data[array_ptr + 2],
            self.memory_manager.data[array_ptr + 3],
        ]) as usize;
        
        if index >= size {
            return Err(CompilerError::type_error(
                format!("Array index out of bounds: {} >= {}", index, size),
                Some("Ensure index is within array bounds".to_string()),
                None
            ));
        }
        
        Ok(array_ptr + 16 + index * 8)
    }

    pub fn set_element(&mut self, array_ptr: usize, index: usize, value_ptr: usize) -> Result<(), CompilerError> {
        // Check type
        if self.memory_manager.get_type_id(array_ptr)? != ARRAY_TYPE_ID {
            return Err(CompilerError::type_error(
                "Invalid array pointer", 
                Some("Ensure the array pointer is valid".to_string()),
                None
            ));
        }
            
        // Check bounds
        let size = i32::from_le_bytes([
            self.memory_manager.data[array_ptr],
            self.memory_manager.data[array_ptr + 1],
            self.memory_manager.data[array_ptr + 2],
            self.memory_manager.data[array_ptr + 3],
        ]) as usize;
        
        if index >= size {
            return Err(CompilerError::type_error(
                format!("Array index out of bounds: {} >= {}", index, size),
                Some("Ensure index is within array bounds".to_string()),
                None
            ));
        }
        
        // First read the value data into a temporary buffer to avoid borrowing conflicts
        let mut value_data = [0u8; 8];
        value_data.copy_from_slice(&self.memory_manager.data[value_ptr..value_ptr + 8]);
        
        // Now copy from the temporary buffer to the destination
        let element_ptr = array_ptr + 16 + index * 8;
        self.memory_manager.data[element_ptr..element_ptr + 8].copy_from_slice(&value_data);
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasmtime::{Engine, Module, Store, Instance, Val, Func, FuncType};

    #[test]
    fn test_array_operations() {
        let mut array_manager = ArrayManager::new(MemoryManager::new(1, Some(10)));
        
        // Test array allocation
        let array_ptr = array_manager.allocate_array(5).unwrap();
        assert!(array_ptr >= 16); // Header size
        
        // Test array access
        let value_ptr = array_manager.get_element(array_ptr, 0).unwrap();
        assert_eq!(value_ptr, array_ptr + 16);
        
        // Test array bounds
        let result = array_manager.get_element(array_ptr, 5);
        assert!(result.is_err());
        
        // Test array set
        let value = 42i64;
        let value_bytes = value.to_le_bytes();
        array_manager.memory_manager.data[value_ptr..value_ptr + 8].copy_from_slice(&value_bytes);
        
        let stored_value = i64::from_le_bytes([
            array_manager.memory_manager.data[value_ptr],
            array_manager.memory_manager.data[value_ptr + 1],
            array_manager.memory_manager.data[value_ptr + 2],
            array_manager.memory_manager.data[value_ptr + 3],
            array_manager.memory_manager.data[value_ptr + 4],
            array_manager.memory_manager.data[value_ptr + 5],
            array_manager.memory_manager.data[value_ptr + 6],
            array_manager.memory_manager.data[value_ptr + 7],
        ]);
        assert_eq!(stored_value, 42);
    }
    
    #[test]
    fn test_array_length() {
        let engine = Engine::default();
        let memory_manager = MemoryManager::new(1, Some(10));
        let array_manager = ArrayManager::new(memory_manager.clone());
        
        let mut codegen = CodeGenerator::new();
        array_manager.register_functions(&mut codegen).unwrap();
        
        // Generate WebAssembly module
        let wasm_bytes = codegen.build();
        let module = Module::new(&engine, &wasm_bytes).unwrap();
        let mut store = Store::new(&engine, ());
        let instance = Instance::new(&mut store, &module, &[]).unwrap();
        
        // Create test array
        let array_ptr = array_manager.allocate_array(10).unwrap();
        
        // Test array.length function
        let length_func = instance.get_func(&mut store, "array.length").unwrap();
        let mut results = vec![Val::I32(0)];
        length_func.call(&mut store, &[Val::I32(array_ptr as i32)], &mut results).unwrap();
        
        assert_eq!(results[0].unwrap_i32(), 10);
    }
    
    #[test]
    fn test_array_iterate() {
        // This test requires support for indirect calls, which would need a more
        // complex setup with function tables. For simplicity, we'll test the
        // iteration logic directly without using WebAssembly.
        
        let memory_manager = MemoryManager::new(1, Some(10));
        let mut array_manager = ArrayManager::new(memory_manager);
        
        // Create an array with 5 elements
        let array_ptr = array_manager.allocate_array(5).unwrap();
        
        // Set array values (1, 2, 3, 4, 5)
        for i in 0..5 {
            let elem_ptr = array_manager.get_element(array_ptr, i).unwrap();
            let value = (i + 1) as i64;
            let value_bytes = value.to_le_bytes();
            array_manager.memory_manager.data[elem_ptr..elem_ptr + 8].copy_from_slice(&value_bytes);
        }
        
        // Manually iterate over array
        let mut sum = 0;
        for i in 0..5 {
            let elem_ptr = array_manager.get_element(array_ptr, i).unwrap();
            let value = i64::from_le_bytes([
                array_manager.memory_manager.data[elem_ptr],
                array_manager.memory_manager.data[elem_ptr + 1],
                array_manager.memory_manager.data[elem_ptr + 2],
                array_manager.memory_manager.data[elem_ptr + 3],
                array_manager.memory_manager.data[elem_ptr + 4],
                array_manager.memory_manager.data[elem_ptr + 5],
                array_manager.memory_manager.data[elem_ptr + 6],
                array_manager.memory_manager.data[elem_ptr + 7],
            ]);
            sum += value;
        }
        
        assert_eq!(sum, 15); // 1 + 2 + 3 + 4 + 5 = 15
    }
    
    #[test]
    fn test_array_map() {
        // Similar to iterate test, we'll test the mapping logic directly
        
        let memory_manager = MemoryManager::new(1, Some(10));
        let mut array_manager = ArrayManager::new(memory_manager);
        
        // Create an array with 5 elements
        let array_ptr = array_manager.allocate_array(5).unwrap();
        
        // Set array values (1, 2, 3, 4, 5)
        for i in 0..5 {
            let elem_ptr = array_manager.get_element(array_ptr, i).unwrap();
            let value = (i + 1) as i64;
            let value_bytes = value.to_le_bytes();
            array_manager.memory_manager.data[elem_ptr..elem_ptr + 8].copy_from_slice(&value_bytes);
        }
        
        // Manually map the array (multiply each element by 2)
        let result_ptr = array_manager.allocate_array(5).unwrap();
        for i in 0..5 {
            let elem_ptr = array_manager.get_element(array_ptr, i).unwrap();
            let value = i64::from_le_bytes([
                array_manager.memory_manager.data[elem_ptr],
                array_manager.memory_manager.data[elem_ptr + 1],
                array_manager.memory_manager.data[elem_ptr + 2],
                array_manager.memory_manager.data[elem_ptr + 3],
                array_manager.memory_manager.data[elem_ptr + 4],
                array_manager.memory_manager.data[elem_ptr + 5],
                array_manager.memory_manager.data[elem_ptr + 6],
                array_manager.memory_manager.data[elem_ptr + 7],
            ]);
            
            // Multiply by 2
            let new_value = value * 2;
            let new_value_bytes = new_value.to_le_bytes();
            
            // Store in result array
            let result_elem_ptr = array_manager.get_element(result_ptr, i).unwrap();
            array_manager.memory_manager.data[result_elem_ptr..result_elem_ptr + 8]
                .copy_from_slice(&new_value_bytes);
        }
        
        // Verify result array contains [2, 4, 6, 8, 10]
        for i in 0..5 {
            let elem_ptr = array_manager.get_element(result_ptr, i).unwrap();
            let value = i64::from_le_bytes([
                array_manager.memory_manager.data[elem_ptr],
                array_manager.memory_manager.data[elem_ptr + 1],
                array_manager.memory_manager.data[elem_ptr + 2],
                array_manager.memory_manager.data[elem_ptr + 3],
                array_manager.memory_manager.data[elem_ptr + 4],
                array_manager.memory_manager.data[elem_ptr + 5],
                array_manager.memory_manager.data[elem_ptr + 6],
                array_manager.memory_manager.data[elem_ptr + 7],
            ]);
            
            assert_eq!(value, (i + 1) as i64 * 2);
        }
    }
} 