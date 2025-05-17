// A standalone parser test with all required dependencies
// To run: rustc parser_fix_standalone.rs -L deps --extern pest=deps/libpest.rlib --extern pest_derive=deps/libpest_derive.rlib && ./parser_fix_standalone

extern crate pest;
extern crate pest_derive;

use pest::Parser;
use pest::iterators::{Pair, Pairs};
use pest_derive::Parser;

// Define our own CleanParser for testing purposes
#[derive(Parser)]
#[grammar = "src/parser/grammar.pest"]
pub struct CleanParser;

// This is the key fix needed in the main parser module
// We need to use the Rule enum that pest_derive generates automatically
pub use self::Rule;

fn main() {
    println!("Starting parser fix test...");
    
    // Simple source code to parse
    let source = r#"
        start() {
            let x: integer = 10;
            print x;
        }
    "#;
    
    // Parse the program
    match CleanParser::parse(Rule::program, source) {
        Ok(pairs) => {
            println!("Successfully parsed the program!");
            // Iterate through the parse tree
            for pair in pairs {
                print_pair(pair, 0);
            }
        }
        Err(e) => {
            println!("Failed to parse the program: {}", e);
            std::process::exit(1);
        }
    }
    
    println!("Parser test completed successfully.");
}

// Helper function to print the parse tree
fn print_pair(pair: Pair<Rule>, indent: usize) {
    let indent_str = " ".repeat(indent);
    println!("{}Rule::{:?} => {}", indent_str, pair.as_rule(), pair.as_str());
    
    // Print inner pairs
    for inner_pair in pair.into_inner() {
        print_pair(inner_pair, indent + 2);
    }
} 