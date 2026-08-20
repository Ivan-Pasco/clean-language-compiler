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
    /// `str_to_num(base) -> f64` — parses `[+-]?digits(.digits)?`
    /// correctly rounded (bbdf483: roundTiesToEven) via the Clinger fast
    /// path; anything else traps (RUN003 family).
    StrToNum = 7,
    // Chapter-15 string methods (bodies in `runtime_str.rs`). Indexes and
    // lengths on the user surface count **code points** (local adoption —
    // DISCOVERIES-M6); the object layout stays byte-addressed (MMD-04).
    /// `str_cp_len(s) -> i64` — code-point count.
    StrCpLen = 8,
    /// `str_char_at(s, i: i64) -> base` — one-code-point string; out of
    /// range traps (RUN013 family).
    StrCharAt = 9,
    /// `str_char_code_at(s, i: i64) -> i64` — the code point; out of
    /// range traps.
    StrCharCodeAt = 10,
    /// `str_index_of(s, needle) -> i64` — code-point index of the first
    /// occurrence, or -1.
    StrIndexOf = 11,
    /// `str_last_index_of(s, needle) -> i64`.
    StrLastIndexOf = 12,
    /// `str_starts_with(s, prefix) -> i32`.
    StrStartsWith = 13,
    /// `str_ends_with(s, suffix) -> i32`.
    StrEndsWith = 14,
    /// `str_substring(s, start: i64, end: i64) -> base` — code-point
    /// bounds, clamped to `[0, len]`, `end < start` yields `""` (local
    /// adoption — DISCOVERIES-M6).
    StrSubstring = 15,
    /// `str_trim(s, mode: i32) -> base` — 0 both, 1 start, 2 end; ASCII
    /// whitespace (space, \t, \n, \r — local adoption).
    StrTrim = 16,
    /// `str_pad_start(s, target: i64, pad) -> base` — target in code
    /// points; an empty pad returns the receiver.
    StrPadStart = 17,
    /// `str_pad_end(s, target: i64, pad) -> base`.
    StrPadEnd = 18,
    /// `str_replace(s, old, new) -> base` — every occurrence; an empty
    /// `old` returns the receiver (local adoption).
    StrReplace = 19,
    /// `str_split(s, delim, tag: i32) -> list base` — a `list<string>`
    /// object; an empty delimiter yields one element (local adoption).
    StrSplit = 20,
    /// `str_is_blank(s) -> i32` — empty or all ASCII whitespace.
    StrIsBlank = 21,
    // Chapter-15 list methods (bodies in `runtime_list.rs`). All produce
    // fresh §3.4.1 objects except `list_remove_at`, which mutates in
    // place (no relocation — safe under aliasing).
    /// `list_slice(src, start: i64, end: i64, stride: i32, tag: i32) ->
    /// base` — code follows the substring clamp adoption.
    ListSlice = 22,
    /// `list_reverse(src, stride: i32, tag: i32) -> base`.
    ListReverse = 23,
    /// `list_concat(a, b, stride: i32, tag: i32) -> base`.
    ListConcat = 24,
    /// `list_range(start: i64, end: i64, tag: i32) -> base` — inclusive,
    /// descending when start > end (15 §List).
    ListRange = 25,
    /// `list_fill_64(size: i64, value: i64, tag: i32) -> base`.
    ListFill64 = 26,
    /// `list_fill_f64(size: i64, value: f64, tag: i32) -> base`.
    ListFillF64 = 27,
    /// `list_fill_32(size: i64, value: i32, tag: i32) -> base` — 4-byte
    /// elements (pointers, narrow integers, booleans, enums).
    ListFill32 = 28,
    /// `list_join(items, sep) -> string base` — `list<string>` only.
    ListJoin = 29,
    /// `list_sort_i64(src, tag) -> base` — ascending, fresh list.
    ListSortI64 = 30,
    /// `list_sort_f64(src, tag) -> base`.
    ListSortF64 = 31,
    /// `list_sort_str(src, tag) -> base` — string_compare order.
    ListSortStr = 32,
    /// `list_index_of_i64(list, v: i64, backward: i32) -> i64` — element
    /// index or -1.
    ListIndexOfI64 = 33,
    /// `list_index_of_f64(list, v: f64, backward: i32) -> i64`.
    ListIndexOfF64 = 34,
    /// `list_index_of_str(list, v, backward: i32) -> i64` — string_eq.
    ListIndexOfStr = 35,
    /// `list_index_of_32(list, v: i32, backward: i32) -> i64` — 4-byte
    /// scalar elements.
    ListIndexOf32 = 36,
    /// `list_remove_at(list, i: i64, stride: i32)` — shifts the tail left
    /// one element and decrements the length; bounds are the caller's.
    ListRemoveAt = 37,
    /// `bytes_slice(b, start: i64, end: i64) -> base` — byte indexes,
    /// clamped like `substring` (local adoption).
    BytesSlice = 38,
    /// `bytes_to_text(b) -> (disc: i32, base: i32)` — full RFC 3629
    /// validation; well-formed input aliases the receiver (both types are
    /// immutable with identical layouts), otherwise `(0, 0)`.
    BytesToText = 39,
    // The `any` box (ADR 0005; bodies in `runtime_any.rs`). Box layout:
    // [tag u32 @0][pad][payload @8]; tag 8 adds a source-text ptr @16.
    /// `any_box_int(v: i64) -> box` (tag 2).
    AnyBoxInt = 40,
    /// `any_box_num(v: f64) -> box` (tag 3).
    AnyBoxNum = 41,
    /// `any_box_bool(v: i32) -> box` (tag 1).
    AnyBoxBool = 42,
    /// `any_box_str(s) -> box` (tag 4).
    AnyBoxStr = 43,
    /// `any_box_list(l) -> box` (tag 6).
    AnyBoxList = 44,
    /// `any_box_pairs(p) -> box` (tag 7).
    AnyBoxPairs = 45,
    /// `any_unbox_int(box) -> i64` — tag 2 verbatim; tags 3/8 truncate
    /// toward zero; anything else traps (RUN005 family).
    AnyUnboxInt = 46,
    /// `any_unbox_num(box) -> f64` — tags 3/8 verbatim; tag 2 widens;
    /// anything else traps.
    AnyUnboxNum = 47,
    /// `any_unbox_bool(box) -> i32` — tag 1 or trap.
    AnyUnboxBool = 48,
    /// `any_unbox_str(box) -> base` — tag 4 or trap.
    AnyUnboxStr = 49,
    /// `any_is_none(box) -> i32`.
    AnyIsNone = 50,
    /// `any_index_int(box, i: i64) -> box` — list element; none for out
    /// of range or a non-list box (never a trap — chapter 15 access
    /// semantics).
    AnyIndexInt = 51,
    /// `any_index_str(box, key) -> box` — pairs lookup by string key;
    /// none when missing or not a pairs box.
    AnyIndexStr = 52,
    // JSON (chapter 15 §JSON Module; bodies in `runtime_json.rs`).
    /// `json_parse(s) -> box` — RUN006–RUN010 conditions trap.
    JsonParse = 53,
    /// `json_try_parse(s) -> box` — none in exactly the RUN006–RUN010
    /// conditions.
    JsonTryParse = 54,
    /// `json_serialize(box) -> s` — compact.
    JsonSerialize = 55,
    /// `json_serialize_pretty(box) -> s` — tab-indented, one member per
    /// line.
    JsonSerializePretty = 56,
}

