use crate::ast::{self, Program, Statement, Expression, Value, Type, Function};
use crate::error::{CompilerError, ErrorContext};
use std::collections::{HashMap, HashSet};

/// Represents an optimization pass
pub trait OptimizationPass {
    fn run(&mut self, program: &mut Program) -> Result<bool, CompilerError>;
    fn name(&self) -> &'static str;
}

/// Main optimizer that runs all optimization passes
pub struct Optimizer {
    passes: Vec<Box<dyn OptimizationPass>>,
}

impl Optimizer {
    pub fn new() -> Self {
        let mut optimizer = Self {
            passes: Vec::new(),
        };
        
        // Add optimization passes in order
        optimizer.add_pass(Box::new(ConstantFolding::new()));
        optimizer.add_pass(Box::new(DeadCodeElimination::new()));
        optimizer.add_pass(Box::new(CommonSubexpressionElimination::new()));
        optimizer.add_pass(Box::new(ControlFlowOptimization::new()));
        
        optimizer
    }

    pub fn add_pass(&mut self, pass: Box<dyn OptimizationPass>) {
        self.passes.push(pass);
    }

    pub fn optimize(&mut self, program: &mut Program) -> Result<(), CompilerError> {
        let mut changed = true;
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 10;

        // Run optimization passes until no more changes or max iterations reached
        while changed && iterations < MAX_ITERATIONS {
            changed = false;
            for pass in &mut self.passes {
                if pass.run(program)? {
                    changed = true;
                }
            }
            iterations += 1;
        }

        Ok(())
    }
}

/// Constant folding optimization pass
pub struct ConstantFolding {
    constants: HashMap<String, Value>,
}

impl ConstantFolding {
    pub fn new() -> Self {
        Self {
            constants: HashMap::new(),
        }
    }

    fn fold_expression(&mut self, expr: &mut Expression) -> Result<bool, CompilerError> {
        let mut changed = false;
        
        match expr {
            Expression::Binary(left, op, right, location) => {
                // Recursively fold operands
                changed |= self.fold_expression(left)?;
                changed |= self.fold_expression(right)?;
                
                // Try to evaluate constant expressions
                if let (Expression::Literal(left_val), Expression::Literal(right_val)) = (&**left, &**right) {
                    if let Some(result) = evaluate_binary_op(left_val, op, right_val) {
                        *expr = Expression::Literal(result);
                        changed = true;
                    }
                }
            }
            Expression::Call(name, args, location) => {
                // Fold arguments
                for arg in args {
                    changed |= self.fold_expression(arg)?;
                }
            }
            _ => {}
        }
        
        Ok(changed)
    }

    fn fold_statement(&mut self, stmt: &mut Statement) -> Result<bool, CompilerError> {
        let mut changed = false;
        
        match stmt {
            Statement::VariableDecl { initializer, name, .. } => {
                if let Some(expr) = initializer {
                    changed |= self.fold_expression(expr)?;
                    
                    // If the initializer is a constant, store it
                    if let Expression::Literal(value) = expr {
                        self.constants.insert(name.clone(), value.clone());
                    }
                }
            }
            Statement::Assignment { target, value, .. } => {
                changed |= self.fold_expression(value)?;
                
                // If assigning a constant, update our map
                if let Expression::Variable(name) = target {
                    if let Expression::Literal(value) = value {
                        self.constants.insert(name.clone(), value.clone());
                    } else {
                        self.constants.remove(name);
                    }
                }
            }
            Statement::Return { value, .. } => {
                if let Some(expr) = value {
                    changed |= self.fold_expression(expr)?;
                }
            }
            Statement::If { condition, then_branch, else_branch, .. } => {
                changed |= self.fold_expression(condition)?;
                
                // If condition is constant, we can eliminate dead branches
                if let Expression::Literal(Value::Boolean(cond_val)) = condition {
                    if *cond_val {
                        // Condition is true, eliminate else branch
                        else_branch.clear();
                    } else {
                        // Condition is false, eliminate then branch
                        then_branch.clear();
                    }
                    changed = true;
                }
                
                // Fold both branches
                for stmt in then_branch {
                    changed |= self.fold_statement(stmt)?;
                }
                for stmt in else_branch {
                    changed |= self.fold_statement(stmt)?;
                }
            }
            Statement::While { condition, body, .. } => {
                changed |= self.fold_expression(condition)?;
                
                // If condition is constant false, eliminate loop
                if let Expression::Literal(Value::Boolean(false)) = condition {
                    body.clear();
                    changed = true;
                } else {
                    for stmt in body {
                        changed |= self.fold_statement(stmt)?;
                    }
                }
            }
            Statement::Expression(expr, _) => {
                changed |= self.fold_expression(expr)?;
            }
            _ => {}
        }
        
        Ok(changed)
    }
}

