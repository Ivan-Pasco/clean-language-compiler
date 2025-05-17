use std::collections::{HashMap, HashSet};
use crate::ast::{self, Program, Function, Statement, Expression, Type, Value, MatrixOperator, UnaryOperator, BinaryOperator, SourceLocation, Visibility, Class};
use crate::error::{CompilerError, ErrorContext, ErrorType};
use std::convert::TryFrom;
mod scope;
use scope::Scope;

// Fix for the "no method 'is_public' on Visibility" error
// Add an extension implementation for Visibility
impl Visibility {
    /// Returns true if the visibility is Public
    pub fn is_public(&self) -> bool {
        match self {
            Visibility::Public => true,
            Visibility::Private => false,
        }
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
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        Self {
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
        }
    }

    /// Analyze a program for semantic correctness
    pub fn analyze(&mut self, program: &Program) -> Result<Program, CompilerError> {
        // We'll just do a check for now and return the program unchanged
        self.check(program)?;
        Ok(program.clone())
    }

    pub fn check(&mut self, program: &Program) -> Result<(), CompilerError> {
        // First pass: collect all class definitions
        for class in &program.classes {
            self.class_table.insert(class.name.clone(), class.clone());
        }

        // Check class inheritance hierarchy for cycles
        self.check_inheritance_cycles()?;

        // Second pass: check all classes
        for class in &program.classes {
            self.current_class = Some(class.name.clone());
            self.check_class(class)?;
            self.current_class = None;
        }

        // Third pass: check all functions
        for function in &program.functions {
            self.current_function = Some(function.name.clone());
            self.check_function(function)?;
            self.current_function = None;
        }

        Ok(())
    }

    fn check_inheritance_cycles(&self) -> Result<(), CompilerError> {
        for class_name in self.class_table.keys() {
            let mut visited = HashSet::new();
            let mut current = Some(class_name.clone());

            while let Some(class) = current {
                if !visited.insert(class.clone()) {
                    return Err(CompilerError::type_error(
                        "Cyclic inheritance detected",
                        Some("Remove the inheritance cycle".to_string()),
                        None
                    ));
                }

                current = self.class_table.get(&class)
                    .and_then(|c| c.base_class.clone());
            }
        }
        Ok(())
    }

    fn check_class(&mut self, class: &Class) -> Result<(), CompilerError> {
        let old_class = self.current_class.clone();
        self.current_class = Some(class.name.clone());

        // Check base class exists if specified
        if let Some(ref base) = class.base_class {
            if !self.class_table.contains_key(base) {
                return Err(CompilerError::type_error(
                    &format!("Base class not found: {}", base),
                    Some("Check if the base class is correct and the class is defined".to_string()),
                    Some(class.location.clone().unwrap_or_default())
                ));
            }
        }

        // Check fields
        let mut field_types = HashMap::new();
        for field in &class.fields {
            // Check field type
            self.check_type(&field.type_)?;
            
            // Check field name uniqueness
            if field_types.insert(field.name.clone(), field.type_.clone()).is_some() {
                return Err(CompilerError::type_error(
                    &format!("Duplicate field name: {}", field.name),
                    Some("Use unique field names within a class".to_string()),
                    Some(class.location.clone().unwrap_or_default())
                ));
            }
        }

        // Check constructor
        if let Some(ref constructor) = class.constructor {
            self.check_constructor(constructor, class)?;
        }

        // Check methods
        for method in &class.methods {
            self.check_method(method, class)?;
        }

        self.current_class = old_class;
        Ok(())
    }

    fn check_constructor(&mut self, constructor: &ast::Constructor, class: &Class) -> Result<(), CompilerError> {
        // Create new scope for constructor parameters
        let mut old_symbols = HashMap::new();
        for param in &constructor.parameters {
            if let Some(old_type) = self.symbol_table.insert(param.name.clone(), param.type_.clone()) {
                old_symbols.insert(param.name.clone(), old_type);
            }
        }

        // Add 'this' to symbol table
        self.symbol_table.insert("this".to_string(), Type::Object(class.name.clone()));

        // Check constructor body
        for statement in &constructor.body {
            self.check_statement(statement)?;
        }

        // Restore old scope
        for (name, type_) in old_symbols {
            self.symbol_table.insert(name, type_);
        }
        self.symbol_table.remove("this");

        Ok(())
    }

