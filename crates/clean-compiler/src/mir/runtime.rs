//! Always-emitted runtime helpers (ADR 0004): the MMD-02 bump allocator
//! and the MMD-04 string operations, built as ordinary [`MirFunction`]s so
//! emission and snapshots treat them like any other code. Their function
//! indices follow the user functions in [`RuntimeFn`] discriminant order.
//!
//! `clean:bridge/*` always-on imports (BRG-05) are deferred — see the ADR
//! and DISCOVERIES-M6 item 1; these guest functions implement the same
//! observable semantics.

use crate::layout::{Tier, ALIGNMENT, EMPTY_STRING_ADDR, HEAP_PTR_GLOBAL, WASM_PAGE_SIZE};

use super::{CmpOp, I32Op, Inst, MirFunction, Val};

/// The helper set, in emitted order. `CallRuntime(f)` resolves to
/// `import_count + user_function_count + f as u32`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeFn {
    /// `alloc(size, align) -> ptr` — MMD-02: aligned bump allocation;
    /// grows per TIER-02; traps (never returns a failure value) when the
    /// tier limit cannot satisfy the request.
    Alloc = 0,
    /// `string_concat(a, b) -> base` — fresh `[len][payload]` object;
    /// `a + b == ""` returns the shared empty-string constant.
    StringConcat = 1,
    /// `string_compare(a, b) -> i32` — **the** comparison convention:
    /// returns 0 iff equal, otherwise the sign of the first differing byte
    /// (or of the length difference for equal prefixes). Every equality
    /// and ordering derives from this single definition (KNOWLEDGE §2).
    StringCompare = 2,
    /// `string_eq(a, b) -> i32` — 1 iff equal: length fast path, then
    /// `string_compare == 0`.
    StringEq = 3,
    /// `lift_string(ptr, len) -> base` — copies a Canonical ABI payload
    /// the host wrote into a fresh `[len][payload]` object (§3.7: values
    /// crossing the boundary are copies).
    LiftString = 4,
}

pub fn build(tier: Tier) -> Vec<MirFunction> {
    vec![
        alloc(tier),
        string_concat(),
        string_compare(),
        string_eq(),
        lift_string(),
    ]
}

fn function(
    name: &str,
    params: &[Val],
    results: &[Val],
    locals: &[Val],
    body: Vec<Inst>,
) -> MirFunction {
    MirFunction {
        name: name.to_string(),
        params: params.to_vec(),
        results: results.to_vec(),
        locals: locals.to_vec(),
        body,
        export: false,
    }
}

