use crate::ast::{Program, Function, Statement, Expression, Type, Value, Parameter, SourceLocation};
use crate::parser::CleanParser;
use crate::semantic::SemanticAnalyzer;
use crate::codegen::CodeGenerator;
use crate::error::CompilerError;
use crate::stdlib::memory::MemoryManager;
use wasmtime::{Engine, Module, Store, Instance, Func, GlobalType, Mutability, ValType};

#[test]
fn test_string_interpolation() -> Result<(), CompilerError> {
    let source = r#"
        start() {
            let name = "Alice"
            let age = 25
            printl "Hello ${name}, you are ${age} years old!"
        }
    "#;

    let program = CleanParser::parse_program(source)?;
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.check(&program)?;
    let mut codegen = CodeGenerator::new();
    codegen.generate(&program)?;
    Ok(())
}

#[test]
fn test_control_flow() -> Result<(), CompilerError> {
    let source = r#"
        start() {
            let sum = 0
            from 1 to 10 step 1 {
                if sum is 15 {
                    printl "Found 15!"
                }
                sum = sum + 1
            }

            let numbers = [1, 2, 3, 4, 5]
            iterate num in numbers {
                printl num
            }
        }
    "#;

    let program = CleanParser::parse_program(source)?;
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.check(&program)?;
    let mut codegen = CodeGenerator::new();
    codegen.generate(&program)?;
    Ok(())
}

#[test]
fn test_function_calls() -> Result<(), CompilerError> {
    let source = r#"
        function add(a: integer, b: integer) returns integer {
            return a + b
        }

        function multiply(a: integer, b: integer) returns integer {
            return a * b
        }

        start() {
            let x = add(5, 3)
            let y = multiply(x, 2)
            printl y
        }
    "#;

    let program = CleanParser::parse_program(source)?;
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.check(&program)?;
    let mut codegen = CodeGenerator::new();
    codegen.generate(&program)?;
    Ok(())
}

#[test]
fn test_error_handling() -> Result<(), CompilerError> {
    // Test type mismatch
    let source = r#"
        start() {
            let x: integer = "not an integer"
        }
    "#;

    let program = CleanParser::parse_program(source)?;
    let mut analyzer = SemanticAnalyzer::new();
    assert!(analyzer.check(&program).is_err());

    // Test undefined variable
    let source = r#"
        start() {
            printl undefined_variable
        }
    "#;

    let program = CleanParser::parse_program(source)?;
    let mut analyzer = SemanticAnalyzer::new();
    assert!(analyzer.check(&program).is_err());

    // Test wrong number of function arguments
    let source = r#"
        function add(a: integer, b: integer) returns integer {
            return a + b
        }

        start() {
            let x = add(1)
        }
    "#;

    let program = CleanParser::parse_program(source)?;
    let mut analyzer = SemanticAnalyzer::new();
    assert!(analyzer.check(&program).is_err());

    Ok(())
}

#[test]
fn test_matrix_operations() -> Result<(), CompilerError> {
    let source = r#"
        start() {
            let matrix1 = [[1, 2], [3, 4]]
            let matrix2 = [[5, 6], [7, 8]]
            
            let sum = matrix1 @+ matrix2
            let product = matrix1 @* matrix2
            let transposed = matrix1 @T
            let inverse = matrix1 @I
            
            printl sum
            printl product
            printl transposed
            printl inverse
        }
    "#;

    let program = CleanParser::parse_program(source)?;
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.check(&program)?;
    let mut codegen = CodeGenerator::new();
    codegen.generate(&program)?;
    Ok(())
}

#[test]
fn test_complex_program() -> Result<(), CompilerError> {
    let source = r#"
        function fibonacci(n: integer) returns integer {
            if n is 0 {
                return 0
            }
            if n is 1 {
                return 1
            }
            return fibonacci(n - 1) + fibonacci(n - 2)
        }

        function print_matrix(mat: integer[][]) {
            iterate row in mat {
                iterate elem in row {
                    print elem
                    print " "
                }
                printl ""
            }
        }

        start() {
            // Calculate and print first 10 Fibonacci numbers
            from 0 to 9 step 1 {
                let fib = fibonacci(i)
                print fib
                print " "
            }
            printl ""

            // Create and manipulate matrices
            let mat1 = [[1, 2], [3, 4]]
            let mat2 = [[5, 6], [7, 8]]
            let result = mat1 @* mat2
            
            printl "Matrix multiplication result:"
            print_matrix(result)

            // String interpolation
            let name = "Clean Language"
            let version = "1.0"
            printl "Welcome to ${name} version ${version}!"
        }
    "#;

    let program = CleanParser::parse_program(source)?;
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.check(&program)?;
    let mut codegen = CodeGenerator::new();
    codegen.generate(&program)?;
    Ok(())
}

