//! Module for WebAssembly code generation.

use wasm_encoder::{
    BlockType, CodeSection, DataSection, ExportKind, ExportSection,
    Function, FunctionSection, GlobalSection, Instruction,
    MemorySection, Module, TypeSection, ValType,
    MemoryType, ImportSection, MemArg, ElementSection, TableSection,
};
use wasmparser::FuncType;

use crate::ast::{self, Program, Expression, Statement, Type, Value, Function as AstFunction, BinaryOperator, SourceLocation};
use crate::error::{CompilerError};

use crate::types::{WasmType};
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

/// Debug information for WebAssembly output
#[derive(Debug, Clone)]
pub struct DebugInfo {
    pub source_map: HashMap<u32, SourceLocation>,
    pub function_names: Vec<String>,
    pub local_names: HashMap<u32, Vec<String>>,
}

impl DebugInfo {
    pub fn new() -> Self {
        Self {
            source_map: HashMap::new(),
            function_names: Vec::new(),
            local_names: HashMap::new(),
        }
    }
    
    pub fn add_function_name(&mut self, index: u32, name: String) {
        if index as usize >= self.function_names.len() {
            self.function_names.resize(index as usize + 1, String::new());
        }
        self.function_names[index as usize] = name;
    }
    
    pub fn add_source_location(&mut self, instruction_offset: u32, location: SourceLocation) {
        self.source_map.insert(instruction_offset, location);
    }
    
    pub fn add_local_names(&mut self, function_index: u32, names: Vec<String>) {
        self.local_names.insert(function_index, names);
    }
}