/// MMD-02 §3.2.1 + TIER-02 §5.2. Params: size@0, align@1. Locals:
/// aligned@2, new_ptr@3, cur@4, target@5, floor@6.
fn alloc(tier: Tier) -> MirFunction {
    use Inst::*;
    let max = tier.max_bytes as i32;
    let body = vec![
        // aligned = (heap_ptr + align - 1) & !(align - 1)
        GlobalGet(HEAP_PTR_GLOBAL),
        LocalGet(1),
        I32Bin(I32Op::Add),
        I32Const(1),
        I32Bin(I32Op::Sub),
        LocalGet(1),
        I32Const(1),
        I32Bin(I32Op::Sub),
        I32Const(-1),
        I32Bin(I32Op::Xor),
        I32Bin(I32Op::And),
        LocalTee(2),
        // new_ptr = aligned + size; unsigned wrap means the request itself
        // was absurd — trap.
        LocalGet(0),
        I32Bin(I32Op::Add),
        LocalTee(3),
        LocalGet(2),
        I32Cmp(CmpOp::LtU),
        If {
            result: None,
            then: vec![Unreachable],
            els: vec![],
        },
        // Tier ceiling (MEM001 is the host's attribution of this trap).
        LocalGet(3),
        I32Const(max),
        I32Cmp(CmpOp::GtU),
        If {
            result: None,
            then: vec![Unreachable],
            els: vec![],
        },
        // cur = memory.size * 64Ki
        MemorySize,
        I32Const(16),
        I32Bin(I32Op::Shl),
        LocalSet(4),
        // Grow when the commit would pass the current end (TIER-02).
        LocalGet(3),
        LocalGet(4),
        I32Cmp(CmpOp::GtU),
        If {
            result: None,
            then: vec![
                // target = cur * 3 / 2   (1.5× amortized; cur ≤ 64 MiB so
                // the ×3 cannot overflow i32)
                LocalGet(4),
                I32Const(3),
                I32Bin(I32Op::Mul),
                I32Const(2),
                I32Bin(I32Op::DivU),
                LocalSet(5),
                // target = max(target, cur + 4 pages)
                LocalGet(5),
                LocalGet(4),
                I32Const(4 * WASM_PAGE_SIZE as i32),
                I32Bin(I32Op::Add),
                LocalTee(6),
                LocalGet(5),
                LocalGet(6),
                I32Cmp(CmpOp::GtU),
                Select,
                LocalSet(5),
                // target = max(target, new_ptr)
                LocalGet(5),
                LocalGet(3),
                LocalGet(5),
                LocalGet(3),
                I32Cmp(CmpOp::GtU),
                Select,
                LocalSet(5),
                // target = min(target, tier max) — never speculate past the
                // tier; new_ptr already passed the ceiling guard, so the
                // clipped target still covers it.
                LocalGet(5),
                I32Const(max),
                LocalGet(5),
                I32Const(max),
                I32Cmp(CmpOp::LtU),
                Select,
                LocalSet(5),
                // pages = ceil((target - cur) / 64Ki)
                LocalGet(5),
                LocalGet(4),
                I32Bin(I32Op::Sub),
                I32Const(WASM_PAGE_SIZE as i32 - 1),
                I32Bin(I32Op::Add),
                I32Const(16),
                I32Bin(I32Op::ShrU),
                MemoryGrow,
                I32Const(-1),
                I32Cmp(CmpOp::Eq),
                If {
                    result: None,
                    then: vec![Unreachable],
                    els: vec![],
                },
            ],
            els: vec![],
        },
        // Commit and return the aligned base.
        LocalGet(3),
        GlobalSet(HEAP_PTR_GLOBAL),
        LocalGet(2),
    ];
    function(
        "__clean_alloc",
        &[Val::I32, Val::I32],
        &[Val::I32],
        &[Val::I32; 5],
        body,
    )
}

/// Params: a@0, b@1. Locals: la@2, lb@3, r@4.
fn string_concat() -> MirFunction {
    use Inst::*;
    let body = vec![
        LocalGet(0),
        I32Load(0),
        LocalSet(2),
        LocalGet(1),
        I32Load(0),
        LocalSet(3),
        // "" + "" is the shared constant, never an allocation.
        LocalGet(2),
        LocalGet(3),
        I32Bin(I32Op::Add),
        I32Eqz,
        If {
            result: None,
            then: vec![I32Const(EMPTY_STRING_ADDR as i32), Return],
            els: vec![],
        },
        // r = alloc(4 + la + lb, 8)
        LocalGet(2),
        LocalGet(3),
        I32Bin(I32Op::Add),
        I32Const(4),
        I32Bin(I32Op::Add),
        I32Const(ALIGNMENT as i32),
        CallRuntime(RuntimeFn::Alloc),
        LocalTee(4),
        LocalGet(2),
        LocalGet(3),
        I32Bin(I32Op::Add),
        I32Store(0),
        // copy a's payload to r+4
        LocalGet(4),
        I32Const(4),
        I32Bin(I32Op::Add),
        LocalGet(0),
        I32Const(4),
        I32Bin(I32Op::Add),
        LocalGet(2),
        MemoryCopy,
        // copy b's payload to r+4+la
        LocalGet(4),
        I32Const(4),
        I32Bin(I32Op::Add),
        LocalGet(2),
        I32Bin(I32Op::Add),
        LocalGet(1),
        I32Const(4),
        I32Bin(I32Op::Add),
        LocalGet(3),
        MemoryCopy,
        LocalGet(4),
    ];
    function(
        "__clean_string_concat",
        &[Val::I32, Val::I32],
        &[Val::I32],
        &[Val::I32; 3],
        body,
    )
}

