//! Module for WebAssembly code generation.

use wasm_encoder::{
    BlockType, CodeSection, ConstExpr, DataSegment, DataSection, ExportKind, ExportSection,
    Function, FunctionSection, GlobalSection, GlobalType, Instruction,
    MemorySection, Module, TypeSection, ValType,
    MemoryType, ImportSection, MemArg, ElementSection, TableSection,
};
use wasmparser::FuncType;
use wasmtime::{Export, Store};
use crate::ast::{self, Program, Expression, Statement, Type, Value, Function as AstFunction, 
    MatrixOperator, BinaryOperator, SourceLocation};
use crate::error::{CompilerError, ErrorContext, ErrorType};
use crate::stdlib::memory::MemoryManager;
use crate::parser::StringPart;
use crate::stdlib::StdLib;
use crate::types::{WasmType, ValTypeConverter};
use std::collections::HashMap;

// Declare the modules
mod string_pool;
mod memory;
mod type_manager;
mod instruction_generator;

#[cfg(test)]
mod tests;

// Import the StringPool struct
use string_pool::StringPool;
use memory::MemoryUtils;
use type_manager::TypeManager;
use instruction_generator::{InstructionGenerator, LocalVarInfo};

// Add these constants for memory type IDs
pub const INTEGER_TYPE_ID: u32 = 1;
pub const FLOAT_TYPE_ID: u32 = 2;
pub const STRING_TYPE_ID: u32 = 3;
pub const ARRAY_TYPE_ID: u32 = 4;
pub const MATRIX_TYPE_ID: u32 = 5;

// Memory constants
pub const PAGE_SIZE: u32 = 65536;
pub const HEADER_SIZE: u32 = 16;  // 16-byte header for memory blocks
pub const MIN_ALLOCATION: u32 = 16;
pub const HEAP_START: usize = 65536;  // Start heap at 64KB
const DEFAULT_ALIGN: u32 = 2;
const DEFAULT_OFFSET: u32 = 0;

/// Code generator for Clean Language
pub struct CodeGenerator {
    module: Module,
    function_section: FunctionSection,
    export_section: ExportSection,
    code_section: CodeSection,
    import_section: ImportSection,
    string_pool: StringPool,
    memory_section: MemorySection,
    global_section: GlobalSection,
    type_section: TypeSection,
    data_section: DataSection,
    element_section: ElementSection,
    table_section: TableSection,
    type_manager: TypeManager,
    instruction_generator: InstructionGenerator,
    variable_map: HashMap<String, LocalVarInfo>,
    memory_utils: MemoryUtils,
    stdlib_registered: bool,
    next_function_index: u32,
    next_type_index: u32,
    function_count: u32,
    current_locals: Vec<LocalVarInfo>,
    function_map: HashMap<String, u32>,
    function_types: Vec<FuncType>,
    function_names: Vec<String>,
}

impl CodeGenerator {
    /// Create a new code generator
    pub fn new() -> Self {
        let type_manager = TypeManager::new();
        let instruction_generator = InstructionGenerator::new(type_manager.clone());
        
        Self {
            module: Module::new(),
            function_section: FunctionSection::new(),
            export_section: ExportSection::new(),
            code_section: CodeSection::new(),
            import_section: ImportSection::new(),
            string_pool: StringPool::new(),
            memory_section: MemorySection::new(),
            global_section: GlobalSection::new(),
            type_section: TypeSection::new(),
            data_section: DataSection::new(),
            element_section: ElementSection::new(),
            table_section: TableSection::new(),
            type_manager,
            instruction_generator,
            variable_map: HashMap::new(),
            memory_utils: MemoryUtils::new(HEAP_START),
            stdlib_registered: false,
            next_function_index: 0,
            next_type_index: 0,
            function_count: 0,
            current_locals: Vec::new(),
            function_map: HashMap::new(),
            function_types: Vec::new(),
            function_names: Vec::new(),
        }
    }

