# ADR-0019 — Precision modifiers on numeric types

The type-system chapter carried ~75 lines of width-and-signedness modifiers (`integer:8u`, `integer:32`, `number:32`, …) — a sub-type-system with no memory representation, no narrowing rule, no diagnostic for out-of-range literals, and exactly two consumers, both at the WIT boundary. This ADR withdraws precision modifiers from the surface language and keeps them only in `host function` declarations where they name the width on the other side, closing the sub-type-system's four unanswered questions by removing what would have needed answering.

---

## Context

[04 — Type System](../../04%20language/04-type-system.md) devotes roughly seventy-five lines to a sub-type-system: `integer:8`, `integer:8u`, `integer:16`, `integer:32`, `integer:32u`, `integer:64`, `number:32`, and so on — width-and-signedness modifiers on the two numeric types, each with its documented range.

Outside that section, the whole facility has **one consumer in the repository**: the Clean-to-WIT type table, which maps `integer:32` to `s32`. Nothing else uses a precision modifier — not the standard library, not the framework libraries, not the host bridge, not one worked example.

What is missing to make it implementable:

- **No memory representation.** [03 — Memory Model](../../03%20platform/03-memory-model.md) specifies the layout of `list<T>`, of strings and of the heap, and says nothing about narrow numerics. Whether an `integer:8` occupies one byte in a list or is widened to the natural word is unstated, which means the layout of `list<integer:8>` is undefined.
- **No diagnostic for an out-of-range literal.** `integer:8 x = 300` has no code. Neither does an assignment that narrows.
- **No conversion rule.** The chapter states an implicit widening rule for `integer` → `number` and says nothing about `integer:8` → `integer:32`, or about what happens at the boundary of a narrowing assignment: trap, wrap, saturate, or compile error.
- **No interaction with the default width.** Now that `integer` is 64-bit, `integer:64` and `integer` are the same type, and the chapter's claim that `integer:32` is *"the same as standard integer"* is stale.

Until these exist, the modifiers are notation without semantics — and a code-generating agent that meets `integer:8` has to invent all four answers.

## Options considered

**A — Keep, and specify.** Precision modifiers stay, gain a memory representation, a narrowing rule with a diagnostic, and conversion semantics. Justified by the WIT boundary, which genuinely distinguishes `s8` from `s64`, and by any future embedded target. Cost: real work in the memory model and the type checker, for a facility nothing currently uses.

**B — Withdraw from the surface language; keep at the bridge.** The language has `integer` and `number`; narrow widths exist only as WIT types a host declares, and the compiler checks the range at the boundary. The surface language stays small ([C-09](../05-concerns.md)), and the one real consumer is still served. Cost: a library author who needs a `u8` buffer has no way to say so in Clean.

**C — Keep only where a buffer demands it.** Narrow integers exist solely as the element type of `bytes` and of lists crossing the bridge, not as general variable types. Middle path, smallest specification that still covers the real use. Cost: an irregularity in the type system — a type usable in one position only.

## Decision

**Option B — withdrawn from the surface language, kept at the bridge.** The surface language has `integer` and `number` and no width or signedness modifier on either. A width suffix is valid in exactly one place: a `host function` declaration, where it names the width the WIT interface on the other side uses ([Libraries Specification §8.3](../../02%20components/framework/09-libraries-specification.md#8-host-bridge-as-typed-host-function-declarations)). The compiler checks a value's range as it crosses.

**The ADR's own count was low.** It records one consumer, the Clean-to-WIT mapping table. There are two: that table, and `sseRetry(ms: integer:32)`, a `host function` parameter in the server library's `host_bridge.cln`. The second one matters because it is the case that would have argued for keeping the facility in the language — and on inspection it is a boundary declaration too. Both consumers are served by B exactly, and no Clean code anywhere in the repository uses a modifier.

**B's stated cost does not exist.** The ADR gives it as "a library author who needs a `u8` buffer has no way to say so in Clean". They do: `bytes` is a built-in type, defined as a raw byte buffer. The use case that would have justified narrow integers is already served by another type — which also empties **option C**, whose whole proposal was to keep them as a buffer element type.

**What A would have cost**, for a facility with no surface use: a memory representation for narrow numerics and therefore a defined layout for `list<integer:8>`, a narrowing rule at the assignment boundary (trap, wrap, saturate, or compile error — four viable answers, none written), a conversion lattice across seven widths, and a diagnostic for each. The withdrawal is also the reversible direction: adding modifiers to the surface later breaks no existing program, removing them later would.

**The deferred diagnostic is now registered.** [ADR-0014](0014-source-text-encoding-and-identifier-charset.md) settled that a range is measured after unary minus applies but deliberately left the diagnostic unregistered, because the gap was recorded here. Withdrawing the modifiers reduced it from a matrix of seven widths to a single condition over `integer` and `number`, and it is [`SEM026`](../../03%20platform/09-error-codes.md#32-semantic-codes-sem) `LiteralOutOfRange` — whose condition carries the post-fold requirement, so the asymmetry of a signed range does not make the documented minimum unwritable.

## Consequences

Roughly seventy-five lines of unimplementable sub-type-system leave the language spec, and with them the four unanswered questions the ADR listed. The stale claim that `integer:32` equals the standard integer disappears with the section that made it.

The glossary's **precision modifier** entry is redefined rather than removed: the term still names something real, but it belongs to the host boundary and its owner moves from the type system to the libraries specification.

Nothing was added to the memory model. The layout question the ADR raised — what `list<integer:8>` looks like in linear memory — does not arise, because the type does not exist.

---

## Metadata

- **Status:** Accepted (2026-08-02)
- **Date:** 2026-08-01
- **Supersedes:** None
- **Spec impact:** [04 language / 04 — Type System](../../04%20language/04-type-system.md) (§Precision Control withdrawn, replaced by §Numeric widths) · [02 components / framework 09 — Libraries Specification](../../02%20components/framework/09-libraries-specification.md) (§8.3) · [03 platform / 09 — Error Codes](../../03%20platform/09-error-codes.md) (`SEM026`) · [03 platform / 10 — Semantic Rules](../../03%20platform/10-semantic-rules.md) (`SEM026`) · [01 governance / 06 — Glossary](../06-glossary.md) (*precision modifier*)
