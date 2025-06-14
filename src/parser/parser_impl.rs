use pest::{Parser, iterators::Pair};
use crate::ast::{Program, Function, Type, Parameter, FunctionSyntax, Visibility, Statement, Expression, Value, Class, Field, Constructor};
use crate::error::{CompilerError, ErrorUtils};
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
        match self.parse_internal(source) {
            Ok(program) => {
                if self.errors.is_empty() {
                    Ok(program)
                } else {
                    Err(self.errors.clone())
                }
            }
            Err(error) => {
                self.errors.push(error);
                Err(self.errors.clone())
            }
        }
    }

    fn parse_internal(&mut self, source: &str) -> Result<Program, CompilerError> {
        let trimmed_source = source.trim();
        let pairs = CleanParser::parse(Rule::program, trimmed_source)
            .map_err(|e| ErrorUtils::from_pest_error(e, source, &self.file_path))?;

        parse_program_ast(pairs)
    }
}

pub fn parse(source: &str) -> Result<Program, CompilerError> {
    let trimmed_source = source.trim();
    let pairs = CleanParser::parse(Rule::program, trimmed_source)
        .map_err(|e| ErrorUtils::from_pest_error(e, source, "<unknown>"))?;

    parse_program_ast(pairs)
}

pub fn parse_with_file(source: &str, file_path: &str) -> Result<Program, CompilerError> {
    let trimmed_source = source.trim();
    let pairs = CleanParser::parse(Rule::program, trimmed_source)
        .map_err(|e| ErrorUtils::from_pest_error(e, source, file_path))?;

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
                                    Rule::standalone_function => {
                                        let func = parse_standalone_function(program_item_inner)?;
                                        functions.push(func);
                                    },
                                    Rule::start_function => {
                                        let func = parse_start_function(program_item_inner)?;
                                        start_function = Some(func);
                                    },
                                    Rule::implicit_start_function => {
                                        let func = parse_start_function(program_item_inner)?;
                                        start_function = Some(func);
                                    },
                                    Rule::class_decl => {
                                        let class = parse_class_decl(program_item_inner)?;
                                        classes.push(class);
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

    // Start function must always have void return type for WebAssembly compatibility
    let return_type = Type::Void;

    Ok(Function {
        name,
        type_parameters: Vec::new(),
        type_constraints: Vec::new(),
        parameters: Vec::new(),
        return_type,
        body,
        description: None,
        syntax: FunctionSyntax::Simple,
        visibility: Visibility::Public,
        location,
    })
}

/// Parse a standalone function definition like: function Any identity() { ... }
pub fn parse_standalone_function(pair: Pair<Rule>) -> Result<Function, CompilerError> {
    let mut name = String::new();
    let mut type_parameters = Vec::new();
    let mut parameters = Vec::new();
    let mut return_type = Type::Void;
    let mut body = Vec::new();
    let mut description: Option<String> = None;
    let location = Some(convert_to_ast_location(&get_location(&pair)));

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::function_type => {
                // This is the return type that comes first in Clean Language syntax
                return_type = parse_type(inner)?;
                
                // If the return type is a type parameter (like "Any"), add it to type_parameters
                if let Type::TypeParameter(ref param_name) = return_type {
                    type_parameters.push(param_name.clone());
                }
            },
            Rule::identifier => {
                name = inner.as_str().to_string();
            },
            Rule::parameter_list => {
                for param in inner.into_inner() {
                    if param.as_rule() == Rule::parameter {
                        let parameter = parse_parameter(param)?;
                        
                        // If parameter type is a type parameter, add it to type_parameters if not already present
                        if let Type::TypeParameter(ref param_name) = parameter.type_ {
                            if !type_parameters.contains(param_name) {
                                type_parameters.push(param_name.clone());
                            }
                        }
                        
                        parameters.push(parameter);
                    }
                }
            },
            Rule::function_body => {
                // function_body = (setup_block ~ indented_block) | indented_block
                let mut found_body = false;
                for body_item in inner.into_inner() {
                    match body_item.as_rule() {
                        Rule::setup_block => {
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
                                        for param in &params {
                                            // If parameter type is a type parameter, add it to type_parameters if not already present
                                            if let Type::TypeParameter(ref param_name) = param.type_ {
                                                if !type_parameters.contains(param_name) {
                                                    type_parameters.push(param_name.clone());
                                                }
                                            }
                                        }
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
                        format!("Function '{}' is missing a body.", name),
                        location.clone(),
                        None,
                    ));
                }
            },
            _ => {}
        }
    }

    Ok(Function {
        name,
        type_parameters,
        type_constraints: Vec::new(),
        parameters,
        return_type,
        body,
        description,
        syntax: FunctionSyntax::Simple,
        visibility: Visibility::Public,
        location,
    })
}

