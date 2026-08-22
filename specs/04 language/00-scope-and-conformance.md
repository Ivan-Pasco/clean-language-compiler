# 00. Scope and Conformance

This chapter says what the Clean Language specification covers, what the words MUST, SHOULD, and MAY mean everywhere in it, and what an implementation has to do before it may call itself an implementation of Clean. Every other chapter states rules about programs; this one states the rules about the specification itself and about the implementations that claim to follow it. It exists so that "valid Clean program" and "conforming implementation" are defined terms with one home, rather than assumptions each reader fills in differently.

---

## 0.1 Scope

This specification defines the Clean Language: which source texts are valid Clean programs, and what observable behavior a valid program has when compiled and run. "Observable behavior" is the program's interaction with its host through the declared bridge surface — values produced, host functions called and in what order, diagnostics reported, traps raised.

The following are **out of scope** for this specification:

- **Performance.** How fast a conforming implementation compiles or runs a program is a quality property, not a conformance property. Performance commitments live in [01 governance / 09 — Performance Principles](../01%20governance/09-performance-principles.md).
- **Internal structure of implementations.** Pass layout, intermediate representations, and packaging are implementation decisions. The reference implementation's internals are described in [Platform 14](../03%20platform/14-compiler-architecture.md) and its ADRs; they bind the reference implementation, not the language.
- **Tooling surfaces.** The `cln` command surface, project layout, and IDE behavior are owned by their components ([02 components](../02%20components/), [Platform 04](../03%20platform/04-ide-lsp-architecture.md)).

## 0.2 Normative references

The following external documents are incorporated by reference. Where a version is pinned, the pin's home is cited rather than repeated here.

