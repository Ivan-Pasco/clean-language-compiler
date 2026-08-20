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
pub mod runtime_any;
pub mod runtime_json;
pub mod runtime_list;
pub mod runtime_num;
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
    /// Module state variables (SMG-01) lowered onto wasm globals, emitted
    /// after the two MMD-01 heap globals in declaration order.
    pub state_globals: Vec<(Val, StateInit)>,
}

/// A state global's constant initializer.
#[derive(Debug, Clone, Copy)]
pub enum StateInit {
    I32(i32),
    I64(i64),
    F64(f64),
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
    /// `Some(ok)` when the world return is `result<ok, err>` (framework
    /// 09 §8): the call takes the retptr form with the result's memory
    /// layout, and call sites branch on the discriminant.
    pub fallible_ok: Option<Ty>,
    /// Canonical offset of the ok payload in the result's memory form.
    pub fallible_payload_offset: u32,
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
    /// `i64.reinterpret_f64` — the IEEE 754 bit pattern, for the exact
    /// mantissa/exponent decomposition `number.toString` needs.
    I64ReinterpretF64,
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
        // An `any` is a single pointer to its box (ADR 0005); `pairs`
        // stays representable only inside `any` for now.
        Ty::Any => vec![Val::I32],
        Ty::Datetime | Ty::Matrix(_) | Ty::Pairs(_, _) => return None,
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

/// What the target world knows about one host import beyond its Clean
/// declaration: the interface's qualified package path, and — when the
/// WIT return is `result<T, E>` — the fallibility the Clean surface never
/// writes (framework 09 §8: fallible functions declare the ok type;
/// `onError` is the error channel).
pub struct WorldFacts {
    /// e.g. `clean:fake-bridge/store@0.1.0`; `None` falls back to the
    /// `clean:host/{interface}@{version}` convention.
    pub module: Option<String>,
    /// `Some(ok)` when the world return is `result<ok, err>`; the err
    /// side's payload is never read (expression `onError` binds no error
    /// value) but its alignment still positions the ok payload.
    pub fallible_ok: Option<Ty>,
    /// Canonical offset of the ok payload inside the result's memory
    /// form: `align_to(1, max(align(ok), align(err)))`.
    pub fallible_payload_offset: u32,
}

/// Canonical ABI alignment of a WIT type (the subset our worlds use).
fn cabi_align(resolve: &wit_parser::Resolve, ty: &wit_parser::Type) -> u32 {
    use wit_parser::{Type as W, TypeDefKind};
    match ty {
        W::Bool | W::U8 | W::S8 => 1,
        W::U16 | W::S16 => 2,
        W::U32 | W::S32 | W::F32 | W::Char | W::String => 4,
        W::U64 | W::S64 | W::F64 => 8,
        W::Id(id) => match &resolve.types[*id].kind {
            TypeDefKind::Enum(_) | TypeDefKind::Flags(_) => 4,
            TypeDefKind::List(_) => 4,
            TypeDefKind::Record(r) => r
                .fields
                .iter()
                .map(|f| cabi_align(resolve, &f.ty))
                .max()
                .unwrap_or(1),
            TypeDefKind::Tuple(t) => t
                .types
                .iter()
                .map(|ty| cabi_align(resolve, ty))
                .max()
                .unwrap_or(1),
            TypeDefKind::Option(inner) => cabi_align(resolve, inner).max(1),
            TypeDefKind::Variant(v) => v
                .cases
                .iter()
                .filter_map(|c| c.ty.as_ref())
                .map(|ty| cabi_align(resolve, ty))
                .max()
                .unwrap_or(1),
            TypeDefKind::Result(r) => {
                r.ok.iter()
                    .chain(r.err.iter())
                    .map(|ty| cabi_align(resolve, ty))
                    .max()
                    .unwrap_or(1)
            }
            TypeDefKind::Type(inner) => cabi_align(resolve, inner),
            _ => 8,
        },
        _ => 8,
    }
}

/// Looks a declared import up in the target world (by interface name and
/// kebab function name). Absence is not an error here — pass [9] owns
/// COM012 — it only means no enrichment.
fn world_facts(world: &crate::codegen::world::ParsedWorld, import: &HostImport) -> WorldFacts {
    use wit_parser::{TypeDefKind, WorldItem, WorldKey};
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
        let module = iface.package.map(|pkg| {
            let pkg = &resolve.packages[pkg];
            let version = pkg
                .name
                .version
                .as_ref()
                .map(|v| format!("@{v}"))
                .unwrap_or_default();
            format!("{}:{}/{}{version}", pkg.name.namespace, pkg.name.name, name)
        });
        let mut fallible_payload_offset = 0;
        let fallible_ok = iface.functions.get(&import.wit_name).and_then(|function| {
            let wit_parser::Type::Id(id) = function.result.as_ref()? else {
                return None;
            };
            let TypeDefKind::Result(r) = &resolve.types[*id].kind else {
                return None;
            };
            // 37cda47: the error payload is discarded, never read — every
            // err shape lowers; its alignment still positions the ok
            // payload after the 1-byte discriminant.
            fallible_payload_offset =
                r.ok.iter()
                    .chain(r.err.iter())
                    .map(|ty| cabi_align(resolve, ty))
                    .max()
                    .unwrap_or(1);
            Some(match r.ok {
                Some(ok) => crate::typecheck::types::project_wit(resolve, &ok).unwrap_or(Ty::Error),
                None => Ty::Void,
            })
        });
        return WorldFacts {
            module,
            fallible_ok,
            fallible_payload_offset,
        };
    }
    WorldFacts {
        module: None,
        fallible_ok: None,
        fallible_payload_offset: 0,
    }
}

