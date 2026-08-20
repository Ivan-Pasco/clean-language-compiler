//! `number.toString` (15 §Conversions): the shortest round-trip rendering,
//! computed exactly in the guest — no host import, no floating-point
//! estimation. "Fewest digits that round-trip" is decided by the
//! Steele-White criterion over exact decimal expansions:
//!
//! 1. Decompose `v = m × 2^e` from the IEEE bits.
//! 2. Expand `v` and its two neighbor midpoints exactly — every binary
//!    float has a finite decimal expansion: `m × 2^e` for `e ≥ 0`,
//!    `m × 5^-e × 10^e` for `e < 0` — as base-10⁹ limbs, then digits.
//! 3. For k = 1, 2, …: the k-digit candidate nearest `v` (half-even),
//!    and if that fails its k-digit neighbor, is accepted iff it lies
//!    inside the midpoint interval (bounds inclusive iff the mantissa is
//!    even, mirroring round-to-nearest-even). Everything is digit-string
//!    arithmetic — exact, no rounding drift anywhere.
//!
//! Notation (normative since 15 §Conversions, 2026-08-20): plain decimal
//! for -4 ≤ E ≤ 21 with E from the NORMALIZED scientific form
//! `d₁.d₂…×10^E`, scientific otherwise; integral plain values append
//! `.0`; scientific mantissas carry no `.0` ("1e22"); spellings `NaN` /
//! `Infinity` / `-Infinity` and `-0.0` are output-only —
//! `string.toNumber` rejects the non-finite spellings (RUN003).

use super::runtime::RuntimeFn;
use super::{CmpOp, I32Op, I64Op, Inst, MirFunction, Val};

/// Scratch-buffer layout (one arena allocation per call). Limb areas hold
/// base-10⁹ u32 limbs (≤ 88 needed; 100 reserved), digit areas raw 0–9
/// bytes (≤ 785 needed; 800 reserved).
const LIMBS_V: i32 = 0;
const LIMBS_LO: i32 = 400;
const LIMBS_HI: i32 = 800;
const DIG_V: i32 = 1200;
const DIG_LO: i32 = 2000;
const DIG_HI: i32 = 2800;
const CAND: i32 = 3600;
const OUT: i32 = 3632;
const BUF_SIZE: i32 = 3712;

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

pub fn build() -> Vec<MirFunction> {
    vec![
        num_expand(),
        limbs_to_digits(),
        digits_cmp(),
        num_to_string(),
    ]
}

