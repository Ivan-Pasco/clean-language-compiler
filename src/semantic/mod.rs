use std::collections::{HashMap, HashSet};
use crate::ast::*;
use crate::error::{CompilerError, CompilerWarning, WarningType};

mod scope;

use scope::Scope;

impl Visibility {
    pub fn is_public(&self) -> bool {
        matches!(self, Visibility::Public)
    }
}

pub struct SemanticAnalyzer {
    symbol_table: HashMap<String, Type>,
    function_table: HashMap<String, (Vec<Type>, Type)>, // (parameter types, return type)
    class_table: HashMap<String, Class>,
    current_class: Option<String>,
    current_function: Option<String>,
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
}

/// Type constraint trait
trait TypeConstraint {
    fn check(&self, type_: &Type) -> bool;
}

/// Numeric type constraint
struct NumericConstraint;

impl TypeConstraint for NumericConstraint {
    fn check(&self, type_: &Type) -> bool {
        matches!(type_, Type::Integer | Type::Float | Type::IntegerSized { .. } | Type::FloatSized { .. })
    }
}

/// Comparable type constraint
struct ComparableConstraint;

impl TypeConstraint for ComparableConstraint {
    fn check(&self, type_: &Type) -> bool {
        matches!(type_, Type::Integer | Type::Float | Type::String | Type::IntegerSized { .. } | Type::FloatSized { .. })
    }
}

/// Inheritance type constraint
struct InheritanceConstraint {
    base_type: Type,
}

impl InheritanceConstraint {
    fn is_subclass(&self, child_name: &str, base_name: &str) -> bool {
        // Implement proper inheritance checking
        if child_name == base_name {
            return true;
        }
        
        // This would need access to the class table, so we'll implement this
        // as a method on SemanticAnalyzer instead
        false
    }
}

impl TypeConstraint for InheritanceConstraint {
    fn check(&self, type_: &Type) -> bool {
        if let Type::Class { name, .. } = type_ {
            if let Type::Class { name: base_name, .. } = &self.base_type {
                return name == base_name || self.is_subclass(name, base_name);
            }
        }
        false
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
        };
        
