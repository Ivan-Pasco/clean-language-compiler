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
    number x = 42
    string name = "Test"
    boolean flag = true
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
    if x is true
      print "It's true!"
      
    a = from 1 to 10
    
    iterate colors in c
      print c
    "#;
    
    let result = CleanParser::parse(Rule::program, input);
    assert!(result.is_ok());
}

#[test]
fn test_function_declaration() {
    let input = r#"
    functions:
      add() returns number
        input:
          number:
            - a = 0
            - b = 0
        
        number result
        result = a + b
        return result
    "#;
    
    let result = CleanParser::parse(Rule::program, input);
    assert!(result.is_ok());
}

#[test]
fn test_class_definition() {
    let input = r#"
    class Car
      description: Represents a car
      public
        string color = "red"
        unsigned wheels = 4
      private
        static integer serialNumber = 45

      constructor()
        input:
          string c = ""
          unsigned w = 0
        color = c
        wheels = w
    "#;
    
    let result = CleanParser::parse(Rule::program, input);
    assert!(result.is_ok());
}

#[test]
fn test_object_creation() {
    let input = r#"
    object car = new Car("blue", 4)
    print car.color

    object Car:
      string brand = "Audi"
      number motor = 1
    "#;
    
    let result = CleanParser::parse(Rule::program, input);
    assert!(result.is_ok());
}

#[test]
fn test_error_handling() {
    let input = r#"
    print "Starting"
    onError:
      print "Could not load data."
    "#;
    
    let result = CleanParser::parse(Rule::program, input);
    assert!(result.is_ok());
}

#[test]
fn test_testing_syntax() {
    let input = r#"
    test:
      check 2 + 2 is 4
      check "Clean".length is 5
    "#;
    
    let result = CleanParser::parse(Rule::program, input);
    assert!(result.is_ok());
} 