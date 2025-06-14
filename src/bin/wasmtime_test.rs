use std::env;
use std::fs;
use wasmtime::{Engine, Module, Store, Instance, Linker, Caller, Val, ValType, FuncType};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <wasm_file>", args[0]);
        std::process::exit(1);
    }

    let wasm_file = &args[1];
    println!("🔍 Testing WASM file with wasmtime: {}", wasm_file);

    // Load the WASM file
    let wasm_bytes = fs::read(wasm_file)?;
    println!("📦 File size: {} bytes", wasm_bytes.len());

    // Create wasmtime engine and store
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);

    // Define the print functions
    linker.func_wrap("env", "print_simple", |_caller: Caller<'_, ()>, value: i32| {
        print!("{}", value);
    })?;

    linker.func_wrap("env", "printl_simple", |_caller: Caller<'_, ()>, value: i32| {
        println!("{}", value);
    })?;

    // Try to compile the module
    println!("🔧 Compiling WASM module...");
    let module = match Module::from_binary(&engine, &wasm_bytes) {
        Ok(m) => {
            println!("✅ WASM module compiled successfully");
            m
        },
        Err(e) => {
            println!("❌ Failed to compile WASM module: {}", e);
            return Err(e.into());
        }
    };

    // Print module info
    println!("📋 Module info:");
    for import in module.imports() {
        println!("  Import: {} {:?} ({:?})", import.module(), import.name(), import.ty());
    }
    for export in module.exports() {
        println!("  Export: {} ({:?})", export.name(), export.ty());
    }

    // Try to instantiate
    println!("🚀 Instantiating WASM module...");
    let instance = match linker.instantiate(&mut store, &module) {
        Ok(i) => {
            println!("✅ WASM module instantiated successfully");
            i
        },
        Err(e) => {
            println!("❌ Failed to instantiate WASM module: {}", e);
            return Err(e.into());
        }
    };

    // Try to call the start function
    println!("▶️  Calling start function...");
    match instance.get_func(&mut store, "start") {
        Some(start_func) => {
            match start_func.call(&mut store, &[], &mut []) {
                Ok(_) => println!("✅ Start function executed successfully"),
                Err(e) => {
                    println!("❌ Error calling start function: {}", e);
                    return Err(e.into());
                }
            }
        },
        None => {
            println!("⚠️  No start function found");
        }
    }

    Ok(())
} 