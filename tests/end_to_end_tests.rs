//! End-to-end tests for the Clean Language compiler

use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_simple_return() {
    // Create a temporary directory for the output file
    let temp_dir = tempdir().unwrap();
    let output_file = temp_dir.path().join("test_simple.wasm");
    
    // Run the compiler to compile the test file
    let status = Command::new("cargo")
        .args(&["run", "--bin", "cleanc", "compile", 
               "tests/test_inputs/test_simple.cln", 
               output_file.to_str().unwrap()])
        .status()
        .expect("Failed to execute compiler");
    
    assert!(status.success(), "Compilation failed");
    
    // Run the compiled wasm file
    let output = Command::new("cargo")
        .args(&["run", "--bin", "cleanc", "run", 
                output_file.to_str().unwrap()])
        .output()
        .expect("Failed to execute wasm runner");
    
    assert!(output.status.success(), "WebAssembly execution failed");
    
    // Check for the expected return value
    let output_str = String::from_utf8_lossy(&output.stdout);
    println!("Program output: {}", output_str);
    assert!(output_str.contains("Return value: I32(42)"), 
           "Expected return value of 42 not found in output: {}", output_str);
}
