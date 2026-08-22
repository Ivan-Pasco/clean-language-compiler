# ADR-0026 — Pin WAC in the compiler and clean-server reference stacks

**Status:** Draft

[ADR-0025](./0025-wac-as-composition-transport.md) adopts WAC as the composition transport but leaves its version-pinning as follow-up, and the two ADRs that record the reference stacks it needs to land in ([ADR-0002](./0002-clean-server-reference-stack.md), [ADR-0006](./0006-compiler-reference-stack.md)) are already Accepted and cannot be edited. This ADR is the clean way to reconcile that: an extension ADR that adds `wac-graph` and `wac-parser` at `^0.7` to both reference stacks without superseding either parent decision — the same pattern the tree already uses when a new dependency is genuinely additive rather than a re-decision.

---

## Context

[ADR-0025 §Consequences](./0025-wac-as-composition-transport.md) requires the compiler and clean-server reference stacks to pin **WAC** (the Bytecode Alliance's WebAssembly Composition tool) as a first-class dependency, so that:

- Reproducibility ([ADR-0022 §8](./0022-foundational-technology-stack.md)) applies to composition as it does to compilation — a WAC version bump becomes a lockfile event, not a silent behavior change.
- The framework's composer step ([clean-host-core §5.3 / CLNH-25](../../02%20components/hosts/clean-host-core/01-specification.md)) can name a specific WAC library API version instead of tracking upstream heuristically.
- The build cache key ([Platform 14 §14.15.1](../../03%20platform/14-compiler-architecture.md#14151-key-structure)) can incorporate the WAC version alongside the compiler version.

ADR-0006 (compiler reference stack) and ADR-0002 (clean-server reference stack) are both **Accepted**. Per [DOC-07](../00-documentation-principles.md#doc-07--the-ladder-of-intent), an Accepted ADR MUST NOT be edited except to set its status to `Superseded by <link>`. Adding a single new dependency does not warrant superseding either ADR — the decisions those ADRs record (Option C in both cases: "record the stack in an ADR / implementation-notes appendix so it can evolve without spec revisions") remain correct and are the mechanism this amendment uses.

The clean way to reconcile these is a new ADR that adds the specific dependency, cites the parent ADRs it extends, and leaves the parent decisions intact. That is this ADR.

## Decision

Add **WAC** to the compiler and clean-server reference stacks, pinned at the version pin below. This is an *addition* to the dependency lists ADR-0006 and ADR-0002 already record; neither is superseded.

**Compiler side ([ADR-0006](./0006-compiler-reference-stack.md)):** add `wac-graph` and `wac-parser` at `^0.7` (or the latest compatible with `wasm-tools ^0.220`, which ADR-0006 already pins) to the "wasm-tools workspace" bullet. These are the crates exposing WAC's composition library API — the framework calls the library, not the `wac` CLI, so process-boundary and lockfile semantics stay clean.

**clean-server side ([ADR-0002](./0002-clean-server-reference-stack.md) → [implementation-notes.md](../../02%20components/hosts/clean-server/implementation-notes.md)):** add a `wac-graph` (and, if the reference host builds against WAC's parser rather than only its graph API, `wac-parser`) entry at the same `^0.7` pin, in the "bridge components the reference distribution ships" section (or the closest equivalent section for composition tooling). The exact section placement is an implementation-notes edit, not a spec edit.

Both pins are tracked as any other dependency: recorded in the build manifest ([Platform 14 §14.8](../../03%20platform/14-compiler-architecture.md#148-build-manifest)), verified against the lockfile before use per [ADR-0022 §8](./0022-foundational-technology-stack.md), and treated as a normal SemVer-major bump when the crate makes breaking changes.

The reference-implementation nature of these pins is preserved: **none of these names is normative** (per ADR-0006 §Decision and ADR-0002 §Decision). An alternative implementation using a different Component Model composer that satisfies [COMP-20..COMP-24](../../03%20platform/18-component-composition.md#44-composition-transport) is fully conformant.

## Options considered

- **A — Amend ADR-0002 and ADR-0006 in place.** Simplest change. Violates [DOC-07](../00-documentation-principles.md#doc-07--the-ladder-of-intent), which is a load-bearing rule for making decision history stable.

- **B — Supersede both ADRs with fresh ones that include WAC.** Clean at the mechanical level but heavy-handed: the underlying decisions of ADR-0002 (Option C — record stack in appendix) and ADR-0006 (Option C — three-crate workspace with the listed dependencies) are correct as written. Superseding them would either restate those decisions verbatim (adding no value) or omit them (losing the reasoning). Rejected as disproportionate.

- **C — New ADR that extends both (chosen).** Records the specific addition, cites the parents, extends them without replacing them. Matches how other extension-shaped ADRs in the tree work (e.g. [ADR-0024](./0024-sql-dialect-resolution-under-wit-bridges.md) reopens ADR-0003 with a targeted supersede; here the parent decisions are not being reopened, so a plain extension ADR is the closer analogue).

- **D — Skip the pin, let framework track WAC informally.** Sacrifices reproducibility ([ADR-0022 §8](./0022-foundational-technology-stack.md)); WAC would be the only unpinned link in a tree where every other artifact — compiler, bridges, plugins, wasm-tools — is lockfile-verified. Rejected: composition is on the critical path, and the byte-identity guarantee of [COMP-23](../../03%20platform/18-component-composition.md#44-composition-transport) depends on WAC's version being fixed.

## Consequences

**What becomes easier:**

- **The follow-up work in ADR-0025 §Consequences is closed.** ADR-0025 said "WAC's version is pinned in the reference stack ([ADR-0002], [ADR-0006]) and treated as any other bridge dependency." This ADR discharges that requirement without touching the parents.
- **Composition reproducibility is auditable.** Given the same guest, bridges, composition script, *and* pinned WAC version, [COMP-23](../../03%20platform/18-component-composition.md#44-composition-transport) guarantees byte-identical output. The pin is what makes "same pinned WAC version" a checkable condition rather than a hope.

**What becomes harder:**

- **A new dependency to track.** WAC's release cadence is younger than `wasm-tools`' own. Breaking changes in WAC's library API will surface as SemVer-major bumps requiring an updated ADR (or superseding this one), same as any other pinned dep.

**Required follow-up edits (per [DOC-07](../00-documentation-principles.md#doc-07--the-ladder-of-intent)):**

- `02 components/hosts/clean-server/implementation-notes.md` — add the WAC entry to the composition-tooling section (or nearest equivalent). Not a spec edit; an implementation-notes edit per ADR-0002 §Decision.
- The compiler reference implementation's `Cargo.toml` gains the `wac-graph` / `wac-parser` deps at the pinned version. Not a spec edit; a repo-side change owned by the compiler component, tracked here as the trigger.

**Not required:**

- No changes to Platform 18, clean-host-core, or clean-server spec text. The WAC-as-transport decision lives in ADR-0025; the WAC-version-pin decision lives here. Neither adds normative rules — both add mechanism.

---

## Metadata

- **Status:** Draft
- **Date:** 2026-08-05
- **Supersedes:** None
- **Spec impact:** [ADR-0006 — Compiler reference stack](./0006-compiler-reference-stack.md) (adds `wac` to the pinned dependency list, but does not supersede ADR-0006); [ADR-0002 — clean-server reference stack](./0002-clean-server-reference-stack.md) (its `implementation-notes.md` appendix gains a `wac` entry); [02 components / hosts / clean-server / implementation-notes](../../02%20components/hosts/clean-server/implementation-notes.md)
