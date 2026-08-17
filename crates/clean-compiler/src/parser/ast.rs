//! AST for the Milestone 1 surface. Every node carries a real source span
//! (§14.4.2[3]: no synthetic spans without a source anchor).

use crate::source::ByteSpan;

#[derive(Debug)]
pub struct SourceFile {
    pub path: String,
    pub items: Vec<Item>,
}

#[derive(Debug)]
pub enum Item {
    /// LBS-02 `host interface` block.
    HostInterface(HostInterface),
    /// `functions:` block (08-file-structure `FunctionsBlock`).
    Functions(Vec<Function>),
    /// `class` declaration — fields only in M1 (record projection, ADR-0002).
    Class(ClassDecl),
    /// `start:` section.
    Start(Block),
}

#[derive(Debug)]
pub struct HostInterface {
    /// Kebab-case interface name as written (`routing`, `session-envelope`).
    pub name: String,
    pub version: String,
    pub worlds: Vec<String>,
    pub functions: Vec<HostFunction>,
    pub span: ByteSpan,
}

#[derive(Debug)]
pub struct HostFunction {
    /// camelCase Clean name; kebab-cased when matched against WIT.
    pub name: String,
    pub params: Vec<HostParam>,
    /// `None` means no `returns` clause — a void host function.
    pub ret: Option<TypeExpr>,
    pub description: String,
    pub span: ByteSpan,
}

#[derive(Debug)]
pub struct HostParam {
    pub name: String,
    pub ty: TypeExpr,
    pub span: ByteSpan,
}

#[derive(Debug)]
pub struct Function {
    pub ret: TypeExpr,
    pub name: String,
    pub params: Vec<Param>,
    pub body: Block,
    pub span: ByteSpan,
}

#[derive(Debug)]
pub struct Param {
    pub ty: TypeExpr,
    pub name: String,
    pub default: Option<Expr>,
    pub span: ByteSpan,
}

#[derive(Debug)]
pub struct ClassDecl {
    pub name: String,
    pub fields: Vec<Field>,
    pub span: ByteSpan,
}

#[derive(Debug)]
pub struct Field {
    pub ty: TypeExpr,
    pub name: String,
    pub span: ByteSpan,
}

pub type Block = Vec<Stmt>;

/// The source of an `iterate` loop (FLW-02): a range (`a to b`) or any
/// iterable expression. `to` is iterate-only, never a general operator.
#[derive(Debug)]
pub enum IterateSource {
    Range { from: Expr, to: Expr },
    Expr(Expr),
}

#[derive(Debug)]
pub enum Stmt {
    /// `TypeExpression Identifier [= Expression]` (STM-01). `on_error`
    /// carries the ERH-02 block form terminating the statement
    /// (`T x = expr onError:` + indented handler).
    VarDecl {
        ty: TypeExpr,
        name: String,
        init: Option<Expr>,
        on_error: Option<Block>,
        span: ByteSpan,
    },
    /// `target = value` (STM-02 — statement, never expression).
    Assign {
        target: Expr,
        value: Expr,
        on_error: Option<Block>,
        span: ByteSpan,
    },
    /// `return [expr]` (STM-03).
    Return { value: Option<Expr>, span: ByteSpan },
    /// Expression whose result is discarded; `on_error` is the ERH-02
    /// block form (`expr onError:` + indented handler).
    Expr { expr: Expr, on_error: Option<Block> },
    /// `if` / `else if` / `else` (FLW-01).
    If {
        cond: Expr,
        then: Block,
        else_ifs: Vec<(Expr, Block)>,
        els: Option<Block>,
        span: ByteSpan,
    },
    /// `iterate <binder> in <source> [step <expr>]` (FLW-02).
    Iterate {
        binder: String,
        binder_span: ByteSpan,
        source: IterateSource,
        step: Option<Expr>,
        body: Block,
        span: ByteSpan,
    },
    /// `while <condition>` (FLW-02 §While).
    While {
        cond: Expr,
        body: Block,
        span: ByteSpan,
    },
    /// `break` (FLW-03).
    Break { span: ByteSpan },
    /// `continue` (FLW-03).
    Continue { span: ByteSpan },
    /// `print:` block (STM prose; SYN008 checked at parse).
    Print { items: Vec<Expr>, span: ByteSpan },
}

/// One segment of a string literal (06-expressions §3): literal text or a
/// `{expr}` interpolation, in source order.
#[derive(Debug)]
pub enum StrSeg {
    Text(String),
    Interp { expr: Expr, span: ByteSpan },
}

