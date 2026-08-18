//! Pass [8] — MIR Lowering (Platform 14 §14.4.2): a linear, wasm-shaped IR
//! for direct emission. Control flow stays structured (wasm is structured)
//! and no optimizations run — `debug` and `release` emit the same code
//! until the conformance suite exists to prove semantics preservation
//! (§14.4.2[8]).
//!
//! Value representation: surface `integer` is `i64`; width-suffixed
//! boundary integers ≤32 bits, `boolean`, and enum discriminants are
//! `i32`; `integer:u64` is `i64`. A `string`/`bytes` value is a single
//! `i32` pointer to its `[u32 LE length][payload]` object (MMD-04: the
//! address of a string is the address of the length field). The **internal
//! representation** ([`val_types`]) and the **Canonical ABI boundary
//! flattening** ([`cabi_flat`]) are distinct vocabularies: a boundary
//! string is `(ptr, len)` with `ptr` pointing at the payload
//! (`base + 4`); call-site lowering converts between the two.

use crate::diag::DiagnosticSink;
use crate::hir::{HExpr, HExprKind, HFunction, HIterSource, HStmt, HirProgram};
use crate::layout::{Tier, DATA_SECTION_START, EMPTY_STRING_ADDR};
use crate::parser::ast::{BinOp, IntWidth, UnOp};
use crate::resolver::ResolvedAst;
use crate::typecheck::tir::HostImport;
use crate::typecheck::types::Ty;

pub mod runtime;

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
    /// Always-emitted runtime helpers (ADR 0004), appended after the user
    /// functions at emission; [`Inst::CallRuntime`] resolves into this
    /// vector by [`runtime::RuntimeFn`] discriminant order.
    pub runtime: Vec<MirFunction>,
    /// Static data blob, placed at `layout::DATA_SECTION_START` in linear
    /// memory. Its first 4 bytes are the shared empty-string constant
    /// (MMD-01); after that, interned string objects and
    /// compile-time-constant aggregates, deduplicated in first-use order
    /// (deterministic, §14.5).
    pub data: Vec<u8>,
    /// The resolved memory tier (TIER-01), fixed at build time; pass [10]
    /// derives the memory's minimum/maximum from it and the allocator
    /// embeds its byte limit.
    pub tier: Tier,
}

