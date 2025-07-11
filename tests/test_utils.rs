use clean_language_compiler::{
    ast::{Program},
    parser::CleanParser,
    semantic::SemanticAnalyzer,
    codegen::CodeGenerator,
};
use std::fs;
use std::path::Path;

/// Reads a test file from the test_inputs directory
pub fn read_test_file(filename: &str) -> String {
    let path = format!("tests/test_inputs/{}", filename);
    fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("Could not read test file: {}", path))
}

/// Parses a source string and returns the AST
pub fn parse_source(source: &str) -> Program {
    CleanParser::parse_program(source)
        .unwrap_or_else(|e| panic!("Failed to parse test input: {}", e))
}

/// Runs semantic analysis on a program
pub fn analyze_program(program: &Program) -> Result<(), String> {
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(program).map_err(|e| e.to_string()).map(|_| ())
}

/// Generates WebAssembly from a program
pub fn generate_wasm(program: &Program) -> Vec<u8> {
    // Run semantic analysis first
    let mut analyzer = SemanticAnalyzer::new();
    let analyzed_program = analyzer.analyze(program)
        .unwrap_or_else(|e| panic!("Failed semantic analysis: {}", e));
    
    let mut generator = CodeGenerator::new();
    generator.generate(&analyzed_program)
        .unwrap_or_else(|e| panic!("Failed to generate WASM: {}", e))
}

/// Validates generated WebAssembly
pub fn validate_wasm(wasm_binary: &[u8]) -> bool {
    // Simple WASM validation - check magic bytes and minimum size
    if wasm_binary.len() < 8 || &wasm_binary[0..4] != b"\0asm" {
        eprintln!("WASM Validation Error: Invalid magic bytes or too short");
        return false;
    }
    
    // Use wasmparser for detailed validation
    match wasmparser::validate(wasm_binary) {
        Ok(_) => {
            println!("WASM validation passed successfully!");
            true
        }
        Err(e) => {
            eprintln!("WASM Validation Error: {}", e);
            // Print the location of the error for debugging
            eprintln!("Error details: {:?}", e);
            false
        }
    }
}

/// Helper to create the test_inputs directory if it doesn't exist
pub fn ensure_test_inputs_dir() {
    let path = Path::new("tests/test_inputs");
    if !path.exists() {
        fs::create_dir_all(path).expect("Could not create test_inputs directory");
    }
}

/// Creates a test input file with the given content
pub fn create_test_file(filename: &str, content: &str) {
    ensure_test_inputs_dir();
    let path = format!("tests/test_inputs/{}", filename);
    fs::write(&path, content).expect("Could not write test file");
}

/// End-to-end test helper: parse, analyze, and generate code
pub fn compile_source(source: &str) -> Result<Vec<u8>, String> {
    let program = parse_source(source);
    let mut analyzer = SemanticAnalyzer::new();
    let analyzed_program = analyzer.analyze(&program).map_err(|e| e.to_string())?;
    Ok(generate_wasm(&analyzed_program))
}

pub fn compile_program(program: &Program) -> Result<Vec<u8>, String> {
    let mut analyzer = SemanticAnalyzer::new();
    let mut codegen = CodeGenerator::new();
    
    // Run semantic analysis
    let analyzed_program = analyzer.analyze(program).map_err(|e| e.to_string())?;
    
    // Generate WASM using the analyzed program
    codegen.generate(&analyzed_program)
        .map_err(|e| e.to_string())
} 