    pub fn generate(&mut self, program: &Program) -> Result<Vec<u8>, CompilerError> {
        // Register standard library functions (not used yet for minimal module)
        // self.register_stdlib_functions()?;

        // ------------------------------------------------------------------
        // 1. Type section ---------------------------------------------------
        // ------------------------------------------------------------------
        let mut type_section = TypeSection::new();
        
        // First add the start function type
        // Find the start function in the program
        let start_fn_index = program.functions.iter()
            .position(|f| f.name == "start")
            .ok_or_else(|| CompilerError::codegen_error("No 'start' function found in the program", None, None))?;
        
        let start_function = &program.functions[start_fn_index];
        
        // Convert parameters to WasmType
        let params: Vec<WasmType> = start_function.parameters.iter()
                .map(|param| WasmType::from(&param.type_))
                .collect();
            
        // Convert return_type safely
        let return_type = WasmType::from(&start_function.return_type);
        
        // Convert WasmType to ValType for the encoder
        let param_val_types: Vec<ValType> = params.iter().map(|t| (*t).into()).collect();
        let return_val_types = if return_type == WasmType::Unit {
            vec![]
        } else {
            vec![return_type.into()]
        };
        
        // Add the start function type
        type_section.function(param_val_types.clone(), return_val_types.clone());
        
        // Push type section first (id = 1)
        self.module.section(&type_section);

        // ------------------------------------------------------------------
        // 2. Function section (id = 3) -------------------------------------
        // ------------------------------------------------------------------
        let mut function_section = FunctionSection::new();
        
        // Add the start function using type index 0
        function_section.function(0);
        
        // Push function section
        self.module.section(&function_section);

        // ------------------------------------------------------------------
        // 3. Memory section  (id = 5) --------------------------------------
        // ------------------------------------------------------------------
        // For now create a single 1-page memory so future code can store data
        self.memory_section.memory(MemoryType {
            minimum: 1,
            maximum: None,
            memory64: false,
            shared: false,
        });

        self.module.section(&self.memory_section);

        // ------------------------------------------------------------------
        // 4. Export section (id = 7) ---------------------------------------
        // ------------------------------------------------------------------
        let mut export_section = ExportSection::new();

        // Export ONLY the start function for now (index 0)
        export_section.export("start", ExportKind::Func, 0);

        // Push export section
        self.module.section(&export_section);

        // ------------------------------------------------------------------
        // 5. Code section (id = 10) ----------------------------------------
        // ------------------------------------------------------------------
        let mut code_section = CodeSection::new();
        
        // Generate instructions for the start function
        let mut instructions = Vec::new();
        
        // Generate code for the start function body
        for stmt in &start_function.body {
            self.generate_statement(stmt, &mut instructions)?;
        }
        
        // Create the function
        let mut start_func = Function::new(vec![]);
        
        // Add all instructions
        for instruction in instructions {
            start_func.instruction(&instruction);
        }
        
        // Add the End instruction
        start_func.instruction(&Instruction::End);
        
        // Add function to code section
        code_section.function(&start_func);
        
        // Add code section to module
        self.module.section(&code_section);
        
        // Return the WebAssembly module
        Ok(self.module.clone().finish())
    }

    /// Finalize and return the WebAssembly binary
    pub fn finish(&self) -> Vec<u8> {
        self.module.clone().finish()
    }

    fn add_function_type(&mut self, params: &[WasmType], return_type: Option<WasmType>) -> Result<u32, CompilerError> {
        // Use the type manager to add the function type
        let type_index = self.type_manager.add_function_type(params, return_type)?;
        
        // For compatibility with existing code, also maintain our own function_types list
        let param_val_types: Vec<ValType> = params.iter().map(|t| (*t).into()).collect();
        let return_val_types: Vec<ValType> = return_type.map(|t| vec![t.into()]).unwrap_or_default();
        
        // Convert to wasmparser ValType for FuncType with explicit type annotation
        let parser_param_types: Vec<wasmparser::ValType> = param_val_types.iter()
            .map(|vt| WasmType::from(*vt).to_parser_val_type())
            .collect();
        let parser_result_types: Vec<wasmparser::ValType> = return_val_types.iter()
            .map(|vt| WasmType::from(*vt).to_parser_val_type())
            .collect();
        
        // Create and store the FuncType
        self.function_types.push(FuncType::new(parser_param_types, parser_result_types));

        Ok(type_index)
    }

    fn add_memory_functions(&mut self) -> Result<(), CompilerError> {
        // Add malloc function
        let malloc_type = self.add_function_type(&[WasmType::I32], Some(WasmType::I32))?;
        self.function_section.function(malloc_type);
        self.export_section.export(
            "malloc",
            ExportKind::Func,
            self.function_count
        );
        self.function_count += 1;

        // Add retain function
        let retain_type = self.add_function_type(&[WasmType::I32], None)?;
        self.function_section.function(retain_type);
        self.export_section.export(
            "retain",
            ExportKind::Func,
            self.function_count
        );
        self.function_count += 1;

        // Add release function
        let release_type = self.add_function_type(&[WasmType::I32], None)?;
        self.function_section.function(release_type);
            self.export_section.export(
            "release",
            ExportKind::Func,
            self.function_count
        );
        self.function_count += 1;

        Ok(())
    }

    fn generate_malloc_function(&mut self) -> Vec<Instruction> {
        vec![
            // Get size parameter
            Instruction::LocalGet(0),
            
            // Call memory manager's allocate
            Instruction::Call(self.get_allocate_function_index()),
            
            // Return pointer
            Instruction::Return,
        ]
    }