/// `num_expand(m: i64, e2: i32, limbs: i32) -> i32` — the exact integer
/// `m × 2^e2` (e2 ≥ 0) or `m × 5^(-e2)` (e2 < 0) as base-10⁹ limbs,
/// little-endian; returns the limb count. The caller accounts the
/// remaining `10^min(e2,0)` scale in the decimal exponent.
///
/// Params: m@0 (i64), e2@1, limbs@2. Locals: n@3, i@4, carry@5 (i64),
/// t@6 (i64), k@7, f@8 (i64), c@9.
fn num_expand() -> MirFunction {
    use Inst::*;
    let mut body = Vec::new();

    // Seed the limbs from m.
    body.extend([LocalGet(0), LocalSet(5), I32Const(0), LocalSet(3)]);
    body.push(Block {
        body: vec![Loop {
            body: vec![
                // while !(carry == 0 && n > 0)
                LocalGet(5),
                I64Const(0),
                I64Cmp(CmpOp::Eq),
                LocalGet(3),
                I32Const(0),
                I32Cmp(CmpOp::GtS),
                I32Bin(I32Op::And),
                BrIf(1),
                // limbs[n] = carry % 1e9
                LocalGet(2),
                LocalGet(3),
                I32Const(4),
                I32Bin(I32Op::Mul),
                I32Bin(I32Op::Add),
                LocalGet(5),
                I64Const(1_000_000_000),
                I64Bin(I64Op::RemU),
                I32WrapI64,
                I32Store(0),
                // carry /= 1e9; n += 1
                LocalGet(5),
                I64Const(1_000_000_000),
                I64Bin(I64Op::DivU),
                LocalSet(5),
                LocalGet(3),
                I32Const(1),
                I32Bin(I32Op::Add),
                LocalSet(3),
                Br(0),
            ],
        }],
    });

    // k = |e2|
    body.extend([
        LocalGet(1),
        I32Const(0),
        I32Cmp(CmpOp::LtS),
        If {
            result: Some(Val::I32),
            then: vec![I32Const(0), LocalGet(1), I32Bin(I32Op::Sub)],
            els: vec![LocalGet(1)],
        },
        LocalSet(7),
    ]);

    // Scale in chunks: ×2^min(k,29) or ×5^min(k,12).
    let chunk_factor = |base: i64| -> Vec<Inst> {
        // f = base^c via a small loop on c (clobbers c).
        vec![
            I64Const(1),
            LocalSet(8),
            Block {
                body: vec![Loop {
                    body: vec![
                        LocalGet(9),
                        I32Eqz,
                        BrIf(1),
                        LocalGet(8),
                        I64Const(base),
                        I64Bin(I64Op::Mul),
                        LocalSet(8),
                        LocalGet(9),
                        I32Const(1),
                        I32Bin(I32Op::Sub),
                        LocalSet(9),
                        Br(0),
                    ],
                }],
            },
        ]
    };
    let clamp_c = |max: i32| -> Vec<Inst> {
        // c = min(k, max); k -= c   (leaves c in local 9)
        vec![
            LocalGet(7),
            I32Const(max),
            LocalGet(7),
            I32Const(max),
            I32Cmp(CmpOp::LtS),
            Select,
            LocalSet(9),
            LocalGet(7),
            LocalGet(9),
            I32Bin(I32Op::Sub),
            LocalSet(7),
        ]
    };

    let mut two_chunk = clamp_c(29);
    two_chunk.extend(chunk_factor(2));
    let mut five_chunk = clamp_c(12);
    five_chunk.extend(chunk_factor(5));

    let mut scale_loop = vec![LocalGet(7), I32Eqz, BrIf(1)];
    scale_loop.extend([
        LocalGet(1),
        I32Const(0),
        I32Cmp(CmpOp::GeS),
        If {
            result: None,
            then: two_chunk,
            els: five_chunk,
        },
    ]);
    // Multiply every limb by f, carrying in base 1e9.
    scale_loop.extend([I64Const(0), LocalSet(5), I32Const(0), LocalSet(4)]);
    scale_loop.push(Block {
        body: vec![Loop {
            body: vec![
                LocalGet(4),
                LocalGet(3),
                I32Cmp(CmpOp::Eq),
                BrIf(1),
                // t = limbs[i] * f + carry
                LocalGet(2),
                LocalGet(4),
                I32Const(4),
                I32Bin(I32Op::Mul),
                I32Bin(I32Op::Add),
                I32Load(0),
                I64ExtendI32U,
                LocalGet(8),
                I64Bin(I64Op::Mul),
                LocalGet(5),
                I64Bin(I64Op::Add),
                LocalSet(6),
                // limbs[i] = t % 1e9; carry = t / 1e9
                LocalGet(2),
                LocalGet(4),
                I32Const(4),
                I32Bin(I32Op::Mul),
                I32Bin(I32Op::Add),
                LocalGet(6),
                I64Const(1_000_000_000),
                I64Bin(I64Op::RemU),
                I32WrapI64,
                I32Store(0),
                LocalGet(6),
                I64Const(1_000_000_000),
                I64Bin(I64Op::DivU),
                LocalSet(5),
                LocalGet(4),
                I32Const(1),
                I32Bin(I32Op::Add),
                LocalSet(4),
                Br(0),
            ],
        }],
    });
    // Flush the carry into new top limbs.
    scale_loop.push(Block {
        body: vec![Loop {
            body: vec![
                LocalGet(5),
                I64Const(0),
                I64Cmp(CmpOp::Eq),
                BrIf(1),
                LocalGet(2),
                LocalGet(3),
                I32Const(4),
                I32Bin(I32Op::Mul),
                I32Bin(I32Op::Add),
                LocalGet(5),
                I64Const(1_000_000_000),
                I64Bin(I64Op::RemU),
                I32WrapI64,
                I32Store(0),
                LocalGet(5),
                I64Const(1_000_000_000),
                I64Bin(I64Op::DivU),
                LocalSet(5),
                LocalGet(3),
                I32Const(1),
                I32Bin(I32Op::Add),
                LocalSet(3),
                Br(0),
            ],
        }],
    });
    scale_loop.push(Br(0));

    body.push(Block {
        body: vec![Loop { body: scale_loop }],
    });
    body.push(LocalGet(3));

    function(
        "__clean_num_expand",
        &[Val::I64, Val::I32, Val::I32],
        &[Val::I32],
        &[
            Val::I32,
            Val::I32,
            Val::I64,
            Val::I64,
            Val::I32,
            Val::I64,
            Val::I32,
        ],
        body,
    )
}

