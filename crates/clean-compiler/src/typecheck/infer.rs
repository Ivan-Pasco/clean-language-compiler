//! Bidirectional inference context (M4): an `ena` union-find over
//! `Ty::Var` keys, plus the directional assignability relation that the
//! whole checker funnels through.
//!
//! Variables are deliberately scarce — Clean declarations always carry
//! their type (STM-01), so inference only bridges the gaps the grammar
//! leaves open: the element type of a bare list literal, the payload of a
//! bare `none`. `finalize` collapses anything still unconstrained to
//! `any` (TYP-02's universal generic), so no `Ty::Var` leaves pass [5].

use ena::unify::{EqUnifyValue, InPlaceUnificationTable, UnifyKey};

use super::types::{Ty, TyVid};

impl UnifyKey for TyVid {
    type Value = Option<Ty>;
    fn index(&self) -> u32 {
        self.0
    }
    fn from_index(u: u32) -> Self {
        TyVid(u)
    }
    fn tag() -> &'static str {
        "TyVid"
    }
}

impl EqUnifyValue for Ty {}

/// The outcome of a directional assignability check (`from` value into a
/// `to` slot). `Promote` is TYP-06's single implicit conversion,
/// `integer` → `number`; the caller materialises it as a TIR coercion so
/// codegen never has to re-discover it. `Wrap` is TYP-03's `T` into `T?`
/// (with `depth` nested `some` constructors, and possibly a promotion
/// innermost).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fit {
    Exact,
    /// `integer` value into a `number` slot (TYP-06).
    Promote,
    /// `T` value into a `T?` slot (TYP-03); `promote` marks an innermost
    /// integer→number promotion under the wrap.
    Wrap {
        promote: bool,
    },
    No,
}

pub struct InferCtx {
    table: InPlaceUnificationTable<TyVid>,
}

impl Default for InferCtx {
    fn default() -> Self {
        Self::new()
    }
}

impl InferCtx {
    pub fn new() -> InferCtx {
        InferCtx {
            table: InPlaceUnificationTable::new(),
        }
    }

    pub fn fresh(&mut self) -> Ty {
        Ty::Var(self.table.new_key(None))
    }

    /// Follows a variable to its current binding, one level.
    fn shallow(&mut self, ty: &Ty) -> Ty {
        let mut current = ty.clone();
        while let Ty::Var(vid) = current {
            match self.table.probe_value(vid) {
                Some(bound) => current = bound,
                None => return Ty::Var(vid),
            }
        }
        current
    }

    /// Deep-resolves every bound variable; unbound variables survive as
    /// `Ty::Var` (use `finalize` at the end of a body).
    pub fn resolve(&mut self, ty: &Ty) -> Ty {
        let shallow = self.shallow(ty);
        self.map_inner(&shallow, false)
    }

    /// Deep-resolves and collapses still-unbound variables to `any`
    /// (TYP-02): an unconstrained literal is checked nowhere, which is
    /// exactly what `any` means.
    pub fn finalize(&mut self, ty: &Ty) -> Ty {
        let shallow = self.shallow(ty);
        if matches!(shallow, Ty::Var(_)) {
            return Ty::Any;
        }
        self.map_inner(&shallow, true)
    }

    fn map_inner(&mut self, ty: &Ty, collapse: bool) -> Ty {
        let recurse = |cx: &mut Self, t: &Ty| {
            if collapse {
                cx.finalize(t)
            } else {
                cx.resolve(t)
            }
        };
        match ty {
            Ty::Option(inner) => Ty::Option(Box::new(recurse(self, inner))),
            Ty::List(inner, behavior) => Ty::List(Box::new(recurse(self, inner)), *behavior),
            Ty::Matrix(inner) => Ty::Matrix(Box::new(recurse(self, inner))),
            Ty::Pairs(key, value) => {
                Ty::Pairs(Box::new(recurse(self, key)), Box::new(recurse(self, value)))
            }
            Ty::Record { wit_name, fields } => Ty::Record {
                wit_name: wit_name.clone(),
                fields: fields
                    .iter()
                    .map(|(n, t)| (n.clone(), recurse(self, t)))
                    .collect(),
            },
            other => other.clone(),
        }
    }

    /// Structural unification: makes the two types equal by binding
    /// variables, or reports that they cannot be. `Error` absorbs (the
    /// mismatch was already reported); `Any` unifies with everything
    /// (TYP-02: checking is skipped).
    pub fn unify(&mut self, a: &Ty, b: &Ty) -> bool {
        let a = self.shallow(a);
        let b = self.shallow(b);
        match (&a, &b) {
            (Ty::Error, _) | (_, Ty::Error) => true,
            (Ty::Any, _) | (_, Ty::Any) => true,
            (Ty::Var(va), Ty::Var(vb)) => self.table.unify_var_var(*va, *vb).is_ok(),
            (Ty::Var(vid), other) | (other, Ty::Var(vid)) => {
                if self.occurs(*vid, other) {
                    return false;
                }
                self.table
                    .unify_var_value(*vid, Some(other.clone()))
                    .is_ok()
            }
            (Ty::Option(ia), Ty::Option(ib)) => self.unify(ia, ib),
            (Ty::List(ia, ba), Ty::List(ib, bb)) => ba == bb && self.unify(ia, ib),
            (Ty::Matrix(ia), Ty::Matrix(ib)) => self.unify(ia, ib),
            (Ty::Pairs(ka, va), Ty::Pairs(kb, vb)) => self.unify(ka, kb) && self.unify(va, vb),
            (
                Ty::Record {
                    wit_name: na,
                    fields: fa,
                },
                Ty::Record {
                    wit_name: nb,
                    fields: fb,
                },
            ) => {
                na == nb
                    && fa.len() == fb.len()
                    && fa
                        .iter()
                        .zip(fb)
                        .all(|((xa, ta), (xb, tb))| xa == xb && self.unify(ta, tb))
            }
            _ => a == b,
        }
    }

