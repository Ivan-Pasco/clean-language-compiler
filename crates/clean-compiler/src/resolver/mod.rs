//! Pass [4] — Resolve (Platform 14 §14.4.2), M4: builds the module graph
//! from the request's parsed files (each file is a module named after its
//! filename, chapter 17), resolves `import:` entries and `import "path"`
//! statements strictly across `sources[]` (MOD-03 — no filesystem), reports
//! cycles (IMPORT001, once per cycle, resolution continues), missing
//! modules (IMPORT002), missing/private symbols (IMPORT003) and duplicate
//! import items (IMPORT004), and assembles one visibility scope per module
//! (MOD-02: private by default, `public:` exports).
//!
//! Local adoptions recorded in docs/DISCOVERIES-M4.md: host interfaces are
//! request-global (they describe the world boundary, ADR-0002); classes are
//! exported by default until foundation pins a class-export syntax; imports
//! are non-transitive; a locally declared name shadows an imported one.
//! Local-variable binding is handled during type checking, which owns
//! SEM002/SCOPE002 for locals.

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

/// Output of pass [4]: every parsed file plus declaration tables and one
/// visibility scope per module, all in deterministic `sources[]`/
/// declaration order (§14.4.1). Coordinates are `(file, item, …)` indexes
/// into `files`.
pub struct ResolvedAst {
    pub files: Vec<ParsedFile>,
    pub decls: Declarations,
}

/// The chapter-15 built-in modules: available without import, valid as
/// import targets. Their surfaces land in M6.
pub const BUILTIN_MODULES: [&str; 8] = [
    "console",
    "math",
    "string",
    "list",
    "file",
    "http",
    "json",
    "validator",
];

#[derive(Default)]
pub struct Declarations {
    /// Host-interface slots: `(file, item)` per `Item::HostInterface`.
    pub host_interfaces: Vec<(usize, usize)>,
    /// camelCase Clean name → (host-interface slot, function index).
    /// Request-global: the world boundary is a request-level property.
    pub host_functions: IndexMap<String, (usize, usize)>,
    /// Every declared function across all modules, flat, in `sources[]`
    /// then declaration order. `CallFn` and the signature table index
    /// into this.
    pub functions: Vec<FnDecl>,
    /// Every declared class, flat, in the same order discipline.
    pub classes: Vec<ClassRef>,
    /// Every declared capability (CLS-03), flat, same order discipline.
    pub capabilities: Vec<CapRef>,
    /// `start:` blocks, in `sources[]` order: `(file, item)` (FNC-01).
    pub starts: Vec<(usize, usize)>,
    /// `state:` sections (SMG-01), `(file, item)`.
    pub states: Vec<(usize, usize)>,
    /// `watch` blocks (SMG-04), `(file, item)`.
    pub watches: Vec<(usize, usize)>,
    /// `tests:` sections (TST-01), `(file, item)`.
    pub tests: Vec<(usize, usize)>,
    /// `Item::LibraryBlock` sites in `sources[]` then item order —
    /// pass [6]'s work list (chapter 21).
    pub blocks: Vec<(usize, usize)>,
    /// `compiletime function` declarations (BLK-01), `(file, item)`.
    pub compiletime_functions: Vec<(usize, usize)>,
    /// `handles block "…" with …` registrations (BLK-01), `(file, item)`;
    /// validated in pass [6] against the unit's compiletime functions.
    pub handles: Vec<(usize, usize)>,
    /// Visibility scope per module; index = file index.
    pub modules: Vec<ModuleScope>,
}

pub struct FnDecl {
    pub name: String,
    /// `(file, item, function index)` into `files`.
    pub coords: (usize, usize, usize),
    /// Declared inside a `public:` wrapper (MOD-02).
    pub public: bool,
}

pub struct ClassRef {
    pub name: String,
    /// `(file, item)` into `files`.
    pub coords: (usize, usize),
}

pub struct CapRef {
    pub name: String,
    /// `(file, item)` into `files`.
    pub coords: (usize, usize),
}

