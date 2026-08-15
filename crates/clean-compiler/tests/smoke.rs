//! The canonical entry point end to end: a minimal valid request compiles
//! to a Component Model component with its manifest and empty diagnostics.

use clean_compiler::compile;

mod common;

#[test]
fn minimal_request_compiles_to_a_component_with_manifest() {
    let request = common::minimal_valid_request();
    let artifact = compile(request).expect("minimal request compiles");
    assert_eq!(
        &artifact.wasm[..8],
        &[0x00, 0x61, 0x73, 0x6d, 0x0d, 0x00, 0x01, 0x00],
        "output is a component, not a core module"
    );
    assert!(artifact.diagnostics.is_empty());
    assert_eq!(artifact.manifest.spec_version, "1");
    assert_eq!(artifact.manifest.inputs.sources.len(), 1);
    assert!(!artifact.manifest.outputs.wasm_sha256.is_empty());
}
