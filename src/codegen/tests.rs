//! Integration and unit tests for the code generator module.

// Make parent module items accessible
use super::*;
// Import ast module for tests if needed for constructing test cases
use crate::ast;
use crate::ast::{Expression, Statement, Value, Type, BinaryOperator, Program, Parameter, Function as AstFunction};
use wasmtime::{Engine, Module, Store, Instance, Val};
use wasm_encoder::{Instruction, ConstExpr, GlobalType, ValType};

#[test]
fn test_code_generation() {
    let mut codegen = CodeGenerator::new();
    // Example Program structure 
    let program = Program {
        statements: vec![],
        functions: vec![
            AstFunction {
                name: "add".to_string(),
                parameters: vec![
                    Parameter { name: "a".to_string(), type_: Type::Integer },
                    Parameter { name: "b".to_string(), type_: Type::Integer },
                ],
                return_type: Some(Type::Integer),
                body: vec![
                    Statement::Return {
                        value: Some(Expression::Binary(
                            Box::new(Expression::Variable("a".to_string())),
                            BinaryOperator::Add,
                            Box::new(Expression::Variable("b".to_string())),
                            None
                        )),
                        location: None
                     }
                ],
                location: None,
            }
        ],
        classes: vec![],
        constants: vec![],
    };
    
    let result = codegen.generate(&program);
    assert!(result.is_ok(), "Code generation failed: {:?}", result.err());
    let wasm_bytes = result.unwrap();
    assert!(!wasm_bytes.is_empty(), "Generated WASM bytes are empty");
    
    // TODO: Add more detailed validation of generated WASM 
    // (e.g., using wasmparser or wasmtime to validate/run the module)
    // let validation_result = wasmparser::validate(&wasm_bytes);
    // assert!(validation_result.is_ok(), "WASM validation failed: {:?}", validation_result.err());
}

#[test]
fn test_add_memory() {
    let mut codegen = CodeGenerator::new();
    let result = codegen.add_memory(1, Some(10));
    assert!(result.is_ok(), "Failed to add memory: {:?}", result.err());
}

#[test]
fn test_add_global() {
    let mut codegen = CodeGenerator::new();
    let global_type = GlobalType {
        val_type: ValType::I32,
        mutable: true,
    };
    let init_expr = ConstExpr::i32_const(42);
    codegen.add_global("test_global", global_type, &init_expr);
    // No direct way to verify this worked, but it shouldn't panic
}

#[test]
fn test_string_pool() {
    let mut pool = StringPool::new();
    
    // Test adding a new string
    let index1 = pool.add_string("hello");
    assert_eq!(index1, 0);
    
    // Test retrieving the string
    let retrieved = pool.get_string(index1);
    assert_eq!(retrieved, Some("hello"));
    
    // Test adding the same string again (should return same index)
    let index2 = pool.add_string("hello");
    assert_eq!(index1, index2);
    
    // Test adding a different string
    let index3 = pool.add_string("world");
    assert_eq!(index3, 1);
    
    // Test nonexistent index
    let nonexistent = pool.get_string(99);
    assert_eq!(nonexistent, None);
}

#[test]
fn test_memory_utils() {
    let mut memory_utils = memory::MemoryUtils::new();
    
    // Test string allocation
    let string_result = memory_utils.allocate_string("hello");
    assert!(string_result.is_ok(), "Failed to allocate string: {:?}", string_result.err());
    let string_ptr = string_result.unwrap();
    assert!(string_ptr >= memory::HEAP_START, "String pointer should be >= HEAP_START");
    
    // Test array allocation
    let array_values = vec![Value::Integer(1), Value::Integer(2), Value::Integer(3)];
    let array_result = memory_utils.allocate_array(&array_values);
    assert!(array_result.is_ok(), "Failed to allocate array: {:?}", array_result.err());
    let array_ptr = array_result.unwrap();
    assert!(array_ptr > string_ptr, "Array pointer should be after string pointer");
    
    // Test matrix allocation
    let matrix_values = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
    let matrix_result = memory_utils.allocate_matrix(&matrix_values);
    assert!(matrix_result.is_ok(), "Failed to allocate matrix: {:?}", matrix_result.err());
    let matrix_ptr = matrix_result.unwrap();
    assert!(matrix_ptr > array_ptr, "Matrix pointer should be after array pointer");
}