/// `limbs_to_digits(limbs: i32, n: i32, digits: i32) -> i32` — the decimal
/// digits (raw 0–9 bytes, most significant first, no leading zeros);
/// returns the digit count. The top limb is nonzero by construction.
///
/// Params: limbs@0, n@1, digits@2. Locals: i@3, p@4, v@5 (i64),
/// pow@6 (i64), d@7 (i64), started@8.
fn limbs_to_digits() -> MirFunction {
    use Inst::*;
    let mut body = vec![
        LocalGet(2),
        LocalSet(4),
        I32Const(0),
        LocalSet(8),
        LocalGet(1),
        LocalSet(3),
    ];
    body.push(Block {
        body: vec![Loop {
            body: vec![
                LocalGet(3),
                I32Eqz,
                BrIf(1),
                LocalGet(3),
                I32Const(1),
                I32Bin(I32Op::Sub),
                LocalSet(3),
                // v = limbs[i]
                LocalGet(0),
                LocalGet(3),
                I32Const(4),
                I32Bin(I32Op::Mul),
                I32Bin(I32Op::Add),
                I32Load(0),
                I64ExtendI32U,
                LocalSet(5),
                I64Const(100_000_000),
                LocalSet(6),
                Block {
                    body: vec![Loop {
                        body: vec![
                            LocalGet(6),
                            I64Const(0),
                            I64Cmp(CmpOp::Eq),
                            BrIf(1),
                            // d = v / pow % 10
                            LocalGet(5),
                            LocalGet(6),
                            I64Bin(I64Op::DivU),
                            I64Const(10),
                            I64Bin(I64Op::RemU),
                            LocalSet(7),
                            // emit unless still skipping leading zeros
                            LocalGet(8),
                            LocalGet(7),
                            I64Const(0),
                            I64Cmp(CmpOp::Ne),
                            I32Bin(I32Op::Or),
                            If {
                                result: None,
                                then: vec![
                                    LocalGet(4),
                                    LocalGet(7),
                                    I32WrapI64,
                                    I32Store8(0),
                                    LocalGet(4),
                                    I32Const(1),
                                    I32Bin(I32Op::Add),
                                    LocalSet(4),
                                    I32Const(1),
                                    LocalSet(8),
                                ],
                                els: vec![],
                            },
                            LocalGet(6),
                            I64Const(10),
                            I64Bin(I64Op::DivU),
                            LocalSet(6),
                            Br(0),
                        ],
                    }],
                },
                Br(0),
            ],
        }],
    });
    body.extend([LocalGet(4), LocalGet(2), I32Bin(I32Op::Sub)]);
    function(
        "__clean_limbs_to_digits",
        &[Val::I32, Val::I32, Val::I32],
        &[Val::I32],
        &[Val::I32, Val::I32, Val::I64, Val::I64, Val::I64, Val::I32],
        body,
    )
}

/// `digits_cmp(a, alen, aE, b, blen, bE) -> i32` — order of the two
/// positive values `0.A × 10^aE` and `0.B × 10^bE` (digit arrays are raw
/// 0–9 with no leading zero): -1, 0, or 1.
///
/// Params: a@0, alen@1, aE@2, b@3, blen@4, bE@5. Locals: i@6, da@7, db@8.
fn digits_cmp() -> MirFunction {
    use Inst::*;
    let digit_or_zero = |ptr: u32, len: u32| -> Vec<Inst> {
        vec![
            LocalGet(6),
            LocalGet(len),
            I32Cmp(CmpOp::LtS),
            If {
                result: Some(Val::I32),
                then: vec![LocalGet(ptr), LocalGet(6), I32Bin(I32Op::Add), I32Load8U(0)],
                els: vec![I32Const(0)],
            },
        ]
    };
    let mut loop_body = vec![
        // both exhausted → equal
        LocalGet(6),
        LocalGet(1),
        I32Cmp(CmpOp::GeS),
        LocalGet(6),
        LocalGet(4),
        I32Cmp(CmpOp::GeS),
        I32Bin(I32Op::And),
        If {
            result: None,
            then: vec![I32Const(0), Return],
            els: vec![],
        },
    ];
    loop_body.extend(digit_or_zero(0, 1));
    loop_body.push(LocalSet(7));
    loop_body.extend(digit_or_zero(3, 4));
    loop_body.push(LocalSet(8));
    loop_body.extend([
        LocalGet(7),
        LocalGet(8),
        I32Cmp(CmpOp::GtS),
        If {
            result: None,
            then: vec![I32Const(1), Return],
            els: vec![],
        },
        LocalGet(7),
        LocalGet(8),
        I32Cmp(CmpOp::LtS),
        If {
            result: None,
            then: vec![I32Const(-1), Return],
            els: vec![],
        },
        LocalGet(6),
        I32Const(1),
        I32Bin(I32Op::Add),
        LocalSet(6),
        Br(0),
    ]);
    let body = vec![
        LocalGet(2),
        LocalGet(5),
        I32Cmp(CmpOp::GtS),
        If {
            result: None,
            then: vec![I32Const(1), Return],
            els: vec![],
        },
        LocalGet(2),
        LocalGet(5),
        I32Cmp(CmpOp::LtS),
        If {
            result: None,
            then: vec![I32Const(-1), Return],
            els: vec![],
        },
        Loop { body: loop_body },
        // The loop always returns; this satisfies the type checker.
        I32Const(0),
    ];
    function(
        "__clean_digits_cmp",
        &[Val::I32, Val::I32, Val::I32, Val::I32, Val::I32, Val::I32],
        &[Val::I32],
        &[Val::I32, Val::I32, Val::I32],
        body,
    )
}

