use clean_language_compiler::{
    parser::CleanParser,
    codegen::CodeGenerator,
};

#[test]
fn test_basic_error_handling() {
    let source = r#"
start()
	number result = 10
	number divisor = 0
	if divisor == 0
		result = 42
	print(result)
    "#;
    
    // Parse and compile - focus on compilation success rather than execution
    let program = CleanParser::parse_program(source).expect("Failed to parse");
    
    // Run semantic analysis
    let mut semantic_analyzer = clean_language_compiler::semantic::SemanticAnalyzer::new();
    let analyzed_program = semantic_analyzer.analyze(&program).expect("Failed semantic analysis");
    
    // Generate WASM
    let mut code_gen = CodeGenerator::new();
    let _wasm_binary = code_gen.generate(&analyzed_program).expect("Failed to generate WASM");
    
    println!("✓ Basic error handling pattern compiled successfully");
}

#[test]
fn test_error_variable_access() {
    let source = r#"
functions:
	number getErrorCode()
		return 123.0

start()
	number errorCode = getErrorCode()
	print(errorCode)
    "#;
    
    // Parse and compile - focus on compilation success rather than execution
    let program = CleanParser::parse_program(source).expect("Failed to parse");
    
    // Run semantic analysis
    let mut semantic_analyzer = clean_language_compiler::semantic::SemanticAnalyzer::new();
    let analyzed_program = semantic_analyzer.analyze(&program).expect("Failed semantic analysis");
    
    // Generate WASM
    let mut code_gen = CodeGenerator::new();
    let _wasm_binary = code_gen.generate(&analyzed_program).expect("Failed to generate WASM");
    
    println!("✓ Error variable access pattern compiled successfully");
}

#[test]
fn test_nested_error_handling() {
    let source = r#"
functions:
	number inner()
		return 5.0
	
	number outer()
		return inner() + 1.0

start()
	number result = 0.0
	number value = outer()
	result = value * 10.0
	print(result)
    "#;
    
    // Parse and compile - focus on compilation success rather than execution
    let program = CleanParser::parse_program(source).expect("Failed to parse");
    
    // Run semantic analysis
    let mut semantic_analyzer = clean_language_compiler::semantic::SemanticAnalyzer::new();
    let analyzed_program = semantic_analyzer.analyze(&program).expect("Failed semantic analysis");
    
    // Generate WASM
    let mut code_gen = CodeGenerator::new();
    let _wasm_binary = code_gen.generate(&analyzed_program).expect("Failed to generate WASM");
    
    println!("✓ Nested function pattern compiled successfully");
} 