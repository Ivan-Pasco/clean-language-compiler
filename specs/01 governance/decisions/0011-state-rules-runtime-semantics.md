# ADR-0011 — Runtime semantics of `state: rules:` and multiple guard clauses

State declarations offered two validation constructs, `guard` and `rules:`, but neither had runtime semantics: `rules:` parsed and type-checked but did nothing observable, and `guard` had no defined behaviour when several clauses applied to one declaration. This ADR gives them distinct runtime meanings — guards reject an individual update, rules assert a whole-state invariant checked when a function that changed state returns, and multiple guards are permitted with written-order short-circuit evaluation.

---

## Context

[20 — State Management](../../04%20language/20-state-management.md) gives state declarations two validation mechanisms:

- a **guard** clause, `guard <expr> else "<message>"`, which rejects an update; and
- a **`rules:` block**, a set of boolean expressions attached to a state declaration.

Both have a syntax and a compile-time type check (`STATE001` requires the guard expression to be a pure boolean; `STATE005` requires each rule expression to be boolean). Neither has runtime semantics, and the chapter says so in its own text: *"When rule expressions are evaluated at runtime and what happens when one evaluates to `false` is (unspecified)"*, and *"Whether a declaration may carry more than one guard clause, and the evaluation order if so, is (unspecified)"*.

So `rules:` is today a construct that parses, type-checks, and then does nothing observable — while `guard` has a runtime failure (`STATE002`) but no defined behaviour when several guards apply to one declaration.

The open questions are: when is a rule evaluated (on every assignment, at declaration, on read, at a transaction boundary)? What happens on failure (the `STATE002` rejection path, a distinct diagnostic, a trap)? Is the failed update discarded or partially applied? May a declaration carry more than one guard, and if so are they evaluated in written order, short-circuiting on the first failure?

## Options considered

**A — `rules:` is guard sugar.** Every rule is a guard with a generated message; evaluation on assignment, rejection on `false`, `STATE002` for both. Simplest, and collapses two mechanisms into one. Cost: the two constructs then have no reason to both exist, and `STATE005` becomes redundant with `STATE001`.

**B — Guards reject an update; rules are invariants over the state as a whole.** Guards run per assignment and veto it; rules run after the update completes and assert a whole-state property, failing with a distinct diagnostic. This mirrors the `before` / `always` split already in [10 — Contracts](../../04%20language/10-contracts.md). Cost: needs a new diagnostic code and a defined point at which "the update completes".

**C — Withdraw `rules:`.** If guards cover the need, an unimplemented construct with no semantics is worse than no construct. Cost: `STATE005` is withdrawn, and any library or example relying on the syntax breaks.

On multiple guards, the sub-options are: forbid more than one (a compile error); allow, evaluate in written order, short-circuit on first failure; or allow and evaluate all, reporting every failure.

## Decision

**Option B — guards reject an update, rules assert an invariant.** Written as [SMG-02](../../04%20language/20-state-management.md) and [SMG-03](../../04%20language/20-state-management.md#smg-03--state-rules), with the new [`STATE006`](../../03%20platform/09-error-codes.md#37-state-codes-state).

The two mechanisms are not redundant, and the syntax already showed why: a guard is attached to **one declaration** and sees `value`, the proposed update; a `rules:` block sits on the `state:` block and sees **every variable in it**. Only the second can express a relation between two state variables, which is the case a guard structurally cannot reach. They are the state-block counterpart of the `before` / `always` split [10 — Contracts](../../04%20language/10-contracts.md) already draws.

**Rules are evaluated when a function that changed state returns**, not on each assignment. This is the ADR's "transaction boundary" question, and the answer is that the boundary already exists. Evaluating per assignment would make multi-variable rules unsatisfiable in practice: with `rules: end > start`, setting `start` before `end` passes through a state the rule forbids, so a valid update would be impossible to write. The enclosing function's return needs no new construct and is where `after` and `always` are already checked.

**A violated rule raises rather than rejects.** A guard failing is an expected outcome — the update is refused, the state is unchanged, the program continues. A rule failing means the state as a whole reached a combination the program declared impossible, which is a defect. `STATE006` is therefore distinct from `STATE002`, and the state is not rolled back: the function has already run, exactly as with a violated `always`.

**Option A** — rules as guard sugar — was rejected because it discards the only thing rules can do that guards cannot. **Option C** — withdrawing `rules:` — was rejected for the same reason, and would have removed the cross-variable invariant with nothing to replace it.

**Multiple guards are permitted**, evaluated in written order, stopping at the first failure. Each guard carries its own message, so the first failure is the one the developer needs; evaluating the rest would produce several messages for one rejected update with no defined way to report them. Forbidding a second guard would have pushed users to merge conditions into one clause with one merged message, which reports less.

## Consequences

`rules:` stops being a construct that parses, type-checks and does nothing. It had `STATE005` for its compile-time check and no runtime effect at all.

`STATE005` and `STATE001` remain distinct, which option A would have collapsed.

---

## Metadata

- **Status:** Accepted (2026-08-02)
- **Date:** 2026-08-01
- **Supersedes:** None
- **Spec impact:** [04 language / 20 — State Management](../../04%20language/20-state-management.md) §Guards, §Rules
