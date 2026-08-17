//! The TypedAST (Platform 14 pass [5] output): every expression carries its
//! resolved type; enum literals and record constructions are already lowered
//! to their boundary representations (case index, ordered fields). Implicit
//! conversions are materialised as explicit nodes (`IntToNumber`,
//! `WrapSome`) so later passes never re-discover TYP-06/TYP-03.

use crate::parser::ast::{BinOp, UnOp};
use crate::source::ByteSpan;

use super::types::Ty;

pub struct TypedProgram {
    /// One entry per declared host function, in declaration order — the
    /// import surface later passes verify against the world (pass [9]) and
    /// emit (pass [10]).
    pub host_imports: Vec<HostImport>,
    pub functions: Vec<TFunction>,
}

pub struct HostImport {
    /// Kebab-case interface name as declared (`routing`).
    pub interface: String,
    /// Clean-side camelCase name (`setStatus`).
    pub clean_name: String,
    /// Kebab-case WIT function name (`set-status`).
    pub wit_name: String,
    pub params: Vec<Ty>,
    pub ret: Ty,
    pub span: ByteSpan,
}

pub struct TFunction {
    pub name: String,
    pub params: Vec<Local>,
    pub ret: Ty,
    pub locals: Vec<Local>,
    /// `before:` precondition expressions, in order (CTR-01) — always
    /// checked at runtime, never strippable.
    pub before: Vec<TExpr>,
    /// `after:` postcondition expressions (CTR-02); `result` refers to
    /// the return value inside these.
    pub after: Vec<TExpr>,
    pub body: Vec<TStmt>,
    pub span: ByteSpan,
    /// Which parsed file the declaration lives in (for span conversion).
    pub file: usize,
}

#[derive(Debug, Clone)]
pub struct Local {
    pub name: String,
    pub ty: Ty,
}

/// Index into a function's combined local space: parameters first, then
/// declared locals (`index < params.len()` means parameter).
pub type LocalId = usize;

// Statements are built once and traversed; the size spread is inherent
// to the tree shape (same rationale as `hir::HStmt`).
#[allow(clippy::large_enum_variant)]
pub enum TStmt {
    Let {
        local: LocalId,
        init: Option<TExpr>,
    },
    Assign {
        local: LocalId,
        value: TExpr,
    },
    Return {
        value: Option<TExpr>,
        span: ByteSpan,
    },
    Expr(TExpr),
    If {
        cond: TExpr,
        then: Vec<TStmt>,
        else_ifs: Vec<(TExpr, Vec<TStmt>)>,
        els: Option<Vec<TStmt>>,
    },
    /// `while` (FLW-02 §While).
    While {
        cond: TExpr,
        body: Vec<TStmt>,
    },
    /// `iterate <binder> in <source> [step]` (FLW-02). The binder is a
    /// declared local scoped to the body.
    Iterate {
        binder: LocalId,
        source: TIterSource,
        step: Option<TExpr>,
        body: Vec<TStmt>,
    },
    /// `break` (FLW-03).
    Break {
        span: ByteSpan,
    },
    /// `continue` (FLW-03).
    Continue {
        span: ByteSpan,
    },
    /// `print:` block — one expression per line (07 §Console).
    Print {
        items: Vec<TExpr>,
        span: ByteSpan,
    },
    /// `assert <expr>` (11 §3).
    Assert {
        cond: TExpr,
        span: ByteSpan,
    },
}

/// A typed `iterate` source (FLW-02): the four iterable shapes.
pub enum TIterSource {
    /// Over a list; the binder holds the element type.
    List(TExpr),
    /// Over a string's characters; the binder holds `string`.
    Chars(TExpr),
    /// Over a matrix's rows; the binder holds `list<T>`.
    Rows(TExpr),
    /// `from to to`; the binder holds `integer`.
    Range { from: TExpr, to: TExpr },
}

pub struct TExpr {
    pub ty: Ty,
    pub span: ByteSpan,
    pub kind: TExprKind,
}

