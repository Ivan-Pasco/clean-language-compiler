use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use crate::ast::{Program, Function, Class, ImportItem, Type, Visibility};
use crate::error::CompilerError;
use crate::parser;

/// Module resolver handles loading and linking of external modules
pub struct ModuleResolver {
    /// Cache of loaded modules to avoid duplicate loading
    module_cache: HashMap<String, Module>,
    /// Search paths for finding modules
    module_paths: Vec<PathBuf>,
    /// Current module being processed (for relative imports)
    current_module: Option<String>,
}

/// Represents a loaded module with its exported symbols
#[derive(Debug, Clone)]
pub struct Module {
    pub name: String,
    pub file_path: PathBuf,
    pub program: Program,
    pub exports: ModuleExports,
}

/// Exported symbols from a module
#[derive(Debug, Clone)]
pub struct ModuleExports {
    pub functions: HashMap<String, Function>,
    pub classes: HashMap<String, Class>,
    pub types: HashMap<String, Type>,
}

/// Import resolution result
#[derive(Debug, Clone)]
pub struct ImportResolution {
    pub resolved_imports: HashMap<String, Module>,
    pub symbol_map: HashMap<String, String>, // alias -> actual_name
}

impl ModuleResolver {
    /// Create a new module resolver
    pub fn new() -> Self {
        Self {
            module_cache: HashMap::new(),
            module_paths: vec![
                PathBuf::from("./"),           // Current directory
                PathBuf::from("./modules/"),   // Local modules directory
                PathBuf::from("./lib/"),       // Library directory
                PathBuf::from("./stdlib/"),    // Standard library
            ],
            current_module: None,
        }
    }

    /// Add a search path for modules
    pub fn add_module_path<P: AsRef<Path>>(&mut self, path: P) {
        self.module_paths.push(path.as_ref().to_path_buf());
    }

    /// Set the current module context
    pub fn set_current_module(&mut self, module_name: String) {
        self.current_module = Some(module_name);
    }

    /// Resolve all imports for a program
    pub fn resolve_imports(&mut self, program: &Program) -> Result<ImportResolution, CompilerError> {
        let mut resolved_imports = HashMap::new();
        let mut symbol_map = HashMap::new();

        for import_item in &program.imports {
            // Load the module
            let module = self.load_module(&import_item.name)?;
            
            // Register the module with its actual name or alias
            let import_name = import_item.alias.as_ref().unwrap_or(&import_item.name);
            resolved_imports.insert(import_name.clone(), module.clone());
            
            // Create symbol mapping
            if let Some(alias) = &import_item.alias {
                symbol_map.insert(alias.clone(), import_item.name.clone());
            }
        }

        Ok(ImportResolution {
            resolved_imports,
            symbol_map,
        })
    }

    /// Load a module by name
    fn load_module(&mut self, module_name: &str) -> Result<Module, CompilerError> {
        // Check cache first
        if let Some(cached_module) = self.module_cache.get(module_name) {
            return Ok(cached_module.clone());
        }

        // Find the module file
        let module_path = self.find_module_file(module_name)?;

        // Read and parse the module
        let source = fs::read_to_string(&module_path)
            .map_err(|e| CompilerError::module_error(
                format!("Failed to read module '{}': {}", module_name, e),
                Some(format!("Check if the file exists and is readable: {}", module_path.display())),
                None
            ))?;

        // Parse the module
        let program = parser::parse_with_file(&source, &module_path.to_string_lossy())
            .map_err(|e| CompilerError::module_error(
                format!("Failed to parse module '{}': {}", module_name, e),
                Some("Check the syntax of the module file".to_string()),
                None
            ))?;

        // Extract exports
        let exports = self.extract_exports(&program);

        // Create module
        let module = Module {
            name: module_name.to_string(),
            file_path: module_path,
            program,
            exports,
        };

        // Cache the module
        self.module_cache.insert(module_name.to_string(), module.clone());

        Ok(module)
    }

    /// Find a module file in the search paths
    fn find_module_file(&self, module_name: &str) -> Result<PathBuf, CompilerError> {
        let possible_extensions = ["clean", "cl"];
        
        for search_path in &self.module_paths {
            for extension in &possible_extensions {
                let file_path = search_path.join(format!("{}.{}", module_name, extension));
                if file_path.exists() {
                    return Ok(file_path);
                }
            }
        }

        let search_paths_str = self.module_paths.iter()
            .map(|p| p.to_string_lossy())
            .collect::<Vec<_>>()
            .join(", ");

        Err(CompilerError::module_error(
            format!("Module '{}' not found in search paths", module_name),
            Some(format!("Search paths: {}", search_paths_str)),
            None
        ))
    }

    /// Extract exported symbols from a program
    fn extract_exports(&self, program: &Program) -> ModuleExports {
        let mut exports = ModuleExports {
            functions: HashMap::new(),
            classes: HashMap::new(),
            types: HashMap::new(),
        };

        // Export public functions
        for function in &program.functions {
            if function.visibility == Visibility::Public {
                exports.functions.insert(function.name.clone(), function.clone());
            }
        }

        // Export public classes
        for class in &program.classes {
            // Note: Classes don't have visibility field in the current AST
            // We'll export all classes for now
            exports.classes.insert(class.name.clone(), class.clone());
            
            // Also export the class as a type
            exports.types.insert(
                class.name.clone(), 
                Type::Object(class.name.clone())
            );
        }

        exports
    }

    /// Get all loaded modules
    pub fn get_loaded_modules(&self) -> &HashMap<String, Module> {
        &self.module_cache
    }

    /// Clear the module cache
    pub fn clear_cache(&mut self) {
        self.module_cache.clear();
    }
}

impl ModuleExports {
    /// Check if a function is exported
    pub fn has_function(&self, name: &str) -> bool {
        self.functions.contains_key(name)
    }

    /// Check if a class is exported
    pub fn has_class(&self, name: &str) -> bool {
        self.classes.contains_key(name)
    }

    /// Get an exported function
    pub fn get_function(&self, name: &str) -> Option<&Function> {
        self.functions.get(name)
    }

    /// Get an exported class
    pub fn get_class(&self, name: &str) -> Option<&Class> {
        self.classes.get(name)
    }

    /// Get all exported function names
    pub fn function_names(&self) -> Vec<&String> {
        self.functions.keys().collect()
    }

    /// Get all exported class names
    pub fn class_names(&self) -> Vec<&String> {
        self.classes.keys().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_module_resolver_creation() {
        let resolver = ModuleResolver::new();
        assert_eq!(resolver.module_cache.len(), 0);
        assert!(resolver.module_paths.len() > 0);
    }

    #[test]
    fn test_add_module_path() {
        let mut resolver = ModuleResolver::new();
        let initial_paths = resolver.module_paths.len();
        
        resolver.add_module_path("/custom/path");
        assert_eq!(resolver.module_paths.len(), initial_paths + 1);
    }

    #[test]
    fn test_module_file_search() {
        let dir = tempdir().unwrap();
        let module_path = dir.path().join("TestModule.clean");
        fs::write(&module_path, "function test() -> void\n\tprint(\"test\")").unwrap();

        let mut resolver = ModuleResolver::new();
        resolver.add_module_path(dir.path());

        let found_path = resolver.find_module_file("TestModule").unwrap();
        assert_eq!(found_path, module_path);
    }

    #[test]
    fn test_module_not_found() {
        let resolver = ModuleResolver::new();
        let result = resolver.find_module_file("NonExistentModule");
        assert!(result.is_err());
    }
} 