impl OptimizationPass for ConstantFolding {
    fn run(&mut self, program: &mut Program) -> Result<bool, CompilerError> {
        let mut changed = false;
        
        // Fold constants in functions
        for function in &mut program.functions {
            for stmt in &mut function.body {
                changed |= self.fold_statement(stmt)?;
            }
        }
        
        Ok(changed)
    }

    fn name(&self) -> &'static str {
        "constant_folding"
    }
}

/// Dead code elimination optimization pass
pub struct DeadCodeElimination {
    used_variables: HashSet<String>,
}

impl DeadCodeElimination {
    pub fn new() -> Self {
        Self {
            used_variables: HashSet::new(),
        }
    }

    fn collect_used_variables(&mut self, program: &Program) {
        // Clear previous state
        self.used_variables.clear();
        
        // Collect variables used in functions
        for function in &program.functions {
            // Parameters are always considered used
            for param in &function.parameters {
                self.used_variables.insert(param.name.clone());
            }
            
            // Analyze function body
            for stmt in &function.body {
                self.collect_used_in_statement(stmt);
            }
        }
        
        // Collect variables used in top-level statements
        for stmt in &program.statements {
            self.collect_used_in_statement(stmt);
        }
    }

    fn collect_used_in_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::VariableDecl { initializer, .. } => {
                if let Some(expr) = initializer {
                    self.collect_used_in_expression(expr);
                }
            }
            Statement::Assignment { target, value, .. } => {
                self.collect_used_in_expression(target);
                self.collect_used_in_expression(value);
            }
            Statement::Return { value, .. } => {
                if let Some(expr) = value {
                    self.collect_used_in_expression(expr);
                }
            }
            Statement::If { condition, then_branch, else_branch, .. } => {
                self.collect_used_in_expression(condition);
                for stmt in then_branch {
                    self.collect_used_in_statement(stmt);
                }
                for stmt in else_branch {
                    self.collect_used_in_statement(stmt);
                }
            }
            Statement::While { condition, body, .. } => {
                self.collect_used_in_expression(condition);
                for stmt in body {
                    self.collect_used_in_statement(stmt);
                }
            }
            Statement::Expression(expr, _) => {
                self.collect_used_in_expression(expr);
            }
            _ => {}
        }
    }

    fn collect_used_in_expression(&mut self, expr: &Expression) {
        match expr {
            Expression::Variable(name) => {
                self.used_variables.insert(name.clone());
            }
            Expression::Binary(left, _, right, _) => {
                self.collect_used_in_expression(left);
                self.collect_used_in_expression(right);
            }
            Expression::Unary(_, expr, _) => {
                self.collect_used_in_expression(expr);
            }
            Expression::Call(_, args, _) => {
                for arg in args {
                    self.collect_used_in_expression(arg);
                }
            }
            Expression::Index(array, index, _) => {
                self.collect_used_in_expression(array);
                self.collect_used_in_expression(index);
            }
            Expression::Array(elements, _) => {
                for elem in elements {
                    self.collect_used_in_expression(elem);
                }
            }
            Expression::Matrix(rows, _) => {
                for row in rows {
                    for elem in row {
                        self.collect_used_in_expression(elem);
                    }
                }
            }
            _ => {}
        }
    }

    fn is_dead_code(&self, stmt: &Statement) -> bool {
        match stmt {
            Statement::VariableDecl { name, initializer, .. } => {
                // Variable declarations with side effects in initializer are not dead code
                if let Some(expr) = initializer {
                    if self.has_side_effects(expr) {
                        return false;
                    }
                }
                // Variable is dead if it's not used
                !self.used_variables.contains(name)
            }
            Statement::Assignment { target, value, .. } => {
                // Assignments with side effects in value are not dead code
                if self.has_side_effects(value) {
                    return false;
                }
                // Assignment to unused variable is dead code
                if let Expression::Variable(name) = target {
                    !self.used_variables.contains(name)
                } else {
                    false
                }
            }
            Statement::Return { .. } | Statement::Break { .. } | Statement::Continue { .. } => false,
            Statement::If { condition, .. } | Statement::While { condition, .. } => {
                // Control flow statements with side effects in condition are not dead code
                !self.has_side_effects(condition)
            }
            Statement::Expression(expr, _) => {
                // Expressions without side effects are dead code
                !self.has_side_effects(expr)
            }
            _ => false,
        }
    }

    fn has_side_effects(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Call(..) => true, // Assume all function calls have side effects
            Expression::Binary(left, _, right, _) => {
                self.has_side_effects(left) || self.has_side_effects(right)
            }
            Expression::Unary(_, expr, _) => self.has_side_effects(expr),
            Expression::Index(array, index, _) => {
                self.has_side_effects(array) || self.has_side_effects(index)
            }
            Expression::Array(elements, _) => {
                elements.iter().any(|e| self.has_side_effects(e))
            }
            Expression::Matrix(rows, _) => {
                rows.iter().any(|row| row.iter().any(|e| self.has_side_effects(e)))
            }
            _ => false,
        }
    }
}

