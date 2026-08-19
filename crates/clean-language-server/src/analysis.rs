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

pub struct Index {
    /// Expression entries first, declaration entries after — on equal span
    /// width the earlier entry wins, so the specific beats the enclosing.
    hovers: Vec<HoverEntry>,
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

pub fn build(_resolved: &ResolvedAst, typed: &TypedProgram) -> Index {
    let mut hovers = Vec::new();
    typed.for_each_expr(&mut |file, expr| {
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
    Index { hovers }
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
}
