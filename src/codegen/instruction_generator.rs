//! Module for generating WebAssembly instructions.

use wasm_encoder::{Instruction, BlockType, MemArg, ValType};
use crate::ast::{self, Expression, BinaryOperator, Value, Statement, SourceLocation, StringPart};
use crate::types::WasmType;
use crate::error::CompilerError;

// Removed unused import DEFAULT_ALIGN
use super::type_manager::TypeManager;

/// Represents a local variable in a function
#[derive(Debug, Clone)]
pub struct LocalVarInfo {
    pub index: u32,
    pub type_: wasm_encoder::ValType,
}

/// Define a simple FuncType struct for our purposes
#[derive(Clone)]
pub struct FuncType {
    params: Vec<ValType>,
    results: Vec<ValType>,
}

impl FuncType {
    pub fn new(params: Vec<ValType>, results: Vec<ValType>) -> Self {
        Self { params, results }
    }
    
    pub fn params(&self) -> &[ValType] {
        &self.params
    }
    
    pub fn results(&self) -> &[ValType] {
        &self.results
    }
}

/// Generates WebAssembly instructions for various language constructs
pub(crate) struct InstructionGenerator {
    type_manager: TypeManager,
    variable_map: std::collections::HashMap<String, LocalVarInfo>,
    current_locals: Vec<LocalVarInfo>,
    function_map: std::collections::HashMap<String, u32>, // Simple name -> primary index mapping
    function_signatures: std::collections::HashMap<String, u32>, // signature -> index mapping
    function_types: std::collections::HashMap<u32, FuncType>,
}

impl InstructionGenerator {
    /// Create a new instruction generator
    pub(crate) fn new(type_manager: TypeManager) -> Self {
        Self {
            type_manager,
            variable_map: std::collections::HashMap::new(),
            current_locals: Vec::new(),
            function_map: std::collections::HashMap::new(),
            function_signatures: std::collections::HashMap::new(),
            function_types: std::collections::HashMap::new(),
        }
    }
    
    /// Add a function mapping from name to index
    pub(crate) fn add_function_mapping(&mut self, name: &str, index: u32) {
        self.function_map.insert(name.to_string(), index);
    }
    
    /// Reset locals for a new function
    pub(crate) fn reset_locals(&mut self) {
        self.current_locals.clear();
        self.variable_map.clear();
    }
    
    /// Get the current locals
    pub(crate) fn get_current_locals(&self) -> &Vec<LocalVarInfo> {
        &self.current_locals
    }
    
    /// Get a function index by name (returns the first/primary overload)
    pub(crate) fn get_function_index(&self, name: &str) -> Option<u32> {
        self.function_map.get(name).copied()
    }
    
    /// Get a function index by signature (for overload resolution)
    pub(crate) fn get_function_index_by_signature(&self, name: &str, param_types: &[WasmType]) -> Option<u32> {
        let signature = self.create_function_signature(name, param_types);
        self.function_signatures.get(&signature).copied()
    }
    
    /// Create a function signature string for overload resolution
    fn create_function_signature(&self, name: &str, param_types: &[WasmType]) -> String {
        let param_str = param_types.iter()
            .map(|t| format!("{:?}", t))
            .collect::<Vec<_>>()
            .join(",");
        format!("{}({})", name, param_str)
    }
    
    /// Determine the type of an expression without generating instructions
    pub(crate) fn get_expression_type(&self, expr: &Expression) -> Result<WasmType, CompilerError> {
        match expr {
            Expression::Literal(value) => {
                match value {
                    ast::Value::Integer(_) => Ok(WasmType::I32),
                    ast::Value::Number(_) => Ok(WasmType::F64),
                    ast::Value::String(_) => Ok(WasmType::I32), // String pointer
                    ast::Value::Boolean(_) => Ok(WasmType::I32),
                    _ => Ok(WasmType::I32), // Default
                }
            },
            Expression::Variable(name) => {
                // Look up variable type from local variables
                if let Some(local_info) = self.find_local(name) {
                    let wasm_type = match local_info.type_ {
                        wasm_encoder::ValType::I32 => WasmType::I32,
                        wasm_encoder::ValType::I64 => WasmType::I64,
                        wasm_encoder::ValType::F32 => WasmType::F32,
                        wasm_encoder::ValType::F64 => WasmType::F64,
                        wasm_encoder::ValType::V128 => WasmType::V128,
                        _ => WasmType::I32,
                    };
                    eprintln!("DEBUG: Variable '{}' has type {:?}", name, wasm_type);
                    Ok(wasm_type)
                } else {
                    eprintln!("DEBUG: Variable '{}' not found in locals, defaulting to I32", name);
                    // Default to I32 if we can't determine the type
                    Ok(WasmType::I32)
                }
            },
            Expression::Call(func_name, _args) => {
                // Get function return type by looking up the function
                if let Some(func_index) = self.get_function_index(func_name) {
                    self.get_function_return_type(func_index)
                } else {
                    Ok(WasmType::I32) // Default
                }
            },
            Expression::Binary(_, op, _) => {
                // Most binary operations return the same type as their operands
                // For simplicity, assume numeric operations
                match op {
                    ast::BinaryOperator::Equal | ast::BinaryOperator::NotEqual |
                    ast::BinaryOperator::Less | ast::BinaryOperator::Greater |
                    ast::BinaryOperator::LessEqual | ast::BinaryOperator::GreaterEqual => {
                        Ok(WasmType::I32) // Boolean result
                    },
                    _ => Ok(WasmType::F64), // Most math operations use F64
                }
            },
            _ => Ok(WasmType::I32), // Default for other expression types
        }
    }
    
