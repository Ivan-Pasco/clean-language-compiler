//! Pass [5] — Type Check, Milestone 1 slice: direct checking over the
//! resolved AST (bidirectional inference with `ena` is M4). Message wording
//! comes verbatim from Platform 10 §3/§5. Error types absorb downstream
//! mismatches so one mistake reports once (§14.4.2[5]).

use clean_compiler_types::{codes, Annotation, Level};
use indexmap::IndexMap;
use wit_parser::{TypeDefKind, WorldItem, WorldKey};

use crate::codegen::world::ParsedWorld;
use crate::diag::{build, DiagnosticSink};
use crate::parser::ast;
use crate::resolver::ResolvedAst;
use crate::source::ByteSpan;

use super::tir::*;
use super::types::{kebab, Ty};

pub fn check(
    resolved: &ResolvedAst,
    world: &ParsedWorld,
    sink: &mut DiagnosticSink,
) -> TypedProgram {
    let mut checker = Checker {
        resolved,
        world,
        class_records: IndexMap::new(),
        host_imports: Vec::new(),
        function_sigs: Vec::new(),
    };
    checker.project_classes(sink);
    checker.project_host_interfaces(sink);
    checker.collect_function_signatures(sink);
    let functions = checker.check_functions(sink);
    TypedProgram {
        host_imports: checker.host_imports,
        functions,
    }
}

struct FunctionSig {
    name: String,
    params: Vec<Local>,
    ret: Ty,
}

struct Checker<'a> {
    resolved: &'a ResolvedAst,
    world: &'a ParsedWorld,
    /// Class name → projected record type (LBS-02 class↔record).
    class_records: IndexMap<String, Ty>,
    host_imports: Vec<HostImport>,
    function_sigs: Vec<FunctionSig>,
}

impl<'a> Checker<'a> {
    fn span(&self, file: usize, span: ByteSpan) -> clean_compiler_types::Span {
        self.resolved.span(file, span)
    }

    // ----- projections -------------------------------------------------

    fn project_classes(&mut self, sink: &mut DiagnosticSink) {
        for coords in self.resolved.decls.classes.values() {
            let (class, file) = self.resolved.class(*coords);
            let mut fields = Vec::new();
            for field in &class.fields {
                let ty = self.project_surface_type(&field.ty, file, sink);
                fields.push((kebab(&field.name), ty));
            }
            self.class_records.insert(
                class.name.clone(),
                Ty::Record {
                    wit_name: kebab(&class.name),
                    fields,
                },
            );
        }
    }

    fn project_host_interfaces(&mut self, sink: &mut DiagnosticSink) {
        for slot in 0..self.resolved.decls.host_interfaces.len() {
            let (hi, file) = self.resolved.host_interface(slot);
            let (name, functions) = (hi.name.clone(), &hi.functions);
            for f in functions {
                let params: Vec<Ty> = f
                    .params
                    .iter()
                    .map(|p| self.project_host_type(&p.ty, &name, file, sink))
                    .collect();
                let ret = match &f.ret {
                    Some(ty) => self.project_host_type(ty, &name, file, sink),
                    None => Ty::Void,
                };
                self.host_imports.push(HostImport {
                    interface: name.clone(),
                    clean_name: f.name.clone(),
                    wit_name: kebab(&f.name),
                    params,
                    ret,
                    span: f.span,
                });
            }
        }
    }

    fn collect_function_signatures(&mut self, sink: &mut DiagnosticSink) {
        for coords in self.resolved.decls.functions.values() {
            let (f, file) = self.resolved.function(*coords);
            let params = f
                .params
                .iter()
                .map(|p| Local {
                    name: p.name.clone(),
                    ty: self.project_surface_type(&p.ty, file, sink),
                })
                .collect();
            let ret = self.project_surface_type(&f.ret, file, sink);
            self.function_sigs.push(FunctionSig {
                name: f.name.clone(),
                params,
                ret,
            });
        }
    }

