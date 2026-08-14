//! Milestone 1 step 1 check: the workspace builds and the canonical entry
//! point exists with the §14.2.1 signature. Deeper behaviour is proven by
//! the per-step suites (`request_validation.rs`, …).

use clean_compiler::{compile, CompileError};

mod common;

#[test]
fn compile_symbol_exists_and_pipeline_reports_its_own_prefix() {
    let request = common::minimal_valid_request();
    match compile(request) {
        // The pipeline prefix implemented so far ran clean; everything after
        // it is still unbuilt. This arm shrinks as milestone steps land.
        Err(CompileError::Incomplete { completed }) => {
            assert_eq!(completed, "request-validation");
        }
        Err(CompileError::Rejected(diagnostics)) => {
            panic!("minimal valid request was rejected: {diagnostics:#?}");
        }
        Ok(_) => panic!("pipeline claims completeness it does not have yet"),
    }
}