pub fn lower(
    program: &HirProgram,
    resolved: &ResolvedAst,
    world: &crate::codegen::world::ParsedWorld,
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
        .map(|&index| {
            let import = &program.host_imports[index];
            let facts = world_facts(world, import);
            lower_import(import, facts, world_version, resolved, sink)
        })
        .collect();
    let imports: Vec<MirImport> = imports;
    let mut data = DataPool::new();
    let mut tags = TagRegistry::default();

    // State variables → wasm globals (SMG-01). Initializers must be
    // constant-representable; anything else stays a frontier note.
    let mut state_globals: Vec<(Val, StateInit)> = Vec::new();
    let mut states: std::collections::BTreeMap<(usize, String), (u32, Ty)> =
        std::collections::BTreeMap::new();
    for var in &program.state_vars {
        let Some(vals) = val_types(&var.ty) else {
            sink.note_unsupported(
                "state variables of this type",
                resolved.span(var.module, var.span),
            );
            continue;
        };
        let Some(inits) = const_state_init(&var.init, &var.ty, &vals, &mut data) else {
            sink.note_unsupported(
                "state initialisers beyond constants",
                resolved.span(var.module, var.init.span),
            );
            continue;
        };
        let base = crate::layout::FIRST_STATE_GLOBAL + state_globals.len() as u32;
        states.insert((var.module, var.name.clone()), (base, var.ty.clone()));
        state_globals.extend(vals.iter().copied().zip(inits));
    }

    let functions = program
        .functions
        .iter()
        .map(|f| {
            lower_function(
                f, program, resolved, &remap, &imports, &states, &mut data, &mut tags, sink,
            )
        })
        .collect();
    let raise_msgs = runtime::RaiseMsgs {
        run003_code: data.intern_string("RUN003") as i32,
        not_an_integer: data.intern_string("the string is not a valid integer literal") as i32,
        not_a_number: data.intern_string("the string is not a valid number literal") as i32,
    };
    MirProgram {
        imports,
        functions,
        runtime: runtime::build(tier, raise_msgs),
        data: data.blob,
        tier,
        state_globals,
    }
}

/// Constant per-slot initializers for a state variable, or `None` when the
/// initializer is not constant-representable.
fn const_state_init(
    init: &HExpr,
    ty: &Ty,
    vals: &[Val],
    data: &mut DataPool,
) -> Option<Vec<StateInit>> {
    match (&init.kind, ty) {
        (HExprKind::Int(v), _) => Some(match vals {
            [Val::I64] => vec![StateInit::I64(*v as i64)],
            [Val::I32] => vec![StateInit::I32(*v as i32)],
            _ => return None,
        }),
        (HExprKind::Bool(v), _) => Some(vec![StateInit::I32(*v as i32)]),
        (HExprKind::Num(v), _) => Some(vec![StateInit::F64(*v)]),
        (HExprKind::EnumCase(i), _) => Some(vec![StateInit::I32(*i as i32)]),
        (HExprKind::Str(v), _) => Some(vec![StateInit::I32(data.intern_string(v) as i32)]),
        (HExprKind::NoneLit, Ty::Option(_)) => Some(
            vals.iter()
                .map(|v| match v {
                    Val::I32 => StateInit::I32(0),
                    Val::I64 => StateInit::I64(0),
                    Val::F64 => StateInit::F64(0.0),
                })
                .collect(),
        ),
        _ => None,
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
            HExprKind::OnError { value, fallback } => {
                expr(value, used);
                expr(fallback, used);
            }
            HExprKind::Raise(operand) => expr(operand, used),
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
        HStmt::SetState { value, .. } => expr(value, used),
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
        // The shared `none` box (ADR 0005): 16 zero bytes, 8-aligned.
        let none = pool.intern_aligned(&[0u8; 16], 8);
        debug_assert_eq!(none, crate::layout::NONE_BOX_ADDR);
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
        self.intern_bytes(value.as_bytes())
    }

    /// Interns a bytes payload as its `[u32 LE length][payload]` object
    /// (§3.4.3: string-shaped, no UTF-8 constraint).
    fn intern_bytes(&mut self, value: &[u8]) -> u32 {
        let mut object = Vec::with_capacity(4 + value.len());
        object.extend_from_slice(&(value.len() as u32).to_le_bytes());
        object.extend_from_slice(value);
        self.intern(&object)
    }
}