impl OptimizationPass for DeadCodeElimination {
    fn run(&mut self, program: &mut Program) -> Result<bool, CompilerError> {
        let mut changed = false;
        
        // First pass: collect used variables
        self.collect_used_variables(program);
        
        // Second pass: remove unused variables and dead code
        for function in &mut program.functions {
            function.body.retain(|stmt| !self.is_dead_code(stmt));
            changed = true;
        }
        
        Ok(changed)
    }

    fn name(&self) -> &'static str {
        "dead_code_elimination"
    }
}

/// Common subexpression elimination optimization pass
pub struct CommonSubexpressionElimination {
    expressions: HashMap<String, Expression>,
}

impl CommonSubexpressionElimination {
    pub fn new() -> Self {
        Self {
            expressions: HashMap::new(),
        }
    }

    fn eliminate_common_subexpressions(&mut self, program: &mut Program) -> Result<bool, CompilerError> {
        let mut changed = false;
        
        // Process functions
        for function in &mut program.functions {
            changed |= self.eliminate_in_statements(&mut function.body)?;
        }
        
        // Process top-level statements
        changed |= self.eliminate_in_statements(&mut program.statements)?;
        
        Ok(changed)
    }

    fn eliminate_in_statements(&mut self, statements: &mut Vec<Statement>) -> Result<bool, CompilerError> {
        let mut changed = false;
        
        // Clear expression map for each block
        self.expressions.clear();
        
        for stmt in statements.iter_mut() {
            changed |= self.eliminate_in_statement(stmt)?;
        }
        
        Ok(changed)
    }

    fn eliminate_in_statement(&mut self, stmt: &mut Statement) -> Result<bool, CompilerError> {
        let mut changed = false;
        
        match stmt {
            Statement::VariableDecl { initializer, .. } => {
                if let Some(expr) = initializer {
                    changed |= self.eliminate_in_expression(expr)?;
                }
            }
            Statement::Assignment { target, value, .. } => {
                changed |= self.eliminate_in_expression(target)?;
                changed |= self.eliminate_in_expression(value)?;
            }
            Statement::Return { value, .. } => {
                if let Some(expr) = value {
                    changed |= self.eliminate_in_expression(expr)?;
                }
            }
            Statement::If { condition, then_branch, else_branch, .. } => {
                changed |= self.eliminate_in_expression(condition)?;
                
                // Process branches with their own scope
                let saved_expressions = self.expressions.clone();
                changed |= self.eliminate_in_statements(then_branch)?;
                self.expressions = saved_expressions.clone();
                changed |= self.eliminate_in_statements(else_branch)?;
                self.expressions = saved_expressions;
            }
            Statement::While { condition, body, .. } => {
                changed |= self.eliminate_in_expression(condition)?;
                
                // Process loop body with its own scope
                let saved_expressions = self.expressions.clone();
                changed |= self.eliminate_in_statements(body)?;
                self.expressions = saved_expressions;
            }
            Statement::Expression(expr, _) => {
                changed |= self.eliminate_in_expression(expr)?;
            }
            _ => {}
        }
        
        Ok(changed)
    }