    fn check_method(&mut self, method: &Function, class: &Class) -> Result<(), CompilerError> {
        // Add method to function table with class prefix
        let method_key = format!("{}.{}", class.name, method.name);
        let param_types: Vec<Type> = method.parameters.iter()
            .map(|p| p.type_.clone())
            .collect();
        self.function_table.insert(
            method_key,
            (param_types, method.return_type.clone()),
        );

        // Create new scope for method
        let mut old_symbols = HashMap::new();
        for param in &method.parameters {
            if let Some(old_type) = self.symbol_table.insert(param.name.clone(), param.type_.clone()) {
                old_symbols.insert(param.name.clone(), old_type);
            }
        }

        // Add 'this' to symbol table
        self.symbol_table.insert("this".to_string(), Type::Object(class.name.clone()));

        // Check method body
        for statement in &method.body {
            self.check_statement(statement)?;
        }

        // Restore old scope
        for (name, type_) in old_symbols {
            self.symbol_table.insert(name, type_);
        }
        self.symbol_table.remove("this");

        Ok(())
    }

    fn check_function(&mut self, function: &Function) -> Result<(), CompilerError> {
        let mut function_scope = Scope::new();
        let mut old_scope = Scope::new();
        std::mem::swap(&mut old_scope, &mut self.current_scope);
        
        let old_return_type = self.current_function_return_type.clone();
        self.current_function_return_type = Some(function.return_type.clone());

        // Add parameters to scope
        for param in &function.parameters {
            function_scope.add_variable(param.name.clone(), param.type_.clone());
        }
        
        // Set the current scope to the function scope
        self.current_scope = function_scope;

        // Check function body
        for stmt in &function.body {
            self.check_statement(stmt)?;
        }

        std::mem::swap(&mut old_scope, &mut self.current_scope);
        self.current_function_return_type = old_return_type;
        Ok(())
    }

    fn check_statement(&mut self, stmt: &Statement) -> Result<(), CompilerError> {
        match stmt {
            Statement::VariableDecl { name, type_, initializer, location } => {
                if let Some(init) = initializer {
                    self.check_expression(init)?;
                }
                Ok(())
            }
            Statement::Assignment { target, value, location } => {
                self.check_expression(value)?;
                Ok(())
            }
            Statement::Print { expression, newline, location } => {
                self.check_expression(expression)?;
                Ok(())
            }
            Statement::Return { value, location } => {
                if let Some(expr) = value {
                    self.check_expression(expr)?;
                }
                Ok(())
            }
            Statement::If { condition, then_branch, else_branch, location } => {
                self.check_expression(condition)?;
                for stmt in then_branch {
                    self.check_statement(stmt)?;
                }
                if let Some(else_stmts) = else_branch {
                    for stmt in else_stmts {
                        self.check_statement(stmt)?;
                    }
                }
                Ok(())
            }
            Statement::Iterate { iterator, collection, body, location } => {
                self.check_expression(collection)?;
                
                // Create a new scope for the iterate body
                let mut body_scope = Scope::new();
                body_scope.set_parent(Box::new(self.current_scope.clone()));
                
                // Add iterator variable
                body_scope.add_variable(iterator.clone(), Type::Any);
                
                let old_scope = std::mem::replace(&mut self.current_scope, body_scope);
                
                for stmt in body {
                    self.check_statement(stmt)?;
                }
                
                self.current_scope = old_scope;
                Ok(())
            }
            Statement::FromTo { start, end, step, body, location } => {
                self.check_expression(start)?;
                self.check_expression(end)?;
                if let Some(step_expr) = step {
                    self.check_expression(step_expr)?;
                }
                
                for stmt in body {
                    self.check_statement(stmt)?;
                }
                
                Ok(())
            }
            Statement::ErrorHandler { stmt, handler, location } => {
                // Use the check_try_catch method to check error handler statements
                self.check_try_catch(stmt, handler, location.as_ref().unwrap_or(&SourceLocation::default()))
            }
            Statement::Expression { expr, location } => {
                self.check_expression(expr)?;
                Ok(())
            }
            _ => Ok(())
        }
    }

