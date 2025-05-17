// Standalone type conversion test
// Run with: rustc type_test.rs && ./type_test

// Define ValType enum for standalone testing
#[derive(Debug, Clone, Copy, PartialEq)]
enum ValType {
    I32,
    I64,
    F32,
    F64,
    V128,
    FuncRef,
    ExternRef
}

// Define WasmType enum for standalone testing
#[derive(Debug, Clone, Copy, PartialEq)]
enum WasmType {
    I32,
    I64,
    F32,
    F64,
    V128,
}

impl WasmType {
    // Convert to ValType
    fn to_val_type(self) -> ValType {
        match self {
            WasmType::I32 => ValType::I32,
            WasmType::I64 => ValType::I64,
            WasmType::F32 => ValType::F32,
            WasmType::F64 => ValType::F64,
            WasmType::V128 => ValType::V128,
        }
    }

    // Convert from ValType
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

    // Convert to (integer, ValType) tuple representation
    fn to_tuple(self) -> (u8, ValType) {
        match self {
            WasmType::I32 => (0, ValType::I32),
            WasmType::I64 => (1, ValType::I64),
            WasmType::F32 => (2, ValType::F32),
            WasmType::F64 => (3, ValType::F64),
            WasmType::V128 => (4, ValType::V128),
        }
    }

    // Convert from (integer, ValType) tuple representation
    fn from_tuple(tuple: (u8, ValType)) -> Self {
        match tuple {
            (0, ValType::I32) => WasmType::I32,
            (1, ValType::I64) => WasmType::I64,
            (2, ValType::F32) => WasmType::F32,
            (3, ValType::F64) => WasmType::F64,
            (4, ValType::V128) => WasmType::V128,
            _ => panic!("Invalid type tuple: ({}, {:?})", tuple.0, tuple.1),
        }
    }
}

// Helper functions
fn to_val_type(wasm_type: WasmType) -> ValType {
    wasm_type.to_val_type()
}

fn from_val_type(val_type: ValType) -> WasmType {
    WasmType::from_val_type(val_type)
}

fn to_tuple(wasm_type: WasmType) -> (u8, ValType) {
    wasm_type.to_tuple()
}

fn from_tuple(tuple: (u8, ValType)) -> WasmType {
    WasmType::from_tuple(tuple)
}

// Assert macro for tests
macro_rules! assert_eq_or_panic {
    ($left:expr, $right:expr) => {
        if $left != $right {
            panic!("Assertion failed: {:?} != {:?}", $left, $right);
        } else {
            println!("✅ Assertion passed: {:?} == {:?}", $left, $right);
        }
    };
}

fn main() {
    println!("Starting WasmType conversion test...");
    
    // Test to_val_type and from_val_type
    let types = [
        WasmType::I32,
        WasmType::I64,
        WasmType::F32,
        WasmType::F64,
        WasmType::V128,
    ];
    
    for wasm_type in &types {
        let val_type = to_val_type(*wasm_type);
        let round_trip = from_val_type(val_type);
        assert_eq_or_panic!(*wasm_type, round_trip);
        println!("Round trip to_val_type/from_val_type successful for {:?}", wasm_type);
    }
    
    // Test to_tuple and from_tuple
    for wasm_type in &types {
        let tuple = to_tuple(*wasm_type);
        let round_trip = from_tuple(tuple);
        assert_eq_or_panic!(*wasm_type, round_trip);
        println!("Round trip to_tuple/from_tuple successful for {:?}", wasm_type);
    }
    
    // Test direct conversions
    let tuples = [
        (0, ValType::I32),
        (1, ValType::I64),
        (2, ValType::F32),
        (3, ValType::F64),
        (4, ValType::V128),
    ];
    
    for tuple in &tuples {
        let wasm_type = from_tuple(*tuple);
        let round_trip = to_tuple(wasm_type);
        assert_eq_or_panic!(*tuple, round_trip);
        println!("Round trip from_tuple/to_tuple successful for {:?}", tuple);
    }
    
    // Test ValType conversions
    let val_types = [
        ValType::I32,
        ValType::I64,
        ValType::F32,
        ValType::F64,
        ValType::V128,
    ];
    
    for val_type in &val_types {
        let wasm_type = from_val_type(*val_type);
        let round_trip = to_val_type(wasm_type);
        assert_eq_or_panic!(*val_type, round_trip);
        println!("Round trip from_val_type/to_val_type successful for {:?}", val_type);
    }
    
    println!("All type conversion tests passed successfully!");
} 