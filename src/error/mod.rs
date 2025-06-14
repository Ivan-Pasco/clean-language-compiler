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
    pub suggestions: Vec<String>,
    pub source_snippet: Option<String>,
    pub stack_trace: Vec<StackFrame>,
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
            suggestions: Vec::new(),
            source_snippet: None,
            stack_trace: Vec::new(),
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

    pub fn with_suggestion<T: Into<String>>(mut self, suggestion: T) -> Self {
        self.suggestions.push(suggestion.into());
        self
    }

    pub fn with_suggestions(mut self, suggestions: Vec<String>) -> Self {
        self.suggestions.extend(suggestions);
        self
    }

    pub fn with_source_snippet<T: Into<String>>(mut self, snippet: T) -> Self {
        self.source_snippet = Some(snippet.into());
        self
    }

    pub fn add_stack_frame(&mut self, frame: StackFrame) {
        self.stack_trace.push(frame);
    }

    pub fn with_stack_frame(mut self, frame: StackFrame) -> Self {
        self.stack_trace.push(frame);
        self
    }

    /// Create an enhanced syntax error with context and suggestions
    pub fn enhanced_syntax_error<T: Into<String>>(
        message: T,
        location: Option<SourceLocation>,
        source_snippet: Option<String>,
        suggestions: Vec<String>,
    ) -> Self {
        Self {
            message: message.into(),
            help: None,
            error_type: ErrorType::Syntax,
            location,
            suggestions,
            source_snippet,
            stack_trace: Vec::new(),
        }
    }

    /// Create an enhanced type error with detailed type information
    pub fn enhanced_type_error<T: Into<String>>(
        message: T,
        expected_type: Option<String>,
        actual_type: Option<String>,
        location: Option<SourceLocation>,
    ) -> Self {
        let mut help_text = String::new();
        if let (Some(expected), Some(actual)) = (expected_type, actual_type) {
            help_text = format!("Expected type '{}', but found '{}'", expected, actual);
        }

        Self {
            message: message.into(),
            help: if help_text.is_empty() { None } else { Some(help_text) },
            error_type: ErrorType::Type,
            location,
            suggestions: Vec::new(),
            source_snippet: None,
            stack_trace: Vec::new(),
        }
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

        // Show source snippet if available
        if let Some(snippet) = &self.source_snippet {
            writeln!(f, "\nSource:")?;
            writeln!(f, "{}", snippet)?;
        }

        // Show suggestions if available
        if !self.suggestions.is_empty() {
            writeln!(f, "\nSuggestions:")?;
            for suggestion in &self.suggestions {
                writeln!(f, "  - {}", suggestion)?;
            }
        }

        // Show stack trace if available
        if !self.stack_trace.is_empty() {
            writeln!(f, "\nStack trace:")?;
            for frame in &self.stack_trace {
                if let Some(location) = &frame.location {
                    writeln!(f, "  at {} ({}:{}:{})", 
                        frame.function_name, location.file, location.line, location.column)?;
                } else {
                    writeln!(f, "  at {}", frame.function_name)?;
                }
            }
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
            suggestions: Vec::new(),
            source_snippet: None,
            stack_trace: Vec::new(),
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
    
    pub fn parse_error<T: Into<String>>(message: T, location: Option<SourceLocation>, help: Option<String>) -> Self {
        CompilerError::Syntax {
            context: ErrorContext::new(message, help, ErrorType::Syntax, location),
        }
    }

    /// Enhanced syntax error with context, suggestions, and source snippet
    pub fn enhanced_syntax_error<T: Into<String>>(
        message: T,
        location: Option<SourceLocation>,
        source_snippet: Option<String>,
        suggestions: Vec<String>,
        help: Option<String>,
    ) -> Self {
        let context = ErrorContext::enhanced_syntax_error(message, location, source_snippet, suggestions)
            .with_help_option(help);
        CompilerError::Syntax { context }
    }

    /// Enhanced type error with detailed type information
    pub fn enhanced_type_error<T: Into<String>>(
        message: T,
        expected_type: Option<String>,
        actual_type: Option<String>,
        location: Option<SourceLocation>,
        suggestions: Vec<String>,
    ) -> Self {
        let context = ErrorContext::enhanced_type_error(message, expected_type, actual_type, location)
            .with_suggestions(suggestions);
        CompilerError::Type { context }
    }

    /// Create a parse error with suggestions for common mistakes
    pub fn parse_error_with_suggestions<T: Into<String>>(
        message: T,
        location: Option<SourceLocation>,
        suggestions: Vec<String>,
        source_snippet: Option<String>,
    ) -> Self {
        Self::enhanced_syntax_error(message, location, source_snippet, suggestions, None)
    }

    /// Create an error for unexpected tokens with suggestions
    pub fn unexpected_token_error(
        found: &str,
        expected: Vec<&str>,
        location: Option<SourceLocation>,
        source_snippet: Option<String>,
    ) -> Self {
        let message = if expected.len() == 1 {
            format!("Expected {}, but found '{}'", expected[0], found)
        } else {
            format!("Expected one of [{}], but found '{}'", expected.join(", "), found)
        };

        let suggestions = if expected.len() <= 3 {
            expected.iter().map(|s| format!("Try using '{}'", s)).collect()
        } else {
            vec!["Check the syntax documentation for valid tokens".to_string()]
        };

        Self::enhanced_syntax_error(
            message,
            location,
            source_snippet,
            suggestions,
            Some("Verify the token matches the expected syntax".to_string()),
        )
    }

    /// Create an error for missing required elements
    pub fn missing_element_error<T: Into<String>>(
        element_type: T,
        location: Option<SourceLocation>,
        suggestions: Vec<String>,
    ) -> Self {
        let element = element_type.into();
        let message = format!("Missing required {}", element);
        let help = Some(format!("Add the missing {} to complete the syntax", element));

        Self::enhanced_syntax_error(message, location, None, suggestions, help)
    }

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
            suggestions: Vec::new(),
            source_snippet: None,
            stack_trace: Vec::new(),
        };
        
        CompilerError::Type { context }
    }
    
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
            suggestions: Vec::new(),
            source_snippet: None,
            stack_trace: Vec::new(),
        };
        
        CompilerError::Memory { context }
    }
    
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
            suggestions: Vec::new(),
            source_snippet: None,
            stack_trace: Vec::new(),
        };
        
        CompilerError::Runtime { context }
    }
    
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
            suggestions: Vec::new(),
            source_snippet: None,
            stack_trace: Vec::new(),
        };
        
        CompilerError::Validation { context }
    }
    
    pub fn division_by_zero_error(location: Option<SourceLocation>) -> Self {
        let context = ErrorContext {
            message: "Division by zero".to_string(),
            error_type: ErrorType::Runtime,
            location,
            help: Some("Check divisor values to ensure they are not zero.".to_string()),
            suggestions: Vec::new(),
            source_snippet: None,
            stack_trace: Vec::new(),
        };
        
        CompilerError::Runtime { context }
    }
    
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
                suggestions: Vec::new(),
                source_snippet: None,
                stack_trace: Vec::new(),
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
                suggestions: Vec::new(),
                source_snippet: None,
                stack_trace: Vec::new(),
            }
        }
    }
}

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

        if let Some(location) = &context.location {
            if !location.file.is_empty() && location.file != "<unknown>" {
                write!(f, "\n  at {}:{}:{}", location.file, location.line, location.column)?;
            }
        }

        if let Some(help) = &context.help {
            write!(f, "\n  Help: {}", help)?;
        }

        Ok(())
    }
}