    fn generate_retain_function(&mut self) -> Vec<Instruction> {
        vec![
            // Get pointer parameter
            Instruction::LocalGet(0),
            
            // Call memory manager's retain
            Instruction::Call(self.get_retain_function_index()),
            
            // Return
            Instruction::Return,
        ]
    }

    fn generate_release_function(&mut self) -> Vec<Instruction> {
        vec![
            // Get pointer parameter
            Instruction::LocalGet(0),
            
            // Call memory manager's release
            Instruction::Call(self.get_release_function_index()),
            
            // Return
            Instruction::Return,
        ]
    }

    fn get_allocate_function_index(&self) -> u32 {
        // Index of the memory allocate function
        0
    }

    fn get_retain_function_index(&self) -> u32 {
        // Index of the retain function
        1
    }

    fn get_release_function_index(&self) -> u32 {
        // Index of the release function
        2
    }

    fn generate_program(&mut self, program: &Program) -> Result<(), CompilerError> {
        // Generate code for each function in the program
        for function in &program.functions {
            self.generate_function(function)?;
        }

        Ok(())
    }

    pub fn generate_function(&mut self, function: &AstFunction) -> Result<(), CompilerError> {
        self.current_locals.clear();
        self.variable_map.clear();
        
        // Convert parameters to WasmType
        let params: Vec<WasmType> = function.parameters.iter()
            .map(|param| WasmType::from(&param.type_))
            .collect();
            
        // Convert return_type safely
        let return_type = WasmType::from(&function.return_type);
            
        // Add function type using type manager
        let type_index = self.add_function_type(&params, Some(return_type))?;
        
        // Add to function section
        self.function_section.function(type_index);
        
        // Track function index and name mapping
        let func_index = self.function_count;
        self.function_map.insert(function.name.clone(), func_index);
        self.function_names.push(function.name.clone());

        // Export the function if it's the main 'start' function
        if function.name == "start" {
            self.export_section.export(
                "start",
                ExportKind::Func,
                func_index
            );
        }
        
        // Increment function count for next function
        self.function_count += 1;

        // Add parameters to locals
        for param in &function.parameters {
            let wasm_type = WasmType::from(&param.type_);
            let local_info = LocalVarInfo {
                index: self.current_locals.len() as u32,
                type_: wasm_type.into(),
            };
            self.current_locals.push(local_info.clone());
            self.variable_map.insert(param.name.clone(), local_info);
        }
            
        let mut instructions = Vec::new();
        
        // Generate code for function body
        for statement in &function.body {
            self.generate_statement(statement, &mut instructions)?;
        }
        
        // Add implicit return if needed
        if self.function_types.get(func_index as usize).map_or(true, |func_type| func_type.results().is_empty()) {
            // If no explicit return or trap, we don't need to add an implicit return as the END instruction implies return void
        }
        
        // Process locals for wasm_encoder::Function::new
        // Group consecutive locals of the same type
        let mut local_declarations: Vec<(u32, ValType)> = Vec::new();
        if !self.current_locals.is_empty() {
            let mut current_type: ValType = self.current_locals[0].type_;
            let mut current_count: u32 = 0;

            for local_info in &self.current_locals {
                let val_type: ValType = local_info.type_;
                if val_type == current_type {
                    current_count += 1;
                } else {
                    if current_count > 0 {
                        local_declarations.push((current_count, current_type));
                    }
                    current_type = val_type;
                    current_count = 1;
                }
            }
            // Add the last group
            if current_count > 0 {
                local_declarations.push((current_count, current_type));
            }
        }
            
        let mut func = Function::new(local_declarations); // Use the grouped locals
        
        // Add instructions using proper error handling
        for instruction in instructions {
            func.instruction(&instruction);
        }
        
        // Add END instruction with proper error handling
        func.instruction(&Instruction::End);

        self.code_section.function(&func);
        Ok(())
    }