fn lower_import(
    import: &HostImport,
    facts: WorldFacts,
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
    let results = if facts.fallible_ok.is_some() {
        // result<T, E> flattens past one core value for every supported
        // shape, so the call always takes the retptr form.
        params.push(Val::I32);
        vec![]
    } else {
        match cabi_flat(&import.ret) {
            Some(vals) if vals.len() <= 1 => vals,
            // Wider results take the Canonical ABI retptr form: a trailing
            // i32 pointer parameter, no core results.
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
        }
    };
    MirImport {
        module: facts
            .module
            .unwrap_or_else(|| format!("clean:host/{}@{}", import.interface, world_version)),
        interface: import.interface.clone(),
        name: import.wit_name.clone(),
        params,
        results,
        fallible_ok: facts.fallible_ok,
        fallible_payload_offset: facts.fallible_payload_offset,
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

#[allow(clippy::too_many_arguments)]
fn lower_function(
    function: &HFunction,
    program: &HirProgram,
    resolved: &ResolvedAst,
    remap: &std::collections::BTreeMap<usize, usize>,
    imports: &[MirImport],
    states: &std::collections::BTreeMap<(usize, String), (u32, Ty)>,
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

    let mut local_tys: Vec<Ty> = function.params.clone();
    local_tys.extend(function.locals.iter().cloned());
    let mut lowerer = FnLowerer {
        program,
        resolved,
        slots,
        slot_widths,
        local_tys,
        file: function.file,
        data,
        next_slot: next,
        scratch: Vec::new(),
        remap,
        imports,
        states,
        label_depth: 0,
        loops: Vec::new(),
        tags,
        ret_vals: results.clone(),
        handlers: Vec::new(),
        error_bindings: Vec::new(),
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
    /// LocalId → declared semantic type (params then locals).
    local_tys: Vec<Ty>,
    file: usize,
    data: &'a mut DataPool,
    /// Next free wasm slot (scratch allocations continue the local space).
    next_slot: u32,
    /// Scratch locals appended after the declared locals.
    scratch: Vec<Val>,
    /// Original host-import index → pruned MIR import index.
    remap: &'a std::collections::BTreeMap<usize, usize>,
    /// The lowered imports (world-enriched: qualified module, fallibility).
    imports: &'a [MirImport],
    /// State variables lowered to globals: (module, name) → (base, Ty).
    states: &'a std::collections::BTreeMap<(usize, String), (u32, Ty)>,
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
    /// The function's flattened result slots, for the dummy values a
    /// propagating failure returns with (13 §ERH: the flag stays set, the
    /// caller checks).
    ret_vals: Vec<Val>,
    /// Innermost-last absolute label indices of the active `onError`
    /// raised-path blocks: a raise or a propagating callee failure
    /// branches to the innermost one, or returns when none is active.
    handlers: Vec<u32>,
    /// Innermost-last `(message, code)` scratch slots holding the caught
    /// failure while its handler runs — the `error` binding (ERH-04) reads
    /// these, so a nested catch cannot clobber an outer binding.
    error_bindings: Vec<(u32, u32)>,
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
        // Out of range raises RUN013 (ERH-03 family), catchable.
        self.emit_raise_run013_if(
            "list",
            idx64,
            vec![
                Inst::LocalGet(base),
                Inst::I32Load(crate::layout::LIST_LEN_OFFSET),
                Inst::I64ExtendI32U,
            ],
            out,
        );
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
        // An empty collection has no first/last element: RUN013,
        // catchable, with the template's fields both zero.
        self.emit_raise_if(
            "RUN013",
            "Index 0 is out of range for a list of length 0",
            out,
        );
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

    /// One dummy per result slot — the values a propagating failure
    /// returns with (never observed: the caller checks the flag first).
    fn emit_ret_dummies(&self, out: &mut Vec<Inst>) {
        for val in &self.ret_vals {
            out.push(match val {
                Val::I32 => Inst::I32Const(0),
                Val::I64 => Inst::I64Const(0),
                Val::F64 => Inst::F64Const(0.0),
            });
        }
    }

    /// Unconditional unwind of a raised failure: branch to the innermost
    /// `onError` raised path, or leave the function with the flag still
    /// set for the caller's check (13 §ERH-02/05).
    fn emit_unwind(&mut self, out: &mut Vec<Inst>) {
        match self.handlers.last() {
            Some(&raised_abs) => out.push(Inst::Br(self.label_depth - 1 - raised_abs)),
            None => {
                self.emit_ret_dummies(out);
                out.push(Inst::Return);
            }
        }
    }

    /// Raise (13 §ERH-01/03) with `error.message` already on the stack:
    /// store message and code, set the flag, unwind. `code` is the
    /// registered runtime code, or `None` for a program's own `error(...)`.
    fn emit_raise_with_message_on_stack(&mut self, code: Option<&str>, out: &mut Vec<Inst>) {
        out.push(Inst::GlobalSet(crate::layout::ERR_MSG_GLOBAL));
        let code_base = code.map_or(0, |c| self.data.intern_string(c) as i32);
        out.push(Inst::I32Const(code_base));
        out.push(Inst::GlobalSet(crate::layout::ERR_CODE_GLOBAL));
        out.push(Inst::I32Const(1));
        out.push(Inst::GlobalSet(crate::layout::ERR_FLAG_GLOBAL));
        self.emit_unwind(out);
    }

    /// Conditional raise: with an i32 condition on the stack, raise the
    /// registered runtime code with a static message when it is true.
    /// The message wordings are local pinnings — Platform 10 defines no
    /// runtime-message templates for the RUN003 family (DISCOVERIES-M8).
    fn emit_raise_if(&mut self, code: &str, message: &str, out: &mut Vec<Inst>) {
        let msg = self.data.intern_string(message) as i32;
        let mut then = Vec::new();
        // The If arm is a labeled block; the unwind branch accounts for it.
        self.label_depth += 1;
        then.push(Inst::I32Const(msg));
        self.emit_raise_with_message_on_stack(Some(code), &mut then);
        self.label_depth -= 1;
        out.push(Inst::If {
            result: None,
            then,
            els: vec![],
        });
    }

    /// Conditional RUN013 raise with the Platform 10 template — "Index
    /// {index} is out of range for a {kind} of length {length}" — built at
    /// raise time from the index slot and the length instructions (both
    /// i64). The i32 condition is already on the stack.
    fn emit_raise_run013_if(
        &mut self,
        kind: &str,
        idx64: u32,
        len_insts: Vec<Inst>,
        out: &mut Vec<Inst>,
    ) {
        let head = self.data.intern_string("Index ") as i32;
        let mid = self
            .data
            .intern_string(&format!(" is out of range for a {kind} of length "))
            as i32;
        let mut then = Vec::new();
        self.label_depth += 1;
        then.push(Inst::I32Const(head));
        then.push(Inst::LocalGet(idx64));
        then.push(Inst::CallRuntime(runtime::RuntimeFn::IntToString));
        then.push(Inst::CallRuntime(runtime::RuntimeFn::StringConcat));
        then.push(Inst::I32Const(mid));
        then.push(Inst::CallRuntime(runtime::RuntimeFn::StringConcat));
        then.extend(len_insts);
        then.push(Inst::CallRuntime(runtime::RuntimeFn::IntToString));
        then.push(Inst::CallRuntime(runtime::RuntimeFn::StringConcat));
        self.emit_raise_with_message_on_stack(Some("RUN013"), &mut then);
        self.label_depth -= 1;
        out.push(Inst::If {
            result: None,
            then,
            els: vec![],
        });
    }

    /// After any call that can raise: if the flag is set, propagate — to
    /// the innermost handler, or out of the function.
    fn emit_propagate_check(&mut self, out: &mut Vec<Inst>) {
        out.push(Inst::GlobalGet(crate::layout::ERR_FLAG_GLOBAL));
        match self.handlers.last() {
            Some(&raised_abs) => out.push(Inst::BrIf(self.label_depth - 1 - raised_abs)),
            None => {
                let mut then = Vec::new();
                self.emit_ret_dummies(&mut then);
                then.push(Inst::Return);
                out.push(Inst::If {
                    result: None,
                    then,
                    els: vec![],
                });
            }
        }
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
                let declared = self.local_tys[*local].clone();
                // TYP-02 coercions at the store: box into `any`, unbox out
                // of it (ADR 0005).
                if matches!(declared, Ty::Any)
                    && !matches!(value.ty, Ty::Any)
                    && !self.emit_any_box(&value.ty, out)
                {
                    self.note(sink, "boxing this type into `any`", value.span);
                    return;
                }
                if matches!(value.ty, Ty::Any)
                    && !matches!(declared, Ty::Any)
                    && !self.emit_any_unbox(&declared, out)
                {
                    self.note(sink, "unboxing `any` into this type", value.span);
                    return;
                }
                // A narrower boundary integer widens into an `integer`
                // local (and vice versa) — the checker accepts the fit,
                // the store adjusts the width.
                match (is_i64(&value.ty), is_i64(&declared)) {
                    (false, true) if value.ty.is_integer() => {
                        out.push(if matches!(value.ty, Ty::IntegerW(IntWidth::S32)) {
                            Inst::I64ExtendI32S
                        } else {
                            Inst::I64ExtendI32U
                        });
                    }
                    (true, false) if declared.is_integer() => {
                        out.push(Inst::I32WrapI64);
                    }
                    _ => {}
                }
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
            // SMG-01: state writes hit the lowered globals, with the same
            // TYP-02 box/unbox coercions as local stores.
            HStmt::SetState {
                module,
                name,
                value,
            } => {
                let Some((base, declared)) = self.states.get(&(*module, name.clone())).cloned()
                else {
                    self.note(sink, "state assignment", value.span);
                    return;
                };
                self.expr(value, out, sink);
                if matches!(declared, Ty::Any)
                    && !matches!(value.ty, Ty::Any)
                    && !self.emit_any_box(&value.ty, out)
                {
                    self.note(sink, "boxing this type into `any`", value.span);
                    return;
                }
                if matches!(value.ty, Ty::Any)
                    && !matches!(declared, Ty::Any)
                    && !self.emit_any_unbox(&declared, out)
                {
                    self.note(sink, "unboxing `any` into this type", value.span);
                    return;
                }
                match (is_i64(&value.ty), is_i64(&declared)) {
                    (false, true) if value.ty.is_integer() => {
                        out.push(if matches!(value.ty, Ty::IntegerW(IntWidth::S32)) {
                            Inst::I64ExtendI32S
                        } else {
                            Inst::I64ExtendI32U
                        });
                    }
                    (true, false) if declared.is_integer() => {
                        out.push(Inst::I32WrapI64);
                    }
                    _ => {}
                }
                let width = val_types(&declared).map(|v| v.len()).unwrap_or(1) as u32;
                for offset in (0..width).rev() {
                    out.push(Inst::GlobalSet(base + offset));
                }
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
                // Fallible imports (world return result<T, E> — framework
                // 09 §8): a bare call traps on the error arm (RUN018's
                // shape until error lowering; `onError` is the handled
                // form, lowered by the OnError arm below).
                if self.imports[self.remap[import]].fallible_ok.is_some() {
                    self.lower_fallible_call(*import, args, None, expr.span, out, sink);
                    return;
                }
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
                // Any user function can raise (ERH-01); propagate a set
                // flag before the result is used.
                self.emit_propagate_check(out);
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
            // LEX-06 `b"…"`: an interned `[len][payload]` object — the
            // layout is string-shaped without the UTF-8 constraint
            // (§3.4.3).
            HExprKind::BytesLit(value) => {
                let base = self.data.intern_bytes(value);
                out.push(Inst::I32Const(base as i32));
            }
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
                // §14.14.2: single-byte read, out of range traps.
                tir::IndexKind::Bytes => {
                    let base = self.alloc_scratch(&[Val::I32]);
                    let idx64 = self.alloc_scratch(&[Val::I64]);
                    self.expr(recv, out, sink);
                    out.push(Inst::LocalSet(base));
                    self.expr(index, out, sink);
                    if !is_i64(&index.ty) {
                        out.push(Inst::I64ExtendI32U);
                    }
                    out.push(Inst::LocalTee(idx64));
                    out.push(Inst::LocalGet(base));
                    out.push(Inst::I32Load(0));
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
                    out.push(Inst::I32Bin(I32Op::Add));
                    out.push(Inst::I32Load8U(4));
                    out.push(Inst::I64ExtendI32U);
                }
                // 15 §Accessing JSON Data: lookups yield `none` for a
                // missing key, an out-of-range index, or a non-container
                // box — never a trap.
                tir::IndexKind::Any => {
                    self.expr(recv, out, sink);
                    match &index.ty {
                        Ty::Integer | Ty::IntegerW(_) => {
                            self.expr(index, out, sink);
                            if !is_i64(&index.ty) {
                                out.push(Inst::I64ExtendI32U);
                            }
                            out.push(Inst::CallRuntime(runtime::RuntimeFn::AnyIndexInt));
                        }
                        Ty::Str => {
                            self.expr(index, out, sink);
                            out.push(Inst::CallRuntime(runtime::RuntimeFn::AnyIndexStr));
                        }
                        _ => {
                            out.push(Inst::Drop);
                            self.note(sink, "index access", expr.span);
                        }
                    }
                }
                tir::IndexKind::Matrix | tir::IndexKind::Pairs => {
                    self.note(sink, "index access", expr.span)
                }
            },
            HExprKind::NonNone(_) => self.note(sink, "postfix `!` assertion", expr.span),
            HExprKind::IsNone { operand, negated } if matches!(operand.ty, Ty::Any) => {
                self.expr(operand, out, sink);
                out.push(Inst::CallRuntime(runtime::RuntimeFn::AnyIsNone));
                if *negated {
                    out.push(Inst::I32Eqz);
                }
            }
            HExprKind::IsNone { .. } => self.note(sink, "is-none checks", expr.span),
            HExprKind::ResultRef => self.note(sink, "contract blocks in compiled code", expr.span),
            HExprKind::This => self.note(sink, "class values in compiled code", expr.span),
            HExprKind::GetState { module, name } => {
                match self.states.get(&(*module, name.clone())) {
                    Some((base, ty)) => {
                        let width = val_types(ty).map(|v| v.len()).unwrap_or(1) as u32;
                        for offset in 0..width {
                            out.push(Inst::GlobalGet(base + offset));
                        }
                    }
                    // Computed state derives on read; its lowering waits.
                    None => self.note(sink, "computed state in compiled code", expr.span),
                }
            }
            HExprKind::GuardValue => self.note(sink, "state access in compiled code", expr.span),
            // `value onError fallback` over a fallible host call: branch
            // on the result discriminant, fallback on the error arm (the
            // expression form binds no error value).
            HExprKind::OnError { value, fallback }
                if matches!(&value.kind, HExprKind::CallHost { import, .. }
                    if self.imports[self.remap[import]].fallible_ok.is_some()) =>
            {
                let HExprKind::CallHost { import, args } = &value.kind else {
                    unreachable!("guard matched CallHost");
                };
                self.lower_fallible_call(*import, args, Some(fallback), expr.span, out, sink);
            }
            // An infallible value cannot fail: `onError` is inert and the
            // fallback is dead (M4 typing already coerced it).
            HExprKind::OnError { value, fallback: _ }
                if matches!(&value.kind, HExprKind::CallHost { .. }) =>
            {
                self.expr(value, out, sink);
            }
            // `error(message)` (ERH-01): a signal, not a value — store the
            // message, no code (a program's own failure carries none),
            // set the flag, unwind.
            HExprKind::Raise(message) => {
                self.expr(message, out, sink);
                self.emit_raise_with_message_on_stack(None, out);
            }
            // General `onError` (ERH-02): run the protected expression
            // with a raised-path landing block armed; a raise inside it —
            // or inside anything it calls — lands in the fallback with
            // the failure bound to `error`.
            HExprKind::OnError { value, fallback } => {
                let Some(result_vals) = val_types(&expr.ty) else {
                    self.note(sink, "onError over this value type", expr.span);
                    return;
                };
                let result_base = self.alloc_scratch(&result_vals);
                let base = self.label_depth;

                // Inner block: the protected expression; success stores
                // the value and jumps past the raised path.
                let mut inner = Vec::new();
                self.label_depth += 2;
                self.handlers.push(base + 1);
                self.expr(value, &mut inner, sink);
                self.handlers.pop();
                for offset in (0..result_vals.len() as u32).rev() {
                    inner.push(Inst::LocalSet(result_base + offset));
                }
                inner.push(Inst::Br(1));
                self.label_depth -= 2;

                // Raised path: copy the failure into locals (so a nested
                // catch cannot clobber the binding — ERH-04), clear the
                // flag, run the fallback.
                let msg_slot = self.alloc_scratch(&[Val::I32]);
                let code_slot = self.alloc_scratch(&[Val::I32]);
                let mut raised = vec![
                    Inst::GlobalGet(crate::layout::ERR_MSG_GLOBAL),
                    Inst::LocalSet(msg_slot),
                    Inst::GlobalGet(crate::layout::ERR_CODE_GLOBAL),
                    Inst::LocalSet(code_slot),
                    Inst::I32Const(0),
                    Inst::GlobalSet(crate::layout::ERR_FLAG_GLOBAL),
                ];
                self.label_depth += 1;
                self.error_bindings.push((msg_slot, code_slot));
                self.expr(fallback, &mut raised, sink);
                self.error_bindings.pop();
                self.label_depth -= 1;
                for offset in (0..result_vals.len() as u32).rev() {
                    raised.push(Inst::LocalSet(result_base + offset));
                }

                let mut outer = vec![Inst::Block { body: inner }];
                outer.extend(raised);
                out.push(Inst::Block { body: outer });
                for offset in 0..result_vals.len() as u32 {
                    out.push(Inst::LocalGet(result_base + offset));
                }
            }
            // The `error` binding (ERH-04), as the flattened Error record:
            // [message, code disc, code payload].
            HExprKind::ErrorBinding => {
                let Some((msg_slot, code_slot)) = self.error_bindings.last().copied() else {
                    self.note(sink, "the error binding outside a handler", expr.span);
                    return;
                };
                out.push(Inst::LocalGet(msg_slot));
                out.push(Inst::LocalGet(code_slot));
                out.push(Inst::I32Const(0));
                out.push(Inst::I32Cmp(CmpOp::Ne));
                out.push(Inst::LocalGet(code_slot));
            }
            // `error.message` / `error.code` — the two fields of the
            // built-in Error record, read from the binding's locals.
            HExprKind::GetRecordField { recv, field }
                if matches!(recv.kind, HExprKind::ErrorBinding) =>
            {
                let Some((msg_slot, code_slot)) = self.error_bindings.last().copied() else {
                    self.note(sink, "the error binding outside a handler", expr.span);
                    return;
                };
                match field {
                    0 => out.push(Inst::LocalGet(msg_slot)),
                    _ => {
                        out.push(Inst::LocalGet(code_slot));
                        out.push(Inst::I32Const(0));
                        out.push(Inst::I32Cmp(CmpOp::Ne));
                        out.push(Inst::LocalGet(code_slot));
                    }
                }
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
                    // Truncate toward zero; NaN or out-of-range raises
                    // RUN003 (ERH-03) — the wasm trunc would trap.
                    (Ty::Number, Ty::Integer) => {
                        self.expr(operand, out, sink);
                        let f = self.alloc_scratch(&[Val::F64]);
                        out.push(Inst::LocalTee(f));
                        out.push(Inst::LocalGet(f));
                        out.push(Inst::F64Cmp(CmpOp::Ne));
                        self.emit_raise_if("RUN003", "cannot convert NaN to integer", out);
                        out.push(Inst::LocalGet(f));
                        out.push(Inst::F64Const(9_223_372_036_854_775_808.0));
                        out.push(Inst::F64Cmp(CmpOp::GeS));
                        out.push(Inst::LocalGet(f));
                        out.push(Inst::F64Const(-9_223_372_036_854_775_808.0));
                        out.push(Inst::F64Cmp(CmpOp::LtS));
                        out.push(Inst::I32Bin(I32Op::Or));
                        self.emit_raise_if("RUN003", "number is out of the integer range", out);
                        out.push(Inst::LocalGet(f));
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
                    // Parses; a non-literal raises RUN003 inside the
                    // runtime helper — propagate it here (ERH-03).
                    (Ty::Str, Ty::Integer) => {
                        self.expr(operand, out, sink);
                        out.push(Inst::CallRuntime(runtime::RuntimeFn::StrToInt));
                        self.emit_propagate_check(out);
                    }
                    (Ty::Str, Ty::Number) => {
                        self.expr(operand, out, sink);
                        out.push(Inst::CallRuntime(runtime::RuntimeFn::StrToNum));
                        self.emit_propagate_check(out);
                    }
                    // 15 §Conversions: the shortest round-trip rendering,
                    // computed exactly in the guest (runtime_num.rs).
                    (Ty::Number, Ty::Str) => {
                        self.expr(operand, out, sink);
                        out.push(Inst::CallRuntime(runtime::RuntimeFn::NumToString));
                    }
                    // string.toBoolean is not in the 15 §Conversions table
                    // at all.
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
                match func {
                    StdFn::JsonTextToData | StdFn::JsonTryTextToData => {
                        self.expr(&args[0], out, sink);
                        out.push(Inst::CallRuntime(
                            if matches!(func, StdFn::JsonTextToData) {
                                runtime::RuntimeFn::JsonParse
                            } else {
                                runtime::RuntimeFn::JsonTryParse
                            },
                        ));
                        return;
                    }
                    StdFn::JsonDataToText | StdFn::JsonPrettyDataToText => {
                        let arg = &args[0];
                        self.expr(arg, out, sink);
                        if !matches!(arg.ty, Ty::Any) && !self.emit_any_box(&arg.ty, out) {
                            self.note(sink, "boxing this type into `any`", arg.span);
                            return;
                        }
                        out.push(Inst::CallRuntime(
                            if matches!(func, StdFn::JsonDataToText) {
                                runtime::RuntimeFn::JsonSerialize
                            } else {
                                runtime::RuntimeFn::JsonSerializePretty
                            },
                        ));
                        return;
                    }
                    // Code-point index out of range raises RUN013
                    // (catchable) before the runtime helper runs — the
                    // helper's own guard would trap instead.
                    StdFn::StrCharAt | StdFn::StrCharCodeAt => {
                        let s_slot = self.alloc_scratch(&[Val::I32]);
                        let i_slot = self.alloc_scratch(&[Val::I64]);
                        self.expr(&args[0], out, sink);
                        out.push(Inst::LocalSet(s_slot));
                        self.expr(&args[1], out, sink);
                        out.push(Inst::LocalSet(i_slot));
                        out.push(Inst::LocalGet(i_slot));
                        out.push(Inst::LocalGet(s_slot));
                        out.push(Inst::CallRuntime(runtime::RuntimeFn::StrCpLen));
                        out.push(Inst::I64Cmp(CmpOp::LtU));
                        out.push(Inst::I32Eqz);
                        self.emit_raise_run013_if(
                            "string",
                            i_slot,
                            vec![
                                Inst::LocalGet(s_slot),
                                Inst::CallRuntime(runtime::RuntimeFn::StrCpLen),
                            ],
                            out,
                        );
                        out.push(Inst::LocalGet(s_slot));
                        out.push(Inst::LocalGet(i_slot));
                        out.push(Inst::CallRuntime(if matches!(func, StdFn::StrCharAt) {
                            runtime::RuntimeFn::StrCharAt
                        } else {
                            runtime::RuntimeFn::StrCharCodeAt
                        }));
                        return;
                    }
                    _ => {}
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
                    StdFn::BytesLength => {
                        out.push(Inst::I32Load(0));
                        out.push(Inst::I64ExtendI32U);
                    }
                    // Identical immutable layouts: fromText is the identity.
                    StdFn::BytesFromText => {}
                    StdFn::BytesSlice => {
                        out.push(Inst::CallRuntime(runtime::RuntimeFn::BytesSlice))
                    }
                    StdFn::BytesToText => {
                        out.push(Inst::CallRuntime(runtime::RuntimeFn::BytesToText))
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

    /// Boxes the concrete value on the stack into an `any` box (ADR
    /// 0005); false when the type has no boxing yet.
    fn emit_any_box(&mut self, from: &Ty, out: &mut Vec<Inst>) -> bool {
        use runtime::RuntimeFn;
        match from {
            Ty::Str => out.push(Inst::CallRuntime(RuntimeFn::AnyBoxStr)),
            Ty::Bytes => out.push(Inst::CallRuntime(RuntimeFn::AnyBoxStr)),
            Ty::Boolean => out.push(Inst::CallRuntime(RuntimeFn::AnyBoxBool)),
            Ty::Number => out.push(Inst::CallRuntime(RuntimeFn::AnyBoxNum)),
            Ty::Integer | Ty::IntegerW(IntWidth::U64) => {
                out.push(Inst::CallRuntime(RuntimeFn::AnyBoxInt))
            }
            Ty::IntegerW(IntWidth::S32) => {
                out.push(Inst::I64ExtendI32S);
                out.push(Inst::CallRuntime(RuntimeFn::AnyBoxInt));
            }
            Ty::IntegerW(_) | Ty::Enum { .. } => {
                out.push(Inst::I64ExtendI32U);
                out.push(Inst::CallRuntime(RuntimeFn::AnyBoxInt));
            }
            _ => return false,
        }
        true
    }

    /// Unboxes the `any` box on the stack into a concrete value, trapping
    /// on a tag mismatch (RUN005 family until error lowering); false when
    /// the target has no unboxing yet.
    fn emit_any_unbox(&mut self, to: &Ty, out: &mut Vec<Inst>) -> bool {
        use runtime::RuntimeFn;
        match to {
            Ty::Str => out.push(Inst::CallRuntime(RuntimeFn::AnyUnboxStr)),
            Ty::Boolean => out.push(Inst::CallRuntime(RuntimeFn::AnyUnboxBool)),
            Ty::Number => out.push(Inst::CallRuntime(RuntimeFn::AnyUnboxNum)),
            Ty::Integer => out.push(Inst::CallRuntime(RuntimeFn::AnyUnboxInt)),
            Ty::IntegerW(IntWidth::U64) => out.push(Inst::CallRuntime(RuntimeFn::AnyUnboxInt)),
            Ty::IntegerW(_) => {
                out.push(Inst::CallRuntime(RuntimeFn::AnyUnboxInt));
                out.push(Inst::I32WrapI64);
            }
            _ => return false,
        }
        true
    }

    /// Lowers a call to a fallible import (world return `result<ok, err>`).
    /// The retptr area holds the canonical result: discriminant byte at
    /// +0, ok payload at its natural alignment. The error payload is never
    /// read (expression `onError` binds no error value); a bare call traps
    /// on the error arm.
    fn lower_fallible_call(
        &mut self,
        import: usize,
        args: &[HExpr],
        fallback: Option<&HExpr>,
        span: crate::source::ByteSpan,
        out: &mut Vec<Inst>,
        sink: &mut DiagnosticSink,
    ) {
        let mir_import = &self.imports[self.remap[&import]];
        let ok_ty = mir_import
            .fallible_ok
            .clone()
            .expect("caller checked fallibility");
        let off = mir_import.fallible_payload_offset;
        let param_tys = self.program.host_imports[import].params.clone();
        for (arg, param_ty) in args.iter().zip(&param_tys) {
            self.lower_boundary_arg(arg, param_ty, out, sink);
        }
        out.push(Inst::RetAreaPtr);
        out.push(Inst::CallImport(self.remap[&import] as u32));

        // The ok payload's loads, at its canonical offset.
        let ok_loads: Option<Vec<Inst>> = match &ok_ty {
            Ty::Void => Some(vec![]),
            Ty::Integer | Ty::IntegerW(IntWidth::U64) => {
                Some(vec![Inst::RetAreaPtr, Inst::I64Load(off)])
            }
            Ty::Number => Some(vec![Inst::RetAreaPtr, Inst::F64Load(off)]),
            Ty::IntegerW(_) | Ty::Boolean | Ty::Enum { .. } => {
                Some(vec![Inst::RetAreaPtr, Inst::I32Load(off)])
            }
            Ty::Str | Ty::Bytes => Some(vec![
                Inst::RetAreaPtr,
                Inst::I32Load(off),
                Inst::RetAreaPtr,
                Inst::I32Load(off + 4),
                Inst::CallRuntime(runtime::RuntimeFn::LiftString),
            ]),
            _ => None,
        };
        let Some(ok_loads) = ok_loads else {
            self.note(sink, "this host result type", span);
            return;
        };
        let result_val = val_types(&ok_ty).and_then(|v| v.first().copied());

        // The host's error payload never surfaces to the program (LBS
        // §8.3): the binding a handler sees carries a generic message and
        // no code — local wording, DISCOVERIES-M8.
        let host_failure_msg = self.data.intern_string(&format!(
            "host function `{}` failed",
            self.program.host_imports[import].clean_name
        )) as i32;

        out.push(Inst::RetAreaPtr);
        out.push(Inst::I32Load8U(0));
        match fallback {
            None => {
                // Error arm: raise through the ordinary chapter-13 path
                // (LBS §8.3) — unhandled at the top it is RUN018.
                let mut err_arm = Vec::new();
                self.label_depth += 1;
                err_arm.push(Inst::I32Const(host_failure_msg));
                self.emit_raise_with_message_on_stack(None, &mut err_arm);
                self.label_depth -= 1;
                out.push(Inst::If {
                    result: None,
                    then: err_arm,
                    els: vec![],
                });
                out.extend(ok_loads);
            }
            Some(fb) => {
                // Direct branch — no flag round-trip — but the handler
                // still binds `error` (ERH-04), with the generic message.
                let msg_slot = self.alloc_scratch(&[Val::I32]);
                let code_slot = self.alloc_scratch(&[Val::I32]);
                let mut err_arm = vec![
                    Inst::I32Const(host_failure_msg),
                    Inst::LocalSet(msg_slot),
                    Inst::I32Const(0),
                    Inst::LocalSet(code_slot),
                ];
                self.label_depth += 1;
                self.error_bindings.push((msg_slot, code_slot));
                self.expr(fb, &mut err_arm, sink);
                self.error_bindings.pop();
                self.label_depth -= 1;
                out.push(Inst::If {
                    result: result_val,
                    then: err_arm,
                    els: ok_loads,
                });
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
                // base (MMD-04). An `any` argument unboxes first.
                let base = self.alloc_scratch(&[Val::I32]);
                self.expr(arg, out, sink);
                if matches!(arg.ty, Ty::Any) {
                    out.push(Inst::CallRuntime(runtime::RuntimeFn::AnyUnboxStr));
                }
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
                // TYP-02: an `any` value meeting a concrete parameter
                // unboxes at the boundary (trap on tag mismatch).
                if matches!(arg.ty, Ty::Any) && !matches!(param_ty, Ty::Any) {
                    if !self.emit_any_unbox(param_ty, out) {
                        self.note(sink, "unboxing `any` into this type", arg.span);
                        return;
                    }
                    return;
                }
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
        if matches!(lhs.ty, Ty::Bytes) && matches!(op, Add | Eq | NEq) {
            // §14.14.2: `bytes + bytes` and `==` — the layout is string-
            // shaped, so the string helpers apply byte-for-byte.
            self.expr(lhs, out, sink);
            self.expr(rhs, out, sink);
            match op {
                Add => out.push(Inst::CallRuntime(runtime::RuntimeFn::StringConcat)),
                Eq => out.push(Inst::CallRuntime(runtime::RuntimeFn::StringEq)),
                _ => {
                    out.push(Inst::CallRuntime(runtime::RuntimeFn::StringEq));
                    out.push(Inst::I32Eqz);
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
                // The arm is a labeled block: unwind branches inside the
                // fallback must account for it.
                self.label_depth += 1;
                self.expr(rhs, &mut els, sink);
                self.label_depth -= 1;
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
                // The arm is a labeled block (unwind branch accounting).
                self.label_depth += 1;
                self.expr(rhs, &mut rhs_body, sink);
                self.label_depth -= 1;
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
            Add | Sub | Mul => {
                self.lower_arith_operand(lhs, out, sink);
                self.lower_arith_operand(rhs, out, sink);
                out.push(Inst::I64Bin(match op {
                    Add => I64Op::Add,
                    Sub => I64Op::Sub,
                    _ => I64Op::Mul,
                }));
            }
            // ERH-03: arithmetic failure raises RUN003, catchable — the
            // wasm div instructions would trap instead, so guard first.
            Div | Rem => {
                self.lower_arith_operand(lhs, out, sink);
                self.lower_arith_operand(rhs, out, sink);
                let rhs_slot = self.alloc_scratch(&[Val::I64]);
                let lhs_slot = self.alloc_scratch(&[Val::I64]);
                out.push(Inst::LocalSet(rhs_slot));
                out.push(Inst::LocalSet(lhs_slot));
                out.push(Inst::LocalGet(rhs_slot));
                out.push(Inst::I64Const(0));
                out.push(Inst::I64Cmp(CmpOp::Eq));
                self.emit_raise_if("RUN003", "division by zero", out);
                if matches!(op, Div) {
                    // i64.div_s also traps on MIN / -1.
                    out.push(Inst::LocalGet(lhs_slot));
                    out.push(Inst::I64Const(i64::MIN));
                    out.push(Inst::I64Cmp(CmpOp::Eq));
                    out.push(Inst::LocalGet(rhs_slot));
                    out.push(Inst::I64Const(-1));
                    out.push(Inst::I64Cmp(CmpOp::Eq));
                    out.push(Inst::I32Bin(I32Op::And));
                    self.emit_raise_if("RUN003", "integer overflow in division", out);
                }
                out.push(Inst::LocalGet(lhs_slot));
                out.push(Inst::LocalGet(rhs_slot));
                out.push(Inst::I64Bin(if matches!(op, Div) {
                    I64Op::DivS
                } else {
                    I64Op::RemS
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