/// What one module sees: its own declarations plus what its imports
/// brought in (public names only, MOD-01/MOD-02).
#[derive(Default)]
pub struct ModuleScope {
    /// Module display name — the filename stem (chapter 17 §Module
    /// Definition).
    pub name: String,
    /// Visible name → index into `Declarations::functions`.
    pub functions: IndexMap<String, usize>,
    /// Visible name → index into `Declarations::classes`.
    pub classes: IndexMap<String, usize>,
    /// Visible name → index into `Declarations::capabilities` (exported
    /// by default, same adoption as classes).
    pub capabilities: IndexMap<String, usize>,
    /// `utils as u` module aliases → file index (namespace-style access
    /// is chapter-16 dispatch, M4 stage 3).
    pub module_aliases: IndexMap<String, usize>,
    /// Library names this file imported explicitly — §21.2 rule 1 block
    /// scope (adoption: an import path or dotted prefix matching a
    /// `library_manifests[].name`, DISCOVERIES-M5 item 7). Insertion
    /// order, deduplicated.
    pub library_imports: Vec<String>,
}

/// The filename stem (`app/data/models.cln` → `models`).
fn module_stem(path: &str) -> String {
    let base = path.rsplit('/').next().unwrap_or(path);
    base.strip_suffix(".cln").unwrap_or(base).to_string()
}

/// The directory prefix of a source path, empty for top-level files.
fn dir_of(path: &str) -> &str {
    match path.rfind('/') {
        Some(i) => &path[..i],
        None => "",
    }
}

