# ADR-0025 — WAC as the composition transport, with `[http-chain]` TOML as declarative front-end

**Status:** Draft

The framework and `clean-host-core` currently compose component graphs in hand-rolled Rust that walks the `[http-chain]` TOML block and re-implements composition-time verification the Component Model already knows how to do. This ADR adopts WAC (the Bytecode Alliance's Wasm Composition tool) as the internal transport — the TOML surface stays for the linear-chain 90% case and generates a WAC script under the hood, and a `wac-script` escape hatch lets operators drop into WAC directly when they need non-linear graph shapes the TOML front-end cannot express.

---

## Context

Under [ADR-0022 §3](./0022-foundational-technology-stack.md), the Component Model is the composition and host-loading model, and [Platform 18](../../03%20platform/18-component-composition.md) specifies how bridges and HTTP middleware are wired together. Today the framework and `clean-host-core` implement composition in Rust: they read `[http-chain]` from `host.toml`, walk the middleware array, and call the underlying composition primitives directly. The verification rules **[COMP-09..COMP-12]** are hand-implemented against the composed graph.

Two things about the current state deserve reconsideration now that we have shipped rules that depend on composition being a first-class operation ([INTEROP-06](../10-interoperability-principles.md#interop-06--composition-is-a-first-class-operation)):

1. **The `[http-chain]` block is a hand-rolled composition graph in TOML.** It expresses a strictly linear chain (§4.1 example). Non-linear graphs — a tracing middleware wrapping two parallel branches, a shared logging sink observing both an auth path and a public path — are not expressible without extending the TOML schema. Extending the TOML schema every time the graph shape changes is spec churn that never ends.

2. **Composition-time verification is duplicated.** COMP-09..COMP-12 in [Platform 18 §4.2](../../03%20platform/18-component-composition.md#42-load-time-verification) restate what Component Model composition tooling already verifies. Every check we write in framework Rust is another place where our verification can drift from the Component Model's actual composition semantics.

The Bytecode Alliance's **WAC** ("Wasm Composition") tool is the ecosystem's declarative composition language. It compiles a small text format (`.wac`) into a composed `.wasm`, resolves imports and exports across component graphs, and enforces the Component Model's linkability rules upstream. It is what `wasm-tools compose` grew into. Language-server support (VS Code, IntelliJ WIT plugins) is already available.

The question this ADR answers: what is the mechanism `clean-host-core` uses to compose the graph, and what surface does the operator use to describe it?

## Decision

Adopt WAC as the **composition transport** inside `clean-host-core` and `clean-framework`. Keep `[http-chain]` in `host.toml` as the **declarative front-end** for the common linear case, and add an escape hatch that lets operators supply a `.wac` script directly when the graph is not linear.

Concretely, two steps:

1. **Step 1 — WAC as internal transport.** `clean-host-core`'s composer step ([clean-host-core §5.3 / CLNH-25](../../02%20components/hosts/clean-host-core/01-specification.md)) invokes the WAC library (or `wac compose` CLI in reference implementations) to produce the composed component. When the operator provides only `[http-chain]`, the framework generates a WAC script from the TOML and passes it to WAC. Users see zero surface change; the composition mechanism becomes WAC.

2. **Step 2 — `[http-chain] wac-script` escape hatch.** When the operator needs a graph shape the TOML surface cannot express, they set `[http-chain] wac-script = "path/to/chain.wac"` in `host.toml`. When present, this overrides `middleware = [...]` and `guest = { ... }` in the same block. The framework passes the operator's script to WAC directly, without generating one. This is the escape hatch for non-linear graphs, shared sinks, and any composition pattern that outgrows the TOML front-end.

The composed artifact is byte-identical between the two paths for any graph the TOML can express — Step 1 is a pure re-plumbing of the mechanism, and the escape hatch is opt-in for cases the TOML front-end cannot reach.

WAC's version is pinned in the reference stack ([ADR-0002](./0002-clean-server-reference-stack.md), [ADR-0006](./0006-compiler-reference-stack.md)) and treated as any other bridge dependency: reproducibility ([ADR-0022 §8](./0022-foundational-technology-stack.md)) requires the WAC version to be recorded in the build manifest and verified against the lockfile before use.

## Options considered

- **A — Status quo: composition stays hand-rolled Rust in `clean-host-core` / `clean-framework`.** Fewest dependencies, complete control over verification error text. Costs: the framework must keep its own composition and verification code in step with the Component Model as the Model evolves; non-linear graphs remain unexpressible without spec churn every time; framework composition errors are Rust prose, not Component-Model-native errors.

- **B — Replace `[http-chain]` TOML with `.wac` files as the only surface.** Cleanest end state: one surface, one mechanism. Costs: every operator wanting a three-middleware chain must learn WAC syntax; the discoverability of the TOML front-end is lost; the incremental adoption path is missing.

- **C — WAC as transport, TOML as declarative front-end with an escape hatch (chosen).** Preserves the current TOML surface for the 90% case (linear middleware chain), delegates the actual composition to WAC so we inherit its verification and error surface, and lets power users drop into WAC when they need graph shapes TOML cannot express. Cost: two surfaces to document during the transition.

- **D — Composition via a Clean-native language on top of the compiler.** Would eventually consume WAC's role. Costs: substantially larger scope; requires Clean-language semantics for a graph description that would compete with WIT/WAC. Rejected as premature — the ecosystem tool exists and works.

## Consequences

**What becomes easier:**

- **Framework code shrinks.** The COMP-09..COMP-12 checks move from framework Rust to WAC. The framework's verification code becomes "call WAC; surface its diagnostics." Fewer lines to maintain, fewer places for Clean's verification to drift from Component Model semantics.
- **Composition errors improve.** Users see the Bytecode Alliance's composition diagnostics (typed import/export mismatch with actual WIT signatures printed) instead of framework Rust prose.
- **Non-linear graphs become expressible** without extending the TOML schema. Power users use the `wac-script` escape hatch.
- **The build cache decomposes cleanly.** [Platform 14 §14.15](../../03%20platform/14-compiler-architecture.md#1415-external-build-cache) hashes the whole request. With WAC as an explicit phase, the cache can distinguish "guest changed" from "middleware changed" from "composition script changed," turning some full-build cache misses into partial-build cache hits. The build-cache key structure ([§14.15.1](../../03%20platform/14-compiler-architecture.md#14151-key-structure)) does not change in this ADR — the key still hashes the whole request. Any decomposition is a later refinement to §14.15 and out of scope here.
- **Per-middleware hot-swap simplifies.** clean-server's SRVH-03..SRVH-08 ([§1.10.1 / §1.10.2](../../02%20components/hosts/clean-server/01-server.md)) currently owns bespoke graph-surgery logic. Under WAC, the swap becomes a `wac compose --replace <old>=<new>` invocation. clean-server keeps ownership of the wire protocol and policy (dev-mode-only, ack semantics, auth); the graph mutation is delegated to WAC. This ADR does not modify SRVH-03..SRVH-08; it makes the future simplification possible.

**What becomes harder:**

- **A new pinned dependency.** WAC (or the equivalent library API from `wasm-tools`) becomes part of the framework's reference stack. Its version must be recorded in the build manifest and verified against the lockfile. Ecosystem cadence risk: WAC's syntax is younger than WIT's and may evolve; pinning an exact version, treating breaking upgrades as ADR-worthy, mitigates this.
- **Two surfaces to document.** During the transition, both `[http-chain] middleware = [...]` and `[http-chain] wac-script = "..."` are documented. The TOML surface stays the primary; the escape hatch is documented with a "when the TOML front-end is not enough" framing. Whether the TOML surface eventually retires is a future decision, out of scope here.

**Required follow-up spec edits (per [DOC-07](../00-documentation-principles.md#doc-07--the-ladder-of-intent)):**

- [Platform 18 §4](../../03%20platform/18-component-composition.md#4-composition-mechanics) — new sub-section §4.4 "Composition transport" naming WAC and specifying the two-path behavior (TOML front-end generates a WAC script; `wac-script` overrides). COMP-09..COMP-12 stay as normative requirements but their check text is amended to state that WAC (or an equivalent Component Model composer) satisfies them.
- [clean-host-core §5.3 / CLNH-25](../../02%20components/hosts/clean-host-core/01-specification.md) — the composer step names WAC as the mechanism. The `Host::compose()` behavior is unchanged; the *how* becomes WAC.
- [clean-server §1.10](../../02%20components/hosts/clean-server/01-server.md) — a note in §1.10.1 that SRVH-03 (single-middleware swap targeting) is implemented via WAC's replace primitive under the hood. SRVH-03..SRVH-08 rule text is unchanged.
- Reference stack ADRs ([ADR-0002](./0002-clean-server-reference-stack.md), [ADR-0006](./0006-compiler-reference-stack.md)) — add `wac` as a pinned dependency with its version.

**Reproducibility:**

The composed artifact's byte-identity property from [CMP-02](../../03%20platform/14-compiler-architecture.md#cmp-02--same-request-in-byte-identical-outputs-out) extends to composition: given the same guest, the same bridge components, the same composition script (whether TOML-generated or user-supplied `wac-script`), and the same pinned WAC version, WAC MUST produce a byte-identical composed component. Framework verifies this property in the same determinism suite that guards CMP-02.

---

## Metadata

- **Status:** Draft
- **Date:** 2026-08-05
- **Supersedes:** None
- **Spec impact:** [03 platform / 18 — Component Composition](../../03%20platform/18-component-composition.md), [02 components / hosts / clean-host-core §5](../../02%20components/hosts/clean-host-core/01-specification.md), [02 components / hosts / clean-server §1.10](../../02%20components/hosts/clean-server/01-server.md), [03 platform / 14 — Compiler Architecture §14.15](../../03%20platform/14-compiler-architecture.md)
