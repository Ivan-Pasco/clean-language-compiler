use clean_language_compiler::parser::CleanParser;

fn main() {
    let source = r#"
    // A simple Clean Language program for testing the parser
    start() {
        let value = 42
        printl "Hello, world!"
        return value
    }
    "#;
    
    match CleanParser::parse_program(source) {
        Ok(program) => {
            println!("Parsing successful! Found {} functions", program.functions.len());
        },
        Err(error) => {
            eprintln!("Parsing failed: {}", error);
        }
    }
} 