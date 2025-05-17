pub mod ast;
pub mod parser;
pub mod semantic;
pub mod codegen;
pub mod error;
pub mod validation;
pub mod stdlib;
pub mod types;

use crate::parser::CleanParser;
use crate::semantic::SemanticAnalyzer;
use crate::codegen::CodeGenerator;
use crate::error::CompilerError;

/// Compiles Clean Language source code to WebAssembly
pub fn compile(source: &str) -> Result<Vec<u8>, CompilerError> {
    // Parse the source code
    let program = CleanParser::parse_program(source)?;

    // Perform semantic analysis
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.check(&program)?;

    // Generate WASM code
    let mut codegen = CodeGenerator::new();
    codegen.generate(&program)?;

    Ok(codegen.finish())
} 