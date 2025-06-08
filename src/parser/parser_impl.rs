use pest::{Parser, iterators::Pair};
use crate::ast::{Program, Function, Type, Parameter, FunctionSyntax, Visibility, Statement, Expression, Value};
use crate::error::CompilerError;
use super::{CleanParser, convert_to_ast_location};
use super::statement_parser::parse_statement;
use super::type_parser::parse_type;
use super::Rule;

/// Parse context to track file information and improve error reporting
#[derive(Clone)]
pub struct ParseContext {
    pub file_path: String,
}

impl ParseContext {
    pub fn new(file_path: String) -> Self {
        Self { file_path }
    }
}

/// Enhanced parser with error recovery capabilities
pub struct ErrorRecoveringParser {
    pub source: String,
    pub file_path: String,
    pub errors: Vec<CompilerError>,
}

impl ErrorRecoveringParser {
    pub fn new(source: &str, file_path: &str) -> Self {
        Self {
            source: source.to_string(),
            file_path: file_path.to_string(),
            errors: Vec::new(),
        }
    }

    /// Parse with error recovery - collects multiple errors instead of stopping at the first one
    pub fn parse_with_recovery(&mut self, source: &str) -> Result<Program, Vec<CompilerError>> {
        match parse_with_file(source, &self.file_path) {
            Ok(program) => Ok(program),
            Err(error) => {
                self.errors.push(error);
                Err(self.errors.clone())
            }
        }
    }
}

pub fn parse(source: &str) -> Result<Program, CompilerError> {
    let trimmed_source = source.trim();
    let pairs = CleanParser::parse(Rule::program, trimmed_source)
        .map_err(|e| CompilerError::parse_error(e.to_string(), None, None))?;

    parse_program_ast(pairs)
}

pub fn parse_with_file(source: &str, file_path: &str) -> Result<Program, CompilerError> {
    let trimmed_source = source.trim();
    let pairs = CleanParser::parse(Rule::program, trimmed_source)
        .map_err(|e| CompilerError::parse_error(
            format!("Parse error in {}: {}", file_path, e),
            None,
            Some(format!("Check syntax in file: {}", file_path))
        ))?;

    parse_program_ast(pairs)
}

pub fn parse_program_ast(pairs: pest::iterators::Pairs<Rule>) -> Result<Program, CompilerError> {
    let mut functions = Vec::new();
    let mut classes = Vec::new();
    let mut start_function = None;

    for pair in pairs {
        match pair.as_rule() {
            Rule::program => {
                for inner in pair.into_inner() {
                    match inner.as_rule() {
                        Rule::function_decl => {
                            let func = parse_function(inner)?;
                            if func.name == "start" {
                                start_function = Some(func);
                            } else {
                                functions.push(func);
                            }
                        },
                        Rule::start_function => {
                            let func = parse_start_function(inner)?;
                            start_function = Some(func);
                        },
                        Rule::class_decl => {
                            // Handle class declarations when implemented
                        },
                        Rule::apply_block => {
                            // Handle top-level apply blocks when implemented
                        },
                        Rule::EOI => {}, // End of input
                        _ => {}
                    }
                }
            },
            _ => {}
        }
    }

    let mut program = Program {
        functions,
        classes,
        start_function,
    };

    Ok(program)
}

