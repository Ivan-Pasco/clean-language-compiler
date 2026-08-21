# Platform 05. Memory Policy

Where [§03 Memory Model](./03-memory-model.md) defines *what* memory looks like, this chapter defines *how much* memory a program is allowed to use, *when* it may grow, and *when* it is reclaimed. It is the operational policy layer. Clean runs across radically different hosts — an embedded IoT board with a few MB of RAM, a browser tab that must not evict the user's other work, a server API where each request must not affect other tenants, a canvas game that allocates every frame. One universal budget would either starve the server or crash the IoT, so Clean defines **memory tiers**: named budget profiles a program picks per deployment target.

---

## 5.1 Memory Tiers


A **tier** is a triple `(initial bytes, maximum bytes, growth factor)` plus a reset policy.

### TIER-01 — Tiers and their budgets


Every Clean program MUST target exactly one tier per deployment. The tiers and their budgets are:

| Tier | Initial | Maximum | Reset policy | Default target |
|------|---------|---------|--------------|----------------|
| `embedded` | 256 KiB | 1 MiB | none | IoT, microcontrollers |
| `minimal` | 512 KiB | 8 MiB | none | CLI, batch scripts |
| `standard` | 2 MiB | 32 MiB | per-request | Server APIs, web/PWA/mobile |
| `heavy` | 4 MiB | 64 MiB | per-request | SSR, desktop |
| `canvas` | 16 MiB | 64 MiB | per-frame | Games, real-time canvas apps |

