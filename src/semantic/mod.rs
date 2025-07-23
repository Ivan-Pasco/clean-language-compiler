use std::collections::{HashMap, HashSet};
use crate::ast::*;
use crate::error::{CompilerError, CompilerWarning, WarningType};
use crate::module::{ModuleResolver, ImportResolution};

mod scope;
use scope::Scope;

pub struct SemanticAnalyzer {
    #[allow(dead_code)]
    symbol_table: HashMap<String, Type>,
    function_table: HashMap<String, Vec<(Vec<Type>, Type, usize)>>, // Multiple overloads per function name
    class_table: HashMap<String, Class>,
    current_class: Option<String>,
    current_function: Option<String>,
    current_constructor: bool, // Track if we're in a constructor
    loop_depth: i32,
    type_environment: HashSet<String>,
    variable_environment: HashSet<String>,
    function_environment: HashSet<String>,
    #[allow(dead_code)]
    class_environment: HashSet<String>,
    current_scope: Scope,
    current_function_return_type: Option<Type>,
    warnings: Vec<CompilerWarning>,
    used_variables: HashSet<String>,
    used_functions: HashSet<String>,
    #[allow(dead_code)]
    error_context_depth: i32,
    module_resolver: ModuleResolver,
    current_imports: Option<ImportResolution>,
    #[allow(dead_code)]
    scope_stack: Vec<Scope>,
    #[allow(dead_code)]
    errors: Vec<CompilerError>,
    #[allow(dead_code)]
    imported_modules: HashSet<String>,
}

