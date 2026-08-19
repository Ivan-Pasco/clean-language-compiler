//! The pipeline driver: sequences the passes of Platform 14 §14.4, collects
//! diagnostics, and enforces the all-or-nothing output contract (CMP-05).

use clean_compiler_types::{BuildManifest, CompileRequest, Diagnostic, SourceMap};
use thiserror::Error;

use crate::diag::DiagnosticSink;

/// The artifact set of a successful compilation (Platform 14 §14.2.1).
#[derive(Debug)]
pub struct CompileArtifact {
    pub wasm: Vec<u8>,
    pub manifest: BuildManifest,
    /// Warnings and infos, in emission order — an error would have made the
    /// whole compilation fail instead.
    pub diagnostics: Vec<Diagnostic>,
    pub source_map: Option<SourceMap>,
}

/// Failure modes of `compile` (§14.2.1: the error carries the failing
/// diagnostics, never a stringly-typed message).
#[derive(Debug, Error)]
pub enum CompileError {
    /// The request or program was rejected; the diagnostics say why and no
    /// artifact was produced (CMP-05).
    #[error("compilation rejected with {} diagnostic(s)", .0.len())]
    Rejected(Vec<Diagnostic>),
    /// Pre-v1 only: the program uses constructs this build recognises but
    /// cannot compile yet. Not a user error, not a registered diagnostic —
    /// each entry names a milestone gap.
    #[error("program uses {} construct(s) outside the current milestone surface", .0.len())]
    Unsupported(Vec<crate::diag::Unsupported>),
    /// Pre-v1 only: the pipeline prefix implemented so far ran clean, but the
    /// pass after `completed` does not exist yet. This variant shrinks with
    /// every milestone step and is deleted when pass [10] lands.
    #[error("pipeline is implemented through `{completed}`; later passes do not exist yet")]
    Incomplete { completed: &'static str },
}

/// What passes [1]–[9] hand to pass [10]: the validated request (world,
/// sources, config) and the MIR the code generator lowers.
struct FrontArtifacts {
    validated: crate::request::ValidatedRequest,
    mir: crate::mir::MirProgram,
}

/// Runs passes [1] through [9] — everything except codegen — appending to
/// `sink`. `Ok((cache, None))` means a pass raised errors and the pipeline
/// stopped after it completed (the sink holds the diagnostics); `Ok((cache,
/// Some(front)))` means the program is ready for pass [10]. Shared verbatim
/// by [`compile`] and [`check`], so the fast path can never diverge from
/// the build (Platform 14 §14.14.4).
#[allow(clippy::type_complexity)]
fn front(
    request: CompileRequest,
    sink: &mut DiagnosticSink,
) -> Result<(crate::diag::SourceCache, Option<FrontArtifacts>), CompileError> {
    // Pass [1] — Request Validation (→ ValidatedRequest). Request-level
    // diagnostics quote no source (Platform 10 §16).
    let validated = crate::request::validate(request, sink);
    if sink.has_errors() {
        return Ok((crate::diag::SourceCache::empty(), None));
    }
    let validated = validated.expect("pass [1] returned no value yet raised no error");
    // Every later drain renders against the request's sources (§4.2).
    let cache = crate::diag::SourceCache::from_sources(&validated.request.sources);

    // §3.6: memory64 guests are shape-compatible but the 32-bit pointer
    // codegen does not speak them yet.
    if validated.request.build.memory64 {
        sink.note_unsupported(
            "memory64 guest memories",
            clean_compiler_types::Span::request_document(),
        );
    }

    // Passes [2]+[3] — Lex and Parse, per file, in `sources[]` order
    // (deterministic reduction, §14.5). Both are error-recovering; every
    // file reports before the pipeline decides to stop.
    let mut files = Vec::new();
    for source in &validated.request.sources {
        let stream = crate::lexer::lex(&source.path, &source.content, sink);
        let ast = crate::parser::parse(&stream, sink);
        files.push(crate::resolver::ParsedFile { ast, stream });
    }
    if sink.has_errors() {
        return Ok((cache, None));
    }

    // Pass [4] — Resolve (single compilation unit in M1).
    let library_names: Vec<String> = validated
        .request
        .library_manifests
        .iter()
        .map(|m| m.name.clone())
        .collect();
    let resolved = crate::resolver::resolve(files, &library_names, sink);
    if sink.has_errors() {
        return Ok((cache, None));
    }

    // Pass [5] — Type Check, against the world-typed boundary (ADR-0002).
    // Provisional when the program declares library blocks (Platform 14
    // §14.4.2, normative since the 2026-08-18 erratum): user code may
    // reference symbols a handler emits, so pre-expansion findings are
    // held back and pass [6]'s re-validation of the expanded program is
    // authoritative.
    let blocks_declared = !resolved.decls.blocks.is_empty();
    let mut provisional = DiagnosticSink::new();
    let pass5_sink: &mut DiagnosticSink = if blocks_declared {
        &mut provisional
    } else {
        &mut *sink
    };
    let typed = crate::typecheck::check(&resolved, &validated.world, pass5_sink);
    if sink.has_errors() {
        return Ok((cache, None));
    }

    // Pass [6] — Block Handler Expansion (chapter 21, ADR-0004): library
    // catalog intake, block-name resolution, handler execution in the
    // sandbox, IR splice, and re-validation of the expanded program.
    let (resolved, typed) = crate::blocks::expand(resolved, typed, &validated, sink);
    if sink.has_errors() {
        return Ok((cache, None));
    }
    if !sink.unsupported().is_empty() {
        return Err(CompileError::Unsupported(sink.unsupported().to_vec()));
    }

    // Pass [7] — HIR Lowering.
    let hir = crate::hir::lower(typed);

    // Pass [8] — MIR Lowering (no optimizations run yet). The memory tier
    // is fixed here (TIER-01): the request's explicit tier — validated in
    // pass [1] — or the target-derived inference.
    let tier = resolve_tier(&validated.request);
    let mir = crate::mir::lower(
        &hir,
        &resolved,
        &validated.world,
        &validated.world.package_version(),
        tier,
        sink,
    );
    if !sink.unsupported().is_empty() {
        return Err(CompileError::Unsupported(sink.unsupported().to_vec()));
    }

    // Pass [9] — World Import Check (CMP-03): abort before codegen on any
    // call site the delivered world does not provide.
    crate::codegen::world_check::check(
        &hir,
        &validated.world,
        &validated.request.target_world.world,
        &resolved,
        sink,
    );
    if sink.has_errors() {
        return Ok((cache, None));
    }

    Ok((cache, Some(FrontArtifacts { validated, mir })))
}

/// The canonical entry point (Platform 14 §14.2.1). Every other surface —
/// process adapter, JSON-RPC/MCP — wraps this function.
pub fn compile(request: CompileRequest) -> Result<CompileArtifact, CompileError> {
    let mut sink = DiagnosticSink::new();
    let (cache, artifacts) = front(request, &mut sink)?;
    let Some(FrontArtifacts { validated, mir }) = artifacts else {
        return Err(CompileError::Rejected(crate::diag::finalize(
            sink.into_diagnostics(),
            &cache,
        )));
    };

    // Pass [10] — Codegen + Assembly: core module, then the component wrap
    // with the synthesized guest world embedded (Platform 15 §6.1).
    let package_version = validated.world.package_version();
    let wasm = match crate::codegen::core::emit_core(&mir).and_then(|core| {
        crate::codegen::component::assemble(
            core,
            &mir,
            &validated.request.target_world.wit,
            &package_version,
        )
    }) {
        Ok(wasm) => wasm,
        Err(invariant) => {
            // CMP-04: a broken internal invariant is COM013 — a compiler
            // bug, never a user error. Template from Platform 10 §11.
            let diagnostic = crate::diag::build(
                clean_compiler_types::Level::Error,
                clean_compiler_types::codes::COM013,
                format!(
                    "internal compiler invariant violated: {invariant} — this is a compiler bug, please report it"
                ),
                clean_compiler_types::Span::request_document(),
                None,
            );
            return Err(CompileError::Rejected(crate::diag::finalize(
                vec![diagnostic],
                &cache,
            )));
        }
    };

    let mut manifest = build_manifest(&validated.request, &wasm);
    // Remaining diagnostics are warnings/infos (errors returned earlier);
    // the manifest records them too (§14.8).
    let diagnostics = crate::diag::finalize(sink.into_diagnostics(), &cache);
    manifest.diagnostics = diagnostics.clone();
    Ok(CompileArtifact {
        wasm,
        manifest,
        diagnostics,
        source_map: None,
    })
}

/// The diagnostics-only build (Platform 14 §14.14.4, behind `cln check`):
/// passes [1] through [9] — including the World Import Check, so a check
/// request carries `target_world` exactly as a build request does — and no
/// codegen. Returns every finalized diagnostic the run produced, errors
/// included; the caller fails the operation iff any is `level = error`.
/// `Err` is only the pre-v1 `Unsupported` channel.
pub fn check(request: CompileRequest) -> Result<Vec<Diagnostic>, CompileError> {
    let mut sink = DiagnosticSink::new();
    let (cache, _artifacts) = front(request, &mut sink)?;
    Ok(crate::diag::finalize(sink.into_diagnostics(), &cache))
}

/// `--emit=hir-json` (M4, the AI-review precondition): passes [1]–[7]
/// only, serializing the HIR. Runs past the pass-[8] unsupported frontier
/// on purpose — the HIR exists for the full typed surface even where core
/// lowering has not landed. `Ok` carries the JSON and every diagnostic
/// (the caller fails iff any is `level = error`, mirroring `check`).
pub fn emit_hir(request: CompileRequest) -> Result<(String, Vec<Diagnostic>), CompileError> {
    let mut sink = DiagnosticSink::new();
    let validated = crate::request::validate(request, &mut sink);
    let (json, cache) = 'front: {
        let Some(validated) = validated else {
            break 'front (None, crate::diag::SourceCache::empty());
        };
        let cache = crate::diag::SourceCache::from_sources(&validated.request.sources);
        if sink.has_errors() {
            break 'front (None, cache);
        }
        let mut files = Vec::new();
        for source in &validated.request.sources {
            let stream = crate::lexer::lex(&source.path, &source.content, &mut sink);
            let ast = crate::parser::parse(&stream, &mut sink);
            files.push(crate::resolver::ParsedFile { ast, stream });
        }
        if sink.has_errors() {
            break 'front (None, cache);
        }
        let library_names: Vec<String> = validated
            .request
            .library_manifests
            .iter()
            .map(|m| m.name.clone())
            .collect();
        let resolved = crate::resolver::resolve(files, &library_names, &mut sink);
        if sink.has_errors() {
            break 'front (None, cache);
        }
        // Same pass-[5]/[6] discipline as `front`: pre-expansion type
        // findings are provisional when blocks are declared.
        let blocks_declared = !resolved.decls.blocks.is_empty();
        let mut provisional = DiagnosticSink::new();
        let pass5_sink: &mut DiagnosticSink = if blocks_declared {
            &mut provisional
        } else {
            &mut sink
        };
        let typed = crate::typecheck::check(&resolved, &validated.world, pass5_sink);
        if sink.has_errors() {
            break 'front (None, cache);
        }
        let (_resolved, typed) = crate::blocks::expand(resolved, typed, &validated, &mut sink);
        if sink.has_errors() {
            break 'front (None, cache);
        }
        let hir = crate::hir::lower(typed);
        let json = serde_json::to_string_pretty(&hir).expect("HIR serializes");
        (Some(json), cache)
    };
    let diagnostics = crate::diag::finalize(sink.into_diagnostics(), &cache);
    Ok((json.unwrap_or_default(), diagnostics))
}

