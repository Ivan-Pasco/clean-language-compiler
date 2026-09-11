> Moved from `clean-language-foundation` — `02 components/compiler/implementation-notes.md`, at foundation commit c72d093 (2026-09-11). Informative: the law it refers to lives in foundation, cited by id; nothing here binds another implementation (SDD-02).

# Clean-Compiler — Reference Implementation Notes

This file records what the reference compiler is actually built with — workspace shape, pinned crates, parser strategy, and the compile-time sandbox configuration. None of it is normative: an alternative compiler with a different workspace, engine, or parser strategy is conformant as long as it satisfies the observable contracts in 14 — Compiler Architecture — the request document, the pass contracts, determinism, and the diagnostic surface.

---

## Why this file exists

Specs state observable behavior, never mechanism (SDD-02). The stack below is still worth recording — the contracts were designed and validated against these specific crates (the determinism contract against wasmtime's epoch interruption, the parser strategy against LSP-grade recovery requirements) — so it lives here as dated, revisable implementation record, mirroring clean-server's notes. Content relocated from the retired ADR-0006 (Accepted 2026-08-01, corrected 2026-08-18) and the sandbox paragraph of the retired ADR-0004 (Accepted 2026-08-01) on 2026-08-22.

## 1. Workspace shape

A Cargo workspace of **three crates**, with pass modules layered one-directionally and each IR owned by exactly one module:

- **`clean-compiler-types`** — spans, diagnostics, error codes, request-document types: the cheap, stable surface external tools depend on.
- **`clean-compiler`** — the pipeline behind the canonical `compile()` API.
- **A thin binary adapter crate** — argv/TOML handling only; the compiler library never sees TOML or argv.

## 2. Dependency stack

- **`wasm-encoder`, `wit-component`, `wit-parser`, `wasmparser`** pinned together as one `wasm-tools` workspace (originally `^0.220`; the shipped workspace tracks the `0.256` family — the family moves as one) — emission, componentization, WIT resolution, and self-validation. `wit-bindgen` is deliberately excluded: it generates guest bindings, and this compiler *is* the guest author.
- **`wasmtime`** at `^38` — the compile-time block-handler sandbox (§3 below). `wasmtime-wasi` is **not linked at all** (corrected 2026-08-18): everything WASI provides (clocks, randomness, file descriptors, environment) is exactly the `BLOCK006` forbidden list — 21 §21.7 stubs *all* host imports, so the sandbox links no WASI provider (`clean-language-compiler/docs/DISCOVERIES-M5.md`, item 3).
- **Hand-written recursive-descent lexer and parser** — no parser library. `chumsky` and `nom` were evaluated and rejected: LSP-quality per-production error recovery is the top priority and fights the combinator model; the Clean grammar is fixed by the language spec, so combinator agility buys nothing; and spans plus comments must be preserved verbatim for diagnostics and future formatter work. `rustc`, `swiftc`, `roslyn`, `rust-analyzer`, and `tsc` made the same call for the same reasons.
- **`ena`** `^0.14` — union-find for type inference, confined to the typecheck module.
- **`clap`** `^4` and **`toml`** `^0.8` — confined to the binary adapter.
- **`serde`/`serde_json`** `^1`, **`sha2`** `^0.10`, **`thiserror`** `^2`, **`indexmap`** `^2` — serialization, hashing, typed errors, deterministic iteration.
- **Excluded from v1:** `anyhow`, `tracing`, `rayon`.
- **Composition transport:** none — the compiler links no WAC; composition is `clean-host-core`'s job and its pin lives in clean-host-core's notes §2.

## 3. Compile-time sandbox configuration

The sandbox budgets arrive per-request in `compile_limits` (14 §14.1.1) and are enforced during pass [6] (14 §14.4). The reference configuration uses:

- **`StoreLimits`** with `max_memory_size` to enforce the per-handler memory limit.
- **Epoch interruption** to enforce the per-handler timeout — epoch over fuel: near-zero cost while the handler stays within budget. This is what the shipped sandbox does (`crates/clean-compiler/src/blocks/sandbox.rs`: `epoch_interruption(true)`, deadline from `handler_timeout_ms`); no fuel metering is used anywhere in the reference stack. (The retired ADR-0007 carried a stale aside claiming fuel was reserved for this sandbox; the implementation and ADR-0004's text agree on epoch, recorded here as the fact.)
- **Deterministic execution flags** (no wall clock, seeded randomness) so handler expansion preserves reproducible builds.

Guest-visible sandbox memory behavior stays specified in 03 §3.8.

## 4. What this file does NOT cover

Everything observable: the request document schema, pass pipeline, determinism rules, diagnostics, and output artifacts are contracts in 14 — Compiler Architecture and 15 — Component Model Architecture. CI jobs and coverage gates are owned by 05 execution/automation.

---

## Metadata

- **Status:** Informative (implementation record; never normative)
- **Audience:** Anyone building, reviewing, or replacing the reference compiler
- **References:** 14 — Compiler Architecture, 06 — Spec-Driven Design SDD-02, clean-server implementation notes
