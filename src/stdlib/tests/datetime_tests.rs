use crate::codegen::CodeGenerator;
use crate::error::CompilerError;
use crate::stdlib::{DateOperations, TimeOperations, FormatOperations};
use wasmtime::{Store, Module, Instance, Val};

struct TestDateTime {
    store: Store<()>,
    instance: Instance,
}

impl TestDateTime {
    fn new() -> Result<Self, CompilerError> {
        let mut codegen = CodeGenerator::new();
        
        // Register all datetime operations
        let date_ops = DateOperations::new();
        let time_ops = TimeOperations::new();
        let format_ops = FormatOperations::new();
        
        date_ops.register_functions(&mut codegen)?;
        time_ops.register_functions(&mut codegen)?;
        format_ops.register_functions(&mut codegen)?;

        let wasm = codegen.build();
        let engine = wasmtime::Engine::default();
        let module = Module::new(&engine, &wasm)?;
        let mut store = Store::new(&engine, ());
        let instance = Instance::new(&mut store, &module, &[])?;

        Ok(Self { store, instance })
    }

    fn call_func(&mut self, name: &str, params: &[Val]) -> Result<Val, CompilerError> {
        let mut results = vec![Val::I32(0)];
        self.instance
            .get_func(&mut self.store, name)
            .ok_or_else(|| CompilerError::RuntimeError(format!("Function {} not found", name)))?
            .call(&mut self.store, params, &mut results)
            .map_err(|e| CompilerError::RuntimeError(e.to_string()))?;
        Ok(results[0])
    }
}

#[test]
fn test_current_time() -> Result<(), CompilerError> {
    let mut dt = TestDateTime::new()?;
    
    // Test now()
    let now_result = dt.call_func("now", &[])?;
    let timestamp = now_result.unwrap_i64();
    assert!(timestamp > 0, "Current timestamp should be positive");
    
    // Test now_ms()
    let now_ms_result = dt.call_func("now_ms", &[])?;
    let timestamp_ms = now_ms_result.unwrap_i64();
    assert!(timestamp_ms > timestamp * 1000, "Millisecond timestamp should be larger");

    Ok(())
}

#[test]
fn test_time_components() -> Result<(), CompilerError> {
    let mut dt = TestDateTime::new()?;
    let timestamp = 1234567890; // 2009-02-13 23:31:30 UTC

    // Test hour
    let hour = dt.call_func("hour", &[Val::I64(timestamp)])?.unwrap_i32();
    assert!(hour >= 0 && hour < 24, "Hour should be between 0 and 23");

    // Test minute
    let minute = dt.call_func("minute", &[Val::I64(timestamp)])?.unwrap_i32();
    assert!(minute >= 0 && minute < 60, "Minute should be between 0 and 59");

    // Test second
    let second = dt.call_func("second", &[Val::I64(timestamp)])?.unwrap_i32();
    assert!(second >= 0 && second < 60, "Second should be between 0 and 59");

    // Test millisecond
    let ms = dt.call_func("millisecond", &[Val::I64(timestamp)])?.unwrap_i32();
    assert!(ms >= 0 && ms < 1000, "Millisecond should be between 0 and 999");

    Ok(())
}

#[test]
fn test_date_components() -> Result<(), CompilerError> {
    let mut dt = TestDateTime::new()?;
    let timestamp = 1234567890; // 2009-02-13 23:31:30 UTC

    // Test year
    let year = dt.call_func("year", &[Val::I64(timestamp)])?.unwrap_i32();
    assert_eq!(year, 2009, "Year should be 2009");

    // Test month
    let month = dt.call_func("month", &[Val::I64(timestamp)])?.unwrap_i32();
    assert_eq!(month, 2, "Month should be 2 (February)");

    // Test day
    let day = dt.call_func("day", &[Val::I64(timestamp)])?.unwrap_i32();
    assert_eq!(day, 13, "Day should be 13");

    // Test day_of_week
    let dow = dt.call_func("day_of_week", &[Val::I64(timestamp)])?.unwrap_i32();
    assert_eq!(dow, 5, "Day of week should be 5 (Friday)");

    Ok(())
}