#[test]
fn test_type_manager() {
    let mut type_manager = type_manager::TypeManager::new();
    
    // Test adding a function type
    let params = vec![WasmType::I32, WasmType::I32];
    let return_type = Some(WasmType::I32);
    let result = type_manager.add_function_type(&params, return_type);
    assert!(result.is_ok(), "Failed to add function type: {:?}", result.err());
    
    // Check if the type was added
    let type_index = result.unwrap();
    assert_eq!(type_index, 0);
    
    // Test is_string_type
    let string_expr = Expression::Literal(Value::String("test".to_string()));
    let int_expr = Expression::Literal(Value::Integer(42));
    assert!(type_manager.is_string_type(&string_expr));
    assert!(!type_manager.is_string_type(&int_expr));
    
    // Test type conversion
    assert!(type_manager.can_convert(WasmType::I32, WasmType::F64));
    assert!(type_manager.can_convert(WasmType::F64, WasmType::I32));
    assert!(type_manager.can_convert(WasmType::I32, WasmType::I32));
    assert!(!type_manager.can_convert(WasmType::I32, WasmType::F32));
}

#[test]
fn test_instruction_generator() {
    let type_manager = type_manager::TypeManager::new();
    let mut instr_gen = instruction_generator::InstructionGenerator::new(type_manager);
    
    // Add a function mapping
    instr_gen.add_function_mapping("test_func", 0);
    assert_eq!(instr_gen.get_function_index("test_func"), Some(0));
    assert_eq!(instr_gen.get_function_index("nonexistent"), None);
    
    // Test adding parameters and finding locals
    instr_gen.add_parameter("param1", WasmType::I32);
    let local = instr_gen.find_local("param1");
    assert!(local.is_some());
    assert_eq!(local.unwrap().index, 0);
    
    // Test resetting locals
    instr_gen.reset_locals();
    assert!(instr_gen.find_local("param1").is_none());
}

#[test]
fn test_group_locals() {
    // Empty case
    let empty: Vec<ValType> = vec![];
    let grouped_empty = instruction_generator::group_locals(&empty);
    assert!(grouped_empty.is_empty());
    
    // Single type
    let single_type = vec![ValType::I32, ValType::I32, ValType::I32];
    let grouped_single = instruction_generator::group_locals(&single_type);
    assert_eq!(grouped_single, vec![(3, ValType::I32)]);
    
    // Multiple types
    let mixed_types = vec![
        ValType::I32, ValType::I32,
        ValType::F64, ValType::F64, ValType::F64,
        ValType::I32
    ];
    let grouped_mixed = instruction_generator::group_locals(&mixed_types);
    assert_eq!(grouped_mixed, vec![
        (2, ValType::I32),
        (3, ValType::F64),
        (1, ValType::I32)
    ]);
}

#[test]
fn test_memory_operations() {
    let mut codegen = CodeGenerator::new();
    // Test string allocation
    let hello_str = "hello";
    let string_ptr_result = codegen.allocate_string(hello_str);
    assert!(string_ptr_result.is_ok(), "Failed to allocate string: {:?}", string_ptr_result.err());
    let string_ptr = string_ptr_result.unwrap();
    assert!(string_ptr > 0, "String pointer should be positive");
    
    // Test retrieving the string back (might require runtime/mock memory access)
    // This part usually requires executing the generated WASM or mocking memory.
    // For a pure codegen test, we might just check that data segments were created.
    // let retrieved_string_result = codegen.get_string_from_memory(string_ptr as u64);
    // assert!(retrieved_string_result.is_ok(), "Failed to retrieve string: {:?}", retrieved_string_result.err());
    // assert_eq!(retrieved_string_result.unwrap(), hello_str);

    // TODO: Add tests for array and matrix allocation if possible in isolation
    // This might involve checking the data segments created in codegen.data_section
}

