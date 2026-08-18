//! Pass [6] — Block Handler Expansion (Platform 14 §14.4.2, ADR-0004).
//! Executes library `compiletime` handlers in a sandboxed wasmtime
//! sub-instance (epoch interruption, memory cap, no host imports) and
//! splices the returned IR into the program.

pub mod resolve;
pub mod sandbox;

use clean_compiler_types::request::CompileRequest;

use crate::diag::DiagnosticSink;
use crate::resolver::ResolvedAst;

/// Pass [6] entry point. Builds the library catalog (manifest, dependency
/// and folder-scope findings fire even for programs with no blocks —
/// "library load" per LEX-05), then resolves every `Item::LibraryBlock`
/// site per §21.2. Handler execution and IR splicing land in the next
/// stage; a block that resolves cleanly is reported through the pre-v1
/// Unsupported channel until then.
pub fn expand(resolved: &ResolvedAst, request: &CompileRequest, sink: &mut DiagnosticSink) {
    let catalog = resolve::build_catalog(request, sink);

    for &(file_index, item_index) in &resolved.decls.blocks {
        let file = &resolved.files[file_index];
        let crate::parser::ast::Item::LibraryBlock(block) = &file.ast.items[item_index] else {
            continue;
        };
        let span = file.stream.diag_span(block.span);
        let resolution = resolve::resolve_block(
            &block.name,
            &file.stream.path,
            &resolved.decls.modules[file_index].library_imports,
            &catalog,
            &request.folders,
            span.clone(),
            sink,
        );
        if resolution.is_some() {
            sink.note_unsupported("library block expansion", span);
        }
    }
}
