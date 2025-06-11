use std::collections::{HashMap, HashSet};
use crate::ast::*;
use crate::error::{CompilerError, CompilerWarning};

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
        };
        
        analyzer.register_builtin_functions();
        analyzer
    }
    
    // Register built-in functions like print, println, etc.
    fn register_builtin_functions(&mut self) {
        // print function
        self.function_table.insert(
            "print".to_string(),
            (vec![Type::Any], Type::Void)
        );
        
        // println function
        self.function_table.insert(
            "println".to_string(),
            (vec![Type::Any], Type::Void)
        );
        
        // printl function (print with newline)
        self.function_table.insert(
            "printl".to_string(),
            (vec![Type::Any], Type::Void)
        );
        
        // abs function (absolute value)
        self.function_table.insert(
            "abs".to_string(),
            (vec![Type::Integer], Type::Integer)
        );
        
        // array_get function
        self.function_table.insert(
            "array_get".to_string(),
            (vec![Type::Array(Box::new(Type::Integer)), Type::Integer], Type::Integer)
        );
        
        // array_length function
        self.function_table.insert(
            "array_length".to_string(),
            (vec![Type::Array(Box::new(Type::Integer))], Type::Integer)
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
            self.function_table.insert(
                function.name.clone(),
                (param_types, function.return_type.clone())
            );
        }

        if let Some(start_fn) = &program.start_function {
            let param_types = start_fn.parameters.iter().map(|p| p.type_.clone()).collect();
            self.function_table.insert(
                start_fn.name.clone(),
                (param_types, start_fn.return_type.clone())
            );
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

        // Check fields
        for field in &class.fields {
            self.check_type(&field.type_)?;
        }
            
        // Check base class if it exists
        if let Some(base_class_name) = &class.base_class {
            if !self.class_table.contains_key(base_class_name) {
                return Err(CompilerError::type_error(
                    &format!("Base class '{}' not found", base_class_name),
                    Some("Check if the base class name is correct and defined".to_string()),
                    class.location.clone()
                ));
            }
        }

        // Check constructor
        if let Some(constructor) = &class.constructor {
            self.check_constructor(constructor, class)?;
        }

        // Check methods
        for method in &class.methods {
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

        // Add constructor parameters to scope
        for param in &constructor.parameters {
            self.check_type(&param.type_)?;
            self.current_scope.define_variable(param.name.clone(), param.type_.clone());
            }

        // Add class fields to scope (accessible in constructor)
        for field in &class.fields {
            self.current_scope.define_variable(field.name.clone(), field.type_.clone());
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

        // Add method parameters to scope
        for param in &method.parameters {
            self.check_type(&param.type_)?;
            self.current_scope.define_variable(param.name.clone(), param.type_.clone());
        }

        // Add class fields to scope (accessible in methods)
        for field in &class.fields {
            if field.visibility == Visibility::Public || self.current_class == Some(class.name.clone()) {
                self.current_scope.define_variable(field.name.clone(), field.type_.clone());
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

        // Add function parameters to scope
        for param in &function.parameters {
            self.check_type(&param.type_)?;
            self.current_scope.define_variable(param.name.clone(), param.type_.clone());
        }

        // Check function body
        for stmt in &function.body {
            self.check_statement(stmt)?;
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
                            &format!("Cannot assign {:?} to variable of type {:?}", init_type, resolved_type),
                            Some("Change the initializer expression to match the variable type".to_string()),
                            location.clone()
                        ));
                    }
                }
                
                self.current_scope.define_variable(name.clone(), resolved_type);
                Ok(())
            },

            Statement::ApplyBlock { target, items, location: _ } => {
                // Check apply block - for now just check the items
                for item in items {
                    match item {
                        ApplyItem::FunctionCall(expr) => {
                            self.check_expression(expr)?;
                        },
                        ApplyItem::VariableDecl { name, initializer } => {
                            if let Some(init_expr) = initializer {
                                let init_type = self.check_expression(init_expr)?;
                                // For apply blocks, we might not have explicit type information
                                self.current_scope.define_variable(name.clone(), init_type);
                            }
                        },
                        ApplyItem::ConstantDecl { type_, name, value } => {
                            self.check_type(type_)?;
                            let value_type = self.check_expression(value)?;
                            if !self.types_compatible(type_, &value_type) {
                    return Err(CompilerError::type_error(
                                    &format!("Constant value type {:?} doesn't match declared type {:?}", value_type, type_),
                                    Some("Ensure the constant value matches the declared type".to_string()),
                        None
                    ));
                            }
                            self.current_scope.define_variable(name.clone(), type_.clone());
                        }
                    }
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
                    Type::String => Type::String, // Iterating over characters
                    _ => return Err(CompilerError::type_error(
                        &format!("Cannot iterate over type {:?}", collection_type),
                        Some("Use an array or string in iterate statements".to_string()),
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
                let start_type = self.check_expression(start)?;
                let end_type = self.check_expression(end)?;

                if start_type != Type::Integer || end_type != Type::Integer {
                    return Err(CompilerError::type_error(
                        "Range iterate requires integer start and end values".to_string(),
                        Some("Use integer expressions for range bounds".to_string()),
                        None
                    ));
                }

                self.current_scope.enter();
                self.current_scope.define_variable(iterator.clone(), Type::Integer);
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
        }
    }

    fn check_expression(&mut self, expr: &Expression) -> Result<Type, CompilerError> {
        match expr {
            Expression::Literal(value) => Ok(self.check_literal(value)),
            
            Expression::Variable(name) => {
                if let Some(var_type) = self.current_scope.lookup_variable(name) {
                    self.used_variables.insert(name.clone());
                    Ok(var_type)
                } else {
                    Err(CompilerError::type_error(
                        &format!("Variable '{}' not found", name),
                        Some("Check if the variable name is correct and the variable is declared".to_string()),
                        None
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
                // Check if this is actually a constructor call (class name)
                if self.class_table.contains_key(name) {
                    // Convert function call to object creation
                    let location = SourceLocation { line: 0, column: 0, file: "unknown".to_string() };
                    return self.check_constructor_call(name, args, &location);
                }

                self.used_functions.insert(name.clone());
                
                if let Some((param_types, return_type)) = self.function_table.get(name).cloned() {
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
                    _ => Err(CompilerError::type_error(
                        &format!("Cannot access property '{}' on type {:?}", property, object_type),
                        Some("Properties can only be accessed on objects".to_string()),
                        None
                    ))
                }
            },

            Expression::MethodCall { object, method, arguments, location } => {
                // Check if this is a static method call (ClassName.method())
                if let Expression::Variable(class_name) = object.as_ref() {
                    if self.class_table.contains_key(class_name) {
                        // This is a static method call
                        return self.check_static_method_call(class_name, method, arguments, location);
                    }
                }
                
                let object_type = self.check_expression(object)?;
                match object_type {
                    Type::Object(class_name) => {
                        self.check_method_call(object, method, arguments, location)
                    },
                    Type::Matrix(_) => {
                        // Handle matrix methods like transpose, inverse
                        match method.as_str() {
                            "transpose" | "inverse" | "determinant" => {
                                if !arguments.is_empty() {
                                    return Err(CompilerError::type_error(
                                        &format!("Matrix method '{}' takes no arguments", method),
                                        Some("Remove arguments from matrix method call".to_string()),
                                        Some(location.clone())
                                    ));
                                }
                                if method == "determinant" {
                                    Ok(Type::Float)
                                } else {
                                    Ok(object_type)
                                }
                            },
                            _ => Err(CompilerError::type_error(
                                &format!("Method '{}' not found for matrix type", method),
                                Some("Use valid matrix methods like transpose, inverse, determinant".to_string()),
                                Some(location.clone())
                            ))
                        }
                    },
                    Type::Array(element_type) => {
                        // Handle array methods like at, length
                        match method.as_str() {
                            "at" => {
                                if arguments.len() != 1 {
                                    return Err(CompilerError::type_error(
                                        &format!("Array method 'at' takes exactly 1 argument, found {}", arguments.len()),
                                        Some("Use array.at(index) with a single integer argument".to_string()),
                                        Some(location.clone())
                                    ));
                                }
                                let index_type = self.check_expression(&arguments[0])?;
                                if index_type != Type::Integer {
                                    return Err(CompilerError::type_error(
                                        &format!("Array.at() index must be integer, found {:?}", index_type),
                                        Some("Use integer expressions for array indices".to_string()),
                                        Some(location.clone())
                                    ));
                                }
                                Ok(*element_type)
                            },
                            "length" => {
                                if !arguments.is_empty() {
                                    return Err(CompilerError::type_error(
                                        &format!("Array method 'length' takes no arguments, found {}", arguments.len()),
                                        Some("Use array.length() without arguments".to_string()),
                                        Some(location.clone())
                                    ));
                                }
                                Ok(Type::Integer)
                            },
                            _ => Err(CompilerError::type_error(
                                &format!("Method '{}' not found for array type", method),
                                Some("Use valid array methods like at(index), length()".to_string()),
                                Some(location.clone())
                            ))
                        }
                    },
                    _ => Err(CompilerError::type_error(
                        &format!("Cannot call method '{}' on type {:?}", method, object_type),
                        Some("Methods can only be called on objects, matrices, and arrays".to_string()),
                        Some(location.clone())
                    ))
                }
            },

            Expression::ArrayAccess(array, index) => {
        let array_type = self.check_expression(array)?;
        let index_type = self.check_expression(index)?;

                if index_type != Type::Integer {
            return Err(CompilerError::type_error(
                        &format!("Array index must be integer, found {:?}", index_type),
                        Some("Use integer expressions for array indices".to_string()),
                        None
                    ));
                }

        match array_type {
            Type::Array(element_type) => Ok(*element_type),
            _ => Err(CompilerError::type_error(
                        &format!("Cannot index into type {:?}", array_type),
                        Some("Array access can only be used on array types".to_string()),
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
                    (Type::Integer, Type::Integer) => Ok(Type::Integer),
                    (Type::Float, Type::Float) => Ok(Type::Float),
                    (Type::Integer, Type::Float) | (Type::Float, Type::Integer) => Ok(Type::Float),
                    (Type::String, Type::String) if matches!(op, BinaryOperator::Add) => Ok(Type::String),
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

    fn types_compatible(&self, t1: &Type, t2: &Type) -> bool {
        t1 == t2 || (matches!((t1, t2), (Type::Integer, Type::Float) | (Type::Float, Type::Integer)))
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
            Type::Matrix(element_type) => self.check_type(element_type),
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
            Type::Matrix(element_type) => {
                Type::Matrix(Box::new(self.resolve_type(element_type)))
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
                if let Some(class) = self.class_table.get(&class_name).cloned() {
                    for method_def in &class.methods {
                        if method_def.name == method {
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
                        }
                    }
                    
                    Err(CompilerError::type_error(
                        &format!("Method '{}' not found in class '{}'", method, class_name),
                        Some("Check if the method name is correct".to_string()),
                        Some(location.clone())
                    ))
                } else {
                    Err(CompilerError::type_error(
                        &format!("Class '{}' not found", class_name),
                        Some("Check if the class name is correct".to_string()),
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
        
        // Get the class from the user-defined class table
        let class = self.class_table.get(class_name).cloned().ok_or_else(|| {
            CompilerError::type_error(
                &format!("Class '{}' not found", class_name),
                Some("Check if the class name is correct and the class is defined".to_string()),
                Some(location.clone())
            )
        })?;

        // Find the method in the class
        let class_method = class.methods.iter().find(|m| m.name == method).ok_or_else(|| {
            CompilerError::type_error(
                &format!("Static method '{}' not found in class '{}'", method, class_name),
                Some("Check if the method name is correct and the method is defined in the class".to_string()),
                Some(location.clone())
            )
        })?;

        // Check that the method doesn't access instance fields (validation for static call)
        // This would require analyzing the method body to ensure it doesn't use 'this' or instance fields
        // For now, we'll allow all methods to be called statically and warn the user about the restriction

        // Check argument count
        if args.len() != class_method.parameters.len() {
            return Err(CompilerError::type_error(
                &format!("Static method '{}' expects {} arguments, but {} were provided", 
                    method, class_method.parameters.len(), args.len()),
                Some("Provide the correct number of arguments".to_string()),
                Some(location.clone())
            ));
        }

        // Check argument types
        for (i, (arg, param)) in args.iter().zip(class_method.parameters.iter()).enumerate() {
            let arg_type = self.check_expression(arg)?;
            if !self.types_compatible(&arg_type, &param.type_) {
                return Err(CompilerError::type_error(
                    &format!("Argument {} has type {:?}, but parameter '{}' expects {:?}", 
                        i + 1, arg_type, param.name, param.type_),
                    Some("Provide arguments of the correct type".to_string()),
                    Some(location.clone())
                ));
            }
        }

        Ok(class_method.return_type.clone())
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
                        if args.len() != 1 {
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
                    _ => Ok(None), // Method not found in ArrayUtils
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
} 