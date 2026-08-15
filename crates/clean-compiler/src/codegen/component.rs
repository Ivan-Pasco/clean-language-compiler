//! Component assembly (Milestone 1 steps 4 and 8).
//!
//! The compiler emits a Component Model component, never a bare core module
//! (Platform 15 §6.1). Step 8: synthesize the guest's mirror world — it
//! imports exactly the `clean:host/*` interfaces the program calls and
//! exports the `init`/`handle` entry points the host world imports — embed
//! that world's WIT into the core module, and wrap it as a component so the
//! host's Moment 3 check has something to read (HCV-01).

use wasm_encoder::Component;
use wit_component::{ComponentEncoder, StringEncoding};
use wit_parser::Resolve;

use crate::mir::MirProgram;

/// The component-model magic + version/layer header every emitted artifact
/// must start with (brief acceptance check 1).
pub const COMPONENT_HEADER: [u8; 8] = [0x00, 0x61, 0x73, 0x6d, 0x0d, 0x00, 0x01, 0x00];

/// Emits a component with no imports, no exports, and no core module —
/// step 4's "can we produce a component at all", kept as the degenerate
/// base case of assembly.
pub fn emit_empty_component() -> Vec<u8> {
    Component::new().finish()
}

/// The guest's mirror world, in WIT text: one import per interface the
/// program actually calls (in first-use order), plus the two entry points
/// the host world imports (ADR-0002 §1). Mirrors the shape of
/// clean-server's own acceptance fixture (`clean:guest/app`).
pub fn synthesize_guest_world(program: &MirProgram, package_version: &str) -> String {
    let mut interfaces: Vec<&str> = Vec::new();
    for import in &program.imports {
        if !interfaces.contains(&import.interface.as_str()) {
            interfaces.push(&import.interface);
        }
    }
    let mut wit = String::from("package clean:guest@0.1.0;\n\nworld app {\n");
    for interface in interfaces {
        wit.push_str(&format!(
            "    import clean:host/{interface}@{package_version};\n"
        ));
    }
    // Export exactly the entry points the program defines (ADR-0002 §1);
    // whether the set satisfies the host is the host's Moment 3 decision.
    for function in &program.functions {
        if !function.export {
            continue;
        }
        match function.name.as_str() {
            "init" => wit.push_str("    export init: func();\n"),
            "handle" => wit.push_str("    export handle: func(handler-id: u32);\n"),
            _ => {}
        }
    }
    wit.push_str("}\n");
    wit
}

/// Wraps the emitted core module as a component: parses the host contract
/// plus the synthesized guest world, embeds the world's metadata into the
/// module, and componentizes. Failures here are compiler bugs (COM013
/// territory), surfaced as strings for the driver to wrap.
pub fn assemble(
    core: Vec<u8>,
    program: &MirProgram,
    host_wit: &str,
    package_version: &str,
) -> Result<Vec<u8>, String> {
    let mut resolve = Resolve::new();
    resolve
        .push_str("host.wit", host_wit)
        .map_err(|e| format!("host wit failed to re-parse during assembly: {e}"))?;
    let guest_wit = synthesize_guest_world(program, package_version);
    let guest_package = resolve
        .push_str("guest.wit", &guest_wit)
        .map_err(|e| format!("synthesized guest world does not parse: {e}\n{guest_wit}"))?;
    let world = resolve
        .select_world(&[guest_package], Some("app"))
        .map_err(|e| format!("synthesized guest world has no `app` world: {e}"))?;

    let mut module = core;
    wit_component::embed_component_metadata(&mut module, &resolve, world, StringEncoding::UTF8)
        .map_err(|e| format!("embedding component metadata failed: {e:#}"))?;

    ComponentEncoder::default()
        .validate(true)
        .module(&module)
        .map_err(|e| format!("core module rejected by componentizer: {e:#}"))?
        .encode()
        .map_err(|e| format!("component encoding failed: {e:#}"))
}
