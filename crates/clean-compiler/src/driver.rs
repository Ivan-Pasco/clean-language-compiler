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

/// The canonical entry point (Platform 14 §14.2.1). Every other surface —
/// process adapter, JSON-RPC/MCP — wraps this function.
pub fn compile(request: CompileRequest) -> Result<CompileArtifact, CompileError> {
    let mut sink = DiagnosticSink::new();

    // Pass [1] — Request Validation (→ ValidatedRequest).
    let validated = crate::request::validate(request, &mut sink);
    if sink.has_errors() {
        return Err(CompileError::Rejected(sink.into_diagnostics()));
    }
    let validated = validated.expect("pass [1] returned no value yet raised no error");

    // Passes [2]+[3] — Lex and Parse, per file, in `sources[]` order
    // (deterministic reduction, §14.5). Both are error-recovering; every
    // file reports before the pipeline decides to stop.
    let mut files = Vec::new();
    for source in &validated.request.sources {
        let stream = crate::lexer::lex(&source.path, &source.content, &mut sink);
        let ast = crate::parser::parse(&stream, &mut sink);
        files.push(crate::resolver::ParsedFile { ast, stream });
    }
    if sink.has_errors() {
        return Err(CompileError::Rejected(sink.into_diagnostics()));
    }

    // Pass [4] — Resolve (single compilation unit in M1).
    let resolved = crate::resolver::resolve(files, &mut sink);
    if sink.has_errors() {
        return Err(CompileError::Rejected(sink.into_diagnostics()));
    }

    // Pass [5] — Type Check, against the world-typed boundary (ADR-0002).
    let typed = crate::typecheck::check(&resolved, &validated.world, &mut sink);
    if sink.has_errors() {
        return Err(CompileError::Rejected(sink.into_diagnostics()));
    }
    if !sink.unsupported().is_empty() {
        return Err(CompileError::Unsupported(sink.unsupported().to_vec()));
    }

    // Pass [6] — Block Handler Expansion: a typed pass-through until M5;
    // M1 programs declare no library blocks.

    // Pass [7] — HIR Lowering.
    let hir = crate::hir::lower(typed);

    // Pass [8] — MIR Lowering (no optimization in M1).
    let mir = crate::mir::lower(
        &hir,
        &resolved,
        &validated.world.package_version(),
        &mut sink,
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
        &mut sink,
    );
    if sink.has_errors() {
        return Err(CompileError::Rejected(sink.into_diagnostics()));
    }

    // Pass [10] — the core half exists; component assembly is step 8. The
    // artifact set is withheld until the emitted bytes are a component
    // (CCMP-19: no other target ships).
    let _core = crate::codegen::core::emit_core(&mir);
    Err(CompileError::Incomplete {
        completed: "core-emission",
    })
}
