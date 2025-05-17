use pest::Parser;
use pest::iterators::Pair;
use std::process;
use clean_language::parser::{CleanParser, Rule};
use clean_language::error::CompilerError;

fn main() {
    println!("Starting parser test...");
    
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
            process::exit(1);
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