/// Params: a@0, b@1. Locals: la@2, lb@3, n@4, i@5, d@6.
fn string_compare() -> MirFunction {
    use Inst::*;
    let body = vec![
        LocalGet(0),
        I32Load(0),
        LocalSet(2),
        LocalGet(1),
        I32Load(0),
        LocalSet(3),
        // n = min(la, lb)
        LocalGet(2),
        LocalGet(3),
        LocalGet(2),
        LocalGet(3),
        I32Cmp(CmpOp::LtU),
        Select,
        LocalSet(4),
        I32Const(0),
        LocalSet(5),
        Block {
            body: vec![Loop {
                body: vec![
                    LocalGet(5),
                    LocalGet(4),
                    I32Cmp(CmpOp::Eq),
                    BrIf(1),
                    // d = a.payload[i] - b.payload[i]
                    LocalGet(0),
                    LocalGet(5),
                    I32Bin(I32Op::Add),
                    I32Load8U(4),
                    LocalGet(1),
                    LocalGet(5),
                    I32Bin(I32Op::Add),
                    I32Load8U(4),
                    I32Bin(I32Op::Sub),
                    LocalTee(6),
                    If {
                        result: None,
                        then: vec![LocalGet(6), Return],
                        els: vec![],
                    },
                    LocalGet(5),
                    I32Const(1),
                    I32Bin(I32Op::Add),
                    LocalSet(5),
                    Br(0),
                ],
            }],
        },
        // Equal prefix: the length difference decides (0 iff equal).
        LocalGet(2),
        LocalGet(3),
        I32Bin(I32Op::Sub),
    ];
    function(
        "__clean_string_compare",
        &[Val::I32, Val::I32],
        &[Val::I32],
        &[Val::I32; 5],
        body,
    )
}

/// Params: a@0, b@1.
fn string_eq() -> MirFunction {
    use Inst::*;
    let body = vec![
        // Pointer equality is a hit for interned constants and aliases.
        LocalGet(0),
        LocalGet(1),
        I32Cmp(CmpOp::Eq),
        If {
            result: None,
            then: vec![I32Const(1), Return],
            els: vec![],
        },
        LocalGet(0),
        I32Load(0),
        LocalGet(1),
        I32Load(0),
        I32Cmp(CmpOp::Ne),
        If {
            result: None,
            then: vec![I32Const(0), Return],
            els: vec![],
        },
        // The convention: compare returns 0 iff equal.
        LocalGet(0),
        LocalGet(1),
        CallRuntime(RuntimeFn::StringCompare),
        I32Eqz,
    ];
    function(
        "__clean_string_eq",
        &[Val::I32, Val::I32],
        &[Val::I32],
        &[],
        body,
    )
}

/// Params: ptr@0, len@1. Locals: r@2.
fn lift_string() -> MirFunction {
    use Inst::*;
    let body = vec![
        LocalGet(1),
        I32Eqz,
        If {
            result: None,
            then: vec![I32Const(EMPTY_STRING_ADDR as i32), Return],
            els: vec![],
        },
        LocalGet(1),
        I32Const(4),
        I32Bin(I32Op::Add),
        I32Const(ALIGNMENT as i32),
        CallRuntime(RuntimeFn::Alloc),
        LocalTee(2),
        LocalGet(1),
        I32Store(0),
        LocalGet(2),
        I32Const(4),
        I32Bin(I32Op::Add),
        LocalGet(0),
        LocalGet(1),
        MemoryCopy,
        LocalGet(2),
    ];
    function(
        "__clean_lift_string",
        &[Val::I32, Val::I32],
        &[Val::I32],
        &[Val::I32],
        body,
    )
}