pub enum TExprKind {
    Int(i128),
    /// `number` literal (TYP-01: IEEE-754 binary64).
    Num(f64),
    Bool(bool),
    Str(String),
    /// A string literal with `{expr}` interpolations, in source order.
    StrInterp(Vec<TInterpSeg>),
    NoneLit,
    /// Enum case, lowered to its WIT discriminant (ADR-0002 §3).
    EnumCase(u32),
    /// Record construction; field values in WIT declaration order.
    MakeRecord(Vec<TExpr>),
    /// List literal; element values in source order.
    MakeList(Vec<TExpr>),
    /// Matrix literal; each row is a list-typed expression.
    MakeMatrix(Vec<TExpr>),
    Local(LocalId),
    /// Call to a declared host function (index into `host_imports`).
    CallHost {
        import: usize,
        args: Vec<TExpr>,
    },
    /// Call to a user function (index into `functions`).
    CallFn {
        func: usize,
        args: Vec<TExpr>,
    },
    Binary {
        op: BinOp,
        lhs: Box<TExpr>,
        rhs: Box<TExpr>,
    },
    Unary {
        op: UnOp,
        operand: Box<TExpr>,
    },
    /// Bracket access, already classified by receiver shape (IDX rules).
    Index {
        recv: Box<TExpr>,
        index: Box<TExpr>,
        kind: IndexKind,
    },
    /// Postfix `!` (EXP-03): asserts non-`none`, narrows `T?` to `T`;
    /// `RUN004` at runtime when the value is `none`.
    NonNone(Box<TExpr>),
    /// `value is none` / `value is not none`-shaped absence test (TYP-03).
    IsNone {
        operand: Box<TExpr>,
        negated: bool,
    },
    /// TYP-06's implicit `integer` → `number` conversion, materialised.
    IntToNumber(Box<TExpr>),
    /// TYP-03's `T` → `T?` injection, materialised.
    WrapSome(Box<TExpr>),
    /// `result` inside an `after:` contract expression (CTR-02) — the
    /// function's return value.
    ResultRef,
    /// `this` inside a class method/constructor body (CLS-02).
    This,
    /// Instance method call, statically dispatched: the receiver's class
    /// is known (`class` owns the method — possibly an ancestor of the
    /// receiver's class, CLS-02).
    CallMethod {
        class: usize,
        method: usize,
        recv: Box<TExpr>,
        args: Vec<TExpr>,
    },
    /// Dynamically dispatched call on a capability-typed receiver
    /// (CLS-03); `cap` is `None` for an `any` receiver (TYP-02).
    CallDyn {
        cap: Option<usize>,
        method: String,
        recv: Box<TExpr>,
        args: Vec<TExpr>,
    },
    /// Declared-constructor instantiation (CLS-02); lowering is M6.
    CallCtor {
        class: usize,
        ctor: usize,
        args: Vec<TExpr>,
    },
    /// Static method call on a class name (14 §Static Methods).
    CallStatic {
        class: usize,
        method: usize,
        args: Vec<TExpr>,
    },
    /// Instance field access; `class` declares the field (CLS-04).
    GetField {
        class: usize,
        field: usize,
        recv: Box<TExpr>,
    },
    /// Explicit conversion method (TYP-06: `.toInteger()`, `.toNumber()`,
    /// `.toString()`, `.toBoolean()`); the node's `ty` is the target.
    Convert(Box<TExpr>),
    /// A subexpression whose type failed; absorbs downstream checks.
    Error,
}

/// One typed interpolation segment (03 §String Interpolation).
pub enum TInterpSeg {
    Text(String),
    Expr(TExpr),
}

/// The receiver classification of a bracket access (IDX001–IDX005).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexKind {
    /// `list<T>[integer]` → `T`.
    List,
    /// `matrix<T>[integer]` → `list<T>` (a row).
    Matrix,
    /// `pairs<K, V>[K]` → collapsed `V?` (TYP-03: absence does not stack).
    Pairs,
    /// `any[·]` → `any` (TYP-02: checking skipped).
    Any,
}
