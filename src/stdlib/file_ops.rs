use crate::error::CompilerError;
use crate::codegen::CodeGenerator;
use crate::types::WasmType;
use wasm_encoder::Instruction;
use crate::stdlib::register_stdlib_function;

/// File operations implementation for Clean Language
/// Provides file I/O functionality including read, write, append, exists, delete, and lines operations
pub struct FileOperations {
    heap_start: usize,
}

impl FileOperations {
    /// Create a new FileOperations instance
    pub fn new(heap_start: usize) -> Self {
        Self { heap_start }
    }

    /// Register all file operation functions with the code generator
    pub fn register_functions(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // File.read(path: string) -> string
        // Reads the entire file content as a single string
        register_stdlib_function(
            codegen,
            "file_read",
            &[WasmType::I32], // path pointer
            Some(WasmType::I32), // return string pointer
            self.generate_file_read_function()
        )?;

        // File.write(path: string, content: string) -> void
        // Writes text to a file, replacing any existing content
        register_stdlib_function(
            codegen,
            "file_write",
            &[WasmType::I32, WasmType::I32], // path pointer, content pointer
            None, // void return
            self.generate_file_write_function()
        )?;

        // File.append(path: string, content: string) -> void
        // Adds text to the end of an existing file
        register_stdlib_function(
            codegen,
            "file_append",
            &[WasmType::I32, WasmType::I32], // path pointer, content pointer
            None, // void return
            self.generate_file_append_function()
        )?;

        // File.exists(path: string) -> boolean
        // Checks if a file exists at the given path
        register_stdlib_function(
            codegen,
            "file_exists",
            &[WasmType::I32], // path pointer
            Some(WasmType::I32), // return boolean (i32)
            self.generate_file_exists_function()
        )?;

        // File.delete(path: string) -> void
        // Removes a file from the filesystem
        register_stdlib_function(
            codegen,
            "file_delete",
            &[WasmType::I32], // path pointer
            None, // void return
            self.generate_file_delete_function()
        )?;

        // File.lines(path: string) -> List<string>
        // Reads the file and returns each line as a separate string
        register_stdlib_function(
            codegen,
            "file_lines",
            &[WasmType::I32], // path pointer
            Some(WasmType::I32), // return List pointer
            self.generate_file_lines_function()
        )?;

        Ok(())
    }

    /// Generate WebAssembly instructions for File.read()
    fn generate_file_read_function(&self) -> Vec<Instruction> {
        vec![
            // Get the path parameter (local 0)
            Instruction::LocalGet(0),
            
            // Call WASI fd_read or similar file reading function
            // For now, we'll use a placeholder that calls an imported function
            // In a real implementation, this would:
            // 1. Convert the path string to a C-style string
            // 2. Open the file using WASI file operations
            // 3. Read the entire content
            // 4. Allocate memory for the result string
            // 5. Return the pointer to the result string
            
            // Placeholder: Call imported file_read_impl function
            Instruction::Call(0), // Assuming import index 0 is file_read_impl
            
            // The imported function should return a string pointer
            // Return the result
        ]
    }

    /// Generate WebAssembly instructions for File.write()
    fn generate_file_write_function(&self) -> Vec<Instruction> {
        vec![
            // Get the path parameter (local 0)
            Instruction::LocalGet(0),
            // Get the content parameter (local 1)
            Instruction::LocalGet(1),
            
            // Call WASI file writing function
            // In a real implementation, this would:
            // 1. Convert path and content strings to C-style strings
            // 2. Open/create the file using WASI file operations
            // 3. Write the content to the file
            // 4. Close the file
            
            // Placeholder: Call imported file_write_impl function
            Instruction::Call(1), // Assuming import index 1 is file_write_impl
            
            // No return value for void function
        ]
    }

    /// Generate WebAssembly instructions for File.append()
    fn generate_file_append_function(&self) -> Vec<Instruction> {
        vec![
            // Get the path parameter (local 0)
            Instruction::LocalGet(0),
            // Get the content parameter (local 1)
            Instruction::LocalGet(1),
            
            // Call WASI file appending function
            // Similar to write, but opens file in append mode
            
            // Placeholder: Call imported file_append_impl function
            Instruction::Call(2), // Assuming import index 2 is file_append_impl
        ]
    }

