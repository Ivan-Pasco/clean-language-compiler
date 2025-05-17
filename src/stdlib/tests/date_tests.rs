use crate::codegen::CodeGenerator;
use crate::error::CompilerError;
use crate::stdlib::DateOperations;
use wasmtime::{Store, Module, Instance, Val};

struct TestDate {
    store: Store<()>,
    instance: Instance,
}

impl TestDate {
    fn new() -> Result<Self, CompilerError> {
        let mut codegen = CodeGenerator::new();
        
        // Register date operations
        let date_ops = DateOperations::new();
        date_ops.register_functions(&mut codegen)?;

        let wasm = codegen.build();
        let engine = wasmtime::Engine::default();
        let module = Module::new(&engine, &wasm)?;
        let mut store = Store::new(&engine, ());
        let instance = Instance::new(&mut store, &module, &[])?;

        Ok(Self { store, instance })
    }

    fn call_func(&mut self, name: &str, params: &[Val]) -> Result<Vec<Val>, CompilerError> {
        let mut results = vec![Val::I32(0)];
        self.instance
            .get_func(&mut self.store, name)
            .ok_or_else(|| CompilerError::RuntimeError(format!("Function {} not found", name)))?
            .call(&mut self.store, params, &mut results)
            .map_err(|e| CompilerError::RuntimeError(e.to_string()))?;
        Ok(results)
    }
}

#[test]
fn test_date_components() -> Result<(), CompilerError> {
    let mut date = TestDate::new()?;
    let timestamp = 1234567890; // 2009-02-13 23:31:30 UTC

    // Test year
    let result = date.call_func("year", &[Val::I64(timestamp)])?;
    assert_eq!(result[0].unwrap_i32(), 2009, "Year should be 2009");

    // Test month
    let result = date.call_func("month", &[Val::I64(timestamp)])?;
    assert_eq!(result[0].unwrap_i32(), 2, "Month should be 2 (February)");

    // Test day
    let result = date.call_func("day", &[Val::I64(timestamp)])?;
    assert_eq!(result[0].unwrap_i32(), 13, "Day should be 13");

    // Test day_of_week
    let result = date.call_func("day_of_week", &[Val::I64(timestamp)])?;
    assert_eq!(result[0].unwrap_i32(), 5, "Day of week should be 5 (Friday)");

    Ok(())
}

#[test]
fn test_leap_year() -> Result<(), CompilerError> {
    let mut date = TestDate::new()?;

    // Test leap years
    let test_years = vec![
        (2000, true),  // Century leap year
        (2004, true),  // Regular leap year
        (2100, false), // Century non-leap year
        (2020, true),  // Recent leap year
        (2021, false), // Non-leap year
    ];

    for (year, expected) in test_years {
        let result = date.call_func("is_leap_year", &[Val::I32(year)])?;
        assert_eq!(
            result[0].unwrap_i32() == 1,
            expected,
            "{} leap year test failed",
            year
        );
    }

    Ok(())
}

#[test]
fn test_date_arithmetic() -> Result<(), CompilerError> {
    let mut date = TestDate::new()?;
    let base_timestamp = 1234567890; // 2009-02-13 23:31:30 UTC

    // Test add_days
    let days_to_add = 10;
    let result = date.call_func(
        "add_days",
        &[Val::I64(base_timestamp), Val::I32(days_to_add)],
    )?;
    assert_eq!(
        result[0].unwrap_i64(),
        base_timestamp + (days_to_add as i64 * 86400),
        "Adding days calculation incorrect"
    );

    // Test add_months
    let result = date.call_func(
        "add_months",
        &[Val::I64(base_timestamp), Val::I32(1)],
    )?;
    let month_result = date.call_func("month", &[Val::I64(result[0].unwrap_i64())])?;
    assert_eq!(month_result[0].unwrap_i32(), 3, "Adding 1 month should result in March");

    Ok(())
}

#[test]
fn test_days_between() -> Result<(), CompilerError> {
    let mut date = TestDate::new()?;
    let date1 = 1234567890; // 2009-02-13 23:31:30 UTC
    let date2 = date1 + 86400; // One day later

    // Test days_between
    let result = date.call_func(
        "days_between",
        &[Val::I64(date1), Val::I64(date2)],
    )?;
    assert_eq!(result[0].unwrap_i32(), 1, "Days between should be 1");

    Ok(())
} 