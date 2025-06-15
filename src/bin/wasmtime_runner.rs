use wasmtime::*;
use std::env;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <wasm_file>", args[0]);
        return Ok(());
    }

    let wasm_file = &args[1];
    println!("🚀 Loading WebAssembly file: {}", wasm_file);

    // Read the WASM file
    let wasm_bytes = fs::read(wasm_file)?;
    println!("📦 File size: {} bytes", wasm_bytes.len());

    // Create engine and store
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());

    // Create module
    let module = Module::new(&engine, &wasm_bytes)?;

    // Create linker and add imports
    let mut linker = Linker::new(&engine);

    // Add print function: print(ptr: i32, len: i32) -> void
    linker.func_wrap("env", "print", |ptr: i32, len: i32| {
        println!("Print called with ptr={}, len={}", ptr, len);
        // In a real implementation, we would read from WebAssembly memory
        print!("(printed string)");
    })?;

    // Add printl function: printl(ptr: i32, len: i32) -> void  
    linker.func_wrap("env", "printl", |ptr: i32, len: i32| {
        println!("Printl called with ptr={}, len={}", ptr, len);
        // In a real implementation, we would read from WebAssembly memory
        println!("(printed string with newline)");
    })?;

    // Add print_simple function: print_simple(value: i32) -> void
    linker.func_wrap("env", "print_simple", |value: i32| {
        print!("{}", value);
    })?;

    // Add printl_simple function: printl_simple(value: i32) -> void
    linker.func_wrap("env", "printl_simple", |value: i32| {
        println!("{}", value);
    })?;

    // Add file operation stubs (they won't be used in this test)
    linker.func_wrap("env", "file_write", |_: i32, _: i32, _: i32, _: i32| -> i32 { 0 })?;
    linker.func_wrap("env", "file_read", |_: i32, _: i32, _: i32| -> i32 { 0 })?;
    linker.func_wrap("env", "file_exists", |_: i32, _: i32| -> i32 { 0 })?;
    linker.func_wrap("env", "file_delete", |_: i32, _: i32| -> i32 { 0 })?;
    linker.func_wrap("env", "file_append", |_: i32, _: i32, _: i32, _: i32| -> i32 { 0 })?;
    linker.func_wrap("env", "http_get", |_: i32, _: i32| -> i32 { 0 })?;
    linker.func_wrap("env", "http_post", |_: i32, _: i32, _: i32, _: i32| -> i32 { 0 })?;
    linker.func_wrap("env", "http_put", |_: i32, _: i32, _: i32, _: i32| -> i32 { 0 })?;
    linker.func_wrap("env", "http_delete", |_: i32, _: i32| -> i32 { 0 })?;
    linker.func_wrap("env", "http_patch", |_: i32, _: i32, _: i32, _: i32| -> i32 { 0 })?;
    linker.func_wrap("env", "http_head", |_: i32, _: i32| -> i32 { 0 })?;
    linker.func_wrap("env", "http_options", |_: i32, _: i32| -> i32 { 0 })?;

    // Instantiate the module
    let instance = linker.instantiate(&mut store, &module)?;

    println!("✅ WebAssembly module loaded successfully");
    println!("📋 Exported functions: {:?}", instance.exports(&mut store).map(|e| e.name()).collect::<Vec<_>>());

    // Get and call the start function
    let start_func = instance.get_func(&mut store, "start")
        .ok_or_else(|| anyhow::anyhow!("start function not found"))?;
    println!("🎯 Executing start function...");
    println!("--- Output ---");
    start_func.call(&mut store, &[], &mut [])?;
    println!("--- End Output ---");
    println!("✅ Execution completed successfully!");

    Ok(())
} 