- `initial` is the memory reserved and available before the program's first `memory.grow`.
- `maximum` is the ceiling. `memory.grow` beyond it MUST trap with `MEM001` (TierExceeded) — see [TIER-03](#53-enforcement).
- `growth factor` is the strategy for satisfying an `alloc` that would exceed the current commit — see [TIER-02](#52-growth-strategy).
- `reset policy` is when the runtime automatically resets the bump allocator's high-water mark back to `HEAP_START` — see [TIER-04](#54-reset-policies).

The tier is selected in `clean.toml`:

```toml
[memory]
tier = "standard"
```

If `[memory].tier` is absent, the compiler MUST infer the default from the target: `wasm32-server` → `standard`, `wasm32-browser` → `standard`, `wasm32-cli` → `minimal`. (`wasm32-embedded` → `embedded` is reserved — the embedded world is not shipped in V2; see [§07.4](./07-build-config.md#74-build-targets).) The `canvas` tier is never inferred: it MUST be selected explicitly (`tier = "canvas"`), because it changes the reset frequency dramatically. See [§07 Build Configuration](./07-build-config.md).

Libraries MAY declare a *minimum* tier in their `library.toml`. If the project targets a lower tier than a dependency requires, the build is rejected with [`CFG002`](./09-error-codes.md#316-configuration-codes-cfg) (`ManifestConstraintViolation`) — an error, not a warning.

**Coherence with MMD-01.** A shippable tier's `maximum` MUST exceed [`HEAP_START`](./03-memory-model.md#mmd-01--layout-and-guest-visible-constants) (1 MiB): the heap available to a program is `maximum − HEAP_START`, so a ceiling that does not clear the fixed heap base can satisfy no allocation at all. The `embedded` row as tabled violates this — its 1 MiB maximum equals `HEAP_START` exactly, leaving a zero-byte heap, and its 256 KiB `initial` does not even reach the heap base. No conforming deployment exists to break (the tier is reserved; the embedded world is not shipped in V2, [§07.4](./07-build-config.md#74-build-targets)), so the row's budgets are **provisional**: they MUST be re-derived jointly with MMD-01's constants — either a larger ceiling or a smaller, tier-specific `HEAP_START` — before the embedded world ships.

---

## 5.2 Growth Strategy


### TIER-02 — Amortized growth, bounded by the tier


When an allocation request would exceed the current committed size, the runtime MUST call `memory.grow(pages)` where:

```
current_bytes    = current committed memory
needed_bytes     = current_bytes + request_size
target_bytes     = max(needed_bytes, current_bytes * 3 / 2)     // 1.5× amortized
target_bytes     = max(target_bytes, current_bytes + 4 * 65536)  // 4-page floor
target_bytes     = min(target_bytes, tier.max_bytes)             // Never exceed tier
new_pages_needed = ceil((target_bytes - current_bytes) / 65536)
```

**Rationale for 1.5×:** exact-fit growth causes O(N²) copy cost in loops that append. Doubling wastes half the growth on programs with steady-state working sets. 1.5× is a well-established amortized compromise (used by Rust `Vec`, Go `append`, C++ `std::vector` in most implementations).

**Rationale for the 4-page floor:** without a floor, tiny allocations would issue single-page grows each time, thrashing the host's `mmap`. A 256 KiB floor amortizes system-call cost.

**Rationale for the tier cap:** `target_bytes` is clipped so the runtime never speculatively over-allocates past the tier. A tier-cap breach is a hard trap, not a warning.

---

## 5.3 Enforcement


### TIER-03 — The host enforces the tier; exceeding it traps as `MEM001`


Tier limits are enforced by the host, not by guest bookkeeping. The observable contract (see also the host-backing contract in [§03.5](./03-memory-model.md#35-host-backing--observable-contract)):

- Every guest instance's memory MUST be capped at `tier.max_bytes`; table elements and instance counts are bounded per tier the same way.
- A `memory.grow` beyond the tier limit MUST trap at the offending call site. **Trapping is not optional.** A grow failure surfaced as a return value reaches guest code that may or may not check it; an unchecked failure propagates as a garbage address and produces indirect crashes hours later. Trapping immediately makes the failure visible where it happened.

The reference enforcement mechanism (wasmtime `StoreLimits` with trap-on-grow-failure) is recorded in [ADR-0007 — Host Memory Backing](../01%20governance/decisions/0007-host-memory-backing.md).

The trap MUST be reported through the [error reporting pipeline](./06-error-reporting.md) with:
- `error_code: "MEM001"` (TierExceeded)
- `error_message: "Memory grow to N bytes exceeded tier limit M bytes"`
- `component: <compiler|library|application>` (attributed by span)
- `severity: crash`

---

## 5.4 Reset Policies


### TIER-04 — Reset fires at exactly the boundary the policy names


The `reset policy` on a tier tells the host when to reset `__heap_ptr` back to `__heap_start`, invalidating everything allocated in between. The host MUST fire the reset at exactly the boundary the policy names, and at no other point:

| Policy | When it fires | Applies to |
|--------|--------------|------------|
| `none` | Never (memory grows monotonically until instance drop) | `embedded`, `minimal` |
| `per-request` | After each HTTP request handler returns and the response is fully flushed | `standard`, `heavy` on the `server` world |
| `per-frame` | After each canvas frame commits | `canvas` |
| `per-task` | After each async task completes | `standard`/`heavy` on background job workers |

Reset is a runtime action, not a language action. From the guest's perspective, values allocated during the previous request or frame simply cease to exist — dereferencing a pointer that survived a reset traps.

### TIER-05 — Arena escape is warned as `MEM002`


The compiler MUST emit a warning (`MEM002`, ArenaEscape) when it detects a value allocated in a request-scoped arena being stored in a persistent structure (e.g. a `static var`). This is the class of bug that manifests as "works for the first request, mysteriously broken for the second."

---

## 5.5 Observability


The metrics defined in [§03.9](./03-memory-model.md#39-debugging-and-observability) are exposed by every host. In addition, this policy defines:

| Metric | Type | Meaning |
|--------|------|---------|
| `clean_wasm_tier` | label on all above metrics | Which tier the instance is running under |
| `clean_wasm_resets_total` | counter | Number of arena resets performed |
| `clean_wasm_reset_wasted_bytes` | histogram | Bytes reclaimed by each reset (helps size the initial commit) |
| `clean_wasm_grow_rejected_total` | counter | Number of tier-exceeded traps |
| `clean_wasm_time_in_grow_ms_total` | counter | Cumulative time spent inside `memory.grow` |

Alerting thresholds (server tier defaults — operational guidance for deployments, not part of the host contract):
- `clean_wasm_grow_rejected_total > 0` → page ops (a request was killed).
- `clean_wasm_memory_peak_bytes / tier_max > 0.9 for 5m` → warn (approaching cap).
- `clean_wasm_time_in_grow_ms_total rate > 100 ms/s` → warn (thrashing).

---

## 5.6 Configuration via `clean.toml`

The `[memory]` schema is owned by [§07 Build Configuration §7.3](./07-build-config.md#73-memory--full-schema); this section does not restate it. The policy-relevant facts:

- `tier` is required or inferred from the target (§5.1).
- Every override (`initial-pages`, `maximum-pages`, `growth-factor`, `reset-policy`) is bounded by the tier — an override can never exceed the tier's declared maximum.
- `[memory.arena] transient-scope` enables the per-iteration arena in `iterate` bodies (§5.4); `transient-scope-warn-bytes` sets the per-iteration allocation threshold above which the compiler warns.

---

## 5.7 Non-Goals

- **Per-allocation quotas.** Clean does not attempt to charge individual allocations to individual callers. The tier is a global instance budget.
- **Priority-based reclamation.** No "kill low-priority arena first" logic; reclamation is scope-driven and deterministic.
- **Cross-instance memory sharing.** Not supported; see [§03.10](./03-memory-model.md#310-non-goals).
- **Runtime tier switching.** A program's tier is fixed at build time. Switching tiers means recompiling.

---

## 5.8 Deferred Refinements

1. **Adaptive growth factor.** The growth factor is fixed (1.5×) regardless of instance lifetime. V2 does not shift long-lived instances to a footprint-optimized factor after warm-up; production measurements will drive any future change.
2. **Reset diagnostics.** The runtime emits a structured event on every reset at debug log level. Aggregate reset telemetry is the metrics defined in §5.5 (`clean_wasm_resets_total`, `clean_wasm_reset_wasted_bytes`); no further per-reset detail is exported.

---

## Changelog

- 2026-08-19 — Erratum from the compiler's Milestone 6 (`clean-language-compiler/docs/DISCOVERIES-M6.md`, item 3): §5.1 gains the MMD-01 coherence rule — a shippable tier's `maximum` MUST exceed `HEAP_START`, since the usable heap is `maximum − HEAP_START`. The `embedded` row's two Accepted numbers (1 MiB ceiling vs the fixed 1 MiB heap base) are jointly unusable: zero-byte heap, every allocation trips `MEM001`. The row stays in the table but its budgets are marked provisional, to be re-derived with MMD-01's constants before the embedded world ships; no shipped configuration is affected.
- 2026-08-01 — Technical-debt closure pass (lote X, approved 2026-08-01): §5.1 tier-below-library-minimum rejection assigned its code — [`CFG002`](./09-error-codes.md#316-configuration-codes-cfg) `ManifestConstraintViolation` (Error), closing the "(diagnostic code: pending — CFG range)" marker; resolves the 05-vs-07 boundary conflict in this section's favor (the case is an error; [07 §7.10](./07-build-config.md#710-validation) corrected, [10 §15](./10-semantic-rules.md) boundary note updated).
- 2026-08-01 — Governance compliance (traceability pass): registered rule prefix `TIER-` and minted TIER-01 (tiers and budgets, C-16), TIER-02 (growth strategy, C-10), TIER-03 (host-enforced trap on tier-exceeded grow → `MEM001`, C-02/C-08), TIER-04 (reset boundaries, C-05), TIER-05 (arena-escape warning `MEM002`, C-02) — all reusing the existing normative text. Sections §5.1–§5.4 marked *Normative*; §5.5 marked *Informative*, with the alerting thresholds explicitly labeled operational guidance rather than host contract. Tier inference and the explicit-`canvas` requirement written as checkable MUSTs.
- 2026-08-01 — Conflict-log remediation (Fase 3): diagnostic codes converted to the `PREFIX###` format of 09 §1 (resolution 0.4; formal registration is Fase 4) — `MEM-TIER-EXCEEDED` and `resource-exhausted:memory` unified as `MEM001` (one event, one code), `WARN-M001` → `MEM002`. §5.3 rewritten as the observable enforcement contract (trap on tier-exceeded grow remains mandatory), with the wasmtime `StoreLimits` mechanism extracted to [ADR-0007](../01%20governance/decisions/0007-host-memory-backing.md). §5.8.2 harmonized with §5.5 per P13c — the reset metrics exist and win. Tier inference for `wasm32-embedded` marked reserved (P16.1); the libraries-minimum-tier rejection marked "(diagnostic code: pending — CFG range)". §5.6 reduced to a citation of 07 §7.3 (the schema home), keeping only policy-specific facts; world name normalized per 15 §0.3.

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Program authors picking a deployment tier; host implementors enforcing memory budgets
- **Rule prefix:** `TIER-`
- **Part of:** [Clean Language Specification — Platform](./README.md)
- **References:** [Memory Model](./03-memory-model.md), [Build Configuration](./07-build-config.md), [ADR-0007 — Host Memory Backing](../01%20governance/decisions/0007-host-memory-backing.md)
- **Satisfies:** LANG-03, PERF-06, SEC-08
