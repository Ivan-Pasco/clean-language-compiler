use crate::error::CompilerError;
use crate::ast::{Program, Function, Statement, Expression};
use crate::error::{CompilerWarning};

/// Debug utilities for Clean Language development
pub struct DebugUtils;

impl DebugUtils {
    /// Pretty print the AST structure for debugging
    pub fn print_ast(program: &Program) {
        println!("=== Clean Language AST Debug ===");
        
        if let Some(start_fn) = &program.start_function {
            println!("Start Function:");
            Self::print_function(start_fn, 1);
        }
        
        if !program.functions.is_empty() {
            println!("Functions:");
            for func in &program.functions {
                Self::print_function(func, 1);
            }
        }
        
        if !program.classes.is_empty() {
            println!("Classes:");
            for class in &program.classes {
                println!("  Class: {}", class.name);
            }
        }
    }
    
    /// Print function details with indentation
    fn print_function(func: &Function, indent: usize) {
        let indent_str = "  ".repeat(indent);
        println!("{}Function: {} -> {:?}", indent_str, func.name, func.return_type);
        
        if !func.parameters.is_empty() {
            println!("{}  Parameters:", indent_str);
            for param in &func.parameters {
                println!("{}    {}: {:?}", indent_str, param.name, param.type_);
            }
        }
        
        if !func.body.is_empty() {
            println!("{}  Body:", indent_str);
            for stmt in &func.body {
                println!("{}    {}", indent_str, Self::statement_summary(stmt));
            }
        }
    }
    
    /// Get a summary string for a statement
    fn statement_summary(stmt: &Statement) -> String {
        match stmt {
            Statement::VariableDecl { name, type_, .. } => {
                format!("Variable: {} : {:?}", name, type_)
            },
            Statement::Assignment { target, .. } => format!("Assignment: {}", target),
            Statement::Print { .. } => "Print statement".to_string(),
            Statement::PrintBlock { .. } => "Print block".to_string(),
            Statement::Return { .. } => "Return statement".to_string(),
            Statement::If { .. } => "If statement".to_string(),
            Statement::Iterate { iterator, .. } => format!("Iterate: {}", iterator),
            Statement::RangeIterate { iterator, .. } => format!("Range iterate: {}", iterator),
            Statement::Test { name, .. } => format!("Test: {}", name),
            Statement::Expression { .. } => "Expression statement".to_string(),
            Statement::Error { .. } => "Error statement".to_string(),
            Statement::Import { .. } => "Import statement".to_string(),
            Statement::LaterAssignment { variable, .. } => format!("Later assignment: {}", variable),
            Statement::Background { .. } => "Background statement".to_string(),
            Statement::TypeApplyBlock { .. } => "Type apply block".to_string(),
            Statement::FunctionApplyBlock { .. } => "Function apply block".to_string(),
            Statement::MethodApplyBlock { .. } => "Method apply block".to_string(),
            Statement::ConstantApplyBlock { .. } => "Constant apply block".to_string(),
        }
    }
    
    /// Get a summary string for an expression
    fn expression_summary(expr: &Expression) -> String {
        match expr {
            Expression::Literal(_) => "Literal".to_string(),
            Expression::Variable(name) => format!("Variable({})", name),
            Expression::Call(name, _) => format!("Call({})", name),
            Expression::MethodCall { method, .. } => format!("MethodCall({})", method),
            Expression::PropertyAccess { property, .. } => format!("PropertyAccess({})", property),
            Expression::Binary(_, operator, _) => format!("BinaryOp({:?})", operator),
            Expression::Unary(operator, _) => format!("UnaryOp({:?})", operator),
            Expression::ArrayAccess(_, _) => "ArrayAccess".to_string(),
            Expression::MatrixAccess(_, _, _) => "MatrixAccess".to_string(),
            Expression::StringInterpolation(_) => "StringInterpolation".to_string(),
            Expression::ObjectCreation { class_name, .. } => format!("ObjectCreation({})", class_name),
            Expression::OnError { .. } => "OnError".to_string(),
            Expression::OnErrorBlock { .. } => "OnErrorBlock".to_string(),
            Expression::ErrorVariable { .. } => "ErrorVariable".to_string(),
            Expression::Conditional { .. } => "Conditional".to_string(),
            Expression::BaseCall { .. } => "BaseCall".to_string(),
            Expression::StartExpression { .. } => "StartExpression".to_string(),
            Expression::LaterAssignment { variable, .. } => format!("LaterAssignment({})", variable),
            Expression::StaticMethodCall { class_name, method, .. } => {
                format!("StaticMethodCall({}.{})", class_name, method)
            },
            Expression::PropertyAssignment { property, .. } => format!("PropertyAssignment({})", property),
        }
    }
    