// num_to_string locals.
const V: u32 = 0; // f64 param
const BITS: u32 = 1; // i64
const M: u32 = 2; // i64
const MM: u32 = 3; // i64
const E2: u32 = 4;
const BUF: u32 = 5;
const NV: u32 = 6;
const EV: u32 = 7;
const NLO: u32 = 8;
const ELO: u32 = 9;
const NHI: u32 = 10;
const EHI: u32 = 11;
const INCL: u32 = 12;
const K: u32 = 13;
const KC: u32 = 14;
const EC: u32 = 15;
const I: u32 = 16;
const P: u32 = 17;
const D: u32 = 18;
const OK: u32 = 19;
const EX: u32 = 20;
const SIGN: u32 = 21;
const RUP: u32 = 22;
const BEXP: u32 = 23;
const TNZ: u32 = 24;
const CMPR: u32 = 25;

/// Writes one constant byte at the current output pointer and advances it.
fn emit_byte(b: u8) -> Vec<Inst> {
    use Inst::*;
    vec![
        LocalGet(P),
        I32Const(b as i32),
        I32Store8(0),
        LocalGet(P),
        I32Const(1),
        I32Bin(I32Op::Add),
        LocalSet(P),
    ]
}

/// Writes a constant ASCII text at the output pointer.
fn emit_text(text: &str) -> Vec<Inst> {
    text.bytes().flat_map(emit_byte).collect()
}

/// `LiftString(out_start, p - out_start)` and return.
fn emit_finish() -> Vec<Inst> {
    use Inst::*;
    vec![
        LocalGet(BUF),
        I32Const(OUT),
        I32Bin(I32Op::Add),
        LocalGet(P),
        LocalGet(BUF),
        I32Bin(I32Op::Sub),
        I32Const(OUT),
        I32Bin(I32Op::Sub),
        CallRuntime(RuntimeFn::LiftString),
        Return,
    ]
}

/// `P = BUF + OUT`, then '-' when the sign bit is set.
fn emit_out_head() -> Vec<Inst> {
    use Inst::*;
    let mut out = vec![
        LocalGet(BUF),
        I32Const(OUT),
        I32Bin(I32Op::Add),
        LocalSet(P),
    ];
    out.push(LocalGet(SIGN));
    out.push(If {
        result: None,
        then: emit_byte(b'-'),
        els: vec![],
    });
    out
}

/// One expansion: digits and decimal exponent of `MM × 2^EX` into the
/// given areas; the digit count lands in `n_local`, the exponent in
/// `e_local`.
fn expand_into(limbs_off: i32, dig_off: i32, n_local: u32, e_local: u32) -> Vec<Inst> {
    use Inst::*;
    vec![
        // n_limbs = num_expand(MM, EX, buf+limbs)  (kept on the stack)
        LocalGet(BUF),
        I32Const(limbs_off),
        I32Bin(I32Op::Add),
        LocalGet(MM),
        LocalGet(EX),
        LocalGet(BUF),
        I32Const(limbs_off),
        I32Bin(I32Op::Add),
        CallRuntime(RuntimeFn::NumExpand),
        // n_digits = limbs_to_digits(buf+limbs, n_limbs, buf+dig)
        LocalGet(BUF),
        I32Const(dig_off),
        I32Bin(I32Op::Add),
        CallRuntime(RuntimeFn::LimbsToDigits),
        LocalSet(n_local),
        // e = n_digits + min(EX, 0)
        LocalGet(n_local),
        LocalGet(EX),
        I32Const(0),
        I32Cmp(CmpOp::LtS),
        If {
            result: Some(Val::I32),
            then: vec![LocalGet(EX)],
            els: vec![I32Const(0)],
        },
        I32Bin(I32Op::Add),
        LocalSet(e_local),
    ]
}

/// Copies the first K digits of the v expansion into CAND (zero-padded)
/// and sets KC = K, EC = EV.
fn build_floor() -> Vec<Inst> {
    use Inst::*;
    let mut out = vec![I32Const(0), LocalSet(I)];
    out.push(Block {
        body: vec![Loop {
            body: vec![
                LocalGet(I),
                LocalGet(K),
                I32Cmp(CmpOp::Eq),
                BrIf(1),
                // D = I < NV ? digv[I] : 0
                LocalGet(I),
                LocalGet(NV),
                I32Cmp(CmpOp::LtS),
                If {
                    result: Some(Val::I32),
                    then: vec![
                        LocalGet(BUF),
                        LocalGet(I),
                        I32Bin(I32Op::Add),
                        I32Load8U(DIG_V as u32),
                    ],
                    els: vec![I32Const(0)],
                },
                LocalSet(D),
                LocalGet(BUF),
                LocalGet(I),
                I32Bin(I32Op::Add),
                LocalGet(D),
                I32Store8(CAND as u32),
                LocalGet(I),
                I32Const(1),
                I32Bin(I32Op::Add),
                LocalSet(I),
                Br(0),
            ],
        }],
    });
    out.extend([LocalGet(K), LocalSet(KC), LocalGet(EV), LocalSet(EC)]);
    out
}

