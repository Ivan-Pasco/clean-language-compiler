use crate::parser::CleanParser;
use crate::error::CompilerError;
use crate::ast::{Type, Value, Expression, Statement, Program};

#[test]
fn test_parser_complex_input() -> Result<(), CompilerError> {
    // Complex program with nested expressions, multiple function declarations,
    // matrix operations, string interpolation, and complex control flow
    let source = r#"
    /**
     * A complex matrix calculation function
     */
    function matrix_multiply<T>(matrix1: T[][], matrix2: T[][]) returns T[][] {
        let result: T[][] = []
        
        // Validate matrix dimensions
        if matrix1[0].length != matrix2.length {
            return result  // Empty result for invalid dimensions
        }
        
        // Initialize result matrix
        from 0 to matrix1.length - 1 step 1 {
            let row: T[] = []
            from 0 to matrix2[0].length - 1 step 1 {
                row.push(0)
            }
            result.push(row)
        }
        
        // Perform matrix multiplication
        from 0 to matrix1.length - 1 step 1 {
            from 0 to matrix2[0].length - 1 step 1 {
                from 0 to matrix1[0].length - 1 step 1 {
                    result[i][j] = result[i][j] + (matrix1[i][k] * matrix2[k][j])
                }
            }
        }
        
        return result
    }
    
    /**
     * Calculate the determinant of a 2x2 matrix
     */
    function calculate_determinant(matrix: float[][]) returns float {
        if matrix.length != 2 || matrix[0].length != 2 {
            return 0.0
        }
        
        return (matrix[0][0] * matrix[1][1]) - (matrix[0][1] * matrix[1][0])
    }
    
    /**
     * Complex string processing with nested interpolation
     */
    function process_text(template: string, values: string[]) returns string {
        let result = template
        
        from 0 to values.length - 1 step 1 {
            let placeholder = "${" + i + "}"
            result = result.replace(placeholder, values[i])
        }
        
        return result
    }
    
    /**
     * Entry point with complex logic and nested expressions
     */
    start() {
        // Complex variable declarations with nested expressions
        let matrix1 = [[1.0, 2.0], [3.0, 4.0]]
        let matrix2 = [[5.0, 6.0], [7.0, 8.0]]
        
        // Matrix operations
        let product = matrix_multiply<float>(matrix1, matrix2)
        let det1 = calculate_determinant(matrix1)
        let det2 = calculate_determinant(matrix2)
        
        // Complex string interpolation
        let message = "Matrix 1 determinant: ${det1}, Matrix 2 determinant: ${det2}"
        
        // Nested conditionals
        if det1 > 0 {
            if det2 > 0 {
                printl "Both matrices have positive determinants"
            } else {
                printl "Only matrix 1 has a positive determinant"
            }
        } else {
            if det2 > 0 {
                printl "Only matrix 2 has a positive determinant"
            } else {
                printl "Both matrices have non-positive determinants"
            }
        }
        
        // Advanced array operations
        let values = ["Alice", "Bob", "Charlie"]
        let template = "Person 0: ${0}, Person 1: ${1}, Person 2: ${2}"
        let formatted = process_text(template, values)
        
        printl formatted
        
        // Complex conditional with compound expressions
        if (det1 * det2) > (matrix1[0][0] * matrix2[0][0]) && formatted.length > 20 {
            printl "Complex condition satisfied"
        }
        
        // Return complex expression
        return det1 * det2 + product[0][0]
    }
    "#;
    
    // Parse the program
    let program = CleanParser::parse_program(source)?;
    
    // Verify program structure
    assert_eq!(program.functions.len(), 4); // Should have 4 functions
    
    // Verify function names
    let function_names: Vec<String> = program.functions.iter()
        .map(|f| f.name.clone())
        .collect();
    assert!(function_names.contains(&"matrix_multiply".to_string()));
    assert!(function_names.contains(&"calculate_determinant".to_string()));
    assert!(function_names.contains(&"process_text".to_string()));
    assert!(function_names.contains(&"start".to_string()));
    
    // Find the matrix_multiply function and verify its type parameters
    let matrix_multiply = program.functions.iter()
        .find(|f| f.name == "matrix_multiply")
        .expect("matrix_multiply function not found");
    assert_eq!(matrix_multiply.type_parameters.len(), 1);
    assert_eq!(matrix_multiply.type_parameters[0], "T");
    
    // Find the start function and verify its body
    let start_function = program.functions.iter()
        .find(|f| f.name == "start")
        .expect("start function not found");
    
    // Start function should have multiple statements
    assert!(start_function.body.len() > 5);
    
    // Test parsing more complex scenarios with deeply nested expressions
    let complex_expression = r#"
    start() {
        let result = ((((5 + 3) * 2) - (10 / 2)) % 3) + (7 * (4 - 2))
        let complex_array = [[[1, 2], [3, 4]], [[5, 6], [7, 8]]]
        let nested_condition = if x > 5 { if y < 10 { 1 } else { 2 } } else { 3 }
        return result
    }
    "#;
    
    let complex_program = CleanParser::parse_program(complex_expression)?;
    assert_eq!(complex_program.functions.len(), 1);
    assert_eq!(complex_program.functions[0].name, "start");
    
    Ok(())
}

