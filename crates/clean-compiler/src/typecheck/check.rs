//! Pass [5] — Type Check (M4): bidirectional checking over the resolved
//! AST with an `ena` inference context (`infer::InferCtx`). Checking mode
//! pushes the expected type into literals; synthesis mode types everything
//! else; the single `fit` relation owns assignability, and every implicit
//! conversion it grants is materialised as a TIR coercion node.
//!
//! Message wording comes verbatim from Platform 10 §3/§5 where a template
//! exists; stub rules (RUL-03) carry locally-adopted wording pinned by
//! their DIA-06 fixtures and recorded in docs/DISCOVERIES-M4.md. Error
//! types absorb downstream mismatches so one mistake reports once
//! (§14.4.2[5]).

use std::collections::HashSet;

use clean_compiler_types::{codes, Annotation, Level};
use indexmap::IndexMap;
use wit_parser::{WorldItem, WorldKey};

use crate::codegen::world::ParsedWorld;
use crate::diag::{build, DiagnosticSink};
use crate::parser::ast;
use crate::resolver::ResolvedAst;
use crate::source::ByteSpan;

use super::infer::{Fit, InferCtx};
use super::tir::*;
use super::types::{kebab, ListBehavior, Removal, Ty};

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

/// Which grammatical position a type expression sits in: host-function
/// signatures admit width suffixes and world-declared names; everywhere
/// else a width is SEM009 (TYP-01).
#[derive(Clone, Copy, PartialEq, Eq)]
enum TyPos {
    Surface,
    Host,
}

impl<'a> Checker<'a> {
    fn span(&self, file: usize, span: ByteSpan) -> clean_compiler_types::Span {
        self.resolved.span(file, span)
    }

    // ----- projections -------------------------------------------------

    fn project_classes(&mut self, sink: &mut DiagnosticSink) {
        for coords in self.resolved.decls.classes.values() {
            let (class, file) = self.resolved.class(*coords);
            // The LBS-02 record projection covers plain field bags; richer
            // class features land in the M4 class stage.
            if class.parent.is_some() || !class.capabilities.is_empty() {
                sink.note_unsupported(
                    "class inheritance and capability claims",
                    self.span(file, class.span),
                );
            }
            if class.always.is_some() {
                sink.note_unsupported("always: invariants", self.span(file, class.span));
            }
            if !class.constructors.is_empty() || !class.functions.is_empty() {
                sink.note_unsupported(
                    "class constructors and methods",
                    self.span(file, class.span),
                );
            }
            let mut fields = Vec::new();
            for field in &class.fields {
                if field.init.is_some() {
                    sink.note_unsupported("field initialisers", self.span(file, field.span));
                }
                let ty = self.project_type(&field.ty, TyPos::Surface, file, sink);
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
            // FNC-04: input-block parameters are equivalent to
            // ParameterList entries — they extend the signature.
            let params = f
                .params
                .iter()
                .chain(&f.body.input)
                .map(|p| Local {
                    name: p.name.clone(),
                    ty: self.project_type(&p.ty, TyPos::Surface, file, sink),
                })
                .collect();
            let ret = self.project_type(&f.ret, TyPos::Surface, file, sink);
            self.function_sigs.push(FunctionSig {
                name: f.name.clone(),
                params,
                ret,
            });
        }
    }

    /// SEM009 — InvalidTypeSpecification (stub rule; local wording pinned
    /// by the DIA-06 fixture).
    fn sem009(
        &self,
        sink: &mut DiagnosticSink,
        message: String,
        label: &str,
        file: usize,
        span: ByteSpan,
    ) {
        sink.push(build(
            Level::Error,
            codes::SEM009,
            message,
            self.span(file, span),
            Some(label.to_string()),
        ));
    }

    /// Projects one written type expression, reporting every TYP-01/03/05
    /// structural violation as SEM009.
    fn project_type(
        &self,
        ty: &ast::TypeExpr,
        pos: TyPos,
        file: usize,
        sink: &mut DiagnosticSink,
    ) -> Ty {
        let mut errored = false;
        let base = match &ty.base {
            ast::BaseType::Boolean => Ty::Boolean,
            ast::BaseType::Integer(None) => Ty::Integer,
            ast::BaseType::Integer(Some(width)) => {
                if pos == TyPos::Host {
                    Ty::IntegerW(*width)
                } else {
                    // TYP-01: no width or signedness modifiers — widths
                    // exist only inside host function declarations.
                    self.sem009(
                        sink,
                        "integer widths exist only in host function signatures".to_string(),
                        "not a Clean type",
                        file,
                        ty.span,
                    );
                    errored = true;
                    Ty::Error
                }
            }
            ast::BaseType::Number => Ty::Number,
            ast::BaseType::String_ => Ty::Str,
            ast::BaseType::Bytes => Ty::Bytes,
            ast::BaseType::Datetime => Ty::Datetime,
            ast::BaseType::Any => Ty::Any,
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
                    errored = true;
                    Ty::Error
                }
            },
            ast::BaseType::List(element) => {
                let elem = self.project_type(element, pos, file, sink);
                let behavior = self.project_behaviors(&ty.behaviors, file, sink);
                Ty::List(Box::new(elem), behavior)
            }
            ast::BaseType::Matrix(element) => {
                Ty::Matrix(Box::new(self.project_type(element, pos, file, sink)))
            }
            ast::BaseType::Pairs(key, value) => Ty::Pairs(
                Box::new(self.project_type(key, pos, file, sink)),
                Box::new(self.project_type(value, pos, file, sink)),
            ),
        };
        // TYP-03: a single `?` only — absence does not stack; `T??` written
        // in source is SEM009.
        if let Some(extra) = ty.extra_optionals.first() {
            self.sem009(
                sink,
                "the optional marker `?` cannot be repeated".to_string(),
                "absence does not stack",
                file,
                *extra,
            );
            errored = true;
        }
        if errored {
            return Ty::Error;
        }
        if ty.optional {
            Ty::Option(Box::new(base))
        } else {
            base
        }
    }

    /// TYP-05: one removal discipline (`.line` xor `.pile`), `.unique`
    /// independent, no repeats. The grammar admits any chain; the checker
    /// restricts (SEM009).
    fn project_behaviors(
        &self,
        behaviors: &[ast::Behavior],
        file: usize,
        sink: &mut DiagnosticSink,
    ) -> ListBehavior {
        let mut out = ListBehavior::NONE;
        for b in behaviors {
            match b.name {
                ast::BehaviorName::Line | ast::BehaviorName::Pile => {
                    let removal = if b.name == ast::BehaviorName::Line {
                        Removal::Line
                    } else {
                        Removal::Pile
                    };
                    if out.removal.is_some() {
                        self.sem009(
                            sink,
                            "a list takes at most one removal discipline (`.line` or `.pile`)"
                                .to_string(),
                            "second removal discipline",
                            file,
                            b.span,
                        );
                    } else {
                        out.removal = Some(removal);
                    }
                }
                ast::BehaviorName::Unique => {
                    if out.unique {
                        self.sem009(
                            sink,
                            "`.unique` is already applied to this list".to_string(),
                            "repeated behavior",
                            file,
                            b.span,
                        );
                    } else {
                        out.unique = true;
                    }
                }
            }
        }
        out
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
        self.project_type(ty, TyPos::Host, file, sink)
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
            return super::types::project_wit(resolve, &wit_parser::Type::Id(*type_id));
        }
        None
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
            if f.background {
                sink.note_unsupported("background functions", self.span(file, f.span));
            }
            if f.body.before.is_some() || f.body.after.is_some() {
                sink.note_unsupported("contract blocks", self.span(file, f.span));
            }
            let mut body_checker = BodyChecker {
                outer: self,
                file,
                locals: sig.params.clone(),
                scopes: vec![scope],
                ret: sig.ret.clone(),
                fn_name: sig.name.clone(),
                infcx: InferCtx::new(),
                loop_depth: 0,
                known_none: HashSet::new(),
            };
            let body = body_checker.check_block(&f.body.statements, sink);
            let mut function = TFunction {
                name: sig.name.clone(),
                params: sig.params.clone(),
                ret: sig.ret.clone(),
                locals: body_checker.locals,
                body,
                span: f.span,
                file,
            };
            // No `Ty::Var` leaves pass [5]: collapse what stayed
            // unconstrained to `any` (TYP-02).
            finalize_function(&mut body_checker.infcx, &mut function);
            out.push(function);
        }
        // `start:` blocks (FNC-01) check as parameterless void bodies.
        // They are not callable by name, so appending them after the
        // declared functions keeps `CallFn` indices stable.
        for coords in self.resolved.decls.starts.clone() {
            let (block, file) = self.resolved.start(coords);
            let span = block
                .first()
                .map(stmt_span)
                .unwrap_or(ByteSpan { start: 0, end: 0 });
            let mut body_checker = BodyChecker {
                outer: self,
                file,
                locals: Vec::new(),
                scopes: vec![IndexMap::new()],
                ret: Ty::Void,
                fn_name: "start".to_string(),
                infcx: InferCtx::new(),
                loop_depth: 0,
                known_none: HashSet::new(),
            };
            let body = body_checker.check_block(block, sink);
            let mut function = TFunction {
                name: "start".to_string(),
                params: Vec::new(),
                ret: Ty::Void,
                locals: body_checker.locals,
                body,
                span,
                file,
            };
            finalize_function(&mut body_checker.infcx, &mut function);
            out.push(function);
        }
        out
    }
}

