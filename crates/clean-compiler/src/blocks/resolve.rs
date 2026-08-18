//! Library catalog intake and block-name resolution (chapter 21 §21.2,
//! LEX-05, Platform 10 §10.1/§10.7).
//!
//! The catalog is built from `request.library_manifests` unconditionally —
//! manifest defects (`LIB004`), reserved registrations (`BLOCK003`),
//! unresolved dependencies (`LIB001`) and unclaimed folder scopes
//! (`LIB017`) are library-load findings and fire whether or not the
//! program uses a single block. Block-name resolution then follows §21.2's
//! order: explicit library import first, implicit folder scope second;
//! ambiguity is `BLOCK001`, absence is `BLOCK002`.

use base64::Engine as _;
use clean_compiler_types::{codes, request::CompileRequest, Level, Span};
use indexmap::IndexMap;
use sha2::{Digest, Sha256};

use crate::diag::{build, DiagnosticSink};

/// One loaded library: its manifest index and, when the manifest carried a
/// hash-checked artifact, the decoded handler wasm.
pub struct LibEntry<'r> {
    pub name: &'r str,
    pub handles_blocks: &'r [String],
    pub wasm: Option<Vec<u8>>,
}

/// Libraries by name, in `library_manifests[]` order (deterministic
/// iteration, CMP-02).
pub struct Catalog<'r> {
    pub libraries: IndexMap<&'r str, LibEntry<'r>>,
}

impl<'r> Catalog<'r> {
    /// The libraries among `names` that register a handler for `block` —
    /// preserves the order of `names`, deduplicated.
    fn handlers_among<'c>(&'c self, names: &[String], block: &str) -> Vec<&'c LibEntry<'r>> {
        let mut found: Vec<&LibEntry> = Vec::new();
        for name in names {
            if let Some(entry) = self.libraries.get(name.as_str()) {
                if entry.handles_blocks.iter().any(|b| b == block)
                    && !found.iter().any(|e| e.name == entry.name)
                {
                    found.push(entry);
                }
            }
        }
        found
    }
}

/// A block name must be a qualified identifier: `name` or `name.name.name`
/// (BLK-01), each segment in LEX-03 form.
fn is_qualified_identifier(name: &str) -> bool {
    !name.is_empty()
        && name.split('.').all(|segment| {
            let mut chars = segment.chars();
            chars.next().is_some_and(|c| c.is_ascii_alphabetic())
                && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        })
}

fn is_hex_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
}

/// LEX-05: a registration may not claim a word from the four keyword
/// tables nor any name beginning with `core.` (or `core` itself).
pub(super) fn is_reserved_block_name(name: &str) -> bool {
    crate::lexer::is_reserved_word(name) || name == "core" || name.starts_with("core.")
}

fn lib004(sink: &mut DiagnosticSink, name: &str, reason: &str) {
    // Template from Platform 10 §LIB004; `{path}` carries the library's
    // name — the compiler validates the lowered manifest entry and holds
    // no library.toml path (CMP-01; boundary note added to Platform 10 by
    // the 2026-08-18 erratum, ratifying this wording).
    sink.push(build(
        Level::Error,
        codes::LIB004,
        format!("Library '{name}' has invalid manifest: {reason}"),
        Span::request_document(),
        None,
    ));
}