#[test]
fn test_leap_year() -> Result<(), CompilerError> {
    let mut dt = TestDateTime::new()?;

    // Test leap years
    let test_years = vec![
        (2000, true),  // Century leap year
        (2004, true),  // Regular leap year
        (2100, false), // Century non-leap year
        (2020, true),  // Recent leap year
        (2021, false), // Non-leap year
    ];

    for (year, expected) in test_years {
        let result = dt.call_func("is_leap_year", &[Val::I32(year)])?.unwrap_i32();
        assert_eq!(
            result == 1,
            expected,
            "{} leap year test failed",
            year
        );
    }

    Ok(())
}

#[test]
fn test_date_arithmetic() -> Result<(), CompilerError> {
    let mut dt = TestDateTime::new()?;
    let base_timestamp = 1234567890; // 2009-02-13 23:31:30 UTC

    // Test add_days
    let days_to_add = 10;
    let new_timestamp = dt.call_func(
        "add_days",
        &[Val::I64(base_timestamp), Val::I32(days_to_add)],
    )?.unwrap_i64();
    assert_eq!(
        new_timestamp,
        base_timestamp + (days_to_add as i64 * 86400),
        "Adding days calculation incorrect"
    );

    // Test add_months
    let months_result = dt.call_func(
        "add_months",
        &[Val::I64(base_timestamp), Val::I32(1)],
    )?.unwrap_i64();
    let new_month = dt.call_func("month", &[Val::I64(months_result)])?.unwrap_i32();
    assert_eq!(new_month, 3, "Adding 1 month should result in March");

    Ok(())
}

#[test]
fn test_timezone_operations() -> Result<(), CompilerError> {
    let mut dt = TestDateTime::new()?;
    let timestamp = 1234567890; // UTC timestamp

    // Test UTC conversion
    let utc_timestamp = dt.call_func("to_utc", &[Val::I64(timestamp)])?.unwrap_i64();
    let local_timestamp = dt.call_func("from_utc", &[Val::I64(utc_timestamp)])?.unwrap_i64();
    
    // Converting to UTC and back should give us the original timestamp
    assert_eq!(timestamp, local_timestamp, "UTC conversion round-trip failed");

    // Test timezone offset
    let offset = dt.call_func("timezone_offset", &[])?.unwrap_i32();
    assert!(offset >= -12 * 3600 && offset <= 14 * 3600, "Timezone offset should be within valid range");

    Ok(())
}

#[test]
fn test_formatting() -> Result<(), CompilerError> {
    let mut dt = TestDateTime::new()?;
    let timestamp = 1234567890; // 2009-02-13 23:31:30 UTC

    // Test date formatting
    let date_ptr = dt.call_func("format_date", &[Val::I64(timestamp)])?.unwrap_i32();
    assert!(date_ptr > 0, "Date format should return valid string pointer");

    // Test time formatting
    let time_ptr = dt.call_func("format_time", &[Val::I64(timestamp)])?.unwrap_i32();
    assert!(time_ptr > 0, "Time format should return valid string pointer");

    // Test ISO datetime formatting
    let iso_ptr = dt.call_func("format_iso_datetime", &[Val::I64(timestamp)])?.unwrap_i32();
    assert!(iso_ptr > 0, "ISO datetime format should return valid string pointer");

    Ok(())
}

#[test]
fn test_date_comparison() -> Result<(), CompilerError> {
    let mut dt = TestDateTime::new()?;
    let date1 = 1234567890;
    let date2 = date1 + 86400; // One day later

    // Test days_between
    let days = dt.call_func(
        "days_between",
        &[Val::I64(date1), Val::I64(date2)],
    )?.unwrap_i32();
    assert_eq!(days, 1, "Days between should be 1");

    // Test compare_dates
    let comparison = dt.call_func(
        "compare_dates",
        &[Val::I64(date1), Val::I64(date2)],
    )?.unwrap_i32();
    assert!(comparison < 0, "First date should be less than second date");

    Ok(())
} 