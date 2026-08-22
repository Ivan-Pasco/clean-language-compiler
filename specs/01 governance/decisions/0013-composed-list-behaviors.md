# ADR-0013 — Semantics of composed list behaviors

Clean lists carry a behaviour suffix — `.line` (FIFO), `.pile` (LIFO), `.unique` (set) — and the type-system chapter presented composed forms like `.line.pile` and `.line.unique.pile` with no defined semantics: `remove()` on a `.line.pile` has two answers and the chapter gives neither. This ADR decides that `.line` and `.pile` are mutually exclusive removal disciplines while `.unique` is an independent membership constraint, and that a list's behaviour is part of its type — a static fact the compiler resolves at each call site.

---

## Context

Clean lists carry a *behavior* declared as a suffix on the type: `.line` (FIFO — remove from the front), `.pile` (LIFO — remove from the top), `.unique` (a set — duplicate additions are ignored). Each is defined on its own.

[04 — Type System](../../04%20language/04-type-system.md) also presents **composed** behaviors — `.line.pile` described as *"FIFO + LIFO combined"* and `.line.unique.pile` as *"all three behaviors"* — and defines nothing about them. The composition is not merely undocumented, it is contradictory on its face: `remove()` on a `.line` takes from the front and on a `.pile` takes from the top, so `remove()` on a `.line.pile` has two answers and the chapter gives neither.

Three of the seven canonical suffix forms therefore have no semantics. A second, related question is whether the behavior is part of the type: the chapter states both that behavior is declared *"at variable declaration time"* as part of the type, and that properties *"can be changed at runtime"*, and that a property-modified list *"remains the same `list<T>` type"* — which cannot all hold at once, since there is nothing left to type-check.

The memory model compounds it: `list<T>` is specified there as `length / capacity / elem_type / elements`, with no field encoding a behavior — so no representation exists for the composed forms, and the O(1) complexity claims made for `.line` and `.unique` are not achievable on that layout.

## Options considered

**A — Composition is illegal.** Exactly one behavior suffix per declaration; `.line.pile` is a compile error with a new diagnostic. Smallest surface, no undefined cases, and it matches what the memory layout can represent. Cost: removes a documented (if meaningless) form.

**B — Composition is ordered and the last suffix wins for removal.** `.line.pile` is a unique-free list whose `remove()` follows `.pile`; each suffix contributes only the operations the later ones do not override. Preserves the syntax and makes it total. Cost: the reading order is a convention the user must memorise, and `.line.pile` becomes a confusing spelling of `.pile`.

**C — Orthogonal axes.** `.unique` is a membership constraint; `.line`/`.pile` are removal disciplines and are mutually exclusive. `.line.unique` and `.pile.unique` are legal; `.line.pile` is a compile error. Composition means something precise and the illegal combination is named. Cost: the axis model must be stated, and the type system must carry it.

## Decision

**Option C — orthogonal axes.** `.line` and `.pile` are removal disciplines and are mutually exclusive; `.unique` is a membership constraint independent of both. `.line.unique` and `.pile.unique` are legal in either order; `.line.pile` and `.line.unique.pile` are [`SEM009`](../../03%20platform/09-error-codes.md#32-semantic-codes-sem).

**Option A would have removed a form that works.** Banning composition outright also bans `.line.unique` — a FIFO queue that ignores duplicates, which is unambiguous and useful. The contradiction was never composition itself; it was composing two answers to the same question.

**Option B would have kept a spelling that lies.** Under last-suffix-wins, `.line.pile` is a confusing way to write `.pile`, and the reading order becomes a convention to memorise. A form whose meaning is "ignore the first half of what I wrote" is worse than a form that is rejected.

**The second question had to be answered first.** The ADR notes that composition cannot be specified without deciding whether a behavior is part of the type, and the chapter asserted both readings. It is **part of the type**, fixed at declaration: that is the only reading under which `remove()` has a checkable meaning, since a list free to change discipline is a list whose `remove()` the compiler cannot resolve.

**The memory model needs nothing.** The ADR raised that `list<T>`'s layout has no field for a behavior and that the complexity claims are unachievable on it. With the behavior static, no field is required — the compiler knows the discipline at the call site and emits the operation directly. The layout is untouched.

## Consequences

Three of the seven documented suffix forms are withdrawn as unspecifiable, and the five that remain each have one meaning.

The chapter's opening sentence is reversed. It read that a behavior changes how a list works "without changing its type"; it now states the opposite, because the alternative leaves `remove()` undefined.

---

## Metadata

- **Status:** Accepted (2026-08-02)
- **Date:** 2026-08-01
- **Supersedes:** None
- **Spec impact:** [04 language / 04 — Type System](../../04%20language/04-type-system.md) §List Behaviors
