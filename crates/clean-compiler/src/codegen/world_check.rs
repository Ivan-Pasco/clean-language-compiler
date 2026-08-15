//! Pass [9] — World Import Check (Platform 14 §14.4.2, CMP-03): every host
//! call site is verified against the target world — the interface exists,
//! the function exists, and the declared signature projects onto the WIT
//! signature. Any miss is `COM012` at the call site and the pipeline aborts
//! before codegen; no `component.wasm` is written. The compiler never
//! fetches a world (CCMP-03) — this checks against the WIT the request
//! delivered.

use clean_compiler_types::{codes, Level};
use wit_parser::{WorldItem, WorldKey};

use crate::diag::{build, DiagnosticSink};
use crate::hir::{HExpr, HExprKind, HStmt, HirProgram};
use crate::resolver::ResolvedAst;
use crate::source::ByteSpan;
use crate::typecheck::tir::HostImport;
use crate::typecheck::types::{project_wit, Ty};

use super::world::ParsedWorld;

pub fn check(
    program: &HirProgram,
    world: &ParsedWorld,
    world_name: &str,
    resolved: &ResolvedAst,
    sink: &mut DiagnosticSink,
) {
    // Verdict per import, computed once; reported per call site.
    let verdicts: Vec<Option<String>> = program
        .host_imports
        .iter()
        .map(|import| verify_import(import, world))
        .collect();

    for function in &program.functions {
        for stmt in &function.body {
            walk_stmt(stmt, &verdicts, world_name, resolved, function.file, sink);
        }
    }
}

/// `None` = the world provides it; `Some(qualified-name)` = it does not.
fn verify_import(import: &HostImport, world: &ParsedWorld) -> Option<String> {
    let qualified = format!("clean:host/{}#{}", import.interface, import.wit_name);
    let resolve = &world.resolve;
    let world_def = &resolve.worlds[world.world];
    for (key, item) in &world_def.exports {
        let WorldItem::Interface { id, .. } = item else {
            continue;
        };
        let name = match key {
            WorldKey::Name(n) => n.clone(),
            WorldKey::Interface(i) => resolve.interfaces[*i].name.clone().unwrap_or_default(),
        };
        if name != import.interface {
            continue;
        }
        let iface = &resolve.interfaces[*id];
        let Some(function) = iface.functions.get(&import.wit_name) else {
            return Some(qualified);
        };
        // Full-signature verification: parameter count, each parameter's
        // projection, and the result.
        if function.params.len() != import.params.len() {
            return Some(qualified);
        }
        for (param, declared) in function.params.iter().zip(&import.params) {
            match project_wit(resolve, &param.ty) {
                Some(projected) if projected == *declared => {}
                _ => return Some(qualified),
            }
        }
        let declared_result = match &import.ret {
            Ty::Void => None,
            other => Some(other.clone()),
        };
        let world_result = function
            .result
            .as_ref()
            .and_then(|ty| project_wit(resolve, ty));
        if declared_result != world_result
            && !(function.result.is_none() && declared_result.is_none())
        {
            return Some(qualified);
        }
        return None;
    }
    Some(qualified)
}

fn walk_stmt(
    stmt: &HStmt,
    verdicts: &[Option<String>],
    world_name: &str,
    resolved: &ResolvedAst,
    file: usize,
    sink: &mut DiagnosticSink,
) {
    match stmt {
        HStmt::Set { value, .. } => walk_expr(value, verdicts, world_name, resolved, file, sink),
        HStmt::Return { value } => {
            if let Some(value) = value {
                walk_expr(value, verdicts, world_name, resolved, file, sink);
            }
        }
        HStmt::Expr(expr) => walk_expr(expr, verdicts, world_name, resolved, file, sink),
        HStmt::If { cond, then, els } => {
            walk_expr(cond, verdicts, world_name, resolved, file, sink);
            for s in then.iter().chain(els) {
                walk_stmt(s, verdicts, world_name, resolved, file, sink);
            }
        }
    }
}

fn walk_expr(
    expr: &HExpr,
    verdicts: &[Option<String>],
    world_name: &str,
    resolved: &ResolvedAst,
    file: usize,
    sink: &mut DiagnosticSink,
) {
    match &expr.kind {
        HExprKind::CallHost { import, args } => {
            if let Some(qualified) = &verdicts[*import] {
                com012(sink, qualified, world_name, resolved, file, expr.span);
            }
            for arg in args {
                walk_expr(arg, verdicts, world_name, resolved, file, sink);
            }
        }
        HExprKind::CallFn { args, .. } => {
            for arg in args {
                walk_expr(arg, verdicts, world_name, resolved, file, sink);
            }
        }
        HExprKind::MakeRecord(items) | HExprKind::MakeList(items) => {
            for item in items {
                walk_expr(item, verdicts, world_name, resolved, file, sink);
            }
        }
        HExprKind::Binary { lhs, rhs, .. } => {
            walk_expr(lhs, verdicts, world_name, resolved, file, sink);
            walk_expr(rhs, verdicts, world_name, resolved, file, sink);
        }
        HExprKind::Unary { operand, .. } => {
            walk_expr(operand, verdicts, world_name, resolved, file, sink)
        }
        _ => {}
    }
}

/// COM012 — message template from Platform 10 §11, verbatim:
/// `"your program uses '{interface}' but you compiled for '{world}', which
/// does not provide it"`.
fn com012(
    sink: &mut DiagnosticSink,
    qualified: &str,
    world_name: &str,
    resolved: &ResolvedAst,
    file: usize,
    span: ByteSpan,
) {
    sink.push(build(
        Level::Error,
        codes::COM012,
        format!(
            "your program uses '{qualified}' but you compiled for '{world_name}', which does not provide it"
        ),
        resolved.span(file, span),
        Some("not in the target world".to_string()),
    ));
}
