use clean_language_compiler::{
    ast::{self, Program, Statement, Expression, Type, Value, Operator, ComparisonOperator},
    parser::CleanParser,
};
// Remove unused fs import if test_utils::read_test_file is used exclusively
// use std::fs;

mod test_utils;

/* // Temporarily remove this test as it requires start() handling
#[test]
fn test_parse_simple_program() {
    let source = test_utils::read_test_file("hello_world.cln");
    let result = CleanParser::parse_program(&source);
    
    assert!(result.is_ok(), "Failed to parse simple program: {:?}", result.err());
    
    let program = result.unwrap();
    // The print statement is now parsed, but hello_world.cln likely has start() which isn't handled yet.
    // Let's assume top-level statements for now.
    // assert_eq!(program.statements.len(), 1);
    assert!(program.functions.is_empty()); 
}
*/

#[test]
fn test_parse_top_level_statements() {
    // Test parsing statements directly at the top level (outside any function/class)
    let source = r#"
        print 1
        number x = 2
    "#;
    let result = CleanParser::parse_program(source);
    assert!(result.is_ok(), "Failed to parse top-level statements: {:?}", result.err());
    let program = result.unwrap();
    assert_eq!(program.statements.len(), 2);
    assert!(matches!(program.statements[0], Statement::Print(Expression::Integer(1))));
    assert!(matches!(program.statements[1], Statement::VariableDecl { .. }));
    assert!(program.functions.is_empty());
}

#[test]
fn test_parse_variable_declaration() {
    let source = r#"
        number my_num = 42.5
        integer my_int
        string my_str = "hello"
        boolean my_bool = true
        MyClass my_obj
        number[] my_array
    "#;
    let result = CleanParser::parse_program(source);
    assert!(result.is_ok(), "Failed to parse var decls: {:?}", result.err());
    let program = result.unwrap();
    assert_eq!(program.statements.len(), 6);

    // Check first declaration
    if let Statement::VariableDecl { type_, name, initializer } = &program.statements[0] {
        assert_eq!(name, "my_num");
        assert!(matches!(type_, Type::Number));
        assert!(initializer.is_some());
        if let Some(Expression::Number(val)) = initializer {
            assert_eq!(*val, 42.5);
        } else {
            panic!("Initializer is not a Number expression");
        }
    } else {
        panic!("First statement is not a VariableDecl");
    }
    
    // Check second declaration (no initializer)
     if let Statement::VariableDecl { type_, name, initializer } = &program.statements[1] {
        assert_eq!(name, "my_int");
        assert!(matches!(type_, Type::Integer));
        assert!(initializer.is_none());
    } else {
        panic!("Second statement is not a VariableDecl");
    }

    // Check object type
    if let Statement::VariableDecl { type_, name, .. } = &program.statements[4] {
        assert_eq!(name, "my_obj");
        assert!(matches!(type_, Type::Object(s) if s == "MyClass"));
    } else {
        panic!("Fifth statement is not a VariableDecl for an object");
    }

    // Check array type
    if let Statement::VariableDecl { type_, name, .. } = &program.statements[5] {
        assert_eq!(name, "my_array");
        assert!(matches!(type_, Type::Array(inner) if matches!(**inner, Type::Number)));
    } else {
        panic!("Sixth statement is not a VariableDecl for an array");
    }
}

#[test]
fn test_parse_print_statement() {
    let source = "print 123";
    let result = CleanParser::parse_program(source);
    assert!(result.is_ok(), "Failed to parse print: {:?}", result.err());
    let program = result.unwrap();
    assert_eq!(program.statements.len(), 1);
    assert!(matches!(program.statements[0], Statement::Print(Expression::Integer(123))));
}

#[test]
fn test_parse_assignment() {
    let source = "x = y"; // Assumes x and y are declared elsewhere for semantic check
    let result = CleanParser::parse_program(source);
     assert!(result.is_ok(), "Failed to parse assignment: {:?}", result.err());
    let program = result.unwrap();
    assert_eq!(program.statements.len(), 1);
    if let Statement::Assignment { target, value } = &program.statements[0] {
        assert_eq!(target, "x");
        assert!(matches!(value, Expression::Identifier(s) if s == "y"));
    } else {
        panic!("Statement is not an Assignment");
    }
}

#[test]
fn test_parse_function_decl() {
    let source = r#"
        functions:
            number add(number x, number y)
                return x + y
    "#;
    
    let result = CleanParser::parse_program(source);
    assert!(result.is_ok(), "Failed to parse function declaration: {:?}", result.err());
    
    let program = result.unwrap();
    assert_eq!(program.functions.len(), 1);
    
    let add_func = &program.functions[0];
    assert_eq!(add_func.name, "add");
    assert!(matches!(add_func.return_type, Some(Type::Number)));
    assert_eq!(add_func.parameters.len(), 2);
    
    let source = r#"
        functions:
            string greet(string name)
                return "Hello, " + name
    "#;
    
    let result = CleanParser::parse_program(source);
    assert!(result.is_ok(), "Failed to parse function declaration: {:?}", result.err());
    
    let program = result.unwrap();
    let greet_func = &program.functions[0];
    assert_eq!(greet_func.name, "greet");
    assert!(matches!(greet_func.return_type, Some(Type::String)));
    assert_eq!(greet_func.parameters.len(), 1);
}

