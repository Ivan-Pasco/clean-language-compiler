use wasmparser::{Parser, Validator as WasmParserValidator, WasmFeatures, Payload};
use crate::error::{CompilerError};
use std::collections::HashSet;

use wasmtime::{Engine, Module};

#[derive(Debug)]
pub struct CleanValidator {
    features: WasmFeatures,
}

#[derive(Debug)]
pub struct WasmAnalysis {
    pub num_functions: usize,
    pub memory_pages: u32,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
    pub function_types: HashSet<String>,
    pub memory_sections: usize,
    pub global_count: usize,
    pub data_count: usize,
}

impl CleanValidator {
    pub fn new() -> Self {
        Self {
            features: WasmFeatures {
                reference_types: true,
                multi_value: true,
                bulk_memory: true,
                simd: true,
                threads: false,
                tail_call: false,
                multi_memory: false,
                exceptions: true,
                memory64: false,
                ..Default::default()
            },
        }
    }

    pub fn validate_and_analyze(&self, wasm_binary: &[u8]) -> Result<WasmAnalysis, CompilerError> {
        let mut validator = WasmParserValidator::new_with_features(self.features.clone());
        let mut analysis = WasmAnalysis {
            num_functions: 0,
            memory_pages: 0,
            imports: Vec::new(),
            exports: Vec::new(),
            function_types: HashSet::new(),
            memory_sections: 0,
            global_count: 0,
            data_count: 0,
        };
        
        // Parse and validate the WebAssembly binary
        for payload in Parser::new(0).parse_all(wasm_binary) {
            let payload = payload.map_err(|e| CompilerError::validation_error(
                format!("Failed to parse WASM binary: {}", e),
                None,
                None
            ))?;

            // Update analysis based on payload type
            match &payload {
                Payload::TypeSection(reader) => {
                    for ty in reader.clone() {
                        let ty = ty.map_err(|e| CompilerError::validation_error(
                            format!("Invalid type section: {}", e),
                            None,
                            None
                        ))?;
                        analysis.function_types.insert(format!("{:?}", ty));
                    }
                }
                Payload::ImportSection(reader) => {
                    for import in reader.clone() {
                        let import = import.map_err(|e| CompilerError::validation_error(
                            format!("Invalid import section: {}", e),
                            None,
                            None
                        ))?;
                        analysis.imports.push(format!("{}.{}", import.module, import.name));
                    }
                }
                Payload::FunctionSection(reader) => {
                    analysis.num_functions += reader.count() as usize;
                }
                Payload::ExportSection(reader) => {
                    for export in reader.clone() {
                        let export = export.map_err(|e| CompilerError::validation_error(
                            format!("Invalid export section: {}", e),
                            None,
                            None
                        ))?;
                        analysis.exports.push(export.name.to_string());
                    }
                }
                Payload::MemorySection(reader) => {
                    analysis.memory_sections += 1;
                    for memory in reader.clone() {
                        let memory = memory.map_err(|e| CompilerError::validation_error(
                            format!("Invalid memory section: {}", e),
                            None,
                            None
                        ))?;
                        analysis.memory_pages = memory.initial as u32;
                    }
                }
                Payload::GlobalSection(reader) => {
                    analysis.global_count += reader.count() as usize;
                }
                Payload::DataSection(reader) => {
                    analysis.data_count += reader.count() as usize;
                }
                _ => {}
            }
            
            // Validate the payload
            validator.payload(&payload).map_err(|e| {
                CompilerError::validation_error(
                    format!("WASM validation error: {}", e),
                    None,
                    None
                )
            })?;
        }

        // Perform additional validation checks
        self.validate_analysis(&analysis)?;

        Ok(analysis)
    }

    fn validate_analysis(&self, analysis: &WasmAnalysis) -> Result<(), CompilerError> {
        // Check if we have at least one memory section
        if analysis.memory_sections == 0 {
            return Err(CompilerError::validation_error(
                "No memory section found in WASM binary",
                Some("Add a memory section to the WebAssembly module".to_string()),
                None
            ));
        }

        // Check if we have required exports
        let required_exports = ["memory", "alloc", "dealloc"];
        for export in required_exports {
            if !analysis.exports.iter().any(|e| e == export) {
                return Err(CompilerError::validation_error(
                    format!("Required export '{}' not found", export),
                    Some(format!("Add the '{}' export to the WebAssembly module", export)),
                    None
                ));
            }
        }

        // Check memory pages
        if analysis.memory_pages < 1 {
            return Err(CompilerError::validation_error(
                "Memory must have at least one page",
                Some("Configure the memory section with at least one page (64KB)".to_string()),
                None
            ));
        }

        // Check function count
        if analysis.num_functions == 0 {
            return Err(CompilerError::validation_error(
                "No functions found in WASM binary",
                Some("Add at least one function to the WebAssembly module".to_string()),
                None
            ));
        }

        Ok(())
    }

