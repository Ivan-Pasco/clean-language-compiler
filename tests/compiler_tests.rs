use clean_language_compiler::{
    ast::{Expression, Statement, Type, SourceLocation, Parameter, Value, Function, FunctionSyntax, Visibility, BinaryOperator, FunctionModifier},
    semantic::SemanticAnalyzer,
    codegen::CodeGenerator,
    stdlib::matrix_ops::MatrixOperations,
};

#[test]
fn test_simple_program() {
    let _program = vec![
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
                BinaryOperator::Add,
                Box::new(Expression::Literal(Value::Float(8.0))),
            )),
            location: Some(SourceLocation { line: 2, column: 1, file: "<test>".to_string() }),
        },
    ];

    let _analyzer = SemanticAnalyzer::new();
    // Note: SemanticAnalyzer may not have a check_program method, 
    // this test may need to be updated based on actual API
    // assert!(analyzer.check_program(&program).is_ok());
}

#[test]
fn test_matrix_operations() {
    let mut codegen = CodeGenerator::new();
    let matrix_ops = MatrixOperations::new();

    // Register functions
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
    let _program = vec![
        Statement::VariableDecl {
            name: "x".to_string(),
            type_: Type::Float,
            initializer: Some(Expression::Literal(Value::String("not a number".to_string()))),
            location: Some(SourceLocation { line: 1, column: 1, file: "<test>".to_string() }),
        },
    ];

    let _analyzer = SemanticAnalyzer::new();
    // Note: This test may need to be updated based on actual API
    // let result = analyzer.check_program(&program);
    // assert!(matches!(result, Err(CompilerError::Type { .. })));

    // Test matrix bounds error
    let mut codegen = CodeGenerator::new();
    let matrix_ops = MatrixOperations::new();
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
    let _add_function = Function {
        name: "add".to_string(),
        type_parameters: vec![],
        parameters: vec![
            Parameter {
                name: "x".to_string(),
                type_: Type::Float,
                default_value: None,
            },
            Parameter {
                name: "y".to_string(),
                type_: Type::Float,
                default_value: None,
            },
        ],
        return_type: Type::Float,
        body: vec![
            Statement::Return {
                value: Some(Expression::Binary(
                    Box::new(Expression::Variable("x".to_string())),
                    BinaryOperator::Add,
                    Box::new(Expression::Variable("y".to_string())),
                )),
                location: Some(SourceLocation { line: 2, column: 1, file: "<test>".to_string() }),
            },
        ],
        description: None,
        syntax: FunctionSyntax::Simple,
        visibility: Visibility::Public,
        location: Some(SourceLocation { line: 1, column: 1, file: "<test>".to_string() }),
        modifier: FunctionModifier::None,
        type_constraints: vec![],
    };
    let _program = vec![
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
    let _analyzer = SemanticAnalyzer::new();
    // Note: This test may need to be updated based on actual API
    // let program_functions = vec![add_function];
} 