/// Parse a parameter from a parameter list: type identifier
fn parse_parameter(pair: Pair<Rule>) -> Result<Parameter, CompilerError> {
    let mut param_type = None;
    let mut param_name = String::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::type_ => {
                param_type = Some(parse_type(inner)?);
            },
            Rule::identifier => {
                param_name = inner.as_str().to_string();
            },
            _ => {}
        }
    }

    let param_type = param_type.ok_or_else(|| CompilerError::parse_error(
        "Parameter missing type".to_string(),
        None,
        Some("Parameters must have a type".to_string())
    ))?;

    if param_name.is_empty() {
        return Err(CompilerError::parse_error(
            "Parameter missing name".to_string(),
            None,
            Some("Parameters must have a name".to_string())
        ));
    }

    Ok(Parameter::new(param_name, param_type))
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

/// Parse a class declaration
pub fn parse_class_decl(class_pair: Pair<Rule>) -> Result<Class, CompilerError> {
    let mut class_name = String::new();
    let mut type_parameters = Vec::new();
    let mut base_class = None;
    let mut base_class_type_args = Vec::new();
    let mut fields = Vec::new();
    let mut methods = Vec::new();
    let mut constructor = None;
    let location = Some(convert_to_ast_location(&get_location(&class_pair)));

    for item in class_pair.into_inner() {
        match item.as_rule() {
            Rule::identifier => {
                class_name = item.as_str().to_string();
            },
            Rule::type_ => {
                // Parse "is BaseClass" or "is BaseClass<Args>" inheritance
                let type_result = parse_type(item)?;
                match type_result {
                    Type::Object(class_name) => {
                        base_class = Some(class_name);
                    },
                    Type::Generic(boxed_type, type_args) => {
                        if let Type::Object(class_name) = *boxed_type {
                            base_class = Some(class_name);
                            base_class_type_args = type_args;
                        }
                    },
                    Type::TypeParameter(class_name) => {
                        // Handle simple class names that are parsed as type parameters
                        base_class = Some(class_name);
                    },
                    _ => {
                        return Err(CompilerError::parse_error(
                            "Base class must be a class type".to_string(),
                            location.clone(),
                            Some("Use a valid class name for inheritance".to_string())
                        ));
                    }
                }
            },
            Rule::indented_class_body => {
                // Parse the class body containing field declarations, constructors, methods
                for body_item in item.into_inner() {
                    match body_item.as_rule() {
                        Rule::class_body_item => {
                            for class_item in body_item.into_inner() {
                                match class_item.as_rule() {
                                    Rule::class_field => {
                                        let field = parse_class_field(class_item)?;
                                        
                                        // If field type is a type parameter, add it to type_parameters if not already present
                                        if let Type::TypeParameter(ref param_name) = field.type_ {
                                            if !type_parameters.contains(param_name) {
                                                type_parameters.push(param_name.clone());
                                            }
                                        }
                                        
                                        fields.push(field);
                                    },
                                    Rule::constructor => {
                                        constructor = Some(parse_constructor(class_item)?);
                                    },
                                    Rule::functions_block => {
                                        let block_methods = parse_functions_block(class_item)?;
                                        methods.extend(block_methods);
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

    if class_name.is_empty() {
        return Err(CompilerError::parse_error(
            "Class is missing a name".to_string(),
            location.clone(),
            None,
        ));
    }

    Ok(Class {
        name: class_name,
        type_parameters,
        description: None,
        base_class,
        base_class_type_args,
        fields,
        methods,
        constructor,
        location,
    })
}

/// Parse a field declaration within a class
fn parse_class_field(field_pair: Pair<Rule>) -> Result<Field, CompilerError> {
    let mut field_name = String::new();
    let mut field_type = None;

    for item in field_pair.into_inner() {
        match item.as_rule() {
            Rule::type_ => {
                field_type = Some(parse_type(item)?);
            },
            Rule::identifier => {
                field_name = item.as_str().to_string();
            },
            _ => {}
        }
    }

    if field_name.is_empty() {
        return Err(CompilerError::parse_error(
            "Field is missing a name".to_string(),
            None,
            None,
        ));
    }

    let field_type = field_type.ok_or_else(|| {
        CompilerError::parse_error(
            "Field is missing a type".to_string(),
            None,
            None,
        )
    })?;

    Ok(Field {
        name: field_name,
        type_: field_type,
        visibility: Visibility::Public,
        is_static: false,
    })
}

/// Parse a constructor within a class
fn parse_constructor(constructor_pair: Pair<Rule>) -> Result<Constructor, CompilerError> {
    let mut parameters = Vec::new();
    let mut body = Vec::new();
    let location = Some(convert_to_ast_location(&get_location(&constructor_pair)));

    for item in constructor_pair.into_inner() {
        match item.as_rule() {
            Rule::constructor_parameter_list => {
                for param_item in item.into_inner() {
                    if param_item.as_rule() == Rule::constructor_parameter {
                        let param = parse_constructor_parameter(param_item)?;
                        parameters.push(param);
                    }
                }
            },
            Rule::indented_block => {
                for stmt_pair in item.into_inner() {
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

    Ok(Constructor::new(parameters, body, location))
}

/// Parse a constructor parameter
fn parse_constructor_parameter(param_pair: Pair<Rule>) -> Result<Parameter, CompilerError> {
    let mut param_name = String::new();
    let mut param_type = None;

    for item in param_pair.into_inner() {
        match item.as_rule() {
            Rule::constructor_type => {
                param_type = Some(parse_type(item)?);
            },
            Rule::identifier => {
                param_name = item.as_str().to_string();
            },
            _ => {}
        }
    }

    if param_name.is_empty() {
        return Err(CompilerError::parse_error(
            "Constructor parameter is missing a name".to_string(),
            None,
            None,
        ));
    }

    // If no type is specified, we'll infer it from class fields later in semantic analysis
    // For now, mark it as unresolved with a special marker
    let param_type = param_type.unwrap_or(Type::Any);

    Ok(Parameter::new(param_name, param_type))
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
    let mut type_parameters = Vec::new();
    let mut parameters = Vec::new();
    let mut body = Vec::new();
    let mut description: Option<String> = None;
    let location = Some(convert_to_ast_location(&get_location(&func_pair)));

    // Parse the function signature and body
    for item in func_pair.into_inner() {
        match item.as_rule() {
            Rule::function_type => {
                let parsed_type = parse_type(item)?;
                return_type = Some(parsed_type.clone());
                
                // If the return type is a type parameter (like "Any"), add it to type_parameters
                if let Type::TypeParameter(ref param_name) = parsed_type {
                    type_parameters.push(param_name.clone());
                }
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
                                        for param in &params {
                                            // If parameter type is a type parameter, add it to type_parameters if not already present
                                            if let Type::TypeParameter(ref param_name) = param.type_ {
                                                if !type_parameters.contains(param_name) {
                                                    type_parameters.push(param_name.clone());
                                                }
                                            }
                                        }
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
        type_parameters,
        type_constraints: Vec::new(),
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