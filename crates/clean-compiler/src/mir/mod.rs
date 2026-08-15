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

/// Linear-memory layout (KNOWLEDGE.md trap #1 from the retired compiler:
/// the heap pointer is placed *after* static data, never before).
pub const DATA_OFFSET: u32 = 8;

pub struct MirProgram {
    pub imports: Vec<MirImport>,
    pub functions: Vec<MirFunction>,
    /// Static data blob, placed at `DATA_OFFSET` in linear memory: interned
    /// string literals and compile-time-constant aggregates, deduplicated
    /// in first-use order (deterministic, §14.5).
    pub data: Vec<u8>,
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

/// How a host result wider than one core value comes back: through a
/// caller-provided return area (Canonical ABI retptr form). The layouts are
/// the canonical in-memory representations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetLift {
    /// `string` / `list<u8>`: ptr at +0, len at +4.
    PtrLen,
    /// `option<string>`: discriminant byte at +0, ptr at +4, len at +8.
    OptionPtrLen,
}

/// The retptr classification for a host-function return type, shared by
/// import lowering and call-site lowering so the two can never disagree.
pub fn ret_lift(ty: &Ty) -> Option<RetLift> {
    match ty {
        Ty::Str | Ty::Bytes => Some(RetLift::PtrLen),
        Ty::Option(inner) if matches!(**inner, Ty::Str | Ty::Bytes) => Some(RetLift::OptionPtrLen),
        _ => None,
    }
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
    /// Pushes the address of the fixed return area (resolved at emission,
    /// after the static data size is final).
    RetAreaPtr,
    /// i32 load at constant offset from the popped address.
    I32Load(u32),
    /// Zero-extending byte load at constant offset from the popped address.
    I32Load8U(u32),
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

/// Flattened core-type shape of a semantic type, per the Canonical ABI
/// flattening: `string`/`bytes` are `(ptr, len)`, records concatenate their
/// fields, options prepend an `i32` discriminant. `None` marks a type with
/// no lowering (only `Error`, which never survives a clean typecheck).
pub fn val_types(ty: &Ty) -> Option<Vec<Val>> {
    Some(match ty {
        Ty::Void => vec![],
        Ty::Integer | Ty::IntegerW(IntWidth::U64) => vec![Val::I64],
        Ty::IntegerW(_) | Ty::Boolean | Ty::Enum { .. } => vec![Val::I32],
        Ty::Str | Ty::Bytes | Ty::List(_) => vec![Val::I32, Val::I32],
        Ty::Record { fields, .. } => {
            let mut out = Vec::new();
            for (_, field_ty) in fields {
                out.extend(val_types(field_ty)?);
            }
            out
        }
        Ty::Option(inner) => {
            let mut out = vec![Val::I32];
            out.extend(val_types(inner)?);
            out
        }
        Ty::Error => return None,
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
    let mut data = DataPool::default();
    let functions = program
        .functions
        .iter()
        .map(|f| lower_function(f, program, resolved, &mut data, sink))
        .collect();
    MirProgram {
        imports,
        functions,
        data: data.blob,
    }
}

/// Interns static byte runs into one deduplicated blob. Offsets are final
/// linear-memory addresses (blob starts at `DATA_OFFSET`).
#[derive(Default)]
struct DataPool {
    blob: Vec<u8>,
    seen: std::collections::BTreeMap<Vec<u8>, u32>,
}

impl DataPool {
    fn intern(&mut self, bytes: &[u8]) -> u32 {
        if let Some(&offset) = self.seen.get(bytes) {
            return offset;
        }
        let offset = DATA_OFFSET + self.blob.len() as u32;
        self.blob.extend_from_slice(bytes);
        // Keep every run 4-aligned so aggregate layouts stay canonical.
        while !self.blob.len().is_multiple_of(4) {
            self.blob.push(0);
        }
        self.seen.insert(bytes.to_vec(), offset);
        offset
    }
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
        Some(vals) if vals.len() <= 1 => vals,
        // Wider results take the Canonical ABI retptr form: a trailing i32
        // pointer parameter, no core results.
        _ => match ret_lift(&import.ret) {
            Some(_) => {
                params.push(Val::I32);
                vec![]
            }
            None => {
                note_type_gap(&import.ret, import, resolved, sink);
                vec![]
            }
        },
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
    data: &mut DataPool,
    sink: &mut DiagnosticSink,
) -> MirFunction {
    // LocalId → first wasm slot; flattened types occupy consecutive slots.
    let mut slots = Vec::new();
    let mut slot_widths = Vec::new();
    let mut params = Vec::new();
    let mut locals = Vec::new();
    let mut next: u32 = 0;
    for ty in &function.params {
        slots.push(next);
        let vals = val_types(ty).unwrap_or_else(|| vec![Val::I32]);
        slot_widths.push(vals.len() as u32);
        next += vals.len() as u32;
        params.extend(vals);
    }
    for ty in &function.locals {
        slots.push(next);
        let vals = val_types(ty).unwrap_or_else(|| vec![Val::I32]);
        slot_widths.push(vals.len() as u32);
        next += vals.len() as u32;
        locals.extend(vals);
    }
    let results = val_types(&function.ret).unwrap_or_default();

    let mut lowerer = FnLowerer {
        program,
        resolved,
        slots,
        slot_widths,
        file: function.file,
        data,
        next_slot: next,
        scratch: Vec::new(),
    };
    let mut body = Vec::new();
    for stmt in &function.body {
        lowerer.stmt(stmt, &mut body, sink);
    }
    locals.extend(lowerer.scratch);
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
    /// LocalId → number of consecutive slots the value occupies.
    slot_widths: Vec<u32>,
    file: usize,
    data: &'a mut DataPool,
    /// Next free wasm slot (scratch allocations continue the local space).
    next_slot: u32,
    /// Scratch locals appended after the declared locals.
    scratch: Vec<Val>,
}

impl<'a> FnLowerer<'a> {
    /// Serializes a list of compile-time-constant elements into the
    /// canonical contiguous layout. Supported constant shapes in M1:
    /// strings and records whose fields are themselves constant.
    fn serialize_const_list(&mut self, items: &[HExpr]) -> Option<Vec<u8>> {
        let mut blob = Vec::new();
        for item in items {
            self.serialize_const(item, &mut blob)?;
        }
        Some(blob)
    }

    fn serialize_const(&mut self, expr: &HExpr, blob: &mut Vec<u8>) -> Option<()> {
        match &expr.kind {
            HExprKind::Str(value) => {
                let ptr = self.data.intern(value.as_bytes());
                blob.extend_from_slice(&ptr.to_le_bytes());
                blob.extend_from_slice(&(value.len() as u32).to_le_bytes());
                Some(())
            }
            HExprKind::MakeRecord(fields) => {
                for field in fields {
                    self.serialize_const(field, blob)?;
                }
                Some(())
            }
            _ => None,
        }
    }

    /// Reserves consecutive scratch slots and returns the base slot index.
    fn alloc_scratch(&mut self, vals: &[Val]) -> u32 {
        let base = self.next_slot;
        self.next_slot += vals.len() as u32;
        self.scratch.extend_from_slice(vals);
        base
    }

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
                // Flattened values sit on the stack in slot order; stores
                // pop in reverse.
                let base = self.slots[*local];
                for offset in (0..self.slot_widths[*local]).rev() {
                    out.push(Inst::LocalSet(base + offset));
                }
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
            HExprKind::Local(id) => {
                let base = self.slots[*id];
                for offset in 0..self.slot_widths[*id] {
                    out.push(Inst::LocalGet(base + offset));
                }
            }
            HExprKind::Str(value) => {
                let offset = self.data.intern(value.as_bytes());
                out.push(Inst::I32Const(offset as i32));
                out.push(Inst::I32Const(value.len() as i32));
            }
            HExprKind::NoneLit => {
                // Discriminant 0 plus zeroed payload slots.
                let width = val_types(&expr.ty).map(|v| v.len()).unwrap_or(1);
                let payload = val_types(match &expr.ty {
                    Ty::Option(inner) => inner,
                    _ => &Ty::Boolean,
                })
                .unwrap_or_default();
                debug_assert_eq!(width, payload.len() + 1);
                out.push(Inst::I32Const(0));
                for slot in payload {
                    out.push(match slot {
                        Val::I32 => Inst::I32Const(0),
                        Val::I64 => Inst::I64Const(0),
                    });
                }
            }
            HExprKind::MakeRecord(fields) => {
                // Canonical flattening: fields in declaration order.
                for field in fields {
                    self.expr(field, out, sink);
                }
            }
            HExprKind::MakeList(items) => {
                // Compile-time-constant lists serialize to static data in
                // the canonical element layout; runtime list construction
                // needs the allocator story of M6.
                match self.serialize_const_list(items) {
                    Some(blob) => {
                        let ptr = if blob.is_empty() {
                            DATA_OFFSET
                        } else {
                            self.data.intern(&blob)
                        };
                        out.push(Inst::I32Const(ptr as i32));
                        out.push(Inst::I32Const(items.len() as i32));
                    }
                    None => self.note(sink, "runtime-constructed list values", expr.span),
                }
            }
            HExprKind::CallHost { import, args } => {
                let param_tys = self.program.host_imports[*import].params.clone();
                for (arg, param_ty) in args.iter().zip(&param_tys) {
                    self.expr(arg, out, sink);
                    self.boundary_convert(&arg.ty, param_ty, out);
                }
                let ret = &self.program.host_imports[*import].ret;
                let needs_retptr = val_types(ret).map(|v| v.len() > 1).unwrap_or(false);
                if needs_retptr {
                    out.push(Inst::RetAreaPtr);
                }
                out.push(Inst::CallImport(*import as u32));
                if needs_retptr {
                    // Lift the canonical in-memory result back onto the
                    // stack in flattened slot order.
                    match ret_lift(ret) {
                        Some(RetLift::PtrLen) => {
                            out.push(Inst::RetAreaPtr);
                            out.push(Inst::I32Load(0));
                            out.push(Inst::RetAreaPtr);
                            out.push(Inst::I32Load(4));
                        }
                        Some(RetLift::OptionPtrLen) => {
                            out.push(Inst::RetAreaPtr);
                            out.push(Inst::I32Load8U(0));
                            out.push(Inst::RetAreaPtr);
                            out.push(Inst::I32Load(4));
                            out.push(Inst::RetAreaPtr);
                            out.push(Inst::I32Load(8));
                        }
                        None => self.note(sink, "this host result type", expr.span),
                    }
                }
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
            Default => {
                // `opt default fb`: stack after lhs is [disc, payload…].
                // Park the payload in scratch, branch on the discriminant,
                // and refill the scratch from the fallback when none.
                let payload_ty = match &lhs.ty {
                    Ty::Option(inner) => (**inner).clone(),
                    _ => rhs.ty.clone(),
                };
                let payload_vals = val_types(&payload_ty).unwrap_or_default();
                let base = self.alloc_scratch(&payload_vals);
                self.expr(lhs, out, sink);
                for offset in (0..payload_vals.len() as u32).rev() {
                    out.push(Inst::LocalSet(base + offset));
                }
                let mut els = Vec::new();
                self.expr(rhs, &mut els, sink);
                for offset in (0..payload_vals.len() as u32).rev() {
                    els.push(Inst::LocalSet(base + offset));
                }
                out.push(Inst::If {
                    result: None,
                    then: Vec::new(),
                    els,
                });
                for offset in 0..payload_vals.len() as u32 {
                    out.push(Inst::LocalGet(base + offset));
                }
            }
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
