# 01. Overview

Clean Language is a modern, type-safe programming language that compiles to WebAssembly. This chapter is a reader-orientation to the language — its design goals, its syntax character, its runtime shape. It states no rules of its own; every fact it mentions is owned by the chapter it links to, and this file is here so a newcomer can build a mental map before descending into the specific chapters.

Clean Language emphasizes strong static typing, first-class functions, matrix operations, and comprehensive error handling.

### Design Goals

Each goal below is owned by the durable principles in [01 governance / 07 — Language Principles](../01%20governance/07-language-principles.md); this list is the orientation, the `LANG-NN` principles are the authority.

- **Developer Experience**: readable first, plain-English surface, developer time as the scarce resource ([LANG-01](../01%20governance/07-language-principles.md#lang-01--readable-first-then-everything-else), [LANG-02](../01%20governance/07-language-principles.md#lang-02--reads-like-plain-english), [LANG-03](../01%20governance/07-language-principles.md#lang-03--developer-time-is-the-scarce-resource))
- **Simplicity**: one obvious way to do everything ([LANG-04](../01%20governance/07-language-principles.md#lang-04--one-obvious-way-always); enforced by the rules in [02 — Language Design Rules](./02-language-design-rules.md))
- **Type Safety**: strong static typing with inference, no null, no implicit conversions ([LANG-08](../01%20governance/07-language-principles.md#lang-08--strong-static-typing-with-inference), [LANG-09](../01%20governance/07-language-principles.md#lang-09--absence-is-a-value-not-a-hole), [LANG-10](../01%20governance/07-language-principles.md#lang-10--no-implicit-conversions))
- **Error Handling**: errors are values with handlers, contracts are first-class ([LANG-16](../01%20governance/07-language-principles.md#lang-16--errors-are-values-with-handlers-not-exceptions), [LANG-13](../01%20governance/07-language-principles.md#lang-13--contracts-are-first-class-preconditions-cannot-be-silenced))
- **Expressiveness**: first-class mathematical operations and data structures ([04 — Type System](./04-type-system.md), [15 — Standard Library](./15-standard-library.md))
- **Performance**: efficient compilation to WebAssembly — a quality commitment, not a conformance property ([01 governance / 09 — Performance Principles](../01%20governance/09-performance-principles.md))

### Non-Goals

Clean deliberately refuses a list of constructs — null, exceptions, macros, user generics, and more. That list is normative and lives in one place: [LANG-20 — What Clean Language refuses to have](../01%20governance/07-language-principles.md#lang-20--what-clean-language-refuses-to-have). Each omission is a positive design choice, not a deferral.

### Scope and Conformance

What this specification covers, the meaning of MUST/SHOULD/MAY, and what an implementation must do to call itself Clean are defined in [00 — Scope and Conformance](./00-scope-and-conformance.md).

### File Extension
Clean Language source files use the `.cln` extension.

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Anyone new to Clean Language
- **References:** [Clean Language Specification — Language](./README.md)