    /// Surface-language type positions (variables, parameters, fields).
    fn project_surface_type(
        &self,
        ty: &ast::TypeExpr,
        file: usize,
        sink: &mut DiagnosticSink,
    ) -> Ty {
        let base = match &ty.base {
            ast::BaseType::Boolean => Ty::Boolean,
            ast::BaseType::Integer(None) => Ty::Integer,
            ast::BaseType::Integer(Some(width)) => Ty::IntegerW(*width),
            ast::BaseType::String_ => Ty::Str,
            ast::BaseType::Bytes => Ty::Bytes,
            ast::BaseType::Void => Ty::Void,
            ast::BaseType::Named(name) => match self.class_records.get(name) {
                Some(record) => record.clone(),
                None => {
                    sink.push(build(
                        Level::Error,
                        codes::SEM020,
                        format!("I cannot find a class named `{name}`"),
                        self.span(file, ty.span),
                        Some("no class with this name is in scope".to_string()),
                    ));
                    Ty::Error
                }
            },
            ast::BaseType::Number
            | ast::BaseType::Datetime
            | ast::BaseType::Any
            | ast::BaseType::List(_)
            | ast::BaseType::Pairs(_, _) => {
                sink.note_unsupported(
                    "type outside the Milestone 1 surface",
                    self.span(file, ty.span),
                );
                Ty::Error
            }
        };
        if ty.optional {
            Ty::Option(Box::new(base))
        } else {
            base
        }
    }

    /// Host-function type positions additionally admit world-declared type
    /// names (ADR-0002 §3): `method`, `options`, `level`, `field`, …
    fn project_host_type(
        &self,
        ty: &ast::TypeExpr,
        interface: &str,
        file: usize,
        sink: &mut DiagnosticSink,
    ) -> Ty {
        if let ast::BaseType::Named(name) = &ty.base {
            if !self.class_records.contains_key(name) {
                let projected = self.project_world_type(interface, name);
                return match projected {
                    Some(base) if ty.optional => Ty::Option(Box::new(base)),
                    Some(base) => base,
                    None => {
                        // Neither a class nor a world type. The world check
                        // (pass [9]) reports the function-level mismatch; a
                        // missing type name here is SEM020 either way.
                        sink.push(build(
                            Level::Error,
                            codes::SEM020,
                            format!("I cannot find a class named `{name}`"),
                            self.span(file, ty.span),
                            Some("no class with this name is in scope".to_string()),
                        ));
                        Ty::Error
                    }
                };
            }
        }
        self.project_surface_type(ty, file, sink)
    }

    /// Looks up `type_name` among the types of the world interface with the
    /// declared name, projecting enum and record shapes.
    fn project_world_type(&self, interface: &str, type_name: &str) -> Option<Ty> {
        let resolve = &self.world.resolve;
        let world = &resolve.worlds[self.world.world];
        for (key, item) in &world.exports {
            let WorldItem::Interface { id, .. } = item else {
                continue;
            };
            let name = match key {
                WorldKey::Name(n) => n.clone(),
                WorldKey::Interface(i) => resolve.interfaces[*i].name.clone().unwrap_or_default(),
            };
            if name != interface {
                continue;
            }
            let iface = &resolve.interfaces[*id];
            let type_id = iface.types.get(type_name)?;
            return self.project_wit_type(&wit_parser::Type::Id(*type_id));
        }
        None
    }

    fn project_wit_type(&self, ty: &wit_parser::Type) -> Option<Ty> {
        use wit_parser::Type as W;
        Some(match ty {
            W::Bool => Ty::Boolean,
            W::U8 => Ty::IntegerW(ast::IntWidth::U8),
            W::U16 => Ty::IntegerW(ast::IntWidth::U16),
            W::U32 => Ty::IntegerW(ast::IntWidth::U32),
            W::U64 => Ty::IntegerW(ast::IntWidth::U64),
            W::S32 => Ty::IntegerW(ast::IntWidth::S32),
            W::S64 => Ty::Integer,
            W::String => Ty::Str,
            W::Id(id) => {
                let def = &self.world.resolve.types[*id];
                match &def.kind {
                    TypeDefKind::Enum(e) => Ty::Enum {
                        wit_name: def.name.clone()?,
                        cases: e.cases.iter().map(|c| c.name.clone()).collect(),
                    },
                    TypeDefKind::Record(r) => Ty::Record {
                        wit_name: def.name.clone()?,
                        fields: r
                            .fields
                            .iter()
                            .map(|f| Some((f.name.clone(), self.project_wit_type(&f.ty)?)))
                            .collect::<Option<Vec<_>>>()?,
                    },
                    TypeDefKind::List(W::U8) => Ty::Bytes,
                    TypeDefKind::List(_) => return None,
                    TypeDefKind::Option(inner) => {
                        Ty::Option(Box::new(self.project_wit_type(inner)?))
                    }
                    _ => return None,
                }
            }
            _ => return None,
        })
    }

