use pest::iterators::Pair;
use crate::ast::{Expression, Value, BinaryOperator, UnaryOperator, StringPart};
use crate::error::CompilerError;
use super::{get_location, convert_to_ast_location};
use super::Rule;

// Helper function to convert location from parser format to AST format

#[derive(Debug, Clone)]
enum ParsedOperator {
    Binary(BinaryOperator),
}

impl ParsedOperator {
    fn precedence(&self) -> u8 {
        match self {
            ParsedOperator::Binary(op) => match op {
                BinaryOperator::Power => 5,
                BinaryOperator::Multiply | BinaryOperator::Divide | BinaryOperator::Modulo => 4,
                BinaryOperator::Add | BinaryOperator::Subtract => 3,
                BinaryOperator::Less | BinaryOperator::Greater | 
                BinaryOperator::LessEqual | BinaryOperator::GreaterEqual => 2,
                BinaryOperator::Equal | BinaryOperator::NotEqual | 
                BinaryOperator::Is | BinaryOperator::Not => 1,
                BinaryOperator::And => 1,
                BinaryOperator::Or => 0,
            },
        }
    }
}

pub fn parse_expression(pair: Pair<Rule>) -> Result<Expression, CompilerError> {
    match pair.as_rule() {
        Rule::expression => {
            // Handle the top-level expression rule
            let inner = pair.into_inner().next().unwrap();
            parse_expression(inner)
        }
        Rule::on_error_expr => {
            // Handle onError expression
            let location = convert_to_ast_location(&get_location(&pair));
            let mut inner = pair.into_inner();
            let expression = parse_expression(inner.next().unwrap())?;
            let fallback = parse_expression(inner.next().unwrap())?;
            
            Ok(Expression::OnError {
                expression: Box::new(expression),
                fallback: Box::new(fallback),
                location,
            })
        }
        Rule::on_error_block => {
            // Handle onError block
            let location = convert_to_ast_location(&get_location(&pair));
            let mut inner = pair.into_inner();
            let expression = parse_expression(inner.next().unwrap())?;
            
            // Parse the indented block
            let block_pair = inner.next().unwrap();
            let mut error_handler = Vec::new();
            
            for stmt_pair in block_pair.into_inner() {
                if stmt_pair.as_rule() == Rule::statement {
                    error_handler.push(crate::parser::statement_parser::parse_statement(stmt_pair)?);
                }
            }
            
            Ok(Expression::OnErrorBlock {
                expression: Box::new(expression),
                error_handler,
                location,
            })
        }
        Rule::base_expression => {
            parse_base_expression(pair)
        }
        Rule::error_variable => {
            // Parse error variable
            let location = convert_to_ast_location(&get_location(&pair));
            Ok(Expression::ErrorVariable { location })
        }
        _ => {
            // For backward compatibility, try parsing as base_expression
            parse_base_expression(pair)
        }
    }
}

pub fn parse_base_expression(pair: Pair<Rule>) -> Result<Expression, CompilerError> {
    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::logical_expression => {
                return parse_logical_expression(item);
            }
            _ => return Err(CompilerError::parse_error(
                format!("Unexpected rule in base expression: {:?}", item.as_rule()),
                Some(convert_to_ast_location(&get_location(&item))),
                Some("Expected logical expression".to_string())
            )),
        }
    }
    
    Err(CompilerError::parse_error(
        "Empty base expression".to_string(),
        None,
        Some("Base expression must contain a logical expression".to_string())
    ))
}

