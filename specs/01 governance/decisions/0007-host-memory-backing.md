# ADR-0007 — Host memory backing: reference wasmtime configuration

The memory model chapters had embedded snippets of `wasmtime::Config` and `PoolingAllocationConfig` and norming statements against them — mechanism only one embedder can satisfy literally, planted inside documents meant to state observable contracts. This ADR pulls the reference wasmtime configuration out of the spec into a decision record, leaving the chapters to state what a guest observes (stable addresses, tier-limit traps, interruption) and letting alternative embedders satisfy those contracts by other means.

---

## Context

The memory model chapter defines the guest-visible contract: linear-memory layout, allocator behavior, string and collection representations, and what a program observes when it exceeds its tier (a trap, per [05 §5.3](../../03%20platform/05-memory-policy.md#53-enforcement)). But [03 §3.5](../../03%20platform/03-memory-model.md#35-host-backing-and-wasmtime-configuration) and [05 §5.3](../../03%20platform/05-memory-policy.md#53-enforcement) went further, embedding Rust snippets of `wasmtime::Config`, `PoolingAllocationConfig`, and `StoreLimitsBuilder` — with 03 §3.6 norming *against the mechanism* ("configure them consistently with §3.5") and 05 declaring `trap_on_grow_failure(true)` "not optional," a sentence only one embedder can satisfy literally.

Under [SDD-02](../03-spec-driven-design.md), engine configuration is mechanism. What binds an implementer is the observable behavior: stable memory addresses for the store's lifetime, tier-limit enforcement, an immediate trap on failed growth, and interruption of runaway guests. How wasmtime is configured to deliver that is our choice, worth recording — the contracts were designed against these exact wasmtime capabilities — but as a decision, not as spec text. This mirrors [ADR-0002](0002-clean-server-reference-stack.md) and [ADR-0006](0006-compiler-reference-stack.md).

## Options considered

**A — Keep the configuration in the spec chapters.** Readers treat `signals_based_traps(true)` as a conformance requirement and an alternative embedder appears non-conformant by construction. Rejected.

**B — Delete it.** Loses the provenance that explains *why* the contracts are satisfiable (e.g. bounds-check elision is what makes the 4 GiB reservation worth mandating nothing about). Rejected.

**C — Record it as a decision; the chapters keep only observable contracts.** Chosen.

## Decision

**Option C.** The reference hosts back guest memory with **wasmtime**, configured as follows:

- **Reservation and guards (64-bit hosts):** `memory_reservation(4 GiB)` virtual per instance, `memory_guard_size(32 MiB)`, `signals_based_traps(true)`, and `memory_may_move(false)`. The full reservation plus guard lets wasmtime elide explicit bounds checks (a 1.2×–1.8× win on hot paths); static addresses uphold the bump allocator's stable-address invariant.
- **Pooling allocator:** `PoolingAllocationConfig` with `max_memory_size` set to the tier limit, `total_memories` sized to the worker pool, and `linear_memory_keep_resident(2 MiB)` to amortize instantiation across requests.
- **StoreLimits:** every guest store gets `StoreLimitsBuilder` with `memory_size(tier.max_bytes())`, `trap_on_grow_failure(true)`, `table_elements(tier.max_table_elements())`, and `instances(1)`. `trap_on_grow_failure` is how the reference stack delivers the observable contract that a failed grow traps at the offending call site instead of leaking `-1` into guest arithmetic.
- **Interruption: epoch over fuel.** Request-scoped guests are bound to an epoch counter advanced on a timer (server default 5 s, canvas 100 ms). Epoch interruption is chosen because its steady-state overhead is negligible; fuel is reserved for the compile-time sandbox ([ADR-0004](0004-block-handler-execution-model.md)), where determinism outweighs throughput.

None of these names or calls is normative. The **observable contract stays in the spec chapters**: the memory layout and constants, tier limits and the trap on exceeding them (`MEM-TIER-EXCEEDED` reporting per [05 §5.3](../../03%20platform/05-memory-policy.md#53-enforcement)), stable addresses, memory64 acceptance, and interruption behavior. An embedder that produces equivalent guest-observable behavior by other means is conformant.

## Consequences

**Easier.** Engine upgrades and configuration tuning become new ADRs; the memory chapters read as pure contract; alternative embedders (browser hosts have no wasmtime at all) are no longer nominally non-conformant.

**Harder.** The chapters must state every behavior the configuration was silently guaranteeing — e.g. "addresses are stable for the store's lifetime" must live in 03 as a contract now that `memory_may_move(false)` moves here.

**Now required (DOC-07).** 03 §3.5 is reduced to the observable backing requirements plus a pointer here; 03 §3.6 norms against those observable requirements rather than "§3.5's configuration"; 05 §5.3 keeps the trap contract and its error reporting, and cites this ADR for the enforcement mechanism.

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Date:** 2026-08-01
- **Supersedes:** None
- **Spec impact:** [03 — Memory Model §3.5–3.6](../../03%20platform/03-memory-model.md#35-host-backing-and-wasmtime-configuration), [05 — Memory Policy §5.3](../../03%20platform/05-memory-policy.md#53-enforcement) (mechanism moves here; both chapters keep the observable contracts)
