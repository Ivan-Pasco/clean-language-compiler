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
    // string — method style (receiver is args[0] in the CallStd node).
    // Indexes and lengths count code points (local adoption,
    // DISCOVERIES-M6).
    StrLength,
    StrIsEmpty,
    StrIsBlank,
    StrContains,
    StrIndexOf,
    StrLastIndexOf,
    StrStartsWith,
    StrEndsWith,
    StrCharAt,
    StrCharCodeAt,
    StrSubstring,
    StrTrim,
    StrTrimStart,
    StrTrimEnd,
    StrPadStart,
    StrPadEnd,
    StrReplace,
    StrSplit,
    // Unicode case folding is clean:bridge/string territory (Platform 02
    // §2.2.1) — typed, blocked in codegen (DISCOVERIES-M6 item 1).
    StrToUpperCase,
    StrToLowerCase,
    // string — namespace style.
    StrConcat,
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

/// Method-style `string` surface, verbatim from 15 §String Module.
/// Params exclude the receiver; `matches` stays with its SEM010 check in
/// `check.rs`, and `string.concat` is namespace-style below.
pub fn string_method(name: &str) -> Option<(StdFn, Vec<Ty>, Ty)> {
    use StdFn::*;
    let (func, params, ret) = match name {
        "length" => (StrLength, vec![], Ty::Integer),
        "isEmpty" => (StrIsEmpty, vec![], Ty::Boolean),
        "isBlank" => (StrIsBlank, vec![], Ty::Boolean),
        "contains" => (StrContains, vec![Ty::Str], Ty::Boolean),
        "indexOf" => (StrIndexOf, vec![Ty::Str], Ty::Integer),
        "lastIndexOf" => (StrLastIndexOf, vec![Ty::Str], Ty::Integer),
        "startsWith" => (StrStartsWith, vec![Ty::Str], Ty::Boolean),
        "endsWith" => (StrEndsWith, vec![Ty::Str], Ty::Boolean),
        "charAt" => (StrCharAt, vec![Ty::Integer], Ty::Str),
        "charCodeAt" => (StrCharCodeAt, vec![Ty::Integer], Ty::Integer),
        "substring" => (StrSubstring, vec![Ty::Integer, Ty::Integer], Ty::Str),
        "trim" => (StrTrim, vec![], Ty::Str),
        "trimStart" => (StrTrimStart, vec![], Ty::Str),
        "trimEnd" => (StrTrimEnd, vec![], Ty::Str),
        "padStart" => (StrPadStart, vec![Ty::Integer, Ty::Str], Ty::Str),
        "padEnd" => (StrPadEnd, vec![Ty::Integer, Ty::Str], Ty::Str),
        "replace" => (StrReplace, vec![Ty::Str, Ty::Str], Ty::Str),
        "split" => (StrSplit, vec![Ty::Str], Ty::list(Ty::Str)),
        "toUpperCase" => (StrToUpperCase, vec![], Ty::Str),
        "toLowerCase" => (StrToLowerCase, vec![], Ty::Str),
        _ => return None,
    };
    Some((func, params, ret))
}

/// Namespace-style `string.<name>(…)` — exactly one function
/// (`string.join` was retired in favour of `list.join`).
pub fn string_namespace_fn(name: &str) -> Option<(StdFn, Vec<Ty>, Ty)> {
    match name {
        "concat" => Some((StdFn::StrConcat, vec![Ty::Str, Ty::Str], Ty::Str)),
        _ => None,
    }
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
