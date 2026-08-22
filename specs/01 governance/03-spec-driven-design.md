# Spec-Driven Design — Clean Language Ecosystem

In an ecosystem built primarily by AI agents, every design decision the spec does not make will be made by a code-generating agent — silently, and differently each run. This document is the small set of rules that make the spec the place where design actually happens, and that make code stay faithful to the spec after it ships. Part 1 covers how specifications should be designed; Part 2 covers what "the code matches the spec" means in practice, and how to keep the two aligned when reality bites back.

---

## Part 0 — Purpose

The [Documentation Principles](00-documentation-principles.md) govern the *document system*: statuses, templates, IDs, precedence. This document governs the two activities that system exists to serve: **designing through specifications** and **keeping code faithful to them**. Together with the [Quality Playbook](02-quality-playbook.md), which mechanizes enforcement, they close the loop: design → law → enforcement.

---

## Part 1 — Designing specifications

### SDD-01 — Design happens in the spec, or it happens by accident

*(Addresses: C-23, C-26)*

Every system is designed exactly once; the only question is whether that happens deliberately in a document or emergently in code, one improvised decision at a time. Design work MUST be done at the spec rung of the ladder ([DOC-07](00-documentation-principles.md)), where a wrong decision costs a paragraph, not a refactor.

**Why:** Prose is the cheapest point to be wrong. Whatever design the spec omits, the implementing agent will supply — invisibly and non-deterministically.

### SDD-02 — Specify the observable, never the mechanism

*(Addresses: C-05, C-08)*

A specification states what can be verified from outside: inputs, outputs, diagnostics, invariants, protocol. It MUST stay silent on internal mechanism.

