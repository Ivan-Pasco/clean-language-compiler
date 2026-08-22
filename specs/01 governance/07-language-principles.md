# Language Principles — Clean Language

Clean Language exists to be pleasant to read and to write, and the twenty principles in this document are the durable commitments that shape it toward that goal. They answer the "why is the language this way" questions — why block structure is whitespace, why there is no null, why generics are closed to containers, why tests live in source — without pinning down the exact keyword or syntax that carries each commitment today. The specification chapters under `04 language/` state the mechanics; these principles state the policy the mechanics have to honour, so that a spec change that would quietly retire a commitment is caught before it ships.

---

## Part 0 — Purpose

The Clean Language specification (`04 language/`) is thousands of lines of normative detail. This document names the durable commitments those details flow from. A proposed change to any language chapter MUST be checked against the principles here: if the change contradicts a principle, either the change is defective or the principle is being retired (which requires an ADR under [DOC-07](00-documentation-principles.md#doc-07)).

**Policy, not mechanism.** Every LANG-NN below states a durable rule about the language's behaviour, syntax shape, or organisation without naming the specific technology, syntax token, flag, or file that currently implements it. When a principle depends on a concrete mechanism (indentation unit, keyword name, escape-hatch syntax), the principle points at the spec chapter that carries the mechanic or at [ADR-0022](decisions/0022-foundational-technology-stack.md) for foundational technology choices.

Principles are durable. Specifications are versioned. Foundational technology choices are ADR-recorded and supersedable. When principle and specification disagree, the specification wins on mechanics and the principle wins on intent — reconcile by opening an ADR, never by silently changing either.

Each principle carries a stable `LANG-NN` ID ([DOC-13](00-documentation-principles.md#doc-13)) and cites the architectural concerns ([05 — Concerns](05-concerns.md)) it addresses.

---

## Part 1 — The Principles

### Developer Experience — the foundation

*Clean Language exists to be pleasant to read and write. These four principles come first because everything else in the language is a technique for delivering on them.*

#### LANG-01 — Readable first, then everything else

*(Addresses: C-01, C-02)*

Clean Language's primary allegiance is to the human reader. Every other design choice — safety, performance, feature scope, elegance for the language designer — is subordinate to whether the resulting code reads clearly to a developer on first encounter, including a beginner opening a file for the first time and an AI agent generating or reviewing code.

When two designs are otherwise comparable, the one that produces more readable code at the call site MUST win. When a feature would make writing easier but reading harder, the feature is a defect. Terseness is not readability; consistency and obviousness are.

Every principle below is a *technique* for delivering on LANG-01, not an alternative to it. If a downstream rule ever appears to conflict with readability at the call site, LANG-01 is the tiebreak — the downstream rule is either being misapplied or is due for revision.

Source: [04 language / 01 — Overview](../04%20language/01-overview.md) (design goals); [04 language / 02 — Language Design Rules](../04%20language/02-language-design-rules.md).

#### LANG-02 — Reads like plain English

*(Addresses: C-01, C-02)*

Keywords, method names, and block markers MUST be pronounceable English words or short phrases. A non-programmer reading a well-written Clean function aloud SHOULD be able to describe roughly what it does.

Punctuation-heavy or symbolic syntax is a defect wherever an English word would carry the same meaning. Symbols are reserved for well-established mathematical operators and for the small set of structural tokens the language commits to.

The specific keyword vocabulary, operator set, and the boundary between "an English word is used" and "a symbol is acceptable" are defined in [04 language / 03 — Lexical Structure](../04%20language/03-lexical-structure.md) and enumerated across the chapters they belong to (control flow, contracts, testing, state, async).

Source: [04 language / 03 — Lexical Structure](../04%20language/03-lexical-structure.md); [04 language / 02 — Language Design Rules](../04%20language/02-language-design-rules.md).

#### LANG-03 — Developer time is the scarce resource

*(Addresses: C-01, C-02, C-08, C-09)*

When a tradeoff appears between work the compiler could do and work the developer would have to do, the compiler MUST do the work. Language complexity is acceptable if it hides complexity from the reader; it is unacceptable if it merely shifts complexity onto the developer.

Concrete consequences the specification MUST honour:

- **Inference over annotation.** Types are inferred wherever unambiguous (LANG-08); annotations are required only where inference would guess.
- **Errors that tell the developer what to do next** ([C-02](05-concerns.md)), not only what went wrong. Stringly-typed error messages are a defect.
- **A single, discoverable command surface.** The developer never invokes component binaries directly or hand-edits toolchain internals ([C-03](05-concerns.md)). The concrete command binary and layout are recorded in [ADR-0022](decisions/0022-foundational-technology-stack.md) and specified in [02 components / manager / 00 — Manager](../02%20components/manager/00-manager.md).
- **Sensible defaults over required configuration.** A new project MUST run with zero configuration; any setting the developer might reasonably not care about MUST have a default that "just works."
- **Fast, deterministic feedback.** Reproducible builds ([C-04](05-concerns.md)), stable diagnostic codes ([C-02](05-concerns.md)), and language-server-as-single-source-of-truth are all instances of this principle.

Language-designer elegance, feature completeness, and performance micro-optimisations MUST NOT be pursued at the cost of developer thinking time.

Source: [04 language / 01 — Overview](../04%20language/01-overview.md); [05 — Architectural Concerns](05-concerns.md) C-01, C-02, C-03, C-04.

#### LANG-04 — One obvious way, always

*(Addresses: C-01, C-02)*

Every operation MUST have exactly one canonical name and one canonical calling style. Aliases, synonyms, and alternate forms of the same operation are defects. The developer never spends effort choosing between equivalent forms — because there are none.

The specific canonical mappings (operators vs. namespace calls for math, method-style vs. namespace calls for utilities, naming conventions for namespaces, explicit parentheses on invocations) are defined in [04 language / 02 — Language Design Rules](../04%20language/02-language-design-rules.md) and [04 language / 16 — Method-Style Syntax](../04%20language/16-method-style-syntax.md).

Source: [04 language / 02 — Language Design Rules](../04%20language/02-language-design-rules.md); [04 language / 16 — Method-Style Syntax](../04%20language/16-method-style-syntax.md).

### Structural clarity

*How DX shows up in the shape of a file.*

#### LANG-05 — Whitespace defines block structure

*(Addresses: C-01)*

Block structure MUST be defined by leading whitespace, not by explicit terminators (braces, `end`, `endif`, or equivalents). A file's shape on the page is its shape in the AST.

The specific indentation unit is a foundational technology choice recorded in [ADR-0022 §5](decisions/0022-foundational-technology-stack.md); the rules for tokenising and enforcing indentation live in [04 language / 03 — Lexical Structure](../04%20language/03-lexical-structure.md).

Source: [04 language / 03 — Lexical Structure](../04%20language/03-lexical-structure.md); [ADR-0022 §5](decisions/0022-foundational-technology-stack.md).

#### LANG-06 — The language provides a native remedy for horizontal repetition

*(Addresses: C-01)*

Repeated application of the same operation to a list of arguments MUST be expressible through a native language construct that collapses the repetition, rather than as N separate calls or through a text-macro system.

The concrete construct (apply-blocks) is defined in [04 language / 05 — Apply-Blocks](../04%20language/05-apply-blocks.md). Adding a macro system or code-generation layer to solve the same problem is a defect (see LANG-19 for DSL extension via block handlers instead).

Source: [04 language / 05 — Apply-Blocks](../04%20language/05-apply-blocks.md).

#### LANG-07 — Explicit function organisation

*(Addresses: C-01, C-09)*

Functions MUST be organised inside named grouping blocks — never as standalone top-level declarations that a reader has to hunt for. The same rule applies inside classes: methods live in a named grouping block. Program entry is a distinct construct, not a magically-named function.

The concrete block names and the entry-point construct are defined in [04 language / 09 — Functions](../04%20language/09-functions.md) and [04 language / 14 — Classes and Objects](../04%20language/14-classes-and-objects.md).

Source: [04 language / 02 — Language Design Rules](../04%20language/02-language-design-rules.md); [04 language / 09 — Functions](../04%20language/09-functions.md); [04 language / 14 — Classes and Objects](../04%20language/14-classes-and-objects.md).

### Safety and Correctness

#### LANG-08 — Strong static typing with inference

*(Addresses: C-02, C-08)*

Every value MUST have a statically known type at compile time. Types SHOULD be inferred wherever the inference is unambiguous; annotations are required only where inference would guess.

Source: [04 language / 01 — Overview](../04%20language/01-overview.md); [04 language / 04 — Type System](../04%20language/04-type-system.md).

#### LANG-09 — Absence is a value, not a hole

*(Addresses: C-02)*

The language MUST NOT have a null pointer or a null literal. Absence MUST be expressed as an explicit, typed value distinct from zero, false, empty string, and empty collection. A variable that may be absent MUST be typed to reflect that; the compiler enforces the distinction.

The specific value name and type-system encoding are defined in [04 language / 04 — Type System](../04%20language/04-type-system.md).

Source: [04 language / 03 — Lexical Structure](../04%20language/03-lexical-structure.md); [04 language / 04 — Type System](../04%20language/04-type-system.md).

#### LANG-10 — No implicit conversions

*(Addresses: C-02)*

The language MUST NOT convert between types silently. Conversions across type boundaries MUST be explicit, named, and visible in the source.

Source: [04 language / 04 — Type System](../04%20language/04-type-system.md).

#### LANG-11 — Type escape hatch is compile-time only

*(Addresses: C-02, C-08)*

The language MUST provide an explicit, opt-in mechanism for values whose type genuinely cannot be known at compile time (external data, library returns, JSON parsing). This mechanism MUST be a compile-time construct — the compiler declines to check that specific value — and MUST NOT be implemented as a runtime box, type tag, or reflection surface. It MUST NOT be inferred silently; using it is a visible choice the developer makes.

The specific type name and syntax are defined in [04 language / 04 — Type System](../04%20language/04-type-system.md).

Source: [04 language / 02 — Language Design Rules](../04%20language/02-language-design-rules.md); [04 language / 04 — Type System](../04%20language/04-type-system.md).

#### LANG-12 — Generics are closed; containers only

*(Addresses: C-01, C-09)*

Generic type parameters MUST be reserved for the language's built-in container types. User code MUST NOT declare its own generic classes or generic functions. Heterogeneity that cannot be expressed with the built-in containers uses the type escape hatch (LANG-11).

The specific container types, their syntax, and the boundary between "built-in generic" and "user-visible generic" are defined in [04 language / 04 — Type System](../04%20language/04-type-system.md).

Source: [04 language / 02 — Language Design Rules](../04%20language/02-language-design-rules.md); [04 language / 04 — Type System](../04%20language/04-type-system.md).

#### LANG-13 — Contracts are first-class; preconditions cannot be silenced

*(Addresses: C-02)*

Design-by-contract MUST be available as a native language construct with three roles: preconditions (guarding function entry), postconditions (guarding function return), and invariants (guarding an object's lifetime). Contracts are opt-in per function or class — code without them is valid.

When a developer writes a precondition, the compiler MUST always emit and enforce it; no build option may strip it, because a precondition guards the caller's contract with the function. Postconditions and invariants MAY be stripped in release builds, since they verify the implementor's work rather than the caller's contract; stripping MUST be a whole-project build choice, not a per-contract override.

The specific keyword names (`before`, `after`, `always`), the build option that strips postconditions and invariants, and the exact evaluation rules are defined in [04 language / 10 — Contracts](../04%20language/10-contracts.md).

Source: [04 language / 10 — Contracts](../04%20language/10-contracts.md).

### Structure of a Program

#### LANG-14 — State is first-class and observable

*(Addresses: C-01, C-02)*

Program state MUST be declared explicitly, mutated through named operations, and observable through a native language construct — never through ad-hoc globals or hidden side effects. Computed state MUST be a read-only derivation of other state, re-evaluated only when its declared dependencies change.

The specific observation construct, mutation rules, and dependency-tracking semantics are defined in [04 language / 20 — State Management](../04%20language/20-state-management.md).

Source: [04 language / 20 — State Management](../04%20language/20-state-management.md).

#### LANG-15 — Asynchrony is simple and sequential-by-default

*(Addresses: C-01, C-02)*

Background execution MUST be expressible through named language constructs for launching, awaiting, and marking always-async functions. State mutations from background tasks MUST remain sequential — the language MUST NOT expose interleaved memory races to the programmer. Errors in background tasks MUST NOT propagate implicitly; they surface only when the result is accessed or when a handler is explicitly attached.

The specific launch, await, and error-handler constructs are defined in [04 language / 18 — Async](../04%20language/18-async.md) and [04 language / 13 — Error Handling](../04%20language/13-error-handling.md).

Source: [04 language / 18 — Async](../04%20language/18-async.md); [04 language / 13 — Error Handling](../04%20language/13-error-handling.md); [04 language / 20 — State Management](../04%20language/20-state-management.md).

#### LANG-16 — Errors are values with handlers, not exceptions

*(Addresses: C-02)*

Errors MUST propagate through the type system and be handled by named language constructs. The language MUST NOT have unwinding-based exceptions or non-local control flow that bypasses the type system. Every diagnostic emitted by the compiler or runtime MUST carry a stable code from a documented registry.

The specific handler construct, the error registry, and the diagnostic-code scheme are defined in [04 language / 13 — Error Handling](../04%20language/13-error-handling.md) and [03 platform / 09 — Error Codes](../03%20platform/09-error-codes.md).

Source: [04 language / 13 — Error Handling](../04%20language/13-error-handling.md); [03 platform / 09 — Error Codes](../03%20platform/09-error-codes.md).

### Tests, Specs, and AI

#### LANG-17 — Tests live in source

*(Addresses: C-02)*

Tests MUST be expressible in the same file as the code they verify, through a native language construct, without an external test framework. Test discovery, execution, and reporting are compiler and runtime responsibilities, not third-party ones.

The specific test-block construct and its semantics are defined in [04 language / 11 — Testing](../04%20language/11-testing.md).

Source: [04 language / 11 — Testing](../04%20language/11-testing.md).

#### LANG-18 — Specification traceability is a language feature

*(Addresses: C-12, C-20)*

Code MUST be linkable to its specification through native language keywords. Generated files MUST declare their generator and source, so that a reader — human or agent — can walk from any line of code to the spec section that justifies it.

The specific keywords and their semantics are defined in [04 language / 19 — AI Integration](../04%20language/19-ai-integration.md).

Source: [04 language / 19 — AI Integration](../04%20language/19-ai-integration.md).

#### LANG-19 — DSL extension via sandboxed compile-time handlers, not macros

*(Addresses: C-05, C-07, C-08, C-11)*

New DSL blocks MUST be added by libraries declaring compile-time handler functions that transform block syntax into typed IR. The compiler MUST NOT be modified to add a new block form. Handlers MUST be pure, deterministic, sandboxed, and subject to compiler-enforced time and memory budgets — no I/O, no system state, no wall-clock reads.

The specific handler construct, its sandbox model, and its budget semantics are defined in [04 language / 21 — Block Handlers](../04%20language/21-block-handlers.md).

Source: [04 language / 21 — Block Handlers](../04%20language/21-block-handlers.md).

### Deliberate Omissions

#### LANG-20 — What Clean Language refuses to have

*(Addresses: C-01, C-02, C-08, C-09)*

The following classes of construct are permanently out of scope. Proposals to add any of them require an ADR that retires this rule.

- **Null pointers or a null literal** — see LANG-09.
- **Unwinding-based exceptions and non-local control flow that bypasses the type system** — see LANG-16.
- **Implicit type conversions** — see LANG-10.
- **User-defined generic classes or functions** — see LANG-12.
- **Text macros or preprocessor directives** — DSL extension goes through LANG-19.
- **Explicit block terminators (braces, `end`, `endif`, etc.)** — see LANG-05.
- **Aliases for existing operations, or multiple call styles for the same operation** — see LANG-04.
- **Standalone top-level function declarations outside a grouping block** — see LANG-07.
- **Symbolic or punctuation-heavy syntax where an English word would work** — see LANG-02.
- **Hidden or self-updating language behaviour** — see [C-19](05-concerns.md).

Each omission is a positive design choice, not a deferral. "Because it is simpler for the reader" (LANG-01) is the standing justification; the ADR that would add any of these MUST explain why that justification no longer applies.

---

## Part 2 — How principles interact with the specification

1. A spec chapter MUST NOT contradict a Language Principle. If a proposed spec change would contradict a principle, the change MUST either be revised or accompanied by an ADR that retires the principle (LANG-NN → withdrawn) in the same commit.
2. When adding or amending a normative rule in `04 language/`, the rule SHOULD cite the LANG-NN principle it derives from, in addition to the concerns it addresses ([DOC-14](00-documentation-principles.md#doc-14)).
3. Retiring a principle MUST NOT reuse its ID. The retired principle keeps its number and is marked *Withdrawn (YYYY-MM-DD, ADR-NNNN)*.
4. Principles are stakeholder-neutral by design: they answer "what is Clean Language" rather than "who cares about this." Concern citations carry the stakeholder trace ([05 — Concerns](05-concerns.md)).
5. LANG-01 is the ultimate tiebreak. When two principles pull in different directions in an edge case, the resolution MUST favour the reading (call-site clarity for a developer encountering the code cold), not the writing. LANG-02, LANG-03, and LANG-04 are the primary techniques by which LANG-01 is made concrete.
6. **Principles state policy; specs state mechanics; [ADR-0022](decisions/0022-foundational-technology-stack.md) records foundational technology choices.** A principle that names a specific keyword, flag, file path, or benchmark number is a defect — that content belongs in the spec chapter or the ADR. When a spec or ADR change forces a principle to be reworded, the reword MUST NOT change the policy; if it does, an ADR that retires the affected principle is required.

---

## Metadata

- **Status:** Draft
- **Audience:** Language designers, spec authors, compiler and framework maintainers, library authors, and AI agents generating Clean code
- **Rule prefix:** `LANG-`
- **References:** [Documentation Principles](00-documentation-principles.md) — DOC-13, DOC-14; [Architectural Concerns](05-concerns.md); [ADR-0022 — Foundational Technology Stack](decisions/0022-foundational-technology-stack.md)
