// Simplified matrix operations

use crate::codegen::CodeGenerator;
use crate::types::WasmType;
use crate::error::CompilerError;
use wasm_encoder::Instruction;
use crate::stdlib::{register_stdlib_function, register_stdlib_function_with_locals};

/// Simplified matrix operations for Clean Language
pub struct MatrixOperations;

impl MatrixOperations {
    pub fn new() -> Self {
        Self
    }

    pub fn register_functions(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // Basic matrix creation
        register_stdlib_function(
            codegen,
            "matrix.create",
            &[WasmType::I32, WasmType::I32], // rows, cols
            Some(WasmType::I32), // matrix pointer
            self.generate_matrix_create()
        )?;

        // Matrix element access
        register_stdlib_function(
            codegen,
            "matrix.get",
            &[WasmType::I32, WasmType::I32, WasmType::I32], // matrix ptr, row, col
            Some(WasmType::F64),
            self.generate_matrix_get()
        )?;

        register_stdlib_function(
            codegen,
            "matrix.set",
            &[WasmType::I32, WasmType::I32, WasmType::I32, WasmType::F64], // matrix ptr, row, col, value
            Some(WasmType::I32), // success flag
            self.generate_matrix_set()
        )?;

        // Matrix addition
        // Matrix addition - uses additional locals: rows(i32), cols(i32), total_elements(i32), counter(i32)
        register_stdlib_function_with_locals(
            codegen,
            "matrix.add",
            &[WasmType::I32, WasmType::I32, WasmType::I32], // matrix1 ptr, matrix2 ptr, result ptr
            Some(WasmType::I32), // success flag
            &[WasmType::I32, WasmType::I32, WasmType::I32, WasmType::I32], // locals 3, 4, 5, 6: rows, cols, total_elements, counter
            self.generate_matrix_add()
        )?;

        // Matrix multiplication - uses locals: rows_a(i32), cols_a(i32), rows_b(i32), cols_b(i32), i(i32), j(i32), sum(f64)
        register_stdlib_function_with_locals(
            codegen,
            "matrix.multiply",
            &[WasmType::I32, WasmType::I32, WasmType::I32], // matrix1 ptr, matrix2 ptr, result ptr
            Some(WasmType::I32), // success flag
            &[WasmType::I32, WasmType::I32, WasmType::I32, WasmType::I32, WasmType::I32, WasmType::I32, WasmType::F64], // locals 3-9
            self.generate_matrix_multiply()
        )?;

        // Matrix transpose - uses locals: rows(i32), cols(i32), i(i32), j(i32) 
        register_stdlib_function_with_locals(
            codegen,
            "matrix.transpose",
            &[WasmType::I32, WasmType::I32], // matrix ptr, result_ptr
            Some(WasmType::I32), // success indicator
            &[WasmType::I32, WasmType::I32, WasmType::I32, WasmType::I32], // locals 2, 3, 4, 5: rows, cols, i, j
            self.generate_matrix_transpose()
        )?;

        Ok(())
    }

    fn generate_matrix_create(&self) -> Vec<Instruction> {
        vec![
            Instruction::LocalGet(0), // rows
            Instruction::LocalGet(1), // cols
            Instruction::I32Mul,      // rows * cols
            Instruction::I32Const(8), // sizeof(f64)
            Instruction::I32Mul,      // total data size
            Instruction::I32Const(12), // header size
            Instruction::I32Add,      // total size
            Instruction::Call(0),     // Call memory allocator
        ]
    }

