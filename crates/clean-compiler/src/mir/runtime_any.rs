//! The `any` box helpers (ADR 0005): the [`RuntimeFn::AnyBoxInt`]..=
//! `AnyIndexStr` bodies, built as ordinary [`MirFunction`]s in
//! discriminant order (the contracts live on the enum in `runtime.rs`).
//! A box is `[tag u32 @0][pad @4][payload @8]`, 16 bytes, 8-aligned; tag
//! [`ANY_TAG_NUM_SRC`] boxes are 24 bytes with the source-text pointer at
//! +16 and read as NUM here (the f64 payload sits at +8 either way). The
//! shared `none` box is the static constant [`NONE_BOX_ADDR`] — helpers
//! that produce `none` return it and never allocate one. Unboxing at a
//! wrong tag traps (RUN005 family until error lowering); indexing never
//! traps — chapter 15 access semantics answer `none` instead.

use crate::layout::{
    ALIGNMENT, ANY_TAG_BOOL, ANY_TAG_INT, ANY_TAG_LIST, ANY_TAG_NONE, ANY_TAG_NUM, ANY_TAG_NUM_SRC,
    ANY_TAG_PAIRS, ANY_TAG_STR, LIST_ELEMS_OFFSET, LIST_LEN_OFFSET, NONE_BOX_ADDR,
    PAIRS_COUNT_OFFSET, PAIRS_ENTRIES_OFFSET,
};

use super::runtime::RuntimeFn;
use super::{CmpOp, I32Op, Inst, MirFunction, Val};

