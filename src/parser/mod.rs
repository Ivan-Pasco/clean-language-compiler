use pest_derive::Parser;
use crate::ast::{Program, Expression};
use crate::error::CompilerError;

// Define the CleanParser with the proper grammar path
#[derive(Parser)]
#[grammar = "src/parser/grammar.pest"]
pub struct CleanParser;

// Define a parser-specific SourceLocation that uses start/end
#[derive(Debug, Clone, PartialEq)]
pub struct SourceLocation {
    pub start: usize,
    pub end: usize,
}

// Define a function to convert parser SourceLocation to AST SourceLocation
pub fn convert_to_ast_location(loc: &SourceLocation) -> crate::ast::SourceLocation {
    // Convert the parser location to AST location format
    // In a real implementation, we would need to calculate the line/column based on the text
    // For now we're using a simplified approach that provides at least some useful information
    crate::ast::SourceLocation {
        line: loc.start,   // Using start position as a fallback for line number
        column: loc.end - loc.start,  // Using length as a fallback for column
        file: String::new()  // We don't have file information in the parser location
    }
}

// Define submodules
mod parser_impl;
mod expression_parser;
mod statement_parser;
mod type_parser;
mod class_parser;
mod program_parser;
mod grammar;

// Re-export just what's needed
pub use parser_impl::{parse, parse_start_function, get_location, parse_function, parse_with_file, ParseContext, ErrorRecoveringParser};
pub use expression_parser::{parse_expression, parse_primary, parse_string, parse_array_literal, parse_matrix_literal, parse_function_call};
pub use statement_parser::parse_statement;
pub use type_parser::parse_type;
pub use class_parser::parse_class;
pub use program_parser::parse_program_ast;

impl CleanParser {
    pub fn parse_program(source: &str) -> Result<Program, CompilerError> {
        parser_impl::parse(source)
    }

    /// Parse a program with file path information for better error reporting
    pub fn parse_program_with_file(source: &str, file_path: &str) -> Result<Program, CompilerError> {
        parser_impl::parse_with_file(source, file_path)
    }