/// The reproducibility record (Platform 14 §14.8). Two M1 placeholders,
/// both recorded in the milestone Discoveries: `compiler.sha256` derives
/// from the crate version until the release pipeline stamps the binary
/// hash, and `timings` are zero because CMP-02 requires byte-identical
/// manifests — wall-clock timings and that rule are in tension in the spec
/// as written.
/// TIER-01 resolution: the request's explicit tier (validated in pass [1],
/// so the lookup cannot miss) or the target-derived inference.
fn resolve_tier(request: &CompileRequest) -> crate::layout::Tier {
    match &request.build.memory.tier {
        Some(name) => crate::layout::tier(name)
            .unwrap_or_else(|| crate::layout::infer_tier(&request.build.target)),
        None => crate::layout::infer_tier(&request.build.target),
    }
}

fn build_manifest(request: &CompileRequest, wasm: &[u8]) -> BuildManifest {
    use sha2::{Digest, Sha256};
    let sha_hex = |bytes: &[u8]| {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        format!("{:x}", hasher.finalize())
    };

    let request_json = serde_json::to_string(request).expect("request serializes");
    // The resolved config records resolved values, not request echoes: an
    // absent tier appears as its TIER-01 resolution.
    let mut resolved_build = request.build.clone();
    resolved_build.memory.tier = Some(resolve_tier(request).name.to_string());
    let resolved_config = serde_json::json!({
        "build": resolved_build,
        "folders": request.folders,
        "dependencies": request.dependencies,
        "compile_limits": request.compile_limits,
        "telemetry": request.telemetry,
    });

    let mut timings = indexmap::IndexMap::new();
    for pass in [
        "lex_ms",
        "parse_ms",
        "resolve_ms",
        "typecheck_ms",
        "codegen_ms",
    ] {
        timings.insert(pass.to_string(), 0u64);
    }

    BuildManifest {
        spec_version: "1".to_string(),
        compiler: clean_compiler_types::manifest::CompilerId {
            version: env!("CARGO_PKG_VERSION").to_string(),
            sha256: sha_hex(env!("CARGO_PKG_VERSION").as_bytes()),
        },
        request_sha256: sha_hex(request_json.as_bytes()),
        inputs: clean_compiler_types::manifest::Inputs {
            sources: request
                .sources
                .iter()
                .map(|s| clean_compiler_types::manifest::SourceHash {
                    path: s.path.clone(),
                    sha256: s.sha256.clone(),
                })
                .collect(),
            library_manifests: request
                .library_manifests
                .iter()
                .map(|l| clean_compiler_types::manifest::LibraryHash {
                    name: l.name.clone(),
                    version: l.version.clone(),
                    wit_sha256: sha_hex(l.wit.as_bytes()),
                    compiletime_wasm_sha256: l.compiletime_wasm_sha256.clone(),
                })
                .collect(),
        },
        resolved_config,
        overrides: request.overrides.clone(),
        outputs: clean_compiler_types::manifest::Outputs {
            wasm_sha256: sha_hex(wasm),
            source_map_sha256: None,
        },
        diagnostics: Vec::new(),
        timings,
    }
}