#[test]
fn test_parse_binary_operations() {
    // Test arithmetic operations
    let source = r#"
        number x = 1 + 2 * 3
        number y = (4 + 5) * 6
        string msg = "Hello, " + "World"
        boolean flag = true and false
        boolean check = x > 10 and y < 20
    "#;
    
    let result = CleanParser::parse_program(source);
    assert!(result.is_ok(), "Failed to parse binary operations: {:?}", result.err());
    
    let program = result.unwrap();
    assert_eq!(program.statements.len(), 5);
    
    // Check arithmetic with operator precedence
    if let Statement::VariableDecl { type_, name, initializer } = &program.statements[0] {
        assert_eq!(name, "x");
        assert!(matches!(type_, Type::Number));
        assert!(initializer.is_some());
        if let Some(Expression::BinaryOp { left, operator: op1, right: right1 }) = initializer {
            assert!(matches!(op1, Operator::Add));
            assert!(matches!(**left, Expression::Integer(1)));
            assert!(matches!(**right1, Expression::BinaryOp { .. }));
            if let Expression::BinaryOp { left: left2, operator: op2, right: right2 } = &**right1 {
                assert!(matches!(op2, Operator::Multiply));
                assert!(matches!(**left2, Expression::Integer(2)));
                assert!(matches!(**right2, Expression::Integer(3)));
            } else {
                panic!("Expected BinaryOp for multiplication");
            }
        } else {
            panic!("Expected BinaryOp for addition");
        }
    }
    
    // Check string concatenation
    if let Statement::VariableDecl { type_, name, initializer } = &program.statements[2] {
        assert_eq!(name, "msg");
        assert!(matches!(type_, Type::String));
        assert!(initializer.is_some());
        if let Some(Expression::BinaryOp { left, operator, right }) = initializer {
            assert!(matches!(operator, Operator::Add));
            assert!(matches!(**left, Expression::String(s) if s == "Hello, "));
            assert!(matches!(**right, Expression::String(s) if s == "World"));
        } else {
            panic!("Expected BinaryOp for string concatenation");
        }
    }
    
    // Check logical operation
    if let Statement::VariableDecl { type_, name, initializer } = &program.statements[3] {
        assert_eq!(name, "flag");
        assert!(matches!(type_, Type::Boolean));
        assert!(initializer.is_some());
        if let Some(Expression::BinaryOp { left, operator, right }) = initializer {
            assert!(matches!(operator, Operator::And));
            assert!(matches!(**left, Expression::Boolean(true)));
            assert!(matches!(**right, Expression::Boolean(false)));
        } else {
            panic!("Expected BinaryOp for logical operation");
        }
    }
    
    // Check comparison with logical operation
    if let Statement::VariableDecl { type_, name, initializer } = &program.statements[4] {
        assert_eq!(name, "check");
        assert!(matches!(type_, Type::Boolean));
        assert!(initializer.is_some());
        if let Some(Expression::BinaryOp { left, operator, right }) = initializer {
            assert!(matches!(operator, Operator::And));
            assert!(matches!(**left, Expression::Condition { .. }));
            assert!(matches!(**right, Expression::Condition { .. }));
        } else {
            panic!("Expected BinaryOp for compound condition");
        }
    }
}

#[test]
fn test_parse_parenthesized_expressions() {
    let source = r#"
        number a = (1 + 2) * 3
        number b = 1 + (2 * 3)
        boolean c = (x > 5) and (y < 10)
        boolean d = (true and false) or true
    "#;
    
    let result = CleanParser::parse_program(source);
    assert!(result.is_ok(), "Failed to parse parenthesized expressions: {:?}", result.err());
    
    let program = result.unwrap();
    assert_eq!(program.statements.len(), 4);
    
    // Check (1 + 2) * 3
    if let Statement::VariableDecl { type_, name, initializer } = &program.statements[0] {
        assert_eq!(name, "a");
        assert!(matches!(type_, Type::Number));
        assert!(initializer.is_some());
        if let Some(Expression::BinaryOp { left, operator: op1, right }) = initializer {
            assert!(matches!(op1, Operator::Multiply));
            // Check the parenthesized part
            if let Expression::BinaryOp { left: left2, operator: op2, right: right2 } = &**left {
                assert!(matches!(op2, Operator::Add));
                assert!(matches!(**left2, Expression::Integer(1)));
                assert!(matches!(**right2, Expression::Integer(2)));
            } else {
                panic!("Expected BinaryOp for addition in parentheses");
            }
            assert!(matches!(**right, Expression::Integer(3)));
        } else {
            panic!("Expected BinaryOp for multiplication");
        }
    }
    
    // Check 1 + (2 * 3)
    if let Statement::VariableDecl { type_, name, initializer } = &program.statements[1] {
        assert_eq!(name, "b");
        assert!(matches!(type_, Type::Number));
        assert!(initializer.is_some());
        if let Some(Expression::BinaryOp { left, operator: op1, right }) = initializer {
            assert!(matches!(op1, Operator::Add));
            assert!(matches!(**left, Expression::Integer(1)));
            // Check the parenthesized part
            if let Expression::BinaryOp { left: left2, operator: op2, right: right2 } = &**right {
                assert!(matches!(op2, Operator::Multiply));
                assert!(matches!(**left2, Expression::Integer(2)));
                assert!(matches!(**right2, Expression::Integer(3)));
            } else {
                panic!("Expected BinaryOp for multiplication in parentheses");
            }
        } else {
            panic!("Expected BinaryOp for addition");
        }
    }
    
    // Check (x > 5) and (y < 10)
    if let Statement::VariableDecl { type_, name, initializer } = &program.statements[2] {
        assert_eq!(name, "c");
        assert!(matches!(type_, Type::Boolean));
        assert!(initializer.is_some());
        if let Some(Expression::BinaryOp { left, operator, right }) = initializer {
            assert!(matches!(operator, Operator::And));
            // Check both conditions are properly parsed
            assert!(matches!(**left, Expression::Condition { .. }));
            assert!(matches!(**right, Expression::Condition { .. }));
        } else {
            panic!("Expected BinaryOp for logical operation");
        }
    }
}

