# ADR-0022 — Foundational technology stack

**Status:** Draft

The Clean Language ecosystem rests on a small set of foundational technology choices — what to compile to, how boundaries are described, how the toolchain is invoked, how source is indented — that were made implicitly as the spec took shape and named piecemeal inside otherwise-durable policy documents. This ADR consolidates those eight choices in one place so the principles docs can be rewritten as pure policy and each choice can be superseded individually by a later ADR without touching the others.

---

## Context

The Clean Language ecosystem rests on a small set of foundational technology choices that were made implicitly as the specification took shape: what to compile to, how boundaries are described, how the toolchain is invoked, where artifacts live, how source code is structured on the page. Until now these choices were only visible by reading the spec chapters that embody them.

The problem this ADR exists to solve: the principles docs ([07 — Language](../07-language-principles.md), [08 — Security](../08-security-principles.md), [09 — Performance](../09-performance-principles.md), [10 — Interoperability](../10-interoperability-principles.md)) were repeatedly naming these technologies inside otherwise-durable policy statements. This conflated *why the ecosystem behaves this way* (durable policy) with *how it happens to be implemented today* (mechanism). Under [DOC-07](../00-documentation-principles.md), those are different rungs of the ladder and should not share a document.

Rather than write one ADR per choice (which fragments the story — a reader wanting to understand "why this stack" would have to open six documents), this ADR consolidates the foundational choices in one place. Each subsection can be superseded individually by a later ADR without touching the others; the consolidation is for readability, not for coupling.

The list of foundational choices in scope for this ADR:

1. Compilation target
2. Boundary description language
3. Composition and host loading model
4. Interface versioning scheme
5. Source-code indentation unit
6. Toolchain command surface
7. On-disk artifact layout
8. Reproducibility mechanism

Anything not in this list is either (a) a spec-level mechanic that lives in its owning chapter, or (b) not yet a load-bearing choice.

## Decision

The Clean Language ecosystem is built on the following foundational technology stack. Each choice is Draft pending review; once Accepted, changes to any single choice require a new ADR that supersedes the relevant subsection of this one.

Every choice below carries a **Satisfies:** line naming the principles it exists to serve. This is the reverse-index that lets a reader on a spec chapter or here walk back to the governance policy the mechanism is downstream of.

1. **Compilation target: WebAssembly.**
   *Satisfies: [LANG-03](../07-language-principles.md) (fast, portable feedback loop), [SEC-01](../08-security-principles.md) (sandboxable execution), [SEC-03](../08-security-principles.md) (reproducible codegen target), [INTEROP-02](../10-interoperability-principles.md) (portable across hosts), [PERF-01](../09-performance-principles.md) (characterisable cost model).*

2. **Boundary description language: WIT (WebAssembly Interface Types).**
   *Satisfies: [INTEROP-01](../10-interoperability-principles.md) (machine-checkable interface at every boundary), [INTEROP-03](../10-interoperability-principles.md) (versioned, mechanically comparable), [SEC-01](../08-security-principles.md) (capabilities visible in the interface), [SEC-10](../08-security-principles.md) (authority declared in function signatures).*

3. **Composition and host loading model: WebAssembly Component Model, LTS floor at level `0.3.0` (Preview 3 / Canonical ABI v2, WASI 0.3 ratified 2026-06-11).**
   *Satisfies: [INTEROP-02](../10-interoperability-principles.md) (hosts swappable by construction), [INTEROP-06](../10-interoperability-principles.md) (composition is a first-class operation), [SEC-01](../08-security-principles.md) (sandboxed component execution), [INTEROP-04](../10-interoperability-principles.md) (host-specific behaviour stays in the host).*
   *The floor is the level every conformant host MUST run. Preview 3 is chosen — not Preview 2 — because WASI 0.3 introduces native `async func`, `future<T>`, and `stream<T>` as canonical-ABI types. Adopting them at the floor lets the language, bridges, and hosts speak the same async vocabulary without a synthesized `poll`-loop convention that would later need to be unwound. Preview 2 and Preview 3 are not maintained in parallel; guests that need pre-0.3.0 features are out of scope for V2. Guests that need features above the floor opt in via `cln.toml`'s `[project].component_model` field; the check surfaces at build time, not at instantiation. Advancing the floor requires an ADR that supersedes this line — the mechanism is defined in [03 platform / 08 — Bridge Versioning §8.0](../../03%20platform/08-bridge-versioning.md).*