pub fn parse_logical_expression(pair: Pair<Rule>) -> Result<Expression, CompilerError> {
    let mut expr_stack = Vec::new();
    let mut op_stack = Vec::new();

    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::comparison_expression => {
                expr_stack.push(parse_comparison_expression(item)?);
            }
            Rule::logical_op => {
                let op = match item.as_str() {
                    "and" => BinaryOperator::And,
                    "or" => BinaryOperator::Or,
                    _ => return Err(CompilerError::parse_error(
                        format!("Invalid logical operator: {}", item.as_str()),
                        Some(convert_to_ast_location(&get_location(&item))),
                        Some("Valid logical operators are: and, or".to_string())
                    )),
                };
                op_stack.push(op);
            }
            _ => return Err(CompilerError::parse_error(
                format!("Unexpected rule in logical expression: {:?}", item.as_rule()),
                Some(convert_to_ast_location(&get_location(&item))),
                Some("Expected comparison expression or logical operator".to_string())
            )),
        }
    }

    // Build the expression tree from the stacks
    if expr_stack.is_empty() {
        return Err(CompilerError::parse_error(
            "Empty logical expression".to_string(),
            None,
            Some("Logical expression must contain at least one comparison".to_string())
        ));
    }

    let mut result = expr_stack.remove(0);
    let mut i = 0;

    while i < op_stack.len() && i < expr_stack.len() {
        let right = expr_stack.remove(0);
        result = Expression::Binary(Box::new(result), op_stack[i].clone(), Box::new(right));
        i += 1;
    }

    Ok(result)
}

pub fn parse_comparison_expression(pair: Pair<Rule>) -> Result<Expression, CompilerError> {
    let mut expr_stack = Vec::new();
    let mut op_stack = Vec::new();

    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::unary_expression => {
                expr_stack.push(parse_unary_expression(item)?);
            }
            Rule::comparison_op => {
                let op = match item.as_str() {
                    "==" => BinaryOperator::Equal,
                    "!=" => BinaryOperator::NotEqual,
                    "<" => BinaryOperator::Less,
                    "<=" => BinaryOperator::LessEqual,
                    ">" => BinaryOperator::Greater,
                    ">=" => BinaryOperator::GreaterEqual,
                    "is" => BinaryOperator::Is,
                    _ => return Err(CompilerError::parse_error(
                        format!("Invalid comparison operator: {}", item.as_str()),
                        Some(convert_to_ast_location(&get_location(&item))),
                        Some("Valid comparison operators are: ==, !=, <, <=, >, >=, is".to_string())
                    )),
                };
                op_stack.push(op);
            }
            _ => return Err(CompilerError::parse_error(
                format!("Unexpected rule in comparison expression: {:?}", item.as_rule()),
                Some(convert_to_ast_location(&get_location(&item))),
                Some("Expected arithmetic expression or comparison operator".to_string())
            )),
        }
    }

    // Build the expression tree from the stacks
    if expr_stack.is_empty() {
        return Err(CompilerError::parse_error(
            "Empty comparison expression".to_string(),
            None,
            Some("Comparison expression must contain at least one arithmetic expression".to_string())
        ));
    }

    let mut result = expr_stack.remove(0);
    let mut i = 0;

    while i < op_stack.len() && i < expr_stack.len() {
        let right = expr_stack.remove(0);
        result = Expression::Binary(Box::new(result), op_stack[i].clone(), Box::new(right));
        i += 1;
    }

    Ok(result)
}

pub fn parse_unary_expression(pair: Pair<Rule>) -> Result<Expression, CompilerError> {
    let mut unary_ops = Vec::new();
    let mut arithmetic_expr = None;
    
    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::unary_op => {
                let op = match item.as_str() {
                    "not" => UnaryOperator::Not,
                    "-" => UnaryOperator::Negate,
                    _ => return Err(CompilerError::parse_error(
                        format!("Invalid unary operator: {}", item.as_str()),
                        Some(convert_to_ast_location(&get_location(&item))),
                        Some("Valid unary operators are: not, -".to_string())
                    )),
                };
                unary_ops.push(op);
            }
            Rule::arithmetic_expression => {
                arithmetic_expr = Some(parse_arithmetic_expression(item)?);
            }
            _ => return Err(CompilerError::parse_error(
                format!("Unexpected rule in unary expression: {:?}", item.as_rule()),
                Some(convert_to_ast_location(&get_location(&item))),
                Some("Expected unary operator or arithmetic expression".to_string())
            )),
        }
    }
    
    let mut result = arithmetic_expr.ok_or_else(|| {
        CompilerError::parse_error(
            "Missing arithmetic expression in unary expression".to_string(),
            None,
            Some("Unary expression must contain an arithmetic expression".to_string())
        )
    })?;
    
    // Apply unary operators from right to left (since we parsed left to right)
    for op in unary_ops.into_iter().rev() {
        result = Expression::Unary(op, Box::new(result));
    }
    
    Ok(result)
}