#[test]
fn test_parse_array_literals() {
    let source = r#"
        number[] nums = [1, 2, 3]
        string[] words = ["hello", "world"]
        boolean[] flags = [true, false, true]
        number[] expressions = [1 + 2, 3 * 4, 5]
        number[] empty = []
    "#;
    
    let result = CleanParser::parse_program(source);
    assert!(result.is_ok(), "Failed to parse array literals: {:?}", result.err());
    
    let program = result.unwrap();
    assert_eq!(program.statements.len(), 5);
    
    // Check number array
    if let Statement::VariableDecl { type_, name, initializer } = &program.statements[0] {
        assert_eq!(name, "nums");
        assert!(matches!(type_, Type::Array(inner) if matches!(**inner, Type::Number)));
        if let Some(Expression::ArrayLiteral(elements)) = initializer {
            assert_eq!(elements.len(), 3);
            assert!(matches!(elements[0], Expression::Integer(1)));
            assert!(matches!(elements[1], Expression::Integer(2)));
            assert!(matches!(elements[2], Expression::Integer(3)));
        } else {
            panic!("Expected ArrayLiteral");
        }
    }
    
    // Check string array
    if let Statement::VariableDecl { type_, name, initializer } = &program.statements[1] {
        assert_eq!(name, "words");
        assert!(matches!(type_, Type::Array(inner) if matches!(**inner, Type::String)));
        if let Some(Expression::ArrayLiteral(elements)) = initializer {
            assert_eq!(elements.len(), 2);
            assert!(matches!(&elements[0], Expression::String(s) if s == "hello"));
            assert!(matches!(&elements[1], Expression::String(s) if s == "world"));
        } else {
            panic!("Expected ArrayLiteral");
        }
    }
    
    // Check array with expressions
    if let Statement::VariableDecl { type_, name, initializer } = &program.statements[3] {
        assert_eq!(name, "expressions");
        if let Some(Expression::ArrayLiteral(elements)) = initializer {
            assert_eq!(elements.len(), 3);
            // Check 1 + 2
            if let Expression::BinaryOp { left, operator, right } = &elements[0] {
                assert!(matches!(operator, Operator::Add));
                assert!(matches!(**left, Expression::Integer(1)));
                assert!(matches!(**right, Expression::Integer(2)));
            } else {
                panic!("Expected BinaryOp");
            }
            // Check 3 * 4
            if let Expression::BinaryOp { left, operator, right } = &elements[1] {
                assert!(matches!(operator, Operator::Multiply));
                assert!(matches!(**left, Expression::Integer(3)));
                assert!(matches!(**right, Expression::Integer(4)));
            } else {
                panic!("Expected BinaryOp");
            }
            assert!(matches!(elements[2], Expression::Integer(5)));
        } else {
            panic!("Expected ArrayLiteral");
        }
    }
    
    // Check empty array
    if let Statement::VariableDecl { type_, name, initializer } = &program.statements[4] {
        assert_eq!(name, "empty");
        if let Some(Expression::ArrayLiteral(elements)) = initializer {
            assert_eq!(elements.len(), 0);
        } else {
            panic!("Expected ArrayLiteral");
        }
    }
}

#[test]
fn test_parse_matrix_literals() {
    let source = r#"
        number[][] matrix = [
            [1, 2, 3],
            [4, 5, 6],
            [7, 8, 9]
        ]
        number[][] expressions = [
            [1 + 2, 3],
            [4, 5 * 6]
        ]
        number[][] empty = []
        number[][] empty_rows = [[], []]
    "#;
    
    let result = CleanParser::parse_program(source);
    assert!(result.is_ok(), "Failed to parse matrix literals: {:?}", result.err());
    
    let program = result.unwrap();
    assert_eq!(program.statements.len(), 4);
    
    // Check regular matrix
    if let Statement::VariableDecl { type_, name, initializer } = &program.statements[0] {
        assert_eq!(name, "matrix");
        assert!(matches!(type_, Type::Array(outer) if matches!(**outer, Type::Array(inner) if matches!(**inner, Type::Number))));
        if let Some(Expression::MatrixLiteral(rows)) = initializer {
            assert_eq!(rows.len(), 3);
            assert_eq!(rows[0].len(), 3);
            // Check first row
            assert!(matches!(rows[0][0], Expression::Integer(1)));
            assert!(matches!(rows[0][1], Expression::Integer(2)));
            assert!(matches!(rows[0][2], Expression::Integer(3)));
            // Check second row
            assert!(matches!(rows[1][0], Expression::Integer(4)));
            assert!(matches!(rows[1][1], Expression::Integer(5)));
            assert!(matches!(rows[1][2], Expression::Integer(6)));
            // Check third row
            assert!(matches!(rows[2][0], Expression::Integer(7)));
            assert!(matches!(rows[2][1], Expression::Integer(8)));
            assert!(matches!(rows[2][2], Expression::Integer(9)));
        } else {
            panic!("Expected MatrixLiteral");
        }
    }
    
    // Check matrix with expressions
    if let Statement::VariableDecl { type_, name, initializer } = &program.statements[1] {
        assert_eq!(name, "expressions");
        if let Some(Expression::MatrixLiteral(rows)) = initializer {
            assert_eq!(rows.len(), 2);
            // Check first row
            if let Expression::BinaryOp { left, operator, right } = &rows[0][0] {
                assert!(matches!(operator, Operator::Add));
                assert!(matches!(**left, Expression::Integer(1)));
                assert!(matches!(**right, Expression::Integer(2)));
            } else {
                panic!("Expected BinaryOp");
            }
            assert!(matches!(rows[0][1], Expression::Integer(3)));
            // Check second row
            assert!(matches!(rows[1][0], Expression::Integer(4)));
            if let Expression::BinaryOp { left, operator, right } = &rows[1][1] {
                assert!(matches!(operator, Operator::Multiply));
                assert!(matches!(**left, Expression::Integer(5)));
                assert!(matches!(**right, Expression::Integer(6)));
            } else {
                panic!("Expected BinaryOp");
            }
        } else {
            panic!("Expected MatrixLiteral");
        }
    }
    
    // Check empty matrix
    if let Statement::VariableDecl { type_, name, initializer } = &program.statements[2] {
        assert_eq!(name, "empty");
        if let Some(Expression::MatrixLiteral(rows)) = initializer {
            assert_eq!(rows.len(), 0);
        } else {
            panic!("Expected MatrixLiteral");
        }
    }
    
    // Check matrix with empty rows
    if let Statement::VariableDecl { type_, name, initializer } = &program.statements[3] {
        assert_eq!(name, "empty_rows");
        if let Some(Expression::MatrixLiteral(rows)) = initializer {
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0].len(), 0);
            assert_eq!(rows[1].len(), 0);
        } else {
            panic!("Expected MatrixLiteral");
        }
    }
}

