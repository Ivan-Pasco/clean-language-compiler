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

#[derive(Debug)]
pub enum Stmt {
    /// `TypeExpression Identifier [= Expression]` (STM-01).
    VarDecl {
        ty: TypeExpr,
        name: String,
        init: Option<Expr>,
        span: ByteSpan,
    },
    /// `target = value` (STM-02 — statement, never expression).
    Assign {
        target: Expr,
        value: Expr,
        span: ByteSpan,
    },
    /// `return [expr]` (STM-03).
    Return { value: Option<Expr>, span: ByteSpan },
    /// Expression whose result is discarded.
    Expr(Expr),
    /// `if` / `else if` / `else` (FLW-01).
    If {
        cond: Expr,
        then: Block,
        else_ifs: Vec<(Expr, Block)>,
        els: Option<Block>,
        span: ByteSpan,
    },
    /// `print:` block (STM prose; SYN008 checked at parse).
    Print { items: Vec<Expr>, span: ByteSpan },
}

#[derive(Debug)]
pub enum Expr {
    Int {
        value: u128,
        span: ByteSpan,
    },
    /// Float-shaped literal; `number` support is outside the M1 surface.
    Number {
        text: String,
        span: ByteSpan,
    },
    Str {
        value: String,
        /// Non-empty means interpolation — outside the M1 surface, rejected
        /// in typecheck with these exact spans.
        interpolations: Vec<ByteSpan>,
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
            | Expr::Call { span, .. }
            | Expr::Member { span, .. }
            | Expr::Binary { span, .. }
            | Expr::Unary { span, .. }
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
    pub span: ByteSpan,
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
    Pairs(Box<TypeExpr>, Box<TypeExpr>),
    /// Class, capability, or world-declared type referenced by name.
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