    /// Find a local variable by name
    pub(crate) fn find_local(&self, name: &str) -> Option<&LocalVarInfo> {
        self.variable_map.get(name)
    }
    
    /// Add a parameter to the list of locals
    pub(crate) fn add_parameter(&mut self, name: &str, wasm_type: WasmType) {
        let local_info = LocalVarInfo {
            index: self.current_locals.len() as u32,
            type_: wasm_type.into(),
        };
        self.current_locals.push(local_info.clone());
        self.variable_map.insert(name.to_string(), local_info);
    }

    /// Generate instructions for a binary operation
    pub(crate) fn generate_binary_operation(
        &mut self,
        left: &Expression,
        op: &BinaryOperator,
        right: &Expression,
        instructions: &mut Vec<Instruction>
    ) -> Result<WasmType, CompilerError> {
        let left_type = self.generate_expression(left, instructions)?;
        let right_type = self.generate_expression(right, instructions)?;
        
        // Handle Any type conversions
        // Note: For now, we'll skip Any type checking as it requires more complex type analysis
        // This would need to be implemented with proper type inference
        
        match (left_type, right_type) {
            // Handle string operations first (with guard condition)
            (WasmType::I32, WasmType::I32) if self.type_manager.is_string_type(left) || self.type_manager.is_string_type(right) => {
                match op {
                    ast::BinaryOperator::Add => { 
                        if let Some(string_concat_index) = self.get_function_index("string_concat") {
                            instructions.push(Instruction::Call(string_concat_index)); 
                            Ok(WasmType::I32) 
                        } else {
                            Err(CompilerError::codegen_error("String concatenation function not found", None, None))
                        }
                    },
                                        ast::BinaryOperator::Equal | ast::BinaryOperator::NotEqual | 
                    ast::BinaryOperator::Less | ast::BinaryOperator::Greater | 
                    ast::BinaryOperator::LessEqual | ast::BinaryOperator::GreaterEqual => {
                        if let Some(string_compare_index) = self.get_function_index("string_compare") {
                            instructions.push(Instruction::Call(string_compare_index));
                            match op {
                                ast::BinaryOperator::Equal => instructions.push(Instruction::I32Eqz),
                                ast::BinaryOperator::NotEqual => { 
                                    instructions.push(Instruction::I32Const(0)); 
                                    instructions.push(Instruction::I32Eq); 
                                },
                                ast::BinaryOperator::Less => { 
                                    instructions.push(Instruction::I32Const(0)); 
                                    instructions.push(Instruction::I32LtS); 
                                },
                                ast::BinaryOperator::Greater => { 
                                    instructions.push(Instruction::I32Const(0)); 
                                    instructions.push(Instruction::I32GtS); 
                                },
                                ast::BinaryOperator::LessEqual => { 
                                    instructions.push(Instruction::I32Const(0)); 
                                    instructions.push(Instruction::I32LeS); 
                                },
                                ast::BinaryOperator::GreaterEqual => { 
                                    instructions.push(Instruction::I32Const(0)); 
                                    instructions.push(Instruction::I32GeS); 
                                },
                                _ => {
                                    return Err(CompilerError::codegen_error("Invalid comparison operator for strings", None, None));
                                }
                            }
                            Ok(WasmType::I32)
                        } else {
                            Err(CompilerError::codegen_error("String comparison function not found", None, None))
                        }
                    },
                    _ => Err(CompilerError::type_error(
                        format!("Unsupported string binary operator: {:?}", op), None, None
                    )),
                }
            },
            // Handle regular I32 operations
            (WasmType::I32, WasmType::I32) => {
                match op {
                    ast::BinaryOperator::Add => { instructions.push(Instruction::I32Add); Ok(WasmType::I32) },
                    ast::BinaryOperator::Subtract => { instructions.push(Instruction::I32Sub); Ok(WasmType::I32) },
                    ast::BinaryOperator::Multiply => { instructions.push(Instruction::I32Mul); Ok(WasmType::I32) },
                    ast::BinaryOperator::Divide => { instructions.push(Instruction::I32DivS); Ok(WasmType::I32) },
                    ast::BinaryOperator::Equal => { instructions.push(Instruction::I32Eq); Ok(WasmType::I32) }, 
                    ast::BinaryOperator::NotEqual => { instructions.push(Instruction::I32Ne); Ok(WasmType::I32) }, 
                    ast::BinaryOperator::Less => { instructions.push(Instruction::I32LtS); Ok(WasmType::I32) }, 
                    ast::BinaryOperator::Greater => { instructions.push(Instruction::I32GtS); Ok(WasmType::I32) }, 
                    ast::BinaryOperator::LessEqual => { instructions.push(Instruction::I32LeS); Ok(WasmType::I32) }, 
                    ast::BinaryOperator::GreaterEqual => { instructions.push(Instruction::I32GeS); Ok(WasmType::I32) }, 
                    _ => Err(CompilerError::type_error(
                        format!("Unsupported I32 binary operator: {:?}", op), None, None
                    )),
                }
            },
            // Handle F64 operations
            (WasmType::F64, WasmType::F64) => {
                match op {
                    ast::BinaryOperator::Add => { instructions.push(Instruction::F64Add); Ok(WasmType::F64) },
                    ast::BinaryOperator::Subtract => { instructions.push(Instruction::F64Sub); Ok(WasmType::F64) },
                    ast::BinaryOperator::Multiply => { instructions.push(Instruction::F64Mul); Ok(WasmType::F64) },
                    ast::BinaryOperator::Divide => { instructions.push(Instruction::F64Div); Ok(WasmType::F64) },
                    ast::BinaryOperator::Equal => { instructions.push(Instruction::F64Eq); Ok(WasmType::I32) }, 
                    ast::BinaryOperator::NotEqual => { instructions.push(Instruction::F64Ne); Ok(WasmType::I32) }, 
                    ast::BinaryOperator::Less => { instructions.push(Instruction::F64Lt); Ok(WasmType::I32) }, 
                    ast::BinaryOperator::Greater => { instructions.push(Instruction::F64Gt); Ok(WasmType::I32) }, 
                    ast::BinaryOperator::LessEqual => { instructions.push(Instruction::F64Le); Ok(WasmType::I32) }, 
                    ast::BinaryOperator::GreaterEqual => { instructions.push(Instruction::F64Ge); Ok(WasmType::I32) }, 
                    _ => Err(CompilerError::type_error(
                        format!("Unsupported F64 binary operator: {:?}", op), None, None
                    ))
                }
            },
            (WasmType::I32, WasmType::F64) => {
                // Convert I32 to F64 and perform F64 operation
                instructions.insert(instructions.len() - 2, Instruction::F64ConvertI32S);
                match op {
                    ast::BinaryOperator::Add => { instructions.push(Instruction::F64Add); Ok(WasmType::F64) },
                    ast::BinaryOperator::Subtract => { instructions.push(Instruction::F64Sub); Ok(WasmType::F64) },
                    ast::BinaryOperator::Multiply => { instructions.push(Instruction::F64Mul); Ok(WasmType::F64) },
                    ast::BinaryOperator::Divide => { instructions.push(Instruction::F64Div); Ok(WasmType::F64) },
                    ast::BinaryOperator::Equal => { instructions.push(Instruction::F64Eq); Ok(WasmType::I32) },
                    ast::BinaryOperator::NotEqual => { instructions.push(Instruction::F64Ne); Ok(WasmType::I32) },
                    ast::BinaryOperator::Less => { instructions.push(Instruction::F64Lt); Ok(WasmType::I32) },
                    ast::BinaryOperator::Greater => { instructions.push(Instruction::F64Gt); Ok(WasmType::I32) },
                    ast::BinaryOperator::LessEqual => { instructions.push(Instruction::F64Le); Ok(WasmType::I32) },
                    ast::BinaryOperator::GreaterEqual => { instructions.push(Instruction::F64Ge); Ok(WasmType::I32) },
                    _ => Err(CompilerError::type_error(
                        format!("Unsupported mixed I32/F64 binary operator: {:?}", op), None, None
                    ))
                }
            },
            (WasmType::F64, WasmType::I32) => {
                // Convert I32 to F64 and perform F64 operation
                instructions.push(Instruction::F64ConvertI32S);
                match op {
                    ast::BinaryOperator::Add => { instructions.push(Instruction::F64Add); Ok(WasmType::F64) },
                    ast::BinaryOperator::Subtract => { instructions.push(Instruction::F64Sub); Ok(WasmType::F64) },
                    ast::BinaryOperator::Multiply => { instructions.push(Instruction::F64Mul); Ok(WasmType::F64) },
                    ast::BinaryOperator::Divide => { instructions.push(Instruction::F64Div); Ok(WasmType::F64) },
                    ast::BinaryOperator::Equal => { instructions.push(Instruction::F64Eq); Ok(WasmType::I32) },
                    ast::BinaryOperator::NotEqual => { instructions.push(Instruction::F64Ne); Ok(WasmType::I32) },
                    ast::BinaryOperator::Less => { instructions.push(Instruction::F64Lt); Ok(WasmType::I32) },
                    ast::BinaryOperator::Greater => { instructions.push(Instruction::F64Gt); Ok(WasmType::I32) },
                    ast::BinaryOperator::LessEqual => { instructions.push(Instruction::F64Le); Ok(WasmType::I32) },
                    ast::BinaryOperator::GreaterEqual => { instructions.push(Instruction::F64Ge); Ok(WasmType::I32) },
                    _ => Err(CompilerError::type_error(
                        format!("Unsupported mixed F64/I32 binary operator: {:?}", op), None, None
                    ))
                }
            },
            _ => Err(CompilerError::type_error(
                format!("Type mismatch: Cannot apply {:?} to {:?} and {:?}", op, left_type, right_type),
                None, None
            )),
        }
    }

