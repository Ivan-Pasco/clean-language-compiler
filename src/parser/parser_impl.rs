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
                        Rule::program_item => {
                            for program_item_inner in inner.into_inner() {
                                match program_item_inner.as_rule() {
                                    Rule::functions_block => {
                                        let block_functions = parse_functions_block(program_item_inner)?;
                                        functions.extend(block_functions);
                                    },
                                    Rule::start_function => {
                                        let func = parse_start_function(program_item_inner)?;
                                        start_function = Some(func);
                                    },
                                    Rule::class_decl => {
                                        // Handle class declarations when implemented
                                    },
                                    Rule::statement => {
                                        // Handle top-level statements - these should be added to the start function
                                        // For now, we'll ignore them as the codegen expects a start function
                                    },
                                    _ => {}
                                }
                            }
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

pub fn get_location(pair: &Pair<Rule>) -> super::SourceLocation {
    let span = pair.as_span();
    super::SourceLocation {
        start: span.start(),
        end: span.end(),
    }
}

pub fn parse_functions_block(functions_block: Pair<Rule>) -> Result<Vec<Function>, CompilerError> {
    let mut functions = Vec::new();
    
    for item in functions_block.into_inner() {
        match item.as_rule() {
            Rule::indented_functions_block => {
                for func_item in item.into_inner() {
                    if func_item.as_rule() == Rule::function_in_block {
                        let func = parse_function_in_block(func_item)?;
                        functions.push(func);
                    }
                }
            },
            _ => {}
        }
    }
    
    Ok(functions)
}

/// Helper to parse parameters from an input_block
///
/// Expects a Pair<Rule> for input_block, which contains an indented_input_block.
/// Each input_declaration is parsed as a parameter (type and name).
/// Returns a Vec<Parameter>.
fn parse_parameters_from_input_block(input_block: Pair<Rule>) -> Result<Vec<Parameter>, CompilerError> {
    let mut parameters = Vec::new();
    let mut seen_names = std::collections::HashSet::new();
    for input_inner in input_block.into_inner() {
        if input_inner.as_rule() == Rule::indented_input_block {
            for input_decl in input_inner.into_inner() {
                if input_decl.as_rule() == Rule::input_declaration {
                    let mut param_type = None;
                    let mut param_name = String::new();
                    for param_decl in input_decl.into_inner() {
                        match param_decl.as_rule() {
                            Rule::input_type => param_type = Some(parse_type(param_decl)?),
                            Rule::identifier => param_name = param_decl.as_str().to_string(),
                            _ => {}
                        }
                    }
                    // TODO: Support default values or annotations for parameters in the future
                    if let Some(pt) = param_type {
                        if !seen_names.insert(param_name.clone()) {
                            return Err(CompilerError::parse_error(
                                format!("Duplicate parameter name '{}' in input block", param_name),
                                None,
                                None,
                            ));
                        }
                        parameters.push(Parameter::new(param_name, pt));
                    } else {
                        return Err(CompilerError::parse_error(
                            "Missing type in input parameter declaration",
                            None,
                            None,
                        ));
                    }
                }
            }
        }
    }
    Ok(parameters)
}

/// Parses a function declared inside a functions block.
///
/// According to the grammar, a function_in_block has the following structure:
///   function_in_block = { function_type? ~ identifier ~ "(" ~ ")" ~ function_body }
///   function_body = { (setup_block ~ indented_block) | indented_block }
///
/// - setup_block may contain a description_block and/or input_block (for parameters)
/// - indented_block contains the function body statements
///
/// This function extracts the function name, return type, parameters, description, and body.
/// It returns a Function AST node.
pub fn parse_function_in_block(func_pair: Pair<Rule>) -> Result<Function, CompilerError> {
    let mut func_name = String::new();
    let mut return_type: Option<Type> = None;
    let mut parameters = Vec::new();
    let mut body = Vec::new();
    let mut description: Option<String> = None;
    let location = Some(convert_to_ast_location(&get_location(&func_pair)));

    // Parse the function signature and body
    for item in func_pair.into_inner() {
        match item.as_rule() {
            Rule::function_type => {
                return_type = Some(parse_type(item)?);
            },
            Rule::identifier => {
                func_name = item.as_str().to_string();
            },
            Rule::function_body => {
                // function_body = (setup_block ~ indented_block) | indented_block
                let mut found_body = false;
                let mut found_setup = false;
                for body_item in item.into_inner() {
                    match body_item.as_rule() {
                        Rule::setup_block => {
                            found_setup = true;
                            // setup_block may contain description_block and/or input_block
                            for setup_item in body_item.into_inner() {
                                match setup_item.as_rule() {
                                    Rule::description_block => {
                                        for desc_inner in setup_item.into_inner() {
                                            if desc_inner.as_rule() == Rule::string {
                                                description = Some(desc_inner.as_str().trim_matches('"').to_string());
                                            }
                                        }
                                    },
                                    Rule::input_block => {
                                        let params = parse_parameters_from_input_block(setup_item)?;
                                        parameters.extend(params);
                                    },
                                    _ => {}
                                }
                            }
                        },
                        Rule::function_statements => {
                            found_body = true;
                            // Only add statements from the function_statements to the function body
                            for stmt_pair in body_item.into_inner() {
                                if stmt_pair.as_rule() == Rule::statement {
                                    body.push(parse_statement(stmt_pair)?);
                                }
                            }
                        },
                        _ => {}
                    }
                }
                if !found_body {
                    return Err(CompilerError::parse_error(
                        format!("Function '{}' is missing a body.", func_name),
                        location.clone(),
                        None,
                    ));
                }
            },
            _ => {}
        }
    }

    if func_name.is_empty() {
        return Err(CompilerError::parse_error(
            "Function is missing a name.",
            location.clone(),
            None,
        ));
    }

    let return_type = return_type.unwrap_or(Type::Void);

    Ok(Function {
        name: func_name,
        type_parameters: Vec::new(),
        parameters,
        return_type,
        body,
        description,
        syntax: FunctionSyntax::Block,
        visibility: Visibility::Public,
        location,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pest::Parser;
    use crate::CleanParser;

    #[test]
    fn test_parse_function_in_block_valid() {
        let source = "integer add()\n\tinput\n\t\tinteger a\n\t\tinteger b\n\treturn a + b";
        let mut pairs = CleanParser::parse(Rule::function_in_block, source).unwrap();
        let pair = pairs.next().unwrap();
        let func = parse_function_in_block(pair).unwrap();
        assert_eq!(func.name, "add");
        assert_eq!(func.parameters.len(), 2);
        assert_eq!(func.parameters[0].name, "a");
        assert_eq!(func.parameters[1].name, "b");
        assert_eq!(func.body.len(), 1);
    }

    #[test]
    fn test_parse_function_in_block_duplicate_param() {
        let source = "integer add()\n\tinput\n\t\tinteger a\n\t\tinteger a\n\treturn a + a";
        let mut pairs = CleanParser::parse(Rule::function_in_block, source).unwrap();
        let pair = pairs.next().unwrap();
        let err = parse_function_in_block(pair).unwrap_err();
        assert!(err.to_string().contains("Duplicate parameter name"));
    }

    #[test]
    fn test_parse_function_in_block_missing_name() {
        let source = "integer ()\n\treturn 42";
        let mut pairs = CleanParser::parse(Rule::function_in_block, source).unwrap();
        let pair = pairs.next().unwrap();
        let err = parse_function_in_block(pair).unwrap_err();
        assert!(err.to_string().contains("missing a name"));
    }

    #[test]
    fn debug_parse_tree_for_function_in_block() {
        let source = "integer add()\n\tinput\n\t\tinteger a\n\t\tinteger b\n\t\n\treturn a + b";
        let mut pairs = CleanParser::parse(Rule::function_in_block, source).unwrap();
        let pair = pairs.next().unwrap();
        println!("Parse tree: {:#?}", pair);
    }
} 