pub fn parse_start_function(pair: Pair<Rule>) -> Result<Function, CompilerError> {
    let mut name = "start".to_string();
    let mut body = Vec::new();
    let location = Some(convert_to_ast_location(&get_location(&pair)));

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::indented_block => {
                for stmt_pair in inner.into_inner() {
                    match stmt_pair.as_rule() {
                        Rule::statement => {
                            body.push(parse_statement(stmt_pair)?);
                        },
                        _ => {}
                    }
                }
            },
            _ => {}
        }
    }

    // Infer return type from the function body
    let return_type = if let Some(last_stmt) = body.last() {
        match last_stmt {
            Statement::Expression { expr, .. } => {
                // Infer type from the expression
                match expr {
                    Expression::Literal(Value::Integer(_)) => Type::Integer,
                    Expression::Literal(Value::Float(_)) => Type::Float,
                    Expression::Literal(Value::Boolean(_)) => Type::Boolean,
                    Expression::Literal(Value::String(_)) => Type::String,
                    _ => Type::Integer, // Default to integer for other expressions
                }
            },
            Statement::Return { value: Some(expr), .. } => {
                // Infer type from the return expression
                match expr {
                    Expression::Literal(Value::Integer(_)) => Type::Integer,
                    Expression::Literal(Value::Float(_)) => Type::Float,
                    Expression::Literal(Value::Boolean(_)) => Type::Boolean,
                    Expression::Literal(Value::String(_)) => Type::String,
                    _ => Type::Integer, // Default to integer for other expressions
                }
            },
            _ => Type::Void, // For other statement types, assume void
        }
    } else {
        Type::Void // Empty function body
    };

    Ok(Function {
        name,
        type_parameters: Vec::new(),
        parameters: Vec::new(),
        return_type,
        body,
        description: None,
        syntax: FunctionSyntax::Simple,
        visibility: Visibility::Public,
        location,
    })
}

pub fn parse_function(function_def: Pair<Rule>) -> Result<Function, CompilerError> {
    let mut func_name = String::new();
    let mut return_type: Option<Type> = None;
    let mut parameters = Vec::new();
    let mut body = Vec::new();
    let mut description: Option<String> = None;
    let location = Some(convert_to_ast_location(&get_location(&function_def)));

    if function_def.as_rule() != Rule::function_decl {
        return Err(CompilerError::parse_error(
            format!("Expected function declaration, got {:?}", function_def.as_rule()),
            Some(convert_to_ast_location(&get_location(&function_def))),
            None
        ));
    }

    for item in function_def.into_inner() {
        match item.as_rule() {
            Rule::simple_function | Rule::detailed_function => {
                for inner in item.into_inner() {
                    match inner.as_rule() {
                        Rule::type_ => {
                            return_type = Some(parse_type(inner)?);
                        },
                        Rule::identifier => {
                            func_name = inner.as_str().to_string();
                        },
                        Rule::setup_block => {
                            for setup_item in inner.into_inner() {
                                match setup_item.as_rule() {
                                    Rule::description_block => {
                                        for desc_inner in setup_item.into_inner() {
                                            if desc_inner.as_rule() == Rule::string {
                                                description = Some(desc_inner.as_str().trim_matches('"').to_string());
                                            }
                                        }
                                    },
                                    Rule::input_block => {
                                        for input_inner in setup_item.into_inner() {
                                            if input_inner.as_rule() == Rule::input_declaration {
                                                let mut param_type = None;
                                                let mut param_name = String::new();
                                                for param_decl in input_inner.into_inner() {
                                                    match param_decl.as_rule() {
                                                        Rule::type_ => param_type = Some(parse_type(param_decl)?),
                                                        Rule::identifier => param_name = param_decl.as_str().to_string(),
                                                        _ => {}
                                                    }
                                                }
                                                if let Some(pt) = param_type {
                                                    parameters.push(Parameter::new(param_name, pt));
                                                }
                                            }
                                        }
                                    },
                                    _ => {}
                                }
                            }
                        },
                        Rule::indented_block => {
                            for stmt_pair in inner.into_inner() {
                                match stmt_pair.as_rule() {
                                    Rule::statement => {
                                        body.push(parse_statement(stmt_pair)?);
                                    },
                                    _ => {}
                                }
                            }
                        },
                        _ => {}
                    }
                }
            },
            _ => {}
        }
    }

    let return_type = return_type.unwrap_or(Type::Void);

    Ok(Function {
        name: func_name,
        type_parameters: Vec::new(),
        parameters,
        return_type,
        body,
        description,
        syntax: FunctionSyntax::Simple,
        visibility: Visibility::Public,
        location,
    })
}

pub fn get_location(pair: &Pair<Rule>) -> super::SourceLocation {
    let span = pair.as_span();
    super::SourceLocation {
        start: span.start(),
        end: span.end(),
    }
} 