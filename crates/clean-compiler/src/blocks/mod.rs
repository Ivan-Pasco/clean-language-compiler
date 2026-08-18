//! Pass [6] — Block Handler Expansion (Platform 14 §14.4.2, ADR-0004).
//! Executes library `compiletime` handlers in a sandboxed wasmtime
//! sub-instance (epoch interruption, memory cap, no host imports) and
//! splices the returned IR into the program.
//!
//! Shape of the pass: library catalog intake (load-time findings fire even
//! with zero blocks), §21.2 name resolution per block site, handler
//! execution over the ADR-0003 wire ABI, budget enforcement (`BLOCK005`
//! per invocation, `LIB014` per library), envelope intake (`LIB010`
//! diagnostics, IR lowering), then splice-and-revalidate: the expanded
//! program re-enters resolve and type-check, which is how the returned IR
//! is "validated against the surrounding compilation unit" (§21.4). A
//! re-validation error anchored inside an expanded block is the handler's
//! defect (`BLOCK004`); one anchored in user code keeps its own code and
//! span.

pub mod abi;
pub mod ir;
pub mod resolve;
pub mod sandbox;

use clean_compiler_types::{codes, Diagnostic, Level, Span};
use indexmap::IndexMap;

use crate::diag::{build, DiagnosticSink};
use crate::parser::ast;
use crate::request::ValidatedRequest;
use crate::resolver::{ParsedFile, ResolvedAst};
use crate::source::ByteSpan;
use crate::typecheck::tir::TypedProgram;

/// Chapter 21 §21.7: compile-time heap across all of one library's handler
/// invocations. Not configurable through the request — `compile_limits`
/// carries no per-library keys (DISCOVERIES-M5 item 5).
const LIBRARY_HEAP_LIMIT: u64 = 512 * 1024 * 1024;

