use clean_language_compiler::parser::CleanParser;

fn main() {
    // Test parsing a complete program
    let program_source = r#"
    // A test program with multiple functions
    start() {
        let a = 10
        let b = 20
        let result = add(a, b)
        printl "The result is ${result}"
        return result
    }
    
    add(x, y) {
        return x + y
    }
    "#;
    
    match CleanParser::parse_program(program_source) {
        Ok(program) => {
            println!("Program parsing successful!");
            println!("Found {} functions", program.functions.len());
            for function in &program.functions {
                println!("Function: {}", function.name);
            }
        },
        Err(error) => {
            eprintln!("Program parsing failed: {}", error);
        }
    }
    
    // Note: We can't easily test individual expression or statement parsing
    // because the current API expects Pest Pairs not raw strings
} 