use clap::{Parser, Subcommand};
use std::fs;
use std::path::Path;
use clean_language_compiler::compile_with_file;

/// Clean Language Compiler and Test Runner
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Compile a Clean Language file to WebAssembly
    Compile {
    /// Input file to compile
    #[arg(short, long)]
    input: String,

    /// Output file for the WebAssembly binary
    #[arg(short, long)]
    output: String,

    /// Optimization level (0-3)
    #[arg(short = 'l', long, default_value_t = 2)]
    opt_level: u8,
    },
    /// Run the Clean Language test suite
    Test {
        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,

        /// Additional test directories to include
        #[arg(short, long)]
        dirs: Vec<String>,
    },
    /// Run simple compilation tests
    SimpleTest {
        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,
    },
    /// Run comprehensive Clean Language test suite
    ComprehensiveTest {
        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    match args.command {
        Commands::Compile { input, output, opt_level } => {
            println!("Compiling {} to {}", input, output);
    
    // Read the input file
            let source = fs::read_to_string(&input)?;
    
    // Compile the source code to WebAssembly with file path for better error reporting
            let wasm_binary = compile_with_file(&source, &input)?;
    
    // Create output directory if it doesn't exist
            if let Some(parent) = Path::new(&output).parent() {
        fs::create_dir_all(parent)?;
    }
    
    // Write the WebAssembly binary to the output file
            fs::write(&output, wasm_binary)?;
    
            println!("Successfully compiled to {}", output);
        }
        Commands::Test { verbose, dirs } => {
            println!("🧪 Running Clean Language Test Suite...\n");
            
            // Import the test runner
            use clean_language_compiler::tests::test_runner;
            
            // Convert string dirs to &str
            let dir_refs: Vec<&str> = dirs.iter().map(|s| s.as_str()).collect();
            
            // Run tests with configuration
            test_runner::run_tests_with_config(verbose, dir_refs).await?;
        }
        Commands::SimpleTest { verbose } => {
            println!("🚀 Running Clean Language Simple Tests...\n");
            
            // Import the simple test runner
            use clean_language_compiler::tests::simple_test_runner;
            
            // Run simple tests
            simple_test_runner::run_simple_tests(verbose);
        }
        Commands::ComprehensiveTest { verbose } => {
            println!("🧪 Running Comprehensive Clean Language Test Suite...\n");
            
            // Import the comprehensive test runner
            use clean_language_compiler::tests::simple_test_runner;
            
            // Run comprehensive tests
            simple_test_runner::run_comprehensive_tests_with_reporting(verbose);
        }
    }
    
    Ok(())
} 