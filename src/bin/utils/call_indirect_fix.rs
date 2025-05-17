// A helper script to fix CallIndirect instructions
// This script processes array_ops.rs and fixes CallIndirect field names
// Run with: cargo run --bin call_indirect_fix

use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;
use regex::Regex;

fn main() -> io::Result<()> {
    let filepath = "src/stdlib/array_ops.rs";
    let path = Path::new(filepath);
    if !path.exists() {
        println!("File not found: {}", filepath);
        return Ok(());
    }

    println!("Processing file: {}", filepath);
    let mut file_content = String::new();
    fs::File::open(path)?.read_to_string(&mut file_content)?;

    // Fix CallIndirect instructions
    let re_call_indirect = Regex::new(r"CallIndirect\s*\{\s*ty_idx:\s*(\d+),\s*table_idx:\s*(\d+)\s*\}").unwrap();
    let fixed_content = re_call_indirect.replace_all(&file_content, "CallIndirect { ty: $1, table: $2 }").to_string();

    // Write the fixed content back to the file
    if file_content != fixed_content {
        println!("Applying fixes to {}", filepath);
        let mut file = fs::File::create(path)?;
        file.write_all(fixed_content.as_bytes())?;
        println!("Fixed CallIndirect instructions in {}", filepath);
    } else {
        println!("No fixes needed for {}", filepath);
    }

    Ok(())
} 