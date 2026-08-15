//! Pass [8] — MIR Lowering (Platform 14 §14.4.2): a linear, wasm-shaped IR
//! for direct emission. Milestone 1 keeps control flow structured (wasm is
//! structured) and runs no optimizations — `debug` and `release` emit the
//! same code until the conformance suite exists to prove semantics
//! preservation (§14.4.2[8]).
//!
//! Value representation (M1 scalars): surface `integer` is `i64`;
//! width-suffixed boundary integers ≤32 bits, `boolean`, and enum
//! discriminants are `i32`; `integer:u64` is `i64`. Strings, bytes,
//! records, and options land with the Canonical ABI work (step 6) and are
//! reported as unsupported until then.

use crate::diag::DiagnosticSink;
use crate::hir::{HExpr, HExprKind, HFunction, HStmt, HirProgram};
use crate::parser::ast::{BinOp, IntWidth, UnOp};
use crate::resolver::ResolvedAst;
use crate::typecheck::tir::HostImport;
use crate::typecheck::types::Ty;

/// Core-wasm value types MIR speaks in (a subset of `wasm_encoder`'s,
/// owned here so MIR does not depend on the encoder).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Val {
    I32,
    I64,
}

pub struct MirProgram {
    pub imports: Vec<MirImport>,
    pub functions: Vec<MirFunction>,
}

/// A host import with its core-level signature already flattened.
pub struct MirImport {
    /// Interface-qualified module name, e.g. `clean:host/routing@0.1.0`
    /// (Platform 15 P2: one naming scheme, no flat namespace).
    pub module: String,
    /// Kebab-case function name within the interface.
    pub name: String,
    pub params: Vec<Val>,
    pub results: Vec<Val>,
}

pub struct MirFunction {
    pub name: String,
    pub params: Vec<Val>,
    pub results: Vec<Val>,
    /// Extra locals after the parameters.
    pub locals: Vec<Val>,
    pub body: Vec<Inst>,
    /// Exported entry point (ADR-0002 §1: `init` and `handle`).
    pub export: bool,
}

pub enum Inst {
    I32Const(i32),
    I64Const(i64),
    LocalGet(u32),
    LocalSet(u32),
    CallImport(u32),
    Call(u32),
    I64Bin(I64Op),
    /// i64 comparison producing an i32 boolean.
    I64Cmp(CmpOp),
    /// i32 comparison producing an i32 boolean.
    I32Cmp(CmpOp),
    I32Eqz,
    I32WrapI64,
    I64ExtendI32U,
    /// Structured conditional; `result` is the value it leaves on the stack.
    If {
        result: Option<Val>,
        then: Vec<Inst>,
        els: Vec<Inst>,
    },
    Return,
    Drop,
}

#[derive(Debug, Clone, Copy)]
pub enum I64Op {
    Add,
    Sub,
    Mul,
    DivS,
    RemS,
}

#[derive(Debug, Clone, Copy)]
pub enum CmpOp {
    Eq,
    Ne,
    LtS,
    LeS,
    GtS,
    GeS,
}

/// Flattened core-type shape of a semantic type. `None` marks a type whose
/// lowering is not implemented yet (step 6 surface).
pub fn val_types(ty: &Ty) -> Option<Vec<Val>> {
    Some(match ty {
        Ty::Void => vec![],
        Ty::Integer | Ty::IntegerW(IntWidth::U64) => vec![Val::I64],
        Ty::IntegerW(_) | Ty::Boolean | Ty::Enum { .. } => vec![Val::I32],
        Ty::Str | Ty::Bytes | Ty::Record { .. } | Ty::Option(_) | Ty::Error => return None,
    })
}

fn is_i64(ty: &Ty) -> bool {
    matches!(ty, Ty::Integer | Ty::IntegerW(IntWidth::U64))
}

pub fn lower(
    program: &HirProgram,
    resolved: &ResolvedAst,
    world_version: &str,
    sink: &mut DiagnosticSink,
) -> MirProgram {
    let imports = program
        .host_imports
        .iter()
        .map(|import| lower_import(import, world_version, resolved, sink))
        .collect();
    let functions = program
        .functions
        .iter()
        .map(|f| lower_function(f, program, resolved, sink))
        .collect();
    MirProgram { imports, functions }
}

fn lower_import(
    import: &HostImport,
    world_version: &str,
    resolved: &ResolvedAst,
    sink: &mut DiagnosticSink,
) -> MirImport {
    let mut params = Vec::new();
    for ty in &import.params {
        match val_types(ty) {
            Some(vals) => params.extend(vals),
            None => note_type_gap(ty, import, resolved, sink),
        }
    }
    let results = match val_types(&import.ret) {
        Some(vals) => vals,
        None => {
            note_type_gap(&import.ret, import, resolved, sink);
            vec![]
        }
    };
    MirImport {
        module: format!("clean:host/{}@{}", import.interface, world_version),
        name: import.wit_name.clone(),
        params,
        results,
    }
}

