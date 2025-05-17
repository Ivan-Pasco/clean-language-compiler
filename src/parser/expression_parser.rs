use pest::iterators::Pair;
use crate::ast::{Expression, Value, BinaryOperator, MatrixOperator, ComparisonOperator};
use crate::error::CompilerError;
use super::{get_location, StringPart, convert_to_ast_location, SourceLocation};
use super::Rule;

// Helper function to convert location from parser format to AST format
fn get_ast_location(location: &super::SourceLocation) -> crate::ast::SourceLocation {
    convert_to_ast_location(location)
}

#[derive(Debug, Clone, PartialEq)]
enum ParsedOperator {
    Binary(BinaryOperator),
    Comparison(ComparisonOperator),
    Matrix(MatrixOperator),
}

impl ParsedOperator {
    fn precedence(&self) -> u8 {
        match self {
            ParsedOperator::Matrix(_) => 4,
            ParsedOperator::Binary(op) => match op {
                BinaryOperator::Multiply | BinaryOperator::Divide => 3,
                BinaryOperator::Add | BinaryOperator::Subtract => 2,
                BinaryOperator::And | BinaryOperator::Or => 1,
                _ => 0,
            },
            ParsedOperator::Comparison(_) => 0,
        }
    }
}

pub fn parse_expression(pair: Pair<Rule>) -> Result<Expression, CompilerError> {
    let mut expr_stack = Vec::new();
    let mut op_stack = Vec::new();

    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::primary => {
                expr_stack.push(parse_primary(item)?);
            }
            Rule::matrix_operation => {
                let op = match item.as_str() {
                    "@*" => MatrixOperator::Multiply,
                    "@+" => MatrixOperator::Add,
                    "@-" => MatrixOperator::Subtract,
                    "@T" => MatrixOperator::Transpose,
                    "@I" => MatrixOperator::Inverse,
                    _ => return Err(CompilerError::parse_error(
                        format!("Invalid matrix operator: {}", item.as_str()),
                        Some(convert_to_ast_location(&get_location(&item))),
                        Some("Use a valid matrix operator (@*, @+, @-, @T, @I)".to_string())
                    )),
                };
                op_stack.push(ParsedOperator::Matrix(op));
            }
            Rule::binary_op => {
                let op = match item.as_str() {
                    "+" => BinaryOperator::Add,
                    "-" => BinaryOperator::Subtract,
                    "*" => BinaryOperator::Multiply,
                    "/" => BinaryOperator::Divide,
                    "&&" | "and" => BinaryOperator::And,
                    "||" | "or" => BinaryOperator::Or,
                    _ => return Err(CompilerError::parse_error(
                        format!("Invalid binary operator: {}", item.as_str()),
                        Some(convert_to_ast_location(&get_location(&item))),
                        Some("Valid binary operators are: +, -, *, /, %, and/&&, or/||".to_string())
                    )),
                };
                op_stack.push(ParsedOperator::Binary(op));
            }
            Rule::comparison_op => {
                let op = match item.as_str() {
                    "==" | "is" => ComparisonOperator::Equal,
                    "!=" | "not" => ComparisonOperator::NotEqual,
                    "<" => ComparisonOperator::Less,
                    "<=" => ComparisonOperator::LessEquals,
                    ">" => ComparisonOperator::Greater,
                    ">=" => ComparisonOperator::GreaterEquals,
                    _ => return Err(CompilerError::parse_error(
                        format!("Invalid comparison operator: {}", item.as_str()),
                        Some(convert_to_ast_location(&get_location(&item))),
                        Some("Valid comparison operators are: ==/is, !=/not, <, <=, >, >=".to_string())
                    )),
                };
                // Convert ComparisonOperator to BinaryOperator for AST consistency
                // Use From<ComparisonOperator> for BinaryOperator implementation from ast/mod.rs
                let binary_op = BinaryOperator::from(op);
                op_stack.push(ParsedOperator::Binary(binary_op));
            }
            _ => {}
        }
    }

    // Apply operators with precedence
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
        "Empty expression".to_string(),
        None,
        Some("An expression must contain at least one value".to_string())
    ))
}