/// The source span of a statement (for anchoring `start:` bodies).
fn stmt_span(stmt: &ast::Stmt) -> ByteSpan {
    use ast::Stmt::*;
    match stmt {
        VarDecl { span, .. }
        | Assign { span, .. }
        | Return { span, .. }
        | If { span, .. }
        | Iterate { span, .. }
        | While { span, .. }
        | Break { span }
        | Continue { span }
        | Print { span, .. }
        | Assert { span, .. }
        | Apply { span, .. }
        | Later { span, .. }
        | Background { span, .. }
        | Reset { span, .. } => *span,
        Expr { expr, .. } => expr.span(),
    }
}

/// Deep-resolves every type in the function through the inference table.
fn finalize_function(infcx: &mut InferCtx, f: &mut TFunction) {
    for local in &mut f.locals {
        local.ty = infcx.finalize(&local.ty);
    }
    f.ret = infcx.finalize(&f.ret);
    for stmt in &mut f.body {
        finalize_stmt(infcx, stmt);
    }
}

fn finalize_stmt(infcx: &mut InferCtx, stmt: &mut TStmt) {
    match stmt {
        TStmt::Let { init, .. } => {
            if let Some(init) = init {
                finalize_expr(infcx, init);
            }
        }
        TStmt::Assign { value, .. } => finalize_expr(infcx, value),
        TStmt::Return { value, .. } => {
            if let Some(value) = value {
                finalize_expr(infcx, value);
            }
        }
        TStmt::Expr(expr) => finalize_expr(infcx, expr),
        TStmt::If {
            cond,
            then,
            else_ifs,
            els,
        } => {
            finalize_expr(infcx, cond);
            then.iter_mut().for_each(|s| finalize_stmt(infcx, s));
            for (c, b) in else_ifs {
                finalize_expr(infcx, c);
                b.iter_mut().for_each(|s| finalize_stmt(infcx, s));
            }
            if let Some(els) = els {
                els.iter_mut().for_each(|s| finalize_stmt(infcx, s));
            }
        }
        TStmt::While { cond, body } => {
            finalize_expr(infcx, cond);
            body.iter_mut().for_each(|s| finalize_stmt(infcx, s));
        }
        TStmt::Iterate {
            source, step, body, ..
        } => {
            match source {
                TIterSource::List(e) | TIterSource::Chars(e) | TIterSource::Rows(e) => {
                    finalize_expr(infcx, e)
                }
                TIterSource::Range { from, to } => {
                    finalize_expr(infcx, from);
                    finalize_expr(infcx, to);
                }
            }
            if let Some(step) = step {
                finalize_expr(infcx, step);
            }
            body.iter_mut().for_each(|s| finalize_stmt(infcx, s));
        }
        TStmt::Break { .. } | TStmt::Continue { .. } => {}
        TStmt::Print { items, .. } => items.iter_mut().for_each(|e| finalize_expr(infcx, e)),
        TStmt::Assert { cond, .. } => finalize_expr(infcx, cond),
    }
}

