#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use clean_language_compiler::parser::CleanParser;

    #[test]
    fn test_parser_with_complex_syntax() {
        let test_file_path = Path::new("tests/complex_syntax_test.cln");
        assert!(test_file_path.exists(), "Test file does not exist");

        let source = fs::read_to_string(test_file_path)
            .expect("Failed to read test file");

        let result = CleanParser::parse_program(&source);
        
        // Only check if parsing succeeds, not the exact AST structure
        match result {
            Ok(program) => {
                // Check that we have functions and classes
                assert!(!program.functions.is_empty(), "No functions parsed");
                assert!(!program.classes.is_empty(), "No classes parsed");
                
                // Check the start function
                let start_fn = program.functions.iter()
                    .find(|f| f.name == "start")
                    .expect("Start function not found");
                    
                assert!(!start_fn.body.is_empty(), "Start function body is empty");
                
                // Check the add function
                let add_fn = program.functions.iter()
                    .find(|f| f.name == "add")
                    .expect("Add function not found");
                    
                assert!(!add_fn.type_parameters.is_empty(), "No type parameters in add function");
                assert!(!add_fn.parameters.is_empty(), "No parameters in add function");
                assert!(add_fn.description.is_some(), "No description in add function");
                
                // Check the Vector class
                let vector_class = program.classes.iter()
                    .find(|c| c.name == "Vector")
                    .expect("Vector class not found");
                    
                assert!(!vector_class.type_parameters.is_empty(), "No type parameters in Vector class");
                assert!(vector_class.base_class.is_some(), "No base class in Vector class");
                assert!(vector_class.description.is_some(), "No description in Vector class");
                assert!(!vector_class.methods.is_empty(), "No methods in Vector class");
                assert!(vector_class.constructor.is_some(), "No constructor in Vector class");
                
                println!("Successfully parsed complex syntax test file!");
            },
            Err(e) => {
                panic!("Failed to parse complex syntax test file: {}", e);
            }
        }
    }
} 