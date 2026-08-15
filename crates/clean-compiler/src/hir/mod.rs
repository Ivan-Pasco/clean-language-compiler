//! Pass [7] — HIR Lowering (Platform 14 §14.4.2): erases sugar and
//! canonicalizes control flow. The Milestone 1 surface has little sugar to
//! erase — the visible work is folding `else if` chains into right-nested
//! `if`/`else` (FLW-01's own definition of the chain) and dropping names in
//! favour of local slots. String-interpolation desugar to concatenation
//! arrives with strings support in later milestones.
//!
//! HIR is the last IR where source spans are the primary addressing (kept
//! on every expression for the passes that still diagnose).

use crate::source::ByteSpan;
use crate::typecheck::tir;
use crate::typecheck::types::Ty;

pub struct HirProgram {
    pub host_imports: Vec<tir::HostImport>,
    pub functions: Vec<HFunction>,
}

pub struct HFunction {
    pub name: String,
    pub params: Vec<Ty>,
    pub ret: Ty,
    /// Declared locals after the parameters (LocalId space continues).
    pub locals: Vec<Ty>,
    pub body: Vec<HStmt>,
    /// Which parsed file the declaration lives in (for span conversion).
    pub file: usize,
}

pub enum HStmt {
    Set {
        local: usize,
        value: HExpr,
    },
    Return {
        value: Option<HExpr>,
    },
    Expr(HExpr),
    If {
        cond: HExpr,
        then: Vec<HStmt>,
        els: Vec<HStmt>,
    },
}

pub struct HExpr {
    pub ty: Ty,
    pub span: ByteSpan,
    pub kind: HExprKind,
}

pub enum HExprKind {
    Int(i128),
    Bool(bool),
    Str(String),
    NoneLit,
    EnumCase(u32),
    MakeRecord(Vec<HExpr>),
    Local(usize),
    CallHost {
        import: usize,
        args: Vec<HExpr>,
    },
    CallFn {
        func: usize,
        args: Vec<HExpr>,
    },
    Binary {
        op: crate::parser::ast::BinOp,
        lhs: Box<HExpr>,
        rhs: Box<HExpr>,
    },
    Unary {
        op: crate::parser::ast::UnOp,
        operand: Box<HExpr>,
    },
}

pub fn lower(program: tir::TypedProgram) -> HirProgram {
    HirProgram {
        host_imports: program.host_imports,
        functions: program
            .functions
            .into_iter()
            .map(|f| {
                let param_count = f.params.len();
                HFunction {
                    name: f.name,
                    params: f.params.into_iter().map(|p| p.ty).collect(),
                    ret: f.ret,
                    locals: f
                        .locals
                        .into_iter()
                        .skip(param_count)
                        .map(|l| l.ty)
                        .collect(),
                    body: lower_block(f.body),
                    file: f.file,
                }
            })
            .collect(),
    }
}

fn lower_block(block: Vec<tir::TStmt>) -> Vec<HStmt> {
    block.into_iter().filter_map(lower_stmt).collect()
}

fn lower_stmt(stmt: tir::TStmt) -> Option<HStmt> {
    match stmt {
        tir::TStmt::Let { local, init } => init.map(|value| HStmt::Set {
            local,
            value: lower_expr(value),
        }),
        tir::TStmt::Assign { local, value } => Some(HStmt::Set {
            local,
            value: lower_expr(value),
        }),
        tir::TStmt::Return { value, .. } => Some(HStmt::Return {
            value: value.map(lower_expr),
        }),
        tir::TStmt::Expr(expr) => Some(HStmt::Expr(lower_expr(expr))),
        tir::TStmt::If {
            cond,
            then,
            else_ifs,
            els,
        } => {
            // FLW-01: an `else if` is an `else` whose body is a single `if`.
            let mut els = els.map(lower_block).unwrap_or_default();
            for (elif_cond, elif_body) in else_ifs.into_iter().rev() {
                els = vec![HStmt::If {
                    cond: lower_expr(elif_cond),
                    then: lower_block(elif_body),
                    els,
                }];
            }
            Some(HStmt::If {
                cond: lower_expr(cond),
                then: lower_block(then),
                els,
            })
        }
    }
}

fn lower_expr(expr: tir::TExpr) -> HExpr {
    let kind = match expr.kind {
        tir::TExprKind::Int(v) => HExprKind::Int(v),
        tir::TExprKind::Bool(v) => HExprKind::Bool(v),
        tir::TExprKind::Str(v) => HExprKind::Str(v),
        tir::TExprKind::NoneLit => HExprKind::NoneLit,
        tir::TExprKind::EnumCase(i) => HExprKind::EnumCase(i),
        tir::TExprKind::MakeRecord(fields) => {
            HExprKind::MakeRecord(fields.into_iter().map(lower_expr).collect())
        }
        tir::TExprKind::Local(id) => HExprKind::Local(id),
        tir::TExprKind::CallHost { import, args } => HExprKind::CallHost {
            import,
            args: args.into_iter().map(lower_expr).collect(),
        },
        tir::TExprKind::CallFn { func, args } => HExprKind::CallFn {
            func,
            args: args.into_iter().map(lower_expr).collect(),
        },
        tir::TExprKind::Binary { op, lhs, rhs } => HExprKind::Binary {
            op,
            lhs: Box::new(lower_expr(*lhs)),
            rhs: Box::new(lower_expr(*rhs)),
        },
        tir::TExprKind::Unary { op, operand } => HExprKind::Unary {
            op,
            operand: Box::new(lower_expr(*operand)),
        },
        tir::TExprKind::Error => {
            unreachable!("error expressions never survive a clean typecheck")
        }
    };
    HExpr {
        ty: expr.ty,
        span: expr.span,
        kind,
    }
}