    fn eliminate_in_expression(&mut self, expr: &mut Expression) -> Result<bool, CompilerError> {
        let mut changed = false;
        
        // First eliminate in subexpressions
        match expr {
            Expression::Binary(left, _, right, _) => {
                changed |= self.eliminate_in_expression(left)?;
                changed |= self.eliminate_in_expression(right)?;
            }
            Expression::Unary(_, inner, _) => {
                changed |= self.eliminate_in_expression(inner)?;
            }
            Expression::Call(_, args, _) => {
                for arg in args {
                    changed |= self.eliminate_in_expression(arg)?;
                }
            }
            Expression::Index(array, index, _) => {
                changed |= self.eliminate_in_expression(array)?;
                changed |= self.eliminate_in_expression(index)?;
            }
            Expression::Array(elements, _) => {
                for elem in elements {
                    changed |= self.eliminate_in_expression(elem)?;
                }
            }
            Expression::Matrix(rows, _) => {
                for row in rows {
                    for elem in row {
                        changed |= self.eliminate_in_expression(elem)?;
                    }
                }
            }
            _ => {}
        }
        
        // Then try to find a common subexpression
        let expr_key = self.expression_key(expr);
        if let Some(existing_expr) = self.expressions.get(&expr_key) {
            // Found a common subexpression, replace current expression with a variable
            let temp_var = format!("_cse_{}", self.expressions.len());
            *expr = Expression::Variable(temp_var);
            changed = true;
        } else {
            // No common subexpression found, add this one to the map
            self.expressions.insert(expr_key, expr.clone());
        }
        
        Ok(changed)
    }

    fn expression_key(&self, expr: &Expression) -> String {
        match expr {
            Expression::Literal(value) => format!("lit:{:?}", value),
            Expression::Variable(name) => format!("var:{}", name),
            Expression::Binary(left, op, right, _) => {
                format!("bin:{}:{}:{}", 
                    self.expression_key(left),
                    op,
                    self.expression_key(right))
            }
            Expression::Unary(op, expr, _) => {
                format!("un:{}:{}", op, self.expression_key(expr))
            }
            Expression::Call(name, args, _) => {
                let args_key = args.iter()
                    .map(|arg| self.expression_key(arg))
                    .collect::<Vec<_>>()
                    .join(",");
                format!("call:{}:{}", name, args_key)
            }
            Expression::Index(array, index, _) => {
                format!("idx:{}:{}",
                    self.expression_key(array),
                    self.expression_key(index))
            }
            Expression::Array(elements, _) => {
                let elems_key = elements.iter()
                    .map(|e| self.expression_key(e))
                    .collect::<Vec<_>>()
                    .join(",");
                format!("arr:{}", elems_key)
            }
            Expression::Matrix(rows, _) => {
                let rows_key = rows.iter()
                    .map(|row| {
                        row.iter()
                            .map(|e| self.expression_key(e))
                            .collect::<Vec<_>>()
                            .join(",")
                    })
                    .collect::<Vec<_>>()
                    .join(";");
                format!("mat:{}", rows_key)
            }
            _ => format!("other:{:?}", expr),
        }
    }
}

impl OptimizationPass for CommonSubexpressionElimination {
    fn run(&mut self, program: &mut Program) -> Result<bool, CompilerError> {
        self.eliminate_common_subexpressions(program)
    }

    fn name(&self) -> &'static str {
        "common_subexpression_elimination"
    }
}

/// Control flow optimization pass
pub struct ControlFlowOptimization;

impl ControlFlowOptimization {
    pub fn new() -> Self {
        Self
    }

    fn optimize_control_flow(&mut self, program: &mut Program) -> Result<bool, CompilerError> {
        let mut changed = false;
        
        // Optimize functions
        for function in &mut program.functions {
            changed |= self.optimize_statements(&mut function.body)?;
        }
        
        // Optimize top-level statements
        changed |= self.optimize_statements(&mut program.statements)?;
        
        Ok(changed)
    }