/// A host import with its core-level signature already flattened.
pub struct MirImport {
    /// Interface-qualified module name, e.g. `clean:host/routing@0.1.0`
    /// (Platform 15 P2: one naming scheme, no flat namespace).
    pub module: String,
    /// Bare kebab-case interface name (`routing`), for guest-world synthesis.
    pub interface: String,
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
    LocalTee(u32),
    CallImport(u32),
    Call(u32),
    /// Call into an always-emitted runtime helper (ADR 0004); resolved to a
    /// concrete function index at emission.
    CallRuntime(runtime::RuntimeFn),
    I64Bin(I64Op),
    I32Bin(I32Op),
    /// i64 comparison producing an i32 boolean.
    I64Cmp(CmpOp),
    /// i32 comparison producing an i32 boolean.
    I32Cmp(CmpOp),
    I32Eqz,
    I32WrapI64,
    I64ExtendI32U,
    /// `select(a, b, cond) -> cond ? a : b`.
    Select,
    /// Pushes the address of the fixed return area (resolved at emission,
    /// after the static data size is final).
    RetAreaPtr,
    /// i32 load at constant offset from the popped address.
    I32Load(u32),
    /// Zero-extending byte load at constant offset from the popped address.
    I32Load8U(u32),
    /// i32 store at constant offset: pops value, then address.
    I32Store(u32),
    /// Byte store at constant offset: pops value, then address.
    I32Store8(u32),
    /// Current memory size in pages.
    MemorySize,
    /// Grow by the popped page count; pushes the old size or -1.
    MemoryGrow,
    /// `memory.copy(dst, src, n)` (bulk memory).
    MemoryCopy,
    GlobalGet(u32),
    GlobalSet(u32),
    Unreachable,
    /// Structured conditional; `result` is the value it leaves on the stack.
    If {
        result: Option<Val>,
        then: Vec<Inst>,
        els: Vec<Inst>,
    },
    /// Structured block: `Br(depth)` targeting it jumps past its end.
    Block {
        body: Vec<Inst>,
    },
    /// Structured loop: `Br(depth)` targeting it jumps back to its start.
    Loop {
        body: Vec<Inst>,
    },
    Br(u32),
    BrIf(u32),
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
pub enum I32Op {
    Add,
    Sub,
    Mul,
    DivU,
    And,
    Or,
    Xor,
    Shl,
    ShrU,
}

#[derive(Debug, Clone, Copy)]
pub enum CmpOp {
    Eq,
    Ne,
    LtS,
    LeS,
    GtS,
    GeS,
    LtU,
    GtU,
}

/// **Internal** core-type shape of a semantic type — how a value lives in
/// locals and on the operand stack. `string`/`bytes` are a single `i32`
/// pointer to the `[u32 length][payload]` object (MMD-04); records
/// concatenate their fields; options prepend an `i32` discriminant.
/// `None` marks a type with no core lowering yet — `number`, `datetime`,
/// `any`, `matrix` and `pairs` land with later M6 stages; `Error` never
/// survives a clean typecheck, and `Var` never leaves pass [5].
///
/// Lists are transitionally `(ptr, count)` pointing at a Canonical-ABI-
/// shaped constant element array; the MMD §3.4.1 header layout arrives
/// with runtime list construction.
pub fn val_types(ty: &Ty) -> Option<Vec<Val>> {
    Some(match ty {
        Ty::Void => vec![],
        Ty::Integer | Ty::IntegerW(IntWidth::U64) => vec![Val::I64],
        Ty::IntegerW(_) | Ty::Boolean | Ty::Enum { .. } => vec![Val::I32],
        Ty::Str | Ty::Bytes => vec![Val::I32],
        Ty::List(_, _) => vec![Val::I32, Val::I32],
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
        Ty::Number | Ty::Datetime | Ty::Any | Ty::Matrix(_) | Ty::Pairs(_, _) => return None,
        // Nominal class instances and capability-typed values get their
        // memory representation with the M6 memory model; only the
        // structural `Record` boundary projection lowers today.
        Ty::Class { .. } | Ty::Cap { .. } => return None,
        Ty::Var(_) | Ty::Error => return None,
    })
}

/// **Boundary** flattening of a semantic type, per the Canonical ABI:
/// `string`/`bytes`/`list` flatten to `(ptr, len)`, records concatenate
/// their flattened fields, options prepend an `i32` discriminant. Import
/// signatures and retptr classification use this vocabulary, never
/// [`val_types`].
pub fn cabi_flat(ty: &Ty) -> Option<Vec<Val>> {
    Some(match ty {
        Ty::Str | Ty::Bytes | Ty::List(_, _) => vec![Val::I32, Val::I32],
        Ty::Record { fields, .. } => {
            let mut out = Vec::new();
            for (_, field_ty) in fields {
                out.extend(cabi_flat(field_ty)?);
            }
            out
        }
        Ty::Option(inner) => {
            let mut out = vec![Val::I32];
            out.extend(cabi_flat(inner)?);
            out
        }
        _ => return val_types(ty),
    })
}

fn is_i64(ty: &Ty) -> bool {
    matches!(ty, Ty::Integer | Ty::IntegerW(IntWidth::U64))
}

pub fn lower(
    program: &HirProgram,
    resolved: &ResolvedAst,
    world_version: &str,
    tier: Tier,
    sink: &mut DiagnosticSink,
) -> MirProgram {
    // Emit only imports that are actually called: an unused declaration
    // must not appear in the component's import list (the world check is
    // call-site-scoped, and hosts refuse imports they do not provide).
    let mut used: Vec<usize> = Vec::new();
    for function in &program.functions {
        for stmt in &function.body {
            collect_used_imports(stmt, &mut used);
        }
    }
    used.sort_unstable();
    used.dedup();
    let remap: std::collections::BTreeMap<usize, usize> = used
        .iter()
        .enumerate()
        .map(|(new, &old)| (old, new))
        .collect();

    let imports = used
        .iter()
        .map(|&index| lower_import(&program.host_imports[index], world_version, resolved, sink))
        .collect();
    let mut data = DataPool::new();
    let functions = program
        .functions
        .iter()
        .map(|f| lower_function(f, program, resolved, &remap, &mut data, sink))
        .collect();
    MirProgram {
        imports,
        functions,
        runtime: runtime::build(tier),
        data: data.blob,
        tier,
    }
}

fn collect_used_imports(stmt: &HStmt, used: &mut Vec<usize>) {
    fn expr(e: &HExpr, used: &mut Vec<usize>) {
        match &e.kind {
            HExprKind::CallHost { import, args } => {
                used.push(*import);
                args.iter().for_each(|a| expr(a, used));
            }
            HExprKind::CallFn { args, .. } => args.iter().for_each(|a| expr(a, used)),
            HExprKind::MakeRecord(items)
            | HExprKind::MakeList(items)
            | HExprKind::MakeMatrix(items) => items.iter().for_each(|i| expr(i, used)),
            HExprKind::Binary { lhs, rhs, .. } => {
                expr(lhs, used);
                expr(rhs, used);
            }
            HExprKind::Unary { operand, .. }
            | HExprKind::NonNone(operand)
            | HExprKind::IsNone { operand, .. }
            | HExprKind::IntToNumber(operand)
            | HExprKind::WrapSome(operand)
            | HExprKind::Convert(operand)
            | HExprKind::GetField { recv: operand, .. } => expr(operand, used),
            HExprKind::CallMethod { recv, args, .. } | HExprKind::CallDyn { recv, args, .. } => {
                expr(recv, used);
                args.iter().for_each(|a| expr(a, used));
            }
            HExprKind::CallStatic { args, .. } | HExprKind::CallCtor { args, .. } => {
                args.iter().for_each(|a| expr(a, used))
            }
            HExprKind::Index { recv, index, .. } => {
                expr(recv, used);
                expr(index, used);
            }
            HExprKind::StrInterp(segs) => {
                for seg in segs {
                    if let crate::hir::HInterpSeg::Expr(e) = seg {
                        expr(e, used);
                    }
                }
            }
            _ => {}
        }
    }
    match stmt {
        HStmt::Set { value, .. } => expr(value, used),
        HStmt::Return { value } => {
            if let Some(value) = value {
                expr(value, used);
            }
        }
        HStmt::Expr(e) => expr(e, used),
        HStmt::If { cond, then, els } => {
            expr(cond, used);
            then.iter()
                .chain(els)
                .for_each(|s| collect_used_imports(s, used));
        }
        HStmt::While { cond, body } => {
            expr(cond, used);
            body.iter().for_each(|s| collect_used_imports(s, used));
        }
        HStmt::Iterate {
            source, step, body, ..
        } => {
            match source {
                HIterSource::List(e) | HIterSource::Chars(e) | HIterSource::Rows(e) => {
                    expr(e, used)
                }
                HIterSource::Range { from, to } => {
                    expr(from, used);
                    expr(to, used);
                }
            }
            if let Some(step) = step {
                expr(step, used);
            }
            body.iter().for_each(|s| collect_used_imports(s, used));
        }
        HStmt::Break { .. } | HStmt::Continue { .. } => {}
        HStmt::Print { items, .. } => items.iter().for_each(|e| expr(e, used)),
        HStmt::Assert { cond, .. } => expr(cond, used),
    }
}

/// Interns static byte runs into one deduplicated blob. Offsets are final
/// linear-memory addresses (blob starts at `layout::DATA_SECTION_START`).
/// The pool is seeded with the 4-byte empty-string constant so every empty
/// string shares `EMPTY_STRING_ADDR` (MMD-01).
struct DataPool {
    blob: Vec<u8>,
    seen: std::collections::BTreeMap<Vec<u8>, u32>,
}

impl DataPool {
    fn new() -> Self {
        let mut pool = DataPool {
            blob: Vec::new(),
            seen: std::collections::BTreeMap::new(),
        };
        let empty = pool.intern(&0u32.to_le_bytes());
        debug_assert_eq!(empty, EMPTY_STRING_ADDR);
        pool
    }

