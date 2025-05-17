use clap::Parser;
use std::fs;
use std::path::Path;
use clean_language_compiler::compile;

/// Clean Language Compiler
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input file to compile
    #[arg(short, long)]
    input: String,

    /// Output file for the WebAssembly binary
    #[arg(short, long)]
    output: String,

    /// Optimization level (0-3)
    #[arg(short = 'l', long, default_value_t = 2)]
    opt_level: u8,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    println!("Compiling {} to {}", args.input, args.output);
    
    // Read the input file
    let source = fs::read_to_string(&args.input)?;
    
    // Compile the source code to WebAssembly
    let wasm_binary = compile(&source)?;
    
    // Create output directory if it doesn't exist
    if let Some(parent) = Path::new(&args.output).parent() {
        fs::create_dir_all(parent)?;
    }
    
    // Write the WebAssembly binary to the output file
    fs::write(&args.output, wasm_binary)?;
    
    println!("Successfully compiled to {}", args.output);
    Ok(())
} 