    fn check_expression(&mut self, expr: &Expression) -> Result<Type, CompilerError> {
        let location = self.get_expr_location(expr);
        
        match expr {
            Expression::Literal(value) => {
                Ok(self.check_literal(value))
            },
            Expression::Variable(name) => {
                self.check_variable(name, &location)
            },
            Expression::Binary(left, op, right) => {
                let loc_opt = Some(location.clone());
                self.check_binary_operation(op, left, right, &loc_opt)
            },
            Expression::Call(name, args) => { 
                self.check_function_call(name, args, &location)
            },
            Expression::ArrayAccess(array, index) => {
                self.check_array_access(array, index, &location)
            },
            Expression::MatrixAccess(matrix, row, col) => {
                self.check_matrix_access(matrix, row, col, &location)
            },
            Expression::FieldAccess { object, field, location } => {
                self.check_field_access(object, field, location)
            },
            Expression::MethodCall { object, method, arguments, location } => {
                self.check_method_call(object, method, arguments, location)
            },
            Expression::ObjectCreation { class_name, arguments, location } => {
                self.check_constructor_call(class_name, arguments, location)
            },
            Expression::MatrixOperation(left, op, right, location) => {
                self.check_matrix_operation(left, op, right, location)
            },
            Expression::StringConcat(parts) => {
                // Check all parts are strings
                for part in parts {
                    let part_type = self.check_expression(part)?;
                    if part_type != Type::String {
                        return Err(CompilerError::type_error(
                            &format!("String concatenation requires string parts, found {:?}", part_type),
                            Some("All parts of a string concatenation must evaluate to strings".to_string()),
                            Some(location.clone())
                        ));
                    }
                }
                Ok(Type::String)
            },
            Expression::Unary(op, expr) => {
                self.check_unary_operation(op, expr, &location)
            },
        }
    }

