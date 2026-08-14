//! Milestone 1 step 4 checks: the emitted bytes are a component — component
//! header, and valid under a component-model-aware validator. The same
//! assertion runs against `wasm-tools validate` in CI (`tests/acceptance.sh`
//! once step 8 lands).

use clean_compiler::codegen::component::{emit_empty_component, COMPONENT_HEADER};

#[test]
fn empty_component_has_component_header() {
    let bytes = emit_empty_component();
    assert!(bytes.len() >= 8, "component must at least carry its header");
    assert_eq!(
        &bytes[..8],
        &COMPONENT_HEADER,
        "expected `00 61 73 6d 0d 00 01 00` (asm magic, component version/layer)"
    );
}

#[test]
fn empty_component_validates() {
    let bytes = emit_empty_component();
    let mut validator = wasmparser::Validator::new();
    validator
        .validate_all(&bytes)
        .expect("empty component passes component-model validation");
}

#[test]
fn emission_is_deterministic() {
    assert_eq!(
        emit_empty_component(),
        emit_empty_component(),
        "same (empty) input, byte-identical output (CMP-02)"
    );
}