#[test]
fn test_parser_error_recovery() -> Result<(), ()> {
    // Test with syntax errors to see how parser handles them
    let invalid_source = r#"
    function missing_brace(x: integer) returns integer {
        let y = x + 5
        return y
    // Missing closing brace
    
    start() {
        let result = missing_brace(10)
        printl result
    }
    "#;
    
    match CleanParser::parse_program(invalid_source) {
        Err(CompilerError::Parse { .. }) => {
            // This is expected - we should get a parse error
            Ok(())
        },
        _ => {
            // We didn't get the expected error
            Err(())
        }
    }
}

#[test]
fn test_parser_complex_types() -> Result<(), CompilerError> {
    // Test parsing of complex type expressions
    let source = r#"
    function complex_types() {
        let simple_int: integer = 5
        let simple_float: float = 3.14
        let simple_string: string = "Hello"
        let simple_bool: boolean = true
        
        // Array types
        let int_array: integer[] = [1, 2, 3]
        let string_array: string[] = ["a", "b", "c"]
        
        // Multi-dimensional arrays
        let matrix: integer[][] = [[1, 2], [3, 4]]
        let cube: integer[][][] = [[[1, 2], [3, 4]], [[5, 6], [7, 8]]]
        
        // Function types
        let func_ref: function(integer, integer) returns integer = add
        
        // Optional types
        let optional_int: integer? = 5
        let null_value: string? = null
        
        // Map types
        let string_map: map<string, integer> = {"a": 1, "b": 2}
        
        // Nested complex types
        let complex: map<string, integer[]>[] = [{"a": [1, 2]}, {"b": [3, 4]}]
        
        return complex
    }
    "#;
    
    let program = CleanParser::parse_program(source)?;
    assert_eq!(program.functions.len(), 1);
    assert_eq!(program.functions[0].name, "complex_types");
    
    // Verify the function has multiple variable declarations
    let var_decls = program.functions[0].body.iter()
        .filter(|stmt| matches!(stmt, Statement::VariableDecl { .. }))
        .count();
    assert!(var_decls > 10);
    
    Ok(())
}

#[test]
fn test_parser_error_handling_robustness() -> Result<(), ()> {
    // Test with various complex syntax errors to validate error reporting
    
    // 1. Mismatched parentheses in a complex expression
    let mismatched_parens = r#"
    function test() {
        let x = (5 + 3) * (2 - (4 / 2)
        return x
    }
    "#;
    
    match CleanParser::parse_program(mismatched_parens) {
        Err(CompilerError::Parse { .. }) => {
            // Expected error for mismatched parentheses
        },
        _ => return Err(()),
    }
    
    // 2. Invalid type annotations with nested generics
    let invalid_generics = r#"
    function test<T, U<V>> {  // Invalid nested generic parameter
        return null
    }
    "#;
    
    match CleanParser::parse_program(invalid_generics) {
        Err(CompilerError::Parse { .. }) => {
            // Expected error for invalid generic syntax
        },
        _ => return Err(()),
    }
    
    // 3. Unclosed string literal with interpolation
    let unclosed_string = r#"
    function test() {
        let s = "This string has ${interpolation} but no closing quote
        return s
    }
    "#;
    
    match CleanParser::parse_program(unclosed_string) {
        Err(CompilerError::Parse { .. }) => {
            // Expected error for unclosed string
        },
        _ => return Err(()),
    }
    
    // 4. Invalid array access syntax
    let invalid_array_access = r#"
    function test() {
        let arr = [1, 2, 3]
        let x = arr[0, 1]  // Invalid syntax - should be arr[0]
        return x
    }
    "#;
    
    match CleanParser::parse_program(invalid_array_access) {
        Err(CompilerError::Parse { .. }) => {
            // Expected error for invalid array access
        },
        _ => return Err(()),
    }
    
    // 5. Missing comma in function parameters
    let missing_comma = r#"
    function test(x: integer y: integer) {  // Missing comma between parameters
        return x + y
    }
    "#;
    
    match CleanParser::parse_program(missing_comma) {
        Err(CompilerError::Parse { .. }) => {
            // Expected error for missing comma
        },
        _ => return Err(()),
    }
    
    // All errors were properly handled
    Ok(())
} 