    /// Analyze code complexity and provide suggestions
    pub fn analyze_complexity(program: &Program) -> Vec<String> {
        let mut suggestions = Vec::new();
        
        // Check function complexity
        for func in &program.functions {
            if func.body.len() > 20 {
                suggestions.push(format!(
                    "Function '{}' has {} statements. Consider breaking it into smaller functions.",
                    func.name, func.body.len()
                ));
            }
            
            if func.parameters.len() > 5 {
                suggestions.push(format!(
                    "Function '{}' has {} parameters. Consider using a struct or reducing parameters.",
                    func.name, func.parameters.len()
                ));
            }
        }
        
        // Check for deeply nested structures
        for func in &program.functions {
            let max_depth = Self::calculate_nesting_depth(&func.body);
            if max_depth > 4 {
                suggestions.push(format!(
                    "Function '{}' has nesting depth of {}. Consider refactoring to reduce complexity.",
                    func.name, max_depth
                ));
            }
        }
        
        suggestions
    }
    
    /// Calculate maximum nesting depth in statements
    fn calculate_nesting_depth(statements: &[Statement]) -> usize {
        let mut max_depth = 0;
        
        for stmt in statements {
            let depth = match stmt {
                Statement::If { then_branch, else_branch, .. } => {
                    let then_depth = Self::calculate_nesting_depth(then_branch);
                    let else_depth = else_branch.as_ref()
                        .map(|branch| Self::calculate_nesting_depth(branch))
                        .unwrap_or(0);
                    1 + then_depth.max(else_depth)
                },
                Statement::Iterate { body, .. } | Statement::RangeIterate { body, .. } => {
                    1 + Self::calculate_nesting_depth(body)
                },
                Statement::Test { body, .. } => {
                    1 + Self::calculate_nesting_depth(body)
                },
                _ => 0,
            };
            max_depth = max_depth.max(depth);
        }
        
        max_depth
    }
    
    /// Generate a code style report
    pub fn generate_style_report(program: &Program) -> StyleReport {
        let mut report = StyleReport::new();
        
        // Check naming conventions
        for func in &program.functions {
            if !Self::is_camel_case(&func.name) {
                report.add_warning(format!(
                    "Function '{}' should use camelCase naming convention",
                    func.name
                ));
            }
            
            for param in &func.parameters {
                if !Self::is_camel_case(&param.name) {
                    report.add_warning(format!(
                        "Parameter '{}' in function '{}' should use camelCase naming convention",
                        param.name, func.name
                    ));
                }
            }
        }
        
        // Check for missing descriptions
        for func in &program.functions {
            if func.description.is_none() && func.name != "start" {
                report.add_suggestion(format!(
                    "Function '{}' should have a description for better documentation",
                    func.name
                ));
            }
        }
        
        report
    }
    
    /// Check if a string follows camelCase convention
    fn is_camel_case(s: &str) -> bool {
        if s.is_empty() {
            return false;
        }
        
        let first_char = s.chars().next().unwrap();
        if !first_char.is_lowercase() {
            return false;
        }
        
        // Check for underscores (not allowed in camelCase)
        !s.contains('_')
    }
    
    /// Print detailed error analysis
    pub fn analyze_error(error: &CompilerError) {
        println!("=== Error Analysis ===");
        println!("Error: {}", error);
        
        // Provide contextual help based on error type
        match error {
            CompilerError::Syntax { .. } => {
                println!("💡 Syntax Error Help:");
                println!("  - Check for missing indentation (Clean Language uses tabs)");
                println!("  - Verify function declarations have proper syntax");
                println!("  - Ensure all blocks are properly indented");
            },
            CompilerError::Type { .. } => {
                println!("💡 Type Error Help:");
                println!("  - Check variable types match expected values");
                println!("  - Verify function arguments match parameter types");
                println!("  - Consider using type conversion methods");
            },
            CompilerError::Memory { .. } => {
                println!("💡 Memory Error Help:");
                println!("  - Check for large data structures");
                println!("  - Consider optimizing memory usage");
                println!("  - Verify array/matrix sizes are reasonable");
            },
            _ => {
                println!("💡 General Help:");
                println!("  - Check the Clean Language documentation");
                println!("  - Verify syntax matches language specification");
                println!("  - Consider simplifying complex expressions");
            }
        }
    }

