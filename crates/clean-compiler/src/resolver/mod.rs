//! Pass [4] — Resolve (Platform 14 §14.4.2), Milestone 1 slice: merges the
//! request's parsed files into one compilation unit, collecting host
//! interfaces, functions, and classes into declaration tables and reporting
//! top-level redefinitions (SEM003). The full module graph — imports,
//! folder-to-library mapping — is M4: an M1 program is one unit with no
//! imports to walk, matching the acceptance guest's `main.cln` +
//! `host_bridge.cln` layout. Local-variable binding is handled during type
//! checking, which owns SEM002/SCOPE002 for locals.

use indexmap::IndexMap;

use clean_compiler_types::{codes, Level};

use crate::diag::{build, DiagnosticSink};
use crate::lexer::TokenStream;
use crate::parser::ast;
use crate::source::ByteSpan;

/// One lexed-and-parsed source file, kept together so later passes can
/// convert its byte spans into diagnostic positions.
pub struct ParsedFile {
    pub ast: ast::SourceFile,
    pub stream: TokenStream,
}

/// Output of pass [4]: every parsed file plus name→declaration tables in
/// declaration order (deterministic iteration, §14.4.1). Coordinates are
/// `(file, item, …)` indexes into `files`.
pub struct ResolvedAst {
    pub files: Vec<ParsedFile>,
    pub decls: Declarations,
}

#[derive(Default)]
pub struct Declarations {
    /// Host-interface slots: `(file, item)` per `Item::HostInterface`.
    pub host_interfaces: Vec<(usize, usize)>,
    /// camelCase Clean name → (host-interface slot, function index).
    pub host_functions: IndexMap<String, (usize, usize)>,
    /// Function name → (file, item, function index).
    pub functions: IndexMap<String, (usize, usize, usize)>,
    /// Class name → (file, item).
    pub classes: IndexMap<String, (usize, usize)>,
    /// `start:` blocks, in `sources[]` order: `(file, item)` (FNC-01).
    pub starts: Vec<(usize, usize)>,
}

pub fn resolve(files: Vec<ParsedFile>, sink: &mut DiagnosticSink) -> ResolvedAst {
    let mut decls = Declarations::default();

    for (file_index, file) in files.iter().enumerate() {
        for (item_index, item) in file.ast.items.iter().enumerate() {
            match item {
                ast::Item::HostInterface(hi) => {
                    let slot = decls.host_interfaces.len();
                    decls.host_interfaces.push((file_index, item_index));
                    for (fn_index, f) in hi.functions.iter().enumerate() {
                        if decls.host_functions.contains_key(&f.name)
                            || decls.functions.contains_key(&f.name)
                        {
                            redefinition(sink, &file.stream, &f.name, f.span);
                            continue;
                        }
                        decls
                            .host_functions
                            .insert(f.name.clone(), (slot, fn_index));
                    }
                }
                ast::Item::Functions(functions) => {
                    for (fn_index, f) in functions.iter().enumerate() {
                        if decls.functions.contains_key(&f.name)
                            || decls.host_functions.contains_key(&f.name)
                        {
                            redefinition(sink, &file.stream, &f.name, f.span);
                            continue;
                        }
                        decls
                            .functions
                            .insert(f.name.clone(), (file_index, item_index, fn_index));
                    }
                }
                ast::Item::Class(class) => {
                    if decls.classes.contains_key(&class.name) {
                        redefinition(sink, &file.stream, &class.name, class.span);
                        continue;
                    }
                    decls
                        .classes
                        .insert(class.name.clone(), (file_index, item_index));
                }
                ast::Item::Start(_) => {
                    decls.starts.push((file_index, item_index));
                }
                // Parsed forms whose semantics land in later milestones
                // (M4 imports/state, M5 block handlers, M6 stdlib). Each is
                // reported through the pre-v1 Unsupported channel — never
                // silently dropped, never an invented code.
                ast::Item::Function(f) => {
                    unsupported(
                        sink,
                        &file.stream,
                        "top-level function declarations",
                        f.span,
                    );
                }
                ast::Item::Imports(entries) => {
                    if let Some(first) = entries.first() {
                        unsupported(sink, &file.stream, "import declarations", first.span);
                    }
                }
                ast::Item::FileImport { span, .. } => {
                    unsupported(sink, &file.stream, "import declarations", *span);
                }
                ast::Item::Source(section) => {
                    unsupported(
                        sink,
                        &file.stream,
                        "source: provenance blocks",
                        section.span,
                    );
                }
                ast::Item::Constants(constants) => {
                    if let Some(first) = constants.first() {
                        unsupported(sink, &file.stream, "constant: sections", first.span);
                    }
                }
                ast::Item::State(section) => {
                    unsupported(sink, &file.stream, "state: blocks", section.span);
                }
                ast::Item::ConstantFunction(f) => {
                    unsupported(sink, &file.stream, "constant functions", f.span);
                }
                ast::Item::CompiletimeFunction(f) => {
                    unsupported(sink, &file.stream, "compiletime functions", f.span);
                }
                ast::Item::HandlesBlock(h) => {
                    unsupported(sink, &file.stream, "handles block registrations", h.span);
                }
                ast::Item::Capability(capability) => {
                    unsupported(
                        sink,
                        &file.stream,
                        "capability declarations",
                        capability.span,
                    );
                }
                ast::Item::Watch(watch) => {
                    unsupported(sink, &file.stream, "watch blocks", watch.span);
                }
                ast::Item::LibraryBlock(block) => {
                    unsupported(sink, &file.stream, "library blocks", block.span);
                }
                ast::Item::Tests(tests) => {
                    if let Some(first) = tests.first() {
                        let span = match first {
                            ast::TestDecl::Named { span, .. }
                            | ast::TestDecl::Anonymous { span, .. }
                            | ast::TestDecl::Block { span, .. } => *span,
                        };
                        unsupported(sink, &file.stream, "tests: sections", span);
                    }
                }
            }
        }
    }

    ResolvedAst { files, decls }
}