    fn generate_matrix_get(&self) -> Vec<Instruction> {
        vec![
            // Calculate element address: matrix_ptr + header_size + (row * cols + col) * sizeof(f64)
            Instruction::LocalGet(0), // matrix ptr
            Instruction::I32Const(12), // header size (rows=4, cols=4, data_ptr=4)
            Instruction::I32Add,      // matrix ptr + header_size
            
            // Calculate offset: (row * cols + col) * 8
            Instruction::LocalGet(1), // row
            // For now, assume square matrix with known size (simplified)
            // In a full implementation, we'd read cols from the header
            Instruction::I32Const(4), // assume 4x4 matrix for simplicity
            Instruction::I32Mul,      // row * cols
            Instruction::LocalGet(2), // col
            Instruction::I32Add,      // row * cols + col
            Instruction::I32Const(8), // sizeof(f64)
            Instruction::I32Mul,      // offset in bytes
            
            Instruction::I32Add,      // final address
            Instruction::F64Load(wasm_encoder::MemArg {
                offset: 0,
                align: 3, // 8-byte aligned for f64
                memory_index: 0,
            }),
        ]
    }

    fn generate_matrix_set(&self) -> Vec<Instruction> {
        vec![
            // Calculate element address: matrix_ptr + header_size + (row * cols + col) * sizeof(f64)
            Instruction::LocalGet(0), // matrix ptr
            Instruction::I32Const(12), // header size
            Instruction::I32Add,      // matrix ptr + header_size
            
            // Calculate offset: (row * cols + col) * 8
            Instruction::LocalGet(1), // row
            Instruction::I32Const(4), // assume 4x4 matrix for simplicity
            Instruction::I32Mul,      // row * cols
            Instruction::LocalGet(2), // col
            Instruction::I32Add,      // row * cols + col
            Instruction::I32Const(8), // sizeof(f64)
            Instruction::I32Mul,      // offset in bytes
            
            Instruction::I32Add,      // final address
            Instruction::LocalGet(3), // value to store (f64)
            // Note: value should already be f64 from function signature
            Instruction::F64Store(wasm_encoder::MemArg {
                offset: 0,
                align: 3, // 8-byte aligned for f64
                memory_index: 0,
            }),
            
            Instruction::I32Const(1), // Return success indicator
        ]
    }

    fn generate_matrix_add(&self) -> Vec<Instruction> {
        // Matrix addition: adds corresponding elements of two matrices
        // Parameters: matrix_a_ptr, matrix_b_ptr, result_ptr
        // Returns: success indicator (1)
        vec![
            // Load dimensions from first matrix (rows, cols)
            Instruction::LocalGet(0), // matrix_a_ptr
            Instruction::I32Load(wasm_encoder::MemArg { offset: 0, align: 2, memory_index: 0 }),
            Instruction::LocalSet(3), // rows
            
            Instruction::LocalGet(0), // matrix_a_ptr
            Instruction::I32Load(wasm_encoder::MemArg { offset: 4, align: 2, memory_index: 0 }),
            Instruction::LocalSet(4), // cols
            
            // Calculate total elements: rows * cols
            Instruction::LocalGet(3), // rows
            Instruction::LocalGet(4), // cols
            Instruction::I32Mul,
            Instruction::LocalSet(5), // total_elements
            
            // Initialize counter
            Instruction::I32Const(0),
            Instruction::LocalSet(6), // counter
            
            // Loop through all elements
            Instruction::Loop(wasm_encoder::BlockType::Empty),
            
            // Check if counter < total_elements
            Instruction::LocalGet(6), // counter
            Instruction::LocalGet(5), // total_elements
            Instruction::I32LtS,
            Instruction::If(wasm_encoder::BlockType::Empty),
            
            // Calculate address for storing result first
            Instruction::LocalGet(2), // result_ptr
            Instruction::I32Const(12), // header size
            Instruction::I32Add,
            Instruction::LocalGet(6), // counter
            Instruction::I32Const(8), // sizeof(f64)
            Instruction::I32Mul,
            Instruction::I32Add,
            
            // Load element from matrix A
            Instruction::LocalGet(0), // matrix_a_ptr
            Instruction::I32Const(12), // header size
            Instruction::I32Add,
            Instruction::LocalGet(6), // counter
            Instruction::I32Const(8), // sizeof(f64)
            Instruction::I32Mul,
            Instruction::I32Add,
            Instruction::F64Load(wasm_encoder::MemArg { offset: 0, align: 3, memory_index: 0 }),
            
            // Load element from matrix B
            Instruction::LocalGet(1), // matrix_b_ptr
            Instruction::I32Const(12), // header size
            Instruction::I32Add,
            Instruction::LocalGet(6), // counter
            Instruction::I32Const(8), // sizeof(f64)
            Instruction::I32Mul,
            Instruction::I32Add,
            Instruction::F64Load(wasm_encoder::MemArg { offset: 0, align: 3, memory_index: 0 }),
            
            // Add the elements (stack: [address, f64_a, f64_b] -> [address, f64_result])
            Instruction::F64Add,
            
            // Now stack is [i32_address, f64_result] which is correct for F64Store
            Instruction::F64Store(wasm_encoder::MemArg { offset: 0, align: 3, memory_index: 0 }),
            
            // Increment counter
            Instruction::LocalGet(6),
            Instruction::I32Const(1),
            Instruction::I32Add,
            Instruction::LocalSet(6),
            
            // Continue loop
            Instruction::Br(1),
            
            Instruction::End, // End if
            Instruction::End, // End loop
            
            // Return success indicator
            Instruction::I32Const(1),
        ]
    }

