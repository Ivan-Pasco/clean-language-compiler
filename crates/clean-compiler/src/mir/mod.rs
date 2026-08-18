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
use crate::typecheck::tir::{self, HostImport};
use crate::typecheck::types::Ty;

pub mod runtime;
pub mod runtime_list;
pub mod runtime_str;

/// Core-wasm value types MIR speaks in (a subset of `wasm_encoder`'s,
/// owned here so MIR does not depend on the encoder).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Val {
    I32,
    I64,
    F64,
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
    F64Const(f64),
    I64Bin(I64Op),
    I32Bin(I32Op),
    F64Bin(F64Op),
    /// Unary f64 instruction (the wasm-native `math` subset rides here).
    F64Un(F64Un),
    /// i64 comparison producing an i32 boolean.
    I64Cmp(CmpOp),
    /// i32 comparison producing an i32 boolean.
    I32Cmp(CmpOp),
    /// f64 comparison producing an i32 boolean (unsigned variants never
    /// apply).
    F64Cmp(CmpOp),
    I32Eqz,
    I32WrapI64,
    I64ExtendI32U,
    /// `f64.convert_i64_s` — surface `integer` widening into `number`.
    F64ConvertI64S,
    F64ConvertI32S,
    I64ExtendI32S,
    /// `i64.trunc_f64_s` — truncate toward zero; traps on NaN or out of
    /// range (the RUN003 family surfaces as a trap until error lowering).
    I64TruncF64S,
    /// `select(a, b, cond) -> cond ? a : b`.
    Select,
    /// Pushes the address of the fixed return area (resolved at emission,
    /// after the static data size is final).
    RetAreaPtr,
    /// i32 load at constant offset from the popped address.
    I32Load(u32),
    /// Zero-extending byte load at constant offset from the popped address.
    I32Load8U(u32),
    I64Load(u32),
    F64Load(u32),
    /// i32 store at constant offset: pops value, then address.
    I32Store(u32),
    /// Byte store at constant offset: pops value, then address.
    I32Store8(u32),
    I64Store(u32),
    F64Store(u32),
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
    DivU,
    RemU,
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

#[derive(Debug, Clone, Copy)]
pub enum F64Op {
    Add,
    Sub,
    Mul,
    Div,
    Min,
    Max,
}

#[derive(Debug, Clone, Copy)]
pub enum F64Un {
    Neg,
    Abs,
    Ceil,
    Floor,
    Trunc,
    Nearest,
    Sqrt,
}

/// **Internal** core-type shape of a semantic type — how a value lives in
/// locals and on the operand stack. `string`/`bytes` are a single `i32`
/// pointer to the `[u32 length][payload]` object (MMD-04); records
/// concatenate their fields; options prepend an `i32` discriminant.
/// `None` marks a type with no core lowering yet — `datetime`, `any`,
/// `matrix` and `pairs` land with later M6 stages; `Error` never survives
/// a clean typecheck, and `Var` never leaves pass [5]. A `list<T>` is a
/// single pointer to its MMD §3.4.1 header.
pub fn val_types(ty: &Ty) -> Option<Vec<Val>> {
    Some(match ty {
        Ty::Void => vec![],
        Ty::Integer | Ty::IntegerW(IntWidth::U64) => vec![Val::I64],
        Ty::IntegerW(_) | Ty::Boolean | Ty::Enum { .. } => vec![Val::I32],
        Ty::Str | Ty::Bytes | Ty::List(_, _) => vec![Val::I32],
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
        Ty::Number => vec![Val::F64],
        Ty::Datetime | Ty::Any | Ty::Matrix(_) | Ty::Pairs(_, _) => return None,
        // Nominal class instances and capability-typed values get their
        // memory representation with the M6 memory model; only the
        // structural `Record` boundary projection lowers today.
        Ty::Class { .. } | Ty::Cap { .. } => return None,
        Ty::Var(_) | Ty::Error => return None,
    })
}

/// How one leaf scalar of a list element is stored and loaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scalar {
    /// 8 bytes: `integer`, `integer:u64`.
    I64,
    /// 8 bytes: `number`.
    F64,
    /// 4 bytes: narrower boundary integers, `boolean`, enum discriminants.
    I32,
    /// 4 bytes: a pointer to a `string`/`bytes`/`list` object.
    Ptr(PtrTo),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtrTo {
    Text,
    List,
}

impl Scalar {
    fn size(self) -> u32 {
        match self {
            Scalar::I64 | Scalar::F64 => 8,
            Scalar::I32 | Scalar::Ptr(_) => 4,
        }
    }
}

/// Packed in-memory layout of one list element: the flattened leaves as
/// `(byte offset, scalar)` in field order, plus the element stride.
///
/// MMD §3.4.1 gives lists their header but no chapter tabulates element
/// sizes or record packing (DISCOVERIES-M6 items 7–8). Local adoption:
/// natural alignment, fields in declaration order, stride rounded up to
/// the widest leaf's alignment.
#[derive(Debug, Clone)]
pub struct ElemLayout {
    pub leaves: Vec<(u32, Scalar)>,
    pub stride: u32,
    pub align: u32,
}

pub fn elem_layout(ty: &Ty) -> Option<ElemLayout> {
    let mut leaves = Vec::new();
    let mut align = 4u32;
    let mut end = 0u32;
    collect_leaves(ty, &mut leaves, &mut align, &mut end)?;
    let stride = end.div_ceil(align) * align;
    Some(ElemLayout {
        leaves,
        stride,
        align,
    })
}

fn collect_leaves(
    ty: &Ty,
    leaves: &mut Vec<(u32, Scalar)>,
    align: &mut u32,
    end: &mut u32,
) -> Option<()> {
    let mut place = |scalar: Scalar, end: &mut u32, leaves: &mut Vec<(u32, Scalar)>| {
        let size = scalar.size();
        let offset = end.div_ceil(size) * size;
        leaves.push((offset, scalar));
        *align = (*align).max(size);
        *end = offset + size;
    };
    match ty {
        Ty::Integer | Ty::IntegerW(IntWidth::U64) => place(Scalar::I64, end, leaves),
        Ty::Number => place(Scalar::F64, end, leaves),
        Ty::IntegerW(_) | Ty::Boolean | Ty::Enum { .. } => place(Scalar::I32, end, leaves),
        Ty::Str | Ty::Bytes => place(Scalar::Ptr(PtrTo::Text), end, leaves),
        Ty::List(_, _) => place(Scalar::Ptr(PtrTo::List), end, leaves),
        Ty::Record { fields, .. } => {
            for (_, field_ty) in fields {
                collect_leaves(field_ty, leaves, align, end)?;
            }
        }
        _ => return None,
    }
    Some(())
}

