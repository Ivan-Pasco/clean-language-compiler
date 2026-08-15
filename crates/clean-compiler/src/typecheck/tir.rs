//! The TypedAST (Platform 14 pass [5] output): every expression carries its
//! resolved type; enum literals and record constructions are already lowered
//! to their boundary representations (case index, ordered fields).

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
}

pub struct TExpr {
    pub ty: Ty,
    pub span: ByteSpan,
    pub kind: TExprKind,
}

pub enum TExprKind {
    Int(i128),
    Bool(bool),
    Str(String),
    NoneLit,
    /// Enum case, lowered to its WIT discriminant (ADR-0002 §3).
    EnumCase(u32),
    /// Record construction; field values in WIT declaration order.
    MakeRecord(Vec<TExpr>),
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
    /// A subexpression whose type failed; absorbs downstream checks.
    Error,
}