        analyzer.register_builtin_functions();
        analyzer
    }
    
    // Register built-in functions like print, println, etc.
    fn register_builtin_functions(&mut self) {
        // print function - type-safe overloads for different types
        // print(value: Integer) -> Void
        self.function_table.insert(
            "print_int".to_string(),
            (vec![Type::Integer], Type::Void)
        );
        
        // print(value: Float) -> Void  
        self.function_table.insert(
            "print_float".to_string(),
            (vec![Type::Float], Type::Void)
        );
        
        // print(value: String) -> Void
        self.function_table.insert(
            "print_string".to_string(),
            (vec![Type::String], Type::Void)
        );
        
        // print(value: Boolean) -> Void
        self.function_table.insert(
            "print_bool".to_string(),
            (vec![Type::Boolean], Type::Void)
        );
        
        // Generic print function that dispatches to type-specific versions
        // This maintains backward compatibility while adding type safety
        self.function_table.insert(
            "print".to_string(),
            (vec![Type::Any], Type::Void)
        );
        
        // printl function (print with newline) - same overloads
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
        
        self.function_table.insert(
            "printl".to_string(),
            (vec![Type::Any], Type::Void)
        );

        // println function (legacy compatibility)
        self.function_table.insert(
            "println".to_string(),
            (vec![Type::Any], Type::Void)
        );
        
        // abs function (absolute value)
        self.function_table.insert(
            "abs".to_string(),
            (vec![Type::Any], Type::Any)
        );

        // Array functions
        self.function_table.insert(
            "array_get".to_string(),
            (vec![Type::Array(Box::new(Type::Any)), Type::Integer], Type::Any)
        );

        self.function_table.insert(
            "array_length".to_string(),
            (vec![Type::Array(Box::new(Type::Any))], Type::Integer)
        );

        // Assert function
        self.function_table.insert(
            "assert".to_string(),
            (vec![Type::Boolean], Type::Void)
        );

        // String functions
        self.function_table.insert(
            "string_concat".to_string(),
            (vec![Type::String, Type::String], Type::String)
        );

        self.function_table.insert(
            "string_compare".to_string(),
            (vec![Type::String, Type::String], Type::Integer)
        );

        // HTTP functions
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
                if let Some((parent_class_name, parent_method)) = self.find_method_in_hierarchy(base_class_name, &method.name) {
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

            Statement::RangeIterate { iterator, start, end, step: _, body, location: _ } => {
                self.check_range_iterate(stmt)?;
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
                // Check if this is a static method call (ClassName.method())
                if let Expression::Variable(class_name) = object.as_ref() {
                    if self.class_table.contains_key(class_name) || self.is_builtin_class(class_name) {
                        // This is a static method call
                        return self.check_static_method_call(class_name, method, arguments, location);
                    }
                }
                
                // Check if this is a type conversion method
                if self.is_type_conversion_method(method) {
                    return self.check_type_conversion_method(object, method, arguments);
                }
                
                // This is a regular method call - check object type first
                let object_type = self.check_expression(object)?;
                
                // Check each argument type
                let mut arg_types = Vec::new();
                for arg in arguments {
                    arg_types.push(self.check_expression(arg)?);
                }
                
                // Handle method calls based on object type
                match &object_type {
                    Type::String => {
                        match method.as_str() {
                            "length" => {
                                if !arguments.is_empty() {
                                    return Err(CompilerError::type_error(
                                        "String.length() takes no arguments",
                                        Some("String.length() is a property access with no parameters".to_string()),
                                        None
                                    ));
                                }
                                Ok(Type::Integer)
                            },
                            "toUpper" | "toLower" | "trim" => {
                                if !arguments.is_empty() {
                                    return Err(CompilerError::type_error(
                                        &format!("String.{}() takes no arguments", method),
                                        Some(format!("String.{}() is a transformation method with no parameters", method)),
                                        None
                                    ));
                                }
                                Ok(Type::String)
                            },
                            "startsWith" | "endsWith" => {
                                if arguments.len() != 1 {
                                    return Err(CompilerError::type_error(
                                        &format!("String.{}() expects exactly 1 argument", method),
                                        Some(format!("String.{}(prefix/suffix) requires one string argument", method)),
                                        None
                                    ));
                                }
                                if !matches!(arg_types[0], Type::String) {
                                    return Err(CompilerError::type_error(
                                        &format!("String.{}() requires a string argument", method),
                                        Some("String comparison methods require string arguments".to_string()),
                                        None
                                    ));
                                }
                                Ok(Type::Boolean)
                            },
                            "indexOf" => {
                                if arguments.len() != 1 {
                                    return Err(CompilerError::type_error(
                                        "String.indexOf() expects exactly 1 argument",
                                        Some("String.indexOf(substring) requires one string argument".to_string()),
                                        None
                                    ));
                                }
                                if !matches!(arg_types[0], Type::String) {
                                    return Err(CompilerError::type_error(
                                        "String.indexOf() requires a string argument",
                                        Some("String search methods require string arguments".to_string()),
                                        None
                                    ));
                                }
                                Ok(Type::Integer)
                            },
                            "substring" => {
                                if arguments.len() != 2 {
                                    return Err(CompilerError::type_error(
                                        "String.substring() expects exactly 2 arguments",
                                        Some("String.substring(start, end) requires two integer arguments".to_string()),
                                        None
                                    ));
                                }
                                if !matches!(arg_types[0], Type::Integer) || !matches!(arg_types[1], Type::Integer) {
                                    return Err(CompilerError::type_error(
                                        "String.substring() requires integer arguments",
                                        Some("String.substring(start, end) requires integer indices".to_string()),
                                        None
                                    ));
                                }
                                Ok(Type::String)
                            },
                            "replace" => {
                                if arguments.len() != 2 {
                                    return Err(CompilerError::type_error(
                                        "String.replace() expects exactly 2 arguments",
                                        Some("String.replace(old, new) requires two string arguments".to_string()),
                                        None
                                    ));
                                }
                                if !matches!(arg_types[0], Type::String) || !matches!(arg_types[1], Type::String) {
                                    return Err(CompilerError::type_error(
                                        "String.replace() requires string arguments",
                                        Some("String.replace(old, new) requires string arguments".to_string()),
                                        None
                                    ));
                                }
                                Ok(Type::String)
                            },
                            "split" => {
                                if arguments.len() != 1 {
                                    return Err(CompilerError::type_error(
                                        "String.split() expects exactly 1 argument",
                                        Some("String.split(delimiter) requires one string argument".to_string()),
                                        None
                                    ));
                                }
                                if !matches!(arg_types[0], Type::String) {
                                    return Err(CompilerError::type_error(
                                        "String.split() requires a string argument",
                                        Some("String.split(delimiter) requires a string delimiter".to_string()),
                                        None
                                    ));
                                }
                                Ok(Type::Array(Box::new(Type::String)))
                            },
                            _ => {
                                Err(CompilerError::type_error(
                                    &format!("Method '{}' not found on String type", method),
                                    Some("Available methods: length(), toUpper(), toLower(), trim(), startsWith(str), endsWith(str), indexOf(str), substring(start, end), replace(old, new), split(delimiter)".to_string()),
                                    None
                                ))
                            }
                        }
                    },
                    Type::Array(_element_type) => {
                        match method.as_str() {
                            "at" => {
                                if arguments.len() != 1 {
                                    return Err(CompilerError::type_error(
                                        "Array.at() expects exactly 1 argument",
                                        Some("Array.at(index) requires one index argument".to_string()),
                                        None
                                    ));
                                }
                                if !matches!(arg_types[0], Type::Integer) {
                                    return Err(CompilerError::type_error(
                                        "Array.at() requires an integer index",
                                        Some("Array indices must be integers".to_string()),
                                        None
                                    ));
                                }
                                // Return the element type - for now assume Integer
                                Ok(Type::Integer)
                            },
                            "length" => {
                                if !arguments.is_empty() {
                                    return Err(CompilerError::type_error(
                                        "Array.length() takes no arguments",
                                        Some("Array.length() is a property access with no parameters".to_string()),
                                        None
                                    ));
                                }
                                Ok(Type::Integer)
                            },
                            _ => {
                                Err(CompilerError::type_error(
                                    &format!("Method '{}' not found on Array type", method),
                                    Some("Available methods: at(index), length()".to_string()),
                                    None
                                ))
                            }
                        }
                    },
                    Type::List(element_type) => {
                        match method.as_str() {
                            "add" => {
                                if arguments.len() != 1 {
                                    return Err(CompilerError::type_error(
                                        "List.add() expects exactly 1 argument",
                                        Some("List.add(item) requires one item argument".to_string()),
                                        None
                                    ));
                                }
                                // Check that the argument type matches the list element type
                                if !self.types_compatible(element_type, &arg_types[0]) {
                                    return Err(CompilerError::type_error(
                                        &format!("List.add() expects {:?}, found {:?}", element_type, arg_types[0]),
                                        Some("The item type must match the list's element type".to_string()),
                                        None
                                    ));
                                }
                                Ok(Type::Boolean) // Returns success/failure for unique lists
                            },
                            "remove" => {
                                if !arguments.is_empty() {
                                    return Err(CompilerError::type_error(
                                        "List.remove() takes no arguments",
                                        Some("List.remove() removes based on behavior (FIFO/LIFO)".to_string()),
                                        None
                                    ));
                                }
                                Ok(element_type.as_ref().clone()) // Returns the removed element type
                            },
                            "peek" => {
                                if !arguments.is_empty() {
                                    return Err(CompilerError::type_error(
                                        "List.peek() takes no arguments",
                                        Some("List.peek() views the next element without removing".to_string()),
                                        None
                                    ));
                                }
                                Ok(element_type.as_ref().clone()) // Returns the element type
                            },
                            "contains" => {
                                if arguments.len() != 1 {
                                    return Err(CompilerError::type_error(
                                        "List.contains() expects exactly 1 argument",
                                        Some("List.contains(item) requires one item argument".to_string()),
                                        None
                                    ));
                                }
                                if !self.types_compatible(element_type, &arg_types[0]) {
                                    return Err(CompilerError::type_error(
                                        &format!("List.contains() expects {:?}, found {:?}", element_type, arg_types[0]),
                                        Some("The item type must match the list's element type".to_string()),
                                        None
                                    ));
                                }
                                Ok(Type::Boolean)
                            },
                            "size" => {
                                if !arguments.is_empty() {
                                    return Err(CompilerError::type_error(
                                        "List.size() takes no arguments",
                                        Some("List.size() returns the number of elements".to_string()),
                                        None
                                    ));
                                }
                                Ok(Type::Integer)
                            },
                            "get" => {
                                if arguments.len() != 1 {
                                    return Err(CompilerError::type_error(
                                        "List.get() expects exactly 1 argument",
                                        Some("List.get(index) requires one index argument".to_string()),
                                        None
                                    ));
                                }
                                if !matches!(arg_types[0], Type::Integer) {
                                    return Err(CompilerError::type_error(
                                        "List.get() requires an integer index",
                                        Some("List indices must be integers".to_string()),
                                        None
                                    ));
                                }
                                Ok(element_type.as_ref().clone())
                            },
                            "set" => {
                                if arguments.len() != 2 {
                                    return Err(CompilerError::type_error(
                                        "List.set() expects exactly 2 arguments",
                                        Some("List.set(index, item) requires index and item arguments".to_string()),
                                        None
                                    ));
                                }
                                if !matches!(arg_types[0], Type::Integer) {
                                    return Err(CompilerError::type_error(
                                        "List.set() requires an integer index",
                                        Some("List indices must be integers".to_string()),
                                        None
                                    ));
                                }
                                if !self.types_compatible(element_type, &arg_types[1]) {
                                    return Err(CompilerError::type_error(
                                        &format!("List.set() expects {:?}, found {:?}", element_type, arg_types[1]),
                                        Some("The item type must match the list's element type".to_string()),
                                        None
                                    ));
                                }
                                Ok(Type::Boolean) // Returns success/failure
                            },
                            _ => {
                                Err(CompilerError::type_error(
                                    &format!("Method '{}' not found on List type", method),
                                    Some("Available methods: add(item), remove(), peek(), contains(item), size(), get(index), set(index, item)".to_string()),
                                    None
                                ))
                            }
                        }
                    },
                    Type::Object(class_name) => {
                        // Check if the class exists and has this method
                        if let Some(class_def) = self.class_table.get(class_name) {
                            // Look for the method in the class
                            for method_def in &class_def.methods {
                                if method_def.name == *method {
                                    // Found the method - check parameter types
                                    if arguments.len() != method_def.parameters.len() {
                                        return Err(CompilerError::type_error(
                                            &format!("Method '{}' expects {} arguments, but {} were provided", 
                                                method, method_def.parameters.len(), arguments.len()),
                                            Some("Check the method signature".to_string()),
                                            None
                                        ));
                                    }
                                    
                                    // Check parameter types match
                                    for (i, (arg_type, param)) in arg_types.iter().zip(method_def.parameters.iter()).enumerate() {
                                        if !self.types_compatible(&param.type_, arg_type) {
                                            return Err(CompilerError::type_error(
                                                &format!("Argument {} has type {:?}, but parameter '{}' expects {:?}", 
                                                    i + 1, arg_type, param.name, param.type_),
                                                Some("Check the argument types match the method parameters".to_string()),
                                                None
                                            ));
                                        }
                                    }
                                    
                                    // Return the method's return type
                                    return Ok(method_def.return_type.clone());
                                }
                            }
                            
                            // Method not found in class
                            Err(CompilerError::type_error(
                                &format!("Method '{}' not found in class '{}'", method, class_name),
                                Some("Check if the method name is correct and defined in the class".to_string()),
                                None
                            ))
                        } else {
                            // Class not found
                            Err(CompilerError::type_error(
                                &format!("Class '{}' not found", class_name),
                                Some("Check if the class name is correct and defined".to_string()),
                                None
                            ))
                        }
                    },
                    _ => {
                        Err(CompilerError::type_error(
                        &format!("Cannot call method '{}' on type {:?}", method, object_type),
                            Some("Methods can only be called on objects, arrays, and lists".to_string()),
                            None
                    ))
                    }
                }
            },

            Expression::ArrayAccess(array, index) => {
        let array_type = self.check_expression(array)?;
        let index_type = self.check_expression(index)?;

                if index_type != Type::Integer {
            return Err(CompilerError::type_error(
                        &format!("Index must be integer, found {:?}", index_type),
                        Some("Use integer expressions for indices".to_string()),
                        None
                    ));
                }

        match array_type {
            Type::Array(element_type) => Ok(*element_type),
            Type::List(element_type) => Ok(*element_type),
            _ => Err(CompilerError::type_error(
                        &format!("Cannot index into type {:?}", array_type),
                        Some("Index access can only be used on array and list types".to_string()),
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
                        Some("Use integer expressions for matrix indices".to_string()),
                        None
                    ));
                }

        match matrix_type {
            Type::Matrix(element_type) => Ok(*element_type),
            _ => Err(CompilerError::type_error(
                        &format!("Cannot index into type {:?}", matrix_type),
                        Some("Matrix access can only be used on matrix types".to_string()),
                        None
                    ))
                }
            },

            Expression::StringInterpolation(parts) => {
                // Check each interpolated expression
                for part in parts {
                    if let StringPart::Interpolation(expr) = part {
                        self.check_expression(expr)?;
                    }
                }
                Ok(Type::String)
            },

            Expression::ObjectCreation { class_name, arguments, location } => {
                self.check_constructor_call(class_name, arguments, location)
            },

            Expression::StaticMethodCall { class_name, method, arguments, location } => {
                self.check_static_method_call(class_name, method, arguments, location)
            },

            Expression::OnError { expression, fallback, location: _ } => {
                let expr_type = self.check_expression(expression)?;
                let fallback_type = self.check_expression(fallback)?;
                
                if !self.types_compatible(&expr_type, &fallback_type) {
                                return Err(CompilerError::type_error(
                        &format!("onError fallback type {:?} doesn't match expression type {:?}", fallback_type, expr_type),
                        Some("Ensure the fallback value has the same type as the main expression".to_string()),
                        None
                    ));
                }
                
                Ok(expr_type)
            },
            
            Expression::OnErrorBlock { expression, error_handler, location: _ } => {
                let expr_type = self.check_expression(expression)?;
                
                // Check the error handler statements in a new scope with error variable
                self.push_error_scope();
                for stmt in error_handler {
                    self.check_statement(stmt)?;
                }
                self.pop_error_scope();
                
                Ok(expr_type)
            },
            
            Expression::ErrorVariable { location } => {
                if !self.in_error_context() {
                    return Err(CompilerError::enhanced_type_error(
                        "Error variable 'error' can only be used in onError blocks".to_string(),
                        Some("onError block".to_string()),
                        Some("global scope".to_string()),
                        Some(location.clone()),
                        vec![
                            "Move this usage inside an onError block".to_string(),
                            "Use 'expression onError: error' syntax to access error information".to_string(),
                            "The error variable contains message, code, and location properties".to_string(),
                        ]
                    ));
                }
                
                // Error variable has a special error type with message, code, and location
                Ok(self.create_error_type())
            },
            
            Expression::Conditional { condition, then_expr, else_expr, location: _ } => {
                // Check that condition is boolean
                let condition_type = self.check_expression(condition)?;
                if condition_type != Type::Boolean {
                    return Err(CompilerError::type_error(
                        &format!("Conditional expression condition must be boolean, found {:?}", condition_type),
                        Some("Use a boolean expression for the condition".to_string()),
                        None
                    ));
                }
                
                // Check both branches and ensure they have compatible types
                let then_type = self.check_expression(then_expr)?;
                let else_type = self.check_expression(else_expr)?;
                
                if self.types_compatible(&then_type, &else_type) {
                    // Return the more general type if they're compatible
                    if then_type == else_type {
                        Ok(then_type)
                    } else {
                        // Handle type promotion (e.g., integer + float = float)
                        match (&then_type, &else_type) {
                            (Type::Integer, Type::Float) | (Type::Float, Type::Integer) => Ok(Type::Float),
                            (Type::IntegerSized { .. }, Type::Float) | (Type::Float, Type::IntegerSized { .. }) => Ok(Type::Float),
                            (Type::FloatSized { .. }, Type::Integer) | (Type::Integer, Type::FloatSized { .. }) => Ok(Type::Float),
                            _ => Ok(then_type) // Default to then type
                        }
                    }
                } else {
                    Err(CompilerError::type_error(
                        &format!("Conditional expression branches have incompatible types: {:?} and {:?}", then_type, else_type),
                        Some("Ensure both branches of the conditional expression return the same type".to_string()),
                        None
                    ))
                }
            },
            Expression::BaseCall { arguments, location } => {
                self.check_base_call(arguments, location)
            },
        }
    }

    fn check_binary_operation(&mut self, op: &BinaryOperator, left: &Expression, right: &Expression, location: &Option<SourceLocation>) -> Result<Type, CompilerError> {
        let left_type = self.check_expression(left)?;
        let right_type = self.check_expression(right)?;

        match op {
            BinaryOperator::Add | BinaryOperator::Subtract | BinaryOperator::Multiply | BinaryOperator::Divide | BinaryOperator::Modulo | BinaryOperator::Power => {
                // Type-based overloading for matrices
                match (&left_type, &right_type) {
                    (Type::Matrix(element_type1), Type::Matrix(element_type2)) => {
                        if self.types_compatible(element_type1, element_type2) {
                            Ok(left_type)
                } else {
                            Err(CompilerError::type_error(
                                &format!("Matrix operation requires same element types, found {:?} and {:?}", element_type1, element_type2),
                                Some("Ensure both matrices have the same element type".to_string()),
                                location.clone()
                            ))
                        }
                    },
                    // Handle exact type matches
                    (Type::Integer, Type::Integer) => Ok(Type::Integer),
                    (Type::Float, Type::Float) => Ok(Type::Float),
                    (Type::Integer, Type::Float) | (Type::Float, Type::Integer) => Ok(Type::Float),
                    (Type::String, Type::String) if matches!(op, BinaryOperator::Add) => Ok(Type::String),
                    
                    // Handle sized integer types
                    (Type::IntegerSized { .. }, Type::IntegerSized { .. }) => Ok(left_type), // Return left type for consistency
                    (Type::IntegerSized { .. }, Type::Integer) | (Type::Integer, Type::IntegerSized { .. }) => {
                        // When mixing sized and unsized integers, return the more specific type
                        match (&left_type, &right_type) {
                            (Type::IntegerSized { .. }, Type::Integer) => Ok(left_type),
                            (Type::Integer, Type::IntegerSized { .. }) => Ok(right_type),
                            _ => unreachable!()
                        }
                    },
                    
                    // Handle sized float types
                    (Type::FloatSized { .. }, Type::FloatSized { .. }) => Ok(left_type), // Return left type for consistency
                    (Type::FloatSized { .. }, Type::Float) | (Type::Float, Type::FloatSized { .. }) => {
                        // When mixing sized and unsized floats, return the more specific type
                        match (&left_type, &right_type) {
                            (Type::FloatSized { .. }, Type::Float) => Ok(left_type),
                            (Type::Float, Type::FloatSized { .. }) => Ok(right_type),
                            _ => unreachable!()
                        }
                    },
                    
                    // Handle mixed integer/float with sizing
                    (Type::IntegerSized { .. }, Type::Float) | (Type::Float, Type::IntegerSized { .. }) => Ok(Type::Float),
                    (Type::IntegerSized { .. }, Type::FloatSized { .. }) | (Type::FloatSized { .. }, Type::IntegerSized { .. }) => {
                        // When mixing integer and float types, return the float type
                        match (&left_type, &right_type) {
                            (Type::FloatSized { .. }, Type::IntegerSized { .. }) => Ok(left_type),
                            (Type::IntegerSized { .. }, Type::FloatSized { .. }) => Ok(right_type),
                            _ => unreachable!()
                        }
                    },
                    (Type::Integer, Type::FloatSized { .. }) | (Type::FloatSized { .. }, Type::Integer) => {
                        // When mixing unsized integer with sized float, return the sized float
                        match (&left_type, &right_type) {
                            (Type::FloatSized { .. }, Type::Integer) => Ok(left_type),
                            (Type::Integer, Type::FloatSized { .. }) => Ok(right_type),
                            _ => unreachable!()
                        }
                    },
                    
            _ => Err(CompilerError::type_error(
                        &format!("Cannot apply {:?} to types {:?} and {:?}", op, left_type, right_type),
                        Some("Ensure both operands have compatible types".to_string()),
                        location.clone()
                    ))
                }
            },

            BinaryOperator::Equal | BinaryOperator::NotEqual => {
                if self.types_compatible(&left_type, &right_type) {
                    Ok(Type::Boolean)
        } else {
                    Err(CompilerError::type_error(
                        &format!("Cannot compare types {:?} and {:?}", left_type, right_type),
                        Some("Ensure both operands have compatible types".to_string()),
                        location.clone()
                    ))
                }
            },

            BinaryOperator::Less | BinaryOperator::Greater | BinaryOperator::LessEqual | BinaryOperator::GreaterEqual => {
                match (&left_type, &right_type) {
                    (Type::Integer, Type::Integer) | (Type::Float, Type::Float) | 
                    (Type::Integer, Type::Float) | (Type::Float, Type::Integer) => Ok(Type::Boolean),
                    _ => Err(CompilerError::type_error(
                        &format!("Cannot compare types {:?} and {:?}", left_type, right_type),
                        Some("Comparison operators require numeric types".to_string()),
                        location.clone()
                    ))
                }
            },

            BinaryOperator::And | BinaryOperator::Or => {
                if left_type == Type::Boolean && right_type == Type::Boolean {
                    Ok(Type::Boolean)
            } else {
                    Err(CompilerError::type_error(
                        &format!("Logical operators require boolean operands, found {:?} and {:?}", left_type, right_type),
                        Some("Use boolean expressions with logical operators".to_string()),
                        location.clone()
                    ))
                }
            },

            BinaryOperator::Is | BinaryOperator::Not => {
                // Identity comparison - can compare any types
                Ok(Type::Boolean)
            }
        }
    }

    fn types_compatible(&self, left: &Type, right: &Type) -> bool {
        // Any type is compatible with any other type
        if matches!(left, Type::Any) || matches!(right, Type::Any) {
            return true;
        }
        
        match (left, right) {
            (Type::Boolean, Type::Boolean) => true,
            (Type::Integer, Type::Integer) => true,
            (Type::Float, Type::Float) => true,
            (Type::String, Type::String) => true,
            (Type::Void, Type::Void) => true,
            (Type::Array(left_inner), Type::Array(right_inner)) => self.types_compatible(left_inner, right_inner),
            (Type::List(left_inner), Type::List(right_inner)) => self.types_compatible(left_inner, right_inner),
            (Type::Matrix(left_inner), Type::Matrix(right_inner)) => self.types_compatible(left_inner, right_inner),
            (Type::Pairs(left_first, left_second), Type::Pairs(right_first, right_second)) => {
                self.types_compatible(left_first, right_first) && self.types_compatible(left_second, right_second)
            },
            (Type::Object(left_name), Type::Object(right_name)) => left_name == right_name,
            (Type::Generic(left_name, left_params), Type::Generic(right_name, right_params)) => {
                if left_name != right_name || left_params.len() != right_params.len() {
                    return false;
                }
                left_params.iter().zip(right_params.iter()).all(|(left, right)| self.types_compatible(left, right))
            },
            (Type::TypeParameter(left_name), Type::TypeParameter(right_name)) => left_name == right_name,
            // Sized types
            (Type::IntegerSized { bits: left_bits, .. }, Type::IntegerSized { bits: right_bits, .. }) => left_bits == right_bits,
            (Type::FloatSized { bits: left_bits }, Type::FloatSized { bits: right_bits }) => left_bits == right_bits,
            // Standard conversions
            (Type::Integer, Type::Float) => true,
            (Type::Float, Type::Integer) => true,
            _ => false,
        }
    }

    fn value_to_type(value: &Value) -> Type {
        match value {
            Value::Integer(_) => Type::Integer,
            Value::Float(_) => Type::Float,
            Value::Boolean(_) => Type::Boolean,
            Value::String(_) => Type::String,
            Value::Array(elements) => {
                if elements.is_empty() {
                    Type::Array(Box::new(Type::Any))
                    } else {
                    let element_type = Self::value_to_type(&elements[0]);
                    Type::Array(Box::new(element_type))
                }
            },
            Value::List(elements, _behavior) => {
                if elements.is_empty() {
                    Type::List(Box::new(Type::Any))
                } else {
                    let element_type = Self::value_to_type(&elements[0]);
                    Type::List(Box::new(element_type))
                }
            },
            Value::Matrix(_) => Type::Matrix(Box::new(Type::Float)),
            Value::Void => Type::Void,
            Value::Integer8(_) => Type::IntegerSized { bits: 8, unsigned: false },
            Value::Integer8u(_) => Type::IntegerSized { bits: 8, unsigned: true },
            Value::Integer16(_) => Type::IntegerSized { bits: 16, unsigned: false },
            Value::Integer16u(_) => Type::IntegerSized { bits: 16, unsigned: true },
            Value::Integer32(_) => Type::IntegerSized { bits: 32, unsigned: false },
            Value::Integer64(_) => Type::IntegerSized { bits: 64, unsigned: false },
            Value::Float32(_) => Type::FloatSized { bits: 32 },
            Value::Float64(_) => Type::FloatSized { bits: 64 },
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
        let param_types: Vec<Type> = constructor.parameters.iter()
            .map(|param| {
                if matches!(param.type_, Type::Any) {
                    // Try to infer type from class field with matching name
                    class.fields.iter()
                        .find(|field| field.name == param.name)
                        .map(|field| field.type_.clone())
                        .unwrap_or(Type::Any)
                } else {
                    param.type_.clone()
                }
            })
            .collect();
            
        for (i, (arg, param_type)) in args.iter().zip(param_types.iter()).enumerate() {
            let arg_type = self.check_expression(arg)?;
            if !self.types_compatible(&arg_type, param_type) {
                return Err(CompilerError::type_error(
                    &format!("Argument {} has type {:?}, but constructor parameter expects {:?}",
                        i + 1, arg_type, param_type),
                    Some("Provide arguments of the correct type".to_string()),
                    Some(self.get_expr_location(arg))
                ));
            }
        }

        Ok(Type::Object(class_name.to_string()))
    }

    fn check_type(&mut self, type_: &Type) -> Result<(), CompilerError> {
        // You can expand this with actual type checking logic if needed
        match type_ {
            Type::Object(class_name) => {
                if !self.class_table.contains_key(class_name) {
                    return Err(CompilerError::type_error(
                        &format!("Class '{}' not found", class_name),
                        Some("Check if the class name is correct and defined".to_string()),
                        None
                    ));
                }
                Ok(())
            },
            Type::Array(element_type) => self.check_type(element_type),
            Type::List(element_type) => self.check_type(element_type),
            Type::Matrix(element_type) => self.check_type(element_type),
            Type::Pairs(key_type, value_type) => {
                // Check both key and value types
                self.check_type(key_type)?;
                self.check_type(value_type)?;
                Ok(())
            },
            Type::Generic(base_type, type_args) => {
                self.check_type(base_type)?;
                for arg in type_args {
                    self.check_type(arg)?;
                }
                Ok(())
            },
            _ => Ok(()) // Other primitives types are always valid
        }
    }

    /// Resolve type parameters that might actually be class names
    fn resolve_type(&self, type_: &Type) -> Type {
        match type_ {
            Type::TypeParameter(name) => {
                if self.class_table.contains_key(name) {
                    Type::Object(name.clone())
                } else {
                    type_.clone()
                }
            },
            Type::Array(element_type) => {
                Type::Array(Box::new(self.resolve_type(element_type)))
            },
            Type::List(element_type) => {
                Type::List(Box::new(self.resolve_type(element_type)))
            },
            Type::Matrix(element_type) => {
                Type::Matrix(Box::new(self.resolve_type(element_type)))
            },
            Type::Pairs(key_type, value_type) => {
                Type::Pairs(
                    Box::new(self.resolve_type(key_type)),
                    Box::new(self.resolve_type(value_type))
                )
            },
            Type::Generic(base_type, type_args) => {
                let resolved_base = Box::new(self.resolve_type(base_type));
                let resolved_args = type_args.iter().map(|arg| self.resolve_type(arg)).collect();
                Type::Generic(resolved_base, resolved_args)
            },
            _ => type_.clone()
        }
    }

    fn check_literal(&mut self, value: &Value) -> Type {
        match value {
            Value::Integer(_) => Type::Integer,
            Value::Boolean(_) => Type::Boolean,
            Value::String(_) => Type::String,
            Value::Array(elements) => {
                if elements.is_empty() {
                    Type::Array(Box::new(Type::Any))
                } else {
                    let element_type = self.check_literal(&elements[0]);
                    Type::Array(Box::new(element_type))
                }
            },
            Value::List(elements, _behavior) => {
                if elements.is_empty() {
                    Type::List(Box::new(Type::Any))
                } else {
                    let element_type = self.check_literal(&elements[0]);
                    Type::List(Box::new(element_type))
                }
            },
            Value::Matrix(rows) => {
                if rows.is_empty() || rows[0].is_empty() {
                    Type::Matrix(Box::new(Type::Any))
                } else {
                    // Matrix contains f64 elements, not Value objects
                    Type::Matrix(Box::new(Type::Float))
                }
            },
            Value::Float(_) => Type::Float,
            Value::Void => Type::Void,
            Value::Integer8(_) => Type::IntegerSized { bits: 8, unsigned: false },
            Value::Integer8u(_) => Type::IntegerSized { bits: 8, unsigned: true },
            Value::Integer16(_) => Type::IntegerSized { bits: 16, unsigned: false },
            Value::Integer16u(_) => Type::IntegerSized { bits: 16, unsigned: true },
            Value::Integer32(_) => Type::IntegerSized { bits: 32, unsigned: false },
            Value::Integer64(_) => Type::IntegerSized { bits: 64, unsigned: false },
            Value::Float32(_) => Type::FloatSized { bits: 32 },
            Value::Float64(_) => Type::FloatSized { bits: 64 },
        }
    }

    fn analyze_literal(&self, value: &Value) -> Type {
        match value {
            Value::Integer(_) => Type::Integer,
            Value::Boolean(_) => Type::Boolean,
            Value::String(_) => Type::String,
            Value::Array(_) => Type::Array(Box::new(Type::Any)),
            Value::List(_, _) => Type::List(Box::new(Type::Any)),
            Value::Matrix(_) => Type::Matrix(Box::new(Type::Float)),
            Value::Float(_) => Type::Float,
            Value::Void => Type::Void,
            Value::Integer8(_) => Type::IntegerSized { bits: 8, unsigned: false },
            Value::Integer8u(_) => Type::IntegerSized { bits: 8, unsigned: true },
            Value::Integer16(_) => Type::IntegerSized { bits: 16, unsigned: false },
            Value::Integer16u(_) => Type::IntegerSized { bits: 16, unsigned: true },
            Value::Integer32(_) => Type::IntegerSized { bits: 32, unsigned: false },
            Value::Integer64(_) => Type::IntegerSized { bits: 64, unsigned: false },
            Value::Float32(_) => Type::FloatSized { bits: 32 },
            Value::Float64(_) => Type::FloatSized { bits: 64 },
        }
    }

    fn check_method_call(&mut self, object: &Expression, method: &str, args: &[Expression], location: &SourceLocation) -> Result<Type, CompilerError> {
        let object_type = self.check_expression(object)?;
        
        match object_type {
            Type::Object(class_name) => {
                // Use inheritance hierarchy to find the method
                if let Some((defining_class, method_def)) = self.find_method_in_hierarchy(&class_name, method) {
                    if args.len() != method_def.parameters.len() {
                        return Err(CompilerError::type_error(
                            &format!("Method '{}' expects {} arguments, but {} were provided", 
                                method, method_def.parameters.len(), args.len()),
                            Some("Provide the correct number of arguments".to_string()),
                            Some(location.clone())
                        ));
                    }

                    for (i, (arg, param)) in args.iter().zip(method_def.parameters.iter()).enumerate() {
                        let arg_type = self.check_expression(arg)?;
                        if !self.types_compatible(&param.type_, &arg_type) {
                            return Err(CompilerError::type_error(
                                &format!("Argument {} has type {:?}, but method parameter expects {:?}", 
                                    i + 1, arg_type, param.type_),
                                Some("Provide arguments of the correct type".to_string()),
                                Some(location.clone())
                            ));
                        }
                    }

                    return Ok(method_def.return_type.clone());
                } else {
                    Err(CompilerError::type_error(
                        &format!("Method '{}' not found in class '{}' or its parent classes", method, class_name),
                        Some("Check if the method name is correct and defined in the class hierarchy".to_string()),
                        Some(location.clone())
                    ))
                }
            },
            _ => Err(CompilerError::type_error(
                &format!("Cannot call method '{}' on type {:?}", method, object_type),
                Some("Methods can only be called on objects".to_string()),
                Some(location.clone())
            ))
        }
    }

    fn check_static_method_call(&mut self, class_name: &str, method: &str, args: &[Expression], location: &SourceLocation) -> Result<Type, CompilerError> {
        // Check if this is a built-in system class first
        if let Some(return_type) = self.check_builtin_static_method(class_name, method, args, location)? {
            return Ok(return_type);
        }

        // Check user-defined classes - clone the class definition to avoid borrowing issues
        if let Some(class_def) = self.class_table.get(class_name).cloned() {
            // Look for the static method in the class
            for method_def in &class_def.methods {
                if method_def.name == method {
                    // Check if this method is static (for now, assume all methods can be static)
                    
                    // Check parameter count
                    if args.len() != method_def.parameters.len() {
                        return Err(CompilerError::type_error(
                            &format!("Static method '{}::{}' expects {} arguments, but {} were provided", 
                                class_name, method, method_def.parameters.len(), args.len()),
                            Some("Check the method signature".to_string()),
                            None
                        ));
                    }
                    
                    // Check parameter types
                    for (i, (arg, param)) in args.iter().zip(method_def.parameters.iter()).enumerate() {
                        let arg_type = self.check_expression(arg)?;
                        if !self.types_compatible(&param.type_, &arg_type) {
                            return Err(CompilerError::type_error(
                                &format!("Argument {} has type {:?}, but parameter '{}' expects {:?}", 
                                    i + 1, arg_type, param.name, param.type_),
                                Some("Check that argument types match the method parameters".to_string()),
                                None
                            ));
                        }
                    }
                    
                    // Return the method's return type
                    return Ok(method_def.return_type.clone());
                }
            }
            
            // Method not found in class
            Err(CompilerError::type_error(
                &format!("Static method '{}' not found in class '{}'", method, class_name),
                Some("Check if the method name is correct and defined in the class".to_string()),
                None
            ))
        } else {
            // Class not found
            Err(CompilerError::type_error(
                &format!("Class '{}' not found", class_name),
                Some("Check if the class name is correct and defined".to_string()),
                None
            ))
        }
    }

    fn check_builtin_static_method(&mut self, class_name: &str, method: &str, args: &[Expression], location: &SourceLocation) -> Result<Option<Type>, CompilerError> {
        match class_name {
            "MathUtils" => {
                match method {
                    "add" | "subtract" | "multiply" | "divide" | "modulo" => {
                        if args.len() != 2 {
                            return Err(CompilerError::type_error(
                                &format!("MathUtils.{} expects 2 arguments, but {} were provided", method, args.len()),
                                Some("Provide exactly 2 numeric arguments".to_string()),
                                Some(location.clone())
                            ));
                        }
                        let arg1_type = self.check_expression(&args[0])?;
                        let arg2_type = self.check_expression(&args[1])?;

                        // Both arguments should be numeric
                        let numeric_types = [Type::Integer, Type::Float];
                        if !numeric_types.contains(&arg1_type) || !numeric_types.contains(&arg2_type) {
                            return Err(CompilerError::type_error(
                                &format!("MathUtils.{} requires numeric arguments", method),
                                Some("Use integer or float values".to_string()),
                                Some(location.clone())
                            ));
                        }
                        
                        // Return float if any argument is float, otherwise integer
                        if arg1_type == Type::Float || arg2_type == Type::Float {
                            Ok(Some(Type::Float))
                        } else {
                            Ok(Some(Type::Integer))
                        }
                    },
                    "abs" | "sqrt" | "floor" | "ceil" | "round" | "sin" | "cos" | "tan" | "log" | "exp" => {
                        if args.len() != 1 {
                            return Err(CompilerError::type_error(
                                &format!("MathUtils.{} expects 1 argument, but {} were provided", method, args.len()),
                                Some("Provide exactly 1 numeric argument".to_string()),
                                Some(location.clone())
                            ));
                        }
                        let arg_type = self.check_expression(&args[0])?;
                        let numeric_types = [Type::Integer, Type::Float];
                        if !numeric_types.contains(&arg_type) {
                            return Err(CompilerError::type_error(
                                &format!("MathUtils.{} requires a numeric argument", method),
                                Some("Use integer or float values".to_string()),
                                Some(location.clone())
                            ));
                        }
                        
                        // Most math functions return float
                        if method == "abs" && arg_type == Type::Integer {
                            Ok(Some(Type::Integer))
                        } else {
                            Ok(Some(Type::Float))
                        }
                    },
                    "min" | "max" | "pow" => {
                        if args.len() != 2 {
                            return Err(CompilerError::type_error(
                                &format!("MathUtils.{} expects 2 arguments, but {} were provided", method, args.len()),
                                Some("Provide exactly 2 numeric arguments".to_string()),
                                Some(location.clone())
                            ));
                        }
                        let arg1_type = self.check_expression(&args[0])?;
                        let arg2_type = self.check_expression(&args[1])?;
                        
                        let numeric_types = [Type::Integer, Type::Float];
                        if !numeric_types.contains(&arg1_type) || !numeric_types.contains(&arg2_type) {
                            return Err(CompilerError::type_error(
                                &format!("MathUtils.{} requires numeric arguments", method),
                                Some("Use integer or float values".to_string()),
                                Some(location.clone())
                            ));
                        }
                        
                        // Return the more general type
                        if arg1_type == Type::Float || arg2_type == Type::Float {
                            Ok(Some(Type::Float))
                        } else {
                            Ok(Some(Type::Integer))
                        }
                    },
                    "clamp" => {
                        if args.len() != 3 {
                            return Err(CompilerError::type_error(
                                &format!("MathUtils.clamp expects 3 arguments, but {} were provided", args.len()),
                                Some("Provide exactly 3 numeric arguments (value, min, max)".to_string()),
                                Some(location.clone())
                            ));
                        }
                        // Check all arguments are numeric
                        for (i, arg) in args.iter().enumerate() {
                            let arg_type = self.check_expression(arg)?;
                            let numeric_types = [Type::Integer, Type::Float];
                            if !numeric_types.contains(&arg_type) {
                                return Err(CompilerError::type_error(
                                    &format!("MathUtils.clamp argument {} must be numeric", i + 1),
                                    Some("Use integer or float values".to_string()),
                                    Some(location.clone())
                                ));
                            }
                        }
                        // Return float if any argument is float
                        let mut result_type = Type::Integer;
                        for arg in args {
                            let arg_type = self.check_expression(arg)?;
                            if arg_type == Type::Float {
                                result_type = Type::Float;
                                break;
                            }
                        }
                        Ok(Some(result_type))
                    },
                    _ => Ok(None), // Method not found in MathUtils
                }
            },
            "StringUtils" => {
                match method {
                    "length" => {
                        if args.len() != 1 {
                            return Err(CompilerError::type_error(
                                &format!("StringUtils.length expects 1 argument, but {} were provided", args.len()),
                                Some("Provide exactly 1 string argument".to_string()),
                                Some(location.clone())
                            ));
                        }
                        let arg_type = self.check_expression(&args[0])?;
                        if arg_type != Type::String {
                            return Err(CompilerError::type_error(
                                "StringUtils.length requires a string argument".to_string(),
                                Some("Use a string value".to_string()),
                                Some(location.clone())
                            ));
                        }
                        Ok(Some(Type::Integer))
                    },
                    "concat" => {
                        if args.len() != 2 {
                            return Err(CompilerError::type_error(
                                &format!("StringUtils.concat expects 2 arguments, but {} were provided", args.len()),
                                Some("Provide exactly 2 string arguments".to_string()),
                                Some(location.clone())
                            ));
                        }
                        let arg1_type = self.check_expression(&args[0])?;
                        let arg2_type = self.check_expression(&args[1])?;
                        if arg1_type != Type::String || arg2_type != Type::String {
                            return Err(CompilerError::type_error(
                                "StringUtils.concat requires string arguments".to_string(),
                                Some("Use string values".to_string()),
                                Some(location.clone())
                            ));
                        }
                        Ok(Some(Type::String))
                    },
                    "toUpper" | "toLower" | "trim" => {
                        if !args.is_empty() {
                            return Err(CompilerError::type_error(
                                &format!("StringUtils.{} expects 1 argument, but {} were provided", method, args.len()),
                                Some("Provide exactly 1 string argument".to_string()),
                                Some(location.clone())
                            ));
                        }
                        let arg_type = self.check_expression(&args[0])?;
                        if arg_type != Type::String {
                            return Err(CompilerError::type_error(
                                &format!("StringUtils.{} requires a string argument", method),
                                Some("Use a string value".to_string()),
                                Some(location.clone())
                            ));
                        }
                        Ok(Some(Type::String))
                    },
                    "startsWith" | "endsWith" => {
                        if args.len() != 2 {
                            return Err(CompilerError::type_error(
                                &format!("StringUtils.{} expects 2 arguments, but {} were provided", method, args.len()),
                                Some("Provide exactly 2 string arguments (string, prefix/suffix)".to_string()),
                                Some(location.clone())
                            ));
                        }
                        let arg1_type = self.check_expression(&args[0])?;
                        let arg2_type = self.check_expression(&args[1])?;
                        if arg1_type != Type::String || arg2_type != Type::String {
                            return Err(CompilerError::type_error(
                                &format!("StringUtils.{} requires string arguments", method),
                                Some("Use string values".to_string()),
                                Some(location.clone())
                            ));
                        }
                        Ok(Some(Type::Boolean))
                    },
                    "indexOf" => {
                        if args.len() != 2 {
                            return Err(CompilerError::type_error(
                                "StringUtils.indexOf expects 2 arguments, but {} were provided",
                                Some("Provide exactly 2 string arguments (string, substring)".to_string()),
                                Some(location.clone())
                            ));
                        }
                        let arg1_type = self.check_expression(&args[0])?;
                        let arg2_type = self.check_expression(&args[1])?;
                        if arg1_type != Type::String || arg2_type != Type::String {
                            return Err(CompilerError::type_error(
                                "StringUtils.indexOf requires string arguments",
                                Some("Use string values".to_string()),
                                Some(location.clone())
                            ));
                        }
                        Ok(Some(Type::Integer))
                    },
                    "substring" => {
                        if args.len() < 2 || args.len() > 3 {
                            return Err(CompilerError::type_error(
                                &format!("StringUtils.substring expects 2 or 3 arguments, but {} were provided", args.len()),
                                Some("Provide string, start index, and optionally end index".to_string()),
                                Some(location.clone())
                            ));
                        }
                        let arg1_type = self.check_expression(&args[0])?;
                        let arg2_type = self.check_expression(&args[1])?;
                        if arg1_type != Type::String || arg2_type != Type::Integer {
                            return Err(CompilerError::type_error(
                                "StringUtils.substring requires string and integer arguments",
                                Some("Use string for text and integer for indices".to_string()),
                                Some(location.clone())
                            ));
                        }
                        if args.len() == 3 {
                            let arg3_type = self.check_expression(&args[2])?;
                            if arg3_type != Type::Integer {
                                return Err(CompilerError::type_error(
                                    "StringUtils.substring end index must be an integer",
                                    Some("Use integer for end index".to_string()),
                                    Some(location.clone())
                                ));
                            }
                        }
                        Ok(Some(Type::String))
                    },
                    "replace" => {
                        if args.len() != 3 {
                            return Err(CompilerError::type_error(
                                &format!("StringUtils.replace expects 3 arguments, but {} were provided", args.len()),
                                Some("Provide exactly 3 string arguments (string, search, replacement)".to_string()),
                                Some(location.clone())
                            ));
                        }
                        let arg1_type = self.check_expression(&args[0])?;
                        let arg2_type = self.check_expression(&args[1])?;
                        let arg3_type = self.check_expression(&args[2])?;
                        if arg1_type != Type::String || arg2_type != Type::String || arg3_type != Type::String {
                            return Err(CompilerError::type_error(
                                "StringUtils.replace requires string arguments",
                                Some("Use string values".to_string()),
                                Some(location.clone())
                            ));
                        }
                        Ok(Some(Type::String))
                    },
                    "split" => {
                        if args.len() != 2 {
                            return Err(CompilerError::type_error(
                                &format!("StringUtils.split expects 2 arguments, but {} were provided", args.len()),
                                Some("Provide exactly 2 string arguments (string, delimiter)".to_string()),
                                Some(location.clone())
                            ));
                        }
                        let arg1_type = self.check_expression(&args[0])?;
                        let arg2_type = self.check_expression(&args[1])?;
                        if arg1_type != Type::String || arg2_type != Type::String {
                            return Err(CompilerError::type_error(
                                "StringUtils.split requires string arguments",
                                Some("Use string values".to_string()),
                                Some(location.clone())
                            ));
                        }
                        Ok(Some(Type::Array(Box::new(Type::String))))
                    },
                    _ => Ok(None), // Method not found in StringUtils
                }
            },
            "ArrayUtils" => {
                match method {
                    "length" => {
                        if args.len() != 1 {
                            return Err(CompilerError::type_error(
                                &format!("ArrayUtils.length expects 1 argument, but {} were provided", args.len()),
                                Some("Provide exactly 1 array argument".to_string()),
                                Some(location.clone())
                            ));
                        }
                        let arg_type = self.check_expression(&args[0])?;
                        if let Type::Array(_) = arg_type {
                            Ok(Some(Type::Integer))
                        } else {
                            Err(CompilerError::type_error(
                                "ArrayUtils.length requires an array argument".to_string(),
                                Some("Use an array value".to_string()),
                                Some(location.clone())
                            ))
                        }
                    },
                    "slice" => {
                        if args.len() < 2 || args.len() > 3 {
                            return Err(CompilerError::type_error(
                                &format!("ArrayUtils.slice expects 2 or 3 arguments, but {} were provided", args.len()),
                                Some("Provide array, start index, and optionally end index".to_string()),
                                Some(location.clone())
                            ));
                        }
                        let array_type = self.check_expression(&args[0])?;
                        let start_type = self.check_expression(&args[1])?;
                        if start_type != Type::Integer {
                            return Err(CompilerError::type_error(
                                "ArrayUtils.slice start index must be an integer",
                                Some("Use integer for start index".to_string()),
                                Some(location.clone())
                            ));
                        }
                        if args.len() == 3 {
                            let end_type = self.check_expression(&args[2])?;
                            if end_type != Type::Integer {
                                return Err(CompilerError::type_error(
                                    "ArrayUtils.slice end index must be an integer",
                                    Some("Use integer for end index".to_string()),
                                    Some(location.clone())
                                ));
                            }
                        }
                        if let Type::Array(_) = array_type {
                            Ok(Some(array_type)) // Return same array type
                        } else {
                            Err(CompilerError::type_error(
                                "ArrayUtils.slice requires an array argument".to_string(),
                                Some("Use an array value".to_string()),
                                Some(location.clone())
                            ))
                        }
                    },
                    "join" => {
                        if args.len() != 2 {
                            return Err(CompilerError::type_error(
                                &format!("ArrayUtils.join expects 2 arguments, but {} were provided", args.len()),
                                Some("Provide exactly 2 arguments (array, separator)".to_string()),
                                Some(location.clone())
                            ));
                        }
                        let array_type = self.check_expression(&args[0])?;
                        let separator_type = self.check_expression(&args[1])?;
                        if separator_type != Type::String {
                            return Err(CompilerError::type_error(
                                "ArrayUtils.join separator must be a string",
                                Some("Use string for separator".to_string()),
                                Some(location.clone())
                            ));
                        }
                        if let Type::Array(_) = array_type {
                            Ok(Some(Type::String)) // Join returns a string
                        } else {
                            Err(CompilerError::type_error(
                                "ArrayUtils.join requires an array argument".to_string(),
                                Some("Use an array value".to_string()),
                                Some(location.clone())
                            ))
                        }
                    },
                    "reverse" | "sort" => {
                        if args.len() != 1 {
                            return Err(CompilerError::type_error(
                                &format!("ArrayUtils.{} expects 1 argument, but {} were provided", method, args.len()),
                                Some("Provide exactly 1 array argument".to_string()),
                                Some(location.clone())
                            ));
                        }
                        let array_type = self.check_expression(&args[0])?;
                        if let Type::Array(_) = array_type {
                            Ok(Some(array_type)) // Return same array type
                        } else {
                            Err(CompilerError::type_error(
                                &format!("ArrayUtils.{} requires an array argument", method),
                                Some("Use an array value".to_string()),
                                Some(location.clone())
                            ))
                        }
                    },
                    "push" => {
                        if args.len() != 2 {
                            return Err(CompilerError::type_error(
                                &format!("ArrayUtils.push expects 2 arguments, but {} were provided", args.len()),
                                Some("Provide exactly 2 arguments (array, element)".to_string()),
                                Some(location.clone())
                            ));
                        }
                        let array_type = self.check_expression(&args[0])?;
                        let element_type = self.check_expression(&args[1])?;
                        if let Type::Array(element_array_type) = &array_type {
                            if !self.types_compatible(&element_type, element_array_type) {
                                return Err(CompilerError::type_error(
                                    "ArrayUtils.push element type doesn't match array element type",
                                    Some("Ensure the element type matches the array's element type".to_string()),
                                    Some(location.clone())
                                ));
                            }
                            Ok(Some(array_type)) // Return same array type
                        } else {
                            Err(CompilerError::type_error(
                                "ArrayUtils.push requires an array argument".to_string(),
                                Some("Use an array value".to_string()),
                                Some(location.clone())
                            ))
                        }
                    },
                    "pop" => {
                        if args.len() != 1 {
                            return Err(CompilerError::type_error(
                                &format!("ArrayUtils.pop expects 1 argument, but {} were provided", args.len()),
                                Some("Provide exactly 1 array argument".to_string()),
                                Some(location.clone())
                            ));
                        }
                        let array_type = self.check_expression(&args[0])?;
                        if let Type::Array(element_type) = array_type {
                            Ok(Some(*element_type)) // Return element type
                        } else {
                            Err(CompilerError::type_error(
                                "ArrayUtils.pop requires an array argument".to_string(),
                                Some("Use an array value".to_string()),
                                Some(location.clone())
                            ))
                        }
                    },
                    _ => Ok(None), // Method not found in ArrayUtils
                }
            },
            "File" => {
                match method {
                    "read" => {
                        if args.len() != 1 {
                            return Err(CompilerError::type_error(
                                &format!("File.read expects 1 argument, but {} were provided", args.len()),
                                Some("Provide exactly 1 string argument (file path)".to_string()),
                                Some(location.clone())
                            ));
                        }
                        let arg_type = self.check_expression(&args[0])?;
                        if arg_type != Type::String {
                            return Err(CompilerError::type_error(
                                "File.read requires a string argument (file path)".to_string(),
                                Some("Use a string value for the file path".to_string()),
                                Some(location.clone())
                            ));
                        }
                        Ok(Some(Type::String))
                    },
                    "write" | "append" => {
                        if args.len() != 2 {
                            return Err(CompilerError::type_error(
                                &format!("File.{} expects 2 arguments, but {} were provided", method, args.len()),
                                Some("Provide exactly 2 string arguments (file path, content)".to_string()),
                                Some(location.clone())
                            ));
                        }
                        let path_type = self.check_expression(&args[0])?;
                        let content_type = self.check_expression(&args[1])?;
                        if path_type != Type::String || content_type != Type::String {
                            return Err(CompilerError::type_error(
                                &format!("File.{} requires string arguments (file path, content)", method),
                                Some("Use string values for both file path and content".to_string()),
                                Some(location.clone())
                            ));
                        }
                        Ok(Some(Type::Void))
                    },
                    "exists" => {
                        if args.len() != 1 {
                            return Err(CompilerError::type_error(
                                &format!("File.exists expects 1 argument, but {} were provided", args.len()),
                                Some("Provide exactly 1 string argument (file path)".to_string()),
                                Some(location.clone())
                            ));
                        }
                        let arg_type = self.check_expression(&args[0])?;
                        if arg_type != Type::String {
                            return Err(CompilerError::type_error(
                                "File.exists requires a string argument (file path)".to_string(),
                                Some("Use a string value for the file path".to_string()),
                                Some(location.clone())
                            ));
                        }
                        Ok(Some(Type::Boolean))
                    },
                    "delete" => {
                        if args.len() != 1 {
                            return Err(CompilerError::type_error(
                                &format!("File.delete expects 1 argument, but {} were provided", args.len()),
                                Some("Provide exactly 1 string argument (file path)".to_string()),
                                Some(location.clone())
                            ));
                        }
                        let arg_type = self.check_expression(&args[0])?;
                        if arg_type != Type::String {
                            return Err(CompilerError::type_error(
                                "File.delete requires a string argument (file path)".to_string(),
                                Some("Use a string value for the file path".to_string()),
                                Some(location.clone())
                            ));
                        }
                        Ok(Some(Type::Void))
                    },
                    "lines" => {
                        if args.len() != 1 {
                            return Err(CompilerError::type_error(
                                &format!("File.lines expects 1 argument, but {} were provided", args.len()),
                                Some("Provide exactly 1 string argument (file path)".to_string()),
                                Some(location.clone())
                            ));
                        }
                        let arg_type = self.check_expression(&args[0])?;
                        if arg_type != Type::String {
                            return Err(CompilerError::type_error(
                                "File.lines requires a string argument (file path)".to_string(),
                                Some("Use a string value for the file path".to_string()),
                                Some(location.clone())
                            ));
                        }
                        Ok(Some(Type::List(Box::new(Type::String))))
                    },
                    _ => Ok(None), // Method not found in File
                }
            },
            "Http" => {
                match method {
                    "get" => {
                        if args.len() != 1 {
                            return Err(CompilerError::type_error(
                                &format!("Http.get expects 1 argument, but {} were provided", args.len()),
                                Some("Provide exactly 1 string argument (URL)".to_string()),
                                Some(location.clone())
                            ));
                        }
                        let arg_type = self.check_expression(&args[0])?;
                        if arg_type != Type::String {
                            return Err(CompilerError::type_error(
                                "Http.get requires a string argument (URL)".to_string(),
                                Some("Use a string value for the URL".to_string()),
                                Some(location.clone())
                            ));
                        }
                        Ok(Some(Type::String))
                    },
                    "post" | "put" | "patch" => {
                        if args.len() != 2 {
                            return Err(CompilerError::type_error(
                                &format!("Http.{} expects 2 arguments, but {} were provided", method, args.len()),
                                Some("Provide exactly 2 string arguments (URL, body)".to_string()),
                                Some(location.clone())
                            ));
                        }
                        let url_type = self.check_expression(&args[0])?;
                        let body_type = self.check_expression(&args[1])?;
                        if url_type != Type::String || body_type != Type::String {
                            return Err(CompilerError::type_error(
                                &format!("Http.{} requires string arguments (URL, body)", method),
                                Some("Use string values for both URL and request body".to_string()),
                                Some(location.clone())
                            ));
                        }
                        Ok(Some(Type::String))
                    },
                    "delete" => {
                        if args.len() != 1 {
                            return Err(CompilerError::type_error(
                                &format!("Http.delete expects 1 argument, but {} were provided", args.len()),
                                Some("Provide exactly 1 string argument (URL)".to_string()),
                                Some(location.clone())
                            ));
                        }
                        let arg_type = self.check_expression(&args[0])?;
                        if arg_type != Type::String {
                            return Err(CompilerError::type_error(
                                "Http.delete requires a string argument (URL)".to_string(),
                                Some("Use a string value for the URL".to_string()),
                                Some(location.clone())
                            ));
                        }
                        Ok(Some(Type::String))
                    },
                    _ => Ok(None), // Method not found in Http
                }
            },
            _ => Ok(None), // Class not found in built-ins
        }
    }

    /// Get all collected warnings
    pub fn get_warnings(&self) -> &[CompilerWarning] {
        &self.warnings
    }

    /// Clear all collected warnings
    pub fn clear_warnings(&mut self) {
        self.warnings.clear();
    }

    /// Add a warning to the collection
    fn add_warning(&mut self, warning: CompilerWarning) {
        self.warnings.push(warning);
    }

    /// Check for unused variables and functions after analysis
    fn check_unused_items(&mut self) {
        let mut warnings_to_add = Vec::new();
        
        // Check for unused variables
        for (var_name, _) in &self.symbol_table {
            if !self.used_variables.contains(var_name) {
                warnings_to_add.push(CompilerWarning::unused_variable(var_name, None));
            }
        }

        // Check for unused functions (except main/start functions)
        for (func_name, _) in &self.function_table {
            if !self.used_functions.contains(func_name) && 
               func_name != "main" && func_name != "start" {
                warnings_to_add.push(CompilerWarning::unused_function(func_name, None));
            }
        }
        
        // Add all warnings at once
        for warning in warnings_to_add {
            self.add_warning(warning);
        }
    }

    fn is_builtin_class(&self, class_name: &str) -> bool {
        matches!(class_name, "MathUtils" | "StringUtils" | "ArrayUtils" | "File" | "Http")
    }

    fn is_builtin_function(&self, function_name: &str) -> bool {
        matches!(function_name, 
            "print" | "printl" | "println" | "error" |
            "abs" | "array_get" | "array_length" | "assert" | 
            "string_concat" | "string_compare" |
            "http_get" | "http_post" | "http_put" | "http_delete" | "http_patch"
        )
    }

    fn is_type_conversion_method(&self, method: &str) -> bool {
        matches!(method, "toInteger" | "toFloat" | "toString" | "toBoolean")
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

    /// Check if a range iteration statement is valid
    fn check_range_iterate(&mut self, stmt: &Statement) -> Result<(), CompilerError> {
        if let Statement::RangeIterate { iterator, start, end, step, body, .. } = stmt {
            // Check start expression
            let start_type = self.check_expression(start)?;
            
            // Check end expression
            let end_type = self.check_expression(end)?;
            
            // Check step expression if present
            if let Some(step_expr) = step {
                let step_type = self.check_expression(step_expr)?;
                
                // Step must be numeric
                if !self.is_numeric_type(&step_type) {
                    return Err(CompilerError::type_error(
                        format!("Step expression must be numeric, got {}", step_type),
                        None,
                        None
                    ));
                }
                
                // Step must match start/end type
                if !self.types_compatible(&start_type, &step_type) || !self.types_compatible(&end_type, &step_type) {
                    return Err(CompilerError::type_error(
                        format!("Step type {} must match range type {}", step_type, start_type),
                        None,
                        None
                    ));
                }
            }
            
            // Start and end must be numeric
            if !self.is_numeric_type(&start_type) || !self.is_numeric_type(&end_type) {
                return Err(CompilerError::type_error(
                    format!("Range expressions must be numeric, got {} and {}", start_type, end_type),
                    None,
                    None
                ));
            }
            
            // Start and end must be compatible
            if !self.types_compatible(&start_type, &end_type) {
                return Err(CompilerError::type_error(
                    format!("Range expressions must have compatible types, got {} and {}", start_type, end_type),
                    None,
                    None
                ));
            }
            
            // Add iterator to symbol table
            self.symbol_table.insert(iterator.clone(), start_type.clone());
            
            // Check body
            self.check_statements(body)?;
            
            // Remove iterator from symbol table
            self.symbol_table.remove(iterator);
            
            Ok(())
        } else {
            Err(CompilerError::type_error(
                "Expected range iteration statement".to_string(),
                None,
                None
            ))
        }
    }

    /// Check if a type is numeric
    fn is_numeric_type(&self, type_: &Type) -> bool {
        matches!(type_, Type::Integer | Type::Float | Type::IntegerSized { .. } | Type::FloatSized { .. })
    }

    fn is_valid_type(&self, type_: &Type) -> bool {
        match type_ {
            Type::Boolean | Type::Integer | Type::Float | Type::String | Type::Void | Type::Any => true,
            Type::IntegerSized { bits, .. } => *bits == 8 || *bits == 16 || *bits == 32 || *bits == 64,
            Type::FloatSized { bits } => *bits == 32 || *bits == 64,
            Type::Array(inner) => self.is_valid_type(inner),
            Type::List(inner) => self.is_valid_type(inner),
            Type::Matrix(inner) => self.is_valid_type(inner),
            Type::Pairs(key, value) => self.is_valid_type(key) && self.is_valid_type(value),
            Type::Generic(base, args) => {
                self.is_valid_type(base) && args.iter().all(|arg| self.is_valid_type(arg))
            },
            Type::TypeParameter(name) => self.type_environment.contains(name),
            Type::Object(name) => self.class_table.contains_key(name),
            Type::Class { name, type_args } => {
                self.class_table.contains_key(name) && 
                type_args.iter().all(|arg| self.is_valid_type(arg))
            },
            Type::Function(params, return_type) => {
                params.iter().all(|param| self.is_valid_type(param)) && 
                self.is_valid_type(return_type)
            },
        }
    }

    fn check_statements(&mut self, statements: &[Statement]) -> Result<(), CompilerError> {
        for stmt in statements {
            self.check_statement(stmt)?;
        }
        Ok(())
    }

    fn is_builtin_type_constructor(&self, name: &str) -> bool {
        matches!(name, "List" | "Array" | "Matrix" | "Set" | "Map" | "Queue" | "Stack")
    }

    fn check_builtin_type_constructor(&mut self, name: &str, args: &[Expression]) -> Result<Type, CompilerError> {
        match name {
            "List" => {
                // List constructor can take 0 or more arguments
                Ok(Type::List(Box::new(Type::Any)))
            },
            _ => Err(CompilerError::type_error(
                &format!("Unknown built-in type constructor: {}", name),
                None,
                None
            ))
        }
    }

    /// Type-safe print function call validation following printf best practices
    /// Dispatches to appropriate type-specific print function based on argument type
    fn check_print_function_call(&mut self, func_name: &str, args: &[Expression]) -> Result<Type, CompilerError> {
        // Validate argument count
        if args.len() != 1 {
            return Err(CompilerError::type_error(
                &format!("Print function '{}' expects exactly 1 argument, got {}", func_name, args.len()),
                Some("Print functions accept a single value to output".to_string()),
                None
            ));
        }

        // Determine argument type for type-safe dispatch
        let arg_type = self.check_expression(&args[0])?;
        
        // Validate that the argument type is printable
        match arg_type {
            Type::Integer | Type::Float | Type::String | Type::Boolean => {
                // Type is valid for printing
                self.used_functions.insert(func_name.to_string());
                Ok(Type::Void)
            },
            Type::Object(class_name) if class_name == "Error" => {
                // Error objects can be printed (will show error message)
                self.used_functions.insert(func_name.to_string());
                Ok(Type::Void)
            },
            Type::Any => {
                // Allow Any type for backward compatibility, but warn about potential type safety issues
                self.add_warning(CompilerWarning::new(
                    format!("Print function '{}' called with 'Any' type - consider using specific types for better type safety", func_name),
                    WarningType::TypeInference,
                    None
                ));
                self.used_functions.insert(func_name.to_string());
                Ok(Type::Void)
            },
            _ => {
                Err(CompilerError::type_error(
                    &format!("Cannot print value of type {:?}", arg_type),
                    Some("Print functions support Integer, Float, String, Boolean, and Error types".to_string()),
                    None
                ))
            }
        }
    }

    // Inheritance support methods
    fn is_subclass(&self, child_name: &str, base_name: &str) -> bool {
        if child_name == base_name {
            return true;
        }

        if let Some(child_class) = self.class_table.get(child_name) {
            if let Some(parent_name) = &child_class.base_class {
                return self.is_subclass(parent_name, base_name);
            }
        }

        false
    }

    fn get_class_hierarchy(&self, class_name: &str) -> Vec<String> {
        let mut hierarchy = vec![class_name.to_string()];
        let mut current = class_name;

        while let Some(class) = self.class_table.get(current) {
            if let Some(base_class) = &class.base_class {
                hierarchy.push(base_class.clone());
                current = base_class;
            } else {
                break;
            }
        }

        hierarchy
    }

    fn find_method_in_hierarchy(&self, class_name: &str, method_name: &str) -> Option<(String, Function)> {
        let hierarchy = self.get_class_hierarchy(class_name);
        
        for class_name in hierarchy {
            if let Some(class) = self.class_table.get(&class_name) {
                for method in &class.methods {
                    if method.name == method_name {
                        return Some((class_name, method.clone()));
                    }
                }
            }
        }

        None
    }

    fn find_field_in_hierarchy(&self, class_name: &str, field_name: &str) -> Option<(String, Field)> {
        let hierarchy = self.get_class_hierarchy(class_name);
        
        for class_name in hierarchy {
            if let Some(class) = self.class_table.get(&class_name) {
                for field in &class.fields {
                    if field.name == field_name {
                        return Some((class_name, field.clone()));
                    }
                }
            }
        }

        None
    }

    fn check_method_override(&self, child_method: &Function, parent_method: &Function, child_class: &str, parent_class: &str) -> Result<(), CompilerError> {
        // Check that method signatures are compatible for overriding
        if child_method.parameters.len() != parent_method.parameters.len() {
            return Err(CompilerError::type_error(
                format!("Method '{}' in class '{}' has different parameter count than parent method in '{}'", 
                    child_method.name, child_class, parent_class),
                Some("Method overrides must have the same parameter count".to_string()),
                child_method.location.clone()
            ));
        }

        // Check parameter types
        for (i, (child_param, parent_param)) in child_method.parameters.iter().zip(parent_method.parameters.iter()).enumerate() {
            if !self.types_compatible(&child_param.type_, &parent_param.type_) {
                return Err(CompilerError::type_error(
                    format!("Parameter {} of method '{}' in class '{}' has incompatible type with parent method", 
                        i + 1, child_method.name, child_class),
                    Some("Method override parameters must have compatible types".to_string()),
                    child_method.location.clone()
                ));
            }
        }

        // Check return type (covariant return types allowed)
        if !self.types_compatible(&child_method.return_type, &parent_method.return_type) {
            return Err(CompilerError::type_error(
                format!("Method '{}' in class '{}' has incompatible return type with parent method", 
                    child_method.name, child_class),
                Some("Method override return type must be compatible with parent".to_string()),
                child_method.location.clone()
            ));
        }

        Ok(())
    }

    fn check_base_call(&mut self, arguments: &[Expression], location: &SourceLocation) -> Result<Type, CompilerError> {
        // Base calls are only valid in constructors
        if self.current_function.is_none() {
            return Err(CompilerError::type_error(
                "Base calls can only be used in constructors".to_string(),
                Some("Move the base call inside a constructor".to_string()),
                Some(location.clone())
            ));
        }

        // Get the current class
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