/// Pre-interned static strings the raising runtime helpers need (the
/// data pool lives with the caller): the `RUN003` code text and the two
/// parse-failure messages — local wordings, Platform 10 defines no
/// runtime-message templates for RUN003 (DISCOVERIES-M8).
#[derive(Debug, Clone, Copy)]
pub struct RaiseMsgs {
    pub run003_code: i32,
    pub not_an_integer: i32,
    pub not_a_number: i32,
}

/// The raise sequence for a runtime-helper body (13 §ERH-03): store the
/// message and code, set the flag, return a dummy — the caller checks the
/// flag and unwinds.
fn raise_run003(msgs: RaiseMsgs, message: i32, dummy: Inst) -> Vec<Inst> {
    vec![
        Inst::I32Const(message),
        Inst::GlobalSet(crate::layout::ERR_MSG_GLOBAL),
        Inst::I32Const(msgs.run003_code),
        Inst::GlobalSet(crate::layout::ERR_CODE_GLOBAL),
        Inst::I32Const(1),
        Inst::GlobalSet(crate::layout::ERR_FLAG_GLOBAL),
        dummy,
        Inst::Return,
    ]
}

pub fn build(tier: Tier, msgs: RaiseMsgs) -> Vec<MirFunction> {
    let mut fns = vec![
        alloc(tier),
        string_concat(),
        string_compare(),
        string_eq(),
        lift_string(),
        int_to_string(),
        str_to_int(msgs),
        str_to_num(msgs),
    ];
    fns.extend(super::runtime_str::build());
    fns.extend(super::runtime_list::build());
    fns.push(bytes_slice());
    fns.push(bytes_to_text());
    fns.extend(super::runtime_any::build());
    fns.extend(super::runtime_json::build());
    fns
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
/// Any non-literal input raises RUN003, catchable (13 §ERH-03).
fn str_to_int(msgs: RaiseMsgs) -> MirFunction {
    let raise = || raise_run003(msgs, msgs.not_an_integer, Inst::I64Const(0));
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
            then: raise(),
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
            then: raise(),
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
                        then: raise(),
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
                        then: raise(),
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
                        then: raise(),
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
                    then: raise(),
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

/// Params: s@0. Locals: len@1, i@2, c@3, m@4 (i64 mantissa), digits@5,
/// exp10@6, truncated@7, neg@8, seen@9, f@10 (f64), p@11 (f64),
/// step@12. Grammar: `[+-]? digits* ('.' digits+)?` with at least one
/// digit; anything else raises RUN003, catchable (13 §ERH-03).
///
/// Rounding (bbdf483: correctly rounded, roundTiesToEven): up to 19
/// significant digits accumulate exactly in the i64 mantissa; when the
/// mantissa fits 2^53 and |exp10| ≤ 22 the single multiply/divide by an
/// exact power of ten is provably correctly rounded (Clinger). Inputs
/// past that window fall back to chunked scaling — documented residue,
/// unreachable from hand-written literals.
fn str_to_num(msgs: RaiseMsgs) -> MirFunction {
    let raise = || raise_run003(msgs, msgs.not_a_number, Inst::F64Const(0.0));
    use Inst::*;
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
    // m = m*10 + (c-'0'); digits += 1
    let accumulate = |out: &mut Vec<Inst>| {
        out.push(LocalGet(4));
        out.push(I64Const(10));
        out.push(I64Bin(I64Op::Mul));
        out.push(LocalGet(3));
        out.push(I32Const('0' as i32));
        out.push(I32Bin(I32Op::Sub));
        out.push(I64ExtendI32U);
        out.push(I64Bin(I64Op::Add));
        out.push(LocalSet(4));
        out.push(LocalGet(5));
        out.push(I32Const(1));
        out.push(I32Bin(I32Op::Add));
        out.push(LocalSet(5));
    };
    // 1 iff this digit is a leading zero (no significance yet).
    let leading_zero = |out: &mut Vec<Inst>| {
        out.push(LocalGet(5));
        out.push(I32Eqz);
        out.push(LocalGet(3));
        out.push(I32Const('0' as i32));
        out.push(I32Cmp(CmpOp::Eq));
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
        out.push(I32Const(1));
        out.push(LocalSet(9));
        leading_zero(out);
        out.push(If {
            result: None,
            then: vec![],
            els: vec![
                LocalGet(5),
                I32Const(19),
                I32Cmp(CmpOp::LtS),
                If {
                    result: None,
                    then: {
                        let mut t = Vec::new();
                        accumulate(&mut t);
                        t
                    },
                    els: vec![
                        I32Const(1),
                        LocalSet(7),
                        LocalGet(6),
                        I32Const(1),
                        I32Bin(I32Op::Add),
                        LocalSet(6),
                    ],
                },
            ],
        });
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
        out.push(I32Const(1));
        out.push(LocalSet(9));
        leading_zero(out);
        out.push(If {
            result: None,
            // A zero before the first significant digit still shifts the
            // scale (0.0003).
            then: vec![LocalGet(6), I32Const(1), I32Bin(I32Op::Sub), LocalSet(6)],
            els: vec![
                LocalGet(5),
                I32Const(19),
                I32Cmp(CmpOp::LtS),
                If {
                    result: None,
                    then: {
                        let mut t = Vec::new();
                        accumulate(&mut t);
                        t.push(LocalGet(6));
                        t.push(I32Const(1));
                        t.push(I32Bin(I32Op::Sub));
                        t.push(LocalSet(6));
                        t
                    },
                    els: vec![I32Const(1), LocalSet(7)],
                },
            ],
        });
        out.push(LocalGet(2));
        out.push(I32Const(1));
        out.push(I32Bin(I32Op::Add));
        out.push(LocalSet(2));
        out.push(Br(0));
    }

    // One fallback scaling chunk: pick |k| clamped to 22 into step,
    // advance k toward zero, apply multiply or divide.
    let fallback_chunk = vec![
        LocalGet(6),
        I32Eqz,
        BrIf(1),
        LocalGet(6),
        I32Const(0),
        I32Cmp(CmpOp::GtS),
        If {
            result: None,
            then: vec![
                LocalGet(6),
                I32Const(22),
                LocalGet(6),
                I32Const(22),
                I32Cmp(CmpOp::LtS),
                Select,
                LocalSet(12),
                LocalGet(6),
                LocalGet(12),
                I32Bin(I32Op::Sub),
                LocalSet(6),
                // f *= 10^step
                F64Const(1.0),
                LocalSet(11),
                Block {
                    body: vec![Loop {
                        body: vec![
                            LocalGet(12),
                            I32Eqz,
                            BrIf(1),
                            LocalGet(11),
                            F64Const(10.0),
                            F64Bin(F64Op::Mul),
                            LocalSet(11),
                            LocalGet(12),
                            I32Const(1),
                            I32Bin(I32Op::Sub),
                            LocalSet(12),
                            Br(0),
                        ],
                    }],
                },
                LocalGet(10),
                LocalGet(11),
                F64Bin(F64Op::Mul),
                LocalSet(10),
            ],
            els: vec![
                I32Const(0),
                LocalGet(6),
                I32Bin(I32Op::Sub),
                LocalTee(12),
                I32Const(22),
                I32Cmp(CmpOp::GtS),
                If {
                    result: None,
                    then: vec![I32Const(22), LocalSet(12)],
                    els: vec![],
                },
                LocalGet(6),
                LocalGet(12),
                I32Bin(I32Op::Add),
                LocalSet(6),
                // f /= 10^step
                F64Const(1.0),
                LocalSet(11),
                Block {
                    body: vec![Loop {
                        body: vec![
                            LocalGet(12),
                            I32Eqz,
                            BrIf(1),
                            LocalGet(11),
                            F64Const(10.0),
                            F64Bin(F64Op::Mul),
                            LocalSet(11),
                            LocalGet(12),
                            I32Const(1),
                            I32Bin(I32Op::Sub),
                            LocalSet(12),
                            Br(0),
                        ],
                    }],
                },
                LocalGet(10),
                LocalGet(11),
                F64Bin(F64Op::Div),
                LocalSet(10),
            ],
        },
        Br(0),
    ];

    let mut body = vec![
        LocalGet(0),
        I32Load(0),
        LocalSet(1),
        LocalGet(1),
        I32Eqz,
        If {
            result: None,
            then: raise(),
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
            then: vec![I32Const(1), LocalSet(8), I32Const(1), LocalSet(2)],
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
                    then: raise(),
                    els: vec![],
                },
                LocalGet(2),
                I32Const(1),
                I32Bin(I32Op::Add),
                LocalSet(2),
                // a trailing dot is not a literal
                LocalGet(2),
                LocalGet(1),
                I32Cmp(CmpOp::Eq),
                If {
                    result: None,
                    then: raise(),
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
                    then: raise(),
                    els: vec![],
                },
            ],
            els: vec![],
        },
        LocalGet(9),
        I32Eqz,
        If {
            result: None,
            then: raise(),
            els: vec![],
        },
        // f = m (exact i64 → f64 rounding is the ONLY rounding on the
        // fast path)
        LocalGet(4),
        F64ConvertI64S,
        LocalSet(10),
        // fast path: !truncated && |exp10| <= 22 && m <= 2^53
        LocalGet(7),
        I32Eqz,
        LocalGet(6),
        I32Const(-22),
        I32Cmp(CmpOp::GeS),
        I32Bin(I32Op::And),
        LocalGet(6),
        I32Const(22),
        I32Cmp(CmpOp::LeS),
        I32Bin(I32Op::And),
        LocalGet(4),
        I64Const(9_007_199_254_740_992),
        I64Cmp(CmpOp::LeS),
        I32Bin(I32Op::And),
        If {
            result: None,
            then: vec![
                // step = |exp10|
                LocalGet(6),
                I32Const(0),
                I32Cmp(CmpOp::LtS),
                If {
                    result: Some(Val::I32),
                    then: vec![I32Const(0), LocalGet(6), I32Bin(I32Op::Sub)],
                    els: vec![LocalGet(6)],
                },
                LocalSet(12),
                F64Const(1.0),
                LocalSet(11),
                Block {
                    body: vec![Loop {
                        body: vec![
                            LocalGet(12),
                            I32Eqz,
                            BrIf(1),
                            LocalGet(11),
                            F64Const(10.0),
                            F64Bin(F64Op::Mul),
                            LocalSet(11),
                            LocalGet(12),
                            I32Const(1),
                            I32Bin(I32Op::Sub),
                            LocalSet(12),
                            Br(0),
                        ],
                    }],
                },
                LocalGet(6),
                I32Const(0),
                I32Cmp(CmpOp::LtS),
                If {
                    result: Some(Val::F64),
                    then: vec![LocalGet(10), LocalGet(11), F64Bin(F64Op::Div)],
                    els: vec![LocalGet(10), LocalGet(11), F64Bin(F64Op::Mul)],
                },
                LocalSet(10),
            ],
            els: vec![Block {
                body: vec![Loop {
                    body: fallback_chunk,
                }],
            }],
        },
        LocalGet(8),
        If {
            result: Some(Val::F64),
            then: vec![LocalGet(10), F64Un(super::F64Un::Neg)],
            els: vec![LocalGet(10)],
        },
    ];
    body.shrink_to_fit();
    function(
        "__clean_str_to_num",
        &[Val::I32],
        &[Val::F64],
        &[
            Val::I32,
            Val::I32,
            Val::I32,
            Val::I64,
            Val::I32,
            Val::I32,
            Val::I32,
            Val::I32,
            Val::I32,
            Val::F64,
            Val::F64,
            Val::I32,
        ],
        body,
    )
}

