//! Module for type management during code generation.

use wasmparser::FuncType;
use wasm_encoder::{ValType, TypeSection};
use crate::types::WasmType;
use crate::error::CompilerError;
use crate::ast::{Type, Expression, Value};

/// Manages type information and conversions during code generation
#[derive(Clone)]
pub(crate) struct TypeManager {
    type_section: TypeSection,
    function_types: Vec<FuncType>,
}

impl TypeManager {
    /// Create a new type manager
    pub(crate) fn new() -> Self {
        Self {
            type_section: TypeSection::new(),
            function_types: Vec::new(),
        }
    }

    /// Get a reference to the type section
    pub(crate) fn get_type_section(&self) -> &TypeSection {
        &self.type_section
    }

    /// Get a cloned type section for module assembly
    pub(crate) fn clone_type_section(&self) -> TypeSection {
        self.type_section.clone()
    }

    /// Add a function type to the type section
    pub(crate) fn add_function_type(
        &mut self, 
        params: &[WasmType], 
        return_type: Option<WasmType>
    ) -> Result<u32, CompilerError> {
        let param_val_types: Vec<ValType> = params.iter().map(|t| (*t).into()).collect();
        let return_val_type: Vec<ValType> = return_type.map(|t| vec![t.into()]).unwrap_or_default();

        self.type_section.function(param_val_types.clone(), return_val_type.clone());
        let type_index = self.function_types.len() as u32;

        let parser_param_types: Vec<wasmparser::ValType> = param_val_types.iter()
            .map(|vt| WasmType::from(*vt).to_parser_val_type())
            .collect();
        let parser_result_types: Vec<wasmparser::ValType> = return_val_type.iter()
            .map(|vt| WasmType::from(*vt).to_parser_val_type())
            .collect();
        self.function_types.push(FuncType::new(parser_param_types, parser_result_types));

        Ok(type_index)
    }

    /// Get the function types stored in this manager
    pub(crate) fn get_function_types(&self) -> &Vec<FuncType> {
        &self.function_types
    }

    /// Check if conversion is possible between two types
    pub(crate) fn can_convert(&self, from: WasmType, to: WasmType) -> bool {
        match (from, to) {
            (WasmType::I32, WasmType::F64) => true,
            (WasmType::F64, WasmType::I32) => true,
            _ => from == to,
        }
    }
    
    /// Check if the given expression is a string type
    pub(crate) fn is_string_type(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Literal(Value::String(_)) => true,
            Expression::StringInterpolation(_) => true,
            // For variables, ideally this would check the variable's type
            _ => false,
        }
    }

    /// Convert AST type to WasmType
    pub(crate) fn ast_type_to_wasm_type(&self, ast_type: &Type) -> Result<WasmType, CompilerError> {
        Ok(WasmType::from(ast_type))
    }

    /// Infer the WasmType from a Value
    pub(crate) fn infer_type(&self, value: &Value) -> Result<WasmType, CompilerError> {
        Ok(match value {
            Value::Integer(_) => WasmType::I32,
            Value::Boolean(_) => WasmType::I32, // Booleans are represented as I32 in WASM
            Value::String(_) => WasmType::I32,  // Strings are pointers in WASM
            Value::Float(_) => WasmType::F64,
            Value::Array(_) => WasmType::I32,   // Arrays are pointers in WASM
            Value::Matrix(_) => WasmType::I32,  // Matrices are pointers in WASM
            Value::Void => WasmType::I32,       // Void represented as I32
            // Sized types
            Value::Integer8(_) => WasmType::I32,
            Value::Integer8u(_) => WasmType::I32,
            Value::Integer16(_) => WasmType::I32,
            Value::Integer16u(_) => WasmType::I32,
            Value::Integer32(_) => WasmType::I32,
            Value::Integer64(_) => WasmType::I64,
            Value::Float32(_) => WasmType::F32,
            Value::Float64(_) => WasmType::F64,
        })
    }

    pub fn convert_value_to_wasm_type(&self, value: &Value) -> Result<WasmType, CompilerError> {
        Ok(match value {
            Value::Integer(_) => WasmType::I32,
            Value::Boolean(_) => WasmType::I32, // Booleans are represented as I32 in WASM
            Value::String(_) => WasmType::I32,  // Strings are pointers in WASM
            Value::Float(_) => WasmType::F64,
            Value::Array(_) => WasmType::I32,   // Arrays are pointers in WASM
            Value::Matrix(_) => WasmType::I32,  // Matrices are pointers in WASM
            Value::Void => WasmType::I32,       // Void represented as I32
            // Sized types
            Value::Integer8(_) => WasmType::I32,
            Value::Integer8u(_) => WasmType::I32,
            Value::Integer16(_) => WasmType::I32,
            Value::Integer16u(_) => WasmType::I32,
            Value::Integer32(_) => WasmType::I32,
            Value::Integer64(_) => WasmType::I64,
            Value::Float32(_) => WasmType::F32,
            Value::Float64(_) => WasmType::F64,
        })
    }
} 