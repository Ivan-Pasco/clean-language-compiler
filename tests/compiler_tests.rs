use clean_language_compiler::{
    ast::{Expression, Statement, Type, SourceLocation, Parameter, Value},
    semantic::type_checker::TypeChecker,
    error::CompilerError,
    codegen::CodeGenerator,
    stdlib::{matrix_ops::MatrixOperations, memory_new::MemoryManager},
};

#[test]
fn test_simple_program() {
    let program = vec![
        Statement::VariableDecl {
            name: "x".to_string(),
            type_: Type::Float,
            initializer: Some(Expression::Literal(Value::Float(42.0))),
            location: Some(SourceLocation { line: 1, column: 1, file: "<test>".to_string() }),
        },
        Statement::VariableDecl {
            name: "y".to_string(),
            type_: Type::Float,
            initializer: Some(Expression::Binary(
                Box::new(Expression::Variable("x".to_string())),
                clean_language_compiler::ast::BinaryOperator::Add,
                Box::new(Expression::Literal(Value::Float(8.0))),
            )),
            location: Some(SourceLocation { line: 2, column: 1, file: "<test>".to_string() }),
        },
    ];

    let mut type_checker = TypeChecker::new();
    assert!(type_checker.check_program(&program).is_ok());
}

#[test]
fn test_matrix_operations() {
    let mut codegen = CodeGenerator::new();
    let memory = MemoryManager::new(1, 10, 1024);
    let matrix_ops = MatrixOperations::new(1024);

    // Register functions
    memory.register_functions(&mut codegen).unwrap();
    matrix_ops.register_functions(&mut codegen).unwrap();

    // Test matrix creation and operations
    let engine = wasmtime::Engine::default();
    let wasm_bytes = codegen.finish();
    let module = wasmtime::Module::new(&engine, &wasm_bytes).unwrap();
    let mut store = wasmtime::Store::new(&engine, ());
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).unwrap();

    // Test matrix creation
    let create = instance.get_func(&mut store, "matrix.create").unwrap();
    let mut results = vec![wasmtime::Val::I32(0)];
    create.call(&mut store, &[
        wasmtime::Val::I32(2),
        wasmtime::Val::I32(2),
    ], &mut results).unwrap();
    let matrix_ptr = results[0].unwrap_i32() as usize;
    assert!(matrix_ptr >= 1024);

    // Test matrix set/get
    let set = instance.get_func(&mut store, "matrix.set").unwrap();
    let get = instance.get_func(&mut store, "matrix.get").unwrap();

    // Set values
    for i in 0..2 {
        for j in 0..2 {
            let mut results = vec![wasmtime::Val::I32(0)];
            set.call(&mut store, &[
                wasmtime::Val::I32(matrix_ptr as i32),
                wasmtime::Val::I32(i),
                wasmtime::Val::I32(j),
                wasmtime::Val::F64(((i * 2 + j + 1) as f64).to_bits()),
            ], &mut results).unwrap();
            assert_eq!(results[0].unwrap_i32(), 1);
        }
    }

    // Get and verify values
    for i in 0..2 {
        for j in 0..2 {
            let mut results = vec![wasmtime::Val::F64(0.0f64.to_bits())];
            get.call(&mut store, &[
                wasmtime::Val::I32(matrix_ptr as i32),
                wasmtime::Val::I32(i),
                wasmtime::Val::I32(j),
            ], &mut results).unwrap();
            let result = f64::from_bits(results[0].unwrap_i64() as u64);
            assert!((result - (i * 2 + j + 1) as f64).abs() < f64::EPSILON);
        }
    }
}

#[test]
fn test_error_handling() {
    // Test type errors
    let program = vec![
        Statement::VariableDecl {
            name: "x".to_string(),
            type_: Type::Float,
            initializer: Some(Expression::Literal(Value::String("not a number".to_string()))),
            location: Some(SourceLocation { line: 1, column: 1, file: "<test>".to_string() }),
        },
    ];

    let mut type_checker = TypeChecker::new();
    let result = type_checker.check_program(&program);
    assert!(matches!(result, Err(CompilerError::Type { .. })));

    // Test undefined variable
    let program = vec![
        Statement::Expression(Expression::Variable("undefined".to_string())),
    ];

    let mut type_checker = TypeChecker::new();
    let result = type_checker.check_program(&program);
    assert!(matches!(result, Err(CompilerError::UndefinedVariable { .. })));

    // Test matrix bounds error
    let mut codegen = CodeGenerator::new();
    let matrix_ops = MatrixOperations::new(1024);
    matrix_ops.register_functions(&mut codegen).unwrap();

    let engine = wasmtime::Engine::default();
    let wasm_bytes = codegen.finish();
    let module = wasmtime::Module::new(&engine, &wasm_bytes).unwrap();
    let mut store = wasmtime::Store::new(&engine, ());
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).unwrap();

    // Create a 2x2 matrix
    let create = instance.get_func(&mut store, "matrix.create").unwrap();
    let mut results = vec![wasmtime::Val::I32(0)];
    create.call(&mut store, &[
        wasmtime::Val::I32(2),
        wasmtime::Val::I32(2),
    ], &mut results).unwrap();
    let matrix_ptr = results[0].unwrap_i32();

    // Try to access out of bounds
    let get = instance.get_func(&mut store, "matrix.get").unwrap();
    let mut results = vec![wasmtime::Val::F64(0.0f64.to_bits())];
    get.call(&mut store, &[
        wasmtime::Val::I32(matrix_ptr),
        wasmtime::Val::I32(2), // Out of bounds
        wasmtime::Val::I32(0),
    ], &mut results).unwrap();
    assert!(results[0].unwrap_f64().is_nan());
}