    /// Generate instructions for a statement
    pub(crate) fn generate_statement(&mut self, stmt: &Statement, instructions: &mut Vec<Instruction>) -> Result<(), CompilerError> {
        match stmt {
            Statement::VariableDecl { name, type_, initializer, location: _ } => {
                let specified_type = Some(self.type_manager.ast_type_to_wasm_type(type_)?);
                
                let (var_type, init_instructions) = if let Some(init_expr) = initializer {
                    let mut init_instr = Vec::new();
                    let init_type = self.generate_expression(init_expr, &mut init_instr)?;
                    (init_type, Some(init_instr))
                } else {
                    if specified_type.is_none() {
                        return Err(CompilerError::codegen_error(
                            "Variable declaration without initializer must have a type annotation",
                            None, None
                        ));
                    }
                    (specified_type.unwrap(), None)
                };

                if let (Some(st), declared_type) = (specified_type, var_type) {
                    if st != declared_type {
                        return Err(CompilerError::type_error(
                            format!("Initializer type {:?} does not match specified type {:?} for variable '{}'", 
                                declared_type, st, name),
                            None, None
                        ));
                    }
                }

                let local_info = LocalVarInfo {
                    index: self.current_locals.len() as u32,
                    type_: var_type.into(),
                };
                self.current_locals.push(local_info.clone()); 
                self.variable_map.insert(name.clone(), local_info.clone()); 
                
                if let Some(init_instr) = init_instructions {
                    instructions.extend(init_instr);
                    instructions.push(Instruction::LocalSet(local_info.index));
                } else {
                    match var_type {
                        WasmType::I32 => instructions.push(Instruction::I32Const(0)),
                        WasmType::I64 => instructions.push(Instruction::I64Const(0)),
                        WasmType::F32 => instructions.push(Instruction::F32Const(0.0)),
                        WasmType::F64 => instructions.push(Instruction::F64Const(0.0)),
                        _ => return Err(CompilerError::codegen_error(
                            format!("Cannot determine default value for type {:?}", var_type), 
                            None, None
                        ))
                    }
                    instructions.push(Instruction::LocalSet(local_info.index));
                }
                
                return Ok(());
            },
            Statement::Assignment { target, value, location: _ } => {
                // First find the local variable
                if let Some(local_info) = self.find_local(target) {
                    let local_index = local_info.index;
                    
                    // Generate instructions for the value
                    self.generate_expression(value, instructions)?;
                    
                    // Set the local variable
                    instructions.push(Instruction::LocalSet(local_index));
                } else {
                    return Err(CompilerError::codegen_error(
                        format!("Cannot assign to unknown variable: {}", target),
                        None, None
                    ));
                }
            },
            Statement::Print { expression, newline, location: _ } => {
                // For print functions, we need to handle them specially since they expect (ptr, len)
                // but the old implementation was just generating the expression and calling print
                // This is causing stack mismatches. For now, we'll generate a placeholder.
                
                // Generate the expression to get its value
                self.generate_expression(expression, instructions)?;
                
                // For string literals, we need to convert to (ptr, len) format
                // For now, we'll drop the value and generate a placeholder string
                instructions.push(Instruction::Drop);
                
                // Generate placeholder string data
                instructions.push(Instruction::I32Const(0)); // ptr placeholder
                instructions.push(Instruction::I32Const(0)); // len placeholder
                
                let function_name = if *newline { "printl" } else { "print" };
                if let Some(print_function_index) = self.get_function_index(function_name) {
                    instructions.push(Instruction::Call(print_function_index));
                } else {
                    return Err(CompilerError::codegen_error(
                        format!("{} function not found", function_name),
                        None,
                        None
                    ));
                }
            },
            Statement::Return { value, location: _ } => {
                if let Some(expr) = value {
                    self.generate_expression(expr, instructions)?;
                }
                instructions.push(Instruction::Return);
            },
            Statement::If { condition, then_branch, else_branch, location: _ } => {
                self.generate_expression(condition, instructions)?;
                
                instructions.push(Instruction::I32Eqz);
                instructions.push(Instruction::BrIf(1));
                
                instructions.push(Instruction::Block(BlockType::Empty));
                
                for stmt in then_branch {
                    self.generate_statement(stmt, instructions)?;
                }
                
                if let Some(else_branch) = else_branch {
                    instructions.push(Instruction::Br(1));
                    instructions.push(Instruction::End);
                    instructions.push(Instruction::Block(BlockType::Empty));
                    
                    for stmt in else_branch {
                        self.generate_statement(stmt, instructions)?;
                    }
                }
                
                instructions.push(Instruction::End);
                
            },
            Statement::Iterate { iterator, collection, body, location: _ } => {
                self.generate_expression(collection, instructions)?;
                
                let array_ptr_index = self.current_locals.len() as u32;
                self.current_locals.push(LocalVarInfo {
                    index: array_ptr_index,
                    type_: wasm_encoder::ValType::I32,
                });
                instructions.push(Instruction::LocalSet(array_ptr_index));
                
                let counter_index = self.current_locals.len() as u32;
                self.current_locals.push(LocalVarInfo {
                    index: counter_index,
                    type_: wasm_encoder::ValType::I32,
                });
                
                let iterator_index = self.current_locals.len() as u32;
                self.current_locals.push(LocalVarInfo {
                    index: iterator_index,
                    type_: wasm_encoder::ValType::I32,
                });
                
                self.variable_map.insert(iterator.clone(), LocalVarInfo {
                    index: iterator_index,
                    type_: wasm_encoder::ValType::I32,
                });
                
                instructions.push(Instruction::LocalGet(array_ptr_index));
                
                if let Some(array_length_index) = self.get_function_index("array_length") {
                    instructions.push(Instruction::Call(array_length_index));
                    
                    let length_index = self.current_locals.len() as u32;
                    self.current_locals.push(LocalVarInfo {
                        index: length_index,
                        type_: wasm_encoder::ValType::I32,
                    });
                    instructions.push(Instruction::LocalSet(length_index));
                    
                    instructions.push(Instruction::I32Const(0));
                    instructions.push(Instruction::LocalSet(counter_index));
                    
                    instructions.push(Instruction::Block(BlockType::Empty));
                    instructions.push(Instruction::Loop(BlockType::Empty));
                    
                    instructions.push(Instruction::LocalGet(counter_index));
                    instructions.push(Instruction::LocalGet(length_index));
                    instructions.push(Instruction::I32LtU);
                    
                    instructions.push(Instruction::I32Eqz);
                    instructions.push(Instruction::BrIf(1));
                    
                    instructions.push(Instruction::LocalGet(array_ptr_index));
                    instructions.push(Instruction::LocalGet(counter_index));
                    
                    if let Some(array_get_index) = self.get_function_index("array_get") {
                        instructions.push(Instruction::Call(array_get_index));
                        
                        instructions.push(Instruction::I32Load(MemArg {
                            offset: 0,
                            align: 4, // 4-byte alignment for i32
                            memory_index: 0,
                        }));
                        instructions.push(Instruction::LocalSet(iterator_index));
                        
                        for stmt in body {
                            self.generate_statement(stmt, instructions)?;
                        }
                        
                        instructions.push(Instruction::LocalGet(counter_index));
                        instructions.push(Instruction::I32Const(1));
                        instructions.push(Instruction::I32Add);
                        instructions.push(Instruction::LocalSet(counter_index));
                        
                        instructions.push(Instruction::Br(0));
                        
                        instructions.push(Instruction::End);
                        instructions.push(Instruction::End);
                        
                        self.variable_map.remove(iterator);
                    } else {
                        return Err(CompilerError::codegen_error("array_get function not found", None, None));
                    }
                } else {
                    return Err(CompilerError::codegen_error("array_length function not found", None, None));
                }
            },
            Statement::Expression { expr, location: _ } => {
                let result_type = self.generate_expression(expr, instructions)?;
                // Only drop if the expression actually produces a value
                if result_type != WasmType::Unit {
                    instructions.push(Instruction::Drop);
                }
            },
            Statement::Test { name: _, body: _body, location: _ } => {
                #[cfg(test)]
                for stmt in _body {
                    self.generate_statement(stmt, instructions)?;
                }
            },
            _ => return Err(CompilerError::codegen_error(
                "Unsupported statement type", None, None
            )),
        }
        
        Ok(())
    }

