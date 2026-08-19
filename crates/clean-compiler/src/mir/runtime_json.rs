//! Chapter-15 JSON module: the [`RuntimeFn::JsonParse`]..=
//! `JsonSerializePretty` bodies, built as ordinary [`MirFunction`]s in
//! discriminant order (the contracts live on the enum in `runtime.rs`).
//! The accept/reject boundary is Platform 10 §RUN006–RUN010 exactly:
//! `json_parse` traps on invalid input and `json_try_parse` returns the
//! shared `none` box in exactly the same conditions — both bodies come
//! from one generator parameterized on the failure fragment.
//!
//! Values follow ADR 0005: scalars box into 16-byte `any` boxes, parsed
//! numbers into 24-byte tag-8 boxes carrying the exact source bytes
//! (round-trip fidelity comes from re-emitting that text verbatim, never
//! from f64 re-rendering), arrays into `list<any>` and objects into
//! `pairs<string, any>`, both with the compiler-local element-tag
//! sentinel `u32::MAX`.
//!
//! `CallRuntime` can only address enum-listed helpers, so neither the
//! parser nor the serializer recurses: both walk nesting with an explicit
//! frame stack in heap memory (16-byte frames — kind, container, state —
//! grown by doubling), a `mode` local switching between "produce a value"
//! and "advance the innermost open container", and one dispatch loop.
//! Failure exits are `Return`/`Unreachable` only, so no fragment ever
//! branches across the dispatch loop's labels; every inner `Block{Loop}`
//! is self-contained.

use crate::layout::{
    ALIGNMENT, ANY_TAG_BOOL, ANY_TAG_BYTES, ANY_TAG_INT, ANY_TAG_LIST, ANY_TAG_NONE, ANY_TAG_NUM,
    ANY_TAG_NUM_SRC, ANY_TAG_PAIRS, ANY_TAG_STR, LIST_CAP_OFFSET, LIST_ELEMS_OFFSET,
    LIST_LEN_OFFSET, LIST_TAG_OFFSET, NONE_BOX_ADDR, PAIRS_CAP_OFFSET, PAIRS_COUNT_OFFSET,
    PAIRS_ENTRIES_OFFSET,
};

use super::runtime::RuntimeFn;
use super::{CmpOp, F64Op, I32Op, Inst, MirFunction, Val};

