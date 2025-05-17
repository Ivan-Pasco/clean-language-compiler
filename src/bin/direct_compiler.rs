use clean_language_compiler::parser::CleanParser;
use clean_language_compiler::ast::{Program, Statement, Expression, Value};
use clean_language_compiler::error::CompilerError;
use wasm_encoder::{
    Function, Instruction, Module, TypeSection, FunctionSection, 
    ExportSection, CodeSection, ValType, ExportKind
};
use std::fs;
use std::path::Path;

fn main() -> Result<(), CompilerError> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: direct_compiler <input_file.cln> [output_file.wasm]");
        return Ok(());
    }

    let input_file = &args[1];
    let output_file = if args.len() >= 3 {
        args[2].clone()
    } else {
        format!("{}.wasm", input_file)
    };

    println!("Compiling {} to {}...", input_file, output_file);

    // Read the input file
    let mut source = String::new();
    let mut file = fs::File::open(input_file)
        .map_err(|e| CompilerError::io_error(format!("Failed to open file: {}", e), None, None))?;
    std::io::Read::read_to_string(&mut file, &mut source)
        .map_err(|e| CompilerError::io_error(format!("Failed to read file: {}", e), None, None))?;

    // Parse the program
    let program = CleanParser::parse_program(&source)?;

    // Generate WebAssembly directly
    let wasm_bytes = generate_wasm(&program)?;

    // Write the output file
    fs::write(output_file, wasm_bytes)
        .map_err(|e| CompilerError::io_error(format!("Failed to write output file: {}", e), None, None))?;

    println!("Compilation successful!");
    Ok(())
}

/// Generate WebAssembly binary directly from the AST
fn generate_wasm(program: &Program) -> Result<Vec<u8>, CompilerError> {
    // Create a new WebAssembly module
    let mut module = Module::new();
    
    // Type section - define function signatures
    let mut types = TypeSection::new();
    // Type 0: () -> i32 (function with no parameters returning i32)
    types.function(vec![], vec![ValType::I32]);
    
    // Add type section to the module
    module.section(&types);
    
    // Function section - associate function bodies with their type signatures
    let mut functions = FunctionSection::new();
    // Function 0 uses type 0
    functions.function(0);
    
    // Add function section to the module
    module.section(&functions);
    
    // Export section - make functions accessible from outside
    let mut exports = ExportSection::new();
    
    // Find the start function in the program
    let start_fn_index = program.functions.iter()
        .position(|f| f.name == "start")
        .ok_or_else(|| CompilerError::codegen_error("No 'start' function found in the program", None, None))?;
    
    // Export function 0 as "start"
    exports.export("start", ExportKind::Func, 0);
    
    // Add export section to the module
    module.section(&exports);
    
    // Code section - contains the actual function bodies
    let mut codes = CodeSection::new();
    
    // Define the start function's body
    let mut start_fn = Function::new(vec![]);
    
    // Generate instructions for the start function
    let start_function = &program.functions[start_fn_index];
    
    for stmt in &start_function.body {
        match stmt {
            Statement::Return { value, .. } => {
                if let Some(expr) = value {
                    match expr {
                        Expression::Literal(Value::Integer(n)) => {
                            start_fn.instruction(&Instruction::I32Const(*n));
                        },
                        _ => {
                            return Err(CompilerError::codegen_error(
                                "Only integer literal return values are supported", None, None
                            ));
                        }
                    }
                } else {
                    // If no return value, push a default value (0)
                    start_fn.instruction(&Instruction::I32Const(0));
                }
                start_fn.instruction(&Instruction::Return);
            },
            _ => {
                // For simplicity, we're ignoring other statement types
                // In a real implementation, you'd handle all statement types
            }
        }
    }
    
    // End the function definition
    start_fn.instruction(&Instruction::End);
    
    // Add the function body to the code section
    codes.function(&start_fn);
    
    // Add code section to the module
    module.section(&codes);
    
    // Encode the module to a binary
    Ok(module.finish())
} 