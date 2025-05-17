use wasm_encoder::ValType;
use crate::ast::Type as AstType;

/// WebAssembly value types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WasmType {
    I32,
    I64,
    F32,
    F64,
    V128,
    Unit,
}

// Define the ValTypeConverter trait that was referenced in the codebase
pub trait ValTypeConverter {
    fn to_val_type(&self) -> ValType;
    fn from_val_type(val_type: ValType) -> Self where Self: Sized;
}

impl ValTypeConverter for WasmType {
    fn to_val_type(&self) -> ValType {
        match self {
            WasmType::I32 => ValType::I32,
            WasmType::I64 => ValType::I64,
            WasmType::F32 => ValType::F32,
            WasmType::F64 => ValType::F64,
            WasmType::V128 => ValType::V128,
            WasmType::Unit => ValType::I32, // Map Unit to I32 for WebAssembly compatibility
        }
    }

    fn from_val_type(val_type: ValType) -> Self {
        match val_type {
            ValType::I32 => WasmType::I32,
            ValType::I64 => WasmType::I64,
            ValType::F32 => WasmType::F32,
            ValType::F64 => WasmType::F64,
            ValType::V128 => WasmType::V128,
            _ => panic!("Unsupported ValType"),
        }
    }
}

impl WasmType {
    /// Convert to wasm_encoder::ValType
    pub fn to_val_type(self) -> ValType {
        <Self as ValTypeConverter>::to_val_type(&self)
    }

    /// Convert from wasm_encoder::ValType
    pub fn from_val_type(val_type: ValType) -> Self {
        <Self as ValTypeConverter>::from_val_type(val_type)
    }

    /// Convert to (integer, ValType) tuple representation used in some modules
    pub fn to_tuple(self) -> (u8, ValType) {
        match self {
            WasmType::I32 => (0, ValType::I32),
            WasmType::I64 => (1, ValType::I64),
            WasmType::F32 => (2, ValType::F32),
            WasmType::F64 => (3, ValType::F64),
            WasmType::V128 => (4, ValType::V128),
            WasmType::Unit => (5, ValType::I32), // Map Unit to I32 for WebAssembly compatibility
        }
    }

    /// Convert from (integer, ValType) tuple representation
    pub fn from_tuple(tuple: (u8, ValType)) -> Self {
        match tuple {
            (0, ValType::I32) => WasmType::I32,
            (1, ValType::I64) => WasmType::I64,
            (2, ValType::F32) => WasmType::F32,
            (3, ValType::F64) => WasmType::F64,
            (4, ValType::V128) => WasmType::V128,
            (5, ValType::I32) => WasmType::Unit,
            _ => panic!("Invalid type tuple: ({}, {:?})", tuple.0, tuple.1),
        }
    }
    
    /// Get the size in bytes for this type
    pub fn size_in_bytes(&self) -> usize {
        match self {
            WasmType::I32 => 4,
            WasmType::I64 => 8,
            WasmType::F32 => 4,
            WasmType::F64 => 8,
            WasmType::V128 => 16,
            WasmType::Unit => 0,
        }
    }
    
    /// Get the type ID
    pub fn to_id(&self) -> u32 {
        match self {
            WasmType::I32 => 1,  // INTEGER_TYPE_ID
            WasmType::I64 => 1,  // Also maps to INTEGER_TYPE_ID
            WasmType::F32 => 2,  // FLOAT_TYPE_ID
            WasmType::F64 => 2,  // Also maps to FLOAT_TYPE_ID
            WasmType::V128 => 5, // MATRIX_TYPE_ID (arbitrary choice)
            WasmType::Unit => 0, // No specific type ID
        }
    }
    
    /// Convert to wasmparser ValType
    pub fn to_parser_val_type(&self) -> wasmparser::ValType {
        match self {
            WasmType::I32 => wasmparser::ValType::I32,
            WasmType::I64 => wasmparser::ValType::I64,
            WasmType::F32 => wasmparser::ValType::F32,
            WasmType::F64 => wasmparser::ValType::F64,
            WasmType::V128 => wasmparser::ValType::V128,
            WasmType::Unit => wasmparser::ValType::I32, // Map Unit to I32 for WebAssembly compatibility
        }
    }
    
    /// Convert from wasmparser ValType
    pub fn from_parser_val_type(val_type: wasmparser::ValType) -> Self {
        match val_type {
            wasmparser::ValType::I32 => WasmType::I32,
            wasmparser::ValType::I64 => WasmType::I64,
            wasmparser::ValType::F32 => WasmType::F32,
            wasmparser::ValType::F64 => WasmType::F64,
            wasmparser::ValType::V128 => WasmType::V128,
            _ => panic!("Unsupported wasmparser ValType"),
        }
    }
}

impl From<WasmType> for ValType {
    fn from(wasm_type: WasmType) -> Self {
        wasm_type.to_val_type()
    }
}

impl From<ValType> for WasmType {
    fn from(val_type: ValType) -> Self {
        WasmType::from_val_type(val_type)
    }
}

impl From<&AstType> for WasmType {
    fn from(ast_type: &AstType) -> Self {
        match ast_type {
            AstType::Integer | AstType::Boolean | AstType::Byte | AstType::Unsigned => WasmType::I32,
            AstType::Long | AstType::ULong => WasmType::I64,
            AstType::Number => WasmType::F64,
            AstType::String | AstType::Array(_) | AstType::Matrix(_) | AstType::Object(_) => WasmType::I32,
            _ => WasmType::I32, // Default to I32 for other types
        }
    }
}

// Helper function to convert WasmType to (integer, ValType) tuple
pub fn wasm_type_to_tuple(wasm_type: WasmType) -> (u32, ValType) {
    (1, wasm_type.into())
}

// Helper function to convert a slice of WasmType to Vec<(u32, ValType)>
pub fn wasm_types_to_tuples(wasm_types: &[WasmType]) -> Vec<(u32, ValType)> {
    wasm_types.iter().map(|&t| wasm_type_to_tuple(t)).collect()
}

// Helper function to convert Vec<(u32, ValType)> to Vec<WasmType>
pub fn tuples_to_wasm_types(tuples: &[(u32, ValType)]) -> Vec<WasmType> {
    tuples.iter().map(|(_, vt)| (*vt).into()).collect()
}

// Helper functions for common conversions
pub fn to_val_type(wasm_type: WasmType) -> ValType {
    wasm_type.to_val_type()
}

pub fn from_val_type(val_type: ValType) -> WasmType {
    WasmType::from_val_type(val_type)
}

pub fn to_tuple(wasm_type: WasmType) -> (u8, ValType) {
    wasm_type.to_tuple()
}

pub fn from_tuple(tuple: (u8, ValType)) -> WasmType {
    WasmType::from_tuple(tuple)
}

// Add wrapper functions that fix the type conversion issue

// Original version returns (u8, ValType) but we need WasmType
pub fn to_tuple_wrapper(wasm_type: WasmType) -> WasmType {
    // Just pass through the original type
    wasm_type
}

// Original version returns ValType but we need (integer, ValType)
pub fn from_tuple_wrapper(tuple: (u8, ValType)) -> (u8, ValType) {
    // Just pass through the original tuple
    tuple
} 