    /// Generate instructions for an expression
    pub(crate) fn generate_expression(&mut self, expr: &Expression, instructions: &mut Vec<Instruction>) -> Result<WasmType, CompilerError> {
        // Extract location if available, or use None
        let loc = match expr {
            Expression::Binary(_, _, _) => None, // Binary has no location field
            // Add other expression variants with locations as needed
            _ => None,
        };

        match expr {
            Expression::Literal(value) => { 
                self.generate_value(value, instructions)
            },
            Expression::Variable(name) => {
                let local = self.find_local(name)
                    .ok_or_else(|| CompilerError::codegen_error(
                        &format!("Undefined variable '{}'", name),
                        Some("Check if the variable is defined".to_string()),
                        None
                    ))?;
                instructions.push(Instruction::LocalGet(local.index));
                Ok(Self::from_parser_val_type(local.type_))
            },
            Expression::Binary(left, op, right) => {
                let left_type = self.generate_expression(left, instructions)?;
                let right_type = self.generate_expression(right, instructions)?;
                
                if left_type != right_type {
                    return Err(CompilerError::codegen_error(
                        &format!("Type mismatch in binary operation: {:?} and {:?}", left_type, right_type),
                        Some("Use operands of the same type".to_string()),
                        None
                    ));
                }
                
                match (left_type, op) {
                    (WasmType::I32, BinaryOperator::Add) => {
                        instructions.push(Instruction::I32Add);
                        return Ok(WasmType::I32);
                    },
                    // Add more operators as needed
                    _ => {
                        return Err(CompilerError::codegen_error(
                            &format!("Unsupported binary operation: {:?} for type {:?}", op, left_type),
                            Some("Check operand types".to_string()),
                            None
                        ));
                    }
                }
            },
            Expression::Call(func_name, args) => { 
                // First, determine argument types for signature-based function resolution
                let mut arg_types = Vec::new();
                for arg in args {
                    let arg_type = self.get_expression_type(arg)?;
                    arg_types.push(arg_type);
                }
                
                // Try signature-based function resolution first
                eprintln!("DEBUG: Function call '{}' with arg types: {:?}", func_name, arg_types);
                let func_index = if let Some(index) = self.get_function_index_by_signature(func_name, &arg_types) {
                    eprintln!("DEBUG: Found signature-based match: function[{}]", index);
                    Some(index)
                } else {
                    eprintln!("DEBUG: No signature match, trying name-based lookup");
                    // Fall back to name-based resolution for backwards compatibility
                    if let Some(index) = self.get_function_index(func_name) {
                        eprintln!("DEBUG: Found name-based match: function[{}]", index);
                        Some(index)
                    } else {
                        eprintln!("DEBUG: No function found for '{}'", func_name);
                        None
                    }
                };
                
                if let Some(func_index) = func_index {
                    // Generate argument instructions
                    for arg in args {
                        self.generate_expression(arg, instructions)?;
                    }
                    instructions.push(Instruction::Call(func_index));
                    
                    // Determine return type
                    if let Some(func_type) = self.get_function_type(func_index) {
                        if let Some(return_val_type) = func_type.results().first() {
                            Ok(Self::from_parser_val_type(*return_val_type))
                        } else {
                            Ok(WasmType::I32) // Default to I32 if no return type
                        }
                    } else {
                        Ok(WasmType::I32) // Default to I32 if type info not available
                    }
                } else {
                    Err(CompilerError::codegen_error(
                        format!("Function not found: {}", func_name), None, None
                    ))
                }
            },
            Expression::ArrayAccess(array, index) => {
                self.generate_array_access(array, index, instructions)
            },
            Expression::MatrixAccess(matrix, row, col) => {
                self.generate_expression(matrix, instructions)?;
                self.generate_expression(row, instructions)?;
                self.generate_expression(col, instructions)?;
                
                if let Some(matrix_get_index) = self.get_function_index("matrix_get") {
                    instructions.push(Instruction::Call(matrix_get_index)); 
                    Ok(WasmType::F64)
                } else {
                    Err(CompilerError::codegen_error("matrix_get function not found", None, loc.clone()))
                }
            },
            Expression::MethodCall { object, method, arguments, location: _ } => {
                // Handle matrix methods like transpose(), inverse(), etc.
                self.generate_expression(object, instructions)?;
                
                for arg in arguments {
                    self.generate_expression(arg, instructions)?;
                }
                
                if let Some(method_index) = self.get_function_index(&format!("matrix_{}", method)) {
                    instructions.push(Instruction::Call(method_index));
                    Ok(WasmType::I32) // Method calls return appropriate type
                } else {
                Err(CompilerError::codegen_error(
                        &format!("Method '{}' not found", method), None, None
                ))
                }
            },
            Expression::StringInterpolation(parts) => {
                self.generate_string_interpolation(parts, instructions)?;
                Ok(WasmType::I32) // String interpolation returns a string pointer
            },
            _ => {
                return Err(CompilerError::codegen_error(
                    &format!("Unsupported expression: {:?}", expr),
                    Some("This expression type is not yet implemented".to_string()),
                    None
                ));
            }
        }
    }