fn note_type_gap(ty: &Ty, import: &HostImport, resolved: &ResolvedAst, sink: &mut DiagnosticSink) {
    // The span of the host-function declaration; file 0 fallback is safe —
    // every declaration span originated in some parsed file, and the
    // Unsupported channel is pre-v1 reporting, not a diagnostic.
    let file = resolved
        .decls
        .host_functions
        .get(&import.clean_name)
        .map(|(slot, _)| resolved.decls.host_interfaces[*slot].0)
        .unwrap_or(0);
    let construct: &'static str = match ty {
        Ty::Str => "string values at the host boundary",
        Ty::Bytes => "bytes values at the host boundary",
        Ty::Record { .. } => "record values at the host boundary",
        Ty::Option(_) => "optional values at the host boundary",
        _ => "this type at the host boundary",
    };
    sink.note_unsupported(construct, resolved.span(file, import.span));
}

fn lower_function(
    function: &HFunction,
    program: &HirProgram,
    resolved: &ResolvedAst,
    sink: &mut DiagnosticSink,
) -> MirFunction {
    // LocalId → wasm slot. Scalars occupy one slot in M1; a type whose
    // lowering does not exist yet still consumes a placeholder slot so the
    // mapping stays total while the gap is reported.
    let mut slots = Vec::new();
    let mut params = Vec::new();
    let mut locals = Vec::new();
    let mut next: u32 = 0;
    for ty in &function.params {
        slots.push(next);
        let vals = val_types(ty).unwrap_or_else(|| vec![Val::I32]);
        next += vals.len() as u32;
        params.extend(vals);
    }
    for ty in &function.locals {
        slots.push(next);
        let vals = val_types(ty).unwrap_or_else(|| vec![Val::I32]);
        next += vals.len() as u32;
        locals.extend(vals);
    }
    let results = val_types(&function.ret).unwrap_or_default();

    let mut lowerer = FnLowerer {
        program,
        resolved,
        slots,
        file: function.file,
    };
    let mut body = Vec::new();
    for stmt in &function.body {
        lowerer.stmt(stmt, &mut body, sink);
    }
    MirFunction {
        name: function.name.clone(),
        params,
        results,
        locals,
        body,
        export: matches!(function.name.as_str(), "init" | "handle"),
    }
}

struct FnLowerer<'a> {
    program: &'a HirProgram,
    resolved: &'a ResolvedAst,
    /// LocalId → first wasm local slot.
    slots: Vec<u32>,
    file: usize,
}

impl<'a> FnLowerer<'a> {
    fn note(
        &self,
        sink: &mut DiagnosticSink,
        construct: &'static str,
        span: crate::source::ByteSpan,
    ) {
        sink.note_unsupported(construct, self.resolved.span(self.file, span));
    }

    fn stmt(&mut self, stmt: &HStmt, out: &mut Vec<Inst>, sink: &mut DiagnosticSink) {
        match stmt {
            HStmt::Set { local, value } => {
                self.expr(value, out, sink);
                out.push(Inst::LocalSet(self.slots[*local]));
            }
            HStmt::Return { value } => {
                if let Some(value) = value {
                    self.expr(value, out, sink);
                }
                out.push(Inst::Return);
            }
            HStmt::Expr(expr) => {
                self.expr(expr, out, sink);
                // Discard any produced value (statement position).
                if let Some(vals) = val_types(&expr.ty) {
                    for _ in vals {
                        out.push(Inst::Drop);
                    }
                }
            }
            HStmt::If { cond, then, els } => {
                self.expr(cond, out, sink);
                let mut then_body = Vec::new();
                for s in then {
                    self.stmt(s, &mut then_body, sink);
                }
                let mut else_body = Vec::new();
                for s in els {
                    self.stmt(s, &mut else_body, sink);
                }
                out.push(Inst::If {
                    result: None,
                    then: then_body,
                    els: else_body,
                });
            }
        }
    }

