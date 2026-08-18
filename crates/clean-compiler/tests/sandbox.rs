//! Unit checks for the pass-[6] handler sandbox (ADR-0004): each failure
//! class is distinguishable — timeout (epoch), memory (limiter denial),
//! forbidden import (stub), crash (any other trap), malformed artifact —
//! and a well-behaved handler round-trips its envelope byte-exactly.

use clean_compiler::blocks::sandbox::{run_handler, HandlerOutcome};
use clean_compiler_types::request::CompileLimits;

fn tight_limits() -> CompileLimits {
    CompileLimits {
        handler_timeout_ms: 100,
        handler_memory_mb: 1,
        ..CompileLimits::default()
    }
}

/// A bump-allocating handler that echoes its input back as its envelope.
const ECHO: &str = r#"
(module
  (memory (export "memory") 1)
  (global $heap (mut i32) (i32.const 1024))
  (func (export "alloc") (param $size i32) (result i32)
    (local $ptr i32)
    global.get $heap
    local.set $ptr
    (global.set $heap (i32.add (global.get $heap) (local.get $size)))
    local.get $ptr)
  (func (export "expand") (param $ptr i32) (param $len i32) (result i32)
    (i32.store (i32.const 0) (local.get $ptr))
    (i32.store (i32.const 4) (local.get $len))
    (i32.const 0)))
"#;

#[test]
fn well_behaved_handler_round_trips_its_envelope() {
    let input = r#"{"name":"data","arguments":[],"body":[],"attributes":[]}"#;
    let run = run_handler(ECHO.as_bytes(), input, &tight_limits());
    match run.outcome {
        HandlerOutcome::Success(text) => assert_eq!(text, input),
        other => panic!("expected success, got {other:?}"),
    }
    assert!(
        run.memory_bytes >= 65536,
        "linear memory footprint recorded"
    );
}

#[test]
fn infinite_loop_times_out_via_epoch_deadline() {
    let wat = r#"
    (module
      (memory (export "memory") 1)
      (func (export "alloc") (param i32) (result i32) (i32.const 1024))
      (func (export "expand") (param i32 i32) (result i32)
        (loop $spin br $spin)
        (i32.const 0)))
    "#;
    let run = run_handler(wat.as_bytes(), "{}", &tight_limits());
    assert!(
        matches!(run.outcome, HandlerOutcome::Timeout),
        "expected timeout, got {:?}",
        run.outcome
    );
}

#[test]
fn memory_hog_is_classified_as_budget_not_crash() {
    // Grows one page at a time until the limiter denies it, then traps.
    let wat = r#"
    (module
      (memory (export "memory") 1)
      (func (export "alloc") (param i32) (result i32) (i32.const 1024))
      (func (export "expand") (param i32 i32) (result i32)
        (loop $grow
          (br_if $grow (i32.ne (memory.grow (i32.const 1)) (i32.const -1))))
        unreachable))
    "#;
    let run = run_handler(wat.as_bytes(), "{}", &tight_limits());
    assert!(
        matches!(run.outcome, HandlerOutcome::MemoryExceeded),
        "expected memory budget breach, got {:?}",
        run.outcome
    );
}

#[test]
fn calling_a_host_import_is_forbidden_and_named() {
    let wat = r#"
    (module
      (import "clean" "now" (func $now (result i64)))
      (memory (export "memory") 1)
      (func (export "alloc") (param i32) (result i32) (i32.const 1024))
      (func (export "expand") (param i32 i32) (result i32)
        (drop (call $now))
        (i32.const 0)))
    "#;
    let run = run_handler(wat.as_bytes(), "{}", &tight_limits());
    match run.outcome {
        HandlerOutcome::ForbiddenImport(name) => assert_eq!(name, "clean::now"),
        other => panic!("expected forbidden import, got {other:?}"),
    }
}

#[test]
fn declared_but_uncalled_import_is_not_an_offense() {
    // Chapter 21 §21.7: imports are stubbed; only calling one is BLOCK006.
    let wat = r#"
    (module
      (import "clean" "now" (func $now (result i64)))
      (memory (export "memory") 1)
      (func (export "alloc") (param i32) (result i32) (i32.const 1024))
      (func (export "expand") (param $ptr i32) (param $len i32) (result i32)
        (i32.store (i32.const 0) (local.get $ptr))
        (i32.store (i32.const 4) (local.get $len))
        (i32.const 0)))
    "#;
    let run = run_handler(wat.as_bytes(), "{}", &tight_limits());
    assert!(
        matches!(run.outcome, HandlerOutcome::Success(_)),
        "expected success, got {:?}",
        run.outcome
    );
}

#[test]
fn trap_is_a_crash() {
    let wat = r#"
    (module
      (memory (export "memory") 1)
      (func (export "alloc") (param i32) (result i32) (i32.const 1024))
      (func (export "expand") (param i32 i32) (result i32) unreachable))
    "#;
    let run = run_handler(wat.as_bytes(), "{}", &tight_limits());
    assert!(
        matches!(run.outcome, HandlerOutcome::Crash(_)),
        "expected crash, got {:?}",
        run.outcome
    );
}

#[test]
fn invalid_wasm_and_missing_exports_are_malformed() {
    let run = run_handler(b"not wasm at all", "{}", &tight_limits());
    assert!(
        matches!(run.outcome, HandlerOutcome::Malformed(_)),
        "expected malformed, got {:?}",
        run.outcome
    );

    let no_expand = r#"
    (module
      (memory (export "memory") 1)
      (func (export "alloc") (param i32) (result i32) (i32.const 1024)))
    "#;
    let run = run_handler(no_expand.as_bytes(), "{}", &tight_limits());
    match run.outcome {
        HandlerOutcome::Malformed(reason) => assert!(reason.contains("expand")),
        other => panic!("expected malformed, got {other:?}"),
    }
}

#[test]
fn same_input_same_envelope_twice() {
    // CMP-02 leg: the sandbox itself introduces no nondeterminism.
    let input = r#"{"name":"data","body":[{"kind":"line"}]}"#;
    let first = run_handler(ECHO.as_bytes(), input, &tight_limits());
    let second = run_handler(ECHO.as_bytes(), input, &tight_limits());
    match (first.outcome, second.outcome) {
        (HandlerOutcome::Success(a), HandlerOutcome::Success(b)) => assert_eq!(a, b),
        (a, b) => panic!("expected two successes, got {a:?} / {b:?}"),
    }
}