    /// Generate instructions for a value
    pub(crate) fn generate_value(&mut self, value: &Value, instructions: &mut Vec<Instruction>) -> Result<WasmType, CompilerError> {
        match value {
            Value::Number(f) => {
                instructions.push(Instruction::F64Const(*f));
                Ok(WasmType::F64)
            },
            Value::Integer(i) => {
                instructions.push(Instruction::I32Const((*i).try_into().unwrap()));
                Ok(WasmType::I32)
            },
            Value::String(_s) => {
                // This should use memory.allocate_string
                // For now, just return a placeholder pointing to "empty string"
                instructions.push(Instruction::I32Const(0));
                Ok(WasmType::I32)
            },
            Value::Boolean(b) => {
                instructions.push(Instruction::I32Const(if *b { 1 } else { 0 }));
                Ok(WasmType::I32)
            },
            Value::List(_) => {
                // This should use memory.allocate_array
                // For now, just return a placeholder pointing to "empty array"
                instructions.push(Instruction::I32Const(0));
                Ok(WasmType::I32)
            },
            Value::Matrix(_) => {
                // This should use memory.allocate_matrix
                // For now, just return a placeholder pointing to "empty matrix"
                instructions.push(Instruction::I32Const(0));
                Ok(WasmType::I32)
            },
            Value::Void => { 
                instructions.push(Instruction::I32Const(0));
                Ok(WasmType::I32)
            },
            // Sized integer types
            Value::Integer8(i) => {
                instructions.push(Instruction::I32Const(*i as i32));
                Ok(WasmType::I32)
            },
            Value::Integer8u(u) => {
                instructions.push(Instruction::I32Const(*u as i32));
                Ok(WasmType::I32)
            },
            Value::Integer16(i) => {
                instructions.push(Instruction::I32Const(*i as i32));
                Ok(WasmType::I32)
            },
            Value::Integer16u(u) => {
                instructions.push(Instruction::I32Const(*u as i32));
                Ok(WasmType::I32)
            },
            Value::Integer32(i) => {
                instructions.push(Instruction::I32Const(*i));
                Ok(WasmType::I32)
            },
            Value::Integer64(l) => {
                instructions.push(Instruction::I64Const(*l));
                Ok(WasmType::I64)
            },
            // Sized float types
            Value::Number32(f) => {
                instructions.push(Instruction::F32Const(*f));
                Ok(WasmType::F32)
            },
            Value::Number64(f) => {
                instructions.push(Instruction::F64Const(*f));
                Ok(WasmType::F64)
            },
            Value::List(items) => {
                // Implement real list literal generation
                // 1. Allocate memory for the list structure
                // 2. Store list metadata (length, capacity, behavior)
                // 3. Store each item in the list
                // 4. Return pointer to the list
                
                let list_length = items.len() as i32;
                
                // For now, implement a simple list as: [length][item1][item2]...[itemN]
                // Each item is 4 bytes (I32), plus 4 bytes for length = (n+1)*4 bytes total
                let list_size = (list_length + 1) * 4;
                
                // Allocate memory for the list
                if let Some(alloc_fn) = self.get_function_index("allocate_memory") {
                    instructions.push(Instruction::I32Const(list_size));
                    instructions.push(Instruction::Call(alloc_fn));
                    
                    // Store the list pointer in a local variable
                    let list_ptr_local = self.current_locals.len() as u32;
                    self.current_locals.push(LocalVarInfo {
                        index: list_ptr_local,
                        type_: wasm_encoder::ValType::I32,
                    });
                    instructions.push(Instruction::LocalSet(list_ptr_local));
                    
                    // Store the length at the beginning of the list
                    instructions.push(Instruction::LocalGet(list_ptr_local));
                    instructions.push(Instruction::I32Const(list_length));
                    instructions.push(Instruction::I32Store(wasm_encoder::MemArg {
                        offset: 0,
                        align: 2,
                        memory_index: 0,
                    }));
                    
                    // Store each item in the list
                    for (i, item) in items.iter().enumerate() {
                        instructions.push(Instruction::LocalGet(list_ptr_local));
                        
                        // Generate the item value
                        let item_type = self.generate_value(item, instructions)?;
                        
                        // Convert to I32 if needed (for simplicity, store everything as I32)
                        match item_type {
                            WasmType::I32 => {}, // Already correct type
                            WasmType::F32 => {
                                // Convert F32 to I32 (reinterpret bits)
                                instructions.push(Instruction::I32ReinterpretF32);
                            },
                            WasmType::F64 => {
                                // Convert F64 to I32 (truncate and cast)
                                instructions.push(Instruction::I32TruncF64S);
                            },
                            _ => {
                                // For other types, just use as-is (already I32)
                            }
                        }
                        
                        // Store at offset (i+1)*4 (skip the length field)
                        instructions.push(Instruction::I32Store(wasm_encoder::MemArg {
                            offset: ((i + 1) * 4) as u64,
                            align: 2,
                            memory_index: 0,
                        }));
                    }
                    
                    // Return the list pointer
                    instructions.push(Instruction::LocalGet(list_ptr_local));
                    Ok(WasmType::I32)
                } else {
                    // Fallback: create empty list if no allocator available
                    instructions.push(Instruction::I32Const(0));
                    Ok(WasmType::I32)
                }
            },
        }
    }
    
