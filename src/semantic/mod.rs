use std::collections::{HashMap, HashSet};
use crate::ast::*;
use crate::error::{CompilerError, CompilerWarning, WarningType};
use crate::module::{ModuleResolver, ImportResolution};

mod scope;
use scope::Scope;
mod type_constraint;
use type_constraint::{TypeConstraint, NumericTypeConstraint, BaseTypeConstraint, AnyTypeConstraint};

pub struct SemanticAnalyzer {
    symbol_table: HashMap<String, Type>,
    function_table: HashMap<String, (Vec<Type>, Type)>, // (parameter types, return type)
    class_table: HashMap<String, Class>,
    current_class: Option<String>,
    current_function: Option<String>,
    current_constructor: bool, // Track if we're in a constructor
    loop_depth: i32,
    type_environment: HashSet<String>,
    variable_environment: HashSet<String>,
    function_environment: HashSet<String>,
    class_environment: HashSet<String>,
    current_scope: Scope,
    current_function_return_type: Option<Type>,
    warnings: Vec<CompilerWarning>,
    used_variables: HashSet<String>,
    used_functions: HashSet<String>,
    error_context_depth: i32,
    type_constraints: HashMap<String, Box<dyn TypeConstraint>>,
    // Module system integration
    module_resolver: ModuleResolver,
    current_imports: Option<ImportResolution>,
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
            type_constraints: HashMap::new(),
            module_resolver: ModuleResolver::new(),
            current_imports: None,
        };
        
        analyzer.register_builtin_functions();
        analyzer
    }

    /// Register built-in functions that are available in the global scope
    fn register_builtin_functions(&mut self) {
        // Basic I/O functions
        self.function_table.insert(
            "print".to_string(),
            (vec![Type::Any], Type::Void)
        );

        // Println variations for different types
        self.function_table.insert(
            "printl_int".to_string(),
            (vec![Type::Integer], Type::Void)
        );

        self.function_table.insert(
            "printl_float".to_string(),
            (vec![Type::Float], Type::Void)
        );

        self.function_table.insert(
            "printl_string".to_string(),
            (vec![Type::String], Type::Void)
        );

        self.function_table.insert(
            "printl_bool".to_string(),
            (vec![Type::Boolean], Type::Void)
        );

        // Generic println function
        self.function_table.insert(
            "printl".to_string(),
            (vec![Type::Any], Type::Void)
        );

        // Generic println function alias
        self.function_table.insert(
            "println".to_string(),
            (vec![Type::Any], Type::Void)
        );

        // Mathematical functions
        self.function_table.insert(
            "abs".to_string(),
            (vec![Type::Any], Type::Any)
        );

        // Array utilities
        self.function_table.insert(
            "array_get".to_string(),
            (vec![Type::Array(Box::new(Type::Any)), Type::Integer], Type::Any)
        );

        self.function_table.insert(
            "array_length".to_string(),
            (vec![Type::Array(Box::new(Type::Any))], Type::Integer)
        );

        // Validation/assertion functions
        self.function_table.insert(
            "assert".to_string(),
            (vec![Type::Boolean], Type::Void)
        );

        // String manipulation functions
        self.function_table.insert(
            "string_concat".to_string(),
            (vec![Type::String, Type::String], Type::String)
        );
        
        self.function_table.insert(
            "string_compare".to_string(),
            (vec![Type::String, Type::String], Type::Integer)
        );

        // HTTP functionality
        self.function_table.insert(
            "http_get".to_string(),
            (vec![Type::String], Type::String)
        );
        
        self.function_table.insert(
            "http_post".to_string(),
            (vec![Type::String, Type::String], Type::String)
        );
        
        self.function_table.insert(
            "http_put".to_string(),
            (vec![Type::String, Type::String], Type::String)
        );
        
        self.function_table.insert(
            "http_delete".to_string(),
            (vec![Type::String], Type::String)
        );
        
        self.function_table.insert(
            "http_patch".to_string(),
            (vec![Type::String, Type::String], Type::String)
        );
    }

    pub fn analyze(&mut self, program: &Program) -> Result<Program, CompilerError> {
        // First, resolve imports if any
        if !program.imports.is_empty() {
            let import_resolution = self.module_resolver.resolve_imports(program)?;
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
            // Don't overwrite builtin functions like print, printl, etc.
            if !self.is_builtin_function(&function.name) {
                self.function_table.insert(
                    function.name.clone(),
                    (param_types, function.return_type.clone())
                );
            }
        }

        if let Some(start_fn) = &program.start_function {
            let param_types = start_fn.parameters.iter().map(|p| p.type_.clone()).collect();
            // Don't overwrite builtin functions like print, printl, etc.
            if !self.is_builtin_function(&start_fn.name) {
                self.function_table.insert(
                    start_fn.name.clone(),
                    (param_types, start_fn.return_type.clone())
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
                        &format!("Inheritance cycle detected involving class '{}'", class_name),
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
                            &format!("Cannot assign {:?} to variable of type {:?}", init_type, resolved_type),
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
                // Check that the function exists
                if !self.function_table.contains_key(function_name) && !self.is_builtin_function(function_name) {
                    return Err(CompilerError::type_error(
                        &format!("Function '{}' not found", function_name),
                        Some("Check if the function name is correct and the function is declared".to_string()),
                        None
                    ));
                            }
                
                // Check all expressions
                for expr in expressions {
                    self.check_expression(expr)?;
                }
                Ok(())
            },

            Statement::MethodApplyBlock { object_name, method_chain, expressions, location: _ } => {
                // Check that the object exists
                if !self.current_scope.lookup_variable(object_name).is_some() {
                    return Err(CompilerError::type_error(
                        &format!("Object '{}' not found", object_name),
                        Some("Check if the object name is correct and the object is declared".to_string()),
                        None
                    ));
                }
                
                // For now, we'll do basic validation - in a full implementation we'd check method signatures
                if method_chain.is_empty() {
                    return Err(CompilerError::type_error(
                        "Method apply block requires at least one method".to_string(),
                        Some("Use the format: object.method: arguments".to_string()),
                        None
                    ));
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
                    Type::Array(element_type) => *element_type,
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

            Statement::Expression { expr, location: _ } => {
                self.check_expression(expr)?;
                Ok(())
            },
            
            Statement::Error { message, location: _ } => {
                // Check that the message expression is valid and returns a string
                let message_type = self.check_expression(message)?;
                if message_type != Type::String {
                    return Err(CompilerError::enhanced_type_error(
                        "Error message must be a string".to_string(),
                        Some("String".to_string()),
                        Some(format!("{:?}", message_type)),
                        None,
                        vec![
                            "Use a string literal like \"error message\"".to_string(),
                            "Use a string variable or expression".to_string(),
                            "Convert the value to string using .toString()".to_string(),
                        ],
                    ));
                }
                Ok(())
            },
            
            // Module and async statements
            Statement::Import { imports: _, location: _ } => {
                // For now, imports are just validated for syntax but not resolved
                // TODO: Implement module resolution and validation
                Ok(())
            },
            
            Statement::LaterAssignment { variable, expression, location: _ } => {
                // later variable = start expression
                let expr_type = self.check_expression(expression)?;
                // Create a Future type wrapper
                let future_type = Type::Future(Box::new(expr_type));
                self.symbol_table.insert(variable.clone(), future_type);
                Ok(())
            },
            
            Statement::Background { expression, location: _ } => {
                // background expression - fire and forget
                let _expr_type = self.check_expression(expression)?;
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
                    Ok(var_type)
                } else if self.is_builtin_class(name) {
                    // Built-in class names are valid "variables" that represent the class itself
                    // This allows static method calls like File.read() to work
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
            },

            Expression::Binary(left, op, right) => {
                self.check_binary_operation(op, left, right, &None)
            },

            Expression::Unary(op, expr) => {
                let expr_type = self.check_expression(expr)?;
        match op {
                    UnaryOperator::Negate => {
                        if expr_type == Type::Integer || expr_type == Type::Float {
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

                self.used_functions.insert(name.clone());
                
                if let Some((param_types, return_type)) = self.function_table.get(name).cloned() {
                    // Special case: if this is a print function but it has wrong parameter count in function table,
                    // use the builtin print function validation instead
                    if (name == "print" || name == "printl" || name == "println") && param_types.len() != 1 {
                        return self.check_print_function_call(name, args);
                    }
                    
                    if args.len() != param_types.len() {
                    return Err(CompilerError::type_error(
                            &format!("Function '{}' called with wrong number of arguments\nExpected type: {}\nActual type: {}", 
                                name, param_types.len(), args.len()),
                            Some(format!("Function '{}' expects {} arguments, but {} were provided", name, param_types.len(), args.len())),
                            None
                        ));
                    }

                    for (i, (arg, param_type)) in args.iter().zip(param_types.iter()).enumerate() {
                        let arg_type = self.check_expression(arg)?;
                        if !self.types_compatible(&param_type, &arg_type) {
                    return Err(CompilerError::type_error(
                                &format!("Argument {} has type {:?}, but parameter expects {:?}", 
                                    i + 1, arg_type, param_type),
                                Some("Provide arguments of the correct type".to_string()),
                                None
                            ));
                        }
                    }

                    Ok(return_type)
                } else {
                    Err(CompilerError::type_error(
                        &format!("Function '{}' not found", name),
                        Some("Check if the function name is correct and the function is defined".to_string()),
                        None
                    ))
                }
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
                    _ => Err(CompilerError::type_error(
                        &format!("Cannot assign property '{}' on type {:?}", property, object_type),
                        Some("Property assignment is only supported on lists".to_string()),
                        None
                    ))
                }
            },

            Expression::MethodCall { object, method, arguments, location } => {
                // Check if this is a call to an imported module's method
                if let Expression::Variable(module_name) = &**object {
                    if let Some(ref imports) = self.current_imports {
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

                                // Type check arguments
                                for (i, (arg, param)) in arguments.iter().zip(function.parameters.iter()).enumerate() {
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

                                return Ok(function.return_type.clone());
                            } else {
                                return Err(CompilerError::symbol_error(
                                    "Function not found in module",
                                    method,
                                    Some(module_name)
                                ));
                            }
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

        let constructor = class.constructor.as_ref().ok_or_else(|| {
            CompilerError::type_error(
                &format!("No constructor found for class '{}'", class_name),
                Some("Define a constructor for the class".to_string()),
                Some(location.clone())
            )
        })?;

        if args.len() != constructor.parameters.len() {
            return Err(CompilerError::type_error(
                &format!("Constructor for class '{}' expects {} arguments, but {} were provided",
                    class_name, constructor.parameters.len(), args.len()),
                Some("Provide the correct number of arguments".to_string()),
                Some(location.clone())
            ));
        }

        // Infer parameter types from class fields and clone to avoid borrow issues
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

        Ok(Type::Object(class_name.to_string()))
    }

    fn check_method_call(&mut self, object: &Expression, method: &str, args: &[Expression], location: &SourceLocation) -> Result<Type, CompilerError> {
        let object_type = self.check_expression(object)?;
        
        match &object_type {
            Type::Object(class_name) => {
                // Look up the class in the class table
                let class = self.class_table.get(class_name).ok_or_else(|| {
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

                        // Check argument types
                        for (i, (arg, param)) in args.iter().zip(method_def.parameters.iter()).enumerate() {
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

                        return Ok(method_def.return_type.clone());
                    }
                }

                // If we reach here, the method was not found
                Err(CompilerError::type_error(
                    &format!("Method '{}' not found in class '{}' or its parent classes", method, class_name),
                    Some("Check if the method name is correct and defined in the class hierarchy".to_string()),
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
            "toFloat" => Ok(Type::Float),
            "toString" => Ok(Type::String),
            "toBoolean" => Ok(Type::Boolean),
            _ => unreachable!("Invalid type conversion method: {}", method)
        }
    }
    
    fn push_error_scope(&mut self) {
        self.error_context_depth += 1;
        // Add error variable to the current scope with proper Error type
        self.symbol_table.insert("error".to_string(), self.create_error_type());
    }
    
    /// Create the Error type with proper structure
    fn create_error_type(&self) -> Type {
        // Error object has message (String), code (Integer), and location (String) properties
        Type::Object("Error".to_string())
    }
    
    fn pop_error_scope(&mut self) {
        self.error_context_depth -= 1;
        if self.error_context_depth == 0 {
            // Remove error variable from scope
            self.symbol_table.remove("error");
        }
    }
    
    fn in_error_context(&self) -> bool {
        self.error_context_depth > 0
    }

    /// Check for unused variables and generate warnings
    fn check_unused_variables(&mut self) {
        for var_name in &self.variable_environment {
            if !self.used_variables.contains(var_name) {
                self.add_warning(CompilerWarning::unused_variable(var_name, None));
            }
        }
    }

    /// Check for unused functions and generate warnings
    fn check_unused_functions(&mut self) {
        for func_name in &self.function_environment {
            if !self.used_functions.contains(func_name) && 
               !["main", "start"].contains(&func_name.as_str()) {
                self.add_warning(CompilerWarning::unused_function(func_name, None));
            }
        }
    }

    fn is_valid_type(&self, type_: &Type) -> bool {
        match type_ {
            Type::Integer | Type::Float | Type::String | Type::Boolean | Type::Void | Type::Any => true,
            Type::Array(element_type) => self.is_valid_type(element_type),
            Type::Object(class_name) => self.class_table.contains_key(class_name),
            Type::List(element_type) => self.is_valid_type(element_type),
            Type::Future(inner_type) => self.is_valid_type(inner_type),
            Type::IntegerSized { .. } | Type::FloatSized { .. } => true,
            Type::Class { .. } => true, // Assume class types are valid if parsed
            Type::TypeParameter(name) => self.type_environment.contains(name),
        }
    }

    fn check_function_call(&mut self, name: &str, args: &[Expression], call_location: &SourceLocation) -> Result<Type, CompilerError> {
        // Check if it's a built-in function first
        if let Some((param_types, return_type)) = self.function_table.get(name).cloned() {
            // Add to used functions for warning analysis
            if self.function_environment.contains(name) {
                self.used_functions.insert(name.to_string());
            }

            // Generic functions that accept any type don't need strict checking
            if let Some(Type::Any) = param_types.first() {
                self.used_functions.insert(name.to_string());
                // For generic functions, just check that some arguments are provided
                if name == "print" || name == "println" || name == "printl" {
                    self.used_functions.insert(name.to_string());
                    if args.is_empty() {
                        return Err(CompilerError::type_error(
                            &format!("Function '{}' requires at least one argument", name),
                            Some("Provide an argument to print".to_string()),
                            Some(call_location.clone())
                        ));
                    }
                    // Check that all arguments are valid expressions
                    for arg in args {
                        self.check_expression(arg)?;
                    }
                    return Ok(return_type);
                }

                if name == "abs" {
                    self.used_functions.insert(name.to_string());
                    if args.len() != 1 {
                        return Err(CompilerError::type_error(
                            &format!("Function '{}' expects 1 argument, but {} were provided", name, args.len()),
                            Some("Provide exactly one argument".to_string()),
                            Some(call_location.clone())
                        ));
                    }
                    let arg_type = self.check_expression(&args[0])?;
                    // Return the same type as the argument for abs
                    return Ok(arg_type);
                }
            }

            // For other functions, do strict type checking
            if args.len() != param_types.len() {
                return Err(CompilerError::type_error(
                    &format!("Function '{}' expects {} arguments, but {} were provided",
                        name, param_types.len(), args.len()),
                    Some("Provide the correct number of arguments".to_string()),
                    Some(call_location.clone())
                ));
            }

            for (i, (arg, expected_type)) in args.iter().zip(param_types.iter()).enumerate() {
                let arg_type = self.check_expression(arg)?;
                if !self.types_compatible(expected_type, &arg_type) {
                    return Err(CompilerError::type_error(
                        &format!("Argument {} has incorrect type. Expected {:?}, got {:?}",
                            i + 1, expected_type, arg_type),
                        Some("Provide an argument of the correct type".to_string()),
                        Some(call_location.clone())
                    ));
                }
            }

            return Ok(return_type);
        }

        // Check if it's a user-defined function
        if let Some((param_types, return_type)) = self.function_table.get(name).cloned() {
            // Mark function as used
            self.used_functions.insert(name.to_string());

            if args.len() != param_types.len() {
                return Err(CompilerError::type_error(
                    &format!("Function '{}' expects {} arguments, but {} were provided",
                        name, param_types.len(), args.len()),
                    Some("Check the function definition and provide the correct number of arguments".to_string()),
                    Some(call_location.clone())
                ));
            }

            for (i, (arg, expected_type)) in args.iter().zip(param_types.iter()).enumerate() {
                let arg_type = self.check_expression(arg)?;
                if !self.types_compatible(expected_type, &arg_type) {
                    return Err(CompilerError::type_error(
                        &format!("Argument {} to function '{}' has incorrect type. Expected {:?}, got {:?}",
                            i + 1, name, expected_type, arg_type),
                        Some("Provide arguments of the correct type".to_string()),
                        Some(call_location.clone())
                    ));
                }
            }

            return Ok(return_type);
        }

        // Function not found
        Err(CompilerError::type_error(
            &format!("Function '{}' not found", name),
            Some("Check if the function name is correct and the function is defined".to_string()),
            Some(call_location.clone())
        ))
    }

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
            Value::Float(_) => Type::Float,
            Value::String(_) => Type::String,
            Value::Boolean(_) => Type::Boolean,
            Value::Array(elements) => {
                if elements.is_empty() {
                    Type::Array(Box::new(Type::Any))
                } else {
                    // Use the type of the first element
                    let element_type = self.check_literal(&elements[0]);
                    Type::Array(Box::new(element_type))
                }
            }
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
        matches!(name, "Array" | "List" | "String" | "Object")
    }

    fn is_builtin_type_constructor(&self, name: &str) -> bool {
        matches!(name, "Array" | "List")
    }

    fn check_builtin_type_constructor(&mut self, name: &str, args: &[Expression]) -> Result<Type, CompilerError> {
        match name {
            "Array" => {
                if args.len() != 1 {
                    return Err(CompilerError::type_error(
                        "Array constructor expects exactly one argument (element type)".to_string(),
                        Some("Usage: Array(elementType)".to_string()),
                        None
                    ));
                }
                
                // For now, assume the argument represents the element type
                // In a full implementation, this would be more sophisticated
                Ok(Type::Array(Box::new(Type::Any)))
            },
            "List" => {
                if args.len() != 1 {
                    return Err(CompilerError::type_error(
                        "List constructor expects exactly one argument (element type)".to_string(),
                        Some("Usage: List(elementType)".to_string()),
                        None
                    ));
                }
                
                Ok(Type::List(Box::new(Type::Any)))
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
            BinaryOperator::Add | BinaryOperator::Subtract | BinaryOperator::Multiply | BinaryOperator::Divide => {
                if matches!(left_type, Type::Integer | Type::Float) && matches!(right_type, Type::Integer | Type::Float) {
                    // If either operand is float, result is float
                    if matches!(left_type, Type::Float) || matches!(right_type, Type::Float) {
                        Ok(Type::Float)
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
                if matches!(left_type, Type::Integer | Type::Float | Type::String) && 
                   matches!(right_type, Type::Integer | Type::Float | Type::String) &&
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
            }
        }
    }

    fn resolve_type(&self, type_: &Type) -> Type {
        // For now, just return the type as-is
        // In a full implementation, this would resolve type aliases, generics, etc.
        type_.clone()
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

        // Additional compatibility rules can be added here
        // For example, implicit conversions between numeric types
        match (expected, actual) {
            (Type::Float, Type::Integer) => true, // Integer can be promoted to Float
            (Type::Array(expected_elem), Type::Array(actual_elem)) => {
                self.types_compatible(expected_elem, actual_elem)
            }
            _ => false,
        }
    }

    /// Add a warning to the warnings list
    pub fn add_warning(&mut self, warning: CompilerWarning) {
        self.warnings.push(warning);
    }
} 