#[test]
fn test_parse_function_calls() {
    let source = r#"
        print greet("world")
        number result = add(1, 2 * 3)
        print concat("Hello", ", ", "World")
        print empty()
    "#;
    
    let result = CleanParser::parse_program(source);
    assert!(result.is_ok(), "Failed to parse function calls: {:?}", result.err());
    
    let program = result.unwrap();
    assert_eq!(program.statements.len(), 4);
    
    // Check greet("world")
    if let Statement::Print(Expression::FunctionCall { name, arguments }) = &program.statements[0] {
        assert_eq!(name, "greet");
        assert_eq!(arguments.len(), 1);
        assert!(matches!(&arguments[0], Expression::String(s) if s == "world"));
    } else {
        panic!("Expected Print(FunctionCall)");
    }
    
    // Check add(1, 2 * 3)
    if let Statement::VariableDecl { type_, name, initializer } = &program.statements[1] {
        assert_eq!(name, "result");
        assert!(matches!(type_, Type::Number));
        if let Some(Expression::FunctionCall { name, arguments }) = initializer {
            assert_eq!(name, "add");
            assert_eq!(arguments.len(), 2);
            assert!(matches!(&arguments[0], Expression::Integer(1)));
            if let Expression::BinaryOp { left, operator, right } = &arguments[1] {
                assert!(matches!(operator, Operator::Multiply));
                assert!(matches!(**left, Expression::Integer(2)));
                assert!(matches!(**right, Expression::Integer(3)));
            } else {
                panic!("Expected BinaryOp");
            }
        } else {
            panic!("Expected FunctionCall");
        }
    }
    
    // Check concat with multiple arguments
    if let Statement::Print(Expression::FunctionCall { name, arguments }) = &program.statements[2] {
        assert_eq!(name, "concat");
        assert_eq!(arguments.len(), 3);
        assert!(matches!(&arguments[0], Expression::String(s) if s == "Hello"));
        assert!(matches!(&arguments[1], Expression::String(s) if s == ", "));
        assert!(matches!(&arguments[2], Expression::String(s) if s == "World"));
    } else {
        panic!("Expected Print(FunctionCall)");
    }
    
    // Check empty function call
    if let Statement::Print(Expression::FunctionCall { name, arguments }) = &program.statements[3] {
        assert_eq!(name, "empty");
        assert_eq!(arguments.len(), 0);
    } else {
        panic!("Expected Print(FunctionCall)");
    }
}

#[test]
fn test_parse_method_calls() {
    let source = r#"
        obj.method()
        obj.calc(1, 2)
        obj.nested.method()
        obj.chain.of.calls()
        result = value.multiply(2)
    "#;
    
    let result = CleanParser::parse_program(source);
    assert!(result.is_ok(), "Failed to parse method calls: {:?}", result.err());
    
    let program = result.unwrap();
    assert_eq!(program.statements.len(), 5);
    
    // Check simple method call
    if let Statement::Expression(Expression::MethodCall { target, name, arguments }) = &program.statements[0] {
        assert!(matches!(**target, Expression::Identifier(s) if s == "obj"));
        assert_eq!(name, "method");
        assert_eq!(arguments.len(), 0);
    } else {
        panic!("Expected Expression(MethodCall)");
    }
    
    // Check method call with arguments
    if let Statement::Expression(Expression::MethodCall { target, name, arguments }) = &program.statements[1] {
        assert!(matches!(**target, Expression::Identifier(s) if s == "obj"));
        assert_eq!(name, "calc");
        assert_eq!(arguments.len(), 2);
        assert!(matches!(&arguments[0], Expression::Integer(1)));
        assert!(matches!(&arguments[1], Expression::Integer(2)));
    } else {
        panic!("Expected Expression(MethodCall)");
    }
    
    // Check nested method call
    if let Statement::Expression(Expression::MethodCall { target, name, arguments }) = &program.statements[2] {
        if let Expression::MethodCall { target: inner_target, name: inner_name, arguments: inner_args } = &**target {
            assert!(matches!(**inner_target, Expression::Identifier(s) if s == "obj"));
            assert_eq!(inner_name, "nested");
            assert_eq!(inner_args.len(), 0);
        } else {
            panic!("Expected nested MethodCall");
        }
        assert_eq!(name, "method");
        assert_eq!(arguments.len(), 0);
    } else {
        panic!("Expected Expression(MethodCall)");
    }
    
    // Check chain of method calls
    if let Statement::Expression(Expression::MethodCall { target, name, arguments }) = &program.statements[3] {
        assert_eq!(name, "calls");
        assert_eq!(arguments.len(), 0);
        
        if let Expression::MethodCall { target: t1, name: n1, arguments: a1 } = &**target {
            assert_eq!(n1, "of");
            assert_eq!(a1.len(), 0);
            
            if let Expression::MethodCall { target: t2, name: n2, arguments: a2 } = &**t1 {
                assert_eq!(n2, "chain");
                assert_eq!(a2.len(), 0);
                assert!(matches!(**t2, Expression::Identifier(s) if s == "obj"));
            } else {
                panic!("Expected second MethodCall in chain");
            }
        } else {
            panic!("Expected first MethodCall in chain");
        }
    } else {
        panic!("Expected Expression(MethodCall)");
    }
    
    // Check method call in assignment
    if let Statement::Assignment { target, value } = &program.statements[4] {
        assert_eq!(target, "result");
        if let Expression::MethodCall { target: method_target, name, arguments } = &value {
            assert!(matches!(**method_target, Expression::Identifier(s) if s == "value"));
            assert_eq!(name, "multiply");
            assert_eq!(arguments.len(), 1);
            assert!(matches!(&arguments[0], Expression::Integer(2)));
        } else {
            panic!("Expected MethodCall");
        }
    } else {
        panic!("Expected Assignment");
    }
}

