use pest::iterators::Pair;
use crate::ast::Statement;
use crate::error::CompilerError;
use super::{get_location, convert_to_ast_location};
use super::expression_parser::parse_expression;
use super::type_parser::parse_type;
use super::Rule;

pub fn parse_statement(pair: Pair<Rule>) -> Result<Statement, CompilerError> {
    let ast_location = convert_to_ast_location(&get_location(&pair));
    let inner = pair.into_inner().next().unwrap();

    match inner.as_rule() {
        Rule::variable_decl => parse_variable_declaration(inner, ast_location),
        Rule::assignment => parse_assignment_statement(inner, ast_location),
        Rule::print_stmt => parse_print_statement(inner, ast_location),
        Rule::println_stmt => parse_println_statement(inner, ast_location),
        Rule::if_stmt => parse_if_statement(inner, ast_location),
        Rule::iterate_stmt => parse_iterate_statement(inner, ast_location),
        Rule::range_iterate_stmt => parse_range_iterate_statement(inner, ast_location),
        Rule::test => parse_test_statement(inner, ast_location),
        Rule::return_stmt => parse_return_statement(inner, ast_location),
        Rule::apply_block => parse_apply_block_statement(inner, ast_location),
        Rule::expression => {
            let expr = parse_expression(inner)?;
            Ok(Statement::Expression {
                expr,
                location: Some(ast_location),
            })
        },
        _ => Err(CompilerError::parse_error(
            format!("Unexpected statement: {:?}", inner.as_rule()),
            Some(ast_location),
            Some("Expected a valid statement".to_string())
        )),
    }
}

fn parse_variable_declaration(pair: Pair<Rule>, ast_location: crate::ast::SourceLocation) -> Result<Statement, CompilerError> {
    let mut parts = pair.into_inner();
    let type_part = parts.next().unwrap();
    let name_part = parts.next().unwrap();
    let initializer = parts.next().map(|expr_part| parse_expression(expr_part)).transpose()?;

    let type_ = parse_type(type_part)?;
    let name = name_part.as_str().to_string();

    Ok(Statement::VariableDecl {
        name,
        type_,
        initializer,
        location: Some(ast_location),
    })
}

fn parse_assignment_statement(pair: Pair<Rule>, ast_location: crate::ast::SourceLocation) -> Result<Statement, CompilerError> {
    let mut parts = pair.into_inner();
    let target = parts.next().unwrap().as_str().to_string();
    let value = parse_expression(parts.next().unwrap())?;

    Ok(Statement::Assignment {
        target,
        value,
        location: Some(ast_location),
    })
}

fn parse_print_statement(pair: Pair<Rule>, ast_location: crate::ast::SourceLocation) -> Result<Statement, CompilerError> {
    let inner = pair.into_inner().next().unwrap();
    
    match inner.as_rule() {
        Rule::indented_print_block => {
            // Block syntax: print: followed by indented expressions
            let mut expressions = Vec::new();
            for item_pair in inner.into_inner() {
                if item_pair.as_rule() == Rule::print_item {
                    let expr_pair = item_pair.into_inner().next().unwrap();
                    expressions.push(parse_expression(expr_pair)?);
                }
            }
            Ok(Statement::PrintBlock {
                expressions,
                newline: false,
                location: Some(ast_location),
            })
        },
        Rule::expression => {
            // Simple syntax: print expression
            let expr = parse_expression(inner)?;
            Ok(Statement::Print {
                expression: expr,
                newline: false,
                location: Some(ast_location),
            })
        },
        _ => {
            // Handle parenthesized expressions or other cases
            let expr = parse_expression(inner)?;
            Ok(Statement::Print {
                expression: expr,
                newline: false,
                location: Some(ast_location),
            })
        }
    }
}

fn parse_println_statement(pair: Pair<Rule>, ast_location: crate::ast::SourceLocation) -> Result<Statement, CompilerError> {
    let inner = pair.into_inner().next().unwrap();
    
    match inner.as_rule() {
        Rule::indented_print_block => {
            // Block syntax: println: followed by indented expressions
            let mut expressions = Vec::new();
            for item_pair in inner.into_inner() {
                if item_pair.as_rule() == Rule::print_item {
                    let expr_pair = item_pair.into_inner().next().unwrap();
                    expressions.push(parse_expression(expr_pair)?);
                }
            }
            Ok(Statement::PrintBlock {
                expressions,
                newline: true,
                location: Some(ast_location),
            })
        },
        Rule::expression => {
            // Simple syntax: println expression
            let expr = parse_expression(inner)?;
            Ok(Statement::Print {
                expression: expr,
                newline: true,
                location: Some(ast_location),
            })
        },
        _ => {
            // Handle parenthesized expressions or other cases
            let expr = parse_expression(inner)?;
            Ok(Statement::Print {
                expression: expr,
                newline: true,
                location: Some(ast_location),
            })
        }
    }
}