#[test]
fn test_full_compiler_pipeline() -> Result<(), CompilerError> {
    // Test source code - a simple Clean language program
    let source = r#"
    start()
        x = 1 + 2 * 3
        y = x + 5
        print(y)
    "#;

    // Step 1: Parse the program
    let program = CleanParser::parse_program(source)?;
    
    // Verify the AST structure
    assert!(!program.functions.is_empty());
    assert_eq!(program.functions[0].name, "start");
    assert_eq!(program.functions[0].parameters.len(), 0);
    assert!(!program.functions[0].body.is_empty());
    
    // Step 2: Semantic analysis
    let semantic_analyzer = SemanticAnalyzer::new();
    let analyzed_program = semantic_analyzer.analyze(&program)?;
    
    // Step 3: Code generation
    let mut code_generator = CodeGenerator::new();
    let wasm_binary = code_generator.generate(&analyzed_program)?;
    
    // Step 4: Execute the generated WebAssembly
    let engine = Engine::default();
    let module = Module::new(&engine, &wasm_binary)?;
    let mut store = Store::new(&engine, ());
    
    // Create memory for the WebAssembly instance
    let memory_manager = MemoryManager::new(1, Some(10));
    
    // Mock print function to capture output
    let mut output = Vec::new();
    let print_func = Func::wrap(&mut store, |value: i32| {
        output.push(value);
    });
    
    // Create the instance with imported functions
    let instance = Instance::new(&mut store, &module, &[print_func.into()])?;
    
    // Get the start function and call it
    let start_func = instance.get_func(&mut store, "start").expect("start function not found");
    start_func.call(&mut store, &[], &mut [])?;
    
    // Verify the output is correct
    // Output should be y = x + 5 = (1 + 2 * 3) + 5 = (1 + 6) + 5 = 7 + 5 = 12
    assert_eq!(output, vec![12]);
    
    Ok(())
}

#[test]
fn test_error_handling_integration() -> Result<(), ()> {
    // Test source code with deliberate errors
    let source_with_type_error = r#"
    start()
        x = "hello"
        y = x + 5  // Type error: cannot add string and integer
        print(y)
    "#;
    
    let result = CleanParser::parse_program(source_with_type_error)
        .and_then(|program| {
            let semantic_analyzer = SemanticAnalyzer::new();
            semantic_analyzer.analyze(&program)
        })
        .and_then(|analyzed_program| {
            let mut code_generator = CodeGenerator::new();
            code_generator.generate(&analyzed_program)
        });
    
    // Verify that we get an error and it's a type error
    match result {
        Err(CompilerError::Type { .. }) => {
            // This is expected - we should get a type error
            Ok(())
        },
        _ => {
            // We didn't get the expected error
            println!("Error: {:?}", result);
            Err(())
        }
    }
}

#[test]
fn test_module_system_integration() -> Result<(), CompilerError> {
    // Test source code for module system
    let module_a = r#"
    export function add(a, b)
        return a + b
    "#;
    
    let module_b = r#"
    import add from "module_a"
    
    start()
        result = add(5, 7)
        print(result)
    "#;
    
    // Step 1: Parse and compile module A
    let program_a = CleanParser::parse_program(module_a)?;
    let semantic_analyzer = SemanticAnalyzer::new();
    let analyzed_program_a = semantic_analyzer.analyze(&program_a)?;
    let mut code_generator_a = CodeGenerator::new();
    let wasm_binary_a = code_generator_a.generate(&analyzed_program_a)?;
    
    // Step 2: Parse and compile module B with imports from A
    let program_b = CleanParser::parse_program(module_b)?;
    let analyzed_program_b = semantic_analyzer.analyze(&program_b)?;
    let mut code_generator_b = CodeGenerator::new();
    // Add module A's exports to the imports of module B
    code_generator_b.add_import("module_a", "add", analyzed_program_a.exports.get("add").unwrap().clone());
    let wasm_binary_b = code_generator_b.generate(&analyzed_program_b)?;
    
    // Step 3: Execute the generated WebAssembly for module B
    let engine = Engine::default();
    let module_b_wasm = Module::new(&engine, &wasm_binary_b)?;
    let mut store = Store::new(&engine, ());
    
    // Mock print function to capture output
    let mut output = Vec::new();
    let print_func = Func::wrap(&mut store, |value: i32| {
        output.push(value);
    });
    
    // Create module A instance for imports
    let module_a_wasm = Module::new(&engine, &wasm_binary_a)?;
    let instance_a = Instance::new(&mut store, &module_a_wasm, &[])?;
    let add_func = instance_a.get_func(&mut store, "add").expect("add function not found");
    
    // Create module B instance with module A's exports as imports
    let instance_b = Instance::new(&mut store, &module_b_wasm, &[add_func.into(), print_func.into()])?;
    
    // Get the start function and call it
    let start_func = instance_b.get_func(&mut store, "start").expect("start function not found");
    start_func.call(&mut store, &[], &mut [])?;
    
    // Verify the output is correct
    // Output should be 5 + 7 = 12
    assert_eq!(output, vec![12]);
    
    Ok(())
} 