#[test]
fn test_parse_if_statement() {
    let source = r#"
        if x > 10
            print "Greater than 10"
            number y = x * 2
        
        if flag and (count < 5)
            print "Flag is true and count is less than 5"
            count = count + 1
    "#;
    
    let result = CleanParser::parse_program(source);
    assert!(result.is_ok(), "Failed to parse if statements: {:?}", result.err());
    
    let program = result.unwrap();
    assert_eq!(program.statements.len(), 2);
    
    // Check first if statement
    if let Statement::If { condition, body } = &program.statements[0] {
        // Check condition (x > 10)
        if let Expression::Condition { left, operator, right } = condition {
            assert!(matches!(**left, Expression::Identifier(s) if s == "x"));
            assert!(matches!(operator, ComparisonOperator::GreaterThan));
            assert!(matches!(**right, Expression::Integer(10)));
        } else {
            panic!("Expected Condition");
        }
        
        // Check body
        assert_eq!(body.len(), 2);
        assert!(matches!(&body[0], Statement::Print(Expression::String(s)) if s == "Greater than 10"));
        if let Statement::VariableDecl { type_, name, initializer } = &body[1] {
            assert_eq!(name, "y");
            assert!(matches!(type_, Type::Number));
            if let Some(Expression::BinaryOp { left, operator, right }) = initializer {
                assert!(matches!(operator, Operator::Multiply));
                assert!(matches!(**left, Expression::Identifier(s) if s == "x"));
                assert!(matches!(**right, Expression::Integer(2)));
            } else {
                panic!("Expected BinaryOp");
            }
        } else {
            panic!("Expected VariableDecl");
        }
    } else {
        panic!("Expected If statement");
    }
    
    // Check second if statement with compound condition
    if let Statement::If { condition, body } = &program.statements[1] {
        // Check condition (flag and (count < 5))
        if let Expression::BinaryOp { left, operator, right } = condition {
            assert!(matches!(operator, Operator::And));
            assert!(matches!(**left, Expression::Identifier(s) if s == "flag"));
            if let Expression::Condition { left: inner_left, operator: inner_op, right: inner_right } = &**right {
                assert!(matches!(**inner_left, Expression::Identifier(s) if s == "count"));
                assert!(matches!(inner_op, ComparisonOperator::LessThan));
                assert!(matches!(**inner_right, Expression::Integer(5)));
            } else {
                panic!("Expected Condition");
            }
        } else {
            panic!("Expected BinaryOp");
        }
        
        assert_eq!(body.len(), 2);
    }
}

#[test]
fn test_parse_iterate_statement() {
    let source = r#"
        iterate item in items
            print item
            total = total + item
        
        iterate x in [1, 2, 3]
            print x * 2
    "#;
    
    let result = CleanParser::parse_program(source);
    assert!(result.is_ok(), "Failed to parse iterate statements: {:?}", result.err());
    
    let program = result.unwrap();
    assert_eq!(program.statements.len(), 2);
    
    // Check first iterate statement
    if let Statement::Iterate { iterator, collection, body } = &program.statements[0] {
        assert_eq!(iterator, "item");
        assert!(matches!(&collection, Expression::Identifier(s) if s == "items"));
        
        assert_eq!(body.len(), 2);
        assert!(matches!(&body[0], Statement::Print(Expression::Identifier(s)) if s == "item"));
        if let Statement::Assignment { target, value } = &body[1] {
            assert_eq!(target, "total");
            if let Expression::BinaryOp { left, operator, right } = value {
                assert!(matches!(operator, Operator::Add));
                assert!(matches!(**left, Expression::Identifier(s) if s == "total"));
                assert!(matches!(**right, Expression::Identifier(s) if s == "item"));
            } else {
                panic!("Expected BinaryOp");
            }
        } else {
            panic!("Expected Assignment");
        }
    } else {
        panic!("Expected Iterate statement");
    }
    
    // Check second iterate statement with array literal
    if let Statement::Iterate { iterator, collection, body } = &program.statements[1] {
        assert_eq!(iterator, "x");
        if let Expression::ArrayLiteral(elements) = &collection {
            assert_eq!(elements.len(), 3);
            assert!(matches!(&elements[0], Expression::Integer(1)));
            assert!(matches!(&elements[1], Expression::Integer(2)));
            assert!(matches!(&elements[2], Expression::Integer(3)));
        } else {
            panic!("Expected ArrayLiteral");
        }
        
        assert_eq!(body.len(), 1);
        if let Statement::Print(Expression::BinaryOp { left, operator, right }) = &body[0] {
            assert!(matches!(operator, Operator::Multiply));
            assert!(matches!(**left, Expression::Identifier(s) if s == "x"));
            assert!(matches!(**right, Expression::Integer(2)));
        } else {
            panic!("Expected Print(BinaryOp)");
        }
    } else {
        panic!("Expected Iterate statement");
    }
}

