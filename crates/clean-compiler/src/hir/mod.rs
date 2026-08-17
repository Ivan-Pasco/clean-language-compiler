//! Pass [7] — HIR Lowering (Platform 14 §14.4.2): erases sugar and
//! canonicalizes control flow. The visible work is folding `else if`
//! chains into right-nested `if`/`else` (FLW-01's own definition of the
//! chain) and dropping names in favour of local slots. Typed constructs
//! that codegen cannot lower yet (loops, `number` arithmetic, string
//! interpolation, …) travel through HIR unchanged; pass [8] reports them
//! through the pre-v1 unsupported channel — the M4 frontier lives there,
//! not in the type checker.
//!
//! HIR is the last IR where source spans are the primary addressing (kept
//! on every expression for the passes that still diagnose).

use crate::source::ByteSpan;
use crate::typecheck::tir;
use crate::typecheck::types::Ty;

#[derive(serde::Serialize)]
pub struct HirProgram {
    pub host_imports: Vec<tir::HostImport>,
    pub functions: Vec<HFunction>,
}

#[derive(serde::Serialize)]
pub struct HFunction {
    pub name: String,
    pub params: Vec<Ty>,
    pub ret: Ty,
    /// Declared locals after the parameters (LocalId space continues).
    pub locals: Vec<Ty>,
    /// `before:`/`after:` contract expressions (chapter 10); their core
    /// lowering is a later milestone (pass [8] reports them).
    pub before: Vec<HExpr>,
    pub after: Vec<HExpr>,
    pub body: Vec<HStmt>,
    /// Which parsed file the declaration lives in (for span conversion).
    pub file: usize,
}

// Statements are built once and traversed; the size spread between a bare
// `Break` and a `Set { value: HExpr }` is inherent to the tree shape and
// not worth boxing every expression for.
#[allow(clippy::large_enum_variant)]
#[derive(serde::Serialize)]
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
    While {
        cond: HExpr,
        body: Vec<HStmt>,
    },
    Iterate {
        binder: usize,
        source: HIterSource,
        step: Option<HExpr>,
        body: Vec<HStmt>,
    },
    Break {
        span: ByteSpan,
    },
    Continue {
        span: ByteSpan,
    },
    Print {
        items: Vec<HExpr>,
        span: ByteSpan,
    },
    Assert {
        cond: HExpr,
        span: ByteSpan,
    },
}

#[derive(serde::Serialize)]
pub enum HIterSource {
    List(HExpr),
    Chars(HExpr),
    Rows(HExpr),
    Range { from: HExpr, to: HExpr },
}

#[derive(serde::Serialize)]
pub struct HExpr {
    pub ty: Ty,
    pub span: ByteSpan,
    pub kind: HExprKind,
}

#[derive(serde::Serialize)]
pub enum HExprKind {
    Int(i128),
    Num(f64),
    Bool(bool),
    Str(String),
    StrInterp(Vec<HInterpSeg>),
    NoneLit,
    EnumCase(u32),
    MakeRecord(Vec<HExpr>),
    MakeList(Vec<HExpr>),
    MakeMatrix(Vec<HExpr>),
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
    Index {
        recv: Box<HExpr>,
        index: Box<HExpr>,
        kind: tir::IndexKind,
    },
    NonNone(Box<HExpr>),
    IsNone {
        operand: Box<HExpr>,
        negated: bool,
    },
    IntToNumber(Box<HExpr>),
    WrapSome(Box<HExpr>),
    ResultRef,
    This,
    GetState {
        module: usize,
        name: String,
    },
    GuardValue,
    Raise(Box<HExpr>),
    OnError {
        value: Box<HExpr>,
        fallback: Box<HExpr>,
    },
    ErrorBinding,
    GetRecordField {
        recv: Box<HExpr>,
        field: usize,
    },
    CallMethod {
        class: usize,
        method: usize,
        recv: Box<HExpr>,
        args: Vec<HExpr>,
    },
    CallDyn {
        cap: Option<usize>,
        method: String,
        recv: Box<HExpr>,
        args: Vec<HExpr>,
    },
    CallCtor {
        class: usize,
        ctor: usize,
        args: Vec<HExpr>,
    },
    CallStatic {
        class: usize,
        method: usize,
        args: Vec<HExpr>,
    },
    GetField {
        class: usize,
        field: usize,
        recv: Box<HExpr>,
    },
    Convert(Box<HExpr>),
}