impl Error for CompilerError {}

impl From<wasmtime::MemoryAccessError> for CompilerError {
    fn from(error: wasmtime::MemoryAccessError) -> Self {
        CompilerError::memory_error(
            format!("Memory access error: {}", error),
            Some("Check memory bounds and access patterns".to_string()),
            None,
        )
    }
}

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

/// Utility functions for enhanced error reporting
pub struct ErrorUtils;

impl ErrorUtils {
    /// Extract a source snippet around the error location
    pub fn extract_source_snippet(
        source: &str,
        location: &SourceLocation,
        context_lines: usize,
    ) -> String {
        let lines: Vec<&str> = source.lines().collect();
        let error_line = location.line.saturating_sub(1); // Convert to 0-based indexing
        
        let start_line = error_line.saturating_sub(context_lines);
        let end_line = std::cmp::min(error_line + context_lines + 1, lines.len());
        
        let mut snippet = String::new();
        
        for (i, line_content) in lines[start_line..end_line].iter().enumerate() {
            let line_num = start_line + i + 1; // Convert back to 1-based
            let is_error_line = line_num == location.line;
            
            if is_error_line {
                snippet.push_str(&format!(" --> {}: {}\n", line_num, line_content));
                
                // Add pointer to the specific column
                let pointer_line = format!("     {}{}", 
                    " ".repeat(location.column.saturating_sub(1)), 
                    "^".repeat(std::cmp::max(1, 1))
                );
                snippet.push_str(&format!("{}\n", pointer_line));
            } else {
                snippet.push_str(&format!("     {}: {}\n", line_num, line_content));
            }
        }
        
        snippet
    }

