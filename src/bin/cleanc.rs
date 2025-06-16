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
    
    // Parse the program with enhanced error reporting
    let program = match CleanParser::parse_program_with_file(&source, input_file) {
        Ok(p) => p,
        Err(e) => {
            display_enhanced_error(&e, &source, input_file);
            std::process::exit(1);
        }
    };
    
    // Debug print the parsed AST
    println!("Parsed AST: {:#?}", program);
    
    // Semantic analysis with enhanced error reporting
    let mut semantic_analyzer = SemanticAnalyzer::new();
    let analyzed_program = match semantic_analyzer.analyze(&program) {
        Ok(p) => p,
        Err(e) => {
            display_enhanced_error(&e, &source, input_file);
            std::process::exit(1);
        }
    };
    
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
    // Try to use the enhanced async runtime first
    if let Ok(rt) = tokio::runtime::Runtime::new() {
        return rt.block_on(async {
            match clean_language_compiler::runtime::run_clean_program_async(wasm_bytes).await {
                Ok(()) => Ok(()),
                Err(_) => {
                    // Fall back to synchronous runtime
                    println!("⚠️  Async runtime failed, falling back to synchronous execution");
                    run_wasm_sync(wasm_bytes)
                }
            }
        });
    }
    
    // Fallback to synchronous execution
    run_wasm_sync(wasm_bytes)
}