    fn check_binary_operation(&mut self, op: &BinaryOperator, left: &Expression, right: &Expression, location: &Option<SourceLocation>) -> Result<Type, CompilerError> {
        let left_type = self.check_expression(left)?;
        let right_type = self.check_expression(right)?;
        
        match op {
            BinaryOperator::Add => {
                match (&left_type, &right_type) {
                    (Type::Integer, Type::Integer) => Ok(Type::Integer),
                    (Type::Float, Type::Float) => Ok(Type::Float),
                    (Type::Integer, Type::Float) | (Type::Float, Type::Integer) => Ok(Type::Float),
                    (Type::String, Type::String) => Ok(Type::String),
                    // Allow concatenation with any type and string
                    (Type::String, _) | (_, Type::String) => Ok(Type::String),
                    _ => Err(CompilerError::detailed_type_error(
                        &format!("Cannot add values of types {:?} and {:?}", left_type, right_type),
                        format!("numeric or string types"),
                        format!("{:?} and {:?}", left_type, right_type),
                        location.clone(),
                        Some("Addition is only supported for numeric types or string concatenation".to_string())
                    ))
                }
            },
            BinaryOperator::Subtract => {
                match (&left_type, &right_type) {
                    (Type::Integer, Type::Integer) => Ok(Type::Integer),
                    (Type::Float, Type::Float) => Ok(Type::Float),
                    (Type::Integer, Type::Float) | (Type::Float, Type::Integer) => Ok(Type::Float),
                    _ => Err(CompilerError::detailed_type_error(
                        &format!("Cannot subtract values of types {:?} and {:?}", left_type, right_type),
                        format!("numeric types"),
                        format!("{:?} and {:?}", left_type, right_type),
                        location.clone(),
                        Some("Subtraction is only supported for numeric types".to_string())
                    ))
                }
            },
            BinaryOperator::Multiply => {
                match (&left_type, &right_type) {
                    (Type::Integer, Type::Integer) => Ok(Type::Integer),
                    (Type::Float, Type::Float) => Ok(Type::Float),
                    (Type::Integer, Type::Float) | (Type::Float, Type::Integer) => Ok(Type::Float),
                    _ => Err(CompilerError::detailed_type_error(
                        &format!("Cannot multiply values of types {:?} and {:?}", left_type, right_type),
                        format!("numeric types"),
                        format!("{:?} and {:?}", left_type, right_type),
                        location.clone(),
                        Some("Multiplication is only supported for numeric types".to_string())
                    ))
                }
            },
            BinaryOperator::Divide => {
                match (&left_type, &right_type) {
                    (Type::Integer, Type::Integer) => Ok(Type::Integer),
                    (Type::Float, Type::Float) => Ok(Type::Float),
                    (Type::Integer, Type::Float) | (Type::Float, Type::Integer) => Ok(Type::Float),
                    _ => Err(CompilerError::detailed_type_error(
                        &format!("Cannot divide values of types {:?} and {:?}", left_type, right_type),
                        format!("numeric types"),
                        format!("{:?} and {:?}", left_type, right_type),
                        location.clone(),
                        Some("Division is only supported for numeric types".to_string())
                    ))
                }
            },
            BinaryOperator::Equal | BinaryOperator::NotEqual => {
                // Most types can be compared for equality
                if left_type == right_type || left_type == Type::Any || right_type == Type::Any {
                    Ok(Type::Boolean)
                } else {
                    // Incompatible types for equality comparison, but might work at runtime
                    // For now just warn but allow it, returning Boolean
                    Ok(Type::Boolean)
                }
            },
            BinaryOperator::Less | BinaryOperator::LessEqual | 
            BinaryOperator::Greater | BinaryOperator::GreaterEqual => {
                match (&left_type, &right_type) {
                    (Type::Integer, Type::Integer) | (Type::Float, Type::Float) |
                    (Type::Integer, Type::Float) | (Type::Float, Type::Integer) => Ok(Type::Boolean),
                    (Type::String, Type::String) => Ok(Type::Boolean), // String comparisons are allowed
                    _ => Err(CompilerError::detailed_type_error(
                        &format!("Cannot compare values of types {:?} and {:?}", left_type, right_type),
                        format!("comparable types (numbers or strings)"),
                        format!("{:?} and {:?}", left_type, right_type),
                        location.clone(),
                        Some("Comparison operations are only supported for numeric types or strings".to_string())
                    ))
                }
            },
            _ => Err(CompilerError::type_error(
                &format!("Unsupported binary operator: {:?}", op),
                Some("Check the operator is supported in this context".to_string()),
                location.clone()
            ))
        }
    }

    fn check_unary_operation(&mut self, op: &UnaryOperator, expr: &Expression, location: &SourceLocation) -> Result<Type, CompilerError> {
        let expr_type = self.check_expression(expr)?;
        match op {
            UnaryOperator::Negate => {
                match expr_type {
                    Type::Integer | Type::Float => Ok(expr_type),
                    _ => Err(CompilerError::type_error(
                        "Cannot negate non-numeric type",
                        Some("Use numeric types for negation".to_string()),
                        Some(location.clone())
                    )),
                }
            }
            UnaryOperator::Not => {
                if expr_type != Type::Boolean {
                    return Err(CompilerError::type_error(
                        "Logical not requires boolean operand",
                        Some("Use boolean values for logical not".to_string()),
                        Some(location.clone())
                    ));
                }
                Ok(Type::Boolean)
            }
        }
    }