    /// Analyze multiple errors and provide comprehensive feedback
    pub fn analyze_errors(errors: &[CompilerError]) -> Vec<String> {
        let mut analysis = Vec::new();
        
        for error in errors {
            analysis.push(format!("Error: {}", error));
            
            // Add specific suggestions based on error type
            match error {
                CompilerError::Syntax { .. } => {
                    analysis.push("  → Check indentation and syntax".to_string());
                },
                CompilerError::Type { .. } => {
                    analysis.push("  → Verify type compatibility".to_string());
                },
                CompilerError::Memory { .. } => {
                    analysis.push("  → Optimize memory usage".to_string());
                },
                _ => {
                    analysis.push("  → Review code structure".to_string());
                }
            }
        }
        
        analysis
    }

    /// Validate code style and return issues
    pub fn validate_style(source: &str) -> Vec<String> {
        let mut issues = Vec::new();
        
        // Basic style checks on source code
        let lines: Vec<&str> = source.lines().collect();
        
        for (line_num, line) in lines.iter().enumerate() {
            // Check for mixed indentation
            if line.starts_with(' ') && line.contains('\t') {
                issues.push(format!("Line {}: Mixed spaces and tabs", line_num + 1));
            }
            
            // Check for trailing whitespace
            if line.ends_with(' ') || line.ends_with('\t') {
                issues.push(format!("Line {}: Trailing whitespace", line_num + 1));
            }
            
            // Check line length
            if line.len() > 120 {
                issues.push(format!("Line {}: Line too long ({} characters)", line_num + 1, line.len()));
            }
        }
        
        issues
    }

    /// Create a comprehensive debug report
    pub fn create_debug_report(
        source: &str,
        file_path: &str,
        parse_result: &Result<crate::ast::Program, CompilerError>,
        warnings: &[CompilerWarning],
    ) -> String {
        let mut report = String::new();
        
        report.push_str(&format!("=== Debug Report for {} ===\n", file_path));
        report.push_str(&format!("Source length: {} characters\n", source.len()));
        report.push_str(&format!("Source lines: {}\n", source.lines().count()));
        
        match parse_result {
            Ok(program) => {
                report.push_str("✅ Parsing: SUCCESS\n");
                report.push_str(&format!("Functions: {}\n", program.functions.len()));
                report.push_str(&format!("Classes: {}\n", program.classes.len()));
                report.push_str(&format!("Has start function: {}\n", program.start_function.is_some()));
            },
            Err(error) => {
                report.push_str("❌ Parsing: FAILED\n");
                report.push_str(&format!("Error: {}\n", error));
            }
        }
        
        if !warnings.is_empty() {
            report.push_str(&format!("⚠️  Warnings: {}\n", warnings.len()));
            for warning in warnings {
                report.push_str(&format!("  - {}\n", warning));
            }
        }
        
        // Add style analysis
        let style_issues = Self::validate_style(source);
        if !style_issues.is_empty() {
            report.push_str(&format!("🎨 Style Issues: {}\n", style_issues.len()));
            for issue in style_issues {
                report.push_str(&format!("  - {}\n", issue));
            }
        }
        
        report
    }
}

/// Style report for code analysis
pub struct StyleReport {
    warnings: Vec<String>,
    suggestions: Vec<String>,
}

impl StyleReport {
    fn new() -> Self {
        Self {
            warnings: Vec::new(),
            suggestions: Vec::new(),
        }
    }
    
    fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }
    
    fn add_suggestion(&mut self, suggestion: String) {
        self.suggestions.push(suggestion);
    }
    
    pub fn print(&self) {
        println!("=== Style Report ===");
        
        if !self.warnings.is_empty() {
            println!("⚠️  Warnings:");
            for warning in &self.warnings {
                println!("  - {}", warning);
            }
        }
        
        if !self.suggestions.is_empty() {
            println!("💡 Suggestions:");
            for suggestion in &self.suggestions {
                println!("  - {}", suggestion);
            }
        }
        
        if self.warnings.is_empty() && self.suggestions.is_empty() {
            println!("✅ No style issues found!");
        }
    }
} 