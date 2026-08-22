# ADR-0029 — Cross-component deep-test tier for host and framework components

**Status:** Draft

The compiler and compiler-testing repos have a `Nightly Deep Tests` workflow: snapshots across opt levels, WASM determinism, mutation testing, full-corpus compile smoke. Server, framework, and manager have **only canaries** at the deep-test tier. Canaries prove "the shipped stack runs a corpus"; they do not stress-test the component under load, adversarial input, or config permutations — which is where mission-critical host bugs actually live. This ADR extends the deep-test tier to every Tier 1 and Tier 2 component (per [ADR-0027](./0027-coverage-policy.md)) with a per-component suite tuned to that component's failure modes.

---

## Context

The current asymmetry, from [`../../05 execution/automation/03-quality-gates.md`](../../05%20execution/automation/03-quality-gates.md):

| Component | Deep tier today |
|---|---|
| `clean-language-compiler` | 5 jobs — full suite, snapshots-across-opt-levels, WASM determinism, mutation testing, corpus compile smoke |
| `clean-language-compiler-testing` | Coverage + cln-corpus + fuzz |
| `clean-server` | Canaries only (currently manual per [ADR-0028](./0028-nightly-canary-release-asset-wait.md) until the fix lands) |
| `clean-framework` | Canaries only (same manual state) |
| `clean-manager` | Nothing beyond CI |
| `clean-runner` | Nothing beyond CI |

The gap is that canaries catch **integration drift** — "did the shipped binary still run the reference corpus?" — but not **stress-mode bugs**: bridge determinism under random call sequences, plugin load-order sensitivity, memory growth under sustained traffic, config-permutation boot failures. These are the bug classes that hit users in production and cannot be caught by unit tests (too narrow) or canaries (too shallow).

Compiler's deep tier evolved organically over time. Hosts and framework never got the same treatment because "canaries are enough" was the default assumption. Empirically it is not — the reporter-artifact issues on `clean-framework` and `clean-server` include multiple bugs that a deep-tier gate would have caught before the release.

## Decision

Adopt a **per-component deep-test tier** for every Tier 1 and Tier 2 component named in [ADR-0027](./0027-coverage-policy.md). Each component's deep suite runs nightly on `schedule` + `workflow_dispatch`, follows the baseline-gating philosophy (max-failures, not zero), and comprises the jobs listed below. Every job's failure opens a dashboard bug via `report_error` rather than blocking merges — the deep tier is a **detection** layer, not a **gating** layer, because merge-blocking a nightly would freeze the ecosystem on runner outages.

### Per-component job lists

Each component gains a new workflow: `.github/workflows/nightly-deep-tests.yml`. Job selection is driven by the component's failure modes, not by uniformity.

#### `clean-server` (Tier 1 bridge / Tier 2 non-bridge)

| Job | Purpose | Baseline gate |
|---|---|---|
| **Bridge fuzz** | Random-but-typed call sequences at every WIT-typed entry point; asserts no panic, no memory leak, no host trap | 300s per host binding × parallel matrix over WIT worlds; max 0 crashes |
| **Config-matrix boot** | Start server under every declared config permutation (TLS on/off × auth backend × storage backend × log level); hit `/healthz` and one bridge endpoint | Full matrix must return 200; failures reported with the failing permutation |
| **Long-run leak check** | 10 min sustained traffic against representative bridge endpoints; assert RSS growth < 50 MB, no file-descriptor leak | Max 50 MB RSS growth, max 10 open FDs beyond baseline |
| **JSON conformance stress** | JSONTestSuite × 100 randomized transformations per input, run through the JSON bridge | Same pass/fail semantics as CI conformance job |

#### `clean-framework` (Tier 1 core, Tier 3 non-core plugins)

| Job | Purpose | Baseline gate |
|---|---|---|
| **Plugin-load determinism** | Load all 10 `frame.*` plugins in every valid load order (or a Latin-square sample if all orders is intractable); snapshot resolved registry; assert byte-identical across orders | Zero divergence tolerated — divergence is a bug |
| **Cross-plugin contract fuzz** | For each cross-plugin call surface (e.g. `frame.auth` → `frame.data`, `frame.data` → `frame.storage`), feed random-but-typed inputs; assert no panic, no host trap | 300s per surface; max 0 crashes |
| **Runtime canary at every opt level** | Existing framework canary corpus × opt levels 0–3 | Baseline: same failure count at every opt level |
| **Plugin ABI compat** | Load current plugins against last N released compiler versions (N = 3); assert every plugin still loads | All plugins must load on all N versions |

#### `clean-manager` (Tier 2)

| Job | Purpose | Baseline gate |
|---|---|---|
| **Install / uninstall matrix** | `cln install`, `cln frame install`, `cln uninstall` across 4 target platforms × 3 recent versions × 2 install modes (fresh, upgrade); assert exit code 0 and expected filesystem state | All 24 permutations must succeed |
| **Corrupt-install recovery** | Simulate partial install (kill mid-download, checksum mismatch, permission errors); assert `cln` reports actionable error and repairs on retry | Every simulated corruption must produce a documented error class |
| **CLI parse fuzz** | Random argument sequences through the CLI parser; assert no panic, only well-formed errors | 300s; max 0 crashes |