pub fn parse_arithmetic_expression(pair: Pair<Rule>) -> Result<Expression, CompilerError> {
    let mut expr_stack = Vec::new();
    let mut op_stack = Vec::new();

    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::primary => {
                expr_stack.push(parse_primary(item)?);
            }
            Rule::arithmetic_op => {
                let op = match item.as_str() {
                    "+" => BinaryOperator::Add,
                    "-" => BinaryOperator::Subtract,
                    "*" => BinaryOperator::Multiply,
                    "/" => BinaryOperator::Divide,
                    "%" => BinaryOperator::Modulo,
                    "^" => BinaryOperator::Power,
                    _ => return Err(CompilerError::parse_error(
                        format!("Invalid arithmetic operator: {}", item.as_str()),
                        Some(convert_to_ast_location(&get_location(&item))),
                        Some("Valid arithmetic operators are: +, -, *, /, %, ^".to_string())
                    )),
                };
                op_stack.push(op);
            }
            _ => return Err(CompilerError::parse_error(
                format!("Unexpected rule in arithmetic expression: {:?}", item.as_rule()),
                Some(convert_to_ast_location(&get_location(&item))),
                Some("Expected primary expression or arithmetic operator".to_string())
            )),
        }
    }

    // Build the expression tree from the stacks
    if expr_stack.is_empty() {
        return Err(CompilerError::parse_error(
            "Empty arithmetic expression".to_string(),
            None,
            Some("Arithmetic expression must contain at least one primary expression".to_string())
        ));
    }

    let mut result = expr_stack.remove(0);
    let mut i = 0;

    while i < op_stack.len() && i < expr_stack.len() {
        let right = expr_stack.remove(0);
        result = Expression::Binary(Box::new(result), op_stack[i].clone(), Box::new(right));
        i += 1;
    }

    Ok(result)
}

fn apply_operator(left: Expression, op: ParsedOperator, right: Expression) -> Result<Expression, CompilerError> {
    match op {
        ParsedOperator::Binary(op) => Ok(Expression::Binary(Box::new(left), op, Box::new(right))),
    }
}

pub fn parse_primary(pair: Pair<Rule>) -> Result<Expression, CompilerError> {
    let location = get_location(&pair);
    let inner = pair.clone().into_inner().next().ok_or_else(|| CompilerError::parse_error(
        "Empty primary expression".to_string(),
        Some(convert_to_ast_location(&location)),
        Some("Expected a value inside the primary expression".to_string())
    ))?;
    
    match inner.as_rule() {
        Rule::number => {
            parse_number_literal(inner)
        },
        Rule::integer => {
            let num_str = inner.as_str();
            num_str.parse::<i64>()
                .map(Value::Integer)
                .map(Expression::Literal)
                .map_err(|_| CompilerError::parse_error(
                    format!("Invalid integer: {num_str}"),
                    Some(convert_to_ast_location(&location)),
                    Some("Check that the integer is in a valid format".to_string())
                ))
        },
        Rule::float => {
            let num_str = inner.as_str();
            num_str.parse::<f64>()
                .map(Value::Number)
                .map(Expression::Literal)
                .map_err(|_| CompilerError::parse_error(
                    format!("Invalid float: {num_str}"),
                    Some(convert_to_ast_location(&location)),
                    Some("Check that the float is in a valid format".to_string())
                ))
        },
        Rule::boolean => {
            let value = match inner.as_str() {
                "true" => true,
                "false" => false,
                _ => return Err(CompilerError::parse_error(
                    format!("Invalid boolean: {}", inner.as_str()),
                    Some(convert_to_ast_location(&location)),
                    Some("Boolean values must be 'true' or 'false'".to_string())
                )),
            };
            Ok(Expression::Literal(Value::Boolean(value)))
        },
        Rule::string => parse_string(inner),
        Rule::list_literal => parse_list_literal(inner),
        Rule::matrix_literal => parse_matrix_literal(inner),
        Rule::function_call => parse_function_call(inner),
        Rule::method_call => parse_method_call(inner),
        Rule::property_access => parse_property_access(inner),
        Rule::list_access => parse_list_access(inner),
        Rule::error_variable => {
            // Parse error variable
            Ok(Expression::ErrorVariable {
                location: convert_to_ast_location(&location),
            })
        },
        Rule::identifier => {
            let identifier = inner.as_str();
            Ok(Expression::Variable(identifier.to_string()))
        },
        Rule::expression => {
            // Handle parenthesized expressions: (expression)
            parse_expression(inner)
        },
        Rule::multiline_parenthesized_expr => {
            // Handle multi-line parenthesized expressions: (expr + \n expr)
            parse_multiline_parenthesized_expression(inner)
        },
        Rule::conditional_expr => {
            // Handle conditional expressions: if condition then value else value
            parse_conditional_expression(inner)
        },
        Rule::base_call => {
            // Handle base constructor calls: base(args...)
            parse_base_call(inner)
        },
        _ => Err(CompilerError::parse_error(
            format!("Unexpected primary expression: {}", inner.as_str()),
            Some(convert_to_ast_location(&location)),
            Some("Expected a literal, identifier, or function call".to_string())
        )),
    }
}