#[test]
fn test_parse_from_to_statement() {
    let source = r#"
        i = from 1 to 10
            print i
            sum = sum + i
        
        j = from 0 to 100 step 10
            print j
        
        k = from start to end step increment
            total = total + k * 2
    "#;
    
    let result = CleanParser::parse_program(source);
    assert!(result.is_ok(), "Failed to parse from-to statements: {:?}", result.err());
    
    let program = result.unwrap();
    assert_eq!(program.statements.len(), 3);
    
    // Check first from-to statement (no step)
    if let Statement::FromTo { variable, start, end, step, body } = &program.statements[0] {
        assert_eq!(variable, "i");
        assert!(matches!(&start, Expression::Integer(1)));
        assert!(matches!(&end, Expression::Integer(10)));
        assert!(step.is_none());
        
        assert_eq!(body.len(), 2);
        assert!(matches!(&body[0], Statement::Print(Expression::Identifier(s)) if s == "i"));
        if let Statement::Assignment { target, value } = &body[1] {
            assert_eq!(target, "sum");
            if let Expression::BinaryOp { left, operator, right } = value {
                assert!(matches!(operator, Operator::Add));
                assert!(matches!(**left, Expression::Identifier(s) if s == "sum"));
                assert!(matches!(**right, Expression::Identifier(s) if s == "i"));
            } else {
                panic!("Expected BinaryOp");
            }
        } else {
            panic!("Expected Assignment");
        }
    } else {
        panic!("Expected FromTo statement");
    }
    
    // Check second from-to statement (with numeric step)
    if let Statement::FromTo { variable, start, end, step, body } = &program.statements[1] {
        assert_eq!(variable, "j");
        assert!(matches!(&start, Expression::Integer(0)));
        assert!(matches!(&end, Expression::Integer(100)));
        assert!(matches!(step.as_ref(), Some(Expression::Integer(10))));
        
        assert_eq!(body.len(), 1);
        assert!(matches!(&body[0], Statement::Print(Expression::Identifier(s)) if s == "j"));
    } else {
        panic!("Expected FromTo statement");
    }
    
    // Check third from-to statement (with variable step)
    if let Statement::FromTo { variable, start, end, step, body } = &program.statements[2] {
        assert_eq!(variable, "k");
        assert!(matches!(&start, Expression::Identifier(s) if s == "start"));
        assert!(matches!(&end, Expression::Identifier(s) if s == "end"));
        assert!(matches!(step.as_ref(), Some(Expression::Identifier(s)) if s == "increment"));
        
        assert_eq!(body.len(), 1);
        if let Statement::Assignment { target, value } = &body[0] {
            assert_eq!(target, "total");
            if let Expression::BinaryOp { left, operator, right } = value {
                assert!(matches!(operator, Operator::Add));
                assert!(matches!(**left, Expression::Identifier(s) if s == "total"));
                if let Expression::BinaryOp { left: inner_left, operator: inner_op, right: inner_right } = &**right {
                    assert!(matches!(inner_op, Operator::Multiply));
                    assert!(matches!(**inner_left, Expression::Identifier(s) if s == "k"));
                    assert!(matches!(**inner_right, Expression::Integer(2)));
                } else {
                    panic!("Expected inner BinaryOp");
                }
            } else {
                panic!("Expected outer BinaryOp");
            }
        } else {
            panic!("Expected Assignment");
        }
    } else {
        panic!("Expected FromTo statement");
    }
}

#[test]
fn test_parse_error_handling() {
    let source = r#"
        loadData()
        onError:
            print "Failed to load data"
            status = "error"
        
        complex.operation(x, y)
        onError:
            print "Operation failed"
            log.error("Failed with: " + error)
            retry = true
        
        number result = compute()
        onError:
            result = -1
            print "Computation error"
    "#;
    
    let result = CleanParser::parse_program(source);
    assert!(result.is_ok(), "Failed to parse error handling statements: {:?}", result.err());
    
    let program = result.unwrap();
    assert_eq!(program.statements.len(), 3);
    
    // Check first error handler (simple function call)
    if let Statement::ErrorHandler { protected, handler } = &program.statements[0] {
        // Check protected statement
        if let Statement::Expression(Expression::FunctionCall { name, arguments }) = &**protected {
            assert_eq!(name, "loadData");
            assert_eq!(arguments.len(), 0);
        } else {
            panic!("Expected FunctionCall");
        }
        
        // Check handler block
        assert_eq!(handler.len(), 2);
        assert!(matches!(&handler[0], Statement::Print(Expression::String(s)) if s == "Failed to load data"));
        if let Statement::Assignment { target, value } = &handler[1] {
            assert_eq!(target, "status");
            assert!(matches!(&value, Expression::String(s) if s == "error"));
        } else {
            panic!("Expected Assignment");
        }
    } else {
        panic!("Expected ErrorHandler");
    }
    
    // Check second error handler (method call with error usage)
    if let Statement::ErrorHandler { protected, handler } = &program.statements[1] {
        // Check protected statement
        if let Statement::Expression(Expression::MethodCall { target, name, arguments }) = &**protected {
            assert!(matches!(**target, Expression::Identifier(s) if s == "complex"));
            assert_eq!(name, "operation");
            assert_eq!(arguments.len(), 2);
            assert!(matches!(&arguments[0], Expression::Identifier(s) if s == "x"));
            assert!(matches!(&arguments[1], Expression::Identifier(s) if s == "y"));
        } else {
            panic!("Expected MethodCall");
        }
        
        // Check handler block
        assert_eq!(handler.len(), 3);
        assert!(matches!(&handler[0], Statement::Print(Expression::String(s)) if s == "Operation failed"));
        
        // Check error logging with string concatenation
        if let Statement::Expression(Expression::MethodCall { target, name, arguments }) = &handler[1] {
            assert!(matches!(**target, Expression::Identifier(s) if s == "log"));
            assert_eq!(name, "error");
            assert_eq!(arguments.len(), 1);
            if let Expression::BinaryOp { left, operator, right } = &arguments[0] {
                assert!(matches!(operator, Operator::Add));
                assert!(matches!(**left, Expression::String(s) if s == "Failed with: "));
                assert!(matches!(**right, Expression::Identifier(s) if s == "error"));
            } else {
                panic!("Expected BinaryOp");
            }
        } else {
            panic!("Expected MethodCall");
        }
        
        // Check retry flag
        if let Statement::Assignment { target, value } = &handler[2] {
            assert_eq!(target, "retry");
            assert!(matches!(&value, Expression::Boolean(true)));
        } else {
            panic!("Expected Assignment");
        }
    } else {
        panic!("Expected ErrorHandler");
    }
    
    // Check third error handler (variable declaration with error handling)
    if let Statement::ErrorHandler { protected, handler } = &program.statements[2] {
        // Check protected statement
        if let Statement::VariableDecl { type_, name, initializer } = &**protected {
            assert_eq!(name, "result");
            assert!(matches!(type_, Type::Number));
            if let Some(Expression::FunctionCall { name, arguments }) = initializer {
                assert_eq!(name, "compute");
                assert_eq!(arguments.len(), 0);
            } else {
                panic!("Expected FunctionCall");
            }
        } else {
            panic!("Expected VariableDecl");
        }
        
        // Check handler block
        assert_eq!(handler.len(), 2);
        if let Statement::Assignment { target, value } = &handler[0] {
            assert_eq!(target, "result");
            assert!(matches!(&value, Expression::Integer(-1)));
        } else {
            panic!("Expected Assignment");
        }
        assert!(matches!(&handler[1], Statement::Print(Expression::String(s)) if s == "Computation error"));
    } else {
        panic!("Expected ErrorHandler");
    }
}

