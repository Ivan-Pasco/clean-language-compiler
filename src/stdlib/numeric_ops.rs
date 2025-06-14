use crate::codegen::CodeGenerator;
use crate::types::WasmType;
use crate::error::CompilerError;

use wasm_encoder::Instruction;
use crate::stdlib::register_stdlib_function;

/// Numeric operations implementation
pub struct NumericOperations {}

impl NumericOperations {
    pub fn new() -> Self {
        Self {}
    }

    pub fn register_functions(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // Add function
        register_stdlib_function(
            codegen,
            "add",
            &[WasmType::F64, WasmType::F64],
            Some(WasmType::F64),
            vec![
                Instruction::LocalGet(0),
                Instruction::LocalGet(1),
                Instruction::F64Add,
            ]
        )?;

        // Subtract function
        register_stdlib_function(
            codegen,
            "subtract",
            &[WasmType::F64, WasmType::F64],
            Some(WasmType::F64),
            vec![
                Instruction::LocalGet(0),
                Instruction::LocalGet(1),
                Instruction::F64Sub,
            ]
        )?;

        // Multiply function
        register_stdlib_function(
            codegen,
            "multiply",
            &[WasmType::F64, WasmType::F64],
            Some(WasmType::F64),
            vec![
                Instruction::LocalGet(0),
                Instruction::LocalGet(1),
                Instruction::F64Mul,
            ]
        )?;

        // Divide function
        register_stdlib_function(
            codegen,
            "divide",
            &[WasmType::F64, WasmType::F64],
            Some(WasmType::F64),
            vec![
                Instruction::LocalGet(0),
                Instruction::LocalGet(1),
                Instruction::F64Div,
            ]
        )?;

        // Equals function
        register_stdlib_function(
            codegen,
            "equals",
            &[WasmType::F64, WasmType::F64],
            Some(WasmType::I32),
            vec![
                Instruction::LocalGet(0),
                Instruction::LocalGet(1),
                Instruction::F64Eq,
            ]
        )?;

        // Not equals function
        register_stdlib_function(
            codegen,
            "not_equals",
            &[WasmType::F64, WasmType::F64],
            Some(WasmType::I32),
            vec![
                Instruction::LocalGet(0),
                Instruction::LocalGet(1),
                Instruction::F64Ne,
            ]
        )?;

        // Less than function
        register_stdlib_function(
            codegen,
            "less_than",
            &[WasmType::F64, WasmType::F64],
            Some(WasmType::I32),
            vec![
                Instruction::LocalGet(0),
                Instruction::LocalGet(1),
                Instruction::F64Lt,
            ]
        )?;

        // Greater than function
        register_stdlib_function(
            codegen,
            "greater_than",
            &[WasmType::F64, WasmType::F64],
            Some(WasmType::I32),
            vec![
                Instruction::LocalGet(0),
                Instruction::LocalGet(1),
                Instruction::F64Gt,
            ]
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::CodeGenerator;
    use wasmtime::{Engine, Instance, Module, Store, Val};

    fn setup_test_environment() -> (Store<()>, Instance) {
        let mut codegen = CodeGenerator::new();
        let numeric_ops = NumericOperations::new();
        numeric_ops.register_functions(&mut codegen).unwrap();

        let engine = Engine::default();
        let wasm_bytes = codegen.finish();
        let module = Module::new(&engine, &wasm_bytes).unwrap();
        let mut store = Store::new(&engine, ());
        let instance = Instance::new(&mut store, &module, &[]).unwrap();
        (store, instance)
    }

    #[test]
    fn test_add() {
        let (mut store, instance) = setup_test_environment();
        let add = instance.get_func(&mut store, "add").unwrap();
        
        let mut results = vec![Val::F64(0)];
        add.call(&mut store, &[
            Val::F64(f64::to_bits(2.5)), 
            Val::F64(f64::to_bits(3.7))
        ], &mut results).unwrap();
        
        let result = f64::from_bits(results[0].unwrap_i64() as u64);
        assert!((result - 6.2).abs() < f64::EPSILON);
    }

    #[test]
    fn test_subtract() {
        let (mut store, instance) = setup_test_environment();
        let subtract = instance.get_func(&mut store, "subtract").unwrap();
        
        let mut results = vec![Val::F64(0)];
        subtract.call(&mut store, &[
            Val::F64(f64::to_bits(5.0)), 
            Val::F64(f64::to_bits(2.5))
        ], &mut results).unwrap();
        
        let result = f64::from_bits(results[0].unwrap_i64() as u64);
        assert!((result - 2.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_equals() {
        let (mut store, instance) = setup_test_environment();
        let equals = instance.get_func(&mut store, "equals").unwrap();
        
        let mut results = vec![Val::I32(0)];
        equals.call(&mut store, &[
            Val::F64(f64::to_bits(2.5)), 
            Val::F64(f64::to_bits(2.5))
        ], &mut results).unwrap();
        
        assert_eq!(results[0].unwrap_i32(), 1);
    }

    #[test]
    fn test_not_equals() {
        let (mut store, instance) = setup_test_environment();
        let not_equals = instance.get_func(&mut store, "not_equals").unwrap();
        
        let mut results = vec![Val::I32(0)];
        not_equals.call(&mut store, &[
            Val::F64(f64::to_bits(2.5)), 
            Val::F64(f64::to_bits(3.0))
        ], &mut results).unwrap();
        
        assert_eq!(results[0].unwrap_i32(), 1);
    }

    #[test]
    fn test_less_than() {
        let (mut store, instance) = setup_test_environment();
        let less_than = instance.get_func(&mut store, "less_than").unwrap();
        
        let mut results = vec![Val::I32(0)];
        less_than.call(&mut store, &[
            Val::F64(f64::to_bits(2.5)), 
            Val::F64(f64::to_bits(3.0))
        ], &mut results).unwrap();
        
        assert_eq!(results[0].unwrap_i32(), 1);
    }

    #[test]
    fn test_greater_than() {
        let (mut store, instance) = setup_test_environment();
        let greater_than = instance.get_func(&mut store, "greater_than").unwrap();
        
        let mut results = vec![Val::I32(0)];
        greater_than.call(&mut store, &[
            Val::F64(f64::to_bits(3.0)), 
            Val::F64(f64::to_bits(2.5))
        ], &mut results).unwrap();
        
        assert_eq!(results[0].unwrap_i32(), 1);
    }
} 