// Synchronous WebAssembly execution (fallback)
fn run_wasm_sync(wasm_bytes: &[u8]) -> Result<(), CompilerError> {
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
    
    // Add print functions to the linker
    
    // print(strPtr: i32, strLen: i32) -> void
    linker.func_wrap("env", "print", |mut caller: Caller<'_, ()>, str_ptr: i32, str_len: i32| {
        if let Some(memory) = caller.get_export("memory") {
            if let Some(memory) = memory.into_memory() {
                let data = memory.data(&caller);
                if str_ptr >= 0 && str_len >= 0 {
                    let start = str_ptr as usize;
                    let len = str_len as usize;
                    if start + len <= data.len() {
                        if let Ok(string) = std::str::from_utf8(&data[start..start + len]) {
                            print!("{}", string);
                        } else {
                            print!("[invalid UTF-8]");
                        }
                    } else {
                        print!("[out of bounds]");
                    }
                } else {
                    print!("[invalid pointer/length]");
                }
            }
        }
        Ok(())
    })
    .map_err(|e| CompilerError::runtime_error(
        format!("Failed to create print function: {}", e),
        None, None
    ))?;
    
    // printl(strPtr: i32, strLen: i32) -> void
    linker.func_wrap("env", "printl", |mut caller: Caller<'_, ()>, str_ptr: i32, str_len: i32| {
        if let Some(memory) = caller.get_export("memory") {
            if let Some(memory) = memory.into_memory() {
                let data = memory.data(&caller);
                if str_ptr >= 0 && str_len >= 0 {
                    let start = str_ptr as usize;
                    let len = str_len as usize;
                    if start + len <= data.len() {
                        if let Ok(string) = std::str::from_utf8(&data[start..start + len]) {
                            println!("{}", string);
                        } else {
                            println!("[invalid UTF-8]");
                        }
                    } else {
                        println!("[out of bounds]");
                    }
                } else {
                    println!("[invalid pointer/length]");
                }
            }
        }
        Ok(())
    })
    .map_err(|e| CompilerError::runtime_error(
        format!("Failed to create printl function: {}", e),
        None, None
    ))?;
    
    // print_simple(value: i32) -> void
    linker.func_wrap("env", "print_simple", |_caller: Caller<'_, ()>, value: i32| {
        print!("{}", value);
        Ok(())
    })
    .map_err(|e| CompilerError::runtime_error(
        format!("Failed to create print_simple function: {}", e),
        None, None
    ))?;
    
    // printl_simple(value: i32) -> void
    linker.func_wrap("env", "printl_simple", |_caller: Caller<'_, ()>, value: i32| {
        println!("{}", value);
        Ok(())
    })
    .map_err(|e| CompilerError::runtime_error(
        format!("Failed to create printl_simple function: {}", e),
        None, None
    ))?;
    
    // Add HTTP functions
    linker.func_wrap("env", "http_get", |_caller: Caller<'_, ()>, _url_ptr: i32, _url_len: i32| -> i32 {
        println!("[HTTP GET] Mock response");
        0 // Return mock string pointer
    })
    .map_err(|e| CompilerError::runtime_error(
        format!("Failed to create http_get function: {}", e),
        None, None
    ))?;
    
    linker.func_wrap("env", "http_post", |_caller: Caller<'_, ()>, _url_ptr: i32, _url_len: i32, _body_ptr: i32, _body_len: i32| -> i32 {
        println!("[HTTP POST] Mock response");
        0 // Return mock string pointer
    })
    .map_err(|e| CompilerError::runtime_error(
        format!("Failed to create http_post function: {}", e),
        None, None
    ))?;
    
    linker.func_wrap("env", "http_put", |_caller: Caller<'_, ()>, _url_ptr: i32, _url_len: i32, _body_ptr: i32, _body_len: i32| -> i32 {
        println!("[HTTP PUT] Mock response");
        0 // Return mock string pointer
    })
    .map_err(|e| CompilerError::runtime_error(
        format!("Failed to create http_put function: {}", e),
        None, None
    ))?;
    
    linker.func_wrap("env", "http_patch", |_caller: Caller<'_, ()>, _url_ptr: i32, _url_len: i32, _body_ptr: i32, _body_len: i32| -> i32 {
        println!("[HTTP PATCH] Mock response");
        0 // Return mock string pointer
    })
    .map_err(|e| CompilerError::runtime_error(
        format!("Failed to create http_patch function: {}", e),
        None, None
    ))?;
    
    linker.func_wrap("env", "http_delete", |_caller: Caller<'_, ()>, _url_ptr: i32, _url_len: i32| -> i32 {
        println!("[HTTP DELETE] Mock response");
        0 // Return mock string pointer
    })
    .map_err(|e| CompilerError::runtime_error(
        format!("Failed to create http_delete function: {}", e),
        None, None
    ))?;
    
    // Add file functions
    linker.func_wrap("env", "file_write", |_caller: Caller<'_, ()>, _path_ptr: i32, _path_len: i32, _content_ptr: i32, _content_len: i32| -> i32 {
        println!("[FILE WRITE] Mock operation");
        0 // Return success
    })
    .map_err(|e| CompilerError::runtime_error(
        format!("Failed to create file_write function: {}", e),
        None, None
    ))?;
    
    linker.func_wrap("env", "file_read", |_caller: Caller<'_, ()>, _path_ptr: i32, _path_len: i32, _result_ptr: i32| -> i32 {
        println!("[FILE READ] Mock operation");
        0 // Return length or -1 for error
    })
    .map_err(|e| CompilerError::runtime_error(
        format!("Failed to create file_read function: {}", e),
        None, None
    ))?;
    
    linker.func_wrap("env", "file_exists", |_caller: Caller<'_, ()>, _path_ptr: i32, _path_len: i32| -> i32 {
        println!("[FILE EXISTS] Mock operation");
        1 // Return 1 if exists, 0 if not
    })
    .map_err(|e| CompilerError::runtime_error(
        format!("Failed to create file_exists function: {}", e),
        None, None
    ))?;
    
    linker.func_wrap("env", "file_delete", |_caller: Caller<'_, ()>, _path_ptr: i32, _path_len: i32| -> i32 {
        println!("[FILE DELETE] Mock operation");
        0 // Return 0 for success, -1 for error
    })
    .map_err(|e| CompilerError::runtime_error(
        format!("Failed to create file_delete function: {}", e),
        None, None
    ))?;
    
    linker.func_wrap("env", "file_append", |_caller: Caller<'_, ()>, _path_ptr: i32, _path_len: i32, _content_ptr: i32, _content_len: i32| -> i32 {
        println!("[FILE APPEND] Mock operation");
        0 // Return success
    })
    .map_err(|e| CompilerError::runtime_error(
        format!("Failed to create file_append function: {}", e),
        None, None
    ))?;
    
    // Add async runtime functions (simplified synchronous versions)
    linker.func_wrap("env", "create_future", |_caller: Caller<'_, ()>, _future_name_ptr: i32, _future_name_len: i32| -> i32 {
        println!("🔮 [SYNC] Created future (mock)");
        1 // Return success
    })
    .map_err(|e| CompilerError::runtime_error(
        format!("Failed to create create_future function: {}", e),
        None, None
    ))?;
    
    linker.func_wrap("env", "start_background_task", |_caller: Caller<'_, ()>, _task_name_ptr: i32, _task_name_len: i32| -> i32 {
        println!("🔄 [SYNC] Started background task (mock)");
        1 // Return task ID
    })
    .map_err(|e| CompilerError::runtime_error(
        format!("Failed to create start_background_task function: {}", e),
        None, None
    ))?;
    
    linker.func_wrap("env", "execute_background", |_caller: Caller<'_, ()>, _operation_ptr: i32, _operation_len: i32| -> i32 {
        println!("🔄 [SYNC] Executing background operation (mock)");
        1 // Return success
    })
    .map_err(|e| CompilerError::runtime_error(
        format!("Failed to create execute_background function: {}", e),
        None, None
    ))?;
    
    linker.func_wrap("env", "resolve_future", |_caller: Caller<'_, ()>, _future_id: i32, _value: i32| -> i32 {
        println!("✅ [SYNC] Resolved future (mock)");
        1 // Return success
    })
    .map_err(|e| CompilerError::runtime_error(
        format!("Failed to create resolve_future function: {}", e),
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
} /// Display enhanced error information with source snippets and suggestions
fn display_enhanced_error(error: &CompilerError, source: &str, file_path: &str) {
    use clean_language_compiler::error::ErrorUtils;
    
    eprintln!("\n🚨 Compilation Error 🚨");
    eprintln!("File: {}", file_path);
    eprintln!();
    
    match error {
        CompilerError::Syntax { context } => {
            eprintln!("❌ Syntax Error: {}", context.message);
            
            if let Some(location) = &context.location {
                eprintln!("📍 Location: Line {}, Column {}", location.line, location.column);
            }
            
            if let Some(snippet) = &context.source_snippet {
                eprintln!("\n📝 Source Context:");
                eprintln!("{}", snippet);
            }
            
            if let Some(help) = &context.help {
                eprintln!("💡 Help: {}", help);
            }
            
            if !context.suggestions.is_empty() {
                eprintln!("\n🔧 Suggestions:");
                for suggestion in &context.suggestions {
                    eprintln!("  • {}", suggestion);
                }
            }
        },
        CompilerError::Type { context } => {
            eprintln!("❌ Type Error: {}", context.message);
            
            if let Some(location) = &context.location {
                eprintln!("📍 Location: Line {}, Column {}", location.line, location.column);
            }
            
            if let Some(help) = &context.help {
                eprintln!("💡 Help: {}", help);
            }
            
            if !context.suggestions.is_empty() {
                eprintln!("\n🔧 Suggestions:");
                for suggestion in &context.suggestions {
                    eprintln!("  • {}", suggestion);
                }
            }
        },
        _ => {
            eprintln!("❌ Error: {}", error);
        }
    }
    
    eprintln!("\n📚 For more help, check the Clean Language documentation.");
} 