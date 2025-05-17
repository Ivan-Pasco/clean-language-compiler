// A simple test to verify that the parser works correctly
// This is a standalone test file that can be run with:
// rustc -L ~/.cargo/registry/src/github.com-*/pest-2.7.* -L ~/.cargo/registry/src/github.com-*/pest_derive-2.7.* parser_standalone_test.rs && ./parser_standalone_test

extern crate pest;
extern crate pest_derive;

use pest::Parser;
use pest::error::Error;
use pest::iterators::Pair;
use std::fs;

#[derive(pest_derive::Parser)]
#[grammar = "src/parser/grammar.pest"]
pub struct CleanParser;

fn main() {
    println!("Testing parser with a simple program...");
    
    // Simple test program
    let source = r#"
    start()
        print("Hello, world!")
        x = 1 + 2 * 3
        if x > 5
            print("x is greater than 5")
        else
            print("x is not greater than 5")
    "#;
    
    match CleanParser::parse(pest::pratt_parser::Rule::program, source) {
        Ok(pairs) => {
            println!("Parsing successful!");
            // Print the parsed AST
            for pair in pairs {
                print_pair(pair, 0);
            }
        },
        Err(e) => {
            println!("Parsing failed:");
            println!("{}", e);
        }
    }
}

// Helper function to print the parse tree
fn print_pair(pair: Pair<pest::pratt_parser::Rule>, indent: usize) {
    let indent_str = " ".repeat(indent * 2);
    println!("{}{:?}: {}", indent_str, pair.as_rule(), pair.as_str());
    
    for inner_pair in pair.into_inner() {
        print_pair(inner_pair, indent + 1);
    }
} 