impl Default for SemanticAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        let mut analyzer = Self {
            symbol_table: HashMap::new(),
            function_table: HashMap::new(),
            class_table: HashMap::new(),
            current_class: None,
            current_function: None,
            current_constructor: false,
            loop_depth: 0,
            type_environment: HashSet::new(),
            variable_environment: HashSet::new(),
            function_environment: HashSet::new(),
            class_environment: HashSet::new(),
            current_scope: Scope::new(),
            current_function_return_type: None,
            warnings: Vec::new(),
            used_variables: HashSet::new(),
            used_functions: HashSet::new(),
            error_context_depth: 0,
            module_resolver: ModuleResolver::new(),
            current_imports: None,
            scope_stack: Vec::new(),
            errors: Vec::new(),
            imported_modules: HashSet::new(),
        };
        
        analyzer.register_builtin_functions();
        analyzer
    }

    /// Helper function to register a builtin function
    fn register_builtin(&mut self, name: &str, params: Vec<Type>, return_type: Type) {
        let param_count = params.len();
        let overload = (params, return_type, param_count);
        
        // Add to existing overloads or create new entry
        self.function_table.entry(name.to_string())
            .or_insert_with(Vec::new)
            .push(overload);
    }

    /// Register built-in functions that are available in the global scope
    fn register_builtin_functions(&mut self) {
        // Register standard library functions
        self.register_builtin("print", vec![Type::String], Type::Void);
        self.register_builtin("println", vec![Type::String], Type::Void);
        self.register_builtin("printl", vec![Type::String], Type::Void);

        // Assertion functions (keep as traditional functions)
        self.register_builtin("mustBeTrue", vec![Type::Boolean], Type::Void);
        self.register_builtin("mustBeFalse", vec![Type::Boolean], Type::Void);

        self.function_table.insert(
            "mustBeEqual".to_string(),
            vec![(vec![Type::Any, Type::Any], Type::Void, 2)]
        );

        // List and string operations (removed - now only available as methods)
        // length, isEmpty, isNotEmpty, isDefined, isNotDefined, keepBetween
        // are now ONLY available as method-style calls

        // Math functions - module.function() syntax
        self.function_table.insert(
            "math.abs".to_string(),
            vec![(vec![Type::Integer], Type::Integer, 1), (vec![Type::Number], Type::Number, 1)]
        );
        self.function_table.insert(
            "math.sqrt".to_string(),
            vec![(vec![Type::Number], Type::Number, 1)]
        );
        self.function_table.insert(
            "math.pow".to_string(),
            vec![(vec![Type::Number, Type::Number], Type::Number, 2)]
        );
        self.function_table.insert(
            "math.sin".to_string(),
            vec![(vec![Type::Number], Type::Number, 1)]
        );
        self.function_table.insert(
            "math.cos".to_string(),
            vec![(vec![Type::Number], Type::Number, 1)]
        );
        self.function_table.insert(
            "math.tan".to_string(),
            vec![(vec![Type::Number], Type::Number, 1)]
        );

        // Console functions - accessed directly without module prefix
        self.function_table.insert(
            "print".to_string(),
            vec![(vec![Type::String], Type::Void, 1)]
        );
        self.function_table.insert(
            "println".to_string(),
            vec![(vec![Type::String], Type::Void, 1)]
        );
        self.function_table.insert(
            "printl".to_string(),
            vec![(vec![Type::String], Type::Void, 1)]
        );
        self.function_table.insert(
            "input".to_string(),
            vec![(vec![Type::String], Type::String, 1)]
        );

        // Additional mathematical functions - module.function() syntax
        self.function_table.insert(
            "math.ln".to_string(),
            vec![(vec![Type::Number], Type::Number, 1)]
        );

        self.function_table.insert(
            "math.log10".to_string(),
            vec![(vec![Type::Number], Type::Number, 1)]
        );

        self.function_table.insert(
            "math.log2".to_string(),
            vec![(vec![Type::Number], Type::Number, 1)]
        );

        self.function_table.insert(
            "math.exp".to_string(),
            vec![(vec![Type::Number], Type::Number, 1)]
        );

        self.function_table.insert(
            "math.exp2".to_string(),
            vec![(vec![Type::Number], Type::Number, 1)]
        );

        self.function_table.insert(
            "math.sinh".to_string(),
            vec![(vec![Type::Number], Type::Number, 1)]
        );

        self.function_table.insert(
            "math.cosh".to_string(),
            vec![(vec![Type::Number], Type::Number, 1)]
        );

        self.function_table.insert(
            "math.tanh".to_string(),
            vec![(vec![Type::Number], Type::Number, 1)]
        );

        self.function_table.insert(
            "math.asin".to_string(),
            vec![(vec![Type::Number], Type::Number, 1)]
        );

        self.function_table.insert(
            "math.acos".to_string(),
            vec![(vec![Type::Number], Type::Number, 1)]
        );

        self.function_table.insert(
            "math.atan".to_string(),
            vec![(vec![Type::Number], Type::Number, 1)]
        );

        self.function_table.insert(
            "math.atan2".to_string(),
            vec![(vec![Type::Number, Type::Number], Type::Number, 2)]
        );

        self.function_table.insert(
            "math.pi".to_string(),
            vec![(vec![], Type::Number, 0)]
        );

        self.function_table.insert(
            "math.e".to_string(),
            vec![(vec![], Type::Number, 0)]
        );

        self.function_table.insert(
            "math.floor".to_string(),
            vec![(vec![Type::Number], Type::Number, 1)]
        );

        self.function_table.insert(
            "math.ceil".to_string(),
            vec![(vec![Type::Number], Type::Number, 1)]
        );

        self.function_table.insert(
            "math.round".to_string(),
            vec![(vec![Type::Number], Type::Number, 1)]
        );

        self.function_table.insert(
            "math.min".to_string(),
            vec![(vec![Type::Number, Type::Number], Type::Number, 2)]
        );

        self.function_table.insert(
            "math.max".to_string(),
            vec![(vec![Type::Number, Type::Number], Type::Number, 2)]
        );

        self.function_table.insert(
            "math.mod".to_string(),
            vec![(vec![Type::Number, Type::Number], Type::Number, 2)]
        );

        // Type conversion functions
        self.function_table.insert(
            "float_to_string".to_string(),
            vec![(vec![Type::Number], Type::String, 1)]
        );
        
        // Add type conversion functions from stdlib
        self.function_table.insert(
            "to_string".to_string(),
            vec![(vec![Type::Integer], Type::String, 1)]
        );
        
        self.function_table.insert(
            "int_to_string".to_string(),
            vec![(vec![Type::Integer], Type::String, 1)]
        );
        
        self.function_table.insert(
            "number_to_string".to_string(),
            vec![(vec![Type::Number], Type::String, 1)]
        );
        
        self.function_table.insert(
            "bool_to_string".to_string(),
            vec![(vec![Type::Boolean], Type::String, 1)]
        );
        
        self.function_table.insert(
            "to_number".to_string(),
            vec![(vec![Type::String], Type::Number, 1)]
        );
        
        self.function_table.insert(
            "to_integer".to_string(),
            vec![(vec![Type::Number], Type::Integer, 1)]
        );
        
        self.function_table.insert(
            "string_to_int".to_string(),
            vec![(vec![Type::String], Type::Integer, 1)]
        );
        
        self.function_table.insert(
            "string_to_float".to_string(),
            vec![(vec![Type::String], Type::Number, 1)]
        );
        
        self.function_table.insert(
            "float_to_int".to_string(),
            vec![(vec![Type::Number], Type::Integer, 1)]
        );
        
        self.function_table.insert(
            "int_to_float".to_string(),
            vec![(vec![Type::Integer], Type::Number, 1)]
        );
        
        // Add commonly used math functions available directly (without math. prefix)
        self.function_table.insert(
            "abs".to_string(),
            vec![(vec![Type::Integer], Type::Integer, 1), (vec![Type::Number], Type::Number, 1)]
        );
        
        self.function_table.insert(
            "sqrt".to_string(),
            vec![(vec![Type::Number], Type::Number, 1)]
        );
        
        self.function_table.insert(
            "pow".to_string(),
            vec![(vec![Type::Number, Type::Number], Type::Number, 2)]
        );
        
        self.function_table.insert(
            "sin".to_string(),
            vec![(vec![Type::Number], Type::Number, 1)]
        );
        
        self.function_table.insert(
            "cos".to_string(),
            vec![(vec![Type::Number], Type::Number, 1)]
        );
        
        self.function_table.insert(
            "tan".to_string(),
            vec![(vec![Type::Number], Type::Number, 1)]
        );
        
        self.function_table.insert(
            "floor".to_string(),
            vec![(vec![Type::Number], Type::Number, 1)]
        );
        
        self.function_table.insert(
            "ceil".to_string(),
            vec![(vec![Type::Number], Type::Number, 1)]
        );
        
        self.function_table.insert(
            "round".to_string(),
            vec![(vec![Type::Number], Type::Number, 1)]
        );

        // Console input functions - accessed directly without module prefix
        self.function_table.insert(
            "inputInteger".to_string(),
            vec![(vec![Type::String], Type::Integer, 1)]
        );

        self.function_table.insert(
            "inputFloat".to_string(),
            vec![(vec![Type::String], Type::Number, 1)]
        );

        self.function_table.insert(
            "inputYesNo".to_string(),
            vec![(vec![Type::String], Type::Boolean, 1)]
        );

        // Console class static methods
        self.function_table.insert(
            "Console.inputInteger".to_string(),
            vec![(vec![Type::String], Type::Integer, 1)]
        );

        self.function_table.insert(
            "Console.inputNumber".to_string(),
            vec![(vec![Type::String], Type::Number, 1)]
        );

        self.function_table.insert(
            "Console.inputBoolean".to_string(),
            vec![(vec![Type::String], Type::Boolean, 1)]
        );

        self.function_table.insert(
            "Console.inputYesNo".to_string(),
            vec![(vec![Type::String], Type::Boolean, 1)]
        );

        self.function_table.insert(
            "Console.inputRange".to_string(),
            vec![(vec![Type::String, Type::Integer, Type::Integer], Type::Integer, 3)]
        );

        // String operations - module.function() syntax
        self.function_table.insert(
            "string.concat".to_string(),
            vec![(vec![Type::String, Type::String], Type::String, 2)]
        );

        self.function_table.insert(
            "string.compare".to_string(),
            vec![(vec![Type::String, Type::String], Type::Integer, 2)]
        );

        self.function_table.insert(
            "string.indexOf".to_string(),
            vec![(vec![Type::String, Type::String], Type::Integer, 2)]
        );

        self.function_table.insert(
            "string.lastIndexOf".to_string(),
            vec![(vec![Type::String, Type::String], Type::Integer, 2)]
        );

        self.function_table.insert(
            "string.startsWith".to_string(),
            vec![(vec![Type::String, Type::String], Type::Boolean, 2)]
        );

        self.function_table.insert(
            "string.endsWith".to_string(),
            vec![(vec![Type::String, Type::String], Type::Boolean, 2)]
        );

        self.function_table.insert(
            "string.toUpperCase".to_string(),
            vec![(vec![Type::String], Type::String, 1)]
        );

        self.function_table.insert(
            "string.toLowerCase".to_string(),
            vec![(vec![Type::String], Type::String, 1)]
        );

        // Add missing string functions
        self.function_table.insert(
            "string.length".to_string(),
            vec![(vec![Type::String], Type::Integer, 1)]
        );

        self.function_table.insert(
            "string.replace".to_string(),
            vec![(vec![Type::String, Type::String, Type::String], Type::String, 3)]
        );

        self.function_table.insert(
            "string.replaceAll".to_string(),
            vec![(vec![Type::String, Type::String, Type::String], Type::String, 3)]
        );

        self.function_table.insert(
            "string.trim".to_string(),
            vec![(vec![Type::String], Type::String, 1)]
        );

        self.function_table.insert(
            "string.split".to_string(),
            vec![(vec![Type::String, Type::String], Type::List(Box::new(Type::String)), 2)]
        );

        // List operations - module.function() syntax
        self.function_table.insert(
            "array.get".to_string(),
            vec![(vec![Type::List(Box::new(Type::Any)), Type::Integer], Type::Any, 2)]
        );

        self.function_table.insert(
            "array.length".to_string(),
            vec![(vec![Type::List(Box::new(Type::Any))], Type::Integer, 1)]
        );

        self.function_table.insert(
            "array.join".to_string(),
            vec![(vec![Type::List(Box::new(Type::Any)), Type::String], Type::String, 2)]
        );

        self.function_table.insert(
            "array.push".to_string(),
            vec![(vec![Type::List(Box::new(Type::Any)), Type::Any], Type::Integer, 2)]
        );

        self.function_table.insert(
            "array.pop".to_string(),
            vec![(vec![Type::List(Box::new(Type::Any))], Type::Any, 1)]
        );

        self.function_table.insert(
            "array.slice".to_string(),
            vec![(vec![Type::List(Box::new(Type::Any)), Type::Integer, Type::Integer], Type::List(Box::new(Type::Any)), 3)]
        );

        self.function_table.insert(
            "array.concat".to_string(),
            vec![(vec![Type::List(Box::new(Type::Any)), Type::List(Box::new(Type::Any))], Type::List(Box::new(Type::Any)), 2)]
        );

        self.function_table.insert(
            "array.reverse".to_string(),
            vec![(vec![Type::List(Box::new(Type::Any))], Type::List(Box::new(Type::Any)), 1)]
        );

        self.function_table.insert(
            "array.contains".to_string(),
            vec![(vec![Type::List(Box::new(Type::Any)), Type::Any], Type::Boolean, 2)]
        );

        self.function_table.insert(
            "array.indexOf".to_string(),
            vec![(vec![Type::List(Box::new(Type::Any)), Type::Any], Type::Integer, 2)]
        );

        self.function_table.insert(
            "array.map".to_string(),
            vec![(vec![Type::List(Box::new(Type::Any)), Type::Function(vec![Type::Any], Box::new(Type::Any))], Type::List(Box::new(Type::Any)), 2)]
        );

        self.function_table.insert(
            "array.iterate".to_string(),
            vec![(vec![Type::List(Box::new(Type::Any)), Type::Function(vec![Type::Any], Box::new(Type::Void))], Type::Void, 2)]
        );

        // HTTP functionality - module.function() syntax
        self.function_table.insert(
            "http.get".to_string(),
            vec![(vec![Type::String], Type::String, 1)]
        );
        
        self.function_table.insert(
            "http.post".to_string(),
            vec![(vec![Type::String, Type::String], Type::String, 2)]
        );
        
        self.function_table.insert(
            "http.put".to_string(),
            vec![(vec![Type::String, Type::String], Type::String, 2)]
        );
        
        self.function_table.insert(
            "http.delete".to_string(),
            vec![(vec![Type::String], Type::String, 1)]
        );
        
        self.function_table.insert(
            "http.patch".to_string(),
            vec![(vec![Type::String, Type::String], Type::String, 2)]
        );

        // Additional HTTP methods
        self.function_table.insert(
            "http.head".to_string(),
            vec![(vec![Type::String], Type::String, 1)]
        );
        
        self.function_table.insert(
            "http.options".to_string(),
            vec![(vec![Type::String], Type::String, 1)]
        );
        
        self.function_table.insert(
            "http.postJson".to_string(),
            vec![(vec![Type::String, Type::String], Type::String, 2)]
        );
        
        self.function_table.insert(
            "http.putJson".to_string(),
            vec![(vec![Type::String, Type::String], Type::String, 2)]
        );
        
        self.function_table.insert(
            "http.patchJson".to_string(),
            vec![(vec![Type::String, Type::String], Type::String, 2)]
        );

        // HTTP configuration functions
        self.function_table.insert(
            "http.setTimeout".to_string(),
            vec![(vec![Type::Integer], Type::Void, 1)]
        );
        
        self.function_table.insert(
            "http.setUserAgent".to_string(),
            vec![(vec![Type::String], Type::Void, 1)]
        );
        
        self.function_table.insert(
            "http.enableCookies".to_string(),
            vec![(vec![Type::Boolean], Type::Void, 1)]
        );

        // HTTP response functions
        self.function_table.insert(
            "http.getResponseCode".to_string(),
            vec![(vec![], Type::Integer, 0)]
        );
        
        self.function_table.insert(
            "http.getResponseHeaders".to_string(),
            vec![(vec![], Type::String, 0)]
        );

        // HTTP utility functions
        self.function_table.insert(
            "http.encodeUrl".to_string(),
            vec![(vec![Type::String], Type::String, 1)]
        );
        
        self.function_table.insert(
            "http.decodeUrl".to_string(),
            vec![(vec![Type::String], Type::String, 1)]
        );

        // File I/O functionality - module.function() syntax
        self.function_table.insert(
            "file.read".to_string(),
            vec![(vec![Type::String], Type::String, 1)]
        );
        
        self.function_table.insert(
            "file.write".to_string(),
            vec![(vec![Type::String, Type::String], Type::Boolean, 2)]
        );
        
        self.function_table.insert(
            "file.append".to_string(),
            vec![(vec![Type::String, Type::String], Type::Boolean, 2)]
        );
        
        self.function_table.insert(
            "file.exists".to_string(),
            vec![(vec![Type::String], Type::Boolean, 1)]
        );
        
        self.function_table.insert(
            "file.delete".to_string(),
            vec![(vec![Type::String], Type::Boolean, 1)]
        );
    }

    pub fn analyze(&mut self, program: &Program) -> Result<Program, CompilerError> {
        // First, resolve imports if any
        if !program.imports.is_empty() {
            let import_resolution = self.module_resolver.resolve_imports(program)?;
            
            // Add imported symbols to our function and class tables
            for (module_name, module) in &import_resolution.resolved_imports {
                // Add imported functions with qualified names
                for (func_name, function) in &module.exports.functions {
                    let param_types = function.parameters.iter().map(|p| p.type_.clone()).collect();
                    let required_param_count = function.parameters.iter()
                        .take_while(|p| p.default_value.is_none())
                        .count();
                    let qualified_name = format!("{module_name}.{func_name}");
                    println!("DEBUG: Adding function '{qualified_name}' to function table");
                    self.function_table.insert(qualified_name, vec![(param_types, function.return_type.clone(), required_param_count)]);
                }
                
                // Add imported classes with qualified names
                for (class_name, class) in &module.exports.classes {
                    let qualified_name = format!("{module_name}.{class_name}");
                    self.class_table.insert(qualified_name, class.clone());
                }
            }
            
            // Add single symbol imports directly (without qualification)
            for (symbol_name, (module_name, actual_symbol)) in &import_resolution.single_symbols {
                if let Some(module) = import_resolution.resolved_imports.get(module_name) {
                    if let Some(function) = module.exports.functions.get(actual_symbol) {
                        let param_types = function.parameters.iter().map(|p| p.type_.clone()).collect();
                        let required_param_count = function.parameters.iter()
                            .take_while(|p| p.default_value.is_none())
                            .count();
                        self.function_table.insert(symbol_name.clone(), vec![(param_types, function.return_type.clone(), required_param_count)]);
                    }
                    if let Some(class) = module.exports.classes.get(actual_symbol) {
                        self.class_table.insert(symbol_name.clone(), class.clone());
                    }
                }
            }
            
            self.current_imports = Some(import_resolution);
        }

        self.check(program)?;
        Ok(program.clone())
    }

    pub fn check(&mut self, program: &Program) -> Result<(), CompilerError> {
        // First pass: register all classes and functions
        for class in &program.classes {
            self.class_table.insert(class.name.clone(), class.clone());
        }

        for function in &program.functions {
            let param_types = function.parameters.iter().map(|p| p.type_.clone()).collect();
            // Calculate required parameter count (parameters without default values)
            let required_param_count = function.parameters.iter()
                .take_while(|p| p.default_value.is_none())
                .count();
            // Don't overwrite builtin functions like print, printl, etc.
            if !self.is_builtin_function(&function.name) {
                self.function_table.insert(
                    function.name.clone(),
                    vec![(param_types, function.return_type.clone(), required_param_count)]
                );
            }
        }

        if let Some(start_fn) = &program.start_function {
            let param_types = start_fn.parameters.iter().map(|p| p.type_.clone()).collect();
            // Calculate required parameter count (parameters without default values)
            let required_param_count = start_fn.parameters.iter()
                .take_while(|p| p.default_value.is_none())
                .count();
            // Don't overwrite builtin functions like print, printl, etc.
            if !self.is_builtin_function(&start_fn.name) {
                self.function_table.insert(
                    start_fn.name.clone(),
                    vec![(param_types, start_fn.return_type.clone(), required_param_count)]
                );
            }
        }

        // Check for inheritance cycles
        self.check_inheritance_cycles()?;

        // Second pass: check all items
        for class in &program.classes {
            self.check_class(class)?;
        }

        for function in &program.functions {
            self.check_function(function)?;
        }

        if let Some(start_fn) = &program.start_function {
            self.check_function(start_fn)?;
        }

        // Third pass: check for unused variables and functions
        self.check_unused_items();

        Ok(())
    }

    fn check_inheritance_cycles(&self) -> Result<(), CompilerError> {
        for class in self.class_table.values() {
            let mut visited = HashSet::new();
            let mut current = Some(class.name.clone());

            while let Some(class_name) = current {
                if visited.contains(&class_name) {
                    return Err(CompilerError::type_error(
                        &format!("Inheritance cycle detected involving class '{class_name}'"),
                        Some("Remove circular inheritance relationships".to_string()),
                        class.location.clone()
                    ));
                }

                visited.insert(class_name.clone());
                current = self.class_table.get(&class_name)
                    .and_then(|c| c.base_class.clone());
            }
        }
        Ok(())
    }

    fn check_class(&mut self, class: &Class) -> Result<(), CompilerError> {
        self.current_class = Some(class.name.clone());

        // Check type parameters
        for type_param in &class.type_parameters {
            self.type_environment.insert(type_param.clone());
        }

        // Check inheritance cycles
        if let Some(_base_class) = &class.base_class {
            self.check_inheritance_cycles()?;
        }

        // Check fields
        for field in &class.fields {
            // Any type is valid for fields
            if matches!(field.type_, Type::Any) {
                continue;
            }
            
            // Check if field type is valid
            if !self.is_valid_type(&field.type_) {
                return Err(CompilerError::type_error(
                    format!("Invalid type for field {}: {}", field.name, field.type_),
                    None,
                    None
                ));
            }
        }

        // Check constructor
        if let Some(constructor) = &class.constructor {
            self.check_constructor(constructor, class)?;
        }

        // Check methods and validate overrides
        for method in &class.methods {
            // Check for method overrides if this class has a base class
            if let Some(base_class_name) = &class.base_class {
                if let Some((parent_method, parent_class_name)) = self.find_method_in_hierarchy(base_class_name, &method.name) {
                    self.check_method_override(method, &parent_method, &class.name, &parent_class_name)?;
                }
            }
            
            // Check method with proper scope setup
            self.check_method(method, class)?;
        }

        // Clear type parameters
        for type_param in &class.type_parameters {
            self.type_environment.remove(type_param);
        }

        self.current_class = None;
        Ok(())
    }

    fn check_constructor(&mut self, constructor: &Constructor, class: &Class) -> Result<(), CompilerError> {
        // Enter constructor scope
        self.current_scope.enter();
        self.current_constructor = true; // Mark that we're in a constructor

        // Add constructor parameters to scope first (they take precedence)
        for param in &constructor.parameters {
            self.check_type(&param.type_)?;
            self.current_scope.define_variable(param.name.clone(), param.type_.clone());
            }

        // Add class fields to scope (accessible in constructor), including inherited fields
        // These will be available as implicit context when not shadowed by parameters
        let hierarchy = self.get_class_hierarchy(&class.name);
        for class_name in hierarchy {
            if let Some(class_def) = self.class_table.get(&class_name) {
                for field in &class_def.fields {
                    // Include public fields from any class in hierarchy, or any field from current class
                    if field.visibility == Visibility::Public || class_name == class.name {
                        // Only add if not already defined (parameters take precedence)
                        if self.current_scope.lookup_variable(&field.name).is_none() {
                            self.current_scope.define_variable(field.name.clone(), field.type_.clone());
                        }
                    }
                }
            }
        }

        // Check constructor body
        for stmt in &constructor.body {
            self.check_statement(stmt)?;
        }

        // Exit constructor scope
        self.current_scope.exit();
        self.current_constructor = false; // Exit constructor context
        Ok(())
    }

    fn check_method(&mut self, method: &Function, class: &Class) -> Result<(), CompilerError> {
        self.current_function = Some(method.name.clone());
        self.current_function_return_type = Some(method.return_type.clone());

        // Enter method scope
        self.current_scope.enter();

        // Add method parameters to scope first (they take precedence)
        for param in &method.parameters {
            self.check_type(&param.type_)?;
            self.current_scope.define_variable(param.name.clone(), param.type_.clone());
        }

        // Add class fields to scope (accessible in methods), including inherited fields
        // These will be available as implicit context when not shadowed by parameters
        let hierarchy = self.get_class_hierarchy(&class.name);
        for class_name in hierarchy {
            if let Some(class_def) = self.class_table.get(&class_name) {
                for field in &class_def.fields {
                    // Include public fields from any class in hierarchy, or any field from current class
                    if field.visibility == Visibility::Public || class_name == class.name {
                        // Only add if not already defined (parameters take precedence)
                        if self.current_scope.lookup_variable(&field.name).is_none() {
                            self.current_scope.define_variable(field.name.clone(), field.type_.clone());
                        }
                    }
                }
            }
        }

        // Check method body
        for stmt in &method.body {
            self.check_statement(stmt)?;
        }

        // Exit method scope
        self.current_scope.exit();

        self.current_function = None;
        self.current_function_return_type = None;
        Ok(())
    }

    fn check_function(&mut self, function: &Function) -> Result<(), CompilerError> {
        self.current_function = Some(function.name.clone());
        self.current_function_return_type = Some(function.return_type.clone());

        // Enter function scope
        self.current_scope.enter();

        // Check type parameters
        for type_param in &function.type_parameters {
            self.type_environment.insert(type_param.clone());
        }
        
        // Check parameters
        for param in &function.parameters {
            self.check_type(&param.type_)?;
            self.current_scope.declare_variable(&param.name, param.type_.clone());
        }

        // Check return type
        self.check_type(&function.return_type)?;
        
        // Check body
        for stmt in &function.body {
            self.check_statement(stmt)?;
        }

        // Check if return type matches the last expression
        if let Some(last_stmt) = function.body.last() {
            match last_stmt {
                Statement::Expression { expr, .. } => {
                    let expr_type = self.check_expression(expr)?;
                    if !self.types_compatible(&expr_type, &function.return_type) {
                        return Err(CompilerError::type_error(
                            &format!("Return type mismatch: expected {:?}, got {:?}", function.return_type, expr_type),
                            Some("Make sure the last expression matches the function's return type".to_string()),
                            Some(self.get_expr_location(expr))
                        ));
                    }
                },
                Statement::Return { value: Some(expr), .. } => {
                    let expr_type = self.check_expression(expr)?;
                    if !self.types_compatible(&expr_type, &function.return_type) {
                        return Err(CompilerError::type_error(
                            &format!("Return type mismatch: expected {:?}, got {:?}", function.return_type, expr_type),
                            Some("Make sure the return expression matches the function's return type".to_string()),
                            Some(self.get_expr_location(expr))
                        ));
                    }
                },
                _ => {}
            }
        }

        // Exit function scope
        self.current_scope.exit();

        self.current_function = None;
        self.current_function_return_type = None;
        Ok(())
    }

    fn check_statement(&mut self, stmt: &Statement) -> Result<(), CompilerError> {
        match stmt {
            Statement::VariableDecl { name, type_, initializer, location } => {
                // Resolve type parameters that might be class names
                let resolved_type = self.resolve_type(type_);
                self.check_type(&resolved_type)?;
                
                if let Some(init_expr) = initializer {
                    let init_type = self.check_expression(init_expr)?;
                    if !self.types_compatible(&resolved_type, &init_type) {
                        return Err(CompilerError::type_error(
                            &format!("Cannot assign {init_type:?} to variable of type {resolved_type:?}"),
                            Some("Change the initializer expression to match the variable type".to_string()),
                            location.clone()
                        ));
                    }
                }
                
                self.current_scope.define_variable(name.clone(), resolved_type);
                Ok(())
            },

            Statement::TypeApplyBlock { type_, assignments, location: _ } => {
                self.check_type(type_)?;
                for assignment in assignments {
                    if let Some(init_expr) = &assignment.initializer {
                                let init_type = self.check_expression(init_expr)?;
                        if !self.types_compatible(type_, &init_type) {
                            return Err(CompilerError::type_error(
                                &format!("Variable '{}' initializer type {:?} doesn't match declared type {:?}", 
                                         assignment.name, init_type, type_),
                                Some("Ensure the initializer matches the declared type".to_string()),
                                None
                            ));
                        }
                    }
                    self.current_scope.define_variable(assignment.name.clone(), type_.clone());
                }
                Ok(())
            },

            Statement::FunctionApplyBlock { function_name, expressions, location: _ } => {
                // Check that the function exists and validate signature
                if let Some(overloads) = self.function_table.get(function_name).cloned() {
                    // For apply blocks, use the first overload for simplicity
                    if let Some((param_types, _return_type, required_param_count)) = overloads.first() {
                    // Check argument count
                    if expressions.len() != *required_param_count {
                        return Err(CompilerError::type_error(
                            &format!("Function '{}' expects {} arguments, but {} provided", 
                                   function_name, required_param_count, expressions.len()),
                            Some("Check the function signature and provide the correct number of arguments".to_string()),
                            None
                        ));
                    }
                    
                    // Check argument types
                    for (i, expr) in expressions.iter().enumerate() {
                        let expr_type = self.check_expression(expr)?;
                        if i < param_types.len() && !self.types_compatible(&param_types[i], &expr_type) {
                            return Err(CompilerError::type_error(
                                &format!("Function '{}' parameter {} expects type {:?}, but got {:?}", 
                                       function_name, i + 1, param_types[i], expr_type),
                                Some("Check the function signature and ensure argument types match".to_string()),
                                None
                            ));
                        }
                    }
                    } // Close the first() check
                } else if !self.is_builtin_function(function_name) {
                    return Err(CompilerError::type_error(
                        &format!("Function '{function_name}' not found"),
                        Some("Check if the function name is correct and the function is declared".to_string()),
                        None
                    ));
                } else {
                    // For builtin functions, just check expressions are valid
                    for expr in expressions {
                        self.check_expression(expr)?;
                    }
                }
                Ok(())
            },

            Statement::MethodApplyBlock { object_name, method_chain, expressions, location: _ } => {
                // Check that the object exists and get its type
                let object_type = if let Some(var_type) = self.current_scope.lookup_variable(object_name) {
                    var_type
                } else {
                    return Err(CompilerError::type_error(
                        &format!("Object '{object_name}' not found"),
                        Some("Check if the object name is correct and the object is declared".to_string()),
                        None
                    ));
                };
                
                if method_chain.is_empty() {
                    return Err(CompilerError::type_error(
                        "Method apply block requires at least one method".to_string(),
                        Some("Use the format: object.method: arguments".to_string()),
                        None
                    ));
                }
                
                // Enhanced method validation - check if methods exist on the object type
                for method_name in method_chain {
                    // For built-in types, validate against known methods
                    match &object_type {
                        Type::String => {
                            let valid_string_methods = ["length", "isEmpty", "contains", "startsWith", "endsWith", "toUpper", "toLower"];
                            if !valid_string_methods.contains(&method_name.as_str()) {
                                return Err(CompilerError::type_error(
                                    &format!("Method '{method_name}' not found on String type"),
                                    Some("Valid String methods: length, isEmpty, contains, startsWith, endsWith, toUpper, toLower".to_string()),
                                    None
                                ));
                            }
                        },
                        Type::List(_) => {
                            let valid_array_methods = ["length", "isEmpty", "push", "pop", "get", "set"];
                            if !valid_array_methods.contains(&method_name.as_str()) {
                                return Err(CompilerError::type_error(
                                    &format!("Method '{method_name}' not found on List type"),
                                    Some("Valid List methods: length, isEmpty, push, pop, get, set".to_string()),
                                    None
                                ));
                            }
                        },
                        Type::Object(class_name) => {
                            // For user-defined classes, check if class has the method
                            if let Some(class_def) = self.class_table.get(class_name) {
                                let has_method = class_def.methods.iter().any(|m| &m.name == method_name);
                                if !has_method {
                                    return Err(CompilerError::type_error(
                                        &format!("Method '{method_name}' not found on class '{class_name}'"),
                                        Some("Check the class definition for available methods".to_string()),
                                        None
                                    ));
                                }
                            }
                        },
                        _ => {
                            // For other types, we'll allow the method call but issue a warning
                            self.warnings.push(CompilerWarning::new(
                                &format!("Cannot verify method '{}' on type {:?}", method_name, object_type),
                                WarningType::TypeInference,
                                None
                            ));
                        }
                    }
                }
                
                // Check all expressions
                for expr in expressions {
                    self.check_expression(expr)?;
                }
                Ok(())
            },

            Statement::ConstantApplyBlock { constants, location: _ } => {
                for constant in constants {
                    self.check_type(&constant.type_)?;
                    let value_type = self.check_expression(&constant.value)?;
                    if !self.types_compatible(&constant.type_, &value_type) {
                        return Err(CompilerError::type_error(
                            &format!("Constant '{}' value type {:?} doesn't match declared type {:?}", 
                                     constant.name, value_type, constant.type_),
                            Some("Ensure the constant value matches the declared type".to_string()),
                            None
                        ));
                    }
                    self.current_scope.define_variable(constant.name.clone(), constant.type_.clone());
                }
                Ok(())
            },

            Statement::Assignment { target, value, location } => {
                let value_type = self.check_expression(value)?;
                
                if let Some(var_type) = self.current_scope.lookup_variable(target) {
                    if !self.types_compatible(&var_type, &value_type) {
                        return Err(CompilerError::type_error(
                            &format!("Cannot assign {:?} to variable of type {:?}", value_type, var_type),
                            Some("Ensure the assignment value matches the variable type".to_string()),
                            location.clone()
                        ));
                    }
                    self.used_variables.insert(target.clone());
                Ok(())
                } else {
                    Err(CompilerError::type_error(
                        &format!("Variable '{}' not found", target),
                        Some("Check if the variable name is correct and the variable is declared".to_string()),
                        location.clone()
                    ))
                }
            },

            Statement::Print { expression, newline: _, location: _ } => {
                self.check_expression(expression)?;
                Ok(())
            },
            
            Statement::PrintBlock { expressions, newline: _, location: _ } => {
                for expression in expressions {
                    self.check_expression(expression)?;
                }
                Ok(())
            },

            Statement::Return { value, location } => {
                if let Some(return_type) = &self.current_function_return_type {
                if let Some(expr) = value {
                        let return_type_clone = return_type.clone();
                        let expr_type = self.check_expression(expr)?;
                        if !self.types_compatible(&return_type_clone, &expr_type) {
                            return Err(CompilerError::type_error(
                                &format!("Return type {:?} doesn't match expected return type {:?}", expr_type, return_type_clone),
                                Some("Ensure the return value matches the function's return type".to_string()),
                                location.clone()
                            ));
                        }
                    } else if *return_type != Type::Void {
                        return Err(CompilerError::type_error(
                            &format!("Function expects return type {:?}, but no value returned", return_type),
                            Some("Return a value of the expected type".to_string()),
                            location.clone()
                        ));
                    }
                } else {
                    return Err(CompilerError::type_error(
                        "Return statement outside of function".to_string(),
                        Some("Return statements can only be used inside functions".to_string()),
                        location.clone()
                    ));
                }
                Ok(())
            },

            Statement::If { condition, then_branch, else_branch, location: _ } => {
                let condition_type = self.check_expression(condition)?;
                if condition_type != Type::Boolean {
                    return Err(CompilerError::type_error(
                        &format!("If condition must be boolean, found {:?}", condition_type),
                        Some("Use a boolean expression in the if condition".to_string()),
                        None
                    ));
                }

                self.current_scope.enter();
                for stmt in then_branch {
                    self.check_statement(stmt)?;
                }
                self.current_scope.exit();

                if let Some(else_stmts) = else_branch {
                    self.current_scope.enter();
                    for stmt in else_stmts {
                        self.check_statement(stmt)?;
                    }
                    self.current_scope.exit();
                }

                Ok(())
            },

            Statement::Iterate { iterator, collection, body, location: _ } => {
                let collection_type = self.check_expression(collection)?;
                
                let element_type = match collection_type {
                    Type::List(element_type) => *element_type,
                    Type::String => Type::String, // Iterating over characters
                    _ => return Err(CompilerError::type_error(
                        &format!("Cannot iterate over type {:?}", collection_type),
                        Some("Use an array, list, or string in iterate statements".to_string()),
                        None
                    ))
                };

                self.current_scope.enter();
                self.current_scope.define_variable(iterator.clone(), element_type);
                self.loop_depth += 1;
                
                for stmt in body {
                    self.check_statement(stmt)?;
                }
                
                self.loop_depth -= 1;
                self.current_scope.exit();
                Ok(())
            },

            Statement::Test { name: _, body, location: _ } => {
                self.current_scope.enter();
                for stmt in body {
                    self.check_statement(stmt)?;
                }
                self.current_scope.exit();
                Ok(())
            },

            Statement::TestsBlock { tests, location: _ } => {
                // Check each test case
                for test in tests {
                    // Check that test expression and expected value have compatible types
                    let test_type = self.check_expression(&test.test_expression)?;
                    let expected_type = self.check_expression(&test.expected_value)?;
                    
                    if !self.types_compatible(&test_type, &expected_type) {
                        return Err(CompilerError::type_error(
                            &format!("Test expression type {:?} doesn't match expected type {:?}", test_type, expected_type),
                            Some("Ensure the test expression and expected value have compatible types".to_string()),
                            test.location.clone()
                        ));
                    }
                }
                Ok(())
            },

            Statement::Expression { expr, location: _ } => {
                self.check_expression(expr)?;
                Ok(())
            },
            
            Statement::Error { message, location: _ } => {
                // Check that the message expression is valid
                // Allow strings, numbers, or any other type for error values
                let message_type = self.check_expression(message)?;
                
                // Accept common error value types: String, Integer, Number
                match message_type {
                    Type::String | Type::Integer | Type::Number | Type::Any => {
                        Ok(())
                    },
                    _ => {
                        Err(CompilerError::enhanced_type_error(
                            "Error value must be a string, number, or convertible type".to_string(),
                            Some("String, Integer, or Number".to_string()),
                            Some(format!("{:?}", message_type)),
                            None,
                            vec![
                                "Use a string literal like \"error message\"".to_string(),
                                "Use a numeric error code like 404 or 500".to_string(),
                                "Use a variable containing a string or number".to_string(),
                            ],
                        ))
                    }
                }
            },
            
            // Module and async statements
            Statement::Import { imports, location } => {
                // Imports are already resolved in the analyze phase
                // Here we just validate that all imports were successfully resolved
                if let Some(ref import_resolution) = self.current_imports {
                    for import_item in imports {
                        // Check if this import was successfully resolved
                        let import_name = import_item.alias.as_ref().unwrap_or(&import_item.name);
                        
                        // For single symbol imports, check if the symbol exists
                        if import_item.name.contains('.') {
                            let (module_name, symbol_name) = import_item.name.split_once('.').unwrap();
                            if let Some(module) = import_resolution.resolved_imports.get(module_name) {
                                if !module.exports.has_function(symbol_name) && !module.exports.has_class(symbol_name) {
                                    return Err(CompilerError::symbol_error(
                                        format!("Symbol '{}' not found in module '{}'", symbol_name, module_name),
                                        symbol_name,
                                        Some(module_name)
                                    ));
                                }
                            } else {
                                return Err(CompilerError::import_error(
                                    format!("Module '{}' not found", module_name),
                                    module_name,
                                    location.clone()
                                ));
                            }
                        } else {
                            // Whole module import - check if module exists
                            if !import_resolution.resolved_imports.contains_key(import_name) {
                                return Err(CompilerError::import_error(
                                    format!("Module '{}' not found", import_name),
                                    import_name,
                                    location.clone()
                                ));
                            }
                        }
                    }
                }
                Ok(())
            },
            
            Statement::LaterAssignment { variable, expression, location: _ } => {
                // later variable = start expression
                let expr_type = self.check_expression(expression)?;
                // Create a Future type wrapper
                let future_type = Type::Future(Box::new(expr_type));
                self.current_scope.define_variable(variable.clone(), future_type);
                Ok(())
            },
            
            Statement::Background { expression, location: _ } => {
                // background expression - fire and forget
                let _expr_type = self.check_expression(expression)?;
                Ok(())
            },
            
            Statement::RangeIterate { .. } => {
                // Range iteration - handled separately
                Ok(())
            },
        }
    }

    fn check_expression(&mut self, expr: &Expression) -> Result<Type, CompilerError> {
        match expr {
            Expression::Literal(value) => Ok(self.check_literal(value)),
            
            Expression::Variable(name) => {
                if let Some(var_type) = self.current_scope.lookup_variable(name) {
                    self.used_variables.insert(name.clone());
                    // Implicit await: if the variable is a Future<T>, return T
                    match var_type {
                        Type::Future(inner_type) => Ok(*inner_type),
                        _ => Ok(var_type)
                    }
                } else if self.is_builtin_class(name) {
                    // Built-in class names are valid "variables" that represent the class itself
                    // This allows static method calls like File.read() to work
                    Ok(Type::Object(name.clone()))
                } else if let Some(ref imports) = self.current_imports {
                    // Check if this is a module name from imports
                    if imports.resolved_imports.contains_key(name) {
                        // Module names are valid "variables" that represent the module itself
                        // This allows method calls like TestModule.add() to work
                        Ok(Type::Object(name.clone()))
                    } else {
                        // Enhanced error with suggestions for similar variable names
                        let available_vars = self.current_scope.get_all_variable_names();
                        let available_var_refs: Vec<&str> = available_vars.iter().map(|s| s.as_str()).collect();
                        let suggestions = crate::error::ErrorUtils::suggest_similar_names(name, &available_var_refs, 3);
                        
                        let mut enhanced_suggestions = suggestions;
                        enhanced_suggestions.push("Check if the variable name is correct and the variable is declared".to_string());
                        enhanced_suggestions.push("Ensure the variable is declared before use".to_string());
                        
                        Err(CompilerError::enhanced_type_error(
                            format!("Variable '{}' not found", name),
                            Some("variable".to_string()),
                            None,
                            None,
                            enhanced_suggestions,
                        ))
                    }
                } else {
                    // Enhanced error with suggestions for similar variable names
                    let available_vars = self.current_scope.get_all_variable_names();
                    let available_var_refs: Vec<&str> = available_vars.iter().map(|s| s.as_str()).collect();
                    let suggestions = crate::error::ErrorUtils::suggest_similar_names(name, &available_var_refs, 3);
                    
                    let mut enhanced_suggestions = suggestions;
                    enhanced_suggestions.push("Check if the variable name is correct and the variable is declared".to_string());
                    enhanced_suggestions.push("Ensure the variable is declared before use".to_string());
                    
                    Err(CompilerError::enhanced_type_error(
                        format!("Variable '{}' not found", name),
                        Some("variable".to_string()),
                        None,
                        None,
                        enhanced_suggestions,
                    ))
                }
            },

            Expression::Binary(left, op, right) => {
                self.check_binary_operation(op, left, right, &None)
            },

            Expression::Unary(op, expr) => {
                let expr_type = self.check_expression(expr)?;
        match op {
                    UnaryOperator::Negate => {
                        if expr_type == Type::Integer || expr_type == Type::Number {
                            Ok(expr_type)
                        } else {
                            Err(CompilerError::type_error(
                                &format!("Cannot negate type {:?}", expr_type),
                                Some("Use numeric types for negation".to_string()),
                                None
                    ))
                }
            },
                    UnaryOperator::Not => {
                        if expr_type == Type::Boolean {
                    Ok(Type::Boolean)
                } else {
                            Err(CompilerError::type_error(
                                &format!("Cannot apply logical NOT to type {:?}", expr_type),
                                Some("Use boolean expressions with NOT operator".to_string()),
                                None
                            ))
                        }
                    }
                }
            },

            Expression::Call(name, args) => {
                // Special case: Check if this is a zero-argument "function call" that should be a variable reference
                // This can happen when a variable is mistakenly parsed as a function call
                if args.is_empty() {
                    if let Some(var_type) = self.current_scope.lookup_variable(name) {
                        self.used_variables.insert(name.clone());
                        // Implicit await: if the variable is a Future<T>, return T
                        return match var_type {
                            Type::Future(inner_type) => Ok(*inner_type),
                            _ => Ok(var_type)
                        };
                    }
                }

                // Special handling for type-safe print functions
                if name == "print" || name == "printl" || name == "println" {
                    return self.check_print_function_call(name, args);
                }

                // Check if this is a built-in type constructor
                if self.is_builtin_type_constructor(name) {
                    return self.check_builtin_type_constructor(name, args);
                }

                // Check if this is actually a constructor call (class name)
                if self.class_table.contains_key(name) {
                    // Convert function call to object creation
                    let location = SourceLocation { line: 0, column: 0, file: "unknown".to_string() };
                    return self.check_constructor_call(name, args, &location);
                }

                // Check if this is a built-in class being called (should be a static method call instead)
                if self.is_builtin_class(name) {
                    return Err(CompilerError::type_error(
                        &format!("Built-in class '{}' cannot be called as a function", name),
                        Some("Use static method syntax like MathUtils.add(a, b) instead".to_string()),
                        None
                    ));
                }

                // Use the proper overload resolution logic
                self.check_function_call(name, args, None)
            },

            Expression::PropertyAccess { object, property, location: _ } => {
                let object_type = self.check_expression(object)?;
                match object_type {
                    Type::Object(class_name) => {
                        if let Some(class) = self.class_table.get(&class_name) {
                            for field in &class.fields {
                                if field.name == *property {
                                    return Ok(field.type_.clone());
                                }
                            }
                            Err(CompilerError::type_error(
                                &format!("Property '{}' not found in class '{}'", property, class_name),
                                Some("Check if the property name is correct".to_string()),
                                None
                            ))
                        } else {
                            Err(CompilerError::type_error(
                                &format!("Class '{}' not found", class_name),
                                Some("Check if the class name is correct".to_string()),
                                None
                            ))
                        }
                    },
                    Type::List(_) => {
                        // Handle List property access (e.g., list.type)
                        match property.as_str() {
                            "type" => Ok(Type::String), // Property access returns current behavior as string
                            _ => Err(CompilerError::type_error(
                                &format!("Property '{}' not found on List type", property),
                                Some("Available properties: type".to_string()),
                                None
                            ))
                        }
                    },
                    _ => Err(CompilerError::type_error(
                        &format!("Cannot access property '{}' on type {:?}", property, object_type),
                        Some("Properties can only be accessed on objects and lists".to_string()),
                        None
                    ))
                }
            },

            Expression::PropertyAssignment { object, property, value, location: _ } => {
                let object_type = self.check_expression(object)?;
                let value_type = self.check_expression(value)?;
                
                match object_type {
                    Type::List(_) => {
                        // Handle List property assignment (e.g., list.type = "line")
                        match property.as_str() {
                            "type" => {
                                if value_type != Type::String {
                                    return Err(CompilerError::type_error(
                                        &format!("List.type property expects string, found {:?}", value_type),
                                        Some("Use string values like \"line\", \"pile\", or \"unique\"".to_string()),
                                        None
                                    ));
                                }
                                Ok(Type::Void) // Assignment returns void
                            },
                            _ => Err(CompilerError::type_error(
                                &format!("Property '{}' cannot be assigned on List type", property),
                                Some("Only 'type' property can be assigned on lists".to_string()),
                                None
                            ))
                        }
                    },
                    Type::Object(class_name) => {
                        // Handle field assignment on user-defined classes
                        if let Some(class) = self.class_table.get(&class_name).cloned() {
                            // Find the field in the class
                            for field in &class.fields {
                                if field.name == *property {
                                    // Check if the assignment value type is compatible with the field type
                                    if !self.types_compatible(&field.type_, &value_type) {
                                        return Err(CompilerError::type_error(
                                            &format!("Cannot assign {:?} to field '{}' of type {:?}", 
                                                value_type, property, field.type_),
                                            Some("Ensure the assignment value matches the field type".to_string()),
                                            None
                                        ));
                                    }
                                    return Ok(Type::Void); // Assignment returns void
                                }
                            }
                            // Field not found
                            Err(CompilerError::type_error(
                                &format!("Field '{}' not found in class '{}'", property, class_name),
                                Some("Check the class definition for available fields".to_string()),
                                None
                            ))
                        } else {
                            Err(CompilerError::type_error(
                                &format!("Class '{}' not found", class_name),
                                Some("Check if the class name is correct and the class is defined".to_string()),
                                None
                            ))
                        }
                    },
                    _ => Err(CompilerError::type_error(
                        &format!("Cannot assign property '{}' on type {:?}", property, object_type),
                        Some("Property assignment is only supported on lists and objects".to_string()),
                        None
                    ))
                }
            },

            Expression::MethodCall { object, method, arguments, location } => {
                // Check for console input method calls
                if let Expression::Variable(var_name) = &**object {
                    if var_name == "input" {
                        return match method.as_str() {
                            "integer" => {
                                if arguments.len() != 1 {
                                    return Err(CompilerError::type_error(
                                        format!("input.integer() expects 1 argument, but {} were provided", arguments.len()),
                                        Some("Provide a prompt string".to_string()),
                                        Some(location.clone())
                                    ));
                                }
                                let arg_type = self.check_expression(&arguments[0])?;
                                if arg_type != Type::String {
                                    return Err(CompilerError::type_error(
                                        format!("input.integer() expects string prompt, got {:?}", arg_type),
                                        Some("Use a string for the prompt".to_string()),
                                        Some(location.clone())
                                    ));
                                }
                                Ok(Type::Integer)
                            },
                            "number" => {
                                if arguments.len() != 1 {
                                    return Err(CompilerError::type_error(
                                        format!("input.number() expects 1 argument, but {} were provided", arguments.len()),
                                        Some("Provide a prompt string".to_string()),
                                        Some(location.clone())
                                    ));
                                }
                                let arg_type = self.check_expression(&arguments[0])?;
                                if arg_type != Type::String {
                                    return Err(CompilerError::type_error(
                                        format!("input.number() expects string prompt, got {:?}", arg_type),
                                        Some("Use a string for the prompt".to_string()),
                                        Some(location.clone())
                                    ));
                                }
                                Ok(Type::Number)
                            },
                            "yesNo" => {
                                if arguments.len() != 1 {
                                    return Err(CompilerError::type_error(
                                        format!("input.yesNo() expects 1 argument, but {} were provided", arguments.len()),
                                        Some("Provide a prompt string".to_string()),
                                        Some(location.clone())
                                    ));
                                }
                                let arg_type = self.check_expression(&arguments[0])?;
                                if arg_type != Type::String {
                                    return Err(CompilerError::type_error(
                                        format!("input.yesNo() expects string prompt, got {:?}", arg_type),
                                        Some("Use a string for the prompt".to_string()),
                                        Some(location.clone())
                                    ));
                                }
                                Ok(Type::Boolean)
                            },
                            _ => Err(CompilerError::type_error(
                                format!("Unknown input method: {}", method),
                                Some("Available methods: integer, number, yesNo".to_string()),
                                Some(location.clone())
                            ))
                        };
                    }
                }

                // Check for built-in module calls
                if let Expression::Variable(module_name) = &**object {
                    match module_name.as_str() {
                        "http" => {
                            let function_name = format!("http.{}", method);
                            return self.check_function_call(&function_name, arguments, Some(location.clone()));
                        },
                        "math" => {
                            let function_name = format!("math.{}", method);
                            return self.check_function_call(&function_name, arguments, Some(location.clone()));
                        },
                        "array" => {
                            let function_name = format!("array.{}", method);
                            return self.check_function_call(&function_name, arguments, Some(location.clone()));
                        },
                        "string" => {
                            let function_name = format!("string.{}", method);
                            return self.check_function_call(&function_name, arguments, Some(location.clone()));
                        },
                        "file" => {
                            let function_name = format!("file.{}", method);
                            return self.check_function_call(&function_name, arguments, Some(location.clone()));
                        },
                        _ => {}
                    }
                }

                // Check if this is a call to an imported module's method
                if let Expression::Variable(module_name) = &**object {
                    if let Some(ref imports) = self.current_imports.clone() {
                        if let Some(module) = imports.resolved_imports.get(module_name) {
                            // Check if the method exists in the imported module
                            if let Some(function) = module.exports.get_function(method) {
                                // Validate argument count and types
                                if arguments.len() != function.parameters.len() {
                                    return Err(CompilerError::type_error(
                                        format!("Function '{}' in module '{}' expects {} arguments, but {} were provided",
                                            method, module_name, function.parameters.len(), arguments.len()),
                                        Some("Check the function signature".to_string()),
                                        Some(location.clone())
                                    ));
                                }

                                // Clone the function info to avoid borrowing issues
                                let function_params = function.parameters.clone();
                                let function_return_type = function.return_type.clone();

                                // Type check arguments
                                for (i, (arg, param)) in arguments.iter().zip(function_params.iter()).enumerate() {
                                    let arg_type = self.check_expression(arg)?;
                                    if !self.types_compatible(&arg_type, &param.type_) {
                                        return Err(CompilerError::type_error(
                                            format!("Argument {} to function '{}' in module '{}' has incorrect type",
                                                i + 1, method, module_name),
                                            Some(format!("Expected {:?}, got {:?}", param.type_, arg_type)),
                                            Some(location.clone())
                                        ));
                                    }
                                }

                                return Ok(function_return_type);
                            } else {
                                return Err(CompilerError::symbol_error(
                                    "Function not found in module",
                                    method,
                                    Some(module_name)
                                ));
                            }
                        } else {
                            // Module not found in imports, but it might be a valid module name
                            // Check if we have a qualified function in the function table
                            let qualified_name = format!("{}.{}", module_name, method);
                            if self.function_table.contains_key(&qualified_name) {
                                return self.check_function_call(&qualified_name, arguments, Some(location.clone()));
                            }
                        }
                    } else {
                        // No imports, but check if we have a qualified function in the function table
                        let qualified_name = format!("{}.{}", module_name, method);
                        if self.function_table.contains_key(&qualified_name) {
                            return self.check_function_call(&qualified_name, arguments, Some(location.clone()));
                        }
                    }
                }

                // Check if this is a module method call (even if module not found in imports)
                if let Expression::Variable(module_name) = &**object {
                    let qualified_name = format!("{}.{}", module_name, method);
                    println!("DEBUG: Checking for function '{}' in function table", qualified_name);
                    if self.function_table.contains_key(&qualified_name) {
                        println!("DEBUG: Found function '{}' in function table", qualified_name);
                        return self.check_function_call(&qualified_name, arguments, Some(location.clone()));
                    } else {
                        println!("DEBUG: Function '{}' not found in function table", qualified_name);
                        // Check if this looks like a module method call but function not found
                        if module_name.chars().next().unwrap_or('a').is_uppercase() {
                            return Err(CompilerError::type_error(
                                format!("Function '{}' not found in module '{}'", method, module_name),
                                Some(format!("Available functions can be checked in the module definition")),
                                Some(location.clone())
                            ));
                        }
                    }
                }

                // Fall back to existing method call analysis
                self.check_method_call(object, method, arguments, location)
            },

            Expression::BaseCall { arguments, location } => {
                // Check if we're in a constructor context
                if !self.current_constructor {
                    return Err(CompilerError::type_error(
                        "Base calls can only be used within a constructor".to_string(),
                        Some("Base calls are only valid in class constructors".to_string()),
                        Some(location.clone())
                    ));
                }

                let current_class_name = self.current_class.as_ref().ok_or_else(|| {
                    CompilerError::type_error(
                        "Base calls can only be used within a class".to_string(),
                        Some("Base calls are only valid in class constructors".to_string()),
                        Some(location.clone())
                    )
                })?;

                let current_class = self.class_table.get(current_class_name).cloned().ok_or_else(|| {
                    CompilerError::type_error(
                        format!("Current class '{}' not found", current_class_name),
                        None,
                        Some(location.clone())
                    )
                })?;

                // Check if this class has a base class
                let base_class_name = current_class.base_class.as_ref().ok_or_else(|| {
                    CompilerError::type_error(
                        format!("Class '{}' has no parent class to call base() on", current_class_name),
                        Some("Remove the base call or add inheritance with 'is ParentClass'".to_string()),
                        Some(location.clone())
                    )
                })?;

                let base_class = self.class_table.get(base_class_name).cloned().ok_or_else(|| {
                    CompilerError::type_error(
                        format!("Base class '{}' not found", base_class_name),
                        None,
                        Some(location.clone())
                    )
                })?;

                // Check if the base class has a constructor
                if let Some(base_constructor) = &base_class.constructor {
                    // Check argument count
                    if arguments.len() != base_constructor.parameters.len() {
                        return Err(CompilerError::type_error(
                            format!("Base call expects {} arguments, but {} were provided", 
                                base_constructor.parameters.len(), arguments.len()),
                            Some("Provide the correct number of arguments for the parent constructor".to_string()),
                            Some(location.clone())
                        ));
                    }

                    // Check argument types
                    for (i, (arg, param)) in arguments.iter().zip(base_constructor.parameters.iter()).enumerate() {
                        let arg_type = self.check_expression(arg)?;
                        if !self.types_compatible(&param.type_, &arg_type) {
                            return Err(CompilerError::type_error(
                                format!("Argument {} has type {:?}, but parent constructor parameter expects {:?}", 
                                    i + 1, arg_type, param.type_),
                                Some("Provide arguments of the correct type for the parent constructor".to_string()),
                                Some(location.clone())
                            ));
                        }
                    }

                    // Base call returns void (it's a statement, not an expression that returns a value)
                    Ok(Type::Void)
                } else {
                    // Base class has no constructor, base() should have no arguments
                    if !arguments.is_empty() {
                        return Err(CompilerError::type_error(
                            format!("Parent class '{}' has no constructor, but base() was called with {} arguments", 
                                base_class_name, arguments.len()),
                            Some("Remove arguments from base() call or add a constructor to the parent class".to_string()),
                            Some(location.clone())
                        ));
                    }

                    Ok(Type::Void)
                }
            }
            
            Expression::StaticMethodCall { class_name, method: _, arguments, location: _ } => {
                // Handle static method calls
                if class_name == "MathUtils" || class_name == "List" || class_name == "File" || class_name == "Http" {
                    // Built-in static methods - validate arguments and return appropriate type
                    for arg in arguments {
                        self.check_expression(arg)?;
                    }
                    Ok(Type::Any) // Simplified return type for now
                } else {
                    Err(CompilerError::type_error(
                        &format!("Unknown static class '{}'", class_name),
                        Some("Check if the class name is correct".to_string()),
                        None
                    ))
                }
            },
            
            Expression::ListAccess(array, index) => {
                let array_type = self.check_expression(array)?;
                let index_type = self.check_expression(index)?;
                
                if index_type != Type::Integer {
                    return Err(CompilerError::type_error(
                        "List index must be an integer".to_string(),
                        Some("Use integer values for array indexing".to_string()),
                        None
                    ));
                }
                
                match array_type {
                    Type::List(element_type) => Ok(*element_type),
                    _ => Err(CompilerError::type_error(
                        "List access can only be used on lists".to_string(),
                        None,
                        None
                    ))
                }
            },
            
            Expression::MatrixAccess(matrix, row, col) => {
                let matrix_type = self.check_expression(matrix)?;
                let row_type = self.check_expression(row)?;
                let col_type = self.check_expression(col)?;
                
                if row_type != Type::Integer || col_type != Type::Integer {
                    return Err(CompilerError::type_error(
                        "Matrix indices must be integers".to_string(),
                        None,
                        None
                    ));
                }
                
                match matrix_type {
                    Type::Matrix(element_type) => Ok(*element_type),
                    _ => Err(CompilerError::type_error(
                        "Matrix access can only be used on matrices".to_string(),
                        None,
                        None
                    ))
                }
            },
            
            Expression::StringInterpolation(_parts) => {
                // String interpolation always returns a string
                Ok(Type::String)
            },
            
            Expression::ObjectCreation { class_name, arguments, location: _ } => {
                // Check if class exists
                if self.class_table.contains_key(class_name) {
                    // Validate constructor arguments
                    for arg in arguments {
                        self.check_expression(arg)?;
                    }
                    Ok(Type::Object(class_name.clone()))
                } else {
                    Err(CompilerError::type_error(
                        &format!("Class '{}' not found", class_name),
                        None,
                        None
                    ))
                }
            },
            
            // Async expressions
            Expression::StartExpression { expression, location: _ } => {
                // start expression returns Future<T> where T is the type of the expression
                let expr_type = self.check_expression(expression)?;
                Ok(Type::Future(Box::new(expr_type)))
            },
            
            
            Expression::OnError { expression, fallback, location: _ } => {
                // OnError expression returns the type of the expression if successful,
                // or the type of the fallback if an error occurs
                let expr_type = self.check_expression(expression)?;
                let fallback_type = self.check_expression(fallback)?;
                
                // Both types should be compatible - for now return the expression type
                if self.types_compatible(&expr_type, &fallback_type) {
                    Ok(expr_type)
                } else {
                    // If types don't match, return the more general type
                    Ok(Type::Any)
                }
            },
            
            Expression::OnErrorBlock { expression, error_handler: _, location: _ } => {
                // OnErrorBlock expression returns the type of the expression
                self.check_expression(expression)
            },
            
            Expression::ErrorVariable { location: _ } => {
                // Error variable contains error information - return String for now
                Ok(Type::String)
            },
            
            Expression::Conditional { condition, then_expr, else_expr, location: _ } => {
                // Check condition is boolean
                let condition_type = self.check_expression(condition)?;
                if condition_type != Type::Boolean {
                    return Err(CompilerError::type_error(
                        format!("Conditional condition must be boolean, got {:?}", condition_type),
                        Some("Use a boolean expression for the condition".to_string()),
                        None
                    ));
                }
                
                // Check both branches have compatible types
                let then_type = self.check_expression(then_expr)?;
                let else_type = self.check_expression(else_expr)?;
                
                if self.types_compatible(&then_type, &else_type) {
                    Ok(then_type)
                } else {
                    // If types don't match exactly, return the more general type
                    Ok(Type::Any)
                }
            },
            
            Expression::LaterAssignment { variable: _, expression, location: _ } => {
                // Later assignment returns the type of the expression being assigned
                self.check_expression(expression)
            },
        }
    }

    fn get_expr_location(&self, expr: &Expression) -> SourceLocation {
        match expr {
            Expression::PropertyAccess { location, .. } |
            Expression::MethodCall { location, .. } |
            Expression::ObjectCreation { location, .. } |
            Expression::OnError { location, .. } => location.clone(),
            _ => SourceLocation::default()
        }
    }

    // Convert ast::SourceLocation to a location we can use
    #[allow(dead_code)]
    fn convert_location(&self, location: &SourceLocation) -> SourceLocation {
        location.clone()
    }

    fn check_constructor_call(&mut self, class_name: &str, args: &[Expression], location: &SourceLocation) -> Result<Type, CompilerError> {
        // Clone class to avoid borrow issues
        let class_opt = self.class_table.get(class_name).cloned();
        
        let class = class_opt.ok_or_else(|| {
            CompilerError::type_error(
                &format!("Class '{}' not found", class_name),
                Some("Check if the class name is correct and the class is defined".to_string()),
                Some(location.clone())
            )
        })?;

        // If no explicit constructor, allow default constructor with no arguments
        if let Some(constructor) = &class.constructor {
            // Explicit constructor defined
            if args.len() != constructor.parameters.len() {
                return Err(CompilerError::type_error(
                    &format!("Constructor for class '{}' expects {} arguments, but {} were provided",
                        class_name, constructor.parameters.len(), args.len()),
                    Some("Provide the correct number of arguments".to_string()),
                    Some(location.clone())
                ));
            }

            // Validate parameter types for explicit constructor
            for (i, (arg, param)) in args.iter().zip(constructor.parameters.iter()).enumerate() {
                let arg_type = self.check_expression(arg)?;
                if !self.types_compatible(&arg_type, &param.type_) {
                    return Err(CompilerError::type_error(
                        &format!("Argument {} to constructor has incorrect type. Expected {:?}, got {:?}",
                            i + 1, param.type_, arg_type),
                        Some("Provide arguments of the correct type".to_string()),
                        Some(location.clone())
                    ));
                }
            }
        } else {
            // Default constructor - must have no arguments
            if !args.is_empty() {
                return Err(CompilerError::type_error(
                    &format!("Class '{}' has no explicit constructor, so it only accepts a default constructor with no arguments. {} arguments were provided.",
                        class_name, args.len()),
                    Some("Either define a constructor in the class or call the constructor with no arguments".to_string()),
                    Some(location.clone())
                ));
            }
        }

        Ok(Type::Object(class_name.to_string()))
    }

    fn check_method_call(&mut self, object: &Expression, method: &str, args: &[Expression], location: &SourceLocation) -> Result<Type, CompilerError> {
        // Check for imported modules first before trying to resolve the object
        if let Expression::Variable(module_name) = object {
            if let Some(ref imports) = self.current_imports.clone() {
                if imports.resolved_imports.contains_key(module_name) {
                    // This is an imported module, check if we have a qualified function
                    let qualified_name = format!("{}.{}", module_name, method);
                    if self.function_table.contains_key(&qualified_name) {
                        return self.check_function_call(&qualified_name, args, Some(location.clone()));
                    }
                }
            }
            
            // Also check if we have a qualified function regardless of imports
            let qualified_name = format!("{}.{}", module_name, method);
            if self.function_table.contains_key(&qualified_name) {
                return self.check_function_call(&qualified_name, args, Some(location.clone()));
            }
        }
        
        let object_type = self.check_expression(object)?;
        
        // Check for built-in method-style functions first
        match (&object_type, method) {
            // Integer methods
            (Type::Integer, "keepBetween") => {
                if args.len() != 2 {
                    return Err(CompilerError::type_error(
                        format!("Method 'keepBetween' expects 2 arguments (min, max), but {} were provided", args.len()),
                        Some("Usage: value.keepBetween(min, max)".to_string()),
                        Some(location.clone())
                    ));
                }
                // Check that both arguments are integers
                for (i, arg) in args.iter().enumerate() {
                    let arg_type = self.check_expression(arg)?;
                    if !self.types_compatible(&Type::Integer, &arg_type) {
                        return Err(CompilerError::type_error(
                            format!("Argument {} to 'keepBetween' must be an integer", i + 1),
                            Some("Provide integer values for min and max".to_string()),
                            Some(location.clone())
                        ));
                    }
                }
                return Ok(Type::Integer);
            },
            
            // Number methods
            (Type::Number, "keepBetween") => {
                if args.len() != 2 {
                    return Err(CompilerError::type_error(
                        format!("Method 'keepBetween' expects 2 arguments (min, max), but {} were provided", args.len()),
                        Some("Usage: value.keepBetween(min, max)".to_string()),
                        Some(location.clone())
                    ));
                }
                // Check that both arguments are floats
                for (i, arg) in args.iter().enumerate() {
                    let arg_type = self.check_expression(arg)?;
                    if !self.types_compatible(&Type::Number, &arg_type) {
                        return Err(CompilerError::type_error(
                            format!("Argument {} to 'keepBetween' must be a float", i + 1),
                            Some("Provide float values for min and max".to_string()),
                            Some(location.clone())
                        ));
                    }
                }
                return Ok(Type::Number);
            },
            
            // String and List methods
            (Type::String | Type::List(_), "length") => {
                if !args.is_empty() {
                    return Err(CompilerError::type_error(
                        "Method 'length' doesn't take any arguments".to_string(),
                        Some("Usage: value.length()".to_string()),
                        Some(location.clone())
                    ));
                }
                return Ok(Type::Integer);
            },
            
            // Generic List methods - handle List<T> syntax parsed as Generic
            (Type::Generic(base_type, _type_args), method_name) => {
                if let Type::Object(class_name) = base_type.as_ref() {
                    if class_name == "List" {
                        // Treat Generic(Object("List"), [T]) as Type::List(T) for method calls
                        match method_name {
                            "length" => {
                                if !args.is_empty() {
                                    return Err(CompilerError::type_error(
                                        "Method 'length' doesn't take any arguments".to_string(),
                                        Some("Usage: list.length()".to_string()),
                                        Some(location.clone())
                                    ));
                                }
                                return Ok(Type::Integer);
                            },
                            "isEmpty" => {
                                if !args.is_empty() {
                                    return Err(CompilerError::type_error(
                                        "Method 'isEmpty' doesn't take any arguments".to_string(),
                                        Some("Usage: list.isEmpty()".to_string()),
                                        Some(location.clone())
                                    ));
                                }
                                return Ok(Type::Boolean);
                            },
                            _ => {
                                return Err(CompilerError::type_error(
                                    &format!("Method '{}' not found for List type", method_name),
                                    Some("Available list methods: length, isEmpty".to_string()),
                                    Some(location.clone())
                                ));
                            }
                        }
                    }
                }
                // If not a List generic, fall through to default handling
                return Err(CompilerError::type_error(
                    &format!("Cannot call method '{}' on type {:?}", method, object_type),
                    Some("Methods can only be called on objects".to_string()),
                    Some(location.clone())
                ));
            },
            
            (Type::String | Type::List(_), "isEmpty") => {
                if !args.is_empty() {
                    return Err(CompilerError::type_error(
                        "Method 'isEmpty' doesn't take any arguments".to_string(),
                        Some("Usage: value.isEmpty()".to_string()),
                        Some(location.clone())
                    ));
                }
                return Ok(Type::Boolean);
            },
            
            (Type::String | Type::List(_), "isNotEmpty") => {
                if !args.is_empty() {
                    return Err(CompilerError::type_error(
                        "Method 'isNotEmpty' doesn't take any arguments".to_string(),
                        Some("Usage: value.isNotEmpty()".to_string()),
                        Some(location.clone())
                    ));
                }
                return Ok(Type::Boolean);
            },
            
            // String-specific methods
            (Type::String, "startsWith") => {
                if args.len() != 1 {
                    return Err(CompilerError::type_error(
                        "Method 'startsWith' expects exactly 1 argument".to_string(),
                        Some("Usage: text.startsWith(prefix)".to_string()),
                        Some(location.clone())
                    ));
                }
                let arg_type = self.check_expression(&args[0])?;
                if !self.types_compatible(&Type::String, &arg_type) {
                    return Err(CompilerError::type_error(
                        "Method 'startsWith' expects a string argument".to_string(),
                        Some("Usage: text.startsWith(\"prefix\")".to_string()),
                        Some(location.clone())
                    ));
                }
                return Ok(Type::Boolean);
            },
            
            (Type::String, "endsWith") => {
                if args.len() != 1 {
                    return Err(CompilerError::type_error(
                        "Method 'endsWith' expects exactly 1 argument".to_string(),
                        Some("Usage: text.endsWith(suffix)".to_string()),
                        Some(location.clone())
                    ));
                }
                let arg_type = self.check_expression(&args[0])?;
                if !self.types_compatible(&Type::String, &arg_type) {
                    return Err(CompilerError::type_error(
                        "Method 'endsWith' expects a string argument".to_string(),
                        Some("Usage: text.endsWith(\"suffix\")".to_string()),
                        Some(location.clone())
                    ));
                }
                return Ok(Type::Boolean);
            },
            
            (Type::String, "indexOf") => {
                if args.len() != 1 {
                    return Err(CompilerError::type_error(
                        "Method 'indexOf' expects exactly 1 argument".to_string(),
                        Some("Usage: text.indexOf(searchString)".to_string()),
                        Some(location.clone())
                    ));
                }
                let arg_type = self.check_expression(&args[0])?;
                if !self.types_compatible(&Type::String, &arg_type) {
                    return Err(CompilerError::type_error(
                        "Method 'indexOf' expects a string argument".to_string(),
                        Some("Usage: text.indexOf(\"search\")".to_string()),
                        Some(location.clone())
                    ));
                }
                return Ok(Type::Integer);
            },
            
            (Type::String, "toLowerCase") => {
                if !args.is_empty() {
                    return Err(CompilerError::type_error(
                        "Method 'toLowerCase' doesn't take any arguments".to_string(),
                        Some("Usage: text.toLowerCase()".to_string()),
                        Some(location.clone())
                    ));
                }
                return Ok(Type::String);
            },
            
            (Type::String, "toUpperCase") => {
                if !args.is_empty() {
                    return Err(CompilerError::type_error(
                        "Method 'toUpperCase' doesn't take any arguments".to_string(),
                        Some("Usage: text.toUpperCase()".to_string()),
                        Some(location.clone())
                    ));
                }
                return Ok(Type::String);
            },
            
            (Type::String, "trim") => {
                if !args.is_empty() {
                    return Err(CompilerError::type_error(
                        "Method 'trim' doesn't take any arguments".to_string(),
                        Some("Usage: text.trim()".to_string()),
                        Some(location.clone())
                    ));
                }
                return Ok(Type::String);
            },
            
            (Type::String, "trimStart") => {
                if !args.is_empty() {
                    return Err(CompilerError::type_error(
                        "Method 'trimStart' doesn't take any arguments".to_string(),
                        Some("Usage: text.trimStart()".to_string()),
                        Some(location.clone())
                    ));
                }
                return Ok(Type::String);
            },
            
            (Type::String, "trimEnd") => {
                if !args.is_empty() {
                    return Err(CompilerError::type_error(
                        "Method 'trimEnd' doesn't take any arguments".to_string(),
                        Some("Usage: text.trimEnd()".to_string()),
                        Some(location.clone())
                    ));
                }
                return Ok(Type::String);
            },
            
            (Type::String, "lastIndexOf") => {
                if args.len() != 1 {
                    return Err(CompilerError::type_error(
                        "Method 'lastIndexOf' expects exactly 1 argument".to_string(),
                        Some("Usage: text.lastIndexOf(searchString)".to_string()),
                        Some(location.clone())
                    ));
                }
                let arg_type = self.check_expression(&args[0])?;
                if !self.types_compatible(&Type::String, &arg_type) {
                    return Err(CompilerError::type_error(
                        "Method 'lastIndexOf' expects a string argument".to_string(),
                        Some("Usage: text.lastIndexOf(\"search\")".to_string()),
                        Some(location.clone())
                    ));
                }
                return Ok(Type::Integer);
            },
            
            (Type::String, "substring") => {
                if args.len() != 1 && args.len() != 2 {
                    return Err(CompilerError::type_error(
                        "Method 'substring' expects 1 or 2 arguments".to_string(),
                        Some("Usage: text.substring(start) or text.substring(start, end)".to_string()),
                        Some(location.clone())
                    ));
                }
                for (i, arg) in args.iter().enumerate() {
                    let arg_type = self.check_expression(arg)?;
                    if !self.types_compatible(&Type::Integer, &arg_type) {
                        return Err(CompilerError::type_error(
                            format!("Argument {} to 'substring' must be an integer", i + 1),
                            Some("Usage: text.substring(0, 5)".to_string()),
                            Some(location.clone())
                        ));
                    }
                }
                return Ok(Type::String);
            },
            
            (Type::String, "replace") => {
                if args.len() != 2 {
                    return Err(CompilerError::type_error(
                        "Method 'replace' expects exactly 2 arguments".to_string(),
                        Some("Usage: text.replace(searchValue, replaceValue)".to_string()),
                        Some(location.clone())
                    ));
                }
                for (i, arg) in args.iter().enumerate() {
                    let arg_type = self.check_expression(arg)?;
                    if !self.types_compatible(&Type::String, &arg_type) {
                        return Err(CompilerError::type_error(
                            format!("Argument {} to 'replace' must be a string", i + 1),
                            Some("Usage: text.replace(\"old\", \"new\")".to_string()),
                            Some(location.clone())
                        ));
                    }
                }
                return Ok(Type::String);
            },
            
            (Type::String, "padStart") => {
                if args.len() != 2 {
                    return Err(CompilerError::type_error(
                        "Method 'padStart' expects exactly 2 arguments".to_string(),
                        Some("Usage: text.padStart(targetLength, padString)".to_string()),
                        Some(location.clone())
                    ));
                }
                let length_type = self.check_expression(&args[0])?;
                if !self.types_compatible(&Type::Integer, &length_type) {
                    return Err(CompilerError::type_error(
                        "First argument to 'padStart' must be an integer (target length)".to_string(),
                        Some("Usage: text.padStart(5, \"0\")".to_string()),
                        Some(location.clone())
                    ));
                }
                let pad_type = self.check_expression(&args[1])?;
                if !self.types_compatible(&Type::String, &pad_type) {
                    return Err(CompilerError::type_error(
                        "Second argument to 'padStart' must be a string (pad string)".to_string(),
                        Some("Usage: text.padStart(5, \"0\")".to_string()),
                        Some(location.clone())
                    ));
                }
                return Ok(Type::String);
            },
            
            // List-specific methods
            (Type::List(_), "join") => {
                if args.len() != 1 {
                    return Err(CompilerError::type_error(
                        "Method 'join' expects exactly 1 argument".to_string(),
                        Some("Usage: array.join(separator)".to_string()),
                        Some(location.clone())
                    ));
                }
                let separator_type = self.check_expression(&args[0])?;
                if !self.types_compatible(&Type::String, &separator_type) {
                    return Err(CompilerError::type_error(
                        "Argument to 'join' must be a string (separator)".to_string(),
                        Some("Usage: array.join(\", \")".to_string()),
                        Some(location.clone())
                    ));
                }
                return Ok(Type::String);
            },
            
            // List behavior methods
            (Type::List(element_type), "add") => {
                if args.len() != 1 {
                    return Err(CompilerError::type_error(
                        "Method 'add' expects exactly 1 argument".to_string(),
                        Some("Usage: list.add(item)".to_string()),
                        Some(location.clone())
                    ));
                }
                let arg_type = self.check_expression(&args[0])?;
                if !self.types_compatible(element_type, &arg_type) {
                    return Err(CompilerError::type_error(
                        &format!("Method 'add' expects argument of type {:?}, found {:?}", element_type, arg_type),
                        Some("Usage: list.add(item)".to_string()),
                        Some(location.clone())
                    ));
                }
                return Ok(Type::Void);
            },
            
            (Type::List(element_type), "remove") => {
                if !args.is_empty() {
                    return Err(CompilerError::type_error(
                        "Method 'remove' doesn't take any arguments".to_string(),
                        Some("Usage: list.remove()".to_string()),
                        Some(location.clone())
                    ));
                }
                return Ok(*element_type.clone());
            },
            
            (Type::List(element_type), "peek") => {
                if !args.is_empty() {
                    return Err(CompilerError::type_error(
                        "Method 'peek' doesn't take any arguments".to_string(),
                        Some("Usage: list.peek()".to_string()),
                        Some(location.clone())
                    ));
                }
                return Ok(*element_type.clone());
            },
            
            (Type::List(element_type), "contains") => {
                if args.len() != 1 {
                    return Err(CompilerError::type_error(
                        "Method 'contains' expects exactly 1 argument".to_string(),
                        Some("Usage: list.contains(item)".to_string()),
                        Some(location.clone())
                    ));
                }
                let arg_type = self.check_expression(&args[0])?;
                if !self.types_compatible(element_type, &arg_type) {
                    return Err(CompilerError::type_error(
                        &format!("Method 'contains' expects argument of type {:?}, found {:?}", element_type, arg_type),
                        Some("Usage: list.contains(item)".to_string()),
                        Some(location.clone())
                    ));
                }
                return Ok(Type::Boolean);
            },
            
            (Type::List(_), "size") => {
                if !args.is_empty() {
                    return Err(CompilerError::type_error(
                        "Method 'size' doesn't take any arguments".to_string(),
                        Some("Usage: list.size()".to_string()),
                        Some(location.clone())
                    ));
                }
                return Ok(Type::Integer);
            },
            
            // Any type methods
            (_, "isDefined") => {
                if !args.is_empty() {
                    return Err(CompilerError::type_error(
                        "Method 'isDefined' doesn't take any arguments".to_string(),
                        Some("Usage: value.isDefined()".to_string()),
                        Some(location.clone())
                    ));
                }
                return Ok(Type::Boolean);
            },
            
            (_, "isNotDefined") => {
                if !args.is_empty() {
                    return Err(CompilerError::type_error(
                        "Method 'isNotDefined' doesn't take any arguments".to_string(),
                        Some("Usage: value.isNotDefined()".to_string()),
                        Some(location.clone())
                    ));
                }
                return Ok(Type::Boolean);
            },
            
            // Type conversion methods - work on any type
            (_, "toInteger") => {
                if !args.is_empty() {
                    return Err(CompilerError::type_error(
                        "Method 'toInteger' doesn't take any arguments".to_string(),
                        Some("Usage: value.toInteger()".to_string()),
                        Some(location.clone())
                    ));
                }
                return Ok(Type::Integer);
            },
            
            (_, "toFloat") => {
                if !args.is_empty() {
                    return Err(CompilerError::type_error(
                        "Method 'toFloat' doesn't take any arguments".to_string(),
                        Some("Usage: value.toFloat()".to_string()),
                        Some(location.clone())
                    ));
                }
                return Ok(Type::Number);
            },
            
            (_, "toString") => {
                if !args.is_empty() {
                    return Err(CompilerError::type_error(
                        "Method 'toString' doesn't take any arguments".to_string(),
                        Some("Usage: value.toString()".to_string()),
                        Some(location.clone())
                    ));
                }
                return Ok(Type::String);
            },
            
            (_, "toBoolean") => {
                if !args.is_empty() {
                    return Err(CompilerError::type_error(
                        "Method 'toBoolean' doesn't take any arguments".to_string(),
                        Some("Usage: value.toBoolean()".to_string()),
                        Some(location.clone())
                    ));
                }
                return Ok(Type::Boolean);
            },
            
            _ => {} // Fall through to class method checking
        }
        
        match &object_type {
            Type::Matrix(element_type) => {
                // Handle Matrix methods
                match method {
                    "transpose" => {
                        if !args.is_empty() {
                            return Err(CompilerError::type_error(
                                "Method 'transpose' doesn't take any arguments".to_string(),
                                Some("Usage: matrix.transpose()".to_string()),
                                Some(location.clone())
                            ));
                        }
                        return Ok(Type::Matrix(element_type.clone()));
                    },
                    "get" => {
                        if args.len() != 2 {
                            return Err(CompilerError::type_error(
                                format!("Method 'get' expects 2 arguments (row, col), but {} were provided", args.len()),
                                Some("Usage: matrix.get(row, col)".to_string()),
                                Some(location.clone())
                            ));
                        }
                        // Check that both arguments are integers
                        for (i, arg) in args.iter().enumerate() {
                            let arg_type = self.check_expression(arg)?;
                            if !self.types_compatible(&Type::Integer, &arg_type) {
                                return Err(CompilerError::type_error(
                                    format!("Argument {} to 'get' must be an integer", i + 1),
                                    Some("Provide integer values for row and col".to_string()),
                                    Some(location.clone())
                                ));
                            }
                        }
                        return Ok((**element_type).clone());
                    },
                    "set" => {
                        if args.len() != 3 {
                            return Err(CompilerError::type_error(
                                format!("Method 'set' expects 3 arguments (row, col, value), but {} were provided", args.len()),
                                Some("Usage: matrix.set(row, col, value)".to_string()),
                                Some(location.clone())
                            ));
                        }
                        // Check argument types: row (int), col (int), value (element_type)
                        let row_type = self.check_expression(&args[0])?;
                        let col_type = self.check_expression(&args[1])?;
                        let value_type = self.check_expression(&args[2])?;
                        
                        if !self.types_compatible(&Type::Integer, &row_type) {
                            return Err(CompilerError::type_error(
                                "First argument to 'set' must be an integer (row)".to_string(),
                                Some("Provide an integer value for row".to_string()),
                                Some(location.clone())
                            ));
                        }
                        if !self.types_compatible(&Type::Integer, &col_type) {
                            return Err(CompilerError::type_error(
                                "Second argument to 'set' must be an integer (col)".to_string(),
                                Some("Provide an integer value for col".to_string()),
                                Some(location.clone())
                            ));
                        }
                        if !self.types_compatible(&(**element_type), &value_type) {
                            return Err(CompilerError::type_error(
                                format!("Third argument to 'set' must be of type {:?}", **element_type),
                                Some("Provide a value of the correct matrix element type".to_string()),
                                Some(location.clone())
                            ));
                        }
                        return Ok(Type::Void);
                    },
                    _ => {
                        return Err(CompilerError::type_error(
                            &format!("Method '{}' not found for Matrix type", method),
                            Some("Available matrix methods: transpose, get, set".to_string()),
                            Some(location.clone())
                        ));
                    }
                }
            },
            Type::Object(class_name) => {
                // Special handling for built-in classes
                if self.is_builtin_class(class_name) {
                    // For built-in classes, we allow any method call and return Type::Any
                    // The actual validation happens at the codegen level
                    for arg in args {
                        self.check_expression(arg)?;
                    }
                    return Ok(Type::Any);
                }
                
                // Look up the class in the class table and clone the needed data
                let class = self.class_table.get(class_name).cloned().ok_or_else(|| {
                    CompilerError::type_error(
                        &format!("Class '{}' not found", class_name),
                        Some("Check if the class name is correct and the class is defined".to_string()),
                        Some(location.clone())
                    )
                })?;

                // Look for the method in the class methods
                for method_def in &class.methods {
                    if method_def.name == method {
                        // Check if the number of arguments matches
                        if args.len() != method_def.parameters.len() {
                            return Err(CompilerError::type_error(
                                &format!("Method '{}' expects {} arguments, but {} were provided",
                                    method, method_def.parameters.len(), args.len()),
                                Some("Provide the correct number of arguments".to_string()),
                                Some(location.clone())
                            ));
                        }

                        // Clone the method parameters to avoid borrowing issues
                        let method_params = method_def.parameters.clone();
                        let method_return_type = method_def.return_type.clone();

                        // Check argument types
                        for (i, (arg, param)) in args.iter().zip(method_params.iter()).enumerate() {
                            let arg_type = self.check_expression(arg)?;
                            if !self.types_compatible(&arg_type, &param.type_) {
                                return Err(CompilerError::type_error(
                                    &format!("Argument {} has incorrect type. Expected {:?}, got {:?}",
                                        i + 1, arg_type, param.type_),
                                    Some("Provide arguments of the correct type".to_string()),
                                    Some(location.clone())
                                ));
                            }
                        }

                        return Ok(method_return_type);
                    }
                }

                // If we reach here, the method was not found in the class
                // Try to find a global function with the same name that can be called as a method
                if let Some(function_signatures) = self.function_table.get(method).cloned() {
                    // Check if any of the function signatures match (considering the object as first parameter)
                    for (param_types, return_type, required_param_count) in function_signatures {
                        // The function should accept the object type as first parameter, plus the method arguments
                        let expected_param_count = 1 + args.len(); // object + arguments
                        if expected_param_count >= required_param_count && expected_param_count <= param_types.len() {
                            // Check if first parameter type is compatible with the object type
                            if let Some(first_param_type) = param_types.get(0) {
                                let object_type_for_param = Type::Object(class_name.clone());
                                if self.types_compatible(&object_type_for_param, first_param_type) || 
                                   first_param_type == &Type::Any {
                                    
                                    // Check the remaining parameter types match the method arguments
                                    let mut types_match = true;
                                    for (i, arg) in args.iter().enumerate() {
                                        if let Some(expected_type) = param_types.get(i + 1) {
                                            let arg_type = self.check_expression(arg)?;
                                            if !self.types_compatible(&arg_type, expected_type) && expected_type != &Type::Any {
                                                types_match = false;
                                                break;
                                            }
                                        }
                                    }
                                    
                                    if types_match {
                                        // Found a matching global function - call it with object as first parameter
                                        return Ok(return_type.clone());
                                    }
                                }
                            }
                        }
                    }
                }
                
                // No matching global function found either
                Err(CompilerError::type_error(
                    &format!("Method '{}' not found in class '{}' or as a global function", method, class_name),
                    Some("Check if the method name is correct and defined in the class hierarchy or as a global function".to_string()),
                    Some(location.clone())
                ))
            }
            _ => {
                Err(CompilerError::type_error(
                    &format!("Cannot call method '{}' on type {:?}", method, object_type),
                    Some("Methods can only be called on objects".to_string()),
                    Some(location.clone())
                ))
            }
        }
    }

    #[allow(dead_code)]
    fn check_type_conversion_method(&mut self, object: &Expression, method: &str, args: &[Expression]) -> Result<Type, CompilerError> {
        // Type conversion methods don't take arguments
        if !args.is_empty() {
            return Err(CompilerError::type_error(
                &format!("Type conversion method '{}' doesn't take arguments", method),
                Some("Remove the arguments from the method call".to_string()),
                None
            ));
        }

        // Check that the object expression is valid
        let _object_type = self.check_expression(object)?;

        // Return the target type based on the method name
        match method {
            "toInteger" => Ok(Type::Integer),
            "toFloat" => Ok(Type::Number),
            "toString" => Ok(Type::String),
            "toBoolean" => Ok(Type::Boolean),
            _ => unreachable!("Invalid type conversion method: {}", method)
        }
    }
    
    #[allow(dead_code)]
    fn push_error_scope(&mut self) {
        self.error_context_depth += 1;
        // Add error variable to the current scope with proper Error type
        self.symbol_table.insert("error".to_string(), self.create_error_type());
    }
    
    /// Create the Error type with proper structure
    #[allow(dead_code)]
    fn create_error_type(&self) -> Type {
        // Error object has message (String), code (Integer), and location (String) properties
        Type::Object("Error".to_string())
    }
    
    #[allow(dead_code)]
    fn pop_error_scope(&mut self) {
        self.error_context_depth -= 1;
        if self.error_context_depth == 0 {
            // Remove error variable from scope
            self.symbol_table.remove("error");
        }
    }
    
    #[allow(dead_code)]
    fn in_error_context(&self) -> bool {
        self.error_context_depth > 0
    }

    /// Check for unused variables and generate warnings
    fn check_unused_variables(&mut self) {
        let variable_environment = self.variable_environment.clone();
        for var_name in &variable_environment {
            if !self.used_variables.contains(var_name) {
                self.add_warning(CompilerWarning::unused_variable(var_name, None));
            }
        }
    }

    /// Check for unused functions and generate warnings
    fn check_unused_functions(&mut self) {
        let function_environment = self.function_environment.clone();
        for func_name in &function_environment {
            if !self.used_functions.contains(func_name) && 
               !["main", "start"].contains(&func_name.as_str()) {
                self.add_warning(CompilerWarning::unused_function(func_name, None));
            }
        }
    }

    fn is_valid_type(&self, type_: &Type) -> bool {
        match type_ {
            Type::Integer | Type::Number | Type::String | Type::Boolean | Type::Void | Type::Any => true,
            Type::List(element_type) => self.is_valid_type(element_type),
            Type::Object(class_name) => self.class_table.contains_key(class_name),
            Type::Future(inner_type) => self.is_valid_type(inner_type),
            Type::IntegerSized { .. } | Type::NumberSized { .. } => true,
            Type::Class { .. } => true, // Assume class types are valid if parsed
            Type::TypeParameter(name) => self.type_environment.contains(name),
            Type::Matrix(_) => true, // Matrix types are valid
            Type::Pairs(_, _) => true, // Pair types are valid
            Type::Generic(_, _) => true, // Generic types are valid  
            Type::Function(_, _) => true, // Function types are valid
        }
    }

    fn check_function_call(&mut self, name: &str, args: &[Expression], location: Option<SourceLocation>) -> Result<Type, CompilerError> {
        // Special case: Check if this is a zero-argument "function call" that should be a variable reference
        if args.is_empty() {
            if let Some(var_type) = self.current_scope.lookup_variable(name) {
                self.used_variables.insert(name.to_string());
                // Implicit await: if the variable is a Future<T>, return T
                return match var_type {
                    Type::Future(inner_type) => Ok(*inner_type),
                    _ => Ok(var_type)
                };
            }
        }

        // Check if this is a method-style function being called as traditional function
        let method_functions = ["length", "isEmpty", "isNotEmpty", "isDefined", "isNotDefined", "keepBetween"];
        if method_functions.contains(&name) {
            return Err(CompilerError::method_suggestion_error(name, location, None));
        }

        if let Some(overloads) = self.function_table.get(name).cloned() {
            eprintln!("DEBUG: Found function '{}' with {} overloads", name, overloads.len());
            // Try to find a matching overload based on parameter types
            let arg_types: Result<Vec<Type>, CompilerError> = args.iter()
                .map(|arg| self.check_expression(arg))
                .collect();
            let arg_types = arg_types?;
            
            // Find the best matching overload
            let mut best_match = None;
            let mut exact_match = None;
            
            // Debug: print overload resolution details
            eprintln!("DEBUG: Resolving function '{}' with {} args", name, arg_types.len());
            for (i, arg_type) in arg_types.iter().enumerate() {
                eprintln!("DEBUG:   arg[{}]: {:?}", i, arg_type);
            }
            eprintln!("DEBUG: Available overloads:");
            for (i, (param_types, return_type, required_param_count)) in overloads.iter().enumerate() {
                eprintln!("DEBUG:   overload[{}]: {:?} -> {:?} (required: {})", i, param_types, return_type, required_param_count);
            }
            
            for (param_types, return_type, required_param_count) in &overloads {
                // Check basic parameter count constraints
                if arg_types.len() < *required_param_count || arg_types.len() > param_types.len() {
                    continue;
                }
                
                // Check if all provided arguments are compatible
                let mut is_compatible = true;
                let mut is_exact = true;
                
                for (_i, (arg_type, expected_type)) in arg_types.iter().zip(param_types.iter()).enumerate() {
                    if !self.types_compatible(expected_type, arg_type) {
                        is_compatible = false;
                        break;
                    }
                    if arg_type != expected_type {
                        is_exact = false;
                    }
                }
                
                if is_compatible {
                    if is_exact {
                        exact_match = Some((param_types.clone(), return_type.clone(), *required_param_count));
                        break; // Exact match is always preferred
                    } else if best_match.is_none() {
                        best_match = Some((param_types.clone(), return_type.clone(), *required_param_count));
                    }
                }
            }
            
            // Use exact match if found, otherwise use best compatible match
            let (param_types, return_type, _required_param_count) = exact_match.or(best_match)
                .ok_or_else(|| {
                    let arg_type_str = arg_types.iter()
                        .map(|t| format!("{:?}", t))
                        .collect::<Vec<_>>()
                        .join(", ");
                    CompilerError::type_error(
                        format!("No compatible overload found for function '{}' with arguments ({})", name, arg_type_str),
                        Some("Check function signature and argument types".to_string()),
                        location
                    )
                })?;
            
            eprintln!("DEBUG: Selected overload: {:?} -> {:?}", param_types, return_type);
            // Parameter validation is now handled in overload resolution above

            Ok(return_type)
        } else {
            // Get available function names for suggestions
            let available_functions: Vec<&str> = self.function_table.keys().map(|s| s.as_str()).collect();
            Err(CompilerError::function_not_found_error(name, &available_functions, location.unwrap_or_default()))
        }
    }

    #[allow(dead_code)]
    fn check_this_access(&mut self, location: &SourceLocation) -> Result<Type, CompilerError> {
        if !self.current_constructor {
            return Err(CompilerError::type_error(
                "The 'this' keyword can only be used within a constructor".to_string(),
                Some("Use 'this' only inside class constructors".to_string()),
                Some(location.clone())
            ));
        }

        let current_class_name = self.current_class.as_ref().ok_or_else(|| {
            CompilerError::type_error(
                "The 'this' keyword can only be used within a class".to_string(),
                Some("'this' is only valid inside class methods or constructors".to_string()),
                Some(location.clone())
            )
        })?;

        let current_class = self.class_table.get(current_class_name).cloned().ok_or_else(|| {
            CompilerError::type_error(
                format!("Current class '{}' not found", current_class_name),
                None,
                Some(location.clone())
            )
        })?;

        Ok(Type::Object(current_class.name.clone()))
    }

    // Additional helper methods required by the semantic analyzer
    fn is_builtin_function(&self, name: &str) -> bool {
        self.function_table.contains_key(name)
    }

    fn check_literal(&self, value: &Value) -> Type {
        match value {
            Value::Integer(_) => Type::Integer,
            Value::Number(_) => Type::Number,
            Value::String(_) => Type::String,
            Value::Boolean(_) => Type::Boolean,
            Value::List(elements) => {
                if elements.is_empty() {
                    Type::List(Box::new(Type::Any))
                } else {
                    // Use the type of the first element
                    let element_type = self.check_literal(&elements[0]);
                    Type::List(Box::new(element_type))
                }
            },
            Value::Matrix(_) => Type::Matrix(Box::new(Type::Number)),
            Value::Void => Type::Void,
            Value::Integer8(_) => Type::Integer,
            Value::Integer8u(_) => Type::Integer,
            Value::Integer16(_) => Type::Integer,
            Value::Integer16u(_) => Type::Integer,
            Value::Integer32(_) => Type::Integer,
            Value::Integer64(_) => Type::Integer,
            Value::Number32(_) => Type::Number,
            Value::Number64(_) => Type::Number,
        }
    }

    fn check_unused_items(&mut self) {
        self.check_unused_variables();
        self.check_unused_functions();
    }

    fn find_method_in_hierarchy(&self, class_name: &str, method_name: &str) -> Option<(Function, String)> {
        if let Some(class) = self.class_table.get(class_name) {
            // Check methods in current class
            for method in &class.methods {
                if method.name == method_name {
                    return Some((method.clone(), class_name.to_string()));
                }
            }
            
            // Check parent class
            if let Some(parent_name) = &class.base_class {
                return self.find_method_in_hierarchy(parent_name, method_name);
            }
        }
        None
    }

    fn check_method_override(&mut self, method: &Function, parent_method: &Function, class_name: &str, parent_class_name: &str) -> Result<(), CompilerError> {
        // Check if return types match
        if method.return_type != parent_method.return_type {
            return Err(CompilerError::type_error(
                format!("Method '{}' in class '{}' has different return type than parent method in '{}'", 
                    method.name, class_name, parent_class_name),
                Some(format!("Expected {:?}, got {:?}", parent_method.return_type, method.return_type)),
                None
            ));
        }

        // Check if parameter counts match
        if method.parameters.len() != parent_method.parameters.len() {
            return Err(CompilerError::type_error(
                format!("Method '{}' in class '{}' has different parameter count than parent method", 
                    method.name, class_name),
                Some("Override methods must have the same parameter signature".to_string()),
                None
            ));
        }

        // Check parameter types
        for (i, (param, parent_param)) in method.parameters.iter().zip(parent_method.parameters.iter()).enumerate() {
            if param.type_ != parent_param.type_ {
                return Err(CompilerError::type_error(
                    format!("Parameter {} in method '{}' has different type than parent method", 
                        i + 1, method.name),
                    Some(format!("Expected {:?}, got {:?}", parent_param.type_, param.type_)),
                    None
                ));
            }
        }

        Ok(())
    }

    fn check_type(&self, type_: &Type) -> Result<(), CompilerError> {
        if !self.is_valid_type(type_) {
            return Err(CompilerError::type_error(
                format!("Invalid type: {:?}", type_),
                Some("Check if the type is defined and available in the current scope".to_string()),
                None
            ));
        }
        Ok(())
    }

    fn get_class_hierarchy(&self, class_name: &str) -> Vec<String> {
        let mut hierarchy = vec![class_name.to_string()];
        
        if let Some(class) = self.class_table.get(class_name) {
            if let Some(parent_name) = &class.base_class {
                let mut parent_hierarchy = self.get_class_hierarchy(parent_name);
                hierarchy.append(&mut parent_hierarchy);
            }
        }
        
        hierarchy
    }

    fn is_builtin_class(&self, name: &str) -> bool {
        matches!(name, "List" | "String" | "Object" | "File" | "MathUtils" | "Http")
    }

    fn is_builtin_type_constructor(&self, name: &str) -> bool {
        matches!(name, "List")
    }

    fn check_builtin_type_constructor(&mut self, name: &str, args: &[Expression]) -> Result<Type, CompilerError> {
        match name {
            "List" => {
                if args.len() != 1 {
                    return Err(CompilerError::type_error(
                        "List constructor expects exactly one argument (element type)".to_string(),
                        Some("Usage: List(elementType)".to_string()),
                        None
                    ));
                }
                
                // Get the element type from the argument
                let element_type = self.check_expression(&args[0])?;
                Ok(Type::List(Box::new(element_type)))
            },
            _ => Err(CompilerError::type_error(
                format!("Unknown builtin type constructor: {}", name),
                None,
                None
            ))
        }
    }

    fn check_print_function_call(&mut self, name: &str, args: &[Expression]) -> Result<Type, CompilerError> {
        // Mark function as used
        self.used_functions.insert(name.to_string());
        
        if args.is_empty() {
            return Err(CompilerError::type_error(
                format!("Function '{}' requires at least one argument", name),
                Some("Provide an argument to print".to_string()),
                None
            ));
        }
        
        // Check that all arguments are valid expressions
        for arg in args {
            self.check_expression(arg)?;
        }
        
        Ok(Type::Void)
    }

    fn check_binary_operation(&mut self, op: &BinaryOperator, left: &Expression, right: &Expression, _location: &Option<SourceLocation>) -> Result<Type, CompilerError> {
        let left_type = self.check_expression(left)?;
        let right_type = self.check_expression(right)?;

        match op {
            BinaryOperator::Add => {
                // Handle string concatenation
                if left_type == Type::String && right_type == Type::String {
                    Ok(Type::String)
                }
                // Handle numeric addition
                else if matches!(left_type, Type::Integer | Type::Number) && matches!(right_type, Type::Integer | Type::Number) {
                    // If either operand is float, result is float
                    if matches!(left_type, Type::Number) || matches!(right_type, Type::Number) {
                        Ok(Type::Number)
                    } else {
                        Ok(Type::Integer)
                    }
                } else {
                    Err(CompilerError::type_error(
                        format!("Cannot apply {:?} to types {:?} and {:?}", op, left_type, right_type),
                        Some("Add operator requires either two strings (for concatenation) or two numeric types (for arithmetic)".to_string()),
                        None
                    ))
                }
            },
            BinaryOperator::Subtract | BinaryOperator::Multiply | BinaryOperator::Divide => {
                if matches!(left_type, Type::Integer | Type::Number) && matches!(right_type, Type::Integer | Type::Number) {
                    // If either operand is float, result is float
                    if matches!(left_type, Type::Number) || matches!(right_type, Type::Number) {
                        Ok(Type::Number)
                    } else {
                        Ok(Type::Integer)
                    }
                } else {
                    Err(CompilerError::type_error(
                        format!("Cannot apply {:?} to types {:?} and {:?}", op, left_type, right_type),
                        Some("Arithmetic operations require numeric types".to_string()),
                        None
                    ))
                }
            },
            BinaryOperator::Equal | BinaryOperator::NotEqual => {
                if self.types_compatible(&left_type, &right_type) {
                    Ok(Type::Boolean)
                } else {
                    Err(CompilerError::type_error(
                        format!("Cannot compare types {:?} and {:?}", left_type, right_type),
                        Some("Comparison requires compatible types".to_string()),
                        None
                    ))
                }
            },
            BinaryOperator::Less | BinaryOperator::LessEqual | BinaryOperator::Greater | BinaryOperator::GreaterEqual => {
                if matches!(left_type, Type::Integer | Type::Number | Type::String) && 
                   matches!(right_type, Type::Integer | Type::Number | Type::String) &&
                   self.types_compatible(&left_type, &right_type) {
                    Ok(Type::Boolean)
                } else {
                    Err(CompilerError::type_error(
                        format!("Cannot compare types {:?} and {:?}", left_type, right_type),
                        Some("Comparison requires compatible numeric or string types".to_string()),
                        None
                    ))
                }
            },
            BinaryOperator::And | BinaryOperator::Or => {
                if left_type == Type::Boolean && right_type == Type::Boolean {
                    Ok(Type::Boolean)
                } else {
                    Err(CompilerError::type_error(
                        format!("Logical operations require boolean operands, got {:?} and {:?}", left_type, right_type),
                        Some("Use boolean expressions with logical operators".to_string()),
                        None
                    ))
                }
            },
            BinaryOperator::Modulo => {
                // Modulo operation requires numeric types
                if matches!(left_type, Type::Integer | Type::Number) && matches!(right_type, Type::Integer | Type::Number) {
                    // If either operand is float, result is float
                    if matches!(left_type, Type::Number) || matches!(right_type, Type::Number) {
                        Ok(Type::Number)
                    } else {
                        Ok(Type::Integer)
                    }
                } else {
                    Err(CompilerError::type_error(
                        "Modulo operation requires numeric operands".to_string(),
                        Some("Use integer or float types with modulo operator".to_string()),
                        None
                    ))
                }
            },
            BinaryOperator::Power => {
                // Power operation requires numeric types
                if matches!(left_type, Type::Integer | Type::Number) && matches!(right_type, Type::Integer | Type::Number) {
                    // Power operations typically return float
                    Ok(Type::Number)
                } else {
                    Err(CompilerError::type_error(
                        "Power operation requires numeric operands".to_string(),
                        Some("Use numeric types with power operator".to_string()),
                        None
                    ))
                }
            },
            BinaryOperator::Is => {
                // Type checking operation - returns boolean
                Ok(Type::Boolean)
            },
            BinaryOperator::Not => {
                // Not operation requires boolean operands
                if left_type == Type::Boolean && right_type == Type::Boolean {
                    Ok(Type::Boolean)
                } else {
                    Err(CompilerError::type_error(
                        "Not operation requires boolean operands".to_string(),
                        Some("Use boolean types with not operator".to_string()),
                        None
                    ))
                }
            }
        }
    }

    fn resolve_type(&self, type_: &Type) -> Type {
        match type_ {
            // Resolve generic array types
            Type::List(element_type) => {
                let resolved_element = self.resolve_type(element_type);
                Type::List(Box::new(resolved_element))
            },
            
            // Resolve generic matrix types  
            Type::Matrix(element_type) => {
                let resolved_element = self.resolve_type(element_type);
                Type::Matrix(Box::new(resolved_element))
            },
            
            // Resolve future types
            Type::Future(inner_type) => {
                let resolved_inner = self.resolve_type(inner_type);
                Type::Future(Box::new(resolved_inner))
            },
            
            // For custom class types, check if they exist in the class table
            Type::Class { name, type_args: _ } => {
                if self.class_table.contains_key(name) {
                    type_.clone()
                } else {
                    // If class doesn't exist, return Any as fallback
                    Type::Any
                }
            },
            
            // Basic types and others pass through unchanged
            _ => type_.clone()
        }
    }

    /// Type compatibility checking with proper coercion rules
    fn types_compatible(&self, expected: &Type, actual: &Type) -> bool {
        if expected == actual {
            return true;
        }

        // Handle Any type - it's compatible with everything
        if matches!(expected, Type::Any) || matches!(actual, Type::Any) {
            return true;
        }

        // Additional compatibility rules
        match (expected, actual) {
            // Numeric type promotions
            (Type::Number, Type::Integer) => true, // Integer can be promoted to Number
            
            // List element type compatibility
            (Type::List(expected_elem), Type::List(actual_elem)) => {
                self.types_compatible(expected_elem, actual_elem)
            }
            
            // Generic List compatibility - handle List<T> syntax parsed as Generic
            (Type::Generic(base_type, type_args), Type::List(actual_elem)) => {
                if let Type::Object(class_name) = base_type.as_ref() {
                    if class_name == "List" && type_args.len() == 1 {
                        return self.types_compatible(&type_args[0], actual_elem);
                    }
                }
                false
            }
            (Type::List(expected_elem), Type::Generic(base_type, type_args)) => {
                if let Type::Object(class_name) = base_type.as_ref() {
                    if class_name == "List" && type_args.len() == 1 {
                        return self.types_compatible(expected_elem, &type_args[0]);
                    }
                }
                false
            }
            
            // Class inheritance compatibility
            (Type::Object(expected_class), Type::Object(actual_class)) => {
                self.is_subclass_of(actual_class, expected_class)
            }
            
            // Handle Class variant compatibility
            (Type::Class { name: expected_class, .. }, Type::Class { name: actual_class, .. }) => {
                self.is_subclass_of(actual_class, expected_class)
            }
            
            // Mixed Object and Class compatibility (treat Object as string-based class name)
            (Type::Object(expected_class), Type::Class { name: actual_class, .. }) => {
                self.is_subclass_of(actual_class, expected_class)
            }
            (Type::Class { name: expected_class, .. }, Type::Object(actual_class)) => {
                self.is_subclass_of(actual_class, expected_class)
            }
            
            _ => false,
        }
    }

    /// Check if actual_class is a subclass of (or is the same as) expected_class
    fn is_subclass_of(&self, actual_class: &str, expected_class: &str) -> bool {
        // Same class is always compatible
        if actual_class == expected_class {
            return true;
        }
        
        // Get the inheritance hierarchy for the actual class
        let hierarchy = self.get_class_hierarchy(actual_class);
        
        // Check if expected_class is anywhere in the hierarchy
        hierarchy.contains(&expected_class.to_string())
    }

    /// Add a warning to the warnings list
    pub fn add_warning(&mut self, warning: CompilerWarning) {
        self.warnings.push(warning);
    }
} 