    fn check_matrix_operation(&mut self, left: &Expression, op: &MatrixOperator, right: &Expression, location: &SourceLocation) -> Result<Type, CompilerError> {
        let left_type = self.check_expression(left)?;
        let right_type = self.check_expression(right)?;

        match op {
            MatrixOperator::Add | MatrixOperator::Subtract | MatrixOperator::Multiply => {
                if !matches!(left_type, Type::Matrix(_)) || !matches!(right_type, Type::Matrix(_)) {
                    return Err(CompilerError::type_error(
                        "Matrix operation requires matrix operands",
                        Some("Use matrix types for matrix operations".to_string()),
                        Some(location.clone())
                    ));
                }
                Ok(Type::Matrix(Box::new(Type::Number)))
            }
            MatrixOperator::Transpose => {
                if !matches!(left_type, Type::Matrix(_)) {
                    return Err(CompilerError::type_error(
                        "Matrix transpose requires a matrix operand",
                        Some("Use a matrix type for transpose operation".to_string()),
                        Some(location.clone())
                    ));
                }
                Ok(Type::Matrix(Box::new(Type::Number)))
            }
            MatrixOperator::Inverse => {
                if !matches!(left_type, Type::Matrix(_)) {
                    return Err(CompilerError::type_error(
                        "Matrix inverse requires a matrix operand",
                        Some("Use a matrix type for inverse operation".to_string()),
                        Some(location.clone())
                    ));
                }
                Ok(Type::Matrix(Box::new(Type::Number)))
            }
        }
    }

    fn types_compatible(&self, t1: &Type, t2: &Type) -> bool {
        match (t1, t2) {
            (Type::Integer, Type::Float) | (Type::Float, Type::Integer) => true,
            (Type::Array(t1), Type::Array(t2)) | (Type::Matrix(t1), Type::Matrix(t2)) => self.types_compatible(t1, t2),
            _ => t1 == t2,
        }
    }

    fn value_to_type(value: &Value) -> Type {
        match value {
            Value::Boolean(_) => Type::Boolean,
            Value::String(_) => Type::String,
            Value::Integer(_) => Type::Integer,
            Value::Float(_) => Type::Float,
            Value::Number(_) => Type::Float,
            Value::Byte(_) => Type::Byte,
            Value::Unsigned(_) => Type::Unsigned,
            Value::Long(_) => Type::Long,
            Value::ULong(_) => Type::ULong,
            Value::Big(_) => Type::Big,
            Value::UBig(_) => Type::UBig,
            Value::Array(values) => {
                if values.is_empty() {
                    Type::Array(Box::new(Type::Unit))
                } else {
                    Type::Array(Box::new(Self::value_to_type(&values[0])))
                }
            }
            Value::Matrix(rows) => {
                if rows.is_empty() || rows[0].is_empty() {
                    Type::Matrix(Box::new(Type::Unit))
                } else {
                    Type::Matrix(Box::new(Type::Float))
                }
            }
            Value::Null => Type::Any,
            Value::Unit => Type::Unit,
        }
    }

    // Helper method to safely get location from expression
    fn get_expr_location(&self, expr: &Expression) -> SourceLocation {
        match expr {
            Expression::Literal(_) => SourceLocation::default(),
            Expression::Variable(_) => SourceLocation::default(),
            Expression::Binary(_, _, _) => SourceLocation::default(),
            Expression::Unary(_, _) => SourceLocation::default(),
            Expression::Call(_, _) => SourceLocation::default(),
            Expression::ArrayAccess(_, _) => SourceLocation::default(),
            Expression::MatrixAccess(_, _, _) => SourceLocation::default(),
            Expression::FieldAccess { location, .. } => location.clone(),
            Expression::MethodCall { location, .. } => location.clone(),
            Expression::ObjectCreation { location, .. } => location.clone(),
            Expression::MatrixOperation(_, _, _, location) => location.clone(),
            Expression::StringConcat(_) => SourceLocation::default(),
            // Add a default case to handle any new variants
            _ => SourceLocation::default(),
        }
    }

