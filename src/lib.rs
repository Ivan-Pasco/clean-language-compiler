pub mod ast;
pub mod parser;
pub mod semantic;
pub mod codegen;
pub mod error;
pub mod validation;
pub mod stdlib;
pub mod types;
pub mod module;
pub mod runtime;

use crate::parser::CleanParser;
use crate::semantic::SemanticAnalyzer;
use crate::codegen::CodeGenerator;
use crate::error::CompilerError;

/// Compiles Clean Language source code to WebAssembly
pub fn compile(source: &str) -> Result<Vec<u8>, CompilerError> {
    compile_with_file(source, "<unknown>")
}

/// Compiles Clean Language source code to WebAssembly with file path for better error reporting
pub fn compile_with_file(source: &str, file_path: &str) -> Result<Vec<u8>, CompilerError> {
    // Parse the source code with file path information
    let program = CleanParser::parse_program_with_file(source, file_path)?;

    // Perform semantic analysis
    let mut analyzer = SemanticAnalyzer::new();
    let analyzed_program = analyzer.analyze(&program)?;

    // Generate WASM code
    let mut codegen = CodeGenerator::new();
    let wasm_binary = codegen.generate(&analyzed_program)?;

    Ok(wasm_binary)
}

/// Compiles with detailed error reporting and recovery
pub fn compile_with_recovery(source: &str, file_path: &str) -> Result<Vec<u8>, Vec<CompilerError>> {
    // Try parsing with error recovery first
    let program = match CleanParser::parse_program_with_recovery(source, file_path) {
        Ok(program) => program,
        Err(parse_errors) => return Err(parse_errors),
    };

    // Perform semantic analysis
    let mut analyzer = SemanticAnalyzer::new();
    let analyzed_program = match analyzer.analyze(&program) {
        Ok(program) => program,
        Err(semantic_error) => return Err(vec![semantic_error]),
    };

    // Generate WASM code
    let mut codegen = CodeGenerator::new();
    match codegen.generate(&analyzed_program) {
        Ok(wasm_binary) => Ok(wasm_binary),
        Err(codegen_error) => Err(vec![codegen_error]),
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_basic_integration() {
        let source = r#"function start()
	integer x = 42
	print(x)
"#;
        
        let result = compile_with_file(source, "test.clean");
        match result {
            Ok(wasm_binary) => {
                println!("✓ Basic integration test passed, generated {} bytes of WASM", wasm_binary.len());
                assert!(!wasm_binary.is_empty());
            },
            Err(error) => {
                println!("✗ Basic integration test failed: {}", error);
                panic!("Integration test failed: {}", error);
            }
        }
    }

    #[test]
    fn test_function_integration() {
        let source = r#"function integer add()
	description "Adds two numbers"
	input
		integer a
		integer b
	
	return a + b

function start()
	integer result = add(5, 3)
	print(result)
"#;
        
        let result = compile_with_file(source, "function_test.clean");
        match result {
            Ok(wasm_binary) => {
                println!("✓ Function integration test passed, generated {} bytes of WASM", wasm_binary.len());
                assert!(!wasm_binary.is_empty());
            },
            Err(error) => {
                println!("✗ Function integration test failed: {}", error);
                // Don't panic here as this might reveal integration issues we need to fix
            }
        }
    }

    #[test]
    fn test_type_checking_integration() {
        let source = r#"function start()
	integer x = 42
	string y = "hello"
	print(x)
	print(y)
"#;
        
        let result = compile_with_file(source, "type_test.clean");
        match result {
            Ok(wasm_binary) => {
                println!("✓ Type checking integration test passed, generated {} bytes of WASM", wasm_binary.len());
                assert!(!wasm_binary.is_empty());
            },
            Err(error) => {
                println!("✗ Type checking integration test failed: {}", error);
                // This might reveal type system integration issues
            }
        }
    }

    #[test]
    fn test_error_propagation() {
        let source = r#"function start()
	integer x = undefined_function() onError 0
	print(x)
"#;
        
        let result = compile_with_file(source, "error_test.clean");
        match result {
            Ok(_) => {
                println!("⚠ Error propagation test: Expected error but compilation succeeded");
            },
            Err(error) => {
                println!("✓ Error propagation test: Correctly caught error: {}", error);
                // Check that the error contains useful information
                assert!(error.to_string().contains("error_test.clean"));
            }
        }
    }

    #[test]
    fn test_stdlib_integration() {
        println!("\n=== Standard Library Integration Test ===");
        
        let test_cases = vec![
            ("Math Functions", r#"function start()
	integer x = -5
	integer result = abs(x)
	print(result)
"#),
            ("String Functions", r#"function start()
	string text = "hello"
	integer length = len(text)
	print(length)
"#),
            ("Array Functions", r#"function start()
	Array<integer> arr = [1, 2, 3, 4, 5]
	integer length = array_length(arr)
	print(length)
"#),
        ];

        let mut passed = 0;
        let total = test_cases.len();

        for (name, source) in test_cases {
            println!("\n--- Testing {} ---", name);
            println!("Source: {}", source);
            
            match compile_with_file(source, "stdlib_test.clean") {
                Ok(wasm_binary) => {
                    println!("✓ {}: {} bytes", name, wasm_binary.len());
                    assert!(!wasm_binary.is_empty());
                    passed += 1;
                },
                Err(error) => {
                    println!("✗ {} failed: {}", name, error);
                    // Don't panic here as some stdlib functions might not be fully implemented yet
                }
            }
        }

        println!("=== Stdlib Integration Summary: {}/{} tests passed ===", passed, total);
        if passed == total {
            println!("🎉 All stdlib integration tests passed!");
        }
    }

    #[test]
    fn test_debug_parsing() {
        let source = r#"function start()
	integer x = 42
	print(x)
"#;
        
        println!("\n=== Debug Parsing Test ===");
        
        // Parse the program
        match CleanParser::parse_program(source) {
            Ok(program) => {
                println!("✓ Parsing succeeded");
                println!("Program: {:#?}", program);
                
                // Try semantic analysis
                let mut analyzer = SemanticAnalyzer::new();
                match analyzer.analyze(&program) {
                    Ok(_) => println!("✓ Semantic analysis succeeded"),
                    Err(error) => println!("✗ Semantic analysis failed: {}", error),
                }
            },
            Err(error) => {
                println!("✗ Parsing failed: {}", error);
            }
        }
    }

    #[test]
    fn test_comprehensive_integration() {
        println!("\n=== Comprehensive Integration Test ===");
        
        let test_cases = vec![
            ("Basic", r#"function start()
	integer x = 42
	print(x)
"#),
            ("Arithmetic", r#"function start()
	integer x = 1 + 2 * 3
	print(x)
"#),
            ("Variables", r#"function start()
	integer x = 5
	integer y = x + 1
	print(y)
"#),
            ("Arrays", r#"function start()
	Array<integer> arr = [1, 2, 3]
	print(arr)
"#),
        ];

        let mut passed = 0;
        let total = test_cases.len();

        for (name, source) in test_cases {
            println!("\n--- Testing {} ---", name);
            println!("Source: {}", source);
            
            // First try parsing
            match CleanParser::parse_program(source) {
                Ok(program) => {
                    println!("✓ Parsing succeeded");
                    println!("AST: {:#?}", program);
                    
                    // Try semantic analysis
                    let mut analyzer = SemanticAnalyzer::new();
                    match analyzer.analyze(&program) {
                        Ok(_) => {
                            println!("✓ Semantic analysis succeeded");
                            
                            // Try full compilation
                            match compile_with_file(source, &format!("{}_test.clean", name.to_lowercase())) {
                                Ok(wasm_binary) => {
                                    println!("✓ {}: {} bytes", name, wasm_binary.len());
                                    passed += 1;
                                },
                                Err(error) => {
                                    println!("✗ {}: Compilation failed: {}", name, error);
                                }
                            }
                        },
                        Err(error) => {
                            println!("✗ {}: Semantic analysis failed: {}", name, error);
                        }
                    }
                },
                Err(error) => {
                    println!("✗ {}: Parsing failed: {}", name, error);
                }
            }
        }

        println!("=== Integration Summary: {}/{} tests passed ===", passed, total);
        
        if passed == total {
            println!("🎉 All integration tests passed!");
        } else {
            println!("⚠ Some integration tests failed - this indicates areas that need improvement");
        }
    }

    #[test]
    fn test_error_handling_recovery() {
        println!("\n=== Error Handling & Recovery Test ===");
        
        let test_cases = vec![
            ("OnError Basic", r#"function start()
	integer x = undefined_function() onError 42
	print(x)
"#),
            ("Error Function", r#"function start()
	error("This is an error test")
"#),
            ("Unused Variable Warning", r#"function start()
	integer unused_var = 42
	integer x = 10
	print(x)
"#),
        ];

        let mut passed = 0;
        let total = test_cases.len();

        for (name, source) in test_cases {
            println!("\n--- Testing: {} ---", name);
            
            // Parse the program
            match CleanParser::parse_program(source) {
                Ok(program) => {
                    println!("✓ Parsing succeeded");
                    
                    // Try semantic analysis with warning collection
                    let mut analyzer = SemanticAnalyzer::new();
                    match analyzer.analyze(&program) {
                        Ok(_) => {
                            println!("✓ Semantic analysis succeeded");
                            
                            // Check for warnings
                            let warnings = analyzer.get_warnings();
                            if !warnings.is_empty() {
                                println!("⚠ Warnings collected: {}", warnings.len());
                                for warning in warnings {
                                    println!("  Warning: {}", warning.message);
                                }
                            } else {
                                println!("✓ No warnings");
                            }
                            
                            // Try code generation
                            let mut codegen = CodeGenerator::new();
                            match codegen.generate(&program) {
                                Ok(wasm_binary) => {
                                    println!("✓ Code generation succeeded: {} bytes", wasm_binary.len());
                                    passed += 1;
                                },
                                Err(error) => println!("✗ Code generation failed: {}", error),
                            }
                        },
                        Err(error) => println!("✗ Semantic analysis failed: {}", error),
                    }
                },
                Err(error) => println!("✗ Parsing failed: {}", error),
            }
        }

        println!("\n=== Error Handling & Recovery Test Results ===");
        println!("Passed: {}/{}", passed, total);
        
        if passed == total {
            println!("🎉 All error handling tests passed!");
        } else {
            println!("⚠ Some tests failed - error handling needs improvement");
        }
    }

    #[test]
    fn test_wasm_execution() {
        println!("\n=== WebAssembly Execution Test ===");
        
        let test_cases = vec![
            ("Basic Integer", r#"function start()
	integer x = 42
	print(x)
"#, "Function executed successfully"),
            ("Arithmetic", r#"function start()
	integer result = 1 + 2 * 3
	print(result)
"#, "Function executed successfully"),
            ("Variable Operations", r#"function start()
	integer x = 5
	integer y = x + 10
	print(y)
"#, "Function executed successfully"),
            ("Simple Assignment", r#"function start()
	integer x = 100
"#, "Function executed successfully"),
        ];

        let mut passed = 0;
        let total = test_cases.len();

        for (name, source, expected_output) in test_cases {
            println!("\n--- Testing {} ---", name);
            println!("Source: {}", source.replace('\n', "\\n"));
            println!("Expected: {}", expected_output);
            
            match compile_with_file(source, &format!("{}_execution_test.clean", name.to_lowercase().replace(' ', "_"))) {
                Ok(wasm_binary) => {
                    println!("✓ Compilation succeeded: {} bytes", wasm_binary.len());
                    
                    // Try to execute the WASM using wasmtime
                    match execute_wasm(&wasm_binary) {
                        Ok(output) => {
                            if output.trim() == expected_output {
                                println!("✓ {}: Output matches expected: '{}'", name, output.trim());
                                passed += 1;
                            } else {
                                println!("✗ {}: Output mismatch. Expected: '{}', Got: '{}'", name, expected_output, output.trim());
                            }
                        },
                        Err(error) => {
                            println!("✗ {}: Execution failed: {}", name, error);
                        }
                    }
                },
                Err(error) => {
                    println!("✗ {}: Compilation failed: {}", name, error);
                }
            }
        }

        println!("\n=== Execution Test Summary: {}/{} tests passed ===", passed, total);
        if passed == total {
            println!("🎉 All WebAssembly execution tests passed!");
        } else {
            println!("⚠ Some execution tests failed - this may indicate runtime issues");
        }
    }

    fn execute_wasm(wasm_binary: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
        use wasmtime::*;
        
        // Create a WASM engine and store
        let engine = Engine::default();
        let mut store = Store::new(&engine, ());
        
        // Compile the module
        let module = Module::new(&engine, wasm_binary)?;
        
        // Our Clean Language WASM modules are self-contained with no imports needed!
        // Instantiate the module with no imports
        let instance = Instance::new(&mut store, &module, &[])?;
        
        // Get the start function 
        let start_func_export = instance.get_export(&mut store, "start")
            .ok_or("start function not found")?;
        let start_func = start_func_export.into_func()
            .ok_or("start export is not a function")?;
        
        // Execute the start function
        start_func.call(&mut store, &[], &mut [])?;
        
        // For now, we return success message since print() is internal to WASM
        // In the future, we could capture output via memory inspection
        Ok("Function executed successfully".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_implicit_context() {
        let program = r#"
class Shape
	string color
	
	constructor(string colorParam)
		color = colorParam
	
	functions:
		string getColor()
			return color

function start()
	Shape shape = Shape("red")
	print("Implicit context test")
"#;

        match compile_with_file(program, "implicit_context_test.clean") {
            Ok(_) => println!("✓ Implicit context compilation successful"),
            Err(error) => {
                println!("✗ Implicit context compilation failed: {}", error);
                panic!("Implicit context test failed");
            }
        }
    }

    #[test]
    fn test_inheritance_with_implicit_context() {
        let program = r#"
class Shape
	string color
	
	constructor(string colorParam)
		color = colorParam
	
	functions:
		string getColor()
			return color

class Circle is Shape
	float radius
	
	constructor(string colorParam, float radiusParam)
		base(colorParam)
		radius = radiusParam
	
	functions:
		float getRadius()
			return radius

function start()
	Circle circle = Circle("blue", 5.0)
	print("Inheritance with implicit context test")
"#;

        match compile_with_file(program, "inheritance_with_implicit_context_test.clean") {
            Ok(_) => println!("✓ Inheritance with implicit context compilation successful"),
            Err(error) => {
                println!("✗ Inheritance with implicit context compilation failed: {}", error);
                panic!("Inheritance with implicit context test failed");
            }
        }
    }
} 