/// Pass [6] entry point: takes pass [4]/[5]'s outputs and returns them
/// unchanged when the program declares no library blocks, or the expanded
/// and re-validated pair when it does. On any error the inputs are
/// returned as-is — the driver rejects on the sink's errors (CMP-05).
pub fn expand(
    mut resolved: ResolvedAst,
    typed: TypedProgram,
    validated: &ValidatedRequest,
    sink: &mut DiagnosticSink,
) -> (ResolvedAst, TypedProgram) {
    let request = &validated.request;
    let catalog = resolve::build_catalog(request, sink);
    check_source_registrations(&resolved, request, sink);
    if resolved.decls.blocks.is_empty() {
        return (resolved, typed);
    }

    let limits = &request.compile_limits;
    let mut heap: IndexMap<String, u64> = IndexMap::new();
    let mut heap_reported: Vec<String> = Vec::new();
    // (file, item) → the items replacing that `LibraryBlock`, plus the
    // block's provenance for re-validation error attribution.
    let mut replacements: Vec<(usize, usize, Vec<ast::Item>)> = Vec::new();
    let mut expanded_sites: Vec<ExpandedSite> = Vec::new();

    for &(file_index, item_index) in &resolved.decls.blocks {
        let file = &resolved.files[file_index];
        let ast::Item::LibraryBlock(block) = &file.ast.items[item_index] else {
            continue;
        };
        // `block.span` anchors the header (schema); the range handler spans
        // may point into is the whole block, header and body.
        let extent = block_extent(block);
        let span = file.stream.diag_span(block.span);
        let Some(entry) = resolve::resolve_block(
            &block.name,
            &file.stream.path,
            &resolved.decls.modules[file_index].library_imports,
            &catalog,
            &request.folders,
            span.clone(),
            sink,
        ) else {
            continue;
        };
        let lib = entry.name.to_string();
        let Some(wasm) = &entry.wasm else {
            // The manifest defect was already LIB004 at load.
            continue;
        };

        let input = abi::block_json(&file.stream, block).to_string();
        let run = sandbox::run_handler(wasm, &input, limits);

        let total = heap.entry(lib.clone()).or_insert(0);
        *total += run.memory_bytes;
        if *total > LIBRARY_HEAP_LIMIT && !heap_reported.contains(&lib) {
            heap_reported.push(lib.clone());
            let total = *total;
            // LIB014 template verbatim (Platform 10 §10.5); heap measured
            // as the sum of end-of-call linear-memory sizes
            // (DISCOVERIES-M5 item 5).
            sink.push(build(
                Level::Error,
                codes::LIB014,
                format!(
                    "Library '{lib}' exceeded compile-time heap: {total} > {LIBRARY_HEAP_LIMIT}"
                ),
                Span::request_document(),
                None,
            ));
        }

        match run.outcome {
            sandbox::HandlerOutcome::Success(text) => {
                let envelope = match ir::parse_envelope(&text) {
                    Ok(envelope) => envelope,
                    Err(reason) => {
                        block004(
                            sink,
                            &lib,
                            &block.name,
                            span,
                            &format!("returned malformed IR: envelope does not parse: {reason}"),
                        );
                        continue;
                    }
                };
                // BLK-03: the handler's collected error/warning/info calls.
                // An `error` halts this file's expansion after the handler
                // returns; the loop still visits every other block so the
                // whole project reports in one run.
                let mut halted = false;
                for diag in &envelope.diagnostics {
                    let level = match diag.severity.as_str() {
                        "error" => Level::Error,
                        "warning" => Level::Warning,
                        "info" => Level::Info,
                        other => {
                            block004(
                                sink,
                                &lib,
                                &block.name,
                                span.clone(),
                                &format!("returned a diagnostic with unknown severity '{other}'"),
                            );
                            halted = true;
                            break;
                        }
                    };
                    let (start, end) = diag.span.byte_range;
                    if diag.span.file != file.stream.path
                        || start > end
                        || start < extent.start
                        || end > extent.end
                    {
                        // BLK-03: a synthetic span is the library's error.
                        block004(
                            sink,
                            &lib,
                            &block.name,
                            span.clone(),
                            &format!("emitted diagnostic '{}' with a synthetic span", diag.code),
                        );
                        halted = true;
                        break;
                    }
                    let diag_span = file.stream.diag_span(ByteSpan::new(start, end));
                    // LIB010 wrapper: the registry's `{library}::{function}`
                    // cannot be filled — manifests carry no handler function
                    // names — so attribution is `{library}::{block}` and the
                    // kebab-case sub-label rides as the primary label
                    // (DISCOVERIES-M5 item 11).
                    sink.push(build(
                        level,
                        codes::LIB010,
                        format!("[via {lib}::{}] {}", block.name, diag.message),
                        diag_span,
                        Some(diag.code.clone()),
                    ));
                    if level == Level::Error {
                        halted = true;
                    }
                }
                if halted {
                    continue;
                }
                let mut lowerer = ir::Lowerer::new(extent, &file.stream.path);
                match lowerer.items(&envelope.ir) {
                    Ok(items) => {
                        expanded_sites.push(ExpandedSite {
                            library: lib.clone(),
                            block: block.name.clone(),
                            span: file.stream.diag_span(extent),
                        });
                        replacements.push((file_index, item_index, items));
                    }
                    Err(ir::LowerError::NodeLimit) => {
                        // LIB014 template verbatim; the count stops at the
                        // first node past the cap.
                        sink.push(build(
                            Level::Error,
                            codes::LIB014,
                            format!(
                                "Library '{lib}' exceeded generated-IR node count: {} > {}",
                                ir::MAX_IR_NODES + 1,
                                ir::MAX_IR_NODES
                            ),
                            span,
                            None,
                        ));
                    }
                    Err(ir::LowerError::Malformed(reason)) => {
                        block004(
                            sink,
                            &lib,
                            &block.name,
                            span,
                            &format!("returned malformed IR: {reason}"),
                        );
                    }
                }
            }
            sandbox::HandlerOutcome::Timeout => {
                block005(
                    sink,
                    &lib,
                    &block.name,
                    span,
                    &format!("wall-clock limit {} ms", limits.handler_timeout_ms),
                );
            }
            sandbox::HandlerOutcome::MemoryExceeded => {
                block005(
                    sink,
                    &lib,
                    &block.name,
                    span,
                    &format!("memory limit {} MiB", limits.handler_memory_mb),
                );
            }
            sandbox::HandlerOutcome::ForbiddenImport(import) => {
                let mut diagnostic = build(
                    Level::Error,
                    codes::BLOCK006,
                    format!(
                        "library '{lib}' attempted a forbidden side effect expanding block `{}`: called host import '{import}'",
                        block.name
                    ),
                    span,
                    Some("forbidden side effect".to_string()),
                );
                diagnostic.notes.push(
                    "compile-time code cannot reach the filesystem, network, clock, or randomness (chapter 21 §21.7)"
                        .to_string(),
                );
                sink.push(diagnostic);
            }
            sandbox::HandlerOutcome::Crash(reason) => {
                block004(sink, &lib, &block.name, span, &format!("crashed: {reason}"));
            }
            sandbox::HandlerOutcome::Malformed(reason) => {
                block004(
                    sink,
                    &lib,
                    &block.name,
                    span,
                    &format!("is malformed: {reason}"),
                );
            }
        }
    }

    if sink.has_errors() {
        return (resolved, typed);
    }

    // Splice, per file in descending item order so earlier indices stay
    // valid (`decls.blocks` is ascending, so the reverse walk is exactly
    // that order).
    for (file_index, item_index, items) in replacements.into_iter().rev() {
        resolved.files[file_index]
            .ast
            .items
            .splice(item_index..=item_index, items);
    }

    // Re-validate: the expanded program re-enters resolve and type-check
    // (§21.4). Diagnostics that also fired on the first run dedup away in
    // `finalize`; unsupported notes dedup in the sink.
    let library_names: Vec<String> = request
        .library_manifests
        .iter()
        .map(|m| m.name.clone())
        .collect();
    let files: Vec<ParsedFile> = std::mem::take(&mut resolved.files);
    let mut second = DiagnosticSink::new();
    let resolved = crate::resolver::resolve(files, &library_names, &mut second);
    let typed_expanded = crate::typecheck::check(&resolved, &validated.world, &mut second);
    let had_errors = second.has_errors();
    for note in second.unsupported() {
        sink.note_unsupported(note.construct, note.span.clone());
    }
    for diagnostic in second.into_diagnostics() {
        sink.push(attribute_to_handler(diagnostic, &expanded_sites));
    }
    if had_errors {
        // The driver rejects on the sink; `typed` is the pre-expansion
        // program and is never emitted (CMP-05).
        return (resolved, typed);
    }
    (resolved, typed_expanded)
}