#[derive(Debug)]
pub enum Expr {
    Int {
        value: u128,
        span: ByteSpan,
    },
    /// Float-shaped literal; the `number` type's semantics land in M6.
    Number {
        text: String,
        span: ByteSpan,
    },
    Str {
        segments: Vec<StrSeg>,
        span: ByteSpan,
    },
    Bool {
        value: bool,
        span: ByteSpan,
    },
    NoneLit {
        span: ByteSpan,
    },
    Ident {
        name: String,
        span: ByteSpan,
    },
    /// `this` in class-method scope (CLS prose §5).
    This {
        span: ByteSpan,
    },
    /// `base` — parent-constructor callee in class scope (CLS-02).
    Base {
        span: ByteSpan,
    },
    /// `error` as the bound Error value inside a handler (ERH-04), or as
    /// the raise/emission callee when a Call wraps it (ERH-01, BLK-03) —
    /// the parser dispatches on the following token per 13 §3.
    ErrorRef {
        span: ByteSpan,
    },
    /// `result` inside an `after:` contract block (CTR-02).
    ResultRef {
        span: ByteSpan,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        span: ByteSpan,
    },
    Member {
        receiver: Box<Expr>,
        name: String,
        span: ByteSpan,
    },
    /// `receiver[index]` (06 §1 IndexAccess).
    Index {
        receiver: Box<Expr>,
        index: Box<Expr>,
        span: ByteSpan,
    },
    /// Postfix `!` — required-assertion on an optional (EXP-03).
    NonNone {
        operand: Box<Expr>,
        span: ByteSpan,
    },
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        span: ByteSpan,
    },
    Unary {
        op: UnOp,
        operand: Box<Expr>,
        span: ByteSpan,
    },
    /// `value onError fallback` — suffix form, level 13 (ERH-02). The block
    /// form is a statement tail, `Stmt`-side.
    OnError {
        value: Box<Expr>,
        fallback: Box<Expr>,
        span: ByteSpan,
    },
    List {
        items: Vec<Expr>,
        span: ByteSpan,
    },
}

impl Expr {
    pub fn span(&self) -> ByteSpan {
        match self {
            Expr::Int { span, .. }
            | Expr::Number { span, .. }
            | Expr::Str { span, .. }
            | Expr::Bool { span, .. }
            | Expr::NoneLit { span }
            | Expr::Ident { span, .. }
            | Expr::This { span }
            | Expr::Base { span }
            | Expr::ErrorRef { span }
            | Expr::ResultRef { span }
            | Expr::Call { span, .. }
            | Expr::Member { span, .. }
            | Expr::Index { span, .. }
            | Expr::NonNone { span, .. }
            | Expr::Binary { span, .. }
            | Expr::Unary { span, .. }
            | Expr::OnError { span, .. }
            | Expr::List { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Pow,
    Eq,
    NEq,
    /// `a is b` — identity comparison, level 8 (EXP-01).
    Is,
    /// `a not b` — binary `not` in operator position, level 8 (EXP-01).
    NotIs,
    Lt,
    LtEq,
    Gt,
    GtEq,
    And,
    Or,
    /// EXP-03 none-coalescing fallback (`value default fallback`), level 11.
    Default,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Not,
    Neg,
}

/// A type as written (04-type-system.ebnf.md), plus the width-suffix forms
/// valid only in host-function positions (LBS-02, ADR-0002).
#[derive(Debug, Clone, PartialEq)]
pub struct TypeExpr {
    pub base: BaseType,
    pub optional: bool,
    /// Spans of `?` markers beyond the first. TYP-03 forbids `T??`; the
    /// parser records the extras so the checker can report SEM009 (M4).
    pub extra_optionals: Vec<ByteSpan>,
    /// TYP-05 behavior suffix chain (`list<T>.line`), declaration-LHS only.
    /// The grammar admits any chain; the checker restricts combinations.
    pub behaviors: Vec<Behavior>,
    pub span: ByteSpan,
}

/// One `.line` / `.pile` / `.unique` suffix (TYP-05).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Behavior {
    pub name: BehaviorName,
    pub span: ByteSpan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BehaviorName {
    Line,
    Pile,
    Unique,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BaseType {
    Boolean,
    /// `integer`, or width-suffixed `integer:32` / `integer:u32` etc. in
    /// host-function declarations only.
    Integer(Option<IntWidth>),
    Number,
    String_,
    Bytes,
    Datetime,
    Any,
    Void,
    List(Box<TypeExpr>),
    Matrix(Box<TypeExpr>),
    Pairs(Box<TypeExpr>, Box<TypeExpr>),
    /// Class, capability, world-declared, or compile-time (TYP-04) type
    /// referenced by name.
    Named(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntWidth {
    S32,
    U8,
    U16,
    U32,
    U64,
}
