#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use pest::Parser;
    use pest_derive::Parser;

    #[derive(Parser)]
    #[grammar = "src/parser/grammar.pest"]
    struct CleanParser;

    #[test]
    fn test_parser_with_complex_syntax() {
        let test_file_path = Path::new("tests/complex_syntax_test.cln");
        assert!(test_file_path.exists(), "Test file does not exist");

        let source = fs::read_to_string(test_file_path)
            .expect("Failed to read test file");

        let result = CleanParser::parse(Rule::program, &source);
        
        match result {
            Ok(_) => {
                println!("Successfully parsed complex syntax test file!");
            },
            Err(e) => {
                panic!("Failed to parse complex syntax test file: {}", e);
            }
        }
    }
} 