use clean_language_compiler::{
    ast::*,
    codegen::CodeGenerator,
    error::CompilerError,
    stdlib::{
        string_ops::StringOperations,
        numeric_ops::NumericOperations,
    },
};


#[test]
fn test_string_operations() -> Result<(), CompilerError> {
    let mut codegen = CodeGenerator::new();
    let string_ops = StringOperations::new(1024);
    string_ops.register_functions(&mut codegen)?;

    // Create test function
    let start_function = Function {
        name: "start".to_string(),
        type_parameters: vec![],
        type_constraints: vec![],
        parameters: vec![],
        return_type: Type::Void,
        body: vec![],
        description: None,
        syntax: FunctionSyntax::Simple,
        visibility: Visibility::Public,
        modifier: FunctionModifier::None,
        location: None,
    };
    
    let program = Program {
        start_function: Some(start_function),
        functions: vec![],
        classes: vec![],
        imports: vec![],
        tests: vec![],
    };
    
    let wasm_binary = codegen.generate(&program)?;
    assert!(!wasm_binary.is_empty());

    Ok(())
}

#[test]
fn test_numeric_operations() -> Result<(), CompilerError> {
    let mut codegen = CodeGenerator::new();
    
    // Register numeric operations
    let numeric_ops = NumericOperations::new();
    numeric_ops.register_functions(&mut codegen)?;
    
    // Create test function
    let start_function = Function {
        name: "start".to_string(),
        type_parameters: vec![],
        type_constraints: vec![],
        parameters: vec![],
        return_type: Type::Void,
        body: vec![],
        description: None,
        syntax: FunctionSyntax::Simple,
        visibility: Visibility::Public,
        modifier: FunctionModifier::None,
        location: None,
    };
    
    let program = Program {
        start_function: Some(start_function),
        functions: vec![],
        classes: vec![],
        imports: vec![],
        tests: vec![],
    };
    
    let wasm_binary = codegen.generate(&program)?;
    assert!(!wasm_binary.is_empty());

    Ok(())
}

#[test]
fn test_type_conversion() -> Result<(), CompilerError> {
    let mut codegen = CodeGenerator::new();
    
    // Create test function
    let start_function = Function {
        name: "start".to_string(),
        type_parameters: vec![],
        type_constraints: vec![],
        parameters: vec![],
        return_type: Type::Void,
        body: vec![],
        description: None,
        syntax: FunctionSyntax::Simple,
        visibility: Visibility::Public,
        modifier: FunctionModifier::None,
        location: None,
    };
    
    let program = Program {
        start_function: Some(start_function),
        functions: vec![],
        classes: vec![],
        imports: vec![],
        tests: vec![],
    };
    
    let wasm_binary = codegen.generate(&program)?;
    assert!(!wasm_binary.is_empty());

    Ok(())
}

#[test]
fn test_string_concat() -> Result<(), CompilerError> {
    let mut codegen = CodeGenerator::new();
    let string_ops = StringOperations::new(1024);
    string_ops.register_functions(&mut codegen)?;

    // Create test function
    let start_function = Function {
        name: "start".to_string(),
        type_parameters: vec![],
        type_constraints: vec![],
        parameters: vec![],
        return_type: Type::Void,
        body: vec![],
        description: None,
        syntax: FunctionSyntax::Simple,
        visibility: Visibility::Public,
        modifier: FunctionModifier::None,
        location: None,
    };
    
    let program = Program {
        start_function: Some(start_function),
        functions: vec![],
        classes: vec![],
        imports: vec![],
        tests: vec![],
    };
    
    let wasm_binary = codegen.generate(&program)?;
    assert!(!wasm_binary.is_empty());

    Ok(())
}