/// Lexically normalizes `base_dir` joined with `rel` (`.` and `..`
/// segments). Purely textual — MOD-03 forbids touching the filesystem.
fn join_normalize(base_dir: &str, rel: &str) -> String {
    let mut parts: Vec<&str> = if base_dir.is_empty() {
        Vec::new()
    } else {
        base_dir.split('/').collect()
    };
    for seg in rel.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

/// One resolved import edge, for cycle detection and scope building.
struct ImportEdge {
    from: usize,
    to: usize,
    /// The imported symbol (`math.sqrt` form), if any, with its bind name.
    symbol: Option<(String, String)>,
    /// `utils as u` module alias.
    module_alias: Option<String>,
    span: ByteSpan,
}

/// `libraries` are the `library_manifests[].name`s from the request; an
/// import path matching one is a §21.2 explicit library import, not an
/// IMPORT002 (adoption, DISCOVERIES-M5 item 7).
pub fn resolve(
    files: Vec<ParsedFile>,
    libraries: &[String],
    sink: &mut DiagnosticSink,
) -> ResolvedAst {
    let mut decls = Declarations::default();
    for file in &files {
        decls.modules.push(ModuleScope {
            name: module_stem(&file.stream.path),
            ..Default::default()
        });
    }

    // Phase 1 — collect declarations per module (SEM003 within a module;
    // host functions clash globally because the boundary is global).
    for (file_index, file) in files.iter().enumerate() {
        for (item_index, item) in file.ast.items.iter().enumerate() {
            match item {
                ast::Item::HostInterface(hi) => {
                    let slot = decls.host_interfaces.len();
                    decls.host_interfaces.push((file_index, item_index));
                    for (fn_index, f) in hi.functions.iter().enumerate() {
                        if decls.host_functions.contains_key(&f.name) {
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
                        let scope = &mut decls.modules[file_index];
                        if scope.functions.contains_key(&f.name)
                            || decls.host_functions.contains_key(&f.name)
                        {
                            redefinition(sink, &file.stream, &f.name, f.span);
                            continue;
                        }
                        let index = decls.functions.len();
                        decls.functions.push(FnDecl {
                            name: f.name.clone(),
                            coords: (file_index, item_index, fn_index),
                            public: f.public,
                        });
                        decls.modules[file_index]
                            .functions
                            .insert(f.name.clone(), index);
                    }
                }
                ast::Item::Class(class) => {
                    if decls.modules[file_index].classes.contains_key(&class.name) {
                        redefinition(sink, &file.stream, &class.name, class.span);
                        continue;
                    }
                    let index = decls.classes.len();
                    decls.classes.push(ClassRef {
                        name: class.name.clone(),
                        coords: (file_index, item_index),
                    });
                    decls.modules[file_index]
                        .classes
                        .insert(class.name.clone(), index);
                }
                ast::Item::Start(_) => {
                    decls.starts.push((file_index, item_index));
                }
                // Parsed forms whose semantics land in later milestones
                // (M4 later stages state/capabilities, M5 block handlers,
                // M6 stdlib). Each is reported through the pre-v1
                // Unsupported channel — never silently dropped, never an
                // invented code.
                ast::Item::Function(f) => {
                    // FUNC013 — template from Platform 10 §5 (FNC-02).
                    sink.push(build(
                        Level::Error,
                        codes::FUNC013,
                        format!(
                            "Function '{}' must be declared inside a 'functions:' block",
                            f.name
                        ),
                        file.stream.diag_span(f.span),
                        Some("not inside functions:".to_string()),
                    ));
                }
                ast::Item::Imports(_) | ast::Item::FileImport { .. } => {}
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
                ast::Item::State(_) => {
                    decls.states.push((file_index, item_index));
                }
                ast::Item::ConstantFunction(f) => {
                    unsupported(sink, &file.stream, "constant functions", f.span);
                }
                ast::Item::CompiletimeFunction(f) => {
                    decls.compiletime_functions.push((file_index, item_index));
                    // The declaration surface is accepted (BLK-01); the
                    // body's TYP-04 checking and self-hosted execution are
                    // blocked on spec gaps recorded in DISCOVERIES-M5 —
                    // BlockNode pattern-matching has no defined syntax.
                    unsupported(sink, &file.stream, "compiletime function bodies", f.span);
                }
                ast::Item::HandlesBlock(_) => {
                    decls.handles.push((file_index, item_index));
                }
                ast::Item::Capability(capability) => {
                    if decls.modules[file_index]
                        .capabilities
                        .contains_key(&capability.name)
                    {
                        redefinition(sink, &file.stream, &capability.name, capability.span);
                        continue;
                    }
                    let index = decls.capabilities.len();
                    decls.capabilities.push(CapRef {
                        name: capability.name.clone(),
                        coords: (file_index, item_index),
                    });
                    decls.modules[file_index]
                        .capabilities
                        .insert(capability.name.clone(), index);
                }
                ast::Item::Watch(_) => {
                    decls.watches.push((file_index, item_index));
                }
                ast::Item::LibraryBlock(_) => {
                    decls.blocks.push((file_index, item_index));
                }
                ast::Item::Tests(_) => {
                    decls.tests.push((file_index, item_index));
                }
            }
        }
    }

    // Phase 2 — resolve import entries to edges (IMPORT002/004; the
    // symbol leg of IMPORT003 waits for the scopes, phase 4).
    let mut edges: Vec<ImportEdge> = Vec::new();
    for (file_index, file) in files.iter().enumerate() {
        // Duplicate-item tracking per importing file (IMPORT004): the
        // resolved target plus the imported symbol, aliases ignored.
        let mut seen: Vec<(usize, Option<String>)> = Vec::new();
        for item in &file.ast.items {
            match item {
                ast::Item::Imports(entries) => {
                    for entry in entries {
                        resolve_module_entry(
                            entry,
                            file_index,
                            &files,
                            libraries,
                            &mut decls.modules[file_index].library_imports,
                            &mut edges,
                            &mut seen,
                            sink,
                        );
                    }
                }
                ast::Item::FileImport { path, span } => {
                    let target = join_normalize(dir_of(&file.stream.path), path);
                    match files.iter().position(|f| f.stream.path == target) {
                        Some(to) => {
                            push_edge(
                                ImportEdge {
                                    from: file_index,
                                    to,
                                    symbol: None,
                                    module_alias: None,
                                    span: *span,
                                },
                                &mut edges,
                                &mut seen,
                                &files[file_index],
                                path,
                                sink,
                            );
                        }
                        None => {
                            // IMPORT002 (stub rule; local wording).
                            sink.push(build(
                                Level::Error,
                                codes::IMPORT002,
                                format!("I cannot find the module `{path}` in this build"),
                                file.stream.diag_span(*span),
                                Some("no source in the compilation request has this path".into()),
                            ));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Phase 3 — cycle detection over the module graph (IMPORT001, one
    // diagnostic per cycle; resolution continues).
    report_cycles(&files, &edges, &decls, sink);

    // Phase 4 — extend each module's scope with what its imports bring
    // in: public functions and all classes (see the class-export
    // adoption) for whole-module imports, the named symbol otherwise.
    // A locally declared name shadows an imported one; two imports of
    // the same name keep the first (deterministic in entry order).
    for edge in &edges {
        let from = edge.from;
        match &edge.symbol {
            None => {
                let target_fns: Vec<(String, usize)> = decls.modules[edge.to]
                    .functions
                    .iter()
                    .filter(|(_, &i)| decls.functions[i].public)
                    .map(|(n, &i)| (n.clone(), i))
                    .collect();
                for (name, index) in target_fns {
                    decls.modules[from].functions.entry(name).or_insert(index);
                }
                let target_classes: Vec<(String, usize)> = decls.modules[edge.to]
                    .classes
                    .iter()
                    .map(|(n, &i)| (n.clone(), i))
                    .collect();
                for (name, index) in target_classes {
                    decls.modules[from].classes.entry(name).or_insert(index);
                }
                let target_caps: Vec<(String, usize)> = decls.modules[edge.to]
                    .capabilities
                    .iter()
                    .map(|(n, &i)| (n.clone(), i))
                    .collect();
                for (name, index) in target_caps {
                    decls.modules[from]
                        .capabilities
                        .entry(name)
                        .or_insert(index);
                }
                // The module is addressable namespace-style (chapter 16)
                // under its alias, or its stem when no alias was given.
                let handle = edge
                    .module_alias
                    .clone()
                    .unwrap_or_else(|| decls.modules[edge.to].name.clone());
                decls.modules[from].module_aliases.insert(handle, edge.to);
            }
            Some((symbol, bind_name)) => {
                let function = decls.modules[edge.to]
                    .functions
                    .get(symbol)
                    .copied()
                    .filter(|&i| decls.functions[i].public);
                let class = decls.modules[edge.to].classes.get(symbol).copied();
                match (function, class) {
                    (Some(index), _) => {
                        decls.modules[from]
                            .functions
                            .entry(bind_name.clone())
                            .or_insert(index);
                    }
                    (None, Some(index)) => {
                        decls.modules[from]
                            .classes
                            .entry(bind_name.clone())
                            .or_insert(index);
                    }
                    (None, None) => {
                        // IMPORT003 (stub rule; local wording): absent and
                        // private are the same answer from outside (MOD-02).
                        sink.push(build(
                            Level::Error,
                            codes::IMPORT003,
                            format!(
                                "module `{}` has no public symbol named `{symbol}`",
                                decls.modules[edge.to].name
                            ),
                            files[from].stream.diag_span(edge.span),
                            Some("not exported by this module".into()),
                        ));
                    }
                }
            }
        }
    }

    ResolvedAst { files, decls }
}

/// Resolves one `import:` block entry (MOD-01): whole module, single
/// symbol, or either with an alias; built-ins pass through as a frontier
/// note (their surfaces are M6).
#[allow(clippy::too_many_arguments)]
fn resolve_module_entry(
    entry: &ast::ImportEntry,
    file_index: usize,
    files: &[ParsedFile],
    libraries: &[String],
    library_imports: &mut Vec<String>,
    edges: &mut Vec<ImportEdge>,
    seen: &mut Vec<(usize, Option<String>)>,
    sink: &mut DiagnosticSink,
) {
    let parts = &entry.path;
    let display = parts.join(".");
    let file = &files[file_index];

    // Whole dotted path as a module (`data.models` → `data/models.cln`)?
    if let Some(to) = find_module(parts, file_index, files) {
        push_edge(
            ImportEdge {
                from: file_index,
                to,
                symbol: None,
                module_alias: entry.alias.clone(),
                span: entry.span,
            },
            edges,
            seen,
            file,
            &display,
            sink,
        );
        return;
    }
    // Prefix as module + last segment as symbol (`math.sqrt`)?
    if parts.len() >= 2 {
        let (symbol, prefix) = parts.split_last().unwrap();
        if let Some(to) = find_module(prefix, file_index, files) {
            let bind = entry.alias.clone().unwrap_or_else(|| symbol.clone());
            push_edge(
                ImportEdge {
                    from: file_index,
                    to,
                    symbol: Some((symbol.clone(), bind)),
                    module_alias: None,
                    span: entry.span,
                },
                edges,
                seen,
                file,
                &display,
                sink,
            );
            return;
        }
        // Built-in module symbol (`json.decode`)?
        if prefix.len() == 1 && BUILTIN_MODULES.contains(&prefix[0].as_str()) {
            sink.note_unsupported(
                "standard-library imports",
                file.stream.diag_span(entry.span),
            );
            return;
        }
    }
    // Whole built-in module (`math`)?
    if parts.len() == 1 && BUILTIN_MODULES.contains(&parts[0].as_str()) {
        sink.note_unsupported(
            "standard-library imports",
            file.stream.diag_span(entry.span),
        );
        return;
    }
    // §21.2 rule 1: an import path (or dotted prefix, longest first)
    // matching a library manifest's name is an explicit library import —
    // it brings the library's block handlers into scope for this file.
    // Sources-module resolution was tried first, so a module/library name
    // collision resolves in the module's favor (DISCOVERIES-M5 item 7).
    for end in (1..=parts.len()).rev() {
        let candidate = parts[..end].join(".");
        if libraries.contains(&candidate) {
            if !library_imports.contains(&candidate) {
                library_imports.push(candidate);
            }
            return;
        }
    }
    // IMPORT002 (stub rule; local wording).
    sink.push(build(
        Level::Error,
        codes::IMPORT002,
        format!("I cannot find the module `{display}` in this build"),
        file.stream.diag_span(entry.span),
        Some("no source in the compilation request provides this module".into()),
    ));
}

/// MOD-03: a dotted module name resolves across `sources[]` and nowhere
/// else. Preference order (all purely textual): the path relative to the
/// importing file's directory, the path from the request root, then a
/// unique `…/name.cln` suffix match anywhere in the set. An ambiguous
/// suffix resolves to nothing (the framework should have disambiguated).
fn find_module(parts: &[String], importing: usize, files: &[ParsedFile]) -> Option<usize> {
    if parts.is_empty() {
        return None;
    }
    let rel = format!("{}.cln", parts.join("/"));
    let from_here = join_normalize(dir_of(&files[importing].stream.path), &rel);
    if let Some(i) = files.iter().position(|f| f.stream.path == from_here) {
        return Some(i);
    }
    if let Some(i) = files.iter().position(|f| f.stream.path == rel) {
        return Some(i);
    }
    let suffix = format!("/{rel}");
    let mut matches = files
        .iter()
        .enumerate()
        .filter(|(_, f)| f.stream.path.ends_with(&suffix));
    match (matches.next(), matches.next()) {
        (Some((i, _)), None) => Some(i),
        _ => None,
    }
}

/// Appends an edge unless the same (target, symbol) item was already
/// imported by this file — IMPORT004 (stub rule; local wording).
fn push_edge(
    edge: ImportEdge,
    edges: &mut Vec<ImportEdge>,
    seen: &mut Vec<(usize, Option<String>)>,
    file: &ParsedFile,
    display: &str,
    sink: &mut DiagnosticSink,
) {
    let key = (edge.to, edge.symbol.as_ref().map(|(s, _)| s.clone()));
    if seen.contains(&key) {
        sink.push(build(
            Level::Error,
            codes::IMPORT004,
            format!("`{display}` is imported more than once"),
            file.stream.diag_span(edge.span),
            Some("duplicate import item".into()),
        ));
        return;
    }
    seen.push(key);
    edges.push(edge);
}

/// IMPORT001 — one diagnostic per cycle, template from Platform 10 §9:
/// `import cycle detected: {A → B → … → A}`, primary at the entry that
/// closes the cycle. A depth-first walk reports one cycle per back edge;
/// resolution continues afterwards (MOD-03).
fn report_cycles(
    files: &[ParsedFile],
    edges: &[ImportEdge],
    decls: &Declarations,
    sink: &mut DiagnosticSink,
) {
    let n = files.len();
    let mut adjacency: Vec<Vec<(usize, ByteSpan)>> = vec![Vec::new(); n];
    for edge in edges {
        adjacency[edge.from].push((edge.to, edge.span));
    }

    #[derive(Clone, Copy, PartialEq)]
    enum Color {
        White,
        Grey,
        Black,
    }
    let mut color = vec![Color::White; n];
    let mut stack: Vec<usize> = Vec::new();

    fn visit(
        node: usize,
        adjacency: &[Vec<(usize, ByteSpan)>],
        color: &mut [Color],
        stack: &mut Vec<usize>,
        files: &[ParsedFile],
        decls: &Declarations,
        sink: &mut DiagnosticSink,
    ) {
        color[node] = Color::Grey;
        stack.push(node);
        for &(to, span) in &adjacency[node] {
            match color[to] {
                Color::Grey => {
                    let start = stack.iter().position(|&f| f == to).unwrap_or(0);
                    let mut names: Vec<String> = stack[start..]
                        .iter()
                        .map(|&f| decls.modules[f].name.clone())
                        .collect();
                    names.push(decls.modules[to].name.clone());
                    sink.push(build(
                        Level::Error,
                        codes::IMPORT001,
                        format!("import cycle detected: {}", names.join(" → ")),
                        files[node].stream.diag_span(span),
                        Some("this import closes the cycle".to_string()),
                    ));
                }
                Color::White => {
                    visit(to, adjacency, color, stack, files, decls, sink);
                }
                Color::Black => {}
            }
        }
        stack.pop();
        color[node] = Color::Black;
    }

    for node in 0..n {
        if color[node] == Color::White {
            visit(node, &adjacency, &mut color, &mut stack, files, decls, sink);
        }
    }
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

    pub fn capability(&self, coords: (usize, usize)) -> (&ast::CapabilityDecl, usize) {
        match &self.files[coords.0].ast.items[coords.1] {
            ast::Item::Capability(capability) => (capability, coords.0),
            _ => unreachable!("capabilities indexes only Capability items"),
        }
    }

    pub fn state(&self, coords: (usize, usize)) -> (&ast::StateSection, usize) {
        match &self.files[coords.0].ast.items[coords.1] {
            ast::Item::State(section) => (section, coords.0),
            _ => unreachable!("states indexes only State items"),
        }
    }

    pub fn watch(&self, coords: (usize, usize)) -> (&ast::WatchBlock, usize) {
        match &self.files[coords.0].ast.items[coords.1] {
            ast::Item::Watch(watch) => (watch, coords.0),
            _ => unreachable!("watches indexes only Watch items"),
        }
    }

    pub fn tests(&self, coords: (usize, usize)) -> (&[ast::TestDecl], usize) {
        match &self.files[coords.0].ast.items[coords.1] {
            ast::Item::Tests(tests) => (tests, coords.0),
            _ => unreachable!("tests indexes only Tests items"),
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
