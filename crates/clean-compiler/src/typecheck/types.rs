//! Semantic types for the Milestone 1 surface, including the boundary-only
//! projections of world-declared WIT types (ADR-0002 §3).

use crate::parser::ast::IntWidth;

#[derive(Debug, Clone, PartialEq)]
pub enum Ty {
    /// Surface `integer` — s64 (TYP-01).
    Integer,
    /// Width-suffixed boundary integer (`integer:32`, `integer:u32`, …) —
    /// valid only in host-function positions (LBS-02).
    IntegerW(IntWidth),
    Boolean,
    Str,
    /// `bytes` ↔ WIT `list<u8>` (LBS-02).
    Bytes,
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
    /// `list<T>` for element types other than `u8` (which is `Bytes`).
    List(Box<Ty>),
    /// A type error already reported; absorbs further checks silently.
    Error,
}

impl Ty {
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
            Ty::Boolean => "boolean".to_string(),
            Ty::Str => "string".to_string(),
            Ty::Bytes => "bytes".to_string(),
            Ty::Void => "void".to_string(),
            Ty::Option(inner) => format!("{}?", inner.display()),
            Ty::Enum { wit_name, .. } => wit_name.clone(),
            Ty::Record { wit_name, .. } => wit_name.clone(),
            Ty::List(inner) => format!("list<{}>", inner.display()),
            Ty::Error => "<error>".to_string(),
        }
    }
}

/// Projects a WIT type onto the M1 semantic types — the boundary reading of
/// the LBS-02 table. Shared by the type checker (declaration projection) and
/// the World Import Check (signature comparison), so the two can never
/// disagree. `None` marks a WIT shape outside the M1 surface.
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
                TypeDefKind::List(inner) => Ty::List(Box::new(project_wit(resolve, inner)?)),
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