    fn check_array_access(&mut self, array: &Expression, index: &Expression, location: &SourceLocation) -> Result<Type, CompilerError> {
        let array_type = self.check_expression(array)?;
        let index_type = self.check_expression(index)?;

        // Check index is integer
        if !matches!(index_type, Type::Integer) {
            return Err(CompilerError::type_error(
                "Array index must be an integer",
                Some("Use an integer value for array indexing".to_string()),
                Some(location.clone())
            ));
        }

        // Check array is an array type
        match array_type {
            Type::Array(element_type) => Ok(*element_type),
            _ => Err(CompilerError::type_error(
                "Cannot index a non-array type",
                Some("Only arrays can be indexed".to_string()),
                Some(location.clone())
            )),
        }
    }

    fn check_matrix_access(&mut self, matrix: &Expression, row: &Expression, col: &Expression, location: &SourceLocation) -> Result<Type, CompilerError> {
        let matrix_type = self.check_expression(matrix)?;
        let row_type = self.check_expression(row)?;
        let col_type = self.check_expression(col)?;

        // Check row and column indices are integers
        if !matches!(row_type, Type::Integer) || !matches!(col_type, Type::Integer) {
            return Err(CompilerError::type_error(
                "Matrix indices must be integers",
                Some("Use integer values for matrix row and column indexing".to_string()),
                Some(location.clone())
            ));
        }

        // Check matrix is a matrix type
        match matrix_type {
            Type::Matrix(element_type) => Ok(*element_type),
            _ => Err(CompilerError::type_error(
                "Cannot index a non-matrix type",
                Some("Only matrices can be indexed with two indices".to_string()),
                Some(location.clone())
            )),
        }
    }

    fn check_class_lookup(&mut self, class_name: &str, location: &SourceLocation) -> Result<(), CompilerError> {
        if !self.class_table.contains_key(class_name) {
            return Err(CompilerError::type_error(
                &format!("Class '{}' not found", class_name),
                Some("Check if the class name is correct and the class is defined".to_string()),
                Some(location.clone())
            ));
        }
        Ok(())
    }

    fn check_method_lookup(&mut self, class_name: &str, method_name: &str, location: &SourceLocation) -> Result<Type, CompilerError> {
        let class = self.class_table.get(class_name).ok_or_else(|| {
            CompilerError::type_error(
                &format!("Class '{}' not found", class_name),
                Some("Check if the class name is correct and the class is defined".to_string()),
                Some(location.clone())
            )
        })?;

        // Loop through methods to find one with the matching name
        for method in &class.methods {
            if method.name == method_name {
                return Ok(method.return_type.clone());
            }
        }
        
        // If not found, return error
        Err(CompilerError::type_error(
            &format!("Method '{}' not found in class '{}'", method_name, class_name),
            Some("Check if the method name is correct and the method is defined in the class".to_string()),
            Some(location.clone())
        ))
    }

    fn check_field_access(&mut self, object: &Expression, field: &str, location: &SourceLocation) -> Result<Type, CompilerError> {
        let object_type = self.check_expression(object)?;
        
        match object_type {
            Type::Object(class_name) => {
                let class = self.class_table.get(&class_name).ok_or_else(|| {
                    CompilerError::type_error(
                        &format!("Class '{}' not found", class_name),
                        Some("Check if the class name is correct and the class is defined".to_string()),
                        Some(location.clone())
                    )
                })?;

                let field_type = class.fields.iter().find(|f| f.name == *field).ok_or_else(|| {
                    CompilerError::type_error(
                        &format!("Field '{}' not found in class '{}'", field, class_name),
                        Some("Check if the field name is correct and the field is defined in the class".to_string()),
                        Some(location.clone())
                    )
                })?;

                if !field_type.visibility.is_public() {
                    return Err(CompilerError::type_error(
                        &format!("Field '{}' is private in class '{}'", field, class_name),
                        Some("Access only public fields or use appropriate getter methods".to_string()),
                        Some(location.clone())
                    ));
                }

                Ok(field_type.type_.clone())
            }
            _ => Err(CompilerError::type_error(
                "Cannot access field on non-object type",
                Some("Only objects can have fields".to_string()),
                Some(location.clone())
            )),
        }
    }