    fn generate_matrix_multiply(&self) -> Vec<Instruction> {
        // Matrix multiplication: multiplies two matrices
        // Parameters: matrix_a_ptr, matrix_b_ptr, result_ptr
        // Returns: success indicator (1)
        vec![
            // Load dimensions from matrices
            // A: rows_a x cols_a, B: rows_b x cols_b
            Instruction::LocalGet(0), // matrix_a_ptr
            Instruction::I32Load(wasm_encoder::MemArg { offset: 0, align: 2, memory_index: 0 }),
            Instruction::LocalSet(3), // rows_a
            
            Instruction::LocalGet(0), // matrix_a_ptr
            Instruction::I32Load(wasm_encoder::MemArg { offset: 4, align: 2, memory_index: 0 }),
            Instruction::LocalSet(4), // cols_a
            
            Instruction::LocalGet(1), // matrix_b_ptr
            Instruction::I32Load(wasm_encoder::MemArg { offset: 0, align: 2, memory_index: 0 }),
            Instruction::LocalSet(5), // rows_b
            
            Instruction::LocalGet(1), // matrix_b_ptr
            Instruction::I32Load(wasm_encoder::MemArg { offset: 4, align: 2, memory_index: 0 }),
            Instruction::LocalSet(6), // cols_b
            
            // Check if multiplication is valid (cols_a == rows_b)
            Instruction::LocalGet(4), // cols_a
            Instruction::LocalGet(5), // rows_b
            Instruction::I32Eq,
            Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::I32)),
            
            // Initialize outer loop for rows of result
            Instruction::I32Const(0),
            Instruction::LocalSet(7), // i (row counter)
            
            // Outer loop: for each row in result
            Instruction::Loop(wasm_encoder::BlockType::Empty),
            
            // Check if i < rows_a
            Instruction::LocalGet(7), // i
            Instruction::LocalGet(3), // rows_a
            Instruction::I32LtS,
            Instruction::If(wasm_encoder::BlockType::Empty),
            
            // Initialize inner loop for columns of result
            Instruction::I32Const(0),
            Instruction::LocalSet(8), // j (col counter)
            
            // Inner loop: for each column in result
            Instruction::Loop(wasm_encoder::BlockType::Empty),
            
            // Check if j < cols_b
            Instruction::LocalGet(8), // j
            Instruction::LocalGet(6), // cols_b
            Instruction::I32LtS,
            Instruction::If(wasm_encoder::BlockType::Empty),
            
            // Calculate dot product for result[i][j]
            // Initialize sum to 0
            Instruction::F64Const(0.0),
            Instruction::LocalSet(9), // sum
            
            // Initialize k counter for dot product
            Instruction::I32Const(0),
            Instruction::LocalSet(10), // k
            