    /// Generate suggestions for similar identifiers using Levenshtein distance
    pub fn suggest_similar_names(target: &str, available: &[&str], max_suggestions: usize) -> Vec<String> {
        let mut suggestions: Vec<(usize, &str)> = available
            .iter()
            .map(|name| (levenshtein_distance(target, name), *name))
            .filter(|(distance, _)| *distance <= 3) // Only suggest if distance is reasonable
            .collect();
        
        suggestions.sort_by_key(|(distance, _)| *distance);
        suggestions
            .into_iter()
            .take(max_suggestions)
            .map(|(_, name)| format!("Did you mean '{}'?", name))
            .collect()
    }

    /// Create suggestions for common syntax mistakes
    pub fn suggest_syntax_fixes(error_context: &str) -> Vec<String> {
        let mut suggestions = Vec::new();
        
        if error_context.contains("expected identifier") {
            suggestions.push("Check if you're using a reserved keyword as an identifier".to_string());
            suggestions.push("Ensure the identifier starts with a letter or underscore".to_string());
        }
        
        if error_context.contains("expected \")\"") {
            suggestions.push("Check for missing closing parenthesis".to_string());
            suggestions.push("Verify parentheses are properly balanced".to_string());
        }
        
        if error_context.contains("expected \"}\"") {
            suggestions.push("Check for missing closing brace".to_string());
            suggestions.push("Verify braces are properly balanced".to_string());
        }
        
        if error_context.contains("expected \"]\"") {
            suggestions.push("Check for missing closing bracket".to_string());
            suggestions.push("Verify brackets are properly balanced".to_string());
        }
        
        if error_context.contains("unexpected token") {
            suggestions.push("Check the syntax documentation for valid tokens".to_string());
            suggestions.push("Verify you're using the correct Clean Language syntax".to_string());
        }
        
        suggestions
    }

    /// Convert a Pest parsing error to an enhanced CompilerError with detailed context
    pub fn from_pest_error(
        pest_error: pest::error::Error<crate::parser::Rule>,
        source: &str,
        file_path: &str,
    ) -> CompilerError {
        let (location, error_span) = match pest_error.location {
            pest::error::InputLocation::Pos(pos) => {
                let (line, col) = Self::calculate_line_column(source, pos);
                let location = SourceLocation {
                    line,
                    column: col,
                    file: file_path.to_string(),
                };
                (Some(location), (pos, pos))
            },
            pest::error::InputLocation::Span((start_pos, end_pos)) => {
                let (line, col) = Self::calculate_line_column(source, start_pos);
                let location = SourceLocation {
                    line,
                    column: col,
                    file: file_path.to_string(),
                };
                (Some(location), (start_pos, end_pos))
            },
        };

        let source_snippet = if let Some(loc) = &location {
            Some(Self::extract_enhanced_source_snippet(source, error_span, loc))
        } else {
            None
        };

        let (enhanced_message, suggestions, help) = Self::enhance_pest_error_message(&pest_error, source, error_span);

        CompilerError::enhanced_syntax_error(
            enhanced_message,
            location,
            source_snippet,
            suggestions,
            help,
        )
    }

