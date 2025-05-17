use std::fs;
use std::env;
use wasmparser::Parser;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 3 {
        println!("Usage: wasm2wat <input.wasm> <output.wat>");
        return;
    }
    
    let input_file = &args[1];
    let output_file = &args[2];
    
    println!("Converting {} to {}...", input_file, output_file);
    
    // Read the WASM file
    let wasm_content = match fs::read(input_file) {
        Ok(content) => content,
        Err(e) => {
            println!("Error reading file {}: {}", input_file, e);
            return;
        }
    };
    
    // Parse the WebAssembly module
    let mut parser = Parser::new(0);
    let mut output = String::new();
    output.push_str("(module\n");
    
    // This is a very simplified conversion just to understand the structure
    for payload in parser.parse_all(&wasm_content) {
        match payload {
            Ok(payload) => {
                match payload {
                    wasmparser::Payload::Version { num, range } => {
                        output.push_str(&format!("  ;; WebAssembly version: {}\n", num));
                    },
                    wasmparser::Payload::TypeSection(reader) => {
                        output.push_str("  ;; Type section\n");
                        for item in reader {
                            if let Ok(func_type) = item {
                                let params = func_type.params();
                                let returns = func_type.returns();
                                
                                output.push_str("  (type (func");
                                if !params.is_empty() {
                                    output.push_str(" (param");
                                    for param in params {
                                        output.push_str(&format!(" {}", val_type_to_string(*param)));
                                    }
                                    output.push_str(")");
                                }
                                if !returns.is_empty() {
                                    output.push_str(" (result");
                                    for ret in returns {
                                        output.push_str(&format!(" {}", val_type_to_string(*ret)));
                                    }
                                    output.push_str(")");
                                }
                                output.push_str("))\n");
                            }
                        }
                    },
                    wasmparser::Payload::ImportSection(reader) => {
                        output.push_str("  ;; Import section\n");
                        for item in reader {
                            if let Ok(import) = item {
                                output.push_str(&format!("  (import \"{}\" \"{}\" ...)\n", 
                                                        import.module, import.name));
                            }
                        }
                    },
                    wasmparser::Payload::FunctionSection(reader) => {
                        output.push_str("  ;; Function section\n");
                        for item in reader {
                            if let Ok(type_idx) = item {
                                output.push_str(&format!("  (func (type {}))\n", type_idx));
                            }
                        }
                    },
                    wasmparser::Payload::ExportSection(reader) => {
                        output.push_str("  ;; Export section\n");
                        for item in reader {
                            if let Ok(export) = item {
                                output.push_str(&format!("  (export \"{}\" {})\n", 
                                                        export.name, export_kind_to_string(export.kind, export.index)));
                            }
                        }
                    },
                    wasmparser::Payload::CodeSection(reader) => {
                        output.push_str("  ;; Code section\n");
                        for item in reader {
                            if let Ok(body) = item {
                                output.push_str("  (func\n");
                                // We'd need a full parser to properly convert instructions
                                output.push_str("    ;; Function body\n");
                                output.push_str("  )\n");
                            }
                        }
                    },
                    wasmparser::Payload::MemorySection(reader) => {
                        output.push_str("  ;; Memory section\n");
                        for item in reader {
                            if let Ok(memory) = item {
                                output.push_str(&format!("  (memory {})\n", memory.initial));
                            }
                        }
                    },
                    _ => {
                        // Handle other section types if needed
                    }
                }
            },
            Err(e) => {
                println!("Error parsing WebAssembly: {}", e);
                return;
            }
        }
    }
    
    output.push_str(")\n");
    
    // Write the output WAT file
    if let Err(e) = fs::write(output_file, output) {
        println!("Error writing file {}: {}", output_file, e);
        return;
    }
    
    println!("Conversion successful!");
}

fn val_type_to_string(val_type: wasmparser::ValType) -> &'static str {
    match val_type {
        wasmparser::ValType::I32 => "i32",
        wasmparser::ValType::I64 => "i64",
        wasmparser::ValType::F32 => "f32",
        wasmparser::ValType::F64 => "f64",
        wasmparser::ValType::V128 => "v128",
        wasmparser::ValType::Ref(_) => "ref",
        _ => "unknown",
    }
}

fn export_kind_to_string(kind: wasmparser::ExternalKind, index: u32) -> String {
    match kind {
        wasmparser::ExternalKind::Function => format!("(func {})", index),
        wasmparser::ExternalKind::Table => format!("(table {})", index),
        wasmparser::ExternalKind::Memory => format!("(memory {})", index),
        wasmparser::ExternalKind::Global => format!("(global {})", index),
        _ => format!("(unknown {})", index),
    }
} 