use std::fmt;
use std::error::Error;

use crate::ast::SourceLocation;

#[derive(Debug, Clone)]
pub struct StackFrame {
    pub function_name: String,
    pub location: Option<SourceLocation>,
}

impl StackFrame {
    pub fn new<T: Into<String>>(function_name: T, location: Option<SourceLocation>) -> Self {
        Self {
            function_name: function_name.into(),
            location,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub message: String,
    pub help: Option<String>,
    pub error_type: ErrorType,
    pub location: Option<SourceLocation>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ErrorType {
    Syntax,
    Type,
    Memory,
    Codegen,
    IO,
    Runtime,
    Validation,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WarningType {
    UnusedVariable,
    UnusedFunction,
    UnusedImport,
    DeadCode,
    TypeInference,
    Performance,
    Style,
    Deprecation,
}

impl ErrorContext {
    pub fn new<T: Into<String>>(message: T, help: Option<String>, error_type: ErrorType, location: Option<SourceLocation>) -> Self {
        Self {
            message: message.into(),
            help,
            error_type,
            location,
        }
    }

    pub fn with_help<T: Into<String>>(mut self, help: T) -> Self {
        self.help = Some(help.into());
        self
    }

    pub fn with_location(mut self, location: SourceLocation) -> Self {
        self.location = Some(location);
        self
    }
    
    // Add convenience methods for optional location and help
    pub fn with_location_option(mut self, location: Option<SourceLocation>) -> Self {
        if let Some(loc) = location {
            self.location = Some(loc);
        }
        self
    }

    pub fn with_help_option(mut self, help: Option<String>) -> Self {
        if let Some(h) = help {
            self.help = Some(h);
        }
        self
    }

    pub fn add_stack_frame(&mut self, _frame: StackFrame) {
        // Implementation needed
    }

    pub fn with_stack_frame(self, _frame: StackFrame) -> Self {
        // Implementation needed
        self
    }
}

impl Into<String> for ErrorContext {
    fn into(self) -> String {
        let mut result = format!("Error: {}\n", self.message);
        
        if let Some(help) = &self.help {
            result.push_str(&format!("Help: {}\n", help));
        }
        
        if let Some(location) = &self.location {
            result.push_str(&format!(
                "Location: {}:{}:{}\n",
                location.file, location.line, location.column
            ));
        }
        
        result
    }
}

impl From<String> for ErrorContext {
    fn from(message: String) -> Self {
        Self::new(message, None, ErrorType::Syntax, None)
    }
}

impl From<&str> for ErrorContext {
    fn from(message: &str) -> Self {
        Self::new(message.to_string(), None, ErrorType::Syntax, None)
    }
}

impl fmt::Display for ErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Error: {}", self.message)?;
        
        if let Some(help) = &self.help {
            writeln!(f, "Help: {}", help)?;
        }
        
        if let Some(location) = &self.location {
            writeln!(
                f,
                "Location: {}:{}:{}",
                location.file, location.line, location.column
            )?;
        }
        
        Ok(())
    }
}

impl Default for ErrorContext {
    fn default() -> Self {
        Self {
            message: String::new(),
            help: None,
            error_type: ErrorType::Syntax,
            location: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum CompilerError {
    Syntax {
        context: ErrorContext,
    },
    Type {
        context: ErrorContext,
    },
    Memory {
        context: ErrorContext,
    },
    Codegen {
        context: ErrorContext,
    },
    IO {
        context: ErrorContext,
    },
    Runtime {
        context: ErrorContext,
    },
    Validation {
        context: ErrorContext,
    },
}

impl CompilerError {
    pub fn syntax_error<T: Into<String>>(message: T, help: Option<String>, location: Option<SourceLocation>) -> Self {
        CompilerError::Syntax {
            context: ErrorContext::new(message, help, ErrorType::Syntax, location),
        }
    }

    pub fn type_error<T: Into<String>>(message: T, help: Option<String>, location: Option<SourceLocation>) -> Self {
        CompilerError::Type {
            context: ErrorContext::new(message, help, ErrorType::Type, location),
        }
    }

    pub fn memory_error<T: Into<String>>(message: T, help: Option<String>, location: Option<SourceLocation>) -> Self {
        CompilerError::Memory {
            context: ErrorContext::new(message, help, ErrorType::Memory, location),
        }
    }

    pub fn codegen_error<T: Into<String>>(message: T, help: Option<String>, location: Option<SourceLocation>) -> Self {
        CompilerError::Codegen {
            context: ErrorContext::new(message, help, ErrorType::Codegen, location),
        }
    }

    pub fn io_error<T: Into<String>>(message: T, help: Option<String>, location: Option<SourceLocation>) -> Self {
        CompilerError::IO {
            context: ErrorContext::new(message, help, ErrorType::IO, location),
        }
    }

    pub fn runtime_error<T: Into<String>>(message: T, help: Option<String>, location: Option<SourceLocation>) -> Self {
        CompilerError::Runtime {
            context: ErrorContext::new(message, help, ErrorType::Runtime, location),
        }
    }

    pub fn validation_error<T: Into<String>>(message: T, help: Option<String>, location: Option<SourceLocation>) -> Self {
        CompilerError::Validation {
            context: ErrorContext::new(message, help, ErrorType::Validation, location),
        }
    }
    
    // Add parse_error method that works with the Syntax variant
    pub fn parse_error<T: Into<String>>(message: T, location: Option<SourceLocation>, help: Option<String>) -> Self {
        CompilerError::Syntax {
            context: ErrorContext::new(message, help, ErrorType::Syntax, location),
        }
    }

    // Add the with_help and with_location methods to CompilerError
    pub fn with_help<T: Into<String>>(self, help_text: T) -> Self {
        match self {
            CompilerError::Syntax { mut context } => {
                context = context.with_help(help_text);
                CompilerError::Syntax { context }
            }
            CompilerError::Type { mut context } => {
                context = context.with_help(help_text);
                CompilerError::Type { context }
            }
            CompilerError::Memory { mut context } => {
                context = context.with_help(help_text);
                CompilerError::Memory { context }
            }
            CompilerError::Codegen { mut context } => {
                context = context.with_help(help_text);
                CompilerError::Codegen { context }
            }
            CompilerError::IO { mut context } => {
                context = context.with_help(help_text);
                CompilerError::IO { context }
            }
            CompilerError::Runtime { mut context } => {
                context = context.with_help(help_text);
                CompilerError::Runtime { context }
            }
            CompilerError::Validation { mut context } => {
                context = context.with_help(help_text);
                CompilerError::Validation { context }
            }
        }
    }

    pub fn with_help_option(self, help_text: Option<String>) -> Self {
        match self {
            CompilerError::Syntax { mut context } => {
                context = context.with_help_option(help_text);
                CompilerError::Syntax { context }
            }
            CompilerError::Type { mut context } => {
                context = context.with_help_option(help_text);
                CompilerError::Type { context }
            }
            CompilerError::Memory { mut context } => {
                context = context.with_help_option(help_text);
                CompilerError::Memory { context }
            }
            CompilerError::Codegen { mut context } => {
                context = context.with_help_option(help_text);
                CompilerError::Codegen { context }
            }
            CompilerError::IO { mut context } => {
                context = context.with_help_option(help_text);
                CompilerError::IO { context }
            }
            CompilerError::Runtime { mut context } => {
                context = context.with_help_option(help_text);
                CompilerError::Runtime { context }
            }
            CompilerError::Validation { mut context } => {
                context = context.with_help_option(help_text);
                CompilerError::Validation { context }
            }
        }
    }

    pub fn with_location(self, location: SourceLocation) -> Self {
        match self {
            CompilerError::Syntax { mut context } => {
                context = context.with_location(location);
                CompilerError::Syntax { context }
            }
            CompilerError::Type { mut context } => {
                context = context.with_location(location);
                CompilerError::Type { context }
            }
            CompilerError::Memory { mut context } => {
                context = context.with_location(location);
                CompilerError::Memory { context }
            }
            CompilerError::Codegen { mut context } => {
                context = context.with_location(location);
                CompilerError::Codegen { context }
            }
            CompilerError::IO { mut context } => {
                context = context.with_location(location);
                CompilerError::IO { context }
            }
            CompilerError::Runtime { mut context } => {
                context = context.with_location(location);
                CompilerError::Runtime { context }
            }
            CompilerError::Validation { mut context } => {
                context = context.with_location(location);
                CompilerError::Validation { context }
            }
        }
    }

    pub fn with_location_option(self, location: Option<SourceLocation>) -> Self {
        match self {
            CompilerError::Syntax { mut context } => {
                context = context.with_location_option(location);
                CompilerError::Syntax { context }
            }
            CompilerError::Type { mut context } => {
                context = context.with_location_option(location);
                CompilerError::Type { context }
            }
            CompilerError::Memory { mut context } => {
                context = context.with_location_option(location);
                CompilerError::Memory { context }
            }
            CompilerError::Codegen { mut context } => {
                context = context.with_location_option(location);
                CompilerError::Codegen { context }
            }
            CompilerError::IO { mut context } => {
                context = context.with_location_option(location);
                CompilerError::IO { context }
            }
            CompilerError::Runtime { mut context } => {
                context = context.with_location_option(location);
                CompilerError::Runtime { context }
            }
            CompilerError::Validation { mut context } => {
                context = context.with_location_option(location);
                CompilerError::Validation { context }
            }
        }
    }

    /// Create a more detailed type error with additional context about the expected types
    pub fn detailed_type_error(
        message: impl AsRef<str>,
        expected_type: impl std::fmt::Debug,
        actual_type: impl std::fmt::Debug,
        location: Option<SourceLocation>,
        help: Option<String>,
    ) -> Self {
        let full_message = format!(
            "{}\nExpected type: {:?}\nActual type: {:?}",
            message.as_ref(),
            expected_type,
            actual_type
        );
        
        let context = ErrorContext {
            message: full_message,
            error_type: ErrorType::Type,
            location,
            help,
        };
        
        CompilerError::Type { context }
    }
    
    /// Create a memory allocation error with additional diagnostics
    pub fn memory_allocation_error(
        message: impl AsRef<str>,
        size_requested: usize,
        available_memory: Option<usize>,
        location: Option<SourceLocation>,
    ) -> Self {
        let mut full_message = format!(
            "{}\nRequested allocation size: {} bytes",
            message.as_ref(),
            size_requested
        );
        
        if let Some(available) = available_memory {
            full_message.push_str(&format!("\nAvailable memory: {} bytes", available));
        }
        
        let help = Some(format!(
            "Consider reducing the size of data structures or optimizing memory usage. \
            Large allocations may require increasing the memory limit."
        ));
        
        let context = ErrorContext {
            message: full_message,
            error_type: ErrorType::Memory,
            location,
            help,
        };
        
        CompilerError::Memory { context }
    }
    
    /// Create a bounds check error for array/matrix access
    pub fn bounds_error(
        message: impl AsRef<str>,
        index: usize,
        max_index: usize,
        location: Option<SourceLocation>,
    ) -> Self {
        let full_message = format!(
            "{}\nIndex: {}\nValid range: 0..{}",
            message.as_ref(),
            index,
            max_index
        );
        
        let help = Some(format!(
            "Ensure the index is within the valid range of 0 to {} (inclusive).",
            max_index - 1
        ));
        
        let context = ErrorContext {
            message: full_message,
            error_type: ErrorType::Runtime,
            location,
            help,
        };
        
        CompilerError::Runtime { context }
    }
    
    /// Create an improved validation error with component details
    pub fn component_validation_error(
        message: impl AsRef<str>,
        component_name: &str,
        component_type: &str,
        location: Option<SourceLocation>,
        help: Option<String>,
    ) -> Self {
        let full_message = format!(
            "{}\nComponent: {} ({})",
            message.as_ref(),
            component_name,
            component_type
        );
        
        let context = ErrorContext {
            message: full_message,
            error_type: ErrorType::Validation,
            location,
            help,
        };
        
        CompilerError::Validation { context }
    }
    
    /// Create a division by zero error
    pub fn division_by_zero_error(location: Option<SourceLocation>) -> Self {
        let context = ErrorContext {
            message: "Division by zero".to_string(),
            error_type: ErrorType::Runtime,
            location,
            help: Some("Check divisor values to ensure they are not zero.".to_string()),
        };
        
        CompilerError::Runtime { context }
    }
    
    /// Create a function not found error with suggestions
    pub fn function_not_found_error<T: Into<String>>(
        name: T,
        available_functions: &[&str],
        location: SourceLocation
    ) -> Self {
        let name_str = name.into();
        let mut message = format!("Function '{}' not found", name_str);
        
        if !available_functions.is_empty() {
            message.push_str(&format!("\nAvailable functions: {}", available_functions.join(", ")));
        }
        
        CompilerError::Type {
            context: ErrorContext {
                message,
                error_type: ErrorType::Type,
                location: Some(location),
                help: Some("Check if the function name is correct and the function is defined".to_string()),
            }
        }
    }

    pub fn variable_not_found_error<T: Into<String>>(
        name: T,
        available_variables: &[&str],
        location: SourceLocation
    ) -> Self {
        let name_str = name.into();
        let mut message = format!("Variable '{}' not found", name_str);
        
        if !available_variables.is_empty() {
            message.push_str(&format!("\nAvailable variables: {}", available_variables.join(", ")));
        }
        
        CompilerError::Type {
            context: ErrorContext {
                message,
                error_type: ErrorType::Type,
                location: Some(location),
                help: Some("Check if the variable name is correct and the variable is defined".to_string()),
            }
        }
    }
}

/// Calculate Levenshtein distance between two strings
/// Used for suggesting similar names for functions/variables
fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let s1_len = s1.chars().count();
    let s2_len = s2.chars().count();
    
    if s1_len == 0 {
        return s2_len;
    }
    if s2_len == 0 {
        return s1_len;
    }
    
    let mut matrix = vec![vec![0; s2_len + 1]; s1_len + 1];
    
    for i in 0..=s1_len {
        matrix[i][0] = i;
    }
    
    for j in 0..=s2_len {
        matrix[0][j] = j;
    }
    
    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();
    
    for i in 1..=s1_len {
        for j in 1..=s2_len {
            let cost = if s1_chars[i - 1] == s2_chars[j - 1] { 0 } else { 1 };
            
            matrix[i][j] = std::cmp::min(
                matrix[i - 1][j] + 1,
                std::cmp::min(
                    matrix[i][j - 1] + 1,
                    matrix[i - 1][j - 1] + cost
                )
            );
        }
    }
    
    matrix[s1_len][s2_len]
}

impl fmt::Display for CompilerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let context = match self {
            CompilerError::Syntax { context } => {
                write!(f, "Syntax error: {}", context.message)?;
                context
            },
            CompilerError::Type { context } => {
                write!(f, "Type error: {}", context.message)?;
                context
            },
            CompilerError::Memory { context } => {
                write!(f, "Memory error: {}", context.message)?;
                context
            },
            CompilerError::Codegen { context } => {
                write!(f, "Code generation error: {}", context.message)?;
                context
            },
            CompilerError::IO { context } => {
                write!(f, "IO error: {}", context.message)?;
                context
            },
            CompilerError::Runtime { context } => {
                write!(f, "Runtime error: {}", context.message)?;
                context
            },
            CompilerError::Validation { context } => {
                write!(f, "Validation error: {}", context.message)?;
                context
            },
        };

        // Add location information if available
        if let Some(location) = &context.location {
            if !location.file.is_empty() && location.file != "<unknown>" {
                write!(f, "\n  at {}:{}:{}", location.file, location.line, location.column)?;
            }
        }

        // Add help information if available
        if let Some(help) = &context.help {
            write!(f, "\n  Help: {}", help)?;
        }

        Ok(())
    }
}

impl Error for CompilerError {}

// Add implementation for From<wasmtime::MemoryAccessError> trait
impl From<wasmtime::MemoryAccessError> for CompilerError {
    fn from(error: wasmtime::MemoryAccessError) -> Self {
        CompilerError::memory_error(
            format!("Memory access error: {}", error),
            Some("Check memory bounds and access patterns".to_string()),
            None,
        )
    }
}

/// Result type alias for compiler operations
pub type CompilerResult<T> = Result<T, CompilerError>;

#[derive(Debug, Clone)]
pub struct CompilerWarning {
    pub message: String,
    pub warning_type: WarningType,
    pub location: Option<SourceLocation>,
    pub help: Option<String>,
    pub suggestion: Option<String>,
}

impl CompilerWarning {
    pub fn new<T: Into<String>>(
        message: T, 
        warning_type: WarningType, 
        location: Option<SourceLocation>
    ) -> Self {
        Self {
            message: message.into(),
            warning_type,
            location,
            help: None,
            suggestion: None,
        }
    }

    pub fn with_help<T: Into<String>>(mut self, help: T) -> Self {
        self.help = Some(help.into());
        self
    }

    pub fn with_suggestion<T: Into<String>>(mut self, suggestion: T) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    pub fn unused_variable(name: &str, location: Option<SourceLocation>) -> Self {
        Self::new(
            format!("Variable '{}' is declared but never used", name),
            WarningType::UnusedVariable,
            location
        ).with_help("Consider removing the variable or using it in your code")
         .with_suggestion(format!("Remove 'let {}' or use the variable", name))
    }

    pub fn unused_function(name: &str, location: Option<SourceLocation>) -> Self {
        Self::new(
            format!("Function '{}' is defined but never called", name),
            WarningType::UnusedFunction,
            location
        ).with_help("Consider removing the function or calling it")
         .with_suggestion(format!("Remove function '{}' or add a call to it", name))
    }

    pub fn dead_code(location: Option<SourceLocation>) -> Self {
        Self::new(
            "This code is unreachable",
            WarningType::DeadCode,
            location
        ).with_help("Code after a return statement or in an impossible branch will never execute")
         .with_suggestion("Remove the unreachable code or fix the control flow")
    }

    pub fn type_inference_warning(inferred_type: &str, location: Option<SourceLocation>) -> Self {
        Self::new(
            format!("Type inferred as '{}' - consider adding explicit type annotation", inferred_type),
            WarningType::TypeInference,
            location
        ).with_help("Explicit type annotations improve code readability and catch type errors early")
         .with_suggestion(format!("Add ': {}' to specify the type explicitly", inferred_type))
    }
}

impl fmt::Display for CompilerWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Warning: {}", self.message)?;
        
        if let Some(location) = &self.location {
            writeln!(f)?;
            write!(f, "  --> {}:{}:{}", location.file, location.line, location.column)?;
        }
        
        if let Some(help) = &self.help {
            writeln!(f)?;
            write!(f, "  = help: {}", help)?;
        }
        
        if let Some(suggestion) = &self.suggestion {
            writeln!(f)?;
            write!(f, "  = suggestion: {}", suggestion)?;
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation_and_display() {
        let error = CompilerError::syntax_error("test error", None, None);
        assert!(error.to_string().contains("test error"));
        
        let error = CompilerError::type_error("type error", None, None);
        assert!(error.to_string().contains("type error"));
        
        let error = CompilerError::memory_error("memory error", None, None);
        assert!(error.to_string().contains("memory error"));
    }

    #[test]
    fn test_error_conversion() {
        let context = ErrorContext::new("test error", None, ErrorType::Syntax, None);
        let error = CompilerError::syntax_error("test error", None, None);
        assert!(error.to_string().contains("test error"));
        
        let string: String = context.into();
        assert!(string.contains("test error"));
    }
} 