    fn optimize_statements(&mut self, statements: &mut Vec<Statement>) -> Result<bool, CompilerError> {
        let mut changed = false;
        let mut i = 0;
        
        while i < statements.len() {
            // Check for unreachable code after return/break/continue
            if i > 0 {
                match &statements[i - 1] {
                    Statement::Return { .. } | Statement::Break { .. } | Statement::Continue { .. } => {
                        statements.truncate(i);
                        changed = true;
                        break;
                    }
                    _ => {}
                }
            }
            
            // Optimize individual statement
            changed |= self.optimize_statement(&mut statements[i])?;
            
            // Remove empty blocks
            match &statements[i] {
                Statement::If { condition, then_branch, else_branch, .. } => {
                    if then_branch.is_empty() && else_branch.is_empty() {
                        // Convert to simple expression if condition has side effects
                        if self.has_side_effects(condition) {
                            statements[i] = Statement::Expression(condition.clone(), None);
                            changed = true;
                        } else {
                            // Remove empty if statement
                            statements.remove(i);
                            changed = true;
                            continue;
                        }
                    }
                }
                Statement::While { condition, body, .. } => {
                    if body.is_empty() {
                        // Convert to simple expression if condition has side effects
                        if self.has_side_effects(condition) {
                            statements[i] = Statement::Expression(condition.clone(), None);
                            changed = true;
                        } else {
                            // Remove empty while loop
                            statements.remove(i);
                            changed = true;
                            continue;
                        }
                    }
                }
                _ => {}
            }
            
            i += 1;
        }
        
        Ok(changed)
    }

    fn optimize_statement(&mut self, stmt: &mut Statement) -> Result<bool, CompilerError> {
        let mut changed = false;
        
        match stmt {
            Statement::If { condition, then_branch, else_branch, .. } => {
                // Optimize condition
                changed |= self.optimize_expression(condition)?;
                
                // Optimize branches
                changed |= self.optimize_statements(then_branch)?;
                changed |= self.optimize_statements(else_branch)?;
                
                // Simplify if-else patterns
                if let Expression::Literal(Value::Boolean(cond_val)) = condition {
                    if *cond_val {
                        // True condition - keep only then branch
                        *stmt = Statement::Block(then_branch.clone(), None);
                        changed = true;
                    } else {
                        // False condition - keep only else branch
                        *stmt = Statement::Block(else_branch.clone(), None);
                        changed = true;
                    }
                }
            }
            Statement::While { condition, body, .. } => {
                // Optimize condition
                changed |= self.optimize_expression(condition)?;
                
                // Optimize loop body
                changed |= self.optimize_statements(body)?;
                
                // Check for infinite/empty loops
                if let Expression::Literal(Value::Boolean(cond_val)) = condition {
                    if !*cond_val {
                        // Condition is always false - remove loop
                        *stmt = Statement::Block(vec![], None);
                        changed = true;
                    }
                }
            }
            Statement::Expression(expr, _) => {
                changed |= self.optimize_expression(expr)?;
            }
            Statement::Return { value, .. } => {
                if let Some(expr) = value {
                    changed |= self.optimize_expression(expr)?;
                }
            }
            Statement::Assignment { target, value, .. } => {
                changed |= self.optimize_expression(target)?;
                changed |= self.optimize_expression(value)?;
            }
            Statement::VariableDecl { initializer, .. } => {
                if let Some(expr) = initializer {
                    changed |= self.optimize_expression(expr)?;
                }
            }
            _ => {}
        }
        
        Ok(changed)
    }

    fn optimize_expression(&mut self, expr: &mut Expression) -> Result<bool, CompilerError> {
        let mut changed = false;
        
        match expr {
            Expression::Binary(left, op, right, location) => {
                // Optimize operands
                changed |= self.optimize_expression(left)?;
                changed |= self.optimize_expression(right)?;
                
                // Perform constant folding
                if let (Expression::Literal(left_val), Expression::Literal(right_val)) = (&**left, &**right) {
                    if let Some(result) = evaluate_binary_op(left_val, op, right_val) {
                        *expr = Expression::Literal(result);
                        changed = true;
                    }
                }
            }
            Expression::Unary(op, inner, location) => {
                // Optimize operand
                changed |= self.optimize_expression(inner)?;
                
                // Perform constant folding
                if let Expression::Literal(val) = &**inner {
                    if let Some(result) = evaluate_unary_op(op, val) {
                        *expr = Expression::Literal(result);
                        changed = true;
                    }
                }
            }
            Expression::Call(name, args, location) => {
                // Optimize arguments
                for arg in args {
                    changed |= self.optimize_expression(arg)?;
                }
            }
            Expression::Index(array, index, location) => {
                changed |= self.optimize_expression(array)?;
                changed |= self.optimize_expression(index)?;
            }
            Expression::Array(elements, location) => {
                for elem in elements {
                    changed |= self.optimize_expression(elem)?;
                }
            }
            Expression::Matrix(rows, location) => {
                for row in rows {
                    for elem in row {
                        changed |= self.optimize_expression(elem)?;
                    }
                }
            }
            _ => {}
        }
        
        Ok(changed)
    }

