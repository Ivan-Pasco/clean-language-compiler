//! The chapter-15 standard-library surface (STD-01): signatures the
//! checker types and the semantic operations pass [8] lowers. One table
//! per namespace module; method-style surfaces live with the receiver
//! type in `check.rs`.
//!
//! Never hardcode return types at call sites (KNOWLEDGE §8): every
//! stdlib lookup flows through this registry.

use super::types::Ty;

/// A standard-library operation, carried from pass [5] to pass [8] as a
/// semantic name — never as MIR detail. `math` basic arithmetic is
/// operators only (chapter 15: `math.add` MUST NOT exist).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum StdFn {
    // math — wasm-native subset (guest instructions).
    MathSqrt,
    MathAbsInteger,
    MathAbsNumber,
    MathMax,
    MathMin,
    MathFloor,
    MathCeil,
    MathRound,
    MathTrunc,
    MathSign,
    // math — transcendentals: typed here, blocked in codegen on the
    // clean:bridge/math conflict (DISCOVERIES-M6 item 2).
    MathSin,
    MathCos,
    MathTan,
    MathAsin,
    MathAcos,
    MathAtan,
    MathAtan2,
    MathLn,
    MathLog10,
    MathLog2,
    MathExp,
    MathExp2,
    MathSinh,
    MathCosh,
    MathTanh,
}

/// `math.<name>(…)` signatures, verbatim from 15 §Math Module.
pub fn math_fn(name: &str) -> Option<(StdFn, &'static [Ty], Ty)> {
    use StdFn::*;
    const NUM1: &[Ty] = &[Ty::Number];
    const NUM2: &[Ty] = &[Ty::Number, Ty::Number];
    const INT1: &[Ty] = &[Ty::Integer];
    let (func, params, ret) = match name {
        "sqrt" => (MathSqrt, NUM1, Ty::Number),
        "absInteger" => (MathAbsInteger, INT1, Ty::Integer),
        "absNumber" => (MathAbsNumber, NUM1, Ty::Number),
        "max" => (MathMax, NUM2, Ty::Number),
        "min" => (MathMin, NUM2, Ty::Number),
        "floor" => (MathFloor, NUM1, Ty::Number),
        "ceil" => (MathCeil, NUM1, Ty::Number),
        "round" => (MathRound, NUM1, Ty::Number),
        "trunc" => (MathTrunc, NUM1, Ty::Number),
        "sign" => (MathSign, NUM1, Ty::Number),
        "sin" => (MathSin, NUM1, Ty::Number),
        "cos" => (MathCos, NUM1, Ty::Number),
        "tan" => (MathTan, NUM1, Ty::Number),
        "asin" => (MathAsin, NUM1, Ty::Number),
        "acos" => (MathAcos, NUM1, Ty::Number),
        "atan" => (MathAtan, NUM1, Ty::Number),
        "atan2" => (MathAtan2, NUM2, Ty::Number),
        "ln" => (MathLn, NUM1, Ty::Number),
        "log10" => (MathLog10, NUM1, Ty::Number),
        "log2" => (MathLog2, NUM1, Ty::Number),
        "exp" => (MathExp, NUM1, Ty::Number),
        "exp2" => (MathExp2, NUM1, Ty::Number),
        "sinh" => (MathSinh, NUM1, Ty::Number),
        "cosh" => (MathCosh, NUM1, Ty::Number),
        "tanh" => (MathTanh, NUM1, Ty::Number),
        _ => return None,
    };
    Some((func, params, ret))
}

/// `math.<name>` constants (no parentheses), verbatim from 15 §Math.
pub fn math_constant(name: &str) -> Option<f64> {
    match name {
        "pi" => Some(std::f64::consts::PI),
        "e" => Some(std::f64::consts::E),
        "tau" => Some(std::f64::consts::TAU),
        _ => None,
    }
}

/// True when this transcendental has no guest lowering yet (blocked on
/// the guest-vs-`clean:bridge/math` ruling).
pub fn is_transcendental(func: StdFn) -> bool {
    use StdFn::*;
    matches!(
        func,
        MathSin
            | MathCos
            | MathTan
            | MathAsin
            | MathAcos
            | MathAtan
            | MathAtan2
            | MathLn
            | MathLog10
            | MathLog2
            | MathExp
            | MathExp2
            | MathSinh
            | MathCosh
            | MathTanh
    )
}
