use pest::iterators::Pair;
use crate::ast::{Statement, Expression, Type};
use crate::error::CompilerError;
use super::{get_location, parse_expression, parse_type, convert_to_ast_location, SourceLocation};
use super::Rule;

pub fn parse_statement(pair: Pair<Rule>) -> Result<Statement, CompilerError> {
    let parser_location = get_location(&pair);
    let ast_location = convert_to_ast_location(&parser_location);
    
    let inner = pair.into_inner().next().ok_or_else(|| CompilerError::parse_error(
        "Empty statement".to_string(),
        Some(ast_location.clone()),
        Some("Statement must contain content".to_string())
    ))?;

    match inner.as_rule() {
        Rule::variable_decl => parse_variable_declaration(inner, ast_location),
        Rule::print_stmt => parse_print_statement(inner, ast_location),
        Rule::printl_stmt => parse_printl_statement(inner, ast_location),
        Rule::if_stmt => parse_if_statement(inner, ast_location),
        Rule::iterate_stmt => parse_iterate_statement(inner, ast_location),
        Rule::from_to_stmt => parse_from_to_statement(inner, ast_location),
        Rule::test => parse_test_statement(inner, ast_location),
        Rule::return_stmt => parse_return_statement(inner, ast_location),
        Rule::expression => parse_expression_statement(inner, ast_location),
        _ => Err(CompilerError::parse_error(
            format!("Invalid statement type: {:?}", inner.as_rule()),
            Some(ast_location.clone()),
            Some("Expected a valid statement".to_string())
        )),
    }
}

fn parse_variable_declaration(pair: Pair<Rule>, location: crate::ast::SourceLocation) -> Result<Statement, CompilerError> {
    let mut name = String::new();
    let mut type_ = None;
    let mut initializer = None;

    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::identifier => name = item.as_str().to_string(),
            Rule::type_ => type_ = Some(parse_type(item)?),
            Rule::expression => initializer = Some(parse_expression(item)?),
            _ => {}
        }
    }

    if name.is_empty() {
        return Err(CompilerError::parse_error(
            "Variable declaration is missing variable name".to_string(),
            Some(location.clone()),
            Some("Variable declarations must have a variable name".to_string())
        ));
    }

    Ok(Statement::VariableDecl {
        name,
        type_,
        initializer,
        location: Some(location),
    })
}

fn parse_print_statement(pair: Pair<Rule>, location: crate::ast::SourceLocation) -> Result<Statement, CompilerError> {
    let expression_pair = pair.into_inner().next().ok_or_else(|| CompilerError::parse_error(
        "Print statement is missing expression".to_string(),
        Some(location.clone()),
        Some("Print statements must have an expression to print".to_string())
    ))?;
    
    let expression = parse_expression(expression_pair)?;
    
    Ok(Statement::Print {
        expression,
        newline: false,
        location: Some(location),
    })
}

fn parse_printl_statement(pair: Pair<Rule>, location: crate::ast::SourceLocation) -> Result<Statement, CompilerError> {
    let expression_pair = pair.into_inner().next().ok_or_else(|| CompilerError::parse_error(
        "Print statement is missing expression".to_string(),
        Some(location.clone()),
        Some("Print statements must have an expression to print".to_string())
    ))?;
    
    let expression = parse_expression(expression_pair)?;
    
    Ok(Statement::Print {
        expression,
        newline: true,
        location: Some(location),
    })
}

fn parse_if_statement(pair: Pair<Rule>, location: crate::ast::SourceLocation) -> Result<Statement, CompilerError> {
    let mut condition = None;
    let mut then_branch = Vec::new();
    let mut else_branch = None;

    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::expression => condition = Some(parse_expression(item)?),
            Rule::block => {
                let mut statements = Vec::new();
                for stmt in item.into_inner() {
                    if stmt.as_rule() == Rule::statement {
                        statements.push(parse_statement(stmt)?);
                    }
                }
                if condition.is_some() && then_branch.is_empty() {
                    then_branch = statements;
                } else {
                    else_branch = Some(statements);
                }
            }
            _ => {}
        }
    }

    let condition = condition.ok_or_else(|| CompilerError::parse_error(
        "If statement is missing condition".to_string(),
        Some(location.clone()),
        Some("If statements must have a condition".to_string())
    ))?;

    Ok(Statement::If {
        condition,
        then_branch,
        else_branch,
        location: Some(location),
    })
}