    fn has_side_effects(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Call(..) => true,
            Expression::Binary(left, _, right, _) => {
                self.has_side_effects(left) || self.has_side_effects(right)
            }
            Expression::Unary(_, expr, _) => self.has_side_effects(expr),
            Expression::Index(array, index, _) => {
                self.has_side_effects(array) || self.has_side_effects(index)
            }
            Expression::Array(elements, _) => {
                elements.iter().any(|e| self.has_side_effects(e))
            }
            Expression::Matrix(rows, _) => {
                rows.iter().any(|row| row.iter().any(|e| self.has_side_effects(e)))
            }
            _ => false,
        }
    }
}

impl OptimizationPass for ControlFlowOptimization {
    fn run(&mut self, program: &mut Program) -> Result<bool, CompilerError> {
        self.optimize_control_flow(program)
    }

    fn name(&self) -> &'static str {
        "control_flow_optimization"
    }
}

// Helper functions

fn evaluate_binary_op(left: &Value, op: &ast::Operator, right: &Value) -> Option<Value> {
    match (left, op, right) {
        (Value::Integer(l), ast::Operator::Add, Value::Integer(r)) => Some(Value::Integer(l + r)),
        (Value::Integer(l), ast::Operator::Subtract, Value::Integer(r)) => Some(Value::Integer(l - r)),
        (Value::Integer(l), ast::Operator::Multiply, Value::Integer(r)) => Some(Value::Integer(l * r)),
        (Value::Integer(l), ast::Operator::Divide, Value::Integer(r)) if *r != 0 => Some(Value::Integer(l / r)),
        (Value::Number(l), ast::Operator::Add, Value::Number(r)) => Some(Value::Number(l + r)),
        (Value::Number(l), ast::Operator::Subtract, Value::Number(r)) => Some(Value::Number(l - r)),
        (Value::Number(l), ast::Operator::Multiply, Value::Number(r)) => Some(Value::Number(l * r)),
        (Value::Number(l), ast::Operator::Divide, Value::Number(r)) if *r != 0.0 => Some(Value::Number(l / r)),
        (Value::Boolean(l), ast::Operator::And, Value::Boolean(r)) => Some(Value::Boolean(*l && *r)),
        (Value::Boolean(l), ast::Operator::Or, Value::Boolean(r)) => Some(Value::Boolean(*l || *r)),
        _ => None,
    }
}

