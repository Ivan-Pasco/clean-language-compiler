use clean_language_compiler::compile;

fn main() {
    let source = r#"start()
	number x = 42
	print(x)"#;

    println!("Source program:");
    println!("{}", source);
    println!();

    match compile(source) {
        Ok(wasm_binary) => {
            println!("✓ Compilation succeeded! Generated {} bytes", wasm_binary.len());
            
            // Let's try to validate the WASM
            match wasmtime::Module::new(&wasmtime::Engine::default(), &wasm_binary) {
                Ok(_module) => {
                    println!("✓ WASM validation succeeded!");
                },
                Err(e) => {
                    println!("✗ WASM validation failed: {}", e);
                    
                    // Write binary for inspection
                    std::fs::write("debug_trace.wasm", &wasm_binary).expect("Failed to write WASM");
                    println!("WASM binary written to debug_trace.wasm for inspection");
                }
            }
        },
        Err(e) => {
            println!("✗ Compilation failed: {}", e);
        }
    }
}