#[test]
fn test_iterate_statement() {
    let mut codegen = CodeGenerator::new();
    
    // Create an array literal with values 1, 2, 3, 4, 5
    let array_values = vec![
        Value::Integer(1),
        Value::Integer(2),
        Value::Integer(3),
        Value::Integer(4),
        Value::Integer(5),
    ];
    let array_expr = Expression::Literal(Value::Array(array_values));
    
    // Create an iterate statement over the array
    let iterate_stmt = Statement::Iterate {
        iterator: "item".to_string(),
        collection: array_expr,
        body: vec![
            // Body statements (e.g., print item)
            Statement::Print {
                expression: Expression::Variable("item".to_string()),
                newline: true,
                location: None,
            }
        ],
        location: None,
    };
    
    // Create a function with the iterate statement
    let function = ast::Function::new(
        "test_iterate".to_string(),
        vec![],
        Type::Unit,
        vec![iterate_stmt],
        None,
    );
    
    // Generate code
    let mut instructions = Vec::new();
    codegen.generate_statement(&Statement::Iterate {
        iterator: "item".to_string(),
        collection: Expression::Literal(Value::Array(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
        ])),
        body: vec![],
        location: None,
    }, &mut instructions).unwrap();
    
    // Verify the generated instructions
    assert!(!instructions.is_empty());
    // Check for loop instructions
    assert!(instructions.iter().any(|i| matches!(i, Instruction::Loop(_))));
    // Check for block instructions
    assert!(instructions.iter().any(|i| matches!(i, Instruction::Block(_))));
}

#[test]
fn test_from_to_statement() {
    let mut codegen = CodeGenerator::new();
    
    // Create a from-to statement (from 1 to 10)
    let from_to_stmt = Statement::FromTo {
        start: Expression::Literal(Value::Integer(1)),
        end: Expression::Literal(Value::Integer(10)),
        step: Some(Expression::Literal(Value::Integer(1))),
        body: vec![
            // Body statements (e.g., print counter)
            Statement::Print {
                expression: Expression::Variable("counter".to_string()),
                newline: true,
                location: None,
            }
        ],
        location: None,
    };
    
    // Generate code
    let mut instructions = Vec::new();
    codegen.generate_statement(&from_to_stmt, &mut instructions).unwrap();
    
    // Verify the generated instructions
    assert!(!instructions.is_empty());
    // Check for loop instructions
    assert!(instructions.iter().any(|i| matches!(i, Instruction::Loop(_))));
    // Check for block instructions
    assert!(instructions.iter().any(|i| matches!(i, Instruction::Block(_))));
    // Check for the step value
    assert!(instructions.iter().any(|i| matches!(i, Instruction::I32Const(1))));
}

