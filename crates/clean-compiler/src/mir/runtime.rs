//! Always-emitted runtime helpers (ADR 0004): the MMD-02 bump allocator
//! and the MMD-04 string operations, built as ordinary [`MirFunction`]s so
//! emission and snapshots treat them like any other code. Their function
//! indices follow the user functions in [`RuntimeFn`] discriminant order.
//!
//! `clean:bridge/*` always-on imports (BRG-05) are deferred — see the ADR
//! and DISCOVERIES-M6 item 1; these guest functions implement the same
//! observable semantics.

use crate::layout::{Tier, ALIGNMENT, EMPTY_STRING_ADDR, HEAP_PTR_GLOBAL, WASM_PAGE_SIZE};

use super::{CmpOp, F64Op, I32Op, I64Op, Inst, MirFunction, Val};

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
    /// `int_to_string(n) -> base` — 15 §Conversions: the literal decimal
    /// rendering.
    IntToString = 5,
    /// `str_to_int(base) -> i64` — 15 §Conversions: parses a decimal
    /// integer literal; anything else is the RUN003 family, surfaced as a
    /// trap until error lowering (DISCOVERIES-M6).
    StrToInt = 6,
    /// `str_to_num(base) -> f64` — parses `[+-]?digits(.digits)?` by
    /// naive accumulation (rounding contract unresolved — DISCOVERIES-M6);
    /// anything else traps (RUN003 family).
    StrToNum = 7,
}