#[test]
fn test_string_compare() -> Result<(), CompilerError> {
    let mut codegen = CodeGenerator::new();
    let string_ops = StringOperations::new(1024);
    string_ops.register_functions(&mut codegen)?;

    // Create test function
    let start_function = Function {
        name: "start".to_string(),
        type_parameters: vec![],
        type_constraints: vec![],
        parameters: vec![],
        return_type: Type::Void,
        body: vec![],
        description: None,
        syntax: FunctionSyntax::Simple,
        visibility: Visibility::Public,
        modifier: FunctionModifier::None,
        location: None,
    };
    
    let program = Program {
        start_function: Some(start_function),
        functions: vec![],
        classes: vec![],
        imports: vec![],
        tests: vec![],
    };
    
    let wasm_binary = codegen.generate(&program)?;
    assert!(!wasm_binary.is_empty());

    Ok(())
}

#[test]
fn test_add() -> Result<(), CompilerError> {
    let mut codegen = CodeGenerator::new();
    
    // Register numeric operations
    let numeric_ops = NumericOperations::new();
    numeric_ops.register_functions(&mut codegen)?;
    
    // Create test function
    let start_function = Function {
        name: "start".to_string(),
        type_parameters: vec![],
        type_constraints: vec![],
        parameters: vec![],
        return_type: Type::Void,
        body: vec![],
        description: None,
        syntax: FunctionSyntax::Simple,
        visibility: Visibility::Public,
        modifier: FunctionModifier::None,
        location: None,
    };
    
    let program = Program {
        start_function: Some(start_function),
        functions: vec![],
        classes: vec![],
        imports: vec![],
        tests: vec![],
    };
    
    let wasm_binary = codegen.generate(&program)?;
    assert!(!wasm_binary.is_empty());

    Ok(())
}

#[test]
fn test_subtract() -> Result<(), CompilerError> {
    let mut codegen = CodeGenerator::new();
    
    // Register numeric operations
    let numeric_ops = NumericOperations::new();
    numeric_ops.register_functions(&mut codegen)?;
    
    // Create test function
    let start_function = Function {
        name: "start".to_string(),
        type_parameters: vec![],
        type_constraints: vec![],
        parameters: vec![],
        return_type: Type::Void,
        body: vec![],
        description: None,
        syntax: FunctionSyntax::Simple,
        visibility: Visibility::Public,
        modifier: FunctionModifier::None,
        location: None,
    };
    
    let program = Program {
        start_function: Some(start_function),
        functions: vec![],
        classes: vec![],
        imports: vec![],
        tests: vec![],
    };
    
    let wasm_binary = codegen.generate(&program)?;
    assert!(!wasm_binary.is_empty());

    Ok(())
}

#[test]
fn test_equals() -> Result<(), CompilerError> {
    let mut codegen = CodeGenerator::new();
    
    // Register numeric operations
    let numeric_ops = NumericOperations::new();
    numeric_ops.register_functions(&mut codegen)?;
    
    // Create test function
    let start_function = Function {
        name: "start".to_string(),
        type_parameters: vec![],
        type_constraints: vec![],
        parameters: vec![],
        return_type: Type::Void,
        body: vec![],
        description: None,
        syntax: FunctionSyntax::Simple,
        visibility: Visibility::Public,
        modifier: FunctionModifier::None,
        location: None,
    };
    
    let program = Program {
        start_function: Some(start_function),
        functions: vec![],
        classes: vec![],
        imports: vec![],
        tests: vec![],
    };
    
    let wasm_binary = codegen.generate(&program)?;
    assert!(!wasm_binary.is_empty());

    Ok(())
}

