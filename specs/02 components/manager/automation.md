# clean-manager (`cln`) — Automation

This document is the CI, coverage, and nightly-test recipe for the Clean Manager component. It records the concrete values — tier assignment, coverage thresholds, nightly job selection, release-asset consumers, and OS matrix — that the automation ADRs leave as placeholders, so that Manager's CI workflows can be generated correctly from a single source. It is a reference sheet for whoever wires up or updates Manager's pipelines; the underlying policies live in the ADRs cited at the bottom.

---

## Tier assignment

`clean-manager` is **Tier 2** end-to-end per [ADR-0027](../../01%20governance/decisions/0027-coverage-policy.md). A bug here breaks install / uninstall / plugin management, which affects every user's ability to run anything — serious, but users can pin to a previous manager version to recover, so it does not meet the Tier 1 "silently corrupts every user program" bar.

The **critical paths within Tier 2** that mutation testing focuses on:

- Install and uninstall (any code path that mutates `~/.cln/`)
- WASM loading (path resolution, version resolution, checksum verification)
- Config file parsing and writing (`~/.cln/config.toml` and per-project overrides)

Coverage on other paths (CLI arg parsing, help text, output formatting) is still enforced at Tier 2 levels but is not mutation-tested.

## coverage.yml — placeholder values

```yaml
env:
  TIER: '2'
  TIER_TARGET_LINE: '70'
  TIER_TARGET_BRANCH: '60'
  BASELINE_LINE: '<measured on first run>'
  BASELINE_BRANCH: '<measured on first run>'
  BASELINE_MEASURED_ON: '<YYYY-MM-DD>'
  MCDC_MODULES: ''
```

Tier 2 does not require MC/DC. Mutation-score gate (50% on critical paths) is enforced by the nightly deep tier below, not by `coverage.yml`.

## nightly-deep-tests.yml — job selection

| Job | Enable? | Notes |
|---|---|---|
| `install-uninstall-matrix` | **yes** | Highest-ROI manager job. Matrix: OS × 3 recent compiler versions × 2 modes (fresh, upgrade) — 3 × 3 × 2 = 18 permutations. Each cell runs `cln install`, verifies filesystem state, runs `cln uninstall`, verifies cleanup. |
| `corrupt-install-recovery` | **yes** (add to skeleton — not in reference template) | Simulate three corruption modes: (a) kill install mid-download, (b) checksum mismatch, (c) permission-denied on target directory. Assert `cln` reports an actionable error class and repairs on retry. |
| `cli-parse-fuzz` | **yes** (add to skeleton) | Random argument sequences through the CLI parser. Assert no panic, only well-formed errors. 300s. |
| `bridge-fuzz` | no | Manager has no bridge surface. |
| `config-matrix-boot` | no | Manager doesn't boot as a server. |
| `leak-check` | no | Short-lived process; no leak surface. |
| `plugin-load-determinism` | no | Framework's job. |
| `plugin-abi-compat` | no | Framework's job. |
| `cross-platform-wasm-execution` | no | CLI's job. |

Additional manager-specific job for mutation testing (per Tier 2 mutation-score gate):

| Job | Purpose |
|---|---|
| `mutation-score-critical-paths` | Run `cargo-mutants` scoped to `src/install/`, `src/uninstall/`, `src/wasm_loader/`, `src/config/`. Fail if score < 50% for two consecutive nights. |

Total nightly wall-clock: ~80 min (install-uninstall-matrix is the long tail — 18 cells × ~4 min each). Fits budget.

## wait-for-release-assets — consumers

| Workflow | Waits on | `EXPECTED_TARGETS` |
|---|---|---|
| `nightly-deep-tests.yml` `install-uninstall-matrix` (per cell) | `clean-compiler` and `clean-framework` (both are installed by `cln`) | compiler: `linux-x64,macos-x64,macos-arm64,windows-x64` · framework: `frame-plugins.tar.gz,version-manifest.json` |

Manager itself is a **producer** of release artifacts (`cln` binary), not a consumer. Its own release workflow does not need wait-for-assets — it emits assets, does not install them.

## Release matrix (informative)

Matches compiler's 4-target matrix:

- `linux-x64`
- `macos-x64`
- `macos-arm64`
- `windows-x64`

`EXPECTED_TARGETS` for downstream consumers of `cln`: `linux-x64,macos-x64,macos-arm64,windows-x64`.

## CI-specific note — cross-OS matrix

Per [`../../05 execution/automation/01-ci-per-component.md`](../../05%20execution/automation/01-ci-per-component.md), manager's CI runs the full test suite on all three OS matrix cells (`ubuntu-latest` / `macos-latest` / `windows-latest`), not just build. This is unusual (most components build-check across OS but test only on ubuntu) — install and uninstall behavior is deeply OS-dependent, so the full test suite must run everywhere.

`fmt` runs on `ubuntu-latest` only to avoid formatter divergence across OSes.

## Exceptions and deviations

None at bootstrap.

---

## Metadata

- **Status:** Accepted (2026-08-07)
- **Kind:** Execution (automation reference sheet)
- **Audience:** Manager CI maintainers, release engineers wiring per-component workflows
- **References:**
  - [Manager spec](./00-manager.md) — the parent this doc is companion to
  - [ADR-0027](../../01%20governance/decisions/0027-coverage-policy.md), [ADR-0028](../../01%20governance/decisions/0028-nightly-canary-release-asset-wait.md), [ADR-0029](../../01%20governance/decisions/0029-cross-component-deep-test-tier.md) — governing decisions
  - [Rollout brief](../../work/2026-08-07-automation-rollout-per-component.md) — the checklist this doc feeds into