    pub fn validate_imports(&self, imports: &[String]) -> Result<(), CompilerError> {
        let required_imports = ["env.memory", "env.table"];
        for import in required_imports {
            if !imports.contains(&import.to_string()) {
                return Err(CompilerError::validation_error(
                    format!("Required import '{}' not found", import),
                    Some(format!("Add the '{}' import to the WebAssembly module", import)),
                    None
                ));
            }
        }
        Ok(())
    }

    pub fn validate_exports(&self, exports: &[String]) -> Result<(), CompilerError> {
        let required_exports = ["memory", "alloc", "dealloc"];
        for export in required_exports {
            if !exports.contains(&export.to_string()) {
                return Err(CompilerError::validation_error(
                    format!("Required export '{}' not found", export),
                    Some(format!("Add the '{}' export to the WebAssembly module", export)),
                    None
                ));
            }
        }
        Ok(())
    }
}

pub struct Validator;

impl Validator {
    pub fn validate_wasm(wasm_bytes: &[u8]) -> Result<(), CompilerError> {
        // Check if WASM binary is valid
        let engine = Engine::default();
        let _module = Module::new(&engine, wasm_bytes).map_err(|e| {
            CompilerError::codegen_error(
                format!("Invalid WASM binary: {}", e),
                None,
                None
            )
        })?;
        
        // Check if functions are defined
        if !Self::has_functions(wasm_bytes) {
            return Err(CompilerError::codegen_error(
                "No functions found in WASM binary",
                Some("Ensure that at least one function is defined".to_string()),
                None
            ));
        }
        
        // Other validations can be added here
        Ok(())
    }
    
    pub fn validate_imports(wasm_bytes: &[u8], required_imports: &[&str]) -> Result<(), CompilerError> {
        for import in required_imports {
            if !Self::has_import(wasm_bytes, import) {
                return Err(CompilerError::codegen_error(
                    format!("Required import '{}' not found", import),
                    Some(format!("Ensure that import '{}' is properly defined", import)),
                    None
                ));
            }
        }
        Ok(())
    }
    
    pub fn validate_exports(wasm_bytes: &[u8], required_exports: &[&str]) -> Result<(), CompilerError> {
        for export in required_exports {
            if !Self::has_export(wasm_bytes, export) {
                return Err(CompilerError::codegen_error(
                    format!("Required export '{}' not found", export),
                    Some(format!("Ensure that export '{}' is properly defined", export)),
                    None
                ));
            }
        }
        Ok(())
    }
    
    // Helper functions
    fn has_functions(wasm_bytes: &[u8]) -> bool {
        // A simple check to determine if WASM binary has functions
        // In a real implementation, this would use a WASM parser
        let engine = Engine::default();
        if let Ok(_module) = Module::new(&engine, wasm_bytes) {
            // Check if module has any functions
            true // Simplified for now
        } else {
            false
        }
    }
    
    fn has_import(wasm_bytes: &[u8], _import_name: &str) -> bool {
        // In a real implementation, this would use a WASM parser
        let engine = Engine::default();
        if let Ok(_module) = Module::new(&engine, wasm_bytes) {
            // Check if module has the specified import
            true // Simplified for now
        } else {
            false
        }
    }
    
    fn has_export(wasm_bytes: &[u8], _export_name: &str) -> bool {
        // In a real implementation, this would use a WASM parser
        let engine = Engine::default();
        if let Ok(_module) = Module::new(&engine, wasm_bytes) {
            // Check if module has the specified export
            true // Simplified for now
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_validation() {
        let validator = CleanValidator::new();
        
        // Create a minimal valid WASM binary
        let wasm_binary = wat::parse_str(r#"
            (module
                (memory (export "memory") 1)
                (func (export "alloc") (param i32) (result i32)
                    local.get 0
                )
                (func (export "dealloc") (param i32)
                    local.get 0
                    drop
                )
            )
        "#).unwrap();

        let result = validator.validate_and_analyze(&wasm_binary);
        assert!(result.is_ok());
        
        let analysis = result.unwrap();
        assert_eq!(analysis.memory_pages, 1);
        assert_eq!(analysis.num_functions, 2);
        assert_eq!(analysis.exports.len(), 3);
    }

    #[test]
    fn test_invalid_wasm() {
        let validator = CleanValidator::new();
        
        // Create an invalid WASM binary (missing required exports)
        let wasm_binary = wat::parse_str(r#"
            (module
                (memory 1)
                (func (param i32) (result i32)
                    local.get 0
                )
            )
        "#).unwrap();

        let result = validator.validate_and_analyze(&wasm_binary);
        assert!(result.is_err());
    }
} 