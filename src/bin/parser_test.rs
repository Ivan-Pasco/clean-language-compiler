use clean_language_compiler::parser::{CleanParser, Rule};
use pest::Parser;
use clean_language_compiler::ast::{Statement, Value, Expression};
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    let source = if args.len() > 1 {
        // Load from file
        let filename = &args[1];
        fs::read_to_string(filename).unwrap_or_else(|e| {
            panic!("Failed to read file {}: {}", filename, e);
        })
    } else {
        // Use new specification syntax as fallback
        "function start()\n    print 42".to_string()
    };
    
    println!("Source code: {}", source);
    
    // Parse using the pest parser directly to see the raw tokens
    let pairs = CleanParser::parse(Rule::program, &source).unwrap_or_else(|e| {
        panic!("Parse error: {}", e);
    });
    
    // Print the parse tree
    println!("\nParse tree:");
    for pair in pairs {
        print_pairs(pair, 0);
    }
    
    // Try to parse the program
    match CleanParser::parse_program(&source) {
        Ok(program) => {
            println!("\nSuccessfully parsed program!");
            println!("Functions: {}", program.functions.len());
            
            if let Some(start_func) = &program.start_function {
                println!("Found start function with {} statements", start_func.body.len());
                
                // Analyze the statements
                for (i, stmt) in start_func.body.iter().enumerate() {
                    println!("Statement {}: {:?}", i, stmt);
                    
                    // Specifically check for return statements
                    if let Statement::Return { value, .. } = stmt {
                        println!("  Return statement value: {:?}", value);
                        
                        if let Some(expr) = value {
                            match expr {
                                Expression::Literal(val) => {
                                    println!("  Literal value: {:?}", val);
                                    if let Value::Integer(i) = val {
                                        println!("  Integer value: {}", i);
                                    }
                                },
                                _ => println!("  Expression: {:?}", expr),
                            }
                        }
                    }
                }
            } else {
                println!("No start function found");
            }
            
            for func in &program.functions {
                println!("Regular function: {} with {} statements", func.name, func.body.len());
            }
        },
        Err(e) => {
            println!("Error parsing program: {:?}", e);
        }
    }
}

fn print_pairs(pair: pest::iterators::Pair<Rule>, level: usize) {
    let indent = "  ".repeat(level);
    println!("{}Rule: {:?}, Span: {:?}", indent, pair.as_rule(), pair.as_span());
    println!("{}Text: '{}'", indent, pair.as_str());
    
    for inner_pair in pair.into_inner() {
        print_pairs(inner_pair, level + 1);
    }
} 