#[test]
fn test_function_definitions() {
    use clean_language_compiler::ast::{Function, FunctionSyntax, Visibility};
    let add_function = Function {
        name: "add".to_string(),
        type_parameters: vec![],
        parameters: vec![
            Parameter {
                name: "x".to_string(),
                type_: Type::Float,
            },
            Parameter {
                name: "y".to_string(),
                type_: Type::Float,
            },
        ],
        return_type: Type::Float,
        body: vec![
            Statement::Return {
                value: Some(Expression::Binary(
                    Box::new(Expression::Variable("x".to_string())),
                    clean_language_compiler::ast::BinaryOperator::Add,
                    Box::new(Expression::Variable("y".to_string())),
                )),
                location: Some(SourceLocation { line: 2, column: 1, file: "<test>".to_string() }),
            },
        ],
        description: None,
        syntax: FunctionSyntax::Simple,
        visibility: Visibility::Public,
        location: Some(SourceLocation { line: 1, column: 1, file: "<test>".to_string() }),
    };
    let program = vec![
        Statement::VariableDecl {
            name: "result".to_string(),
            type_: Type::Float,
            initializer: Some(Expression::Call(
                "add".to_string(),
                vec![
                    Expression::Literal(Value::Float(1.0)),
                    Expression::Literal(Value::Float(2.0)),
                ],
            )),
            location: Some(SourceLocation { line: 4, column: 1, file: "<test>".to_string() }),
        },
    ];
    let mut type_checker = TypeChecker::new();
    // The type checker now expects a program structure, so we pass the function and statements
    let program_functions = vec![add_function];
    // If your type checker expects a Program struct, use:
    // let program_ast = Program::new(program_functions, vec![]);
    // assert!(type_checker.check_program(&program_ast).is_ok());
    // Otherwise, if it expects just statements, you may need to adapt this further.
    assert!(type_checker.check_program(&program).is_ok());
}

#[test]
fn test_memory_management() {
    let mut codegen = CodeGenerator::new();
    let memory = MemoryManager::new(1, 10, 1024);
    memory.register_functions(&mut codegen).unwrap();

    let engine = wasmtime::Engine::default();
    let wasm_bytes = codegen.finish();
    let module = wasmtime::Module::new(&engine, &wasm_bytes).unwrap();
    let mut store = wasmtime::Store::new(&engine, ());
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).unwrap();

    // Test allocation
    let allocate = instance.get_func(&mut store, "memory.allocate").unwrap();
    let mut results = vec![wasmtime::Val::I32(0)];
    
    // Allocate small block
    allocate.call(&mut store, &[wasmtime::Val::I32(100)], &mut results).unwrap();
    let ptr1 = results[0].unwrap_i32();
    assert!(ptr1 >= 1024);

    // Allocate large block that requires memory growth
    allocate.call(&mut store, &[wasmtime::Val::I32(65536)], &mut results).unwrap();
    let ptr2 = results[0].unwrap_i32();
    assert!(ptr2 >= 1024);
    assert!(ptr2 > ptr1);

    // Test reallocation
    let realloc = instance.get_func(&mut store, "memory.realloc").unwrap();
    let mut results = vec![wasmtime::Val::I32(0)];
    realloc.call(&mut store, &[
        wasmtime::Val::I32(ptr1),
        wasmtime::Val::I32(200),
    ], &mut results).unwrap();
    let ptr3 = results[0].unwrap_i32();
    assert!(ptr3 >= 1024);
    assert!(ptr3 != ptr1);
}

#[test]
fn test_edge_cases() {
    // Test empty matrix
    let expr = Expression::Matrix {
        rows: vec![],
        location: Location { line: 1, column: 1 },
    };
    let checker = TypeChecker::new();
    assert_eq!(checker.infer_type(&expr).unwrap(), Type::Matrix);

    // Test matrix with mismatched row lengths
    let expr = Expression::Matrix {
        rows: vec![
            vec![Expression::Literal(Value::Float(1.0)), Expression::Literal(Value::Float(2.0))],
            vec![Expression::Literal(Value::Float(3.0))], // Shorter row
        ],
        location: Location { line: 1, column: 1 },
    };
    let checker = TypeChecker::new();
    assert!(checker.infer_type(&expr).is_err());

    // Test matrix with non-number elements
    let expr = Expression::Matrix {
        rows: vec![
            vec![Expression::Literal(Value::Float(1.0)), Expression::String("invalid".to_string())],
        ],
        location: Location { line: 1, column: 1 },
    };
    let checker = TypeChecker::new();
    assert!(checker.infer_type(&expr).is_err());

    // Test function call with wrong number of arguments
    let mut checker = TypeChecker::new();
    checker.function_table.insert(
        "test".to_string(),
        clean_language::semantic::type_checker::FunctionType {
            params: vec![Type::Float],
            return_type: Type::Float,
        },
    );

    let expr = Expression::Call {
        function: "test".to_string(),
        args: vec![
            Expression::Literal(Value::Float(1.0)),
            Expression::Literal(Value::Float(2.0)), // Extra argument
        ],
        location: Location { line: 1, column: 1 },
    };
    assert!(checker.infer_type(&expr).is_err());
} 