fn parse_iterate_statement(pair: Pair<Rule>, location: crate::ast::SourceLocation) -> Result<Statement, CompilerError> {
    let mut iterator = String::new();
    let mut collection = None;
    let mut body = Vec::new();

    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::identifier => iterator = item.as_str().to_string(),
            Rule::expression => collection = Some(parse_expression(item)?),
            Rule::block => {
                for stmt in item.into_inner() {
                    if stmt.as_rule() == Rule::statement {
                        body.push(parse_statement(stmt)?);
                    }
                }
            }
            _ => {}
        }
    }

    if iterator.is_empty() {
        return Err(CompilerError::parse_error(
            "Iterate statement is missing iterator variable".to_string(),
            Some(location.clone()),
            Some("Iterate statements must declare an iterator variable".to_string())
        ));
    }

    let collection = collection.ok_or_else(|| CompilerError::parse_error(
        "Iterate statement is missing collection expression".to_string(),
        Some(location.clone()),
        Some("Iterate statements must specify a collection to iterate over".to_string())
    ))?;

    Ok(Statement::Iterate {
        iterator,
        collection,
        body,
        location: Some(location),
    })
}

fn parse_from_to_statement(pair: Pair<Rule>, location: crate::ast::SourceLocation) -> Result<Statement, CompilerError> {
    let mut start = None;
    let mut end = None;
    let mut step = None;
    let mut body = Vec::new();

    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::expression => {
                if start.is_none() {
                    start = Some(parse_expression(item)?);
                } else if end.is_none() {
                    end = Some(parse_expression(item)?);
                } else {
                    step = Some(parse_expression(item)?);
                }
            }
            Rule::block => {
                for stmt in item.into_inner() {
                    if stmt.as_rule() == Rule::statement {
                        body.push(parse_statement(stmt)?);
                    }
                }
            }
            _ => {}
        }
    }

    let start_expr = start.ok_or_else(|| CompilerError::parse_error(
        "From-to statement is missing 'from' expression".to_string(),
        Some(location.clone()),
        Some("From-to statements must specify a 'from' value".to_string())
    ))?;

    let end_expr = end.ok_or_else(|| CompilerError::parse_error(
        "From-to statement is missing 'to' expression".to_string(),
        Some(location.clone()),
        Some("From-to statements must specify a 'to' value".to_string())
    ))?;

    Ok(Statement::FromTo {
        start: start_expr,
        end: end_expr,
        step,
        body,
        location: Some(location),
    })
}

fn parse_test_statement(pair: Pair<Rule>, location: crate::ast::SourceLocation) -> Result<Statement, CompilerError> {
    let mut name = String::new();
    let mut description = None;
    let mut body = Vec::new();

    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::string => {
                let str_content = item.as_str().to_string();
                // Remove quotes from the string
                let unquoted = if str_content.starts_with('"') && str_content.ends_with('"') {
                    str_content[1..str_content.len()-1].to_string()
                } else {
                    str_content
                };
                
                if name.is_empty() {
                    name = unquoted.clone();
                    description = Some(unquoted);
                } else {
                    description = Some(unquoted);
                }
            },
            Rule::block => {
                for stmt in item.into_inner() {
                    if stmt.as_rule() == Rule::statement {
                        body.push(parse_statement(stmt)?);
                    }
                }
            }
            _ => {}
        }
    }

    if name.is_empty() {
        return Err(CompilerError::parse_error(
            "Test statement is missing name".to_string(),
            Some(location.clone()),
            Some("Test statements must have a name".to_string())
        ));
    }

    Ok(Statement::Test {
        name,
        description,
        body,
        location: Some(location),
    })
}

fn parse_return_statement(pair: Pair<Rule>, location: crate::ast::SourceLocation) -> Result<Statement, CompilerError> {
    let mut value = None;
    
    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::expression => {
                value = Some(parse_expression(item)?);
            },
            _ => {}
        }
    }
    
    Ok(Statement::Return {
        value,
        location: Some(location),
    })
}

fn parse_expression_statement(pair: Pair<Rule>, location: crate::ast::SourceLocation) -> Result<Statement, CompilerError> {
    let expression = parse_expression(pair)?;
    Ok(Statement::Expression { 
        expr: expression, 
        location: Some(location) 
    })
} 