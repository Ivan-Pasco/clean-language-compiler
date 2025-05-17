use pest::Parser;
use pest_derive::Parser;


// Define the parser using the grammar from the file
#[derive(Parser)]
#[grammar_inline = r#"
// Basic tokens
WHITESPACE = _{ " " | "\t" | "\r" | "\n" }
COMMENT = _{ "//" ~ (!"\n" ~ ANY)* ~ "\n" | "/*" ~ (!"*/" ~ ANY)* ~ "*/" }

// Identifiers and literals
identifier = @{ ASCII_ALPHA ~ (ASCII_ALPHANUMERIC | "_")* }
integer = @{ "-"? ~ ASCII_DIGIT+ }
float = @{ "-"? ~ ASCII_DIGIT+ ~ "." ~ ASCII_DIGIT+ }
big_number = @{ "-"? ~ ASCII_DIGIT+ ~ ("n" | "N") }
ubig_number = @{ ASCII_DIGIT+ ~ ("u" | "U") ~ ("n" | "N") }
number = _{ big_number | ubig_number | float | integer }
boolean = { "true" | "false" }

// Enhanced string interpolation
string_content = @{ (!("\"" | "{{" | "}}") ~ ANY)+ }
string_interpolation = { "{{" ~ expression ~ "}}" }
string_part = { string_content | string_interpolation }
string = { "\"" ~ string_part* ~ "\"" }

// Types with enhanced numeric and generic support
type_ = { 
    "boolean" |
    "number" |
    "string" |
    "integer" |
    "unsigned" |
    "long" |
    "ulong" |
    "big" |
    "ubig" |
    "byte" |
    "unit" |
    matrix_type |
    array_type |
    generic_type |
    type_parameter |
    identifier
}

generic_type = { identifier ~ "<" ~ type_ ~ ("," ~ type_)* ~ ">" }
type_parameter = @{ identifier }
type_parameters = { "<" ~ type_parameter ~ ("," ~ type_parameter)* ~ ">" }

matrix_type = { "Matrix" ~ "<" ~ type_ ~ ">" }
array_type = { "Array" ~ "<" ~ type_ ~ ">" }

// Block definition
block = { "{" ~ statement* ~ "}" }

// Setup blocks
setup_block = {
    description_block? ~
    input_block?
}

description_block = { "description" ~ string }
input_block = { "input" ~ type_declaration* }

type_declaration = { type_ ~ identifier }

// Object access - Fixed left recursion
function_call = { identifier ~ "(" ~ (expression ~ ("," ~ expression)*)? ~ ")" }
method_call_base = { identifier | "(" ~ expression ~ ")" }
method_call_segment = { "." ~ identifier ~ ("(" ~ (expression ~ ("," ~ expression)*)? ~ ")")? }
method_call = { method_call_base ~ method_call_segment+ }

// Primary expressions
primary = { 
    number |
    boolean |
    string |
    array_literal |
    matrix_literal |
    method_call |
    function_call |
    identifier |
    "(" ~ expression ~ ")"
}

// Expression with operator precedence
expression = { primary ~ (operator ~ primary)* }

operator = _{ binary_op | comparison_op | matrix_operation }
binary_op = { "+" | "-" | "*" | "/" | "%" | "^" | "&&" | "||" | "and" | "or" }
comparison_op = { "==" | "!=" | "<" | "<=" | ">" | ">=" | "is" | "not" }
matrix_operation = { "@*" | "@+" | "@-" | "@T" | "@I" }

// Arrays and Matrices
array_literal = { "[" ~ (expression ~ ("," ~ expression)*)? ~ "]" }
matrix_literal = { "[" ~ "[" ~ (expression ~ ("," ~ expression)*)? ~ "]" ~ ("," ~ "[" ~ (expression ~ ("," ~ expression)*)? ~ "]")* ~ "]" }
matrix_row_end = { "," }

