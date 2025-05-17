use wasm_encoder::{
    Function, Instruction, Module, TypeSection, FunctionSection, 
    ExportSection, CodeSection, ValType, BlockType, ExportKind
};

fn main() {
    // Create a new WebAssembly module
    let mut module = Module::new();
    
    // Type section - define function signatures
    let mut types = TypeSection::new();
    // Type 0: () -> i32 (function with no parameters returning i32)
    types.function(vec![], vec![ValType::I32]);
    
    // Add type section to the module
    module.section(&types);
    
    // Function section - associate function bodies with their type signatures
    let mut functions = FunctionSection::new();
    // Function 0 uses type 0
    functions.function(0);
    
    // Add function section to the module
    module.section(&functions);
    
    // Export section - make functions accessible from outside
    let mut exports = ExportSection::new();
    // Export function 0 as "start"
    exports.export("start", ExportKind::Func, 0);
    
    // Add export section to the module
    module.section(&exports);
    
    // Code section - contains the actual function bodies
    let mut codes = CodeSection::new();
    
    // Define function 0's body (the start function)
    let mut start_fn = Function::new(vec![]);
    
    // Push the value 42 onto the stack
    start_fn.instruction(&Instruction::I32Const(42));
    
    // Return from the function (will return the value on the stack)
    start_fn.instruction(&Instruction::Return);
    
    // End the function definition
    start_fn.instruction(&Instruction::End);
    
    // Add the function body to the code section
    codes.function(&start_fn);
    
    // Add code section to the module
    module.section(&codes);
    
    // Encode the module to a binary
    let wasm_bytes = module.finish();
    
    // Write the WASM binary to a file
    std::fs::write("simple_test.wasm", wasm_bytes).expect("Unable to write WASM file");
    
    println!("Generated simple_test.wasm successfully!");
} 