use clean_language_compiler::{
    parser::CleanParser,
    semantic::SemanticAnalyzer,
    codegen::CodeGenerator,
    error::CompilerError,
};

#[test]
fn test_simple_program() -> Result<(), CompilerError> {
    let source = r#"
        function start()
            integer x = 42
            integer y = 10
            
            integer result = x + y
            
            print(result)
    "#;

    let program = CleanParser::parse_program(source)?;
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(&program)?;
    
    let mut codegen = CodeGenerator::new();
    codegen.generate(&program)?;
    let wasm_binary = codegen.finish();
    
    assert!(!wasm_binary.is_empty());
    Ok(())
}

#[test]
fn test_error_handling() -> Result<(), CompilerError> {
    let source = r#"
        function start()
            integer x = 10
            integer y = 0
            
            integer result = x / y onError 42
            print(result)
    "#;

    let program = CleanParser::parse_program(source)?;
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(&program)?;
    
    let mut codegen = CodeGenerator::new();
    codegen.generate(&program)?;
    let wasm_binary = codegen.finish();
    
    assert!(!wasm_binary.is_empty());
    Ok(())
}

#[test]
fn test_string_operations() -> Result<(), CompilerError> {
    let source = r#"
        function start()
            string name = "World"
            string message = "Hello, " + name + "!"
            print(message)
    "#;

    let program = CleanParser::parse_program(source)?;
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(&program)?;
    
    let mut codegen = CodeGenerator::new();
    codegen.generate(&program)?;
    let wasm_binary = codegen.finish();
    
    assert!(!wasm_binary.is_empty());
    Ok(())
} 