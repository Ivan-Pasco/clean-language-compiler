use pest::iterators::Pair;
use crate::ast::{Statement, Expression};
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
        Rule::if_stmt => parse_if_statement(inner, ast_location),
        Rule::iterate_stmt => parse_iterate_statement(inner, ast_location),
        Rule::range_iterate_stmt => parse_range_iterate_statement(inner, ast_location),
        Rule::test => parse_test_statement(inner, ast_location),
        Rule::return_stmt => parse_return_statement(inner, ast_location),
        Rule::error_stmt => parse_error_statement(inner, ast_location),
        Rule::on_error_block => parse_on_error_block_statement(inner, ast_location),
        Rule::apply_block => parse_apply_block_statement(inner, ast_location),
        Rule::type_apply_block => parse_type_apply_block_statement(inner, ast_location),
        Rule::function_apply_block => parse_function_apply_block_statement(inner, ast_location),
        Rule::method_apply_block => parse_method_apply_block_statement(inner, ast_location),
        Rule::constant_apply_block => parse_constant_apply_block_statement(inner, ast_location),
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
    let target_part = parts.next().unwrap();
    let value = parse_expression(parts.next().unwrap())?;

    // Check if this is a property assignment (e.g., list.type = "line")
    if target_part.as_rule() == Rule::property_access {
        // Parse as property assignment expression
        let property_expr = super::expression_parser::parse_property_access(target_part)?;
        if let Expression::PropertyAccess { object, property, location } = property_expr {
            return Ok(Statement::Expression {
                expr: Expression::PropertyAssignment {
                    object,
                    property,
                    value: Box::new(value),
                    location,
                },
                location: Some(ast_location),
            });
        }
        // If we get here, something went wrong with property parsing
        return Err(CompilerError::parse_error(
            "Failed to parse property assignment".to_string(),
            Some(ast_location),
            Some("Property assignment parsing failed".to_string())
        ));
    }

    // Regular variable assignment
    let target = target_part.as_str().to_string();
    Ok(Statement::Assignment {
        target,
        value,
        location: Some(ast_location),
    })
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

fn parse_error_statement(pair: Pair<Rule>, ast_location: crate::ast::SourceLocation) -> Result<Statement, CompilerError> {
    // error("message") syntax
    let mut inner = pair.into_inner();
    let message_expr = inner.next().unwrap();
    let message = parse_expression(message_expr)?;
    
    Ok(Statement::Error {
        message,
        location: Some(ast_location),
    })
}

fn parse_on_error_block_statement(pair: Pair<Rule>, ast_location: crate::ast::SourceLocation) -> Result<Statement, CompilerError> {
    // This should be handled as an expression, not a statement
    // Convert the onError block to an expression statement
    let expr = super::expression_parser::parse_expression(pair)?;
    Ok(Statement::Expression {
        expr,
        location: Some(ast_location),
    })
}

fn parse_apply_block_statement(pair: Pair<Rule>, ast_location: crate::ast::SourceLocation) -> Result<Statement, CompilerError> {
    // apply_block is a choice rule, so we need to dispatch to the specific type
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::type_apply_block => parse_type_apply_block_statement(inner, ast_location),
        Rule::function_apply_block => parse_function_apply_block_statement(inner, ast_location),
        Rule::method_apply_block => parse_method_apply_block_statement(inner, ast_location),
        Rule::constant_apply_block => parse_constant_apply_block_statement(inner, ast_location),
        _ => Err(CompilerError::parse_error(
            format!("Unexpected apply block type: {:?}", inner.as_rule()),
            Some(ast_location),
            Some("Expected type, function, method, or constant apply block".to_string())
        )),
    }
}

