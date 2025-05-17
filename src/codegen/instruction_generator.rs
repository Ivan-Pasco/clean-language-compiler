//! Module for generating WebAssembly instructions.

use wasm_encoder::{Instruction, BlockType, MemArg, Function, ValType};
use crate::ast::{self, Expression, BinaryOperator, Value, Statement, SourceLocation, MatrixOperator};
use crate::types::WasmType;
use crate::error::CompilerError;
use crate::parser::StringPart;

use super::memory::{DEFAULT_ALIGN, DEFAULT_OFFSET};
use super::type_manager::TypeManager;

/// Represents a local variable in a function
#[derive(Debug, Clone)]
pub struct LocalVarInfo {
    pub index: u32,
    pub type_: wasm_encoder::ValType,
}

/// Define a simple FuncType struct for our purposes
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
    function_map: std::collections::HashMap<String, u32>,
}

impl InstructionGenerator {
    /// Create a new instruction generator
    pub(crate) fn new(type_manager: TypeManager) -> Self {
        Self {
            type_manager,
            variable_map: std::collections::HashMap::new(),
            current_locals: Vec::new(),
            function_map: std::collections::HashMap::new(),
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
    
    /// Get a function index by name
    pub(crate) fn get_function_index(&self, name: &str) -> Option<u32> {
        self.function_map.get(name).copied()
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
        
        match (left_type, right_type) {
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
                                _ => unreachable!(), 
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
            (WasmType::I32, WasmType::F64) => {
                instructions.insert(instructions.len() - 1, Instruction::F64ConvertI32S);
                self.generate_binary_operation(left, op, right, instructions)
            },
            (WasmType::F64, WasmType::I32) => {
                instructions.push(Instruction::F64ConvertI32S);
                self.generate_binary_operation(left, op, right, instructions)
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
            Statement::VariableDecl { name, type_, initializer, location } => {
                let specified_type = type_.as_ref().map(|t| self.type_manager.ast_type_to_wasm_type(t))
                    .transpose()?;
                
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
            Statement::Print { expression, newline, location } => {
                self.generate_expression(expression, instructions)?;
                
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
            Statement::Return { value, location } => {
                if let Some(expr) = value {
                    self.generate_expression(expr, instructions)?;
                }
                instructions.push(Instruction::Return);
            },
            Statement::If { condition, then_branch, else_branch, location } => {
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
            Statement::Iterate { iterator, collection, body, location } => {
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
                            align: DEFAULT_ALIGN,
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
            Statement::FromTo { start, end, step, body, location } => {
                let counter_name = format!("_counter_{}", self.current_locals.len());
                let counter_index = self.current_locals.len() as u32;
                self.current_locals.push(LocalVarInfo {
                    index: counter_index,
                    type_: wasm_encoder::ValType::I32,
                });
                self.variable_map.insert(counter_name.clone(), LocalVarInfo {
                    index: counter_index,
                    type_: wasm_encoder::ValType::I32,
                });
                
                let end_index = self.current_locals.len() as u32;
                self.current_locals.push(LocalVarInfo {
                    index: end_index,
                    type_: wasm_encoder::ValType::I32,
                });
                
                let step_index = self.current_locals.len() as u32;
                self.current_locals.push(LocalVarInfo {
                    index: step_index,
                    type_: wasm_encoder::ValType::I32,
                });
                
                self.generate_expression(start, instructions)?;
                instructions.push(Instruction::LocalSet(counter_index));
                
                self.generate_expression(end, instructions)?;
                instructions.push(Instruction::LocalSet(end_index));
                
                if let Some(step_expr) = step {
                    self.generate_expression(step_expr, instructions)?;
                } else {
                    instructions.push(Instruction::I32Const(1));
                }
                instructions.push(Instruction::LocalTee(step_index));
                
                instructions.push(Instruction::I32Eqz);
                instructions.push(Instruction::If(BlockType::Empty));
                
                instructions.push(Instruction::I32Const(1));
                instructions.push(Instruction::LocalSet(step_index));
                
                instructions.push(Instruction::End);
                
                instructions.push(Instruction::Block(BlockType::Empty));
                instructions.push(Instruction::Loop(BlockType::Empty));
                
                instructions.push(Instruction::LocalGet(step_index));
                instructions.push(Instruction::I32Const(0));
                instructions.push(Instruction::I32GtS);
                instructions.push(Instruction::If(BlockType::Empty));
                
                instructions.push(Instruction::LocalGet(counter_index));
                instructions.push(Instruction::LocalGet(end_index));
                instructions.push(Instruction::I32GtS);
                instructions.push(Instruction::BrIf(2));
                
                instructions.push(Instruction::Else);
                
                instructions.push(Instruction::LocalGet(counter_index));
                instructions.push(Instruction::LocalGet(end_index));
                instructions.push(Instruction::I32LtS);
                instructions.push(Instruction::BrIf(2));
                
                instructions.push(Instruction::End);
                
                for stmt in body {
                    self.generate_statement(stmt, instructions)?;
                }
                
                instructions.push(Instruction::LocalGet(counter_index));
                instructions.push(Instruction::LocalGet(step_index));
                instructions.push(Instruction::I32Add);
                instructions.push(Instruction::LocalSet(counter_index));
                
                instructions.push(Instruction::Br(0));
                
                instructions.push(Instruction::End);
                instructions.push(Instruction::End);
                
                self.variable_map.remove(&counter_name);
            },
            Statement::Expression { expr, location } => {
                self.generate_expression(expr, instructions)?;
                instructions.push(Instruction::Drop);
            },
            Statement::ErrorHandler { stmt, handler, location } => {
                self.generate_error_handler(stmt, handler, location, instructions)?;
            },
            Statement::Test { name, description, body, location } => {
                #[cfg(test)]
                for stmt in body {
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
                if let Some(func_index) = self.get_function_index(func_name) {
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
            Expression::MatrixOperation(left, op, right, location) => {
                self.generate_matrix_operation(left, op, right, instructions)
            },
            Expression::StringConcat(parts) => {
                // This would need a proper implementation to handle string parts
                Err(CompilerError::codegen_error(
                    "StringConcat not fully implemented", None, loc.clone()
                ))
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
            Value::Number(n) => {
                instructions.push(Instruction::F64Const(*n));
                Ok(WasmType::F64)
            },
            Value::Integer(i) => {
                instructions.push(Instruction::I32Const(*i));
                Ok(WasmType::I32)
            },
            Value::String(s) => {
                // This should use memory.allocate_string
                // For now, just return a placeholder pointing to "empty string"
                instructions.push(Instruction::I32Const(0));
                Ok(WasmType::I32)
            },
            Value::Boolean(b) => {
                instructions.push(Instruction::I32Const(if *b { 1 } else { 0 }));
                Ok(WasmType::I32)
            },
            Value::Array(_) => {
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
            Value::Byte(b) => {
                instructions.push(Instruction::I32Const(*b as i32));
                Ok(WasmType::I32)
            },
            Value::Unsigned(u) => {
                instructions.push(Instruction::I32Const(*u as i32));
                Ok(WasmType::I32)
            },
            Value::Long(l) => {
                instructions.push(Instruction::I64Const(*l));
                Ok(WasmType::I64)
            },
            Value::ULong(ul) => {
                instructions.push(Instruction::I64Const(*ul as i64));
                Ok(WasmType::I64)
            },
            Value::Big(_) | Value::UBig(_) => {
                // These would need custom big integer handling
                // For now, return a placeholder
                instructions.push(Instruction::I32Const(0));
                Ok(WasmType::I32)
            },
            Value::Float(f) => {
                instructions.push(Instruction::F32Const(*f as f32));
                Ok(WasmType::F32)
            },
            Value::Null | Value::Unit => { 
                instructions.push(Instruction::I32Const(0));
                Ok(WasmType::I32)
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
                    StringPart::Text(text) => {
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
                    StringPart::Expression(expr) => {
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
    pub(crate) fn generate_error_handler(
        &mut self,
        stmt: &Statement,
        handler: &[Statement],
        location: &Option<SourceLocation>,
        instructions: &mut Vec<Instruction>
    ) -> Result<(), CompilerError> {
        instructions.push(Instruction::Try(BlockType::Empty));
        
        self.generate_statement(stmt, instructions)?;
        
        instructions.push(Instruction::Catch(0));
        
        for handler_stmt in handler {
            self.generate_statement(handler_stmt, instructions)?;
        }
        
        instructions.push(Instruction::End);
        
        Ok(())
    }
    
    /// Get the function type for a given function index
    pub fn get_function_type(&self, index: u32) -> Option<FuncType> {
        // Create a simplified version that returns a new FuncType each time
        Some(FuncType::new(
            vec![ValType::I32], // Just assume parameters are I32
            vec![ValType::I32]  // Just assume return type is I32
        ))
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
        op: &MatrixOperator,
        right: &Expression,
        instructions: &mut Vec<Instruction>
    ) -> Result<WasmType, CompilerError> {
        // First generate the left matrix
        self.generate_expression(left, instructions)?;
        
        // Then generate the right matrix
        self.generate_expression(right, instructions)?;
        
        // Call the appropriate matrix operation function based on the operator
        let function_name = match op {
            MatrixOperator::Add => "matrix_add",
            MatrixOperator::Subtract => "matrix_subtract",
            MatrixOperator::Multiply => "matrix_multiply",
            MatrixOperator::Transpose => "matrix_transpose",
            MatrixOperator::Inverse => "matrix_inverse",
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
        instructions: &[Instruction]) -> Result<u32, CompilerError>
    {
        // Check if the function already exists
        if let Some(index) = self.get_function_index(name) {
            return Ok(index);
        }
        
        // Get the next function index
        let index = self.function_map.len() as u32;
        
        // Add the function to the function map
        self.function_map.insert(name.to_string(), index);

        // In a proper implementation, we would also:
        // 1. Create a function type with the given parameters and return type
        // 2. Add the function type to the type section
        // 3. Create a function with the given instructions
        // 4. Add the function to the code section
        // 5. Add an export for the function
        //
        // However, InstructionGenerator doesn't have direct access to these sections.
        // The CodeGenerator should handle this after calling register_function.
        
        // Return the function index
        Ok(index)
    }

    pub(crate) fn get_function_return_type(&self, index: u32) -> Result<WasmType, CompilerError> {
        // For now, just return a default return type
        Ok(WasmType::I32)
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