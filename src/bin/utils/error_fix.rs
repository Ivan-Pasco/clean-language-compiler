// A helper script to fix error method calls
// This script processes files and updates error method calls to use the new signature
// Run with: cargo run --bin error_fix -- <filepath>

use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;
use regex::Regex;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: cargo run --bin error_fix -- <filepath>");
        return Ok(());
    }

    let filepath = &args[1];
    let path = Path::new(filepath);
    if !path.exists() {
        println!("File not found: {}", filepath);
        return Ok(());
    }

    println!("Processing file: {}", filepath);
    let mut file_content = String::new();
    fs::File::open(path)?.read_to_string(&mut file_content)?;

    // Fix runtime_error calls
    let re_runtime = Regex::new(r"CompilerError::runtime_error\(([^,\)]+)\)").unwrap();
    let fixed_runtime = file_content.replace(&re_runtime, "CompilerError::runtime_error($1, None, None)");
    
    // Fix validation_error calls
    let re_validation = Regex::new(r"CompilerError::validation_error\(([^,\)]+)\)").unwrap();
    let fixed_validation = fixed_runtime.replace(&re_validation, "CompilerError::validation_error($1, None, None)");
    
    // Fix codegen_error calls
    let re_codegen = Regex::new(r"CompilerError::codegen_error\(([^,\)]+)\)").unwrap();
    let fixed_codegen = fixed_validation.replace(&re_codegen, "CompilerError::codegen_error($1, None, None)");
    
    // Fix parse_error calls
    let re_parse = Regex::new(r"CompilerError::parse_error\(([^,\)]+)\)").unwrap();
    let fixed_parse = fixed_codegen.replace(&re_parse, "CompilerError::parse_error($1, None, None)");

    // Fix with_help calls to use Option-based methods
    let re_with_help = Regex::new(r"\.with_help\(([^)]+)\)").unwrap();
    let fixed_with_help = fixed_parse.replace(&re_with_help, ".with_help_option(Some($1))");

    // Fix with_location calls to use Option-based methods
    let re_with_location = Regex::new(r"\.with_location\(([^)]+)\)").unwrap();
    let fixed_with_location = fixed_with_help.replace(&re_with_location, ".with_location_option(Some($1))");

    // Final content with all fixes
    let final_content = fixed_with_location;

    // Write the fixed content back to the file
    if file_content != final_content {
        println!("Applying fixes to {}", filepath);
        let mut file = fs::File::create(path)?;
        file.write_all(final_content.as_bytes())?;
        println!("Fixed {} error method calls in {}", count_fixes(&file_content, &final_content), filepath);
    } else {
        println!("No fixes needed for {}", filepath);
    }

    Ok(())
}

fn count_fixes(original: &str, fixed: &str) -> usize {
    let diff_count = original.lines().count().abs_diff(fixed.lines().count());
    if diff_count > 0 {
        return diff_count;
    }

    // If line count is the same, count differences in content
    original.chars()
        .zip(fixed.chars())
        .filter(|(a, b)| a != b)
        .count() / 10 // Rough estimate of number of fixes based on character differences
} 