    fn occurs(&mut self, vid: TyVid, ty: &Ty) -> bool {
        match ty {
            Ty::Var(other) => self.table.unioned(vid, *other),
            Ty::Option(inner) | Ty::List(inner, _) | Ty::Matrix(inner) => {
                let inner = self.shallow(inner);
                self.occurs(vid, &inner)
            }
            Ty::Pairs(key, value) => {
                let (key, value) = (self.shallow(key), self.shallow(value));
                self.occurs(vid, &key) || self.occurs(vid, &value)
            }
            Ty::Record { fields, .. } => fields.iter().any(|(_, t)| {
                let t = self.shallow(t);
                self.occurs(vid, &t)
            }),
            _ => false,
        }
    }

    /// Directional assignability: can a `from` value fill a `to` slot?
    /// The lattice, in order:
    /// - `Error` absorbs (already reported), `Any` skips checking (TYP-02);
    /// - exact/unifiable types fit;
    /// - any integer fits any integer slot (widths are a boundary range
    ///   property, LBS-02 — the range check happens at the boundary);
    /// - `integer` fits a `number` slot — TYP-06's one implicit conversion;
    /// - `T` fits `T?` (TYP-03), including with the promotion innermost.
    ///
    /// `T?` does NOT fit `T` (TYP-03), a behaviored list does not fit a
    /// bare one or vice versa (TYP-05: the behavior is part of the type),
    /// and containers are invariant (no implicit conversion of elements).
    pub fn fit(&mut self, from: &Ty, to: &Ty) -> Fit {
        let from = self.shallow(from);
        let to = self.shallow(to);
        match (&from, &to) {
            (Ty::Error, _) | (_, Ty::Error) => Fit::Exact,
            (Ty::Any, _) | (_, Ty::Any) => Fit::Exact,
            (a, b) if a.is_integer() && b.is_integer() => Fit::Exact,
            (a, Ty::Number) if a.is_integer() => Fit::Promote,
            // `T?` into `T?`: unify payloads (no implicit conversion
            // through the option constructor).
            (Ty::Option(_), Ty::Option(_)) => {
                if self.unify(&from, &to) {
                    Fit::Exact
                } else {
                    Fit::No
                }
            }
            // `T` into `T?` — wrap, possibly promoting innermost (TYP-03).
            (_, Ty::Option(inner)) => {
                let inner = inner.as_ref().clone();
                match self.fit(&from, &inner) {
                    Fit::Exact => Fit::Wrap { promote: false },
                    Fit::Promote => Fit::Wrap { promote: true },
                    // Nested wraps do not stack — absence does not stack
                    // (TYP-03), so one level is always enough.
                    Fit::Wrap { .. } | Fit::No => Fit::No,
                }
            }
            _ => {
                if self.unify(&from, &to) {
                    Fit::Exact
                } else {
                    Fit::No
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unify_binds_list_element_variables() {
        let mut cx = InferCtx::new();
        let var = cx.fresh();
        let inferred = Ty::list(var.clone());
        assert!(cx.unify(&inferred, &Ty::list(Ty::Integer)));
        assert_eq!(cx.resolve(&var), Ty::Integer);
    }

    #[test]
    fn integer_fits_number_as_promotion() {
        let mut cx = InferCtx::new();
        assert_eq!(cx.fit(&Ty::Integer, &Ty::Number), Fit::Promote);
        assert_eq!(cx.fit(&Ty::Number, &Ty::Integer), Fit::No);
    }

    #[test]
    fn optional_is_one_way() {
        let mut cx = InferCtx::new();
        let opt = Ty::Option(Box::new(Ty::Str));
        assert_eq!(cx.fit(&Ty::Str, &opt), Fit::Wrap { promote: false });
        assert_eq!(cx.fit(&opt, &Ty::Str), Fit::No);
    }

    #[test]
    fn behaviors_are_part_of_the_type() {
        use super::super::types::{ListBehavior, Removal};
        let mut cx = InferCtx::new();
        let plain = Ty::list(Ty::Str);
        let line = Ty::List(
            Box::new(Ty::Str),
            ListBehavior {
                removal: Some(Removal::Line),
                unique: false,
            },
        );
        assert_eq!(cx.fit(&plain, &line), Fit::No);
        assert_eq!(cx.fit(&line, &plain), Fit::No);
    }

    #[test]
    fn unbound_variable_finalizes_to_any() {
        let mut cx = InferCtx::new();
        let var = cx.fresh();
        let listy = Ty::list(var);
        assert_eq!(cx.finalize(&listy), Ty::list(Ty::Any));
    }
}