/// BLK-01 over in-source registrations: a `handles block` declaration must
/// name a non-reserved qualified block name (LEX-05 → `BLOCK003`, at the
/// declaration; the library here is the unit being compiled, named by
/// `project.name`) and reference a `compiletime function` defined in the
/// same library — the compilation unit (missing → `SEM019`, whose
/// registered template states exactly this failure). The arity/return
/// constraints of BLK-01 have no registered codes (DISCOVERIES-M5
/// item 15).
fn check_source_registrations(
    resolved: &ResolvedAst,
    request: &clean_compiler_types::CompileRequest,
    sink: &mut DiagnosticSink,
) {
    let handler_names: Vec<&str> = resolved
        .decls
        .compiletime_functions
        .iter()
        .filter_map(|&(f, i)| match &resolved.files[f].ast.items[i] {
            ast::Item::CompiletimeFunction(cf) => Some(cf.name.as_str()),
            _ => None,
        })
        .collect();
    for &(file_index, item_index) in &resolved.decls.handles {
        let file = &resolved.files[file_index];
        let ast::Item::HandlesBlock(handles) = &file.ast.items[item_index] else {
            continue;
        };
        let span = file.stream.diag_span(handles.span);
        if resolve::is_reserved_block_name(&handles.block_name) {
            sink.push(build(
                Level::Error,
                codes::BLOCK003,
                format!(
                    "library '{}' registers reserved block name `{}`",
                    request.project.name, handles.block_name
                ),
                span,
                Some("reserved block name".to_string()),
            ));
            continue;
        }
        if !handler_names.contains(&handles.handler.as_str()) {
            // SEM019 — exact wording from Platform 10 §3.
            sink.push(build(
                Level::Error,
                codes::SEM019,
                format!("I cannot find a function named `{}`", handles.handler),
                span,
                Some("no function with this name is in scope".to_string()),
            ));
        }
    }
}

/// Where one block was expanded, for re-validation error attribution.
/// `span` covers the block's full extent.
struct ExpandedSite {
    library: String,
    block: String,
    span: Span,
}

/// The block's full byte range: header plus every argument, attribute, and
/// body node.
fn block_extent(block: &ast::BlockAst) -> ByteSpan {
    let mut extent = block.span;
    for arg in &block.arguments {
        extent = extent.merge(match arg {
            ast::BlockArg::Positional(expr) => expr.span(),
            ast::BlockArg::Keyword { span, .. } => *span,
        });
    }
    for attribute in &block.attributes {
        extent = extent.merge(attribute.span);
    }
    for node in &block.body {
        extent = extent.merge(match node {
            ast::BlockNode::Line(line) => line.span,
            ast::BlockNode::Block(nested) => block_extent(nested),
        });
    }
    extent
}

/// §21.4: "IR that references an undefined symbol produces a diagnostic at
/// the corresponding user span". A re-validation *error* anchored inside an
/// expanded block can only come from handler-generated code, and the
/// registry files it under `BLOCK004` (malformed IR: undefined symbol,
/// type mismatch); errors anchored elsewhere keep their own code — they
/// point at real user code.
fn attribute_to_handler(diagnostic: Diagnostic, sites: &[ExpandedSite]) -> Diagnostic {
    if diagnostic.level != Level::Error {
        return diagnostic;
    }
    let Some(site) = sites.iter().find(|site| {
        site.span.file == diagnostic.primary_span.file
            && site.span.start <= diagnostic.primary_span.start
            && diagnostic.primary_span.end <= site.span.end
    }) else {
        return diagnostic;
    };
    build(
        Level::Error,
        codes::BLOCK004,
        format!(
            "library '{}' handler for block `{}` returned malformed IR: {}",
            site.library, site.block, diagnostic.message
        ),
        diagnostic.primary_span,
        Some("handler failed".to_string()),
    )
}

fn block004(sink: &mut DiagnosticSink, lib: &str, block: &str, span: Span, detail: &str) {
    // Local wording pinned by the DIA-06 fixture (DISCOVERIES-M5 item 4);
    // covers the registry row's malformed-IR and caught-crash cases.
    sink.push(build(
        Level::Error,
        codes::BLOCK004,
        format!("library '{lib}' handler for block `{block}` {detail}"),
        span,
        Some("handler failed".to_string()),
    ));
}

fn block005(sink: &mut DiagnosticSink, lib: &str, block: &str, span: Span, limit: &str) {
    // 14 §14.4.2: BLOCK005 names the offending library.
    sink.push(build(
        Level::Error,
        codes::BLOCK005,
        format!(
            "library '{lib}' exceeded its compile-time budget expanding block `{block}`: {limit}"
        ),
        span,
        Some("budget exceeded".to_string()),
    ));
}
