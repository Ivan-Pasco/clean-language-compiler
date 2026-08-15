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
    let _typed = typed;

    // Passes [6..10] land step by step through Milestone 1.
    Err(CompileError::Incomplete {
        completed: "typecheck",
    })
}
