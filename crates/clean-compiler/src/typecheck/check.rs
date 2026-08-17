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
        class_records: Vec::new(),
        classes: Vec::new(),
        state_vars: Vec::new(),
        host_imports: Vec::new(),
        function_sigs: Vec::new(),
    };
    checker.project_classes(sink);
    checker.build_class_table(sink);
    checker.project_host_interfaces(sink);
    checker.collect_function_signatures(sink);
    checker.build_state_table(sink);
    checker.check_default_values(sink);
    let functions = checker.check_functions(sink);
    checker.check_state_watch_tests(sink);
    TypedProgram {
        host_imports: checker.host_imports,
        functions,
    }
}

struct FunctionSig {
    name: String,
    params: Vec<Local>,
    ret: Ty,
    /// Parameters before the first defaulted one (FNC-04): the minimum
    /// call arity.
    required: usize,
    /// Typed default expressions, positionally (None = required). Filled
    /// during signature checking; cloned into call sites (FNC-04:
    /// evaluated fresh per call, at the call site).
    defaults: Vec<Option<TExpr>>,
    /// Whether the declaration carries `before:`/`after:` blocks — a
    /// contract expression may not call such a function (CLASS009, 10
    /// §6.3 rule 2).
    has_contracts: bool,
}

struct Checker<'a> {
    resolved: &'a ResolvedAst,
    world: &'a ParsedWorld,
    /// Projected record types (LBS-02 class↔record), parallel to
    /// `decls.classes`; name lookups go through the module scopes.
    class_records: Vec<Ty>,
    /// Semantic class table (CLS-02/03), parallel to `decls.classes`.
    classes: Vec<ClassInfo>,
    /// State variables per module (SMG-01): name → (type, is_computed).
    state_vars: Vec<IndexMap<String, (Ty, bool)>>,
    host_imports: Vec<HostImport>,
    function_sigs: Vec<FunctionSig>,
}

/// One method signature on a class or capability (CLS-03 arrow-return
/// signatures and CLS-01 `functions:` methods share this shape).
struct MethodSig {
    name: String,
    params: Vec<Ty>,
    ret: Ty,
    /// MOD-02/SEM005: method visibility is module-scoped.
    public: bool,
}

/// The semantic reading of one class declaration: inheritance resolved
/// (SEM006/SEM008/CLASS001), members deduplicated (CLASS002/CLASS003),
/// capability claims recorded for SEM011-013 validation.
struct ClassInfo {
    name: String,
    /// Index into `decls.classes`.
    parent: Option<usize>,
    /// Claimed capability indexes (own claims; parents' claims reachable
    /// through `parent`).
    caps: Vec<usize>,
    /// Own fields, in declaration order: (name, type, public).
    fields: Vec<(String, Ty, bool)>,
    /// Own methods.
    methods: Vec<MethodSig>,
    /// Declared constructor parameter lists (empty = implicit
    /// positional constructor over the fields, CLASS004 adoption).
    ctors: Vec<Vec<Ty>>,
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

    /// Replaces nominal `Class` types by their boundary record
    /// projection, recursively. A class not yet projected (forward
    /// reference in a field type) reports SEM020, matching M1.
    fn to_boundary_or_report(
        &self,
        ty: Ty,
        file: usize,
        span: ByteSpan,
        sink: &mut DiagnosticSink,
    ) -> Ty {
        match ty {
            Ty::Class { class, name } => match self.class_records.get(class) {
                Some(record) => record.clone(),
                None => {
                    sink.push(build(
                        Level::Error,
                        codes::SEM020,
                        format!("I cannot find a class named `{name}`"),
                        self.span(file, span),
                        Some("no class with this name is in scope".to_string()),
                    ));
                    Ty::Error
                }
            },
            Ty::Option(inner) => Ty::Option(Box::new(
                self.to_boundary_or_report(*inner, file, span, sink),
            )),
            Ty::List(inner, b) => Ty::List(
                Box::new(self.to_boundary_or_report(*inner, file, span, sink)),
                b,
            ),
            Ty::Matrix(inner) => Ty::Matrix(Box::new(
                self.to_boundary_or_report(*inner, file, span, sink),
            )),
            Ty::Pairs(k, v) => Ty::Pairs(
                Box::new(self.to_boundary_or_report(*k, file, span, sink)),
                Box::new(self.to_boundary_or_report(*v, file, span, sink)),
            ),
            other => other,
        }
    }

    /// Looks a class name up through `file`'s module scope, yielding its
    /// projection only if it was already projected (declaration order —
    /// forward references stay SEM020, matching M1).
    fn class_record(&self, file: usize, name: &str) -> Option<&Ty> {
        let index = *self.resolved.decls.modules[file].classes.get(name)?;
        self.class_records.get(index)
    }

    fn project_classes(&mut self, sink: &mut DiagnosticSink) {
        for i in 0..self.resolved.decls.classes.len() {
            let coords = self.resolved.decls.classes[i].coords;
            let (class, file) = self.resolved.class(coords);
            let mut fields = Vec::new();
            for field in &class.fields {
                if field.init.is_some() {
                    sink.note_unsupported("field initialisers", self.span(file, field.span));
                }
                let ty = self.project_type(&field.ty, TyPos::Surface, file, sink);
                let ty = self.to_boundary_or_report(ty, file, field.span, sink);
                fields.push((kebab(&field.name), ty));
            }
            self.class_records.push(Ty::Record {
                wit_name: kebab(&class.name),
                fields,
            });
        }
    }