fn apply_operator(left: Expression, op: ParsedOperator, right: Expression) -> Result<Expression, CompilerError> {
    // Get a default location as we don't have access to line/column info here
    let location = crate::ast::SourceLocation::default();
    
    match op {
        ParsedOperator::Binary(op) => Ok(Expression::Binary(Box::new(left), op, Box::new(right))),
        ParsedOperator::Comparison(op) => {
            // In the AST, comparison operations are represented as Binary operations with comparison operators
            Ok(Expression::Binary(Box::new(left), BinaryOperator::from(op), Box::new(right)))
        },
        ParsedOperator::Matrix(op) => Ok(Expression::MatrixOperation(Box::new(left), op, Box::new(right), location)),
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
            let num_str = inner.as_str();
            if num_str.ends_with('n') || num_str.ends_with('N') {
                Ok(Expression::Literal(Value::Big(num_str[..num_str.len()-1].to_string())))
            } else if num_str.ends_with("un") || num_str.ends_with("UN") {
                Ok(Expression::Literal(Value::UBig(num_str[..num_str.len()-2].to_string())))
            } else if num_str.contains('.') {
                num_str.parse::<f64>()
                    .map(Value::Number)
                    .map(Expression::Literal)
                    .map_err(|_| CompilerError::parse_error(
                        format!("Invalid number: {}", num_str),
                        Some(convert_to_ast_location(&location)),
                        Some("Check that the number is in a valid format".to_string())
                    ))
            } else {
                num_str.parse::<i64>()
                    .map(|n| Value::Integer(n as i32))
                    .map(Expression::Literal)
                    .map_err(|_| CompilerError::parse_error(
                        format!("Invalid integer: {}", num_str),
                        Some(convert_to_ast_location(&location)),
                        Some("Check that the integer is in a valid format".to_string())
                    ))
            }
        },
        Rule::integer => {
            // Direct handling of integer literals
            let num_str = inner.as_str();
            num_str.parse::<i64>()
                .map(|n| Value::Integer(n as i32))
                .map(Expression::Literal)
                .map_err(|_| CompilerError::parse_error(
                    format!("Invalid integer: {}", num_str),
                    Some(convert_to_ast_location(&location)),
                    Some("Check that the integer is in a valid format".to_string())
                ))
        },
        Rule::float => {
            // Direct handling of float literals
            let num_str = inner.as_str();
            num_str.parse::<f64>()
                .map(Value::Number)
                .map(Expression::Literal)
                .map_err(|_| CompilerError::parse_error(
                    format!("Invalid float: {}", num_str),
                    Some(convert_to_ast_location(&location)),
                    Some("Check that the float is in a valid format".to_string())
                ))
        },
        Rule::string => parse_string(inner),
        Rule::boolean => Ok(Expression::Literal(Value::Boolean(inner.as_str() == "true"))),
        Rule::array_literal => parse_array_literal(inner),
        Rule::matrix_literal => parse_matrix_literal(inner),
        Rule::identifier => Ok(Expression::Variable(inner.as_str().to_string())),
        Rule::function_call => parse_function_call(inner),
        Rule::expression => parse_expression(inner), // For parenthesized expressions
        _ => Err(CompilerError::parse_error(
            format!("Unexpected expression type: {:?}", inner.as_rule()),
            Some(convert_to_ast_location(&location)),
            Some("Check the syntax of this expression".to_string())
        )),
    }
}