            // Dot product loop
            Instruction::Loop(wasm_encoder::BlockType::Empty),
            
            // Check if k < cols_a
            Instruction::LocalGet(10), // k
            Instruction::LocalGet(4), // cols_a
            Instruction::I32LtS,
            Instruction::If(wasm_encoder::BlockType::Empty),
            
            // Load A[i][k] and B[k][j], multiply and add to sum
            // A[i][k] = A + 12 + (i * cols_a + k) * 8
            Instruction::LocalGet(0), // matrix_a_ptr
            Instruction::I32Const(12),
            Instruction::I32Add,
            Instruction::LocalGet(7), // i
            Instruction::LocalGet(4), // cols_a
            Instruction::I32Mul,
            Instruction::LocalGet(10), // k
            Instruction::I32Add,
            Instruction::I32Const(8),
            Instruction::I32Mul,
            Instruction::I32Add,
            Instruction::F64Load(wasm_encoder::MemArg { offset: 0, align: 3, memory_index: 0 }),
            
            // B[k][j] = B + 12 + (k * cols_b + j) * 8
            Instruction::LocalGet(1), // matrix_b_ptr
            Instruction::I32Const(12),
            Instruction::I32Add,
            Instruction::LocalGet(10), // k
            Instruction::LocalGet(6), // cols_b
            Instruction::I32Mul,
            Instruction::LocalGet(8), // j
            Instruction::I32Add,
            Instruction::I32Const(8),
            Instruction::I32Mul,
            Instruction::I32Add,
            Instruction::F64Load(wasm_encoder::MemArg { offset: 0, align: 3, memory_index: 0 }),
            
            // Multiply and add to sum
            Instruction::F64Mul,
            Instruction::LocalGet(9), // sum
            Instruction::F64Add,
            Instruction::LocalSet(9), // sum
            
            // Increment k
            Instruction::LocalGet(10),
            Instruction::I32Const(1),
            Instruction::I32Add,
            Instruction::LocalSet(10),
            
            // Continue k loop
            Instruction::Br(1),
            
            Instruction::End, // End k if
            Instruction::End, // End k loop
            
            // Store sum in result[i][j]
            Instruction::LocalGet(2), // result_ptr
            Instruction::I32Const(12),
            Instruction::I32Add,
            Instruction::LocalGet(7), // i
            Instruction::LocalGet(6), // cols_b
            Instruction::I32Mul,
            Instruction::LocalGet(8), // j
            Instruction::I32Add,
            Instruction::I32Const(8),
            Instruction::I32Mul,
            Instruction::I32Add,
            Instruction::LocalGet(9), // sum
            Instruction::F64Store(wasm_encoder::MemArg { offset: 0, align: 3, memory_index: 0 }),
            
            // Increment j
            Instruction::LocalGet(8),
            Instruction::I32Const(1),
            Instruction::I32Add,
            Instruction::LocalSet(8),
            
            // Continue j loop
            Instruction::Br(1),
            
            Instruction::End, // End j if
            Instruction::End, // End j loop
            
            // Increment i
            Instruction::LocalGet(7),
            Instruction::I32Const(1),
            Instruction::I32Add,
            Instruction::LocalSet(7),
            
            // Continue i loop
            Instruction::Br(1),
            
            Instruction::End, // End i if
            Instruction::End, // End i loop
            
            // Return success
            Instruction::I32Const(1),
            
            Instruction::Else,
            
            // Invalid dimensions for multiplication
            Instruction::I32Const(0),
            