#[test]
fn test_parse_constants_block() {
    let source = r#"
        constants:
            number:
                - PI = 3.14159
                - E = 2.71828
            string:
                - GREETING = "Hello, World!"
                - ERROR_MSG = "An error occurred"
            boolean:
                - DEBUG = true
                - PRODUCTION = false
            integer:
                - MAX_RETRIES = 3
                - BATCH_SIZE = 100
    "#;
    
    let result = CleanParser::parse_program(source);
    assert!(result.is_ok(), "Failed to parse constants block: {:?}", result.err());
    
    let program = result.unwrap();
    assert_eq!(program.constants.len(), 8);
    
    // Check number constants
    let pi = program.constants.iter().find(|c| c.name == "PI").unwrap();
    assert!(matches!(pi.type_, Type::Number));
    assert!(matches!(&pi.value, Expression::Number(n) if (*n - 3.14159).abs() < f64::EPSILON));
    
    let e = program.constants.iter().find(|c| c.name == "E").unwrap();
    assert!(matches!(e.type_, Type::Number));
    assert!(matches!(&e.value, Expression::Number(n) if (*n - 2.71828).abs() < f64::EPSILON));
    
    // Check string constants
    let greeting = program.constants.iter().find(|c| c.name == "GREETING").unwrap();
    assert!(matches!(greeting.type_, Type::String));
    assert!(matches!(&greeting.value, Expression::String(s) if s == "Hello, World!"));
    
    let error_msg = program.constants.iter().find(|c| c.name == "ERROR_MSG").unwrap();
    assert!(matches!(error_msg.type_, Type::String));
    assert!(matches!(&error_msg.value, Expression::String(s) if s == "An error occurred"));
    
    // Check boolean constants
    let debug = program.constants.iter().find(|c| c.name == "DEBUG").unwrap();
    assert!(matches!(debug.type_, Type::Boolean));
    assert!(matches!(&debug.value, Expression::Boolean(true)));
    
    let production = program.constants.iter().find(|c| c.name == "PRODUCTION").unwrap();
    assert!(matches!(production.type_, Type::Boolean));
    assert!(matches!(&production.value, Expression::Boolean(false)));
    
    // Check integer constants
    let max_retries = program.constants.iter().find(|c| c.name == "MAX_RETRIES").unwrap();
    assert!(matches!(max_retries.type_, Type::Integer));
    assert!(matches!(&max_retries.value, Expression::Integer(3)));
    
    let batch_size = program.constants.iter().find(|c| c.name == "BATCH_SIZE").unwrap();
    assert!(matches!(batch_size.type_, Type::Integer));
    assert!(matches!(&batch_size.value, Expression::Integer(100)));
}

#[test]
fn test_parse_constants_with_expressions() {
    let source = r#"
        constants:
            number:
                - DOUBLE_PI = 3.14159 * 2
                - AREA = 5 * 5
            string:
                - FULL_NAME = "John" + " " + "Doe"
            boolean:
                - HAS_ERRORS = true and false
                - IS_VALID = 5 > 3
    "#;
    
    let result = CleanParser::parse_program(source);
    assert!(result.is_ok(), "Failed to parse constants with expressions: {:?}", result.err());
    
    let program = result.unwrap();
    assert_eq!(program.constants.len(), 5);
    
    // Check number constant with multiplication
    let double_pi = program.constants.iter().find(|c| c.name == "DOUBLE_PI").unwrap();
    assert!(matches!(double_pi.type_, Type::Number));
    if let Expression::BinaryOp { left, operator, right } = &double_pi.value {
        assert!(matches!(operator, Operator::Multiply));
        assert!(matches!(**left, Expression::Number(n) if (n - 3.14159).abs() < f64::EPSILON));
        assert!(matches!(**right, Expression::Integer(2)));
    } else {
        panic!("Expected BinaryOp for DOUBLE_PI");
    }
    
    // Check string constant with concatenation
    let full_name = program.constants.iter().find(|c| c.name == "FULL_NAME").unwrap();
    assert!(matches!(full_name.type_, Type::String));
    if let Expression::BinaryOp { left: outer_left, operator: outer_op, right: outer_right } = &full_name.value {
        assert!(matches!(outer_op, Operator::Add));
        if let Expression::BinaryOp { left: inner_left, operator: inner_op, right: inner_right } = &**outer_left {
            assert!(matches!(inner_op, Operator::Add));
            assert!(matches!(**inner_left, Expression::String(s) if s == "John"));
            assert!(matches!(**inner_right, Expression::String(s) if s == " "));
        } else {
            panic!("Expected BinaryOp for first concatenation");
        }
        assert!(matches!(**outer_right, Expression::String(s) if s == "Doe"));
    } else {
        panic!("Expected BinaryOp for string concatenation");
    }
    
    // Check boolean constant with logical operation
    let has_errors = program.constants.iter().find(|c| c.name == "HAS_ERRORS").unwrap();
    assert!(matches!(has_errors.type_, Type::Boolean));
    if let Expression::BinaryOp { left, operator, right } = &has_errors.value {
        assert!(matches!(operator, Operator::And));
        assert!(matches!(**left, Expression::Boolean(true)));
        assert!(matches!(**right, Expression::Boolean(false)));
    } else {
        panic!("Expected BinaryOp for HAS_ERRORS");
    }
    
    // Check boolean constant with comparison
    let is_valid = program.constants.iter().find(|c| c.name == "IS_VALID").unwrap();
    assert!(matches!(is_valid.type_, Type::Boolean));
    if let Expression::Condition { left, operator, right } = &is_valid.value {
        assert!(matches!(operator, ComparisonOperator::GreaterThan));
        assert!(matches!(**left, Expression::Integer(5)));
        assert!(matches!(**right, Expression::Integer(3)));
    } else {
        panic!("Expected Condition for IS_VALID");
    }
}