fn parse_number_literal(pair: Pair<Rule>) -> Result<Expression, CompilerError> {
    let num_str = pair.as_str();
    
    if num_str.contains('.') || num_str.contains('e') || num_str.contains('E') {
        // Float
        num_str.parse::<f64>()
            .map(Value::Number)
            .map(Expression::Literal)
            .map_err(|_| CompilerError::parse_error(
                format!("Invalid float: {num_str}"),
                None,
                Some("Check that the float is in a valid format".to_string())
            ))
    } else {
        // Integer
        num_str.parse::<i64>()
            .map(Value::Integer)
            .map(Expression::Literal)
            .map_err(|_| CompilerError::parse_error(
                format!("Invalid integer: {num_str}"),
                None,
                Some("Check that the integer is in a valid format".to_string())
            ))
    }
}

pub fn parse_string(pair: Pair<Rule>) -> Result<Expression, CompilerError> {
    let mut parts = Vec::new();
    
    for part in pair.into_inner() {
        match part.as_rule() {
            Rule::string_part => {
                // Handle string_part which contains either string_content or string_interpolation
                for inner_part in part.into_inner() {
                    match inner_part.as_rule() {
                        Rule::string_content => {
                            parts.push(StringPart::Text(inner_part.as_str().to_string()));
                        },
                        Rule::string_interpolation => {
                            // Handle {variable} or {object.property}
                            let mut inner = inner_part.into_inner();
                            let expr_str = inner.next().unwrap().as_str();
                            
                            // Parse simple property access
                            if expr_str.contains('.') {
                                let parts_split: Vec<&str> = expr_str.split('.').collect();
                                let object = Expression::Variable(parts_split[0].to_string());
                                let property = parts_split[1].to_string();
                                
                                let location = crate::ast::SourceLocation::default();
                                let property_access = Expression::PropertyAccess {
                                    object: Box::new(object),
                                    property,
                                    location,
                                };
                                parts.push(StringPart::Interpolation(property_access));
                            } else {
                                // Simple variable
                                let variable = Expression::Variable(expr_str.to_string());
                                parts.push(StringPart::Interpolation(variable));
                            }
                        },
                        _ => {}
                    }
                }
            },
            Rule::string_content => {
                // Direct string_content (shouldn't happen with current grammar, but keeping for safety)
                parts.push(StringPart::Text(part.as_str().to_string()));
            },
            Rule::string_interpolation => {
                // Direct string_interpolation (shouldn't happen with current grammar, but keeping for safety)
                let mut inner = part.into_inner();
                let expr_str = inner.next().unwrap().as_str();
                
                // Parse simple property access
                if expr_str.contains('.') {
                    let parts_split: Vec<&str> = expr_str.split('.').collect();
                    let object = Expression::Variable(parts_split[0].to_string());
                    let property = parts_split[1].to_string();
                    
                    let location = crate::ast::SourceLocation::default();
                    let property_access = Expression::PropertyAccess {
                        object: Box::new(object),
                        property,
                        location,
                    };
                    parts.push(StringPart::Interpolation(property_access));
                } else {
                    // Simple variable
                    let variable = Expression::Variable(expr_str.to_string());
                    parts.push(StringPart::Interpolation(variable));
                }
            },
            _ => {}
        }
    }
    
    // Check if this is a simple string (no interpolation)
    if parts.len() == 1 {
        if let StringPart::Text(text) = &parts[0] {
            // This is a simple string literal, return it as a literal value
            return Ok(Expression::Literal(Value::String(text.clone())));
        }
    } else if parts.is_empty() {
        // Empty string
        return Ok(Expression::Literal(Value::String(String::new())));
    }
    
    // This has interpolation parts, return as StringInterpolation
    Ok(Expression::StringInterpolation(parts))
}