    /// Parse a program with error recovery - returns multiple errors if found
    pub fn parse_program_with_recovery(source: &str, file_path: &str) -> Result<Program, Vec<CompilerError>> {
        let mut parser = ErrorRecoveringParser::new(source, file_path);
        parser.parse_with_recovery(source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::*;

    #[test]
    fn test_minimal_parsing() {
        // Test just the function declaration first
        let source = r#"function start()"#;
        let result = CleanParser::parse_program(source);
        println!("Function only result: {:?}", result);
        
        // Test function with empty body
        let source2 = r#"
function start()
	return
        "#;
        let result2 = CleanParser::parse_program(source2);
        println!("Function with return result: {:?}", result2);
        
        // Test simple variable declaration 
        let source3 = r#"
function start()
	integer x = 5
        "#;
        let result3 = CleanParser::parse_program(source3);
        println!("Function with variable result: {:?}", result3);
    }

    #[test]
    fn test_basic_parsing() {
        let source = r#"
function start()
	integer x = 5
	print(x)
        "#;
        
        let result = CleanParser::parse_program(source);
        assert!(result.is_ok(), "Basic parsing should succeed: {:?}", result.err());
    }

    #[test]
    fn test_parse_error_reporting() {
        let source = r#"
function start()
	integer x = 5 +
        "#;
        
        let result = CleanParser::parse_program(source);
        assert!(result.is_err(), "Invalid syntax should produce an error");
        
        if let Err(error) = result {
            println!("Error: {}", error);
            // The error should contain useful information
            assert!(error.to_string().contains("Syntax error"));
        }
    }

    #[test]
    fn test_nested_expression_parsing() {
        let source = r#"
function start()
	integer x = (1 + 2) * (3 - 4)
	print(x)
        "#;
        
        let result = CleanParser::parse_program(source);
        assert!(result.is_ok(), "Nested expressions should parse correctly: {:?}", result.err());
    }

    #[test]
    fn test_apply_blocks() {
        let source = r#"
function start()
	println:
		"Hello"
		"World"
	
	integer:
		count = 0
		maxSize = 100
        "#;
        
        let result = CleanParser::parse_program(source);
        assert!(result.is_ok(), "Apply blocks should parse correctly: {:?}", result.err());
    }

    #[test]
    fn test_function_syntaxes() {
        let source = r#"
function integer add()
	input
		integer a
		integer b
	return a + b

function integer multiply()
	description "Multiplies two integers"
	input
		integer a
		integer b
	return a * b

functions:
	integer square()
		input integer x
		return x * x

function start()
	integer result = add()
	print(result)
        "#;
        
        let result = CleanParser::parse_program(source);
        assert!(result.is_ok(), "All function syntaxes should parse correctly: {:?}", result.err());
    }

    #[test]
    fn test_string_interpolation() {
        let source = r#"
function start()
	string name = "World"
	string greeting = "Hello, {name}!"
	println(greeting)
        "#;
        
        let result = CleanParser::parse_program(source);
        assert!(result.is_ok(), "String interpolation should parse correctly: {:?}", result.err());
    }

    #[test]
    fn test_on_error_syntax() {
        let source = r#"
function start()
	integer result = divide(10, 0) onError 0
	print(result)
        "#;
        
        let result = CleanParser::parse_program(source);
        assert!(result.is_ok(), "onError syntax should parse correctly: {:?}", result.err());
    }

    #[test]
    fn test_inheritance_with_is() {
        let source = r#"
class Shape
	string color
	
	constructor(color)
	
class Circle is Shape
	float radius
	
	constructor(color, radius)
		super(color)
	
function start()
	circle = Circle("red", 5.0)
        "#;
        
        let result = CleanParser::parse_program(source);
        assert!(result.is_ok(), "Class inheritance with 'is' should parse correctly: {:?}", result.err());
    }

    #[test]
    fn test_complex_error_cases() {
        let test_cases = vec![
            // Missing indentation after function
            r#"function start()
integer x = 5"#,
            // Invalid apply block
            r#"function start()
 invalid_block:"#,
            // Missing expression after onError
            r#"function start()
	integer x = divide(10, 0) onError"#,
        ];

        for (i, source) in test_cases.iter().enumerate() {
            let result = CleanParser::parse_program(source);
            assert!(result.is_err(), "Test case {} should fail: {}", i, source);
            
            if let Err(error) = result {
                println!("Test case {}: {}", i, error);
                // Each error should be informative
                assert!(!error.to_string().is_empty());
            }
        }
    }

    #[test]
    fn test_file_path_in_errors() {
        let source = r#"invalid syntax here"#;
        let result = CleanParser::parse_program(source);
        
        if let Err(error) = result {
            println!("Error without file path: {}", error);
        }
    }

    #[test]
    fn test_file_path_in_enhanced_errors() {
        let source = r#"
function start()
	integer x = 5 +
        "#;
        let file_path = "test_file.cln";
        let result = CleanParser::parse_program_with_file(source, file_path);
        
        assert!(result.is_err(), "Invalid syntax should produce an error");
        
        if let Err(error) = result {
            println!("Enhanced error with file path: {}", error);
            assert!(error.to_string().contains(file_path));
        }
    }

    #[test]
    fn test_type_first_declarations() {
        let source = r#"
function start()
	integer count = 0
	float temperature = 23.5
	boolean isActive = true
	string name = "Alice"
        "#;
        
        let result = CleanParser::parse_program(source);
        assert!(result.is_ok(), "Type-first declarations should parse correctly: {:?}", result.err());
    }

    #[test]
    fn test_debug_sized_in_function() {
        // Test regular integer in function (should work)
        let source1 = r#"
function start()
	integer x = 5
        "#;
        let result1 = CleanParser::parse_program(source1);
        println!("Regular integer in function: {:?}", result1);
        
        // Test sized integer in function (currently failing)
        let source2 = r#"
function start()
	integer:8 smallNum = 100
        "#;
        let result2 = CleanParser::parse_program(source2);
        println!("Sized integer in function: {:?}", result2);
        
        // Let's also test if the issue is with the variable name or assignment
        let source3 = r#"
function start()
	integer:8 x
        "#;
        let result3 = CleanParser::parse_program(source3);
        println!("Sized integer without assignment: {:?}", result3);
    }

    #[test]
    fn test_direct_grammar_parsing() {
        use pest::Parser;
        
        // Test if the grammar can parse sized_type directly
        let result1 = CleanParser::parse(Rule::sized_type, "integer:8");
        println!("Direct sized_type parsing: {:?}", result1);
        
        // Test if the grammar can parse type_ with sized_type
        let result2 = CleanParser::parse(Rule::type_, "integer:8");
        println!("Direct type_ parsing: {:?}", result2);
        
        // Test if the grammar can parse variable_decl
        let result3 = CleanParser::parse(Rule::variable_decl, "integer:8 x");
        println!("Direct variable_decl parsing: {:?}", result3);
    }

    #[test]
    fn test_debug_sized_types() {
        // Test just the type parsing
        let source1 = r#"
function start()
	integer x = 5
        "#;
        let result1 = CleanParser::parse_program(source1);
        println!("Regular integer result: {:?}", result1);
        
        // Test sized type
        let source2 = r#"
function start()
	integer:8 x = 5
        "#;
        let result2 = CleanParser::parse_program(source2);
        println!("Sized integer result: {:?}", result2);
    }

    #[test]
    fn test_advanced_types() {
        let source = r#"
function start()
	integer:8 smallNum = 100
	integer:64 bigNum = 999999999999
	float:32 preciseFloat = 3.14159
	boolean flag = true
        "#;
        
        let result = CleanParser::parse_program(source);
        assert!(result.is_ok(), "Advanced sized types should parse correctly: {:?}", result.err());
    }

    #[test]
    fn test_matrix_operations() {
        let source = r#"
function start()
	Matrix<float> m1 = [[1.0, 2.0], [3.0, 4.0]]
	Matrix<float> m2 = [[5.0, 6.0], [7.0, 8.0]]
	Matrix<float> result = m1 + m2
	Matrix<float> transposed = m1.transpose()
        "#;
        
        let result = CleanParser::parse_program(source);
        assert!(result.is_ok(), "Matrix operations should parse correctly: {:?}", result.err());
    }

    #[test]
    fn test_print_parsing() {
        // Test print with literal
        let source1 = r#"
function start()
	print(5)
        "#;
        let result1 = CleanParser::parse_program(source1);
        println!("Print literal result: {:?}", result1);
        
        // Test print with identifier
        let source2 = r#"
function start()
	print(x)
        "#;
        let result2 = CleanParser::parse_program(source2);
        println!("Print identifier result: {:?}", result2);
        
        // Test variable and print together
        let source3 = r#"
function start()
	integer x = 5
	print(x)
        "#;
        let result3 = CleanParser::parse_program(source3);
        println!("Variable + print result: {:?}", result3);
    }

    #[test]
    fn test_pest_tokens_debug() {
        use pest::Parser;
        
        // Let's see what tokens PEST generates for the failing case
        let source = r#"
function start()
	integer:8 smallNum = 100
        "#;
        
        // Parse the full program and see what we get
        let pairs = CleanParser::parse(Rule::program, source.trim()).unwrap();
        
        println!("=== PEST Token Analysis ===");
        for pair in pairs {
            print_pair(&pair, 0);
        }
    }
    
    // Helper function to recursively print PEST pairs with indentation
    fn print_pair(pair: &pest::iterators::Pair<Rule>, depth: usize) {
        let indent = "  ".repeat(depth);
        println!("{}Rule::{:?} -> \"{}\"", indent, pair.as_rule(), pair.as_str());
        
        for inner_pair in pair.clone().into_inner() {
            print_pair(&inner_pair, depth + 1);
        }
    }
}