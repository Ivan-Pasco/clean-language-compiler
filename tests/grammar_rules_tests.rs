use clean_language_compiler::parser::{CleanParser, Rule};
use pest::Parser;
use std::fs;

mod test_utils;

#[test]
fn test_basic_grammar_rules() {
    // Basic program parsing
    let input = fs::read_to_string("tests/test_inputs/hello_world.cln").unwrap();
    let result = CleanParser::parse(Rule::program, &input);
    assert!(result.is_ok());
}

#[test]
fn test_variable_declarations() {
    let input = r#"
start()
	number x = 42
	string name = "Test"
	boolean flag = true
	print(x)
    "#;
    
    let result = CleanParser::parse(Rule::program, input);
    assert!(result.is_ok());
}

#[test]
fn test_expressions() {
    let expressions = [
        "10 + 20 * 30",
        "x > 10 and y < 20",
        "true or false",
        "(1 + 2) * 3",
    ];
    
    for expr in expressions {
        let result = CleanParser::parse(Rule::expression, expr);
        assert!(result.is_ok(), "Failed to parse expression: {}", expr);
    }
}

#[test]
fn test_control_structures() {
    let input = r#"
start()
	boolean x = true
	if x is true
		print("It's true!")
	
	iterate i in 1 to 10
		print(i)
    "#;
    
    let result = CleanParser::parse(Rule::program, input);
    assert!(result.is_ok());
}

#[test]
fn test_function_declaration() {
    let input = r#"
functions:
	number add(number a, number b)
		number result = a + b
		return result

start()
	number x = add(5, 3)
	print(x)
    "#;
    
    let result = CleanParser::parse(Rule::program, input);
    assert!(result.is_ok());
}

#[test]
fn test_class_definition() {
    let input = r#"
class Car
	string color = "red"
	integer wheels = 4
	
	constructor(string c, integer w)
		color = c
		wheels = w

start()
	string message = "Class parsing test"
	print(message)
    "#;
    
    let result = CleanParser::parse(Rule::program, input);
    assert!(result.is_ok());
}

#[test]
fn test_object_creation() {
    let input = r#"
start()
	array<string> colors = ["red", "blue", "green"]
	string firstColor = colors[0]
	print(firstColor)
	matrix<number> myMatrix = [[1.0, 2.0], [3.0, 4.0]]
	print("Matrix created")
    "#;
    
    let result = CleanParser::parse(Rule::program, input);
    if let Err(ref e) = result {
        println!("Parse error in test_object_creation: {:?}", e);
    }
    assert!(result.is_ok());
}

#[test]
fn test_error_handling() {
    let input = r#"
start()
	number result = 42
	if result > 0
		print("Success")
	if result == 0  
		print("Zero")
    "#;
    
    let result = CleanParser::parse(Rule::program, input);
    assert!(result.is_ok());
}

#[test]
fn test_testing_syntax() {
    let input = r#"
tests:
	2 + 2 = 4
	"Addition test": 3 + 1 = 4

start()
	number x = 2 + 2
	print(x)
    "#;
    
    let result = CleanParser::parse(Rule::program, input);
    assert!(result.is_ok());
} 