pub fn parse_list_literal(pair: Pair<Rule>) -> Result<Expression, CompilerError> {
    let mut elements = Vec::new();
    
    for element in pair.into_inner() {
        if let Rule::expression = element.as_rule() {
            elements.push(parse_expression(element)?);
        }
    }
    
    // Convert to array values
    let values: Result<Vec<Value>, _> = elements.into_iter()
        .map(|expr| match expr {
            Expression::Literal(value) => Ok(value),
            _ => Err(CompilerError::parse_error(
                "List literals can only contain literal values".to_string(),
                None,
                Some("Use variables or function calls outside of list literals".to_string())
            ))
        })
        .collect();
    
    Ok(Expression::Literal(Value::List(values?)))
}

pub fn parse_matrix_literal(pair: Pair<Rule>) -> Result<Expression, CompilerError> {
    let mut rows = Vec::new();
    
    for matrix_row_pair in pair.into_inner() {
        if let Rule::matrix_row = matrix_row_pair.as_rule() {
            let mut row = Vec::new();
            
            for element in matrix_row_pair.into_inner() {
                if let Rule::expression = element.as_rule() {
                    let expr = parse_expression(element)?;
                    match expr {
                        Expression::Literal(Value::Number(f)) => row.push(f),
                        Expression::Literal(Value::Integer(i)) => row.push(i as f64),
                        _ => return Err(CompilerError::parse_error(
                            "Matrix literals can only contain numeric values".to_string(),
                            None,
                            Some("Use numeric literals in matrix definitions".to_string())
                        ))
                    }
                }
            }
            
            rows.push(row);
        }
    }
    
    Ok(Expression::Literal(Value::Matrix(rows)))
}

pub fn parse_function_call(pair: Pair<Rule>) -> Result<Expression, CompilerError> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let mut arguments = Vec::new();
    
    for arg in inner {
        if let Rule::expression = arg.as_rule() {
            arguments.push(parse_expression(arg)?);
        }
    }
    
    Ok(Expression::Call(name, arguments))
}

pub fn parse_method_call(pair: Pair<Rule>) -> Result<Expression, CompilerError> {
    let mut inner = pair.into_inner();
    
    // Parse method_call_base
    let base_pair = inner.next().unwrap();
    let object_expr = match base_pair.as_rule() {
        Rule::method_call_base => {
            let mut base_inner = base_pair.into_inner();
            let first = base_inner.next().unwrap();
            match first.as_rule() {
                Rule::identifier => Expression::Variable(first.as_str().to_string()),
                Rule::builtin_class_name => Expression::Variable(first.as_str().to_string()),
                Rule::expression => parse_expression(first)?,
                _ => return Err(CompilerError::parse_error(
                    "Invalid method call base".to_string(),
                    None,
                    None
                ))
            }
        },
        _ => return Err(CompilerError::parse_error(
            "Expected method_call_base".to_string(),
            None,
            None
        ))
    };
    
    let mut current_expr = object_expr;
    
    for segment in inner {
        if let Rule::method_call_segment = segment.as_rule() {
            let mut seg_inner = segment.into_inner();
            let first_child = seg_inner.next().unwrap();
            
            let (method_name, arguments) = match first_child.as_rule() {
                Rule::method_name => {
                    // Method call with mandatory parentheses
                    let method_name = first_child.as_str().to_string();
                    let mut arguments = Vec::new();
                    
                    // Parse arguments from the remaining segments
                    for arg in seg_inner {
                        if let Rule::expression = arg.as_rule() {
                            arguments.push(parse_expression(arg)?);
                        }
                    }
                    
                    (method_name, arguments)
                },
                _ => return Err(CompilerError::parse_error(
                    format!("Unexpected method call segment: {:?}", first_child.as_rule()),
                    None,
                    None
                ))
            };
            
            let location = crate::ast::SourceLocation::default();
            current_expr = Expression::MethodCall {
                object: Box::new(current_expr),
                method: method_name,
                arguments,
                location,
            };
        }
    }
    
    Ok(current_expr)
}

