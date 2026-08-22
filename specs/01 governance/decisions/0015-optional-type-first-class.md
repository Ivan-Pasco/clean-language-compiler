# ADR-0015 — Is `T?` a first-class type or a declaration annotation?

The optional type `T?` had a home in the type system but no rule about how far it composes — whether it may appear as a function return, a field, or a type argument, whether `T??` is legal, and whether it is a distinct type or a modifier on `T`. This ADR makes `T?` first-class in every type position, non-nesting (writing `T??` is a compile error and generic instantiation that would produce it collapses to one level), and a distinct nominal type for capability conformance.

---

## Context

The optional type `T?` now has a home in [04 — Type System](../../04%20language/04-type-system.md), where its syntax, its relation to `none`, and its assignability rule are stated — material promoted from where it had been scattered: an unannounced use in the expression chapter, two diagnostic rule bodies (`IDX005`, and the nullable-receiver rule), and the WIT mapping `T?` → `option<T>`.

Promotion fixed the home. It did not answer how far the type composes, because no document has ever said:

- May `T?` appear anywhere a type may appear — as a function return type, a parameter type, a field type, a type argument (`list<string?>`, `pairs<string, integer?>`)? Or only in a variable declaration?
- Does `T??` exist, and if the language permits writing it, is it distinct from `T?` or collapsed into it?
- Is `T?` a distinct type for overload and capability-conformance purposes, or a modifier on `T`?
- What is the static type of a `none` literal in a context that does not expect an optional?

These matter beyond notation. `list<string?>` and `list<string>?` are different things — a list of optional strings versus an optional list — and the WIT boundary already distinguishes them, since `option<T>` is a real type constructor there. Whichever way this goes, the type checker's shape follows from it.

## Options considered

**A — First-class type constructor.** `T?` is sugar for a nullable type that may appear in any type position and nest as any other generic does; `T??` is either legal and distinct or explicitly collapsed. Composes cleanly, matches `option<T>` at the WIT boundary, and makes `list<string?>` expressible. Cost: the type checker carries a real type constructor, and the `!` and `default` operators must be specified against nested cases.

**B — Declaration annotation only.** `T?` is admissible in a variable declaration and nowhere else; functions signal absence by other means. Smallest type system. Cost: it cannot express `list<string?>`, and it does not map onto `option<T>`, which appears in return position in the bridge today.

**C — First-class but non-nesting.** `T?` composes in any type position, and `T??` is a compile error rather than a distinct type — the pragmatic middle most mainstream languages settle on. Cost: needs a diagnostic, and the collapse rule has to be stated for generic instantiation (`list<T>?` where `T` is itself optional).

## Decision

**Option C — first-class in every type position, non-nesting.** `T?` may appear wherever a type may: variable, parameter, return type, field, and type argument. Writing `T??` is [`SEM009`](../../03%20platform/09-error-codes.md#32-semantic-codes-sem), and an instantiation that would produce an optional of an optional collapses to one level.

**Option B was ruled out by an Accepted document, not by preference.** The WIT mapping already puts `T?` in return position as `option<T>` ([Libraries Specification §8.3](../../02%20components/framework/09-libraries-specification.md#8-host-bridge-as-typed-host-function-declarations)). A declaration-only optional would contradict a mapping the bridge already relies on.

**Between A and C, the collapse is a deliberate loss.** Nesting would let a lookup distinguish "no such key" from "the key holds an absence". Collapsing makes those the same answer. The cost of keeping them apart is not paid by the programs that need the distinction — it is paid by every reader of every optional, who must ask how deep it goes, and by `default` and `!`, which would each need specifying against nested cases. A program that genuinely needs the distinction can carry it in its own type.

The collapse rule is stated for instantiation as well as for source, which is what the ADR identified as C's real cost: `T??` is not only something a developer might write, it is something a generic can produce without anyone writing it.

**`T?` is a distinct type, not a modifier.** Capability conformance is nominal ([CLS-03](../../04%20language/14-classes-and-objects.md#cls-03--capabilities-are-contracts-without-bodies)), so a method declaring `string` is not satisfied by one declaring `string?`. The static type of `none` needed no new rule: TYP-03 already states it has no type of its own and is assignable only to an optional.

## Consequences

`list<string?>` and `list<string>?` are both expressible and distinct, which the host boundary already required.

One placeholder elsewhere is now resolved. [`ERH-04`](../../04%20language/13-error-handling.md#erh-04--the-error-binding-is-an-error-value) had declared the `Error.code` field as a `string` carrying the empty string for a program-raised failure, explicitly because `T?` in field position was undecided. It is now `string?` holding `none` — absence stated rather than encoded.

---

## Metadata

- **Status:** Accepted (2026-08-02)
- **Date:** 2026-08-01
- **Supersedes:** None
- **Spec impact:** [04 language / 04 — Type System](../../04%20language/04-type-system.md) (`TYP-03`) · [04 language / 13 — Error Handling](../../04%20language/13-error-handling.md) (`ERH-04`)