/// Builds the catalog and polices the request's library surface. Every
/// finding is collected; a defective manifest still enters the catalog by
/// name (with no artifact) so block-name resolution can keep reporting.
pub fn build_catalog<'r>(request: &'r CompileRequest, sink: &mut DiagnosticSink) -> Catalog<'r> {
    let mut libraries: IndexMap<&str, LibEntry<'_>> = IndexMap::new();

    for manifest in &request.library_manifests {
        if manifest.name.is_empty() {
            lib004(sink, &manifest.name, "name is empty");
            continue;
        }
        if libraries.contains_key(manifest.name.as_str()) {
            lib004(
                sink,
                &manifest.name,
                "duplicate manifest for this library name",
            );
            continue;
        }
        if !is_hex_sha256(&manifest.compiletime_wasm_sha256) {
            lib004(
                sink,
                &manifest.name,
                &format!(
                    "compiletime_wasm_sha256 '{}' is not a lowercase hex sha-256",
                    manifest.compiletime_wasm_sha256
                ),
            );
        }
        for block in &manifest.handles_blocks {
            if !is_qualified_identifier(block) {
                lib004(
                    sink,
                    &manifest.name,
                    &format!("handles_blocks entry '{block}' is not a qualified block name"),
                );
            } else if is_reserved_block_name(block) {
                // BLOCK003 (chapter 21 §21.6, LEX-05); local wording
                // pinned by the DIA-06 fixture (DISCOVERIES-M5 item 4).
                sink.push(build(
                    Level::Error,
                    codes::BLOCK003,
                    format!(
                        "library '{}' registers reserved block name `{block}`",
                        manifest.name
                    ),
                    Span::request_document(),
                    None,
                ));
            }
        }
        let wasm = match &manifest.compiletime_wasm {
            Some(encoded) => match base64::engine::general_purpose::STANDARD.decode(encoded) {
                Ok(bytes) => {
                    let actual = format!("{:x}", Sha256::digest(&bytes));
                    if actual == manifest.compiletime_wasm_sha256 {
                        Some(bytes)
                    } else {
                        lib004(
                            sink,
                            &manifest.name,
                            "compiletime_wasm does not match compiletime_wasm_sha256",
                        );
                        None
                    }
                }
                Err(_) => {
                    lib004(sink, &manifest.name, "compiletime_wasm is not valid base64");
                    None
                }
            },
            None => {
                if !manifest.handles_blocks.is_empty() {
                    lib004(
                        sink,
                        &manifest.name,
                        "registers handles_blocks but carries no compiletime_wasm",
                    );
                }
                None
            }
        };
        libraries.insert(
            manifest.name.as_str(),
            LibEntry {
                name: &manifest.name,
                handles_blocks: &manifest.handles_blocks,
                wasm,
            },
        );
    }

    // Platform 14 §14.1.1: `library_manifests` is the caller-resolved
    // dependency closure — a dependency with no manifest means the
    // caller's resolution failed (LIB001, template verbatim).
    for name in request.dependencies.keys() {
        if !libraries.contains_key(name.as_str()) {
            sink.push(build(
                Level::Error,
                codes::LIB001,
                format!("Library '{name}' not found in any configured source"),
                Span::request_document(),
                None,
            ));
        }
    }

    // Platform 10 §10.7 (LIB017, template verbatim): every folder-scoped
    // library must be a declared dependency.
    for (folder, libs) in &request.folders {
        for lib in libs {
            if !request.dependencies.contains_key(lib) {
                sink.push(build(
                    Level::Error,
                    codes::LIB017,
                    format!(
                        "Folder scope '{folder}' maps to library '{lib}' which is not a declared dependency"
                    ),
                    Span::request_document(),
                    None,
                ));
            }
        }
    }

    Catalog { libraries }
}

/// The libraries a `[folders]` mapping brings into scope for `path`.
/// LBS-04 (framework 09 §6, ratifying this compiler's adoption): a key
/// matches after stripping one optional trailing `/**` when it
/// path-prefix-matches on whole segments, case-sensitive POSIX. Keys with
/// any other glob form were refused at intake (RQD002). Order: folder-map
/// order, then the key's own list order; deduplicated.
pub fn folder_libraries(folders: &IndexMap<String, Vec<String>>, path: &str) -> Vec<String> {
    let mut scope: Vec<String> = Vec::new();
    for (key, libs) in folders {
        let prefix = key.strip_suffix("/**").unwrap_or(key);
        let matches = path
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.starts_with('/'));
        if matches {
            for lib in libs {
                if !scope.contains(lib) {
                    scope.push(lib.clone());
                }
            }
        }
    }
    scope
}

/// §21.2 resolution for one block occurrence. Emits `BLOCK001`/`BLOCK002`
/// (local wordings pinned by DIA-06 fixtures) and returns the winning
/// library, if any.
pub fn resolve_block<'c, 'r>(
    block_name: &str,
    file_path: &str,
    explicit_imports: &[String],
    catalog: &'c Catalog<'r>,
    folders: &IndexMap<String, Vec<String>>,
    span: Span,
    sink: &mut DiagnosticSink,
) -> Option<&'c LibEntry<'r>> {
    // Rule 1 — explicit library import wins.
    let explicit = catalog.handlers_among(explicit_imports, block_name);
    match explicit.as_slice() {
        [single] => return Some(single),
        [first, second, ..] => {
            ambiguous(block_name, first.name, second.name, span, sink);
            return None;
        }
        [] => {}
    }

    // Rules 2 and 3 — implicit folder scope.
    let scope = folder_libraries(folders, file_path);
    let implicit = catalog.handlers_among(&scope, block_name);
    match implicit.as_slice() {
        [single] => Some(single),
        [first, second, ..] => {
            ambiguous(block_name, first.name, second.name, span, sink);
            None
        }
        [] => {
            // Rule 4 — BLOCK002.
            sink.push(build(
                Level::Error,
                codes::BLOCK002,
                format!("no library in scope registers a handler for block `{block_name}`"),
                span,
                Some("unknown block name".to_string()),
            ));
            None
        }
    }
}

fn ambiguous(block_name: &str, a: &str, b: &str, span: Span, sink: &mut DiagnosticSink) {
    let mut diagnostic = build(
        Level::Error,
        codes::BLOCK001,
        format!(
            "block name `{block_name}` is ambiguous: libraries '{a}' and '{b}' both register a handler for it"
        ),
        span,
        Some("ambiguous block name".to_string()),
    );
    diagnostic.helps.push(
        "narrow the `[folders]` mapping in clean.toml or add an explicit `import` to disambiguate"
            .to_string(),
    );
    sink.push(diagnostic);
}