/// Code generator for Clean Language
pub struct CodeGenerator {
    module: Module,
    function_section: FunctionSection,
    export_section: ExportSection,
    code_section: CodeSection,
    memory_section: MemorySection,
    type_section: TypeSection,
    data_section: DataSection,
    type_manager: TypeManager,
    instruction_generator: InstructionGenerator,
    variable_map: HashMap<String, LocalVarInfo>,
    memory_utils: MemoryUtils,
    function_count: u32,
    current_locals: Vec<LocalVarInfo>,
    function_map: HashMap<String, u32>,
    function_types: Vec<FuncType>,
    function_names: Vec<String>,
    debug_info: DebugInfo,
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
            memory_section: MemorySection::new(),
            type_section: TypeSection::new(),
            data_section: DataSection::new(),
            type_manager,
            instruction_generator,
            variable_map: HashMap::new(),
            memory_utils: MemoryUtils::new(HEAP_START),
            function_count: 0,
            current_locals: Vec::new(),
            function_map: HashMap::new(),
            function_types: Vec::new(),
            function_names: Vec::new(),
            debug_info: DebugInfo::new(),
        }
    }

    /// Generate the complete program
    pub fn generate(&mut self, program: &Program) -> Result<Vec<u8>, CompilerError> {
        // Clear previous state
        self.function_count = 0;
        self.function_map.clear();
        self.function_names.clear();
        self.function_types.clear();
        self.debug_info = DebugInfo::new();

        // ------------------------------------------------------------------
        // 1. Register standard library functions first
        // ------------------------------------------------------------------
        self.register_stdlib_functions()?;

        // ------------------------------------------------------------------
        // 2. Analyze and prepare all functions (including start function and class methods)
        // ------------------------------------------------------------------
        for function in &program.functions {
            self.prepare_function_type(function)?;
        }
        
        // Prepare class methods as static functions and constructors
        for class in &program.classes {
            // Prepare constructor if it exists
            if let Some(constructor) = &class.constructor {
                let constructor_function_name = format!("{}_constructor", class.name);
                let constructor_function = ast::Function::new(
                    constructor_function_name,
                    constructor.parameters.clone(),
                    Type::Object(class.name.clone()), // Constructor returns an object of this class
                    constructor.body.clone(),
                    constructor.location.clone(),
                );
                self.prepare_function_type(&constructor_function)?;
            }
            
            // Prepare class methods as static functions
            for method in &class.methods {
                let static_function_name = format!("{}_{}", class.name, method.name);
                let mut static_function = method.clone();
                static_function.name = static_function_name;
                self.prepare_function_type(&static_function)?;
            }
        }
        
        // Also process the start function if it exists
        if let Some(start_function) = &program.start_function {
            self.prepare_function_type(start_function)?;
        }

        // ------------------------------------------------------------------
        // 3. Generate function code (including start function and class methods)
        // ------------------------------------------------------------------
        for function in &program.functions {
            self.generate_function(function)?;
        }
        
        // Generate class methods as static functions and constructors
        for class in &program.classes {
            // Generate constructor if it exists
            if let Some(constructor) = &class.constructor {
                let constructor_function_name = format!("{}_constructor", class.name);
                let constructor_function = ast::Function::new(
                    constructor_function_name,
                    constructor.parameters.clone(),
                    Type::Object(class.name.clone()), // Constructor returns an object of this class
                    constructor.body.clone(),
                    constructor.location.clone(),
                );
                self.generate_function(&constructor_function)?;
            }
            
            // Generate class methods as static functions
            for method in &class.methods {
                let static_function_name = format!("{}_{}", class.name, method.name);
                let mut static_function = method.clone();
                static_function.name = static_function_name;
                self.generate_function(&static_function)?;
            }
        }
        
        // Also generate the start function if it exists
        if let Some(start_function) = &program.start_function {
            self.generate_function(start_function)?;
        }

        // ------------------------------------------------------------------
        // 4. Setup memory (1 page minimum for basic operations)
        // ------------------------------------------------------------------
        self.memory_section.memory(MemoryType {
            minimum: 1,
            maximum: Some(16), // Limit to 16 pages (1MB) for safety
            memory64: false,
            shared: false,
        });

        // ------------------------------------------------------------------
        // 5. Export the start function
        // ------------------------------------------------------------------
        if let Some(&start_index) = self.function_map.get("start") {
            self.export_section.export("start", ExportKind::Func, start_index);
            
            // Also export memory for debugging/inspection
            self.export_section.export("memory", ExportKind::Memory, 0);
        } else {
            return Err(CompilerError::codegen_error(
                "No 'start' function found in the program", 
                Some("Clean Language programs must have a 'start()' function as the entry point".to_string()), 
                None
            ));
        }

        // ------------------------------------------------------------------
        // 6. Assemble the final module
        // ------------------------------------------------------------------
        self.assemble_module()
    }

    /// Prepare function type information without generating code
    fn prepare_function_type(&mut self, function: &AstFunction) -> Result<(), CompilerError> {
        // Convert parameters to WasmType
        let params: Vec<WasmType> = function.parameters.iter()
            .map(|param| WasmType::from(&param.type_))
            .collect();
            
        // Convert return_type safely
        let return_type = if function.return_type == Type::Void {
            None
        } else {
            Some(WasmType::from(&function.return_type))
        };
        
        // Add function type
        let type_index = self.add_function_type(&params, return_type)?;
        
        // Add to function section
        self.function_section.function(type_index);
        
        // Track function index and name mapping
        let func_index = self.function_count;
        self.function_map.insert(function.name.clone(), func_index);
        self.function_names.push(function.name.clone());
        
        // Register function type with instruction generator
        let param_val_types: Vec<ValType> = params.iter().map(|t| (*t).into()).collect();
        let result_val_types: Vec<ValType> = return_type.map(|t| vec![t.into()]).unwrap_or_default();
        self.instruction_generator.add_function_type(func_index, param_val_types, result_val_types);
        
        // Add debug information
        self.debug_info.add_function_name(func_index, function.name.clone());
        
        // Increment function count for next function
        self.function_count += 1;

        Ok(())
    }

    /// Assemble the final WebAssembly module
    fn assemble_module(&mut self) -> Result<Vec<u8>, CompilerError> {
        let mut module = Module::new();

        // Add sections in the correct order
        if !self.function_types.is_empty() {
            // Use the type section that was already populated by the TypeManager
            module.section(&self.type_manager.clone_type_section());
        }
        
        if self.function_count > 0 {
            module.section(&self.function_section);
        }
        
        // Always add memory section
        module.section(&self.memory_section);
        
        // Add exports if any
        module.section(&self.export_section);
        
        // Add code section if we have functions
        if self.function_count > 0 {
            module.section(&self.code_section);
        }
        
        // Add data section if we have any data
        if !self.memory_utils.is_empty() {
            module.section(&self.data_section);
        }

        // Add debug information as custom sections
        self.add_debug_sections(&mut module)?;

        Ok(module.finish())
    }

    /// Add debugging information as custom sections
    fn add_debug_sections(&self, module: &mut Module) -> Result<(), CompilerError> {
        // Add function names section
        if !self.debug_info.function_names.is_empty() {
            let mut names_data = Vec::new();
            
            // Write function count
            names_data.extend_from_slice(&(self.debug_info.function_names.len() as u32).to_le_bytes());
            
            // Write each function name
            for (index, name) in self.debug_info.function_names.iter().enumerate() {
                names_data.extend_from_slice(&(index as u32).to_le_bytes());
                names_data.extend_from_slice(&(name.len() as u32).to_le_bytes());
                names_data.extend_from_slice(name.as_bytes());
            }
            
            // Add as custom section
            module.section(&wasm_encoder::CustomSection {
                name: "name".into(),
                data: names_data.as_slice().into(),
            });
        }

        // Add source map information
        if !self.debug_info.source_map.is_empty() {
            let mut source_map_data = Vec::new();
            
            // Write source map count
            source_map_data.extend_from_slice(&(self.debug_info.source_map.len() as u32).to_le_bytes());
            
            // Write each source location
            for (offset, location) in &self.debug_info.source_map {
                source_map_data.extend_from_slice(&offset.to_le_bytes());
                source_map_data.extend_from_slice(&(location.line as u32).to_le_bytes());
                source_map_data.extend_from_slice(&(location.column as u32).to_le_bytes());
                
                // Write filename
                let filename = location.file.as_bytes();
                source_map_data.extend_from_slice(&(filename.len() as u32).to_le_bytes());
                source_map_data.extend_from_slice(filename);
            }
            
            // Add as custom section
            module.section(&wasm_encoder::CustomSection {
                name: "sourceMappingURL".into(),
                data: source_map_data.as_slice().into(),
            });
        }

        Ok(())
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
        // Reset locals for this function
        self.current_locals.clear();
        self.variable_map.clear();
        
        // Add parameters as locals
        for param in &function.parameters {
            let local_info = LocalVarInfo {
                index: self.current_locals.len() as u32,
                type_: WasmType::from(&param.type_).into(),
            };
            self.current_locals.push(local_info.clone());
            self.variable_map.insert(param.name.clone(), local_info);
        }
        
        // Generate function body
        let mut instructions = Vec::new();
        
        // Check if the function has a non-void return type
        let needs_return_value = function.return_type != Type::Void;
        
        // Handle function body with implicit return logic
        if !function.body.is_empty() {
            // Generate all statements except the last one normally
            for stmt in &function.body[..function.body.len().saturating_sub(1)] {
                self.generate_statement(stmt, &mut instructions)?;
            }
            
            // Handle the last statement specially for implicit returns
            if let Some(last_stmt) = function.body.last() {
                match last_stmt {
                    Statement::Expression { expr, .. } => {
                        // For expression statements as the last statement, treat as implicit return
                        // unless the function return type is Void
                        if function.return_type == Type::Void {
                            // If function returns void, generate the expression but drop the value
                            self.generate_expression(expr, &mut instructions)?;
                            instructions.push(Instruction::Drop);
                } else {
                            // If function has a return type, use the expression as return value
                            self.generate_expression(expr, &mut instructions)?;
                            // Don't add explicit return instruction - WASM functions implicitly return the top stack value
                        }
                    },
                    Statement::Return { .. } => {
                        // For explicit return statements, generate normally
                        self.generate_statement(last_stmt, &mut instructions)?;
                    },
                    _ => {
                        // For non-expression, non-return statements, generate normally
                        self.generate_statement(last_stmt, &mut instructions)?;
                        
                        // If the function has a non-void return type and the last statement isn't a return,
                        // we need to add a default return value
                        if needs_return_value {
                            match function.return_type {
                                Type::Integer => instructions.push(Instruction::I32Const(0)),
                                Type::Float => instructions.push(Instruction::F64Const(0.0)),
                                Type::Boolean => instructions.push(Instruction::I32Const(0)),
                                _ => instructions.push(Instruction::I32Const(0)), // Default for other types
                            }
                        }
                    }
                }
            }
        } else {
            // Empty function body - add default return if needed
            if needs_return_value {
                match function.return_type {
                    Type::Integer => instructions.push(Instruction::I32Const(0)),
                    Type::Float => instructions.push(Instruction::F64Const(0.0)),
                    Type::Boolean => instructions.push(Instruction::I32Const(0)),
                    _ => {
                        return Err(CompilerError::codegen_error(
                            format!("Cannot generate default return value for type {:?}", function.return_type),
                            None, None
                        ));
                    }
                }
            }
        }

        // Create function with locals and instructions
        let locals = self.current_locals.iter()
            .skip(function.parameters.len()) // Skip parameters, they're not locals
            .map(|local| (1u32, local.type_))
            .collect::<Vec<_>>();

        let mut func = Function::new(locals);
        for instruction in instructions {
            func.instruction(&instruction);
        }
        
        // Always add End instruction for user-defined functions
        func.instruction(&Instruction::End);

        // Add to code section
        self.code_section.function(&func);

        Ok(())
    }

    /// Extract source location from a statement for debugging
    fn get_statement_location(&self, stmt: &Statement) -> Option<SourceLocation> {
        match stmt {
            Statement::VariableDecl { location, .. } => location.clone(),
            Statement::Assignment { location, .. } => location.clone(),
            Statement::Print { location, .. } => location.clone(),
            Statement::PrintBlock { location, .. } => location.clone(),
            Statement::Return { location, .. } => location.clone(),
            Statement::If { location, .. } => location.clone(),
            Statement::Iterate { location, .. } => location.clone(),
            Statement::Test { location, .. } => location.clone(),
            _ => Some(SourceLocation::default()),
        }
    }

    pub fn generate_statement(&mut self, stmt: &Statement, instructions: &mut Vec<Instruction>) -> Result<(), CompilerError> {
        match stmt {
            Statement::VariableDecl { name, type_, initializer, location } => {
                let specified_type = Some(WasmType::from(type_));
                
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
            Statement::Print { expression, newline, location: _ } => {
                self.generate_expression(expression, instructions)?;
                instructions.push(Instruction::Call(
                    if *newline {
                        self.get_printl_function_index()
                    } else {
                        self.get_print_function_index()
                    }
                ));
            }
            Statement::PrintBlock { expressions, newline, location: _ } => {
                for expression in expressions {
                    self.generate_expression(expression, instructions)?;
                    instructions.push(Instruction::Call(
                        if *newline {
                            self.get_printl_function_index()
                        } else {
                            self.get_print_function_index()
                        }
                    ));
                }
            }
            Statement::Return { value, location: _ } => {
                if let Some(expr) = value {
                self.generate_expression(expr, instructions)?;
                }
                instructions.push(Instruction::Return);
            }
            Statement::If { condition, then_branch, else_branch, location: _ } => {
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
            Statement::Iterate { iterator, collection, body, location: _ } => {
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
            Statement::Test { name: _, body, location: _ } => {
                #[cfg(test)]
                for stmt in body {
                    self.generate_statement(stmt, instructions)?;
                }
            }
            Statement::Expression { expr, location: _ } => {
                self.generate_expression(expr, instructions)?;
                instructions.push(Instruction::Drop);
            }
            Statement::TypeApplyBlock { type_, assignments, location: _ } => {
                // Generate variable declarations
                for assignment in assignments {
                    if let Some(init_expr) = &assignment.initializer {
                        self.generate_expression(init_expr, instructions)?;
                        let local_index = self.current_locals.len() as u32;
                        let wasm_type = self.ast_type_to_wasm_type(type_)?;
                        
                        self.current_locals.push(LocalVarInfo {
                            index: local_index,
                            type_: wasm_type.into(),
                        });
                        self.variable_map.insert(assignment.name.clone(), LocalVarInfo {
                            index: local_index,
                            type_: wasm_type.into(),
                        });
                        
                        instructions.push(Instruction::LocalSet(local_index));
                    }
                }
            },
            
            Statement::FunctionApplyBlock { function_name, expressions, location: _ } => {
                // Generate multiple function calls with the same function
                for expr in expressions {
                    if let Some(func_index) = self.get_function_index(function_name) {
                        self.generate_expression(expr, instructions)?;
                        instructions.push(Instruction::Call(func_index));
                        instructions.push(Instruction::Drop); // Drop return value if any
                    }
                }
            },
            
            Statement::ConstantApplyBlock { constants, location: _ } => {
                // Generate constant declarations (treated as variables in WASM)
                for constant in constants {
                    let local_index = self.current_locals.len() as u32;
                    let wasm_type = self.ast_type_to_wasm_type(&constant.type_)?;
                    
                    self.generate_expression(&constant.value, instructions)?;
                    
                    self.current_locals.push(LocalVarInfo {
                        index: local_index,
                        type_: wasm_type.into(),
                    });
                    self.variable_map.insert(constant.name.clone(), LocalVarInfo {
                        index: local_index,
                        type_: wasm_type.into(),
                    });
                    
                    instructions.push(Instruction::LocalSet(local_index));
                }
            },
            Statement::RangeIterate { iterator, start, end, step, body, location: _ } => {
                // Similar to regular iterate but with range
                let counter_index = self.current_locals.len() as u32;
                self.current_locals.push(LocalVarInfo {
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
                
                self.variable_map.insert(iterator.clone(), LocalVarInfo {
                    index: counter_index,
                    type_: ValType::I32.into(),
                });
                
                // Generate start value
                self.generate_expression(start, instructions)?;
                instructions.push(Instruction::LocalSet(counter_index));
                
                // Generate end value
                self.generate_expression(end, instructions)?;
                instructions.push(Instruction::LocalSet(end_index));
                
                // Generate step value (default to 1 if None)
                if let Some(step_expr) = step {
                    self.generate_expression(step_expr, instructions)?;
                } else {
                    instructions.push(Instruction::I32Const(1));
                }
                instructions.push(Instruction::LocalSet(step_index));
                
                // Loop structure
                instructions.push(Instruction::Block(BlockType::Empty));
                instructions.push(Instruction::Loop(BlockType::Empty));
                
                // Check loop condition
                instructions.push(Instruction::LocalGet(counter_index));
                instructions.push(Instruction::LocalGet(end_index));
                instructions.push(Instruction::I32LtS);
                instructions.push(Instruction::I32Eqz);
                instructions.push(Instruction::BrIf(1));
                
                // Execute loop body
                for stmt in body {
                    self.generate_statement(stmt, instructions)?;
                }
                
                // Increment counter
                instructions.push(Instruction::LocalGet(counter_index));
                instructions.push(Instruction::LocalGet(step_index));
                instructions.push(Instruction::I32Add);
                instructions.push(Instruction::LocalSet(counter_index));
                
                instructions.push(Instruction::Br(0));
                instructions.push(Instruction::End);
                instructions.push(Instruction::End);
                
                self.variable_map.remove(iterator);
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
                        loc.unwrap_or_default()
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
                        loc.unwrap_or_default()
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
            Expression::MethodCall { object, method, arguments, location: _ } => {
                // Check if this is a static method call on a built-in class first
                if let Expression::Variable(class_name) = object.as_ref() {
                    // Try to handle as built-in static method call
                    if let Some(result_type) = self.generate_builtin_static_method_call(
                        class_name, 
                        method, 
                        arguments, 
                        instructions
                    )? {
                        return Ok(result_type);
                    }
                }
                
                // Handle method calls on different types (instance methods)
                self.generate_expression(&*object, instructions)?;
                
                // Generate arguments
                for arg in arguments {
                    self.generate_expression(arg, instructions)?;
                }
                
                // Handle specific methods based on method name
                match method.as_str() {
                    "at" => {
                        // Array.at(index) - 1-indexed access
                        // Convert 1-indexed to 0-indexed by subtracting 1
                        instructions.push(Instruction::I32Const(1));
                        instructions.push(Instruction::I32Sub);
                        instructions.push(Instruction::Call(self.get_array_get()));
                        Ok(WasmType::I32)
                    },
                    "length" => {
                        // Array.length() - get array length
                        instructions.push(Instruction::Call(self.get_array_length()));
                        Ok(WasmType::I32)
                    },
                    _ => {
                        // Try to find a function with the method name
                        if let Some(method_index) = self.get_function_index(&format!("{}_{}", "array", method)) {
                            instructions.push(Instruction::Call(method_index));
                            Ok(WasmType::I32) // Default return type
                        } else {
                            Err(CompilerError::codegen_error(
                                &format!("Method '{}' not found", method), 
                                None, 
                                None
                            ))
                        }
                    }
                }
            },
            Expression::MatrixAccess(matrix, row, col) => {
                self.generate_expression(&*matrix, instructions)?;
                self.generate_expression(&*row, instructions)?;
                self.generate_expression(&*col, instructions)?;
                instructions.push(Instruction::Call(self.get_matrix_get())); 
                Ok(WasmType::F64)
            },
            Expression::StringInterpolation(parts) => {
                // Handle string interpolation by concatenating parts
                if parts.is_empty() {
                    // Empty interpolation, return empty string
                    let string_ptr = self.allocate_string("")?;
                    instructions.push(Instruction::I32Const(string_ptr as i32));
                    return Ok(WasmType::I32);
                }
                
                // Start with the first part
                let mut first = true;
                for part in parts {
                    match part {
                        ast::StringPart::Text(text) => {
                            // Allocate string literal
                            let string_ptr = self.allocate_string(text)?;
                            instructions.push(Instruction::I32Const(string_ptr as i32));
                        },
                        ast::StringPart::Interpolation(expr) => {
                            // Generate the expression and convert to string if needed
                            let expr_type = self.generate_expression(expr, instructions)?;
                            
                            // Convert to string if not already a string
                            match expr_type {
                                WasmType::I32 => {
                                    // If it's an integer, convert to string
                                    // For now, assume it's already a string pointer
                                    // TODO: Add proper type checking and conversion
                                },
                                WasmType::F64 => {
                                    // Convert float to string
                                    // TODO: Add float to string conversion
                                    return Err(CompilerError::codegen_error(
                                        "Float to string conversion in interpolation not yet implemented", 
                                        None, 
                                        None
                                    ));
                                },
                                _ => {
                                    return Err(CompilerError::codegen_error(
                                        "Unsupported type in string interpolation", 
                                        None, 
                                        None
                                    ));
                                }
                            }
                        }
                    }
                    
                    // Concatenate with previous parts (except for the first)
                    if !first {
                        // Call string concatenation function
                        instructions.push(Instruction::Call(self.get_string_concat_index()?));
                    }
                    first = false;
                }
                
                Ok(WasmType::I32) // String type is represented as I32 pointer
            },
            Expression::ObjectCreation { class_name, arguments, location: _ } => {
                // Handle object creation (constructor calls)
                
                // Generate arguments
                for arg in arguments {
                    self.generate_expression(arg, instructions)?;
                }
                
                // Create constructor function name
                let constructor_name = format!("{}_constructor", class_name);
                
                // Find the constructor function index
                if let Some(constructor_index) = self.get_function_index(&constructor_name) {
                    instructions.push(Instruction::Call(constructor_index));
                    // Constructor returns an object (represented as I32 pointer)
                    Ok(WasmType::I32)
                } else {
                    Err(CompilerError::codegen_error(
                        &format!("Constructor for class '{}' not found", class_name), 
                        Some("Make sure the class has a constructor defined".to_string()), 
                        None
                    ))
                }
            },
            Expression::StaticMethodCall { class_name, method, arguments, location: _ } => {
                // Handle static method calls - ClassName.method()
                
                // Check if this is a built-in system class first
                if let Some(return_type) = self.generate_builtin_static_method_call(class_name, method, arguments, instructions)? {
                    return Ok(return_type);
                }
                
                // Generate arguments for user-defined static methods
                for arg in arguments {
                    self.generate_expression(arg, instructions)?;
                }
                
                // Create function name from class and method
                let function_name = format!("{}_{}", class_name, method);
                
                // Find the function index
                if let Some(method_index) = self.get_function_index(&function_name) {
                    instructions.push(Instruction::Call(method_index));
                    // Get the return type from the function
                    self.get_function_return_type(method_index)
                } else {
                    Err(CompilerError::codegen_error(
                        &format!("Static method '{}' in class '{}' not found", method, class_name), 
                        Some("Make sure the method is defined in the class".to_string()), 
                        None
                    ))
                }
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
                Expression::Literal(Value::Integer(0)) => {
                    return Err(CompilerError::division_by_zero_error(None));
                },
                Expression::Literal(Value::Float(n)) if *n == 0.0 => {
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
            Expression::Variable(_name) => { /* ... */ false } // Needs type lookup
            Expression::StringInterpolation(_) => true,
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
            Value::Float(n) => {
                instructions.push(Instruction::F64Const(*n));
                Ok(WasmType::F64)
            },
            Value::Integer(i) => {
                instructions.push(Instruction::I32Const((*i).try_into().unwrap()));
                Ok(WasmType::I32)
            },
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
        // Re-enable stdlib functions using the same approach as user-defined functions
        // This avoids the validation issues we had with the register_function approach
        
        // 1. Create stdlib function definitions using AstFunction
        let stdlib_functions = self.create_stdlib_ast_functions()?;
        
        // 2. Process them like regular user functions
        for function in &stdlib_functions {
            self.prepare_function_type(function)?;
        }
        
        // 3. Generate their code
        for function in &stdlib_functions {
            self.generate_function(function)?;
        }
        
        Ok(())
    }
    
    /// Create AST function definitions for stdlib functions
    fn create_stdlib_ast_functions(&self) -> Result<Vec<ast::Function>, CompilerError> {
        use crate::ast::{Function as AstFunction, Parameter, Statement, Expression, Value, Type, FunctionSyntax, Visibility};
        
        let mut functions = Vec::new();
        
        // abs(value: Integer) -> Integer
        functions.push(AstFunction {
            name: "abs".to_string(),
            type_parameters: vec![],
            parameters: vec![
                Parameter {
                    name: "value".to_string(),
                    type_: Type::Integer,
                }
            ],
            return_type: Type::Integer,
            body: vec![
                // if value < 0 then -value else value
                Statement::If {
                    condition: Expression::Binary(
                        Box::new(Expression::Variable("value".to_string())),
                        ast::BinaryOperator::Less,
                        Box::new(Expression::Literal(Value::Integer(0)))
                    ),
                    then_branch: vec![
                        Statement::Return {
                            value: Some(Expression::Binary(
                                Box::new(Expression::Literal(Value::Integer(0))),
                                ast::BinaryOperator::Subtract,
                                Box::new(Expression::Variable("value".to_string()))
                            )),
                            location: None,
                        }
                    ],
                    else_branch: Some(vec![
                        Statement::Return {
                            value: Some(Expression::Variable("value".to_string())),
                            location: None,
                        }
                    ]),
                    location: None,
                }
            ],
            description: Some("Returns the absolute value of an integer".to_string()),
            syntax: FunctionSyntax::Simple,
            visibility: Visibility::Public,
            location: None,
        });
        
        // print(value: Integer) -> Void  
        functions.push(AstFunction {
            name: "print".to_string(),
            type_parameters: vec![],
            parameters: vec![
                Parameter {
                    name: "value".to_string(),
                    type_: Type::Integer,
                }
            ],
            return_type: Type::Void,
            body: vec![
                // This is a placeholder - actual printing would need host function import
                // For now, just drop the value to make it a valid void function
                Statement::Expression {
                    expr: Expression::Variable("value".to_string()),
                    location: None,
                }
            ],
            description: Some("Prints an integer value".to_string()),
            syntax: FunctionSyntax::Simple,
            visibility: Visibility::Public,
            location: None,
        });
        
        // printl(value: Integer) -> Void (print with newline)
        functions.push(AstFunction {
            name: "printl".to_string(),
            type_parameters: vec![],
            parameters: vec![
                Parameter {
                    name: "value".to_string(),
                    type_: Type::Integer,
                }
            ],
            return_type: Type::Void,
            body: vec![
                // This is a placeholder - actual printing would need host function import
                // For now, just drop the value to make it a valid void function
                Statement::Expression {
                    expr: Expression::Variable("value".to_string()),
                    location: None,
                }
            ],
            description: Some("Prints an integer value with newline".to_string()),
            syntax: FunctionSyntax::Simple,
            visibility: Visibility::Public,
            location: None,
        });
        
        // array_get(array: Array, index: Integer) -> Integer
        functions.push(AstFunction {
            name: "array_get".to_string(),
            type_parameters: vec![],
            parameters: vec![
                Parameter {
                    name: "array".to_string(),
                    type_: Type::Array(Box::new(Type::Integer)),
                },
                Parameter {
                    name: "index".to_string(),
                    type_: Type::Integer,
                }
            ],
            return_type: Type::Integer,
            body: vec![
                // Placeholder implementation - would need memory operations
                // Return 0 for now
                Statement::Return {
                    value: Some(Expression::Literal(Value::Integer(0))),
                    location: None,
                }
            ],
            description: Some("Gets an element from an array".to_string()),
            syntax: FunctionSyntax::Simple,
            visibility: Visibility::Public,
            location: None,
        });
        
        // array_length(array: Array) -> Integer
        functions.push(AstFunction {
            name: "array_length".to_string(),
            type_parameters: vec![],
            parameters: vec![
                Parameter {
                    name: "array".to_string(),
                    type_: Type::Array(Box::new(Type::Integer)),
                }
            ],
            return_type: Type::Integer,
            body: vec![
                // Placeholder implementation - would need memory operations
                // Return 0 for now
                Statement::Return {
                    value: Some(Expression::Literal(Value::Integer(0))),
                    location: None,
                }
            ],
            description: Some("Gets the length of an array".to_string()),
            syntax: FunctionSyntax::Simple,
            visibility: Visibility::Public,
            location: None,
        });
        
        // string_concat(str1: String, str2: String) -> String
        functions.push(AstFunction {
            name: "string_concat".to_string(),
            type_parameters: vec![],
            parameters: vec![
                Parameter {
                    name: "str1".to_string(),
                    type_: Type::String,
                },
                Parameter {
                    name: "str2".to_string(),
                    type_: Type::String,
                }
            ],
            return_type: Type::String,
            body: vec![
                // Placeholder implementation - would need memory operations
                // Return first string for now
                Statement::Return {
                    value: Some(Expression::Variable("str1".to_string())),
                    location: None,
                }
            ],
            description: Some("Concatenates two strings".to_string()),
            syntax: FunctionSyntax::Simple,
            visibility: Visibility::Public,
            location: None,
        });
        
        // string_compare(str1: String, str2: String) -> Integer
        functions.push(AstFunction {
            name: "string_compare".to_string(),
            type_parameters: vec![],
            parameters: vec![
                Parameter {
                    name: "str1".to_string(),
                    type_: Type::String,
                },
                Parameter {
                    name: "str2".to_string(),
                    type_: Type::String,
                }
            ],
            return_type: Type::Integer,
            body: vec![
                // Placeholder implementation - would need memory operations
                // Return 0 (equal) for now
                Statement::Return {
                    value: Some(Expression::Literal(Value::Integer(0))),
                    location: None,
                }
            ],
            description: Some("Compares two strings".to_string()),
            syntax: FunctionSyntax::Simple,
            visibility: Visibility::Public,
            location: None,
        });
        
        Ok(functions)
    }

    // Add delegate methods to use instruction_generator
    // These should be part of the CodeGenerator implementation

    pub fn find_local(&self, name: &str) -> Option<LocalVarInfo> {
        self.variable_map.get(name).cloned()
    }

    pub fn get_function_index(&self, name: &str) -> Option<u32> {
        // First check our function_map which contains stdlib functions
        if let Some(&index) = self.function_map.get(name) {
            return Some(index);
        }
        
        // Fallback to instruction_generator for compatibility
        self.instruction_generator.get_function_index(name)
    }

    pub fn get_function_return_type(&self, index: u32) -> Result<WasmType, CompilerError> {
        self.instruction_generator.get_function_return_type(index)
    }

    pub fn get_array_get(&self) -> u32 {
        self.function_map.get("array_get").copied().unwrap_or(0)
    }

    pub fn get_array_length(&self) -> u32 {
        self.function_map.get("array_length").copied().unwrap_or(0)
    }

    pub fn get_matrix_get(&self) -> u32 {
        self.function_map.get("matrix_get").copied().unwrap_or(0)
    }

    pub fn get_print_function_index(&self) -> u32 {
        self.function_map.get("print").copied().unwrap_or(0)
    }

    pub fn get_printl_function_index(&self) -> u32 {
        self.function_map.get("printl").copied().unwrap_or(0)
    }

    pub fn register_function(&mut self, name: &str, params: &[WasmType], return_type: Option<WasmType>, 
        instructions: &[Instruction]) -> Result<u32, CompilerError>
    {
        // Get the current function index (this will be the index for the new function)
        let function_index = self.function_count;
        
        // Register with instruction_generator for internal tracking
        self.instruction_generator.register_function(name, params, return_type, instructions)?;
        
        // Add the function type to the type section
        let type_index = self.add_function_type(params, return_type)?;
        
        // Add the function to the function section
        self.function_section.function(type_index);
        
        // Create a Function - parameters are automatically available as locals 0, 1, 2, ...
        // No additional locals needed for simple stdlib functions
        let mut func = Function::new(vec![]); 
        for inst in instructions {
            func.instruction(inst);
        }
        
        // Always add End instruction for stdlib functions (they don't include it in their definitions)
        func.instruction(&Instruction::End);
        
        // Add the function to the code section
        self.code_section.function(&func);
        
        // Do NOT add exports for stdlib functions - they are for internal use only
        // self.export_section.export(name, wasm_encoder::ExportKind::Func, function_index);
        
        // Update other tracking data
        self.function_names.push(name.to_string());
        self.function_count += 1;
        
        // Return the function index
        Ok(function_index)
    }

    pub fn generate_error_handler_blocks(&mut self, try_block: &[Statement], error_variable: Option<&str>, catch_block: &[Statement], _location: &Option<SourceLocation>, instructions: &mut Vec<Instruction>) -> Result<(), CompilerError> {
        // For now, implement a simple try-catch mechanism using WASM's try-catch instructions
        // Note: Full exception handling in WASM requires the exception handling proposal
        
        // Generate try block instructions
        let mut try_instructions = Vec::new();
        for stmt in try_block {
            self.generate_statement(stmt, &mut try_instructions)?;
        }
        
        // For now, we'll implement a simplified version without actual exception handling
        // In a full implementation, this would use WASM's try-catch instructions
        
        // Add the try block instructions directly
        instructions.extend(try_instructions);
        
        // TODO: Implement proper exception handling when WASM exception handling is stable
        // For now, we just execute the try block and ignore the catch block
        
        Ok(())
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

    pub fn allocate_matrix(&mut self, data: &[f64], _rows: usize, cols: usize) -> Result<u32, CompilerError> {
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

    fn generate_builtin_static_method_call(
        &mut self,
        class_name: &str,
        method: &str,
        arguments: &[Expression],
        instructions: &mut Vec<Instruction>
    ) -> Result<Option<WasmType>, CompilerError> {
        match class_name {
            "MathUtils" => {
                match method {
                    "add" => {
                        // Generate arguments
                        let arg1_type = self.generate_expression(&arguments[0], instructions)?;
                        let arg2_type = self.generate_expression(&arguments[1], instructions)?;
                        
                        // Add based on types
                        match (arg1_type, arg2_type) {
                            (WasmType::I32, WasmType::I32) => {
                                instructions.push(Instruction::I32Add);
                                Ok(Some(WasmType::I32))
                            },
                            (WasmType::F64, WasmType::F64) => {
                                instructions.push(Instruction::F64Add);
                                Ok(Some(WasmType::F64))
                            },
                            (WasmType::I32, WasmType::F64) => {
                                // Convert first argument to float
                                instructions.insert(instructions.len() - 2, Instruction::F64ConvertI32S);
                                instructions.push(Instruction::F64Add);
                                Ok(Some(WasmType::F64))
                            },
                            (WasmType::F64, WasmType::I32) => {
                                // Convert second argument to float
                                instructions.push(Instruction::F64ConvertI32S);
                                instructions.push(Instruction::F64Add);
                                Ok(Some(WasmType::F64))
                            },
                            _ => Err(CompilerError::codegen_error(
                                "MathUtils.add requires numeric arguments".to_string(),
                                None,
                                None
                            ))
                        }
                    },
                    "subtract" => {
                        let arg1_type = self.generate_expression(&arguments[0], instructions)?;
                        let arg2_type = self.generate_expression(&arguments[1], instructions)?;
                        
                        match (arg1_type, arg2_type) {
                            (WasmType::I32, WasmType::I32) => {
                                instructions.push(Instruction::I32Sub);
                                Ok(Some(WasmType::I32))
                            },
                            (WasmType::F64, WasmType::F64) => {
                                instructions.push(Instruction::F64Sub);
                                Ok(Some(WasmType::F64))
                            },
                            (WasmType::I32, WasmType::F64) => {
                                instructions.insert(instructions.len() - 2, Instruction::F64ConvertI32S);
                                instructions.push(Instruction::F64Sub);
                                Ok(Some(WasmType::F64))
                            },
                            (WasmType::F64, WasmType::I32) => {
                                instructions.push(Instruction::F64ConvertI32S);
                                instructions.push(Instruction::F64Sub);
                                Ok(Some(WasmType::F64))
                            },
                            _ => Err(CompilerError::codegen_error(
                                "MathUtils.subtract requires numeric arguments".to_string(),
                                None,
                                None
                            ))
                        }
                    },
                    "multiply" => {
                        let arg1_type = self.generate_expression(&arguments[0], instructions)?;
                        let arg2_type = self.generate_expression(&arguments[1], instructions)?;
                        
                        match (arg1_type, arg2_type) {
                            (WasmType::I32, WasmType::I32) => {
                                instructions.push(Instruction::I32Mul);
                                Ok(Some(WasmType::I32))
                            },
                            (WasmType::F64, WasmType::F64) => {
                                instructions.push(Instruction::F64Mul);
                                Ok(Some(WasmType::F64))
                            },
                            (WasmType::I32, WasmType::F64) => {
                                instructions.insert(instructions.len() - 2, Instruction::F64ConvertI32S);
                                instructions.push(Instruction::F64Mul);
                                Ok(Some(WasmType::F64))
                            },
                            (WasmType::F64, WasmType::I32) => {
                                instructions.push(Instruction::F64ConvertI32S);
                                instructions.push(Instruction::F64Mul);
                                Ok(Some(WasmType::F64))
                            },
                            _ => Err(CompilerError::codegen_error(
                                "MathUtils.multiply requires numeric arguments".to_string(),
                                None,
                                None
                            ))
                        }
                    },
                    _ => Ok(None), // Method not found in MathUtils
                }
            },
            "StringUtils" => {
                match method {
                    "concat" => {
                        // Generate arguments
                        self.generate_expression(&arguments[0], instructions)?;
                        self.generate_expression(&arguments[1], instructions)?;
                        
                        // Call string concatenation function
                        let string_concat_index = self.get_string_concat_index()?;
                        instructions.push(Instruction::Call(string_concat_index));
                        Ok(Some(WasmType::I32)) // String is represented as I32 pointer
                    },
                    _ => Ok(None), // Method not found in StringUtils
                }
            },
            _ => Ok(None), // Class not found in built-ins
        }
    }

    /// Finalize and return the WebAssembly binary
    pub fn finish(&self) -> Vec<u8> {
        // This method is kept for compatibility, but the new approach
        // generates the binary directly in the generate() method
        // For now, return an empty vector as a placeholder
        vec![]
    }
} 