pub fn build(tier: Tier) -> Vec<MirFunction> {
    vec![
        alloc(tier),
        string_concat(),
        string_compare(),
        string_eq(),
        lift_string(),
        int_to_string(),
        str_to_int(),
        str_to_num(),
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

/// Params: n@0 (i64). Locals: m@1 (i64, magnitude), tmp@2 (i64), count@3,
/// len@4, base@5, pos@6, neg@7. The magnitude negates by wrapping, so
/// i64::MIN becomes its own unsigned magnitude and DivU/RemU stay exact.
fn int_to_string() -> MirFunction {
    use Inst::*;
    let body = vec![
        // neg = n < 0
        LocalGet(0),
        I64Const(0),
        I64Cmp(CmpOp::LtS),
        LocalSet(7),
        // m = neg ? 0 - n : n
        LocalGet(7),
        If {
            result: Some(Val::I64),
            then: vec![I64Const(0), LocalGet(0), I64Bin(I64Op::Sub)],
            els: vec![LocalGet(0)],
        },
        LocalSet(1),
        // count digits of tmp = m (do-while: 0 has one digit)
        LocalGet(1),
        LocalSet(2),
        I32Const(0),
        LocalSet(3),
        Loop {
            body: vec![
                LocalGet(3),
                I32Const(1),
                I32Bin(I32Op::Add),
                LocalSet(3),
                LocalGet(2),
                I64Const(10),
                I64Bin(I64Op::DivU),
                LocalTee(2),
                I64Const(0),
                I64Cmp(CmpOp::Ne),
                BrIf(0),
            ],
        },
        // len = count + neg; base = alloc(4 + len, 8); store the length
        LocalGet(3),
        LocalGet(7),
        I32Bin(I32Op::Add),
        LocalSet(4),
        LocalGet(4),
        I32Const(4),
        I32Bin(I32Op::Add),
        I32Const(ALIGNMENT as i32),
        CallRuntime(RuntimeFn::Alloc),
        LocalTee(5),
        LocalGet(4),
        I32Store(0),
        // pos = len - 1; write digits backwards (do-while writes "0")
        LocalGet(4),
        I32Const(1),
        I32Bin(I32Op::Sub),
        LocalSet(6),
        Loop {
            body: vec![
                LocalGet(5),
                LocalGet(6),
                I32Bin(I32Op::Add),
                LocalGet(1),
                I64Const(10),
                I64Bin(I64Op::RemU),
                I32WrapI64,
                I32Const('0' as i32),
                I32Bin(I32Op::Add),
                I32Store8(4),
                LocalGet(1),
                I64Const(10),
                I64Bin(I64Op::DivU),
                LocalSet(1),
                LocalGet(6),
                I32Const(1),
                I32Bin(I32Op::Sub),
                LocalSet(6),
                LocalGet(1),
                I64Const(0),
                I64Cmp(CmpOp::Ne),
                BrIf(0),
            ],
        },
        // sign
        LocalGet(7),
        If {
            result: None,
            then: vec![LocalGet(5), I32Const('-' as i32), I32Store8(4)],
            els: vec![],
        },
        LocalGet(5),
    ];
    function(
        "__clean_int_to_string",
        &[Val::I64],
        &[Val::I32],
        &[
            Val::I64,
            Val::I64,
            Val::I32,
            Val::I32,
            Val::I32,
            Val::I32,
            Val::I32,
        ],
        body,
    )
}

/// Params: s@0. Locals: len@1, i@2, c@3, acc@4 (i64, negative
/// accumulation so i64::MIN parses without overflow), neg@5, d@6 (i64).
/// Any non-literal input traps (RUN003 family until error lowering).
fn str_to_int() -> MirFunction {
    use Inst::*;
    const PRE_MULT_MIN: i64 = i64::MIN / 10; // -922337203685477580
    let body = vec![
        LocalGet(0),
        I32Load(0),
        LocalSet(1),
        // empty → trap
        LocalGet(1),
        I32Eqz,
        If {
            result: None,
            then: vec![Unreachable],
            els: vec![],
        },
        // sign
        LocalGet(0),
        I32Load8U(4),
        LocalSet(3),
        LocalGet(3),
        I32Const('-' as i32),
        I32Cmp(CmpOp::Eq),
        If {
            result: None,
            then: vec![I32Const(1), LocalSet(5), I32Const(1), LocalSet(2)],
            els: vec![
                LocalGet(3),
                I32Const('+' as i32),
                I32Cmp(CmpOp::Eq),
                If {
                    result: None,
                    then: vec![I32Const(1), LocalSet(2)],
                    els: vec![],
                },
            ],
        },
        // a bare sign → trap
        LocalGet(2),
        LocalGet(1),
        I32Cmp(CmpOp::Eq),
        If {
            result: None,
            then: vec![Unreachable],
            els: vec![],
        },
        // digit loop
        Block {
            body: vec![Loop {
                body: vec![
                    LocalGet(2),
                    LocalGet(1),
                    I32Cmp(CmpOp::Eq),
                    BrIf(1),
                    LocalGet(0),
                    LocalGet(2),
                    I32Bin(I32Op::Add),
                    I32Load8U(4),
                    LocalSet(3),
                    // non-digit → trap
                    LocalGet(3),
                    I32Const('0' as i32),
                    I32Cmp(CmpOp::LtS),
                    LocalGet(3),
                    I32Const('9' as i32),
                    I32Cmp(CmpOp::GtS),
                    I32Bin(I32Op::Or),
                    If {
                        result: None,
                        then: vec![Unreachable],
                        els: vec![],
                    },
                    // d = c - '0'
                    LocalGet(3),
                    I32Const('0' as i32),
                    I32Bin(I32Op::Sub),
                    I64ExtendI32U,
                    LocalSet(6),
                    // acc < i64::MIN/10 → the ×10 would overflow → trap
                    LocalGet(4),
                    I64Const(PRE_MULT_MIN),
                    I64Cmp(CmpOp::LtS),
                    If {
                        result: None,
                        then: vec![Unreachable],
                        els: vec![],
                    },
                    LocalGet(4),
                    I64Const(10),
                    I64Bin(I64Op::Mul),
                    LocalSet(4),
                    // acc < i64::MIN + d → the subtraction would overflow
                    LocalGet(4),
                    I64Const(i64::MIN),
                    LocalGet(6),
                    I64Bin(I64Op::Add),
                    I64Cmp(CmpOp::LtS),
                    If {
                        result: None,
                        then: vec![Unreachable],
                        els: vec![],
                    },
                    LocalGet(4),
                    LocalGet(6),
                    I64Bin(I64Op::Sub),
                    LocalSet(4),
                    LocalGet(2),
                    I32Const(1),
                    I32Bin(I32Op::Add),
                    LocalSet(2),
                    Br(0),
                ],
            }],
        },
        // positive result: -acc, trapping on i64::MIN (which only -n holds)
        LocalGet(5),
        If {
            result: Some(Val::I64),
            then: vec![LocalGet(4)],
            els: vec![
                LocalGet(4),
                I64Const(i64::MIN),
                I64Cmp(CmpOp::Eq),
                If {
                    result: None,
                    then: vec![Unreachable],
                    els: vec![],
                },
                I64Const(0),
                LocalGet(4),
                I64Bin(I64Op::Sub),
            ],
        },
    ];
    function(
        "__clean_str_to_int",
        &[Val::I32],
        &[Val::I64],
        &[Val::I32, Val::I32, Val::I32, Val::I64, Val::I32, Val::I64],
        body,
    )
}

/// Params: s@0. Locals: len@1, i@2, c@3, val@4 (f64), scale@5 (f64),
/// neg@6, seen@7. Grammar: `[+-]? digits* ('.' digits+)?` with at least
/// one digit overall; anything else traps (RUN003 family).
fn str_to_num() -> MirFunction {
    use Inst::*;
    // digit: c in '0'..='9' → val = val * 10 + d (integer part) — emitted
    // twice below with different accumulation, so build the fragments.
    let read_c = |out: &mut Vec<Inst>| {
        out.push(LocalGet(0));
        out.push(LocalGet(2));
        out.push(I32Bin(I32Op::Add));
        out.push(I32Load8U(4));
        out.push(LocalSet(3));
    };
    let is_digit = |out: &mut Vec<Inst>| {
        out.push(LocalGet(3));
        out.push(I32Const('0' as i32));
        out.push(I32Cmp(CmpOp::GeS));
        out.push(LocalGet(3));
        out.push(I32Const('9' as i32));
        out.push(I32Cmp(CmpOp::LeS));
        out.push(I32Bin(I32Op::And));
    };
    let mut int_loop = Vec::new();
    {
        let out = &mut int_loop;
        out.push(LocalGet(2));
        out.push(LocalGet(1));
        out.push(I32Cmp(CmpOp::Eq));
        out.push(BrIf(1));
        read_c(out);
        is_digit(out);
        out.push(I32Eqz);
        out.push(BrIf(1));
        // val = val * 10 + d
        out.push(LocalGet(4));
        out.push(F64Const(10.0));
        out.push(F64Bin(F64Op::Mul));
        out.push(LocalGet(3));
        out.push(I32Const('0' as i32));
        out.push(I32Bin(I32Op::Sub));
        out.push(F64ConvertI32S);
        out.push(F64Bin(F64Op::Add));
        out.push(LocalSet(4));
        out.push(I32Const(1));
        out.push(LocalSet(7));
        out.push(LocalGet(2));
        out.push(I32Const(1));
        out.push(I32Bin(I32Op::Add));
        out.push(LocalSet(2));
        out.push(Br(0));
    }
    let mut frac_loop = Vec::new();
    {
        let out = &mut frac_loop;
        out.push(LocalGet(2));
        out.push(LocalGet(1));
        out.push(I32Cmp(CmpOp::Eq));
        out.push(BrIf(1));
        read_c(out);
        is_digit(out);
        out.push(I32Eqz);
        out.push(BrIf(1));
        // scale /= 10; val += d * scale
        out.push(LocalGet(5));
        out.push(F64Const(10.0));
        out.push(F64Bin(F64Op::Div));
        out.push(LocalSet(5));
        out.push(LocalGet(4));
        out.push(LocalGet(3));
        out.push(I32Const('0' as i32));
        out.push(I32Bin(I32Op::Sub));
        out.push(F64ConvertI32S);
        out.push(LocalGet(5));
        out.push(F64Bin(F64Op::Mul));
        out.push(F64Bin(F64Op::Add));
        out.push(LocalSet(4));
        out.push(I32Const(1));
        out.push(LocalSet(7));
        out.push(LocalGet(2));
        out.push(I32Const(1));
        out.push(I32Bin(I32Op::Add));
        out.push(LocalSet(2));
        out.push(Br(0));
    }
    let body = vec![
        LocalGet(0),
        I32Load(0),
        LocalSet(1),
        LocalGet(1),
        I32Eqz,
        If {
            result: None,
            then: vec![Unreachable],
            els: vec![],
        },
        F64Const(1.0),
        LocalSet(5),
        // sign
        LocalGet(0),
        I32Load8U(4),
        LocalSet(3),
        LocalGet(3),
        I32Const('-' as i32),
        I32Cmp(CmpOp::Eq),
        If {
            result: None,
            then: vec![I32Const(1), LocalSet(6), I32Const(1), LocalSet(2)],
            els: vec![
                LocalGet(3),
                I32Const('+' as i32),
                I32Cmp(CmpOp::Eq),
                If {
                    result: None,
                    then: vec![I32Const(1), LocalSet(2)],
                    els: vec![],
                },
            ],
        },
        Block {
            body: vec![Loop { body: int_loop }],
        },
        // optional fraction
        LocalGet(2),
        LocalGet(1),
        I32Cmp(CmpOp::Ne),
        If {
            result: None,
            then: vec![
                LocalGet(0),
                LocalGet(2),
                I32Bin(I32Op::Add),
                I32Load8U(4),
                I32Const('.' as i32),
                I32Cmp(CmpOp::Ne),
                If {
                    result: None,
                    then: vec![Unreachable],
                    els: vec![],
                },
                LocalGet(2),
                I32Const(1),
                I32Bin(I32Op::Add),
                LocalSet(2),
                // A trailing dot is not a literal: at least one digit must
                // follow (a non-digit after the dot is caught by the
                // full-consumption check below).
                LocalGet(2),
                LocalGet(1),
                I32Cmp(CmpOp::Eq),
                If {
                    result: None,
                    then: vec![Unreachable],
                    els: vec![],
                },
                Block {
                    body: vec![Loop { body: frac_loop }],
                },
                LocalGet(2),
                LocalGet(1),
                I32Cmp(CmpOp::Ne),
                If {
                    result: None,
                    then: vec![Unreachable],
                    els: vec![],
                },
            ],
            els: vec![],
        },
        LocalGet(7),
        I32Eqz,
        If {
            result: None,
            then: vec![Unreachable],
            els: vec![],
        },
        LocalGet(6),
        If {
            result: Some(Val::F64),
            then: vec![LocalGet(4), F64Un(super::F64Un::Neg)],
            els: vec![LocalGet(4)],
        },
    ];
    function(
        "__clean_str_to_num",
        &[Val::I32],
        &[Val::F64],
        &[
            Val::I32,
            Val::I32,
            Val::I32,
            Val::F64,
            Val::F64,
            Val::I32,
            Val::I32,
        ],
        body,
    )
}