- **Test:** if two teams could build wildly different internals and both satisfy the document, it is a spec. If only one implementation could satisfy it, it is a design memo wearing a spec's clothes.
- Mechanism choices worth recording are ADR material ([SDD-07](#sdd-07--record-which-decisions-were-actually-made)), not spec text.

**Why:** Mechanisms churn; observable contracts don't. Specifying behavior keeps the spec stable and the implementation free.

### SDD-03 — What the system will NOT do is half the design

*(Addresses: C-09, C-23)*

Every responsibility list MUST be paired with its negative: "is NOT responsible for" lists, boundary tests, non-goals. Scope is defined by its edges.

- The [Architecture Boundaries](01-architecture-boundaries.md) "IS / is NOT / Boundary test" pattern is the reference form.
- The task brief template's *Non-goals* section is never empty ([00 — Documentation Principles §2.2](00-documentation-principles.md)); if truly nothing is excluded, the scope is too big.

**Why:** A spec that only says what a component does leaves every adjacent capability ambiguous, and ambiguity is where responsibility drift begins.

### SDD-04 — The error path is part of the design

*(Addresses: C-02, C-26)*

A specification MUST define behavior for malformed input, unavailable dependencies, and rule collisions — enumerated and named (diagnostic codes, per the [Error Codes](../03%20platform/09-error-codes.md) registry pattern) — before it is `Accepted`.

**Why:** The happy path is usually obvious; a system's real shape is what it does under failure. A spec that hand-waves failure has designed a third of the system, and the generating agent invents the rest.

### SDD-05 — Every normative statement must be falsifiable

*(Addresses: C-26)*

For each normative statement it MUST be possible to build an artifact that violates it, and to detect the violation mechanically. This is [DOC-13](00-documentation-principles.md) applied at design time.

- **Review test:** for each statement ask "what concrete artifact would break this rule?" No answer → the statement is decoration and MUST be rewritten or demoted to informative text.

**Why:** A statement that cannot be violated constrains nothing, so it drives nothing.

### SDD-06 — The spec is the meeting point

*(Addresses: C-23, C-25)*

The specification is where every party that must agree — implementer, test author, adjacent component, each AI agent — meets. Agreement MUST live in the spec text, never in conversations, memory files, or shared context between agents.

**Why:** Once the contract holds the agreement, the parties decouple: the test-writing agent and the code-writing agent can work blind to each other and still converge, because they read the same rules. Conversations don't scale past one context window.

### SDD-07 — Record which decisions were actually made

*(Addresses: C-24)*

Most of a spec is forced — by the domain, by governance, by adjacent contracts. The few points that were genuine choices between viable alternatives MUST be captured as ADRs ([DOC-07](00-documentation-principles.md#doc-07--the-ladder-of-intent)), and the spec text MUST link back to them as rationale.

**Why:** Recorded rationale is what lets a reader distinguish "load-bearing, decided, don't reopen" from "incidental, propose away." A spec without it invites perpetual re-litigation of settled questions.

---

## Part 2 — Code–spec fidelity

### SDD-08 — Code is a projection of the spec, never a source of truth

*(Addresses: C-26)*

Behavior is defined in exactly one place — the `Accepted` spec — and code merely realizes it. When code and spec disagree, the code is wrong, by definition: even if it shipped, even if users depend on it. "The code works" is never an argument in a spec conflict; at most it is input to a proposal to change the spec.

**Why:** This is [DOC-11](00-documentation-principles.md) precedence rank 5, elevated to a principle. The moment observed behavior can outrank the document, the spec becomes description instead of law.

### SDD-09 — Fidelity is bidirectional; silence in the spec is a defect, not a license

*(Addresses: C-26)*

- *Completeness:* every MUST in the spec MUST be implemented.
- *Conservatism:* no observable behavior may exist in code that the spec does not state.

When the spec is silent on a case, the implementer MUST NOT pick something reasonable; the gap is reported one rung up ([DOC-07](00-documentation-principles.md)) and the spec is fixed first.

**Why:** Every "reasonable choice" an implementer silently makes is a shadow spec living only in the code. Shadow specs are how V1 drifted.

### SDD-10 — Divergence is repaired spec-first

*(Addresses: C-26, C-24)*

When implementation reveals the spec should change — wrong, incomplete, impractical — the sequence is fixed: propose (ADR if it is a decision), amend the spec, then change the code. Code MUST NOT move first with the document backfilled after.

**Why:** The moment code moves first, the spec documents the implementation instead of governing it, and SDD-08 is dead. This is the [DOC-07](00-documentation-principles.md) ladder traversed upward.

### SDD-11 — Fidelity is verified mechanically, never trusted

*(Addresses: C-24, C-26)*

- Every normative rule ID MUST have at least one test citing it (`// verifies LEX-01`).
- The parity check ([Quality Playbook](02-quality-playbook.md) E13) fails when a rule has no citing test or a test cites a withdrawn rule.
- A spec rule with no citing test is in the same state as an unintegrated ADR: written, but not yet in force.

**Why:** "Hooks enforce; prompts request," applied to fidelity. A conformance claim that isn't checked will decay.

### SDD-12 — Tests derive from the spec, not from the code

*(Addresses: C-26)*

Conformance tests MUST be authored from the spec text — by an actor reading only the spec, which the [DOC-07](00-documentation-principles.md) reading rules already provide: the test-writing task brief cites rule IDs, not source files.

- Snapshot tests (Playbook E10) guard against *unintended change*, not spec violation, and MUST NOT be counted as conformance evidence.

**Why:** A test written by observing the implementation merely notarizes current behavior, drift included.

### SDD-13 — Traceability runs both ways

*(Addresses: C-24)*

Rule → tests is [SDD-11](#sdd-11--fidelity-is-verified-mechanically-never-trusted). In the other direction, any nontrivial code region SHOULD answer "which rule requires you to exist?" — via module-level mapping plus test citations, not per-line annotation.

**Why:** Code that traces to nothing is either missing spec (an [SDD-09](#sdd-09--fidelity-is-bidirectional-silence-in-the-spec-is-a-defect-not-a-license) violation) or dead weight. Bidirectional tracing turns "is the implementation faithful?" from a review vibe into a coverage query.

---

## Part 3 — Summary

Design in prose where deciding is cheap; state only what is observable; define the edges and the failures, not just the center; write nothing that cannot be violated; let the document hold the agreement; mark which parts were chosen rather than forced. Then: the spec defines and code realizes; silence is a defect, not a license; change flows spec-first; and fidelity is a CI check, not a promise.

---

## Metadata

- **Status:** Accepted (2026-07-30)
- **Audience:** Anyone — human or AI agent — writing a specification or implementing from one
- **Rule prefix:** `SDD-`
- **References:** [Documentation Principles](00-documentation-principles.md) — DOC-07, DOC-11, DOC-13; [Architectural Concerns](05-concerns.md); [Quality Playbook](02-quality-playbook.md)