    /// Generate string interpolation instructions
    pub(crate) fn generate_string_interpolation(
        &mut self, 
        parts: &[StringPart], 
        instructions: &mut Vec<Instruction>
    ) -> Result<(), CompilerError> {
        if let Some(builder_init) = self.get_function_index("string_builder_init") {
            instructions.push(Instruction::Call(builder_init));
            
            for part in parts {
                match part {
                    StringPart::Text(_text) => {
                        // This would allocate string and then call append
                        instructions.push(Instruction::I32Const(0)); // placeholder
                        
                        if let Some(append) = self.get_function_index("string_builder_append") {
                            instructions.push(Instruction::Call(append));
                        } else {
                            return Err(CompilerError::codegen_error(
                                "string_builder_append function not found", None, None
                            ));
                        }
                    },
                    StringPart::Interpolation(expr) => {
                        self.generate_expression(expr, instructions)?;
                        
                        if let Some(append_value) = self.get_function_index("string_builder_append_value") {
                            instructions.push(Instruction::Call(append_value));
                        } else {
                            return Err(CompilerError::codegen_error(
                                "string_builder_append_value function not found", None, None
                            ));
                        }
                    },
                }
            }
            
            if let Some(finish) = self.get_function_index("string_builder_finish") {
                instructions.push(Instruction::Call(finish));
                Ok(())
            } else {
                Err(CompilerError::codegen_error("string_builder_finish function not found", None, None))
            }
        } else {
            Err(CompilerError::codegen_error("string_builder_init function not found", None, None))
        }
    }
    
