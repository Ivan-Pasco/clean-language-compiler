use pest::iterators::Pair;
use crate::ast::{Class, Constructor, Field, Visibility, Parameter};
use crate::error::CompilerError;
use super::{get_location, convert_to_ast_location};
use super::Rule;
use super::statement_parser::parse_statement;
use super::type_parser::parse_type;
use super::parser_impl::parse_functions_block;

pub fn parse_class(pair: Pair<Rule>) -> Result<Class, CompilerError> {
    let mut name = String::new();
    let mut type_parameters = Vec::new();
    let mut description = None;
    let mut base_class = None;
    let mut base_class_type_args = Vec::new();
    let mut fields = Vec::new();
    let mut methods = Vec::new();
    let mut constructor = None;
    let location = get_location(&pair);
    let ast_location = convert_to_ast_location(&location);

    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::identifier => name = item.as_str().to_string(),
            Rule::type_parameters => {
                for param in item.into_inner() {
                    if param.as_rule() == Rule::type_parameter {
                        type_parameters.push(param.as_str().to_string());
                    }
                }
            },
            Rule::type_ => {
                // This is the base class in "is type_"
                let type_result = parse_type(item)?;
                match type_result {
                    crate::ast::Type::Object(class_name) => {
                        base_class = Some(class_name);
                    },
                    crate::ast::Type::Generic(boxed_type, type_args) => {
                        if let crate::ast::Type::Object(class_name) = *boxed_type {
                            base_class = Some(class_name);
                            base_class_type_args = type_args;
                        } else {
                            return Err(CompilerError::parse_error(
                                "Base class must be a class type".to_string(),
                                Some(ast_location.clone()),
                                Some("Use a valid class name for inheritance".to_string())
                            ));
                        }
                    },
                    crate::ast::Type::TypeParameter(class_name) => {
                        // Handle simple class names that are parsed as type parameters
                        base_class = Some(class_name);
                    },
                    _ => {
                        return Err(CompilerError::parse_error(
                            "Base class must be a class type".to_string(),
                            Some(ast_location.clone()),
                            Some("Use a valid class name for inheritance".to_string())
                        ));
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
                                if param_decl.as_rule() == Rule::input_declaration {
                                    // Convert type declarations to class fields
                                    let field = parse_field_from_type_decl(param_decl, ast_location.clone())?;
                                    fields.push(field);
                                }
                            }
                        },
                        _ => {}
                    }
                }
            },
            Rule::constructor => {
                constructor = Some(parse_constructor(item, ast_location.clone())?);
            },
            Rule::functions_block => {
                let class_methods = parse_functions_block(item)?;
                methods.extend(class_methods);
            },
            _ => {}
        }
    }

    Ok(Class {
        name,
        type_parameters,
        description,
        base_class,
        base_class_type_args,
        fields,
        methods,
        constructor,
        location: Some(ast_location),
    })
}

fn parse_field_from_type_decl(pair: Pair<Rule>, location: crate::ast::SourceLocation) -> Result<Field, CompilerError> {
    let mut parts = pair.into_inner();
    
    let type_part = parts.next().ok_or_else(|| CompilerError::parse_error(
        "Field missing type".to_string(),
        Some(location.clone()),
        Some("Fields must have a type".to_string())
    ))?;
    
    let name_part = parts.next().ok_or_else(|| CompilerError::parse_error(
        "Field missing name".to_string(),
        Some(location.clone()),
        Some("Fields must have a name".to_string())
    ))?;
    
    if name_part.as_rule() != Rule::identifier {
        return Err(CompilerError::parse_error(
            "Expected identifier for field name".to_string(),
            Some(location.clone()),
            Some("Fields must have valid identifiers".to_string())
        ));
    }
    
    let name = name_part.as_str().to_string();
    let type_ = parse_type(type_part)?;
    
    Ok(Field {
        name,
        type_,
        visibility: Visibility::Public, // Default visibility for class fields from setup block
        is_static: false,
        default_value: None, // No default value for fields from setup block
    })
}

fn parse_constructor(pair: Pair<Rule>, location: crate::ast::SourceLocation) -> Result<Constructor, CompilerError> {
    let mut parameters = Vec::new();
    let mut body = Vec::new();

    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::constructor_parameter_list => {
                for param in item.into_inner() {
                    if param.as_rule() == Rule::constructor_parameter {
                        parameters.push(parse_constructor_parameter(param)?);
                    }
                }
            },
            Rule::setup_block => {
                for setup_item in item.into_inner() {
                    if setup_item.as_rule() == Rule::input_block {
                        for param_decl in setup_item.into_inner() {
                            if param_decl.as_rule() == Rule::input_declaration {
                                parameters.push(parse_parameter(param_decl)?);
                            }
                        }
                    }
                }
            },
            Rule::indented_block => {
                for stmt in item.into_inner() {
                    if stmt.as_rule() == Rule::statement {
                        body.push(parse_statement(stmt)?);
                    }
                }
            },
            _ => {}
        }
    }

    Ok(Constructor {
        parameters,
        body,
        location: Some(location),
    })
}

fn parse_constructor_parameter(pair: Pair<Rule>) -> Result<Parameter, CompilerError> {
    let mut parts = pair.into_inner();
    
    // Check if we have a type or just an identifier
    let first_part = parts.next().ok_or_else(|| CompilerError::parse_error(
        "Constructor parameter missing identifier".to_string(),
        None,
        Some("Constructor parameters must have an identifier".to_string())
    ))?;
    
    if let Some(second_part) = parts.next() {
        // We have both type and identifier
        let type_ = parse_type(first_part)?;
        if second_part.as_rule() != Rule::identifier {
            return Err(CompilerError::parse_error(
                "Expected identifier for parameter name".to_string(),
                None,
                Some("Parameters must have valid identifiers".to_string())
            ));
        }
        let name = second_part.as_str().to_string();
        Ok(Parameter::new(name, type_))
    } else {
        // We have just an identifier, infer type as string for now
        if first_part.as_rule() != Rule::identifier {
            return Err(CompilerError::parse_error(
                "Expected identifier for parameter name".to_string(),
                None,
                Some("Parameters must have valid identifiers".to_string())
            ));
        }
        let name = first_part.as_str().to_string();
        let type_ = crate::ast::Type::String; // Default type for untyped constructor parameters
        Ok(Parameter::new(name, type_))
    }
}

fn parse_parameter(pair: Pair<Rule>) -> Result<Parameter, CompilerError> {
    let mut parts = pair.into_inner();
    
    let type_part = parts.next().ok_or_else(|| CompilerError::parse_error(
        "Parameter missing type".to_string(),
        None,
        Some("Parameters must have a type".to_string())
    ))?;
    
    let name_part = parts.next().ok_or_else(|| CompilerError::parse_error(
        "Parameter missing name".to_string(),
        None,
        Some("Parameters must have a name".to_string())
    ))?;
    
    if name_part.as_rule() != Rule::identifier {
        return Err(CompilerError::parse_error(
            "Expected identifier for parameter name".to_string(),
            None,
            Some("Parameters must have valid identifiers".to_string())
        ));
    }
    
    let name = name_part.as_str().to_string();
    let type_ = parse_type(type_part)?;
    
    Ok(Parameter::new(name, type_))
} 