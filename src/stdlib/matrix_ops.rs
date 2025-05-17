use crate::codegen::CodeGenerator;
use crate::types::WasmType;
use crate::error::CompilerError;
use crate::stdlib::memory::MemoryManager;
use crate::codegen::MATRIX_TYPE_ID;
use wasm_encoder::{Instruction, MemArg, BlockType, ValType};
use std::convert::TryInto;
use crate::stdlib::register_stdlib_function;

/// Matrix operations implementation
pub struct MatrixOperations {
    heap_start: usize,
}

impl MatrixOperations {
    pub fn new(heap_start: usize) -> Self {
        Self { heap_start }
    }

    pub fn register_functions(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // Matrix creation
        register_stdlib_function(
            codegen,
            "matrix.create",
            &[WasmType::I32, WasmType::I32], // rows, cols
            Some(WasmType::I32), // matrix pointer
            self.generate_matrix_create()
        )?;

        // Matrix get element
        register_stdlib_function(
            codegen,
            "matrix.get",
            &[WasmType::I32, WasmType::I32, WasmType::I32], // matrix ptr, row, col
            Some(WasmType::F64),
            self.generate_matrix_get()
        )?;

        // Matrix set element
        register_stdlib_function(
            codegen,
            "matrix.set",
            &[WasmType::I32, WasmType::I32, WasmType::I32, WasmType::F64], // matrix ptr, row, col, value
            Some(WasmType::I32), // success flag
            self.generate_matrix_set()
        )?;

        // Matrix addition
        register_stdlib_function(
            codegen,
            "matrix.add",
            &[WasmType::I32, WasmType::I32], // matrix1 ptr, matrix2 ptr
            Some(WasmType::I32), // result matrix ptr
            self.generate_matrix_add()
        )?;

        // Matrix multiplication
        register_stdlib_function(
            codegen,
            "matrix.multiply",
            &[WasmType::I32, WasmType::I32], // matrix1 ptr, matrix2 ptr
            Some(WasmType::I32), // result matrix ptr
            self.generate_matrix_multiply()
        )?;

        // Matrix transpose
        register_stdlib_function(
            codegen,
            "matrix.transpose",
            &[WasmType::I32], // matrix ptr
            Some(WasmType::I32), // result matrix ptr
            self.generate_matrix_transpose()
        )?;

        // Matrix determinant
        register_stdlib_function(
            codegen,
            "matrix.determinant",
            &[WasmType::I32], // matrix ptr
            Some(WasmType::F64), // determinant value
            self.generate_matrix_determinant()
        )?;

        // Matrix inverse
        register_stdlib_function(
            codegen,
            "matrix.inverse",
            &[WasmType::I32], // matrix ptr
            Some(WasmType::I32), // result matrix ptr
            self.generate_matrix_inverse()
        )?;

        Ok(())
    }

    fn generate_matrix_create(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // Parameters:
        // - Local 0: rows (i32)
        // - Local 1: cols (i32)
        
        // Calculate memory needed: header (12 bytes) + rows * cols * 8 bytes
        instructions.push(Instruction::LocalGet(0)); // rows
        instructions.push(Instruction::LocalGet(1)); // cols
        instructions.push(Instruction::I32Mul);      // rows * cols
        instructions.push(Instruction::I32Const(8)); // sizeof(f64)
        instructions.push(Instruction::I32Mul);      // rows * cols * 8
        instructions.push(Instruction::I32Const(12)); // header size
        instructions.push(Instruction::I32Add);      // total size
        
        // Allocate memory
        instructions.push(Instruction::I32Const(MATRIX_TYPE_ID.try_into().unwrap()));
        instructions.push(Instruction::Call(0));     // Call memory.allocate
        
        // Save the pointer
        instructions.push(Instruction::LocalTee(2)); // Store ptr in local 2
        
        // Write the rows (4 bytes)
        instructions.push(Instruction::LocalGet(0)); // rows
        instructions.push(Instruction::I32Store(MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));
        
        // Write the cols (4 bytes)
        instructions.push(Instruction::LocalGet(2)); // ptr
        instructions.push(Instruction::I32Const(4)); // offset for cols
        instructions.push(Instruction::I32Add);      // ptr + 4
        instructions.push(Instruction::LocalGet(1)); // cols
        instructions.push(Instruction::I32Store(MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));
        
        // Write the type (4 bytes)
        instructions.push(Instruction::LocalGet(2)); // ptr
        instructions.push(Instruction::I32Const(8)); // offset for type
        instructions.push(Instruction::I32Add);      // ptr + 8
        instructions.push(Instruction::I32Const(2)); // Float64 type
        instructions.push(Instruction::I32Store(MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));
        
        // Initialize all elements to 0.0
        instructions.push(Instruction::LocalGet(0)); // rows
        instructions.push(Instruction::LocalGet(1)); // cols
        instructions.push(Instruction::I32Mul);      // rows * cols
        instructions.push(Instruction::LocalTee(3)); // Save count to local 3
        
        // Check if count > 0
        instructions.push(Instruction::I32Const(0));
        instructions.push(Instruction::I32GtS);      // count > 0?
        instructions.push(Instruction::If(BlockType::Empty));
        
        // Loop to initialize elements
        instructions.push(Instruction::LocalGet(2)); // ptr
        instructions.push(Instruction::I32Const(12)); // header size
        instructions.push(Instruction::I32Add);      // data ptr = ptr + 12
        instructions.push(Instruction::LocalSet(4)); // Store data ptr in local 4
        
        instructions.push(Instruction::I32Const(0)); // i = 0
        instructions.push(Instruction::LocalSet(5)); // Store i in local 5
        
        instructions.push(Instruction::Block(BlockType::Empty));
        instructions.push(Instruction::Loop(BlockType::Empty));
        
        // Loop condition: i < count?
        instructions.push(Instruction::LocalGet(5)); // i
        instructions.push(Instruction::LocalGet(3)); // count
        instructions.push(Instruction::I32LtS);      // i < count?
        instructions.push(Instruction::I32Eqz);      // !(i < count)?
        instructions.push(Instruction::BrIf(1));     // Break if done
        
        // Set element to 0.0
        instructions.push(Instruction::LocalGet(4)); // data ptr
        instructions.push(Instruction::LocalGet(5)); // i
        instructions.push(Instruction::I32Const(8)); // sizeof(f64)
        instructions.push(Instruction::I32Mul);      // i * 8
        instructions.push(Instruction::I32Add);      // elem ptr = data ptr + i * 8
        instructions.push(Instruction::F64Const(0.0)); // value = 0.0
        instructions.push(Instruction::F64Store(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
        
        // Increment i
        instructions.push(Instruction::LocalGet(5)); // i
        instructions.push(Instruction::I32Const(1)); // 1
        instructions.push(Instruction::I32Add);      // i + 1
        instructions.push(Instruction::LocalSet(5)); // i = i + 1
        
        // Loop
        instructions.push(Instruction::Br(0));
        
        // End loop
        instructions.push(Instruction::End);
        instructions.push(Instruction::End);
        
        // End if
        instructions.push(Instruction::End);
        
        // Return the matrix pointer
        instructions.push(Instruction::LocalGet(2));
        
        instructions
    }

    fn generate_matrix_get(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // Parameters:
        // - Local 0: matrix ptr (i32)
        // - Local 1: row (i32)
        // - Local 2: col (i32)
        
        // Load matrix dimensions
        instructions.push(Instruction::LocalGet(0)); // matrix ptr
        instructions.push(Instruction::I32Load(MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        })); // Load rows
        instructions.push(Instruction::LocalSet(3)); // Save rows to local 3
        
        instructions.push(Instruction::LocalGet(0)); // matrix ptr
        instructions.push(Instruction::I32Load(MemArg {
            offset: 4,
            align: 2,
            memory_index: 0,
        })); // Load cols
        instructions.push(Instruction::LocalSet(4)); // Save cols to local 4
        
        // Check bounds for row
        instructions.push(Instruction::LocalGet(1)); // row
        instructions.push(Instruction::I32Const(0)); // 0
        instructions.push(Instruction::I32LtS); // row < 0?
        
        instructions.push(Instruction::LocalGet(1)); // row
        instructions.push(Instruction::LocalGet(3)); // rows
        instructions.push(Instruction::I32GeS); // row >= rows?
        
        instructions.push(Instruction::I32Or); // row < 0 || row >= rows
        
        // Check bounds for col
        instructions.push(Instruction::LocalGet(2)); // col
        instructions.push(Instruction::I32Const(0)); // 0
        instructions.push(Instruction::I32LtS); // col < 0?
        
        instructions.push(Instruction::LocalGet(2)); // col
        instructions.push(Instruction::LocalGet(4)); // cols
        instructions.push(Instruction::I32GeS); // col >= cols?
        
        instructions.push(Instruction::I32Or); // col < 0 || col >= cols
        
        instructions.push(Instruction::I32Or); // out of bounds for row or col
        
        // If out of bounds, return NaN
        instructions.push(Instruction::If(BlockType::Result(ValType::F64)));
        instructions.push(Instruction::F64Const(f64::NAN));
        instructions.push(Instruction::Else);
        
        // Calculate index: row * cols + col
        instructions.push(Instruction::LocalGet(1)); // row
        instructions.push(Instruction::LocalGet(4)); // cols
        instructions.push(Instruction::I32Mul); // row * cols
        instructions.push(Instruction::LocalGet(2)); // col
        instructions.push(Instruction::I32Add); // row * cols + col
        
        // Calculate memory offset: 12 + (row * cols + col) * 8
        instructions.push(Instruction::I32Const(8)); // sizeof(f64)
        instructions.push(Instruction::I32Mul); // (row * cols + col) * 8
        instructions.push(Instruction::I32Const(12)); // header size
        instructions.push(Instruction::I32Add); // 12 + (row * cols + col) * 8
        
        // Add to matrix pointer
        instructions.push(Instruction::LocalGet(0)); // matrix ptr
        instructions.push(Instruction::I32Add); // matrix ptr + offset
        
        // Load value
        instructions.push(Instruction::F64Load(MemArg {
            offset: 0,
            align: 3,  // 2^3 = 8 byte alignment for f64
            memory_index: 0,
        }));
        
        // End if
        instructions.push(Instruction::End);
        
        instructions
    }

    fn generate_matrix_set(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // Parameters:
        // - Local 0: matrix ptr (i32)
        // - Local 1: row (i32)
        // - Local 2: col (i32)
        // - Local 3: value (f64)
        
        // Load matrix dimensions
        instructions.push(Instruction::LocalGet(0)); // matrix ptr
        instructions.push(Instruction::I32Load(MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        })); // Load rows
        instructions.push(Instruction::LocalSet(4)); // Save rows to local 4
        
        instructions.push(Instruction::LocalGet(0)); // matrix ptr
        instructions.push(Instruction::I32Load(MemArg {
            offset: 4,
            align: 2,
            memory_index: 0,
        })); // Load cols
        instructions.push(Instruction::LocalSet(5)); // Save cols to local 5
        
        // Check bounds for row
        instructions.push(Instruction::LocalGet(1)); // row
        instructions.push(Instruction::I32Const(0)); // 0
        instructions.push(Instruction::I32LtS); // row < 0?
        
        instructions.push(Instruction::LocalGet(1)); // row
        instructions.push(Instruction::LocalGet(4)); // rows
        instructions.push(Instruction::I32GeS); // row >= rows?
        
        instructions.push(Instruction::I32Or); // row < 0 || row >= rows
        
        // Check bounds for col
        instructions.push(Instruction::LocalGet(2)); // col
        instructions.push(Instruction::I32Const(0)); // 0
        instructions.push(Instruction::I32LtS); // col < 0?
        
        instructions.push(Instruction::LocalGet(2)); // col
        instructions.push(Instruction::LocalGet(5)); // cols
        instructions.push(Instruction::I32GeS); // col >= cols?
        
        instructions.push(Instruction::I32Or); // col < 0 || col >= cols
        
        instructions.push(Instruction::I32Or); // out of bounds for row or col
        
        // If out of bounds, return 0 (failure)
        instructions.push(Instruction::If(BlockType::Result(ValType::I32)));
        instructions.push(Instruction::I32Const(0)); // return 0 (failure)
        instructions.push(Instruction::Else);
        
        // Calculate index: row * cols + col
        instructions.push(Instruction::LocalGet(1)); // row
        instructions.push(Instruction::LocalGet(5)); // cols
        instructions.push(Instruction::I32Mul); // row * cols
        instructions.push(Instruction::LocalGet(2)); // col
        instructions.push(Instruction::I32Add); // row * cols + col
        
        // Calculate memory offset: 12 + (row * cols + col) * 8
        instructions.push(Instruction::I32Const(8)); // sizeof(f64)
        instructions.push(Instruction::I32Mul); // (row * cols + col) * 8
        instructions.push(Instruction::I32Const(12)); // header size
        instructions.push(Instruction::I32Add); // 12 + (row * cols + col) * 8
        
        // Add to matrix pointer
        instructions.push(Instruction::LocalGet(0)); // matrix ptr
        instructions.push(Instruction::I32Add); // matrix ptr + offset
        
        // Store value
        instructions.push(Instruction::LocalGet(3)); // value
        instructions.push(Instruction::F64Store(MemArg {
            offset: 0,
            align: 3,  // 2^3 = 8 byte alignment for f64
            memory_index: 0,
        }));
        
        // Return 1 (success)
        instructions.push(Instruction::I32Const(1));
        
        // End if
        instructions.push(Instruction::End);
        
        instructions
    }