/// Params: b@0, start@1 (i64), end@2 (i64). Locals: len64@3 (i64),
/// s32@4, e32@5. Clamps both bounds to `[0, len]`; `end <= start` yields
/// the shared empty constant; otherwise the range lifts into a fresh
/// object.
fn bytes_slice() -> MirFunction {
    use Inst::*;
    let clamp = |slot: u32, out: &mut Vec<Inst>| {
        // v = min(max(v, 0), len)
        out.push(LocalGet(slot));
        out.push(I64Const(0));
        out.push(I64Cmp(CmpOp::LtS));
        out.push(If {
            result: None,
            then: vec![I64Const(0), LocalSet(slot)],
            els: vec![],
        });
        out.push(LocalGet(slot));
        out.push(LocalGet(3));
        out.push(I64Cmp(CmpOp::GtS));
        out.push(If {
            result: None,
            then: vec![LocalGet(3), LocalSet(slot)],
            els: vec![],
        });
    };
    let mut body = vec![LocalGet(0), I32Load(0), I64ExtendI32U, LocalSet(3)];
    clamp(1, &mut body);
    clamp(2, &mut body);
    body.extend(vec![
        LocalGet(2),
        LocalGet(1),
        I64Cmp(CmpOp::LeS),
        If {
            result: None,
            then: vec![I32Const(EMPTY_STRING_ADDR as i32), Return],
            els: vec![],
        },
        LocalGet(1),
        I32WrapI64,
        LocalSet(4),
        LocalGet(2),
        I32WrapI64,
        LocalSet(5),
        // LiftString(payload + start, end - start)
        LocalGet(0),
        I32Const(4),
        I32Bin(I32Op::Add),
        LocalGet(4),
        I32Bin(I32Op::Add),
        LocalGet(5),
        LocalGet(4),
        I32Bin(I32Op::Sub),
        CallRuntime(RuntimeFn::LiftString),
    ]);
    function(
        "__clean_bytes_slice",
        &[Val::I32, Val::I64, Val::I64],
        &[Val::I32],
        &[Val::I64, Val::I32, Val::I32],
        body,
    )
}