    // ----- function bodies ----------------------------------------------

    fn check_functions(&mut self, sink: &mut DiagnosticSink) -> Vec<TFunction> {
        let mut out = Vec::new();
        for (index, coords) in self.resolved.decls.functions.values().enumerate() {
            let (f, file) = self.resolved.function(*coords);
            let sig = &self.function_sigs[index];
            let scope: IndexMap<String, LocalId> = sig
                .params
                .iter()
                .enumerate()
                .map(|(i, p)| (p.name.clone(), i))
                .collect();
            let mut body_checker = BodyChecker {
                outer: self,
                file,
                locals: sig.params.clone(),
                scopes: vec![scope],
                ret: sig.ret.clone(),
                fn_name: sig.name.clone(),
            };
            let body = body_checker.check_block(&f.body, sink);
            let locals = body_checker.locals;
            out.push(TFunction {
                name: sig.name.clone(),
                params: sig.params.clone(),
                ret: sig.ret.clone(),
                locals,
                body,
                span: f.span,
            });
        }
        out
    }
}

struct BodyChecker<'c, 'a> {
    outer: &'c Checker<'a>,
    /// Which parsed file this function lives in (for span conversion).
    file: usize,
    /// Parameters first, then every declared local (LocalId space).
    locals: Vec<Local>,
    /// Lexical scope stack: name → LocalId.
    scopes: Vec<IndexMap<String, LocalId>>,
    ret: Ty,
    fn_name: String,
}

impl<'c, 'a> BodyChecker<'c, 'a> {
    fn diag_span(&self, span: ByteSpan) -> clean_compiler_types::Span {
        self.outer.span(self.file, span)
    }