    pub fn generate_statement(&mut self, stmt: &Statement, instructions: &mut Vec<Instruction>) -> Result<(), CompilerError> {
        match stmt {
            Statement::VariableDecl { name, type_, initializer, location } => {
                let specified_type = type_.as_ref().map(|t| WasmType::from(t));
                
                let (var_type, init_instructions) = if let Some(init_expr) = initializer {
                    let mut init_instr = Vec::new();
                    let init_type = self.generate_expression(init_expr, &mut init_instr)?;
                    (init_type, Some(init_instr))
                } else {
                    if specified_type.is_none() {
                         return Err(CompilerError::codegen_error(
                            "Variable declaration without initializer must have a type annotation",
                            None, location.clone()
                        ));
                    }
                    (specified_type.unwrap(), None)
                };

                if let (Some(st), declared_type) = (specified_type, var_type) {
                    if st != declared_type {
                        return Err(CompilerError::type_error(
                            format!("Initializer type {:?} does not match specified type {:?} for variable '{}'", declared_type, st, name),
                            None, location.clone()
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
                        WasmType::I32 | WasmType::I64 => instructions.push(Instruction::I64Const(0)),
                        WasmType::F32 => instructions.push(Instruction::F32Const(0.0)),
                        WasmType::F64 => instructions.push(Instruction::F64Const(0.0)),
                        _ => return Err(CompilerError::codegen_error(format!("Cannot determine default value for type {:?}", var_type), None, location.clone()))
                    }
                    instructions.push(Instruction::LocalSet(local_info.index));
                }
            }
            Statement::Assignment { target, value, location } => {
                if let Some(local_info) = self.find_local(target) {
                    self.generate_expression(value, instructions)?;
                    instructions.push(Instruction::LocalSet(local_info.index));
                } else {
                    return Err(CompilerError::codegen_error(
                        format!("Undefined variable: {}", target),
                        None, // help
                        location.clone() // location
                    ));
                }
            }
            Statement::Print { expression, newline, location } => {
                self.generate_expression(expression, instructions)?;
                instructions.push(Instruction::Call(
                    if *newline {
                        self.get_printl_function_index()
                    } else {
                        self.get_print_function_index()
                    }
                ));
            }
            Statement::Return { value, location } => {
                if let Some(expr) = value {
                self.generate_expression(expr, instructions)?;
                }
                instructions.push(Instruction::Return);
            }
            Statement::If { condition, then_branch, else_branch, location } => {
                self.generate_expression(condition, instructions)?;
                
                if let Some(else_) = else_branch {
                instructions.push(Instruction::If(BlockType::Empty));
                
                    for stmt in then_branch {
                        self.generate_statement(stmt, instructions)?;
                    }
                    
                    instructions.push(Instruction::Else);
                    
                    for stmt in else_ {
                    self.generate_statement(stmt, instructions)?;
                }
                
                instructions.push(Instruction::End);
                } else {
                    instructions.push(Instruction::If(BlockType::Empty));
                    
                    for stmt in then_branch {
                        self.generate_statement(stmt, instructions)?;
                    }
                    
                    instructions.push(Instruction::End);
                }
            }
            Statement::Iterate { iterator, collection, body, location } => {
                self.generate_expression(collection, instructions)?;
                
                let array_ptr_index = self.current_locals.len() as u32;
                self.current_locals.push(LocalVarInfo {
                    index: array_ptr_index,
                    type_: ValType::I32.into(),
                });
                instructions.push(Instruction::LocalSet(array_ptr_index));
                
                let counter_index = self.current_locals.len() as u32;
                self.current_locals.push(LocalVarInfo {
                    index: counter_index,
                    type_: ValType::I32.into(),
                });
                
                let iterator_index = self.current_locals.len() as u32;
                self.current_locals.push(LocalVarInfo {
                    index: iterator_index,
                    type_: ValType::I32.into(),
                });
                
                self.variable_map.insert(iterator.clone(), LocalVarInfo {
                    index: iterator_index,
                    type_: ValType::I32.into(),
                });
                
                instructions.push(Instruction::LocalGet(array_ptr_index));
                instructions.push(Instruction::Call(self.get_array_length()));
                
                let length_index = self.current_locals.len() as u32;
                self.current_locals.push(LocalVarInfo {
                    index: length_index,
                    type_: ValType::I32.into(),
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
                
                instructions.push(Instruction::Call(self.get_array_get()));
                
                instructions.push(Instruction::I32Load(MemArg {
                    offset: 0,
                    align: 2,
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
            }
            Statement::FromTo { start, end, step, body, location } => {
                let counter_name = format!("_counter_{}", self.current_locals.len());
                let counter_index = self.current_locals.len() as u32;
                self.current_locals.push(LocalVarInfo {
                    index: counter_index,
                    type_: ValType::I32.into(),
                });
                self.variable_map.insert(counter_name.clone(), LocalVarInfo {
                    index: counter_index,
                    type_: ValType::I32.into(),
                });
                
                let end_index = self.current_locals.len() as u32;
                self.current_locals.push(LocalVarInfo {
                    index: end_index,
                    type_: ValType::I32.into(),
                });
                
                let step_index = self.current_locals.len() as u32;
                self.current_locals.push(LocalVarInfo {
                    index: step_index,
                    type_: ValType::I32.into(),
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
            }
            Statement::ErrorHandler { stmt, handler, location } => {
                self.generate_error_handler(stmt, handler, location, instructions)?;
            }
            Statement::Test { name, description, body, location } => {
                #[cfg(test)]
                for stmt in body {
                    self.generate_statement(stmt, instructions)?;
                }
            }
            Statement::Expression { expr, location } => {
                self.generate_expression(expr, instructions)?;
                instructions.push(Instruction::Drop);
            }
            Statement::Constructor { params, body, location } => {
                for param in params {
                    let type_ = self.ast_type_to_wasm_type(&param.type_)?;
                    self.current_locals.push(LocalVarInfo {
                        index: self.current_locals.len() as u32,
                        type_: type_.into(),
                    });
                }
                
                for stmt in body {
                    self.generate_statement(stmt, instructions)?;
                }
            }
        }
        Ok(())
    }

    fn generate_expression(&mut self, expr: &Expression, instructions: &mut Vec<Instruction>) -> Result<WasmType, CompilerError> {
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
                // Check if variable exists to provide better error messages
                if let Some(local) = self.find_local(name) {
                instructions.push(Instruction::LocalGet(local.index));
                Ok(WasmType::from(local.type_))
                } else {
                    // Collect all visible variables for better suggestions
                    let variables: Vec<&str> = self.variable_map.keys().map(|s| s.as_str()).collect();
                    
                    Err(CompilerError::function_not_found_error(
                        name,
                        &variables,
                        loc
                    ))
                }
            },
            Expression::Call(func_name, args) => { 
                // Check if function exists to provide better error messages
                if let Some(func_index) = self.get_function_index(func_name) {
                    // First check if argument count matches
                    if let Some(func_type) = self.instruction_generator.get_function_type(func_index) {
                        let expected_arg_count = func_type.params().len();
                        if args.len() != expected_arg_count {
                            return Err(CompilerError::detailed_type_error(
                                &format!("Function '{}' called with wrong number of arguments", func_name),
                                expected_arg_count,
                                args.len(),
                                loc.clone(),
                                Some(format!("Function '{}' expects {} arguments, but {} were provided", 
                                    func_name, expected_arg_count, args.len()))
                            ));
                        }
                    }
                    
                    // Generate code for arguments
                for arg in args {
                    self.generate_expression(arg, instructions)?;
                }
                    
                instructions.push(Instruction::Call(func_index));
                self.get_function_return_type(func_index)
                } else {
                    // Collect all function names for better suggestions
                    let functions: Vec<&str> = self.function_names.iter().map(|s| s.as_str()).collect();
                    
                    Err(CompilerError::function_not_found_error(
                        func_name,
                        &functions,
                        loc
                    ))
                }
            },
            Expression::Binary(left, op, right) => { 
                self.generate_binary_operation(left, op, right, instructions)
            },
            Expression::ArrayAccess(array, index) => {
                self.generate_expression(&*array, instructions)?;
                self.generate_expression(&*index, instructions)?;
                instructions.push(Instruction::Call(self.get_array_get())); 
                Ok(WasmType::I32)
            },
            Expression::MatrixAccess(matrix, row, col) => {
                self.generate_expression(&*matrix, instructions)?;
                self.generate_expression(&*row, instructions)?;
                self.generate_expression(&*col, instructions)?;
                instructions.push(Instruction::Call(self.get_matrix_get())); 
                Ok(WasmType::F64)
            },
            Expression::StringConcat(parts) => {
                // Assuming parts is Vec<Expression> as per AST error E0023
                // self.generate_string_interpolation needs Vec<StringPart>
                // Need to convert/handle this mismatch - placeholder error for now
                Err(CompilerError::codegen_error("StringConcat codegen needs update for Vec<Expression>", None, loc.clone()))
                // self.generate_string_interpolation(parts, instructions)?;
                // Ok(WasmType::I32) 
            },
            // TODO: Add other expression variants based on ast::Expression definition
            // Expression::Unary(op, expr) => { ... }
            _ => Err(CompilerError::codegen_error("Unsupported expression type in codegen", None, loc.clone())),
        }
    }

    fn generate_binary_operation(
        &mut self,
        left: &Expression,
        op: &BinaryOperator,
        right: &Expression,
        instructions: &mut Vec<Instruction>
    ) -> Result<WasmType, CompilerError> {
        let left_type = self.generate_expression(left, instructions)?;
        let right_type = self.generate_expression(right, instructions)?;
        
        // Special handling for division by zero
        if let BinaryOperator::Divide = op {
            match right {
                Expression::Literal(Value::Integer(0)) | Expression::Literal(Value::Number(0.0)) => {
                    return Err(CompilerError::division_by_zero_error(None));
                },
                _ => {
                    // For non-literal divisors, add a runtime check
                    match right_type {
                        WasmType::I32 => {
                            // Add runtime check for integer division
                            instructions.push(Instruction::LocalGet(instructions.len() as u32 - 1)); // Get divisor
                            instructions.push(Instruction::I32Eqz); // Check if zero
                            instructions.push(Instruction::If(BlockType::Empty));
                            
                            // Handle division by zero at runtime
                            // For now, we'll just push a trap instruction
                            instructions.push(Instruction::Unreachable);
                            
                            instructions.push(Instruction::End);
                        },
                        WasmType::F64 => {
                            // Add runtime check for float division
                            instructions.push(Instruction::LocalGet(instructions.len() as u32 - 1)); // Get divisor
                            instructions.push(Instruction::F64Const(0.0));
                            instructions.push(Instruction::F64Eq); // Check if zero
                            instructions.push(Instruction::If(BlockType::Empty));
                            
                            // Handle division by zero at runtime
                            // For now, we'll just push a trap instruction
                            instructions.push(Instruction::Unreachable);
                            
                            instructions.push(Instruction::End);
                        },
                        _ => {} // No check for other types
                    }
                }
            }
        }
        
        match (left_type, right_type) {
            (WasmType::I32, WasmType::I32) => {
                match op {
                    // Use correct AST variant names
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
                    // ast::BinaryOperator::And => { instructions.push(Instruction::I32And); Ok(WasmType::I32) }, // Assuming And/Or are in AST
                    // ast::BinaryOperator::Or => { instructions.push(Instruction::I32Or); Ok(WasmType::I32) },
                     _ => Err(CompilerError::type_error(format!("Unsupported I32 binary operator: {:?}", op), None, None)),
                }
            },
            (WasmType::F64, WasmType::F64) => {
                match op {
                    // Use correct AST variant names
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
                    _ => Err(CompilerError::type_error(format!("Unsupported F64 binary operator: {:?}", op), None, None))
                }
            },
            (WasmType::I32, WasmType::I32) if self.is_string_type(left) || self.is_string_type(right) => {
                 match op {
                    // Use correct AST variant names
                    ast::BinaryOperator::Add => { instructions.push(Instruction::Call(self.get_string_concat_index()?)); Ok(WasmType::I32) },
                    ast::BinaryOperator::Equal | ast::BinaryOperator::NotEqual | 
                    ast::BinaryOperator::Less | ast::BinaryOperator::Greater | 
                    ast::BinaryOperator::LessEqual | ast::BinaryOperator::GreaterEqual => {
                        instructions.push(Instruction::Call(self.get_string_compare_index()?));
                        match op {
                            ast::BinaryOperator::Equal => instructions.push(Instruction::I32Eqz),
                            ast::BinaryOperator::NotEqual => { instructions.push(Instruction::I32Const(0)); instructions.push(Instruction::I32Ne); },
                            ast::BinaryOperator::Less => { instructions.push(Instruction::I32Const(0)); instructions.push(Instruction::I32LtS); },
                            ast::BinaryOperator::Greater => { instructions.push(Instruction::I32Const(0)); instructions.push(Instruction::I32GtS); },
                            ast::BinaryOperator::LessEqual => { instructions.push(Instruction::I32Const(0)); instructions.push(Instruction::I32LeS); },
                            ast::BinaryOperator::GreaterEqual => { instructions.push(Instruction::I32Const(0)); instructions.push(Instruction::I32GeS); },
                            _ => unreachable!(), 
                        }
                        Ok(WasmType::I32)
                    },
                    _ => Err(CompilerError::type_error(format!("Unsupported string binary operator: {:?}", op), None, None)),
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
            
            _ => {
                Err(CompilerError::detailed_type_error(
                    &format!("Type mismatch: Cannot apply {:?} to incompatible types", op),
                    left_type,
                    right_type,
                    None, 
                    Some(format!("The operator {:?} cannot be applied to types {:?} and {:?}. Consider using type conversion.", 
                        op, left_type, right_type))
                ))
            }
        }
    }
    
    fn is_string_type(&self, expr: &Expression) -> bool {
        match expr {
            // Correct patterns
            Expression::Literal(Value::String(_)) => true,
            Expression::Variable(name) => { /* ... */ false } // Needs type lookup
            Expression::StringConcat(_) => true,
            _ => false,
        }
    }
    
    fn can_convert(&self, from: WasmType, to: WasmType) -> bool {
        match (from, to) {
            (WasmType::I32, WasmType::F64) => true,
            (WasmType::F64, WasmType::I32) => true,
            _ => from == to,
        }
    }
    
    fn generate_conversion(&self, from: WasmType, to: WasmType, instructions: &mut Vec<Instruction>) -> Result<(), CompilerError> {
        match (from, to) {
            (WasmType::I32, WasmType::F64) => {
                instructions.push(Instruction::F64ConvertI32S);
                Ok(())
            },
            (WasmType::F64, WasmType::I32) => {
                instructions.push(Instruction::I32TruncF64S);
                Ok(())
            },
            (t1, t2) if t1 == t2 => Ok(()), 
            _ => Err(CompilerError::codegen_error(
                format!("Cannot convert from {:?} to {:?}", from, to),
                None, 
                None
            )),
        }
    }
    
    fn get_string_concat_index(&self) -> Result<u32, CompilerError> {
        self.get_function_index("string_concat")
            .ok_or_else(|| CompilerError::codegen_error("String concatenation function not found", None, None))
    }

    fn get_string_compare_index(&self) -> Result<u32, CompilerError> {
        self.get_function_index("string_compare")
            .ok_or_else(|| CompilerError::codegen_error("String comparison function not found", None, None))
    }

    fn generate_value(&mut self, value: &Value, instructions: &mut Vec<Instruction>) -> Result<WasmType, CompilerError> {
        match value {
            Value::Number(n) => {
                instructions.push(Instruction::F64Const(*n));
                Ok(WasmType::F64)
            }
            Value::Integer(i) => {
                instructions.push(Instruction::I32Const(*i));
                Ok(WasmType::I32)
            }
            Value::String(s) => {
                let ptr = self.allocate_string(s)?;
                instructions.push(Instruction::I32Const(ptr as i32));
                Ok(WasmType::I32)
            }
            Value::Boolean(b) => {
                instructions.push(Instruction::I32Const(if *b { 1 } else { 0 }));
                Ok(WasmType::I32)
            }
            Value::Array(elements) => {
                let ptr = self.allocate_array(elements)?;
                instructions.push(Instruction::I32Const(ptr as i32));
                Ok(WasmType::I32)
            }
            Value::Matrix(rows) => {
                // Convert the matrix values to f64 rows 
                let mut matrix_data = Vec::new();
                for row in rows {
                    for val in row {
                        matrix_data.push(*val); // Since row is Vec<f64>, just dereference
                    }
                }
                
                let ptr = self.allocate_matrix(&matrix_data, rows.len(), rows[0].len())?;
                instructions.push(Instruction::I32Const(ptr as i32));
                Ok(WasmType::I32)
            }
            _ => Err(CompilerError::type_error(
                &format!("Unsupported literal value: {:?}", value),
                Some("Use supported literal types".to_string()),
                None
            )),
        }
    }

    /// Generate a vec of try-catch instructions
    fn generate_try_catch_block(&mut self, try_block: &[Instruction], catch_tag: u32) -> Vec<Instruction> {
        let mut result = Vec::new();
        result.push(Instruction::Try(BlockType::Empty));
        
        // Manually clone each instruction to avoid lifetime issues
        for instr in try_block {
            // Each match arm creates a new instruction, avoiding reference issues
            let cloned_instr = match instr {
                Instruction::I32Const(v) => Instruction::I32Const(*v),
                Instruction::I64Const(v) => Instruction::I64Const(*v),
                Instruction::F32Const(v) => Instruction::F32Const(*v),
                Instruction::F64Const(v) => Instruction::F64Const(*v),
                Instruction::I32Add => Instruction::I32Add,
                Instruction::I32Sub => Instruction::I32Sub,
                Instruction::I32Mul => Instruction::I32Mul,
                Instruction::F64Add => Instruction::F64Add,
                Instruction::F64Sub => Instruction::F64Sub,
                Instruction::F64Mul => Instruction::F64Mul,
                Instruction::LocalGet(i) => Instruction::LocalGet(*i),
                Instruction::LocalSet(i) => Instruction::LocalSet(*i),
                Instruction::LocalTee(i) => Instruction::LocalTee(*i),
                Instruction::Call(i) => Instruction::Call(*i),
                Instruction::If(bt) => Instruction::If(bt.clone()),
                Instruction::Else => Instruction::Else,
                Instruction::End => Instruction::End,
                Instruction::Block(bt) => Instruction::Block(bt.clone()),
                Instruction::Loop(bt) => Instruction::Loop(bt.clone()),
                Instruction::Br(depth) => Instruction::Br(*depth),
                Instruction::BrIf(depth) => Instruction::BrIf(*depth),
                Instruction::Return => Instruction::Return,
                Instruction::Unreachable => Instruction::Unreachable,
                Instruction::Drop => Instruction::Drop,
                Instruction::I32Eqz => Instruction::I32Eqz,
                Instruction::I32Eq => Instruction::I32Eq,
                Instruction::I32Ne => Instruction::I32Ne,
                Instruction::I32LtS => Instruction::I32LtS,
                Instruction::I32LtU => Instruction::I32LtU,
                Instruction::I32GtS => Instruction::I32GtS,
                Instruction::I32GtU => Instruction::I32GtU,
                Instruction::I32LeS => Instruction::I32LeS,
                Instruction::I32LeU => Instruction::I32LeU,
                Instruction::I32GeS => Instruction::I32GeS,
                Instruction::I32GeU => Instruction::I32GeU,
                Instruction::I32Load(memarg) => Instruction::I32Load(memarg.clone()),
                Instruction::I32Store(memarg) => Instruction::I32Store(memarg.clone()),
                Instruction::I32Load8U(memarg) => Instruction::I32Load8U(memarg.clone()),
                Instruction::I32Store8(memarg) => Instruction::I32Store8(memarg.clone()),
                // Default case for other instructions - add more specific cases as needed
                _ => Instruction::Nop,
            };
            result.push(cloned_instr);
        }
        
        result.push(Instruction::Catch(catch_tag));
        result.push(Instruction::End);
        
        result
    }

    // Helper to register stdlib functions
    fn register_stdlib_functions(&mut self) -> Result<(), CompilerError> {
        // In a real implementation, this would register all standard library functions
        // For now, just a placeholder
        if !self.stdlib_registered {
            // Register memory functions
            self.add_memory_functions()?;
            self.stdlib_registered = true;
        }
        Ok(())
    }

    // Add delegate methods to use instruction_generator
    // These should be part of the CodeGenerator implementation

    pub fn find_local(&self, name: &str) -> Option<LocalVarInfo> {
        self.instruction_generator.find_local(name).cloned()
    }

    pub fn get_function_index(&self, name: &str) -> Option<u32> {
        self.instruction_generator.get_function_index(name)
    }

    pub fn get_function_return_type(&self, index: u32) -> Result<WasmType, CompilerError> {
        self.instruction_generator.get_function_return_type(index)
    }

    pub fn get_array_get(&self) -> u32 {
        self.instruction_generator.get_array_get()
    }

    pub fn get_array_length(&self) -> u32 {
        self.instruction_generator.get_array_length()
    }

    pub fn get_matrix_get(&self) -> u32 {
        self.instruction_generator.get_matrix_get()
    }

    pub fn get_print_function_index(&self) -> u32 {
        self.instruction_generator.get_print_function_index()
    }

    pub fn get_printl_function_index(&self) -> u32 {
        self.instruction_generator.get_printl_function_index()
    }

    pub fn register_function(&mut self, name: &str, params: &[WasmType], return_type: Option<WasmType>, 
        instructions: &[Instruction]) -> Result<u32, CompilerError>
    {
        // First, register with instruction_generator to get the function index
        let function_index = self.instruction_generator.register_function(name, params, return_type, instructions)?;
        
        // Convert WasmType params to ValType
        let val_type_params: Vec<ValType> = params.iter().map(|wt| (*wt).into()).collect();
        
        // Convert return_type to Vec<ValType> (empty for None, or single value for Some)
        let val_type_results: Vec<ValType> = if let Some(rt) = return_type {
            vec![rt.into()]
                } else {
            vec![]
        };
        
        // Add the function type to the type section
        let type_index = self.add_function_type(params, return_type)?;
        
        // Add the function to the function section
        self.function_section.function(type_index);
        
        // Create a Function with the instructions
        let mut func = Function::new(vec![]); // No locals for stdlib functions
        for inst in instructions {
            func.instruction(inst);
        }
        func.instruction(&Instruction::End);
        
        // Add the function to the code section
        self.code_section.function(&func);
        
        // Add an export for the function
        self.export_section.export(name, wasm_encoder::ExportKind::Func, function_index);
        
        // Update other tracking data
        self.function_names.push(name.to_string());
        self.function_count += 1;
        
        // Return the function index
        Ok(function_index)
    }

    pub fn generate_error_handler(&mut self, stmt: &Statement, handler: &[Statement], location: &Option<SourceLocation>, instructions: &mut Vec<Instruction>) -> Result<(), CompilerError> {
        self.instruction_generator.generate_error_handler(stmt, handler, location, instructions)
    }

    pub fn ast_type_to_wasm_type(&self, ast_type: &Type) -> Result<WasmType, CompilerError> {
        self.type_manager.ast_type_to_wasm_type(ast_type)
    }

    pub fn allocate_string(&mut self, s: &str) -> Result<u32, CompilerError> {
        let result = self.memory_utils.allocate_string(s)?;
        Ok(result as u32)
    }

    pub fn allocate_array(&mut self, elements: &[Value]) -> Result<u32, CompilerError> {
        let result = self.memory_utils.allocate_array(elements)?;
        Ok(result as u32)
    }

    pub fn allocate_matrix(&mut self, data: &[f64], rows: usize, cols: usize) -> Result<u32, CompilerError> {
        // Create a matrix from the flat array data
        let matrix_data: Vec<Vec<f64>> = data
            .chunks(cols)
            .map(|chunk| chunk.to_vec())
            .collect();
        
        // Now call the memory utils allocate_matrix with the proper structure
        let result = self.memory_utils.allocate_matrix(&matrix_data)?;
        
        // Convert the usize result to u32
        Ok(result as u32)
    }

    pub fn retain_memory(&mut self, ptr: u32) -> Result<(), CompilerError> {
        self.memory_utils.retain(ptr as usize)
    }

    pub fn release_memory(&mut self, ptr: u32) -> Result<(), CompilerError> {
        self.memory_utils.release(ptr as usize)
    }
} 