    /// Generate error handler instructions
    pub(crate) fn generate_error_handler_blocks(
        &mut self,
        try_block: &[Statement],
        _error_variable: Option<&str>,
        _catch_block: &[Statement],
        _location: &Option<SourceLocation>,
        instructions: &mut Vec<Instruction>
    ) -> Result<(), CompilerError> {
        // For now, implement a simple try-catch mechanism
        // In a full implementation, this would use WASM's exception handling proposal
        
        // Generate try block instructions
        for stmt in try_block {
            self.generate_statement(stmt, instructions)?;
        }
        
        // TODO: Implement proper exception handling when WASM exception handling is stable
        // For now, we just execute the try block and ignore the catch block
        // In the future, this would:
        // 1. Wrap try_block in a try instruction
        // 2. Add catch handlers for different exception types
        // 3. Bind the error variable in the catch scope
        
        Ok(())
    }
    
    /// Add function type mapping
    pub(crate) fn add_function_type(&mut self, index: u32, params: Vec<ValType>, results: Vec<ValType>) {
        self.function_types.insert(index, FuncType::new(params, results));
    }

    /// Get the function type for a given function index
    pub fn get_function_type(&self, index: u32) -> Option<FuncType> {
        self.function_types.get(&index).cloned()
    }

    /// Convert a parser ValType to a WasmType
    fn from_parser_val_type(val_type: ValType) -> WasmType {
        match val_type {
            ValType::I32 => WasmType::I32,
            ValType::I64 => WasmType::I64,
            ValType::F32 => WasmType::F32,
            ValType::F64 => WasmType::F64,
            ValType::V128 => WasmType::V128,
            _ => WasmType::I32, // Default for other types
        }
    }

    /// Generate matrix operation instructions
    pub(crate) fn generate_matrix_operation(
        &mut self,
        left: &Expression,
        op: &str,
        right: &Expression,
        instructions: &mut Vec<Instruction>
    ) -> Result<WasmType, CompilerError> {
        // First generate the left matrix
        self.generate_expression(left, instructions)?;
        
        // Then generate the right matrix
        self.generate_expression(right, instructions)?;
        
        // Call the appropriate matrix operation function based on the operator
        let function_name = match op {
            "add" => "matrix_add",
            "subtract" => "matrix_subtract",
            "multiply" => "matrix_multiply",
            "transpose" => "matrix_transpose",
            "inverse" => "matrix_inverse",
            _ => return Err(CompilerError::codegen_error(
                &format!("Unknown matrix operation: {}", op),
                Some("Use valid matrix operations".to_string()),
                None
            ))
        };
        
        // Find the function index for the matrix operation
        if let Some(function_index) = self.get_function_index(function_name) {
            instructions.push(Instruction::Call(function_index));
            Ok(WasmType::I32) // Matrix operations return a pointer to the result matrix
        } else {
            Err(CompilerError::codegen_error(
                &format!("Matrix operation function not found: {}", function_name),
                Some("Ensure the matrix operations are registered".to_string()),
                None
            ))
        }
    }