#[test]
fn test_matrix_operations() {
    let type_manager = type_manager::TypeManager::new();
    let mut instr_gen = instruction_generator::InstructionGenerator::new(type_manager);
    
    // Register the matrix operation functions
    instr_gen.add_function_mapping("matrix_add", 0);
    instr_gen.add_function_mapping("matrix_subtract", 1);
    instr_gen.add_function_mapping("matrix_multiply", 2);
    instr_gen.add_function_mapping("matrix_transpose", 3);
    instr_gen.add_function_mapping("matrix_inverse", 4);
    
    // Create sample matrix expressions
    let matrix_a = Expression::Literal(Value::Matrix(vec![
        vec![1.0, 2.0],
        vec![3.0, 4.0]
    ]));
    
    let matrix_b = Expression::Literal(Value::Matrix(vec![
        vec![5.0, 6.0],
        vec![7.0, 8.0]
    ]));
    
    // Create source location for error context
    let location = ast::SourceLocation {
        line: 1,
        column: 1,
        file: "test.txt".to_string(),
    };
    
    // Test matrix addition
    let mut add_instructions = Vec::new();
    let add_result = instr_gen.generate_matrix_operation(
        &matrix_a, 
        &ast::MatrixOperator::Add, 
        &matrix_b,
        &location,
        &mut add_instructions
    );
    assert!(add_result.is_ok(), "Matrix addition failed: {:?}", add_result.err());
    assert_eq!(add_result.unwrap(), WasmType::I32);
    assert!(!add_instructions.is_empty());
    assert!(add_instructions.iter().any(|i| matches!(i, Instruction::Call(0))));
    
    // Test matrix subtraction
    let mut subtract_instructions = Vec::new();
    let subtract_result = instr_gen.generate_matrix_operation(
        &matrix_a, 
        &ast::MatrixOperator::Subtract, 
        &matrix_b,
        &location,
        &mut subtract_instructions
    );
    assert!(subtract_result.is_ok(), "Matrix subtraction failed: {:?}", subtract_result.err());
    assert_eq!(subtract_result.unwrap(), WasmType::I32);
    assert!(!subtract_instructions.is_empty());
    assert!(subtract_instructions.iter().any(|i| matches!(i, Instruction::Call(1))));
    
    // Test matrix multiplication
    let mut multiply_instructions = Vec::new();
    let multiply_result = instr_gen.generate_matrix_operation(
        &matrix_a, 
        &ast::MatrixOperator::Multiply, 
        &matrix_b,
        &location,
        &mut multiply_instructions
    );
    assert!(multiply_result.is_ok(), "Matrix multiplication failed: {:?}", multiply_result.err());
    assert_eq!(multiply_result.unwrap(), WasmType::I32);
    assert!(!multiply_instructions.is_empty());
    assert!(multiply_instructions.iter().any(|i| matches!(i, Instruction::Call(2))));
    
    // Test matrix transpose
    let mut transpose_instructions = Vec::new();
    let transpose_result = instr_gen.generate_matrix_operation(
        &matrix_a, 
        &ast::MatrixOperator::Transpose, 
        &matrix_b, // Note: Right operand is ignored for transpose
        &location,
        &mut transpose_instructions
    );
    assert!(transpose_result.is_ok(), "Matrix transpose failed: {:?}", transpose_result.err());
    assert_eq!(transpose_result.unwrap(), WasmType::I32);
    assert!(!transpose_instructions.is_empty());
    assert!(transpose_instructions.iter().any(|i| matches!(i, Instruction::Call(3))));
    
    // Test matrix inverse
    let mut inverse_instructions = Vec::new();
    let inverse_result = instr_gen.generate_matrix_operation(
        &matrix_a, 
        &ast::MatrixOperator::Inverse, 
        &matrix_b, // Note: Right operand is ignored for inverse
        &location,
        &mut inverse_instructions
    );
    assert!(inverse_result.is_ok(), "Matrix inverse failed: {:?}", inverse_result.err());
    assert_eq!(inverse_result.unwrap(), WasmType::I32);
    assert!(!inverse_instructions.is_empty());
    assert!(inverse_instructions.iter().any(|i| matches!(i, Instruction::Call(4))));
    
    // Test error handling when function not found
    instr_gen = instruction_generator::InstructionGenerator::new(type_manager::TypeManager::new());
    let mut error_instructions = Vec::new();
    let error_result = instr_gen.generate_matrix_operation(
        &matrix_a, 
        &ast::MatrixOperator::Add, 
        &matrix_b,
        &location,
        &mut error_instructions
    );
    assert!(error_result.is_err(), "Should fail when matrix_add function not found");
    assert!(error_result.err().unwrap().to_string().contains("matrix_add function not found"));
} 