    fn intern(&mut self, bytes: &[u8]) -> u32 {
        if let Some(&offset) = self.seen.get(bytes) {
            return offset;
        }
        let offset = DATA_SECTION_START + self.blob.len() as u32;
        self.blob.extend_from_slice(bytes);
        // Keep every run 4-aligned so aggregate layouts stay canonical.
        while !self.blob.len().is_multiple_of(4) {
            self.blob.push(0);
        }
        self.seen.insert(bytes.to_vec(), offset);
        offset
    }

    /// Interns a string as its in-memory object — `[u32 LE length]
    /// [payload]` — and returns the object's base address (MMD-04).
    fn intern_string(&mut self, value: &str) -> u32 {
        let mut object = Vec::with_capacity(4 + value.len());
        object.extend_from_slice(&(value.len() as u32).to_le_bytes());
        object.extend_from_slice(value.as_bytes());
        self.intern(&object)
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
        match cabi_flat(ty) {
            Some(vals) => params.extend(vals),
            None => note_type_gap(ty, import, resolved, sink),
        }
    }
    let results = match cabi_flat(&import.ret) {
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
        interface: import.interface.clone(),
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
    remap: &std::collections::BTreeMap<usize, usize>,
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
        remap,
        label_depth: 0,
        loops: Vec::new(),
    };
    if let Some(first) = function.before.first().or(function.after.first()) {
        sink.note_unsupported(
            "contract blocks in compiled code",
            resolved.span(function.file, first.span),
        );
    }
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
    /// Original host-import index → pruned MIR import index.
    remap: &'a std::collections::BTreeMap<usize, usize>,
    /// Structured-label bookkeeping for `break`/`continue`: the number of
    /// enclosing labeled blocks (`block`/`loop`/`if`) at the current
    /// emission site.
    label_depth: u32,
    /// Innermost-first loop contexts; branch distances derive from the
    /// stored absolute label indices (FLW-03: the loop-context stack is
    /// the one authority for loop control targets — KNOWLEDGE §13.2).
    loops: Vec<LoopCtx>,
}

/// Absolute label indices (count of enclosing labels *outside* the target)
/// for the two control points of one loop.
struct LoopCtx {
    /// The wrapping `block` — `break` lands after it.
    break_abs: u32,
    /// Where `continue` lands: the `loop` head for `while`, the inner
    /// body `block` end for `iterate` (so the step still applies —
    /// FLW-03).
    continue_abs: u32,
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
                // Constant aggregates serialize in the Canonical ABI
                // element layout (they only ever cross the boundary), so a
                // string element is (payload ptr, len) — the payload sits
                // at `base + 4` inside the interned string object.
                let base = self.data.intern_string(value);
                blob.extend_from_slice(&(base + 4).to_le_bytes());
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
                // An `if` is itself a labeled block; branch distances of any
                // break/continue inside must account for it.
                self.label_depth += 1;
                let mut then_body = Vec::new();
                for s in then {
                    self.stmt(s, &mut then_body, sink);
                }
                let mut else_body = Vec::new();
                for s in els {
                    self.stmt(s, &mut else_body, sink);
                }
                self.label_depth -= 1;
                out.push(Inst::If {
                    result: None,
                    then: then_body,
                    els: else_body,
                });
            }
            // FLW-02 `while`:  block { loop { !cond → br-out; body; br } }.
            HStmt::While { cond, body } => {
                let base = self.label_depth;
                self.loops.push(LoopCtx {
                    break_abs: base,
                    // `continue` re-tests the condition: the loop head.
                    continue_abs: base + 1,
                });
                self.label_depth += 2;
                let mut inner = Vec::new();
                self.expr(cond, &mut inner, sink);
                inner.push(Inst::I32Eqz);
                inner.push(Inst::BrIf(1));
                for s in body {
                    self.stmt(s, &mut inner, sink);
                }
                inner.push(Inst::Br(0));
                self.label_depth -= 2;
                self.loops.pop();
                out.push(Inst::Block {
                    body: vec![Inst::Loop { body: inner }],
                });
            }
            // FLW-02 range `iterate`: endpoints inclusive, signed `step`
            // sets the direction; without one the direction follows the
            // endpoints (mirroring `list.range`, which descends when
            // start > to — local adoption, DISCOVERIES-M6). `from`, `to`
            // and `step` evaluate once, before the first test. The body
            // sits in an inner block so `continue` still applies the step
            // (FLW-03).
            HStmt::Iterate {
                binder,
                source: HIterSource::Range { from, to },
                step,
                body,
            } => {
                let i = self.slots[*binder];
                let to_s = self.alloc_scratch(&[Val::I64]);
                let s = self.alloc_scratch(&[Val::I64]);
                self.expr(from, out, sink);
                out.push(Inst::LocalSet(i));
                self.expr(to, out, sink);
                out.push(Inst::LocalSet(to_s));
                match step {
                    Some(e) => self.expr(e, out, sink),
                    None => {
                        out.push(Inst::LocalGet(i));
                        out.push(Inst::LocalGet(to_s));
                        out.push(Inst::I64Cmp(CmpOp::LeS));
                        out.push(Inst::If {
                            result: Some(Val::I64),
                            then: vec![Inst::I64Const(1)],
                            els: vec![Inst::I64Const(-1)],
                        });
                    }
                }
                out.push(Inst::LocalSet(s));

                let base = self.label_depth;
                self.loops.push(LoopCtx {
                    break_abs: base,
                    // The inner body block: falling out of it runs the step.
                    continue_abs: base + 2,
                });
                self.label_depth += 3;
                let mut inner = Vec::new();
                for stmt in body {
                    self.stmt(stmt, &mut inner, sink);
                }
                self.label_depth -= 3;
                self.loops.pop();

                let mut loop_body = vec![
                    // done = (s >= 0 and i > to) or (s < 0 and i < to)
                    Inst::LocalGet(s),
                    Inst::I64Const(0),
                    Inst::I64Cmp(CmpOp::GeS),
                    Inst::LocalGet(i),
                    Inst::LocalGet(to_s),
                    Inst::I64Cmp(CmpOp::GtS),
                    Inst::I32Bin(I32Op::And),
                    Inst::LocalGet(s),
                    Inst::I64Const(0),
                    Inst::I64Cmp(CmpOp::LtS),
                    Inst::LocalGet(i),
                    Inst::LocalGet(to_s),
                    Inst::I64Cmp(CmpOp::LtS),
                    Inst::I32Bin(I32Op::And),
                    Inst::I32Bin(I32Op::Or),
                    Inst::BrIf(1),
                    Inst::Block { body: inner },
                    Inst::LocalGet(i),
                    Inst::LocalGet(s),
                    Inst::I64Bin(I64Op::Add),
                    Inst::LocalSet(i),
                ];
                loop_body.push(Inst::Br(0));
                out.push(Inst::Block {
                    body: vec![Inst::Loop { body: loop_body }],
                });
            }
            HStmt::Iterate { source, .. } => {
                let (construct, span): (&'static str, _) = match source {
                    HIterSource::List(e) => ("iterate over list values", e.span),
                    HIterSource::Chars(e) => ("iterate over string characters", e.span),
                    HIterSource::Rows(e) => ("iterate over matrix rows", e.span),
                    HIterSource::Range { from, .. } => {
                        unreachable!("range iterate lowered above: {:?}", from.span)
                    }
                };
                self.note(sink, construct, span);
            }
            HStmt::Break { span } => match self.loops.last() {
                Some(ctx) => out.push(Inst::Br(self.label_depth - 1 - ctx.break_abs)),
                // SEM025 rejects orphans in pass [5]; reaching here is a
                // compiler bug, kept loud.
                None => unreachable!("break outside a loop survived typecheck at {span:?}"),
            },
            HStmt::Continue { span } => match self.loops.last() {
                Some(ctx) => out.push(Inst::Br(self.label_depth - 1 - ctx.continue_abs)),
                None => unreachable!("continue outside a loop survived typecheck at {span:?}"),
            },
            HStmt::Print { span, .. } => self.note(sink, "print: blocks", *span),
            HStmt::Assert { span, .. } => self.note(sink, "assert statements", *span),
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
                let base = self.data.intern_string(value);
                out.push(Inst::I32Const(base as i32));
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
                            DATA_SECTION_START
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
                    self.lower_boundary_arg(arg, param_ty, out, sink);
                }
                let ret = &self.program.host_imports[*import].ret;
                let needs_retptr = cabi_flat(ret).map(|v| v.len() > 1).unwrap_or(false);
                if needs_retptr {
                    out.push(Inst::RetAreaPtr);
                }
                out.push(Inst::CallImport(self.remap[import] as u32));
                if needs_retptr {
                    // Lift the canonical in-memory result into the internal
                    // representation. Host-written payloads are copied into
                    // fresh `[len][payload]` objects (§3.7: values crossing
                    // the boundary are copies).
                    match ret_lift(ret) {
                        Some(RetLift::PtrLen) => {
                            out.push(Inst::RetAreaPtr);
                            out.push(Inst::I32Load(0));
                            out.push(Inst::RetAreaPtr);
                            out.push(Inst::I32Load(4));
                            out.push(Inst::CallRuntime(runtime::RuntimeFn::LiftString));
                        }
                        Some(RetLift::OptionPtrLen) => {
                            // [disc] twice: once as the option's first flat
                            // slot, once consumed by the payload branch.
                            let disc = self.alloc_scratch(&[Val::I32]);
                            out.push(Inst::RetAreaPtr);
                            out.push(Inst::I32Load8U(0));
                            out.push(Inst::LocalTee(disc));
                            out.push(Inst::LocalGet(disc));
                            out.push(Inst::If {
                                result: Some(Val::I32),
                                then: vec![
                                    Inst::RetAreaPtr,
                                    Inst::I32Load(4),
                                    Inst::RetAreaPtr,
                                    Inst::I32Load(8),
                                    Inst::CallRuntime(runtime::RuntimeFn::LiftString),
                                ],
                                els: vec![Inst::I32Const(0)],
                            });
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
            // TYP-03: `T` into `T?` — discriminant 1, then the payload
            // (val_types puts the discriminant first).
            HExprKind::WrapSome(operand) => {
                out.push(Inst::I32Const(1));
                self.expr(operand, out, sink);
            }
            // The M4 frontier: typed, not yet lowerable to core wasm.
            HExprKind::Num(_) => self.note(sink, "number values in compiled code", expr.span),
            HExprKind::StrInterp(_) => self.note(sink, "string interpolation", expr.span),
            HExprKind::MakeMatrix(_) => self.note(sink, "matrix values", expr.span),
            HExprKind::Index { .. } => self.note(sink, "index access", expr.span),
            HExprKind::NonNone(_) => self.note(sink, "postfix `!` assertion", expr.span),
            HExprKind::IsNone { .. } => self.note(sink, "is-none checks", expr.span),
            HExprKind::IntToNumber(_) => {
                self.note(sink, "number values in compiled code", expr.span)
            }
            HExprKind::ResultRef => self.note(sink, "contract blocks in compiled code", expr.span),
            HExprKind::This => self.note(sink, "class values in compiled code", expr.span),
            HExprKind::GetState { .. } | HExprKind::GuardValue => {
                self.note(sink, "state access in compiled code", expr.span)
            }
            HExprKind::Raise(_) | HExprKind::OnError { .. } | HExprKind::ErrorBinding => {
                self.note(sink, "error handling in compiled code", expr.span)
            }
            HExprKind::GetRecordField { .. } => {
                self.note(sink, "record field access in compiled code", expr.span)
            }
            HExprKind::CallMethod { .. }
            | HExprKind::CallDyn { .. }
            | HExprKind::CallCtor { .. }
            | HExprKind::CallStatic { .. }
            | HExprKind::GetField { .. } => {
                self.note(sink, "class values in compiled code", expr.span)
            }
            HExprKind::Convert(_) => {
                self.note(sink, "conversion methods in compiled code", expr.span)
            }
        }
    }

    /// Lowers one host-call argument into its Canonical ABI boundary
    /// flattening (`cabi_flat` order). Pointer-shaped values convert from
    /// the internal single-pointer representation; scalars only adjust
    /// width.
    fn lower_boundary_arg(
        &mut self,
        arg: &HExpr,
        param_ty: &Ty,
        out: &mut Vec<Inst>,
        sink: &mut DiagnosticSink,
    ) {
        match param_ty {
            Ty::Str | Ty::Bytes => {
                // base → (payload ptr, len): payload at base + 4, length at
                // base (MMD-04).
                let base = self.alloc_scratch(&[Val::I32]);
                self.expr(arg, out, sink);
                out.push(Inst::LocalTee(base));
                out.push(Inst::I32Const(4));
                out.push(Inst::I32Bin(I32Op::Add));
                out.push(Inst::LocalGet(base));
                out.push(Inst::I32Load(0));
            }
            Ty::Option(inner) if matches!(**inner, Ty::Str | Ty::Bytes) => {
                // [disc, base] → [disc, ptr, len]; the payload slots must
                // not be derived from a none's null base (loads below the
                // null guard are forbidden), so branch.
                let disc = self.alloc_scratch(&[Val::I32]);
                let base = self.alloc_scratch(&[Val::I32]);
                let ptr = self.alloc_scratch(&[Val::I32]);
                let len = self.alloc_scratch(&[Val::I32]);
                self.expr(arg, out, sink);
                out.push(Inst::LocalSet(base));
                out.push(Inst::LocalSet(disc));
                out.push(Inst::LocalGet(disc));
                out.push(Inst::If {
                    result: None,
                    then: vec![
                        Inst::LocalGet(base),
                        Inst::I32Const(4),
                        Inst::I32Bin(I32Op::Add),
                        Inst::LocalSet(ptr),
                        Inst::LocalGet(base),
                        Inst::I32Load(0),
                        Inst::LocalSet(len),
                    ],
                    els: vec![
                        Inst::I32Const(0),
                        Inst::LocalSet(ptr),
                        Inst::I32Const(0),
                        Inst::LocalSet(len),
                    ],
                });
                out.push(Inst::LocalGet(disc));
                out.push(Inst::LocalGet(ptr));
                out.push(Inst::LocalGet(len));
            }
            // Record literals expand field by field; each field converts
            // through this same path.
            Ty::Record { fields, .. } if matches!(&arg.kind, HExprKind::MakeRecord(_)) => {
                let HExprKind::MakeRecord(items) = &arg.kind else {
                    unreachable!()
                };
                for (item, (_, field_ty)) in items.iter().zip(fields) {
                    self.lower_boundary_arg(item, field_ty, out, sink);
                }
            }
            Ty::Record { fields, .. }
                if fields
                    .iter()
                    .any(|(_, t)| val_types(t) != cabi_flat(t) || val_types(t).is_none()) =>
            {
                // A non-literal record whose internal and boundary shapes
                // differ needs a per-field spill; no current world signature
                // exercises this, so it stays a frontier.
                self.note(sink, "record values at the host boundary", arg.span);
            }
            _ => {
                self.expr(arg, out, sink);
                self.boundary_convert(&arg.ty, param_ty, out);
            }
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
        // Operand domains codegen cannot speak yet (M6: memory model and
        // stdlib): report and emit nothing.
        if matches!(expr.ty, Ty::Number)
            || matches!(lhs.ty, Ty::Number)
            || matches!(rhs.ty, Ty::Number)
        {
            self.note(sink, "number values in compiled code", expr.span);
            return;
        }
        if matches!(lhs.ty, Ty::Str) && matches!(op, Add | Eq | NEq | Lt | LtEq | Gt | GtEq) {
            self.expr(lhs, out, sink);
            self.expr(rhs, out, sink);
            match op {
                Add => out.push(Inst::CallRuntime(runtime::RuntimeFn::StringConcat)),
                // Both polarities derive from the single convention
                // "`string_eq` returns 1 iff equal" (ADR 0004; KNOWLEDGE §2:
                // hand-written polarity at call sites inverts silently).
                Eq => out.push(Inst::CallRuntime(runtime::RuntimeFn::StringEq)),
                NEq => {
                    out.push(Inst::CallRuntime(runtime::RuntimeFn::StringEq));
                    out.push(Inst::I32Eqz);
                }
                // Ordering compares against 0 in the `string_compare`
                // domain (0 iff equal, lexicographic sign otherwise).
                _ => {
                    out.push(Inst::CallRuntime(runtime::RuntimeFn::StringCompare));
                    out.push(Inst::I32Const(0));
                    out.push(Inst::I32Cmp(match op {
                        Lt => CmpOp::LtS,
                        LtEq => CmpOp::LeS,
                        Gt => CmpOp::GtS,
                        _ => CmpOp::GeS,
                    }));
                }
            }
            return;
        }
        if matches!(lhs.ty, Ty::Any | Ty::Matrix(_)) || matches!(rhs.ty, Ty::Any | Ty::Matrix(_)) {
            self.note(sink, "this operand type in compiled code", expr.span);
            return;
        }
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
            Is | NotIs => {
                self.note(sink, "identity comparison in compiled code", expr.span);
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
