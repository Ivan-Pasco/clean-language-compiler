//! Semantic types for the chapter-04 surface (TYP-01..05), including the
//! boundary-only projections of world-declared WIT types (ADR-0002 §3).
//!
//! `Ty::Var` is an inference variable owned by `infer::InferCtx`; every type
//! that leaves pass [5] has been resolved (`InferCtx::finalize`), so later
//! passes never see one.

use serde::Serialize;

use crate::parser::ast::IntWidth;

/// An inference variable key (`ena` union-find). Only `infer::InferCtx`
/// creates and resolves these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct TyVid(pub u32);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Ty {
    /// Surface `integer` — s64 (TYP-01).
    Integer,
    /// Width-suffixed boundary integer (`integer:32`, `integer:u32`, …) —
    /// valid only in host-function positions (LBS-02); anywhere else the
    /// checker reports SEM009 (TYP-01: no width modifiers in Clean types).
    IntegerW(IntWidth),
    /// Surface `number` — IEEE-754 binary64 (TYP-01).
    Number,
    Boolean,
    Str,
    /// `bytes` ↔ WIT `list<u8>` (LBS-02). No literal (TYP-01).
    Bytes,
    /// `datetime` — no literal, stdlib-constructed (TYP-01).
    Datetime,
    /// `any` — the compile-time generic escape hatch: the compiler skips
    /// type checking for the value (TYP-02).
    Any,
    Void,
    Option(Box<Ty>),
    /// A world-declared enum, projected for call-site checking. Cases are in
    /// WIT declaration order; the value representation is the case index.
    Enum {
        wit_name: String,
        cases: Vec<String>,
    },
    /// A record shape — either world-declared or projected from a Clean
    /// class (kebab-cased names, LBS-02 class↔record). Fields are in
    /// declaration order.
    Record {
        wit_name: String,
        fields: Vec<(String, Ty)>,
    },
    /// `list<T>` for element types other than `u8` (which is `Bytes`). The
    /// behavior chain is part of the type, fixed at declaration (TYP-05):
    /// `list<string>` and `list<string>.line` are different types.
    List(Box<Ty>, ListBehavior),
    /// `matrix<T>` — a two-dimensional list of lists (TYP-02).
    Matrix(Box<Ty>),
    /// `pairs<K, V>` — Clean's map type; `K` is a free type parameter
    /// (TYP-02, IDX003).
    Pairs(Box<Ty>, Box<Ty>),
    /// A class instance, nominal (CLS-02): `class` indexes
    /// `Declarations::classes`. Distinct from the structural `Record`
    /// boundary projection (LBS-02), which class values take only when
    /// they cross the host boundary.
    Class {
        class: usize,
        name: String,
    },
    /// A capability used as a type (CLS-03): values dispatch dynamically;
    /// `cap` indexes `Declarations::capabilities`.
    Cap {
        cap: usize,
        name: String,
    },
    /// An unresolved inference variable (bidirectional checking).
    Var(TyVid),
    /// A type error already reported; absorbs further checks silently.
    Error,
}

/// The TYP-05 behavior axes: one removal discipline (`.line` FIFO or
/// `.pile` LIFO) and independent `.unique` membership.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct ListBehavior {
    pub removal: Option<Removal>,
    pub unique: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Removal {
    Line,
    Pile,
}

impl ListBehavior {
    pub const NONE: ListBehavior = ListBehavior {
        removal: None,
        unique: false,
    };

    /// Canonical display order: removal discipline, then `.unique`
    /// (declaration order is free, TYP-05; the rendered name is canonical).
    fn display(&self) -> String {
        let mut out = String::new();
        match self.removal {
            Some(Removal::Line) => out.push_str(".line"),
            Some(Removal::Pile) => out.push_str(".pile"),
            None => {}
        }
        if self.unique {
            out.push_str(".unique");
        }
        out
    }
}

impl Ty {
    /// Plain `list<T>` with no behaviors — the shape every non-declaration
    /// position produces.
    pub fn list(elem: Ty) -> Ty {
        Ty::List(Box::new(elem), ListBehavior::NONE)
    }

