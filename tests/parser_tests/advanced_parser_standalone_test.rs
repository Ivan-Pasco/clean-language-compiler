use std::fs;
use std::path::Path;
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "src/parser/grammar.pest"]
struct CleanParser;

fn main() {
    let test_file_path = Path::new("tests/complex_syntax_advanced.cln");
    if !test_file_path.exists() {
        println!("Advanced test file does not exist!");
        return;
    }

    let source = match fs::read_to_string(test_file_path) {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to read advanced test file: {}", e);
            return;
        }
    };

    println!("Attempting to parse advanced syntax test file...");
    
    let result = CleanParser::parse(Rule::program, &source);
    
    match result {
        Ok(pairs) => {
            println!("SUCCESS: Successfully parsed advanced syntax test file!");
            
            // Print summary of top-level items parsed
            println!("\nParsed structure summary:");
            let mut constants_count = 0;
            let mut classes_count = 0;
            let mut functions_count = 0;
            
            for pair in pairs {
                match pair.as_rule() {
                    Rule::constants_block => {
                        constants_count += 1;
                        println!("  - Constants block found");
                    },
                    Rule::class_decl => {
                        classes_count += 1;
                        println!("  - Class declaration found");
                    },
                    Rule::function_decl => {
                        functions_count += 1;
                        println!("  - Function declaration found");
                    },
                    _ => {}
                }
            }
            
            println!("\nTotal items parsed:");
            println!("  Constants blocks: {}", constants_count);
            println!("  Classes: {}", classes_count);
            println!("  Functions: {}", functions_count);
        },
        Err(e) => {
            println!("ERROR: Failed to parse advanced syntax test file:");
            println!("{}", e);
            
            // Extract position information to help pinpoint the issue
            if let Some(pos) = e.line_col {
                println!("\nError position: Line {}, Column {}", pos.0, pos.1);
                
                // Print the problematic line
                let lines: Vec<&str> = source.lines().collect();
                if pos.0 > 0 && pos.0 <= lines.len() {
                    println!("\nProblematic line:");
                    println!("{}", lines[pos.0 - 1]);
                    // Print marker pointing to the error position
                    println!("{:>width$}^", "", width = pos.1);
                }
            }
        }
    }
} 