    fn check_method_call(&mut self, object: &Expression, method: &str, args: &[Expression], location: &SourceLocation) -> Result<Type, CompilerError> {
        let object_type = self.check_expression(object)?;
        
        match object_type {
            Type::Object(class_name) => {
                // Clone class to avoid borrow issues
                let class_opt = self.class_table.get(&class_name).cloned();
                
                if let Some(class) = class_opt {
                    // Find method with matching name
                    for method_info in &class.methods {
                        if method_info.name == method {
                            if args.len() != method_info.parameters.len() {
                                return Err(CompilerError::type_error(
                                    &format!("Method '{}' expects {} arguments, but {} were provided",
                                        method, method_info.parameters.len(), args.len()),
                                    Some("Provide the correct number of arguments".to_string()),
                                    Some(location.clone())
                                ));
                            }
                            
                            // Make a copy of parameter types to avoid borrow issues
                            let param_types: Vec<Type> = method_info.parameters.iter()
                                .map(|p| p.type_.clone())
                                .collect();
                            
                            // Check arguments against parameter types
                            for (i, (arg, param_type)) in args.iter().zip(param_types.iter()).enumerate() {
                                let arg_type = self.check_expression(arg)?;
                                if arg_type != *param_type {
                                    return Err(CompilerError::type_error(
                                        &format!("Argument {} has type {:?}, but method parameter expects {:?}",
                                            i + 1, arg_type, param_type),
                                        Some("Provide arguments of the correct type".to_string()),
                                        Some(self.get_expr_location(arg))
                                    ));
                                }
                            }
                            
                            return Ok(method_info.return_type.clone());
                        }
                    }
                    
                    return Err(CompilerError::type_error(
                        &format!("Method '{}' not found in class '{}'", method, class_name),
                        Some("Check if the method name is correct and the method is defined in the class".to_string()),
                        Some(location.clone())
                    ));
                } else {
                    return Err(CompilerError::type_error(
                        &format!("Class '{}' not found", class_name),
                        Some("Check if the class name is correct and the class is defined".to_string()),
                        Some(location.clone())
                    ));
                }
            },
            _ => Err(CompilerError::type_error(
                "Cannot call method on non-object type",
                Some("Only objects can have methods".to_string()),
                Some(location.clone())
            )),
        }
    }

    fn check_function_call(&mut self, name: &str, args: &[Expression], location: &SourceLocation) -> Result<Type, CompilerError> {
        // Clone function information to avoid borrow issues
        let function_opt = self.function_table.get(name).cloned();
        
        let function = function_opt.ok_or_else(|| {
            CompilerError::type_error(
                &format!("Function '{}' not found", name),
                Some("Check if the function name is correct and the function is defined".to_string()),
                Some(location.clone())
            )
        })?;

        if args.len() != function.0.len() {
            return Err(CompilerError::type_error(
                &format!("Function '{}' expects {} arguments, but {} were provided",
                    name, function.0.len(), args.len()),
                Some("Provide the correct number of arguments".to_string()),
                Some(location.clone())
            ));
        }

        // Clone parameter types to avoid borrow issues
        let param_types = function.0.clone();
        
        for (i, (arg, param)) in args.iter().zip(param_types.iter()).enumerate() {
            let arg_type = self.check_expression(arg)?;
            if arg_type != *param {
                return Err(CompilerError::type_error(
                    &format!("Argument {} has type {:?}, but function parameter expects {:?}",
                        i + 1, arg_type, param),
                    Some("Provide arguments of the correct type".to_string()),
                    Some(self.get_expr_location(arg))
                ));
            }
        }

        Ok(function.1.clone())
    }

    fn check_try_catch(&mut self, try_block: &Statement, catch_block: &[Statement], location: &SourceLocation) -> Result<(), CompilerError> {
        // Check the try block statement
        self.check_statement(try_block)?;

        // Create a new scope for the catch block
        let mut catch_scope = Scope::new();
        catch_scope.set_parent(Box::new(self.current_scope.clone()));
        
        // Add 'error' variable to catch block scope
        catch_scope.add_variable("error", Type::Integer);
        
        let old_scope = std::mem::replace(&mut self.current_scope, catch_scope);

        // Check catch block statements
        for stmt in catch_block {
            self.check_statement(stmt)?;
        }

        // Restore original scope
        self.current_scope = old_scope;
        Ok(())
    }

