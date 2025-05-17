use clean_language_compiler::{
    ast::{Program, Type, Statement, Expression, Function, Operator, ComparisonOperator},
    semantic::SemanticAnalyzer,
    CompilerError,
};
use crate::test_utils;

#[test]
fn test_variable_declaration() {
    let source = r#"
        number x = 42.5
        integer y = 10
        string msg = "hello"
        boolean flag = true
    "#;
    
    let program = test_utils::parse_source(source);
    let result = test_utils::analyze_program(&program);
    assert!(result.is_ok(), "Variable declarations should be valid");
}

#[test]
fn test_type_mismatch() {
    let source = r#"
        number x = "not a number"  // Type mismatch
    "#;
    
    let program = test_utils::parse_source(source);
    let result = test_utils::analyze_program(&program);
    assert!(result.is_err(), "Should detect type mismatch");
}

#[test]
fn test_undefined_variable() {
    let source = r#"
        x = 42  // x is not defined
    "#;
    
    let program = test_utils::parse_source(source);
    let result = test_utils::analyze_program(&program);
    assert!(result.is_err(), "Should detect undefined variable");
}

#[test]
fn test_function_return_type() {
    let source = r#"
        functions:
            add() returns number
                input:
                    number:
                        - x
                        - y
                return x + y

            greet() returns string
                input:
                    string:
                        - name
                return "Hello, " + name
    "#;
    
    let program = test_utils::parse_source(source);
    let result = test_utils::analyze_program(&program);
    assert!(result.is_ok(), "Function return types should be valid");
}

#[test]
fn test_invalid_return_type() {
    let source = r#"
        functions:
            add() returns number
                input:
                    number:
                        - x
                        - y
                return "not a number"  // Type mismatch
    "#;
    
    let program = test_utils::parse_source(source);
    let result = test_utils::analyze_program(&program);
    assert!(result.is_err(), "Should detect invalid return type");
}

#[test]
fn test_binary_operations() {
    let source = r#"
        number a = 10
        number b = 20
        number sum = a + b
        boolean flag = a > b
        string msg = "Hello, " + "World"
    "#;
    
    let program = test_utils::parse_source(source);
    let result = test_utils::analyze_program(&program);
    assert!(result.is_ok(), "Binary operations should be valid");
}

#[test]
fn test_invalid_binary_operation() {
    let source = r#"
        number x = 10
        string y = "hello"
        number z = x + y  // Invalid operation
    "#;
    
    let program = test_utils::parse_source(source);
    let result = test_utils::analyze_program(&program);
    assert!(result.is_err(), "Should detect invalid binary operation");
}

#[test]
fn test_array_type_checking() {
    let source = r#"
        number[] nums = [1, 2, 3]
        string[] strs = ["a", "b", "c"]
        number[] invalid = [1, "not a number", 3]  // Invalid array element
    "#;
    
    let program = test_utils::parse_source(source);
    let result = test_utils::analyze_program(&program);
    assert!(result.is_err(), "Should detect invalid array element type");
}

#[test]
fn test_function_call_arguments() {
    let source = r#"
        functions:
            add() returns number
                input:
                    number:
                        - x
                        - y
                return x + y

        number result = add(1, 2)  // Valid
        number invalid = add("not a number", 2)  // Invalid argument
    "#;
    
    let program = test_utils::parse_source(source);
    let result = test_utils::analyze_program(&program);
    assert!(result.is_err(), "Should detect invalid function argument type");
}

#[test]
fn test_class_field_access() {
    let source = r#"
        class Point
            public
                number x = 0
                number y = 0
            private
                number z = 0

        object p = new Point()
        number x_coord = p.x  // Valid public access
        number z_coord = p.z  // Invalid private access
    "#;
    
    let program = test_utils::parse_source(source);
    let result = test_utils::analyze_program(&program);
    assert!(result.is_err(), "Should detect invalid private field access");
}

#[test]
fn test_method_call() {
    let source = r#"
        class Calculator
            public
                number add(number x, number y) returns number
                    return x + y

        object calc = new Calculator()
        number result = calc.add(1, 2)  // Valid
        number invalid = calc.add("not a number", 2)  // Invalid argument
    "#;
    
    let program = test_utils::parse_source(source);
    let result = test_utils::analyze_program(&program);
    assert!(result.is_err(), "Should detect invalid method argument type");
}

#[test]
fn test_control_flow_type_checking() {
    let source = r#"
        number x = 10
        if x > 5  // Valid boolean condition
            print "Greater than 5"

        if x  // Invalid non-boolean condition
            print "This should not compile"
    "#;
    
    let program = test_utils::parse_source(source);
    let result = test_utils::analyze_program(&program);
    assert!(result.is_err(), "Should detect invalid if condition type");
}

#[test]
fn test_iterate_type_checking() {
    let source = r#"
        number[] nums = [1, 2, 3]
        iterate n in nums  // Valid array iteration
            print n

        number x = 42
        iterate n in x  // Invalid: x is not an array
            print n
    "#;
    
    let program = test_utils::parse_source(source);
    let result = test_utils::analyze_program(&program);
    assert!(result.is_err(), "Should detect invalid iterate target type");
}

#[test]
fn test_from_to_type_checking() {
    let source = r#"
        i = from 1 to 10  // Valid numeric range
            print i

        j = from "1" to "10"  // Invalid: non-numeric range
            print j
    "#;
    
    let program = test_utils::parse_source(source);
    let result = test_utils::analyze_program(&program);
    assert!(result.is_err(), "Should detect invalid from-to range types");
} 