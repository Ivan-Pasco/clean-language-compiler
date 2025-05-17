//! Module for type management during code generation.

use wasmparser::FuncType;
use wasm_encoder::{ValType, TypeSection};
use crate::types::WasmType;
use crate::error::CompilerError;
use crate::ast::{self, Type, Expression, Value};

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

    /// Add a function type to the type section
    pub(crate) fn add_function_type(
        &mut self, 
        params: &[WasmType], 
        return_type: Option<WasmType>
    ) -> Result<u32, CompilerError> {
        let param_val_types: Vec<ValType> = params.iter().map(|t| (*t).into()).collect();
        let return_val_type: Vec<ValType> = return_type.map(|t| vec![t.into()]).unwrap_or_default();

        self.type_section.function(param_val_types.clone(), return_val_type.clone());
        let type_index = self.type_section.len() - 1;

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
            Expression::StringConcat(_) => true,
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
            Value::Number(_) => WasmType::F64,
            Value::String(_) => WasmType::I32, // Pointer to string
            Value::Boolean(_) => WasmType::I32, // 0 or 1
            Value::Array(_) => WasmType::I32, // Pointer to array
            Value::Matrix(_) => WasmType::I32, // Pointer to matrix
            Value::Byte(_) => WasmType::I32,
            Value::Unsigned(_) => WasmType::I32,
            Value::Long(_) => WasmType::I64,
            Value::ULong(_) => WasmType::I64,
            Value::Big(_) => WasmType::I32, // Pointer to big integer
            Value::UBig(_) => WasmType::I32, // Pointer to unsigned big integer
            Value::Float(_) => WasmType::F32,
            Value::Null | Value::Unit => WasmType::I32, // Null pointer or unit value
        })
    }

    pub fn convert_value_to_wasm_type(&self, value: &Value) -> Result<WasmType, CompilerError> {
        Ok(match value {
            Value::Integer(_) => WasmType::I32,
            Value::Number(_) => WasmType::F64,
            Value::String(_) => WasmType::I32, // Pointer to string
            Value::Boolean(_) => WasmType::I32, // 0 or 1
            Value::Array(_) => WasmType::I32, // Pointer to array
            Value::Matrix(_) => WasmType::I32, // Pointer to matrix
            Value::Byte(_) => WasmType::I32,
            Value::Unsigned(_) => WasmType::I32,
            Value::Long(_) => WasmType::I64,
            Value::ULong(_) => WasmType::I64,
            Value::Big(_) => WasmType::I32, // Pointer to big integer
            Value::UBig(_) => WasmType::I32, // Pointer to unsigned big integer
            Value::Float(_) => WasmType::F32,
            Value::Null | Value::Unit => WasmType::I32, // Null pointer or unit value
        })
    }
} 