#[derive(serde::Serialize)]
pub enum HInterpSeg {
    Text(String),
    Expr(HExpr),
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
                    before: f.before.into_iter().map(lower_expr).collect(),
                    after: f.after.into_iter().map(lower_expr).collect(),
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
        tir::TStmt::While { cond, body } => Some(HStmt::While {
            cond: lower_expr(cond),
            body: lower_block(body),
        }),
        tir::TStmt::Iterate {
            binder,
            source,
            step,
            body,
        } => Some(HStmt::Iterate {
            binder,
            source: match source {
                tir::TIterSource::List(e) => HIterSource::List(lower_expr(e)),
                tir::TIterSource::Chars(e) => HIterSource::Chars(lower_expr(e)),
                tir::TIterSource::Rows(e) => HIterSource::Rows(lower_expr(e)),
                tir::TIterSource::Range { from, to } => HIterSource::Range {
                    from: lower_expr(from),
                    to: lower_expr(to),
                },
            },
            step: step.map(lower_expr),
            body: lower_block(body),
        }),
        tir::TStmt::Break { span } => Some(HStmt::Break { span }),
        tir::TStmt::Continue { span } => Some(HStmt::Continue { span }),
        tir::TStmt::Print { items, span } => Some(HStmt::Print {
            items: items.into_iter().map(lower_expr).collect(),
            span,
        }),
        tir::TStmt::Assert { cond, span } => Some(HStmt::Assert {
            cond: lower_expr(cond),
            span,
        }),
    }
}

fn lower_expr(expr: tir::TExpr) -> HExpr {
    let kind = match expr.kind {
        tir::TExprKind::Int(v) => HExprKind::Int(v),
        tir::TExprKind::Num(v) => HExprKind::Num(v),
        tir::TExprKind::Bool(v) => HExprKind::Bool(v),
        tir::TExprKind::Str(v) => HExprKind::Str(v),
        tir::TExprKind::StrInterp(segs) => HExprKind::StrInterp(
            segs.into_iter()
                .map(|seg| match seg {
                    tir::TInterpSeg::Text(t) => HInterpSeg::Text(t),
                    tir::TInterpSeg::Expr(e) => HInterpSeg::Expr(lower_expr(e)),
                })
                .collect(),
        ),
        tir::TExprKind::NoneLit => HExprKind::NoneLit,
        tir::TExprKind::EnumCase(i) => HExprKind::EnumCase(i),
        tir::TExprKind::MakeRecord(fields) => {
            HExprKind::MakeRecord(fields.into_iter().map(lower_expr).collect())
        }
        tir::TExprKind::MakeList(items) => {
            HExprKind::MakeList(items.into_iter().map(lower_expr).collect())
        }
        tir::TExprKind::MakeMatrix(rows) => {
            HExprKind::MakeMatrix(rows.into_iter().map(lower_expr).collect())
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
        tir::TExprKind::Index { recv, index, kind } => HExprKind::Index {
            recv: Box::new(lower_expr(*recv)),
            index: Box::new(lower_expr(*index)),
            kind,
        },
        tir::TExprKind::NonNone(operand) => HExprKind::NonNone(Box::new(lower_expr(*operand))),
        tir::TExprKind::IsNone { operand, negated } => HExprKind::IsNone {
            operand: Box::new(lower_expr(*operand)),
            negated,
        },
        tir::TExprKind::IntToNumber(operand) => {
            HExprKind::IntToNumber(Box::new(lower_expr(*operand)))
        }
        tir::TExprKind::WrapSome(operand) => HExprKind::WrapSome(Box::new(lower_expr(*operand))),
        tir::TExprKind::ResultRef => HExprKind::ResultRef,
        tir::TExprKind::This => HExprKind::This,
        tir::TExprKind::GetState { module, name } => HExprKind::GetState { module, name },
        tir::TExprKind::GuardValue => HExprKind::GuardValue,
        tir::TExprKind::Raise(operand) => HExprKind::Raise(Box::new(lower_expr(*operand))),
        tir::TExprKind::OnError { value, fallback } => HExprKind::OnError {
            value: Box::new(lower_expr(*value)),
            fallback: Box::new(lower_expr(*fallback)),
        },
        tir::TExprKind::ErrorBinding => HExprKind::ErrorBinding,
        tir::TExprKind::GetRecordField { recv, field } => HExprKind::GetRecordField {
            recv: Box::new(lower_expr(*recv)),
            field,
        },
        tir::TExprKind::CallMethod {
            class,
            method,
            recv,
            args,
        } => HExprKind::CallMethod {
            class,
            method,
            recv: Box::new(lower_expr(*recv)),
            args: args.into_iter().map(lower_expr).collect(),
        },
        tir::TExprKind::CallDyn {
            cap,
            method,
            recv,
            args,
        } => HExprKind::CallDyn {
            cap,
            method,
            recv: Box::new(lower_expr(*recv)),
            args: args.into_iter().map(lower_expr).collect(),
        },
        tir::TExprKind::CallCtor { class, ctor, args } => HExprKind::CallCtor {
            class,
            ctor,
            args: args.into_iter().map(lower_expr).collect(),
        },
        tir::TExprKind::CallStatic {
            class,
            method,
            args,
        } => HExprKind::CallStatic {
            class,
            method,
            args: args.into_iter().map(lower_expr).collect(),
        },
        tir::TExprKind::GetField { class, field, recv } => HExprKind::GetField {
            class,
            field,
            recv: Box::new(lower_expr(*recv)),
        },
        tir::TExprKind::Convert(operand) => HExprKind::Convert(Box::new(lower_expr(*operand))),
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
