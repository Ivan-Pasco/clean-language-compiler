# ADR-0017 — `break` and `continue`: grammar, scope, and diagnostics

`break` and `continue` are reserved everywhere but the spec's entire treatment of them was a single sentence in the control-flow chapter — no grammar, no scoping rule, no diagnostic for use outside a loop, no defined interaction with `iterate`'s per-item binding. This ADR makes them unlabelled statements bound to the innermost enclosing loop in the same body, registers `SEM025` for illegal use, and defines `continue` to advance an `iterate` exactly as a normally-finished body does.

---

## Context

`break` and `continue` are hard keywords: reserved everywhere, and using either as an identifier is a compile-time error. They appear exactly once in the whole of `04 language/` — a single sentence in [12 — Control Flow](../../04%20language/12-control-flow.md): *"Use `break` to exit a loop early and `continue` to skip to the next iteration."*

That sentence is the entire specification. Nothing states:

- **Grammar.** Are they statements? May they carry a condition or a label?
- **Scope.** Which construct do they bind to inside nested loops — always the innermost? Is there a labelled form?
- **Interaction with `iterate`.** `iterate` is Clean's only collection loop and its iteration variable is bound per item; whether `continue` advances that binding the same way a normal iteration does is unstated. So is the interaction with the `step` modifier.
- **Illegal use.** `break` outside any loop has no diagnostic code. Neither does `continue`.
- **Interaction with contracts and compile-time code.** Whether either may cross a contract boundary, or appear inside a `compiletime function`, is unstated.

The gap matters more than its size suggests: early exit is the single most common control-flow construct after `if`, and a code-generating agent meeting a loop that must stop early has no rule to follow.

## Options considered

**A — Unlabelled statements binding to the innermost loop.** The conventional minimum: `break` and `continue` are statements, always innermost, illegal outside a loop with one diagnostic each. Cost: no way to exit an outer loop without a flag variable.

**B — Labelled form.** Loops may carry a label and `break label` / `continue label` target it. Expressive, and removes the flag-variable idiom. Cost: labels are a new lexical form and a new scope, in a language that deliberately keeps one way to do things.

**C — Neither; withdraw the keywords.** Early exit via `return` from an extracted function, which is the style the design rules already push toward. Cost: withdraws two reserved words and forces a rewrite of any loop that must stop early mid-body.

## Decision

**Option A — unlabelled statements binding to the innermost loop.** Written as [FLW-03](../../04%20language/12-control-flow.md#flw-03--break-and-continue), with [`SEM025`](../../03%20platform/09-error-codes.md#32-semantic-codes-sem) for use outside a loop.

Both are statements: alone on a line, no operand, no condition, no label, no value. They bind to the nearest enclosing `iterate` or `while` **in the same body**, and a function body, a contract block and a `compiletime function` body each begin a new one — so a `break` inside a function called from a loop ends nothing in the caller. `continue` advances an `iterate` exactly as a normally-finished body does, binding the next item and applying `step` unchanged; the ADR had listed that interaction as unstated and it is the part most likely to be got wrong by an implementer working from intuition.

**Rejected — B, the labelled form.** Labels would be a naming form and a scope that exist nowhere else in the language, added for a case that already has an answer: extract the inner loop into a function and `return`. The expressive gain is real but narrow, and it is paid for in surface ([C-09](../05-concerns.md), [LDR-08](../../04%20language/02-language-design-rules.md#ldr-08--one-way-to-do-things)).

**Rejected — C, withdrawing the keywords.** The proposal was that early exit be a `return` from an extracted function, which is the style the design rules already favour. It fails on the loops that most need early exit: a body updating several local variables is not extractable, so what actually replaces `break` is a flag variable tested in the loop condition — precisely the idiom `break` exists to remove. C would also have contradicted the sentence the specification already carried.

**One diagnostic, not two.** The ADR anticipated separate codes for illegal `break` and illegal `continue`. The condition is identical and differs only in the word reported, so two codes would have been one rule with two homes. `SEM025` names the keyword in its message.

## Consequences

`break` and `continue` are usable. The sentence in [12 — Control Flow](../../04%20language/12-control-flow.md) that was the whole prior specification now cites the rule instead of restating it.

Registering `SEM025` exposed a stale row in the error registry: §5 reserved `SEM023–SEM099` while `SEM023` and `SEM024` were both in use. Corrected in the same change.

Nothing here constrains unreachable code after a `break` or `continue` in the same block. No rule requires it to be diagnosed and none forbids it; if that is wanted it is a new warning and a new code, and it is not part of this decision.

---

## Metadata

- **Status:** Accepted (2026-08-02)
- **Date:** 2026-08-01
- **Supersedes:** None
- **Spec impact:** [04 language / 12 — Control Flow](../../04%20language/12-control-flow.md) (new `FLW-03`) · [03 platform / 09 — Error Codes](../../03%20platform/09-error-codes.md) (`SEM025`) · [03 platform / 10 — Semantic Rules](../../03%20platform/10-semantic-rules.md) (`SEM025`)
