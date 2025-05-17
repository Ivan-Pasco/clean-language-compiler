use std::fs;
use std::path::Path;
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "src/parser/grammar.pest"]
struct CleanParser;

fn main() {
    let test_file_path = Path::new("tests/complex_syntax_test.cln");
    if !test_file_path.exists() {
        println!("Test file does not exist!");
        return;
    }

    let source = match fs::read_to_string(test_file_path) {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to read test file: {}", e);
            return;
        }
    };

    let result = CleanParser::parse(Rule::program, &source);
    
    match result {
        Ok(_) => {
            println!("SUCCESS: Successfully parsed complex syntax test file!");
        },
        Err(e) => {
            println!("ERROR: Failed to parse complex syntax test file: {}", e);
        }
    }
} 