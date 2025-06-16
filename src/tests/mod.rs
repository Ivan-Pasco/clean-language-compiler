#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Program, Function, Statement, Expression, Value, Type, SourceLocation};
    use crate::parser::CleanParser;
    use crate::semantic::SemanticAnalyzer;
    use crate::codegen::CodeGenerator;
    use crate::error::CompilerError;
    use wasmtime::{Engine, Module, Store, Instance, Val, Func};

    fn create_test_location() -> SourceLocation {
        SourceLocation {
            line: 1,
            column: 1,
            file: "test.cl".to_string(),
        }
    }

    fn run_program(source: &str) -> Result<i32, CompilerError> {
        // Parse the program
        let program = CleanParser::parse_program(source)?;

        // Type check the program
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.check(&program)?;

        // Generate code
        let mut codegen = CodeGenerator::new();
        let wasm = codegen.generate(&program)?;

        // Run the program using wasmtime
        let engine = Engine::default();
        let module = Module::new(&engine, &wasm)?;
        let mut store = Store::new(&engine, ());
        let instance = Instance::new(&mut store, &module, &[])?;

        // Get the start function
        let start = instance.get_typed_func::<(), i32>(&mut store, "start")?;
        
        // Run the function and return the result
        start.call(&mut store, ()).map_err(CompilerError::from)
    }

    #[test]
    fn test_string_interpolation() {
        let source = r#"
            start() {
                let msg = "Hello, {{name}}!";
                printl msg;
            }
        "#;

        let program = CleanParser::parse_program(source).unwrap();
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.check(&program).unwrap();
        
        let mut codegen = CodeGenerator::new();
        let wasm_bytes = codegen.generate(&program).unwrap();
        
        // Verify WASM module structure
        assert!(wasm_bytes.len() > 0);
    }

    #[test]
    fn test_array_operations() {
        let location = create_test_location();
        let program = Program::new(
            vec![Function::new(
                "start".to_string(),
                vec![],
                Type::Unit,
                vec![
                    Statement::VariableDecl {
                        name: "numbers".to_string(),
                        type_: Some(Type::Array(Box::new(Type::Integer))),
                        initializer: Some(Expression::Array(
                            vec![
                                Expression::Literal(Value::Integer(1)),
                                Expression::Literal(Value::Integer(2)),
                                Expression::Literal(Value::Integer(3)),
                                Expression::Literal(Value::Integer(4)),
                                Expression::Literal(Value::Integer(5)),
                            ],
                            Some(location.clone()),
                        )),
                        location: Some(location.clone()),
                    },
                    Statement::Print {
                        expression: Expression::Variable("numbers".to_string()),
                        newline: true,
                        location: Some(location.clone()),
                    },
                ],
                Some(location.clone()),
            )],
            vec![],
        );

        let mut analyzer = SemanticAnalyzer::new();
        analyzer.check(&program).unwrap();
        
        let mut codegen = CodeGenerator::new();
        let wasm_bytes = codegen.generate(&program).unwrap();
        
        assert!(wasm_bytes.len() > 0);
    }

    #[test]
    fn test_array_bounds_checking() {
        let location = create_test_location();
        let program = Program::new(
            vec![Function::new(
                "start".to_string(),
                vec![],
                Type::Unit,
                vec![
                    Statement::VariableDecl {
                        name: "numbers".to_string(),
                        type_: Some(Type::Array(Box::new(Type::Integer))),
                        initializer: Some(Expression::Array(
                            vec![
                                Expression::Literal(Value::Integer(1)),
                                Expression::Literal(Value::Integer(2)),
                                Expression::Literal(Value::Integer(3)),
                            ],
                            Some(location.clone()),
                        )),
                        location: Some(location.clone()),
                    },
                    Statement::Expression {
                        expr: Expression::Index(
                            Box::new(Expression::Variable("numbers".to_string())),
                            Box::new(Expression::Literal(Value::Integer(5))),
                            Some(location.clone()),
                        ),
                        location: Some(location.clone()),
                    },
                ],
                Some(location.clone()),
            )],
            vec![],
        );

        let mut analyzer = SemanticAnalyzer::new();
        analyzer.check(&program).unwrap();
        
        let mut codegen = CodeGenerator::new();
        let wasm_bytes = codegen.generate(&program).unwrap();
        
        assert!(wasm_bytes.len() > 0);
    }

    #[test]
    fn test_string_concatenation() {
        let location = create_test_location();
        let program = Program::new(
            vec![Function::new(
                "start".to_string(),
                vec![],
                Type::Unit,
                vec![
                    Statement::VariableDecl {
                        name: "str1".to_string(),
                        type_: Some(Type::String),
                        initializer: Some(Expression::Literal(Value::String("Hello, ".to_string()))),
                        location: Some(location.clone()),
                    },
                    Statement::VariableDecl {
                        name: "str2".to_string(),
                        type_: Some(Type::String),
                        initializer: Some(Expression::Literal(Value::String("World!".to_string()))),
                        location: Some(location.clone()),
                    },
                    Statement::VariableDecl {
                        name: "result".to_string(),
                        type_: Some(Type::String),
                        initializer: Some(Expression::Binary(
                            Box::new(Expression::Variable("str1".to_string())),
                            ast::Operator::Add,
                            Box::new(Expression::Variable("str2".to_string())),
                            Some(location.clone()),
                        )),
                        location: Some(location.clone()),
                    },
                    Statement::Print {
                        expression: Expression::Variable("result".to_string()),
                        newline: true,
                        location: Some(location.clone()),
                    },
                ],
                Some(location.clone()),
            )],
            vec![],
        );

        let mut analyzer = SemanticAnalyzer::new();
        analyzer.check(&program).unwrap();
        
        let mut codegen = CodeGenerator::new();
        let wasm_bytes = codegen.generate(&program).unwrap();
        
        assert!(wasm_bytes.len() > 0);
    }

    #[test]
    fn test_array_concatenation() {
        let location = create_test_location();
        let program = Program::new(
            vec![Function::new(
                "start".to_string(),
                vec![],
                Type::Unit,
                vec![
                    Statement::VariableDecl {
                        name: "arr1".to_string(),
                        type_: Some(Type::Array(Box::new(Type::Integer))),
                        initializer: Some(Expression::Array(
                            vec![
                                Expression::Literal(Value::Integer(1)),
                                Expression::Literal(Value::Integer(2)),
                                Expression::Literal(Value::Integer(3)),
                            ],
                            Some(location.clone()),
                        )),
                        location: Some(location.clone()),
                    },
                    Statement::VariableDecl {
                        name: "arr2".to_string(),
                        type_: Some(Type::Array(Box::new(Type::Integer))),
                        initializer: Some(Expression::Array(
                            vec![
                                Expression::Literal(Value::Integer(4)),
                                Expression::Literal(Value::Integer(5)),
                                Expression::Literal(Value::Integer(6)),
                            ],
                            Some(location.clone()),
                        )),
                        location: Some(location.clone()),
                    },
                    Statement::VariableDecl {
                        name: "result".to_string(),
                        type_: Some(Type::Array(Box::new(Type::Integer))),
                        initializer: Some(Expression::Binary(
                            Box::new(Expression::Variable("arr1".to_string())),
                            ast::Operator::Add,
                            Box::new(Expression::Variable("arr2".to_string())),
                            Some(location.clone()),
                        )),
                        location: Some(location.clone()),
                    },
                    Statement::Print {
                        expression: Expression::Variable("result".to_string()),
                        newline: true,
                        location: Some(location.clone()),
                    },
                ],
                Some(location.clone()),
            )],
            vec![],
        );

        let mut analyzer = SemanticAnalyzer::new();
        analyzer.check(&program).unwrap();
        
        let mut codegen = CodeGenerator::new();
        let wasm_bytes = codegen.generate(&program).unwrap();
        
        assert!(wasm_bytes.len() > 0);
    }

    #[test]
    fn test_simple_arithmetic() -> Result<(), CompilerError> {
        let source = r#"
            start() {
                let x = 5;
                let y = 3;
                let result = x + y;
                printl result;
                result;
            }
        "#;

        let result = run_program(source)?;
        assert_eq!(result, 8); // 5 + 3 = 8
        Ok(())
    }

    #[test]
    fn test_simple_string() {
        let source = r#"
            start() {
                let message = "Hello, World!";
                printl message;
            }
        "#;

        let program = CleanParser::parse_program(source).unwrap();
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.check(&program).unwrap();
        
        let mut codegen = CodeGenerator::new();
        let wasm_bytes = codegen.generate(&program).unwrap();
        assert!(wasm_bytes.len() > 0);
    }

    #[test]
    fn test_simple_if() {
        let source = r#"
            start() {
                let x = 10;
                if x > 5 {
                    printl "Greater than 5";
                } else {
                    printl "Less than or equal to 5";
                }
            }
        "#;

        let program = CleanParser::parse_program(source).unwrap();
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.check(&program).unwrap();
        
        let mut codegen = CodeGenerator::new();
        let wasm_bytes = codegen.generate(&program).unwrap();
        assert!(wasm_bytes.len() > 0);
    }

    #[test]
    fn test_string_operations() -> Result<(), CompilerError> {
        let source = r#"
            start() {
                let greeting = "Hello, ";
                let name = "World";
                let message = greeting + name;
                printl message;
                message;
            }
        "#;

        let result = run_program(source)?;
        assert!(result > 0); // Result should be a pointer to the string
        Ok(())
    }

    #[test]
    fn test_array_operations() -> Result<(), CompilerError> {
        let source = r#"
            start() {
                let numbers = [1, 2, 3, 4, 5];
                let sum = 0;
                for num in numbers {
                    sum = sum + num;
                }
                printl sum;
                sum;
            }
        "#;

        let result = run_program(source)?;
        assert_eq!(result, 15); // 1 + 2 + 3 + 4 + 5 = 15
        Ok(())
    }
}

mod integration_tests;
mod simple_test;
mod compiler_fixes_test;
mod memory_tests;
mod parser_complex_test; 
pub mod test_runner;
pub mod simple_test_runner; 