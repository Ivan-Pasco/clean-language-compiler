use std::path::Path;
use std::fs;
use std::io::{self, Read, Write};
use std::env;
use clean_language_compiler::parser::CleanParser;
use clean_language_compiler::semantic::SemanticAnalyzer;
use clean_language_compiler::codegen::CodeGenerator;
use clean_language_compiler::error::CompilerError;

fn main() -> Result<(), CompilerError> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        print_usage();
        return Ok(());
    }
    
    match args[1].as_str() {
        "compile" => {
            if args.len() < 3 {
                println!("Error: No input file specified.");
                print_usage();
                return Ok(());
            }
            
            let input_file = &args[2];
            let output_file = if args.len() >= 4 {
                args[3].clone()
            } else {
                // Remove the extension (e.g. ".cln") safely and append ".wasm"
                match Path::new(input_file).file_stem() {
                    Some(stem) => format!("{}.wasm", stem.to_string_lossy()),
                    None => format!("{}.wasm", input_file) // fallback – should not happen
                }
            };

            compile_file(input_file, &output_file)?;
        },
        "run" => {
            if args.len() < 3 {
                println!("Error: No input file specified.");
                print_usage();
                return Ok(());
            }
            
            let input_file = &args[2];
            execute_file(input_file)?;
        },
        "help" => {
            print_usage();
        },
        _ => {
            println!("Unknown command: {}", args[1]);
            print_usage();
         }
     }
    
    Ok(())
}

fn print_usage() {
    println!("Clean Language Compiler");
    println!("Usage:");
    println!("  cleanc compile <input-file> [output-file]  # Compile a Clean program to WebAssembly");
    println!("  cleanc run <input-file>                   # Compile and run a Clean program");
    println!("  cleanc help                              # Show this help message");
}

fn compile_file(input_file: &str, output_file: &str) -> Result<(), CompilerError> {
    println!("Compiling {} to {}...", input_file, output_file);
    
    // Read the input file
    let mut source = String::new();
    let mut file = fs::File::open(input_file)
        .map_err(|e| CompilerError::io_error(format!("Failed to open file: {}", e), None, None))?;
    file.read_to_string(&mut source)
        .map_err(|e| CompilerError::io_error(format!("Failed to read file: {}", e), None, None))?;
    
    // Debug: Print source code
    println!("Source code:\n{}", source);
    
    // Parse the program
    let program = CleanParser::parse_program(&source)?;
    
    // Semantic analysis
    let mut semantic_analyzer = SemanticAnalyzer::new();
    let analyzed_program = semantic_analyzer.analyze(&program)?;
    
    // Code generation
    let mut code_generator = CodeGenerator::new();
    let wasm_binary = code_generator.generate(&analyzed_program)?;
    
    // Write the output file
    fs::write(output_file, wasm_binary)
        .map_err(|e| CompilerError::io_error(format!("Failed to write output file: {}", e), None, None))?;
    
    println!("Compilation successful!");
    Ok(())
}

fn execute_file(input_file: &str) -> Result<(), CompilerError> {
    println!("Executing {}...", input_file);
    
    // Check if the file exists
    if !Path::new(input_file).exists() {
        // If the input is a .cln file, compile it first
        if input_file.ends_with(".cln") {
            let wasm_file = format!("{}.wasm", input_file.trim_end_matches(".cln"));
            compile_file(input_file, &wasm_file)?;
            return execute_file(&wasm_file);
        } else {
            return Err(CompilerError::io_error(
                format!("File not found: {}", input_file),
                None, None
            ));
        }
    }
    
    // If it's not a WASM file, try to compile it first
    if !input_file.ends_with(".wasm") {
        let wasm_file = format!("{}.wasm", input_file);
        compile_file(input_file, &wasm_file)?;
        return execute_file(&wasm_file);
    }
    
    // Read the WASM file
    let wasm_bytes = fs::read(input_file)
        .map_err(|e| CompilerError::io_error(
            format!("Failed to read WASM file: {}", e),
            None, None
        ))?;
    
    // Use wasmtime to execute the WASM file
    println!("Running WASM file with wasmtime...");
    match run_wasm_with_wasmtime(&wasm_bytes) {
        Ok(_) => {
            println!("Execution completed successfully!");
            Ok(())
        },
        Err(e) => Err(CompilerError::runtime_error(
            format!("Failed to execute WASM: {}", e),
            None, None
        ))
    }
}

// Function to run a WebAssembly module with wasmtime
fn run_wasm_with_wasmtime(wasm_bytes: &[u8]) -> Result<(), CompilerError> {
    use wasmtime::{Config, Engine, Module, Store, Linker, Caller, Val};
    
    // Use default configuration - simpler and more compatible
    let config = Config::default();
    
    // Create the engine
    let engine = Engine::new(&config)
        .map_err(|e| CompilerError::runtime_error(
            format!("Failed to create WebAssembly engine: {}", e),
            None, None
        ))?;
    
    // Create a module from the bytes
    let module = Module::new(&engine, wasm_bytes)
        .map_err(|e| CompilerError::runtime_error(
            format!("Failed to create WebAssembly module: {}", e),
            None, None
        ))?;
    
    // Create a store
    let mut store = Store::new(&engine, ());
    
    // Create a linker
    let mut linker = Linker::new(&engine);
    
    // Add print function to the linker
    linker.func_wrap("env", "print", |_caller: Caller<'_, ()>, value: i32| {
        println!("Program output: {}", value);
        Ok(())
    })
    .map_err(|e| CompilerError::runtime_error(
        format!("Failed to create print function: {}", e),
        None, None
    ))?;
    
    // Instantiate the module
    let instance = linker.instantiate(&mut store, &module)
        .map_err(|e| CompilerError::runtime_error(
            format!("Failed to instantiate WebAssembly module: {}", e),
            None, None
        ))?;
    
    // Try to get the start function
    if let Some(start) = instance.get_func(&mut store, "start") {
        // Check if the function takes no parameters
        let start_type = start.ty(&store);
        let results_len = start_type.results().len();
        
        // Create a buffer to store return values
        let mut results = vec![Val::I32(0); results_len];
        
        // Call the start function
        start.call(&mut store, &[], &mut results)
            .map_err(|e| CompilerError::runtime_error(
                format!("Failed to call start function: {}", e),
                None, None
            ))?;
            
        println!("Program executed successfully!");
        
        // If there are return values, print them
        if !results.is_empty() {
            println!("Return value: {:?}", results[0]);
        }
        
        return Ok(());
    }
    
    // If no start function, look for an _start function as fallback
    if let Some(start) = instance.get_func(&mut store, "_start") {
        // Check if the function takes no parameters
        let start_type = start.ty(&store);
        let results_len = start_type.results().len();
        
        // Create a buffer to store return values
        let mut results = vec![Val::I32(0); results_len];
        
        // Call the start function
        start.call(&mut store, &[], &mut results)
            .map_err(|e| CompilerError::runtime_error(
                format!("Failed to call _start function: {}", e),
                None, None
            ))?;
            
        println!("Program executed successfully!");
        
        // If there are return values, print them
        if !results.is_empty() {
            println!("Return value: {:?}", results[0]);
        }
        
        return Ok(());
    }
    
    // No suitable entry point found
    Err(CompilerError::runtime_error(
        "No suitable entry function found in the WebAssembly module",
        Some("The module should export a 'start' function with no parameters".to_string()),
        None
    ))
} 