fn parse_type_apply_block_statement(pair: Pair<Rule>, ast_location: crate::ast::SourceLocation) -> Result<Statement, CompilerError> {
    let mut parts = pair.into_inner();
    let type_part = parts.next().unwrap();
    
    // Parse the specific type allowed in type_apply_block: core_type | sized_type | matrix_type | array_type | pairs_type
    let type_ = match type_part.as_rule() {
        Rule::core_type => {
            let type_str = type_part.as_str();
            match type_str {
                "boolean" => crate::ast::Type::Boolean,
                "integer" => crate::ast::Type::Integer,
                "float" => crate::ast::Type::Float,
                "string" => crate::ast::Type::String,
                "void" => crate::ast::Type::Void,
                _ => return Err(CompilerError::parse_error(
                    format!("Unknown core type: {}", type_str),
                    Some(ast_location),
                    Some("Valid core types are: boolean, integer, float, string, void".to_string())
                ))
            }
        },
        Rule::sized_type => parse_type(type_part)?,
        Rule::matrix_type => parse_type(type_part)?,
        Rule::array_type => parse_type(type_part)?,
        Rule::pairs_type => parse_type(type_part)?,
        _ => return Err(CompilerError::parse_error(
            format!("Unexpected type in type apply block: {:?}", type_part.as_rule()),
            Some(ast_location),
            Some("Type apply blocks only support core types, sized types, matrix types, array types, and pairs types".to_string())
        ))
    };
    
    let mut assignments = Vec::new();
    for part in parts {
        match part.as_rule() {
            Rule::indented_variable_assignments => {
                for assignment_pair in part.into_inner() {
                    if assignment_pair.as_rule() == Rule::variable_assignment {
                        let mut assignment_parts = assignment_pair.into_inner();
                        let name = assignment_parts.next().unwrap().as_str().to_string();
                        let initializer = assignment_parts.next().map(|expr_pair| parse_expression(expr_pair)).transpose()?;
                        
                        assignments.push(crate::ast::VariableAssignment { name, initializer });
                    }
                }
            },
            _ => {}
        }
    }
    
    Ok(Statement::TypeApplyBlock {
        type_,
        assignments,
        location: Some(ast_location),
    })
}

fn parse_function_apply_block_statement(pair: Pair<Rule>, ast_location: crate::ast::SourceLocation) -> Result<Statement, CompilerError> {
    let mut parts = pair.into_inner();
    let function_name = parts.next().unwrap().as_str().to_string();
    
    let mut expressions = Vec::new();
    for part in parts {
        match part.as_rule() {
            Rule::indented_expressions => {
                for expr_pair in part.into_inner() {
                    if expr_pair.as_rule() == Rule::expression {
                        expressions.push(parse_expression(expr_pair)?);
                    }
                }
            },
            _ => {}
        }
    }
    
    Ok(Statement::FunctionApplyBlock {
        function_name,
        expressions,
        location: Some(ast_location),
    })
}

fn parse_method_apply_block_statement(pair: Pair<Rule>, ast_location: crate::ast::SourceLocation) -> Result<Statement, CompilerError> {
    let mut parts = pair.into_inner();
    let method_call_chain_part = parts.next().unwrap();
    
    // Parse the method call chain: object.method or object.method1.method2
    let mut chain_parts = method_call_chain_part.into_inner();
    let object_name = chain_parts.next().unwrap().as_str().to_string();
    let mut method_chain = Vec::new();
    
    for part in chain_parts {
        if part.as_rule() == Rule::identifier {
            method_chain.push(part.as_str().to_string());
        }
    }
    
    let mut expressions = Vec::new();
    for part in parts {
        match part.as_rule() {
            Rule::indented_expressions => {
                for expr_pair in part.into_inner() {
                    if expr_pair.as_rule() == Rule::expression {
                        expressions.push(parse_expression(expr_pair)?);
                    }
                }
            },
            _ => {}
        }
    }
    
    Ok(Statement::MethodApplyBlock {
        object_name,
        method_chain,
        expressions,
        location: Some(ast_location),
    })
}

fn parse_constant_apply_block_statement(pair: Pair<Rule>, ast_location: crate::ast::SourceLocation) -> Result<Statement, CompilerError> {
    let mut constants = Vec::new();
    
    for part in pair.into_inner() {
        match part.as_rule() {
            Rule::indented_constant_assignments => {
                for constant_pair in part.into_inner() {
                    if constant_pair.as_rule() == Rule::constant_assignment {
                        let mut constant_parts = constant_pair.into_inner();
                        let type_ = parse_type(constant_parts.next().unwrap())?;
                        let name = constant_parts.next().unwrap().as_str().to_string();
                        let value = parse_expression(constant_parts.next().unwrap())?;
                        
                        constants.push(crate::ast::ConstantAssignment { type_, name, value });
                    }
                }
            },
            _ => {}
        }
    }
    
    Ok(Statement::ConstantApplyBlock {
        constants,
        location: Some(ast_location),
    })
} 