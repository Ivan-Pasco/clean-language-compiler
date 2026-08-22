# ADR-0009 — String pattern vocabulary

Three rival vocabularies for named string patterns (`"email"`, `emailPattern`, plus a phantom `string.match` API) coexisted across the spec tree, and the standard library — the natural home — declared none of them. This ADR gives the vocabulary one home in the standard library as a closed list of twelve named constants; both `string.matches(pattern)` and the validator's `match:` rule consume those constants rather than owning their own overlapping lists.

---

## Context

Three rival vocabularies for string patterns coexist in the tree, and the standard library chapter — the natural home — defines none of them:

- **Bare pattern names, compile-time checked.** [SEM010](../../03%20platform/10-semantic-rules.md#sem010--invalid-match-pattern) specifies `someString.matches("email")` with a closed set of built-in names (`"email"`, `"url"`, `"uuid"`, `"slug"`, `"numeric"`, `"alpha"`, `"phone"`, `"date"`), extensible by importing pattern packs (`import: validate.patterns.financial` unlocks `"iban"`, `"ssn"`, `"creditCard"`), argument restricted to a string literal so the name is validated at compile time.
- **`string.match` for arbitrary patterns.** [11 §1](../../03%20platform/11-stdlib-validator.md#1-overview) points regex-shaped needs at a "built-in `string.match`" — a function no chapter of [15 — Standard Library](../../04%20language/15-standard-library.md) defines.
- **`…Pattern` identifiers plus glob strings.** [11 §2.3](../../03%20platform/11-stdlib-validator.md#2-concepts) gives the validator's `match:` rule its own vocabulary: twelve named identifiers (`emailPattern`, `urlPattern`, `phonePattern`, `uuidPattern`, …) that overlap SEM010's list under different names — each with a default error message in [11 §6](../../03%20platform/11-stdlib-validator.md#6-default-error-messages) — plus custom patterns as glob strings (`*`, `?`).

The three surfaces disagree on naming (`"email"` vs `emailPattern`), on extension (pattern packs vs nothing), on custom patterns (none vs regex vs glob), and on checking (compile-time literal-only vs runtime). This was a genuine design gap, not a conflict resolvable by precedence: no document was the Accepted home of the pattern vocabulary.

## Options considered

**A — One vocabulary of named constants, home in the standard library.** The standard library declares named pattern constants — the `…Pattern` identifiers of 11 §2.3, a closed list of twelve — and both `string.matches(pattern)` and the validator's `match:` rule consume them. One home, one naming shape, one owner; the validator cites the vocabulary instead of owning it. Cost: SEM010 must be rewritten from literal-string checking to constant checking, and the two overlapping lists must be reconciled by retiring one of them.

**B — Bare string names in `string.matches`.** Keep SEM010's shape: pattern names are string literals with no declaration anywhere, extended by importing pattern packs. Rejected: the names have no home and no type — nothing in the language surface distinguishes a pattern name from any other string — and pack-based extension reopens the vocabulary without an owner, so it cannot be extended safely.

**C — Vocabulary owned by the validator.** Keep 11 §2.3 as the defining home and make `string.matches` consume the validator's identifiers. Rejected: the string module of the standard library would depend on a validation module — the dependency points the wrong way; core string matching cannot hinge on an optional validator.

## Decision

Option A: **one vocabulary, with its home in [15 — Standard Library](../../04%20language/15-standard-library.md)**. The standard library declares the named pattern constants `emailPattern`, `urlPattern`, `phonePattern`, `uuidPattern`, `integerPattern`, `numberPattern`, `alphanumericPattern`, `slugPattern`, `datePattern`, `timePattern`, `ipv4Pattern`, `hexColorPattern` — the closed list of twelve that [11 §2.3](../../03%20platform/11-stdlib-validator.md#2-concepts) and [§6](../../03%20platform/11-stdlib-validator.md#6-default-error-messages) already define with their default messages. `string.matches(pattern)` MUST receive one of these constants, not a bare string; the validator's `match:` rule MUST cite the standard-library constants rather than own them; SEM010's bare names (`"email"`, …) and its pattern packs are retired.

## Consequences

**Easier:**

- The pattern vocabulary has exactly one home: a pattern name is a declared constant, so "is this a known pattern" is an ordinary identifier check, and extending the vocabulary is an ordinary spec change to the standard library chapter — no packs, no unowned names.
- Validator and string surface can no longer drift apart: both consume the same constants.

**Harder / retired:**

- SEM010 must be rewritten against the standard-library vocabulary: the literal-string restriction and the bare-name catalog give way to a check against the declared constants; pattern packs (`import: validate.patterns.financial`, `"iban"`, `"ssn"`, `"creditCard"`) disappear.
- SEM010's bare names that lack a constant in the list of twelve (`"alpha"`, `"numeric"` as spelled there) are retired with the bare-name surface; the twelve constants are the vocabulary.

**Follow-up spec edits on acceptance ([DOC-07](../00-documentation-principles.md#doc-07--the-ladder-of-intent)):**

- [15 — Standard Library](../../04%20language/15-standard-library.md): declare the twelve pattern constants and the `string.matches(pattern)` surface in the String module — this chapter becomes the vocabulary's home.
- [11 — Stdlib Validator §2.3](../../03%20platform/11-stdlib-validator.md#2-concepts): replace the defining catalog with a citation of the standard-library constants; [§6](../../03%20platform/11-stdlib-validator.md#6-default-error-messages) default messages key on the cited constants. The undefined `string.match` pointer in §1 is reconciled to the `string.matches` surface in the same edit.
- [10 — Semantic Rules SEM010](../../03%20platform/10-semantic-rules.md#sem010--invalid-match-pattern): rewrite against the standard-library vocabulary; remove the pending-vocabulary marker.

**Companion decisions (ratified 2026-08-01, same approval):**

- **Arbitrary patterns:** the glob strings of the validator's `match:` constraint (`"INV-???-*"`) are the **validator's own micro-syntax** — declared as such in [Platform 11 §2.3](../../03%20platform/11-stdlib-validator.md), distinct from the pattern-constant vocabulary. They already behave this way; the spec now says so. The phantom pointer in 11 §1 to a `string.match` API (which does not exist) is removed. Any future general regex surface remains undecided; nothing may build on one.
- **`alphaPattern` and `numericPattern`** are added to the stdlib vocabulary (14 constants total), so every legacy bare name of SEM010 (`"alpha"`, `"numeric"` included) has an exact destination.

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Date:** 2026-08-01
- **Supersedes:** None
- **Spec impact:** [15 — Standard Library](../../04%20language/15-standard-library.md) (String module — new home of the vocabulary), [10 — Semantic Rules SEM010](../../03%20platform/10-semantic-rules.md#sem010--invalid-match-pattern), [11 — Stdlib Validator §2.3, §6](../../03%20platform/11-stdlib-validator.md#2-concepts)