    fn check_variable(&mut self, name: &str, location: &SourceLocation) -> Result<Type, CompilerError> {
        if let Some(var_type) = self.current_scope.get_variable(name) {
            Ok(var_type)
        } else {
            Err(CompilerError::type_error(
                &format!("Variable '{}' not found", name),
                Some("Check if the variable name is correct and the variable is defined".to_string()),
                Some(location.clone())
            ))
        }
    }

    /// Analyze a try-catch statement semantically
    fn analyze_try_catch(&mut self, try_block: &[Statement], variable: &str, catch_block: &[Statement], location: &SourceLocation) -> Result<Type, CompilerError> {
        let try_result = self.analyze_block(try_block)?;
        
        // Save the current scope
        let mut parent_scope = Scope::new();
        std::mem::swap(&mut parent_scope, &mut self.current_scope);
        
        // Create new scope for catch block with "error" variable
        let mut catch_scope = Scope::new();
        catch_scope.set_parent(Box::new(parent_scope.clone()));
        
        // Add the error variable to the catch scope
        catch_scope.add_variable(variable.to_string(), Type::Integer);
        
        // Set the current scope to the catch scope for analyzing the catch block
        self.current_scope = catch_scope;
        
        // Analyze the catch block
        let catch_result = self.analyze_block(catch_block);
        
        // Restore the original scope
        std::mem::swap(&mut parent_scope, &mut self.current_scope);
        
        // Return the try block's result type (ignoring the catch block's type)
        catch_result?;
        Ok(try_result)
    }

    /// Analyze a block of statements and return the type of the last expression
    fn analyze_block(&mut self, statements: &[Statement]) -> Result<Type, CompilerError> {
        let mut result_type = Type::Unit;
        
        for statement in statements {
            match statement {
                Statement::Return { value, location } => {
                    if let Some(expr) = value {
                        return self.check_expression(expr);
                    } else {
                        return Ok(Type::Unit);
                    }
                },
                Statement::Expression { expr, location } => {
                    result_type = self.check_expression(expr)?;
                },
                _ => {
                    self.check_statement(statement)?;
                }
            }
        }
        
        Ok(result_type)
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

        // Clone parameter types to avoid borrow issues
        let param_types: Vec<Type> = constructor.parameters.iter()
            .map(|p| p.type_.clone())
            .collect();
            
        for (i, (arg, param_type)) in args.iter().zip(param_types.iter()).enumerate() {
            let arg_type = self.check_expression(arg)?;
            if arg_type != *param_type {
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
            Value::Number(_) => Type::Float,
            Value::Byte(_) => Type::Byte,
            Value::Unsigned(_) => Type::Unsigned,
            Value::Long(_) => Type::Long,
            Value::ULong(_) => Type::ULong,
            Value::Big(_) => Type::Big,
            Value::UBig(_) => Type::UBig,
            Value::Float(_) => Type::Float,
            Value::Null => Type::Any,
            Value::Unit => Type::Unit,
        }
    }

    fn analyze_literal(&self, value: &Value) -> Type {
        match value {
            Value::Integer(_) => Type::Integer,
            Value::Boolean(_) => Type::Boolean,
            Value::String(_) => Type::String,
            Value::Array(_) => Type::Array(Box::new(Type::Any)),
            Value::Matrix(_) => Type::Matrix(Box::new(Type::Float)),
            Value::Number(_) => Type::Float,
            Value::Byte(_) => Type::Byte,
            Value::Unsigned(_) => Type::Unsigned,
            Value::Long(_) => Type::Long,
            Value::ULong(_) => Type::ULong,
            Value::Big(_) => Type::Big,
            Value::UBig(_) => Type::UBig,
            Value::Float(_) => Type::Float,
            Value::Null => Type::Any,
            Value::Unit => Type::Unit,
        }
    }
} 