fn evaluate_unary_op(op: &ast::UnaryOperator, val: &Value) -> Option<Value> {
    match (op, val) {
        (ast::UnaryOperator::Negate, Value::Integer(n)) => Some(Value::Integer(-n)),
        (ast::UnaryOperator::Negate, Value::Number(n)) => Some(Value::Number(-n)),
        (ast::UnaryOperator::Not, Value::Boolean(b)) => Some(Value::Boolean(!b)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Statement, Expression, Value, Type};

    #[test]
    fn test_constant_folding() {
        let mut optimizer = Optimizer::new();
        let mut program = Program {
            functions: vec![
                Function {
                    name: "test".to_string(),
                    parameters: vec![],
                    return_type: None,
                    body: vec![
                        Statement::VariableDecl {
                            name: "x".to_string(),
                            type_: Some(Type::Integer),
                            initializer: Some(Expression::Binary(
                                Box::new(Expression::Literal(Value::Integer(2))),
                                ast::Operator::Add,
                                Box::new(Expression::Literal(Value::Integer(3))),
                                None,
                            )),
                            location: None,
                        },
                    ],
                    location: None,
                },
            ],
            statements: vec![],
        };

        optimizer.optimize(&mut program).unwrap();

        if let Statement::VariableDecl { initializer, .. } = &program.functions[0].body[0] {
            assert!(matches!(
                initializer,
                Some(Expression::Literal(Value::Integer(5)))
            ));
        } else {
            panic!("Expected VariableDecl");
        }
    }

    #[test]
    fn test_dead_code_elimination() {
        let mut optimizer = Optimizer::new();
        let mut program = Program {
            functions: vec![
                Function {
                    name: "test".to_string(),
                    parameters: vec![],
                    return_type: None,
                    body: vec![
                        Statement::VariableDecl {
                            name: "unused".to_string(),
                            type_: Some(Type::Integer),
                            initializer: Some(Expression::Literal(Value::Integer(42))),
                            location: None,
                        },
                        Statement::Return {
                            value: Some(Expression::Literal(Value::Integer(0))),
                            location: None,
                        },
                    ],
                    location: None,
                },
            ],
            statements: vec![],
        };

        optimizer.optimize(&mut program).unwrap();
        assert_eq!(program.functions[0].body.len(), 1);
    }

    #[test]
    fn test_control_flow_optimization() {
        let mut optimizer = Optimizer::new();
        let mut program = Program {
            functions: vec![
                Function {
                    name: "test".to_string(),
                    parameters: vec![],
                    return_type: None,
                    body: vec![
                        // Test removal of unreachable code
                        Statement::Return {
                            value: Some(Expression::Literal(Value::Integer(1))),
                            location: None,
                        },
                        Statement::Expression(
                            Expression::Literal(Value::Integer(2)),
                            None,
                        ),
                        
                        // Test empty if statement removal
                        Statement::If {
                            condition: Expression::Literal(Value::Boolean(true)),
                            then_branch: vec![],
                            else_branch: vec![],
                            location: None,
                        },
                        
                        // Test empty while loop removal
                        Statement::While {
                            condition: Expression::Literal(Value::Boolean(false)),
                            body: vec![],
                            location: None,
                        },
                    ],
                    location: None,
                },
            ],
            statements: vec![],
        };

        optimizer.optimize(&mut program).unwrap();

        // Only the return statement should remain
        assert_eq!(program.functions[0].body.len(), 1);
        assert!(matches!(
            program.functions[0].body[0],
            Statement::Return { .. }
        ));
    }

    #[test]
    fn test_constant_condition_optimization() {
        let mut optimizer = Optimizer::new();
        let mut program = Program {
            functions: vec![
                Function {
                    name: "test".to_string(),
                    parameters: vec![],
                    return_type: None,
                    body: vec![
                        // Test if with constant true condition
                        Statement::If {
                            condition: Expression::Literal(Value::Boolean(true)),
                            then_branch: vec![
                                Statement::Expression(
                                    Expression::Literal(Value::Integer(1)),
                                    None,
                                ),
                            ],
                            else_branch: vec![
                                Statement::Expression(
                                    Expression::Literal(Value::Integer(2)),
                                    None,
                                ),
                            ],
                            location: None,
                        },
                        
                        // Test while with constant false condition
                        Statement::While {
                            condition: Expression::Literal(Value::Boolean(false)),
                            body: vec![
                                Statement::Expression(
                                    Expression::Literal(Value::Integer(3)),
                                    None,
                                ),
                            ],
                            location: None,
                        },
                    ],
                    location: None,
                },
            ],
            statements: vec![],
        };

        optimizer.optimize(&mut program).unwrap();

        // The if statement should be replaced with its then branch
        // The while loop should be removed
        assert_eq!(program.functions[0].body.len(), 1);
        if let Statement::Block(block, _) = &program.functions[0].body[0] {
            assert_eq!(block.len(), 1);
            assert!(matches!(
                block[0],
                Statement::Expression(Expression::Literal(Value::Integer(1)), _)
            ));
        } else {
            panic!("Expected Block statement");
        }
    }

    #[test]
    fn test_side_effects_preservation() {
        let mut optimizer = Optimizer::new();
        let mut program = Program {
            functions: vec![
                Function {
                    name: "test".to_string(),
                    parameters: vec![],
                    return_type: None,
                    body: vec![
                        // Test if with side effects in condition
                        Statement::If {
                            condition: Expression::Call(
                                "has_side_effects".to_string(),
                                vec![],
                                None,
                            ),
                            then_branch: vec![],
                            else_branch: vec![],
                            location: None,
                        },
                        
                        // Test while with side effects in condition
                        Statement::While {
                            condition: Expression::Call(
                                "has_side_effects".to_string(),
                                vec![],
                                None,
                            ),
                            body: vec![],
                            location: None,
                        },
                    ],
                    location: None,
                },
            ],
            statements: vec![],
        };

        optimizer.optimize(&mut program).unwrap();

        // Both statements should be converted to expressions
        assert_eq!(program.functions[0].body.len(), 2);
        assert!(matches!(
            program.functions[0].body[0],
            Statement::Expression(Expression::Call(_, _, _), _)
        ));
        assert!(matches!(
            program.functions[0].body[1],
            Statement::Expression(Expression::Call(_, _, _), _)
        ));
    }
} 