fn parse_if_statement(pair: Pair<Rule>, ast_location: crate::ast::SourceLocation) -> Result<Statement, CompilerError> {
    let mut parts = pair.into_inner();
    let condition = parse_expression(parts.next().unwrap())?;
    
    let mut then_branch = Vec::new();
    let mut else_branch = None;

    for part in parts {
        match part.as_rule() {
            Rule::indented_block => {
                if then_branch.is_empty() {
                    // This is the then branch
                    for stmt_pair in part.into_inner() {
                        match stmt_pair.as_rule() {
                            Rule::statement => {
                                then_branch.push(parse_statement(stmt_pair)?);
                            },
                            _ => {}
                        }
                    }
                } else {
                    // This is the else branch
                    let mut else_stmts = Vec::new();
                    for stmt_pair in part.into_inner() {
                        match stmt_pair.as_rule() {
                            Rule::statement => {
                                else_stmts.push(parse_statement(stmt_pair)?);
                            },
                            _ => {}
                        }
                    }
                    else_branch = Some(else_stmts);
                }
            },
            _ => {}
        }
    }

    Ok(Statement::If {
        condition,
        then_branch,
        else_branch,
        location: Some(ast_location),
    })
}

fn parse_iterate_statement(pair: Pair<Rule>, ast_location: crate::ast::SourceLocation) -> Result<Statement, CompilerError> {
    let mut parts = pair.into_inner();
    let iterator = parts.next().unwrap().as_str().to_string();
    let collection = parse_expression(parts.next().unwrap())?;
    
    let mut body = Vec::new();
    for part in parts {
        match part.as_rule() {
            Rule::indented_block => {
                for stmt_pair in part.into_inner() {
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

    Ok(Statement::Iterate {
        iterator,
        collection,
        body,
        location: Some(ast_location),
    })
}

fn parse_range_iterate_statement(pair: Pair<Rule>, ast_location: crate::ast::SourceLocation) -> Result<Statement, CompilerError> {
    let mut parts = pair.into_inner();
    let iterator = parts.next().unwrap().as_str().to_string();
    let start = parse_expression(parts.next().unwrap())?;
    let end = parse_expression(parts.next().unwrap())?;
    
    let mut step = None;
    let mut body = Vec::new();

    for part in parts {
        match part.as_rule() {
            Rule::expression => {
                step = Some(parse_expression(part)?);
            },
            Rule::indented_block => {
                for stmt_pair in part.into_inner() {
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

    Ok(Statement::RangeIterate {
        iterator,
        start,
        end,
        step,
        body,
        location: Some(ast_location),
    })
}

fn parse_test_statement(pair: Pair<Rule>, ast_location: crate::ast::SourceLocation) -> Result<Statement, CompilerError> {
    let mut parts = pair.into_inner();
    let name = parts.next().unwrap().as_str().trim_matches('"').to_string();
    
    let mut body = Vec::new();
    for part in parts {
        match part.as_rule() {
            Rule::indented_block => {
                for stmt_pair in part.into_inner() {
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

    Ok(Statement::Test {
        name,
        body,
        location: Some(ast_location),
    })
}

fn parse_return_statement(pair: Pair<Rule>, ast_location: crate::ast::SourceLocation) -> Result<Statement, CompilerError> {
    let value = pair.into_inner().next().map(|expr_part| parse_expression(expr_part)).transpose()?;

    Ok(Statement::Return {
        value,
        location: Some(ast_location),
    })
}

fn parse_apply_block_statement(pair: Pair<Rule>, ast_location: crate::ast::SourceLocation) -> Result<Statement, CompilerError> {
    let mut parts = pair.into_inner();
    let target = parts.next().unwrap().as_str().to_string();
    
    let mut items = Vec::new();
    for part in parts {
        match part.as_rule() {
            Rule::indented_block => {
                // Handle apply block items - for now just treat as expressions
                for item_pair in part.into_inner() {
                    match item_pair.as_rule() {
                        Rule::statement => {
                            // Convert statement to apply item if it's an expression
                            let stmt = parse_statement(item_pair)?;
                            if let Statement::Expression { expr, .. } = stmt {
                                items.push(crate::ast::ApplyItem::FunctionCall(expr));
                            }
                        },
                        _ => {}
                    }
                }
            },
            _ => {}
        }
    }
    
    Ok(Statement::ApplyBlock {
        target,
        items,
        location: Some(ast_location),
    })
} 