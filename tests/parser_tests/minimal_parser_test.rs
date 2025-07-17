use pest::Parser;
use pest_derive::Parser;
use std::path::Path;

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
array_type = { "list" ~ "<" ~ type_ ~ ">" }

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
    expression
}

variable_decl = { "let" ~ identifier ~ (":" ~ type_)? ~ ("=" ~ expression)? }
print_stmt = { "print" ~ expression }
printl_stmt = { "printl" ~ expression }
if_stmt = { "if" ~ expression ~ block ~ ("else" ~ block)? }
iterate_stmt = { "iterate" ~ identifier ~ "in" ~ expression ~ block }
from_to_stmt = { "from" ~ expression ~ "to" ~ expression ~ ("step" ~ expression)? ~ block }
test = { "test" ~ string ~ block }

// Function declaration
function_decl = { function_def }

function_def = {
    identifier ~ type_parameters? ~ setup_block ~ ("returns" ~ type_)? ~ block
}

// Class definition
class_decl = { 
    "class" ~ identifier ~ type_parameters? ~ 
    ("extends" ~ generic_type)? ~ 
    setup_block ~ 
    constructor? ~ 
    function_decl* 
}

constructor = { "constructor" ~ setup_block ~ block }

// Program
program = { SOI ~ (statement | start_function | function_decl | class_decl)* ~ EOI }

start_function = { "start" ~ "(" ~ ")" ~ block }
"#]
pub struct CleanParser;

fn main() {
    println!("Testing parser with a simple expression");
    
    let source = r#"
        // Simple test
        let x = 1 + 2
        print "Hello, world!"
    "#;
    
    let result = CleanParser::parse(Rule::program, source);
    
    match result {
        Ok(pairs) => {
            println!("Parse successful!");
            for pair in pairs {
                println!("Rule: {:?}, Span: {:?}", pair.as_rule(), pair.as_span());
                // Print the first two levels of the parse tree
                for inner in pair.into_inner() {
                    println!("  Inner Rule: {:?}, Span: {:?}", inner.as_rule(), inner.as_span());
                }
            }
        },
        Err(e) => {
            println!("Parse failed: {}", e);
        }
    }
    
    // Now test the method call rule that had left-recursion issues
    let method_call_source = r#"
        object.method()
        object.property.method()
    "#;
    
    let method_call_result = CleanParser::parse(Rule::program, method_call_source);
    
    match method_call_result {
        Ok(_) => println!("Method call parsing successful!"),
        Err(e) => println!("Method call parsing failed: {}", e)
    }
} 