/// One copy step when serializing an internal list element into its
/// Canonical ABI form at the boundary.
#[derive(Debug, Clone, Copy)]
enum CabiCopy {
    /// 8-byte scalar: internal offset → CABI offset.
    Copy64 { src: u32, dst: u32, float: bool },
    /// 4-byte scalar.
    Copy32 { src: u32, dst: u32 },
    /// A `string`/`bytes` pointer expands to `(payload ptr, len)`.
    Text { src: u32, dst: u32 },
}

/// The per-element copy plan for lowering `list<T>` to the Canonical ABI:
/// `None` when `T` has no CABI serialization here yet (nested lists,
/// sub-4-byte scalars like `boolean`/enums/narrow widths — their canonical
/// sizes are 1–2 bytes and wait for a real need).
fn cabi_elem_plan(ty: &Ty) -> Option<(Vec<CabiCopy>, u32)> {
    fn walk(
        ty: &Ty,
        internal_end: &mut u32,
        cabi_end: &mut u32,
        align: &mut u32,
        plan: &mut Vec<CabiCopy>,
    ) -> Option<()> {
        let src_at = |size: u32, internal_end: &mut u32| {
            let offset = internal_end.div_ceil(size) * size;
            *internal_end = offset + size;
            offset
        };
        let dst_at = |size: u32, align_to: u32, cabi_end: &mut u32, align: &mut u32| {
            let offset = cabi_end.div_ceil(align_to) * align_to;
            *cabi_end = offset + size;
            *align = (*align).max(align_to);
            offset
        };
        match ty {
            Ty::Integer | Ty::IntegerW(IntWidth::U64) => {
                let src = src_at(8, internal_end);
                let dst = dst_at(8, 8, cabi_end, align);
                plan.push(CabiCopy::Copy64 {
                    src,
                    dst,
                    float: false,
                });
            }
            Ty::Number => {
                let src = src_at(8, internal_end);
                let dst = dst_at(8, 8, cabi_end, align);
                plan.push(CabiCopy::Copy64 {
                    src,
                    dst,
                    float: true,
                });
            }
            Ty::IntegerW(IntWidth::U32 | IntWidth::S32) => {
                let src = src_at(4, internal_end);
                let dst = dst_at(4, 4, cabi_end, align);
                plan.push(CabiCopy::Copy32 { src, dst });
            }
            Ty::Str | Ty::Bytes => {
                let src = src_at(4, internal_end);
                let dst = dst_at(8, 4, cabi_end, align);
                plan.push(CabiCopy::Text { src, dst });
            }
            Ty::Record { fields, .. } => {
                for (_, field_ty) in fields {
                    walk(field_ty, internal_end, cabi_end, align, plan)?;
                }
            }
            _ => return None,
        }
        Some(())
    }
    let mut plan = Vec::new();
    let (mut internal_end, mut cabi_end, mut align) = (0u32, 0u32, 4u32);
    walk(ty, &mut internal_end, &mut cabi_end, &mut align, &mut plan)?;
    let stride = cabi_end.div_ceil(align) * align;
    Some((plan, stride))
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
    let mut tags = TagRegistry::default();
    let functions = program
        .functions
        .iter()
        .map(|f| lower_function(f, program, resolved, &remap, &mut data, &mut tags, sink))
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
            HExprKind::CallFn { args, .. } | HExprKind::CallStd { args, .. } => {
                args.iter().for_each(|a| expr(a, used))
            }
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

/// Compiler-local element-type tags for list headers (MMD §3.4.1: not part
/// of the ABI, stable within a single compilation). Assigned in first-use
/// order over the deterministic lowering walk.
#[derive(Default)]
struct TagRegistry {
    ids: std::collections::BTreeMap<String, u32>,
}

impl TagRegistry {
    fn tag(&mut self, ty: &Ty) -> u32 {
        let key = ty.display();
        let next = self.ids.len() as u32;
        *self.ids.entry(key).or_insert(next)
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
        self.intern_aligned(bytes, 4)
    }

    /// Interns with the run's base address aligned to `align` (list objects
    /// holding 8-byte leaves need an 8-aligned base).
    fn intern_aligned(&mut self, bytes: &[u8], align: u32) -> u32 {
        if let Some(&offset) = self.seen.get(bytes) {
            if offset.is_multiple_of(align) {
                return offset;
            }
        }
        while !(DATA_SECTION_START + self.blob.len() as u32).is_multiple_of(align) {
            self.blob.push(0);
        }
        let offset = DATA_SECTION_START + self.blob.len() as u32;
        self.blob.extend_from_slice(bytes);
        // Keep every run 4-aligned so the next base stays canonical.
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
    tags: &mut TagRegistry,
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
        tags,
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
    /// Program-wide element-type tags for list headers.
    tags: &'a mut TagRegistry,
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
    /// Serializes a fully-constant list into its static MMD §3.4.1 object:
    /// header, then elements packed per [`ElemLayout`]. Returns `None` when
    /// any element is not a serializable constant (the runtime path takes
    /// over).
    fn serialize_const_list(
        &mut self,
        items: &[HExpr],
        layout: &ElemLayout,
        tag: u32,
    ) -> Option<Vec<u8>> {
        let mut blob = Vec::new();
        blob.extend_from_slice(&(items.len() as u32).to_le_bytes());
        blob.extend_from_slice(&(items.len() as u32).to_le_bytes());
        blob.extend_from_slice(&tag.to_le_bytes());
        blob.extend_from_slice(&0u32.to_le_bytes());
        for item in items {
            let start = blob.len();
            blob.resize(start + layout.stride as usize, 0);
            let mut leaf = 0usize;
            self.serialize_const_elem(item, layout, &mut leaf, start, &mut blob)?;
        }
        Some(blob)
    }

    fn serialize_const_elem(
        &mut self,
        expr: &HExpr,
        layout: &ElemLayout,
        leaf: &mut usize,
        elem_start: usize,
        blob: &mut Vec<u8>,
    ) -> Option<()> {
        let mut write = |leaf: &mut usize, bytes: &[u8]| {
            let (offset, scalar) = layout.leaves[*leaf];
            debug_assert_eq!(bytes.len() as u32, scalar.size());
            let at = elem_start + offset as usize;
            blob[at..at + bytes.len()].copy_from_slice(bytes);
            *leaf += 1;
        };
        match &expr.kind {
            HExprKind::Str(value) => {
                let base = self.data.intern_string(value);
                write(leaf, &base.to_le_bytes());
                Some(())
            }
            HExprKind::Int(v) => {
                match layout.leaves[*leaf].1 {
                    Scalar::I64 => write(leaf, &(*v as i64).to_le_bytes()),
                    _ => write(leaf, &(*v as i32).to_le_bytes()),
                }
                Some(())
            }
            HExprKind::Num(v) => {
                write(leaf, &v.to_le_bytes());
                Some(())
            }
            HExprKind::Bool(v) => {
                write(leaf, &(*v as i32).to_le_bytes());
                Some(())
            }
            HExprKind::EnumCase(i) => {
                write(leaf, &i.to_le_bytes());
                Some(())
            }
            HExprKind::MakeRecord(fields) => {
                for field in fields {
                    self.serialize_const_elem(field, layout, leaf, elem_start, blob)?;
                }
                Some(())
            }
            _ => None,
        }
    }

    /// Emits stores for one element already evaluated into consecutive
    /// scratch slots: `base_slot` holds the element's base address (header
    /// excluded — callers fold `LIST_ELEMS_OFFSET` into `extra_offset`).
    fn store_element_from_scratch(
        &mut self,
        layout: &ElemLayout,
        addr_slot: u32,
        value_base_slot: u32,
        extra_offset: u32,
        out: &mut Vec<Inst>,
    ) {
        for (index, (offset, scalar)) in layout.leaves.iter().enumerate() {
            out.push(Inst::LocalGet(addr_slot));
            out.push(Inst::LocalGet(value_base_slot + index as u32));
            out.push(match scalar {
                Scalar::I64 => Inst::I64Store(extra_offset + offset),
                Scalar::F64 => Inst::F64Store(extra_offset + offset),
                Scalar::I32 | Scalar::Ptr(_) => Inst::I32Store(extra_offset + offset),
            });
        }
    }

    /// Emits loads pushing one element's flattened slots; `addr_slot` holds
    /// the element's address minus `extra_offset`.
    fn load_element(
        &mut self,
        layout: &ElemLayout,
        addr_slot: u32,
        extra_offset: u32,
        out: &mut Vec<Inst>,
    ) {
        for (offset, scalar) in &layout.leaves {
            out.push(Inst::LocalGet(addr_slot));
            out.push(match scalar {
                Scalar::I64 => Inst::I64Load(extra_offset + offset),
                Scalar::F64 => Inst::F64Load(extra_offset + offset),
                Scalar::I32 | Scalar::Ptr(_) => Inst::I32Load(extra_offset + offset),
            });
        }
    }

    /// Evaluates a list receiver and an index expression, bounds-checks in
    /// the i64 domain (a negative index is a huge unsigned value; out of
    /// range traps — RUN013's catchable form needs error lowering), and
    /// computes the element address. Returns `(base, idx64, addr)` slots;
    /// leaves load/store at `LIST_ELEMS_OFFSET` + leaf offset from `addr`.
    fn emit_list_elem_addr(
        &mut self,
        recv: &HExpr,
        index: &HExpr,
        layout: &ElemLayout,
        out: &mut Vec<Inst>,
        sink: &mut DiagnosticSink,
    ) -> (u32, u32, u32) {
        let base = self.alloc_scratch(&[Val::I32]);
        let idx64 = self.alloc_scratch(&[Val::I64]);
        let addr = self.alloc_scratch(&[Val::I32]);
        self.expr(recv, out, sink);
        out.push(Inst::LocalSet(base));
        self.expr(index, out, sink);
        if !is_i64(&index.ty) {
            out.push(Inst::I64ExtendI32U);
        }
        out.push(Inst::LocalSet(idx64));
        out.push(Inst::LocalGet(idx64));
        out.push(Inst::LocalGet(base));
        out.push(Inst::I32Load(crate::layout::LIST_LEN_OFFSET));
        out.push(Inst::I64ExtendI32U);
        out.push(Inst::I64Cmp(CmpOp::LtU));
        out.push(Inst::I32Eqz);
        out.push(Inst::If {
            result: None,
            then: vec![Inst::Unreachable],
            els: vec![],
        });
        out.push(Inst::LocalGet(base));
        out.push(Inst::LocalGet(idx64));
        out.push(Inst::I32WrapI64);
        out.push(Inst::I32Const(layout.stride as i32));
        out.push(Inst::I32Bin(I32Op::Mul));
        out.push(Inst::I32Bin(I32Op::Add));
        out.push(Inst::LocalSet(addr));
        (base, idx64, addr)
    }

    /// Evaluates a list receiver into a scratch slot and traps when it is
    /// empty (`first()`/`last()`/`remove()`/`peek()` on an empty
    /// collection — RUN013 family).
    fn emit_list_recv_nonempty(
        &mut self,
        recv: &HExpr,
        out: &mut Vec<Inst>,
        sink: &mut DiagnosticSink,
    ) -> u32 {
        let base = self.alloc_scratch(&[Val::I32]);
        self.expr(recv, out, sink);
        out.push(Inst::LocalTee(base));
        out.push(Inst::I32Load(crate::layout::LIST_LEN_OFFSET));
        out.push(Inst::I32Eqz);
        out.push(Inst::If {
            result: None,
            then: vec![Inst::Unreachable],
            els: vec![],
        });
        base
    }

    /// Pushes the address of the last element: `base + (len-1) * stride`.
    fn emit_list_last_addr(&mut self, base: u32, stride: u32, out: &mut Vec<Inst>) -> u32 {
        let addr = self.alloc_scratch(&[Val::I32]);
        out.push(Inst::LocalGet(base));
        out.push(Inst::LocalGet(base));
        out.push(Inst::I32Load(crate::layout::LIST_LEN_OFFSET));
        out.push(Inst::I32Const(1));
        out.push(Inst::I32Bin(I32Op::Sub));
        out.push(Inst::I32Const(stride as i32));
        out.push(Inst::I32Bin(I32Op::Mul));
        out.push(Inst::I32Bin(I32Op::Add));
        out.push(Inst::LocalSet(addr));
        addr
    }

    /// The chapter-15 list operations (all `CallStd` variants prefixed
    /// `List`); returns true when `func` was one of them.
    fn lower_list_std(
        &mut self,
        func: crate::typecheck::stdlib::StdFn,
        args: &[HExpr],
        expr: &HExpr,
        out: &mut Vec<Inst>,
        sink: &mut DiagnosticSink,
    ) -> bool {
        use crate::typecheck::stdlib::StdFn::*;
        use runtime::RuntimeFn;
        const LEN: u32 = crate::layout::LIST_LEN_OFFSET;
        const ELEMS: u32 = crate::layout::LIST_ELEMS_OFFSET;

        // The receiver's element layout, where there is a list receiver.
        let recv_elem = || match args.first().map(|a| &a.ty) {
            Some(Ty::List(elem, _)) => Some((**elem).clone()),
            _ => None,
        };
        // The single-leaf search/sort kind of an element type.
        enum Kind {
            I64,
            F64,
            Str,
            I32,
        }
        let scalar_kind = |elem: &Ty| -> Option<Kind> {
            let layout = elem_layout(elem)?;
            match layout.leaves.as_slice() {
                [(_, Scalar::I64)] => Some(Kind::I64),
                [(_, Scalar::F64)] => Some(Kind::F64),
                [(_, Scalar::Ptr(PtrTo::Text))] => Some(Kind::Str),
                [(_, Scalar::I32)] => Some(Kind::I32),
                _ => None,
            }
        };

        match func {
            // §3.4.1 inline elements + growth relocation break aliasing;
            // blocked on a foundation ruling (DISCOVERIES-M6).
            ListAdd | ListInsert => {
                self.note(sink, "growing list methods", expr.span);
            }
            ListLength => {
                self.expr(&args[0], out, sink);
                out.push(Inst::I32Load(LEN));
                out.push(Inst::I64ExtendI32U);
            }
            ListIsEmpty => {
                self.expr(&args[0], out, sink);
                out.push(Inst::I32Load(LEN));
                out.push(Inst::I32Eqz);
            }
            ListIsNotEmpty => {
                self.expr(&args[0], out, sink);
                out.push(Inst::I32Load(LEN));
                out.push(Inst::I32Const(0));
                out.push(Inst::I32Cmp(CmpOp::Ne));
            }
            ListGet | ListSet | ListRemoveAt => {
                let Some(elem) = recv_elem() else {
                    self.note(sink, "list values of this element type", expr.span);
                    return true;
                };
                let Some(layout) = elem_layout(&elem) else {
                    self.note(sink, "list values of this element type", expr.span);
                    return true;
                };
                let (base, idx64, addr) =
                    self.emit_list_elem_addr(&args[0], &args[1], &layout, out, sink);
                match func {
                    ListGet => self.load_element(&layout, addr, ELEMS, out),
                    ListSet => {
                        self.expr(&args[2], out, sink);
                        let scratch = self.elem_scratch(&layout);
                        for slot in (0..layout.leaves.len() as u32).rev() {
                            out.push(Inst::LocalSet(scratch + slot));
                        }
                        self.store_element_from_scratch(&layout, addr, scratch, ELEMS, out);
                    }
                    _ => {
                        // remove(index): the removed element, then shift.
                        self.load_element(&layout, addr, ELEMS, out);
                        let scratch = self.elem_scratch(&layout);
                        for slot in (0..layout.leaves.len() as u32).rev() {
                            out.push(Inst::LocalSet(scratch + slot));
                        }
                        out.push(Inst::LocalGet(base));
                        out.push(Inst::LocalGet(idx64));
                        out.push(Inst::I32Const(layout.stride as i32));
                        out.push(Inst::CallRuntime(RuntimeFn::ListRemoveAt));
                        for slot in 0..layout.leaves.len() as u32 {
                            out.push(Inst::LocalGet(scratch + slot));
                        }
                    }
                }
            }
            ListFirst | ListLast | ListPeek | ListRemoveBehavior | ListRemoveLast => {
                let Some(elem) = recv_elem() else {
                    self.note(sink, "list values of this element type", expr.span);
                    return true;
                };
                let Some(layout) = elem_layout(&elem) else {
                    self.note(sink, "list values of this element type", expr.span);
                    return true;
                };
                // Behavior resolution: front of a `.line`, top of a
                // `.pile` (pass [5] already rejected the undeclared case).
                let front = match (&func, &args[0].ty) {
                    (ListFirst, _) => true,
                    (ListLast | ListRemoveLast, _) => false,
                    // remove()/peek(): front of a `.line`, top of a `.pile`.
                    (_, Ty::List(_, behavior)) => {
                        matches!(
                            behavior.removal,
                            Some(crate::typecheck::types::Removal::Line)
                        )
                    }
                    _ => true,
                };
                let base = self.emit_list_recv_nonempty(&args[0], out, sink);
                let addr = if front {
                    base
                } else {
                    self.emit_list_last_addr(base, layout.stride, out)
                };
                self.load_element(&layout, addr, ELEMS, out);
                match func {
                    ListRemoveLast | ListRemoveBehavior if !front => {
                        // Drop the last element: len -= 1.
                        out.push(Inst::LocalGet(base));
                        out.push(Inst::LocalGet(base));
                        out.push(Inst::I32Load(LEN));
                        out.push(Inst::I32Const(1));
                        out.push(Inst::I32Bin(I32Op::Sub));
                        out.push(Inst::I32Store(LEN));
                    }
                    ListRemoveBehavior => {
                        // Front removal shifts the tail (in place, no
                        // relocation — aliasing-safe).
                        let scratch = self.elem_scratch(&layout);
                        for slot in (0..layout.leaves.len() as u32).rev() {
                            out.push(Inst::LocalSet(scratch + slot));
                        }
                        out.push(Inst::LocalGet(base));
                        out.push(Inst::I64Const(0));
                        out.push(Inst::I32Const(layout.stride as i32));
                        out.push(Inst::CallRuntime(RuntimeFn::ListRemoveAt));
                        for slot in 0..layout.leaves.len() as u32 {
                            out.push(Inst::LocalGet(scratch + slot));
                        }
                    }
                    _ => {}
                }
            }
            ListContains | ListIndexOf | ListLastIndexOf => {
                let Some(elem) = recv_elem() else {
                    self.note(sink, "list values of this element type", expr.span);
                    return true;
                };
                let Some(kind) = scalar_kind(&elem) else {
                    self.note(sink, "list search over this element type", expr.span);
                    return true;
                };
                self.expr(&args[0], out, sink);
                self.expr(&args[1], out, sink);
                out.push(Inst::I32Const(matches!(func, ListLastIndexOf) as i32));
                out.push(Inst::CallRuntime(match kind {
                    Kind::I64 => RuntimeFn::ListIndexOfI64,
                    Kind::F64 => RuntimeFn::ListIndexOfF64,
                    Kind::Str => RuntimeFn::ListIndexOfStr,
                    Kind::I32 => RuntimeFn::ListIndexOf32,
                }));
                if matches!(func, ListContains) {
                    out.push(Inst::I64Const(0));
                    out.push(Inst::I64Cmp(CmpOp::GeS));
                }
            }
            ListSlice | ListReverse | ListConcat => {
                let Some(elem) = recv_elem() else {
                    self.note(sink, "list values of this element type", expr.span);
                    return true;
                };
                let Some(layout) = elem_layout(&elem) else {
                    self.note(sink, "list values of this element type", expr.span);
                    return true;
                };
                let tag = self.tags.tag(&elem);
                for arg in args {
                    self.expr(arg, out, sink);
                }
                out.push(Inst::I32Const(layout.stride as i32));
                out.push(Inst::I32Const(tag as i32));
                out.push(Inst::CallRuntime(match func {
                    ListSlice => RuntimeFn::ListSlice,
                    ListReverse => RuntimeFn::ListReverse,
                    _ => RuntimeFn::ListConcat,
                }));
            }
            ListSort => {
                let Some(elem) = recv_elem() else {
                    self.note(sink, "list values of this element type", expr.span);
                    return true;
                };
                let Some(kind) = scalar_kind(&elem) else {
                    self.note(sink, "list sort over this element type", expr.span);
                    return true;
                };
                let helper = match kind {
                    Kind::I64 => RuntimeFn::ListSortI64,
                    Kind::F64 => RuntimeFn::ListSortF64,
                    Kind::Str => RuntimeFn::ListSortStr,
                    Kind::I32 => {
                        // 4-byte scalars sort as their i32 payloads only
                        // when signedness is coherent; deferred until a
                        // need appears.
                        self.note(sink, "list sort over this element type", expr.span);
                        return true;
                    }
                };
                let tag = self.tags.tag(&elem);
                self.expr(&args[0], out, sink);
                out.push(Inst::I32Const(tag as i32));
                out.push(Inst::CallRuntime(helper));
            }
            ListRange => {
                self.expr(&args[0], out, sink);
                self.expr(&args[1], out, sink);
                let tag = self.tags.tag(&Ty::Integer);
                out.push(Inst::I32Const(tag as i32));
                out.push(Inst::CallRuntime(RuntimeFn::ListRange));
            }
            ListFill => {
                let elem = args[1].ty.clone();
                let Some(kind) = scalar_kind(&elem) else {
                    self.note(sink, "list fill over this element type", expr.span);
                    return true;
                };
                let tag = self.tags.tag(&elem);
                self.expr(&args[0], out, sink);
                self.expr(&args[1], out, sink);
                out.push(Inst::I32Const(tag as i32));
                out.push(Inst::CallRuntime(match kind {
                    Kind::I64 => RuntimeFn::ListFill64,
                    Kind::F64 => RuntimeFn::ListFillF64,
                    Kind::Str | Kind::I32 => RuntimeFn::ListFill32,
                }));
            }
            ListJoin => {
                self.expr(&args[0], out, sink);
                self.expr(&args[1], out, sink);
                out.push(Inst::CallRuntime(RuntimeFn::ListJoin));
            }
            _ => return false,
        }
        true
    }

    /// Scratch slots matching one element's flattened internal shape.
    fn elem_scratch(&mut self, layout: &ElemLayout) -> u32 {
        let vals: Vec<Val> = layout
            .leaves
            .iter()
            .map(|(_, scalar)| match scalar {
                Scalar::I64 => Val::I64,
                Scalar::F64 => Val::F64,
                Scalar::I32 | Scalar::Ptr(_) => Val::I32,
            })
            .collect();
        self.alloc_scratch(&vals)
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
            // FLW-02 list iterate: index-driven walk over the §3.4.1
            // object; the element binds before the body's inner block so
            // `continue` still advances (FLW-03).
            HStmt::Iterate {
                binder,
                source: HIterSource::List(source),
                step,
                body,
            } if matches!(&source.ty, Ty::List(elem, _) if elem_layout(elem).is_some()) => {
                let Ty::List(elem, _) = &source.ty else {
                    unreachable!("guard checked the source is a list");
                };
                let layout = elem_layout(elem).expect("guard checked the layout");
                if let Some(step) = step {
                    // The M4 brief 2026-08-17-iterate-step-non-range.md
                    // owns step-on-list semantics; until it lands, the
                    // checker types it and codegen declines.
                    self.note(sink, "iterate step over list sources", step.span);
                    return;
                }
                let base = self.alloc_scratch(&[Val::I32]);
                let len = self.alloc_scratch(&[Val::I32]);
                let idx = self.alloc_scratch(&[Val::I32]);
                let addr = self.alloc_scratch(&[Val::I32]);
                self.expr(source, out, sink);
                out.push(Inst::LocalTee(base));
                out.push(Inst::I32Load(crate::layout::LIST_LEN_OFFSET));
                out.push(Inst::LocalSet(len));
                out.push(Inst::I32Const(0));
                out.push(Inst::LocalSet(idx));

                let label_base = self.label_depth;
                self.loops.push(LoopCtx {
                    break_abs: label_base,
                    continue_abs: label_base + 2,
                });
                // Exit test and binder loads sit at the loop's own label
                // depth, before the inner block.
                let mut bind = vec![
                    Inst::LocalGet(idx),
                    Inst::LocalGet(len),
                    Inst::I32Cmp(CmpOp::GeS),
                    Inst::BrIf(1),
                    Inst::LocalGet(base),
                    Inst::LocalGet(idx),
                    Inst::I32Const(layout.stride as i32),
                    Inst::I32Bin(I32Op::Mul),
                    Inst::I32Bin(I32Op::Add),
                    Inst::LocalSet(addr),
                ];
                self.load_element(&layout, addr, crate::layout::LIST_ELEMS_OFFSET, &mut bind);
                let binder_base = self.slots[*binder];
                for offset in (0..self.slot_widths[*binder]).rev() {
                    bind.push(Inst::LocalSet(binder_base + offset));
                }

                self.label_depth += 3;
                let mut inner = Vec::new();
                for stmt in body {
                    self.stmt(stmt, &mut inner, sink);
                }
                self.label_depth -= 3;
                self.loops.pop();

                let mut loop_body = bind;
                loop_body.push(Inst::Block { body: inner });
                loop_body.push(Inst::LocalGet(idx));
                loop_body.push(Inst::I32Const(1));
                loop_body.push(Inst::I32Bin(I32Op::Add));
                loop_body.push(Inst::LocalSet(idx));
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
                        Val::F64 => Inst::F64Const(0.0),
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
                let elem_ty = match &expr.ty {
                    Ty::List(elem, _) => (**elem).clone(),
                    _ => Ty::Error,
                };
                let Some(layout) = elem_layout(&elem_ty) else {
                    self.note(sink, "list values of this element type", expr.span);
                    return;
                };
                let tag = self.tags.tag(&elem_ty);
                // Fully-constant lists live in static data.
                if let Some(blob) = self.serialize_const_list(items, &layout, tag) {
                    let base = self.data.intern_aligned(&blob, 8);
                    out.push(Inst::I32Const(base as i32));
                    return;
                }
                // Runtime construction: allocate, fill the header, store
                // each element.
                let base = self.alloc_scratch(&[Val::I32]);
                let size = crate::layout::LIST_ELEMS_OFFSET + items.len() as u32 * layout.stride;
                out.push(Inst::I32Const(size as i32));
                out.push(Inst::I32Const(crate::layout::ALIGNMENT as i32));
                out.push(Inst::CallRuntime(runtime::RuntimeFn::Alloc));
                out.push(Inst::LocalTee(base));
                out.push(Inst::I32Const(items.len() as i32));
                out.push(Inst::I32Store(crate::layout::LIST_LEN_OFFSET));
                out.push(Inst::LocalGet(base));
                out.push(Inst::I32Const(items.len() as i32));
                out.push(Inst::I32Store(crate::layout::LIST_CAP_OFFSET));
                out.push(Inst::LocalGet(base));
                out.push(Inst::I32Const(tag as i32));
                out.push(Inst::I32Store(crate::layout::LIST_TAG_OFFSET));
                out.push(Inst::LocalGet(base));
                out.push(Inst::I32Const(0));
                out.push(Inst::I32Store(crate::layout::LIST_TAG_OFFSET + 4));
                let scratch = self.elem_scratch(&layout);
                for (i, item) in items.iter().enumerate() {
                    self.expr(item, out, sink);
                    for slot in (0..layout.leaves.len() as u32).rev() {
                        out.push(Inst::LocalSet(scratch + slot));
                    }
                    self.store_element_from_scratch(
                        &layout,
                        base,
                        scratch,
                        crate::layout::LIST_ELEMS_OFFSET + i as u32 * layout.stride,
                        out,
                    );
                }
                out.push(Inst::LocalGet(base));
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
                    if matches!(operand.ty, Ty::Number) {
                        self.expr(operand, out, sink);
                        out.push(Inst::F64Un(F64Un::Neg));
                    } else {
                        out.push(Inst::I64Const(0));
                        self.expr(operand, out, sink);
                        out.push(Inst::I64Bin(I64Op::Sub));
                    }
                }
            },
            // TYP-03: `T` into `T?` — discriminant 1, then the payload
            // (val_types puts the discriminant first).
            HExprKind::WrapSome(operand) => {
                out.push(Inst::I32Const(1));
                self.expr(operand, out, sink);
            }
            HExprKind::Num(v) => out.push(Inst::F64Const(*v)),
            // TYP-06 widening: surface `integer` (i64) into `number`.
            HExprKind::IntToNumber(operand) => {
                self.expr(operand, out, sink);
                if !is_i64(&operand.ty) {
                    out.push(Inst::I64ExtendI32U);
                }
                out.push(Inst::F64ConvertI64S);
            }
            // The M4 frontier: typed, not yet lowerable to core wasm.
            HExprKind::StrInterp(_) => self.note(sink, "string interpolation", expr.span),
            HExprKind::MakeMatrix(_) => self.note(sink, "matrix values", expr.span),
            HExprKind::Index { recv, index, kind } => match kind {
                tir::IndexKind::List => {
                    let Ty::List(elem, _) = &recv.ty else {
                        self.note(sink, "index access", expr.span);
                        return;
                    };
                    let Some(layout) = elem_layout(elem) else {
                        self.note(sink, "list values of this element type", expr.span);
                        return;
                    };
                    let (_, _, addr) = self.emit_list_elem_addr(recv, index, &layout, out, sink);
                    self.load_element(&layout, addr, crate::layout::LIST_ELEMS_OFFSET, out);
                }
                tir::IndexKind::Matrix | tir::IndexKind::Pairs | tir::IndexKind::Any => {
                    self.note(sink, "index access", expr.span)
                }
            },
            HExprKind::NonNone(_) => self.note(sink, "postfix `!` assertion", expr.span),
            HExprKind::IsNone { .. } => self.note(sink, "is-none checks", expr.span),
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
            // TYP-06 / 15 §Conversions: dispatch on (source, target).
            HExprKind::Convert(operand) => {
                let source = operand.ty.clone();
                let target = expr.ty.clone();
                match (&source, &target) {
                    // Identity.
                    (Ty::Integer, Ty::Integer)
                    | (Ty::Number, Ty::Number)
                    | (Ty::Str, Ty::Str)
                    | (Ty::Boolean, Ty::Boolean) => self.expr(operand, out, sink),
                    // Widen into number.
                    (Ty::Integer | Ty::IntegerW(_), Ty::Number) => {
                        self.expr(operand, out, sink);
                        if !is_i64(&source) {
                            out.push(Inst::I64ExtendI32U);
                        }
                        out.push(Inst::F64ConvertI64S);
                    }
                    (Ty::Boolean, Ty::Number) => {
                        self.expr(operand, out, sink);
                        out.push(Inst::F64ConvertI32S);
                    }
                    // Truncate toward zero; NaN or out-of-range traps
                    // (RUN003 family until error lowering).
                    (Ty::Number, Ty::Integer) => {
                        self.expr(operand, out, sink);
                        out.push(Inst::I64TruncF64S);
                    }
                    (Ty::Boolean | Ty::IntegerW(_), Ty::Integer) => {
                        self.expr(operand, out, sink);
                        out.push(Inst::I64ExtendI32U);
                    }
                    // Zero → false, anything else (NaN included) → true.
                    (Ty::Integer, Ty::Boolean) => {
                        self.expr(operand, out, sink);
                        out.push(Inst::I64Const(0));
                        out.push(Inst::I64Cmp(CmpOp::Ne));
                    }
                    (Ty::Number, Ty::Boolean) => {
                        self.expr(operand, out, sink);
                        out.push(Inst::F64Const(0.0));
                        out.push(Inst::F64Cmp(CmpOp::Ne));
                    }
                    // Renderings.
                    (Ty::Integer | Ty::IntegerW(_), Ty::Str) => {
                        self.expr(operand, out, sink);
                        if !is_i64(&source) {
                            out.push(Inst::I64ExtendI32U);
                        }
                        out.push(Inst::CallRuntime(runtime::RuntimeFn::IntToString));
                    }
                    (Ty::Boolean, Ty::Str) => {
                        let t = self.data.intern_string("true");
                        let f = self.data.intern_string("false");
                        self.expr(operand, out, sink);
                        out.push(Inst::If {
                            result: Some(Val::I32),
                            then: vec![Inst::I32Const(t as i32)],
                            els: vec![Inst::I32Const(f as i32)],
                        });
                    }
                    // Parses; non-literals trap (RUN003 family).
                    (Ty::Str, Ty::Integer) => {
                        self.expr(operand, out, sink);
                        out.push(Inst::CallRuntime(runtime::RuntimeFn::StrToInt));
                    }
                    (Ty::Str, Ty::Number) => {
                        self.expr(operand, out, sink);
                        out.push(Inst::CallRuntime(runtime::RuntimeFn::StrToNum));
                    }
                    // number.toString needs a formatting contract the spec
                    // does not state (shortest round-trip vs fixed) —
                    // DISCOVERIES-M6. string.toBoolean is not in the 15
                    // §Conversions table at all.
                    _ => self.note(sink, "conversion methods in compiled code", expr.span),
                }
            }
            HExprKind::CallStd { func, args } => {
                use crate::typecheck::stdlib::{is_transcendental, StdFn};
                if is_transcendental(*func) {
                    // Blocked on the guest-vs-clean:bridge/math ruling
                    // (DISCOVERIES-M6 item 2, ADR 0004).
                    self.note(sink, "math transcendental functions", expr.span);
                    return;
                }
                if matches!(func, StdFn::StrToUpperCase | StdFn::StrToLowerCase) {
                    // Unicode case folding is clean:bridge/string territory
                    // (DISCOVERIES-M6 item 1) — no ASCII approximation.
                    self.note(sink, "string case conversion", expr.span);
                    return;
                }
                if self.lower_list_std(*func, args, expr, out, sink) {
                    return;
                }
                for arg in args {
                    self.expr(arg, out, sink);
                }
                match func {
                    StdFn::StrLength => out.push(Inst::CallRuntime(runtime::RuntimeFn::StrCpLen)),
                    StdFn::StrIsEmpty => {
                        out.push(Inst::I32Load(0));
                        out.push(Inst::I32Eqz);
                    }
                    StdFn::StrIsBlank => {
                        out.push(Inst::CallRuntime(runtime::RuntimeFn::StrIsBlank))
                    }
                    StdFn::StrContains => {
                        out.push(Inst::CallRuntime(runtime::RuntimeFn::StrIndexOf));
                        out.push(Inst::I64Const(0));
                        out.push(Inst::I64Cmp(CmpOp::GeS));
                    }
                    StdFn::StrIndexOf => {
                        out.push(Inst::CallRuntime(runtime::RuntimeFn::StrIndexOf))
                    }
                    StdFn::StrLastIndexOf => {
                        out.push(Inst::CallRuntime(runtime::RuntimeFn::StrLastIndexOf))
                    }
                    StdFn::StrStartsWith => {
                        out.push(Inst::CallRuntime(runtime::RuntimeFn::StrStartsWith))
                    }
                    StdFn::StrEndsWith => {
                        out.push(Inst::CallRuntime(runtime::RuntimeFn::StrEndsWith))
                    }
                    StdFn::StrCharAt => out.push(Inst::CallRuntime(runtime::RuntimeFn::StrCharAt)),
                    StdFn::StrCharCodeAt => {
                        out.push(Inst::CallRuntime(runtime::RuntimeFn::StrCharCodeAt))
                    }
                    StdFn::StrSubstring => {
                        out.push(Inst::CallRuntime(runtime::RuntimeFn::StrSubstring))
                    }
                    StdFn::StrTrim => {
                        out.push(Inst::I32Const(0));
                        out.push(Inst::CallRuntime(runtime::RuntimeFn::StrTrim));
                    }
                    StdFn::StrTrimStart => {
                        out.push(Inst::I32Const(1));
                        out.push(Inst::CallRuntime(runtime::RuntimeFn::StrTrim));
                    }
                    StdFn::StrTrimEnd => {
                        out.push(Inst::I32Const(2));
                        out.push(Inst::CallRuntime(runtime::RuntimeFn::StrTrim));
                    }
                    StdFn::StrPadStart => {
                        out.push(Inst::CallRuntime(runtime::RuntimeFn::StrPadStart))
                    }
                    StdFn::StrPadEnd => out.push(Inst::CallRuntime(runtime::RuntimeFn::StrPadEnd)),
                    StdFn::StrReplace => {
                        out.push(Inst::CallRuntime(runtime::RuntimeFn::StrReplace))
                    }
                    StdFn::StrSplit => {
                        let tag = self.tags.tag(&Ty::Str);
                        out.push(Inst::I32Const(tag as i32));
                        out.push(Inst::CallRuntime(runtime::RuntimeFn::StrSplit));
                    }
                    StdFn::StrConcat => {
                        out.push(Inst::CallRuntime(runtime::RuntimeFn::StringConcat))
                    }
                    StdFn::MathSqrt => out.push(Inst::F64Un(F64Un::Sqrt)),
                    StdFn::MathAbsNumber => out.push(Inst::F64Un(F64Un::Abs)),
                    StdFn::MathFloor => out.push(Inst::F64Un(F64Un::Floor)),
                    StdFn::MathCeil => out.push(Inst::F64Un(F64Un::Ceil)),
                    // 15 §Math "round nearest": wasm f64.nearest rounds
                    // half-to-even; whether the spec means half-away-from-
                    // zero is undecided (DISCOVERIES-M6).
                    StdFn::MathRound => out.push(Inst::F64Un(F64Un::Nearest)),
                    StdFn::MathTrunc => out.push(Inst::F64Un(F64Un::Trunc)),
                    StdFn::MathMax => out.push(Inst::F64Bin(F64Op::Max)),
                    StdFn::MathMin => out.push(Inst::F64Bin(F64Op::Min)),
                    // sign(x): (x > 0) - (x < 0) as number.
                    StdFn::MathSign => {
                        let x = self.alloc_scratch(&[Val::F64]);
                        out.push(Inst::LocalSet(x));
                        out.push(Inst::LocalGet(x));
                        out.push(Inst::F64Const(0.0));
                        out.push(Inst::F64Cmp(CmpOp::GtS));
                        out.push(Inst::LocalGet(x));
                        out.push(Inst::F64Const(0.0));
                        out.push(Inst::F64Cmp(CmpOp::LtS));
                        out.push(Inst::I32Bin(I32Op::Sub));
                        out.push(Inst::F64ConvertI32S);
                    }
                    // |n|: n < 0 ? 0 - n : n.
                    StdFn::MathAbsInteger => {
                        let n = self.alloc_scratch(&[Val::I64]);
                        out.push(Inst::LocalTee(n));
                        out.push(Inst::I64Const(0));
                        out.push(Inst::I64Cmp(CmpOp::LtS));
                        out.push(Inst::If {
                            result: Some(Val::I64),
                            then: vec![
                                Inst::I64Const(0),
                                Inst::LocalGet(n),
                                Inst::I64Bin(I64Op::Sub),
                            ],
                            els: vec![Inst::LocalGet(n)],
                        });
                    }
                    transcendental => {
                        unreachable!("transcendental {transcendental:?} filtered above")
                    }
                }
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
            // Lists serialize element by element into a fresh Canonical
            // ABI buffer (§3.7: the host sees copies, never the §3.4.1
            // object), then lower as (buffer, count).
            Ty::List(elem, _) => {
                let (Some(layout), Some((plan, cabi_stride))) =
                    (elem_layout(elem), cabi_elem_plan(elem))
                else {
                    self.note(
                        sink,
                        "list values of this element type at the host boundary",
                        arg.span,
                    );
                    return;
                };
                let src = self.alloc_scratch(&[Val::I32]);
                let len = self.alloc_scratch(&[Val::I32]);
                let buf = self.alloc_scratch(&[Val::I32]);
                let idx = self.alloc_scratch(&[Val::I32]);
                let saddr = self.alloc_scratch(&[Val::I32]);
                let daddr = self.alloc_scratch(&[Val::I32]);
                self.expr(arg, out, sink);
                out.push(Inst::LocalTee(src));
                out.push(Inst::I32Load(crate::layout::LIST_LEN_OFFSET));
                out.push(Inst::LocalSet(len));
                out.push(Inst::LocalGet(len));
                out.push(Inst::I32Const(cabi_stride as i32));
                out.push(Inst::I32Bin(I32Op::Mul));
                out.push(Inst::I32Const(crate::layout::ALIGNMENT as i32));
                out.push(Inst::CallRuntime(runtime::RuntimeFn::Alloc));
                out.push(Inst::LocalSet(buf));
                out.push(Inst::I32Const(0));
                out.push(Inst::LocalSet(idx));
                let mut copy = vec![
                    Inst::LocalGet(idx),
                    Inst::LocalGet(len),
                    Inst::I32Cmp(CmpOp::GeS),
                    Inst::BrIf(1),
                    Inst::LocalGet(src),
                    Inst::LocalGet(idx),
                    Inst::I32Const(layout.stride as i32),
                    Inst::I32Bin(I32Op::Mul),
                    Inst::I32Bin(I32Op::Add),
                    Inst::LocalSet(saddr),
                    Inst::LocalGet(buf),
                    Inst::LocalGet(idx),
                    Inst::I32Const(cabi_stride as i32),
                    Inst::I32Bin(I32Op::Mul),
                    Inst::I32Bin(I32Op::Add),
                    Inst::LocalSet(daddr),
                ];
                let elems = crate::layout::LIST_ELEMS_OFFSET;
                for step in &plan {
                    match *step {
                        CabiCopy::Copy64 {
                            src: s,
                            dst: d,
                            float,
                        } => {
                            copy.push(Inst::LocalGet(daddr));
                            copy.push(Inst::LocalGet(saddr));
                            copy.push(if float {
                                Inst::F64Load(elems + s)
                            } else {
                                Inst::I64Load(elems + s)
                            });
                            copy.push(if float {
                                Inst::F64Store(d)
                            } else {
                                Inst::I64Store(d)
                            });
                        }
                        CabiCopy::Copy32 { src: s, dst: d } => {
                            copy.push(Inst::LocalGet(daddr));
                            copy.push(Inst::LocalGet(saddr));
                            copy.push(Inst::I32Load(elems + s));
                            copy.push(Inst::I32Store(d));
                        }
                        CabiCopy::Text { src: s, dst: d } => {
                            // (payload ptr, byte length) from the string
                            // object's base.
                            copy.push(Inst::LocalGet(daddr));
                            copy.push(Inst::LocalGet(saddr));
                            copy.push(Inst::I32Load(elems + s));
                            copy.push(Inst::I32Const(4));
                            copy.push(Inst::I32Bin(I32Op::Add));
                            copy.push(Inst::I32Store(d));
                            copy.push(Inst::LocalGet(daddr));
                            copy.push(Inst::LocalGet(saddr));
                            copy.push(Inst::I32Load(elems + s));
                            copy.push(Inst::I32Load(0));
                            copy.push(Inst::I32Store(d + 4));
                        }
                    }
                }
                copy.push(Inst::LocalGet(idx));
                copy.push(Inst::I32Const(1));
                copy.push(Inst::I32Bin(I32Op::Add));
                copy.push(Inst::LocalSet(idx));
                copy.push(Inst::Br(0));
                out.push(Inst::Block {
                    body: vec![Inst::Loop { body: copy }],
                });
                out.push(Inst::LocalGet(buf));
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
        // The float domain: pass [5] already inserted `IntToNumber`
        // widenings, so both operands arrive `number`-typed.
        if matches!(lhs.ty, Ty::Number) || matches!(rhs.ty, Ty::Number) {
            match op {
                Add | Sub | Mul | Div => {
                    self.lower_float_operand(lhs, out, sink);
                    self.lower_float_operand(rhs, out, sink);
                    out.push(Inst::F64Bin(match op {
                        Add => F64Op::Add,
                        Sub => F64Op::Sub,
                        Mul => F64Op::Mul,
                        _ => F64Op::Div,
                    }));
                }
                // fmod: a - trunc(a/b) * b (wasm has no float remainder).
                Rem => {
                    let a = self.alloc_scratch(&[Val::F64]);
                    let b = self.alloc_scratch(&[Val::F64]);
                    self.lower_float_operand(lhs, out, sink);
                    out.push(Inst::LocalSet(a));
                    self.lower_float_operand(rhs, out, sink);
                    out.push(Inst::LocalSet(b));
                    out.push(Inst::LocalGet(a));
                    out.push(Inst::LocalGet(a));
                    out.push(Inst::LocalGet(b));
                    out.push(Inst::F64Bin(F64Op::Div));
                    out.push(Inst::F64Un(F64Un::Trunc));
                    out.push(Inst::LocalGet(b));
                    out.push(Inst::F64Bin(F64Op::Mul));
                    out.push(Inst::F64Bin(F64Op::Sub));
                }
                Eq | NEq | Lt | LtEq | Gt | GtEq => {
                    self.lower_float_operand(lhs, out, sink);
                    self.lower_float_operand(rhs, out, sink);
                    out.push(Inst::F64Cmp(match op {
                        Eq => CmpOp::Eq,
                        NEq => CmpOp::Ne,
                        Lt => CmpOp::LtS,
                        LtEq => CmpOp::LeS,
                        Gt => CmpOp::GtS,
                        _ => CmpOp::GeS,
                    }));
                }
                // `^` is a transcendental — blocked with the rest of
                // clean:bridge/math (DISCOVERIES-M6 item 2).
                Pow => self.note(sink, "exponentiation in compiled code", expr.span),
                And | Or | Default | Is | NotIs => {
                    self.note(sink, "this operand type in compiled code", expr.span)
                }
            }
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

    /// A float-domain operand: integers widen defensively even if pass [5]
    /// missed an `IntToNumber` insertion.
    fn lower_float_operand(
        &mut self,
        operand: &HExpr,
        out: &mut Vec<Inst>,
        sink: &mut DiagnosticSink,
    ) {
        self.expr(operand, out, sink);
        if !matches!(operand.ty, Ty::Number) {
            if !is_i64(&operand.ty) {
                out.push(Inst::I64ExtendI32U);
            }
            out.push(Inst::F64ConvertI64S);
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