/// Pre-v1 reporting for parsed-but-not-yet-compilable constructs.
fn unsupported(
    sink: &mut DiagnosticSink,
    stream: &TokenStream,
    construct: &'static str,
    span: ByteSpan,
) {
    sink.note_unsupported(construct, stream.diag_span(span));
}

impl ResolvedAst {
    pub fn host_interface(&self, slot: usize) -> (&ast::HostInterface, usize) {
        let (file, item) = self.decls.host_interfaces[slot];
        match &self.files[file].ast.items[item] {
            ast::Item::HostInterface(hi) => (hi, file),
            _ => unreachable!("host_interfaces indexes only HostInterface items"),
        }
    }

    pub fn function(&self, coords: (usize, usize, usize)) -> (&ast::Function, usize) {
        match &self.files[coords.0].ast.items[coords.1] {
            ast::Item::Functions(functions) => (&functions[coords.2], coords.0),
            _ => unreachable!("functions indexes only Functions items"),
        }
    }

    pub fn class(&self, coords: (usize, usize)) -> (&ast::ClassDecl, usize) {
        match &self.files[coords.0].ast.items[coords.1] {
            ast::Item::Class(class) => (class, coords.0),
            _ => unreachable!("classes indexes only Class items"),
        }
    }

    pub fn start(&self, coords: (usize, usize)) -> (&ast::Block, usize) {
        match &self.files[coords.0].ast.items[coords.1] {
            ast::Item::Start(block) => (block, coords.0),
            _ => unreachable!("starts indexes only Start items"),
        }
    }

    /// Diagnostic span for a byte span in file `file`.
    pub fn span(&self, file: usize, span: ByteSpan) -> clean_compiler_types::Span {
        self.files[file].stream.diag_span(span)
    }
}

/// SEM003 — Symbol Redefinition (Platform 10 §3).
fn redefinition(sink: &mut DiagnosticSink, stream: &TokenStream, name: &str, span: ByteSpan) {
    sink.push(build(
        Level::Error,
        codes::SEM003,
        format!("`{name}` is declared more than once"),
        stream.diag_span(span),
        Some("redefinition".to_string()),
    ));
}
