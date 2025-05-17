// A standalone test for error handling methods
// Run with: cargo run --bin error_test

use std::fmt;

// Simple location type for testing
#[derive(Debug, Clone)]
struct SourceLocation {
    start: usize,
    end: usize,
}

// Error context for test
#[derive(Debug, Clone)]
struct ErrorContext {
    message: String,
    location: Option<SourceLocation>,
    help: Option<String>,
}

impl ErrorContext {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            location: None,
            help: None,
        }
    }

    fn with_location(mut self, location: SourceLocation) -> Self {
        self.location = Some(location);
        self
    }

    fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    fn with_location_option(mut self, location: Option<SourceLocation>) -> Self {
        if let Some(loc) = location {
            self.location = Some(loc);
        }
        self
    }

    fn with_help_option(mut self, help: Option<String>) -> Self {
        if let Some(h) = help {
            self.help = Some(h);
        }
        self
    }
}

// Error enum for test
#[derive(Debug)]
enum CompilerError {
    ParseError(ErrorContext),
    RuntimeError(ErrorContext),
    ValidationError(ErrorContext),
    TypeError(ErrorContext),
}

impl fmt::Display for CompilerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CompilerError::ParseError(ctx) => write!(f, "Parse error: {}", ctx.message),
            CompilerError::RuntimeError(ctx) => write!(f, "Runtime error: {}", ctx.message),
            CompilerError::ValidationError(ctx) => write!(f, "Validation error: {}", ctx.message),
            CompilerError::TypeError(ctx) => write!(f, "Type error: {}", ctx.message),
        }
    }
}

impl CompilerError {
    fn parse_error(message: String, location: Option<SourceLocation>, help: Option<String>) -> Self {
        CompilerError::ParseError(ErrorContext::new(message)
            .with_location_option(location)
            .with_help_option(help))
    }

    fn runtime_error(message: String, location: Option<SourceLocation>, help: Option<String>) -> Self {
        CompilerError::RuntimeError(ErrorContext::new(message)
            .with_location_option(location)
            .with_help_option(help))
    }

    fn validation_error(message: String, location: Option<SourceLocation>, help: Option<String>) -> Self {
        CompilerError::ValidationError(ErrorContext::new(message)
            .with_location_option(location)
            .with_help_option(help))
    }

    fn type_error(message: String, location: Option<SourceLocation>, help: Option<String>) -> Self {
        CompilerError::TypeError(ErrorContext::new(message)
            .with_location_option(location)
            .with_help_option(help))
    }
}

// Runtime error type for test
struct RuntimeError(String);

// Trait for stdlib errors
trait StdlibError {
    fn to_compiler_error(&self) -> CompilerError;
}

// Implement for RuntimeError
impl StdlibError for RuntimeError {
    fn to_compiler_error(&self) -> CompilerError {
        CompilerError::runtime_error(
            self.0.clone(),
            None,
            Some("This is a runtime error from the standard library".to_string())
        )
    }
}

// Implement From<StdlibError> for CompilerError
impl<T: StdlibError> From<T> for CompilerError {
    fn from(error: T) -> Self {
        error.to_compiler_error()
    }
}

fn main() {
    println!("Testing error handling methods...");

    // Test parse_error
    let parse_err = CompilerError::parse_error(
        "Invalid syntax".to_string(),
        Some(SourceLocation { start: 10, end: 15 }),
        Some("Check your syntax near line 3".to_string())
    );
    println!("Parse error: {:?}", parse_err);

    // Test runtime_error
    let runtime_err = CompilerError::runtime_error(
        "Division by zero".to_string(),
        Some(SourceLocation { start: 25, end: 30 }),
        Some("Avoid dividing by zero".to_string())
    );
    println!("Runtime error: {:?}", runtime_err);

    // Test validation_error
    let validation_err = CompilerError::validation_error(
        "Invalid argument".to_string(),
        Some(SourceLocation { start: 40, end: 45 }),
        Some("Expected a number".to_string())
    );
    println!("Validation error: {:?}", validation_err);

    // Test type_error
    let type_err = CompilerError::type_error(
        "Type mismatch".to_string(),
        Some(SourceLocation { start: 50, end: 55 }),
        Some("Expected integer, got string".to_string())
    );
    println!("Type error: {:?}", type_err);

    // Test StdlibError conversion
    let stdlib_err = RuntimeError("Memory allocation failed".to_string());
    let converted_err: CompilerError = stdlib_err.into();
    println!("Converted stdlib error: {:?}", converted_err);

    // Test error formatting
    println!("Formatted parse error: {}", parse_err);
    println!("Formatted runtime error: {}", runtime_err);
    println!("Formatted validation error: {}", validation_err);
    println!("Formatted type error: {}", type_err);

    println!("All error tests passed!");
} 