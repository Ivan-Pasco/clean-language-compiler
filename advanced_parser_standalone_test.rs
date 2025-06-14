use clean_language_compiler::parser::CleanParser;

fn main() {
    // Test parsing a program with advanced features
    let advanced_program = r#"
    // Advanced program with classes, generics, and error handling
    
    // A simple class definition
    class Vector<T> {
        private values: array<T>
        private size: integer
        
        constructor(capacityParam: integer) {
            values = new array<T>(capacityParam)
            size = 0
        }
        
        push(value: T) {
            if size < values.length {
                values[size] = value
                size = size + 1
            } else {
                printl "Error: Vector is full"
            }
        }
        
        get(index: integer) -> T {
            if index >= 0 && index < size {
                return values[index]
            } else {
                // In a real language, we'd throw an exception here
                printl "Error: Index out of bounds"
                return null
            }
        }
        
        size() -> integer {
            return size
        }
    }
    
    start() {
        // Create a vector of integers
        let v = new Vector<integer>(5)
        
        // Add some values
        v.push(10)
        v.push(20)
        v.push(30)
        
        // Print the values
        printl "Vector contents:"
        for i from 0 to v.size() - 1 {
            printl v.get(i)
        }
        
        // Try error handling with out of bounds access
        let value = v.get(10)
        
        return 0
    }
    "#;
    
    match CleanParser::parse_program(advanced_program) {
        Ok(program) => {
            println!("Advanced program parsing successful!");
            println!("Found {} functions", program.functions.len());
            println!("Found {} classes", program.classes.len());
            
            for class in &program.classes {
                println!("Class: {}", class.name);
                println!("  Methods: {}", class.methods.len());
                println!("  Fields: {}", class.fields.len());
            }
        },
        Err(error) => {
            eprintln!("Advanced program parsing failed: {}", error);
        }
    }
} 