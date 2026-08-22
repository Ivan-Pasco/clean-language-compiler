# ADR-0021 — The `time` and `bytes` namespaces

`time.now()` and `time.parse(...)` were named as the only ways to construct a `datetime` — a core language type — but no chapter of the standard library defined a `time` module at all, and `bytes.from(list<u8>)` leaked WIT vocabulary into surface Clean. This ADR homes both namespaces in the standard library: `time.now()` and `time.parse()` construct a `datetime` (readable from any world since `wasi:clocks/wall-clock` sits in portable L2), and `bytes.fromText`/`bytes.toText` cover byte construction without a `list<u8>` constructor whose element type does not exist in surface Clean.

---

## Context

Two namespaces are referenced across the ecosystem and defined nowhere.

**`time`.** [04 — Type System](../../04%20language/04-type-system.md) originally gave `time.now()` and `time.parse("2026-01-15T10:30:00Z")` as the only ways to obtain a `datetime` — the sole constructors of a core type. [09 — Libraries Specification](../../02%20components/framework/09-libraries-specification.md) independently asserts that a `time.*` namespace is among those available to library authors. [21 — Block Handlers](../../04%20language/21-block-handlers.md) bans `now()` at compile time as a non-deterministic call, which presupposes it exists at runtime. But [15 — Standard Library](../../04%20language/15-standard-library.md) has no `time` module, so `datetime` is a type with no way to construct a value.

**`bytes`.** The same chapter gave `bytes.from(list<u8>)` as the constructor for the `bytes` type. That signature also leaks WIT vocabulary (`u8`) into the surface language, where the Clean spelling would be `integer:8u` — itself an open question ([ADR-0019](0019-precision-modifiers.md)). `bytes` values do arrive from the host bridge, so the type is reachable; what is missing is whether a program can build one itself.

The gap is not cosmetic. A core type whose constructor is unspecified cannot be used at all, and `datetime` is not a marginal type — timestamps appear in the data library, in the server library, and in any application that records when something happened.

Deciding this means deciding more than a function list. `time.now()` reads a clock, which is a host capability, not pure computation: it belongs to the same category as `file.*` and `http.*`, and the block-handler sandbox already forbids it at compile time. So the question is partly "which module" and partly "which layer".

## Options considered

**A — A `time` module in the standard library, alongside `file` and `http`.** Consistent with how the other host-backed capabilities are presented: the standard library exposes them, the world grants them, and a component in a world without a clock does not link. Cost: `datetime` construction becomes world-dependent, which must be stated as clearly as the `http.*` scope note already is.

**B — Constructors on the type itself.** `datetime.now()`, `datetime.parse(...)` — no new namespace, and the constructor sits on the type it constructs. Cost: it is the only type in the language with static constructors, and it needs a rule for what that syntax means.

**C — Host-bridge only.** Neither namespace exists in the language; `datetime` and `bytes` values only ever arrive from a library or a host function. Smallest surface. Cost: a plain Clean program cannot ask what time it is, which is hard to defend.

For `bytes`, the parallel question is whether a constructor exists at all and, if it does, what its argument type is — which cannot be answered before [ADR-0019](0019-precision-modifiers.md) settles whether `integer:8u` is usable surface.

## Decision

**Option A for both — modules in the standard library.** `time.now()` and `time.parse(text)` construct a `datetime`; `bytes.fromText(text)` and `bytes.toText(data)` build and read a `bytes`.

**A's stated cost does not apply.** The ADR expects `datetime` construction to become world-dependent, needing a scope note like `http`'s. It does not: `wasi:clocks/wall-clock` is already in the **portable** L2 catalog ([Platform 02 §2.2.1](../../03%20platform/02-host-bridge.md#221-portable-l2-in-every-world)), available in every world. A component that reads a clock links anywhere, so no scope note is warranted.

**Option B — constructors on the type — was rejected** because it would make `datetime` the only type in the language with static constructors, and that syntax would then need a rule of its own. **Option C** leaves a plain Clean program unable to ask what time it is.

**`time.parse` returns `datetime?`.** Unparseable text is an absence the caller handles, not a failure raised — available since [ADR-0015](0015-optional-type-first-class.md) settled optionals in return position.

**`bytes` gets no constructor from a list of numbers.** The ADR notes that `bytes.from(list<u8>)` leaked WIT vocabulary and that its Clean spelling depended on [ADR-0019](0019-precision-modifiers.md). That is now answered: there is no unsigned 8-bit type in the surface language. The question resolves rather than moves — `bytes` *is* the byte-buffer type, so a list of bytes would have been a second way to say the same thing. Construction goes through text, which is the form a program actually needs.

`bytes.toText` returns `string?` because the invariant that a Clean `string` is UTF-8 holds inside the language and cannot hold for bytes that arrived from outside it.

## Consequences

`datetime` is constructible. It had been a core type of the language with no specified way to produce a value, cited by the data and server libraries and by any program that records when something happened.

`time.now()` is barred from `compiletime` functions — a compile-time value that varies with the moment of compilation would break build reproducibility, and the sandbox already stubs the import to an error. The ban [21 — Block Handlers](../../04%20language/21-block-handlers.md) already stated now refers to a function that exists.

The `time.*` claim in [09 — Libraries Specification](../../02%20components/framework/09-libraries-specification.md) is true for the first time.

---

## Metadata

- **Status:** Accepted (2026-08-02)
- **Date:** 2026-08-01
- **Supersedes:** None
- **Spec impact:** [04 language / 15 — Standard Library](../../04%20language/15-standard-library.md) (new §Time Module, §Bytes Module, `STD-01`)
