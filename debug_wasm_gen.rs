use clean_language_compiler::codegen::CodeGenerator;
use clean_language_compiler::stdlib::numeric_ops::NumericOperations;

fn main() {
    let mut codegen = CodeGenerator::new();
    let numeric_ops = NumericOperations::new();
    
    println!("Registering numeric operations...");
    if let Err(e) = numeric_ops.register_functions(&mut codegen) {
        println!("Error registering functions: {:?}", e);
        return;
    }
    
    println!("Generating test module...");
    match codegen.generate_test_module() {
        Ok(wasm_bytes) => {
            println!("Successfully generated {} bytes of WASM", wasm_bytes.len());
            
            // Try to validate with wasmtime
            let engine = wasmtime::Engine::default();
            match wasmtime::Module::new(&engine, &wasm_bytes) {
                Ok(_) => println!("WASM module is valid!"),
                Err(e) => println!("WASM validation error: {}", e),
            }
        },
        Err(e) => println!("Error generating module: {:?}", e),
    }
}