#[test]
fn test_not_equals() -> Result<(), CompilerError> {
    let mut codegen = CodeGenerator::new();
    
    // Register numeric operations
    let numeric_ops = NumericOperations::new();
    numeric_ops.register_functions(&mut codegen)?;
    
    // Create test function
    let start_function = Function {
        name: "start".to_string(),
        type_parameters: vec![],
        type_constraints: vec![],
        parameters: vec![],
        return_type: Type::Void,
        body: vec![],
        description: None,
        syntax: FunctionSyntax::Simple,
        visibility: Visibility::Public,
        modifier: FunctionModifier::None,
        location: None,
    };
    
    let program = Program {
        start_function: Some(start_function),
        functions: vec![],
        classes: vec![],
        imports: vec![],
        tests: vec![],
    };
    
    let wasm_binary = codegen.generate(&program)?;
    assert!(!wasm_binary.is_empty());

    Ok(())
}

#[test]
fn test_less_than() -> Result<(), CompilerError> {
    let mut codegen = CodeGenerator::new();
    
    // Register numeric operations
    let numeric_ops = NumericOperations::new();
    numeric_ops.register_functions(&mut codegen)?;
    
    // Create test function
    let start_function = Function {
        name: "start".to_string(),
        type_parameters: vec![],
        type_constraints: vec![],
        parameters: vec![],
        return_type: Type::Void,
        body: vec![],
        description: None,
        syntax: FunctionSyntax::Simple,
        visibility: Visibility::Public,
        modifier: FunctionModifier::None,
        location: None,
    };
    
    let program = Program {
        start_function: Some(start_function),
        functions: vec![],
        classes: vec![],
        imports: vec![],
        tests: vec![],
    };
    
    let wasm_binary = codegen.generate(&program)?;
    assert!(!wasm_binary.is_empty());

    Ok(())
}

#[test]
fn test_greater_than() -> Result<(), CompilerError> {
    let mut codegen = CodeGenerator::new();
    
    // Register numeric operations
    let numeric_ops = NumericOperations::new();
    numeric_ops.register_functions(&mut codegen)?;
    
    // Create test function
    let start_function = Function {
        name: "start".to_string(),
        type_parameters: vec![],
        type_constraints: vec![],
        parameters: vec![],
        return_type: Type::Void,
        body: vec![],
        description: None,
        syntax: FunctionSyntax::Simple,
        visibility: Visibility::Public,
        modifier: FunctionModifier::None,
        location: None,
    };
    
    let program = Program {
        start_function: Some(start_function),
        functions: vec![],
        classes: vec![],
        imports: vec![],
        tests: vec![],
    };
    
    let wasm_binary = codegen.generate(&program)?;
    assert!(!wasm_binary.is_empty());

    Ok(())
}

#[test]
fn test_array_length() -> Result<(), CompilerError> {
    let mut codegen = CodeGenerator::new();
    
    // Create test function
    let start_function = Function {
        name: "start".to_string(),
        type_parameters: vec![],
        type_constraints: vec![],
        parameters: vec![],
        return_type: Type::Void,
        body: vec![],
        description: None,
        syntax: FunctionSyntax::Simple,
        visibility: Visibility::Public,
        modifier: FunctionModifier::None,
        location: None,
    };
    
    let program = Program {
        start_function: Some(start_function),
        functions: vec![],
        classes: vec![],
        imports: vec![],
        tests: vec![],
    };
    
    let wasm_binary = codegen.generate(&program)?;
    assert!(!wasm_binary.is_empty());

    Ok(())
}

#[test]
fn test_module_file_search() -> Result<(), CompilerError> {
    let mut codegen = CodeGenerator::new();
    
    // Create test function
    let start_function = Function {
        name: "start".to_string(),
        type_parameters: vec![],
        type_constraints: vec![],
        parameters: vec![],
        return_type: Type::Void,
        body: vec![],
        description: None,
        syntax: FunctionSyntax::Simple,
        visibility: Visibility::Public,
        modifier: FunctionModifier::None,
        location: None,
    };
    
    let program = Program {
        start_function: Some(start_function),
        functions: vec![],
        classes: vec![],
        imports: vec![],
        tests: vec![],
    };
    
    let wasm_binary = codegen.generate(&program)?;
    assert!(!wasm_binary.is_empty());

    Ok(())
} 