    /// Builds the semantic class table (CLS-02/CLS-03): parents resolved
    /// through the declaring module's scope (CLASS001/SEM006), inheritance
    /// cycles reported once (SEM008), duplicate members (CLASS002/003),
    /// constructor-parameter shadowing (CLASS010), capability claims
    /// validated against the capability's signatures (SEM011/012/013).
    fn build_class_table(&mut self, sink: &mut DiagnosticSink) {
        // Pass A: shape of every class without parents.
        for i in 0..self.resolved.decls.classes.len() {
            let coords = self.resolved.decls.classes[i].coords;
            let (class, file) = self.resolved.class(coords);
            let mut fields: Vec<(String, Ty, bool)> = Vec::new();
            for field in &class.fields {
                if fields.iter().any(|(n, _, _)| n == &field.name) {
                    // CLASS002 (stub rule; local wording).
                    sink.push(build(
                        Level::Error,
                        codes::CLASS002,
                        format!(
                            "class `{}` already has a field named `{}`",
                            class.name, field.name
                        ),
                        self.span(file, field.span),
                        Some("duplicate field".to_string()),
                    ));
                    continue;
                }
                let ty = self.project_type(&field.ty, TyPos::Surface, file, sink);
                fields.push((field.name.clone(), ty, field.public));
            }
            let mut methods: Vec<MethodSig> = Vec::new();
            for m in &class.functions {
                if methods.iter().any(|s| s.name == m.name) {
                    // CLASS003 (stub rule; local wording).
                    sink.push(build(
                        Level::Error,
                        codes::CLASS003,
                        format!(
                            "class `{}` already has a method named `{}`",
                            class.name, m.name
                        ),
                        self.span(file, m.span),
                        Some("duplicate method".to_string()),
                    ));
                    continue;
                }
                let params = m
                    .params
                    .iter()
                    .chain(&m.body.input)
                    .map(|p| self.project_type(&p.ty, TyPos::Surface, file, sink))
                    .collect();
                let ret = self.project_type(&m.ret, TyPos::Surface, file, sink);
                methods.push(MethodSig {
                    name: m.name.clone(),
                    params,
                    ret,
                    public: m.public,
                });
            }
            let mut ctors: Vec<Vec<Ty>> = Vec::new();
            for ctor in &class.constructors {
                for p in &ctor.params {
                    if fields.iter().any(|(n, _, _)| n == &p.name) {
                        // CLASS010 — template from Platform 10 §6.
                        sink.push(build(
                            Level::Error,
                            codes::CLASS010,
                            format!(
                                "Constructor parameter '{}' has the same name as a field",
                                p.name
                            ),
                            self.span(file, p.span),
                            Some("rename this parameter".to_string()),
                        ));
                    }
                }
                ctors.push(
                    ctor.params
                        .iter()
                        .map(|p| self.project_type(&p.ty, TyPos::Surface, file, sink))
                        .collect(),
                );
            }
            self.classes.push(ClassInfo {
                name: class.name.clone(),
                parent: None,
                caps: Vec::new(),
                fields,
                methods,
                ctors,
            });
        }
        // Pass B: parents, cycles, capability claims.
        for i in 0..self.resolved.decls.classes.len() {
            let coords = self.resolved.decls.classes[i].coords;
            let (class, file) = self.resolved.class(coords);
            if let Some((parent_name, parent_span)) = &class.parent {
                match self.resolved.decls.modules[file].classes.get(parent_name) {
                    Some(&p) => self.classes[i].parent = Some(p),
                    None => {
                        // CLASS001 (stub rule; local wording). A parent
                        // that names a non-class type is SEM006 territory;
                        // an unknown name is CLASS001.
                        let code = if self.resolved.decls.modules[file]
                            .capabilities
                            .contains_key(parent_name)
                        {
                            codes::SEM006
                        } else {
                            codes::CLASS001
                        };
                        let message = if code == codes::SEM006 {
                            format!(
                                "`{parent_name}` is a capability, not a class — a class extends classes with `is` and claims capabilities with `can`"
                            )
                        } else {
                            format!("I cannot find a parent class named `{parent_name}`")
                        };
                        sink.push(build(
                            Level::Error,
                            code,
                            message,
                            self.span(file, *parent_span),
                            Some("not a known class".to_string()),
                        ));
                    }
                }
            }
            let caps: Vec<usize> = class
                .capabilities
                .iter()
                .filter_map(|(cap_name, cap_span)| {
                    match self.resolved.decls.modules[file].capabilities.get(cap_name) {
                        Some(&c) => Some(c),
                        None => {
                            // SEM012 (stub rule; local wording).
                            sink.push(build(
                                Level::Error,
                                codes::SEM012,
                                format!("I cannot find a capability named `{cap_name}`"),
                                self.span(file, *cap_span),
                                Some("no capability with this name is in scope".to_string()),
                            ));
                            None
                        }
                    }
                })
                .collect();
            self.classes[i].caps = caps;
        }
        // Pass C: inheritance cycles (SEM008), one report per cycle.
        let mut in_cycle = vec![false; self.classes.len()];
        for i in 0..self.classes.len() {
            let mut seen = vec![false; self.classes.len()];
            let mut cursor = Some(i);
            while let Some(c) = cursor {
                if seen[c] {
                    if c == i && !in_cycle[i] {
                        let coords = self.resolved.decls.classes[i].coords;
                        let (class, file) = self.resolved.class(coords);
                        // SEM008 (stub rule; local wording).
                        sink.push(build(
                            Level::Error,
                            codes::SEM008,
                            format!("class `{}` inherits from itself", class.name),
                            self.span(file, class.span),
                            Some("inheritance cycle".to_string()),
                        ));
                        // Break the cycle so member walks terminate.
                        let mut walk = i;
                        loop {
                            in_cycle[walk] = true;
                            match self.classes[walk].parent {
                                Some(p) if p != i => walk = p,
                                _ => break,
                            }
                        }
                        self.classes[i].parent = None;
                    }
                    break;
                }
                seen[c] = true;
                cursor = self.classes[c].parent;
            }
        }
        // Every capability declaration is signatures-only (CLS-03): a
        // body under a signature is SEM014 whether or not the capability
        // is ever claimed.
        for cap_index in 0..self.resolved.decls.capabilities.len() {
            let coords = self.resolved.decls.capabilities[cap_index].coords;
            let (cap, cap_file) = self.resolved.capability(coords);
            for sig in &cap.signatures {
                if let Some(body_span) = sig.body_span {
                    // SEM014 (stub rule; local wording).
                    sink.push(build(
                        Level::Error,
                        codes::SEM014,
                        format!(
                            "capability method `{}` cannot have a body — capabilities are pure contracts",
                            sig.name
                        ),
                        self.span(cap_file, body_span),
                        Some("declare the signature only".to_string()),
                    ));
                }
            }
        }
        // Pass D: capability claims satisfied (SEM011/SEM013).
        for i in 0..self.classes.len() {
            let caps = self.classes[i].caps.clone();
            for cap_index in caps {
                let coords = self.resolved.decls.capabilities[cap_index].coords;
                let (cap, cap_file) = self.resolved.capability(coords);
                for sig in &cap.signatures {
                    let want_params: Vec<Ty> = sig
                        .params
                        .iter()
                        .map(|p| self.project_type(&p.ty, TyPos::Surface, cap_file, sink))
                        .collect();
                    let want_ret = self.project_type(&sig.ret, TyPos::Surface, cap_file, sink);
                    match self.find_method(i, &sig.name) {
                        None => {
                            let coords = self.resolved.decls.classes[i].coords;
                            let (class, file) = self.resolved.class(coords);
                            // SEM011 (stub rule; local wording).
                            sink.push(build(
                                Level::Error,
                                codes::SEM011,
                                format!(
                                    "class `{}` claims `{}` but does not implement `{}`",
                                    class.name, cap.name, sig.name
                                ),
                                self.span(file, class.span),
                                Some("every declared capability method is required".to_string()),
                            ));
                        }
                        Some((owner, m)) => {
                            let have = &self.classes[owner].methods[m];
                            // CLS-03 is nominal and exact: `string` is not
                            // satisfied by `string?` (TYP-03).
                            if have.params != want_params || have.ret != want_ret {
                                let coords = self.resolved.decls.classes[i].coords;
                                let (class, file) = self.resolved.class(coords);
                                // SEM013 (stub rule; local wording).
                                sink.push(build(
                                    Level::Error,
                                    codes::SEM013,
                                    format!(
                                        "`{}` does not match the signature `{}` declares for `{}`",
                                        sig.name, cap.name, sig.name
                                    ),
                                    self.span(file, class.span),
                                    Some("capability signatures match exactly".to_string()),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    /// Finds a method by name on a class or its ancestors (CLS-02
    /// inheritance walk; cycles were broken in the table build).
    fn find_method(&self, class: usize, name: &str) -> Option<(usize, usize)> {
        let mut cursor = Some(class);
        while let Some(c) = cursor {
            if let Some(m) = self.classes[c].methods.iter().position(|s| s.name == name) {
                return Some((c, m));
            }
            cursor = self.classes[c].parent;
        }
        None
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
        for i in 0..self.resolved.decls.functions.len() {
            let coords = self.resolved.decls.functions[i].coords;
            let (f, file) = self.resolved.function(coords);
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
            // FUNC014 — template from Platform 10 §5: parameters with
            // defaults follow all required ones (FNC-04).
            let mut required = 0usize;
            let mut previous_defaulted: Option<String> = None;
            for p in f.params.iter().chain(&f.body.input) {
                match (&p.default, &previous_defaulted) {
                    (Some(_), _) => previous_defaulted = Some(p.name.clone()),
                    (None, Some(previous)) => {
                        sink.push(build(
                            Level::Error,
                            codes::FUNC014,
                            format!(
                                "Parameter '{}' has no default and follows '{previous}', which has one",
                                p.name
                            ),
                            self.span(file, p.span),
                            Some("required parameter after an optional one".to_string()),
                        ));
                    }
                    (None, None) => required += 1,
                }
            }
            self.function_sigs.push(FunctionSig {
                name: f.name.clone(),
                params,
                ret,
                required,
                defaults: Vec::new(),
                has_contracts: f.body.before.is_some() || f.body.after.is_some(),
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
            ast::BaseType::Named(name) => {
                let class = self.resolved.decls.modules[file].classes.get(name).copied();
                let cap = self.resolved.decls.modules[file]
                    .capabilities
                    .get(name)
                    .copied();
                match (pos, class, cap) {
                    // Surface positions are nominal (CLS-02/03); the
                    // structural record projection is boundary-only and
                    // reapplied at finalize.
                    (TyPos::Surface, Some(index), _) => Ty::Class {
                        class: index,
                        name: name.clone(),
                    },
                    // CLS-03: capability names are valid anywhere a type
                    // is expected.
                    (TyPos::Surface, None, Some(index)) => Ty::Cap {
                        cap: index,
                        name: name.clone(),
                    },
                    (TyPos::Host, Some(_), _) => match self.class_record(file, name) {
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
                    _ => {
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
                }
            }
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
            if self.class_record(file, name).is_none() {
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

    /// Types every default parameter value against its declared type
    /// (FNC-04: defaults must match, SEM001) and stores the typed
    /// expression for call-site fill (defaults evaluate fresh per call).
    fn check_default_values(&mut self, sink: &mut DiagnosticSink) {
        let mut all_defaults: Vec<Vec<Option<TExpr>>> = Vec::new();
        for index in 0..self.resolved.decls.functions.len() {
            let coords = self.resolved.decls.functions[index].coords;
            let (f, file) = self.resolved.function(coords);
            let param_tys: Vec<Ty> = self.function_sigs[index]
                .params
                .iter()
                .map(|p| p.ty.clone())
                .collect();
            let mut checker = BodyChecker {
                outer: self,
                file,
                locals: Vec::new(),
                scopes: vec![IndexMap::new()],
                ret: Ty::Void,
                fn_name: f.name.clone(),
                infcx: InferCtx::new(),
                loop_depth: 0,
                known_none: HashSet::new(),
                in_contract: None,
                this_class: None,
                pending_decls: Vec::new(),
                guard_value: None,
                in_tests: false,
                computed_state: None,
                depth_reported: false,
                allow_signal: false,
                error_binding: false,
            };
            let defaults: Vec<Option<TExpr>> = f
                .params
                .iter()
                .chain(&f.body.input)
                .zip(&param_tys)
                .map(|(p, ty)| {
                    p.default.as_ref().map(|expr| {
                        let value = checker.check_expr(expr, Some(ty), sink);
                        let mut value = checker.coerce_assign(value, ty, &p.name, p.span, sink);
                        finalize_expr(&mut checker.infcx, &checker.outer.class_records, &mut value);
                        value
                    })
                })
                .collect();
            all_defaults.push(defaults);
        }
        for (index, defaults) in all_defaults.into_iter().enumerate() {
            self.function_sigs[index].defaults = defaults;
        }
    }

    /// A body checker with empty scopes for section bodies (state
    /// initialisers, guards, computed bodies, watch and test bodies).
    fn section_checker(&self, file: usize, name: String, ret: Ty) -> BodyChecker<'_, 'a> {
        BodyChecker {
            outer: self,
            file,
            locals: Vec::new(),
            scopes: vec![IndexMap::new()],
            ret,
            fn_name: name,
            infcx: InferCtx::new(),
            loop_depth: 0,
            known_none: HashSet::new(),
            in_contract: None,
            this_class: None,
            pending_decls: Vec::new(),
            guard_value: None,
            in_tests: false,
            computed_state: None,
            depth_reported: false,
            allow_signal: false,
            error_binding: false,
        }
    }

    /// Chapters 20 and 11: state sections (SEM017, STATE001/003/005,
    /// SEM018 via the computed bodies), watch blocks (SCOPE004), and
    /// tests sections (SEM023 assertions, SEM024 expected values,
    /// SCOPE006 via the in_tests gate). Bodies type-check and are
    /// discarded — their lowering is a later milestone.
    fn check_state_watch_tests(&mut self, sink: &mut DiagnosticSink) {
        for i in 0..self.resolved.decls.states.len() {
            let coords = self.resolved.decls.states[i];
            let (section, file) = self.resolved.state(coords);
            for member in &section.members {
                match member {
                    ast::StateMember::Var(var) => {
                        let Some((ty, _)) = self.state_vars[file].get(&var.name).cloned() else {
                            continue;
                        };
                        let mut bc =
                            self.section_checker(file, format!("state.{}", var.name), Ty::Void);
                        let value = bc.check_expr(&var.init, Some(&ty), sink);
                        if let Err(value) = bc.coerce(value, &ty) {
                            // SEM017 — templates from Platform 10 §3.
                            let actual = bc.infcx.resolve(&value.ty).display();
                            let mut d = build(
                                Level::Error,
                                codes::SEM017,
                                format!("state initializer for `{}` has the wrong type", var.name),
                                bc.diag_span(var.init.span()),
                                Some(format!("this initializer has type `{actual}`")),
                            );
                            d.notes.push(format!(
                                "`{}` is declared with type `{}`",
                                var.name,
                                ty.display()
                            ));
                            bc.push_rich(sink, d);
                        }
                        for guard in &var.guards {
                            bc.guard_value = Some(ty.clone());
                            let g = bc.check_expr(&guard.cond, Some(&Ty::Boolean), sink);
                            bc.guard_value = None;
                            let resolved = bc.infcx.resolve(&g.ty);
                            let boolean = matches!(resolved, Ty::Boolean | Ty::Error | Ty::Any);
                            if !boolean || bc.find_impurity(&g).is_some() {
                                // STATE001 — template from Platform 10 §8.
                                sink.push(build(
                                    Level::Error,
                                    codes::STATE001,
                                    "Guard condition must be a pure boolean expression".to_string(),
                                    bc.diag_span(guard.cond.span()),
                                    None,
                                ));
                            }
                        }
                    }
                    ast::StateMember::Computed(decls) => {
                        for c in decls {
                            let Some((ty, _)) = self.state_vars[file].get(&c.name).cloned() else {
                                continue;
                            };
                            let mut bc =
                                self.section_checker(file, format!("state.{}", c.name), ty.clone());
                            bc.computed_state = Some(c.name.clone());
                            let mut body = bc.check_block(&c.body, sink);
                            bc.apply_auto_return(&mut body, c.span, sink);
                        }
                    }
                    ast::StateMember::Rules(exprs) => {
                        let mut bc =
                            self.section_checker(file, "state.rules".to_string(), Ty::Void);
                        for expr in exprs {
                            let value = bc.check_expr(expr, Some(&Ty::Boolean), sink);
                            let resolved = bc.infcx.resolve(&value.ty);
                            if !matches!(resolved, Ty::Boolean | Ty::Error | Ty::Any) {
                                // STATE005 — template from Platform 10 §8.
                                sink.push(build(
                                    Level::Error,
                                    codes::STATE005,
                                    format!(
                                        "State rule expression must be a boolean expression, got {}",
                                        resolved.display()
                                    ),
                                    bc.diag_span(expr.span()),
                                    None,
                                ));
                            }
                        }
                    }
                }
            }
        }
        self.check_computed_cycles(sink);
        for i in 0..self.resolved.decls.watches.len() {
            let coords = self.resolved.decls.watches[i];
            let (watch, file) = self.resolved.watch(coords);
            for (target, target_span) in &watch.targets {
                if !self.state_vars[file].contains_key(target) {
                    // SCOPE004 (stub rule; local wording).
                    sink.push(build(
                        Level::Error,
                        codes::SCOPE004,
                        format!("`{target}` is not a state variable"),
                        self.span(file, *target_span),
                        Some("watch targets name variables from a `state:` block".to_string()),
                    ));
                }
            }
            let mut bc = self.section_checker(file, "watch".to_string(), Ty::Void);
            bc.check_block(&watch.body, sink);
        }
        for i in 0..self.resolved.decls.tests.len() {
            let coords = self.resolved.decls.tests[i];
            let (tests, file) = self.resolved.tests(coords);
            for test in tests {
                match test {
                    ast::TestDecl::Named { assertion, .. }
                    | ast::TestDecl::Anonymous { assertion, .. } => {
                        let mut bc = self.section_checker(file, "test".to_string(), Ty::Void);
                        bc.in_tests = true;
                        let value = bc.check_expr(assertion, Some(&Ty::Boolean), sink);
                        let resolved = bc.infcx.resolve(&value.ty);
                        if !matches!(resolved, Ty::Boolean | Ty::Error | Ty::Any) {
                            sink.push(build(
                                Level::Error,
                                codes::SEM023,
                                format!(
                                    "Condition must be a boolean expression, found {}",
                                    resolved.display()
                                ),
                                bc.diag_span(assertion.span()),
                                Some("expected boolean".to_string()),
                            ));
                        }
                        // TST-01: the right side of the comparison is the
                        // expected result and must be compile-time
                        // evaluable (SEM024, template verbatim).
                        if let ast::Expr::Binary {
                            op: ast::BinOp::Eq | ast::BinOp::NEq,
                            rhs,
                            ..
                        } = assertion
                        {
                            if !is_compile_time_constant(rhs) {
                                sink.push(build(
                                    Level::Error,
                                    codes::SEM024,
                                    "Expected value must be evaluable at compile time".to_string(),
                                    self.span(file, rhs.span()),
                                    Some("not a compile-time value".to_string()),
                                ));
                            }
                        }
                    }
                    ast::TestDecl::Block { body, .. } => {
                        let mut bc = self.section_checker(file, "test".to_string(), Ty::Void);
                        bc.in_tests = true;
                        bc.check_block(body, sink);
                    }
                }
            }
        }
    }

    /// STATE003 — computed-state dependency cycles (SMG-05: static
    /// tracking over the names that appear in each computed body),
    /// reported once per cycle with the Platform 10 template.
    fn check_computed_cycles(&mut self, sink: &mut DiagnosticSink) {
        for i in 0..self.resolved.decls.states.len() {
            let coords = self.resolved.decls.states[i];
            let (section, file) = self.resolved.state(coords);
            let mut computed: Vec<(&str, &ast::ComputedDecl)> = Vec::new();
            for member in &section.members {
                if let ast::StateMember::Computed(decls) = member {
                    for c in decls {
                        computed.push((c.name.as_str(), c));
                    }
                }
            }
            let names: Vec<&str> = computed.iter().map(|(n, _)| *n).collect();
            let deps: Vec<Vec<usize>> = computed
                .iter()
                .map(|(_, c)| {
                    let mut mentioned = Vec::new();
                    for stmt in &c.body {
                        collect_idents(stmt, &mut mentioned);
                    }
                    names
                        .iter()
                        .enumerate()
                        .filter(|(_, n)| mentioned.iter().any(|m| m == *n))
                        .map(|(j, _)| j)
                        .collect()
                })
                .collect();
            let mut reported = vec![false; computed.len()];
            for start in 0..computed.len() {
                let mut seen = vec![false; computed.len()];
                let mut stack = vec![start];
                while let Some(node) = stack.pop() {
                    for &next in &deps[node] {
                        if next == start {
                            if !reported[start] {
                                reported[start] = true;
                                sink.push(build(
                                    Level::Error,
                                    codes::STATE003,
                                    format!(
                                        "Circular dependency in computed state: '{}' depends on itself",
                                        names[start]
                                    ),
                                    self.span(file, computed[start].1.span),
                                    None,
                                ));
                            }
                        } else if !seen[next] {
                            seen[next] = true;
                            stack.push(next);
                        }
                    }
                }
            }
        }
    }

    /// SMG-01: registers every module's state variables (declared and
    /// computed) so bodies can resolve them; duplicate names are SEM003
    /// (module-scoped, like every other declaration).
    fn build_state_table(&mut self, sink: &mut DiagnosticSink) {
        self.state_vars = vec![IndexMap::new(); self.resolved.files.len()];
        for i in 0..self.resolved.decls.states.len() {
            let coords = self.resolved.decls.states[i];
            let (section, file) = self.resolved.state(coords);
            for member in &section.members {
                match member {
                    ast::StateMember::Var(var) => {
                        let ty = self.project_type(&var.ty, TyPos::Surface, file, sink);
                        if self.state_vars[file]
                            .insert(var.name.clone(), (ty, false))
                            .is_some()
                        {
                            sink.push(build(
                                Level::Error,
                                codes::SEM003,
                                format!("`{}` is declared more than once", var.name),
                                self.span(file, var.span),
                                Some("redefinition".to_string()),
                            ));
                        }
                    }
                    ast::StateMember::Computed(decls) => {
                        for c in decls {
                            let ty = self.project_type(&c.ty, TyPos::Surface, file, sink);
                            if self.state_vars[file]
                                .insert(c.name.clone(), (ty, true))
                                .is_some()
                            {
                                sink.push(build(
                                    Level::Error,
                                    codes::SEM003,
                                    format!("`{}` is declared more than once", c.name),
                                    self.span(file, c.span),
                                    Some("redefinition".to_string()),
                                ));
                            }
                        }
                    }
                    ast::StateMember::Rules(_) => {}
                }
            }
        }
    }

    fn check_functions(&mut self, sink: &mut DiagnosticSink) -> Vec<TFunction> {
        let mut out = Vec::new();
        for index in 0..self.resolved.decls.functions.len() {
            let coords = self.resolved.decls.functions[index].coords;
            let (f, file) = self.resolved.function(coords);
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
            // CLASS005 (stub rule; local wording): `after:` comes after
            // the `before:` block. The parser accepts either order and
            // records spans; intervening statements cannot parse (the
            // contract prelude precedes the statement sequence).
            if let (Some(b), Some(a)) = (&f.body.before, &f.body.after) {
                if a.span.start < b.span.start {
                    sink.push(build(
                        Level::Error,
                        codes::CLASS005,
                        "'after:' must come after the 'before:' block".to_string(),
                        self.span(file, a.span),
                        Some("swap the contract blocks".to_string()),
                    ));
                }
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
                in_contract: None,
                this_class: None,
                pending_decls: Vec::new(),
                guard_value: None,
                in_tests: false,
                computed_state: None,
                depth_reported: false,
                allow_signal: false,
                error_binding: false,
            };
            let before = f
                .body
                .before
                .as_ref()
                .map(|b| body_checker.check_contract_block(b, ContractKind::Before, sink))
                .unwrap_or_default();
            let after = f
                .body
                .after
                .as_ref()
                .map(|a| body_checker.check_contract_block(a, ContractKind::After, sink))
                .unwrap_or_default();
            let mut body = body_checker.check_block(&f.body.statements, sink);
            body_checker.apply_auto_return(&mut body, f.span, sink);
            let mut function = TFunction {
                name: sig.name.clone(),
                params: sig.params.clone(),
                ret: sig.ret.clone(),
                locals: body_checker.locals,
                before,
                after,
                body,
                span: f.span,
                file,
            };
            // No `Ty::Var` leaves pass [5]: collapse what stayed
            // unconstrained to `any` (TYP-02).
            finalize_function(&mut body_checker.infcx, &self.class_records, &mut function);
            out.push(function);
        }
        // Class method, constructor, and `always:` bodies (CLS-01/02,
        // CTR-03) — checked with `this` in scope; their TIR functions
        // append after the callable space (not name-addressable).
        for class_idx in 0..self.resolved.decls.classes.len() {
            let coords = self.resolved.decls.classes[class_idx].coords;
            let (class, file) = self.resolved.class(coords);
            let class_name = class.name.clone();
            for (m_idx, m) in class.functions.iter().enumerate() {
                // The method table deduplicates (CLASS003), so index by
                // name and check only the first occurrence of each.
                if class.functions.iter().position(|f2| f2.name == m.name) != Some(m_idx) {
                    continue;
                }
                let Some(sig_idx) = self.classes[class_idx]
                    .methods
                    .iter()
                    .position(|s| s.name == m.name)
                else {
                    continue;
                };
                let (params, ret) = {
                    let sig = &self.classes[class_idx].methods[sig_idx];
                    (sig.params.clone(), sig.ret.clone())
                };
                let named_params: Vec<Local> = m
                    .params
                    .iter()
                    .chain(&m.body.input)
                    .zip(&params)
                    .map(|(p, ty)| Local {
                        name: p.name.clone(),
                        ty: ty.clone(),
                    })
                    .collect();
                let scope: IndexMap<String, LocalId> = named_params
                    .iter()
                    .enumerate()
                    .map(|(i, p)| (p.name.clone(), i))
                    .collect();
                let mut body_checker = BodyChecker {
                    outer: self,
                    file,
                    locals: named_params.clone(),
                    scopes: vec![scope],
                    ret: ret.clone(),
                    fn_name: format!("{class_name}.{}", m.name),
                    infcx: InferCtx::new(),
                    loop_depth: 0,
                    known_none: HashSet::new(),
                    in_contract: None,
                    this_class: Some(class_idx),
                    pending_decls: Vec::new(),
                    guard_value: None,
                    in_tests: false,
                    computed_state: None,
                    depth_reported: false,
                    allow_signal: false,
                    error_binding: false,
                };
                let before = m
                    .body
                    .before
                    .as_ref()
                    .map(|b| body_checker.check_contract_block(b, ContractKind::Before, sink))
                    .unwrap_or_default();
                let after = m
                    .body
                    .after
                    .as_ref()
                    .map(|a| body_checker.check_contract_block(a, ContractKind::After, sink))
                    .unwrap_or_default();
                let mut body = body_checker.check_block(&m.body.statements, sink);
                body_checker.apply_auto_return(&mut body, m.span, sink);
                let mut function = TFunction {
                    name: format!("{class_name}.{}", m.name),
                    params: named_params,
                    ret,
                    locals: body_checker.locals,
                    before,
                    after,
                    body,
                    span: m.span,
                    file,
                };
                finalize_function(&mut body_checker.infcx, &self.class_records, &mut function);
                out.push(function);
            }
            for (c_idx, ctor) in class.constructors.iter().enumerate() {
                let params = self.classes[class_idx].ctors[c_idx].clone();
                let named_params: Vec<Local> = ctor
                    .params
                    .iter()
                    .zip(&params)
                    .map(|(p, ty)| Local {
                        name: p.name.clone(),
                        ty: ty.clone(),
                    })
                    .collect();
                let scope: IndexMap<String, LocalId> = named_params
                    .iter()
                    .enumerate()
                    .map(|(i, p)| (p.name.clone(), i))
                    .collect();
                let mut body_checker = BodyChecker {
                    outer: self,
                    file,
                    locals: named_params.clone(),
                    scopes: vec![scope],
                    ret: Ty::Void,
                    fn_name: format!("{class_name}.constructor"),
                    infcx: InferCtx::new(),
                    loop_depth: 0,
                    known_none: HashSet::new(),
                    in_contract: None,
                    this_class: Some(class_idx),
                    pending_decls: Vec::new(),
                    guard_value: None,
                    in_tests: false,
                    computed_state: None,
                    depth_reported: false,
                    allow_signal: false,
                    error_binding: false,
                };
                let body = body_checker.check_block(&ctor.body, sink);
                let mut function = TFunction {
                    name: format!("{class_name}.constructor"),
                    params: named_params,
                    ret: Ty::Void,
                    locals: body_checker.locals,
                    before: Vec::new(),
                    after: Vec::new(),
                    body,
                    span: ctor.span,
                    file,
                };
                finalize_function(&mut body_checker.infcx, &self.class_records, &mut function);
                out.push(function);
            }
            if let Some(always) = &class.always {
                let mut body_checker = BodyChecker {
                    outer: self,
                    file,
                    locals: Vec::new(),
                    scopes: vec![IndexMap::new()],
                    ret: Ty::Void,
                    fn_name: format!("{class_name}.always"),
                    infcx: InferCtx::new(),
                    loop_depth: 0,
                    known_none: HashSet::new(),
                    in_contract: None,
                    this_class: Some(class_idx),
                    pending_decls: Vec::new(),
                    guard_value: None,
                    in_tests: false,
                    computed_state: None,
                    depth_reported: false,
                    allow_signal: false,
                    error_binding: false,
                };
                body_checker.in_contract = Some(ContractKind::Before);
                for expr in &always.exprs {
                    let value = body_checker.check_expr(expr, Some(&Ty::Boolean), sink);
                    let resolved = body_checker.infcx.resolve(&value.ty);
                    if !matches!(resolved, Ty::Boolean | Ty::Error | Ty::Any) {
                        // CLASS006 (stub rule; local wording): every
                        // expression in `always:` must be boolean.
                        sink.push(build(
                            Level::Error,
                            codes::CLASS006,
                            format!(
                                "every expression in `always:` must be boolean, found `{}`",
                                resolved.display()
                            ),
                            body_checker.diag_span(expr.span()),
                            Some("class invariants are boolean".to_string()),
                        ));
                    }
                    body_checker.check_purity(&value, sink);
                }
            }
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
                in_contract: None,
                this_class: None,
                pending_decls: Vec::new(),
                guard_value: None,
                in_tests: false,
                computed_state: None,
                depth_reported: false,
                allow_signal: false,
                error_binding: false,
            };
            let body = body_checker.check_block(block, sink);
            let mut function = TFunction {
                name: "start".to_string(),
                params: Vec::new(),
                ret: Ty::Void,
                locals: body_checker.locals,
                before: Vec::new(),
                after: Vec::new(),
                body,
                span,
                file,
            };
            finalize_function(&mut body_checker.infcx, &self.class_records, &mut function);
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

/// Deep-resolves every type in the function through the inference table,
/// then demotes nominal `Class` types to their boundary record projection
/// (LBS-02) so later passes see the M1 structural shapes. `Cap` survives
/// (no lowering yet).
fn demote_classes(ty: Ty, class_records: &[Ty]) -> Ty {
    match ty {
        Ty::Class { class, .. } => class_records.get(class).cloned().unwrap_or(Ty::Error),
        Ty::Option(inner) => Ty::Option(Box::new(demote_classes(*inner, class_records))),
        Ty::List(inner, b) => Ty::List(Box::new(demote_classes(*inner, class_records)), b),
        Ty::Matrix(inner) => Ty::Matrix(Box::new(demote_classes(*inner, class_records))),
        Ty::Pairs(k, v) => Ty::Pairs(
            Box::new(demote_classes(*k, class_records)),
            Box::new(demote_classes(*v, class_records)),
        ),
        other => other,
    }
}

/// Deep-resolves every type in the function through the inference table.
fn finalize_function(infcx: &mut InferCtx, class_records: &[Ty], f: &mut TFunction) {
    for local in &mut f.locals {
        local.ty = demote_classes(infcx.finalize(&local.ty), class_records);
    }
    f.ret = demote_classes(infcx.finalize(&f.ret), class_records);
    for expr in f.before.iter_mut().chain(f.after.iter_mut()) {
        finalize_expr(infcx, class_records, expr);
    }
    for stmt in &mut f.body {
        finalize_stmt(infcx, class_records, stmt);
    }
}

fn finalize_stmt(infcx: &mut InferCtx, class_records: &[Ty], stmt: &mut TStmt) {
    match stmt {
        TStmt::Let { init, .. } => {
            if let Some(init) = init {
                finalize_expr(infcx, class_records, init);
            }
        }
        TStmt::Assign { value, .. } => finalize_expr(infcx, class_records, value),
        TStmt::Return { value, .. } => {
            if let Some(value) = value {
                finalize_expr(infcx, class_records, value);
            }
        }
        TStmt::Expr(expr) => finalize_expr(infcx, class_records, expr),
        TStmt::If {
            cond,
            then,
            else_ifs,
            els,
        } => {
            finalize_expr(infcx, class_records, cond);
            then.iter_mut()
                .for_each(|s| finalize_stmt(infcx, class_records, s));
            for (c, b) in else_ifs {
                finalize_expr(infcx, class_records, c);
                b.iter_mut()
                    .for_each(|s| finalize_stmt(infcx, class_records, s));
            }
            if let Some(els) = els {
                els.iter_mut()
                    .for_each(|s| finalize_stmt(infcx, class_records, s));
            }
        }
        TStmt::While { cond, body } => {
            finalize_expr(infcx, class_records, cond);
            body.iter_mut()
                .for_each(|s| finalize_stmt(infcx, class_records, s));
        }
        TStmt::Iterate {
            source, step, body, ..
        } => {
            match source {
                TIterSource::List(e) | TIterSource::Chars(e) | TIterSource::Rows(e) => {
                    finalize_expr(infcx, class_records, e)
                }
                TIterSource::Range { from, to } => {
                    finalize_expr(infcx, class_records, from);
                    finalize_expr(infcx, class_records, to);
                }
            }
            if let Some(step) = step {
                finalize_expr(infcx, class_records, step);
            }
            body.iter_mut()
                .for_each(|s| finalize_stmt(infcx, class_records, s));
        }
        TStmt::Break { .. } | TStmt::Continue { .. } => {}
        TStmt::Print { items, .. } => items
            .iter_mut()
            .for_each(|e| finalize_expr(infcx, class_records, e)),
        TStmt::Assert { cond, .. } => finalize_expr(infcx, class_records, cond),
    }
}

fn finalize_expr(infcx: &mut InferCtx, class_records: &[Ty], expr: &mut TExpr) {
    expr.ty = demote_classes(infcx.finalize(&expr.ty), class_records);
    match &mut expr.kind {
        TExprKind::MakeRecord(items)
        | TExprKind::MakeList(items)
        | TExprKind::MakeMatrix(items)
        | TExprKind::CallHost { args: items, .. }
        | TExprKind::CallFn { args: items, .. } => items
            .iter_mut()
            .for_each(|e| finalize_expr(infcx, class_records, e)),
        TExprKind::Binary { lhs, rhs, .. } => {
            finalize_expr(infcx, class_records, lhs);
            finalize_expr(infcx, class_records, rhs);
        }
        TExprKind::Unary { operand, .. }
        | TExprKind::NonNone(operand)
        | TExprKind::IsNone { operand, .. }
        | TExprKind::IntToNumber(operand)
        | TExprKind::WrapSome(operand)
        | TExprKind::Convert(operand)
        | TExprKind::GetField { recv: operand, .. } => finalize_expr(infcx, class_records, operand),
        TExprKind::CallMethod { recv, args, .. } | TExprKind::CallDyn { recv, args, .. } => {
            finalize_expr(infcx, class_records, recv);
            args.iter_mut()
                .for_each(|a| finalize_expr(infcx, class_records, a));
        }
        TExprKind::CallStatic { args, .. } | TExprKind::CallCtor { args, .. } => {
            args.iter_mut()
                .for_each(|a| finalize_expr(infcx, class_records, a));
        }
        TExprKind::Index { recv, index, .. } => {
            finalize_expr(infcx, class_records, recv);
            finalize_expr(infcx, class_records, index);
        }
        TExprKind::StrInterp(segs) => {
            for seg in segs {
                if let TInterpSeg::Expr(e) = seg {
                    finalize_expr(infcx, class_records, e);
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
        | TExprKind::ResultRef
        | TExprKind::This
        | TExprKind::GetState { .. }
        | TExprKind::GuardValue
        | TExprKind::ErrorBinding
        | TExprKind::Error => {}
        TExprKind::Raise(operand) | TExprKind::GetRecordField { recv: operand, .. } => {
            finalize_expr(infcx, class_records, operand)
        }
        TExprKind::OnError { value, fallback } => {
            finalize_expr(infcx, class_records, value);
            finalize_expr(infcx, class_records, fallback);
        }
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
    /// Some while checking a contract expression: gates `result` (only
    /// in `after:`, CLASS008) and purity (CLASS009).
    in_contract: Option<ContractKind>,
    /// Some while checking a class method, constructor, or `always:`
    /// body: `this` is typed, fields resolve bare (CLS-02).
    this_class: Option<usize>,
    /// Names declared anywhere in each open block with their declaration
    /// start offsets — the SCOPE001 use-before-declaration probe.
    pending_decls: Vec<Vec<(String, u32)>>,
    /// The type of `value` inside a guard clause (SMG-02).
    guard_value: Option<Ty>,
    /// Inside a `tests:` block body (SCOPE006 gate).
    in_tests: bool,
    /// Set for computed-state bodies: return mismatches report SEM018
    /// with the state templates instead of SEM015.
    computed_state: Option<String>,
    /// SCOPE003 reported once per body.
    depth_reported: bool,
    /// Statement position directly under a `Stmt::Expr` — the one place
    /// ERH-01 admits the `error(...)` signal.
    allow_signal: bool,
    /// Inside an `onError` handler: the `error` binding is in scope
    /// (ERH-04).
    error_binding: bool,
}

/// The built-in `Error` record (ERH-04): `.message` string, `.code`
/// string? (`none` for the program's own `error(...)`).
fn error_record() -> Ty {
    Ty::Record {
        wit_name: "Error".to_string(),
        fields: vec![
            ("message".to_string(), Ty::Str),
            ("code".to_string(), Ty::Option(Box::new(Ty::Str))),
        ],
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ContractKind {
    Before,
    After,
}

impl<'c, 'a> BodyChecker<'c, 'a> {
    fn diag_span(&self, span: ByteSpan) -> clean_compiler_types::Span {
        self.outer.span(self.file, span)
    }

    /// The visibility scope of the module this body lives in.
    fn module_scope(&self) -> &crate::resolver::ModuleScope {
        &self.outer.resolved.decls.modules[self.file]
    }

    /// SCOPE001 probe: is `name` declared in an open block at an offset
    /// after `use_start`? (SEM002 owns the no-declaration-at-all case.)
    fn declared_later(&self, name: &str, use_start: u32) -> Option<u32> {
        self.pending_decls
            .iter()
            .rev()
            .flat_map(|block| block.iter())
            .find(|(n, start)| n == name && *start > use_start)
            .map(|(_, start)| *start)
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

    /// Nominal assignability (CLS-02 inheritance, CLS-03 capability
    /// claims, class→record at the boundary) layered over the structural
    /// `InferCtx::fit`.
    fn fit(&mut self, from: &Ty, to: &Ty) -> Fit {
        let from_r = self.infcx.resolve(from);
        let to_r = self.infcx.resolve(to);
        match (&from_r, &to_r) {
            (Ty::Error, _) | (_, Ty::Error) | (Ty::Any, _) | (_, Ty::Any) => Fit::Exact,
            (Ty::Class { class: a, .. }, Ty::Class { class: b, .. }) => {
                if self.is_ancestor_or_self(*a, *b) {
                    Fit::Exact
                } else {
                    Fit::No
                }
            }
            (Ty::Class { class, .. }, Ty::Cap { cap, .. }) => {
                if self.class_claims(*class, *cap) {
                    Fit::Exact
                } else {
                    Fit::No
                }
            }
            (Ty::Cap { cap: a, .. }, Ty::Cap { cap: b, .. }) => {
                if a == b {
                    Fit::Exact
                } else {
                    Fit::No
                }
            }
            // A class value crossing into a boundary record slot: same
            // runtime shape when the projections agree (LBS-02).
            (Ty::Class { class, .. }, Ty::Record { .. }) => {
                let record = self
                    .outer
                    .class_records
                    .get(*class)
                    .cloned()
                    .unwrap_or(Ty::Error);
                if self.infcx.unify(&record, &to_r) {
                    Fit::Exact
                } else {
                    Fit::No
                }
            }
            (Ty::Class { .. } | Ty::Cap { .. }, Ty::Option(inner)) => {
                let inner = inner.as_ref().clone();
                match self.fit(&from_r, &inner) {
                    Fit::Exact => Fit::Wrap { promote: false },
                    _ => Fit::No,
                }
            }
            _ => self.infcx.fit(from, to),
        }
    }

    /// Walks `from` up its parent chain looking for `to` (CLS-02).
    fn is_ancestor_or_self(&self, from: usize, to: usize) -> bool {
        let mut cursor = Some(from);
        while let Some(c) = cursor {
            if c == to {
                return true;
            }
            cursor = self.outer.classes[c].parent;
        }
        false
    }

    /// A class satisfies a capability it (or any ancestor) claims
    /// (CLS-03: claims are inherited).
    fn class_claims(&self, class: usize, cap: usize) -> bool {
        let mut cursor = Some(class);
        while let Some(c) = cursor {
            if self.outer.classes[c].caps.contains(&cap) {
                return true;
            }
            cursor = self.outer.classes[c].parent;
        }
        false
    }

    /// Applies the `fit` verdict, materialising promotions and option
    /// wraps. `Err` returns the original value: the caller reports.
    #[allow(clippy::result_large_err)]
    fn coerce(&mut self, value: TExpr, to: &Ty) -> Result<TExpr, TExpr> {
        match self.fit(&value.ty, to) {
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

    /// SEM015 for functions; SEM018 (templates from Platform 10 §3) when
    /// the body is a computed-state declaration (SMG-05 boundary: type
    /// mismatch is SEM018, never STATE003).
    fn return_mismatch(&mut self, value: &TExpr, span: ByteSpan, sink: &mut DiagnosticSink) {
        let actual = self.infcx.resolve(&value.ty).display();
        if let Some(state_name) = self.computed_state.clone() {
            let mut d = build(
                Level::Error,
                codes::SEM018,
                format!("computed state `{state_name}` returns the wrong type"),
                self.diag_span(span),
                Some(format!("this body evaluates to `{actual}`")),
            );
            d.notes.push(format!(
                "`{state_name}` is declared with type `{}`",
                self.ret.display()
            ));
            self.push_rich(sink, d);
            return;
        }
        let mut d = build(
            Level::Error,
            codes::SEM015,
            format!("return type mismatch in `{}`", self.fn_name),
            self.diag_span(span),
            Some(format!("this expression has type `{actual}`")),
        );
        d.notes.push(format!(
            "function declares return type `{}`",
            self.ret.display()
        ));
        self.push_rich(sink, d);
    }

    /// Chapter 09 §Automatic Return: with no explicit `return`, the value
    /// of the last expression is returned. For non-void bodies the last
    /// expression statement becomes the return value (SEM015 on
    /// mismatch); a body that can still complete without returning gets
    /// the FUNC004 warning (stub rule; local wording).
    fn apply_auto_return(
        &mut self,
        body: &mut Vec<TStmt>,
        fn_span: ByteSpan,
        sink: &mut DiagnosticSink,
    ) {
        if matches!(self.ret, Ty::Void | Ty::Error) {
            return;
        }
        if matches!(body.last(), Some(TStmt::Expr(_))) {
            let Some(TStmt::Expr(value)) = body.pop() else {
                unreachable!("just matched an expression statement")
            };
            let span = value.span;
            let ret = self.ret.clone();
            let value = match self.coerce(value, &ret) {
                Ok(value) => value,
                Err(value) => {
                    self.return_mismatch(&value, span, sink);
                    error_expr(span)
                }
            };
            body.push(TStmt::Return {
                value: Some(value),
                span,
            });
            return;
        }
        if !terminates(body) {
            sink.push(build(
                Level::Warning,
                codes::FUNC004,
                format!(
                    "`{}` declares return type `{}` but may complete without returning",
                    self.fn_name,
                    self.ret.display()
                ),
                self.diag_span(fn_span),
                None,
            ));
        }
    }

    // ----- contracts (chapter 10) ---------------------------------------

    /// Checks one `before:`/`after:` block: each line a boolean
    /// expression (SEM023 wording — chapter 10 registers no dedicated
    /// code for a non-boolean contract line; see DISCOVERIES-M4), pure
    /// (CLASS009), a fresh body for FLW-03 purposes.
    fn check_contract_block(
        &mut self,
        block: &ast::ContractBlock,
        kind: ContractKind,
        sink: &mut DiagnosticSink,
    ) -> Vec<TExpr> {
        let saved_depth = self.loop_depth;
        self.loop_depth = 0;
        self.in_contract = Some(kind);
        let out: Vec<TExpr> = block
            .exprs
            .iter()
            .map(|expr| {
                let value = self.check_condition(expr, sink);
                self.check_purity(&value, sink);
                value
            })
            .collect();
        self.in_contract = None;
        self.loop_depth = saved_depth;
        out
    }

    /// CLASS009 — template from Platform 10 §6: contract expressions
    /// cannot perform I/O or call a function that itself carries
    /// contracts. (Assignments cannot appear: contract lines are
    /// expressions.)
    fn check_purity(&mut self, expr: &TExpr, sink: &mut DiagnosticSink) {
        let operation: Option<String> = match &expr.kind {
            TExprKind::CallHost { import, .. } => {
                Some(self.outer.host_imports[*import].clean_name.clone())
            }
            TExprKind::CallFn { func, .. } => {
                let sig = &self.outer.function_sigs[*func];
                sig.has_contracts.then(|| sig.name.clone())
            }
            _ => None,
        };
        if let Some(operation) = operation {
            sink.push(build(
                Level::Error,
                codes::CLASS009,
                format!("Contract expression must be pure: '{operation}' is not allowed here"),
                self.diag_span(expr.span),
                Some("contract expressions cannot have effects".to_string()),
            ));
        }
        self.walk_purity_children(expr, sink);
    }

    /// SMG-02's purity probe: the first effectful operation in the
    /// expression, if any (host calls; calls to contract-carrying
    /// functions).
    fn find_impurity(&self, expr: &TExpr) -> Option<String> {
        match &expr.kind {
            TExprKind::CallHost { import, .. } => {
                return Some(self.outer.host_imports[*import].clean_name.clone());
            }
            TExprKind::CallFn { func, .. } => {
                let sig = &self.outer.function_sigs[*func];
                if sig.has_contracts {
                    return Some(sig.name.clone());
                }
            }
            _ => {}
        }
        purity_children(expr)
            .into_iter()
            .find_map(|e| self.find_impurity(e))
    }

    fn walk_purity_children(&mut self, expr: &TExpr, sink: &mut DiagnosticSink) {
        match &expr.kind {
            TExprKind::MakeRecord(items)
            | TExprKind::MakeList(items)
            | TExprKind::MakeMatrix(items)
            | TExprKind::CallHost { args: items, .. }
            | TExprKind::CallFn { args: items, .. } => {
                for item in items {
                    self.check_purity(item, sink);
                }
            }
            TExprKind::Binary { lhs, rhs, .. } => {
                self.check_purity(lhs, sink);
                self.check_purity(rhs, sink);
            }
            TExprKind::Unary { operand, .. }
            | TExprKind::NonNone(operand)
            | TExprKind::IsNone { operand, .. }
            | TExprKind::IntToNumber(operand)
            | TExprKind::WrapSome(operand) => self.check_purity(operand, sink),
            TExprKind::Index { recv, index, .. } => {
                self.check_purity(recv, sink);
                self.check_purity(index, sink);
            }
            TExprKind::StrInterp(segs) => {
                for seg in segs {
                    if let TInterpSeg::Expr(e) = seg {
                        self.check_purity(e, sink);
                    }
                }
            }
            _ => {}
        }
    }

    // ----- statements ----------------------------------------------------

    fn check_block(&mut self, block: &[ast::Stmt], sink: &mut DiagnosticSink) -> Vec<TStmt> {
        // SCOPE003 (stub rule; local wording): implementation limit of 64
        // nested scopes, reported once per body.
        if self.scopes.len() >= 64 && !self.depth_reported {
            self.depth_reported = true;
            if let Some(first) = block.first() {
                sink.push(build(
                    Level::Error,
                    codes::SCOPE003,
                    "the scope nesting depth exceeds the implementation limit of 64".to_string(),
                    self.diag_span(stmt_span(first)),
                    Some("flatten this nesting".to_string()),
                ));
            }
        }
        self.scopes.push(IndexMap::new());
        self.pending_decls.push(
            block
                .iter()
                .filter_map(|stmt| match stmt {
                    ast::Stmt::VarDecl {
                        name, name_span, ..
                    } => Some((name.clone(), name_span.start)),
                    _ => None,
                })
                .collect(),
        );
        let out = block
            .iter()
            .filter_map(|stmt| self.check_stmt(stmt, sink))
            .collect();
        self.pending_decls.pop();
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
                self.check_on_error_block(on_error, sink);
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
                self.check_on_error_block(on_error, sink);
                match target {
                    ast::Expr::Ident {
                        name,
                        span: name_span,
                    } => {
                        let Some(local) = self.lookup(name) else {
                            if let Some((ty, computed)) =
                                self.outer.state_vars[self.file].get(name).cloned()
                            {
                                if computed {
                                    // STATE004 — template from Platform 10 §8.
                                    sink.push(build(
                                        Level::Error,
                                        codes::STATE004,
                                        format!(
                                            "Cannot assign to computed state variable '{name}': it is read-only"
                                        ),
                                        self.diag_span(*name_span),
                                        Some("computed state is derived".to_string()),
                                    ));
                                    return None;
                                }
                                let value = self.check_expr(value, Some(&ty), sink);
                                self.coerce_assign(value, &ty, name, *name_span, sink);
                                sink.note_unsupported(
                                    "state assignment",
                                    self.diag_span(*name_span),
                                );
                                return None;
                            }
                            // A bare field name inside a class body is a
                            // legal assignment target (CLS-02); its
                            // lowering is M6.
                            if let Some(this_class) = self.this_class {
                                let mut cursor = Some(this_class);
                                while let Some(c) = cursor {
                                    if self.outer.classes[c]
                                        .fields
                                        .iter()
                                        .any(|(n, _, _)| n == name)
                                    {
                                        self.check_expr(value, None, sink);
                                        sink.note_unsupported(
                                            "field assignment",
                                            self.diag_span(*name_span),
                                        );
                                        return None;
                                    }
                                    cursor = self.outer.classes[c].parent;
                                }
                            }
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
                        // FUNC007 — Warning (Platform 09 §3.4): `start:`
                        // should return void; the value still checks.
                        if self.fn_name == "start" {
                            let v = self.check_expr(expr, None, sink);
                            sink.push(build(
                                Level::Warning,
                                codes::FUNC007,
                                "the start block should return void".to_string(),
                                self.diag_span(expr.span()),
                                Some("this value is discarded".to_string()),
                            ));
                            return Some(TStmt::Return {
                                value: Some(v),
                                span: *span,
                            });
                        }
                        let ret = self.ret.clone();
                        let v = self.check_expr(expr, Some(&ret), sink);
                        match self.coerce(v, &ret) {
                            Ok(v) => Some(v),
                            Err(v) => {
                                self.return_mismatch(&v, expr.span(), sink);
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
                self.check_on_error_block(on_error, sink);
                self.allow_signal = true;
                let value = self.check_expr(expr, None, sink);
                Some(TStmt::Expr(value))
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

    /// ERH-02 block form: the handler body checks with the `error`
    /// binding in scope; its lowering is a later milestone.
    fn check_on_error_block(&mut self, on_error: &Option<ast::Block>, sink: &mut DiagnosticSink) {
        if let Some(handler) = on_error {
            let saved = self.error_binding;
            self.error_binding = true;
            self.check_block(handler, sink);
            self.error_binding = saved;
            if let Some(first) = handler.first() {
                sink.note_unsupported("onError handler lowering", self.diag_span(stmt_span(first)));
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
        let signal_ok = std::mem::take(&mut self.allow_signal);
        let expected_resolved = expected.map(|t| self.infcx.resolve(t));
        let expected = expected_resolved.as_ref();
        match expr {
            // ERH-01: `error(message)` is a signal, not a value — legal
            // only directly in statement position.
            ast::Expr::Call { callee, args, .. }
                if matches!(callee.as_ref(), ast::Expr::ErrorRef { .. }) =>
            {
                if args.len() != 1 {
                    sink.push(build(
                        Level::Error,
                        codes::FUNC002,
                        format!("`error` expects 1 argument(s), got {}", args.len()),
                        self.diag_span(span),
                        None,
                    ));
                }
                let message = match args.first() {
                    Some(arg) => {
                        let value = self.check_expr(arg, Some(&Ty::Str), sink);
                        match self.coerce(value, &Ty::Str) {
                            Ok(value) => value,
                            Err(value) => {
                                let mut d = build(
                                    Level::Error,
                                    codes::SEM016,
                                    "argument `1` of `error` has the wrong type".to_string(),
                                    self.diag_span(arg.span()),
                                    Some(format!(
                                        "this argument has type `{}`",
                                        self.infcx.resolve(&value.ty).display()
                                    )),
                                );
                                d.notes
                                    .push("`error` takes one `string` argument".to_string());
                                self.push_rich(sink, d);
                                error_expr(arg.span())
                            }
                        }
                    }
                    None => error_expr(span),
                };
                if !signal_ok {
                    // SEM004 (stub rule; local wording): value position.
                    sink.push(build(
                        Level::Error,
                        codes::SEM004,
                        "`error(...)` is a signal, not a value".to_string(),
                        self.diag_span(span),
                        Some(
                            "a failure interrupts the expression — it cannot be assigned"
                                .to_string(),
                        ),
                    ));
                    return error_expr(span);
                }
                TExpr {
                    ty: Ty::Void,
                    span,
                    kind: TExprKind::Raise(Box::new(message)),
                }
            }
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
                    // SMG-02: `value` is the proposed new value, bound
                    // only inside a guard clause.
                    if name == "value" {
                        if let Some(ty) = &self.guard_value {
                            return TExpr {
                                ty: ty.clone(),
                                span,
                                kind: TExprKind::GuardValue,
                            };
                        }
                    }
                    // SMG-01: module state variables are in scope in
                    // every body of the module.
                    if let Some((ty, _)) = self.outer.state_vars[self.file].get(name) {
                        return TExpr {
                            ty: ty.clone(),
                            span,
                            kind: TExprKind::GetState {
                                module: self.file,
                                name: name.clone(),
                            },
                        };
                    }
                    // CLS-02 implicit context: a bare name inside a class
                    // body reaches the instance fields.
                    if let Some(this_class) = self.this_class {
                        let mut cursor = Some(this_class);
                        while let Some(c) = cursor {
                            if let Some(field) = self.outer.classes[c]
                                .fields
                                .iter()
                                .position(|(n, _, _)| n == name)
                            {
                                let ty = self.outer.classes[c].fields[field].1.clone();
                                let this = TExpr {
                                    ty: Ty::Class {
                                        class: this_class,
                                        name: self.outer.classes[this_class].name.clone(),
                                    },
                                    span,
                                    kind: TExprKind::This,
                                };
                                return TExpr {
                                    ty,
                                    span,
                                    kind: TExprKind::GetField {
                                        class: c,
                                        field,
                                        recv: Box::new(this),
                                    },
                                };
                            }
                            cursor = self.outer.classes[c].parent;
                        }
                    }
                    let is_callable = self.module_scope().functions.contains_key(name)
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
                    } else if let Some(decl_start) = self.declared_later(name, span.start) {
                        // SCOPE001 (stub rule; local wording): the
                        // declaration exists, lexically after this use.
                        let _ = decl_start;
                        sink.push(build(
                            Level::Error,
                            codes::SCOPE001,
                            format!("`{name}` is used before it is declared"),
                            self.diag_span(span),
                            Some("declared later in this scope".to_string()),
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
            ast::Expr::Member {
                receiver,
                name,
                span: member_span,
            } => self.check_member(receiver, name, *member_span, span, sink),
            ast::Expr::OnError {
                value, fallback, ..
            } => {
                let value = self.check_expr(value, expected, sink);
                let saved = self.error_binding;
                self.error_binding = true;
                let fb = self.check_expr(fallback, Some(&value.ty.clone()), sink);
                self.error_binding = saved;
                // ERH-02 leaves the result typing open (DISCOVERIES-M4
                // item 15): the fallback coerces to the guarded
                // expression's type.
                let ty = value.ty.clone();
                let fb = self.coerce_assign(fb, &ty, "onError", fallback.span(), sink);
                TExpr {
                    ty,
                    span,
                    kind: TExprKind::OnError {
                        value: Box::new(value),
                        fallback: Box::new(fb),
                    },
                }
            }
            ast::Expr::This { .. } => match self.this_class {
                Some(class) => TExpr {
                    ty: Ty::Class {
                        class,
                        name: self.outer.classes[class].name.clone(),
                    },
                    span,
                    kind: TExprKind::This,
                },
                None => {
                    // SEM004 (stub rule; local wording): `this` outside a
                    // class body.
                    sink.push(build(
                        Level::Error,
                        codes::SEM004,
                        "`this` is only available inside a class body".to_string(),
                        self.diag_span(span),
                        None,
                    ));
                    error_expr(span)
                }
            },
            ast::Expr::Base { .. } => {
                sink.note_unsupported("`base` constructor calls", self.diag_span(span));
                error_expr(span)
            }
            ast::Expr::ErrorRef { .. } => {
                if self.error_binding {
                    TExpr {
                        ty: error_record(),
                        span,
                        kind: TExprKind::ErrorBinding,
                    }
                } else {
                    // ERH-04: `error` is a binding only inside a handler;
                    // elsewhere the name simply is not in scope.
                    sink.push(build(
                        Level::Error,
                        codes::SEM002,
                        "I cannot find a variable named `error` in scope".to_string(),
                        self.diag_span(span),
                        Some("`error` is bound only inside an `onError` handler".to_string()),
                    ));
                    error_expr(span)
                }
            }
            ast::Expr::ResultRef { .. } => {
                // CTR-02: `result` is the return value, in scope only
                // inside an `after:` expression.
                if self.in_contract == Some(ContractKind::After) {
                    TExpr {
                        ty: self.ret.clone(),
                        span,
                        kind: TExprKind::ResultRef,
                    }
                } else {
                    // CLASS008 — template from Platform 10 §6.
                    sink.push(build(
                        Level::Error,
                        codes::CLASS008,
                        "'result' is only in scope inside an 'after:' expression".to_string(),
                        self.diag_span(span),
                        Some("'result' is not in scope here".to_string()),
                    ));
                    error_expr(span)
                }
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
                let key_ty = (**key).clone();
                if self.fit(&index_t.ty, &key_ty) == Fit::No {
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
            // Chapter 16: `recv.op(args)` — method style and namespace
            // style are one syntactic shape; the receiver decides.
            if let ast::Expr::Member {
                receiver,
                name: method,
                span: member_span,
            } = callee
            {
                return self.check_dot_call(receiver, method, *member_span, args, span, sink);
            }
            // ERH-01's `error(message)` failure signal has its own
            // frontier note (typed in the M4 error-handling stage).
            let construct = if matches!(callee, ast::Expr::ErrorRef { .. }) {
                "`error(...)` failure signals"
            } else {
                "method-style calls"
            };
            sink.note_unsupported(construct, self.diag_span(callee.span()));
            return error_expr(span);
        };

        // CLASS011 — template from Platform 10 §6: a capability name in
        // constructor position.
        if self
            .module_scope()
            .capabilities
            .get(name)
            .copied()
            .is_some()
            && self.lookup(name).is_none()
        {
            sink.push(build(
                Level::Error,
                codes::CLASS011,
                format!("'{name}' is a capability and cannot be instantiated"),
                self.diag_span(*callee_span),
                Some("capabilities have no bodies".to_string()),
            ));
            return error_expr(span);
        }

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

        // User function? (Visible through this module's scope, MOD-01/02.)
        if let Some(index) = self.module_scope().functions.get(name).copied() {
            let (params, ret, required): (Vec<Ty>, Ty, usize) = {
                let sig = &self.outer.function_sigs[index];
                (
                    sig.params.iter().map(|p| p.ty.clone()).collect(),
                    sig.ret.clone(),
                    sig.required,
                )
            };
            if args.len() < required || args.len() > params.len() {
                let expected = if required == params.len() {
                    format!("{}", params.len())
                } else {
                    format!("between {required} and {}", params.len())
                };
                sink.push(build(
                    Level::Error,
                    codes::FUNC002,
                    format!(
                        "`{name}` expects {expected} argument(s), got {}",
                        args.len()
                    ),
                    self.diag_span(span),
                    None,
                ));
            }
            let mut typed = self.check_args_against(name, &params, args, sink);
            // FNC-04: omitted trailing arguments take their defaults,
            // materialised at the call site (fresh per call).
            for i in typed.len()..params.len() {
                if let Some(default) = self
                    .outer
                    .function_sigs
                    .get(index)
                    .and_then(|s| s.defaults.get(i))
                    .and_then(|d| d.clone())
                {
                    typed.push(default);
                }
            }
            return TExpr {
                ty: ret,
                span,
                kind: TExprKind::CallFn {
                    func: index,
                    args: typed,
                },
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

        // Class instantiation? (`Options(false)` — CLS-02/CLASS004.)
        if let Some(&class_idx) = self.module_scope().classes.get(name) {
            let instance = Ty::Class {
                class: class_idx,
                name: name.clone(),
            };
            if let Some(expected_record @ Ty::Record { .. }) = expected {
                if self.fit(&instance, expected_record) == Fit::No {
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
            let ctors = self.outer.classes[class_idx].ctors.clone();
            if ctors.is_empty() {
                // CLASS004's implicit constructor: positional over the
                // boundary record fields (M1 shape preserved).
                let record = self
                    .outer
                    .class_records
                    .get(class_idx)
                    .cloned()
                    .unwrap_or(Ty::Error);
                let params: Vec<Ty> = match &record {
                    Ty::Record { fields, .. } => fields.iter().map(|(_, t)| t.clone()).collect(),
                    _ => Vec::new(),
                };
                let args = self.check_args(name, &params, args, span, false, sink);
                return TExpr {
                    ty: instance,
                    span,
                    kind: TExprKind::MakeRecord(args),
                };
            }
            let Some(ctor) = ctors.iter().position(|c| c.len() == args.len()) else {
                // CLASS004 (stub rule; local wording).
                sink.push(build(
                    Level::Error,
                    codes::CLASS004,
                    format!(
                        "no constructor of `{name}` takes {} argument(s)",
                        args.len()
                    ),
                    self.diag_span(span),
                    Some("no matching constructor".to_string()),
                ));
                return error_expr(span);
            };
            let params = ctors[ctor].clone();
            let args = self.check_args(name, &params, args, span, false, sink);
            return TExpr {
                ty: instance,
                span,
                kind: TExprKind::CallCtor {
                    class: class_idx,
                    ctor,
                    args,
                },
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

    /// One `recv.op(args)` call (chapter 16): the LHS being a value picks
    /// method dispatch; a class name picks static dispatch; a module name
    /// or alias picks a namespace function; a standalone function name is
    /// FUNC012; anything unresolvable module-shaped is SEM021.
    fn check_dot_call(
        &mut self,
        receiver: &ast::Expr,
        method: &str,
        member_span: ByteSpan,
        args: &[ast::Expr],
        span: ByteSpan,
        sink: &mut DiagnosticSink,
    ) -> TExpr {
        // SCOPE006 — template from Platform 10 §4: the `test.compiletime`
        // namespace exists only inside a `tests:` block body.
        if let ast::Expr::Member {
            receiver: inner,
            name: mid,
            ..
        } = receiver
        {
            if mid == "compiletime" {
                if let ast::Expr::Ident { name: root, .. } = inner.as_ref() {
                    if root == "test" && self.lookup(root).is_none() {
                        if self.in_tests {
                            sink.note_unsupported("test.compiletime helpers", self.diag_span(span));
                        } else {
                            sink.push(build(
                                Level::Error,
                                codes::SCOPE006,
                                format!(
                                    "'test.compiletime.{method}' is only available inside a 'tests:' block"
                                ),
                                self.diag_span(member_span),
                                Some("not available here".to_string()),
                            ));
                        }
                        return error_expr(span);
                    }
                }
            }
        }
        if let ast::Expr::Ident {
            name,
            span: recv_span,
        } = receiver
        {
            if self.lookup(name).is_none() {
                // Not a local value: class, module, builtin, or function?
                if let Some(&class) = self.module_scope().classes.get(name) {
                    return self.check_static_call(class, method, member_span, args, span, sink);
                }
                if let Some(&target) = self.module_scope().module_aliases.get(name) {
                    return self.check_namespace_call(
                        target,
                        method,
                        member_span,
                        args,
                        span,
                        sink,
                    );
                }
                if crate::resolver::BUILTIN_MODULES.contains(&name.as_str()) {
                    sink.note_unsupported("standard-library calls", self.diag_span(span));
                    return error_expr(span);
                }
                if self.module_scope().functions.contains_key(name)
                    || self
                        .outer
                        .host_imports
                        .iter()
                        .any(|h| h.clean_name == *name)
                {
                    // FUNC012 (stub rule; local wording): dot-notation on
                    // a symbol that is a standalone function.
                    sink.push(build(
                        Level::Error,
                        codes::FUNC012,
                        format!("`{name}` is a function, not a value or module — call it directly"),
                        self.diag_span(*recv_span),
                        Some("no method call on a standalone function".to_string()),
                    ));
                    return error_expr(span);
                }
                // SEM021 — template from Platform 10 §3.
                sink.push(build(
                    Level::Error,
                    codes::SEM021,
                    format!("I cannot resolve the module `{name}`"),
                    self.diag_span(*recv_span),
                    Some("no source or library provides this module".to_string()),
                ));
                return error_expr(span);
            }
        }
        // The receiver is a value: dispatch on its type.
        let recv = self.check_expr(receiver, None, sink);
        let resolved = self.infcx.resolve(&recv.ty);
        match resolved {
            Ty::Class { class, .. } => {
                let Some((owner, m)) = self.outer.find_method(class, method) else {
                    return self.undefined_method(&recv.ty, method, member_span, span, sink);
                };
                let (params, ret, public) = {
                    let sig = &self.outer.classes[owner].methods[m];
                    (sig.params.clone(), sig.ret.clone(), sig.public)
                };
                // SEM005 — template from Platform 10 §3: private by
                // default; visibility is module-scoped (MOD-02).
                let owner_file = self.outer.resolved.decls.classes[owner].coords.0;
                if !public && owner_file != self.file {
                    let scope = self.outer.classes[owner].name.clone();
                    sink.push(build(
                        Level::Error,
                        codes::SEM005,
                        format!(
                            "'{method}' is private and cannot be accessed from outside '{scope}'"
                        ),
                        self.diag_span(member_span),
                        Some("not inside a `public:` wrapper".to_string()),
                    ));
                }
                let args = self.check_args(method, &params, args, span, false, sink);
                TExpr {
                    ty: ret,
                    span,
                    kind: TExprKind::CallMethod {
                        class: owner,
                        method: m,
                        recv: Box::new(recv),
                        args,
                    },
                }
            }
            Ty::Cap { cap, .. } => {
                let coords = self.outer.resolved.decls.capabilities[cap].coords;
                let (capability, cap_file) = self.outer.resolved.capability(coords);
                let Some(sig) = capability.signatures.iter().find(|s| s.name == method) else {
                    return self.undefined_method(&recv.ty, method, member_span, span, sink);
                };
                let params: Vec<Ty> = sig
                    .params
                    .iter()
                    .map(|p| {
                        self.outer
                            .project_type(&p.ty, TyPos::Surface, cap_file, sink)
                    })
                    .collect();
                let ret = self
                    .outer
                    .project_type(&sig.ret, TyPos::Surface, cap_file, sink);
                let method = method.to_string();
                let args = self.check_args(&method, &params, args, span, false, sink);
                TExpr {
                    ty: ret,
                    span,
                    kind: TExprKind::CallDyn {
                        cap: Some(cap),
                        method,
                        recv: Box::new(recv),
                        args,
                    },
                }
            }
            Ty::Any => {
                // TYP-02: checking is skipped; arguments still check.
                let args = args
                    .iter()
                    .map(|arg| self.check_expr(arg, None, sink))
                    .collect();
                TExpr {
                    ty: Ty::Any,
                    span,
                    kind: TExprKind::CallDyn {
                        cap: None,
                        method: method.to_string(),
                        recv: Box::new(recv),
                        args,
                    },
                }
            }
            Ty::Error => error_expr(span),
            other => {
                // SEM010 (stub rule; local wording): `string.matches()`
                // takes one of the fourteen named pattern constants (15
                // §String Patterns) — an identifier check at compile
                // time, no runtime lookup (ADR-0009).
                if other == Ty::Str && method == "matches" {
                    const PATTERNS: [&str; 14] = [
                        "emailPattern",
                        "urlPattern",
                        "phonePattern",
                        "uuidPattern",
                        "integerPattern",
                        "numberPattern",
                        "alphanumericPattern",
                        "slugPattern",
                        "datePattern",
                        "timePattern",
                        "ipv4Pattern",
                        "hexColorPattern",
                        "alphaPattern",
                        "numericPattern",
                    ];
                    let valid = args.len() == 1
                        && matches!(&args[0], ast::Expr::Ident { name, .. }
                            if PATTERNS.contains(&name.as_str()));
                    if !valid {
                        let arg_span = args.first().map(|a| a.span()).unwrap_or(member_span);
                        sink.push(build(
                            Level::Error,
                            codes::SEM010,
                            "the argument to `string.matches()` must be a named pattern constant"
                                .to_string(),
                            self.diag_span(arg_span),
                            Some(
                                "the pattern vocabulary is closed — no strings, no variables"
                                    .to_string(),
                            ),
                        ));
                        return error_expr(span);
                    }
                    sink.note_unsupported("standard-library methods", self.diag_span(span));
                    return error_expr(span);
                }
                // TYP-06's four explicit conversions are typed here; the
                // rest of the built-in method surface is chapter 15 (M6).
                let target = match method {
                    "toInteger" => Some(Ty::Integer),
                    "toNumber" => Some(Ty::Number),
                    "toString" => Some(Ty::Str),
                    "toBoolean" => Some(Ty::Boolean),
                    _ => None,
                };
                let convertible = other.is_numeric() || matches!(other, Ty::Str | Ty::Boolean);
                if let (Some(target), true) = (&target, convertible) {
                    if !args.is_empty() {
                        sink.push(build(
                            Level::Error,
                            codes::FUNC002,
                            format!("`{method}` expects 0 argument(s), got {}", args.len()),
                            self.diag_span(span),
                            None,
                        ));
                    }
                    return TExpr {
                        ty: target.clone(),
                        span,
                        kind: TExprKind::Convert(Box::new(recv)),
                    };
                }
                sink.note_unsupported("standard-library methods", self.diag_span(span));
                error_expr(span)
            }
        }
    }

    /// Member access without a call (CLS-04 field access; chapter 16
    /// leaves `module.symbol` and companion bare-access to their owners:
    /// SYN010/SEM021/CLASS012).
    fn check_member(
        &mut self,
        receiver: &ast::Expr,
        name: &str,
        member_span: ByteSpan,
        span: ByteSpan,
        sink: &mut DiagnosticSink,
    ) -> TExpr {
        if let ast::Expr::Ident {
            name: recv_name,
            span: recv_span,
        } = receiver
        {
            if self.lookup(recv_name).is_none() {
                if let Some(&class) = self.module_scope().classes.get(recv_name) {
                    // CLS-05: `Outer.fieldName` yields a type used as a
                    // namespace — never a value. Bare access is CLASS012.
                    let receiver_name = self.outer.classes[class].name.clone();
                    sink.push(build(
                        Level::Error,
                        codes::CLASS012,
                        format!(
                            "'{receiver_name}.{name}' is not a valid companion access: a companion is a namespace, not a value"
                        ),
                        self.diag_span(member_span),
                        Some("invalid companion access".to_string()),
                    ));
                    return error_expr(span);
                }
                if let Some(&target) = self.module_scope().module_aliases.get(recv_name) {
                    let is_function = self.outer.resolved.decls.modules[target]
                        .functions
                        .get(name)
                        .copied()
                        .is_some_and(|i| {
                            let decl = &self.outer.resolved.decls.functions[i];
                            decl.public && decl.coords.0 == target
                        });
                    if is_function {
                        // FNC-05: every call carries parentheses.
                        sink.push(build(
                            Level::Error,
                            codes::SYN010,
                            format!("Call to '{name}' is missing parentheses"),
                            self.diag_span(member_span),
                            Some("every call carries parentheses".to_string()),
                        ));
                    } else {
                        sink.push(build(
                            Level::Error,
                            codes::SEM019,
                            format!("I cannot find a function named `{name}`"),
                            self.diag_span(member_span),
                            Some("no function with this name is in scope".to_string()),
                        ));
                    }
                    return error_expr(span);
                }
                if crate::resolver::BUILTIN_MODULES.contains(&recv_name.as_str()) {
                    sink.note_unsupported("standard-library constants", self.diag_span(span));
                    return error_expr(span);
                }
                sink.push(build(
                    Level::Error,
                    codes::SEM021,
                    format!("I cannot resolve the module `{recv_name}`"),
                    self.diag_span(*recv_span),
                    Some("no source or library provides this module".to_string()),
                ));
                return error_expr(span);
            }
        }
        let recv = self.check_expr(receiver, None, sink);
        let resolved = self.infcx.resolve(&recv.ty);
        match resolved {
            Ty::Class { class, .. } => {
                // Field lookup walks the parent chain (CLS-02).
                let mut cursor = Some(class);
                while let Some(c) = cursor {
                    if let Some(field) = self.outer.classes[c]
                        .fields
                        .iter()
                        .position(|(n, _, _)| n == name)
                    {
                        let (_, ty, public) = self.outer.classes[c].fields[field].clone();
                        let owner_file = self.outer.resolved.decls.classes[c].coords.0;
                        if !public && owner_file != self.file {
                            let scope = self.outer.classes[c].name.clone();
                            sink.push(build(
                                Level::Error,
                                codes::SEM005,
                                format!(
                                    "'{name}' is private and cannot be accessed from outside '{scope}'"
                                ),
                                self.diag_span(member_span),
                                Some("not inside a `public:` wrapper".to_string()),
                            ));
                        }
                        return TExpr {
                            ty,
                            span,
                            kind: TExprKind::GetField {
                                class: c,
                                field,
                                recv: Box::new(recv),
                            },
                        };
                    }
                    cursor = self.outer.classes[c].parent;
                }
                // No registered code exists for a missing FIELD; SEM022
                // (UndefinedMethod) is the nearest member-miss code — the
                // gap is recorded in DISCOVERIES-M4.
                self.undefined_method(&recv.ty, name, member_span, span, sink)
            }
            Ty::Any => {
                sink.note_unsupported("member access on `any` values", self.diag_span(span));
                error_expr(span)
            }
            Ty::Record { fields, .. } => {
                let Some(index) = fields.iter().position(|(n, _)| n == name) else {
                    return self.undefined_method(&recv.ty, name, member_span, span, sink);
                };
                let ty = fields[index].1.clone();
                TExpr {
                    ty,
                    span,
                    kind: TExprKind::GetRecordField {
                        recv: Box::new(recv),
                        field: index,
                    },
                }
            }
            Ty::Error => error_expr(span),
            _ => {
                // The chapter-15 property surface (`.length`, …) is M6.
                sink.note_unsupported("standard-library methods", self.diag_span(span));
                error_expr(span)
            }
        }
    }

    /// SEM022 — template from Platform 10 §3.
    fn undefined_method(
        &mut self,
        recv_ty: &Ty,
        method: &str,
        member_span: ByteSpan,
        span: ByteSpan,
        sink: &mut DiagnosticSink,
    ) -> TExpr {
        let display = self.infcx.resolve(recv_ty).display();
        sink.push(build(
            Level::Error,
            codes::SEM022,
            format!("type `{display}` has no method named `{method}`"),
            self.diag_span(member_span),
            Some("no method with this name is defined on the receiver".to_string()),
        ));
        error_expr(span)
    }

    /// `ClassName.method(args)` (14 §Static Methods). The
    /// no-instance-field-access rule needs body analysis and is deferred
    /// (recorded in DISCOVERIES-M4); unknown members are CLASS012.
    fn check_static_call(
        &mut self,
        class: usize,
        method: &str,
        member_span: ByteSpan,
        args: &[ast::Expr],
        span: ByteSpan,
        sink: &mut DiagnosticSink,
    ) -> TExpr {
        let Some((owner, m)) = self.outer.find_method(class, method) else {
            // CLASS012 — template from Platform 10 §6.
            let receiver = self.outer.classes[class].name.clone();
            sink.push(build(
                Level::Error,
                codes::CLASS012,
                format!(
                    "'{receiver}.{method}' is not a valid companion access: `{receiver}` has no method or companion field named `{method}`"
                ),
                self.diag_span(member_span),
                Some("invalid companion access".to_string()),
            ));
            return error_expr(span);
        };
        let (params, ret) = {
            let sig = &self.outer.classes[owner].methods[m];
            (sig.params.clone(), sig.ret.clone())
        };
        let args = self.check_args(method, &params, args, span, false, sink);
        TExpr {
            ty: ret,
            span,
            kind: TExprKind::CallStatic {
                class: owner,
                method: m,
                args,
            },
        }
    }

    /// `module.function(args)` namespace style (chapter 16): resolves to
    /// the target module's own public functions.
    fn check_namespace_call(
        &mut self,
        target: usize,
        method: &str,
        member_span: ByteSpan,
        args: &[ast::Expr],
        span: ByteSpan,
        sink: &mut DiagnosticSink,
    ) -> TExpr {
        let function = self.outer.resolved.decls.modules[target]
            .functions
            .get(method)
            .copied()
            .filter(|&i| {
                let decl = &self.outer.resolved.decls.functions[i];
                decl.public && decl.coords.0 == target
            });
        let Some(index) = function else {
            sink.push(build(
                Level::Error,
                codes::SEM019,
                format!("I cannot find a function named `{method}`"),
                self.diag_span(member_span),
                Some("no function with this name is in scope".to_string()),
            ));
            return error_expr(span);
        };
        let (params, ret): (Vec<Ty>, Ty) = {
            let sig = &self.outer.function_sigs[index];
            (
                sig.params.iter().map(|p| p.ty.clone()).collect(),
                sig.ret.clone(),
            )
        };
        let args = self.check_args(method, &params, args, span, false, sink);
        TExpr {
            ty: ret,
            span,
            kind: TExprKind::CallFn { func: index, args },
        }
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
        self.check_args_core(fn_name, params, args, host_boundary, sink)
    }

    /// Positional argument checking without the arity report (the caller
    /// owns FUNC002 when defaults widen the legal range).
    fn check_args_against(
        &mut self,
        fn_name: &str,
        params: &[Ty],
        args: &[ast::Expr],
        sink: &mut DiagnosticSink,
    ) -> Vec<TExpr> {
        self.check_args_core(fn_name, params, args, false, sink)
    }

    fn check_args_core(
        &mut self,
        fn_name: &str,
        params: &[Ty],
        args: &[ast::Expr],
        host_boundary: bool,
        sink: &mut DiagnosticSink,
    ) -> Vec<TExpr> {
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

/// Every path through the block ends in a `return` (conservative: `if`
/// needs an `else` and all arms terminating; loops never count).
fn terminates(block: &[TStmt]) -> bool {
    match block.last() {
        Some(TStmt::Return { .. }) => true,
        Some(TStmt::If {
            then,
            else_ifs,
            els: Some(els),
            ..
        }) => terminates(then) && else_ifs.iter().all(|(_, b)| terminates(b)) && terminates(els),
        _ => false,
    }
}

/// TST-01: literals and compositions of literals are compile-time
/// evaluable (SEM024's conservative reading; anything with a name or a
/// call is not).
fn is_compile_time_constant(expr: &ast::Expr) -> bool {
    match expr {
        ast::Expr::Int { .. }
        | ast::Expr::Number { .. }
        | ast::Expr::Bool { .. }
        | ast::Expr::NoneLit { .. } => true,
        ast::Expr::Str { segments, .. } => segments
            .iter()
            .all(|seg| matches!(seg, ast::StrSeg::Text(_))),
        ast::Expr::List { items, .. } => items.iter().all(is_compile_time_constant),
        ast::Expr::Unary { operand, .. } => is_compile_time_constant(operand),
        ast::Expr::Binary { lhs, rhs, .. } => {
            is_compile_time_constant(lhs) && is_compile_time_constant(rhs)
        }
        _ => false,
    }
}

/// Names mentioned anywhere in a statement — SMG-05's static dependency
/// probe for computed state.
fn collect_idents(stmt: &ast::Stmt, out: &mut Vec<String>) {
    fn expr(e: &ast::Expr, out: &mut Vec<String>) {
        match e {
            ast::Expr::Ident { name, .. } => out.push(name.clone()),
            ast::Expr::Call { callee, args, .. } => {
                expr(callee, out);
                args.iter().for_each(|a| expr(a, out));
            }
            ast::Expr::Member { receiver, .. } => expr(receiver, out),
            ast::Expr::Index {
                receiver, index, ..
            } => {
                expr(receiver, out);
                expr(index, out);
            }
            ast::Expr::NonNone { operand, .. } | ast::Expr::Unary { operand, .. } => {
                expr(operand, out)
            }
            ast::Expr::Binary { lhs, rhs, .. } => {
                expr(lhs, out);
                expr(rhs, out);
            }
            ast::Expr::OnError {
                value, fallback, ..
            } => {
                expr(value, out);
                expr(fallback, out);
            }
            ast::Expr::List { items, .. } => items.iter().for_each(|i| expr(i, out)),
            ast::Expr::Str { segments, .. } => {
                for seg in segments {
                    if let ast::StrSeg::Interp { expr: e, .. } = seg {
                        expr(e, out);
                    }
                }
            }
            _ => {}
        }
    }
    use ast::Stmt::*;
    match stmt {
        VarDecl {
            init: Some(init), ..
        } => expr(init, out),
        VarDecl { init: None, .. } => {}
        Assign { target, value, .. } => {
            expr(target, out);
            expr(value, out);
        }
        Return {
            value: Some(value), ..
        } => expr(value, out),
        Return { value: None, .. } => {}
        Expr { expr: e, .. } => expr(e, out),
        If {
            cond,
            then,
            else_ifs,
            els,
            ..
        } => {
            expr(cond, out);
            then.iter().for_each(|s| collect_idents(s, out));
            for (c, b) in else_ifs {
                expr(c, out);
                b.iter().for_each(|s| collect_idents(s, out));
            }
            if let Some(els) = els {
                els.iter().for_each(|s| collect_idents(s, out));
            }
        }
        While { cond, body, .. } => {
            expr(cond, out);
            body.iter().for_each(|s| collect_idents(s, out));
        }
        Iterate {
            source, step, body, ..
        } => {
            match source {
                ast::IterateSource::Range { from, to } => {
                    expr(from, out);
                    expr(to, out);
                }
                ast::IterateSource::Expr(e) => expr(e, out),
            }
            if let Some(step) = step {
                expr(step, out);
            }
            body.iter().for_each(|s| collect_idents(s, out));
        }
        Print { items, .. } => items.iter().for_each(|e| expr(e, out)),
        Assert { expr: e, .. } => expr(e, out),
        _ => {}
    }
}

/// The child expressions of a node, for purity walks.
fn purity_children(expr: &TExpr) -> Vec<&TExpr> {
    match &expr.kind {
        TExprKind::MakeRecord(items)
        | TExprKind::MakeList(items)
        | TExprKind::MakeMatrix(items)
        | TExprKind::CallHost { args: items, .. }
        | TExprKind::CallFn { args: items, .. }
        | TExprKind::CallStatic { args: items, .. }
        | TExprKind::CallCtor { args: items, .. } => items.iter().collect(),
        TExprKind::CallMethod { recv, args, .. } | TExprKind::CallDyn { recv, args, .. } => {
            std::iter::once(recv.as_ref()).chain(args.iter()).collect()
        }
        TExprKind::Binary { lhs, rhs, .. } => vec![lhs, rhs],
        TExprKind::Unary { operand, .. }
        | TExprKind::NonNone(operand)
        | TExprKind::IsNone { operand, .. }
        | TExprKind::IntToNumber(operand)
        | TExprKind::WrapSome(operand)
        | TExprKind::Convert(operand)
        | TExprKind::GetField { recv: operand, .. } => vec![operand],
        TExprKind::Index { recv, index, .. } => vec![recv, index],
        TExprKind::Raise(operand) | TExprKind::GetRecordField { recv: operand, .. } => {
            vec![operand]
        }
        TExprKind::OnError { value, fallback } => vec![value, fallback],
        TExprKind::StrInterp(segs) => segs
            .iter()
            .filter_map(|seg| match seg {
                TInterpSeg::Expr(e) => Some(e),
                TInterpSeg::Text(_) => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn error_expr(span: ByteSpan) -> TExpr {
    TExpr {
        ty: Ty::Error,
        span,
        kind: TExprKind::Error,
    }
}