4. **Interface versioning: Semantic versioning on WIT interfaces.**
   *Satisfies: [INTEROP-03](../10-interoperability-principles.md) (explicit, mechanically-checkable evolution), [SEC-04](../08-security-principles.md) (versioned artifacts are checksum-verifiable against a lockfile).*

5. **Source-code indentation unit: Tabs.**
   *Satisfies: [LANG-05](../07-language-principles.md) (whitespace defines block structure), [LANG-01](../07-language-principles.md) (readable-first — one character per level, no misalignment at pixel scale), [LANG-04](../07-language-principles.md) (one obvious way — removes the tabs-vs-spaces debate).*

6. **Toolchain command surface: `cln` as the single command binary.**
   *Satisfies: [LANG-03](../07-language-principles.md) (single discoverable entry point), [SEC-05](../08-security-principles.md) (explicit update commands, no self-update background channel), [C-03](../05-concerns.md) (one command is the entire user surface).*

7. **On-disk artifact layout: `~/.cln/` (user-global) and `.cln/` (project-local); no other locations touched.**
   *Satisfies: [SEC-04](../08-security-principles.md) (bounded footprint enables lockfile verification), [C-14](../05-concerns.md) (bounded on-disk footprint), [LANG-03](../07-language-principles.md) (inspectable, back-up-able single location).*

8. **Reproducibility mechanism: Lockfile + checksum verification on every artifact load.**
   *Satisfies: [SEC-04](../08-security-principles.md) (every artifact checksum-verified against a lockfile), [SEC-03](../08-security-principles.md) (reproducibility as a security property), [PERF-08](../09-performance-principles.md) (measurement is meaningful only against reproducible builds), [C-04](../05-concerns.md) (byte-identical builds across machines).*

## Options considered

For each choice, the "chosen" option is what the current specification already implements. This ADR is documenting decisions that were made in practice; the "options considered" columns below record the alternatives that were rejected or would need to be reconsidered if the chosen option no longer served.

### 1. Compilation target

- **A — WebAssembly (chosen).** Portable, sandboxable, small runtime, mature toolchain, first-class Component Model support, runs in every relevant environment (server, browser, edge, embedded). The bytecode's cost model is characterisable, which is a prerequisite for [PERF-03](../09-performance-principles.md).
- **B — Native code (LLVM).** Faster peak performance; loses sandboxing, cross-host portability, and the ability to run in the browser without an extra transpilation step.
- **C — A custom bytecode with our own VM.** Full control; enormous ongoing maintenance cost, no ecosystem, violates [C-09](../05-concerns.md) (small maintainer surface).
- **D — Source-to-source to an existing high-level language.** Ties Clean's semantics to a moving target owned by someone else.

### 2. Boundary description language

- **A — WIT (WebAssembly Interface Types) (chosen).** The Component Model's native interface language. Machine-checkable, versioned, tool-supported, and the format every conforming host is already expected to consume. Verified conformance ([C-15](../05-concerns.md)) becomes mechanical instead of asserted.
- **B — A Clean-native interface language.** Better ergonomics for Clean authors; loses interoperability with everything else in the WebAssembly ecosystem and forces us to build our own tooling.
- **C — Prose contracts.** Rejected on sight: not machine-checkable, drifts silently.

### 3. Composition and host loading model

- **A — WebAssembly Component Model (chosen).** Declared composition operations, capability passing through explicit imports, host-swappability by construction ([C-16](../05-concerns.md)). The Component Model is the machinery that makes every INTEROP principle enforceable rather than aspirational.
- **B — Custom RPC between components in the same host.** Layered protocol on top of raw WASM modules; reinvents composition, loses standard tooling.
- **C — Monolithic components (no composition).** Simpler, but forecloses the library and framework model the ecosystem already commits to.

### 4. Interface versioning scheme

- **A — Semantic versioning on WIT interfaces (chosen).** Documented compatibility rules, predictable evolution, breaking changes visible in the version. Matches how the rest of the WebAssembly ecosystem versions.
- **B — Content-addressed interfaces (hash-per-version).** Perfect reproducibility, but human-hostile — versions have no readable order or intent.
- **C — Unversioned interfaces with a compatibility oracle.** Rejected: silent evolution is exactly the failure mode this choice exists to prevent.

### 5. Source-code indentation unit

