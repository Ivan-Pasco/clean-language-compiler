//! The emitted-code leg of the M2 1:1 gate: every `codes::X` the pipeline
//! references must be an active, compiler-emittable registry row. Together
//! with `registry_spec.rs` (code ↔ rule in Platform 09/10) and
//! `diagnostics_fixtures.rs` (code ↔ DIA-06 snapshot triple), this closes
//! código ⇔ regla ⇔ snapshot: a pass emitting a withdrawn code, a code
//! owned by another component (Framework/Host/Toolchain), or an
//! unregistered string fails here.

use clean_compiler_types::codes;
use std::collections::BTreeSet;
use std::path::Path;

fn scan_rs_files(dir: &Path, found: &mut BTreeSet<String>) {
    for entry in std::fs::read_dir(dir).expect("source dir reads") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            scan_rs_files(&path, found);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let text = std::fs::read_to_string(&path).expect("source file reads");
            for (idx, _) in text.match_indices("codes::") {
                let tail = &text[idx + "codes::".len()..];
                let ident: String = tail
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect();
                // Only PREFIX### identifiers are code references; the module
                // also exports types (Severity, REGISTRY, lookup, …).
                if ident.len() >= 6
                    && ident.chars().rev().take(3).all(|c| c.is_ascii_digit())
                    && ident.chars().all(|c| c.is_ascii_alphanumeric())
                {
                    found.insert(ident);
                }
            }
        }
    }
}

#[test]
fn every_code_the_pipeline_references_is_compiler_emittable() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut referenced = BTreeSet::new();
    scan_rs_files(&manifest.join("src"), &mut referenced);
    scan_rs_files(&manifest.join("../clean-compiler-bin/src"), &mut referenced);

    assert!(
        !referenced.is_empty(),
        "the scan found no code references; the gate is miswired"
    );
    for code in &referenced {
        let info = codes::lookup(code)
            .unwrap_or_else(|| panic!("pipeline references {code}, which is not in the registry"));
        assert!(
            info.is_compiler_emittable(),
            "pipeline references {code}, which is {:?}/{:?} — not a code this compiler may emit",
            info.status,
            info.emitter
        );
    }
}
