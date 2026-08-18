//! Chapter-15 list methods: the [`RuntimeFn::ListSlice`]..=`ListRemoveAt`
//! bodies, built as ordinary [`MirFunction`]s in discriminant order (the
//! contracts live on the enum in `runtime.rs`). Every helper works on the
//! MMD §3.4.1 object — `{len, cap, tag, pad, elements @16}` — and takes
//! the element byte stride as a parameter where it is not fixed by the
//! element type. All helpers return fresh objects except `list_remove_at`,
//! which mutates in place (no relocation — safe under aliasing).

use crate::layout::{
    ALIGNMENT, EMPTY_STRING_ADDR, LIST_CAP_OFFSET, LIST_ELEMS_OFFSET, LIST_LEN_OFFSET,
    LIST_TAG_OFFSET,
};

use super::runtime::RuntimeFn;
use super::{CmpOp, I32Op, I64Op, Inst, MirFunction, Val};

/// The list helpers, in [`RuntimeFn`] discriminant order
/// (`ListSlice`..=`ListRemoveAt`); `runtime::build` appends them after the
/// string helpers.
pub fn build() -> Vec<MirFunction> {
    vec![
        list_slice(),
        list_reverse(),
        list_concat(),
        list_range(),
        list_fill("__clean_list_fill_64", Val::I64, 8),
        list_fill("__clean_list_fill_f64", Val::F64, 8),
        list_fill("__clean_list_fill_32", Val::I32, 4),
        list_join(),
        list_sort("__clean_list_sort_i64", Val::I64, 8),
        list_sort("__clean_list_sort_f64", Val::F64, 8),
        list_sort("__clean_list_sort_str", Val::I32, 4),
        list_index_of("__clean_list_index_of_i64", IndexEq::I64),
        list_index_of("__clean_list_index_of_f64", IndexEq::F64),
        list_index_of("__clean_list_index_of_str", IndexEq::Str),
        list_index_of("__clean_list_index_of_32", IndexEq::I32),
        list_remove_at(),
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

/// Emits `r = alloc(16 + n*stride, 8)` and the §3.4.1 header — `len =
/// cap =` local `n`, `tag =` local `tag`, pad 0 — leaving the fresh base
/// in local `r`. `stride` is instructions pushing the element byte stride
/// (an i32 local or constant).
fn fresh_list(n: u32, stride: Vec<Inst>, tag: u32, r: u32) -> Vec<Inst> {
    use Inst::*;
    let mut out = vec![I32Const(LIST_ELEMS_OFFSET as i32), LocalGet(n)];
    out.extend(stride);
    out.extend([
        I32Bin(I32Op::Mul),
        I32Bin(I32Op::Add),
        I32Const(ALIGNMENT as i32),
        CallRuntime(RuntimeFn::Alloc),
        LocalTee(r),
        LocalGet(n),
        I32Store(LIST_LEN_OFFSET),
        LocalGet(r),
        LocalGet(n),
        I32Store(LIST_CAP_OFFSET),
        LocalGet(r),
        LocalGet(tag),
        I32Store(LIST_TAG_OFFSET),
        LocalGet(r),
        I32Const(0),
        I32Store(LIST_TAG_OFFSET + 4),
    ]);
    out
}

/// Pushes `list + i*stride` — the address of element `i` minus the header;
/// the following load/store uses `LIST_ELEMS_OFFSET` as its constant
/// offset. Both `list` and `i` are i32 locals, `stride` a constant.
fn elem_addr(list: u32, i: u32, stride: u32) -> Vec<Inst> {
    use Inst::*;
    vec![
        LocalGet(list),
        LocalGet(i),
        I32Const(stride as i32),
        I32Bin(I32Op::Mul),
        I32Bin(I32Op::Add),
    ]
}

/// `list_slice(src, start, end, stride, tag) -> base` — start and end
/// clamped to `[0, len]` (the substring clamp adoption); `end <= start`
/// yields a fresh empty list (a real header object, tag set).
/// Params: src@0, start@1 (i64), end@2 (i64), stride@3, tag@4.
/// Locals: lenl@5 (i64), n@6, r@7.
fn list_slice() -> MirFunction {
    use Inst::*;
    // Clamps the i64 local `x` into `[0, lenl]`.
    let clamp = |x: u32| -> Vec<Inst> {
        vec![
            LocalGet(x),
            I64Const(0),
            I64Cmp(CmpOp::LtS),
            If {
                result: None,
                then: vec![I64Const(0), LocalSet(x)],
                els: vec![],
            },
            LocalGet(x),
            LocalGet(5),
            I64Cmp(CmpOp::GtS),
            If {
                result: None,
                then: vec![LocalGet(5), LocalSet(x)],
                els: vec![],
            },
        ]
    };
    let mut body = vec![
        LocalGet(0),
        I32Load(LIST_LEN_OFFSET),
        I64ExtendI32U,
        LocalSet(5),
    ];
    body.extend(clamp(1));
    body.extend(clamp(2));
    body.extend([
        // n = end <= start ? 0 : end - start (fits i32: program-sized).
        LocalGet(2),
        LocalGet(1),
        I64Cmp(CmpOp::LeS),
        If {
            result: Some(Val::I32),
            then: vec![I32Const(0)],
            els: vec![LocalGet(2), LocalGet(1), I64Bin(I64Op::Sub), I32WrapI64],
        },
        LocalSet(6),
    ]);
    body.extend(fresh_list(6, vec![LocalGet(3)], 4, 7));
    body.extend([
        // One copy of n*stride bytes (n = 0 is a legal no-op copy).
        LocalGet(7),
        I32Const(LIST_ELEMS_OFFSET as i32),
        I32Bin(I32Op::Add),
        LocalGet(0),
        I32Const(LIST_ELEMS_OFFSET as i32),
        I32Bin(I32Op::Add),
        LocalGet(1),
        I32WrapI64,
        LocalGet(3),
        I32Bin(I32Op::Mul),
        I32Bin(I32Op::Add),
        LocalGet(6),
        LocalGet(3),
        I32Bin(I32Op::Mul),
        MemoryCopy,
        LocalGet(7),
    ]);
    function(
        "__clean_list_slice",
        &[Val::I32, Val::I64, Val::I64, Val::I32, Val::I32],
        &[Val::I32],
        &[Val::I64, Val::I32, Val::I32],
        body,
    )
}

/// `list_reverse(src, stride, tag) -> base` — fresh list with the elements
/// in reverse order. Params: src@0, stride@1, tag@2. Locals: n@3, r@4,
/// i@5.
fn list_reverse() -> MirFunction {
    use Inst::*;
    let mut body = vec![LocalGet(0), I32Load(LIST_LEN_OFFSET), LocalSet(3)];
    body.extend(fresh_list(3, vec![LocalGet(1)], 2, 4));
    body.extend([
        I32Const(0),
        LocalSet(5),
        Block {
            body: vec![Loop {
                body: vec![
                    LocalGet(5),
                    LocalGet(3),
                    I32Cmp(CmpOp::Eq),
                    BrIf(1),
                    // dst = r + 16 + (n-1-i)*stride
                    LocalGet(4),
                    I32Const(LIST_ELEMS_OFFSET as i32),
                    I32Bin(I32Op::Add),
                    LocalGet(3),
                    I32Const(1),
                    I32Bin(I32Op::Sub),
                    LocalGet(5),
                    I32Bin(I32Op::Sub),
                    LocalGet(1),
                    I32Bin(I32Op::Mul),
                    I32Bin(I32Op::Add),
                    // src = src + 16 + i*stride
                    LocalGet(0),
                    I32Const(LIST_ELEMS_OFFSET as i32),
                    I32Bin(I32Op::Add),
                    LocalGet(5),
                    LocalGet(1),
                    I32Bin(I32Op::Mul),
                    I32Bin(I32Op::Add),
                    LocalGet(1),
                    MemoryCopy,
                    LocalGet(5),
                    I32Const(1),
                    I32Bin(I32Op::Add),
                    LocalSet(5),
                    Br(0),
                ],
            }],
        },
        LocalGet(4),
    ]);
    function(
        "__clean_list_reverse",
        &[Val::I32, Val::I32, Val::I32],
        &[Val::I32],
        &[Val::I32; 3],
        body,
    )
}

/// `list_concat(a, b, stride, tag) -> base` — fresh list of `len(a) +
/// len(b)`, two copies. Params: a@0, b@1, stride@2, tag@3. Locals: la@4,
/// lb@5, n@6, r@7.
fn list_concat() -> MirFunction {
    use Inst::*;
    let mut body = vec![
        LocalGet(0),
        I32Load(LIST_LEN_OFFSET),
        LocalSet(4),
        LocalGet(1),
        I32Load(LIST_LEN_OFFSET),
        LocalSet(5),
        LocalGet(4),
        LocalGet(5),
        I32Bin(I32Op::Add),
        LocalSet(6),
    ];
    body.extend(fresh_list(6, vec![LocalGet(2)], 3, 7));
    body.extend([
        // a's elements to r+16.
        LocalGet(7),
        I32Const(LIST_ELEMS_OFFSET as i32),
        I32Bin(I32Op::Add),
        LocalGet(0),
        I32Const(LIST_ELEMS_OFFSET as i32),
        I32Bin(I32Op::Add),
        LocalGet(4),
        LocalGet(2),
        I32Bin(I32Op::Mul),
        MemoryCopy,
        // b's elements to r+16+la*stride.
        LocalGet(7),
        I32Const(LIST_ELEMS_OFFSET as i32),
        I32Bin(I32Op::Add),
        LocalGet(4),
        LocalGet(2),
        I32Bin(I32Op::Mul),
        I32Bin(I32Op::Add),
        LocalGet(1),
        I32Const(LIST_ELEMS_OFFSET as i32),
        I32Bin(I32Op::Add),
        LocalGet(5),
        LocalGet(2),
        I32Bin(I32Op::Mul),
        MemoryCopy,
        LocalGet(7),
    ]);
    function(
        "__clean_list_concat",
        &[Val::I32, Val::I32, Val::I32, Val::I32],
        &[Val::I32],
        &[Val::I32; 4],
        body,
    )
}

/// `list_range(start, end, tag) -> base` — a `list<integer>` (stride 8),
/// both ends inclusive; ascending step +1 when `start <= end`, else
/// descending step -1 (15 §List). `n = |end - start| + 1`, always >= 1.
/// Params: start@0 (i64), end@1 (i64), tag@2. Locals: n@3, r@4, i@5,
/// v@6 (i64), step@7 (i64).
fn list_range() -> MirFunction {
    use Inst::*;
    let mut body = vec![
        LocalGet(0),
        LocalGet(1),
        I64Cmp(CmpOp::LeS),
        If {
            result: Some(Val::I64),
            then: vec![I64Const(1)],
            els: vec![I64Const(-1)],
        },
        LocalSet(7),
        LocalGet(0),
        LocalGet(1),
        I64Cmp(CmpOp::LeS),
        If {
            result: Some(Val::I64),
            then: vec![LocalGet(1), LocalGet(0), I64Bin(I64Op::Sub)],
            els: vec![LocalGet(0), LocalGet(1), I64Bin(I64Op::Sub)],
        },
        I32WrapI64,
        I32Const(1),
        I32Bin(I32Op::Add),
        LocalSet(3),
    ];
    body.extend(fresh_list(3, vec![I32Const(8)], 2, 4));
    body.extend([
        LocalGet(0),
        LocalSet(6),
        I32Const(0),
        LocalSet(5),
        Block {
            body: vec![Loop {
                body: vec![
                    LocalGet(5),
                    LocalGet(3),
                    I32Cmp(CmpOp::Eq),
                    BrIf(1),
                    LocalGet(4),
                    LocalGet(5),
                    I32Const(8),
                    I32Bin(I32Op::Mul),
                    I32Bin(I32Op::Add),
                    LocalGet(6),
                    I64Store(LIST_ELEMS_OFFSET),
                    LocalGet(6),
                    LocalGet(7),
                    I64Bin(I64Op::Add),
                    LocalSet(6),
                    LocalGet(5),
                    I32Const(1),
                    I32Bin(I32Op::Add),
                    LocalSet(5),
                    Br(0),
                ],
            }],
        },
        LocalGet(4),
    ]);
    function(
        "__clean_list_range",
        &[Val::I64, Val::I64, Val::I32],
        &[Val::I32],
        &[Val::I32, Val::I32, Val::I32, Val::I64, Val::I64],
        body,
    )
}

/// `list_fill_64` / `list_fill_f64` / `list_fill_32` — a fresh list of
/// `size` elements, every element `value`; a negative size traps, size 0
/// yields an empty list object. `elem` picks the value type and the store,
/// `stride` the element width. Params: size@0 (i64), value@1 (`elem`),
/// tag@2. Locals: n@3, r@4, i@5.
fn list_fill(name: &str, elem: Val, stride: u32) -> MirFunction {
    use Inst::*;
    let store = match elem {
        Val::I64 => I64Store(LIST_ELEMS_OFFSET),
        Val::F64 => F64Store(LIST_ELEMS_OFFSET),
        Val::I32 => I32Store(LIST_ELEMS_OFFSET),
    };
    let mut body = vec![
        LocalGet(0),
        I64Const(0),
        I64Cmp(CmpOp::LtS),
        If {
            result: None,
            then: vec![Unreachable],
            els: vec![],
        },
        LocalGet(0),
        I32WrapI64,
        LocalSet(3),
    ];
    body.extend(fresh_list(3, vec![I32Const(stride as i32)], 2, 4));
    let mut step = vec![LocalGet(5), LocalGet(3), I32Cmp(CmpOp::Eq), BrIf(1)];
    step.extend(elem_addr(4, 5, stride));
    step.extend([
        LocalGet(1),
        store,
        LocalGet(5),
        I32Const(1),
        I32Bin(I32Op::Add),
        LocalSet(5),
        Br(0),
    ]);
    body.extend([
        I32Const(0),
        LocalSet(5),
        Block {
            body: vec![Loop { body: step }],
        },
        LocalGet(4),
    ]);
    function(
        name,
        &[Val::I64, elem, Val::I32],
        &[Val::I32],
        &[Val::I32; 3],
        body,
    )
}

/// `list_join(items, sep) -> string base` — the element strings of a
/// `list<string>` (stride 4) concatenated with `sep` between them; an
/// empty items list or an all-empty result is the shared `""` (MMD-01).
/// Two passes: measure, then copy. Params: items@0, sep@1. Locals: n@2,
/// i@3, total@4, r@5, w@6, e@7, ls@8, el@9.
fn list_join() -> MirFunction {
    use Inst::*;
    // Pass 1: total = sum of element byte lengths.
    let mut measure = vec![LocalGet(3), LocalGet(2), I32Cmp(CmpOp::Eq), BrIf(1)];
    measure.extend(elem_addr(0, 3, 4));
    measure.extend([
        I32Load(LIST_ELEMS_OFFSET),
        LocalSet(7),
        LocalGet(4),
        LocalGet(7),
        I32Load(0),
        I32Bin(I32Op::Add),
        LocalSet(4),
        LocalGet(3),
        I32Const(1),
        I32Bin(I32Op::Add),
        LocalSet(3),
        Br(0),
    ]);
    // Pass 2: separators (before every element but the first), payloads.
    let mut fill = vec![
        LocalGet(3),
        LocalGet(2),
        I32Cmp(CmpOp::Eq),
        BrIf(1),
        LocalGet(3),
        If {
            result: None,
            then: vec![
                LocalGet(6),
                LocalGet(1),
                I32Const(4),
                I32Bin(I32Op::Add),
                LocalGet(8),
                MemoryCopy,
                LocalGet(6),
                LocalGet(8),
                I32Bin(I32Op::Add),
                LocalSet(6),
            ],
            els: vec![],
        },
    ];
    fill.extend(elem_addr(0, 3, 4));
    fill.extend([
        I32Load(LIST_ELEMS_OFFSET),
        LocalSet(7),
        LocalGet(7),
        I32Load(0),
        LocalSet(9),
        LocalGet(6),
        LocalGet(7),
        I32Const(4),
        I32Bin(I32Op::Add),
        LocalGet(9),
        MemoryCopy,
        LocalGet(6),
        LocalGet(9),
        I32Bin(I32Op::Add),
        LocalSet(6),
        LocalGet(3),
        I32Const(1),
        I32Bin(I32Op::Add),
        LocalSet(3),
        Br(0),
    ]);
    let body = vec![
        LocalGet(0),
        I32Load(LIST_LEN_OFFSET),
        LocalSet(2),
        LocalGet(2),
        I32Eqz,
        If {
            result: None,
            then: vec![I32Const(EMPTY_STRING_ADDR as i32), Return],
            els: vec![],
        },
        LocalGet(1),
        I32Load(0),
        LocalSet(8),
        I32Const(0),
        LocalSet(3),
        Block {
            body: vec![Loop { body: measure }],
        },
        // total += (n-1) * len(sep).
        LocalGet(4),
        LocalGet(2),
        I32Const(1),
        I32Bin(I32Op::Sub),
        LocalGet(8),
        I32Bin(I32Op::Mul),
        I32Bin(I32Op::Add),
        LocalSet(4),
        LocalGet(4),
        I32Eqz,
        If {
            result: None,
            then: vec![I32Const(EMPTY_STRING_ADDR as i32), Return],
            els: vec![],
        },
        // r = alloc(4 + total, 8); store the byte length; w = r + 4.
        LocalGet(4),
        I32Const(4),
        I32Bin(I32Op::Add),
        I32Const(ALIGNMENT as i32),
        CallRuntime(RuntimeFn::Alloc),
        LocalTee(5),
        LocalGet(4),
        I32Store(0),
        LocalGet(5),
        I32Const(4),
        I32Bin(I32Op::Add),
        LocalSet(6),
        I32Const(0),
        LocalSet(3),
        Block {
            body: vec![Loop { body: fill }],
        },
        LocalGet(5),
    ];
    function(
        "__clean_list_join",
        &[Val::I32, Val::I32],
        &[Val::I32],
        &[Val::I32; 8],
        body,
    )
}

/// `list_sort_i64` / `list_sort_f64` / `list_sort_str` — copies `src`
/// into a fresh list, then insertion-sorts the copy in place, ascending:
/// signed i64, `f64.lt`, or `string_compare > 0` order (the one comparison
/// convention — KNOWLEDGE §2). `elem` `I32` means string pointers.
/// Params: src@0, tag@1. Locals: n@2, r@3, i@4, j@5, pa@6, pb@7,
/// tmp@8 (`elem`).
fn list_sort(name: &str, elem: Val, stride: u32) -> MirFunction {
    use Inst::*;
    // Pushes 1 iff elem[j] belongs before elem[j-1] (swap needed); pa/pb
    // hold the two element addresses minus the header offset.
    let must_swap = match elem {
        Val::I64 => vec![
            LocalGet(7),
            I64Load(LIST_ELEMS_OFFSET),
            LocalGet(6),
            I64Load(LIST_ELEMS_OFFSET),
            I64Cmp(CmpOp::LtS),
        ],
        Val::F64 => vec![
            LocalGet(7),
            F64Load(LIST_ELEMS_OFFSET),
            LocalGet(6),
            F64Load(LIST_ELEMS_OFFSET),
            F64Cmp(CmpOp::LtS),
        ],
        Val::I32 => vec![
            LocalGet(6),
            I32Load(LIST_ELEMS_OFFSET),
            LocalGet(7),
            I32Load(LIST_ELEMS_OFFSET),
            CallRuntime(RuntimeFn::StringCompare),
            I32Const(0),
            I32Cmp(CmpOp::GtS),
        ],
    };
    // swap(*pa, *pb) through the scratch local.
    let swap = |load: fn(u32) -> Inst, store: fn(u32) -> Inst| -> Vec<Inst> {
        vec![
            LocalGet(6),
            load(LIST_ELEMS_OFFSET),
            LocalSet(8),
            LocalGet(6),
            LocalGet(7),
            load(LIST_ELEMS_OFFSET),
            store(LIST_ELEMS_OFFSET),
            LocalGet(7),
            LocalGet(8),
            store(LIST_ELEMS_OFFSET),
        ]
    };
    let swap = match elem {
        Val::I64 => swap(I64Load, I64Store),
        Val::F64 => swap(F64Load, F64Store),
        Val::I32 => swap(I32Load, I32Store),
    };
    let mut inner = vec![
        LocalGet(5),
        I32Eqz,
        BrIf(1),
        // pa = r + (j-1)*stride; pb = pa + stride.
        LocalGet(3),
        LocalGet(5),
        I32Const(1),
        I32Bin(I32Op::Sub),
        I32Const(stride as i32),
        I32Bin(I32Op::Mul),
        I32Bin(I32Op::Add),
        LocalTee(6),
        I32Const(stride as i32),
        I32Bin(I32Op::Add),
        LocalSet(7),
    ];
    inner.extend(must_swap);
    inner.push(I32Eqz);
    inner.push(BrIf(1));
    inner.extend(swap);
    inner.extend([
        LocalGet(5),
        I32Const(1),
        I32Bin(I32Op::Sub),
        LocalSet(5),
        Br(0),
    ]);
    let outer = vec![
        LocalGet(4),
        LocalGet(2),
        I32Cmp(CmpOp::GeS),
        BrIf(1),
        LocalGet(4),
        LocalSet(5),
        Block {
            body: vec![Loop { body: inner }],
        },
        LocalGet(4),
        I32Const(1),
        I32Bin(I32Op::Add),
        LocalSet(4),
        Br(0),
    ];
    let mut body = vec![LocalGet(0), I32Load(LIST_LEN_OFFSET), LocalSet(2)];
    body.extend(fresh_list(2, vec![I32Const(stride as i32)], 1, 3));
    body.extend([
        LocalGet(3),
        I32Const(LIST_ELEMS_OFFSET as i32),
        I32Bin(I32Op::Add),
        LocalGet(0),
        I32Const(LIST_ELEMS_OFFSET as i32),
        I32Bin(I32Op::Add),
        LocalGet(2),
        I32Const(stride as i32),
        I32Bin(I32Op::Mul),
        MemoryCopy,
        I32Const(1),
        LocalSet(4),
        Block {
            body: vec![Loop { body: outer }],
        },
        LocalGet(3),
    ]);
    let mut locals = vec![Val::I32; 6];
    locals.push(elem);
    function(name, &[Val::I32, Val::I32], &[Val::I32], &locals, body)
}

/// Which equality the `index_of` family tests with (the element type and
/// stride follow from it).
enum IndexEq {
    /// `i64.eq` over stride-8 elements.
    I64,
    /// `f64.eq` over stride-8 elements.
    F64,
    /// `string_eq` over stride-4 pointer elements.
    Str,
    /// `i32.eq` over stride-4 scalar elements.
    I32,
}

/// `list_index_of_*` — the index of the first (backward = 0) or last
/// (backward = 1) element equal to `v`, else -1; one forward and one
/// backward loop selected by an `If`. Params: list@0, v@1, backward@2.
/// Locals: n@3, i@4.
fn list_index_of(name: &str, eq: IndexEq) -> MirFunction {
    use Inst::*;
    let (val, stride) = match eq {
        IndexEq::I64 => (Val::I64, 8u32),
        IndexEq::F64 => (Val::F64, 8),
        IndexEq::Str | IndexEq::I32 => (Val::I32, 4),
    };
    // Pushes 1 iff elem[i] == v.
    let elem_eq = || -> Vec<Inst> {
        let mut out = elem_addr(0, 4, stride);
        out.extend(match eq {
            IndexEq::I64 => vec![I64Load(LIST_ELEMS_OFFSET), LocalGet(1), I64Cmp(CmpOp::Eq)],
            IndexEq::F64 => vec![F64Load(LIST_ELEMS_OFFSET), LocalGet(1), F64Cmp(CmpOp::Eq)],
            IndexEq::Str => vec![
                I32Load(LIST_ELEMS_OFFSET),
                LocalGet(1),
                CallRuntime(RuntimeFn::StringEq),
            ],
            IndexEq::I32 => vec![I32Load(LIST_ELEMS_OFFSET), LocalGet(1), I32Cmp(CmpOp::Eq)],
        });
        out
    };
    let found = || If {
        result: None,
        then: vec![LocalGet(4), I64ExtendI32U, Return],
        els: vec![],
    };
    // Backward: i = n-1 down to 0 (guarded by n != 0).
    let mut back_step = elem_eq();
    back_step.push(found());
    back_step.extend([
        LocalGet(4),
        I32Eqz,
        BrIf(1),
        LocalGet(4),
        I32Const(1),
        I32Bin(I32Op::Sub),
        LocalSet(4),
        Br(0),
    ]);
    let backward = vec![
        LocalGet(3),
        If {
            result: None,
            then: vec![
                LocalGet(3),
                I32Const(1),
                I32Bin(I32Op::Sub),
                LocalSet(4),
                Block {
                    body: vec![Loop { body: back_step }],
                },
            ],
            els: vec![],
        },
    ];
    // Forward: i = 0 up to n.
    let mut fwd_step = vec![LocalGet(4), LocalGet(3), I32Cmp(CmpOp::Eq), BrIf(1)];
    fwd_step.extend(elem_eq());
    fwd_step.push(found());
    fwd_step.extend([
        LocalGet(4),
        I32Const(1),
        I32Bin(I32Op::Add),
        LocalSet(4),
        Br(0),
    ]);
    let forward = vec![
        I32Const(0),
        LocalSet(4),
        Block {
            body: vec![Loop { body: fwd_step }],
        },
    ];
    let body = vec![
        LocalGet(0),
        I32Load(LIST_LEN_OFFSET),
        LocalSet(3),
        LocalGet(2),
        If {
            result: None,
            then: backward,
            els: forward,
        },
        I64Const(-1),
    ];
    function(
        name,
        &[Val::I32, val, Val::I32],
        &[Val::I64],
        &[Val::I32; 2],
        body,
    )
}

/// `list_remove_at(list, i, stride)` — shifts the tail one element left
/// with a single overlap-safe copy ((len-1-i)*stride bytes, legally 0 for
/// the last element) and decrements the length in place; bounds are the
/// caller's. Params: list@0, i@1 (i64), stride@2. Locals: dst@3.
fn list_remove_at() -> MirFunction {
    use Inst::*;
    let body = vec![
        // dst = list + 16 + i*stride
        LocalGet(0),
        I32Const(LIST_ELEMS_OFFSET as i32),
        I32Bin(I32Op::Add),
        LocalGet(1),
        I32WrapI64,
        LocalGet(2),
        I32Bin(I32Op::Mul),
        I32Bin(I32Op::Add),
        LocalSet(3),
        LocalGet(3),
        // src = dst + stride
        LocalGet(3),
        LocalGet(2),
        I32Bin(I32Op::Add),
        // n = (len - 1 - i) * stride
        LocalGet(0),
        I32Load(LIST_LEN_OFFSET),
        I32Const(1),
        I32Bin(I32Op::Sub),
        LocalGet(1),
        I32WrapI64,
        I32Bin(I32Op::Sub),
        LocalGet(2),
        I32Bin(I32Op::Mul),
        MemoryCopy,
        // len -= 1
        LocalGet(0),
        LocalGet(0),
        I32Load(LIST_LEN_OFFSET),
        I32Const(1),
        I32Bin(I32Op::Sub),
        I32Store(LIST_LEN_OFFSET),
    ];
    function(
        "__clean_list_remove_at",
        &[Val::I32, Val::I64, Val::I32],
        &[],
        &[Val::I32],
        body,
    )
}