- **A — Tabs (chosen).** One character per level, accessibility-neutral (screen readers and editors can render tabs at the user's preferred visual width), impossible to misalign at pixel level. Removes the "tabs vs spaces" argument by fiat.
- **B — Spaces (typically 4).** Familiar to more developers today; visually consistent across all viewers; loses user-configurable rendering.
- **C — Either, per file, with a marker line.** Complexity for no gain; violates [LANG-04](../07-language-principles.md).

### 6. Toolchain command surface

- **A — `cln` as the single command (chosen).** One binary, one namespace, verbs discoverable via `cln --help`. Matches [C-03](../05-concerns.md): the developer never invokes component binaries directly. `cln` dispatches to the compiler, framework, manager, and other components internally.
- **B — Separate binaries per component** (`cln-compile`, `cln-frame`, `cln-manager`). Component boundaries visible to the user, but the user doesn't care about our internal component boundaries; leaks implementation detail.
- **C — Framework-driven with the compiler invoked implicitly.** Similar to A in practice; A is a naming convention that makes the model explicit.

### 7. On-disk artifact layout

- **A — Everything under `~/.cln/` (user-global) and project-local `.cln/` (chosen).** Two directories, bounded footprint ([C-14](../05-concerns.md)), grep-able location. `~/.cln/` holds compiler versions, framework, libraries, plugins; project `.cln/` holds build outputs and the lockfile. Nothing else on the user's machine is modified.
- **B — Scattered across OS-standard locations** (`~/.local/share/`, `~/.cache/`, `/usr/local/`, per-OS). "Standard" for each OS but violates the single-location property; harder to inspect, back up, or nuke.
- **C — Project-only, no user-global.** Simpler footprint but forces every project to redownload the compiler and framework.

### 8. Reproducibility mechanism

- **A — Lockfile in the project + checksum verification on every load (chosen).** Every artifact under `.cln/` and every dependency resolved from `~/.cln/` is checksum-matched against the project's lockfile before use. Two developers with the same lockfile get byte-identical builds on the same platform ([C-04](../05-concerns.md)).
- **B — Content-addressed artifacts everywhere (no lockfile).** Reproducibility falls out for free but the developer never sees a human-readable version — everything is a hash. Debugging becomes harder.
- **C — Best-effort reproducibility.** Rejected: "usually reproducible" is not reproducible.

## Consequences

**What becomes easier:**

- The principles docs ([07](../07-language-principles.md), [08](../08-security-principles.md), [09](../09-performance-principles.md), [10](../10-interoperability-principles.md)) can be rewritten as pure policy: each principle states a durable rule and cites this ADR for the technology that currently satisfies it. When the day comes to reconsider a choice, the principles do not need rewriting — the ADR is superseded and the citations follow.
- New contributors have a single place to read "why this stack, and what would trigger revisiting?"
- Spec chapters carry the mechanics (flag names, file paths, version numbers, byte layouts) without repeating the "why."

**What becomes harder:**

- Changing any foundational choice now requires an ADR that explicitly supersedes the relevant subsection of this one. This is a feature, not a cost: it prevents the stack from drifting silently under the specification.

**Required follow-up spec edits (per [DOC-07](../00-documentation-principles.md#doc-07--the-ladder-of-intent)):**

- [03 platform / 00 — Overview](../../03%20platform/00-overview.md) — add a `Foundational stack` section linking to this ADR and enumerating each choice with a one-line pointer to the spec chapter that describes its mechanics.
- No other spec chapter changes required — the mechanics are already documented in their owning chapters; this ADR only extracts the *rationale*.

**Required follow-up principles edits:**

- [07 — Language Principles](../07-language-principles.md), [08 — Security Principles](../08-security-principles.md), [09 — Performance Principles](../09-performance-principles.md), and [10 — Interoperability Principles](../10-interoperability-principles.md) rewritten to state durable policy and cite this ADR where they currently name a technology. Tracked in the same change that promotes this ADR to Accepted.

---

## Metadata

- **Status:** Draft
- **Date:** 2026-08-02
- **Supersedes:** None
- **Spec impact:** [03 platform / 00 — Overview](../../03%20platform/00-overview.md), [03 platform / 02 — Host Bridge](../../03%20platform/02-host-bridge.md), [03 platform / 03 — Memory Model](../../03%20platform/03-memory-model.md), [03 platform / 08 — Bridge Versioning](../../03%20platform/08-bridge-versioning.md), [04 language / 03 — Lexical Structure](../../04%20language/03-lexical-structure.md), [04 language / 15 — Standard Library](../../04%20language/15-standard-library.md), [02 components / manager / 00 — Manager](../../02%20components/manager/00-manager.md)
