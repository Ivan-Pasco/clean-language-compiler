//! Module for WebAssembly code generation.

use wasm_encoder::{
    BlockType, CodeSection, DataSection, ExportKind, ExportSection,
    Function, FunctionSection, GlobalSection, Instruction,
    MemorySection, Module, TypeSection, ValType,
    MemoryType, ImportSection, MemArg, ElementSection, TableSection,
};
use wasmparser::FuncType;

use crate::ast::{self, Program, Expression, Statement, Type, Value, Function as AstFunction, BinaryOperator, SourceLocation, Class};
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
pub const PAIRS_TYPE_ID: u32 = 6;

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
    import_section: ImportSection,
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
    symbol_table: HashMap<String, u32>,
    function_table: HashMap<String, u32>,
    file_import_indices: HashMap<String, u32>,
    http_import_indices: HashMap<String, u32>,
    label_counter: u32,
    
    // Class and inheritance support
    current_class_context: Option<String>,
    class_field_map: HashMap<String, HashMap<String, (Type, u32)>>, // class_name -> (field_name -> (type, offset))
    class_table: HashMap<String, Class>,
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
            import_section: ImportSection::new(),
            type_manager,
            instruction_generator,
            variable_map: HashMap::new(),
            memory_utils: MemoryUtils::new(1024), // Start at 1KB instead of 64KB
            function_count: 0,
            current_locals: Vec::new(),
            function_map: HashMap::new(),
            function_types: Vec::new(),
            function_names: Vec::new(),
            debug_info: DebugInfo::new(),
            symbol_table: HashMap::new(),
            function_table: HashMap::new(),
            file_import_indices: HashMap::new(),
            http_import_indices: HashMap::new(),
            label_counter: 0,
            
            // Class and inheritance support
            current_class_context: None,
            class_field_map: HashMap::new(),
            class_table: HashMap::new(),
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
        // 1. Register imports FIRST (they get indices 0-13)
        // ------------------------------------------------------------------
        
        // 1.1. Register print function imports
        self.register_print_imports()?;

        // 1.2. Register file system imports
        self.register_file_imports()?;

        // 1.3. Register HTTP client imports
        self.register_http_imports()?;

        // ------------------------------------------------------------------
        // 2. Register standard library functions AFTER imports (they get indices 14+)
        // ------------------------------------------------------------------
        self.register_stdlib_functions()?;

        // ------------------------------------------------------------------
        // 3. Store class information and setup field maps
        // ------------------------------------------------------------------
        for class in &program.classes {
            self.class_table.insert(class.name.clone(), class.clone());
            
            // Build field map with offsets - for simple inheritance, inherit parent fields first
            let mut field_map = HashMap::new();
            let mut field_offset = 0u32;
            
            // Add parent class fields first (if any)
            if let Some(base_class_name) = &class.base_class {
                if let Some(base_class) = program.classes.iter().find(|c| c.name == *base_class_name) {
                    for field in &base_class.fields {
                        field_map.insert(field.name.clone(), (field.type_.clone(), field_offset));
                        field_offset += 4; // Simple 4-byte offset for all fields (treating everything as i32 for now)
                    }
                }
            }
            
            // Add this class's fields
            for field in &class.fields {
                field_map.insert(field.name.clone(), (field.type_.clone(), field_offset));
                field_offset += 4; // Simple 4-byte offset for all fields
            }
            
            self.class_field_map.insert(class.name.clone(), field_map);
        }

        // ------------------------------------------------------------------
        // 4. Analyze and prepare all functions (including start function and class methods)
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
        // 4. Generate function code (including start function and class methods)
        // ------------------------------------------------------------------
        for function in &program.functions {
            self.generate_function(function)?;
        }
        
        // Generate class methods as static functions and constructors
        for class in &program.classes {
            // Generate constructor if it exists
            if let Some(constructor) = &class.constructor {
                // Set class context for constructor generation
                self.current_class_context = Some(class.name.clone());
                
                let constructor_function_name = format!("{}_constructor", class.name);
                let constructor_function = ast::Function::new(
                    constructor_function_name,
                    constructor.parameters.clone(),
                    Type::Object(class.name.clone()), // Constructor returns an object of this class
                    constructor.body.clone(),
                    constructor.location.clone(),
                );
                self.generate_function(&constructor_function)?;
                
                // Clear class context
                self.current_class_context = None;
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
        // 5. Setup memory (1 page minimum for basic operations)
        // ------------------------------------------------------------------
        self.memory_section.memory(MemoryType {
            minimum: 1,
            maximum: Some(16), // Limit to 16 pages (1MB) for safety
            memory64: false,
            shared: false,
        });

        // ------------------------------------------------------------------
        // 6. Export the start function
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
        // 7. Assemble the final module
        // ------------------------------------------------------------------
        self.assemble_module()
    }

    /// Prepare function type information without generating code
    fn prepare_function_type(&mut self, function: &AstFunction) -> Result<(), CompilerError> {
        // Convert parameter types to WebAssembly types
        let param_types: Vec<WasmType> = function.parameters.iter()
            .map(|p| self.ast_type_to_wasm_type(&p.type_))
            .collect::<Result<Vec<WasmType>, CompilerError>>()?;
        
        // Convert return type to WebAssembly type
        let return_type = if function.return_type == Type::Void {
            None
        } else {
            Some(self.ast_type_to_wasm_type(&function.return_type)?)
        };
        
        // Add function type to type section
        let type_index = self.add_function_type(&param_types, return_type)?;
        
        // Add function to function section
        self.function_section.function(type_index);
        
        // Store function information
        self.function_map.insert(function.name.clone(), self.function_count);
        self.function_names.push(function.name.clone());
        // Store function type information for later use
        // Note: We'll use our own FuncType struct instead of wasmparser's
        
        // Add debug information
        self.debug_info.add_function_name(self.function_count, function.name.clone());
        
        self.function_count += 1;
        Ok(())
    }

    fn ast_type_to_wasm_type(&self, ast_type: &Type) -> Result<WasmType, CompilerError> {
        match ast_type {
            Type::Boolean => Ok(WasmType::I32),
            Type::Integer => Ok(WasmType::I32),
            Type::Float => Ok(WasmType::F64),
            Type::String => Ok(WasmType::I32), // String pointers
            Type::Void => Ok(WasmType::I32),   // Void represented as I32
            Type::Array(_) => Ok(WasmType::I32), // Array pointers
            Type::Matrix(_) => Ok(WasmType::I32), // Matrix pointers
            Type::Pairs(_, _) => Ok(WasmType::I32), // Pairs are represented as pointers
            Type::Object(_) => Ok(WasmType::I32), // Object pointers
            Type::Generic(_, _) => Ok(WasmType::I32), // Generic type pointers
            Type::TypeParameter(_) => Ok(WasmType::I32), // Type parameter pointers
            Type::Any => Ok(WasmType::I32), // Any type is represented as a pointer
            // Sized types
            Type::IntegerSized { bits: 8..=32, .. } => Ok(WasmType::I32),
            Type::IntegerSized { bits: 64, .. } => Ok(WasmType::I64),
            Type::FloatSized { bits: 32 } => Ok(WasmType::F32),
            Type::FloatSized { bits: 64 } => Ok(WasmType::F64),
            Type::List(_) => Ok(WasmType::I32), // Pointer to list structure
            Type::Class { .. } => Ok(WasmType::I32), // Pointer to object
            Type::Function(_, _) => Ok(WasmType::I32), // Function pointer
            _ => Ok(WasmType::I32), // Default fallback for any other types
        }
    }

    fn types_compatible(&self, from: &WasmType, to: &WasmType) -> bool {
        // Any type is compatible with any other type
        if from == &WasmType::I32 && to == &WasmType::I32 {
            return true;
        }
        
        // Exact type match
        if from == to {
            return true;
        }
        
        // Standard integer/float conversions
        match (from, to) {
            (WasmType::I32, WasmType::F32) => true,
            (WasmType::I32, WasmType::F64) => true,
            (WasmType::I64, WasmType::F64) => true,
            (WasmType::F32, WasmType::F64) => true,
            _ => false
        }
    }

    /// Assemble the final WebAssembly module
    fn assemble_module(&mut self) -> Result<Vec<u8>, CompilerError> {
        let mut module = Module::new();

        // Add sections in the correct order
        if !self.function_types.is_empty() {
            // Use the type section that was already populated by the TypeManager
            module.section(&self.type_manager.clone_type_section());
        }
        
        // Add import section if we have imports
        // Always add import section since we have print functions
        module.section(&self.import_section);
        
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
        
        // Always add data section since we might have string literals
        module.section(&self.data_section);

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
                            // EXCEPT for print functions which already return void
                            self.generate_expression(expr, &mut instructions)?;
                            
                            // Only drop if the expression actually returns a value
                            // Print functions and other void functions don't need to be dropped
                            if let Expression::Call(func_name, _) = expr {
                                if func_name == "print" || func_name == "printl" || func_name == "println" {
                                    // Print functions return void, no need to drop
                                } else {
                                    instructions.push(Instruction::Drop);
                                }
                            } else {
                                instructions.push(Instruction::Drop);
                            }
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
                let specified_type = WasmType::from(type_);
                
                let (var_type, init_instructions) = if let Some(init_expr) = initializer {
                    let mut init_instr = Vec::new();
                    let init_type = self.generate_expression(init_expr, &mut init_instr)?;
                    
                    // Use the specified type as the variable type
                    let target_type = specified_type;
                    
                    // Check type compatibility
                    if !self.types_compatible(&init_type, &target_type) {
                        return Err(CompilerError::type_error(
                            format!("Initializer type {:?} does not match specified type {:?} for variable '{}'", init_type, target_type, name),
                            None, location.clone()
                        ));
                    }
                    
                    // Add type conversion if needed
                    if init_type != target_type {
                        self.generate_conversion(init_type, target_type, &mut init_instr)?;
                }
                    
                    (target_type, Some(init_instr))
                } else {
                    (specified_type, None)
                };

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
                        _ => return Err(CompilerError::codegen_error(format!("Cannot determine default value for type {:?}", var_type), None, location.clone()))
                    }
                    instructions.push(Instruction::LocalSet(local_info.index));
                }
            }
            Statement::Assignment { target, value, location } => {
                if let Some(local_info) = self.find_local(target) {
                    // Regular local variable assignment
                    self.generate_expression(value, instructions)?;
                    instructions.push(Instruction::LocalSet(local_info.index));
                } else if let Some(class_context) = &self.current_class_context {
                    // Check if this is a field assignment in class context
                    let field_info = self.class_field_map.get(class_context)
                        .and_then(|field_map| field_map.get(target).cloned());
                    
                    if let Some((field_type, _field_offset)) = field_info {
                        // For now, treat field assignments as local variables
                        // In a full implementation, this would store to object memory
                        self.generate_expression(value, instructions)?;
                        
                        // Create a local variable for the field if it doesn't exist
                        let local_index = self.current_locals.len() as u32;
                        let wasm_type = self.ast_type_to_wasm_type(&field_type)?;
                        
                        self.current_locals.push(LocalVarInfo {
                            index: local_index,
                            type_: wasm_type.into(),
                        });
                        self.variable_map.insert(target.clone(), LocalVarInfo {
                            index: local_index,
                            type_: wasm_type.into(),
                        });
                        
                        instructions.push(Instruction::LocalSet(local_index));
                    } else {
                        // Check if the class exists to provide better error message
                        if self.class_field_map.contains_key(class_context) {
                            return Err(CompilerError::codegen_error(
                                format!("Field '{}' not found in class '{}'", target, class_context),
                                None,
                                location.clone()
                            ));
                        } else {
                            return Err(CompilerError::codegen_error(
                                format!("Class '{}' not found", class_context),
                                None,
                                location.clone()
                            ));
                        }
                    }
                } else {
                    return Err(CompilerError::codegen_error(
                        format!("Undefined variable: {}", target),
                        None, // help
                        location.clone() // location
                    ));
                }
            }
            Statement::Print { expression, newline, location: _ } => {
                // Use type-safe print function call
                let func_name = if *newline { "printl" } else { "print" };
                self.generate_type_safe_print_call(func_name, expression, instructions)?;
            }
            Statement::PrintBlock { expressions, newline, location: _ } => {
                for expression in expressions {
                    let func_name = if *newline { "printl" } else { "print" };
                    self.generate_type_safe_print_call(func_name, expression, instructions)?;
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
            },
            Statement::Error { message, location: _ } => {
                // Generate error throwing - create error object and throw it
                // First generate the error message
                self.generate_expression(message, instructions)?;
                
                // For now, we'll use a simple trap instruction to halt execution
                // In a full implementation, we'd create an error object and use WebAssembly's exception handling
                instructions.push(Instruction::Unreachable);
            },
            Statement::Test { name: _, body, location: _ } => {
                #[cfg(test)]
                for stmt in body {
                    self.generate_statement(stmt, instructions)?;
                }
            }
            Statement::Expression { expr, location: _ } => {
                // For other expressions, generate and drop the result
                let result_type = self.generate_expression(expr, instructions)?;
                
                // Only drop if the expression actually returns a value
                // Print functions and other void functions don't need to be dropped
                if let Expression::Call(func_name, _) = expr {
                    if func_name == "print" || func_name == "printl" || func_name == "println" {
                        // Print functions return void, no need to drop
                        return Ok(());
                    }
                }
                
                // For all other expressions that return values, drop the result
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
                        
                        // Special handling for print functions which don't return a value
                        if function_name != "print" && function_name != "printl" {
                            instructions.push(Instruction::Drop); // Drop return value if any
                        }
                    }
                }
            },
            
            Statement::MethodApplyBlock { object_name, method_chain, expressions, location: _ } => {
                // Generate multiple method calls with the same object.method
                for expr in expressions {
                    // Load the object
                    if let Some(local) = self.find_local(object_name) {
                        instructions.push(Instruction::LocalGet(local.index));
                    } else {
                        return Err(CompilerError::parse_error(
                            format!("Object '{}' not found", object_name),
                            None,
                            Some("Check if the object is declared".to_string())
                        ));
                    }
                    
                    // Generate the argument
                    self.generate_expression(expr, instructions)?;
                    
                    // For now, we'll generate a simple method call
                    // In a full implementation, we'd resolve the method chain and generate appropriate calls
                    if !method_chain.is_empty() {
                        let method_name = &method_chain[0]; // Use first method in chain for now
                        
                        // Handle built-in array methods
                        if method_name == "push" {
                            // This would be array.push(item) - for now just drop the values
                            instructions.push(Instruction::Drop); // Drop argument
                            instructions.push(Instruction::Drop); // Drop object
                        } else {
                            // Generic method call - drop for now
                            instructions.push(Instruction::Drop); // Drop argument
                            instructions.push(Instruction::Drop); // Drop object
                        }
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
            },
            Statement::Error { message, location: _ } => {
                // Generate error throwing - create error object and throw it
                // First generate the error message
                self.generate_expression(message, instructions)?;
                
                // For now, we'll use a simple trap instruction to halt execution
                // In a full implementation, we'd create an error object and use WebAssembly's exception handling
                instructions.push(Instruction::Unreachable);
            },
            
            // Module and async statements
            Statement::Import { imports: _, location: _ } => {
                // For now, imports are no-ops in code generation
                // TODO: Implement module linking and symbol resolution
            },
            
            Statement::LaterAssignment { variable, expression, location: _ } => {
                // later variable = start expression
                // For now, this is handled as a regular assignment
                // TODO: Implement proper async handling with WebAssembly async support
                let expr_type = self.generate_expression(expression, instructions)?;
                
                // Create a local variable for the future result
                let local_info = LocalVarInfo {
                    index: self.current_locals.len() as u32,
                    type_: expr_type.into(),
                };
                instructions.push(Instruction::LocalSet(local_info.index));
                self.variable_map.insert(variable.clone(), local_info.clone());
                self.current_locals.push(local_info);
            },
            
            Statement::Background { expression, location: _ } => {
                // background expression - fire and forget
                // For now, just evaluate the expression and discard the result
                // TODO: Implement proper background execution
                self.generate_expression(expression, instructions)?;
                instructions.push(Instruction::Drop); // Discard the result
            },
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
                // Handle built-in type constructors first
                if func_name == "List" {
                    if !args.is_empty() {
                        return Err(CompilerError::codegen_error(
                            "List() constructor takes no arguments",
                            Some("List type is inferred from variable declaration: List<T> myList = List()".to_string()),
                            None
                        ));
                    }
                    // Create a new empty list - for now, just return a null pointer
                    // In a full implementation, this would allocate memory for a list structure
                    instructions.push(Instruction::I32Const(0)); // Placeholder - null pointer
                    return Ok(WasmType::I32); // Lists are represented as I32 pointers
                }
                
                // Special handling for print functions - they use type-safe dispatch
                if func_name == "print" || func_name == "printl" || func_name == "println" {
                    if args.len() != 1 {
                        return Err(CompilerError::detailed_type_error(
                            &format!("Print function '{}' called with wrong number of arguments", func_name),
                            1,
                            args.len(),
                            None,
                            Some(format!("Print functions expect exactly 1 argument, but {} were provided", args.len()))
                        ));
                    }
                    // Generate type-safe print call
                    self.generate_type_safe_print_call(func_name, &args[0], instructions)?;
                    return Ok(WasmType::I32); // Void represented as I32
                }
                
                // Special handling for HTTP functions - call import functions directly
                if func_name == "http_get" || func_name == "http_delete" {
                    if args.len() != 1 {
                        return Err(CompilerError::detailed_type_error(
                            &format!("HTTP function '{}' called with wrong number of arguments", func_name),
                            1,
                            args.len(),
                            None,
                            Some(format!("HTTP function '{}' expects exactly 1 argument (URL), but {} were provided", func_name, args.len()))
                        ));
                    }
                    // Generate HTTP call with URL parameter
                    self.generate_http_call(func_name, args, instructions)?;
                    return Ok(WasmType::I32); // String represented as I32 pointer
                }
                
                if func_name == "http_post" || func_name == "http_put" || func_name == "http_patch" {
                    if args.len() != 2 {
                        return Err(CompilerError::detailed_type_error(
                            &format!("HTTP function '{}' called with wrong number of arguments", func_name),
                            2,
                            args.len(),
                            None,
                            Some(format!("HTTP function '{}' expects exactly 2 arguments (URL, data), but {} were provided", func_name, args.len()))
                        ));
                    }
                    // Generate HTTP call with URL and data parameters
                    self.generate_http_call(func_name, args, instructions)?;
                    return Ok(WasmType::I32); // String represented as I32 pointer
                }
                
                // Check if this is a constructor call (function name matches a class name)
                if self.class_table.contains_key(func_name) {
                    // This is a constructor call - redirect to constructor function
                    let constructor_name = format!("{}_constructor", func_name);
                    if let Some(constructor_index) = self.get_function_index(&constructor_name) {
                        // Generate arguments
                        for arg in args {
                            self.generate_expression(arg, instructions)?;
                        }
                        
                        instructions.push(Instruction::Call(constructor_index));
                        // Constructor returns an object (represented as I32 pointer)
                        return Ok(WasmType::I32);
                    } else {
                        return Err(CompilerError::codegen_error(
                            &format!("Constructor for class '{}' not found", func_name),
                            Some("Make sure the class has a constructor defined".to_string()),
                            None
                        ));
                    }
                }
                
                // Check if function exists to provide better error messages
                if let Some(func_index) = self.get_function_index(func_name) {
                    // First check if argument count matches for non-print functions
                    if let Some(func_type) = self.instruction_generator.get_function_type(func_index) {
                        let expected_arg_count = func_type.params().len();
                        if args.len() != expected_arg_count {
                            return Err(CompilerError::detailed_type_error(
                                &format!("Function '{}' called with wrong number of arguments", func_name),
                                expected_arg_count,
                                args.len(),
                                None,
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
            Expression::PropertyAssignment { object, property, value, location: _ } => {
                // Handle property assignments like list.type = "line"
                // For now, this is a no-op since we're not implementing full property storage
                // In a full implementation, this would update the object's property
                self.generate_expression(object, instructions)?;
                self.generate_expression(value, instructions)?;
                // Drop both values and return void (represented as I32)
                instructions.push(Instruction::Drop);
                instructions.push(Instruction::Drop);
                Ok(WasmType::I32) // Void
            },
            Expression::MethodCall { object, method, arguments, location: _ } => {
                // Check if this is a type conversion method first
                if self.is_type_conversion_method(method) {
                    return self.generate_type_conversion_method(object, method, instructions);
                }
                
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
                
                // Check if this is a List method call
                if let Expression::Variable(_) = object.as_ref() {
                    // For now, handle List methods as no-ops that return appropriate values
                    match method.as_str() {
                        "add" => {
                            // List.add(item) - for now, just drop the arguments and return void
                            // In a full implementation, this would add the item to the list
                            return Ok(WasmType::I32); // Void is represented as I32 in some contexts
                        },
                        "remove" => {
                            // List.remove() - for now, return a dummy value
                            // In a full implementation, this would remove and return an item
                            instructions.push(Instruction::I32Const(0)); // Dummy return value
                            return Ok(WasmType::I32);
                        },
                        "size" => {
                            // List.size() - for now, return 1 as a dummy value
                            // In a full implementation, this would return the actual size
                            instructions.push(Instruction::I32Const(1)); // Dummy size
                            return Ok(WasmType::I32);
                        },
                        "peek" => {
                            // List.peek() - for now, return a dummy value
                            instructions.push(Instruction::I32Const(0)); // Dummy return value
                            return Ok(WasmType::I32);
                        },
                        "contains" => {
                            // List.contains(item) - for now, return false
                            instructions.push(Instruction::I32Const(0)); // false
                            return Ok(WasmType::I32);
                        },
                        "get" => {
                            // List.get(index) - for now, return a dummy value
                            instructions.push(Instruction::I32Const(0)); // Dummy return value
                            return Ok(WasmType::I32);
                        },
                        "set" => {
                            // List.set(index, value) - for now, just drop arguments and return void
                            return Ok(WasmType::I32); // Void
                        },
                        _ => {
                            // Fall through to regular method handling
                        }
                    }
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
            Expression::OnErrorBlock { expression, error_handler, .. } => {
                // Generate a try-catch block for error handling
                // First, generate the main expression
                let mut try_instructions = Vec::new();
                let result_type = self.generate_expression(expression, &mut try_instructions)?;
                
                // Generate the error handler block
                let mut catch_instructions = Vec::new();
                
                // Add error variable to scope (represented as an object pointer)
                let error_local_index = self.current_locals.len() as u32;
                let error_var = LocalVarInfo {
                    index: error_local_index,
                    type_: WasmType::I32.into(), // Error object is represented as a pointer
                };
                self.variable_map.insert("error".to_string(), error_var.clone());
                self.current_locals.push(error_var);
                
                // Create error object and store in local variable
                // For now, we'll create a simple error object with a message
                let error_message = "Runtime error occurred";
                let error_ptr = self.allocate_string(error_message)?;
                catch_instructions.push(Instruction::I32Const(error_ptr as i32));
                catch_instructions.push(Instruction::LocalSet(error_local_index));
                
                // Generate error handler statements
                for stmt in error_handler {
                    self.generate_statement(stmt, &mut catch_instructions)?;
                }
                
                // Remove error variable from scope
                self.variable_map.remove("error");
                self.current_locals.pop();
                
                // For now, implement a simplified try-catch using conditional logic
                // In a full implementation, we'd use WebAssembly's exception handling proposal
                
                // Add try block
                instructions.extend(try_instructions);
                
                // TODO: Add proper exception handling when WASM exception handling is stable
                // For now, we assume the try block succeeds and skip the catch block
                
                Ok(result_type)
            },
            Expression::ErrorVariable { .. } => {
                // Access the error variable in an error context
                if let Some(error_local) = self.variable_map.get("error") {
                    instructions.push(Instruction::LocalGet(error_local.index));
                    Ok(WasmType::I32) // Error object is represented as a pointer
                } else {
                    Err(CompilerError::codegen_error(
                        "Error variable accessed outside of error context",
                        Some("Error variable can only be used within onError blocks".to_string()),
                        None
                    ))
                }
            },
            Expression::Conditional { condition, then_expr, else_expr, .. } => {
                // Generate conditional expression: if condition then value else value
                // This generates a WebAssembly if-else block that returns a value
                
                // Generate the condition
                self.generate_expression(condition, instructions)?;
                
                // Start the if block
                let then_type = {
                    let mut then_instructions = Vec::new();
                    let result_type = self.generate_expression(then_expr, &mut then_instructions)?;
                    
                    // Convert to block type
                    let block_type = match result_type {
                        WasmType::I32 => BlockType::Result(ValType::I32),
                        WasmType::I64 => BlockType::Result(ValType::I64),
                        WasmType::F32 => BlockType::Result(ValType::F32),
                        WasmType::F64 => BlockType::Result(ValType::F64),
                        _ => BlockType::Empty,
                    };
                    
                    instructions.push(Instruction::If(block_type));
                    instructions.extend(then_instructions);
                    
                    result_type
                };
                
                // Generate the else branch
                instructions.push(Instruction::Else);
                let else_type = self.generate_expression(else_expr, instructions)?;
                
                // End the if block
                instructions.push(Instruction::End);
                
                // Return the common type (should be compatible from semantic analysis)
                if then_type == else_type {
                    Ok(then_type)
                } else {
                    // Handle type promotion if needed
                    match (then_type, else_type) {
                        (WasmType::I32, WasmType::I64) | (WasmType::I64, WasmType::I32) => Ok(WasmType::I64),
                        (WasmType::F32, WasmType::F64) | (WasmType::F64, WasmType::F32) => Ok(WasmType::F64),
                        (WasmType::I32, WasmType::F32) | (WasmType::F32, WasmType::I32) => Ok(WasmType::F32),
                        (WasmType::I32, WasmType::F64) | (WasmType::F64, WasmType::I32) => Ok(WasmType::F64),
                        (WasmType::I64, WasmType::F32) | (WasmType::F32, WasmType::I64) => Ok(WasmType::F32),
                        (WasmType::I64, WasmType::F64) | (WasmType::F64, WasmType::I64) => Ok(WasmType::F64),
                        _ => Ok(then_type), // Default to then type
                    }
                }
            },
            Expression::BaseCall { arguments, location } => {
                // Generate base constructor call
                self.generate_base_call(arguments, location, instructions)
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
                    ast::BinaryOperator::Add => { 
                        instructions.push(Instruction::F64Add); 
                        Ok(WasmType::F64) 
                    },
                    ast::BinaryOperator::Subtract => { 
                        instructions.push(Instruction::F64Sub); 
                        Ok(WasmType::F64) 
                    },
                    ast::BinaryOperator::Multiply => { 
                        instructions.push(Instruction::F64Mul); 
                        Ok(WasmType::F64) 
                    },
                    ast::BinaryOperator::Divide => { 
                        instructions.push(Instruction::F64Div); 
                        Ok(WasmType::F64) 
                    },
                    ast::BinaryOperator::Equal => { 
                        instructions.push(Instruction::F64Eq); 
                        Ok(WasmType::I32) 
                    }, 
                    ast::BinaryOperator::NotEqual => { 
                        instructions.push(Instruction::F64Ne); 
                        Ok(WasmType::I32) 
                    }, 
                    ast::BinaryOperator::Less => { 
                        instructions.push(Instruction::F64Lt); 
                        Ok(WasmType::I32) 
                    }, 
                    ast::BinaryOperator::Greater => { 
                        instructions.push(Instruction::F64Gt); 
                        Ok(WasmType::I32) 
                    }, 
                    ast::BinaryOperator::LessEqual => { 
                        instructions.push(Instruction::F64Le); 
                        Ok(WasmType::I32) 
                    }, 
                    ast::BinaryOperator::GreaterEqual => { 
                        instructions.push(Instruction::F64Ge); 
                        Ok(WasmType::I32) 
                    }, 
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
            // Integer conversions
            (WasmType::I32, WasmType::I64) => {
                instructions.push(Instruction::I64ExtendI32S);
                Ok(())
            },
            (WasmType::I64, WasmType::I32) => {
                instructions.push(Instruction::I32WrapI64);
                Ok(())
            },
            // Float conversions
            (WasmType::F32, WasmType::F64) => {
                instructions.push(Instruction::F64PromoteF32);
                Ok(())
            },
            (WasmType::F64, WasmType::F32) => {
                instructions.push(Instruction::F32DemoteF64);
                Ok(())
            },
            // Integer to float conversions
            (WasmType::I32, WasmType::F32) => {
                instructions.push(Instruction::F32ConvertI32S);
                Ok(())
            },
            (WasmType::I32, WasmType::F64) => {
                instructions.push(Instruction::F64ConvertI32S);
                Ok(())
            },
            (WasmType::I64, WasmType::F32) => {
                instructions.push(Instruction::F32ConvertI64S);
                Ok(())
            },
            (WasmType::I64, WasmType::F64) => {
                instructions.push(Instruction::F64ConvertI64S);
                Ok(())
            },
            // Float to integer conversions
            (WasmType::F32, WasmType::I32) => {
                instructions.push(Instruction::I32TruncF32S);
                Ok(())
            },
            (WasmType::F64, WasmType::I32) => {
                instructions.push(Instruction::I32TruncF64S);
                Ok(())
            },
            (WasmType::F32, WasmType::I64) => {
                instructions.push(Instruction::I64TruncF32S);
                Ok(())
            },
            (WasmType::F64, WasmType::I64) => {
                instructions.push(Instruction::I64TruncF64S);
                Ok(())
            },
            // No conversion needed
            (t1, t2) if t1 == t2 => Ok(()), 
            // Unsupported conversion
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
                // Handle large integers that don't fit in i32
                if *i >= i32::MIN as i64 && *i <= i32::MAX as i64 {
                    instructions.push(Instruction::I32Const(*i as i32));
                Ok(WasmType::I32)
                } else {
                    // Use i64 for large integers
                    instructions.push(Instruction::I64Const(*i));
                    Ok(WasmType::I64)
                }
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
        use crate::ast::{Function as AstFunction, Parameter, Statement, Expression, Value, Type, FunctionSyntax, Visibility, FunctionModifier};
        
        let mut functions = Vec::new();
        
        // abs(value: Integer) -> Integer
        functions.push(AstFunction {
            name: "abs".to_string(),
            type_parameters: vec![],
            type_constraints: vec![],
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
            modifier: FunctionModifier::None,
            location: None,
        });
        
        // Note: print and printl functions are now imported from the host environment
        // instead of being defined as stdlib functions
        
        // array_get(array: Array, index: Integer) -> Integer
        functions.push(AstFunction {
            name: "array_get".to_string(),
            type_parameters: vec![],
            type_constraints: vec![],
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
            modifier: FunctionModifier::None,
            location: None,
        });
        
        // array_length(array: Array) -> Integer
        functions.push(AstFunction {
            name: "array_length".to_string(),
            type_parameters: vec![],
            type_constraints: vec![],
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
            modifier: FunctionModifier::None,
            location: None,
        });
        
        // assert(condition: Boolean) -> Void
        functions.push(AstFunction {
            name: "assert".to_string(),
            type_parameters: vec![],
            type_constraints: vec![],
            parameters: vec![
                Parameter {
                    name: "condition".to_string(),
                    type_: Type::Boolean,
                }
            ],
            return_type: Type::Void,
            body: vec![
                // Placeholder implementation - in a real implementation this would check the condition
                // For now, just drop the value to make it a valid void function
                Statement::Expression {
                    expr: Expression::Variable("condition".to_string()),
                    location: None,
                }
            ],
            description: Some("Asserts that a condition is true".to_string()),
            syntax: FunctionSyntax::Simple,
            visibility: Visibility::Public,
            modifier: FunctionModifier::None,
            location: None,
        });
        
        // string_concat(str1: String, str2: String) -> String
        functions.push(AstFunction {
            name: "string_concat".to_string(),
            type_parameters: vec![],
            type_constraints: vec![],
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
            modifier: FunctionModifier::None,
            location: None,
        });
        
        // string_compare(str1: String, str2: String) -> Integer
        functions.push(AstFunction {
            name: "string_compare".to_string(),
            type_parameters: vec![],
            type_constraints: vec![],
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
            modifier: FunctionModifier::None,
            location: None,
        });

        // HTTP functions are now handled specially in generate_expression
        // and call import functions directly, so we don't need AST functions for them

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
                    "length" => {
                        // Generate the string argument
                        self.generate_expression(&arguments[0], instructions)?;
                        
                        // Call string length function (would need to be implemented in stdlib)
                        if let Some(string_length_index) = self.get_function_index("string_length") {
                            instructions.push(Instruction::Call(string_length_index));
                            Ok(Some(WasmType::I32)) // Length is an integer
                        } else {
                            // For now, return a placeholder implementation
                            instructions.push(Instruction::I32Const(0)); // Placeholder
                            Ok(Some(WasmType::I32))
                        }
                    },
                    "toUpper" | "toLower" | "trim" => {
                        // Generate the string argument
                        self.generate_expression(&arguments[0], instructions)?;
                        
                        // Call appropriate string transformation function
                        let function_name = match method {
                            "toUpper" => "string_to_upper",
                            "toLower" => "string_to_lower", 
                            "trim" => "string_trim",
                            _ => unreachable!()
                        };
                        
                        if let Some(function_index) = self.get_function_index(function_name) {
                            instructions.push(Instruction::Call(function_index));
                            Ok(Some(WasmType::I32)) // String is represented as I32 pointer
                        } else {
                            // For now, return the original string (placeholder)
                            Ok(Some(WasmType::I32))
                        }
                    },
                    "startsWith" | "endsWith" => {
                        // Generate both string arguments
                        self.generate_expression(&arguments[0], instructions)?;
                        self.generate_expression(&arguments[1], instructions)?;
                        
                        // Call appropriate string comparison function
                        let function_name = match method {
                            "startsWith" => "string_starts_with",
                            "endsWith" => "string_ends_with",
                            _ => unreachable!()
                        };
                        
                        if let Some(function_index) = self.get_function_index(function_name) {
                            instructions.push(Instruction::Call(function_index));
                            Ok(Some(WasmType::I32)) // Boolean is represented as I32
                        } else {
                            // For now, return false (placeholder)
                            instructions.push(Instruction::I32Const(0));
                            Ok(Some(WasmType::I32))
                        }
                    },
                    "indexOf" => {
                        // Generate both string arguments
                        self.generate_expression(&arguments[0], instructions)?;
                        self.generate_expression(&arguments[1], instructions)?;
                        
                        if let Some(function_index) = self.get_function_index("string_index_of") {
                            instructions.push(Instruction::Call(function_index));
                            Ok(Some(WasmType::I32)) // Index is an integer
                        } else {
                            // For now, return -1 (not found, placeholder)
                            instructions.push(Instruction::I32Const(-1));
                            Ok(Some(WasmType::I32))
                        }
                    },
                    "substring" => {
                        // Generate string and index arguments
                        self.generate_expression(&arguments[0], instructions)?;
                        self.generate_expression(&arguments[1], instructions)?;
                        if arguments.len() == 3 {
                            self.generate_expression(&arguments[2], instructions)?;
                        } else {
                            // If no end index provided, use string length
                            instructions.push(Instruction::I32Const(-1)); // Placeholder for end
                        }
                        
                        if let Some(function_index) = self.get_function_index("string_substring") {
                            instructions.push(Instruction::Call(function_index));
                            Ok(Some(WasmType::I32)) // String is represented as I32 pointer
                        } else {
                            // For now, return the original string (placeholder)
                            Ok(Some(WasmType::I32))
                        }
                    },
                    "replace" => {
                        // Generate all three string arguments
                        self.generate_expression(&arguments[0], instructions)?;
                        self.generate_expression(&arguments[1], instructions)?;
                        self.generate_expression(&arguments[2], instructions)?;
                        
                        if let Some(function_index) = self.get_function_index("string_replace") {
                            instructions.push(Instruction::Call(function_index));
                            Ok(Some(WasmType::I32)) // String is represented as I32 pointer
                        } else {
                            // For now, return the original string (placeholder)
                            Ok(Some(WasmType::I32))
                        }
                    },
                    "split" => {
                        // Generate both string arguments
                        self.generate_expression(&arguments[0], instructions)?;
                        self.generate_expression(&arguments[1], instructions)?;
                        
                        if let Some(function_index) = self.get_function_index("string_split") {
                            instructions.push(Instruction::Call(function_index));
                            Ok(Some(WasmType::I32)) // Array is represented as I32 pointer
                        } else {
                            // For now, return an empty array (placeholder)
                            instructions.push(Instruction::I32Const(0));
                            Ok(Some(WasmType::I32))
                        }
                    },
                    _ => Ok(None), // Method not found in StringUtils
                }
            },
            "ArrayUtils" => {
                match method {
                    "length" => {
                        // Generate the array argument
                        self.generate_expression(&arguments[0], instructions)?;
                        
                        // Call array length function
                        if let Some(array_length_index) = self.get_function_index("array_length") {
                            instructions.push(Instruction::Call(array_length_index));
                            Ok(Some(WasmType::I32)) // Length is an integer
                        } else {
                            // For now, return a placeholder implementation
                            instructions.push(Instruction::I32Const(0)); // Placeholder
                            Ok(Some(WasmType::I32))
                        }
                    },
                    "slice" => {
                        // Generate array and index arguments
                        self.generate_expression(&arguments[0], instructions)?;
                        self.generate_expression(&arguments[1], instructions)?;
                        if arguments.len() == 3 {
                            self.generate_expression(&arguments[2], instructions)?;
                        } else {
                            // If no end index provided, use array length
                            instructions.push(Instruction::I32Const(-1)); // Placeholder for end
                        }
                        
                        if let Some(function_index) = self.get_function_index("array_slice") {
                            instructions.push(Instruction::Call(function_index));
                            Ok(Some(WasmType::I32)) // Array is represented as I32 pointer
                        } else {
                            // For now, return the original array (placeholder)
                            Ok(Some(WasmType::I32))
                        }
                    },
                    "join" => {
                        // Generate array and separator arguments
                        self.generate_expression(&arguments[0], instructions)?;
                        self.generate_expression(&arguments[1], instructions)?;
                        
                        if let Some(function_index) = self.get_function_index("array_join") {
                            instructions.push(Instruction::Call(function_index));
                            Ok(Some(WasmType::I32)) // String is represented as I32 pointer
                        } else {
                            // For now, return empty string (placeholder)
                            instructions.push(Instruction::I32Const(0));
                            Ok(Some(WasmType::I32))
                        }
                    },
                    "reverse" | "sort" => {
                        // Generate the array argument
                        self.generate_expression(&arguments[0], instructions)?;
                        
                        // Call appropriate array function
                        let function_name = match method {
                            "reverse" => "array_reverse",
                            "sort" => "array_sort",
                            _ => unreachable!()
                        };
                        
                        if let Some(function_index) = self.get_function_index(function_name) {
                            instructions.push(Instruction::Call(function_index));
                            Ok(Some(WasmType::I32)) // Array is represented as I32 pointer
                        } else {
                            // For now, return the original array (placeholder)
                            Ok(Some(WasmType::I32))
                        }
                    },
                    "push" => {
                        // Generate array and element arguments
                        self.generate_expression(&arguments[0], instructions)?;
                        self.generate_expression(&arguments[1], instructions)?;
                        
                        if let Some(function_index) = self.get_function_index("array_push") {
                            instructions.push(Instruction::Call(function_index));
                            Ok(Some(WasmType::I32)) // Array is represented as I32 pointer
                        } else {
                            // For now, return the original array (placeholder)
                            Ok(Some(WasmType::I32))
                        }
                    },
                    "pop" => {
                        // Generate the array argument
                        self.generate_expression(&arguments[0], instructions)?;
                        
                        if let Some(function_index) = self.get_function_index("array_pop") {
                            instructions.push(Instruction::Call(function_index));
                            Ok(Some(WasmType::I32)) // Element (assuming I32 for now)
                        } else {
                            // For now, return 0 (placeholder)
                            instructions.push(Instruction::I32Const(0));
                            Ok(Some(WasmType::I32))
                        }
                    },
                    _ => Ok(None), // Method not found in ArrayUtils
                }
            },
            "File" => {
                match method {
                    "read" => {
                        // Generate the file path argument
                        self.generate_expression(&arguments[0], instructions)?;
                        
                        // For now, just drop the argument and return a placeholder string pointer
                        // In a real implementation, this would call file_read import with proper string handling
                        instructions.push(Instruction::Drop); // Drop the path argument for now
                        instructions.push(Instruction::I32Const(0)); // Placeholder string pointer
                        Ok(Some(WasmType::I32)) // String is represented as I32 pointer
                    },
                    "write" => {
                        // Generate file path and content arguments
                        self.generate_expression(&arguments[0], instructions)?;
                        self.generate_expression(&arguments[1], instructions)?;
                        
                        // For now, just drop both arguments and return success
                        // In a real implementation, this would call file_write import with proper string handling
                        instructions.push(Instruction::Drop); // Drop content argument
                        instructions.push(Instruction::Drop); // Drop path argument
                        instructions.push(Instruction::I32Const(0)); // Return success
                        Ok(Some(WasmType::I32)) // Return status code
                    },
                    "append" => {
                        // Generate file path and content arguments
                        self.generate_expression(&arguments[0], instructions)?;
                        self.generate_expression(&arguments[1], instructions)?;
                        
                        // For now, just drop both arguments and return success
                        // In a real implementation, this would call file_append import with proper string handling
                        instructions.push(Instruction::Drop); // Drop content argument
                        instructions.push(Instruction::Drop); // Drop path argument
                        instructions.push(Instruction::I32Const(0)); // Return success
                        Ok(Some(WasmType::I32)) // Return status code
                    },
                    "exists" => {
                        // Generate the file path argument
                        self.generate_expression(&arguments[0], instructions)?;
                        
                        // For now, just drop the argument and return false
                        // In a real implementation, this would call file_exists import with proper string handling
                        instructions.push(Instruction::Drop); // Drop the path argument
                        instructions.push(Instruction::I32Const(0)); // Return false
                        Ok(Some(WasmType::I32)) // Boolean is represented as I32
                    },
                    "delete" => {
                        // Generate the file path argument
                        self.generate_expression(&arguments[0], instructions)?;
                        
                        // For now, just drop the argument and return success
                        // In a real implementation, this would call file_delete import with proper string handling
                        instructions.push(Instruction::Drop); // Drop the path argument
                        instructions.push(Instruction::I32Const(0)); // Return success
                        Ok(Some(WasmType::I32)) // Return status code
                    },
                    "lines" => {
                        // Generate the file path argument
                        self.generate_expression(&arguments[0], instructions)?;
                        
                        // For now, return a placeholder list pointer
                        // In a real implementation, this would read file lines into a list
                        // This would require more complex string parsing and list creation
                        instructions.push(Instruction::Drop); // Drop the path argument for now
                        instructions.push(Instruction::I32Const(0)); // Placeholder list pointer
                        Ok(Some(WasmType::I32)) // List is represented as I32 pointer
                    },
                    _ => Ok(None), // Method not found in File
                }
            },
            "Http" => {
                match method {
                    "get" => {
                        // Generate the URL argument
                        self.generate_expression(&arguments[0], instructions)?;
                        
                        // For now, just drop the argument and return a placeholder string pointer
                        // In a real implementation, this would call http_get import with proper string handling
                        instructions.push(Instruction::Drop); // Drop the URL argument for now
                        instructions.push(Instruction::I32Const(0)); // Placeholder response string pointer
                        Ok(Some(WasmType::I32)) // String is represented as I32 pointer
                    },
                    "post" | "put" | "patch" => {
                        // Generate URL and body arguments
                        self.generate_expression(&arguments[0], instructions)?;
                        self.generate_expression(&arguments[1], instructions)?;
                        
                        // For now, just drop both arguments and return a placeholder response
                        // In a real implementation, this would call http_post/put/patch import with proper string handling
                        instructions.push(Instruction::Drop); // Drop body argument
                        instructions.push(Instruction::Drop); // Drop URL argument
                        instructions.push(Instruction::I32Const(0)); // Placeholder response string pointer
                        Ok(Some(WasmType::I32)) // String is represented as I32 pointer
                    },
                    "delete" => {
                        // Generate the URL argument
                        self.generate_expression(&arguments[0], instructions)?;
                        
                        // For now, just drop the argument and return a placeholder response
                        // In a real implementation, this would call http_delete import with proper string handling
                        instructions.push(Instruction::Drop); // Drop the URL argument
                        instructions.push(Instruction::I32Const(0)); // Placeholder response string pointer
                        Ok(Some(WasmType::I32)) // String is represented as I32 pointer
                    },
                    _ => Ok(None), // Method not found in Http
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
    
    fn is_type_conversion_method(&self, method: &str) -> bool {
        matches!(method, "toInteger" | "toFloat" | "toString" | "toBoolean")
    }
    
    fn generate_type_conversion_method(
        &mut self,
        object: &Expression,
        method: &str,
        instructions: &mut Vec<Instruction>
    ) -> Result<WasmType, CompilerError> {
        // Generate the object expression first
        let object_type = self.generate_expression(object, instructions)?;
        
        // Perform the type conversion based on the method name
        match method {
            "toInteger" => {
                match object_type {
                    WasmType::I32 => {
                        // Already an integer, no conversion needed
                        Ok(WasmType::I32)
                    },
                    WasmType::F64 => {
                        // Convert float to integer (truncate)
                        instructions.push(Instruction::I32TruncF64S);
                        Ok(WasmType::I32)
                    },
                    _ => {
                        // For other types (like strings), we'd need more complex conversion
                        // For now, just return an error
                        Err(CompilerError::codegen_error(
                            &format!("Conversion from {:?} to integer not yet implemented", object_type),
                            None,
                            None
                        ))
                    }
                }
            },
            "toFloat" => {
                match object_type {
                    WasmType::I32 => {
                        // Convert integer to float
                        instructions.push(Instruction::F64ConvertI32S);
                        Ok(WasmType::F64)
                    },
                    WasmType::F64 => {
                        // Already a float, no conversion needed
                        Ok(WasmType::F64)
                    },
                    _ => {
                        Err(CompilerError::codegen_error(
                            &format!("Conversion from {:?} to float not yet implemented", object_type),
                            None,
                            None
                        ))
                    }
                }
            },
            "toString" => {
                match object_type {
                    WasmType::I32 => {
                        // Convert integer to string
                        // This would require a runtime function to convert int to string
                        // For now, we'll implement a basic version
                        if let Some(int_to_string_index) = self.get_function_index("int_to_string") {
                            instructions.push(Instruction::Call(int_to_string_index));
                            Ok(WasmType::I32) // String is represented as I32 pointer
                        } else {
                            Err(CompilerError::codegen_error(
                                "Integer to string conversion function not found",
                                Some("int_to_string function needs to be implemented".to_string()),
                                None
                            ))
                        }
                    },
                    WasmType::F64 => {
                        // Convert float to string
                        if let Some(float_to_string_index) = self.get_function_index("float_to_string") {
                            instructions.push(Instruction::Call(float_to_string_index));
                            Ok(WasmType::I32) // String is represented as I32 pointer
                        } else {
                            Err(CompilerError::codegen_error(
                                "Float to string conversion function not found",
                                Some("float_to_string function needs to be implemented".to_string()),
                                None
                            ))
                        }
                    },
                    _ => {
                        // Already a string or other type
                        Ok(WasmType::I32) // Assume string representation
                    }
                }
            },
            "toBoolean" => {
                match object_type {
                    WasmType::I32 => {
                        // Convert integer to boolean (0 = false, non-zero = true)
                        instructions.push(Instruction::I32Const(0));
                        instructions.push(Instruction::I32Ne);
                        Ok(WasmType::I32) // Boolean is represented as I32
                    },
                    WasmType::F64 => {
                        // Convert float to boolean (0.0 = false, non-zero = true)
                        instructions.push(Instruction::F64Const(0.0));
                        instructions.push(Instruction::F64Ne);
                        instructions.push(Instruction::I32TruncF64S); // Convert result to I32
                        Ok(WasmType::I32)
                    },
                    _ => {
                        // For other types, assume truthy conversion
                        Ok(WasmType::I32)
                    }
                }
            },
            _ => {
                Err(CompilerError::codegen_error(
                    &format!("Unknown type conversion method: {}", method),
                    None,
                    None
                ))
            }
        }
    }

    /// Add a string to the string pool and return its pointer
    pub fn add_string_to_pool(&mut self, string: &str) -> u32 {
        // For now, just return a placeholder pointer
        // In a real implementation, this would allocate memory and store the string
        0
    }

    /// Get a string from memory at the given pointer
    pub fn get_string_from_memory(&self, ptr: u64) -> Result<String, CompilerError> {
        // For now, just return an empty string
        // In a real implementation, this would read the string from memory
        Ok(String::new())
    }

    /// Call a function by name with the given arguments
    pub fn call_function(&self, name: &str, args: Vec<wasmtime::Val>) -> Result<Vec<wasmtime::Val>, CompilerError> {
        // For now, just return empty results
        // In a real implementation, this would call the function and return its results
        Ok(vec![])
    }

    fn generate_error_handler(&mut self, protected: &Expression, handler: &[Statement]) -> Result<WasmType, CompilerError> {
        // Generate code for protected expression
        let mut instructions = Vec::new();
        let expr_type = self.generate_expression(protected, &mut instructions)?;
        
        // Add error handling block
        instructions.push(Instruction::Try(BlockType::Result(expr_type.to_val_type())));
        
        // Generate error handler code
        let mut handler_instructions = Vec::new();
        for stmt in handler {
            self.generate_statement(stmt, &mut handler_instructions)?;
        }
        
        // Add catch block
        instructions.push(Instruction::Catch(0));
        instructions.extend(handler_instructions);
        instructions.push(Instruction::End);
        
        Ok(expr_type)
    }

    fn generate_on_error(&mut self, expression: &Expression, fallback: &Expression) -> Result<WasmType, CompilerError> {
        let mut instructions = Vec::new();
        
        // Generate code for main expression
        let expr_type = self.generate_expression(expression, &mut instructions)?;
        
        // Add error handling block
        instructions.push(Instruction::Try(BlockType::Result(expr_type.to_val_type())));
        
        // Generate fallback expression
        let mut fallback_instructions = Vec::new();
        let fallback_type = self.generate_expression(fallback, &mut fallback_instructions)?;
        
        // Verify types match
        if expr_type != fallback_type {
            return Err(CompilerError::type_error(
                format!("onError fallback type {:?} doesn't match expression type {:?}", fallback_type, expr_type),
                Some("Ensure the fallback value has the same type as the main expression".to_string()),
                None
            ));
        }
        
        // Add catch block with fallback
        instructions.push(Instruction::Catch(0));
        instructions.extend(fallback_instructions);
        instructions.push(Instruction::End);
        
        Ok(expr_type)
    }

    /// Generate code for a class
    fn generate_class(&mut self, class: &Class) -> Result<(), CompilerError> {
        // Generate constructor
        if let Some(constructor) = &class.constructor {
            let mut instructions = Vec::new();
            
            // Generate constructor parameters
            for param in &constructor.parameters {
                // Any type is represented as I32 in WebAssembly
                let wasm_type = if matches!(param.type_, Type::Any) {
                    WasmType::I32
                } else {
                    self.type_manager.ast_type_to_wasm_type(&param.type_)?
                };
                
                self.instruction_generator.add_parameter(&param.name, wasm_type);
            }
            
            // Generate constructor body
            self.generate_statements(&constructor.body, &mut instructions)?;
            
            // Add constructor to function table
            let constructor_name = format!("{}_constructor", class.name);
            self.function_table.insert(constructor_name.clone(), self.function_count);
            self.function_count += 1;
            
            // Note: Constructor function would be added to the module during assembly
        }
        
        // Generate methods
        for method in &class.methods {
            let mut instructions = Vec::new();
            
            // Generate method parameters
            for param in &method.parameters {
                // Any type is represented as I32 in WebAssembly
                let wasm_type = if matches!(param.type_, Type::Any) {
                    WasmType::I32
                } else {
                    self.type_manager.ast_type_to_wasm_type(&param.type_)?
                };
                
                self.instruction_generator.add_parameter(&param.name, wasm_type);
            }
            
            // Generate method body
            self.generate_statements(&method.body, &mut instructions)?;
            
            // Add method to function table
            let method_name = format!("{}_{}", class.name, method.name);
            self.function_table.insert(method_name.clone(), self.function_count);
            self.function_count += 1;
            
            // Note: Method function would be added to the module during assembly
        }
        
        Ok(())
    }

    /// Generate code for a range iteration statement
    fn generate_range_iterate(&mut self, stmt: &Statement) -> Result<Vec<Instruction>, CompilerError> {
        if let Statement::RangeIterate { iterator, start, end, step, body, .. } = stmt {
            let mut instructions = Vec::new();
            
            // Get types first to avoid borrow checker issues
            let start_type = self.get_expression_type(start)?;
            let end_type = self.get_expression_type(end)?;
            let step_type = if let Some(step_expr) = step {
                Some(self.get_expression_type(step_expr)?)
            } else {
                None
            };
            
            // Generate start expression
            self.generate_expression(start, &mut instructions)?;
            
            // Store start value
            let start_local = self.add_local(start_type);
            instructions.push(Instruction::LocalSet(start_local));
            
            // Generate end expression
            self.generate_expression(end, &mut instructions)?;
            
            // Store end value
            let end_local = self.add_local(end_type);
            instructions.push(Instruction::LocalSet(end_local));
            
            // Generate step expression if present
            let step_local = if let Some(step_expr) = step {
                self.generate_expression(step_expr, &mut instructions)?;
                
                // Store step value
                let step_local = self.add_local(step_type.unwrap());
                instructions.push(Instruction::LocalSet(step_local));
                Some(step_local)
            } else {
                None
            };
            
            // Add iterator to symbol table
            let iterator_local = self.add_local(start_type);
            self.symbol_table.insert(iterator.clone(), iterator_local);
            
            // Generate loop
            let loop_label = self.next_label();
            let end_label = self.next_label();
            
            // Initialize iterator
            instructions.push(Instruction::LocalGet(start_local));
            instructions.push(Instruction::LocalSet(iterator_local));
            
            // Loop start
            instructions.push(Instruction::Loop(BlockType::Empty));
            
            // Check condition
            instructions.push(Instruction::LocalGet(iterator_local));
            instructions.push(Instruction::LocalGet(end_local));
            
            // Compare based on step direction
            if let Some(step_local) = step_local {
                // Get step value
                instructions.push(Instruction::LocalGet(step_local));
                
                // If step is negative, use greater than or equal
                // If step is positive, use less than or equal
                instructions.push(Instruction::F64Const(0.0));
                instructions.push(Instruction::F64Lt);
                instructions.push(Instruction::If(BlockType::Empty));
                
                // Negative step
                instructions.push(Instruction::LocalGet(iterator_local));
                instructions.push(Instruction::LocalGet(end_local));
                instructions.push(Instruction::F64Ge);
                
                instructions.push(Instruction::Else);
                
                // Positive step
                instructions.push(Instruction::LocalGet(iterator_local));
                instructions.push(Instruction::LocalGet(end_local));
                instructions.push(Instruction::F64Le);
                
                instructions.push(Instruction::End);
            } else {
                // Default to positive step
                instructions.push(Instruction::F64Le);
            }
            
            // Break if condition is false
            instructions.push(Instruction::BrIf(end_label));
            
            // Generate body
            for stmt in body {
                self.generate_statement(stmt, &mut instructions)?;
            }
            
            // Update iterator
            instructions.push(Instruction::LocalGet(iterator_local));
            if let Some(step_local) = step_local {
                instructions.push(Instruction::LocalGet(step_local));
                instructions.push(Instruction::F64Add);
            } else {
                instructions.push(Instruction::F64Const(1.0));
                instructions.push(Instruction::F64Add);
            }
            instructions.push(Instruction::LocalSet(iterator_local));
            
            // Continue loop
            instructions.push(Instruction::Br(loop_label));
            
            // End loop
            instructions.push(Instruction::End);
            
            // Remove iterator from symbol table
            self.symbol_table.remove(iterator);
            
            Ok(instructions)
        } else {
            Err(CompilerError::type_error(
                "Expected range iteration statement".to_string(),
                None,
                None
            ))
        }
    }

    // Missing methods that are referenced in the code
    pub fn add_local(&mut self, wasm_type: WasmType) -> u32 {
        let local_index = self.current_locals.len() as u32;
        self.current_locals.push(LocalVarInfo {
            index: local_index,
            type_: wasm_type.into(),
        });
        local_index
    }

    pub fn get_expression_type(&mut self, expr: &Expression) -> Result<WasmType, CompilerError> {
        // This is a simplified implementation - in a full implementation this would
        // analyze the expression to determine its type
        match expr {
            Expression::Literal(Value::Integer(_)) => Ok(WasmType::I32),
            Expression::Literal(Value::Float(_)) => Ok(WasmType::F64),
            Expression::Literal(Value::Boolean(_)) => Ok(WasmType::I32),
            Expression::Literal(Value::String(_)) => Ok(WasmType::I32), // String pointer
            Expression::Variable(name) => {
                if let Some(local) = self.find_local(name) {
                    Ok(local.type_.into())
                } else {
                    Ok(WasmType::I32) // Default to i32
                }
            },
            _ => Ok(WasmType::I32), // Default fallback
        }
    }

    pub fn next_label(&mut self) -> u32 {
        let label = self.label_counter;
        self.label_counter += 1;
        label
    }

    pub fn generate_statements(&mut self, statements: &[Statement], instructions: &mut Vec<Instruction>) -> Result<(), CompilerError> {
        for stmt in statements {
            self.generate_statement(stmt, instructions)?;
        }
        Ok(())
    }

    /// Type-safe print function call generation following printf best practices
    /// Dispatches to appropriate print function based on argument type to prevent format string vulnerabilities
    fn generate_type_safe_print_call(&mut self, func_name: &str, arg: &Expression, instructions: &mut Vec<Instruction>) -> Result<(), CompilerError> {
        // Check if the argument is a string literal or string variable
        match arg {
            Expression::Literal(Value::String(_)) | Expression::Variable(_) => {
                // Use the string print functions that take (ptr, len)
                self.generate_string_for_import(arg, instructions)?;
                
                // Call the appropriate string print function
                match func_name {
                    "print" => {
                        // Call print import function (index 0)
                        instructions.push(Instruction::Call(0));
                    },
                    "printl" | "println" => {
                        // Call printl import function (index 1)
                        instructions.push(Instruction::Call(1));
                    },
                    _ => {
                        return Err(CompilerError::codegen_error(
                            &format!("Unknown print function: {}", func_name),
                            None,
                            None
                        ));
                    }
                }
            },
            _ => {
                // For non-string arguments, use the simple print functions
                self.generate_expression(arg, instructions)?;
                
                // Call the appropriate simple print function
                match func_name {
                    "print" => {
                        // Call print_simple import function (index 2)
                        instructions.push(Instruction::Call(2));
                    },
                    "printl" | "println" => {
                        // Call printl_simple import function (index 3)
                        instructions.push(Instruction::Call(3));
                    },
                    _ => {
                        return Err(CompilerError::codegen_error(
                            &format!("Unknown print function: {}", func_name),
                            None,
                            None
                        ));
                    }
                }
            }
        }
        
        Ok(())
    }

    fn generate_http_call(&mut self, func_name: &str, args: &[Expression], instructions: &mut Vec<Instruction>) -> Result<(), CompilerError> {
        // Get the import function index for the HTTP function
        let import_index = match self.http_import_indices.get(func_name) {
            Some(&index) => index,
            None => {
                return Err(CompilerError::codegen_error(
                    &format!("HTTP import function '{}' not found", func_name),
                    Some("Make sure HTTP imports are properly registered".to_string()),
                    None
                ));
            }
        };

        match func_name {
            "http_get" | "http_delete" => {
                // Single parameter: URL
                if args.len() != 1 {
                    return Err(CompilerError::codegen_error(
                        &format!("HTTP function '{}' expects 1 argument", func_name),
                        None,
                        None
                    ));
                }
                
                // Generate URL string - this should put ptr and len on stack
                self.generate_string_for_import(&args[0], instructions)?;
                
                // Call the import function
                instructions.push(Instruction::Call(import_index));
            },
            "http_post" | "http_put" | "http_patch" => {
                // Two parameters: URL and data
                if args.len() != 2 {
                    return Err(CompilerError::codegen_error(
                        &format!("HTTP function '{}' expects 2 arguments", func_name),
                        None,
                        None
                    ));
                }
                
                // Generate URL string - this should put ptr and len on stack
                self.generate_string_for_import(&args[0], instructions)?;
                
                // Generate data string - this should put ptr and len on stack
                self.generate_string_for_import(&args[1], instructions)?;
                
                // Call the import function
                instructions.push(Instruction::Call(import_index));
            },
            _ => {
                return Err(CompilerError::codegen_error(
                    &format!("Unknown HTTP function: {}", func_name),
                    None,
                    None
                ));
            }
        }
        
        Ok(())
    }

    fn generate_string_for_import(&mut self, expr: &Expression, instructions: &mut Vec<Instruction>) -> Result<(), CompilerError> {
        // For string literals, we need to put the string in memory and push ptr + len
        if let Expression::Literal(Value::String(s)) = expr {
            // Use proper memory allocation with ARC
            let str_ptr = self.memory_utils.allocate_string(s)
                .map_err(|e| CompilerError::codegen_error(
                    &format!("Failed to allocate string: {}", e),
                    None,
                    None
                ))?;
            
            let str_len = s.len() as i32;
            
            // For function signature (ptr, len):
            // WASM stack is LIFO, so we push ptr first (will be at bottom), then len (will be at top)
            // When the function is called, it will pop len first, then ptr
            
            // Push pointer first (will be first parameter) - points to string content after length field
            instructions.push(Instruction::I32Const((str_ptr + 4) as i32));
            
            // Push length second (will be second parameter)
            instructions.push(Instruction::I32Const(str_len));
        } else {
            // For non-literal strings, generate the expression and extract string data
            let expr_type = self.generate_expression(expr, instructions)?;
            
            if expr_type == WasmType::I32 {
                // Assume it's a string pointer - extract length and content pointer
                // String layout: [header][length][content]
                
                // Duplicate the string pointer for length extraction
                instructions.push(Instruction::LocalTee(self.add_local(WasmType::I32)));
                
                // Load string length (at offset 0 from string data pointer)
                instructions.push(Instruction::I32Load(MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                }));
                
                // Get the original string pointer back and adjust to content
                instructions.push(Instruction::LocalGet(self.current_locals.len() as u32 - 1));
                instructions.push(Instruction::I32Const(4)); // Skip length field
                instructions.push(Instruction::I32Add);
                
                // Now we have [length, content_ptr] on stack - swap them for correct order
                // We need [content_ptr, length] for the function call
                let temp_local = self.add_local(WasmType::I32);
                instructions.push(Instruction::LocalSet(temp_local)); // Store content_ptr
                instructions.push(Instruction::LocalGet(temp_local)); // Push content_ptr first
                // Length is already on stack as second parameter
            } else {
                return Err(CompilerError::codegen_error(
                    "String expression must evaluate to a string pointer",
                    None,
                    None
                ));
            }
        }
        
        Ok(())
    }

          /// Register file system import functions
     fn register_file_imports(&mut self) -> Result<(), CompilerError> {
         // file_write(pathPtr: i32, pathLen: i32, contentPtr: i32, contentLen: i32) -> i32
         let write_type = self.add_function_type(&[WasmType::I32, WasmType::I32, WasmType::I32, WasmType::I32], Some(WasmType::I32))?;
         self.import_section.import("env", "file_write", wasm_encoder::EntityType::Function(write_type));
         self.file_import_indices.insert("file_write".to_string(), self.function_count);
         self.function_count += 1;
         
         // file_read(pathPtr: i32, pathLen: i32, resultPtr: i32) -> i32 (returns length or -1 for error)
         let read_type = self.add_function_type(&[WasmType::I32, WasmType::I32, WasmType::I32], Some(WasmType::I32))?;
         self.import_section.import("env", "file_read", wasm_encoder::EntityType::Function(read_type));
         self.file_import_indices.insert("file_read".to_string(), self.function_count);
         self.function_count += 1;
         
         // file_exists(pathPtr: i32, pathLen: i32) -> i32 (returns 1 if exists, 0 if not)
         let exists_type = self.add_function_type(&[WasmType::I32, WasmType::I32], Some(WasmType::I32))?;
         self.import_section.import("env", "file_exists", wasm_encoder::EntityType::Function(exists_type));
         self.file_import_indices.insert("file_exists".to_string(), self.function_count);
         self.function_count += 1;
         
         // file_delete(pathPtr: i32, pathLen: i32) -> i32 (returns 0 for success, -1 for error)
         let delete_type = self.add_function_type(&[WasmType::I32, WasmType::I32], Some(WasmType::I32))?;
         self.import_section.import("env", "file_delete", wasm_encoder::EntityType::Function(delete_type));
         self.file_import_indices.insert("file_delete".to_string(), self.function_count);
         self.function_count += 1;
         
         // file_append(pathPtr: i32, pathLen: i32, contentPtr: i32, contentLen: i32) -> i32
         let append_type = self.add_function_type(&[WasmType::I32, WasmType::I32, WasmType::I32, WasmType::I32], Some(WasmType::I32))?;
         self.import_section.import("env", "file_append", wasm_encoder::EntityType::Function(append_type));
         self.file_import_indices.insert("file_append".to_string(), self.function_count);
         self.function_count += 1;
         
         Ok(())
     }
     
     /// Register HTTP client import functions
     fn register_http_imports(&mut self) -> Result<(), CompilerError> {
         // http_get(urlPtr: i32, urlLen: i32) -> i32 (returns string pointer)
         let get_type = self.add_function_type(&[WasmType::I32, WasmType::I32], Some(WasmType::I32))?;
         self.import_section.import("env", "http_get", wasm_encoder::EntityType::Function(get_type));
         self.http_import_indices.insert("http_get".to_string(), self.function_count);
         self.function_count += 1;
         
         // http_post(urlPtr: i32, urlLen: i32, bodyPtr: i32, bodyLen: i32) -> i32 (returns string pointer)
         let post_type = self.add_function_type(&[WasmType::I32, WasmType::I32, WasmType::I32, WasmType::I32], Some(WasmType::I32))?;
         self.import_section.import("env", "http_post", wasm_encoder::EntityType::Function(post_type));
         self.http_import_indices.insert("http_post".to_string(), self.function_count);
         self.function_count += 1;
         
         // http_put(urlPtr: i32, urlLen: i32, bodyPtr: i32, bodyLen: i32) -> i32 (returns string pointer)
         let put_type = self.add_function_type(&[WasmType::I32, WasmType::I32, WasmType::I32, WasmType::I32], Some(WasmType::I32))?;
         self.import_section.import("env", "http_put", wasm_encoder::EntityType::Function(put_type));
         self.http_import_indices.insert("http_put".to_string(), self.function_count);
         self.function_count += 1;
         
         // http_patch(urlPtr: i32, urlLen: i32, bodyPtr: i32, bodyLen: i32) -> i32 (returns string pointer)
         let patch_type = self.add_function_type(&[WasmType::I32, WasmType::I32, WasmType::I32, WasmType::I32], Some(WasmType::I32))?;
         self.import_section.import("env", "http_patch", wasm_encoder::EntityType::Function(patch_type));
         self.http_import_indices.insert("http_patch".to_string(), self.function_count);
         self.function_count += 1;
         
         // http_delete(urlPtr: i32, urlLen: i32) -> i32 (returns string pointer)
         let delete_type = self.add_function_type(&[WasmType::I32, WasmType::I32], Some(WasmType::I32))?;
         self.import_section.import("env", "http_delete", wasm_encoder::EntityType::Function(delete_type));
         self.http_import_indices.insert("http_delete".to_string(), self.function_count);
         self.function_count += 1;
         
         Ok(())
     }
     
     /// Register print function imports
    fn register_print_imports(&mut self) -> Result<(), CompilerError> {
        // Original print functions
        // print(strPtr: i32, strLen: i32) -> void (string pointer and length)
        let print_type = self.add_function_type(&[WasmType::I32, WasmType::I32], None)?;
        self.import_section.import("env", "print", wasm_encoder::EntityType::Function(print_type));
        self.function_map.insert("print".to_string(), self.function_count);
        self.function_count += 1;
        
        // printl(strPtr: i32, strLen: i32) -> void (print with newline)
        let printl_type = self.add_function_type(&[WasmType::I32, WasmType::I32], None)?;
        self.import_section.import("env", "printl", wasm_encoder::EntityType::Function(printl_type));
        self.function_map.insert("printl".to_string(), self.function_count);
        self.function_count += 1;
        
        // Simplified print functions
        // print_simple(value: i32) -> void 
        let print_simple_type = self.add_function_type(&[WasmType::I32], None)?;
        self.import_section.import("env", "print_simple", wasm_encoder::EntityType::Function(print_simple_type));
        self.function_map.insert("print_simple".to_string(), self.function_count);
        self.function_count += 1;
        
        // printl_simple(value: i32) -> void 
        let printl_simple_type = self.add_function_type(&[WasmType::I32], None)?;
        self.import_section.import("env", "printl_simple", wasm_encoder::EntityType::Function(printl_simple_type));
        self.function_map.insert("printl_simple".to_string(), self.function_count);
        self.function_count += 1;
        
        Ok(())
    }

    fn generate_base_call(&mut self, arguments: &[Expression], _location: &SourceLocation, instructions: &mut Vec<Instruction>) -> Result<WasmType, CompilerError> {
        // For now, base calls are treated as no-ops in WebAssembly
        // In a full implementation, this would:
        // 1. Look up the parent class constructor
        // 2. Generate arguments
        // 3. Call the parent constructor with the current object instance
        
        // Generate arguments (for side effects)
        for arg in arguments {
            self.generate_expression(arg, instructions)?;
            // Pop the result since we're not using it
            instructions.push(Instruction::Drop);
        }
        
        // Base calls don't produce a value on the WebAssembly stack
        // They are statements that perform side effects
        // We need to indicate that this doesn't leave a value on the stack
        // by using a special marker. Since this is called from Statement::Expression
        // context, we need to return a type that indicates "no value"
        // But since we can't return "void", we'll use a dummy value approach
        instructions.push(Instruction::I32Const(0));
        Ok(WasmType::I32)
    }
}