- **RFC 2119** and **RFC 8174** — the meaning of the uppercase key words ([§0.3](#03-normative-vocabulary)).
- **WebAssembly Component Model and WIT** — the compilation target and interface language; versions and resolution rules pinned in [Platform 15](../03%20platform/15-component-model-architecture.md) and [Platform 08](../03%20platform/08-bridge-versioning.md).
- **WASI** — the standard host interfaces the bridge composes with; baseline versions pinned in [Platform 08 §8.0](../03%20platform/08-bridge-versioning.md).
- **The Unicode Standard (UTF-8)** — source text encoding; the invariant and who validates it are defined in [Platform 17](../03%20platform/17-text-encoding.md).

## 0.3 Normative vocabulary

### CNF-01 — Uppercase key words carry RFC 2119 meaning

*(Addresses: C-23)*

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **MAY**, and **OPTIONAL** in the specification tree are to be interpreted as described in RFC 2119, as clarified by RFC 8174: they carry normative force **only when they appear in uppercase**. Lowercase "must", "should", and "may" are ordinary prose. Text that states requirements without these key words (grammar productions, tables, error-code conditions) is normative by virtue of the chapter that owns it; the key words mark requirement strength, they do not mark which text is normative.

## 0.4 Conformance

Clean has **one conformance class**. An implementation either conforms to this specification or it does not; there are no partial profiles, feature levels, or optional chapters. (Hosts have their own, mechanical conformance regime — see [CNF-05](#cnf-05--host-conformance-is-delegated-to-the-wit-contract) — which is a different contract, not a profile of this one.)

### CNF-02 — A valid Clean program is what the Accepted chapters admit

*(Addresses: C-23, C-26)*

A source text is a **valid Clean program** if and only if it satisfies every rule of every `Accepted` chapter of this specification: it is well-formed under [Platform 17](../03%20platform/17-text-encoding.md) and the lexical rules of [03](./03-lexical-structure.md), derivable from the grammar (the EBNF files under [`grammar/`](./grammar/), the syntax authority per [DOC-15](../01%20governance/00-documentation-principles.md)), and free of violations of the semantic rules registered in [Platform 09](../03%20platform/09-error-codes.md)/[10](../03%20platform/10-semantic-rules.md). A text that violates any such rule is not a valid Clean program, and the violated rule's registered diagnostic code names the reason.

### CNF-03 — A conforming implementation accepts, rejects, and behaves exactly as specified

*(Addresses: C-02, C-10, C-26)*

An implementation of Clean is **conforming** if and only if, for every source text presented to it:

1. **Acceptance.** It accepts every valid Clean program ([CNF-02](#cnf-02--a-valid-clean-program-is-what-the-accepted-chapters-admit)).
2. **Rejection.** It rejects every text that is not a valid Clean program, reporting the diagnostic code registered for the violated rule in [Platform 09](../03%20platform/09-error-codes.md) — not a different code, and not silence.
3. **Behavior.** The compiled program exhibits the observable behavior this specification defines, on every conforming host, under every optimization profile (optimization is semantics-preserving; [Platform 14 §14.4.2 pass 8](../03%20platform/14-compiler-architecture.md#1442-detailed-pass-responsibilities)).
4. **Determinism.** Compilation is deterministic per [CMP-02](../03%20platform/14-compiler-architecture.md#cmp-02--same-request-in-byte-identical-outputs-out): the same inputs produce byte-identical outputs.

### CNF-04 — No dialects: a conforming implementation adds nothing

*(Addresses: C-16, C-23, C-26)*

A conforming implementation MUST NOT extend the language: it MUST reject any source text that is not a valid Clean program, including texts that would be valid under a superset syntax or relaxed semantics of its own invention. There is no conforming way to ship an extension, an extension flag, or a vendor mode. A tool that accepts a superset of Clean is not a conforming implementation of Clean, whatever else it may be. This is [LDR-08](./02-language-design-rules.md#ldr-08--one-way-to-do-things) applied to implementations: one language, no dialects.

### CNF-05 — Host conformance is delegated to the WIT contract

*(Addresses: C-15, C-16)*

This chapter defines conformance for *implementations of the language*. A **host** is conforming under a separate, mechanical regime: it is correct if and only if it exports every function of the target world's WIT at the declared version, with the documented observable behavior ([C-15](../01%20governance/05-concerns.md); [Platform 16](../03%20platform/16-host-contract-validation.md)). Nothing in this chapter adds requirements on hosts.

### CNF-06 — Implementation-defined behavior is enumerated, never invented

*(Addresses: C-04, C-10, C-23)*

An implementation has discretion only where this specification explicitly grants it, and every such grant is recorded: either the granting rule names the choice in place, or the choice is pinned in an ADR under [`01 governance/decisions/`](../01%20governance/decisions/) (pattern: [ADR-0010](../01%20governance/decisions/0010-implementation-defined-parser-decisions.md)). Outside those recorded grants there is no implementation-defined behavior, and there is **no undefined behavior anywhere**: for every input, a conforming implementation's behavior is derivable from this specification plus the recorded grants.

### CNF-07 — The specification outranks every artifact that checks it

*(Addresses: C-24, C-26)*

The conformance corpus ([05 execution / testing](../05%20execution/testing/00-testing-strategy-overview.md)) is the mechanical evidence of conformance, and the reference implementation is its most thorough exercise — but neither defines the language. Where the corpus, the reference implementation, and this specification disagree, **the specification text governs**, and the disagreement is a defect in the artifact that diverges ([SDD](../01%20governance/03-spec-driven-design.md); [C-26](../01%20governance/05-concerns.md)). Where the specification is silent on a question the corpus or an implementation answers, the silence is the defect: it is reported and resolved spec-first, never papered over by promoting the artifact's accidental answer to a rule.

---

## Metadata

- **Status:** Accepted (2026-08-22)
- **Audience:** Implementers of Clean Language compilers, hosts, and conformance tooling; spec authors
- **Rule prefix:** `CNF-`
- **Part of:** [Clean Language Specification — Language](./README.md)
- **References:** [01 governance / 00 — Documentation Principles](../01%20governance/00-documentation-principles.md) (DOC-15, statuses), [01 governance / 05 — Concerns](../01%20governance/05-concerns.md) (C-15, C-26), [Platform 09 — Error Codes](../03%20platform/09-error-codes.md), [Platform 14 — Compiler Architecture](../03%20platform/14-compiler-architecture.md) (CMP-02), [Platform 16 — Host Contract Validation](../03%20platform/16-host-contract-validation.md), [Platform 17 — Text Encoding](../03%20platform/17-text-encoding.md)
