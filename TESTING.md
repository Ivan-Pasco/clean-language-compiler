# TESTING.md — clean-language-compiler

Follows the template of foundation `05 execution/testing/00-testing-strategy-overview.md`. Compiler-specific strategy: foundation `05 execution/testing/01-compiler-testing.md`.

## 1. Surface being tested

The `compile()` library API (Platform 14 §14.2.1) and its process adapter: one request document in; `component.wasm` + `build-manifest.json` + `diagnostics.json` (+ optional source map) out. Consumers: Clean Framework (assembles requests), Clean Manager (installs/dispatches the binary), hosts (instantiate the emitted component), the future LSP (shares lexer/parser/typechecker, CCMP-25). Blast radius: total — every Clean program, every host, every cached build.

## 2. Layers in use

| Layer | Tool | Runs on | Status |
|---|---|---|---|
| L1 unit | `cargo test` (in-crate) | every push | active |
| L2 snapshot | `insta` → `tests/snapshots/<layer>/` | every push | active (grows per milestone step) |
| L3 property | `proptest` | every push | planned — M3 (type laws, parse round-trip) |
| L4 integration | wasmtime in-process (dev-dep) | every push | lands at Milestone 1 step 6 (`canonical_abi`) |
| L5 conformance | spec fixtures `tests/cln/`, vendored corpora | PR | planned — M2 (DIA-06 triples), M6/F4 (JSONTestSuite…) |
| L6 e2e | — | — | deliberately omitted (lives in user apps / Studio) |
| L7 fuzz + differential | `cargo-fuzz` + grammar seeds; debug-vs-release diff | nightly | planned — M9 |
| L8 AI review | codegen + spec reviewers | pre-PR | planned — foundation rollout F5 |

Determinism suite (Platform 14 §14.7) enters at Milestone 1 step 8 and is a release blocker from then on.

## 3. Golden bugs

Design list, seeded from the retired compiler's `KNOWLEDGE.md`; each becomes a regression test the day its layer exists:

- Heap pointer initialized before string constants are placed → heap overwrites constants (L4).
- String equality comparison inverted (`eqz` on the wrong branch) → silently wrong results (L2 WAT + L4).
- Structured control-flow lowering dropping statements after nested `if`/`else`, and `else: break` misread as no-else → valid-but-wrong WASM (L4; the two 2026-06-21 bugs).
- Recursive functions not pre-registered before body generation (L1).
- Diagnostic quality regression — wrong span, missing suggestion or `doc_url` (L2 per error code).
- Emitting dual import names for one bridge function (L2 snapshot of the import section).

## 4. Boundary contracts

Reads: request document (Platform 14 §14.1.1, hash-verified). Writes: Component Model component + manifest + NDJSON diagnostics. Hard gates once their surfaces exist: any `Diagnostic` schema change re-snapshots every diagnostic fixture; any WIT-affecting change re-runs `wasm-tools component targets` against the vendored `host.wit` (bytes pinned by `vendored_wit.rs` — refreshing the copy means updating its recorded sha256 in the same commit); every new error code lands with its rule and a snapshot that produces it (CI-enforced from M2).

## 5. Fingerprint discipline

The errors dashboard is not wired up yet (foundation rollout F1). Until then: compile-time panics and miscompilations become GitHub issues carrying the minimal `.cln` reproducer, expected vs actual, and the wasm dump — and a regression test in the same PR as the fix. Never adjust a snapshot to make a regression pass. Spec ambiguity is not a compiler bug: it goes to foundation `work/` as a task brief.

## 6. Review-agent config

Not yet configured (foundation rollout F5: codegen reviewer + spec reviewer are the first two agents ecosystem-wide, wired to this repo). Placeholder for `.ai-review.toml`.

## 7. Known gaps

- No dashboard/fingerprint pipeline (F1 pending) — tracked by the interim GitHub-issue discipline above.
- L3/L5/L7/L8 not yet active — entry points listed in §2 with their milestones.
- Coverage floors (ADR-0027 Tier 1: 80% line / 75% branch / 60% mutation, MC/DC in codegen, typecheck, marshalling, memory): line measured since M4 (CI `coverage` job, `cargo llvm-cov`); branch needs nightly and joins M9's nightly job with mutation, MC/DC and the blocking floors.
- Acceptance against a running clean-server requires the private `clean-host-core` checkout (see `docs/acceptance.md` when it lands).