fn finalize_expr(infcx: &mut InferCtx, expr: &mut TExpr) {
    expr.ty = infcx.finalize(&expr.ty);
    match &mut expr.kind {
        TExprKind::MakeRecord(items)
        | TExprKind::MakeList(items)
        | TExprKind::MakeMatrix(items)
        | TExprKind::CallHost { args: items, .. }
        | TExprKind::CallFn { args: items, .. } => {
            items.iter_mut().for_each(|e| finalize_expr(infcx, e))
        }
        TExprKind::Binary { lhs, rhs, .. } => {
            finalize_expr(infcx, lhs);
            finalize_expr(infcx, rhs);
        }
        TExprKind::Unary { operand, .. }
        | TExprKind::NonNone(operand)
        | TExprKind::IsNone { operand, .. }
        | TExprKind::IntToNumber(operand)
        | TExprKind::WrapSome(operand) => finalize_expr(infcx, operand),
        TExprKind::Index { recv, index, .. } => {
            finalize_expr(infcx, recv);
            finalize_expr(infcx, index);
        }
        TExprKind::StrInterp(segs) => {
            for seg in segs {
                if let TInterpSeg::Expr(e) = seg {
                    finalize_expr(infcx, e);
                }
            }
        }
        TExprKind::Int(_)
        | TExprKind::Num(_)
        | TExprKind::Bool(_)
        | TExprKind::Str(_)
        | TExprKind::NoneLit
        | TExprKind::EnumCase(_)
        | TExprKind::Local(_)
        | TExprKind::Error => {}
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
    infcx: InferCtx,
    /// Nesting depth of enclosing `iterate`/`while` loops in this body
    /// (FLW-03 / SEM025).
    loop_depth: usize,
    /// Locals provably `none` at this program point (straight-line
    /// tracking; any control-flow join clears it) — the IDX005 analysis.
    /// Never iterated, so ordering cannot leak into output (CMP-02).
    known_none: HashSet<LocalId>,
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

    fn project_type(&self, ty: &ast::TypeExpr, sink: &mut DiagnosticSink) -> Ty {
        self.outer.project_type(ty, TyPos::Surface, self.file, sink)
    }

    /// Pushes a diagnostic that carries secondary annotations or notes,
    /// re-rendering it first (the sink's `build` renders source-less).
    fn push_rich(&self, sink: &mut DiagnosticSink, mut d: clean_compiler_types::Diagnostic) {
        d.rendered = crate::diag::render_cli(&d, &crate::diag::SourceCache::empty());
        sink.push(d);
    }

    // ----- coercion ------------------------------------------------------

    /// Applies the `fit` verdict, materialising promotions and option
    /// wraps. `Err` returns the original value: the caller reports.
    fn coerce(&mut self, value: TExpr, to: &Ty) -> Result<TExpr, TExpr> {
        match self.infcx.fit(&value.ty, to) {
            Fit::Exact => Ok(value),
            Fit::Promote => Ok(promote(value)),
            Fit::Wrap { promote: p } => {
                let inner = if p { promote(value) } else { value };
                let span = inner.span;
                Ok(TExpr {
                    ty: self.infcx.resolve(to),
                    span,
                    kind: TExprKind::WrapSome(Box::new(inner)),
                })
            }
            Fit::No => Err(value),
        }
    }

    /// Coerce-or-SEM001 for assignment shapes (variable declarations,
    /// assignments): primary on the name, RHS secondary (Platform 10 §3,
    /// DISCOVERIES-M2 item 7).
    fn coerce_assign(
        &mut self,
        value: TExpr,
        declared: &Ty,
        name: &str,
        name_span: ByteSpan,
        sink: &mut DiagnosticSink,
    ) -> TExpr {
        match self.coerce(value, declared) {
            Ok(value) => value,
            Err(value) => {
                let mut d = build(
                    Level::Error,
                    codes::SEM001,
                    "type mismatch in assignment".to_string(),
                    self.diag_span(name_span),
                    Some(format!(
                        "`{name}` is declared with type `{}`",
                        declared.display()
                    )),
                );
                d.secondary.push(Annotation {
                    span: self.diag_span(value.span),
                    label: format!(
                        "this expression has type `{}`",
                        self.infcx.resolve(&value.ty).display()
                    ),
                });
                self.push_rich(sink, d);
                error_expr(value.span)
            }
        }
    }

    // ----- statements ----------------------------------------------------

    fn check_block(&mut self, block: &[ast::Stmt], sink: &mut DiagnosticSink) -> Vec<TStmt> {
        self.scopes.push(IndexMap::new());
        let out = block
            .iter()
            .filter_map(|stmt| self.check_stmt(stmt, sink))
            .collect();
        self.scopes.pop();
        out
    }

    /// A loop body: new scope (optionally seeded with the iterate binder),
    /// `known_none` cleared on entry and exit (the back edge is a join).
    fn check_loop_body(
        &mut self,
        binder: Option<(String, LocalId)>,
        block: &[ast::Stmt],
        sink: &mut DiagnosticSink,
    ) -> Vec<TStmt> {
        self.known_none.clear();
        self.loop_depth += 1;
        self.scopes.push(IndexMap::new());
        if let Some((name, id)) = binder {
            self.scopes.last_mut().unwrap().insert(name, id);
        }
        let out = block
            .iter()
            .filter_map(|stmt| self.check_stmt(stmt, sink))
            .collect();
        self.scopes.pop();
        self.loop_depth -= 1;
        self.known_none.clear();
        out
    }

    fn check_stmt(&mut self, stmt: &ast::Stmt, sink: &mut DiagnosticSink) -> Option<TStmt> {
        match stmt {
            ast::Stmt::VarDecl {
                ty,
                name,
                name_span,
                init,
                on_error,
                span,
            } => {
                if on_error.is_some() {
                    sink.note_unsupported("`onError:` block handlers", self.diag_span(*span));
                }
                let declared = self.project_type(ty, sink);
                if self.scopes.last().unwrap().contains_key(name) {
                    sink.push(build(
                        Level::Error,
                        codes::SCOPE002,
                        format!("variable `{name}` cannot be redeclared in the same scope"),
                        self.diag_span(*span),
                        None,
                    ));
                }
                let id = self.locals.len();
                let init = init.as_ref().map(|expr| {
                    let value = self.check_expr(expr, Some(&declared), sink);
                    if matches!(value.kind, TExprKind::NoneLit) {
                        self.known_none.insert(id);
                    }
                    self.coerce_assign(value, &declared, name, *name_span, sink)
                });
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
                on_error,
                span,
            } => {
                if on_error.is_some() {
                    sink.note_unsupported("`onError:` block handlers", self.diag_span(*span));
                }
                match target {
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
                        if matches!(value.kind, TExprKind::NoneLit) {
                            self.known_none.insert(local);
                        } else {
                            self.known_none.remove(&local);
                        }
                        let value = self.coerce_assign(value, &declared, name, *name_span, sink);
                        Some(TStmt::Assign { local, value })
                    }
                    _ => {
                        sink.note_unsupported(
                            "member/index assignment targets",
                            self.diag_span(*span),
                        );
                        None
                    }
                }
            }
            ast::Stmt::Return { value, span } => {
                let value = match value {
                    Some(expr) => {
                        let ret = self.ret.clone();
                        let v = self.check_expr(expr, Some(&ret), sink);
                        match self.coerce(v, &ret) {
                            Ok(v) => Some(v),
                            Err(v) => {
                                let mut d = build(
                                    Level::Error,
                                    codes::SEM015,
                                    format!("return type mismatch in `{}`", self.fn_name),
                                    self.diag_span(expr.span()),
                                    Some(format!(
                                        "this expression has type `{}`",
                                        self.infcx.resolve(&v.ty).display()
                                    )),
                                );
                                d.notes.push(format!(
                                    "function declares return type `{}`",
                                    self.ret.display()
                                ));
                                self.push_rich(sink, d);
                                Some(error_expr(expr.span()))
                            }
                        }
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
            ast::Stmt::Expr { expr, on_error } => {
                if on_error.is_some() {
                    sink.note_unsupported("`onError:` block handlers", self.diag_span(expr.span()));
                }
                Some(TStmt::Expr(self.check_expr(expr, None, sink)))
            }
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
                // The join loses all definite-`none` knowledge (IDX005
                // fires only on provable-on-all-paths facts).
                self.known_none.clear();
                Some(TStmt::If {
                    cond,
                    then,
                    else_ifs,
                    els,
                })
            }
            ast::Stmt::While { cond, body, .. } => {
                let cond = self.check_condition(cond, sink);
                let body = self.check_loop_body(None, body, sink);
                Some(TStmt::While { cond, body })
            }
            ast::Stmt::Iterate {
                binder,
                binder_span: _,
                source,
                step,
                body,
                span: _,
            } => {
                let (source, binder_ty) = self.check_iterate_source(source, sink);
                let step = step.as_ref().map(|s| {
                    let v = self.check_expr(s, Some(&Ty::Integer), sink);
                    if !self.is_integerish(&v.ty) {
                        let resolved = self.infcx.resolve(&v.ty);
                        self.invalid_op(sink, "step", &resolved, v.span);
                    }
                    v
                });
                let id = self.locals.len();
                self.locals.push(Local {
                    name: binder.clone(),
                    ty: binder_ty,
                });
                let body = self.check_loop_body(Some((binder.clone(), id)), body, sink);
                Some(TStmt::Iterate {
                    binder: id,
                    source,
                    step,
                    body,
                })
            }
            ast::Stmt::Break { span } => {
                self.control_flow_outside_loop("break", *span, sink);
                Some(TStmt::Break { span: *span })
            }
            ast::Stmt::Continue { span } => {
                self.control_flow_outside_loop("continue", *span, sink);
                Some(TStmt::Continue { span: *span })
            }
            ast::Stmt::Print { items, span } => {
                let items = items
                    .iter()
                    .map(|item| {
                        let v = self.check_expr(item, None, sink);
                        self.expect_textable(&v, "printed", sink);
                        v
                    })
                    .collect();
                Some(TStmt::Print { items, span: *span })
            }
            ast::Stmt::Assert { expr, span } => {
                let cond = self.check_condition(expr, sink);
                Some(TStmt::Assert { cond, span: *span })
            }
            ast::Stmt::Apply { span, .. } => {
                sink.note_unsupported("apply-blocks", self.diag_span(*span));
                None
            }
            ast::Stmt::Later {
                ty,
                name,
                name_span: _,
                span,
                ..
            } => {
                sink.note_unsupported("later bindings", self.diag_span(*span));
                // Register the binding anyway (ASY-01 declares a typed
                // name) so later references resolve instead of producing a
                // spurious SEM002 next to the unsupported note.
                let declared = self.project_type(ty, sink);
                let id = self.locals.len();
                self.locals.push(Local {
                    name: name.clone(),
                    ty: declared,
                });
                self.scopes.last_mut().unwrap().insert(name.clone(), id);
                None
            }
            ast::Stmt::Background { span, .. } => {
                sink.note_unsupported("background calls", self.diag_span(*span));
                None
            }
            ast::Stmt::Reset { span, .. } => {
                sink.note_unsupported("reset statements", self.diag_span(*span));
                None
            }
        }
    }

    /// SEM025 — ControlFlowOutsideLoop (template from Platform 10 §3).
    fn control_flow_outside_loop(
        &mut self,
        keyword: &str,
        span: ByteSpan,
        sink: &mut DiagnosticSink,
    ) {
        if self.loop_depth == 0 {
            sink.push(build(
                Level::Error,
                codes::SEM025,
                format!("'{keyword}' is not inside a loop"),
                self.diag_span(span),
                Some("no enclosing 'iterate' or 'while'".to_string()),
            ));
        }
    }

    fn check_iterate_source(
        &mut self,
        source: &ast::IterateSource,
        sink: &mut DiagnosticSink,
    ) -> (TIterSource, Ty) {
        match source {
            ast::IterateSource::Range { from, to } => {
                let from = self.check_expr(from, Some(&Ty::Integer), sink);
                let to = self.check_expr(to, Some(&Ty::Integer), sink);
                for (ty, end_span) in [(from.ty.clone(), from.span), (to.ty.clone(), to.span)] {
                    if !self.is_integerish(&ty) {
                        let resolved = self.infcx.resolve(&ty);
                        self.invalid_op(sink, "to", &resolved, end_span);
                    }
                }
                (TIterSource::Range { from, to }, Ty::Integer)
            }
            ast::IterateSource::Expr(expr) => {
                let value = self.check_expr(expr, None, sink);
                let resolved = self.infcx.resolve(&value.ty);
                match resolved {
                    Ty::List(elem, _) => {
                        let elem = (*elem).clone();
                        (TIterSource::List(value), elem)
                    }
                    Ty::Str => (TIterSource::Chars(value), Ty::Str),
                    Ty::Matrix(elem) => {
                        let row = Ty::list((*elem).clone());
                        (TIterSource::Rows(value), row)
                    }
                    Ty::Any => (TIterSource::List(value), Ty::Any),
                    Ty::Error => (TIterSource::List(value), Ty::Error),
                    other => {
                        // SEM004 (stub rule; local wording).
                        sink.push(build(
                            Level::Error,
                            codes::SEM004,
                            format!("cannot iterate over a value of type `{}`", other.display()),
                            self.diag_span(value.span),
                            Some(
                                "iterate takes a list, a string, a matrix, or a range".to_string(),
                            ),
                        ));
                        (TIterSource::List(value), Ty::Error)
                    }
                }
            }
        }
    }

    fn check_condition(&mut self, expr: &ast::Expr, sink: &mut DiagnosticSink) -> TExpr {
        let cond = self.check_expr(expr, Some(&Ty::Boolean), sink);
        let resolved = self.infcx.resolve(&cond.ty);
        if !matches!(resolved, Ty::Boolean | Ty::Error | Ty::Any) {
            // SEM023 — exact wording from Platform 10 §3. No truthiness
            // conversion: `0`, `""` and an empty list are not conditions.
            sink.push(build(
                Level::Error,
                codes::SEM023,
                format!(
                    "Condition must be a boolean expression, found {}",
                    resolved.display()
                ),
                self.diag_span(expr.span()),
                Some("expected boolean".to_string()),
            ));
        }
        cond
    }

    // ----- expressions ---------------------------------------------------

    fn is_integerish(&mut self, ty: &Ty) -> bool {
        let t = self.infcx.resolve(ty);
        t.is_integer() || matches!(t, Ty::Any | Ty::Error)
    }

    fn check_expr(
        &mut self,
        expr: &ast::Expr,
        expected: Option<&Ty>,
        sink: &mut DiagnosticSink,
    ) -> TExpr {
        let span = expr.span();
        let expected_resolved = expected.map(|t| self.infcx.resolve(t));
        let expected = expected_resolved.as_ref();
        match expr {
            ast::Expr::Int { value, .. } => {
                self.integer_literal(*value as i128, expected, span, sink)
            }
            ast::Expr::Number { text, .. } => TExpr {
                ty: Ty::Number,
                span,
                // The lexer guarantees a well-formed decimal literal.
                kind: TExprKind::Num(text.parse::<f64>().unwrap_or(0.0)),
            },
            ast::Expr::Unary {
                op: ast::UnOp::Neg,
                operand,
                ..
            } => {
                // SEM026: the range check applies after unary minus folds.
                match operand.as_ref() {
                    ast::Expr::Int { value, .. } => {
                        return self.integer_literal(-(*value as i128), expected, span, sink);
                    }
                    ast::Expr::Number { text, .. } => {
                        return TExpr {
                            ty: Ty::Number,
                            span,
                            kind: TExprKind::Num(-text.parse::<f64>().unwrap_or(0.0)),
                        };
                    }
                    _ => {}
                }
                let operand = self.check_expr(operand, expected.filter(|t| t.is_numeric()), sink);
                let ty = self.infcx.resolve(&operand.ty);
                if !ty.is_numeric() && !matches!(ty, Ty::Any | Ty::Error) {
                    self.invalid_op(sink, "-", &ty, span);
                }
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
            ast::Expr::NoneLit { .. } => {
                let ty = match expected {
                    Some(t @ Ty::Option(_)) => t.clone(),
                    _ => Ty::Option(Box::new(self.infcx.fresh())),
                };
                TExpr {
                    ty,
                    span,
                    kind: TExprKind::NoneLit,
                }
            }
            ast::Expr::Str { segments, .. } => self.check_str(segments, expected, span, sink),
            ast::Expr::List { items, .. } => self.check_list(items, expected, span, sink),
            ast::Expr::Ident { name, .. } => match self.lookup(name) {
                Some(local) => TExpr {
                    ty: self.locals[local].ty.clone(),
                    span,
                    kind: TExprKind::Local(local),
                },
                None => {
                    let is_callable = self.outer.resolved.decls.functions.contains_key(name)
                        || self
                            .outer
                            .host_imports
                            .iter()
                            .any(|h| h.clean_name == *name);
                    if is_callable {
                        // SYN010 — a standalone name resolving to a callable
                        // without an argument list; template verbatim from
                        // Platform 10 §2 (FNC-05: every call carries
                        // parentheses).
                        sink.push(build(
                            Level::Error,
                            codes::SYN010,
                            format!("Call to '{name}' is missing parentheses"),
                            self.diag_span(span),
                            Some("every call carries parentheses".to_string()),
                        ));
                    } else {
                        sink.push(build(
                            Level::Error,
                            codes::SEM002,
                            format!("I cannot find a variable named `{name}` in scope"),
                            self.diag_span(span),
                            Some("no variable with this name exists here".to_string()),
                        ));
                    }
                    error_expr(span)
                }
            },
            ast::Expr::Call { callee, args, .. } => {
                self.check_call(callee, args, expected, span, sink)
            }
            ast::Expr::Index {
                receiver, index, ..
            } => self.check_index(receiver, index, span, sink),
            ast::Expr::NonNone { operand, .. } => {
                let operand = self.check_expr(operand, None, sink);
                let resolved = self.infcx.resolve(&operand.ty);
                let ty = match resolved {
                    // EXP-03: `!` narrows `T?` to `T`; `RUN004` at runtime.
                    Ty::Option(inner) => *inner,
                    Ty::Any => Ty::Any,
                    Ty::Error => Ty::Error,
                    other => {
                        self.invalid_op(sink, "!", &other, span);
                        Ty::Error
                    }
                };
                TExpr {
                    ty,
                    span,
                    kind: TExprKind::NonNone(Box::new(operand)),
                }
            }
            ast::Expr::Member { .. } => {
                sink.note_unsupported("member access", self.diag_span(span));
                error_expr(span)
            }
            ast::Expr::OnError { .. } => {
                sink.note_unsupported("`onError` fallback", self.diag_span(span));
                error_expr(span)
            }
            ast::Expr::This { .. } => {
                sink.note_unsupported("`this`", self.diag_span(span));
                error_expr(span)
            }
            ast::Expr::Base { .. } => {
                sink.note_unsupported("`base`", self.diag_span(span));
                error_expr(span)
            }
            ast::Expr::ErrorRef { .. } => {
                sink.note_unsupported("`error` values", self.diag_span(span));
                error_expr(span)
            }
            ast::Expr::ResultRef { .. } => {
                sink.note_unsupported("`result` in contracts", self.diag_span(span));
                error_expr(span)
            }
            ast::Expr::Unary {
                op: ast::UnOp::Not,
                operand,
                ..
            } => {
                let operand = self.check_expr(operand, Some(&Ty::Boolean), sink);
                let resolved = self.infcx.resolve(&operand.ty);
                if !matches!(resolved, Ty::Boolean | Ty::Error | Ty::Any) {
                    self.invalid_op(sink, "not", &resolved, span);
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

    /// String literal: enum-case projection (ADR-0002 §3) for pure
    /// literals in enum context, otherwise plain `string`, with `{expr}`
    /// interpolations typed (03 §String Interpolation).
    fn check_str(
        &mut self,
        segments: &[ast::StrSeg],
        expected: Option<&Ty>,
        span: ByteSpan,
        sink: &mut DiagnosticSink,
    ) -> TExpr {
        let pure: Option<String> = {
            let mut value = String::new();
            let mut is_pure = true;
            for seg in segments {
                match seg {
                    ast::StrSeg::Text(text) => value.push_str(text),
                    ast::StrSeg::Interp { .. } => is_pure = false,
                }
            }
            is_pure.then_some(value)
        };
        if let Some(value) = pure {
            if let Some(Ty::Enum { wit_name, cases }) = expected {
                // ADR-0002 §3: an enum-typed parameter takes a
                // compile-time string literal naming a case.
                return match cases.iter().position(|c| c == &value) {
                    Some(index) => TExpr {
                        ty: expected.unwrap().clone(),
                        span,
                        kind: TExprKind::EnumCase(index as u32),
                    },
                    None => {
                        let d = build(
                            Level::Error,
                            codes::SEM016,
                            format!("`\"{value}\"` is not a case of enum `{wit_name}`"),
                            self.diag_span(span),
                            Some(format!("expected one of: {}", cases.join(", "))),
                        );
                        self.push_rich(sink, d);
                        error_expr(span)
                    }
                };
            }
            return TExpr {
                ty: Ty::Str,
                span,
                kind: TExprKind::Str(value),
            };
        }
        let segs = segments
            .iter()
            .map(|seg| match seg {
                ast::StrSeg::Text(text) => TInterpSeg::Text(text.clone()),
                ast::StrSeg::Interp { expr, .. } => {
                    let v = self.check_expr(expr, None, sink);
                    self.expect_textable(&v, "interpolated into a string", sink);
                    TInterpSeg::Expr(v)
                }
            })
            .collect();
        TExpr {
            ty: Ty::Str,
            span,
            kind: TExprKind::StrInterp(segs),
        }
    }

    /// A value that can become text (interpolation, `print:`): the types
    /// with a total `toString` conversion (15 §Conversions).
    fn expect_textable(&mut self, value: &TExpr, action: &str, sink: &mut DiagnosticSink) {
        let resolved = self.infcx.resolve(&value.ty);
        let ok = resolved.is_numeric()
            || matches!(resolved, Ty::Boolean | Ty::Str | Ty::Any | Ty::Error);
        if !ok {
            // SEM004 (stub rule; local wording).
            sink.push(build(
                Level::Error,
                codes::SEM004,
                format!(
                    "a value of type `{}` cannot be {action}",
                    resolved.display()
                ),
                self.diag_span(value.span),
                None,
            ));
        }
    }

    /// List literal: checked against a `list`/`matrix` context, or
    /// synthesised with an inference variable for the element type.
    fn check_list(
        &mut self,
        items: &[ast::Expr],
        expected: Option<&Ty>,
        span: ByteSpan,
        sink: &mut DiagnosticSink,
    ) -> TExpr {
        match expected {
            Some(Ty::Matrix(elem)) => {
                let row_ty = Ty::list((**elem).clone());
                let rows = items
                    .iter()
                    .map(|item| {
                        let value = self.check_expr(item, Some(&row_ty), sink);
                        self.coerce_element(value, &row_ty, "matrix rows", item.span(), sink)
                    })
                    .collect();
                TExpr {
                    ty: expected.unwrap().clone(),
                    span,
                    kind: TExprKind::MakeMatrix(rows),
                }
            }
            Some(Ty::List(elem, behavior)) => {
                let elem = (**elem).clone();
                let items = items
                    .iter()
                    .map(|item| {
                        let value = self.check_expr(item, Some(&elem), sink);
                        self.coerce_element(value, &elem, "list elements", item.span(), sink)
                    })
                    .collect();
                TExpr {
                    ty: Ty::List(Box::new(elem), *behavior),
                    span,
                    kind: TExprKind::MakeList(items),
                }
            }
            _ => {
                // Synthesis: unify every element with a fresh variable;
                // an empty literal stays unconstrained (→ `any` at
                // finalize, TYP-02).
                let elem = self.infcx.fresh();
                let items = items
                    .iter()
                    .map(|item| {
                        let value = self.check_expr(item, Some(&elem), sink);
                        self.coerce_element(value, &elem, "list elements", item.span(), sink)
                    })
                    .collect();
                TExpr {
                    ty: Ty::list(elem),
                    span,
                    kind: TExprKind::MakeList(items),
                }
            }
        }
    }

    fn coerce_element(
        &mut self,
        value: TExpr,
        elem: &Ty,
        what: &str,
        span: ByteSpan,
        sink: &mut DiagnosticSink,
    ) -> TExpr {
        match self.coerce(value, elem) {
            Ok(value) => value,
            Err(value) => {
                let mut d = build(
                    Level::Error,
                    codes::SEM001,
                    "type mismatch in assignment".to_string(),
                    self.diag_span(span),
                    Some(format!(
                        "{what} here have type `{}`",
                        self.infcx.resolve(elem).display()
                    )),
                );
                d.secondary.push(Annotation {
                    span: self.diag_span(value.span),
                    label: format!(
                        "this expression has type `{}`",
                        self.infcx.resolve(&value.ty).display()
                    ),
                });
                self.push_rich(sink, d);
                error_expr(span)
            }
        }
    }

    fn integer_literal(
        &mut self,
        value: i128,
        expected: Option<&Ty>,
        span: ByteSpan,
        sink: &mut DiagnosticSink,
    ) -> TExpr {
        // TYP-06: an integer literal in `number` context is a number (the
        // one implicit conversion, folded at the literal).
        if let Some(Ty::Number) = expected {
            return TExpr {
                ty: Ty::Number,
                span,
                kind: TExprKind::Num(value as f64),
            };
        }
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

    // ----- index access (IDX001–IDX005) ---------------------------------

    fn check_index(
        &mut self,
        recv: &ast::Expr,
        index: &ast::Expr,
        span: ByteSpan,
        sink: &mut DiagnosticSink,
    ) -> TExpr {
        let recv_t = self.check_expr(recv, None, sink);
        let resolved = self.infcx.resolve(&recv_t.ty);
        let (payload, was_optional) = match resolved {
            Ty::Option(inner) => ((*inner).clone(), true),
            other => (other, false),
        };
        if was_optional {
            // IDX005 fires only when the receiver is provably `none` on
            // every path reaching this access; otherwise the check is
            // RUN004 at runtime.
            let known_none = match &recv_t.kind {
                TExprKind::NoneLit => true,
                TExprKind::Local(id) => self.known_none.contains(id),
                _ => false,
            };
            if known_none {
                let name = match recv {
                    ast::Expr::Ident { name, .. } => name.clone(),
                    _ => "this value".to_string(),
                };
                // IDX005 — template from Platform 10 §7.
                sink.push(build(
                    Level::Error,
                    codes::IDX005,
                    format!("cannot index `{name}` because it may be `none`"),
                    self.diag_span(recv_t.span),
                    Some(format!(
                        "receiver has type `{}?` and may be `none` here",
                        payload.display()
                    )),
                ));
                return error_expr(span);
            }
        }
        let index_t = self.check_expr(index, None, sink);
        let (kind, ty) = match &payload {
            Ty::List(elem, _) => {
                if !self.is_integerish(&index_t.ty) {
                    // IDX001 (stub rule; local wording).
                    sink.push(build(
                        Level::Error,
                        codes::IDX001,
                        format!(
                            "list index must be `integer`, found `{}`",
                            self.infcx.resolve(&index_t.ty).display()
                        ),
                        self.diag_span(index_t.span),
                        Some("lists are indexed by integer position".to_string()),
                    ));
                    return error_expr(span);
                }
                (IndexKind::List, (**elem).clone())
            }
            Ty::Matrix(elem) => {
                if !self.is_integerish(&index_t.ty) {
                    // IDX002 (stub rule; local wording).
                    sink.push(build(
                        Level::Error,
                        codes::IDX002,
                        format!(
                            "matrix index must be `integer`, found `{}`",
                            self.infcx.resolve(&index_t.ty).display()
                        ),
                        self.diag_span(index_t.span),
                        Some("matrices are indexed by integer row".to_string()),
                    ));
                    return error_expr(span);
                }
                (IndexKind::Matrix, Ty::list((**elem).clone()))
            }
            Ty::Pairs(key, value) => {
                if self.infcx.fit(&index_t.ty, key) == Fit::No {
                    // IDX003 (stub rule; local wording). `K` is a free
                    // type parameter (TYP-02).
                    sink.push(build(
                        Level::Error,
                        codes::IDX003,
                        format!(
                            "`{}` is indexed with `{}`, found `{}`",
                            payload.display(),
                            key.display(),
                            self.infcx.resolve(&index_t.ty).display()
                        ),
                        self.diag_span(index_t.span),
                        Some("wrong key type for this pairs".to_string()),
                    ));
                    return error_expr(span);
                }
                // TYP-03: the lookup result collapses — `V?` over an
                // already-optional `V` stays `V` (absence does not stack).
                let result = match (**value).clone() {
                    opt @ Ty::Option(_) => opt,
                    plain => Ty::Option(Box::new(plain)),
                };
                (IndexKind::Pairs, result)
            }
            Ty::Any => (IndexKind::Any, Ty::Any),
            Ty::Error => return error_expr(span),
            other => {
                // IDX004 — template from Platform 10 §7.
                let mut d = build(
                    Level::Error,
                    codes::IDX004,
                    format!("type `{}` does not support bracket access", other.display()),
                    self.diag_span(recv_t.span),
                    Some(format!(
                        "cannot index a value of type `{}`",
                        other.display()
                    )),
                );
                if matches!(other, Ty::Str) {
                    d.helps
                        .push("string has no bracket access — use `.charAt(n)`".to_string());
                }
                self.push_rich(sink, d);
                return error_expr(span);
            }
        };
        TExpr {
            ty,
            span,
            kind: TExprKind::Index {
                recv: Box::new(recv_t),
                index: Box::new(index_t),
                kind,
            },
        }
    }

    // ----- calls ---------------------------------------------------------

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
            // ERH-01's `error(message)` failure signal has its own frontier
            // note; every other non-identifier callee is a method-style
            // call (chapter 16).
            let construct = if matches!(callee, ast::Expr::ErrorRef { .. }) {
                "`error(...)` failure signals"
            } else {
                "method-style calls"
            };
            sink.note_unsupported(construct, self.diag_span(callee.span()));
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
            let args = self.check_args(name, &params, args, span, false, sink);
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
            let args = self.check_args(name, &params, args, span, true, sink);
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
                if self.infcx.fit(&record, expected_record) == Fit::No {
                    let d = build(
                        Level::Error,
                        codes::SEM016,
                        format!(
                            "class `{name}` does not match record `{}`",
                            expected_record.display()
                        ),
                        self.diag_span(span),
                        Some("field names and types must match the WIT record".to_string()),
                    );
                    self.push_rich(sink, d);
                }
            }
            let params: Vec<Ty> = fields.iter().map(|(_, t)| t.clone()).collect();
            let args = self.check_args(name, &params, args, span, false, sink);
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
        host_boundary: bool,
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
                let Some(param_ty) = expected else {
                    return value;
                };
                // ADR-0002: at the host boundary a `string` value is
                // accepted where `bytes` is declared — both project to
                // the identical (ptr, len) UTF-8 representation. The
                // surface-language conversion story is M6 (§14.14.2).
                let boundary_identity =
                    host_boundary && value.ty == Ty::Str && *param_ty == Ty::Bytes;
                if boundary_identity {
                    return value;
                }
                match self.coerce(value, param_ty) {
                    Ok(value) => value,
                    Err(value) => {
                        // SEM016 — headline template from Platform 10 §3.
                        let mut d = build(
                            Level::Error,
                            codes::SEM016,
                            format!("argument `{}` of `{fn_name}` has the wrong type", i + 1),
                            self.diag_span(arg.span()),
                            Some(format!(
                                "this argument has type `{}`",
                                self.infcx.resolve(&value.ty).display()
                            )),
                        );
                        d.notes.push(format!(
                            "the parameter is declared with type `{}`",
                            param_ty.display()
                        ));
                        self.push_rich(sink, d);
                        error_expr(arg.span())
                    }
                }
            })
            .collect()
    }

    // ----- binary operators (06 §Operators on built-in types) ------------

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
            Default => {
                // EXP-03: `optional default fallback` yields the unwrapped
                // type; the fallback must fit the payload. `default`
                // coalesces only `none`, never falsy values.
                let l = self.check_expr(lhs, None, sink);
                let inner = match self.infcx.resolve(&l.ty) {
                    Ty::Option(inner) => *inner,
                    Ty::Any => Ty::Any,
                    Ty::Error => Ty::Error,
                    other => {
                        self.invalid_op(sink, "default", &other, span);
                        Ty::Error
                    }
                };
                let r = self.check_expr(rhs, Some(&inner), sink);
                let r = match self.coerce(r, &inner) {
                    Ok(r) => r,
                    Err(r) => {
                        let resolved = self.infcx.resolve(&r.ty);
                        self.invalid_op(sink, "default", &resolved, span);
                        error_expr(r.span)
                    }
                };
                (l, r, inner)
            }
            Add | Sub | Mul | Div | Rem | Pow => {
                return self.check_arith(op, lhs, rhs, span, sink);
            }
            Lt | LtEq | Gt | GtEq => {
                let l = self.check_expr(lhs, None, sink);
                let r = self.check_expr(rhs, None, sink);
                let (l, r) = self.balance_numeric(l, r, op_name(op), span, sink);
                (l, r, Ty::Boolean)
            }
            Eq | NEq => {
                let l = self.check_expr(lhs, None, sink);
                let r = self.check_expr(rhs, Some(&l.ty.clone()), sink);
                let (l, r) = self.check_equality(l, r, op_name(op), span, sink);
                (l, r, Ty::Boolean)
            }
            Is | NotIs => {
                // `value is none` is the TYP-03 absence test; general `is`
                // is value identity (06 §Equality and identity).
                if matches!(rhs, ast::Expr::NoneLit { .. }) {
                    let operand = self.check_expr(lhs, None, sink);
                    let resolved = self.infcx.resolve(&operand.ty);
                    if !matches!(resolved, Ty::Option(_) | Ty::Any | Ty::Error) {
                        self.invalid_op(sink, op_name(op), &resolved, span);
                    }
                    return TExpr {
                        ty: Ty::Boolean,
                        span,
                        kind: TExprKind::IsNone {
                            operand: Box::new(operand),
                            negated: op == NotIs,
                        },
                    };
                }
                let l = self.check_expr(lhs, None, sink);
                let r = self.check_expr(rhs, Some(&l.ty.clone()), sink);
                if !self.infcx.unify(&l.ty, &r.ty) {
                    let resolved = self.infcx.resolve(&l.ty);
                    self.invalid_op(sink, op_name(op), &resolved, span);
                }
                (l, r, Ty::Boolean)
            }
            And | Or => {
                let l = self.check_expr(lhs, Some(&Ty::Boolean), sink);
                let r = self.check_expr(rhs, Some(&Ty::Boolean), sink);
                for side in [&l, &r] {
                    let resolved = self.infcx.resolve(&side.ty);
                    if !matches!(resolved, Ty::Boolean | Ty::Error | Ty::Any) {
                        self.invalid_op(sink, op_name(op), &resolved, span);
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

    /// Arithmetic (`+ - * / % ^`): `integer`/`number` with the TYP-06
    /// promotion; `+` also concatenates strings; `+`/`-` are element-wise
    /// and `*` matrix multiplication on `matrix<T>` (06 §Operators).
    fn check_arith(
        &mut self,
        op: ast::BinOp,
        lhs: &ast::Expr,
        rhs: &ast::Expr,
        span: ByteSpan,
        sink: &mut DiagnosticSink,
    ) -> TExpr {
        use ast::BinOp::*;
        let l = self.check_expr(lhs, None, sink);
        let hint = self.infcx.resolve(&l.ty);
        let r = self.check_expr(rhs, hint.is_numeric().then_some(&l.ty), sink);
        let lt = self.infcx.resolve(&l.ty);
        let rt = self.infcx.resolve(&r.ty);

        let make = |l: TExpr, r: TExpr, ty: Ty| TExpr {
            ty,
            span,
            kind: TExprKind::Binary {
                op,
                lhs: Box::new(l),
                rhs: Box::new(r),
            },
        };

        // String concatenation: `+` on two strings.
        if op == Add && lt == Ty::Str && rt == Ty::Str {
            return make(l, r, Ty::Str);
        }
        // Matrix operators: `+`/`-` element-wise, `*` multiplication.
        if let (Ty::Matrix(_), Ty::Matrix(_)) = (&lt, &rt) {
            if matches!(op, Add | Sub | Mul) && self.infcx.unify(&l.ty, &r.ty) {
                let ty = l.ty.clone();
                return make(l, r, ty);
            }
        }
        // `any` skips checking (TYP-02).
        if lt == Ty::Any || rt == Ty::Any {
            return make(l, r, Ty::Any);
        }
        if lt == Ty::Error || rt == Ty::Error {
            return make(l, r, Ty::Error);
        }
        if lt.is_numeric() && rt.is_numeric() {
            // Mixed operands promote to `number` (TYP-06).
            let result = if lt == Ty::Number || rt == Ty::Number {
                Ty::Number
            } else {
                Ty::Integer
            };
            let l = if result == Ty::Number && lt.is_integer() {
                promote(l)
            } else {
                l
            };
            let r = if result == Ty::Number && rt.is_integer() {
                promote(r)
            } else {
                r
            };
            return make(l, r, result);
        }
        let offender = if lt.is_numeric() { &rt } else { &lt };
        self.invalid_op(sink, op_name(op), offender, span);
        error_expr(span)
    }

    /// Orders `< > <= >=` on `integer`/`number` (mixed operands promote).
    fn balance_numeric(
        &mut self,
        l: TExpr,
        r: TExpr,
        op: &str,
        span: ByteSpan,
        sink: &mut DiagnosticSink,
    ) -> (TExpr, TExpr) {
        let lt = self.infcx.resolve(&l.ty);
        let rt = self.infcx.resolve(&r.ty);
        if matches!(lt, Ty::Any | Ty::Error) || matches!(rt, Ty::Any | Ty::Error) {
            return (l, r);
        }
        if lt.is_numeric() && rt.is_numeric() {
            if lt == Ty::Number || rt == Ty::Number {
                let l = if lt.is_integer() { promote(l) } else { l };
                let r = if rt.is_integer() { promote(r) } else { r };
                return (l, r);
            }
            return (l, r);
        }
        let offender = if lt.is_numeric() { rt } else { lt };
        self.invalid_op(sink, op, &offender, span);
        (l, r)
    }

    /// Equality `==`/`!=`: both sides one comparable type — numeric
    /// (mixed promotes), `boolean`, `string` (TYP-07 byte equality), or a
    /// world enum.
    fn check_equality(
        &mut self,
        l: TExpr,
        r: TExpr,
        op: &str,
        span: ByteSpan,
        sink: &mut DiagnosticSink,
    ) -> (TExpr, TExpr) {
        let lt = self.infcx.resolve(&l.ty);
        let rt = self.infcx.resolve(&r.ty);
        if matches!(lt, Ty::Any | Ty::Error) || matches!(rt, Ty::Any | Ty::Error) {
            return (l, r);
        }
        if lt.is_numeric() && rt.is_numeric() {
            if lt == Ty::Number || rt == Ty::Number {
                let l = if lt.is_integer() { promote(l) } else { l };
                let r = if rt.is_integer() { promote(r) } else { r };
                return (l, r);
            }
            return (l, r);
        }
        let comparable = matches!((&lt, &rt), (Ty::Boolean, Ty::Boolean) | (Ty::Str, Ty::Str))
            || matches!((&lt, &rt), (Ty::Enum { wit_name: a, .. }, Ty::Enum { wit_name: b, .. }) if a == b);
        if !comparable {
            self.invalid_op(sink, op, &lt, span);
        }
        (l, r)
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

fn op_name(op: ast::BinOp) -> &'static str {
    use ast::BinOp::*;
    match op {
        Add => "+",
        Sub => "-",
        Mul => "*",
        Div => "/",
        Rem => "%",
        Pow => "^",
        Lt => "<",
        LtEq => "<=",
        Gt => ">",
        GtEq => ">=",
        Eq => "==",
        NEq => "!=",
        Is => "is",
        NotIs => "not",
        And => "and",
        Or => "or",
        Default => "default",
    }
}

/// Materialises TYP-06's implicit `integer` → `number` conversion.
fn promote(value: TExpr) -> TExpr {
    // Integer literals fold directly (no runtime conversion node).
    if let TExprKind::Int(v) = value.kind {
        return TExpr {
            ty: Ty::Number,
            span: value.span,
            kind: TExprKind::Num(v as f64),
        };
    }
    let span = value.span;
    TExpr {
        ty: Ty::Number,
        span,
        kind: TExprKind::IntToNumber(Box::new(value)),
    }
}

fn error_expr(span: ByteSpan) -> TExpr {
    TExpr {
        ty: Ty::Error,
        span,
        kind: TExprKind::Error,
    }
}
