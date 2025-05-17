use clean_language_compiler::{
    ast::{self, Program, Statement, Expression, Type, Operator, ComparisonOperator, FunctionDecl},
    parser::CleanParser,
};

mod test_utils;

#[test]
fn test_parse_nested_expressions() {
    let source = r#"
        number result = ((1 + 2) * 3) - (4 / (2 - 1))
        boolean complex_check = (a > b and c <= d) or (e != f and g < h)
    "#;
    
    let result = CleanParser::parse_program(source);
    assert!(result.is_ok(), "Failed to parse nested expressions: {:?}", result.err());
    
    let program = result.unwrap();
    assert_eq!(program.statements.len(), 2);
    
    // Test that we have correct nesting in first expression
    if let Statement::VariableDecl { type_, name, initializer } = &program.statements[0] {
        assert_eq!(name, "result");
        assert!(matches!(type_, Type::Number));
        
        // Check if we have binary op with subtraction at the top level
        if let Some(Expression::BinaryOp { operator, .. }) = initializer {
            assert!(matches!(operator, Operator::Subtract));
        } else {
            panic!("Expected a complex binary operation");
        }
    }
    
    // Test that we have correct logic operation nesting
    if let Statement::VariableDecl { type_, name, initializer } = &program.statements[1] {
        assert_eq!(name, "complex_check");
        assert!(matches!(type_, Type::Boolean));
        
        // Check if we have binary op with OR at the top level
        if let Some(Expression::BinaryOp { operator, .. }) = initializer {
            assert!(matches!(operator, Operator::Or));
        } else {
            panic!("Expected a complex binary operation");
        }
    }
}

#[test]
fn test_parse_complex_function() {
    let source = r#"
        functions:
            processData() returns number
                input:
                    number[]:
                        - data
                    string:
                        - operation
                number result = 0
                if operation == "sum"
                    iterate i from 0 to length(data)
                        result = result + data[i]
                else if operation == "average"
                    iterate i from 0 to length(data)
                        result = result + data[i]
                    result = result / length(data)
                else if operation == "max"
                    result = data[0]
                    iterate i from 1 to length(data)
                        if data[i] > result
                            result = data[i]
                else
                    print "Invalid operation"
                result
    "#;
    
    let result = CleanParser::parse_program(source);
    assert!(result.is_ok(), "Failed to parse complex function: {:?}", result.err());
    
    let program = result.unwrap();
    assert_eq!(program.functions.len(), 1);
    
    let func = &program.functions[0];
    assert_eq!(func.name, "processData");
    assert!(matches!(func.return_type, Some(Type::Number)));
    
    // Check parameters
    assert_eq!(func.parameters.len(), 2);
    
    // Check first parameter is an array type
    let data_param = func.parameters.iter().find(|p| p.name == "data");
    assert!(data_param.is_some());
    let data_param = data_param.unwrap();
    assert!(matches!(data_param.param_type, Type::Array(box_type) if matches!(*box_type, Type::Number)));
    
    // Check body contains expected statements (basic structural validation)
    assert!(!func.body.is_empty());
}

#[test]
fn test_parse_multiple_functions_with_class() {
    let source = r#"
        classes:
            MyClass
                properties:
                    number:
                        - value
                    string:
                        - name
                methods:
                    getValue() returns number
                        this.value
                    
                    setValues(input: number: newValue, string: newName)
                        this.value = newValue
                        this.name = newName
                        
                    calculate() returns number
                        this.value * 2
        
        functions:
            createObject() returns MyClass
                input:
                    number:
                        - initialValue
                    string:
                        - initialName
                MyClass obj
                obj.value = initialValue
                obj.name = initialName
                obj
                
            processObject() returns number
                input:
                    MyClass:
                        - obj
                obj.calculate()
    "#;
    
    let result = CleanParser::parse_program(source);
    assert!(result.is_ok(), "Failed to parse program with class and functions: {:?}", result.err());
    
    let program = result.unwrap();
    
    // Check classes
    assert_eq!(program.classes.len(), 1);
    let my_class = &program.classes[0];
    assert_eq!(my_class.name, "MyClass");
    assert_eq!(my_class.properties.len(), 2);
    assert_eq!(my_class.methods.len(), 3);
    
    // Check functions
    assert_eq!(program.functions.len(), 2);
    
    // Validate function names
    let function_names: Vec<&str> = program.functions.iter().map(|f| &f.name[..]).collect();
    assert!(function_names.contains(&"createObject"));
    assert!(function_names.contains(&"processObject"));
    
    // Check function parameter types
    let process_obj_func = program.functions.iter().find(|f| f.name == "processObject").unwrap();
    assert_eq!(process_obj_func.parameters.len(), 1);
    let param = &process_obj_func.parameters[0];
    assert_eq!(param.name, "obj");
    assert!(matches!(param.param_type, Type::Object(class_name) if class_name == "MyClass"));
}

#[test]
fn test_parse_error_recovery() {
    // Test with some minor syntax errors that the parser should be able to recover from
    let source = r#"
        // Missing variable type
        x = 10
        
        // Proper declaration
        number y = 20
        
        // Incorrect function call (missing parentheses)
        print x + y
    "#;
    
    let result = CleanParser::parse_program(source);
    // Even with errors, we should get some result
    assert!(result.is_ok(), "Parser failed to recover from errors: {:?}", result.err());
    
    let program = result.unwrap();
    // At minimum, we should have parsed the correct variable declaration
    assert!(!program.statements.is_empty());
    
    // Check if we at least parsed the valid statement
    let has_y_decl = program.statements.iter().any(|stmt| {
        if let Statement::VariableDecl { name, .. } = stmt {
            name == "y"
        } else {
            false
        }
    });
    
    assert!(has_y_decl, "Parser did not recover to parse valid statements");
} 