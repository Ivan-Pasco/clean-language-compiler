// A helper script to fix WasmType usage
// This script processes files and updates direct tuple usage to use the type conversion helpers
// Run with: cargo run --bin wasm_type_fix -- <filepath>

use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;
use regex::Regex;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: cargo run --bin wasm_type_fix -- <filepath>");
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

    // Import the type conversion helpers if needed
    if !file_content.contains("use crate::types::{to_tuple, from_tuple}") && 
       !file_content.contains("use crate::types::*") {
        // Add the import after the last use statement
        let re_last_use = Regex::new(r"(use [^;]+;)(?!.*use)").unwrap();
        if let Some(captures) = re_last_use.captures(&file_content) {
            let last_use = captures.get(1).unwrap().as_str();
            let replacement = format!("{}\nuse crate::types::{{to_tuple, from_tuple, WasmType}};", last_use);
            file_content = re_last_use.replace(&file_content, replacement).to_string();
        } else {
            // If no use statements found, add it at the beginning
            file_content = format!("use crate::types::{{to_tuple, from_tuple, WasmType}};\n\n{}", file_content);
        }
    }

    // Replace direct tuple patterns with WasmType conversions
    let patterns = [
        (r"\(\d+,\s*ValType::I32\)", "to_tuple(WasmType::I32)"),
        (r"\(\d+,\s*ValType::I64\)", "to_tuple(WasmType::I64)"),
        (r"\(\d+,\s*ValType::F32\)", "to_tuple(WasmType::F32)"),
        (r"\(\d+,\s*ValType::F64\)", "to_tuple(WasmType::F64)"),
        (r"\(\d+,\s*ValType::V128\)", "to_tuple(WasmType::V128)"),
    ];

    let mut fixed_content = file_content.clone();
    for (pattern, replacement) in patterns.iter() {
        let re = Regex::new(pattern).unwrap();
        fixed_content = re.replace_all(&fixed_content, *replacement).to_string();
    }

    // Write the fixed content back to the file
    if file_content != fixed_content {
        println!("Applying fixes to {}", filepath);
        let mut file = fs::File::create(path)?;
        file.write_all(fixed_content.as_bytes())?;
        println!("Fixed WasmType usage in {}", filepath);
    } else {
        println!("No fixes needed for {}", filepath);
    }

    Ok(())
} 