/// Increments CAND (KC digits) by one in the last place; a carry past the
/// first digit collapses to the single digit 1 with EC + 1.
fn build_increment() -> Vec<Inst> {
    use Inst::*;
    let mut out = vec![LocalGet(KC), I32Const(1), I32Bin(I32Op::Sub), LocalSet(I)];
    out.push(Block {
        body: vec![Loop {
            body: vec![
                // CAND[I] += 1
                LocalGet(BUF),
                LocalGet(I),
                I32Bin(I32Op::Add),
                LocalGet(BUF),
                LocalGet(I),
                I32Bin(I32Op::Add),
                I32Load8U(CAND as u32),
                I32Const(1),
                I32Bin(I32Op::Add),
                I32Store8(CAND as u32),
                // done unless it hit 10
                LocalGet(BUF),
                LocalGet(I),
                I32Bin(I32Op::Add),
                I32Load8U(CAND as u32),
                I32Const(10),
                I32Cmp(CmpOp::LtS),
                BrIf(1),
                // CAND[I] = 0
                LocalGet(BUF),
                LocalGet(I),
                I32Bin(I32Op::Add),
                I32Const(0),
                I32Store8(CAND as u32),
                // carry out of the first digit → "1", EC += 1
                LocalGet(I),
                I32Eqz,
                If {
                    result: None,
                    then: vec![
                        LocalGet(BUF),
                        I32Const(1),
                        I32Store8(CAND as u32),
                        I32Const(1),
                        LocalSet(KC),
                        LocalGet(EC),
                        I32Const(1),
                        I32Bin(I32Op::Add),
                        LocalSet(EC),
                        Br(2),
                    ],
                    els: vec![],
                },
                LocalGet(I),
                I32Const(1),
                I32Bin(I32Op::Sub),
                LocalSet(I),
                Br(0),
            ],
        }],
    });
    out
}

/// Interval test on the candidate: OK = low < CAND < high, bounds
/// inclusive iff INCL.
fn test_candidate() -> Vec<Inst> {
    use Inst::*;
    let cmp_against = |dig_off: i32, n_local: u32, e_local: u32| -> Vec<Inst> {
        vec![
            LocalGet(BUF),
            I32Const(CAND),
            I32Bin(I32Op::Add),
            LocalGet(KC),
            LocalGet(EC),
            LocalGet(BUF),
            I32Const(dig_off),
            I32Bin(I32Op::Add),
            LocalGet(n_local),
            LocalGet(e_local),
            CallRuntime(RuntimeFn::DigitsCmp),
        ]
    };
    let above = |strict_ok: Vec<Inst>| -> Vec<Inst> {
        // (cmp > 0) || (INCL && cmp == 0), with cmp in CMPR
        let mut out = strict_ok;
        out.extend([
            LocalSet(CMPR),
            LocalGet(CMPR),
            I32Const(0),
            I32Cmp(CmpOp::GtS),
            LocalGet(INCL),
            LocalGet(CMPR),
            I32Eqz,
            I32Bin(I32Op::And),
            I32Bin(I32Op::Or),
        ]);
        out
    };
    let below = |strict_ok: Vec<Inst>| -> Vec<Inst> {
        let mut out = strict_ok;
        out.extend([
            LocalSet(CMPR),
            LocalGet(CMPR),
            I32Const(0),
            I32Cmp(CmpOp::LtS),
            LocalGet(INCL),
            LocalGet(CMPR),
            I32Eqz,
            I32Bin(I32Op::And),
            I32Bin(I32Op::Or),
        ]);
        out
    };
    let mut out = above(cmp_against(DIG_LO, NLO, ELO));
    out.extend(below(cmp_against(DIG_HI, NHI, EHI)));
    out.push(Inst::I32Bin(I32Op::And));
    out.push(Inst::LocalSet(OK));
    out
}

/// Writes CAND[from..to] as ASCII at P.
fn emit_cand_range(from: Vec<Inst>, to: Vec<Inst>) -> Vec<Inst> {
    use Inst::*;
    let mut out = from;
    out.push(LocalSet(I));
    let mut cond = to;
    cond.push(LocalSet(D)); // D holds the bound while looping
    out.extend(cond);
    out.push(Block {
        body: vec![Loop {
            body: vec![
                LocalGet(I),
                LocalGet(D),
                I32Cmp(CmpOp::GeS),
                BrIf(1),
                LocalGet(P),
                LocalGet(BUF),
                LocalGet(I),
                I32Bin(I32Op::Add),
                I32Load8U(CAND as u32),
                I32Const(b'0' as i32),
                I32Bin(I32Op::Add),
                I32Store8(0),
                LocalGet(P),
                I32Const(1),
                I32Bin(I32Op::Add),
                LocalSet(P),
                LocalGet(I),
                I32Const(1),
                I32Bin(I32Op::Add),
                LocalSet(I),
                Br(0),
            ],
        }],
    });
    out
}