    /// The inclusive value range of an integer type, for SEM026.
    pub fn integer_range(&self) -> Option<(i128, i128)> {
        match self {
            Ty::Integer => Some((i64::MIN as i128, i64::MAX as i128)),
            Ty::IntegerW(IntWidth::S32) => Some((i32::MIN as i128, i32::MAX as i128)),
            Ty::IntegerW(IntWidth::U8) => Some((0, u8::MAX as i128)),
            Ty::IntegerW(IntWidth::U16) => Some((0, u16::MAX as i128)),
            Ty::IntegerW(IntWidth::U32) => Some((0, u32::MAX as i128)),
            Ty::IntegerW(IntWidth::U64) => Some((0, u64::MAX as i128)),
            _ => None,
        }
    }

    pub fn is_integer(&self) -> bool {
        matches!(self, Ty::Integer | Ty::IntegerW(_))
    }

    /// `integer` or `number` — the domain of the arithmetic operators
    /// (06 §Operators on built-in types).
    pub fn is_numeric(&self) -> bool {
        self.is_integer() || matches!(self, Ty::Number)
    }

    /// Rendered name for diagnostics, using surface-language spelling.
    pub fn display(&self) -> String {
        match self {
            Ty::Integer => "integer".to_string(),
            Ty::IntegerW(w) => format!(
                "integer:{}",
                match w {
                    IntWidth::S32 => "32",
                    IntWidth::U8 => "u8",
                    IntWidth::U16 => "u16",
                    IntWidth::U32 => "u32",
                    IntWidth::U64 => "u64",
                }
            ),
            Ty::Number => "number".to_string(),
            Ty::Boolean => "boolean".to_string(),
            Ty::Str => "string".to_string(),
            Ty::Bytes => "bytes".to_string(),
            Ty::Datetime => "datetime".to_string(),
            Ty::Any => "any".to_string(),
            Ty::Void => "void".to_string(),
            Ty::Option(inner) => format!("{}?", inner.display()),
            Ty::Enum { wit_name, .. } => wit_name.clone(),
            Ty::Record { wit_name, .. } => wit_name.clone(),
            Ty::List(inner, behavior) => {
                format!("list<{}>{}", inner.display(), behavior.display())
            }
            Ty::Matrix(inner) => format!("matrix<{}>", inner.display()),
            Ty::Pairs(key, value) => {
                format!("pairs<{}, {}>", key.display(), value.display())
            }
            Ty::Class { name, .. } => name.clone(),
            Ty::Cap { name, .. } => name.clone(),
            Ty::Var(_) => "_".to_string(),
            Ty::Error => "<error>".to_string(),
        }
    }
}

/// Projects a WIT type onto the semantic types — the boundary reading of
/// the LBS-02 table. Shared by the type checker (declaration projection) and
/// the World Import Check (signature comparison), so the two can never
/// disagree. `None` marks a WIT shape outside the supported surface.
pub fn project_wit(resolve: &wit_parser::Resolve, ty: &wit_parser::Type) -> Option<Ty> {
    use wit_parser::Type as W;
    use wit_parser::TypeDefKind;
    Some(match ty {
        W::Bool => Ty::Boolean,
        W::U8 => Ty::IntegerW(IntWidth::U8),
        W::U16 => Ty::IntegerW(IntWidth::U16),
        W::U32 => Ty::IntegerW(IntWidth::U32),
        W::U64 => Ty::IntegerW(IntWidth::U64),
        W::S32 => Ty::IntegerW(IntWidth::S32),
        W::S64 => Ty::Integer,
        W::F64 => Ty::Number,
        W::String => Ty::Str,
        W::Id(id) => {
            let def = &resolve.types[*id];
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
                        .map(|f| Some((f.name.clone(), project_wit(resolve, &f.ty)?)))
                        .collect::<Option<Vec<_>>>()?,
                },
                TypeDefKind::List(W::U8) => Ty::Bytes,
                TypeDefKind::List(inner) => Ty::list(project_wit(resolve, inner)?),
                TypeDefKind::Option(inner) => Ty::Option(Box::new(project_wit(resolve, inner)?)),
                _ => return None,
            }
        }
        _ => return None,
    })
}

/// camelCase → kebab-case, the LBS-02 name projection (`setInnerHTML` →
/// `set-inner-html`).
pub fn kebab(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    let mut prev_lower = false;
    for ch in name.chars() {
        if ch.is_ascii_uppercase() {
            if prev_lower {
                out.push('-');
            }
            out.push(ch.to_ascii_lowercase());
            prev_lower = false;
        } else {
            prev_lower = ch.is_ascii_lowercase() || ch.is_ascii_digit();
            out.push(ch);
        }
    }
    out
}