pub fn parse_property_access(pair: Pair<Rule>) -> Result<Expression, CompilerError> {
    let mut inner = pair.into_inner();
    let object_name = inner.next().unwrap().as_str().to_string();
    let mut current_expr = Expression::Variable(object_name);
    
    for segment in inner {
        let property_name = segment.as_str().to_string();
        let location = crate::ast::SourceLocation::default();
        current_expr = Expression::PropertyAccess {
            object: Box::new(current_expr),
            property: property_name,
            location,
        };
    }
    
    Ok(current_expr)
}

pub fn parse_list_access(pair: Pair<Rule>) -> Result<Expression, CompilerError> {
    let mut inner = pair.into_inner();
    
    // First element is the list identifier
    let list_name = inner.next().unwrap().as_str().to_string();
    let list_expr = Expression::Variable(list_name);
    
    // Second element is the index expression
    let index_pair = inner.next().unwrap();
    let index_expr = parse_expression(index_pair)?;
    
    Ok(Expression::ListAccess(Box::new(list_expr), Box::new(index_expr)))
}

pub fn parse_multiline_parenthesized_expression(pair: Pair<Rule>) -> Result<Expression, CompilerError> {
    // The multiline_parenthesized_expr contains a multiline_expression
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::multiline_expression {
            return parse_multiline_expression(inner);
        }
        // Skip NEWLINE and INDENT tokens
    }
    
    Err(CompilerError::parse_error(
        "Empty multi-line parenthesized expression".to_string(),
        None,
        Some("Multi-line expressions must contain at least one expression".to_string())
    ))
}