    fn expr(&mut self, expr: &HExpr, out: &mut Vec<Inst>, sink: &mut DiagnosticSink) {
        match &expr.kind {
            HExprKind::Int(v) => {
                if is_i64(&expr.ty) {
                    out.push(Inst::I64Const(*v as i64));
                } else {
                    out.push(Inst::I32Const(*v as i32));
                }
            }
            HExprKind::Bool(v) => out.push(Inst::I32Const(*v as i32)),
            HExprKind::EnumCase(i) => out.push(Inst::I32Const(*i as i32)),
            HExprKind::Local(id) => out.push(Inst::LocalGet(self.slots[*id])),
            HExprKind::Str(_) => self.note(sink, "string values in compiled code", expr.span),
            HExprKind::NoneLit => self.note(sink, "optional values in compiled code", expr.span),
            HExprKind::MakeRecord(_) => {
                self.note(sink, "record values in compiled code", expr.span)
            }
            HExprKind::CallHost { import, args } => {
                let param_tys = self.program.host_imports[*import].params.clone();
                for (arg, param_ty) in args.iter().zip(&param_tys) {
                    self.expr(arg, out, sink);
                    self.boundary_convert(&arg.ty, param_ty, out);
                }
                out.push(Inst::CallImport(*import as u32));
            }
            HExprKind::CallFn { func, args } => {
                for arg in args {
                    self.expr(arg, out, sink);
                }
                out.push(Inst::Call(*func as u32));
            }
            HExprKind::Binary { op, lhs, rhs } => self.binary(*op, lhs, rhs, expr, out, sink),
            HExprKind::Unary { op, operand } => match op {
                UnOp::Not => {
                    self.expr(operand, out, sink);
                    out.push(Inst::I32Eqz);
                }
                UnOp::Neg => {
                    out.push(Inst::I64Const(0));
                    self.expr(operand, out, sink);
                    out.push(Inst::I64Bin(I64Op::Sub));
                }
            },
        }
    }

    /// Converts a Clean value already on the stack to the boundary width the
    /// host parameter declares. The LBS-02 range check at the boundary is a
    /// step-6 concern, recorded in TESTING.md §7 until then.
    fn boundary_convert(&self, from: &Ty, to: &Ty, out: &mut Vec<Inst>) {
        match (is_i64(from), is_i64(to)) {
            (true, false) => out.push(Inst::I32WrapI64),
            (false, true) => out.push(Inst::I64ExtendI32U),
            _ => {}
        }
    }

    fn binary(
        &mut self,
        op: BinOp,
        lhs: &HExpr,
        rhs: &HExpr,
        expr: &HExpr,
        out: &mut Vec<Inst>,
        sink: &mut DiagnosticSink,
    ) {
        use BinOp::*;
        match op {
            And | Or => {
                // Short-circuit: `a and b` ⇒ if a { b } else { false };
                // `a or b` ⇒ if a { true } else { b }.
                self.expr(lhs, out, sink);
                let mut rhs_body = Vec::new();
                self.expr(rhs, &mut rhs_body, sink);
                let (then, els) = if op == And {
                    (rhs_body, vec![Inst::I32Const(0)])
                } else {
                    (vec![Inst::I32Const(1)], rhs_body)
                };
                out.push(Inst::If {
                    result: Some(Val::I32),
                    then,
                    els,
                });
            }
            Add | Sub | Mul | Div | Rem => {
                self.lower_arith_operand(lhs, out, sink);
                self.lower_arith_operand(rhs, out, sink);
                out.push(Inst::I64Bin(match op {
                    Add => I64Op::Add,
                    Sub => I64Op::Sub,
                    Mul => I64Op::Mul,
                    Div => I64Op::DivS,
                    _ => I64Op::RemS,
                }));
            }
            Lt | LtEq | Gt | GtEq | Eq | NEq => {
                let cmp = match op {
                    Eq => CmpOp::Eq,
                    NEq => CmpOp::Ne,
                    Lt => CmpOp::LtS,
                    LtEq => CmpOp::LeS,
                    Gt => CmpOp::GtS,
                    _ => CmpOp::GeS,
                };
                // Compare in the wider domain when either side is i64.
                let wide = is_i64(&lhs.ty) || is_i64(&rhs.ty);
                self.expr(lhs, out, sink);
                if wide && !is_i64(&lhs.ty) {
                    out.push(Inst::I64ExtendI32U);
                }
                self.expr(rhs, out, sink);
                if wide && !is_i64(&rhs.ty) {
                    out.push(Inst::I64ExtendI32U);
                }
                out.push(if wide {
                    Inst::I64Cmp(cmp)
                } else {
                    Inst::I32Cmp(cmp)
                });
            }
            Pow => {
                self.note(sink, "exponentiation in compiled code", expr.span);
            }
        }
    }

    /// Arithmetic runs in the i64 domain; narrower operands widen first.
    fn lower_arith_operand(
        &mut self,
        operand: &HExpr,
        out: &mut Vec<Inst>,
        sink: &mut DiagnosticSink,
    ) {
        self.expr(operand, out, sink);
        if !is_i64(&operand.ty) {
            out.push(Inst::I64ExtendI32U);
        }
    }
}