#### `clean-runner` (Tier 2)

| Job | Purpose | Baseline gate |
|---|---|---|
| **Cross-platform WASM execution** | Run compiler's execution corpus on each of the 4 release targets; compare outputs byte-for-byte | Zero cross-platform divergence |
| **Runner leak check** | 1000 sequential WASM invocations in a single runner process; assert bounded memory growth | Max 20 MB RSS growth |

#### `clean-language-compiler` (Tier 1, already has deep tier)

No new jobs. Existing `Nightly Deep Tests` workflow is the reference. Verify its baselines are documented per ADR-0027 §baseline comments as part of the ADR-0027 rollout PR.

### Shared conventions across every deep tier

- **Trigger:** `schedule` (03:47 UTC to match compiler nightly cadence and stay clear of runner peak) + `workflow_dispatch`.
- **Runner:** `ubuntu-latest` unless the job specifically needs OS-matrix coverage.
- **Failure reporting:** every failed job calls `report_error` with a synthesized fingerprint keyed on `component + job + normalized-failure-message`. Fingerprints deduplicate across nights — the same failure does not spam the dashboard.
- **Baselines:** every baseline is committed in the workflow file with `# baseline: <value> measured YYYY-MM-DD`. Ratcheting rule from ADR-0027 §ratchet rule applies here too — baselines can only tighten.
- **Runtime budget:** each component's deep tier caps at 90 min total. Jobs that would push past that are split into a `nightly-deep-tests-heavy.yml` running weekly instead.
- **Artifact retention:** logs and reports retained 30 days, crash cases retained 90 days.

### Rollout order

1. `clean-server` — highest blast radius, most bridge surface, ships to production hosts.
2. `clean-framework` — plugin-load determinism catches the exact class of bug already seen in reporter-artifacts.
3. `clean-manager` — install/uninstall correctness is prerequisite to every user's experience.
4. `clean-runner` — cross-platform determinism is a small suite with high ROI.

Each component's rollout is one PR that adds the workflow + populates baselines from the first successful run.

## Options considered

- **A — Do nothing. Trust canaries (status quo).** Zero cost, keeps the existing gap. Rejected because the empirical evidence (reporter-artifact issues) shows canaries miss the bugs that matter most.

- **B — One shared "deep tests" workflow for all components.** Simpler to maintain, wrong shape: compiler mutation testing does not apply to server, bridge fuzz does not apply to compiler. Component failure modes differ; the suites must differ. Rejected.

- **C — Per-component deep tier tuned to failure modes (chosen).** Matches how compiler's nightly evolved. Each component owner picks jobs from a shared vocabulary (fuzz / matrix boot / leak / determinism / ABI compat). Scales without imposing irrelevant checks.

- **D — Merge-block on nightly deep failures.** Would guarantee no deep-tier regression ships. Rejected because runner flake, GitHub outages, or a single infra hiccup would freeze the ecosystem. Detection layer with `report_error` fan-out is the ecosystem-consistent choice.

## Consequences

**What becomes easier:**

- **Bug classes canaries cannot see get caught pre-release.** Plugin load-order determinism, config-permutation boot, sustained-traffic leaks, install-mode recovery — all become nightly signals.
- **`report_error` fan-out means the dashboard is the single triage surface.** Every nightly failure becomes a fingerprint; the Ready Queue absorbs them per the `/fix` skill's existing flow. No new triage process needed.
- **The compiler / hosts asymmetry closes.** Every Tier 1 and Tier 2 component gets the same class of protection.

**What becomes harder:**

- **Runner cost increases by ~30–90 min per component per night.** At current component count, ~5 hours of nightly runner time added. Paid on Anthropic-owned runners; not a blocker.
- **Each component needs a deep-suite author.** Bridge fuzz is not trivial to write. Owners are compiler and framework maintainers primarily; the reference implementation for bridge fuzz (compiler side) can be lifted for server with modifications.
- **Baselines will initially be noisy.** First 1–2 weeks of nightly runs may open dashboard bugs at high volume. The `report_error` deduplication (fingerprint = component + job + normalized-message) contains the noise; owners triage down to real bugs over the first sprint after each component's rollout.
- **The shared vocabulary (fuzz / matrix boot / leak / determinism / ABI compat) needs a reference document.** Follow-up work: add `foundation/05 execution/automation/06-deep-test-vocabulary.md` describing each job type with parameter guidance and reference implementations.

---

## Metadata

- **Status:** Draft
- **Date:** 2026-08-07
- **Supersedes:** None
- **Spec impact:** Every Tier 1 and Tier 2 component gains a `nightly-deep-tests.yml` workflow. Reference skeleton lives at [`../../scripts/reference-workflows/nightly-deep-tests.yml`](../../scripts/reference-workflows/nightly-deep-tests.yml). Follow-up: add `foundation/05 execution/automation/06-deep-test-vocabulary.md` documenting the shared job types.