pub fn parse_multiline_expression(pair: Pair<Rule>) -> Result<Expression, CompilerError> {
    let mut expr_stack = Vec::new();
    let mut op_stack = Vec::new();

    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::primary => {
                expr_stack.push(parse_primary(item)?);
            }
            Rule::arithmetic_op => {
                let op = match item.as_str() {
                    "+" => BinaryOperator::Add,
                    "-" => BinaryOperator::Subtract,
                    "*" => BinaryOperator::Multiply,
                    "/" => BinaryOperator::Divide,
                    "%" => BinaryOperator::Modulo,
                    "^" => BinaryOperator::Power,
                    _ => return Err(CompilerError::parse_error(
                        format!("Invalid arithmetic operator: {}", item.as_str()),
                        Some(convert_to_ast_location(&get_location(&item))),
                        Some("Valid arithmetic operators are: +, -, *, /, %, ^".to_string())
                    )),
                };
                op_stack.push(ParsedOperator::Binary(op));
            }
            Rule::logical_op => {
                let op = match item.as_str() {
                    "and" => BinaryOperator::And,
                    "or" => BinaryOperator::Or,
                    _ => return Err(CompilerError::parse_error(
                        format!("Invalid logical operator: {}", item.as_str()),
                        Some(convert_to_ast_location(&get_location(&item))),
                        Some("Valid logical operators are: and, or".to_string())
                    )),
                };
                op_stack.push(ParsedOperator::Binary(op));
            }
            Rule::comparison_op => {
                let op = match item.as_str() {
                    "==" => BinaryOperator::Equal,
                    "!=" => BinaryOperator::NotEqual,
                    "<" => BinaryOperator::Less,
                    "<=" => BinaryOperator::LessEqual,
                    ">" => BinaryOperator::Greater,
                    ">=" => BinaryOperator::GreaterEqual,
                    "is" => BinaryOperator::Is,
                    _ => return Err(CompilerError::parse_error(
                        format!("Invalid comparison operator: {}", item.as_str()),
                        Some(convert_to_ast_location(&get_location(&item))),
                        Some("Valid comparison operators are: ==, !=, <, <=, >, >=, is".to_string())
                    )),
                };
                op_stack.push(ParsedOperator::Binary(op));
            }
            _ => {} // Skip NEWLINE and INDENT tokens
        }
    }

    // Apply operators with precedence (same logic as base_expression)
    while op_stack.len() > 1 && expr_stack.len() >= 3 {
        let op2 = op_stack.pop().unwrap();
        let op1 = op_stack.last().unwrap();
        
        if op1.precedence() >= op2.precedence() {
            let right = expr_stack.pop().ok_or_else(|| CompilerError::parse_error(
                "Missing right operand".to_string(),
                None,
                Some("Each operator requires two operands".to_string())
            ))?;
            
            let left = expr_stack.pop().ok_or_else(|| CompilerError::parse_error(
                "Missing left operand".to_string(),
                None,
                Some("Each operator requires two operands".to_string())
            ))?;
            
            expr_stack.push(apply_operator(left, op2, right)?);
        } else {
            op_stack.push(op2);
            break;
        }
    }

    // Apply remaining operators
    while !op_stack.is_empty() && expr_stack.len() >= 2 {
        let op = op_stack.pop().unwrap();
        let right = expr_stack.pop().ok_or_else(|| CompilerError::parse_error(
            "Missing right operand".to_string(),
            None,
            Some("Each operator requires two operands".to_string())
        ))?;
        
        let left = expr_stack.pop().ok_or_else(|| CompilerError::parse_error(
            "Missing left operand".to_string(),
            None,
            Some("Each operator requires two operands".to_string())
        ))?;
        
        expr_stack.push(apply_operator(left, op, right)?);
    }

    expr_stack.pop().ok_or_else(|| CompilerError::parse_error(
        "Empty multiline expression".to_string(),
        None,
        Some("A multiline expression must contain at least one value".to_string())
    ))
}

pub fn parse_conditional_expression(pair: Pair<Rule>) -> Result<Expression, CompilerError> {
    let location = convert_to_ast_location(&get_location(&pair));
    let mut inner = pair.into_inner();
    
    // Parse: if condition then value else value
    // The grammar gives us: expression, expression, expression (condition, then_expr, else_expr)
    let condition_pair = inner.next().ok_or_else(|| CompilerError::parse_error(
        "Missing condition in conditional expression".to_string(),
        Some(location.clone()),
        Some("Conditional expressions require: if condition then value else value".to_string())
    ))?;
    
    let then_pair = inner.next().ok_or_else(|| CompilerError::parse_error(
        "Missing then expression in conditional expression".to_string(),
        Some(location.clone()),
        Some("Conditional expressions require: if condition then value else value".to_string())
    ))?;
    
    let else_pair = inner.next().ok_or_else(|| CompilerError::parse_error(
        "Missing else expression in conditional expression".to_string(),
        Some(location.clone()),
        Some("Conditional expressions require: if condition then value else value".to_string())
    ))?;
    
    let condition = parse_expression(condition_pair)?;
    let then_expr = parse_expression(then_pair)?;
    let else_expr = parse_expression(else_pair)?;
    
    Ok(Expression::Conditional {
        condition: Box::new(condition),
        then_expr: Box::new(then_expr),
        else_expr: Box::new(else_expr),
        location,
    })
}

pub fn parse_base_call(pair: Pair<Rule>) -> Result<Expression, CompilerError> {
    let location = get_location(&pair);
    let mut arguments = Vec::new();
    
    for arg in pair.into_inner() {
        if let Rule::expression = arg.as_rule() {
            arguments.push(parse_expression(arg)?);
        }
    }
    
    Ok(Expression::BaseCall {
        arguments,
        location: convert_to_ast_location(&location),
    })
} 