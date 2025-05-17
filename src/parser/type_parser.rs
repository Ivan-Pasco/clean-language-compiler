use pest::iterators::Pair;
use crate::ast::Type;
use crate::error::CompilerError;
use super::{get_location, convert_to_ast_location};
use super::Rule;

pub fn parse_type(pair: Pair<Rule>) -> Result<Type, CompilerError> {
    let parser_location = get_location(&pair);
    let ast_location = convert_to_ast_location(&parser_location);
    
    let inner = pair.into_inner().next().ok_or_else(|| CompilerError::parse_error(
        "Empty type declaration".to_string(),
        Some(convert_to_ast_location(&parser_location)),
        Some("Type declarations must specify a type".to_string())
    ))?;
    
    match inner.as_rule() {
        Rule::generic_type => parse_generic_type(inner),
        Rule::type_parameter => Ok(Type::TypeParameter(inner.as_str().to_string())),
        Rule::identifier => parse_basic_type(inner),
        Rule::matrix_type => parse_matrix_type(inner),
        Rule::array_type => parse_array_type(inner),
        Rule::map_type => parse_map_type(inner),
        _ => Err(CompilerError::parse_error(
            format!("Unexpected type: {:?}", inner.as_rule()),
            Some(ast_location),
            Some("Type declarations must specify a valid type".to_string())
        ))
    }
}

fn parse_generic_type(pair: Pair<Rule>) -> Result<Type, CompilerError> {
    let parser_location = get_location(&pair);
    let ast_location = convert_to_ast_location(&parser_location);
    
    let mut base_type = String::new();
    let mut type_arguments = Vec::new();
    
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::identifier => base_type = inner.as_str().to_string(),
            Rule::type_arguments => {
                for type_arg in inner.into_inner() {
                    type_arguments.push(parse_type(type_arg)?);
                }
            },
            _ => return Err(CompilerError::parse_error(
                format!("Unexpected rule in generic type: {:?}", inner.as_rule()),
                Some(ast_location),
                Some("Generic types must have a base type and optional type arguments".to_string())
            ))
        }
    }
    
    let boxed_base_type = Box::new(Type::Object(base_type));
    Ok(Type::Generic(boxed_base_type, type_arguments))
}

fn parse_basic_type(pair: Pair<Rule>) -> Result<Type, CompilerError> {
    let parser_location = get_location(&pair);
    let ast_location = convert_to_ast_location(&parser_location);
    
    let type_name = pair.as_str();
    match type_name {
        "int" => Ok(Type::Integer),
        "float" => Ok(Type::Float),
        "bool" => Ok(Type::Boolean),
        "string" => Ok(Type::String),
        "unit" => Ok(Type::Unit),
        _ => Ok(Type::Object(type_name.to_string()))
    }
}

fn parse_matrix_type(pair: Pair<Rule>) -> Result<Type, CompilerError> {
    let parser_location = get_location(&pair);
    let ast_location = convert_to_ast_location(&parser_location);
    
    let element_type = pair.into_inner().next()
        .ok_or_else(|| CompilerError::parse_error(
            "Matrix type must specify element type".to_string(),
            Some(ast_location),
            Some("Matrix types must be in the form Matrix<T>".to_string())
        ))?;
    
    let element_type = parse_type(element_type)?;
    Ok(Type::Matrix(Box::new(element_type)))
}

fn parse_array_type(pair: Pair<Rule>) -> Result<Type, CompilerError> {
    let parser_location = get_location(&pair);
    let ast_location = convert_to_ast_location(&parser_location);
    
    let element_type = pair.into_inner().next()
        .ok_or_else(|| CompilerError::parse_error(
            "Array type must specify element type".to_string(),
            Some(ast_location),
            Some("Array types must be in the form Array<T>".to_string())
        ))?;
    
    let element_type = parse_type(element_type)?;
    Ok(Type::Array(Box::new(element_type)))
}

fn parse_map_type(pair: Pair<Rule>) -> Result<Type, CompilerError> {
    let parser_location = get_location(&pair);
    let ast_location = convert_to_ast_location(&parser_location);
    
    let mut key_type = None;
    let mut value_type = None;
    
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::type_ {
            if key_type.is_none() {
                key_type = Some(parse_type(inner)?);
            } else if value_type.is_none() {
                value_type = Some(parse_type(inner)?);
            }
        }
    }
    
    let key_type = key_type.ok_or_else(|| CompilerError::parse_error(
        "Map type must specify key type".to_string(),
        Some(ast_location.clone()),
        Some("Map types must be in the form Map<K, V>".to_string())
    ))?;
    
    let value_type = value_type.ok_or_else(|| CompilerError::parse_error(
        "Map type must specify value type".to_string(),
        Some(ast_location),
        Some("Map types must be in the form Map<K, V>".to_string())
    ))?;
    
    let boxed_base_type = Box::new(Type::Object("Map".to_string()));
    let type_args = vec![key_type, value_type];
    Ok(Type::Generic(boxed_base_type, type_args))
} 