    fn generate_matrix_add(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // Parameters:
        // - Local 0: matrix1 ptr (i32)
        // - Local 1: matrix2 ptr (i32)
        
        // Load matrix1 dimensions
        instructions.push(Instruction::LocalGet(0)); // matrix1 ptr
        instructions.push(Instruction::I32Load(MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        })); // Load rows1
        instructions.push(Instruction::LocalSet(2)); // Save rows1 to local 2
        
        instructions.push(Instruction::LocalGet(0)); // matrix1 ptr
        instructions.push(Instruction::I32Load(MemArg {
            offset: 4,
            align: 2,
            memory_index: 0,
        })); // Load cols1
        instructions.push(Instruction::LocalSet(3)); // Save cols1 to local 3
        
        // Load matrix2 dimensions
        instructions.push(Instruction::LocalGet(1)); // matrix2 ptr
        instructions.push(Instruction::I32Load(MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        })); // Load rows2
        instructions.push(Instruction::LocalSet(4)); // Save rows2 to local 4
        
        instructions.push(Instruction::LocalGet(1)); // matrix2 ptr
        instructions.push(Instruction::I32Load(MemArg {
            offset: 4,
            align: 2,
            memory_index: 0,
        })); // Load cols2
        instructions.push(Instruction::LocalSet(5)); // Save cols2 to local 5
        
        // Check if dimensions match
        instructions.push(Instruction::LocalGet(2)); // rows1
        instructions.push(Instruction::LocalGet(4)); // rows2
        instructions.push(Instruction::I32Ne); // rows1 != rows2
        
        instructions.push(Instruction::LocalGet(3)); // cols1
        instructions.push(Instruction::LocalGet(5)); // cols2
        instructions.push(Instruction::I32Ne); // cols1 != cols2
        
        instructions.push(Instruction::I32Or); // dimensions don't match
        
        // If dimensions don't match, create and return error result
        instructions.push(Instruction::If(BlockType::Result(ValType::I32)));
        
        // Create and store error message in memory (simplified)
        // In a real implementation, we would store a detailed error message in memory 
        // and return a pointer to the error struct
        
        // For now, just return 0 (null pointer) to indicate error
        instructions.push(Instruction::I32Const(0));
        
        // Store error info in a global error variable (example)
        // Here we could store error details like "Matrix dimension mismatch: 
        // First matrix is rows1 x cols1, second matrix is rows2 x cols2"
        
        instructions.push(Instruction::Else);
        
        // Create result matrix
        instructions.push(Instruction::LocalGet(2)); // rows
        instructions.push(Instruction::LocalGet(3)); // cols
        
        // Call matrix.create
        // This should be index of matrix.create function
        instructions.push(Instruction::Call(1));
        instructions.push(Instruction::LocalSet(6)); // Save result matrix ptr to local 6
        
        // Calculate element count: rows * cols
        instructions.push(Instruction::LocalGet(2)); // rows
        instructions.push(Instruction::LocalGet(3)); // cols
        instructions.push(Instruction::I32Mul); // rows * cols
        instructions.push(Instruction::LocalSet(7)); // Save count to local 7
        
        // Check if count > 0
        instructions.push(Instruction::LocalGet(7)); // count
        instructions.push(Instruction::I32Const(0)); // 0
        instructions.push(Instruction::I32GtS); // count > 0?
        instructions.push(Instruction::If(BlockType::Empty));
        
        // Loop to add elements
        instructions.push(Instruction::I32Const(0)); // i = 0
        instructions.push(Instruction::LocalSet(8)); // Store i in local 8
        
        instructions.push(Instruction::Block(BlockType::Empty));
        instructions.push(Instruction::Loop(BlockType::Empty));
        
        // Loop condition: i < count?
        instructions.push(Instruction::LocalGet(8)); // i
        instructions.push(Instruction::LocalGet(7)); // count
        instructions.push(Instruction::I32LtS); // i < count?
        instructions.push(Instruction::I32Eqz); // !(i < count)?
        instructions.push(Instruction::BrIf(1)); // Break if done
        
        // Calculate memory offset for matrix1: 12 + i * 8
        instructions.push(Instruction::LocalGet(0)); // matrix1 ptr
        instructions.push(Instruction::LocalGet(8)); // i
        instructions.push(Instruction::I32Const(8)); // sizeof(f64)
        instructions.push(Instruction::I32Mul); // i * 8
        instructions.push(Instruction::I32Const(12)); // header size
        instructions.push(Instruction::I32Add); // 12 + i * 8
        instructions.push(Instruction::I32Add); // matrix1 ptr + offset
        
        // Load value from matrix1
        instructions.push(Instruction::F64Load(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
        instructions.push(Instruction::LocalSet(9)); // Save value1 to local 9
        
        // Calculate memory offset for matrix2: 12 + i * 8
        instructions.push(Instruction::LocalGet(1)); // matrix2 ptr
        instructions.push(Instruction::LocalGet(8)); // i
        instructions.push(Instruction::I32Const(8)); // sizeof(f64)
        instructions.push(Instruction::I32Mul); // i * 8
        instructions.push(Instruction::I32Const(12)); // header size
        instructions.push(Instruction::I32Add); // 12 + i * 8
        instructions.push(Instruction::I32Add); // matrix2 ptr + offset
        
        // Load value from matrix2
        instructions.push(Instruction::F64Load(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
        instructions.push(Instruction::LocalSet(10)); // Save value2 to local 10
        
        // Add values
        instructions.push(Instruction::LocalGet(9)); // value1
        instructions.push(Instruction::LocalGet(10)); // value2
        instructions.push(Instruction::F64Add); // value1 + value2
        instructions.push(Instruction::LocalSet(11)); // Save result to local 11
        
        // Calculate memory offset for result matrix: 12 + i * 8
        instructions.push(Instruction::LocalGet(6)); // result matrix ptr
        instructions.push(Instruction::LocalGet(8)); // i
        instructions.push(Instruction::I32Const(8)); // sizeof(f64)
        instructions.push(Instruction::I32Mul); // i * 8
        instructions.push(Instruction::I32Const(12)); // header size
        instructions.push(Instruction::I32Add); // 12 + i * 8
        instructions.push(Instruction::I32Add); // result matrix ptr + offset
        
        // Store result value
        instructions.push(Instruction::LocalGet(11)); // result value
        instructions.push(Instruction::F64Store(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
        
        // Increment i
        instructions.push(Instruction::LocalGet(8)); // i
        instructions.push(Instruction::I32Const(1)); // 1
        instructions.push(Instruction::I32Add); // i + 1
        instructions.push(Instruction::LocalSet(8)); // i = i + 1
        
        // Loop
        instructions.push(Instruction::Br(0));
        
        // End loop
        instructions.push(Instruction::End);
        instructions.push(Instruction::End);
        
        // End if (count > 0)
        instructions.push(Instruction::End);
        
        // Return result matrix pointer
        instructions.push(Instruction::LocalGet(6));
        
        // End if (dimensions match)
        instructions.push(Instruction::End);
        
        instructions
    }

    fn generate_matrix_multiply(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // Parameters:
        // - Local 0: matrix1 ptr (i32)
        // - Local 1: matrix2 ptr (i32)
        
        // Load matrix1 dimensions
        instructions.push(Instruction::LocalGet(0)); // matrix1 ptr
        instructions.push(Instruction::I32Load(MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        })); // Load rows1
        instructions.push(Instruction::LocalSet(2)); // Save rows1 to local 2
        
        instructions.push(Instruction::LocalGet(0)); // matrix1 ptr
        instructions.push(Instruction::I32Load(MemArg {
            offset: 4,
            align: 2,
            memory_index: 0,
        })); // Load cols1
        instructions.push(Instruction::LocalSet(3)); // Save cols1 to local 3
        
        // Load matrix2 dimensions
        instructions.push(Instruction::LocalGet(1)); // matrix2 ptr
        instructions.push(Instruction::I32Load(MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        })); // Load rows2
        instructions.push(Instruction::LocalSet(4)); // Save rows2 to local 4
        
        instructions.push(Instruction::LocalGet(1)); // matrix2 ptr
        instructions.push(Instruction::I32Load(MemArg {
            offset: 4,
            align: 2,
            memory_index: 0,
        })); // Load cols2
        instructions.push(Instruction::LocalSet(5)); // Save cols2 to local 5
        
        // Check if dimensions are compatible (cols1 == rows2)
        instructions.push(Instruction::LocalGet(3)); // cols1
        instructions.push(Instruction::LocalGet(4)); // rows2
        instructions.push(Instruction::I32Ne); // cols1 != rows2
        
        // If dimensions are incompatible, create and return error result
        instructions.push(Instruction::If(BlockType::Result(ValType::I32)));
        
        // Return 0 (null pointer) to indicate error
        instructions.push(Instruction::I32Const(0));
        
        instructions.push(Instruction::Else);
        
        // Create result matrix with dimensions rows1 x cols2
        instructions.push(Instruction::LocalGet(2)); // rows1
        instructions.push(Instruction::LocalGet(5)); // cols2
        
        // Call matrix.create
        // This should be index of matrix.create function
        instructions.push(Instruction::Call(1));
        instructions.push(Instruction::LocalSet(6)); // Save result matrix ptr to local 6
        
        // Loop through rows of matrix1
        instructions.push(Instruction::I32Const(0)); // row = 0
        instructions.push(Instruction::LocalSet(7)); // Store row in local 7
        
        instructions.push(Instruction::Block(BlockType::Empty));
        instructions.push(Instruction::Loop(BlockType::Empty));
        
        // Loop condition: row < rows1?
        instructions.push(Instruction::LocalGet(7)); // row
        instructions.push(Instruction::LocalGet(2)); // rows1
        instructions.push(Instruction::I32LtS); // row < rows1?
        instructions.push(Instruction::I32Eqz); // !(row < rows1)?
        instructions.push(Instruction::BrIf(1)); // Break if done with rows
        
        // Loop through columns of matrix2
        instructions.push(Instruction::I32Const(0)); // col = 0
        instructions.push(Instruction::LocalSet(8)); // Store col in local 8
        
        instructions.push(Instruction::Block(BlockType::Empty));
        instructions.push(Instruction::Loop(BlockType::Empty));
        
        // Loop condition: col < cols2?
        instructions.push(Instruction::LocalGet(8)); // col
        instructions.push(Instruction::LocalGet(5)); // cols2
        instructions.push(Instruction::I32LtS); // col < cols2?
        instructions.push(Instruction::I32Eqz); // !(col < cols2)?
        instructions.push(Instruction::BrIf(1)); // Break if done with cols
        
        // Initialize sum for this result element
        instructions.push(Instruction::F64Const(0.0));
        instructions.push(Instruction::LocalSet(9)); // Store sum in local 9
        
        // Loop through common dimension (cols1/rows2)
        instructions.push(Instruction::I32Const(0)); // k = 0
        instructions.push(Instruction::LocalSet(10)); // Store k in local 10
        
        instructions.push(Instruction::Block(BlockType::Empty));
        instructions.push(Instruction::Loop(BlockType::Empty));
        
        // Loop condition: k < cols1?
        instructions.push(Instruction::LocalGet(10)); // k
        instructions.push(Instruction::LocalGet(3)); // cols1
        instructions.push(Instruction::I32LtS); // k < cols1?
        instructions.push(Instruction::I32Eqz); // !(k < cols1)?
        instructions.push(Instruction::BrIf(1)); // Break if done with k
        
        // Get matrix1[row][k]
        // Calculate offset: 12 + (row * cols1 + k) * 8
        instructions.push(Instruction::LocalGet(7)); // row
        instructions.push(Instruction::LocalGet(3)); // cols1
        instructions.push(Instruction::I32Mul); // row * cols1
        instructions.push(Instruction::LocalGet(10)); // k
        instructions.push(Instruction::I32Add); // row * cols1 + k
        instructions.push(Instruction::I32Const(8)); // sizeof(f64)
        instructions.push(Instruction::I32Mul); // (row * cols1 + k) * 8
        instructions.push(Instruction::I32Const(12)); // header size
        instructions.push(Instruction::I32Add); // 12 + (row * cols1 + k) * 8
        
        // Add to matrix1 pointer
        instructions.push(Instruction::LocalGet(0)); // matrix1 ptr
        instructions.push(Instruction::I32Add); // matrix1 ptr + offset
        
        // Load value from matrix1
        instructions.push(Instruction::F64Load(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
        instructions.push(Instruction::LocalSet(11)); // Store matrix1[row][k] in local 11
        
        // Get matrix2[k][col]
        // Calculate offset: 12 + (k * cols2 + col) * 8
        instructions.push(Instruction::LocalGet(10)); // k
        instructions.push(Instruction::LocalGet(5)); // cols2
        instructions.push(Instruction::I32Mul); // k * cols2
        instructions.push(Instruction::LocalGet(8)); // col
        instructions.push(Instruction::I32Add); // k * cols2 + col
        instructions.push(Instruction::I32Const(8)); // sizeof(f64)
        instructions.push(Instruction::I32Mul); // (k * cols2 + col) * 8
        instructions.push(Instruction::I32Const(12)); // header size
        instructions.push(Instruction::I32Add); // 12 + (k * cols2 + col) * 8
        
        // Add to matrix2 pointer
        instructions.push(Instruction::LocalGet(1)); // matrix2 ptr
        instructions.push(Instruction::I32Add); // matrix2 ptr + offset
        
        // Load value from matrix2
        instructions.push(Instruction::F64Load(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
        instructions.push(Instruction::LocalSet(12)); // Store matrix2[k][col] in local 12
        
        // Multiply and add to sum
        instructions.push(Instruction::LocalGet(11)); // matrix1[row][k]
        instructions.push(Instruction::LocalGet(12)); // matrix2[k][col]
        instructions.push(Instruction::F64Mul); // matrix1[row][k] * matrix2[k][col]
        instructions.push(Instruction::LocalGet(9)); // sum
        instructions.push(Instruction::F64Add); // sum + (matrix1[row][k] * matrix2[k][col])
        instructions.push(Instruction::LocalSet(9)); // Update sum
        
        // Increment k
        instructions.push(Instruction::LocalGet(10)); // k
        instructions.push(Instruction::I32Const(1)); // 1
        instructions.push(Instruction::I32Add); // k + 1
        instructions.push(Instruction::LocalSet(10)); // k = k + 1
        
        // Loop through k
        instructions.push(Instruction::Br(0));
        
        // End k loop
        instructions.push(Instruction::End);
        instructions.push(Instruction::End);
        
        // Store result in matrix3[row][col]
        // Calculate offset: 12 + (row * cols2 + col) * 8
        instructions.push(Instruction::LocalGet(7)); // row
        instructions.push(Instruction::LocalGet(5)); // cols2
        instructions.push(Instruction::I32Mul); // row * cols2
        instructions.push(Instruction::LocalGet(8)); // col
        instructions.push(Instruction::I32Add); // row * cols2 + col
        instructions.push(Instruction::I32Const(8)); // sizeof(f64)
        instructions.push(Instruction::I32Mul); // (row * cols2 + col) * 8
        instructions.push(Instruction::I32Const(12)); // header size
        instructions.push(Instruction::I32Add); // 12 + (row * cols2 + col) * 8
        
        // Add to result matrix pointer
        instructions.push(Instruction::LocalGet(6)); // result matrix ptr
        instructions.push(Instruction::I32Add); // result ptr + offset
        
        // Store sum in result matrix
        instructions.push(Instruction::LocalGet(9)); // sum
        instructions.push(Instruction::F64Store(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
        
        // Increment col
        instructions.push(Instruction::LocalGet(8)); // col
        instructions.push(Instruction::I32Const(1)); // 1
        instructions.push(Instruction::I32Add); // col + 1
        instructions.push(Instruction::LocalSet(8)); // col = col + 1
        
        // Loop through cols
        instructions.push(Instruction::Br(0));
        
        // End col loop
        instructions.push(Instruction::End);
        instructions.push(Instruction::End);
        
        // Increment row
        instructions.push(Instruction::LocalGet(7)); // row
        instructions.push(Instruction::I32Const(1)); // 1
        instructions.push(Instruction::I32Add); // row + 1
        instructions.push(Instruction::LocalSet(7)); // row = row + 1
        
        // Loop through rows
        instructions.push(Instruction::Br(0));
        
        // End row loop
        instructions.push(Instruction::End);
        instructions.push(Instruction::End);
        
        // Return result matrix pointer
        instructions.push(Instruction::LocalGet(6));
        
        // End if (dimensions compatible)
        instructions.push(Instruction::End);
        
        instructions
    }

    fn generate_matrix_transpose(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // Parameters:
        // - Local 0: matrix ptr (i32)
        
        // Load matrix dimensions
        instructions.push(Instruction::LocalGet(0)); // matrix ptr
        instructions.push(Instruction::I32Load(MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        })); // Load rows
        instructions.push(Instruction::LocalSet(1)); // Save rows to local 1
        
        instructions.push(Instruction::LocalGet(0)); // matrix ptr
        instructions.push(Instruction::I32Load(MemArg {
            offset: 4,
            align: 2,
            memory_index: 0,
        })); // Load cols
        instructions.push(Instruction::LocalSet(2)); // Save cols to local 2
        
        // Create result matrix with swapped dimensions (cols x rows)
        instructions.push(Instruction::LocalGet(2)); // cols (becomes rows in result)
        instructions.push(Instruction::LocalGet(1)); // rows (becomes cols in result)
        
        // Call matrix.create
        // This should be index of matrix.create function
        // For now, we'll assume it's registered at index 1
        instructions.push(Instruction::Call(1));
        instructions.push(Instruction::LocalSet(3)); // Save result matrix ptr to local 3
        
        // Check if matrix is not empty
        instructions.push(Instruction::LocalGet(1)); // rows
        instructions.push(Instruction::I32Const(0)); // 0
        instructions.push(Instruction::I32GtS); // rows > 0?
        
        instructions.push(Instruction::LocalGet(2)); // cols
        instructions.push(Instruction::I32Const(0)); // 0
        instructions.push(Instruction::I32GtS); // cols > 0?
        
        instructions.push(Instruction::I32And); // rows > 0 && cols > 0
        instructions.push(Instruction::If(BlockType::Empty));
        
        // Loop through rows
        instructions.push(Instruction::I32Const(0)); // row = 0
        instructions.push(Instruction::LocalSet(4)); // Store row in local 4
        
        instructions.push(Instruction::Block(BlockType::Empty));
        instructions.push(Instruction::Loop(BlockType::Empty));
        
        // Loop condition: row < rows?
        instructions.push(Instruction::LocalGet(4)); // row
        instructions.push(Instruction::LocalGet(1)); // rows
        instructions.push(Instruction::I32LtS); // row < rows?
        instructions.push(Instruction::I32Eqz); // !(row < rows)?
        instructions.push(Instruction::BrIf(1)); // Break if done with rows
        
        // Inner loop through columns
        instructions.push(Instruction::I32Const(0)); // col = 0
        instructions.push(Instruction::LocalSet(5)); // Store col in local 5
        
        instructions.push(Instruction::Block(BlockType::Empty));
        instructions.push(Instruction::Loop(BlockType::Empty));
        
        // Loop condition: col < cols?
        instructions.push(Instruction::LocalGet(5)); // col
        instructions.push(Instruction::LocalGet(2)); // cols
        instructions.push(Instruction::I32LtS); // col < cols?
        instructions.push(Instruction::I32Eqz); // !(col < cols)?
        instructions.push(Instruction::BrIf(1)); // Break if done with cols
        
        // Calculate source offset: 12 + (row * cols + col) * 8
        instructions.push(Instruction::LocalGet(4)); // row
        instructions.push(Instruction::LocalGet(2)); // cols
        instructions.push(Instruction::I32Mul); // row * cols
        instructions.push(Instruction::LocalGet(5)); // col
        instructions.push(Instruction::I32Add); // row * cols + col
        instructions.push(Instruction::I32Const(8)); // sizeof(f64)
        instructions.push(Instruction::I32Mul); // (row * cols + col) * 8
        instructions.push(Instruction::I32Const(12)); // header size
        instructions.push(Instruction::I32Add); // 12 + (row * cols + col) * 8
        
        // Add to source matrix pointer
        instructions.push(Instruction::LocalGet(0)); // matrix ptr
        instructions.push(Instruction::I32Add); // matrix ptr + offset
        
        // Load value from source matrix
        instructions.push(Instruction::F64Load(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
        instructions.push(Instruction::LocalSet(6)); // Save value to local 6
        
        // Calculate destination offset: 12 + (col * rows + row) * 8
        instructions.push(Instruction::LocalGet(5)); // col (becomes row in transpose)
        instructions.push(Instruction::LocalGet(1)); // rows (becomes cols in transpose)
        instructions.push(Instruction::I32Mul); // col * rows
        instructions.push(Instruction::LocalGet(4)); // row (becomes col in transpose)
        instructions.push(Instruction::I32Add); // col * rows + row
        instructions.push(Instruction::I32Const(8)); // sizeof(f64)
        instructions.push(Instruction::I32Mul); // (col * rows + row) * 8
        instructions.push(Instruction::I32Const(12)); // header size
        instructions.push(Instruction::I32Add); // 12 + (col * rows + row) * 8
        
        // Add to dest matrix pointer
        instructions.push(Instruction::LocalGet(3)); // result matrix ptr
        instructions.push(Instruction::I32Add); // result ptr + offset
        
        // Store value in dest matrix
        instructions.push(Instruction::LocalGet(6)); // value
        instructions.push(Instruction::F64Store(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
        
        // Increment col
        instructions.push(Instruction::LocalGet(5)); // col
        instructions.push(Instruction::I32Const(1)); // 1
        instructions.push(Instruction::I32Add); // col + 1
        instructions.push(Instruction::LocalSet(5)); // col = col + 1
        
        // Loop through cols
        instructions.push(Instruction::Br(0));
        
        // End inner loop (cols)
        instructions.push(Instruction::End);
        instructions.push(Instruction::End);
        
        // Increment row
        instructions.push(Instruction::LocalGet(4)); // row
        instructions.push(Instruction::I32Const(1)); // 1
        instructions.push(Instruction::I32Add); // row + 1
        instructions.push(Instruction::LocalSet(4)); // row = row + 1
        
        // Loop through rows
        instructions.push(Instruction::Br(0));
        
        // End outer loop (rows)
        instructions.push(Instruction::End);
        instructions.push(Instruction::End);
        
        // End if (matrix not empty)
        instructions.push(Instruction::End);
        
        // Return result matrix pointer
        instructions.push(Instruction::LocalGet(3));
        
        instructions
    }

    fn generate_matrix_determinant(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // Parameters:
        // - Local 0: matrix ptr (i32)
        
        // Load matrix dimensions
        instructions.push(Instruction::LocalGet(0)); // matrix ptr
        instructions.push(Instruction::I32Load(MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        })); // Load rows
        instructions.push(Instruction::LocalSet(1)); // Save rows to local 1
        
        instructions.push(Instruction::LocalGet(0)); // matrix ptr
        instructions.push(Instruction::I32Load(MemArg {
            offset: 4,
            align: 2,
            memory_index: 0,
        })); // Load cols
        instructions.push(Instruction::LocalSet(2)); // Save cols to local 2
        
        // Check if matrix is square (rows == cols)
        instructions.push(Instruction::LocalGet(1)); // rows
        instructions.push(Instruction::LocalGet(2)); // cols
        instructions.push(Instruction::I32Ne); // rows != cols
        
        // If matrix is not square, return NaN
        instructions.push(Instruction::If(BlockType::Result(ValType::F64)));
        instructions.push(Instruction::F64Const(f64::NAN));
        instructions.push(Instruction::Else);
        
        // Check matrix size
        instructions.push(Instruction::LocalGet(1)); // rows
        instructions.push(Instruction::I32Const(1)); // 1
        instructions.push(Instruction::I32Eq); // rows == 1
        instructions.push(Instruction::If(BlockType::Result(ValType::F64)));
        
        // 1x1 matrix: just return the single element
        instructions.push(Instruction::LocalGet(0)); // matrix ptr
        instructions.push(Instruction::I32Const(12)); // header size
        instructions.push(Instruction::I32Add); // matrix ptr + 12
        instructions.push(Instruction::F64Load(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
        
        instructions.push(Instruction::Else);
        
        // Check if it's a 2x2 matrix
        instructions.push(Instruction::LocalGet(1)); // rows
        instructions.push(Instruction::I32Const(2)); // 2
        instructions.push(Instruction::I32Eq); // rows == 2
        instructions.push(Instruction::If(BlockType::Result(ValType::F64)));
        
        // 2x2 matrix: det = a*d - b*c
        // Load a (0,0)
        instructions.push(Instruction::LocalGet(0)); // matrix ptr
        instructions.push(Instruction::I32Const(12)); // header size
        instructions.push(Instruction::I32Add); // matrix ptr + 12
        instructions.push(Instruction::F64Load(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
        instructions.push(Instruction::LocalSet(3)); // Save a to local 3
        
        // Load d (1,1)
        instructions.push(Instruction::LocalGet(0)); // matrix ptr
        instructions.push(Instruction::I32Const(12 + 8 * 3)); // header + (1*2 + 1)*8
        instructions.push(Instruction::I32Add); // matrix ptr + offset
        instructions.push(Instruction::F64Load(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
        instructions.push(Instruction::LocalSet(4)); // Save d to local 4
        
        // Load b (0,1)
        instructions.push(Instruction::LocalGet(0)); // matrix ptr
        instructions.push(Instruction::I32Const(12 + 8 * 1)); // header + (0*2 + 1)*8
        instructions.push(Instruction::I32Add); // matrix ptr + offset
        instructions.push(Instruction::F64Load(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
        instructions.push(Instruction::LocalSet(5)); // Save b to local 5
        
        // Load c (1,0)
        instructions.push(Instruction::LocalGet(0)); // matrix ptr
        instructions.push(Instruction::I32Const(12 + 8 * 2)); // header + (1*2 + 0)*8
        instructions.push(Instruction::I32Add); // matrix ptr + offset
        instructions.push(Instruction::F64Load(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
        instructions.push(Instruction::LocalSet(6)); // Save c to local 6
        
        // Calculate a*d
        instructions.push(Instruction::LocalGet(3)); // a
        instructions.push(Instruction::LocalGet(4)); // d
        instructions.push(Instruction::F64Mul); // a*d
        
        // Calculate b*c
        instructions.push(Instruction::LocalGet(5)); // b
        instructions.push(Instruction::LocalGet(6)); // c
        instructions.push(Instruction::F64Mul); // b*c
        
        // Calculate a*d - b*c
        instructions.push(Instruction::F64Sub); // a*d - b*c
        
        instructions.push(Instruction::Else);
        
        // Check if it's a 3x3 matrix
        instructions.push(Instruction::LocalGet(1)); // rows
        instructions.push(Instruction::I32Const(3)); // 3
        instructions.push(Instruction::I32Eq); // rows == 3
        instructions.push(Instruction::If(BlockType::Result(ValType::F64)));
        
        // 3x3 matrix: det = a(ei-fh) - b(di-fg) + c(dh-eg)
        // Load all 9 elements into locals 3-11
        
        // For simplicity in this implementation, we'll load them in order:
        // a(0,0) -> local 3, b(0,1) -> local 4, c(0,2) -> local 5
        // d(1,0) -> local 6, e(1,1) -> local 7, f(1,2) -> local 8
        // g(2,0) -> local 9, h(2,1) -> local 10, i(2,2) -> local 11
        
        // Load a (0,0)
        instructions.push(Instruction::LocalGet(0)); // matrix ptr
        instructions.push(Instruction::I32Const(12 + 8 * 0)); // header + (0*3 + 0)*8
        instructions.push(Instruction::I32Add); // matrix ptr + offset
        instructions.push(Instruction::F64Load(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
        instructions.push(Instruction::LocalSet(3)); // Save a to local 3
        
        // Load b (0,1)
        instructions.push(Instruction::LocalGet(0)); // matrix ptr
        instructions.push(Instruction::I32Const(12 + 8 * 1)); // header + (0*3 + 1)*8
        instructions.push(Instruction::I32Add); // matrix ptr + offset
        instructions.push(Instruction::F64Load(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
        instructions.push(Instruction::LocalSet(4)); // Save b to local 4
        
        // Load c (0,2)
        instructions.push(Instruction::LocalGet(0)); // matrix ptr
        instructions.push(Instruction::I32Const(12 + 8 * 2)); // header + (0*3 + 2)*8
        instructions.push(Instruction::I32Add); // matrix ptr + offset
        instructions.push(Instruction::F64Load(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
        instructions.push(Instruction::LocalSet(5)); // Save c to local 5
        
        // Load d (1,0)
        instructions.push(Instruction::LocalGet(0)); // matrix ptr
        instructions.push(Instruction::I32Const(12 + 8 * 3)); // header + (1*3 + 0)*8
        instructions.push(Instruction::I32Add); // matrix ptr + offset
        instructions.push(Instruction::F64Load(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
        instructions.push(Instruction::LocalSet(6)); // Save d to local 6
        
        // Load e (1,1)
        instructions.push(Instruction::LocalGet(0)); // matrix ptr
        instructions.push(Instruction::I32Const(12 + 8 * 4)); // header + (1*3 + 1)*8
        instructions.push(Instruction::I32Add); // matrix ptr + offset
        instructions.push(Instruction::F64Load(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
        instructions.push(Instruction::LocalSet(7)); // Save e to local 7
        
        // Load f (1,2)
        instructions.push(Instruction::LocalGet(0)); // matrix ptr
        instructions.push(Instruction::I32Const(12 + 8 * 5)); // header + (1*3 + 2)*8
        instructions.push(Instruction::I32Add); // matrix ptr + offset
        instructions.push(Instruction::F64Load(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
        instructions.push(Instruction::LocalSet(8)); // Save f to local 8
        
        // Load g (2,0)
        instructions.push(Instruction::LocalGet(0)); // matrix ptr
        instructions.push(Instruction::I32Const(12 + 8 * 6)); // header + (2*3 + 0)*8
        instructions.push(Instruction::I32Add); // matrix ptr + offset
        instructions.push(Instruction::F64Load(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
        instructions.push(Instruction::LocalSet(9)); // Save g to local 9
        
        // Load h (2,1)
        instructions.push(Instruction::LocalGet(0)); // matrix ptr
        instructions.push(Instruction::I32Const(12 + 8 * 7)); // header + (2*3 + 1)*8
        instructions.push(Instruction::I32Add); // matrix ptr + offset
        instructions.push(Instruction::F64Load(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
        instructions.push(Instruction::LocalSet(10)); // Save h to local 10
        
        // Load i (2,2)
        instructions.push(Instruction::LocalGet(0)); // matrix ptr
        instructions.push(Instruction::I32Const(12 + 8 * 8)); // header + (2*3 + 2)*8
        instructions.push(Instruction::I32Add); // matrix ptr + offset
        instructions.push(Instruction::F64Load(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
        instructions.push(Instruction::LocalSet(11)); // Save i to local 11
        
        // Calculate det = a(ei-fh) - b(di-fg) + c(dh-eg)
        
        // Calculate (ei-fh)
        instructions.push(Instruction::LocalGet(7)); // e
        instructions.push(Instruction::LocalGet(11)); // i
        instructions.push(Instruction::F64Mul); // e*i
        
        instructions.push(Instruction::LocalGet(8)); // f
        instructions.push(Instruction::LocalGet(10)); // h
        instructions.push(Instruction::F64Mul); // f*h
        
        instructions.push(Instruction::F64Sub); // ei-fh
        instructions.push(Instruction::LocalSet(12)); // Save ei-fh to local 12
        
        // Calculate (di-fg)
        instructions.push(Instruction::LocalGet(6)); // d
        instructions.push(Instruction::LocalGet(11)); // i
        instructions.push(Instruction::F64Mul); // d*i
        
        instructions.push(Instruction::LocalGet(8)); // f
        instructions.push(Instruction::LocalGet(9)); // g
        instructions.push(Instruction::F64Mul); // f*g
        
        instructions.push(Instruction::F64Sub); // di-fg
        instructions.push(Instruction::LocalSet(13)); // Save di-fg to local 13
        
        // Calculate (dh-eg)
        instructions.push(Instruction::LocalGet(6)); // d
        instructions.push(Instruction::LocalGet(10)); // h
        instructions.push(Instruction::F64Mul); // d*h
        
        instructions.push(Instruction::LocalGet(7)); // e
        instructions.push(Instruction::LocalGet(9)); // g
        instructions.push(Instruction::F64Mul); // e*g
        
        instructions.push(Instruction::F64Sub); // dh-eg
        instructions.push(Instruction::LocalSet(14)); // Save dh-eg to local 14
        
        // Calculate a(ei-fh)
        instructions.push(Instruction::LocalGet(3)); // a
        instructions.push(Instruction::LocalGet(12)); // ei-fh
        instructions.push(Instruction::F64Mul); // a*(ei-fh)
        
        // Calculate b(di-fg)
        instructions.push(Instruction::LocalGet(4)); // b
        instructions.push(Instruction::LocalGet(13)); // di-fg
        instructions.push(Instruction::F64Mul); // b*(di-fg)
        
        // Calculate a(ei-fh) - b(di-fg)
        instructions.push(Instruction::F64Sub); // a*(ei-fh) - b*(di-fg)
        
        // Calculate c(dh-eg)
        instructions.push(Instruction::LocalGet(5)); // c
        instructions.push(Instruction::LocalGet(14)); // dh-eg
        instructions.push(Instruction::F64Mul); // c*(dh-eg)
        
        // Calculate a(ei-fh) - b(di-fg) + c(dh-eg)
        instructions.push(Instruction::F64Add); // a*(ei-fh) - b*(di-fg) + c*(dh-eg)
        
        instructions.push(Instruction::Else);
        
        // Matrices larger than 3x3 - return NaN to indicate error
        // In a real implementation, we would use a recursive method for larger matrices
        instructions.push(Instruction::F64Const(f64::NAN));
        
        // End 3x3 check
        instructions.push(Instruction::End);
        
        // End 2x2 check
        instructions.push(Instruction::End);
        
        // End 1x1 check
        instructions.push(Instruction::End);
        
        // End square check
        instructions.push(Instruction::End);
        
        instructions
    }

    fn generate_matrix_inverse(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // Parameters:
        // - Local 0: matrix ptr (i32)
        
        // Load matrix dimensions
        instructions.push(Instruction::LocalGet(0)); // matrix ptr
        instructions.push(Instruction::I32Load(MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        })); // Load rows
        instructions.push(Instruction::LocalSet(1)); // Save rows to local 1
        
        instructions.push(Instruction::LocalGet(0)); // matrix ptr
        instructions.push(Instruction::I32Load(MemArg {
            offset: 4,
            align: 2,
            memory_index: 0,
        })); // Load cols
        instructions.push(Instruction::LocalSet(2)); // Save cols to local 2
        
        // Check if matrix is square (rows == cols)
        instructions.push(Instruction::LocalGet(1)); // rows
        instructions.push(Instruction::LocalGet(2)); // cols
        instructions.push(Instruction::I32Ne); // rows != cols
        
        // If matrix is not square, return 0 (error)
        instructions.push(Instruction::If(BlockType::Result(ValType::I32)));
        instructions.push(Instruction::I32Const(0));
        instructions.push(Instruction::Else);
        
        // First, calculate the determinant (needed to check if matrix is invertible)
        // We'll reuse the same code pattern as in the determinant function
        
        // Check for 1x1 matrix
        instructions.push(Instruction::LocalGet(1)); // rows
        instructions.push(Instruction::I32Const(1)); // 1
        instructions.push(Instruction::I32Eq); // rows == 1
        instructions.push(Instruction::If(BlockType::Result(ValType::I32)));
        
        // For 1x1 matrix: inverse = 1/a
        // Load the element
        instructions.push(Instruction::LocalGet(0)); // matrix ptr
        instructions.push(Instruction::I32Const(12)); // header size
        instructions.push(Instruction::I32Add); // matrix ptr + 12
        instructions.push(Instruction::F64Load(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
        instructions.push(Instruction::LocalSet(3)); // Save a to local 3
        
        // Check if a == 0 (not invertible)
        instructions.push(Instruction::LocalGet(3)); // a
        instructions.push(Instruction::F64Const(0.0));
        instructions.push(Instruction::F64Eq); // a == 0?
        instructions.push(Instruction::If(BlockType::Result(ValType::I32)));
        
        // Not invertible, return 0
        instructions.push(Instruction::I32Const(0));
        
        instructions.push(Instruction::Else);
        
        // Create a new 1x1 matrix for the result
        instructions.push(Instruction::I32Const(1)); // rows
        instructions.push(Instruction::I32Const(1)); // cols
        
        // Call matrix.create
        instructions.push(Instruction::Call(1));
        instructions.push(Instruction::LocalSet(4)); // Save result matrix ptr to local 4
        
        // Calculate 1/a
        instructions.push(Instruction::F64Const(1.0));
        instructions.push(Instruction::LocalGet(3)); // a
        instructions.push(Instruction::F64Div); // 1/a
        
        // Store 1/a in result matrix
        instructions.push(Instruction::LocalGet(4)); // result ptr
        instructions.push(Instruction::I32Const(12)); // header size
        instructions.push(Instruction::I32Add); // result ptr + 12
        instructions.push(Instruction::F64Store(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
        
        // Return the result matrix pointer
        instructions.push(Instruction::LocalGet(4));
        
        instructions.push(Instruction::End);
        
        instructions.push(Instruction::Else);
        
        // Check for 2x2 matrix
        instructions.push(Instruction::LocalGet(1)); // rows
        instructions.push(Instruction::I32Const(2)); // 2
        instructions.push(Instruction::I32Eq); // rows == 2
        instructions.push(Instruction::If(BlockType::Result(ValType::I32)));
        
        // For 2x2 matrix: inverse = 1/det * [d, -b; -c, a]
        
        // Load all 4 elements
        // Load a (0,0)
        instructions.push(Instruction::LocalGet(0)); // matrix ptr
        instructions.push(Instruction::I32Const(12)); // header size
        instructions.push(Instruction::I32Add); // matrix ptr + 12
        instructions.push(Instruction::F64Load(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
        instructions.push(Instruction::LocalSet(3)); // Save a to local 3
        
        // Load b (0,1)
        instructions.push(Instruction::LocalGet(0)); // matrix ptr
        instructions.push(Instruction::I32Const(12 + 8 * 1)); // header + (0*2 + 1)*8
        instructions.push(Instruction::I32Add); // matrix ptr + offset
        instructions.push(Instruction::F64Load(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
        instructions.push(Instruction::LocalSet(4)); // Save b to local 4
        
        // Load c (1,0)
        instructions.push(Instruction::LocalGet(0)); // matrix ptr
        instructions.push(Instruction::I32Const(12 + 8 * 2)); // header + (1*2 + 0)*8
        instructions.push(Instruction::I32Add); // matrix ptr + offset
        instructions.push(Instruction::F64Load(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
        instructions.push(Instruction::LocalSet(5)); // Save c to local 5
        
        // Load d (1,1)
        instructions.push(Instruction::LocalGet(0)); // matrix ptr
        instructions.push(Instruction::I32Const(12 + 8 * 3)); // header + (1*2 + 1)*8
        instructions.push(Instruction::I32Add); // matrix ptr + offset
        instructions.push(Instruction::F64Load(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
        instructions.push(Instruction::LocalSet(6)); // Save d to local 6
        
        // Calculate determinant: det = a*d - b*c
        instructions.push(Instruction::LocalGet(3)); // a
        instructions.push(Instruction::LocalGet(6)); // d
        instructions.push(Instruction::F64Mul); // a*d
        
        instructions.push(Instruction::LocalGet(4)); // b
        instructions.push(Instruction::LocalGet(5)); // c
        instructions.push(Instruction::F64Mul); // b*c
        
        instructions.push(Instruction::F64Sub); // a*d - b*c
        instructions.push(Instruction::LocalSet(7)); // Save det to local 7
        
        // Check if determinant is zero (not invertible)
        instructions.push(Instruction::LocalGet(7)); // det
        instructions.push(Instruction::F64Const(0.0));
        instructions.push(Instruction::F64Eq); // det == 0?
        instructions.push(Instruction::If(BlockType::Result(ValType::I32)));
        
        // Not invertible, return 0
        instructions.push(Instruction::I32Const(0));
        
        instructions.push(Instruction::Else);
        
        // Create a new 2x2 matrix for the result
        instructions.push(Instruction::I32Const(2)); // rows
        instructions.push(Instruction::I32Const(2)); // cols
        
        // Call matrix.create
        instructions.push(Instruction::Call(1));
        instructions.push(Instruction::LocalSet(8)); // Save result matrix ptr to local 8
        
        // Calculate 1/det
        instructions.push(Instruction::F64Const(1.0));
        instructions.push(Instruction::LocalGet(7)); // det
        instructions.push(Instruction::F64Div); // 1/det
        instructions.push(Instruction::LocalSet(9)); // Save 1/det to local 9
        
        // Calculate and store elements of the inverse matrix
        
        // inv(0,0) = d/det
        instructions.push(Instruction::LocalGet(6)); // d
        instructions.push(Instruction::LocalGet(9)); // 1/det
        instructions.push(Instruction::F64Mul); // d/det
        
        instructions.push(Instruction::LocalGet(8)); // result ptr
        instructions.push(Instruction::I32Const(12)); // header + (0*2 + 0)*8
        instructions.push(Instruction::I32Add); // result ptr + offset
        instructions.push(Instruction::F64Store(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
        
        // inv(0,1) = -b/det
        instructions.push(Instruction::LocalGet(4)); // b
        instructions.push(Instruction::F64Neg); // -b
        instructions.push(Instruction::LocalGet(9)); // 1/det
        instructions.push(Instruction::F64Mul); // -b/det
        
        instructions.push(Instruction::LocalGet(8)); // result ptr
        instructions.push(Instruction::I32Const(12 + 8 * 1)); // header + (0*2 + 1)*8
        instructions.push(Instruction::I32Add); // result ptr + offset
        instructions.push(Instruction::F64Store(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
        
        // inv(1,0) = -c/det
        instructions.push(Instruction::LocalGet(5)); // c
        instructions.push(Instruction::F64Neg); // -c
        instructions.push(Instruction::LocalGet(9)); // 1/det
        instructions.push(Instruction::F64Mul); // -c/det
        
        instructions.push(Instruction::LocalGet(8)); // result ptr
        instructions.push(Instruction::I32Const(12 + 8 * 2)); // header + (1*2 + 0)*8
        instructions.push(Instruction::I32Add); // result ptr + offset
        instructions.push(Instruction::F64Store(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
        
        // inv(1,1) = a/det
        instructions.push(Instruction::LocalGet(3)); // a
        instructions.push(Instruction::LocalGet(9)); // 1/det
        instructions.push(Instruction::F64Mul); // a/det
        
        instructions.push(Instruction::LocalGet(8)); // result ptr
        instructions.push(Instruction::I32Const(12 + 8 * 3)); // header + (1*2 + 1)*8
        instructions.push(Instruction::I32Add); // result ptr + offset
        instructions.push(Instruction::F64Store(MemArg {
            offset: 0,
            align: 3,
            memory_index: 0,
        }));
        
        // Return the result matrix pointer
        instructions.push(Instruction::LocalGet(8));
        
        instructions.push(Instruction::End);
        
        instructions.push(Instruction::Else);
        
        // For larger matrices (3x3 and bigger), we don't implement the full algorithm here
        // In a full implementation, we would use the adjugate method or Gaussian elimination
        // For now, we'll return 0 to indicate "not implemented" for matrices larger than 2x2
        
        instructions.push(Instruction::I32Const(0));
        
        // End 2x2 check
        instructions.push(Instruction::End);
        
        // End 1x1 check
        instructions.push(Instruction::End);
        
        // End square check
        instructions.push(Instruction::End);
        
        instructions
    }

    // Helper functions to provide better error reporting
    
    // This function can be called from host to generate a descriptive error message for matrix dimensions
    fn generate_matrix_dimension_error(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // Parameters:
        // - Local 0: rows1 (i32)
        // - Local 1: cols1 (i32)
        // - Local 2: rows2 (i32)
        // - Local 3: cols2 (i32)
        
        // Allocate memory for the error message
        // In a real implementation, we would format a detailed error message
        // For now, we'll return an error code
        
        instructions.push(Instruction::I32Const(1)); // Error code 1 for dimension mismatch
        
        instructions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::CodeGenerator;
    use crate::stdlib::memory::MemoryManager;

    #[test]
    fn test_matrix_operations() {
        let mut codegen = CodeGenerator::new();
        let memory = MemoryManager::new(16, Some(1024)); // 16 pages, heap starts at 1024
        let matrix_ops = MatrixOperations::new(1024);

        // Register matrix functions
        matrix_ops.register_functions(&mut codegen).unwrap();

        // Test matrix creation
        let matrix_ptr = memory.allocate(24, MATRIX_TYPE_ID).unwrap(); // 8 bytes metadata + 4 * 8 bytes for elements
        assert!(matrix_ptr >= 1024);

        // Test matrix get/set
        unsafe {
            let matrix_ptr = matrix_ptr as *mut f64;
            *matrix_ptr.add(0) = 1.0;
            *matrix_ptr.add(1) = 2.0;
            *matrix_ptr.add(2) = 3.0;
            *matrix_ptr.add(3) = 4.0;
        }

        // Test matrix operations through WASM
        let engine = wasmtime::Engine::default();
        let wasm_bytes = codegen.finish();
        let module = wasmtime::Module::new(&engine, &wasm_bytes).unwrap();
        let mut store = wasmtime::Store::new(&engine, ());
        let instance = wasmtime::Instance::new(&mut store, &module, &[]).unwrap();

        // Test matrix.create
        let create_matrix = instance.get_func(&mut store, "matrix.create").unwrap();
        let mut results = vec![wasmtime::Val::I32(0)];
        create_matrix.call(&mut store, &[
            wasmtime::Val::I32(2),
            wasmtime::Val::I32(2),
        ], &mut results).unwrap();
        let new_matrix_ptr = results[0].unwrap_i32() as usize;
        assert!(new_matrix_ptr >= 1024);
    }

    #[test]
    fn test_matrix_determinant() {
        let mut codegen = CodeGenerator::new();
        let memory = MemoryManager::new(16, Some(1024));
        let matrix_ops = MatrixOperations::new(1024);
        matrix_ops.register_functions(&mut codegen).unwrap();

        // Create a 2x2 matrix
        let matrix_ptr = memory.allocate(24, MATRIX_TYPE_ID).unwrap(); // 8 bytes metadata + 4 * 8 bytes for elements
        unsafe {
            let ptr = matrix_ptr as *mut i32;
            *ptr = 2; // rows
            *ptr.add(1) = 2; // cols
            
            let data_ptr = (matrix_ptr + 8) as *mut f64;
            *data_ptr = 1.0; // a11
            *data_ptr.add(1) = 2.0; // a12
            *data_ptr.add(2) = 3.0; // a21
            *data_ptr.add(3) = 4.0; // a22
        }

        // Test through WASM
        let engine = wasmtime::Engine::default();
        let wasm_bytes = codegen.finish();
        let module = wasmtime::Module::new(&engine, &wasm_bytes).unwrap();
        let mut store = wasmtime::Store::new(&engine, ());
        let instance = wasmtime::Instance::new(&mut store, &module, &[]).unwrap();

        let determinant = instance.get_func(&mut store, "matrix.determinant").unwrap();
        let mut results = vec![wasmtime::Val::F64(0.0f64.to_bits())];
        determinant.call(&mut store, &[wasmtime::Val::I32(matrix_ptr as i32)], &mut results).unwrap();
        
        let det = f64::from_bits(results[0].unwrap_i64() as u64);
        assert!((det - (-2.0)).abs() < f64::EPSILON); // 1*4 - 2*3 = -2
    }

    #[test]
    fn test_matrix_inverse() {
        let mut codegen = CodeGenerator::new();
        let memory = MemoryManager::new(16, Some(1024));
        let matrix_ops = MatrixOperations::new(1024);
        matrix_ops.register_functions(&mut codegen).unwrap();

        // Create a 2x2 invertible matrix
        let matrix_ptr = memory.allocate(24, MATRIX_TYPE_ID).unwrap(); // 8 bytes metadata + 4 * 8 bytes for elements
        unsafe {
            let ptr = matrix_ptr as *mut i32;
            *ptr = 2; // rows
            *ptr.add(1) = 2; // cols
            
            let data_ptr = (matrix_ptr + 8) as *mut f64;
            *data_ptr = 4.0; // a11
            *data_ptr.add(1) = 7.0; // a12
            *data_ptr.add(2) = 2.0; // a21
            *data_ptr.add(3) = 6.0; // a22
        }

        // Test through WASM
        let engine = wasmtime::Engine::default();
        let wasm_bytes = codegen.finish();
        let module = wasmtime::Module::new(&engine, &wasm_bytes).unwrap();
        let mut store = wasmtime::Store::new(&engine, ());
        let instance = wasmtime::Instance::new(&mut store, &module, &[]).unwrap();

        let inverse = instance.get_func(&mut store, "matrix.inverse").unwrap();
        let mut results = vec![wasmtime::Val::I32(0)];
        inverse.call(&mut store, &[wasmtime::Val::I32(matrix_ptr as i32)], &mut results).unwrap();
        
        let inverse_ptr = results[0].unwrap_i32() as usize;
        assert!(inverse_ptr >= 1024);

        // Verify inverse elements
        unsafe {
            let data_ptr = (inverse_ptr + 8) as *const f64;
            assert!(((*data_ptr) - 0.6).abs() < f64::EPSILON);
            assert!(((*data_ptr.add(1)) + 0.7).abs() < f64::EPSILON);
            assert!(((*data_ptr.add(2)) - 0.2).abs() < f64::EPSILON);
            assert!(((*data_ptr.add(3)) - 0.4).abs() < f64::EPSILON);
        }
    }

    #[test]
    fn test_matrix_bounds_checking() {
        let mut codegen = CodeGenerator::new();
        let memory = MemoryManager::new(16, Some(1024));
        let matrix_ops = MatrixOperations::new(1024);
        matrix_ops.register_functions(&mut codegen).unwrap();

        // Create a 2x2 matrix
        let matrix_ptr = memory.allocate(24, MATRIX_TYPE_ID).unwrap(); // 8 bytes metadata + 4 * 8 bytes for elements
        unsafe {
            let ptr = matrix_ptr as *mut i32;
            *ptr = 2; // rows
            *ptr.add(1) = 2; // cols
            
            let data_ptr = (matrix_ptr + 8) as *mut f64;
            *data_ptr = 1.0;
            *data_ptr.add(1) = 2.0;
            *data_ptr.add(2) = 3.0;
            *data_ptr.add(3) = 4.0;
        }

        // Test through WASM
        let engine = wasmtime::Engine::default();
        let wasm_bytes = codegen.finish();
        let module = wasmtime::Module::new(&engine, &wasm_bytes).unwrap();
        let mut store = wasmtime::Store::new(&engine, ());
        let instance = wasmtime::Instance::new(&mut store, &module, &[]).unwrap();

        // Test get bounds checking
        let get = instance.get_func(&mut store, "matrix.get").unwrap();
        let mut results = vec![wasmtime::Val::F64(0.0f64.to_bits())];

        // Valid access
        get.call(&mut store, &[
            wasmtime::Val::I32(matrix_ptr as i32),
            wasmtime::Val::I32(0),
            wasmtime::Val::I32(0),
        ], &mut results).unwrap();
        let result_value = f64::from_bits(results[0].unwrap_i64() as u64);
        assert!((result_value - 1.0).abs() < f64::EPSILON);

        // Out of bounds row
        get.call(&mut store, &[
            wasmtime::Val::I32(matrix_ptr as i32),
            wasmtime::Val::I32(2),
            wasmtime::Val::I32(0),
        ], &mut results).unwrap();
        let result_value = f64::from_bits(results[0].unwrap_i64() as u64);
        assert!(result_value.is_nan());

        // Out of bounds column
        get.call(&mut store, &[
            wasmtime::Val::I32(matrix_ptr as i32),
            wasmtime::Val::I32(0),
            wasmtime::Val::I32(2),
        ], &mut results).unwrap();
        let result_value = f64::from_bits(results[0].unwrap_i64() as u64);
        assert!(result_value.is_nan());

        // Test set bounds checking
        let set = instance.get_func(&mut store, "matrix.set").unwrap();
        let mut results = vec![wasmtime::Val::I32(0)];

        // Valid set
        set.call(&mut store, &[
            wasmtime::Val::I32(matrix_ptr as i32),
            wasmtime::Val::I32(0),
            wasmtime::Val::I32(0),
            wasmtime::Val::F64(5.0f64.to_bits()),
        ], &mut results).unwrap();
        assert_eq!(results[0].unwrap_i32(), 1);

        // Out of bounds row
        set.call(&mut store, &[
            wasmtime::Val::I32(matrix_ptr as i32),
            wasmtime::Val::I32(2),
            wasmtime::Val::I32(0),
            wasmtime::Val::F64(5.0f64.to_bits()),
        ], &mut results).unwrap();
        assert_eq!(results[0].unwrap_i32(), 0);

        // Out of bounds column
        set.call(&mut store, &[
            wasmtime::Val::I32(matrix_ptr as i32),
            wasmtime::Val::I32(0),
            wasmtime::Val::I32(2),
            wasmtime::Val::F64(5.0f64.to_bits()),
        ], &mut results).unwrap();
        assert_eq!(results[0].unwrap_i32(), 0);
    }
}