    fn lookup(&self, name: &str) -> Option<LocalId> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
    }

    fn check_block(&mut self, block: &[ast::Stmt], sink: &mut DiagnosticSink) -> Vec<TStmt> {
        self.scopes.push(IndexMap::new());
        let out = block
            .iter()
            .filter_map(|stmt| self.check_stmt(stmt, sink))
            .collect();
        self.scopes.pop();
        out
    }

    fn check_stmt(&mut self, stmt: &ast::Stmt, sink: &mut DiagnosticSink) -> Option<TStmt> {
        match stmt {
            ast::Stmt::VarDecl {
                ty,
                name,
                init,
                span,
            } => {
                let declared = self.outer.project_surface_type(ty, self.file, sink);
                if self.scopes.last().unwrap().contains_key(name) {
                    sink.push(build(
                        Level::Error,
                        codes::SCOPE002,
                        format!("variable `{name}` cannot be redeclared in the same scope"),
                        self.diag_span(*span),
                        None,
                    ));
                }
                let init = init.as_ref().map(|expr| {
                    let value = self.check_expr(expr, Some(&declared), sink);
                    if !assignable(&value.ty, &declared) {
                        // SEM001 — exact wording from Platform 10 §3.
                        let mut d = build(
                            Level::Error,
                            codes::SEM001,
                            "type mismatch in assignment".to_string(),
                            self.diag_span(*span),
                            Some(format!(
                                "`{name}` is declared with type `{}`",
                                declared.display()
                            )),
                        );
                        d.secondary.push(Annotation {
                            span: self.diag_span(expr.span()),
                            label: format!("this expression has type `{}`", value.ty.display()),
                        });
                        d.rendered = crate::diag::render_cli(&d);
                        sink.push(d);
                    }
                    value
                });
                let id = self.locals.len();
                self.locals.push(Local {
                    name: name.clone(),
                    ty: declared,
                });
                self.scopes.last_mut().unwrap().insert(name.clone(), id);
                Some(TStmt::Let { local: id, init })
            }
            ast::Stmt::Assign {
                target,
                value,
                span,
            } => match target {
                ast::Expr::Ident {
                    name,
                    span: name_span,
                } => {
                    let Some(local) = self.lookup(name) else {
                        sink.push(build(
                            Level::Error,
                            codes::SEM002,
                            format!("I cannot find a variable named `{name}` in scope"),
                            self.diag_span(*name_span),
                            Some("no variable with this name exists here".to_string()),
                        ));
                        return None;
                    };
                    let declared = self.locals[local].ty.clone();
                    let value = self.check_expr(value, Some(&declared), sink);
                    if !assignable(&value.ty, &declared) {
                        let mut d = build(
                            Level::Error,
                            codes::SEM001,
                            "type mismatch in assignment".to_string(),
                            self.diag_span(*name_span),
                            Some(format!(
                                "`{name}` is declared with type `{}`",
                                declared.display()
                            )),
                        );
                        d.secondary.push(Annotation {
                            span: self.diag_span(value.span),
                            label: format!("this expression has type `{}`", value.ty.display()),
                        });
                        d.rendered = crate::diag::render_cli(&d);
                        sink.push(d);
                    }
                    Some(TStmt::Assign { local, value })
                }
                _ => {
                    sink.note_unsupported("member/index assignment targets", self.diag_span(*span));
                    None
                }
            },
            ast::Stmt::Return { value, span } => {
                let value = match value {
                    Some(expr) => {
                        let v = self.check_expr(expr, Some(&self.ret.clone()), sink);
                        if !assignable(&v.ty, &self.ret) {
                            let mut d = build(
                                Level::Error,
                                codes::SEM015,
                                format!("return type mismatch in `{}`", self.fn_name),
                                self.diag_span(expr.span()),
                                Some(format!("this expression has type `{}`", v.ty.display())),
                            );
                            d.notes.push(format!(
                                "function declares return type `{}`",
                                self.ret.display()
                            ));
                            d.rendered = crate::diag::render_cli(&d);
                            sink.push(d);
                        }
                        Some(v)
                    }
                    None => {
                        if self.ret != Ty::Void {
                            sink.push(build(
                                Level::Warning,
                                codes::FUNC005,
                                format!(
                                    "empty return in `{}`, which declares return type `{}`",
                                    self.fn_name,
                                    self.ret.display()
                                ),
                                self.diag_span(*span),
                                None,
                            ));
                        }
                        None
                    }
                };
                Some(TStmt::Return { value, span: *span })
            }
            ast::Stmt::Expr(expr) => Some(TStmt::Expr(self.check_expr(expr, None, sink))),
            ast::Stmt::If {
                cond,
                then,
                else_ifs,
                els,
                ..
            } => {
                let cond = self.check_condition(cond, sink);
                let then = self.check_block(then, sink);
                let else_ifs = else_ifs
                    .iter()
                    .map(|(c, b)| (self.check_condition(c, sink), self.check_block(b, sink)))
                    .collect();
                let els = els.as_ref().map(|b| self.check_block(b, sink));
                Some(TStmt::If {
                    cond,
                    then,
                    else_ifs,
                    els,
                })
            }
            ast::Stmt::Print { span, .. } => {
                sink.note_unsupported("print: blocks", self.diag_span(*span));
                None
            }
        }
    }

    fn check_condition(&mut self, expr: &ast::Expr, sink: &mut DiagnosticSink) -> TExpr {
        let cond = self.check_expr(expr, Some(&Ty::Boolean), sink);
        if cond.ty != Ty::Boolean && cond.ty != Ty::Error {
            // SEM023 — exact wording from Platform 10 §3.
            sink.push(build(
                Level::Error,
                codes::SEM023,
                format!(
                    "Condition must be a boolean expression, found {}",
                    cond.ty.display()
                ),
                self.diag_span(expr.span()),
                Some("expected boolean".to_string()),
            ));
        }
        cond
    }

    fn check_expr(
        &mut self,
        expr: &ast::Expr,
        expected: Option<&Ty>,
        sink: &mut DiagnosticSink,
    ) -> TExpr {
        let span = expr.span();
        match expr {
            ast::Expr::Int { value, .. } => {
                self.integer_literal(*value as i128, expected, span, sink)
            }
            ast::Expr::Unary {
                op: ast::UnOp::Neg,
                operand,
                ..
            } => {
                // SEM026: the range check applies after unary minus folds.
                if let ast::Expr::Int { value, .. } = operand.as_ref() {
                    return self.integer_literal(-(*value as i128), expected, span, sink);
                }
                let operand = self.check_expr(operand, Some(&Ty::Integer), sink);
                self.expect_integer(&operand, sink);
                TExpr {
                    ty: operand.ty.clone(),
                    span,
                    kind: TExprKind::Unary {
                        op: ast::UnOp::Neg,
                        operand: Box::new(operand),
                    },
                }
            }
            ast::Expr::Bool { value, .. } => TExpr {
                ty: Ty::Boolean,
                span,
                kind: TExprKind::Bool(*value),
            },
            ast::Expr::NoneLit { .. } => match expected {
                Some(Ty::Option(_)) => TExpr {
                    ty: expected.unwrap().clone(),
                    span,
                    kind: TExprKind::NoneLit,
                },
                _ => {
                    sink.note_unsupported(
                        "`none` outside an optional context",
                        self.diag_span(span),
                    );
                    error_expr(span)
                }
            },
            ast::Expr::Str {
                value,
                interpolations,
                ..
            } => {
                if !interpolations.is_empty() {
                    sink.note_unsupported("string interpolation", self.diag_span(span));
                    return error_expr(span);
                }
                if let Some(Ty::Enum { wit_name, cases }) = expected {
                    // ADR-0002 §3: an enum-typed parameter takes a
                    // compile-time string literal naming a case.
                    return match cases.iter().position(|c| c == value) {
                        Some(index) => TExpr {
                            ty: expected.unwrap().clone(),
                            span,
                            kind: TExprKind::EnumCase(index as u32),
                        },
                        None => {
                            let mut d = build(
                                Level::Error,
                                codes::SEM016,
                                format!("`\"{value}\"` is not a case of enum `{wit_name}`"),
                                self.diag_span(span),
                                Some(format!("expected one of: {}", cases.join(", "))),
                            );
                            d.rendered = crate::diag::render_cli(&d);
                            sink.push(d);
                            error_expr(span)
                        }
                    };
                }
                TExpr {
                    ty: Ty::Str,
                    span,
                    kind: TExprKind::Str(value.clone()),
                }
            }
            ast::Expr::Number { .. } => {
                sink.note_unsupported("number literals", self.diag_span(span));
                error_expr(span)
            }
            ast::Expr::List { .. } => {
                sink.note_unsupported("list literals", self.diag_span(span));
                error_expr(span)
            }
            ast::Expr::Ident { name, .. } => match self.lookup(name) {
                Some(local) => TExpr {
                    ty: self.locals[local].ty.clone(),
                    span,
                    kind: TExprKind::Local(local),
                },
                None => {
                    sink.push(build(
                        Level::Error,
                        codes::SEM002,
                        format!("I cannot find a variable named `{name}` in scope"),
                        self.diag_span(span),
                        Some("no variable with this name exists here".to_string()),
                    ));
                    error_expr(span)
                }
            },
            ast::Expr::Call { callee, args, .. } => {
                self.check_call(callee, args, expected, span, sink)
            }
            ast::Expr::Member { .. } => {
                sink.note_unsupported("member access", self.diag_span(span));
                error_expr(span)
            }
            ast::Expr::Unary {
                op: ast::UnOp::Not,
                operand,
                ..
            } => {
                let operand = self.check_expr(operand, Some(&Ty::Boolean), sink);
                if operand.ty != Ty::Boolean && operand.ty != Ty::Error {
                    self.invalid_op(sink, "not", &operand.ty, span);
                }
                TExpr {
                    ty: Ty::Boolean,
                    span,
                    kind: TExprKind::Unary {
                        op: ast::UnOp::Not,
                        operand: Box::new(operand),
                    },
                }
            }
            ast::Expr::Binary { op, lhs, rhs, .. } => self.check_binary(*op, lhs, rhs, span, sink),
        }
    }

    fn integer_literal(
        &mut self,
        value: i128,
        expected: Option<&Ty>,
        span: ByteSpan,
        sink: &mut DiagnosticSink,
    ) -> TExpr {
        let ty = match expected {
            Some(t) if t.is_integer() => t.clone(),
            _ => Ty::Integer,
        };
        if let Some((min, max)) = ty.integer_range() {
            if value < min || value > max {
                // SEM026 — template from Platform 10 §3.
                sink.push(build(
                    Level::Error,
                    codes::SEM026,
                    format!(
                        "literal {value} does not fit {} (range {min} to {max})",
                        ty.display()
                    ),
                    self.diag_span(span),
                    Some(format!("out of range for {}", ty.display())),
                ));
                return error_expr(span);
            }
        }
        TExpr {
            ty,
            span,
            kind: TExprKind::Int(value),
        }
    }

    fn check_call(
        &mut self,
        callee: &ast::Expr,
        args: &[ast::Expr],
        expected: Option<&Ty>,
        span: ByteSpan,
        sink: &mut DiagnosticSink,
    ) -> TExpr {
        let ast::Expr::Ident {
            name,
            span: callee_span,
        } = callee
        else {
            sink.note_unsupported("method-style calls", self.diag_span(callee.span()));
            return error_expr(span);
        };

        // A local variable is not callable (FUNC003).
        if self.lookup(name).is_some() {
            sink.push(build(
                Level::Error,
                codes::FUNC003,
                format!("`{name}` is not a function and cannot be called"),
                self.diag_span(*callee_span),
                None,
            ));
            return error_expr(span);
        }

        // User function?
        if let Some(index) = self.outer.resolved.decls.functions.get_index_of(name) {
            let (params, ret): (Vec<Ty>, Ty) = {
                let sig = &self.outer.function_sigs[index];
                (
                    sig.params.iter().map(|p| p.ty.clone()).collect(),
                    sig.ret.clone(),
                )
            };
            let args = self.check_args(name, &params, args, span, sink);
            return TExpr {
                ty: ret,
                span,
                kind: TExprKind::CallFn { func: index, args },
            };
        }

        // Host function?
        if let Some(index) = self
            .outer
            .host_imports
            .iter()
            .position(|h| h.clean_name == *name)
        {
            let (params, ret) = {
                let import = &self.outer.host_imports[index];
                (import.params.clone(), import.ret.clone())
            };
            let args = self.check_args(name, &params, args, span, sink);
            return TExpr {
                ty: ret,
                span,
                kind: TExprKind::CallHost {
                    import: index,
                    args,
                },
            };
        }

        // Class constructor? (`Options(false)` — ADR-0002 §3.)
        if let Some(record) = self.outer.class_records.get(name).cloned() {
            let Ty::Record { fields, .. } = &record else {
                unreachable!("class projections are records")
            };
            if let Some(expected_record @ Ty::Record { .. }) = expected {
                if !assignable(&record, expected_record) {
                    let mut d = build(
                        Level::Error,
                        codes::SEM016,
                        format!(
                            "class `{name}` does not match record `{}`",
                            expected_record.display()
                        ),
                        self.diag_span(span),
                        Some("field names and types must match the WIT record".to_string()),
                    );
                    d.rendered = crate::diag::render_cli(&d);
                    sink.push(d);
                }
            }
            let params: Vec<Ty> = fields.iter().map(|(_, t)| t.clone()).collect();
            let args = self.check_args(name, &params, args, span, sink);
            return TExpr {
                ty: record,
                span,
                kind: TExprKind::MakeRecord(args),
            };
        }

        // SEM019 — exact wording from Platform 10 §3.
        sink.push(build(
            Level::Error,
            codes::SEM019,
            format!("I cannot find a function named `{name}`"),
            self.diag_span(*callee_span),
            Some("no function with this name is in scope".to_string()),
        ));
        error_expr(span)
    }

    fn check_args(
        &mut self,
        fn_name: &str,
        params: &[Ty],
        args: &[ast::Expr],
        call_span: ByteSpan,
        sink: &mut DiagnosticSink,
    ) -> Vec<TExpr> {
        if args.len() != params.len() {
            sink.push(build(
                Level::Error,
                codes::FUNC002,
                format!(
                    "`{fn_name}` expects {} argument(s), got {}",
                    params.len(),
                    args.len()
                ),
                self.diag_span(call_span),
                None,
            ));
        }
        args.iter()
            .enumerate()
            .map(|(i, arg)| {
                let expected = params.get(i);
                let value = self.check_expr(arg, expected, sink);
                if let Some(param_ty) = expected {
                    if !assignable(&value.ty, param_ty) {
                        // SEM016 — headline template from Platform 10 §3.
                        let mut d = build(
                            Level::Error,
                            codes::SEM016,
                            format!("argument `{}` of `{fn_name}` has the wrong type", i + 1),
                            self.diag_span(arg.span()),
                            Some(format!("this argument has type `{}`", value.ty.display())),
                        );
                        d.notes.push(format!(
                            "the parameter is declared with type `{}`",
                            param_ty.display()
                        ));
                        d.rendered = crate::diag::render_cli(&d);
                        sink.push(d);
                    }
                }
                value
            })
            .collect()
    }

    fn check_binary(
        &mut self,
        op: ast::BinOp,
        lhs: &ast::Expr,
        rhs: &ast::Expr,
        span: ByteSpan,
        sink: &mut DiagnosticSink,
    ) -> TExpr {
        use ast::BinOp::*;
        let (lhs, rhs, ty) = match op {
            Add | Sub | Mul | Div | Rem => {
                let l = self.check_expr(lhs, Some(&Ty::Integer), sink);
                let r = self.check_expr(rhs, Some(&Ty::Integer), sink);
                self.expect_integer(&l, sink);
                self.expect_integer(&r, sink);
                (l, r, Ty::Integer)
            }
            Pow => {
                sink.note_unsupported("exponentiation", self.diag_span(span));
                return error_expr(span);
            }
            Lt | LtEq | Gt | GtEq => {
                let l = self.check_expr(lhs, Some(&Ty::Integer), sink);
                let r = self.check_expr(rhs, Some(&Ty::Integer), sink);
                self.expect_integer(&l, sink);
                self.expect_integer(&r, sink);
                (l, r, Ty::Boolean)
            }
            Eq | NEq => {
                let l = self.check_expr(lhs, None, sink);
                let r = self.check_expr(rhs, Some(&l.ty.clone()), sink);
                let comparable = (l.ty.is_integer() && r.ty.is_integer())
                    || (l.ty == Ty::Boolean && r.ty == Ty::Boolean)
                    || l.ty == Ty::Error
                    || r.ty == Ty::Error;
                if !comparable {
                    if l.ty == Ty::Str && r.ty == Ty::Str {
                        sink.note_unsupported("string equality", self.diag_span(span));
                    } else {
                        self.invalid_op(sink, if op == Eq { "==" } else { "!=" }, &l.ty, span);
                    }
                    return error_expr(span);
                }
                (l, r, Ty::Boolean)
            }
            And | Or => {
                let l = self.check_expr(lhs, Some(&Ty::Boolean), sink);
                let r = self.check_expr(rhs, Some(&Ty::Boolean), sink);
                for side in [&l, &r] {
                    if side.ty != Ty::Boolean && side.ty != Ty::Error {
                        self.invalid_op(sink, if op == And { "and" } else { "or" }, &side.ty, span);
                    }
                }
                (l, r, Ty::Boolean)
            }
        };
        TExpr {
            ty,
            span,
            kind: TExprKind::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            },
        }
    }

    fn expect_integer(&self, expr: &TExpr, sink: &mut DiagnosticSink) {
        if !expr.ty.is_integer() && expr.ty != Ty::Error {
            sink.push(build(
                Level::Error,
                codes::SEM004,
                format!("arithmetic is not defined for type `{}`", expr.ty.display()),
                self.diag_span(expr.span),
                None,
            ));
        }
    }

    fn invalid_op(&self, sink: &mut DiagnosticSink, op: &str, ty: &Ty, span: ByteSpan) {
        sink.push(build(
            Level::Error,
            codes::SEM004,
            format!("operator `{op}` is not defined for type `{}`", ty.display()),
            self.diag_span(span),
            None,
        ));
    }
}

fn error_expr(span: ByteSpan) -> TExpr {
    TExpr {
        ty: Ty::Error,
        span,
        kind: TExprKind::Error,
    }
}

/// Assignability for the M1 surface: exact match, integer↔width (the range
/// crosses the boundary under LBS-02's runtime check), `T` into `T?`, and
/// records structurally. `Error` absorbs everything.
fn assignable(from: &Ty, to: &Ty) -> bool {
    match (from, to) {
        (Ty::Error, _) | (_, Ty::Error) => true,
        (a, b) if a == b => true,
        (a, b) if a.is_integer() && b.is_integer() => true,
        (from, Ty::Option(inner)) => assignable(from, inner),
        (
            Ty::Record {
                wit_name: a,
                fields: fa,
            },
            Ty::Record {
                wit_name: b,
                fields: fb,
            },
        ) => {
            a == b
                && fa.len() == fb.len()
                && fa
                    .iter()
                    .zip(fb)
                    .all(|((na, ta), (nb, tb))| na == nb && assignable(ta, tb))
        }
        _ => false,
    }
}