    /// Register a function with the instruction generator
    pub(crate) fn register_function(&mut self, name: &str, params: &[WasmType], return_type: Option<WasmType>, 
        _instructions: &[Instruction]) -> Result<u32, CompilerError>
    {
        // Create signature for this specific overload
        let signature = self.create_function_signature(name, params);
        
        // Check if this exact signature already exists
        if let Some(index) = self.function_signatures.get(&signature) {
            return Ok(*index);
        }
        
        // Get the next function index based on total registered functions
        let index = self.function_signatures.len() as u32;
        
        // Add to signature map (for exact overload lookup)
        self.function_signatures.insert(signature, index);
        
        // Add to function map (for simple name lookup - only store the first registration)
        if !self.function_map.contains_key(name) {
            self.function_map.insert(name.to_string(), index);
        }

        // Create function type and store it
        let param_types: Vec<ValType> = params.iter().map(|wasm_type| match wasm_type {
            WasmType::I32 => ValType::I32,
            WasmType::I64 => ValType::I64,
            WasmType::F32 => ValType::F32,
            WasmType::F64 => ValType::F64,
            WasmType::V128 => ValType::V128,
            _ => ValType::I32, // Default for other types
        }).collect();
        
        let result_types: Vec<ValType> = if let Some(ret_type) = return_type {
            vec![match ret_type {
                WasmType::I32 => ValType::I32,
                WasmType::I64 => ValType::I64,
                WasmType::F32 => ValType::F32,
                WasmType::F64 => ValType::F64,
                WasmType::V128 => ValType::V128,
                _ => ValType::I32, // Default for other types
            }]
        } else {
            vec![] // Void function
        };
        
        // Store the function type
        self.add_function_type(index, param_types, result_types);
        
        // Return the function index
        Ok(index)
    }

    pub(crate) fn get_function_return_type(&self, index: u32) -> Result<WasmType, CompilerError> {
        if let Some(func_type) = self.get_function_type(index) {
            if let Some(return_val_type) = func_type.results().first() {
                let wasm_type = Self::from_parser_val_type(*return_val_type);
                Ok(wasm_type)
            } else {
                // No return type means void function
                // We'll use I32 as a placeholder but this should be handled specially in calling code
                // The calling code should check if the function is void and not expect a value on the stack
                Ok(WasmType::I32) // This represents void functions - but they don't actually put values on stack
            }
        } else {
            // If no function type info is available, default to I32
            Ok(WasmType::I32)
        }
    }
    
    /// Check if a function returns void (has no return values)
    pub(crate) fn is_void_function(&self, index: u32) -> bool {
        if let Some(func_type) = self.get_function_type(index) {
            func_type.results().is_empty()
        } else {
            false // If we can't determine, assume it's not void
        }
    }

    pub(crate) fn get_array_get(&self) -> u32 {
        // Return default array_get function index
        self.get_function_index("array_get").unwrap_or(0)
    }

    pub(crate) fn get_array_length(&self) -> u32 {
        // Return default array_length function index
        self.get_function_index("array_length").unwrap_or(0)
    }

    pub(crate) fn get_matrix_get(&self) -> u32 {
        // Return default matrix_get function index
        self.get_function_index("matrix_get").unwrap_or(0)
    }

    pub(crate) fn get_print_function_index(&self) -> u32 {
        // Return default print function index
        self.get_function_index("print").unwrap_or(0)
    }

    pub(crate) fn get_printl_function_index(&self) -> u32 {
        // Return default printl function index
        self.get_function_index("printl").unwrap_or(0)
    }

    pub(crate) fn generate_array_access(
        &mut self,
        array: &Expression,
        index: &Expression,
        instructions: &mut Vec<Instruction>
    ) -> Result<WasmType, CompilerError> {
        // First, generate the array expression
        let array_type = self.generate_expression(array, instructions)?;
        if array_type != WasmType::I32 {
            return Err(CompilerError::codegen_error(
                "Array access requires array pointer (I32)",
                Some("The array must be a valid array pointer".to_string()),
                None
            ));
        }
        
        // Then, generate the index expression
        let index_type = self.generate_expression(index, instructions)?;
        if index_type != WasmType::I32 {
            return Err(CompilerError::codegen_error(
                "Array index must be I32",
                Some("The array index must be an integer".to_string()),
                None
            ));
        }
        
        // Get the array_get function index
        if let Some(array_get_index) = self.get_function_index("array_get") {
            instructions.push(Instruction::Call(array_get_index));
            
            // Access the element from memory
            instructions.push(Instruction::I32Load(MemArg {
                offset: 0,
                align: 2,
                memory_index: 0,
            }));
            
            return Ok(WasmType::I32);
        } else {
            return Err(CompilerError::codegen_error(
                "array_get function not found",
                Some("The array_get function is not registered".to_string()),
                None
            ));
        }
    }
}

/// Group locals of the same type to reduce WASM size
pub fn group_locals(locals: &[wasm_encoder::ValType]) -> Vec<(u32, wasm_encoder::ValType)> {
    let mut grouped: Vec<(u32, wasm_encoder::ValType)> = Vec::new();
    if locals.is_empty() {
        return grouped;
    }
    
    let mut current_type = locals[0];
    let mut current_count: u32 = 0;
    
    for &local_type in locals {
        if local_type == current_type {
            current_count += 1;
        } else {
            if current_count > 0 {
                grouped.push((current_count, current_type));
            }
            current_type = local_type;
            current_count = 1;
        }
    }
    
    if current_count > 0 {
        grouped.push((current_count, current_type));
    }
    
    grouped
} 