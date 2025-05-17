use pest::iterators::Pair;
use crate::ast::{Program, Function, Parameter, Type, Statement};
use crate::error::CompilerError;
use super::{CleanParser, convert_to_ast_location, SourceLocation};
use super::Rule;
use super::statement_parser::parse_statement;
use super::expression_parser::parse_expression;
use super::type_parser::parse_type;
use super::class_parser::parse_class;
use pest::Parser;

pub fn parse(source: &str) -> Result<Program, CompilerError> {
    let pairs = CleanParser::parse(Rule::program, source)
        .map_err(|e| CompilerError::parse_error(
            format!("Parse error: {}", e),
            None,
            None
        ))?;

    let mut program = Program {
        functions: Vec::new(),
        classes: Vec::new(),
    };

    for pair in pairs {
        if pair.as_rule() == Rule::program {
            for item in pair.into_inner() {
                match item.as_rule() {
                    Rule::function_decl => program.functions.push(parse_function(item)?),
                    Rule::class_decl => program.classes.push(parse_class(item)?),
                    Rule::start_function => program.functions.push(parse_start_function(item)?),
                    Rule::EOI => {}, // End of input, ignore
                    _ => {} // Ignore other rules for now
                }
            }
        }
    }

    Ok(program)
}

pub fn parse_function(pair: Pair<Rule>) -> Result<Function, CompilerError> {
    let mut name = String::new();
    let mut parameters = Vec::new();
    let mut return_type = None;
    let mut body = Vec::new();
    let mut description = None;
    let mut type_parameters = Vec::new();
    let location = get_location(&pair);
    let ast_location = convert_to_ast_location(&location);

    // Find the function_def within function_decl
    let function_def = pair.into_inner().next().ok_or_else(|| CompilerError::parse_error(
        "Empty function declaration".to_string(),
        Some(ast_location.clone()),
        Some("Function declarations must have a definition".to_string())
    ))?;

    if function_def.as_rule() != Rule::function_def {
        return Err(CompilerError::parse_error(
            format!("Expected function_def, found {:?}", function_def.as_rule()),
            Some(ast_location.clone()),
            Some("Check function declaration syntax".to_string())
        ));
    }

    for item in function_def.into_inner() {
        match item.as_rule() {
            Rule::identifier => name = item.as_str().to_string(),
            Rule::type_parameters => {
                for param in item.into_inner() {
                    if param.as_rule() == Rule::type_parameter {
                        type_parameters.push(param.as_str().to_string());
                    }
                }
            },
            Rule::setup_block => {
                for setup_item in item.into_inner() {
                    match setup_item.as_rule() {
                        Rule::description_block => {
                            let desc_str = setup_item.into_inner().next().unwrap();
                            if desc_str.as_rule() == Rule::string {
                                description = Some(desc_str.as_str().to_string());
                            }
                        },
                        Rule::input_block => {
                            for param_decl in setup_item.into_inner() {
                                if param_decl.as_rule() == Rule::type_declaration {
                                    parameters.push(parse_parameter(param_decl)?);
                                }
                            }
                        },
                        _ => {}
                    }
                }
            },
            Rule::type_ => {
                return_type = Some(parse_type(item)?);
            },
            Rule::block => {
                for stmt in item.into_inner() {
                    if stmt.as_rule() == Rule::statement {
                        body.push(parse_statement(stmt)?);
                    }
                }
            },
            _ => {}
        }
    }

    if name.is_empty() {
        return Err(CompilerError::parse_error(
            "Function is missing name".to_string(),
            Some(ast_location),
            Some("Functions must have a name".to_string())
        ));
    }

    // Use a default return type of Type::Unit if none is specified
    let return_type = return_type.unwrap_or(Type::Unit);

    Ok(Function {
        name,
        parameters,
        return_type,
        body,
        location: Some(ast_location),
        description,
        type_parameters,
    })
}

pub fn parse_parameter(pair: Pair<Rule>) -> Result<Parameter, CompilerError> {
    let location = get_location(&pair);
    let ast_location = convert_to_ast_location(&location);
    
    // The pair should be a type_declaration
    let mut parts = pair.into_inner();
    
    let type_part = parts.next().ok_or_else(|| CompilerError::parse_error(
        "Parameter missing type".to_string(),
        Some(ast_location.clone()),
        Some("Parameters must have a type".to_string())
    ))?;
    
    let name_part = parts.next().ok_or_else(|| CompilerError::parse_error(
        "Parameter missing name".to_string(),
        Some(ast_location.clone()),
        Some("Parameters must have a name".to_string())
    ))?;
    
    if name_part.as_rule() != Rule::identifier {
        return Err(CompilerError::parse_error(
            "Expected identifier for parameter name".to_string(),
            Some(ast_location),
            Some("Parameters must have valid identifiers".to_string())
        ));
    }
    
    let name = name_part.as_str().to_string();
    let type_ = parse_type(type_part)?;
    
    Ok(Parameter {
        name,
        type_,
    })
}

pub fn get_location(pair: &Pair<Rule>) -> SourceLocation {
    let span = pair.as_span();
    SourceLocation {
        start: span.start(),
        end: span.end(),
    }
}

pub fn parse_start_function(pair: Pair<Rule>) -> Result<Function, CompilerError> {
    let mut body = Vec::new();
    let location = get_location(&pair);
    let ast_location = convert_to_ast_location(&location);

    for item in pair.into_inner() {
        if item.as_rule() == Rule::block {
            for stmt in item.into_inner() {
                if stmt.as_rule() == Rule::statement {
                    body.push(parse_statement(stmt)?);
                }
            }
        }
    }

    Ok(Function {
        name: "start".to_string(),
        parameters: Vec::new(),
        return_type: Type::Unit,
        body,
        location: Some(ast_location),
        description: None,
        type_parameters: Vec::new(),
    })
} 