# Architectural Concerns — Clean Language Ecosystem

A concern is a reason a rule exists — the thing a stakeholder cares about that a normative rule in the spec is trying to protect or achieve. This document is the flat, stable list of those concerns, grouped by whoever cares about each. Rules elsewhere in the spec can cite these concerns by ID when the trace is not obvious from the rule's own reading; the list itself changes slowly, so citations to it stay stable across spec reorganizations.

---

## Part 0 — Purpose

Concerns change slowly. Rules change often. Keeping the two separate lets the spec be reorganized, split, or renumbered without losing what it is for.

Each concern has a stable ID (`C-01`, `C-02`, …). Other documents cite concerns by ID, not by paraphrase ([DOC-14](00-documentation-principles.md)). IDs are flat across stakeholders so a concern can be cited without re-classifying it when stakeholder groupings evolve.

Stakeholders appear as section headings only. They are not separately ID'd — the section a concern lives in tells you who cares.

Whether a rule *must* cite a concern is answered by the rules governing citation, not by this document. See Part 2.

---

## Part 1 — Concerns by Stakeholder

### Language User

Developers writing Clean applications.

- **C-01 — Learnability.** A developer productive in a mainstream language reaches "hello world running" in under 30 minutes without reading more than one page of docs.
- **C-02 — Error clarity.** Every compile-time and runtime error tells the user what to do next, not only what went wrong. No error is a stringly-typed message.
- **C-03 — One command.** `cln <verb>` is the entire user surface. The user never invokes component binaries directly and never edits files under `~/.cln/` or `.cln/` by hand.
- **C-04 — Reproducible environments.** Two developers with the same `clean.toml` and `.cln/` get byte-identical builds on the same platform.
- **C-22 — Privacy.** Nothing leaves the developer's machine without explicit consent. Transmitted artifacts (error reports, dumps, telemetry) contain no secrets or personal data beyond what the active consent level permits.

### Library Author

Developers writing Clean libraries and plugins.

- **C-05 — Block handler stability.** A compile-time function written today keeps working on future compiler versions within the same major.
- **C-06 — Host bridge contract.** Declared `host function` signatures are the boundary between a library and its host; host implementations may change internally without breaking libraries that only rely on the declared signatures.
- **C-07 — Authoring surface parity.** Everything a library needs to declare (blocks, host functions, folder conventions, MCP docs) is expressible in `library.toml` and Clean source. No hand-written WIT.

### Compiler Maintainer

- **C-08 — Pure function.** The compiler is a pure function of its inputs. No filesystem access, no network, no library awareness, no plugin execution outside the sandboxed compile-time plugin runtime.
- **C-09 — Small surface.** The compiler's public surface is small enough for a solo maintainer to hold in their head. Every capability the compiler could push to the framework or the manager, it does.
- **C-10 — Determinism and reproducibility.** Same inputs produce byte-identical output across runs, hosts, and platforms declared as reproducible. Repro tooling can replay any historical build.

### Framework Maintainer

- **C-11 — Orchestrator, not compiler.** Clean Framework owns project awareness, block expansion, and manifest building. It never contains parser, type-checker, or codegen logic.
- **C-12 — MCP is the AI-facing contract.** Every capability an AI or IDE needs to reason about a Clean project is exposed through the Clean MCP server, not through ad-hoc file parsing.

### Manager Maintainer

- **C-13 — Single front door.** Clean Manager owns the `cln` command surface, argv parsing, dispatch, and version resolution. Adding a new user-facing verb requires a Manager change.
- **C-14 — Bounded on-disk footprint.** Every artifact the ecosystem installs lives under `~/.cln/` or the project's `.cln/`. Nothing else on the user's machine is modified silently.

### Host Implementor

Authors of runtimes that execute Clean `.wasm` components (server, browser, CLI runner, embedded).

- **C-15 — WIT-based conformance.** A host is correct iff it exports every function in the target world's WIT at the declared version. Conformance is verified mechanically, not asserted.
- **C-16 — Hosts are swappable.** A new host implementation can join the ecosystem by implementing the WIT contract, without changes to compiler, framework, or manager.

### Package Maintainer / Operator

Anyone installing, distributing, or operating Clean in production.

- **C-17 — Provenance and integrity.** Every artifact under `~/.cln/` is checksum-verified against a lockfile. Signature verification is required for core artifacts and one-time-warned for community ones.
- **C-18 — Offline operation.** After a `cln fetch`, every command works with no network access. CI in air-gapped environments is a first-class use case.
- **C-19 — Explicit updates.** No component of the ecosystem updates itself in the background. `cln self-update` is the only mechanism, invoked explicitly.

### AI / MCP Client

AI coding assistants and IDEs querying the Clean toolchain.

- **C-20 — Structured introspection.** The MCP surface answers questions with structured, machine-readable data instead of prose. Answers reflect the exact compiler/framework versions pinned by the project.
- **C-21 — Scaffolding contract.** New external-service connectors, libraries, and plugins are added through documented commands (`cln add`, `cln new`) that write to canonical locations, so an AI can create code that fits without inventing conventions.

### Specification Process

The actors of the execution model — spec authors, brief writers, implementing agents, and the humans who gate them. These concerns are about the document system itself; the product concerns above are what that system exists to protect.

- **C-23 — Deterministic input.** An agent reading the documentation derives exactly one interpretation: every fact has one home, normative text is visibly separated from commentary, templates and vocabulary are fixed, and disposable documents never masquerade as durable ones.
- **C-24 — Traceability and history.** Every rule is walkable to the reason it exists and the tests that verify it; every normative change has a dated record; decisions are append-only. "Why is it this way" always has a stable answer.
- **C-25 — Gated autonomy.** Agents work autonomously exactly within human-approved bounds: statuses gate what may be built from, the ladder gates how far one step may jump, and precisely two human gates stand between intent and code.
- **C-26 — Spec supremacy.** The specification outranks its implementations, always: code is a projection, silence is a defect to report upward, divergence is repaired spec-first, and every normative statement is falsifiable so fidelity can be checked mechanically.

---

## Part 2 — How to cite concerns

Rules SHOULD cite the concern(s) they address on their first line when the trace is not obvious from the rule's plain reading. When the concern is obvious from the rule's title or topic, the citation is decoration and should be omitted ([DOC-14](00-documentation-principles.md), [DOC-17](00-documentation-principles.md)).

When a citation is warranted, use plain comma-separated IDs after the rule title, in parentheses:

```
### SDD-02 — Specify the observable, never the mechanism

*(Addresses: C-05, C-08)*
```

Rules that address more than three concerns are usually saying too much and should be split.

A concern citation is a trace, not a summary. It exists so a reader can walk from a non-obvious rule to the reason it exists — not to decorate every rule with the concerns it obviously serves.

---

## Metadata

- **Status:** Accepted (2026-07-30)
- **Audience:** Anyone writing or reviewing a normative statement in this repository
- **References:** [Documentation Principles](00-documentation-principles.md) — DOC-14, DOC-17 govern how these concerns are cited.
