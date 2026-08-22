# ADR-0002 — Reference implementation stack for `clean-server`

The `clean-server` chapter specifies contracts a Clean HTTP host must satisfy — an async host-function surface, instance pooling, execution interruption — but says nothing about which WASM engine or HTTP framework the reference host uses. This ADR moves those concrete choices out of the spec and into an implementation-notes appendix, so the spec keeps only what binds an implementer and the chosen stack can be revised without a spec revision.

---

## Context

The `clean-server` chapter specifies the contracts a Clean HTTP host must satisfy: the interfaces of the `server` world of `clean:host`, an async host-function contract, instance pooling, execution interruption, composed bridge components, and statelessness.

Those contracts do not name a WASM engine, an async runtime, or an HTTP framework — and under [SDD-02](../03-spec-driven-design.md) they must not: a specification states what is observable from outside, never the mechanism, and a rule that only one implementation could satisfy is a design memo rather than a spec.

But the reference implementation does make those choices, and they are worth recording: a reader needs to know what the contracts were validated against, and an alternative implementer needs to know what is a contract and what is merely our choice.

## Decision

**Option C.** Reference-implementation details for `clean-server` — the WASM engine embedded, the language crates the host is built with, the standard-protocol adapters it ships — are recorded outside the specification tree, in [`02 components/hosts/clean-server/implementation-notes.md`](../../02%20components/hosts/clean-server/implementation-notes.md).

The specification chapter for a host describes contracts only (what a conformant host must satisfy). The implementation-notes appendix describes what one specific implementation happens to look like today. Changing what the reference implementation is built with is an edit to that appendix, not a spec revision.

**What lives in implementation-notes today:**

- The WASM engine the reference host embeds (and the engine features required to satisfy the async, pooling, and interruption contracts in the server spec).
- The Rust runtime and HTTP-surface crates the host is built with.
- The bridge components the reference distribution ships for standard protocols (databases, mail).

**What is deliberately NOT recorded — anywhere in Clean governance or specification:**

- **Vendor recommendations for deployment infrastructure** — session stores, job queues, real-time pub/sub, rate limiters. These are backplanes the *operator* runs, not components Clean owns. The clean-server spec defines the *interfaces* these backplanes must satisfy (`clean:host/session`, `clean:host/jobs`, `clean:host/ws`, `clean:host/sse`); any backplane implementing the interface works. Naming preferred vendors (Redis, NATS, etc.) would be an opinion Clean has no standing to hold, and previous drafts of this ADR did so in error. See implementation-notes §3 for the explicit non-scope.

None of the names in the implementation-notes appendix are normative. An implementation using a different WASM engine, async runtime, HTTP layer, or database bridge is fully conformant as long as it satisfies the contracts in the server spec.

## Options considered

**A — Keep the stack in the spec chapter.** Convenient for readers, but it plants mechanism inside a normative document. Over time readers treat "Wasmtime" as a requirement, and swapping the engine looks like a spec revision.

**B — Delete it.** Honours SDD-02 strictly, but throws away information that has real value: the contracts were designed against a specific engine's capabilities, and that provenance matters when judging whether a different engine can satisfy them.

**C — Record it as a decision (chosen).** The spec keeps contracts only; the stack lives in an ADR that the spec cites. Mechanism is documented, dated, and revisable without touching the specification.

## Consequences

**Easier.** Changing an engine or an HTTP layer is a new ADR, not a spec revision. A reader can tell at a glance which statements bind an implementer and which merely describe ours.

**Harder.** Two documents to keep coherent: if a contract in the spec was only satisfiable because of a specific engine capability, that dependency is now implicit and must be stated as a contract rather than left to the stack.

**Now required.** The server chapter's §1.11 is reduced to a pointer at [implementation-notes](../../02%20components/hosts/clean-server/implementation-notes.md), and its §1.4c bridge-composition topology — also mechanism — is reviewed on the same terms ([DOC-07](../00-documentation-principles.md#doc-07--the-ladder-of-intent)). Any vendor names in the server chapter's tables of "example production backplanes" (Redis, NATS, etc.) are pruned in the same sweep — Clean's spec describes the interfaces, not the vendors.

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Date:** 2026-08-01
- **Supersedes:** None
- **Spec impact:** [02 components / hosts / clean-server / implementation-notes](../../02%20components/hosts/clean-server/implementation-notes.md) (informative appendix carrying the concrete tech list); [02 components / hosts / clean-server / 01-server §1.11](../../02%20components/hosts/clean-server/01-server.md) (the spec keeps only the contracts)
