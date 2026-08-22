# ADR-0018 — Operator overloading: the `matrix<T>` case

`A * B` on two `matrix<T>` values is the only place in the spec where an operator means anything other than its arithmetic definition, and its presence raised whether Clean has general operator overloading, a single built-in exception, or something in between — while the operations themselves (`transpose`, `inverse`, `determinant`) had no home in the standard library. This ADR keeps the operators, homes them in a new Matrix module, and states the general rule: the language defines what each operator means for each of its built-in types, and there is no user-defined overloading.

---

## Context

[06 — Expressions](../../04%20language/06-expressions.md) specifies that `A * B` on two `matrix<T>` values performs matrix multiplication, and gives `A.transpose()`, `A.inverse()` and `A.determinant()` alongside it.

This is the only place in the entire repository where an operator means something other than its arithmetic definition. Nothing else overloads: `+` on two strings is not defined as concatenation anywhere in the expression chapter, and every other type-specific operation is a method or a namespace function.

Three things are consequently unsettled:

1. **Is overloading a general facility or a single built-in exception?** If general, the language needs a way for a user type to declare it, and [02 — Language Design Rules](../../04%20language/02-language-design-rules.md) — which forbids user-defined generics — would need a matching position. If it is a built-in exception for one type, that should be stated as such rather than left to be inferred from a single example.
2. **`matrix<T>` has no home in the standard library.** There is no Matrix module in [15 — Standard Library](../../04%20language/15-standard-library.md); `transpose`, `inverse` and `determinant` are defined nowhere else, and their behaviour on a singular or non-square matrix — an error path — is unstated with no diagnostic.
3. **The design rules push the other way.** They say basic arithmetic uses operators and advanced maths uses `math.` functions. Matrix multiplication is advanced maths reached through an operator, which is neither branch.

## Options considered

**A — Built-in exception, stated explicitly.** `matrix<T>` is the one type with operator semantics, written as such in the expression chapter, with the operations homed in a Matrix module in the standard library and their failure modes coded. No user-facing overloading mechanism. Cost: an asymmetry in the language, made honest rather than removed.

**B — Withdraw the operators.** Matrix multiplication becomes `matrix.multiply(a, b)`, consistent with `math.*`. One rule, no exception. Cost: matrix-heavy code reads worse, and it is the one domain where operator notation genuinely matches the written maths.

**C — General operator overloading.** A declaration form lets any type give meaning to arithmetic operators. Most expressive. Cost: directly against the "one way to do things" principle, and it hands a code-generating agent a facility that makes any expression's meaning type-dependent.

## Decision

**Option A, with its premise corrected.** The operators stay; what changes is that they stop being an exception.

The ADR framed `matrix<T>` as "the one type with operator semantics", an asymmetry to be made honest. That framing was wrong. The rule that covers it without any exception is:

> The language defines what each operator means for each of its **built-in** types. There is no user-defined operator overloading.

Under that rule `matrix * matrix` is not special — it is the language defining `*` for one of its own types, exactly as it defines `*` for `number`. This is the model Go uses. What was missing was never a justification for an exception; it was the general sentence, which no chapter had written. It is now [§Operators on built-in types](../../04%20language/06-expressions.md#operators-on-built-in-types).

**Readability was the deciding constraint.** `A * B + C` is the notation matrix arithmetic is written in, and no function spelling of it reads as well. The rule above keeps that without making any *other* expression's meaning type-dependent, which is what a general mechanism would cost — every reader, and every code-generating agent, would have to resolve operand types before reading an expression at all.

**The surface is homed.** [15 §Matrix Module](../../04%20language/15-standard-library.md) is new and holds the operators, `transpose()`, `determinant()` and `inverse()`, their element-type constraints and their failure modes. `determinant()` and `inverse()` are defined on `matrix<number>` only: an inverse generally has fractional entries, so a rule spanning `matrix<integer>` would have to lose information or change element type. Both are in v1.

**Two runtime codes, no compile-time code.** `matrix<T>` is dynamically sized, so shape is not carried in the type and no shape error is decidable at compile time: [`RUN016`](../../03%20platform/09-error-codes.md#312-runtime-codes-run) for shapes an operation does not admit, [`RUN017`](../../03%20platform/09-error-codes.md#312-runtime-codes-run) for the inverse of a singular matrix. Element-type errors needed nothing new — `SEM004` `InvalidOperationForType` already covers them.

**Nothing crosses the bridge.** Every matrix operation is guest computation, including `inverse()` and `determinant()`, which are the two heavy enough to have suggested an L2 helper. Routing them through the host would make a floating-point result depend on which host is running — the same reasoning that already keeps format parsers out of L2 ([Platform 02 §2.2.1](../../03%20platform/02-host-bridge.md#221-portable-l2-in-every-world)). The bridge catalog is untouched by this ADR.

**Rejected — B, withdrawing the operators.** `matrix.add(matrix.multiply(A, B), C)` for `A * B + C` fails the one requirement that mattered here.

**Rejected — C, general operator overloading.** It would make any expression's meaning type-dependent, and it is the facility [LDR-06](../../04%20language/02-language-design-rules.md#ldr-06--generic-containers-are-built-in-user-generics-are-not) already declines to give users for generics.

**Recorded for a future version.** If user-defined operators are ever wanted, the mechanism is a **capability carrying a self type** — `can Multiply` requiring `multiply(Self) -> Self` — not a new declaration form. It is unavailable in v1 because capabilities are neither generic nor self-typed ([CLS-03](../../04%20language/14-classes-and-objects.md#cls-03--capabilities-are-contracts-without-bodies)). Naming the path now is what keeps this decision from being a dead end.

## Consequences

`matrix<T>` has a specified surface, its operators have a stated general rule, and its four failure modes have diagnostics.

Writing the operator table surfaced a defect no question had named: **`+` on two `string` values** is used in examples across several chapters and was defined in none — the same orphan-fact pattern the ADR recorded for `matrix`. It is registered in the same table.

Registering the runtime codes exposed a stale row in the error registry: §5 reserved `RUN013–RUN099` while `RUN013`–`RUN015` were in use. Corrected in the same change.

---

## Metadata

- **Status:** Accepted (2026-08-02)
- **Date:** 2026-08-01
- **Supersedes:** None
- **Spec impact:** [04 language / 06 — Expressions](../../04%20language/06-expressions.md) (§Matrix Operations replaced by §Operators on built-in types) · [04 language / 15 — Standard Library](../../04%20language/15-standard-library.md) (new §Matrix Module, `STD-01`) · [03 platform / 09 — Error Codes](../../03%20platform/09-error-codes.md) (`RUN016`, `RUN017`) · [03 platform / 10 — Semantic Rules](../../03%20platform/10-semantic-rules.md) (`RUN016`, `RUN017`)
