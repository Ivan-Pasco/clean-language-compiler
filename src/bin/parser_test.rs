use clean_language_compiler::parser::{CleanParser, Rule};
use pest::Parser;
use clean_language_compiler::ast::{Statement, Value, Expression};

fn main() {
    let source = "start() { return 42 }";
    println!("Source code: {}", source);
    
    // Parse using the pest parser directly to see the raw tokens
    let pairs = CleanParser::parse(Rule::program, source).unwrap_or_else(|e| {
        panic!("Parse error: {}", e);
    });
    
    // Print the parse tree
    println!("\nParse tree:");
    for pair in pairs {
        print_pairs(pair, 0);
    }
    
    // Try to parse the program
    match CleanParser::parse_program(source) {
        Ok(program) => {
            println!("\nSuccessfully parsed program with {} functions", program.functions.len());
            
            // Check the start function
            if let Some(start_func) = program.functions.iter().find(|f| f.name == "start") {
                println!("Found start function with {} statements", start_func.body.len());
                
                // Analyze the statements
                for (i, stmt) in start_func.body.iter().enumerate() {
                    println!("Statement {}: {:?}", i, stmt);
                    
                    // Specifically check for return statements
                    if let Statement::Return { value, .. } = stmt {
                        println!("  Return statement value: {:?}", value);
                        
                        // Check if the value is an integer literal
                        if let Some(expr) = value {
                            println!("  Return expr: {:?}", expr);
                            
                            // Print more detailed debugging information
                            match expr {
                                Expression::Literal(val) => {
                                    println!("  Literal value: {:?}", val);
                                    
                                    // Check if it's an integer
                                    if let Value::Integer(i) = val {
                                        println!("  Integer value: {}", i);
                                    }
                                },
                                _ => println!("  Not a literal: {:?}", expr),
                            }
                        }
                    }
                }
            } else {
                println!("Start function not found");
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