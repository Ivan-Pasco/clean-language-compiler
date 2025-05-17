use pest::iterators::Pair;
use crate::ast::{Program, Function};
use crate::error::CompilerError;
use crate::parser::grammar::Rule;
use crate::parser::parser_impl;

/// Parse a program from the AST
pub fn parse_program_ast(pair: Pair<Rule>) -> Result<Program, CompilerError> {
    match pair.as_rule() {
        Rule::program => {
            let mut functions = Vec::new();
            
            // Extract all functions
            for inner_pair in pair.into_inner() {
                match inner_pair.as_rule() {
                    Rule::function_decl => {
                        // Parse function using the existing parser_impl
                        let function = parser_impl::parse_function(inner_pair)?;
                        functions.push(function);
                    },
                    Rule::start_function => {
                        // Parse start function using the existing parser_impl
                        let function = parser_impl::parse_start_function(inner_pair)?;
                        functions.push(function);
                    },
                    Rule::class_decl => {
                        // Classes are handled in a different part of the parser
                    },
                    Rule::EOI => {
                        // End of input, ignore
                    },
                    _ => {
                        // Ignore other rules
                    }
                }
            }
            
            // Create and return the Program
            Ok(Program { 
                functions,
                classes: Vec::new()
            })
        },
        _ => Err(CompilerError::parse_error(
            format!("Expected program, found {:?}", pair.as_rule()),
            None,
            None
        ))
    }
} 