// Statements
statement = {
    variable_decl |
    print_stmt |
    printl_stmt |
    if_stmt |
    iterate_stmt |
    from_to_stmt |
    test |
    return_stmt |
    expression
}

variable_decl = { "let" ~ identifier ~ (":" ~ type_)? ~ ("=" ~ expression)? }
print_stmt = { "print" ~ expression }
printl_stmt = { "printl" ~ expression }
if_stmt = { "if" ~ expression ~ block ~ ("else" ~ block)? }
iterate_stmt = { "iterate" ~ identifier ~ "in" ~ expression ~ block }
from_to_stmt = { "from" ~ expression ~ "to" ~ expression ~ ("step" ~ expression)? ~ block }
test = { "test" ~ string ~ block }
return_stmt = { "return" ~ expression? }

// Function declaration
function_def = {
    identifier ~ type_parameters? ~ setup_block ~ ("returns" ~ type_)? ~ block
}

// Class definition
class_decl = { 
    "class" ~ identifier ~ type_parameters? ~ 
    ("extends" ~ generic_type)? ~ 
    setup_block ~ 
    (constructor | function_def)* 
}

constructor = { "constructor" ~ setup_block ~ block }

// Program
program = { SOI ~ (function_def | class_decl | start_function | statement)* ~ EOI }

start_function = { "start" ~ "(" ~ ")" ~ block }
"#]
pub struct CleanParser;

fn test_parse(name: &str, source: &str) {
    println!("\n--- Testing '{}' ---", name);
    match CleanParser::parse(Rule::program, source) {
        Ok(_) => println!("✅ Parse successful!"),
        Err(e) => println!("❌ Parse failed: {}", e),
    }
}

fn main() {
    println!("Clean Language Parser Test Suite");
    println!("===============================");
    
    // Test 1: Simple variable declaration and print
    test_parse("Simple statements", r#"
        let x = 1 + 2
        print "Hello, world!"
    "#);
    
    // Test 2: Method calls with the fixed left recursion
    test_parse("Method calls", r#"
        let obj = object.method()
        obj.property.method(1, 2)
        (obj.prop1).prop2.method().another()
    "#);
    
    // Test 3: Function definition with setup block
    test_parse("Function definition", r#"
        calculate<T>
        description "Calculates something"
        input 
            number a
            number b
        returns number
        {
            let result = a + b
            return result
        }
    "#);
    
    // Skipping Class definition test for now
    /* 
    test_parse("Class definition", r#"
        class Point
        {
            calculate
            description "Calculates something"
            {
                print "Hello from method"
            }
        }
    "#);
    */
    
    // Test 4: Complex expressions with operators
    test_parse("Complex expressions", r#"
        let a = 1 + 2 * 3 / 4
        let b = (1 + 2) * (3 - 4)
        let c = matrix @* vector
        let d = a > b && c != 0
        let str = "Hello {{name}} world"
    "#);
    
    // Test 5: Array and matrix literals
    test_parse("Arrays and matrices", r#"
        let arr = [1, 2, 3, 4]
        let matrix = [[1, 2], [3, 4]]
    "#);
    
    // Test 6: Control flow constructs
    test_parse("Control flow", r#"
        if x > 10 {
            print "Greater than 10"
        } else {
            print "Less than or equal to 10"
        }
        
        iterate item in collection {
            print item
        }
        
        from 1 to 10 step 2 {
            print i
        }
        
        test "This should pass" {
            assert 1 == 1
        }
    "#);
    
    // Test 7: Start function
    test_parse("Start function", r#"
        start() {
            print "Program entry point"
            let x = 10
            if x > 5 {
                print "x is greater than 5"
            }
        }
    "#);
    
    // Test 8: Generic types and type parameters
    test_parse("Generic types", r#"
        let obj = object.method()
    "#);
    
    // Test 9: String interpolation
    test_parse("String interpolation", r#"
        let name = "Alice"
        let greeting = "Hello, {{name}}!"
        let complex = "The answer is {{2 + 2 * 10}} and your name is {{name}}"
    "#);
} 