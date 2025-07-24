use wasm_encoder::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut module = Module::new();
    
    // Type section
    let mut types = TypeSection::new();
    types.function(vec![], vec![ValType::I32]);
    module.section(&types);
    
    // Function section  
    let mut functions = FunctionSection::new();
    functions.function(0);
    module.section(&functions);
    
    // Code section
    let mut codes = CodeSection::new();
    let locals = vec![];
    let mut f = Function::new(locals);
    f.instructions()
        .i32_const(42)
        .end();
    codes.function(&f);
    module.section(&codes);
    
    // Export section
    let mut exports = ExportSection::new();
    exports.export("test", ExportKind::Func, 0);
    module.section(&exports);
    
    let wasm_bytes = module.finish();
    std::fs::write("test_output.wasm", wasm_bytes)?;
    println!("Created test WASM file");
    
    Ok(())
}