/// The `any` helpers, in [`RuntimeFn`] discriminant order
/// (`AnyBoxInt`..=`AnyIndexStr`); `runtime::build` appends them after the
/// bytes helpers.
pub fn build() -> Vec<MirFunction> {
    vec![
        box_fn("__clean_any_box_int", ANY_TAG_INT, Val::I64),
        box_fn("__clean_any_box_num", ANY_TAG_NUM, Val::F64),
        box_fn("__clean_any_box_bool", ANY_TAG_BOOL, Val::I32),
        box_fn("__clean_any_box_str", ANY_TAG_STR, Val::I32),
        box_fn("__clean_any_box_list", ANY_TAG_LIST, Val::I32),
        box_fn("__clean_any_box_pairs", ANY_TAG_PAIRS, Val::I32),
        any_unbox_int(),
        any_unbox_num(),
        any_unbox_bool(),
        any_unbox_str(),
        any_is_none(),
        any_index_int(),
        any_index_str(),
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

/// Loads the box's tag into local `t` (the box pointer sits in param 0).
fn load_tag(t: u32) -> Vec<Inst> {
    use Inst::*;
    vec![LocalGet(0), I32Load(0), LocalSet(t)]
}

/// Pushes 1 iff local `t` equals `tag`.
fn tag_is(t: u32, tag: u32) -> Vec<Inst> {
    use Inst::*;
    vec![LocalGet(t), I32Const(tag as i32), I32Cmp(CmpOp::Eq)]
}

/// One boxing helper: `alloc(16, 8)`, store `tag` at +0 and the value at
/// +8 (the store width follows the parameter type), return the pointer.
/// Params: v@0. Locals: r@1.
fn box_fn(name: &str, tag: u32, param: Val) -> MirFunction {
    use Inst::*;
    let store = match param {
        Val::I64 => I64Store(8),
        Val::F64 => F64Store(8),
        Val::I32 => I32Store(8),
    };
    let body = vec![
        I32Const(16),
        I32Const(ALIGNMENT as i32),
        CallRuntime(RuntimeFn::Alloc),
        LocalTee(1),
        I32Const(tag as i32),
        I32Store(0),
        LocalGet(1),
        LocalGet(0),
        store,
        LocalGet(1),
    ];
    function(name, &[param], &[Val::I32], &[Val::I32], body)
}

/// `any_unbox_int(box) -> i64` — tag INT verbatim; tags NUM/NUM_SRC
/// truncate toward zero (`i64.trunc_f64_s` traps on NaN or out of range,
/// which is the RUN005-family surface); anything else traps.
/// Params: box@0. Locals: t@1.
fn any_unbox_int() -> MirFunction {
    use Inst::*;
    let mut num = tag_is(1, ANY_TAG_NUM);
    num.extend(tag_is(1, ANY_TAG_NUM_SRC));
    num.extend([
        I32Bin(I32Op::Or),
        If {
            result: Some(Val::I64),
            then: vec![LocalGet(0), F64Load(8), I64TruncF64S],
            els: vec![Unreachable],
        },
    ]);
    let mut body = load_tag(1);
    body.extend(tag_is(1, ANY_TAG_INT));
    body.push(If {
        result: Some(Val::I64),
        then: vec![LocalGet(0), I64Load(8)],
        els: num,
    });
    function(
        "__clean_any_unbox_int",
        &[Val::I32],
        &[Val::I64],
        &[Val::I32],
        body,
    )
}

/// `any_unbox_num(box) -> f64` — tags NUM/NUM_SRC verbatim; tag INT
/// widens (`f64.convert_i64_s`); anything else traps.
/// Params: box@0. Locals: t@1.
fn any_unbox_num() -> MirFunction {
    use Inst::*;
    let mut num = tag_is(1, ANY_TAG_NUM);
    num.extend(tag_is(1, ANY_TAG_NUM_SRC));
    num.extend([
        I32Bin(I32Op::Or),
        If {
            result: Some(Val::F64),
            then: vec![LocalGet(0), F64Load(8)],
            els: vec![Unreachable],
        },
    ]);
    let mut body = load_tag(1);
    body.extend(tag_is(1, ANY_TAG_INT));
    body.push(If {
        result: Some(Val::F64),
        then: vec![LocalGet(0), I64Load(8), F64ConvertI64S],
        els: num,
    });
    function(
        "__clean_any_unbox_num",
        &[Val::I32],
        &[Val::F64],
        &[Val::I32],
        body,
    )
}

/// One strict scalar unboxing: the i32 payload at +8 when the tag
/// matches, a trap otherwise. Params: box@0. Locals: t@1.
fn unbox_i32(name: &str, tag: u32) -> MirFunction {
    use Inst::*;
    let mut body = load_tag(1);
    body.extend(tag_is(1, tag));
    body.push(If {
        result: Some(Val::I32),
        then: vec![LocalGet(0), I32Load(8)],
        els: vec![Unreachable],
    });
    function(name, &[Val::I32], &[Val::I32], &[Val::I32], body)
}

/// `any_unbox_bool(box) -> i32` — tag BOOL or trap.
fn any_unbox_bool() -> MirFunction {
    unbox_i32("__clean_any_unbox_bool", ANY_TAG_BOOL)
}

/// `any_unbox_str(box) -> base` — tag STR or trap.
fn any_unbox_str() -> MirFunction {
    unbox_i32("__clean_any_unbox_str", ANY_TAG_STR)
}

/// `any_is_none(box) -> i32` — 1 iff the tag is NONE. Params: box@0.
fn any_is_none() -> MirFunction {
    use Inst::*;
    let body = vec![
        LocalGet(0),
        I32Load(0),
        I32Const(ANY_TAG_NONE as i32),
        I32Cmp(CmpOp::Eq),
    ];
    function("__clean_any_is_none", &[Val::I32], &[Val::I32], &[], body)
}

/// `any_index_int(box, i) -> box` — the list element's box pointer, or
/// the shared `none` box for a non-list receiver or an out-of-range index
/// (never a trap — chapter 15 access semantics). Bounds compare in the
/// i64 domain with the length zero-extended.
/// Params: box@0, i@1 (i64). Locals: list@2.
fn any_index_int() -> MirFunction {
    use Inst::*;
    let none = || vec![I32Const(NONE_BOX_ADDR as i32), Return];
    let body = vec![
        LocalGet(0),
        I32Load(0),
        I32Const(ANY_TAG_LIST as i32),
        I32Cmp(CmpOp::Ne),
        If {
            result: None,
            then: none(),
            els: vec![],
        },
        LocalGet(0),
        I32Load(8),
        LocalSet(2),
        LocalGet(1),
        I64Const(0),
        I64Cmp(CmpOp::LtS),
        If {
            result: None,
            then: none(),
            els: vec![],
        },
        LocalGet(1),
        LocalGet(2),
        I32Load(LIST_LEN_OFFSET),
        I64ExtendI32U,
        I64Cmp(CmpOp::GeS),
        If {
            result: None,
            then: none(),
            els: vec![],
        },
        // elements are 4-byte box pointers: list + i*4, load at +ELEMS.
        LocalGet(2),
        LocalGet(1),
        I32WrapI64,
        I32Const(4),
        I32Bin(I32Op::Mul),
        I32Bin(I32Op::Add),
        I32Load(LIST_ELEMS_OFFSET),
    ];
    function(
        "__clean_any_index_int",
        &[Val::I32, Val::I64],
        &[Val::I32],
        &[Val::I32],
        body,
    )
}

/// `any_index_str(box, key) -> box` — pairs lookup by string key: a
/// linear scan of the entries (§3.4.2 lookup is O(n)), first `string_eq`
/// hit returns the value's box pointer; a miss or a non-pairs receiver
/// answers the shared `none` box.
/// Params: box@0, key@1. Locals: pairs@2, count@3, i@4.
fn any_index_str() -> MirFunction {
    use Inst::*;
    // entry i's base is pairs + i*8; the constant offsets add
    // PAIRS_ENTRIES_OFFSET (key at +0, value at +4 within the entry).
    let entry_base = |out: &mut Vec<Inst>| {
        out.extend([
            LocalGet(2),
            LocalGet(4),
            I32Const(8),
            I32Bin(I32Op::Mul),
            I32Bin(I32Op::Add),
        ]);
    };
    let mut step = vec![LocalGet(4), LocalGet(3), I32Cmp(CmpOp::Eq), BrIf(1)];
    entry_base(&mut step);
    step.push(I32Load(PAIRS_ENTRIES_OFFSET));
    step.push(LocalGet(1));
    step.push(CallRuntime(RuntimeFn::StringEq));
    let mut hit = Vec::new();
    entry_base(&mut hit);
    hit.push(I32Load(PAIRS_ENTRIES_OFFSET + 4));
    hit.push(Return);
    step.push(If {
        result: None,
        then: hit,
        els: vec![],
    });
    step.extend([
        LocalGet(4),
        I32Const(1),
        I32Bin(I32Op::Add),
        LocalSet(4),
        Br(0),
    ]);
    let body = vec![
        LocalGet(0),
        I32Load(0),
        I32Const(ANY_TAG_PAIRS as i32),
        I32Cmp(CmpOp::Ne),
        If {
            result: None,
            then: vec![I32Const(NONE_BOX_ADDR as i32), Return],
            els: vec![],
        },
        LocalGet(0),
        I32Load(8),
        LocalSet(2),
        LocalGet(2),
        I32Load(PAIRS_COUNT_OFFSET),
        LocalSet(3),
        I32Const(0),
        LocalSet(4),
        Block {
            body: vec![Loop { body: step }],
        },
        I32Const(NONE_BOX_ADDR as i32),
    ];
    function(
        "__clean_any_index_str",
        &[Val::I32, Val::I32],
        &[Val::I32],
        &[Val::I32; 3],
        body,
    )
}