/// The JSON helpers, in [`RuntimeFn`] discriminant order
/// (`JsonParse`..=`JsonSerializePretty`); `runtime::build` appends them
/// after the `any`-box helpers.
pub fn build() -> Vec<MirFunction> {
    vec![
        json_parse_fn("__clean_json_parse", true),
        json_parse_fn("__clean_json_try_parse", false),
        json_serialize_fn("__clean_json_serialize", false),
        json_serialize_fn("__clean_json_serialize_pretty", true),
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

// ---------------------------------------------------------------------
// Parser locals (params: s@0). The full map lives on `json_parse_fn`.
// ---------------------------------------------------------------------
const PL_LEN: u32 = 1;
const PL_I: u32 = 2;
const PL_C: u32 = 3;
const PL_STK: u32 = 4;
const PL_SP: u32 = 5;
const PL_SCAP: u32 = 6;
const PL_BOX: u32 = 7;
const PL_MODE: u32 = 8;
const PL_DONE: u32 = 9;
const PL_T0: u32 = 10;
const PL_T1: u32 = 11;
const PL_T2: u32 = 12;
const PL_BUF: u32 = 13;
const PL_BLEN: u32 = 14;
const PL_BCAP: u32 = 15;
const PL_START: u32 = 16;
const PL_FVAL: u32 = 17;
const PL_FSCALE: u32 = 18;
const PL_NEG: u32 = 19;
const PL_EXP: u32 = 20;
const PL_EXPNEG: u32 = 21;
const PL_CP: u32 = 22;
const PL_CP2: u32 = 23;
const PL_SRES: u32 = 24;
const PL_FRAME: u32 = 25;
const PL_CONT: u32 = 26;
const PL_NN: u32 = 27;

/// The RUN006–RUN010 reject path: trap for `json_parse`, the shared
/// `none` box for `json_try_parse`. Both forms end the function, so a
/// fail fragment is legal at any nesting depth.
fn p_fail(trap: bool) -> Vec<Inst> {
    if trap {
        vec![Inst::Unreachable]
    } else {
        vec![Inst::I32Const(NONE_BOX_ADDR as i32), Inst::Return]
    }
}

/// Consumes a pushed i32 condition: true → fail.
fn p_fail_if(trap: bool) -> Inst {
    Inst::If {
        result: None,
        then: p_fail(trap),
        els: vec![],
    }
}

/// `c = s[i]` (payload byte; the constant offset 4 skips the length).
fn p_read_c() -> Vec<Inst> {
    use Inst::*;
    vec![
        LocalGet(0),
        LocalGet(PL_I),
        I32Bin(I32Op::Add),
        I32Load8U(4),
        LocalSet(PL_C),
    ]
}

/// `i += 1`.
fn p_inc_i() -> Vec<Inst> {
    use Inst::*;
    vec![
        LocalGet(PL_I),
        I32Const(1),
        I32Bin(I32Op::Add),
        LocalSet(PL_I),
    ]
}

/// Fails at end of input (`i == len`); `i` never exceeds `len`.
fn p_eof_fails(trap: bool) -> Vec<Inst> {
    use Inst::*;
    vec![
        LocalGet(PL_I),
        LocalGet(PL_LEN),
        I32Cmp(CmpOp::Eq),
        p_fail_if(trap),
    ]
}

/// Advances `i` past the RUN006 whitespace set (space, \t, \n, \r);
/// leaves the last inspected byte in `c` (stale at end of input).
fn p_skip_ws() -> Vec<Inst> {
    use Inst::*;
    vec![Block {
        body: vec![Loop {
            body: vec![
                LocalGet(PL_I),
                LocalGet(PL_LEN),
                I32Cmp(CmpOp::Eq),
                BrIf(1),
                LocalGet(0),
                LocalGet(PL_I),
                I32Bin(I32Op::Add),
                I32Load8U(4),
                LocalTee(PL_C),
                I32Const(0x20),
                I32Cmp(CmpOp::Eq),
                LocalGet(PL_C),
                I32Const(0x09),
                I32Cmp(CmpOp::Eq),
                I32Bin(I32Op::Or),
                LocalGet(PL_C),
                I32Const(0x0A),
                I32Cmp(CmpOp::Eq),
                I32Bin(I32Op::Or),
                LocalGet(PL_C),
                I32Const(0x0D),
                I32Cmp(CmpOp::Eq),
                I32Bin(I32Op::Or),
                I32Eqz,
                BrIf(1),
                LocalGet(PL_I),
                I32Const(1),
                I32Bin(I32Op::Add),
                LocalSet(PL_I),
                Br(0),
            ],
        }],
    }]
}

/// Pushes 1 iff `c` is an ASCII digit.
fn p_is_digit() -> Vec<Inst> {
    use Inst::*;
    vec![
        LocalGet(PL_C),
        I32Const('0' as i32),
        I32Cmp(CmpOp::GeS),
        LocalGet(PL_C),
        I32Const('9' as i32),
        I32Cmp(CmpOp::LeS),
        I32Bin(I32Op::And),
    ]
}

/// `if c == byte { then } else { els }` — the dispatch-chain cell.
fn p_if_c_eq(byte: i32, then: Vec<Inst>, els: Vec<Inst>) -> Vec<Inst> {
    use Inst::*;
    vec![
        LocalGet(PL_C),
        I32Const(byte),
        I32Cmp(CmpOp::Eq),
        If {
            result: None,
            then,
            els,
        },
    ]
}

/// Appends the byte in `t0` to the string-build buffer (`buf`/`blen`/
/// `bcap`), doubling on full. Clobbers `t1`, `t2`.
fn p_append_byte() -> Vec<Inst> {
    use Inst::*;
    vec![
        LocalGet(PL_BLEN),
        LocalGet(PL_BCAP),
        I32Cmp(CmpOp::Eq),
        If {
            result: None,
            then: vec![
                LocalGet(PL_BCAP),
                I32Const(2),
                I32Bin(I32Op::Mul),
                LocalSet(PL_T1),
                LocalGet(PL_T1),
                I32Const(ALIGNMENT as i32),
                CallRuntime(RuntimeFn::Alloc),
                LocalSet(PL_T2),
                LocalGet(PL_T2),
                LocalGet(PL_BUF),
                LocalGet(PL_BLEN),
                MemoryCopy,
                LocalGet(PL_T2),
                LocalSet(PL_BUF),
                LocalGet(PL_T1),
                LocalSet(PL_BCAP),
            ],
            els: vec![],
        },
        LocalGet(PL_BUF),
        LocalGet(PL_BLEN),
        I32Bin(I32Op::Add),
        LocalGet(PL_T0),
        I32Store8(0),
        LocalGet(PL_BLEN),
        I32Const(1),
        I32Bin(I32Op::Add),
        LocalSet(PL_BLEN),
    ]
}

/// Allocates a 16-byte `any` box into `boxv`: tag, zero pad, and the
/// pushed i32 `payload` fragment at +8.
fn p_box16(tag: u32, payload: Vec<Inst>) -> Vec<Inst> {
    use Inst::*;
    let mut v = vec![
        I32Const(16),
        I32Const(ALIGNMENT as i32),
        CallRuntime(RuntimeFn::Alloc),
        LocalSet(PL_BOX),
        LocalGet(PL_BOX),
        I32Const(tag as i32),
        I32Store(0),
        LocalGet(PL_BOX),
        I32Const(0),
        I32Store(4),
        LocalGet(PL_BOX),
    ];
    v.extend(payload);
    v.push(I32Store(8));
    v
}

/// RUN008 `\uXXXX` payload: four hex digits from `i` into `dst`
/// (rejecting non-hex), advancing `i` by 4. Uses `c`.
fn p_hex4(dst: u32, trap: bool) -> Vec<Inst> {
    use Inst::*;
    let mut v = vec![
        LocalGet(PL_I),
        I32Const(4),
        I32Bin(I32Op::Add),
        LocalGet(PL_LEN),
        I32Cmp(CmpOp::GtS),
        p_fail_if(trap),
        I32Const(0),
        LocalSet(dst),
    ];
    for _ in 0..4 {
        v.extend(p_read_c());
        // valid = digit | a-f | A-F, else fail
        v.extend(p_is_digit());
        v.extend([
            LocalGet(PL_C),
            I32Const('a' as i32),
            I32Cmp(CmpOp::GeS),
            LocalGet(PL_C),
            I32Const('f' as i32),
            I32Cmp(CmpOp::LeS),
            I32Bin(I32Op::And),
            I32Bin(I32Op::Or),
            LocalGet(PL_C),
            I32Const('A' as i32),
            I32Cmp(CmpOp::GeS),
            LocalGet(PL_C),
            I32Const('F' as i32),
            I32Cmp(CmpOp::LeS),
            I32Bin(I32Op::And),
            I32Bin(I32Op::Or),
            I32Eqz,
            p_fail_if(trap),
        ]);
        // dst = dst*16 + value(c) — validity established, so `c <= '9'`
        // means digit ('A'..'F' and 'a'..'f' both sit above '9').
        v.extend([
            LocalGet(dst),
            I32Const(16),
            I32Bin(I32Op::Mul),
            LocalGet(PL_C),
            I32Const('9' as i32),
            I32Cmp(CmpOp::LeS),
            If {
                result: Some(Val::I32),
                then: vec![LocalGet(PL_C), I32Const('0' as i32), I32Bin(I32Op::Sub)],
                els: vec![
                    LocalGet(PL_C),
                    I32Const('a' as i32),
                    I32Cmp(CmpOp::GeS),
                    If {
                        result: Some(Val::I32),
                        then: vec![LocalGet(PL_C), I32Const(87), I32Bin(I32Op::Sub)],
                        els: vec![LocalGet(PL_C), I32Const(55), I32Bin(I32Op::Sub)],
                    },
                ],
            },
            I32Bin(I32Op::Add),
            LocalSet(dst),
        ]);
        v.extend(p_inc_i());
    }
    v
}

/// UTF-8-encodes the code point in `cp` (already validated, ≤ 0x10FFFF,
/// no surrogate) into the string-build buffer. Clobbers `t0`..`t2`.
fn p_utf8_append() -> Vec<Inst> {
    use Inst::*;
    // t0 = prefix | (cp >> shift) [& 0x3F for continuations]; append.
    let piece = |prefix: i32, shift: i32, mask: bool| -> Vec<Inst> {
        let mut v = vec![LocalGet(PL_CP)];
        if shift > 0 {
            v.extend([I32Const(shift), I32Bin(I32Op::ShrU)]);
        }
        if mask {
            v.extend([I32Const(0x3F), I32Bin(I32Op::And)]);
        }
        v.extend([I32Const(prefix), I32Bin(I32Op::Or), LocalSet(PL_T0)]);
        v.extend(p_append_byte());
        v
    };
    let mut four = piece(0xF0, 18, false);
    four.extend(piece(0x80, 12, true));
    four.extend(piece(0x80, 6, true));
    four.extend(piece(0x80, 0, true));
    let mut three = piece(0xE0, 12, false);
    three.extend(piece(0x80, 6, true));
    three.extend(piece(0x80, 0, true));
    let mut two = piece(0xC0, 6, false);
    two.extend(piece(0x80, 0, true));
    let mut one = vec![LocalGet(PL_CP), LocalSet(PL_T0)];
    one.extend(p_append_byte());
    vec![
        LocalGet(PL_CP),
        I32Const(0x80),
        I32Cmp(CmpOp::LtU),
        If {
            result: None,
            then: one,
            els: vec![
                LocalGet(PL_CP),
                I32Const(0x800),
                I32Cmp(CmpOp::LtU),
                If {
                    result: None,
                    then: two,
                    els: vec![
                        LocalGet(PL_CP),
                        I32Const(0x10000),
                        I32Cmp(CmpOp::LtU),
                        If {
                            result: None,
                            then: three,
                            els: four,
                        },
                    ],
                },
            ],
        },
    ]
}

/// RUN008: parses the string whose opening `"` sits at `i` (the caller
/// already matched `c == '"'`), leaving a fresh string object in `sres`.
/// Single pass into a growable byte buffer, then `LiftString`. Rejects
/// unterminated strings, raw control bytes < 0x20, unknown escapes,
/// non-hex `\u` payloads and lone/unpaired surrogates; a high surrogate
/// must be followed by `\uXXXX` holding a low one (combined to the
/// supplementary code point). Clobbers `c`, `t0`..`t2`, `buf`/`blen`/
/// `bcap`, `cp`, `cp2`.
fn p_parse_string(trap: bool) -> Vec<Inst> {
    use Inst::*;
    // \uXXXX handling (after the 'u' was consumed).
    let mut u_body = p_hex4(PL_CP, trap);
    let mut pair_then = vec![
        // The low half must literally be `\uXXXX`.
        LocalGet(PL_I),
        I32Const(2),
        I32Bin(I32Op::Add),
        LocalGet(PL_LEN),
        I32Cmp(CmpOp::GtS),
        p_fail_if(trap),
        LocalGet(0),
        LocalGet(PL_I),
        I32Bin(I32Op::Add),
        I32Load8U(4),
        I32Const('\\' as i32),
        I32Cmp(CmpOp::Ne),
        p_fail_if(trap),
        LocalGet(0),
        LocalGet(PL_I),
        I32Bin(I32Op::Add),
        I32Load8U(5),
        I32Const('u' as i32),
        I32Cmp(CmpOp::Ne),
        p_fail_if(trap),
        LocalGet(PL_I),
        I32Const(2),
        I32Bin(I32Op::Add),
        LocalSet(PL_I),
    ];
    pair_then.extend(p_hex4(PL_CP2, trap));
    pair_then.extend([
        LocalGet(PL_CP2),
        I32Const(0xDC00),
        I32Cmp(CmpOp::LtS),
        LocalGet(PL_CP2),
        I32Const(0xDFFF),
        I32Cmp(CmpOp::GtS),
        I32Bin(I32Op::Or),
        p_fail_if(trap),
        // cp = 0x10000 + ((cp - 0xD800) << 10) + (cp2 - 0xDC00)
        LocalGet(PL_CP),
        I32Const(0xD800),
        I32Bin(I32Op::Sub),
        I32Const(10),
        I32Bin(I32Op::Shl),
        LocalGet(PL_CP2),
        I32Const(0xDC00),
        I32Bin(I32Op::Sub),
        I32Bin(I32Op::Add),
        I32Const(0x10000),
        I32Bin(I32Op::Add),
        LocalSet(PL_CP),
    ]);
    u_body.extend([
        LocalGet(PL_CP),
        I32Const(0xD800),
        I32Cmp(CmpOp::GeS),
        LocalGet(PL_CP),
        I32Const(0xDBFF),
        I32Cmp(CmpOp::LeS),
        I32Bin(I32Op::And),
        If {
            result: None,
            then: pair_then,
            els: vec![
                // A lone low surrogate rejects.
                LocalGet(PL_CP),
                I32Const(0xDC00),
                I32Cmp(CmpOp::GeS),
                LocalGet(PL_CP),
                I32Const(0xDFFF),
                I32Cmp(CmpOp::LeS),
                I32Bin(I32Op::And),
                p_fail_if(trap),
            ],
        },
    ]);
    u_body.extend(p_utf8_append());

    // The escape dispatch, innermost (unknown escape → fail) outward.
    let simple = |value: i32| -> Vec<Inst> {
        let mut v = vec![Inst::I32Const(value), Inst::LocalSet(PL_T0)];
        v.extend(p_append_byte());
        v
    };
    let mut esc_chain = p_fail(trap);
    esc_chain = p_if_c_eq('u' as i32, u_body, esc_chain);
    esc_chain = p_if_c_eq('t' as i32, simple(0x09), esc_chain);
    esc_chain = p_if_c_eq('r' as i32, simple(0x0D), esc_chain);
    esc_chain = p_if_c_eq('n' as i32, simple(0x0A), esc_chain);
    esc_chain = p_if_c_eq('f' as i32, simple(0x0C), esc_chain);
    esc_chain = p_if_c_eq('b' as i32, simple(0x08), esc_chain);
    // \" \\ \/ pass the escaped byte through.
    let mut passthrough = vec![LocalGet(PL_C), LocalSet(PL_T0)];
    passthrough.extend(p_append_byte());
    let mut escape_body = p_inc_i(); // consume the backslash
    escape_body.extend(p_eof_fails(trap));
    escape_body.extend(p_read_c());
    escape_body.extend(p_inc_i()); // consume the escape character
    escape_body.extend([
        LocalGet(PL_C),
        I32Const('"' as i32),
        I32Cmp(CmpOp::Eq),
        LocalGet(PL_C),
        I32Const('\\' as i32),
        I32Cmp(CmpOp::Eq),
        I32Bin(I32Op::Or),
        LocalGet(PL_C),
        I32Const('/' as i32),
        I32Cmp(CmpOp::Eq),
        I32Bin(I32Op::Or),
        If {
            result: None,
            then: passthrough,
            els: esc_chain,
        },
    ]);

    // A plain byte: raw controls reject, everything else copies through
    // (input is valid UTF-8 by MMD-04 construction).
    let mut plain_body = vec![
        LocalGet(PL_C),
        I32Const(0x20),
        I32Cmp(CmpOp::LtU),
        p_fail_if(trap),
        LocalGet(PL_C),
        LocalSet(PL_T0),
    ];
    plain_body.extend(p_append_byte());
    plain_body.extend(p_inc_i());

    // Loop labels: If = 0, Loop = 1, Block = 2 — the closing quote exits
    // the Block with Br(2).
    let mut close_then = p_inc_i();
    close_then.push(Br(2));
    let mut lp = p_eof_fails(trap); // unterminated
    lp.extend(p_read_c());
    lp.extend([
        LocalGet(PL_C),
        I32Const('"' as i32),
        I32Cmp(CmpOp::Eq),
        If {
            result: None,
            then: close_then,
            els: vec![],
        },
        LocalGet(PL_C),
        I32Const('\\' as i32),
        I32Cmp(CmpOp::Eq),
        If {
            result: None,
            then: escape_body,
            els: plain_body,
        },
        Br(0),
    ]);

    let mut v = p_inc_i(); // consume the opening quote
    v.extend([
        I32Const(16),
        I32Const(ALIGNMENT as i32),
        CallRuntime(RuntimeFn::Alloc),
        LocalSet(PL_BUF),
        I32Const(0),
        LocalSet(PL_BLEN),
        I32Const(16),
        LocalSet(PL_BCAP),
        Block {
            body: vec![Loop { body: lp }],
        },
        LocalGet(PL_BUF),
        LocalGet(PL_BLEN),
        CallRuntime(RuntimeFn::LiftString),
        LocalSet(PL_SRES),
    ]);
    v
}

/// RUN007: parses the number whose first byte (`-` or digit, already in
/// `c`) sits at `i`. Naive accumulation for the f64; the exact consumed
/// span becomes the tag-8 box's source text. Rejects leading zeros on
/// multi-digit integers, a missing integer/fraction/exponent digit, and
/// a magnitude that overflows binary64 to infinity; `-0` is accepted.
/// Leaves the box in `boxv` and sets `mode = 1`.
fn p_parse_number(trap: bool) -> Vec<Inst> {
    use Inst::*;
    let mut v = vec![
        LocalGet(PL_I),
        LocalSet(PL_START),
        I32Const(0),
        LocalSet(PL_NEG),
        F64Const(0.0),
        LocalSet(PL_FVAL),
    ];
    // Sign.
    let mut sign_then = vec![I32Const(1), LocalSet(PL_NEG)];
    sign_then.extend(p_inc_i());
    sign_then.extend(p_eof_fails(trap));
    sign_then.extend(p_read_c());
    v.extend(p_if_c_eq('-' as i32, sign_then, vec![]));
    // At least one integer digit.
    v.extend(p_is_digit());
    v.extend([I32Eqz, p_fail_if(trap)]);
    // `0` takes no more digits; `1`-`9` accumulate.
    let mut zero_then = p_inc_i();
    {
        let mut next_digit_fails = p_read_c();
        next_digit_fails.extend(p_is_digit());
        next_digit_fails.push(p_fail_if(trap));
        zero_then.extend([
            LocalGet(PL_I),
            LocalGet(PL_LEN),
            I32Cmp(CmpOp::Ne),
            If {
                result: None,
                then: next_digit_fails,
                els: vec![],
            },
        ]);
    }
    let int_loop = {
        let mut lp = vec![
            // fval = fval*10 + (c - '0')
            LocalGet(PL_FVAL),
            F64Const(10.0),
            F64Bin(F64Op::Mul),
            LocalGet(PL_C),
            I32Const('0' as i32),
            I32Bin(I32Op::Sub),
            F64ConvertI32S,
            F64Bin(F64Op::Add),
            LocalSet(PL_FVAL),
        ];
        lp.extend(p_inc_i());
        lp.extend([LocalGet(PL_I), LocalGet(PL_LEN), I32Cmp(CmpOp::Eq), BrIf(1)]);
        lp.extend(p_read_c());
        lp.extend(p_is_digit());
        lp.extend([I32Eqz, BrIf(1), Br(0)]);
        vec![Block {
            body: vec![Loop { body: lp }],
        }]
    };
    v.extend(p_if_c_eq('0' as i32, zero_then, int_loop));
    // Optional fraction.
    let frac_body = {
        let mut b = p_inc_i();
        b.extend(p_eof_fails(trap));
        b.extend(p_read_c());
        b.extend(p_is_digit());
        b.extend([I32Eqz, p_fail_if(trap)]);
        b.extend([F64Const(1.0), LocalSet(PL_FSCALE)]);
        let mut lp = vec![
            LocalGet(PL_FSCALE),
            F64Const(10.0),
            F64Bin(F64Op::Div),
            LocalSet(PL_FSCALE),
            LocalGet(PL_FVAL),
            LocalGet(PL_C),
            I32Const('0' as i32),
            I32Bin(I32Op::Sub),
            F64ConvertI32S,
            LocalGet(PL_FSCALE),
            F64Bin(F64Op::Mul),
            F64Bin(F64Op::Add),
            LocalSet(PL_FVAL),
        ];
        lp.extend(p_inc_i());
        lp.extend([LocalGet(PL_I), LocalGet(PL_LEN), I32Cmp(CmpOp::Eq), BrIf(1)]);
        lp.extend(p_read_c());
        lp.extend(p_is_digit());
        lp.extend([I32Eqz, BrIf(1), Br(0)]);
        b.push(Block {
            body: vec![Loop { body: lp }],
        });
        b
    };
    let mut frac_check = p_read_c();
    frac_check.extend(p_if_c_eq('.' as i32, frac_body, vec![]));
    v.extend([
        LocalGet(PL_I),
        LocalGet(PL_LEN),
        I32Cmp(CmpOp::Ne),
        If {
            result: None,
            then: frac_check,
            els: vec![],
        },
    ]);
    // Optional exponent.
    let exp_body = {
        let mut b = p_inc_i();
        b.extend([I32Const(0), LocalSet(PL_EXPNEG)]);
        b.extend(p_eof_fails(trap));
        b.extend(p_read_c());
        let mut plus_then = p_inc_i();
        plus_then.extend(p_eof_fails(trap));
        plus_then.extend(p_read_c());
        let mut minus_then = vec![I32Const(1), LocalSet(PL_EXPNEG)];
        minus_then.extend(p_inc_i());
        minus_then.extend(p_eof_fails(trap));
        minus_then.extend(p_read_c());
        let minus_check = p_if_c_eq('-' as i32, minus_then, vec![]);
        b.extend(p_if_c_eq('+' as i32, plus_then, minus_check));
        b.extend(p_is_digit());
        b.extend([I32Eqz, p_fail_if(trap)]);
        b.extend([I32Const(0), LocalSet(PL_EXP)]);
        // Digit loop, clamped so the i32 cannot wrap.
        let mut lp = vec![
            LocalGet(PL_EXP),
            I32Const(10),
            I32Bin(I32Op::Mul),
            LocalGet(PL_C),
            I32Const('0' as i32),
            I32Bin(I32Op::Sub),
            I32Bin(I32Op::Add),
            LocalSet(PL_EXP),
            LocalGet(PL_EXP),
            I32Const(100_000),
            I32Cmp(CmpOp::GtS),
            If {
                result: None,
                then: vec![I32Const(100_000), LocalSet(PL_EXP)],
                els: vec![],
            },
        ];
        lp.extend(p_inc_i());
        lp.extend([LocalGet(PL_I), LocalGet(PL_LEN), I32Cmp(CmpOp::Eq), BrIf(1)]);
        lp.extend(p_read_c());
        lp.extend(p_is_digit());
        lp.extend([I32Eqz, BrIf(1), Br(0)]);
        b.push(Block {
            body: vec![Loop { body: lp }],
        });
        // Apply: nn = min(exp, 350) scale steps — beyond 350 the value is
        // ±inf (rejected below) or 0.0 (fine).
        b.extend([
            LocalGet(PL_EXP),
            LocalSet(PL_NN),
            LocalGet(PL_NN),
            I32Const(350),
            I32Cmp(CmpOp::GtS),
            If {
                result: None,
                then: vec![I32Const(350), LocalSet(PL_NN)],
                els: vec![],
            },
            Block {
                body: vec![Loop {
                    body: vec![
                        LocalGet(PL_NN),
                        I32Eqz,
                        BrIf(1),
                        LocalGet(PL_EXPNEG),
                        If {
                            result: None,
                            then: vec![
                                LocalGet(PL_FVAL),
                                F64Const(10.0),
                                F64Bin(F64Op::Div),
                                LocalSet(PL_FVAL),
                            ],
                            els: vec![
                                LocalGet(PL_FVAL),
                                F64Const(10.0),
                                F64Bin(F64Op::Mul),
                                LocalSet(PL_FVAL),
                            ],
                        },
                        LocalGet(PL_NN),
                        I32Const(1),
                        I32Bin(I32Op::Sub),
                        LocalSet(PL_NN),
                        Br(0),
                    ],
                }],
            },
        ]);
        b
    };
    let mut exp_check = p_read_c();
    exp_check.extend([
        LocalGet(PL_C),
        I32Const('e' as i32),
        I32Cmp(CmpOp::Eq),
        LocalGet(PL_C),
        I32Const('E' as i32),
        I32Cmp(CmpOp::Eq),
        I32Bin(I32Op::Or),
        If {
            result: None,
            then: exp_body,
            els: vec![],
        },
    ]);
    v.extend([
        LocalGet(PL_I),
        LocalGet(PL_LEN),
        I32Cmp(CmpOp::Ne),
        If {
            result: None,
            then: exp_check,
            els: vec![],
        },
    ]);
    // Sign, overflow rejection, tag-8 box.
    v.extend([
        LocalGet(PL_NEG),
        If {
            result: None,
            then: vec![
                LocalGet(PL_FVAL),
                F64Un(super::F64Un::Neg),
                LocalSet(PL_FVAL),
            ],
            els: vec![],
        },
        LocalGet(PL_FVAL),
        F64Const(f64::INFINITY),
        F64Cmp(CmpOp::Eq),
        LocalGet(PL_FVAL),
        F64Const(f64::NEG_INFINITY),
        F64Cmp(CmpOp::Eq),
        I32Bin(I32Op::Or),
        p_fail_if(trap),
        I32Const(24),
        I32Const(ALIGNMENT as i32),
        CallRuntime(RuntimeFn::Alloc),
        LocalSet(PL_BOX),
        LocalGet(PL_BOX),
        I32Const(ANY_TAG_NUM_SRC as i32),
        I32Store(0),
        LocalGet(PL_BOX),
        I32Const(0),
        I32Store(4),
        LocalGet(PL_BOX),
        LocalGet(PL_FVAL),
        F64Store(8),
        LocalGet(PL_BOX),
        LocalGet(0),
        I32Const(4),
        I32Bin(I32Op::Add),
        LocalGet(PL_START),
        I32Bin(I32Op::Add),
        LocalGet(PL_I),
        LocalGet(PL_START),
        I32Bin(I32Op::Sub),
        CallRuntime(RuntimeFn::LiftString),
        I32Store(16),
        I32Const(1),
        LocalSet(PL_MODE),
    ]);
    v
}

/// Matches the literal whose first byte (already dispatched on) sits at
/// `i`: checks `rest` byte-for-byte and advances `i` past the whole
/// word. Anything else is RUN006.
fn p_literal(rest: &[u8], trap: bool) -> Vec<Inst> {
    use Inst::*;
    let total = rest.len() as i32 + 1;
    let mut v = vec![
        LocalGet(PL_I),
        I32Const(total),
        I32Bin(I32Op::Add),
        LocalGet(PL_LEN),
        I32Cmp(CmpOp::GtS),
        p_fail_if(trap),
    ];
    for (k, &b) in rest.iter().enumerate() {
        v.extend([
            LocalGet(0),
            LocalGet(PL_I),
            I32Bin(I32Op::Add),
            I32Load8U(5 + k as u32),
            I32Const(b as i32),
            I32Cmp(CmpOp::Ne),
            p_fail_if(trap),
        ]);
    }
    v.extend([
        LocalGet(PL_I),
        I32Const(total),
        I32Bin(I32Op::Add),
        LocalSet(PL_I),
    ]);
    v
}

/// RUN010 depth gate + frame push: grows the frame stack by doubling,
/// allocates a fresh empty container (kind 0: `list<any>`, cap 8, tag
/// sentinel; kind 1: `pairs`, cap 8), and writes the 16-byte frame
/// `{kind @0, container @4, pending key @8}` at `stk + sp*16`, leaving
/// its address in `frame`. Clobbers `t1`, `t2`, `cont`.
fn p_push_frame(kind: i32, trap: bool) -> Vec<Inst> {
    use Inst::*;
    let mut v = vec![
        // Depth 1000 is legal; a push from sp == 1000 would exceed it.
        LocalGet(PL_SP),
        I32Const(1000),
        I32Cmp(CmpOp::GeS),
        p_fail_if(trap),
        LocalGet(PL_SP),
        LocalGet(PL_SCAP),
        I32Cmp(CmpOp::Eq),
        If {
            result: None,
            then: vec![
                LocalGet(PL_SCAP),
                I32Const(2),
                I32Bin(I32Op::Mul),
                LocalSet(PL_T1),
                LocalGet(PL_T1),
                I32Const(16),
                I32Bin(I32Op::Mul),
                I32Const(ALIGNMENT as i32),
                CallRuntime(RuntimeFn::Alloc),
                LocalSet(PL_T2),
                LocalGet(PL_T2),
                LocalGet(PL_STK),
                LocalGet(PL_SP),
                I32Const(16),
                I32Bin(I32Op::Mul),
                MemoryCopy,
                LocalGet(PL_T2),
                LocalSet(PL_STK),
                LocalGet(PL_T1),
                LocalSet(PL_SCAP),
            ],
            els: vec![],
        },
    ];
    if kind == 0 {
        v.extend([
            I32Const((LIST_ELEMS_OFFSET + 8 * 4) as i32),
            I32Const(ALIGNMENT as i32),
            CallRuntime(RuntimeFn::Alloc),
            LocalSet(PL_CONT),
            LocalGet(PL_CONT),
            I32Const(0),
            I32Store(LIST_LEN_OFFSET),
            LocalGet(PL_CONT),
            I32Const(8),
            I32Store(LIST_CAP_OFFSET),
            LocalGet(PL_CONT),
            I32Const(-1),
            I32Store(LIST_TAG_OFFSET),
            LocalGet(PL_CONT),
            I32Const(0),
            I32Store(LIST_TAG_OFFSET + 4),
        ]);
    } else {
        v.extend([
            I32Const((PAIRS_ENTRIES_OFFSET + 8 * 8) as i32),
            I32Const(ALIGNMENT as i32),
            CallRuntime(RuntimeFn::Alloc),
            LocalSet(PL_CONT),
            LocalGet(PL_CONT),
            I32Const(0),
            I32Store(PAIRS_COUNT_OFFSET),
            LocalGet(PL_CONT),
            I32Const(8),
            I32Store(PAIRS_CAP_OFFSET),
        ]);
    }
    v.extend([
        LocalGet(PL_STK),
        LocalGet(PL_SP),
        I32Const(16),
        I32Bin(I32Op::Mul),
        I32Bin(I32Op::Add),
        LocalSet(PL_FRAME),
        LocalGet(PL_FRAME),
        I32Const(kind),
        I32Store(0),
        LocalGet(PL_FRAME),
        LocalGet(PL_CONT),
        I32Store(4),
        LocalGet(PL_FRAME),
        I32Const(0),
        I32Store(8),
        LocalGet(PL_SP),
        I32Const(1),
        I32Bin(I32Op::Add),
        LocalSet(PL_SP),
    ]);
    v
}

/// `frame = stk + (sp - 1) * 16` — the innermost open container.
fn p_top_frame() -> Vec<Inst> {
    use Inst::*;
    vec![
        LocalGet(PL_STK),
        LocalGet(PL_SP),
        I32Const(1),
        I32Bin(I32Op::Sub),
        I32Const(16),
        I32Bin(I32Op::Mul),
        I32Bin(I32Op::Add),
        LocalSet(PL_FRAME),
    ]
}

/// Appends `boxv` to the top frame's list, growing by reallocation
/// (fresh, unaliased — the bump allocator never frees) and updating the
/// frame's container pointer. Clobbers `t0`..`t2`, `cont`.
fn p_append_list() -> Vec<Inst> {
    use Inst::*;
    vec![
        LocalGet(PL_FRAME),
        I32Load(4),
        LocalSet(PL_CONT),
        LocalGet(PL_CONT),
        I32Load(LIST_LEN_OFFSET),
        LocalSet(PL_T0),
        LocalGet(PL_T0),
        LocalGet(PL_CONT),
        I32Load(LIST_CAP_OFFSET),
        I32Cmp(CmpOp::Eq),
        If {
            result: None,
            then: vec![
                LocalGet(PL_CONT),
                I32Load(LIST_CAP_OFFSET),
                I32Const(2),
                I32Bin(I32Op::Mul),
                LocalSet(PL_T1),
                I32Const(LIST_ELEMS_OFFSET as i32),
                LocalGet(PL_T1),
                I32Const(4),
                I32Bin(I32Op::Mul),
                I32Bin(I32Op::Add),
                I32Const(ALIGNMENT as i32),
                CallRuntime(RuntimeFn::Alloc),
                LocalSet(PL_T2),
                LocalGet(PL_T2),
                LocalGet(PL_CONT),
                I32Const(LIST_ELEMS_OFFSET as i32),
                LocalGet(PL_T0),
                I32Const(4),
                I32Bin(I32Op::Mul),
                I32Bin(I32Op::Add),
                MemoryCopy,
                LocalGet(PL_T2),
                LocalGet(PL_T1),
                I32Store(LIST_CAP_OFFSET),
                LocalGet(PL_T2),
                LocalSet(PL_CONT),
                LocalGet(PL_FRAME),
                LocalGet(PL_CONT),
                I32Store(4),
            ],
            els: vec![],
        },
        LocalGet(PL_CONT),
        LocalGet(PL_T0),
        I32Const(4),
        I32Bin(I32Op::Mul),
        I32Bin(I32Op::Add),
        LocalGet(PL_BOX),
        I32Store(LIST_ELEMS_OFFSET),
        LocalGet(PL_CONT),
        LocalGet(PL_T0),
        I32Const(1),
        I32Bin(I32Op::Add),
        I32Store(LIST_LEN_OFFSET),
    ]
}

/// Appends the entry `(frame's pending key, boxv)` to the top frame's
/// pairs, growing like [`p_append_list`]. Clobbers `t0`..`t2`, `cont`.
fn p_append_pairs() -> Vec<Inst> {
    use Inst::*;
    vec![
        LocalGet(PL_FRAME),
        I32Load(4),
        LocalSet(PL_CONT),
        LocalGet(PL_CONT),
        I32Load(PAIRS_COUNT_OFFSET),
        LocalSet(PL_T0),
        LocalGet(PL_T0),
        LocalGet(PL_CONT),
        I32Load(PAIRS_CAP_OFFSET),
        I32Cmp(CmpOp::Eq),
        If {
            result: None,
            then: vec![
                LocalGet(PL_CONT),
                I32Load(PAIRS_CAP_OFFSET),
                I32Const(2),
                I32Bin(I32Op::Mul),
                LocalSet(PL_T1),
                I32Const(PAIRS_ENTRIES_OFFSET as i32),
                LocalGet(PL_T1),
                I32Const(8),
                I32Bin(I32Op::Mul),
                I32Bin(I32Op::Add),
                I32Const(ALIGNMENT as i32),
                CallRuntime(RuntimeFn::Alloc),
                LocalSet(PL_T2),
                LocalGet(PL_T2),
                LocalGet(PL_CONT),
                I32Const(PAIRS_ENTRIES_OFFSET as i32),
                LocalGet(PL_T0),
                I32Const(8),
                I32Bin(I32Op::Mul),
                I32Bin(I32Op::Add),
                MemoryCopy,
                LocalGet(PL_T2),
                LocalGet(PL_T1),
                I32Store(PAIRS_CAP_OFFSET),
                LocalGet(PL_T2),
                LocalSet(PL_CONT),
                LocalGet(PL_FRAME),
                LocalGet(PL_CONT),
                I32Store(4),
            ],
            els: vec![],
        },
        LocalGet(PL_CONT),
        LocalGet(PL_T0),
        I32Const(8),
        I32Bin(I32Op::Mul),
        I32Bin(I32Op::Add),
        LocalGet(PL_FRAME),
        I32Load(8),
        I32Store(PAIRS_ENTRIES_OFFSET),
        LocalGet(PL_CONT),
        LocalGet(PL_T0),
        I32Const(8),
        I32Bin(I32Op::Mul),
        I32Bin(I32Op::Add),
        LocalGet(PL_BOX),
        I32Store(PAIRS_ENTRIES_OFFSET + 4),
        LocalGet(PL_CONT),
        LocalGet(PL_T0),
        I32Const(1),
        I32Bin(I32Op::Add),
        I32Store(PAIRS_COUNT_OFFSET),
    ]
}

/// RUN009: rejects when `sres` byte-equals any key already stored in
/// the top frame's pairs. Clobbers `t0`, `nn`, `cont`.
fn p_dup_check(trap: bool) -> Vec<Inst> {
    use Inst::*;
    vec![
        LocalGet(PL_FRAME),
        I32Load(4),
        LocalSet(PL_CONT),
        LocalGet(PL_CONT),
        I32Load(PAIRS_COUNT_OFFSET),
        LocalSet(PL_T0),
        I32Const(0),
        LocalSet(PL_NN),
        Block {
            body: vec![Loop {
                body: vec![
                    LocalGet(PL_NN),
                    LocalGet(PL_T0),
                    I32Cmp(CmpOp::Eq),
                    BrIf(1),
                    LocalGet(PL_CONT),
                    LocalGet(PL_NN),
                    I32Const(8),
                    I32Bin(I32Op::Mul),
                    I32Bin(I32Op::Add),
                    I32Load(PAIRS_ENTRIES_OFFSET),
                    LocalGet(PL_SRES),
                    CallRuntime(RuntimeFn::StringEq),
                    p_fail_if(trap),
                    LocalGet(PL_NN),
                    I32Const(1),
                    I32Bin(I32Op::Add),
                    LocalSet(PL_NN),
                    Br(0),
                ],
            }],
        },
    ]
}

/// Boxes the top frame's finished container (`tag` 6 or 7), pops the
/// frame, and re-enters have-value mode so the parent consumes the box.
fn p_finish(tag: u32) -> Vec<Inst> {
    use Inst::*;
    let mut v = vec![LocalGet(PL_FRAME), I32Load(4), LocalSet(PL_CONT)];
    v.extend(p_box16(tag, vec![LocalGet(PL_CONT)]));
    v.extend([
        LocalGet(PL_SP),
        I32Const(1),
        I32Bin(I32Op::Sub),
        LocalSet(PL_SP),
        I32Const(1),
        LocalSet(PL_MODE),
    ]);
    v
}

/// The member-key sequence with `c` already holding the byte at `i`:
/// `"key"` (RUN009 duplicate check against the top frame's pairs), `:`,
/// then expect-value mode. Stores the key in the frame's pending slot.
fn p_key_checked(trap: bool) -> Vec<Inst> {
    use Inst::*;
    let mut v = vec![
        LocalGet(PL_C),
        I32Const('"' as i32),
        I32Cmp(CmpOp::Ne),
        p_fail_if(trap),
    ];
    v.extend(p_parse_string(trap));
    v.extend(p_dup_check(trap));
    v.extend([LocalGet(PL_FRAME), LocalGet(PL_SRES), I32Store(8)]);
    v.extend(p_skip_ws());
    v.extend(p_eof_fails(trap));
    v.extend(p_read_c());
    v.extend([
        LocalGet(PL_C),
        I32Const(':' as i32),
        I32Cmp(CmpOp::Ne),
        p_fail_if(trap),
    ]);
    v.extend(p_inc_i());
    v.extend([I32Const(0), LocalSet(PL_MODE)]);
    v
}

/// Expect-value mode: whitespace, then one of `[ { " t f n -` or a
/// digit; anything else is RUN006. Containers push a frame (with the
/// empty-container fast close), scalars box and switch to have-value
/// mode.
fn p_mode0(trap: bool) -> Vec<Inst> {
    use Inst::*;
    // Innermost: not a value start.
    let mut chain = p_fail(trap);
    // Numbers.
    {
        let mut cond = vec![LocalGet(PL_C), I32Const('-' as i32), I32Cmp(CmpOp::Eq)];
        cond.extend(p_is_digit());
        cond.push(I32Bin(I32Op::Or));
        cond.push(If {
            result: None,
            then: p_parse_number(trap),
            els: chain,
        });
        chain = cond;
    }
    // null / false / true.
    {
        let mut body = p_literal(b"ull", trap);
        body.extend([
            I32Const(NONE_BOX_ADDR as i32),
            LocalSet(PL_BOX),
            I32Const(1),
            LocalSet(PL_MODE),
        ]);
        chain = p_if_c_eq('n' as i32, body, chain);
    }
    {
        let mut body = p_literal(b"alse", trap);
        body.extend(p_box16(ANY_TAG_BOOL, vec![I32Const(0)]));
        body.extend([I32Const(1), LocalSet(PL_MODE)]);
        chain = p_if_c_eq('f' as i32, body, chain);
    }
    {
        let mut body = p_literal(b"rue", trap);
        body.extend(p_box16(ANY_TAG_BOOL, vec![I32Const(1)]));
        body.extend([I32Const(1), LocalSet(PL_MODE)]);
        chain = p_if_c_eq('t' as i32, body, chain);
    }
    // String value.
    {
        let mut body = p_parse_string(trap);
        body.extend(p_box16(ANY_TAG_STR, vec![LocalGet(PL_SRES)]));
        body.extend([I32Const(1), LocalSet(PL_MODE)]);
        chain = p_if_c_eq('"' as i32, body, chain);
    }
    // Object open.
    {
        let mut body = p_inc_i();
        body.extend(p_push_frame(1, trap));
        body.extend(p_skip_ws());
        body.extend(p_eof_fails(trap));
        body.extend(p_read_c());
        let mut close = p_inc_i();
        close.extend(p_finish(ANY_TAG_PAIRS));
        body.extend(p_if_c_eq('}' as i32, close, p_key_checked(trap)));
        chain = p_if_c_eq('{' as i32, body, chain);
    }
    // Array open.
    {
        let mut body = p_inc_i();
        body.extend(p_push_frame(0, trap));
        body.extend(p_skip_ws());
        body.extend(p_eof_fails(trap));
        body.extend(p_read_c());
        let mut close = p_inc_i();
        close.extend(p_finish(ANY_TAG_LIST));
        body.extend(p_if_c_eq(']' as i32, close, vec![]));
        chain = p_if_c_eq('[' as i32, body, chain);
    }
    let mut v = p_skip_ws();
    v.extend(p_eof_fails(trap));
    v.extend(p_read_c());
    v.extend(chain);
    v
}

/// Have-value mode: at depth 0 the document is complete; otherwise the
/// finished value in `boxv` lands in the innermost container, and the
/// next byte must continue (`,`) or close (`]`/`}`) it — RUN009
/// otherwise.
fn p_mode1(trap: bool) -> Vec<Inst> {
    use Inst::*;
    let array_advance = {
        let mut a = p_append_list();
        a.extend(p_skip_ws());
        a.extend(p_eof_fails(trap));
        a.extend(p_read_c());
        let mut comma = p_inc_i();
        comma.extend([I32Const(0), LocalSet(PL_MODE)]);
        let mut close = p_inc_i();
        close.extend(p_finish(ANY_TAG_LIST));
        let close_chain = p_if_c_eq(']' as i32, close, p_fail(trap));
        a.extend(p_if_c_eq(',' as i32, comma, close_chain));
        a
    };
    let object_advance = {
        let mut o = p_append_pairs();
        o.extend(p_skip_ws());
        o.extend(p_eof_fails(trap));
        o.extend(p_read_c());
        let mut comma = p_inc_i();
        comma.extend(p_skip_ws());
        comma.extend(p_eof_fails(trap));
        comma.extend(p_read_c());
        comma.extend(p_key_checked(trap));
        let mut close = p_inc_i();
        close.extend(p_finish(ANY_TAG_PAIRS));
        let close_chain = p_if_c_eq('}' as i32, close, p_fail(trap));
        o.extend(p_if_c_eq(',' as i32, comma, close_chain));
        o
    };
    let mut els = p_top_frame();
    els.extend([
        LocalGet(PL_FRAME),
        I32Load(0),
        If {
            result: None,
            then: object_advance,
            els: array_advance,
        },
    ]);
    vec![
        LocalGet(PL_SP),
        I32Eqz,
        If {
            result: None,
            then: vec![I32Const(1), LocalSet(PL_DONE)],
            els,
        },
    ]
}

/// `json_parse(s) -> box` / `json_try_parse(s) -> box` — one grammar
/// (RUN006–RUN010), two failure fragments: trap, or return the shared
/// `none` box. Params: s@0. Locals: len@1, i@2, c@3, stk@4, sp@5,
/// scap@6, boxv@7, mode@8, done@9, t0@10, t1@11, t2@12, buf@13,
/// blen@14, bcap@15, start@16, fval@17 (f64), fscale@18 (f64), neg@19,
/// exp@20, expneg@21, cp@22, cp2@23, sres@24, frame@25, cont@26, nn@27.
fn json_parse_fn(name: &str, trap: bool) -> MirFunction {
    use Inst::*;
    let mut body = vec![
        LocalGet(0),
        I32Load(0),
        LocalSet(PL_LEN),
        // Frame stack: 8 frames × 16 bytes to start.
        I32Const(128),
        I32Const(ALIGNMENT as i32),
        CallRuntime(RuntimeFn::Alloc),
        LocalSet(PL_STK),
        I32Const(8),
        LocalSet(PL_SCAP),
        // i, sp, mode, done all start at their zero defaults.
        Block {
            body: vec![Loop {
                body: vec![
                    LocalGet(PL_DONE),
                    BrIf(1),
                    LocalGet(PL_MODE),
                    If {
                        result: None,
                        then: p_mode1(trap),
                        els: p_mode0(trap),
                    },
                    Br(0),
                ],
            }],
        },
    ];
    // RUN009: trailing non-whitespace after the root value rejects.
    body.extend(p_skip_ws());
    body.extend([
        LocalGet(PL_I),
        LocalGet(PL_LEN),
        I32Cmp(CmpOp::Ne),
        p_fail_if(trap),
        LocalGet(PL_BOX),
    ]);
    let mut locals = vec![Val::I32; 16];
    locals.extend([Val::F64, Val::F64]);
    locals.extend(vec![Val::I32; 9]);
    function(name, &[Val::I32], &[Val::I32], &locals, body)
}

// ---------------------------------------------------------------------
// Serializer locals (params: box@0). The full map lives on
// `json_serialize_fn`.
// ---------------------------------------------------------------------
const SL_STK: u32 = 1;
const SL_SP: u32 = 2;
const SL_SCAP: u32 = 3;
const SL_VAL: u32 = 4;
const SL_MODE: u32 = 5;
const SL_DONE: u32 = 6;
const SL_OBUF: u32 = 7;
const SL_OLEN: u32 = 8;
const SL_OCAP: u32 = 9;
const SL_T0: u32 = 10;
const SL_T1: u32 = 11;
const SL_T2: u32 = 12;
const SL_FRAME: u32 = 13;
const SL_CONT: u32 = 14;
const SL_IDX: u32 = 15;
const SL_N: u32 = 16;
const SL_P: u32 = 17;
const SL_Q: u32 = 18;
const SL_B: u32 = 19;
const SL_TAG: u32 = 20;
const SL_STRP: u32 = 21;
const SL_E10: u32 = 22;
const SL_DC: u32 = 23;
const SL_FV: u32 = 24;
const SL_FM: u32 = 25;

/// Appends the byte in `b` to the output buffer (`obuf`/`olen`/`ocap`),
/// doubling on full. Clobbers `t0`, `t1`.
fn s_append_byte() -> Vec<Inst> {
    use Inst::*;
    vec![
        LocalGet(SL_OLEN),
        LocalGet(SL_OCAP),
        I32Cmp(CmpOp::Eq),
        If {
            result: None,
            then: vec![
                LocalGet(SL_OCAP),
                I32Const(2),
                I32Bin(I32Op::Mul),
                LocalSet(SL_T0),
                LocalGet(SL_T0),
                I32Const(ALIGNMENT as i32),
                CallRuntime(RuntimeFn::Alloc),
                LocalSet(SL_T1),
                LocalGet(SL_T1),
                LocalGet(SL_OBUF),
                LocalGet(SL_OLEN),
                MemoryCopy,
                LocalGet(SL_T1),
                LocalSet(SL_OBUF),
                LocalGet(SL_T0),
                LocalSet(SL_OCAP),
            ],
            els: vec![],
        },
        LocalGet(SL_OBUF),
        LocalGet(SL_OLEN),
        I32Bin(I32Op::Add),
        LocalGet(SL_B),
        I32Store8(0),
        LocalGet(SL_OLEN),
        I32Const(1),
        I32Bin(I32Op::Add),
        LocalSet(SL_OLEN),
    ]
}

/// Appends a constant byte through `b`.
fn s_append_c(byte: i32) -> Vec<Inst> {
    use Inst::*;
    let mut v = vec![I32Const(byte), LocalSet(SL_B)];
    v.extend(s_append_byte());
    v
}

/// Appends a short ASCII literal (`null`, `true`, `false`).
fn s_append_lit(text: &str) -> Vec<Inst> {
    let mut v = Vec::new();
    for &b in text.as_bytes() {
        v.extend(s_append_c(b as i32));
    }
    v
}

/// Appends the payload of the string object in `strp` verbatim: one
/// capacity reservation (doubling until it fits), one `MemoryCopy`.
/// Clobbers `t0`..`t2`, `p`.
fn s_append_str() -> Vec<Inst> {
    use Inst::*;
    vec![
        LocalGet(SL_STRP),
        I32Load(0),
        LocalSet(SL_T0),
        LocalGet(SL_OLEN),
        LocalGet(SL_T0),
        I32Bin(I32Op::Add),
        LocalSet(SL_T1),
        LocalGet(SL_T1),
        LocalGet(SL_OCAP),
        I32Cmp(CmpOp::GtS),
        If {
            result: None,
            then: vec![
                LocalGet(SL_OCAP),
                LocalSet(SL_T2),
                Loop {
                    body: vec![
                        LocalGet(SL_T2),
                        I32Const(2),
                        I32Bin(I32Op::Mul),
                        LocalSet(SL_T2),
                        LocalGet(SL_T2),
                        LocalGet(SL_T1),
                        I32Cmp(CmpOp::LtS),
                        BrIf(0),
                    ],
                },
                LocalGet(SL_T2),
                I32Const(ALIGNMENT as i32),
                CallRuntime(RuntimeFn::Alloc),
                LocalSet(SL_P),
                LocalGet(SL_P),
                LocalGet(SL_OBUF),
                LocalGet(SL_OLEN),
                MemoryCopy,
                LocalGet(SL_P),
                LocalSet(SL_OBUF),
                LocalGet(SL_T2),
                LocalSet(SL_OCAP),
            ],
            els: vec![],
        },
        LocalGet(SL_OBUF),
        LocalGet(SL_OLEN),
        I32Bin(I32Op::Add),
        LocalGet(SL_STRP),
        I32Const(4),
        I32Bin(I32Op::Add),
        LocalGet(SL_T0),
        MemoryCopy,
        LocalGet(SL_OLEN),
        LocalGet(SL_T0),
        I32Bin(I32Op::Add),
        LocalSet(SL_OLEN),
    ]
}

/// Appends the string object in `strp` as a JSON string: quoted, with
/// `\"` `\\`, the `\n` `\r` `\t` short escapes, and `\u00xx` for the
/// remaining control bytes. Payload bytes ≥ 0x20 copy through (bytes
/// boxes may hold non-UTF-8 — accepted v1 behaviour). Clobbers `p`,
/// `q`, `t0`..`t2`, `b`.
fn s_emit_quoted() -> Vec<Inst> {
    use Inst::*;
    // \u00xx for control bytes without a short form: high nibble is 0 or
    // 1, the low nibble needs the letter branch.
    let mut u_escape = s_append_c('\\' as i32);
    u_escape.extend(s_append_c('u' as i32));
    u_escape.extend(s_append_c('0' as i32));
    u_escape.extend(s_append_c('0' as i32));
    u_escape.extend([
        LocalGet(SL_T2),
        I32Const(4),
        I32Bin(I32Op::ShrU),
        I32Const('0' as i32),
        I32Bin(I32Op::Add),
        LocalSet(SL_B),
    ]);
    u_escape.extend(s_append_byte());
    u_escape.extend([
        LocalGet(SL_T2),
        I32Const(15),
        I32Bin(I32Op::And),
        I32Const(10),
        I32Cmp(CmpOp::LtS),
        If {
            result: Some(Val::I32),
            then: vec![
                LocalGet(SL_T2),
                I32Const(15),
                I32Bin(I32Op::And),
                I32Const('0' as i32),
                I32Bin(I32Op::Add),
            ],
            els: vec![
                LocalGet(SL_T2),
                I32Const(15),
                I32Bin(I32Op::And),
                I32Const(87),
                I32Bin(I32Op::Add),
            ],
        },
        LocalSet(SL_B),
    ]);
    u_escape.extend(s_append_byte());

    let short = |esc: i32| -> Vec<Inst> {
        let mut v = s_append_c('\\' as i32);
        v.extend(s_append_c(esc));
        v
    };
    let if_t2_eq = |byte: i32, then: Vec<Inst>, els: Vec<Inst>| -> Vec<Inst> {
        vec![
            Inst::LocalGet(SL_T2),
            Inst::I32Const(byte),
            Inst::I32Cmp(CmpOp::Eq),
            Inst::If {
                result: None,
                then,
                els,
            },
        ]
    };
    let mut ctl_chain = u_escape;
    ctl_chain = if_t2_eq(0x09, short('t' as i32), ctl_chain);
    ctl_chain = if_t2_eq(0x0D, short('r' as i32), ctl_chain);
    ctl_chain = if_t2_eq(0x0A, short('n' as i32), ctl_chain);
    // Plain byte.
    let mut plain = vec![LocalGet(SL_T2), LocalSet(SL_B)];
    plain.extend(s_append_byte());
    let mut ctl_or_plain = vec![
        LocalGet(SL_T2),
        I32Const(0x20),
        I32Cmp(CmpOp::LtU),
        If {
            result: None,
            then: ctl_chain,
            els: plain,
        },
    ];
    ctl_or_plain = if_t2_eq('\\' as i32, short('\\' as i32), ctl_or_plain);
    ctl_or_plain = if_t2_eq('"' as i32, short('"' as i32), ctl_or_plain);

    let mut lp = vec![
        LocalGet(SL_P),
        LocalGet(SL_Q),
        I32Cmp(CmpOp::Eq),
        BrIf(1),
        LocalGet(SL_STRP),
        LocalGet(SL_P),
        I32Bin(I32Op::Add),
        I32Load8U(4),
        LocalSet(SL_T2),
    ];
    lp.extend(ctl_or_plain);
    lp.extend([
        LocalGet(SL_P),
        I32Const(1),
        I32Bin(I32Op::Add),
        LocalSet(SL_P),
        Br(0),
    ]);

    let mut v = s_append_c('"' as i32);
    v.extend([
        LocalGet(SL_STRP),
        I32Load(0),
        LocalSet(SL_Q),
        I32Const(0),
        LocalSet(SL_P),
        Block {
            body: vec![Loop { body: lp }],
        },
    ]);
    v.extend(s_append_c('"' as i32));
    v
}

/// Appends `sp` (or `sp - 1`) tab bytes — the pretty indentation.
/// Clobbers `p`, `b`.
fn s_tabs(minus_one: bool) -> Vec<Inst> {
    use Inst::*;
    let mut v = vec![LocalGet(SL_SP)];
    if minus_one {
        v.extend([I32Const(1), I32Bin(I32Op::Sub)]);
    }
    v.push(LocalSet(SL_P));
    let mut lp = vec![LocalGet(SL_P), I32Eqz, BrIf(1)];
    lp.extend(s_append_c(0x09));
    lp.extend([
        LocalGet(SL_P),
        I32Const(1),
        I32Bin(I32Op::Sub),
        LocalSet(SL_P),
        Br(0),
    ]);
    v.push(Block {
        body: vec![Loop { body: lp }],
    });
    v
}

/// The separator before element `idx` of the open container: compact
/// `,` between elements; pretty `,` + newline + `sp` tabs (a bare
/// newline before the first element).
fn s_sep(pretty: bool) -> Vec<Inst> {
    use Inst::*;
    if pretty {
        let mut later = s_append_c(',' as i32);
        later.extend(s_append_c(0x0A));
        let mut v = vec![
            LocalGet(SL_IDX),
            If {
                result: None,
                then: later,
                els: s_append_c(0x0A),
            },
        ];
        v.extend(s_tabs(false));
        v
    } else {
        vec![
            LocalGet(SL_IDX),
            If {
                result: None,
                then: s_append_c(',' as i32),
                els: vec![],
            },
        ]
    }
}

/// Closes the open container: pretty puts non-empty closers on their own
/// line at the enclosing indentation. Pops the frame.
fn s_close(byte: i32, pretty: bool) -> Vec<Inst> {
    use Inst::*;
    let mut v = Vec::new();
    if pretty {
        let mut nl = s_append_c(0x0A);
        nl.extend(s_tabs(true));
        v.extend([
            LocalGet(SL_N),
            If {
                result: None,
                then: nl,
                els: vec![],
            },
        ]);
    }
    v.extend(s_append_c(byte));
    v.extend([
        LocalGet(SL_SP),
        I32Const(1),
        I32Bin(I32Op::Sub),
        LocalSet(SL_SP),
    ]);
    v
}

/// Pushes a serializer frame `{kind @0, container @4, cursor @8}` for
/// the container held by the box in `val`, growing the frame stack by
/// doubling. Clobbers `t0`, `t1`, `cont`, `frame`.
fn s_push_frame(kind: i32) -> Vec<Inst> {
    use Inst::*;
    vec![
        LocalGet(SL_VAL),
        I32Load(8),
        LocalSet(SL_CONT),
        LocalGet(SL_SP),
        LocalGet(SL_SCAP),
        I32Cmp(CmpOp::Eq),
        If {
            result: None,
            then: vec![
                LocalGet(SL_SCAP),
                I32Const(2),
                I32Bin(I32Op::Mul),
                LocalSet(SL_T0),
                LocalGet(SL_T0),
                I32Const(16),
                I32Bin(I32Op::Mul),
                I32Const(ALIGNMENT as i32),
                CallRuntime(RuntimeFn::Alloc),
                LocalSet(SL_T1),
                LocalGet(SL_T1),
                LocalGet(SL_STK),
                LocalGet(SL_SP),
                I32Const(16),
                I32Bin(I32Op::Mul),
                MemoryCopy,
                LocalGet(SL_T1),
                LocalSet(SL_STK),
                LocalGet(SL_T0),
                LocalSet(SL_SCAP),
            ],
            els: vec![],
        },
        LocalGet(SL_STK),
        LocalGet(SL_SP),
        I32Const(16),
        I32Bin(I32Op::Mul),
        I32Bin(I32Op::Add),
        LocalSet(SL_FRAME),
        LocalGet(SL_FRAME),
        I32Const(kind),
        I32Store(0),
        LocalGet(SL_FRAME),
        LocalGet(SL_CONT),
        I32Store(4),
        LocalGet(SL_FRAME),
        I32Const(0),
        I32Store(8),
        LocalGet(SL_SP),
        I32Const(1),
        I32Bin(I32Op::Add),
        LocalSet(SL_SP),
    ]
}

/// Emits a tag-3 (`number`) box: non-finite values become `null` (JSON
/// has neither Infinity nor NaN, and parsing never produces them);
/// integral values with |v| < 2⁶³ take the exact `IntToString` path;
/// everything else renders as `d.dddddddddddddddd E exp` — 17
/// significant digits, round-trip-correct rather than shortest (ADR
/// 0005). Clobbers `fv`, `fm`, `e10`, `dc`, `t2`, `b`, `strp`.
fn s_num_emit() -> Vec<Inst> {
    use Inst::*;
    let mut int_path = vec![
        LocalGet(SL_FV),
        I64TruncF64S,
        CallRuntime(RuntimeFn::IntToString),
        LocalSet(SL_STRP),
    ];
    int_path.extend(s_append_str());

    // Scientific fallback.
    let mut sci = vec![
        LocalGet(SL_FV),
        F64Const(0.0),
        F64Cmp(CmpOp::LtS),
        If {
            result: None,
            then: s_append_c('-' as i32),
            els: vec![],
        },
        LocalGet(SL_FV),
        F64Un(super::F64Un::Abs),
        LocalSet(SL_FM),
        I32Const(0),
        LocalSet(SL_E10),
        // Normalize fm into [1, 10), counting decimal exponent steps.
        Block {
            body: vec![Loop {
                body: vec![
                    LocalGet(SL_FM),
                    F64Const(10.0),
                    F64Cmp(CmpOp::LtS),
                    BrIf(1),
                    LocalGet(SL_FM),
                    F64Const(10.0),
                    F64Bin(F64Op::Div),
                    LocalSet(SL_FM),
                    LocalGet(SL_E10),
                    I32Const(1),
                    I32Bin(I32Op::Add),
                    LocalSet(SL_E10),
                    Br(0),
                ],
            }],
        },
        Block {
            body: vec![Loop {
                body: vec![
                    LocalGet(SL_FM),
                    F64Const(1.0),
                    F64Cmp(CmpOp::GeS),
                    BrIf(1),
                    LocalGet(SL_FM),
                    F64Const(10.0),
                    F64Bin(F64Op::Mul),
                    LocalSet(SL_FM),
                    LocalGet(SL_E10),
                    I32Const(1),
                    I32Bin(I32Op::Sub),
                    LocalSet(SL_E10),
                    Br(0),
                ],
            }],
        },
        I32Const(0),
        LocalSet(SL_DC),
    ];
    // 17 digits, decimal point after the first. fm stays in [0, 10), so
    // the trunc-to-i64 never traps.
    let mut digit = vec![
        LocalGet(SL_FM),
        I64TruncF64S,
        I32WrapI64,
        LocalSet(SL_T2),
        LocalGet(SL_T2),
        I32Const('0' as i32),
        I32Bin(I32Op::Add),
        LocalSet(SL_B),
    ];
    digit.extend(s_append_byte());
    digit.extend([
        LocalGet(SL_FM),
        LocalGet(SL_T2),
        F64ConvertI32S,
        F64Bin(F64Op::Sub),
        F64Const(10.0),
        F64Bin(F64Op::Mul),
        LocalSet(SL_FM),
        LocalGet(SL_DC),
        I32Eqz,
        If {
            result: None,
            then: s_append_c('.' as i32),
            els: vec![],
        },
        LocalGet(SL_DC),
        I32Const(1),
        I32Bin(I32Op::Add),
        LocalTee(SL_DC),
        I32Const(17),
        I32Cmp(CmpOp::Eq),
        BrIf(1),
        Br(0),
    ]);
    sci.push(Block {
        body: vec![Loop { body: digit }],
    });
    sci.extend(s_append_c('E' as i32));
    sci.extend([
        LocalGet(SL_E10),
        I64ExtendI32S,
        CallRuntime(RuntimeFn::IntToString),
        LocalSet(SL_STRP),
    ]);
    sci.extend(s_append_str());

    vec![
        LocalGet(SL_VAL),
        F64Load(8),
        LocalSet(SL_FV),
        // NaN (fv != fv) or ±infinity → null.
        LocalGet(SL_FV),
        LocalGet(SL_FV),
        F64Cmp(CmpOp::Ne),
        LocalGet(SL_FV),
        F64Const(f64::INFINITY),
        F64Cmp(CmpOp::Eq),
        I32Bin(I32Op::Or),
        LocalGet(SL_FV),
        F64Const(f64::NEG_INFINITY),
        F64Cmp(CmpOp::Eq),
        I32Bin(I32Op::Or),
        If {
            result: None,
            then: s_append_lit("null"),
            els: vec![
                // Integral and safely inside i64 range?
                LocalGet(SL_FV),
                F64Un(super::F64Un::Abs),
                F64Const(9.2e18),
                F64Cmp(CmpOp::LtS),
                LocalGet(SL_FV),
                F64Un(super::F64Un::Trunc),
                LocalGet(SL_FV),
                F64Cmp(CmpOp::Eq),
                I32Bin(I32Op::And),
                If {
                    result: None,
                    then: int_path,
                    els: sci,
                },
            ],
        },
    ]
}

/// Emit-value mode: dispatch on the box tag. Scalars append their
/// rendering; containers append the opener and push a frame (the
/// separators and closers own the pretty layout, so this mode is the
/// same for both). An unregistered tag is an internal invariant breach
/// — trap.
fn s_mode0() -> Vec<Inst> {
    use Inst::*;
    let if_tag_eq = |tag: u32, then: Vec<Inst>, els: Vec<Inst>| -> Vec<Inst> {
        vec![
            Inst::LocalGet(SL_TAG),
            Inst::I32Const(tag as i32),
            Inst::I32Cmp(CmpOp::Eq),
            Inst::If {
                result: None,
                then,
                els,
            },
        ]
    };
    let mut chain = vec![Unreachable];
    // Tag 8: the parsed source text, verbatim (ADR 0005 fidelity).
    {
        let mut body = vec![LocalGet(SL_VAL), I32Load(16), LocalSet(SL_STRP)];
        body.extend(s_append_str());
        chain = if_tag_eq(ANY_TAG_NUM_SRC, body, chain);
    }
    // Tags 4/5: string and bytes both render quoted.
    {
        let mut body = vec![LocalGet(SL_VAL), I32Load(8), LocalSet(SL_STRP)];
        body.extend(s_emit_quoted());
        chain = vec![
            LocalGet(SL_TAG),
            I32Const(ANY_TAG_STR as i32),
            I32Cmp(CmpOp::Eq),
            LocalGet(SL_TAG),
            I32Const(ANY_TAG_BYTES as i32),
            I32Cmp(CmpOp::Eq),
            I32Bin(I32Op::Or),
            If {
                result: None,
                then: body,
                els: chain,
            },
        ];
    }
    {
        let mut body = s_append_c('{' as i32);
        body.extend(s_push_frame(1));
        chain = if_tag_eq(ANY_TAG_PAIRS, body, chain);
    }
    {
        let mut body = s_append_c('[' as i32);
        body.extend(s_push_frame(0));
        chain = if_tag_eq(ANY_TAG_LIST, body, chain);
    }
    chain = if_tag_eq(ANY_TAG_NUM, s_num_emit(), chain);
    {
        let mut body = vec![
            LocalGet(SL_VAL),
            I64Load(8),
            CallRuntime(RuntimeFn::IntToString),
            LocalSet(SL_STRP),
        ];
        body.extend(s_append_str());
        chain = if_tag_eq(ANY_TAG_INT, body, chain);
    }
    {
        let body = vec![
            LocalGet(SL_VAL),
            I32Load(8),
            If {
                result: None,
                then: s_append_lit("true"),
                els: s_append_lit("false"),
            },
        ];
        chain = if_tag_eq(ANY_TAG_BOOL, body, chain);
    }
    chain = if_tag_eq(ANY_TAG_NONE, s_append_lit("null"), chain);

    let mut v = vec![LocalGet(SL_VAL), I32Load(0), LocalSet(SL_TAG)];
    v.extend(chain);
    v.extend([I32Const(1), LocalSet(SL_MODE)]);
    v
}

/// Advance mode: at depth 0 the document is complete; otherwise either
/// close the innermost container or emit its next element (arrays) /
/// `"key":` member (objects, stored order) and drop back to emit mode.
fn s_mode1(pretty: bool) -> Vec<Inst> {
    use Inst::*;
    let array_arm = {
        let mut next = s_sep(pretty);
        next.extend([
            LocalGet(SL_CONT),
            LocalGet(SL_IDX),
            I32Const(4),
            I32Bin(I32Op::Mul),
            I32Bin(I32Op::Add),
            I32Load(LIST_ELEMS_OFFSET),
            LocalSet(SL_VAL),
            LocalGet(SL_FRAME),
            LocalGet(SL_IDX),
            I32Const(1),
            I32Bin(I32Op::Add),
            I32Store(8),
            I32Const(0),
            LocalSet(SL_MODE),
        ]);
        vec![
            LocalGet(SL_CONT),
            I32Load(LIST_LEN_OFFSET),
            LocalSet(SL_N),
            LocalGet(SL_IDX),
            LocalGet(SL_N),
            I32Cmp(CmpOp::Eq),
            If {
                result: None,
                then: s_close(']' as i32, pretty),
                els: next,
            },
        ]
    };
    let object_arm = {
        let mut next = s_sep(pretty);
        next.extend([
            LocalGet(SL_CONT),
            LocalGet(SL_IDX),
            I32Const(8),
            I32Bin(I32Op::Mul),
            I32Bin(I32Op::Add),
            I32Load(PAIRS_ENTRIES_OFFSET),
            LocalSet(SL_STRP),
        ]);
        next.extend(s_emit_quoted());
        next.extend(s_append_c(':' as i32));
        if pretty {
            next.extend(s_append_c(' ' as i32));
        }
        next.extend([
            LocalGet(SL_CONT),
            LocalGet(SL_IDX),
            I32Const(8),
            I32Bin(I32Op::Mul),
            I32Bin(I32Op::Add),
            I32Load(PAIRS_ENTRIES_OFFSET + 4),
            LocalSet(SL_VAL),
            LocalGet(SL_FRAME),
            LocalGet(SL_IDX),
            I32Const(1),
            I32Bin(I32Op::Add),
            I32Store(8),
            I32Const(0),
            LocalSet(SL_MODE),
        ]);
        vec![
            LocalGet(SL_CONT),
            I32Load(PAIRS_COUNT_OFFSET),
            LocalSet(SL_N),
            LocalGet(SL_IDX),
            LocalGet(SL_N),
            I32Cmp(CmpOp::Eq),
            If {
                result: None,
                then: s_close('}' as i32, pretty),
                els: next,
            },
        ]
    };
    let els = vec![
        LocalGet(SL_STK),
        LocalGet(SL_SP),
        I32Const(1),
        I32Bin(I32Op::Sub),
        I32Const(16),
        I32Bin(I32Op::Mul),
        I32Bin(I32Op::Add),
        LocalSet(SL_FRAME),
        LocalGet(SL_FRAME),
        I32Load(4),
        LocalSet(SL_CONT),
        LocalGet(SL_FRAME),
        I32Load(8),
        LocalSet(SL_IDX),
        LocalGet(SL_FRAME),
        I32Load(0),
        If {
            result: None,
            then: object_arm,
            els: array_arm,
        },
    ];
    vec![
        LocalGet(SL_SP),
        I32Eqz,
        If {
            result: None,
            then: vec![I32Const(1), LocalSet(SL_DONE)],
            els,
        },
    ]
}

/// `json_serialize(box) -> s` / `json_serialize_pretty(box) -> s` — one
/// generator, two layouts: compact, or tab-indented with one
/// member/element per line and `": "` after keys (a local adoption —
/// the pretty format is withheld from conformance claims, ADR 0005).
/// Iterative like the parser: an explicit frame stack, an output byte
/// buffer grown by doubling, `LiftString` at the end.
/// Params: box@0. Locals: stk@1, sp@2, scap@3, val@4, mode@5, done@6,
/// obuf@7, olen@8, ocap@9, t0@10, t1@11, t2@12, frame@13, cont@14,
/// idx@15, n@16, p@17, q@18, b@19, tag@20, strp@21, e10@22, dc@23,
/// fv@24 (f64), fm@25 (f64).
fn json_serialize_fn(name: &str, pretty: bool) -> MirFunction {
    use Inst::*;
    let body = vec![
        LocalGet(0),
        LocalSet(SL_VAL),
        I32Const(128),
        I32Const(ALIGNMENT as i32),
        CallRuntime(RuntimeFn::Alloc),
        LocalSet(SL_STK),
        I32Const(8),
        LocalSet(SL_SCAP),
        I32Const(64),
        I32Const(ALIGNMENT as i32),
        CallRuntime(RuntimeFn::Alloc),
        LocalSet(SL_OBUF),
        I32Const(64),
        LocalSet(SL_OCAP),
        // sp, olen, mode, done start at their zero defaults.
        Block {
            body: vec![Loop {
                body: vec![
                    LocalGet(SL_DONE),
                    BrIf(1),
                    LocalGet(SL_MODE),
                    If {
                        result: None,
                        then: s_mode1(pretty),
                        els: s_mode0(),
                    },
                    Br(0),
                ],
            }],
        },
        LocalGet(SL_OBUF),
        LocalGet(SL_OLEN),
        CallRuntime(RuntimeFn::LiftString),
    ];
    let mut locals = vec![Val::I32; 23];
    locals.extend([Val::F64, Val::F64]);
    function(name, &[Val::I32], &[Val::I32], &locals, body)
}