    /// Generate WebAssembly instructions for File.exists()
    fn generate_file_exists_function(&self) -> Vec<Instruction> {
        vec![
            // Get the path parameter (local 0)
            Instruction::LocalGet(0),
            
            // Call WASI file stat function to check existence
            // In a real implementation, this would:
            // 1. Convert path string to C-style string
            // 2. Use WASI path_filestat_get or similar
            // 3. Return 1 if file exists, 0 if not
            
            // Placeholder: Call imported file_exists_impl function
            Instruction::Call(3), // Assuming import index 3 is file_exists_impl
            
            // Return boolean result (0 or 1)
        ]
    }

    /// Generate WebAssembly instructions for File.delete()
    fn generate_file_delete_function(&self) -> Vec<Instruction> {
        vec![
            // Get the path parameter (local 0)
            Instruction::LocalGet(0),
            
            // Call WASI file deletion function
            // In a real implementation, this would:
            // 1. Convert path string to C-style string
            // 2. Use WASI path_unlink_file
            // 3. Handle any errors gracefully
            
            // Placeholder: Call imported file_delete_impl function
            Instruction::Call(4), // Assuming import index 4 is file_delete_impl
        ]
    }

    /// Generate WebAssembly instructions for File.lines()
    fn generate_file_lines_function(&self) -> Vec<Instruction> {
        vec![
            // Get the path parameter (local 0)
            Instruction::LocalGet(0),
            
            // Call function to read file and split into lines
            // In a real implementation, this would:
            // 1. Read the entire file content
            // 2. Split the content by newline characters
            // 3. Create a List<string> with each line
            // 4. Return pointer to the List
            
            // Placeholder: Call imported file_lines_impl function
            Instruction::Call(5), // Assuming import index 5 is file_lines_impl
            
            // Return List pointer
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::CodeGenerator;

    #[test]
    fn test_file_operations_creation() {
        let file_ops = FileOperations::new(65536);
        assert_eq!(file_ops.heap_start, 65536);
    }

    #[test]
    fn test_register_file_functions() -> Result<(), CompilerError> {
        let mut codegen = CodeGenerator::new();
        let file_ops = FileOperations::new(65536);
        
        // This should not panic and should register all functions
        file_ops.register_functions(&mut codegen)?;
        
        // Verify that functions were registered
        assert!(codegen.get_function_index("file_read").is_some());
        assert!(codegen.get_function_index("file_write").is_some());
        assert!(codegen.get_function_index("file_append").is_some());
        assert!(codegen.get_function_index("file_exists").is_some());
        assert!(codegen.get_function_index("file_delete").is_some());
        assert!(codegen.get_function_index("file_lines").is_some());
        
        Ok(())
    }

    #[test]
    fn test_file_read_function_generation() {
        let file_ops = FileOperations::new(65536);
        let instructions = file_ops.generate_file_read_function();
        
        // Should have at least LocalGet and Call instructions
        assert!(!instructions.is_empty());
        assert!(matches!(instructions[0], Instruction::LocalGet(0)));
        assert!(matches!(instructions[1], Instruction::Call(_)));
    }

    #[test]
    fn test_file_write_function_generation() {
        let file_ops = FileOperations::new(65536);
        let instructions = file_ops.generate_file_write_function();
        
        // Should have LocalGet for both parameters and Call
        assert!(!instructions.is_empty());
        assert!(matches!(instructions[0], Instruction::LocalGet(0)));
        assert!(matches!(instructions[1], Instruction::LocalGet(1)));
        assert!(matches!(instructions[2], Instruction::Call(_)));
    }

    #[test]
    fn test_file_exists_function_generation() {
        let file_ops = FileOperations::new(65536);
        let instructions = file_ops.generate_file_exists_function();
        
        // Should have LocalGet and Call instructions
        assert!(!instructions.is_empty());
        assert!(matches!(instructions[0], Instruction::LocalGet(0)));
        assert!(matches!(instructions[1], Instruction::Call(_)));
    }
} 