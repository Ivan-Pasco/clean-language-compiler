use pest::Parser;
use pest::iterators::Pair;
use pest_derive::Parser;
use crate::ast::{Program, Function, Parameter, Statement, Expression, Type, Value, Operator, ComparisonOperator, MatrixOperator};
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
pub use parser_impl::{parse, parse_start_function, get_location, parse_function};
pub use expression_parser::{parse_expression, parse_primary, parse_string, parse_array_literal, parse_matrix_literal, parse_function_call};
pub use statement_parser::parse_statement;
pub use type_parser::parse_type;
pub use class_parser::parse_class;
pub use program_parser::parse_program_ast;

#[derive(Debug, Clone, PartialEq)]
pub enum StringPart {
    Text(String),
    Expression(Box<Expression>),
}

impl CleanParser {
    pub fn parse_program(source: &str) -> Result<Program, CompilerError> {
        parser_impl::parse(source)
    }
}