//! Component assembly (Milestone 1 steps 4 and 8).
//!
//! The compiler emits a Component Model component, never a bare core module
//! (Platform 15 §6.1): pass [10] produces a core module and this module
//! wraps it, attaching the target world's WIT so the host can run its
//! Moment 3 check. Step 4 starts with the smallest observable slice: a
//! component that imports and exports nothing, but already carries the
//! component header (`00 61 73 6d 0d 00 01 00`) and validates.

use wasm_encoder::Component;

/// The component-model magic + version/layer header every emitted artifact
/// must start with (brief acceptance check 1).
pub const COMPONENT_HEADER: [u8; 8] = [0x00, 0x61, 0x73, 0x6d, 0x0d, 0x00, 0x01, 0x00];

/// Emits a component with no imports, no exports, and no core module —
/// step 4's "can we produce a component at all", kept as the degenerate
/// base case of assembly.
pub fn emit_empty_component() -> Vec<u8> {
    Component::new().finish()
}
