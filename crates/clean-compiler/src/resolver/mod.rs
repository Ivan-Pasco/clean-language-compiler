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
                ast::Item::Start(_) => {}
            }
        }
    }

    ResolvedAst { files, decls }
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
