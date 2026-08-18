//! Chapter-15 string methods: the [`RuntimeFn::StrCpLen`]..=`StrIsBlank`
//! bodies, built as ordinary [`MirFunction`]s in discriminant order (the
//! contracts live on the enum in `runtime.rs`). The user surface counts
//! **code points** (local adoption — DISCOVERIES-M6) while the object
//! layout stays byte-addressed (MMD-04): scans walk payload bytes, and a
//! byte starts a code point iff `(b & 0xC0) != 0x80`. Inputs are
//! well-formed UTF-8 by construction, so nothing here validates. Every
//! helper that produces `""` returns the shared empty-string constant,
//! never a fresh object (MMD-01).

use crate::layout::{
    ALIGNMENT, EMPTY_STRING_ADDR, LIST_CAP_OFFSET, LIST_ELEMS_OFFSET, LIST_LEN_OFFSET,
    LIST_TAG_OFFSET,
};

use super::runtime::RuntimeFn;
use super::{CmpOp, I32Op, I64Op, Inst, MirFunction, Val};

/// The string helpers, in [`RuntimeFn`] discriminant order
/// (`StrCpLen`..=`StrIsBlank`); `runtime::build` appends them after the
/// core helpers.
pub fn build() -> Vec<MirFunction> {
    vec![
        str_cp_len(),
        str_char_at(),
        str_char_code_at(),
        str_index_of(),
        str_last_index_of(),
        str_starts_with(),
        str_ends_with(),
        str_substring(),
        str_trim(),
        str_pad("__clean_str_pad_start", true),
        str_pad("__clean_str_pad_end", false),
        str_replace(),
        str_split(),
        str_is_blank(),
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

/// Pushes payload byte `s[i]` (both i32 locals; the constant offset skips
/// the 4-byte length field).
fn payload_byte(s: u32, i: u32) -> Vec<Inst> {
    use Inst::*;
    vec![LocalGet(s), LocalGet(i), I32Bin(I32Op::Add), I32Load8U(4)]
}

/// Appended after a pushed byte: replaces it with 1 iff the byte starts a
/// code point (is not a UTF-8 continuation byte).
fn is_lead() -> Vec<Inst> {
    use Inst::*;
    vec![
        I32Const(0xC0),
        I32Bin(I32Op::And),
        I32Const(0x80),
        I32Cmp(CmpOp::Ne),
    ]
}

/// Pushes 1 iff the byte in local `c` is one of the four whitespace bytes
/// (space, \t, \n, \r — the trim/isBlank set, local adoption).
fn is_ws(c: u32) -> Vec<Inst> {
    use Inst::*;
    vec![
        LocalGet(c),
        I32Const(0x20),
        I32Cmp(CmpOp::Eq),
        LocalGet(c),
        I32Const(0x09),
        I32Cmp(CmpOp::Eq),
        I32Bin(I32Op::Or),
        LocalGet(c),
        I32Const(0x0A),
        I32Cmp(CmpOp::Eq),
        I32Bin(I32Op::Or),
        LocalGet(c),
        I32Const(0x0D),
        I32Cmp(CmpOp::Eq),
        I32Bin(I32Op::Or),
    ]
}

/// Pushes the byte length (1..=4) of the code point whose lead byte sits
/// in local `b`.
fn cp_byte_len(b: u32) -> Vec<Inst> {
    use Inst::*;
    vec![
        LocalGet(b),
        I32Const(0x80),
        I32Cmp(CmpOp::LtU),
        If {
            result: Some(Val::I32),
            then: vec![I32Const(1)],
            els: vec![
                LocalGet(b),
                I32Const(0xE0),
                I32Cmp(CmpOp::LtU),
                If {
                    result: Some(Val::I32),
                    then: vec![I32Const(2)],
                    els: vec![
                        LocalGet(b),
                        I32Const(0xF0),
                        I32Cmp(CmpOp::LtU),
                        If {
                            result: Some(Val::I32),
                            then: vec![I32Const(3)],
                            els: vec![I32Const(4)],
                        },
                    ],
                },
            ],
        },
    ]
}

/// Emits `cnt = <count of lead bytes in s[0..end)>` — i.e. the code-point
/// index of byte offset `end`. `k` is an i32 scratch, `cnt` an i64 local.
fn count_cps_before(s: u32, end: u32, k: u32, cnt: u32) -> Vec<Inst> {
    use Inst::*;
    let mut step = vec![
        LocalGet(k),
        LocalGet(end),
        I32Cmp(CmpOp::Eq),
        BrIf(1),
        LocalGet(cnt),
    ];
    step.extend(payload_byte(s, k));
    step.extend(is_lead());
    step.extend([
        I64ExtendI32U,
        I64Bin(I64Op::Add),
        LocalSet(cnt),
        LocalGet(k),
        I32Const(1),
        I32Bin(I32Op::Add),
        LocalSet(k),
        Br(0),
    ]);
    let mut out = vec![I32Const(0), LocalSet(k), I64Const(0), LocalSet(cnt)];
    out.push(Block {
        body: vec![Loop { body: step }],
    });
    out
}

/// Shared front half of `charAt`/`charCodeAt` (params s@0, i@1): traps
/// unless `0 <= i < cp_len(s)`, then leaves the byte offset of code point
/// `i` in `p` and its lead byte in `b`. `rem` is an i64 countdown local.
fn seek_cp(len: u32, p: u32, rem: u32, b: u32) -> Vec<Inst> {
    use Inst::*;
    let mut out = vec![
        LocalGet(1),
        I64Const(0),
        I64Cmp(CmpOp::LtS),
        If {
            result: None,
            then: vec![Unreachable],
            els: vec![],
        },
        LocalGet(0),
        I32Load(0),
        LocalSet(len),
        I32Const(0),
        LocalSet(p),
        LocalGet(1),
        LocalSet(rem),
    ];
    // Reaching the end of the payload means i >= cp_len: out of range.
    let mut step = vec![
        LocalGet(p),
        LocalGet(len),
        I32Cmp(CmpOp::Eq),
        If {
            result: None,
            then: vec![Unreachable],
            els: vec![],
        },
    ];
    step.extend(payload_byte(0, p));
    step.push(LocalSet(b));
    step.push(LocalGet(b));
    step.extend(is_lead());
    step.push(If {
        result: None,
        then: vec![
            // At the lead byte of code point i: found (exit the block).
            LocalGet(rem),
            I64Const(0),
            I64Cmp(CmpOp::Eq),
            BrIf(2),
            LocalGet(rem),
            I64Const(1),
            I64Bin(I64Op::Sub),
            LocalSet(rem),
        ],
        els: vec![],
    });
    step.extend([
        LocalGet(p),
        I32Const(1),
        I32Bin(I32Op::Add),
        LocalSet(p),
        Br(0),
    ]);
    out.push(Block {
        body: vec![Loop { body: step }],
    });
    out
}

/// Leaves in `p` the payload byte offset of code point `n` (an i64 local,
/// already clamped to `[0, cp_len]`), or the byte length when
/// `n == cp_len`. `c` is an i64 counter local.
fn byte_of_cp(s: u32, n: u32, ls: u32, p: u32, c: u32) -> Vec<Inst> {
    use Inst::*;
    let mut step = vec![LocalGet(p), LocalGet(ls), I32Cmp(CmpOp::Eq), BrIf(1)];
    step.extend(payload_byte(s, p));
    step.extend(is_lead());
    step.push(If {
        result: None,
        then: vec![
            LocalGet(c),
            LocalGet(n),
            I64Cmp(CmpOp::Eq),
            BrIf(2),
            LocalGet(c),
            I64Const(1),
            I64Bin(I64Op::Add),
            LocalSet(c),
        ],
        els: vec![],
    });
    step.extend([
        LocalGet(p),
        I32Const(1),
        I32Bin(I32Op::Add),
        LocalSet(p),
        Br(0),
    ]);
    let mut out = vec![I32Const(0), LocalSet(p), I64Const(0), LocalSet(c)];
    out.push(Block {
        body: vec![Loop { body: step }],
    });
    out
}

/// Byte-compares `needle` against `hay` at byte offset `i` (the caller
/// guarantees `i + nlen <= len(hay)`): resets `j`, then on a full match
/// runs `on_match` — which must leave via `Return` or a branch; its label
/// depths are 0 = its own `if`, 1 = the compare loop, 2 = the compare
/// block, 3 = whatever encloses this fragment. A mismatch falls out of
/// the fragment.
fn match_at(hay: u32, needle: u32, i: u32, nlen: u32, j: u32, on_match: Vec<Inst>) -> Vec<Inst> {
    use Inst::*;
    let step = vec![
        LocalGet(j),
        LocalGet(nlen),
        I32Cmp(CmpOp::Eq),
        If {
            result: None,
            then: on_match,
            els: vec![],
        },
        LocalGet(hay),
        LocalGet(i),
        I32Bin(I32Op::Add),
        LocalGet(j),
        I32Bin(I32Op::Add),
        I32Load8U(4),
        LocalGet(needle),
        LocalGet(j),
        I32Bin(I32Op::Add),
        I32Load8U(4),
        I32Cmp(CmpOp::Ne),
        BrIf(1),
        LocalGet(j),
        I32Const(1),
        I32Bin(I32Op::Add),
        LocalSet(j),
        Br(0),
    ];
    vec![
        I32Const(0),
        LocalSet(j),
        Block {
            body: vec![Loop { body: step }],
        },
    ]
}

/// `str_cp_len(s) -> i64` — code-point count (count of lead bytes).
/// Params: s@0. Locals: len@1, i@2, n@3 (i64).
fn str_cp_len() -> MirFunction {
    use Inst::*;
    let mut body = vec![LocalGet(0), I32Load(0), LocalSet(1)];
    body.extend(count_cps_before(0, 1, 2, 3));
    body.push(LocalGet(3));
    function(
        "__clean_str_cp_len",
        &[Val::I32],
        &[Val::I64],
        &[Val::I32, Val::I32, Val::I64],
        body,
    )
}

/// `str_char_at(s, i) -> base` — the i-th code point as a fresh
/// one-code-point string; out of range traps (RUN013 family).
/// Params: s@0, i@1 (i64). Locals: len@2, p@3, rem@4 (i64), b@5, clen@6.
fn str_char_at() -> MirFunction {
    use Inst::*;
    let mut body = seek_cp(2, 3, 4, 5);
    body.extend(cp_byte_len(5));
    body.push(LocalSet(6));
    body.extend([
        LocalGet(0),
        I32Const(4),
        I32Bin(I32Op::Add),
        LocalGet(3),
        I32Bin(I32Op::Add),
        LocalGet(6),
        CallRuntime(RuntimeFn::LiftString),
    ]);
    function(
        "__clean_str_char_at",
        &[Val::I32, Val::I64],
        &[Val::I32],
        &[Val::I32, Val::I32, Val::I64, Val::I32, Val::I32],
        body,
    )
}

/// `str_char_code_at(s, i) -> i64` — the i-th code point's value (UTF-8
/// decode); out of range traps.
/// Params: s@0, i@1 (i64). Locals: len@2, p@3, rem@4 (i64), b@5.
fn str_char_code_at() -> MirFunction {
    use Inst::*;
    // Pushes continuation byte k's low 6 bits: s[p+k] & 0x3F.
    let cont = |k: u32| -> Vec<Inst> {
        vec![
            LocalGet(0),
            LocalGet(3),
            I32Bin(I32Op::Add),
            I32Load8U(4 + k),
            I32Const(0x3F),
            I32Bin(I32Op::And),
        ]
    };
    let mut two = vec![
        LocalGet(5),
        I32Const(0x1F),
        I32Bin(I32Op::And),
        I32Const(6),
        I32Bin(I32Op::Shl),
    ];
    two.extend(cont(1));
    two.push(I32Bin(I32Op::Or));
    let mut three = vec![
        LocalGet(5),
        I32Const(0x0F),
        I32Bin(I32Op::And),
        I32Const(12),
        I32Bin(I32Op::Shl),
    ];
    three.extend(cont(1));
    three.extend([I32Const(6), I32Bin(I32Op::Shl), I32Bin(I32Op::Or)]);
    three.extend(cont(2));
    three.push(I32Bin(I32Op::Or));
    let mut four = vec![
        LocalGet(5),
        I32Const(0x07),
        I32Bin(I32Op::And),
        I32Const(18),
        I32Bin(I32Op::Shl),
    ];
    four.extend(cont(1));
    four.extend([I32Const(12), I32Bin(I32Op::Shl), I32Bin(I32Op::Or)]);
    four.extend(cont(2));
    four.extend([I32Const(6), I32Bin(I32Op::Shl), I32Bin(I32Op::Or)]);
    four.extend(cont(3));
    four.push(I32Bin(I32Op::Or));

    let mut body = seek_cp(2, 3, 4, 5);
    body.extend([
        LocalGet(5),
        I32Const(0x80),
        I32Cmp(CmpOp::LtU),
        If {
            result: Some(Val::I32),
            then: vec![LocalGet(5)],
            els: vec![
                LocalGet(5),
                I32Const(0xE0),
                I32Cmp(CmpOp::LtU),
                If {
                    result: Some(Val::I32),
                    then: two,
                    els: vec![
                        LocalGet(5),
                        I32Const(0xF0),
                        I32Cmp(CmpOp::LtU),
                        If {
                            result: Some(Val::I32),
                            then: three,
                            els: four,
                        },
                    ],
                },
            ],
        },
        I64ExtendI32U,
    ]);
    function(
        "__clean_str_char_code_at",
        &[Val::I32, Val::I64],
        &[Val::I64],
        &[Val::I32, Val::I32, Val::I64, Val::I32],
        body,
    )
}

/// `str_index_of(s, needle) -> i64` — code-point index of the first
/// occurrence, or -1; an empty needle finds index 0. Byte-level search; a
/// hit's byte offset converts by counting lead bytes before it.
/// Params: s@0, needle@1. Locals: ls@2, ln@3, i@4, j@5, limit@6, k@7,
/// cnt@8 (i64).
fn str_index_of() -> MirFunction {
    use Inst::*;
    let mut on_match = count_cps_before(0, 4, 7, 8);
    on_match.extend([LocalGet(8), Return]);
    let mut outer = vec![LocalGet(4), LocalGet(6), I32Cmp(CmpOp::GtS), BrIf(1)];
    outer.extend(match_at(0, 1, 4, 3, 5, on_match));
    outer.extend([
        LocalGet(4),
        I32Const(1),
        I32Bin(I32Op::Add),
        LocalSet(4),
        Br(0),
    ]);
    let body = vec![
        LocalGet(0),
        I32Load(0),
        LocalSet(2),
        LocalGet(1),
        I32Load(0),
        LocalSet(3),
        LocalGet(3),
        I32Eqz,
        If {
            result: None,
            then: vec![I64Const(0), Return],
            els: vec![],
        },
        // limit = ls - ln; negative (needle longer) exits immediately.
        LocalGet(2),
        LocalGet(3),
        I32Bin(I32Op::Sub),
        LocalSet(6),
        I32Const(0),
        LocalSet(4),
        Block {
            body: vec![Loop { body: outer }],
        },
        I64Const(-1),
    ];
    function(
        "__clean_str_index_of",
        &[Val::I32, Val::I32],
        &[Val::I64],
        &[
            Val::I32,
            Val::I32,
            Val::I32,
            Val::I32,
            Val::I32,
            Val::I32,
            Val::I64,
        ],
        body,
    )
}

/// `str_last_index_of(s, needle) -> i64` — code-point index of the last
/// occurrence, or -1; an empty needle finds `cp_len(s)`.
/// Params: s@0, needle@1. Locals: ls@2, ln@3, i@4, j@5, limit@6, k@7,
/// cnt@8 (i64).
fn str_last_index_of() -> MirFunction {
    use Inst::*;
    let mut on_match = count_cps_before(0, 4, 7, 8);
    on_match.extend([LocalGet(8), Return]);
    let mut outer = match_at(0, 1, 4, 3, 5, on_match);
    outer.extend([
        LocalGet(4),
        I32Eqz,
        BrIf(1),
        LocalGet(4),
        I32Const(1),
        I32Bin(I32Op::Sub),
        LocalSet(4),
        Br(0),
    ]);
    let body = vec![
        LocalGet(0),
        I32Load(0),
        LocalSet(2),
        LocalGet(1),
        I32Load(0),
        LocalSet(3),
        LocalGet(3),
        I32Eqz,
        If {
            result: None,
            then: vec![LocalGet(0), CallRuntime(RuntimeFn::StrCpLen), Return],
            els: vec![],
        },
        LocalGet(2),
        LocalGet(3),
        I32Bin(I32Op::Sub),
        LocalTee(6),
        I32Const(0),
        I32Cmp(CmpOp::LtS),
        If {
            result: None,
            then: vec![I64Const(-1), Return],
            els: vec![],
        },
        LocalGet(6),
        LocalSet(4),
        Block {
            body: vec![Loop { body: outer }],
        },
        I64Const(-1),
    ];
    function(
        "__clean_str_last_index_of",
        &[Val::I32, Val::I32],
        &[Val::I64],
        &[
            Val::I32,
            Val::I32,
            Val::I32,
            Val::I32,
            Val::I32,
            Val::I32,
            Val::I64,
        ],
        body,
    )
}

/// `str_starts_with(s, p) -> i32` — 1 iff `len(p) <= len(s)` and the
/// bytes match at offset 0. Params: s@0, p@1. Locals: ls@2, lp@3, j@4.
fn str_starts_with() -> MirFunction {
    use Inst::*;
    let body = vec![
        LocalGet(0),
        I32Load(0),
        LocalSet(2),
        LocalGet(1),
        I32Load(0),
        LocalSet(3),
        LocalGet(3),
        LocalGet(2),
        I32Cmp(CmpOp::GtU),
        If {
            result: None,
            then: vec![I32Const(0), Return],
            els: vec![],
        },
        I32Const(0),
        LocalSet(4),
        Block {
            body: vec![Loop {
                body: vec![
                    LocalGet(4),
                    LocalGet(3),
                    I32Cmp(CmpOp::Eq),
                    BrIf(1),
                    LocalGet(0),
                    LocalGet(4),
                    I32Bin(I32Op::Add),
                    I32Load8U(4),
                    LocalGet(1),
                    LocalGet(4),
                    I32Bin(I32Op::Add),
                    I32Load8U(4),
                    I32Cmp(CmpOp::Ne),
                    If {
                        result: None,
                        then: vec![I32Const(0), Return],
                        els: vec![],
                    },
                    LocalGet(4),
                    I32Const(1),
                    I32Bin(I32Op::Add),
                    LocalSet(4),
                    Br(0),
                ],
            }],
        },
        I32Const(1),
    ];
    function(
        "__clean_str_starts_with",
        &[Val::I32, Val::I32],
        &[Val::I32],
        &[Val::I32; 3],
        body,
    )
}

/// `str_ends_with(s, p) -> i32` — 1 iff the bytes match at offset
/// `len(s) - len(p)`. Params: s@0, p@1. Locals: ls@2, lp@3, off@4, j@5.
fn str_ends_with() -> MirFunction {
    use Inst::*;
    let body = vec![
        LocalGet(0),
        I32Load(0),
        LocalSet(2),
        LocalGet(1),
        I32Load(0),
        LocalSet(3),
        LocalGet(3),
        LocalGet(2),
        I32Cmp(CmpOp::GtU),
        If {
            result: None,
            then: vec![I32Const(0), Return],
            els: vec![],
        },
        LocalGet(2),
        LocalGet(3),
        I32Bin(I32Op::Sub),
        LocalSet(4),
        I32Const(0),
        LocalSet(5),
        Block {
            body: vec![Loop {
                body: vec![
                    LocalGet(5),
                    LocalGet(3),
                    I32Cmp(CmpOp::Eq),
                    BrIf(1),
                    LocalGet(0),
                    LocalGet(4),
                    I32Bin(I32Op::Add),
                    LocalGet(5),
                    I32Bin(I32Op::Add),
                    I32Load8U(4),
                    LocalGet(1),
                    LocalGet(5),
                    I32Bin(I32Op::Add),
                    I32Load8U(4),
                    I32Cmp(CmpOp::Ne),
                    If {
                        result: None,
                        then: vec![I32Const(0), Return],
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
        I32Const(1),
    ];
    function(
        "__clean_str_ends_with",
        &[Val::I32, Val::I32],
        &[Val::I32],
        &[Val::I32; 4],
        body,
    )
}

/// `str_substring(s, start, end) -> base` — code-point bounds, each
/// clamped to `[0, cp_len]`; `end <= start` yields the shared `""`.
/// Params: s@0, start@1 (i64), end@2 (i64). Locals: cpl@3 (i64), ls@4,
/// p@5, c@6 (i64), bs@7, be@8.
fn str_substring() -> MirFunction {
    use Inst::*;
    let mut body = vec![
        LocalGet(0),
        CallRuntime(RuntimeFn::StrCpLen),
        LocalSet(3),
        LocalGet(0),
        I32Load(0),
        LocalSet(4),
        // Clamp start and end into [0, cpl].
        LocalGet(1),
        I64Const(0),
        I64Cmp(CmpOp::LtS),
        If {
            result: None,
            then: vec![I64Const(0), LocalSet(1)],
            els: vec![],
        },
        LocalGet(1),
        LocalGet(3),
        I64Cmp(CmpOp::GtS),
        If {
            result: None,
            then: vec![LocalGet(3), LocalSet(1)],
            els: vec![],
        },
        LocalGet(2),
        I64Const(0),
        I64Cmp(CmpOp::LtS),
        If {
            result: None,
            then: vec![I64Const(0), LocalSet(2)],
            els: vec![],
        },
        LocalGet(2),
        LocalGet(3),
        I64Cmp(CmpOp::GtS),
        If {
            result: None,
            then: vec![LocalGet(3), LocalSet(2)],
            els: vec![],
        },
        LocalGet(2),
        LocalGet(1),
        I64Cmp(CmpOp::LeS),
        If {
            result: None,
            then: vec![I32Const(EMPTY_STRING_ADDR as i32), Return],
            els: vec![],
        },
    ];
    body.extend(byte_of_cp(0, 1, 4, 5, 6));
    body.extend([LocalGet(5), LocalSet(7)]);
    body.extend(byte_of_cp(0, 2, 4, 5, 6));
    body.extend([LocalGet(5), LocalSet(8)]);
    body.extend([
        LocalGet(0),
        I32Const(4),
        I32Bin(I32Op::Add),
        LocalGet(7),
        I32Bin(I32Op::Add),
        LocalGet(8),
        LocalGet(7),
        I32Bin(I32Op::Sub),
        CallRuntime(RuntimeFn::LiftString),
    ]);
    function(
        "__clean_str_substring",
        &[Val::I32, Val::I64, Val::I64],
        &[Val::I32],
        &[Val::I64, Val::I32, Val::I32, Val::I64, Val::I32, Val::I32],
        body,
    )
}

/// `str_trim(s, mode) -> base` — mode 0 trims both ends, 1 the start
/// only, 2 the end only; all-whitespace yields the shared `""`.
/// Params: s@0, mode@1. Locals: a@2 (start), b@3 (end, exclusive), c@4.
fn str_trim() -> MirFunction {
    use Inst::*;
    let mut start_step = vec![LocalGet(2), LocalGet(3), I32Cmp(CmpOp::Eq), BrIf(1)];
    start_step.extend(payload_byte(0, 2));
    start_step.push(LocalSet(4));
    start_step.extend(is_ws(4));
    start_step.extend([
        I32Eqz,
        BrIf(1),
        LocalGet(2),
        I32Const(1),
        I32Bin(I32Op::Add),
        LocalSet(2),
        Br(0),
    ]);
    // Reads s[b-1]: address s + b, constant offset 3 (= 4 - 1).
    let mut end_step = vec![
        LocalGet(3),
        LocalGet(2),
        I32Cmp(CmpOp::Eq),
        BrIf(1),
        LocalGet(0),
        LocalGet(3),
        I32Bin(I32Op::Add),
        I32Load8U(3),
        LocalSet(4),
    ];
    end_step.extend(is_ws(4));
    end_step.extend([
        I32Eqz,
        BrIf(1),
        LocalGet(3),
        I32Const(1),
        I32Bin(I32Op::Sub),
        LocalSet(3),
        Br(0),
    ]);
    let body = vec![
        I32Const(0),
        LocalSet(2),
        LocalGet(0),
        I32Load(0),
        LocalSet(3),
        // mode != 2 → trim the start.
        LocalGet(1),
        I32Const(2),
        I32Cmp(CmpOp::Ne),
        If {
            result: None,
            then: vec![Block {
                body: vec![Loop { body: start_step }],
            }],
            els: vec![],
        },
        // mode != 1 → trim the end.
        LocalGet(1),
        I32Const(1),
        I32Cmp(CmpOp::Ne),
        If {
            result: None,
            then: vec![Block {
                body: vec![Loop { body: end_step }],
            }],
            els: vec![],
        },
        LocalGet(2),
        LocalGet(3),
        I32Cmp(CmpOp::Eq),
        If {
            result: None,
            then: vec![I32Const(EMPTY_STRING_ADDR as i32), Return],
            els: vec![],
        },
        LocalGet(0),
        I32Const(4),
        I32Bin(I32Op::Add),
        LocalGet(2),
        I32Bin(I32Op::Add),
        LocalGet(3),
        LocalGet(2),
        I32Bin(I32Op::Sub),
        CallRuntime(RuntimeFn::LiftString),
    ];
    function(
        "__clean_str_trim",
        &[Val::I32, Val::I32],
        &[Val::I32],
        &[Val::I32; 3],
        body,
    )
}

/// `str_pad_start` / `str_pad_end` — pads to `target` code points by
/// cycling `pad`'s code points (truncating mid-pad); an empty pad or
/// `cp_len(s) >= target` returns the receiver unchanged. `before` places
/// the padding ahead of `s` (pad_start) or after it (pad_end).
/// Params: s@0, target@1 (i64), pad@2. Locals: lp@3, cps@4 (i64),
/// rem@5 (i64), padbytes@6, pc@7, r@8, w@9, b@10, clen@11, ls@12.
fn str_pad(name: &str, before: bool) -> MirFunction {
    use Inst::*;
    // One cycled step over pad's code points, shared by both passes:
    // wraps pc at len(pad), classifies the lead byte into clen.
    let cycle_step = |out: &mut Vec<Inst>| {
        out.extend([
            LocalGet(7),
            LocalGet(3),
            I32Cmp(CmpOp::Eq),
            If {
                result: None,
                then: vec![I32Const(0), LocalSet(7)],
                els: vec![],
            },
        ]);
        out.extend(payload_byte(2, 7));
        out.push(LocalSet(10));
        out.extend(cp_byte_len(10));
        out.push(LocalSet(11));
    };
    // Pass 1: measure the pad region's byte length. target > cps here, so
    // at least one code point is needed and the do-while shape is safe.
    let mut measure = Vec::new();
    cycle_step(&mut measure);
    measure.extend([
        LocalGet(6),
        LocalGet(11),
        I32Bin(I32Op::Add),
        LocalSet(6),
        LocalGet(7),
        LocalGet(11),
        I32Bin(I32Op::Add),
        LocalSet(7),
        LocalGet(5),
        I64Const(1),
        I64Bin(I64Op::Sub),
        LocalTee(5),
        I64Const(0),
        I64Cmp(CmpOp::GtS),
        BrIf(0),
    ]);
    // Pass 2: copy the cycled code points into the pad region at w.
    let mut fill = Vec::new();
    cycle_step(&mut fill);
    fill.extend([
        LocalGet(9),
        LocalGet(2),
        I32Const(4),
        I32Bin(I32Op::Add),
        LocalGet(7),
        I32Bin(I32Op::Add),
        LocalGet(11),
        MemoryCopy,
        LocalGet(9),
        LocalGet(11),
        I32Bin(I32Op::Add),
        LocalSet(9),
        LocalGet(7),
        LocalGet(11),
        I32Bin(I32Op::Add),
        LocalSet(7),
        LocalGet(5),
        I64Const(1),
        I64Bin(I64Op::Sub),
        LocalTee(5),
        I64Const(0),
        I64Cmp(CmpOp::GtS),
        BrIf(0),
    ]);
    let mut body = vec![
        LocalGet(2),
        I32Load(0),
        LocalSet(3),
        LocalGet(3),
        I32Eqz,
        If {
            result: None,
            then: vec![LocalGet(0), Return],
            els: vec![],
        },
        LocalGet(0),
        CallRuntime(RuntimeFn::StrCpLen),
        LocalSet(4),
        LocalGet(4),
        LocalGet(1),
        I64Cmp(CmpOp::GeS),
        If {
            result: None,
            then: vec![LocalGet(0), Return],
            els: vec![],
        },
        LocalGet(0),
        I32Load(0),
        LocalSet(12),
        I32Const(0),
        LocalSet(6),
        I32Const(0),
        LocalSet(7),
        LocalGet(1),
        LocalGet(4),
        I64Bin(I64Op::Sub),
        LocalSet(5),
        Loop { body: measure },
        // r = alloc(4 + padbytes + ls, 8); store the byte length.
        LocalGet(6),
        LocalGet(12),
        I32Bin(I32Op::Add),
        I32Const(4),
        I32Bin(I32Op::Add),
        I32Const(ALIGNMENT as i32),
        CallRuntime(RuntimeFn::Alloc),
        LocalTee(8),
        LocalGet(6),
        LocalGet(12),
        I32Bin(I32Op::Add),
        I32Store(0),
    ];
    // w = the pad region's start: r+4 for pad_start, r+4+ls for pad_end.
    body.extend([LocalGet(8), I32Const(4), I32Bin(I32Op::Add)]);
    if !before {
        body.extend([LocalGet(12), I32Bin(I32Op::Add)]);
    }
    body.push(LocalSet(9));
    body.extend([
        I32Const(0),
        LocalSet(7),
        LocalGet(1),
        LocalGet(4),
        I64Bin(I64Op::Sub),
        LocalSet(5),
        Loop { body: fill },
    ]);
    // Copy s's payload on the other side of the pad region.
    body.extend([LocalGet(8), I32Const(4), I32Bin(I32Op::Add)]);
    if before {
        body.extend([LocalGet(6), I32Bin(I32Op::Add)]);
    }
    body.extend([
        LocalGet(0),
        I32Const(4),
        I32Bin(I32Op::Add),
        LocalGet(12),
        MemoryCopy,
        LocalGet(8),
    ]);
    function(
        name,
        &[Val::I32, Val::I64, Val::I32],
        &[Val::I32],
        &[
            Val::I32,
            Val::I64,
            Val::I64,
            Val::I32,
            Val::I32,
            Val::I32,
            Val::I32,
            Val::I32,
            Val::I32,
            Val::I32,
        ],
        body,
    )
}

/// `str_replace(s, old, new) -> base` — every non-overlapping occurrence,
/// left to right, byte-level; an empty `old` returns the receiver, no
/// occurrence returns the receiver, an empty result is the shared `""`.
/// Params: s@0, old@1, new@2. Locals: ls@3, lo@4, ln@5, count@6, i@7,
/// j@8, rl@9, r@10, w@11, seg@12.
fn str_replace() -> MirFunction {
    use Inst::*;
    let mut count_outer = vec![
        LocalGet(7),
        LocalGet(4),
        I32Bin(I32Op::Add),
        LocalGet(3),
        I32Cmp(CmpOp::GtU),
        BrIf(1),
    ];
    count_outer.extend(match_at(
        0,
        1,
        7,
        4,
        8,
        vec![
            LocalGet(6),
            I32Const(1),
            I32Bin(I32Op::Add),
            LocalSet(6),
            LocalGet(7),
            LocalGet(4),
            I32Bin(I32Op::Add),
            LocalSet(7),
            Br(3),
        ],
    ));
    count_outer.extend([
        LocalGet(7),
        I32Const(1),
        I32Bin(I32Op::Add),
        LocalSet(7),
        Br(0),
    ]);
    let mut emit_outer = vec![
        LocalGet(7),
        LocalGet(4),
        I32Bin(I32Op::Add),
        LocalGet(3),
        I32Cmp(CmpOp::GtU),
        BrIf(1),
    ];
    emit_outer.extend(match_at(
        0,
        1,
        7,
        4,
        8,
        vec![
            // Copy the pending segment s[seg..i].
            LocalGet(11),
            LocalGet(0),
            I32Const(4),
            I32Bin(I32Op::Add),
            LocalGet(12),
            I32Bin(I32Op::Add),
            LocalGet(7),
            LocalGet(12),
            I32Bin(I32Op::Sub),
            MemoryCopy,
            LocalGet(11),
            LocalGet(7),
            LocalGet(12),
            I32Bin(I32Op::Sub),
            I32Bin(I32Op::Add),
            LocalSet(11),
            // Copy new's payload.
            LocalGet(11),
            LocalGet(2),
            I32Const(4),
            I32Bin(I32Op::Add),
            LocalGet(5),
            MemoryCopy,
            LocalGet(11),
            LocalGet(5),
            I32Bin(I32Op::Add),
            LocalSet(11),
            // Advance past the occurrence.
            LocalGet(7),
            LocalGet(4),
            I32Bin(I32Op::Add),
            LocalSet(7),
            LocalGet(7),
            LocalSet(12),
            Br(3),
        ],
    ));
    emit_outer.extend([
        LocalGet(7),
        I32Const(1),
        I32Bin(I32Op::Add),
        LocalSet(7),
        Br(0),
    ]);
    let body = vec![
        LocalGet(1),
        I32Load(0),
        LocalSet(4),
        LocalGet(4),
        I32Eqz,
        If {
            result: None,
            then: vec![LocalGet(0), Return],
            els: vec![],
        },
        LocalGet(0),
        I32Load(0),
        LocalSet(3),
        LocalGet(2),
        I32Load(0),
        LocalSet(5),
        // Pass 1: count occurrences.
        I32Const(0),
        LocalSet(6),
        I32Const(0),
        LocalSet(7),
        Block {
            body: vec![Loop { body: count_outer }],
        },
        LocalGet(6),
        I32Eqz,
        If {
            result: None,
            then: vec![LocalGet(0), Return],
            els: vec![],
        },
        // rl = ls + count * (ln - lo).
        LocalGet(3),
        LocalGet(6),
        LocalGet(5),
        LocalGet(4),
        I32Bin(I32Op::Sub),
        I32Bin(I32Op::Mul),
        I32Bin(I32Op::Add),
        LocalSet(9),
        LocalGet(9),
        I32Eqz,
        If {
            result: None,
            then: vec![I32Const(EMPTY_STRING_ADDR as i32), Return],
            els: vec![],
        },
        LocalGet(9),
        I32Const(4),
        I32Bin(I32Op::Add),
        I32Const(ALIGNMENT as i32),
        CallRuntime(RuntimeFn::Alloc),
        LocalTee(10),
        LocalGet(9),
        I32Store(0),
        LocalGet(10),
        I32Const(4),
        I32Bin(I32Op::Add),
        LocalSet(11),
        // Pass 2: copy segments and replacements.
        I32Const(0),
        LocalSet(7),
        I32Const(0),
        LocalSet(12),
        Block {
            body: vec![Loop { body: emit_outer }],
        },
        // The tail segment s[seg..ls].
        LocalGet(11),
        LocalGet(0),
        I32Const(4),
        I32Bin(I32Op::Add),
        LocalGet(12),
        I32Bin(I32Op::Add),
        LocalGet(3),
        LocalGet(12),
        I32Bin(I32Op::Sub),
        MemoryCopy,
        LocalGet(10),
    ];
    function(
        "__clean_str_replace",
        &[Val::I32, Val::I32, Val::I32],
        &[Val::I32],
        &[Val::I32; 10],
        body,
    )
}

/// `str_split(s, delim, tag) -> list base` — a `list<string>` of the
/// byte ranges between non-overlapping occurrences of `delim` (left to
/// right); an empty delimiter yields one element (`s` itself). The list
/// object is allocated **before** the pieces: `LiftString` allocates too,
/// so the two must not interleave one region (fine with the bump
/// allocator). Params: s@0, delim@1, tag@2. Locals: ls@3, ld@4, count@5,
/// i@6, j@7, list@8, seg@9, k@10.
fn str_split() -> MirFunction {
    use Inst::*;
    let mut count_outer = vec![
        LocalGet(6),
        LocalGet(4),
        I32Bin(I32Op::Add),
        LocalGet(3),
        I32Cmp(CmpOp::GtU),
        BrIf(1),
    ];
    count_outer.extend(match_at(
        0,
        1,
        6,
        4,
        7,
        vec![
            LocalGet(5),
            I32Const(1),
            I32Bin(I32Op::Add),
            LocalSet(5),
            LocalGet(6),
            LocalGet(4),
            I32Bin(I32Op::Add),
            LocalSet(6),
            Br(3),
        ],
    ));
    count_outer.extend([
        LocalGet(6),
        I32Const(1),
        I32Bin(I32Op::Add),
        LocalSet(6),
        Br(0),
    ]);
    let mut fill_outer = vec![
        LocalGet(6),
        LocalGet(4),
        I32Bin(I32Op::Add),
        LocalGet(3),
        I32Cmp(CmpOp::GtU),
        BrIf(1),
    ];
    fill_outer.extend(match_at(
        0,
        1,
        6,
        4,
        7,
        vec![
            // elems[k] = lift(s[seg..i]).
            LocalGet(8),
            LocalGet(10),
            I32Const(4),
            I32Bin(I32Op::Mul),
            I32Bin(I32Op::Add),
            LocalGet(0),
            I32Const(4),
            I32Bin(I32Op::Add),
            LocalGet(9),
            I32Bin(I32Op::Add),
            LocalGet(6),
            LocalGet(9),
            I32Bin(I32Op::Sub),
            CallRuntime(RuntimeFn::LiftString),
            I32Store(LIST_ELEMS_OFFSET),
            LocalGet(10),
            I32Const(1),
            I32Bin(I32Op::Add),
            LocalSet(10),
            LocalGet(6),
            LocalGet(4),
            I32Bin(I32Op::Add),
            LocalSet(6),
            LocalGet(6),
            LocalSet(9),
            Br(3),
        ],
    ));
    fill_outer.extend([
        LocalGet(6),
        I32Const(1),
        I32Bin(I32Op::Add),
        LocalSet(6),
        Br(0),
    ]);
    let body = vec![
        LocalGet(0),
        I32Load(0),
        LocalSet(3),
        LocalGet(1),
        I32Load(0),
        LocalSet(4),
        // Empty delimiter → one element: s itself.
        LocalGet(4),
        I32Eqz,
        If {
            result: None,
            then: vec![
                I32Const((LIST_ELEMS_OFFSET + 4) as i32),
                I32Const(ALIGNMENT as i32),
                CallRuntime(RuntimeFn::Alloc),
                LocalTee(8),
                I32Const(1),
                I32Store(LIST_LEN_OFFSET),
                LocalGet(8),
                I32Const(1),
                I32Store(LIST_CAP_OFFSET),
                LocalGet(8),
                LocalGet(2),
                I32Store(LIST_TAG_OFFSET),
                LocalGet(8),
                I32Const(0),
                I32Store(LIST_TAG_OFFSET + 4),
                LocalGet(8),
                LocalGet(0),
                I32Store(LIST_ELEMS_OFFSET),
                LocalGet(8),
                Return,
            ],
            els: vec![],
        },
        // Pass 1: count occurrences to size the list.
        I32Const(0),
        LocalSet(5),
        I32Const(0),
        LocalSet(6),
        Block {
            body: vec![Loop { body: count_outer }],
        },
        // list = alloc(16 + 4*(count+1), 8); header per MMD §3.4.1.
        I32Const(LIST_ELEMS_OFFSET as i32),
        LocalGet(5),
        I32Const(1),
        I32Bin(I32Op::Add),
        I32Const(4),
        I32Bin(I32Op::Mul),
        I32Bin(I32Op::Add),
        I32Const(ALIGNMENT as i32),
        CallRuntime(RuntimeFn::Alloc),
        LocalTee(8),
        LocalGet(5),
        I32Const(1),
        I32Bin(I32Op::Add),
        I32Store(LIST_LEN_OFFSET),
        LocalGet(8),
        LocalGet(5),
        I32Const(1),
        I32Bin(I32Op::Add),
        I32Store(LIST_CAP_OFFSET),
        LocalGet(8),
        LocalGet(2),
        I32Store(LIST_TAG_OFFSET),
        LocalGet(8),
        I32Const(0),
        I32Store(LIST_TAG_OFFSET + 4),
        // Pass 2: lift each piece and store its pointer.
        I32Const(0),
        LocalSet(6),
        I32Const(0),
        LocalSet(9),
        I32Const(0),
        LocalSet(10),
        Block {
            body: vec![Loop { body: fill_outer }],
        },
        // The final piece s[seg..ls].
        LocalGet(8),
        LocalGet(10),
        I32Const(4),
        I32Bin(I32Op::Mul),
        I32Bin(I32Op::Add),
        LocalGet(0),
        I32Const(4),
        I32Bin(I32Op::Add),
        LocalGet(9),
        I32Bin(I32Op::Add),
        LocalGet(3),
        LocalGet(9),
        I32Bin(I32Op::Sub),
        CallRuntime(RuntimeFn::LiftString),
        I32Store(LIST_ELEMS_OFFSET),
        LocalGet(8),
    ];
    function(
        "__clean_str_split",
        &[Val::I32, Val::I32, Val::I32],
        &[Val::I32],
        &[Val::I32; 8],
        body,
    )
}

/// `str_is_blank(s) -> i32` — 1 iff every payload byte is whitespace
/// (an empty string is blank). Params: s@0. Locals: ls@1, i@2, c@3.
fn str_is_blank() -> MirFunction {
    use Inst::*;
    let mut step = vec![LocalGet(2), LocalGet(1), I32Cmp(CmpOp::Eq), BrIf(1)];
    step.extend(payload_byte(0, 2));
    step.push(LocalSet(3));
    step.extend(is_ws(3));
    step.extend([
        I32Eqz,
        If {
            result: None,
            then: vec![I32Const(0), Return],
            els: vec![],
        },
        LocalGet(2),
        I32Const(1),
        I32Bin(I32Op::Add),
        LocalSet(2),
        Br(0),
    ]);
    let body = vec![
        LocalGet(0),
        I32Load(0),
        LocalSet(1),
        I32Const(0),
        LocalSet(2),
        Block {
            body: vec![Loop { body: step }],
        },
        I32Const(1),
    ];
    function(
        "__clean_str_is_blank",
        &[Val::I32],
        &[Val::I32],
        &[Val::I32; 3],
        body,
    )
}
