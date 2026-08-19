//! The hover index: what the editor shows at a position, computed from the
//! same typed program the pipeline checked (CCMP-25 — the type checker's
//! answer, never a re-derivation).
//!
//! Built inside `check_with`'s observer, so diagnostics and editor
//! intelligence come from one pipeline run over one request.

use clean_compiler::resolver::ResolvedAst;
use clean_compiler::source::ByteSpan;
use clean_compiler::typecheck::tir::{TExprKind, TFunction, TypedProgram};

/// One hoverable region: `text` is what the editor shows for `span`.
struct HoverEntry {
    file: usize,
    span: ByteSpan,
    text: String,
}

/// One jumpable region: from the use at `span` to the declaration at
/// `target_span` in `target_file`.
struct DefEntry {
    file: usize,
    span: ByteSpan,
    target_file: usize,
    target_span: ByteSpan,
}

pub struct Index {
    /// Expression entries first, declaration entries after — on equal span
    /// width the earlier entry wins, so the specific beats the enclosing.
    hovers: Vec<HoverEntry>,
    defs: Vec<DefEntry>,
}

/// Clean-surface signature: `ret name(param-type param, …)`.
fn signature(function: &TFunction) -> String {
    let params: Vec<String> = function
        .params
        .iter()
        .map(|p| format!("{} {}", p.ty.display(), p.name))
        .collect();
    format!(
        "{} {}({})",
        function.ret.display(),
        function.name,
        params.join(", ")
    )
}

pub fn build(resolved: &ResolvedAst, typed: &TypedProgram) -> Index {
    let mut hovers = Vec::new();
    let mut defs = Vec::new();
    typed.for_each_expr(&mut |file, func, expr| {
        // Platform 04 §4.1: the type of the expression under the cursor —
        // and for calls, the callee's signature, which subsumes the type.
        let text = match &expr.kind {
            TExprKind::CallFn { func, .. } => signature(&typed.functions[*func]),
            TExprKind::CallHost { import, .. } => {
                let host = &typed.host_imports[*import];
                let params: Vec<String> = host.params.iter().map(|ty| ty.display()).collect();
                format!(
                    "{} {}({})",
                    host.ret.display(),
                    host.clean_name,
                    params.join(", ")
                )
            }
            _ => expr.ty.display(),
        };
        hovers.push(HoverEntry {
            file,
            span: expr.span,
            text,
        });
        // Definition targets resolvable from the typed program alone:
        // user-function calls, local/parameter reads, state reads, and
        // host-function calls (whose declaring file comes from the
        // resolver's host tables). Methods, fields, and classes wait on
        // declaration spans the resolver does not surface yet.
        let target = match &expr.kind {
            TExprKind::CallFn { func, .. } => {
                let target = &typed.functions[*func];
                Some((target.file, target.span))
            }
            TExprKind::Local(local) => func.map(|func| {
                let target = typed.local(func, *local);
                (typed.functions[func].file, target.span)
            }),
            TExprKind::GetState { module, name } => typed
                .state_vars
                .iter()
                .find(|v| v.module == *module && v.name == *name)
                .map(|v| (v.module, v.span)),
            TExprKind::CallHost { import, .. } => {
                let host = &typed.host_imports[*import];
                resolved
                    .decls
                    .host_functions
                    .get(host.clean_name.as_str())
                    .map(|(slot, _)| (resolved.decls.host_interfaces[*slot].0, host.span))
            }
            _ => None,
        };
        if let Some((target_file, target_span)) = target {
            defs.push(DefEntry {
                file,
                span: expr.span,
                target_file,
                target_span,
            });
        }
    });
    for function in &typed.functions {
        hovers.push(HoverEntry {
            file: function.file,
            span: function.span,
            text: signature(function),
        });
    }
    for var in &typed.state_vars {
        hovers.push(HoverEntry {
            file: var.module,
            span: var.span,
            text: format!("{} {}", var.ty.display(), var.name),
        });
    }
    Index { hovers, defs }
}

impl Index {
    /// The narrowest entry containing the byte offset in `file`.
    pub fn hover(&self, file: usize, offset: u32) -> Option<(&str, ByteSpan)> {
        self.hovers
            .iter()
            .filter(|e| e.file == file && e.span.start <= offset && offset < e.span.end)
            .min_by_key(|e| e.span.end - e.span.start)
            .map(|e| (e.text.as_str(), e.span))
    }

    /// The declaration site for the narrowest jumpable use at the offset.
    pub fn definition(&self, file: usize, offset: u32) -> Option<(usize, ByteSpan)> {
        self.defs
            .iter()
            .filter(|e| e.file == file && e.span.start <= offset && offset < e.span.end)
            .min_by_key(|e| e.span.end - e.span.start)
            .map(|e| (e.target_file, e.target_span))
    }
}