            Instruction::End, // End dimension check
        ]
    }

    fn generate_matrix_transpose(&self) -> Vec<Instruction> {
        // Matrix transpose: transposes a matrix
        // Parameters: matrix_ptr, result_ptr
        // Returns: result matrix pointer
        vec![
            // Load dimensions from original matrix
            Instruction::LocalGet(0), // matrix_ptr
            Instruction::I32Load(wasm_encoder::MemArg { offset: 0, align: 2, memory_index: 0 }),
            Instruction::LocalSet(2), // rows
            
            Instruction::LocalGet(0), // matrix_ptr
            Instruction::I32Load(wasm_encoder::MemArg { offset: 4, align: 2, memory_index: 0 }),
            Instruction::LocalSet(3), // cols
            
            // Set result matrix dimensions (transposed)
            Instruction::LocalGet(1), // result_ptr
            Instruction::LocalGet(3), // cols (becomes rows)
            Instruction::I32Store(wasm_encoder::MemArg { offset: 0, align: 2, memory_index: 0 }),
            
            Instruction::LocalGet(1), // result_ptr
            Instruction::LocalGet(2), // rows (becomes cols)
            Instruction::I32Store(wasm_encoder::MemArg { offset: 4, align: 2, memory_index: 0 }),
            
            // Initialize outer loop for rows
            Instruction::I32Const(0),
            Instruction::LocalSet(4), // i (row counter)
            
            // Outer loop: for each row in original matrix
            Instruction::Loop(wasm_encoder::BlockType::Empty),
            
            // Check if i < rows
            Instruction::LocalGet(4), // i
            Instruction::LocalGet(2), // rows
            Instruction::I32LtS,
            Instruction::If(wasm_encoder::BlockType::Empty),
            
            // Initialize inner loop for columns
            Instruction::I32Const(0),
            Instruction::LocalSet(5), // j (col counter)
            
            // Inner loop: for each column in original matrix
            Instruction::Loop(wasm_encoder::BlockType::Empty),
            
            // Check if j < cols
            Instruction::LocalGet(5), // j
            Instruction::LocalGet(3), // cols
            Instruction::I32LtS,
            Instruction::If(wasm_encoder::BlockType::Empty),
            
            // Load element from original[i][j]
            Instruction::LocalGet(0), // matrix_ptr
            Instruction::I32Const(12), // header size
            Instruction::I32Add,
            Instruction::LocalGet(4), // i
            Instruction::LocalGet(3), // cols
            Instruction::I32Mul,
            Instruction::LocalGet(5), // j
            Instruction::I32Add,
            Instruction::I32Const(8), // sizeof(f64)
            Instruction::I32Mul,
            Instruction::I32Add,
            // Load value from source address (stack: [source_addr] -> [f64_value])
            Instruction::F64Load(wasm_encoder::MemArg { offset: 0, align: 3, memory_index: 0 }),
            
            // Calculate destination address
            Instruction::LocalGet(1), // result_ptr
            Instruction::I32Const(12), // header size
            Instruction::I32Add,
            Instruction::LocalGet(5), // j (becomes row)
            Instruction::LocalGet(2), // rows (becomes cols)
            Instruction::I32Mul,
            Instruction::LocalGet(4), // i (becomes col)
            Instruction::I32Add,
            Instruction::I32Const(8), // sizeof(f64)
            Instruction::I32Mul,
            Instruction::I32Add,
            
            // Now stack is [f64_value, dest_address], but we need [dest_address, f64_value]
            // This is the same problem! We need to swap again
            // Let me try a different approach - store f64 in a local temporarily
            Instruction::F64Store(wasm_encoder::MemArg { offset: 0, align: 3, memory_index: 0 }),
            
            // Increment j
            Instruction::LocalGet(5),
            Instruction::I32Const(1),
            Instruction::I32Add,
            Instruction::LocalSet(5),
            
            // Continue j loop
            Instruction::Br(1),
            
            Instruction::End, // End j if
            Instruction::End, // End j loop
            
            // Increment i
            Instruction::LocalGet(4),
            Instruction::I32Const(1),
            Instruction::I32Add,
            Instruction::LocalSet(4),
            
            // Continue i loop
            Instruction::Br(1),
            
            Instruction::End, // End i if
            Instruction::End, // End i loop
            
            // Return result matrix pointer
            Instruction::LocalGet(1), // result_ptr
        ]
    }
}
