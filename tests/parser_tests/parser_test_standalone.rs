use std::fs;
use std::path::Path;
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "src/parser/grammar.pest"]
struct CleanParser;

fn main() {
    // Test both the simple and complex test files
    test_parse_file("tests/complex_syntax_advanced.cln", "advanced complex syntax");
    test_parse_file("tests/complex_syntax_test.cln", "standard complex syntax");
    
    // Test a simple piece of code directly
    test_parse_code(
        r#"
        // Simple test program
        functions:
            add() returns number
                input:
                    number:
                        - x
                        - y
                x + y
        "#,
        "simple function"
    );
    
    // Test a more complex piece of code directly
    test_parse_code(
        r#"
        constants:
            PI = 3.14159
            
        classes:
            Circle
                properties:
                    number:
                        - radius
                methods:
                    getArea() returns number
                        PI * this.radius * this.radius
                        
        functions:
            calculateCircleAreas() returns number[]
                input:
                    Circle[]:
                        - circles
                number[] areas
                iterate i from 0 to length(circles)
                    areas[i] = circles[i].getArea()
                areas
        "#,
        "math program with constants, class and function"
    );
}

fn test_parse_file(file_path: &str, description: &str) {
    println!("\nTesting {} file: {}", description, file_path);
    let path = Path::new(file_path);
    
    if !path.exists() {
        println!("❌ Test file does not exist!");
        return;
    }

    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            println!("❌ Failed to read test file: {}", e);
            return;
        }
    };

    test_parse_source(&source, description);
}

fn test_parse_code(source: &str, description: &str) {
    println!("\nTesting {} code snippet", description);
    test_parse_source(source, description);
}

fn test_parse_source(source: &str, description: &str) {
    println!("Attempting to parse {}...", description);
    
    let result = CleanParser::parse(Rule::program, source);
    
    match result {
        Ok(_) => {
            println!("✅ Successfully parsed {}!", description);
        },
        Err(e) => {
            println!("❌ Failed to parse {}: {}", description, e);
            
            // Extract position information to help pinpoint the issue
            if let Some(pos) = e.line_col {
                println!("Error position: Line {}, Column {}", pos.0, pos.1);
                
                // Print the problematic line
                let lines: Vec<&str> = source.lines().collect();
                if pos.0 > 0 && pos.0 <= lines.len() {
                    println!("Problematic line:");
                    println!("{}", lines[pos.0 - 1]);
                    // Print marker pointing to the error position
                    println!("{:>width$}^", "", width = pos.1);
                }
            }
        }
    }
} 