    /// Calculate accurate line and column numbers from position
    fn calculate_line_column(source: &str, pos: usize) -> (usize, usize) {
        let mut line = 1;
        let mut col = 1;
        
        for (i, ch) in source.char_indices() {
            if i >= pos {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        
        (line, col)
    }

    /// Extract enhanced source snippet with error highlighting
    fn extract_enhanced_source_snippet(source: &str, error_span: (usize, usize), location: &SourceLocation) -> String {
        let lines: Vec<&str> = source.lines().collect();
        let error_line_idx = location.line.saturating_sub(1);
        
        if error_line_idx >= lines.len() {
            return "Source line not available".to_string();
        }

        let start_line = error_line_idx.saturating_sub(2);
        let end_line = std::cmp::min(error_line_idx + 3, lines.len());
        
        let mut snippet = String::new();
        
        for (i, line_idx) in (start_line..end_line).enumerate() {
            let line_num = line_idx + 1;
            let line_content = lines[line_idx];
            
            if line_idx == error_line_idx {
                // This is the error line - add highlighting
                snippet.push_str(&format!("{:4} | {}\n", line_num, line_content));
                
                // Add error pointer
                let pointer_offset = location.column.saturating_sub(1);
                let pointer_line = format!("     | {}^", " ".repeat(pointer_offset));
                
                // If we have a span, show the full error range
                if error_span.1 > error_span.0 {
                    let span_length = std::cmp::min(error_span.1 - error_span.0, line_content.len() - pointer_offset);
                    if span_length > 1 {
                        snippet.push_str(&format!("{}{}",
                            pointer_line,
                            "~".repeat(span_length.saturating_sub(1))
                        ));
                    } else {
                        snippet.push_str(&pointer_line);
                    }
                } else {
                    snippet.push_str(&pointer_line);
                }
                snippet.push('\n');
            } else {
                // Context line
                snippet.push_str(&format!("{:4} | {}\n", line_num, line_content));
            }
        }
        
        snippet
    }

    /// Enhance Pest error messages with context-specific information
    fn enhance_pest_error_message(
        pest_error: &pest::error::Error<crate::parser::Rule>,
        source: &str,
        error_span: (usize, usize),
    ) -> (String, Vec<String>, Option<String>) {
        use pest::error::ErrorVariant;
        
        match &pest_error.variant {
            ErrorVariant::ParsingError { positives, negatives } => {
                let mut message = String::new();
                let mut suggestions = Vec::new();
                let mut help = None;

                if !positives.is_empty() {
                    let expected: Vec<String> = positives.iter()
                        .map(|rule| Self::rule_to_friendly_name(rule))
                        .collect();
                    
                    message = if expected.len() == 1 {
                        format!("Expected {}", expected[0])
                    } else {
                        format!("Expected one of: {}", expected.join(", "))
                    };

                    // Add context-specific suggestions
                    suggestions.extend(Self::get_context_specific_suggestions(&expected, source, error_span));
                }

                if !negatives.is_empty() {
                    let unexpected: Vec<String> = negatives.iter()
                        .map(|rule| Self::rule_to_friendly_name(rule))
                        .collect();
                    
                    if !message.is_empty() {
                        message.push_str(&format!(", but found {}", unexpected.join(" or ")));
                    } else {
                        message = format!("Unexpected {}", unexpected.join(" or "));
                    }
                }

                // Add helpful context
                help = Some(Self::get_contextual_help(&message, source, error_span));

                (message, suggestions, help)
            },
            ErrorVariant::CustomError { message } => {
                let suggestions = Self::suggest_syntax_fixes(message);
                let help = Some("Check the Clean Language syntax documentation".to_string());
                (message.clone(), suggestions, help)
            },
        }
    }

    /// Convert Pest rule names to user-friendly descriptions
    fn rule_to_friendly_name(rule: &crate::parser::Rule) -> String {
        use crate::parser::Rule;
        
        match rule {
            Rule::identifier => "identifier".to_string(),
            Rule::number => "number".to_string(),
            Rule::string => "string".to_string(),
            Rule::boolean => "boolean value".to_string(),
            Rule::function_call => "function call".to_string(),
            Rule::method_call => "method call".to_string(),
            Rule::variable_decl => "variable declaration".to_string(),
            Rule::assignment => "assignment".to_string(),
            Rule::if_stmt => "if statement".to_string(),
            Rule::return_stmt => "return statement".to_string(),
            Rule::expression => "expression".to_string(),
            Rule::statement => "statement".to_string(),
            Rule::type_ => "type annotation".to_string(),
            Rule::parameter => "parameter".to_string(),
            Rule::function_body => "function body".to_string(),
            Rule::indented_block => "indented block".to_string(),
            Rule::NEWLINE => "newline".to_string(),
            Rule::INDENT => "indentation".to_string(),
            Rule::EOI => "end of input".to_string(),
            _ => format!("{:?}", rule).to_lowercase(),
        }
    }

    /// Get context-specific suggestions based on expected rules and surrounding code
    fn get_context_specific_suggestions(expected: &[String], source: &str, error_span: (usize, usize)) -> Vec<String> {
        let mut suggestions = Vec::new();
        
        // Get the context around the error
        let context = if error_span.0 > 0 && error_span.0 <= source.len() {
            &source[error_span.0.saturating_sub(50)..std::cmp::min(error_span.1 + 50, source.len())]
        } else {
            ""
        };

        for exp in expected {
            match exp.as_str() {
                "identifier" => {
                    suggestions.push("Use a valid identifier (letters, numbers, underscore, starting with letter)".to_string());
                    if context.contains("function") {
                        suggestions.push("Function names must be valid identifiers".to_string());
                    }
                },
                "type annotation" => {
                    suggestions.push("Add a type annotation (e.g., 'integer', 'string', 'boolean')".to_string());
                    suggestions.push("Check if you're missing a type before the variable name".to_string());
                },
                "expression" => {
                    suggestions.push("Add a valid expression (variable, literal, or function call)".to_string());
                    if context.contains("=") {
                        suggestions.push("Assignment requires a value after the '=' sign".to_string());
                    }
                },
                "indented block" => {
                    suggestions.push("Add proper indentation using tabs".to_string());
                    suggestions.push("Ensure the block is indented more than the parent statement".to_string());
                },
                "newline" => {
                    suggestions.push("Add a newline to separate statements".to_string());
                },
                "end of input" => {
                    suggestions.push("The file may be incomplete or have unclosed constructs".to_string());
                },
                _ => {}
            }
        }

        // Add general suggestions if no specific ones were found
        if suggestions.is_empty() {
            suggestions.push("Check the Clean Language syntax documentation".to_string());
            suggestions.push("Verify proper indentation and statement structure".to_string());
        }

        suggestions
    }

    /// Get contextual help based on the error message and surrounding code
    fn get_contextual_help(message: &str, source: &str, error_span: (usize, usize)) -> String {
        let context = if error_span.0 > 0 && error_span.0 <= source.len() {
            &source[error_span.0.saturating_sub(100)..std::cmp::min(error_span.1 + 100, source.len())]
        } else {
            ""
        };

        if message.contains("identifier") && context.contains("function") {
            "Function declarations require a valid function name after the 'function' keyword".to_string()
        } else if message.contains("type annotation") {
            "Clean Language uses type-first syntax: 'type variable_name = value'".to_string()
        } else if message.contains("indented block") {
            "Clean Language uses indentation to define code blocks. Use tabs for indentation".to_string()
        } else if message.contains("expression") && context.contains("=") {
            "Assignments require a value after the '=' operator".to_string()
        } else if message.contains("newline") {
            "Statements in Clean Language must be separated by newlines".to_string()
        } else {
            "Refer to the Clean Language syntax guide for proper formatting".to_string()
        }
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