pub fn parse_string(pair: Pair<Rule>) -> Result<Expression, CompilerError> {
    let location = get_location(&pair);
    let mut parts = Vec::new();
    
    for part in pair.into_inner() {
        match part.as_rule() {
            Rule::string_content => {
                // Remove escaped characters
                let text = part.as_str()
                    .replace("\\n", "\n")
                    .replace("\\t", "\t")
                    .replace("\\r", "\r")
                    .replace("\\\"", "\"")
                    .replace("\\\\", "\\");
                parts.push(StringPart::Text(text));
            }
            Rule::string_interpolation => {
                let part_location = get_location(&part);
                let expr_pair = part.clone().into_inner().next().ok_or_else(|| CompilerError::parse_error(
                    "Empty string interpolation".to_string(),
                    Some(convert_to_ast_location(&part_location)),
                    Some("A string interpolation must contain an expression".to_string())
                ))?;
                
                let expr = parse_expression(expr_pair)?;
                parts.push(StringPart::Expression(Box::new(expr)));
            }
            _ => {}
        }
    }
    
    // If there's only one part and it's plain text, return a simple string
    if parts.len() == 1 {
        if let StringPart::Text(text) = &parts[0] {
            return Ok(Expression::Literal(Value::String(text.clone())));
        }
    }
    
    // Convert StringPart values to Expression values for string concatenation
    let expressions = parts.into_iter().map(|part| match part {
        StringPart::Text(text) => Expression::Literal(Value::String(text)),
        StringPart::Expression(expr) => *expr,
    }).collect();
    
    // Return a StringConcat expression
    Ok(Expression::StringConcat(expressions))
}

pub fn parse_array_literal(pair: Pair<Rule>) -> Result<Expression, CompilerError> {
    let mut elements = Vec::new();
    
    for item in pair.into_inner() {
        if item.as_rule() == Rule::expression {
            elements.push(parse_expression(item)?);
        }
    }
    
    // Convert array to Value::Array to match the AST definition
    Ok(Expression::Literal(Value::Array(elements.iter().map(|e| {
        match e {
            Expression::Literal(value) => value.clone(),
            _ => Value::Null // For non-literal expressions, use null as placeholder
        }
    }).collect())))
}

pub fn parse_matrix_literal(pair: Pair<Rule>) -> Result<Expression, CompilerError> {
    let location = get_location(&pair);
    let mut rows = Vec::new();
    let mut current_row = Vec::new();
    
    for item in pair.clone().into_inner() {
        match item.as_rule() {
            Rule::expression => {
                // Try to extract number values for the matrix
                let expr = parse_expression(item)?;
                if let Expression::Literal(Value::Number(num)) = expr {
                    current_row.push(num);
                } else if let Expression::Literal(Value::Integer(num)) = expr {
                    current_row.push(num as f64);
                } else {
                    return Err(CompilerError::parse_error(
                        "Matrix elements must be numeric".to_string(),
                        Some(convert_to_ast_location(&location)),
                        Some("Only numbers can be used in matrices".to_string())
                    ));
                }
            }
            Rule::matrix_row_end => {
                if !current_row.is_empty() {
                    rows.push(current_row);
                    current_row = Vec::new();
                }
            }
            _ => {}
        }
    }
    
    // Don't forget the last row if there's no trailing comma
    if !current_row.is_empty() {
        rows.push(current_row);
    }
    
    // Convert to Matrix Value
    Ok(Expression::Literal(Value::Matrix(rows)))
}

pub fn parse_function_call(pair: Pair<Rule>) -> Result<Expression, CompilerError> {
    let mut name = String::new();
    let mut arguments = Vec::new();
    let location = get_location(&pair);
    let ast_location = convert_to_ast_location(&location);
    
    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::identifier => name = item.as_str().to_string(),
            Rule::expression => arguments.push(parse_expression(item)?),
            _ => {}
        }
    }
    
    if name.is_empty() {
        return Err(CompilerError::parse_error(
            "Function call is missing function name".to_string(),
            Some(ast_location),
            Some("Function calls must have a function name".to_string())
        ));
    }
    
    Ok(Expression::Call(name, arguments))
} 