/// Writes `count` ASCII zeros at P (count from instructions, left in D).
fn emit_zeros(count: Vec<Inst>) -> Vec<Inst> {
    use Inst::*;
    let mut out = count;
    out.push(LocalSet(D));
    out.push(Block {
        body: vec![Loop {
            body: {
                let mut b = vec![LocalGet(D), I32Const(0), I32Cmp(CmpOp::LeS), BrIf(1)];
                b.extend(emit_byte(b'0'));
                b.extend([
                    LocalGet(D),
                    I32Const(1),
                    I32Bin(I32Op::Sub),
                    LocalSet(D),
                    Br(0),
                ]);
                b
            },
        }],
    });
    out
}

/// `num_to_string(v: f64) -> base` — see the module doc.
fn num_to_string() -> MirFunction {
    use Inst::*;
    let mut body = Vec::new();

    // Bits, sign, biased exponent, fraction.
    body.extend([
        LocalGet(V),
        I64ReinterpretF64,
        LocalSet(BITS),
        LocalGet(BITS),
        I64Const(i64::MIN), // 2^63 as unsigned
        I64Bin(I64Op::DivU),
        I32WrapI64,
        LocalSet(SIGN),
        LocalGet(BITS),
        I64Const(1 << 52),
        I64Bin(I64Op::DivU),
        I64Const(2048),
        I64Bin(I64Op::RemU),
        I32WrapI64,
        LocalSet(BEXP),
        LocalGet(BITS),
        I64Const(1 << 52),
        I64Bin(I64Op::RemU),
        LocalSet(M),
    ]);

    // Scratch arena for the whole computation.
    body.extend([
        I32Const(BUF_SIZE),
        I32Const(8),
        CallRuntime(RuntimeFn::Alloc),
        LocalSet(BUF),
    ]);

    // Specials: NaN / ±Infinity (local spellings — DISCOVERIES-M8).
    {
        let mut inf = emit_out_head();
        inf.extend(emit_text("Infinity"));
        inf.extend(emit_finish());
        let mut nan = vec![
            LocalGet(BUF),
            I32Const(OUT),
            I32Bin(I32Op::Add),
            LocalSet(P),
        ];
        nan.extend(emit_text("NaN"));
        nan.extend(emit_finish());
        body.extend([
            LocalGet(BEXP),
            I32Const(2047),
            I32Cmp(CmpOp::Eq),
            If {
                result: None,
                then: vec![
                    LocalGet(M),
                    I64Const(0),
                    I64Cmp(CmpOp::Eq),
                    If {
                        result: None,
                        then: inf,
                        els: nan,
                    },
                ],
                els: vec![],
            },
        ]);
    }

    // Zero (sign preserved: "-0.0" round-trips to the negative zero).
    {
        let mut zero = emit_out_head();
        zero.extend(emit_text("0.0"));
        zero.extend(emit_finish());
        body.extend([
            LocalGet(BEXP),
            I32Eqz,
            LocalGet(M),
            I64Const(0),
            I64Cmp(CmpOp::Eq),
            I32Bin(I32Op::And),
            If {
                result: None,
                then: zero,
                els: vec![],
            },
        ]);
    }

    // m, e2 (v = m × 2^e2 exactly).
    body.extend([
        LocalGet(BEXP),
        I32Const(0),
        I32Cmp(CmpOp::GtS),
        If {
            result: None,
            then: vec![
                LocalGet(M),
                I64Const(1 << 52),
                I64Bin(I64Op::Add),
                LocalSet(M),
                LocalGet(BEXP),
                I32Const(1075),
                I32Bin(I32Op::Sub),
                LocalSet(E2),
            ],
            els: vec![I32Const(-1074), LocalSet(E2)],
        },
    ]);

    // Expansion of v.
    body.extend([LocalGet(M), LocalSet(MM), LocalGet(E2), LocalSet(EX)]);
    body.extend(expand_into(LIMBS_V, DIG_V, NV, EV));

    // Low midpoint: (2m-1)×2^(e2-1), except at the power-of-two boundary
    // (fraction zero, not the lowest normal), where the lower gap halves:
    // (4m-1)×2^(e2-2).
    body.extend([
        LocalGet(M),
        I64Const(1 << 52),
        I64Cmp(CmpOp::Eq),
        LocalGet(BEXP),
        I32Const(1),
        I32Cmp(CmpOp::GtS),
        I32Bin(I32Op::And),
        If {
            result: None,
            then: vec![
                LocalGet(M),
                I64Const(4),
                I64Bin(I64Op::Mul),
                I64Const(1),
                I64Bin(I64Op::Sub),
                LocalSet(MM),
                LocalGet(E2),
                I32Const(2),
                I32Bin(I32Op::Sub),
                LocalSet(EX),
            ],
            els: vec![
                LocalGet(M),
                I64Const(2),
                I64Bin(I64Op::Mul),
                I64Const(1),
                I64Bin(I64Op::Sub),
                LocalSet(MM),
                LocalGet(E2),
                I32Const(1),
                I32Bin(I32Op::Sub),
                LocalSet(EX),
            ],
        },
    ]);
    body.extend(expand_into(LIMBS_LO, DIG_LO, NLO, ELO));

    // High midpoint: (2m+1)×2^(e2-1).
    body.extend([
        LocalGet(M),
        I64Const(2),
        I64Bin(I64Op::Mul),
        I64Const(1),
        I64Bin(I64Op::Add),
        LocalSet(MM),
        LocalGet(E2),
        I32Const(1),
        I32Bin(I32Op::Sub),
        LocalSet(EX),
    ]);
    body.extend(expand_into(LIMBS_HI, DIG_HI, NHI, EHI));

    // Bounds inclusive iff the mantissa is even (round-to-nearest-even
    // reads midpoints back as v exactly then).
    body.extend([
        LocalGet(M),
        I64Const(2),
        I64Bin(I64Op::RemU),
        I64Const(0),
        I64Cmp(CmpOp::Eq),
        LocalSet(INCL),
    ]);

    // The shortest-digits loop.
    let mut k_loop = Vec::new();
    {
        let out = &mut k_loop;
        // Rounded candidate: floor, then increment when the dropped tail
        // rounds up (half-even).
        out.extend(build_floor());
        // D = first dropped digit; TNZ = any nonzero beyond it.
        out.extend([
            LocalGet(K),
            LocalGet(NV),
            I32Cmp(CmpOp::LtS),
            If {
                result: Some(Val::I32),
                then: vec![
                    LocalGet(BUF),
                    LocalGet(K),
                    I32Bin(I32Op::Add),
                    I32Load8U(DIG_V as u32),
                ],
                els: vec![I32Const(0)],
            },
            LocalSet(D),
            I32Const(0),
            LocalSet(TNZ),
            LocalGet(K),
            I32Const(1),
            I32Bin(I32Op::Add),
            LocalSet(I),
        ]);
        out.push(Block {
            body: vec![Loop {
                body: vec![
                    LocalGet(I),
                    LocalGet(NV),
                    I32Cmp(CmpOp::GeS),
                    BrIf(1),
                    LocalGet(BUF),
                    LocalGet(I),
                    I32Bin(I32Op::Add),
                    I32Load8U(DIG_V as u32),
                    I32Const(0),
                    I32Cmp(CmpOp::Ne),
                    If {
                        result: None,
                        then: vec![I32Const(1), LocalSet(TNZ), Br(2)],
                        els: vec![],
                    },
                    LocalGet(I),
                    I32Const(1),
                    I32Bin(I32Op::Add),
                    LocalSet(I),
                    Br(0),
                ],
            }],
        });
        // RUP = D > 5 || (D == 5 && (TNZ || last kept digit odd))
        out.extend([
            LocalGet(D),
            I32Const(5),
            I32Cmp(CmpOp::GtS),
            LocalGet(D),
            I32Const(5),
            I32Cmp(CmpOp::Eq),
            LocalGet(TNZ),
            LocalGet(BUF),
            LocalGet(K),
            I32Const(1),
            I32Bin(I32Op::Sub),
            I32Bin(I32Op::Add),
            I32Load8U(CAND as u32),
            I32Const(1),
            I32Bin(I32Op::And),
            I32Const(0),
            I32Cmp(CmpOp::Ne),
            I32Bin(I32Op::Or),
            I32Bin(I32Op::And),
            I32Bin(I32Op::Or),
            LocalSet(RUP),
        ]);
        out.push(LocalGet(RUP));
        out.push(If {
            result: None,
            then: build_increment(),
            els: vec![],
        });
        out.extend(test_candidate());
        out.extend([LocalGet(OK), BrIf(1)]);
        // The other k-digit neighbor: floor when the rounded one was the
        // ceil, ceil otherwise.
        out.extend(build_floor());
        out.push(LocalGet(RUP));
        out.push(If {
            result: None,
            then: vec![],
            els: build_increment(),
        });
        out.extend(test_candidate());
        out.extend([LocalGet(OK), BrIf(1)]);
        // Next k; 17 digits always suffice — past that is a compiler bug.
        out.extend([
            LocalGet(K),
            I32Const(1),
            I32Bin(I32Op::Add),
            LocalSet(K),
            LocalGet(K),
            I32Const(17),
            I32Cmp(CmpOp::GtS),
            If {
                result: None,
                then: vec![Unreachable],
                els: vec![],
            },
            Br(0),
        ]);
    }
    body.extend([I32Const(1), LocalSet(K)]);
    body.push(Block {
        body: vec![Loop { body: k_loop }],
    });

    // Render: plain decimal for -4 ≤ EC ≤ 21, scientific otherwise.
    body.extend(emit_out_head());
    {
        // Plain, integral (EC ≥ KC): digits, zeros, ".0".
        let mut integral = emit_cand_range(vec![I32Const(0)], vec![LocalGet(KC)]);
        integral.extend(emit_zeros(vec![
            LocalGet(EC),
            LocalGet(KC),
            I32Bin(I32Op::Sub),
        ]));
        integral.extend(emit_text(".0"));

        // Plain, split (1 ≤ EC < KC): d[0..EC] "." d[EC..KC].
        let mut split = emit_cand_range(vec![I32Const(0)], vec![LocalGet(EC)]);
        split.extend(emit_byte(b'.'));
        split.extend(emit_cand_range(vec![LocalGet(EC)], vec![LocalGet(KC)]));

        // Plain, small (EC ≤ 0): "0." zeros digits.
        let mut small = emit_text("0.");
        small.extend(emit_zeros(vec![
            I32Const(0),
            LocalGet(EC),
            I32Bin(I32Op::Sub),
        ]));
        small.extend(emit_cand_range(vec![I32Const(0)], vec![LocalGet(KC)]));

        let plain = vec![
            LocalGet(EC),
            LocalGet(KC),
            I32Cmp(CmpOp::GeS),
            If {
                result: None,
                then: integral,
                els: vec![
                    LocalGet(EC),
                    I32Const(1),
                    I32Cmp(CmpOp::GeS),
                    If {
                        result: None,
                        then: split,
                        els: small,
                    },
                ],
            },
        ];

        // Scientific: d1 ["." d2..] "e" (EC-1).
        let mut sci = emit_cand_range(vec![I32Const(0)], vec![I32Const(1)]);
        sci.extend([
            LocalGet(KC),
            I32Const(1),
            I32Cmp(CmpOp::GtS),
            If {
                result: None,
                then: {
                    let mut t = emit_byte(b'.');
                    t.extend(emit_cand_range(vec![I32Const(1)], vec![LocalGet(KC)]));
                    t
                },
                els: vec![],
            },
        ]);
        sci.extend(emit_byte(b'e'));
        // EX = EC - 1; sign, then up to three decimal digits.
        sci.extend([
            LocalGet(EC),
            I32Const(1),
            I32Bin(I32Op::Sub),
            LocalSet(EX),
            LocalGet(EX),
            I32Const(0),
            I32Cmp(CmpOp::LtS),
            If {
                result: None,
                then: {
                    let mut t = emit_byte(b'-');
                    t.extend([I32Const(0), LocalGet(EX), I32Bin(I32Op::Sub), LocalSet(EX)]);
                    t
                },
                els: vec![],
            },
        ]);
        // Hundreds / tens / units, skipping leading zeros.
        // d = EX/div % 10, computed as EX/div - (EX/(div*10))*10 —
        // I32Op carries no remainder.
        let digit_at = |div: i32| -> Vec<Inst> {
            let mut t = Vec::new();
            t.extend([
                LocalGet(P),
                LocalGet(EX),
                I32Const(div),
                I32Bin(I32Op::DivU),
                LocalGet(EX),
                I32Const(div * 10),
                I32Bin(I32Op::DivU),
                I32Const(10),
                I32Bin(I32Op::Mul),
                I32Bin(I32Op::Sub),
                I32Const(b'0' as i32),
                I32Bin(I32Op::Add),
                I32Store8(0),
                LocalGet(P),
                I32Const(1),
                I32Bin(I32Op::Add),
                LocalSet(P),
            ]);
            t
        };
        sci.extend([
            LocalGet(EX),
            I32Const(100),
            I32Cmp(CmpOp::GeS),
            If {
                result: None,
                then: digit_at(100),
                els: vec![],
            },
            LocalGet(EX),
            I32Const(10),
            I32Cmp(CmpOp::GeS),
            If {
                result: None,
                then: digit_at(10),
                els: vec![],
            },
        ]);
        sci.extend(digit_at(1));

        // 15 §Conversions (2026-08-20): plain exactly while −4 ≤ E ≤ 21
        // with E from the NORMALIZED form d₁.d₂…×10^E. EC here uses the
        // 0.d₁d₂… convention (EC = E + 1), so the range is −3 ≤ EC ≤ 22.
        body.extend([
            LocalGet(EC),
            I32Const(-3),
            I32Cmp(CmpOp::GeS),
            LocalGet(EC),
            I32Const(22),
            I32Cmp(CmpOp::LeS),
            I32Bin(I32Op::And),
            If {
                result: None,
                then: plain,
                els: sci,
            },
        ]);
    }
    body.extend(emit_finish());
    // emit_finish returns; this satisfies the function type.
    body.push(I32Const(0));

    function(
        "__clean_num_to_string",
        &[Val::F64],
        &[Val::I32],
        &[
            Val::I64, // BITS
            Val::I64, // M
            Val::I64, // MM
            Val::I32, // E2
            Val::I32, // BUF
            Val::I32, // NV
            Val::I32, // EV
            Val::I32, // NLO
            Val::I32, // ELO
            Val::I32, // NHI
            Val::I32, // EHI
            Val::I32, // INCL
            Val::I32, // K
            Val::I32, // KC
            Val::I32, // EC
            Val::I32, // I
            Val::I32, // P
            Val::I32, // D
            Val::I32, // OK
            Val::I32, // EX
            Val::I32, // SIGN
            Val::I32, // RUP
            Val::I32, // BEXP
            Val::I32, // TNZ
            Val::I32, // CMPR
        ],
        body,
    )
}