/// Params: b@0. Locals: len@1, i@2, c@3, need@4, floor@5, ceil@6. Full RFC 3629
/// validation: continuation structure, overlong forms (C0/C1 leads, E0
/// and F0 second-byte floors), surrogates (ED with second byte ≥ A0) and
/// the U+10FFFF ceiling (F4 with second byte ≥ 90, F5..FF leads).
/// Valid input returns `(1, b)` — an alias, both types being immutable —
/// and invalid input `(0, 0)`.
fn bytes_to_text() -> MirFunction {
    use Inst::*;
    let fail = || vec![I32Const(0), I32Const(0), Return];
    let load_at_i = |out: &mut Vec<Inst>| {
        out.push(LocalGet(0));
        out.push(LocalGet(2));
        out.push(I32Bin(I32Op::Add));
        out.push(I32Load8U(4));
        out.push(LocalSet(3));
    };
    // One continuation byte at i (already bounds-checked): must be
    // 0x80..=0xBF, and for the first continuation of a 3/4-byte form the
    // caller narrows the range via `floor`/ceiling checks before calling.
    let body = vec![
        LocalGet(0),
        I32Load(0),
        LocalSet(1),
        I32Const(0),
        LocalSet(2),
        Block {
            body: vec![Loop {
                body: {
                    let mut b = vec![LocalGet(2), LocalGet(1), I32Cmp(CmpOp::GeS), BrIf(1)];
                    load_at_i(&mut b);
                    b.extend(vec![
                        // ASCII fast path.
                        LocalGet(3),
                        I32Const(0x80),
                        I32Cmp(CmpOp::LtS),
                        If {
                            result: None,
                            then: vec![LocalGet(2), I32Const(1), I32Bin(I32Op::Add), LocalSet(2)],
                            els: {
                                let mut e = vec![
                                    // Continuation or overlong C0/C1 lead.
                                    LocalGet(3),
                                    I32Const(0xC2),
                                    I32Cmp(CmpOp::LtS),
                                    If {
                                        result: None,
                                        then: fail(),
                                        els: vec![],
                                    },
                                    // F5..FF: beyond U+10FFFF.
                                    LocalGet(3),
                                    I32Const(0xF5),
                                    I32Cmp(CmpOp::GeS),
                                    If {
                                        result: None,
                                        then: fail(),
                                        els: vec![],
                                    },
                                    // need = 1/2/3 continuations; floor and
                                    // ceiling on the SECOND byte encode the
                                    // overlong/surrogate/max rules:
                                    //   E0: b1 in A0..BF   ED: b1 in 80..9F
                                    //   F0: b1 in 90..BF   F4: b1 in 80..8F
                                    // (defaults: 80..BF).
                                ];
                                // need
                                e.extend(vec![
                                    LocalGet(3),
                                    I32Const(0xE0),
                                    I32Cmp(CmpOp::LtS),
                                    If {
                                        result: Some(Val::I32),
                                        then: vec![I32Const(1)],
                                        els: vec![
                                            LocalGet(3),
                                            I32Const(0xF0),
                                            I32Cmp(CmpOp::LtS),
                                            If {
                                                result: Some(Val::I32),
                                                then: vec![I32Const(2)],
                                                els: vec![I32Const(3)],
                                            },
                                        ],
                                    },
                                    LocalSet(4),
                                    // truncation check: i + need >= len → fail
                                    LocalGet(2),
                                    LocalGet(4),
                                    I32Bin(I32Op::Add),
                                    LocalGet(1),
                                    I32Cmp(CmpOp::GeS),
                                    If {
                                        result: None,
                                        then: fail(),
                                        els: vec![],
                                    },
                                    // second-byte floor/ceiling by lead value
                                    // floor default 0x80, ceiling default 0xBF
                                    LocalGet(3),
                                    I32Const(0xE0),
                                    I32Cmp(CmpOp::Eq),
                                    If {
                                        result: Some(Val::I32),
                                        then: vec![I32Const(0xA0)],
                                        els: vec![
                                            LocalGet(3),
                                            I32Const(0xF0),
                                            I32Cmp(CmpOp::Eq),
                                            If {
                                                result: Some(Val::I32),
                                                then: vec![I32Const(0x90)],
                                                els: vec![I32Const(0x80)],
                                            },
                                        ],
                                    },
                                    LocalSet(5),
                                    // ceiling: ED → 0x9F, F4 → 0x8F, else BF
                                    LocalGet(3),
                                    I32Const(0xED),
                                    I32Cmp(CmpOp::Eq),
                                    If {
                                        result: Some(Val::I32),
                                        then: vec![I32Const(0x9F)],
                                        els: vec![
                                            LocalGet(3),
                                            I32Const(0xF4),
                                            I32Cmp(CmpOp::Eq),
                                            If {
                                                result: Some(Val::I32),
                                                then: vec![I32Const(0x8F)],
                                                els: vec![I32Const(0xBF)],
                                            },
                                        ],
                                    },
                                    LocalSet(6),
                                    // second byte must sit in [floor, ceiling]
                                    LocalGet(0),
                                    LocalGet(2),
                                    I32Bin(I32Op::Add),
                                    I32Load8U(5),
                                    LocalTee(3),
                                    LocalGet(5),
                                    I32Cmp(CmpOp::LtS),
                                    If {
                                        result: None,
                                        then: fail(),
                                        els: vec![],
                                    },
                                    LocalGet(3),
                                    LocalGet(6),
                                    I32Cmp(CmpOp::GtS),
                                    If {
                                        result: None,
                                        then: fail(),
                                        els: vec![],
                                    },
                                    // remaining continuations (need-1 of
                                    // them) are plain 80..BF
                                    LocalGet(4),
                                    I32Const(2),
                                    I32Cmp(CmpOp::GeS),
                                    If {
                                        result: None,
                                        then: vec![
                                            LocalGet(0),
                                            LocalGet(2),
                                            I32Bin(I32Op::Add),
                                            I32Load8U(6),
                                            I32Const(0xC0),
                                            I32Bin(I32Op::And),
                                            I32Const(0x80),
                                            I32Cmp(CmpOp::Ne),
                                            If {
                                                result: None,
                                                then: fail(),
                                                els: vec![],
                                            },
                                        ],
                                        els: vec![],
                                    },
                                    LocalGet(4),
                                    I32Const(3),
                                    I32Cmp(CmpOp::Eq),
                                    If {
                                        result: None,
                                        then: vec![
                                            LocalGet(0),
                                            LocalGet(2),
                                            I32Bin(I32Op::Add),
                                            I32Load8U(7),
                                            I32Const(0xC0),
                                            I32Bin(I32Op::And),
                                            I32Const(0x80),
                                            I32Cmp(CmpOp::Ne),
                                            If {
                                                result: None,
                                                then: fail(),
                                                els: vec![],
                                            },
                                        ],
                                        els: vec![],
                                    },
                                    // i += 1 + need
                                    LocalGet(2),
                                    I32Const(1),
                                    I32Bin(I32Op::Add),
                                    LocalGet(4),
                                    I32Bin(I32Op::Add),
                                    LocalSet(2),
                                ]);
                                e
                            },
                        },
                        Br(0),
                    ]);
                    b
                },
            }],
        },
        I32Const(1),
        LocalGet(0),
    ];
    function(
        "__clean_bytes_to_text",
        &[Val::I32],
        &[Val::I32, Val::I32],
        &[Val::I32; 6],
        body,
    )
}