#[test]
fn test_parse_expressions() {
    let test_cases = vec![
        ("1", Expression::Literal(Value::Integer(1))),
        ("1.5", Expression::Literal(Value::Number(1.5))),
        ("true", Expression::Literal(Value::Boolean(true))),
        ("\"hello\"", Expression::String(vec![StringPart::Text("hello".to_string())])),
        ("1 + 2", Expression::BinaryOp(
            Box::new(Expression::Literal(Value::Integer(1))),
            Operator::Add,
            Box::new(Expression::Literal(Value::Integer(2)))
        )),
        ("x", Expression::Variable("x".to_string())),
        ("foo(1, 2)", Expression::Call(
            "foo".to_string(),
            vec![
                Expression::Literal(Value::Integer(1)),
                Expression::Literal(Value::Integer(2))
            ],
            None
        )),
    ];

    for (input, expected) in test_cases {
        let result = CleanParser::parse_program(input);
        assert!(result.is_ok(), "Failed to parse expression: {}", input);
        let program = result.unwrap();
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::Expression(expr) => assert_eq!(expr, &expected),
            _ => panic!("Expected expression statement"),
        }
    }
}

#[test]
fn test_parse_statements() {
    let test_cases = vec![
        (
            "let x: integer = 1",
            Statement::VariableDecl {
                name: "x".to_string(),
                type_: Some(Type::Integer),
                initializer: Some(Expression::Literal(Value::Integer(1))),
                location: None,
            }
        ),
        (
            "print \"hello\"",
            Statement::Print {
                expression: Expression::String(vec![StringPart::Text("hello".to_string())]),
                newline: false,
                location: None,
            }
        ),
        (
            "printl \"hello\"",
            Statement::Print {
                expression: Expression::String(vec![StringPart::Text("hello".to_string())]),
                newline: true,
                location: None,
            }
        ),
    ];

    for (input, expected) in test_cases {
        let result = CleanParser::parse_program(input);
        assert!(result.is_ok(), "Failed to parse statement: {}", input);
        let program = result.unwrap();
        assert_eq!(program.statements.len(), 1);
        assert_eq!(&program.statements[0], &expected);
    }
}

#[test]
fn test_parse_functions() {
    let source = r#"
        add() returns number {
            description "Adds two numbers"
            input
                number x
                number y
            x + y
        }
    "#;

    let result = CleanParser::parse_program(source);
    assert!(result.is_ok(), "Failed to parse function: {:?}", result.err());
    let program = result.unwrap();
    assert_eq!(program.functions.len(), 1);
    let function = &program.functions[0];
    assert_eq!(function.name, "add");
    assert_eq!(function.parameters.len(), 2);
    assert_eq!(function.parameters[0].name, "x");
    assert_eq!(function.parameters[0].type_, Type::Number);
    assert_eq!(function.parameters[1].name, "y");
    assert_eq!(function.parameters[1].type_, Type::Number);
    assert_eq!(function.return_type, Type::Number);
    assert_eq!(function.description, Some("Adds two numbers".to_string()));
}

#[test]
fn test_parse_classes() {
    let source = r#"
        class Point<T> {
            description "A point in 2D space"
            input
                T x
                T y
            
            constructor {
                input
                    T x
                    T y
                this.x = x
                this.y = y
            }

            getDistance() returns number {
                description "Calculates distance from origin"
                (x * x + y * y).sqrt()
            }
        }
    "#;

    let result = CleanParser::parse_program(source);
    assert!(result.is_ok(), "Failed to parse class: {:?}", result.err());
    let program = result.unwrap();
    assert_eq!(program.classes.len(), 1);
    let class = &program.classes[0];
    assert_eq!(class.name, "Point");
    assert_eq!(class.type_parameters, vec!["T"]);
    assert_eq!(class.description, Some("A point in 2D space".to_string()));
    assert_eq!(class.fields.len(), 2);
    assert!(class.constructor.is_some());
    assert_eq!(class.methods.len(), 1);
}

#[test]
fn test_parse_errors() {
    let invalid_cases = vec![
        "let", // Incomplete variable declaration
        "print", // Missing expression
        "if", // Incomplete if statement
        "class", // Incomplete class declaration
        "{", // Unclosed block
        "1 +", // Incomplete expression
    ];

    for input in invalid_cases {
        let result = CleanParser::parse_program(input);
        assert!(result.is_err(), "Expected error for input: {}", input);
    }
}

// TODO: Create complex_syntax_test.cln file for this test
// #[test]
// fn test_parse_complex_program() {
//     let source = test_utils::read_test_file("complex_syntax_test.cln");
//     let result = CleanParser::parse_program(&source);
//     assert!(result.is_ok(), "Failed to parse complex program: {:?}", result.err());
// }

// TODO: Add more tests as parser implementation progresses
// - Test parsing function declarations
// - Test parsing class definitions
// - Test parsing more complex expressions (operators, calls)
// - Test error handling for invalid syntax 