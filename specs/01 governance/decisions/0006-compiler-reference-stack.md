# ADR-0006 — Reference implementation stack for the compiler

The compiler chapter specifies observable contracts — the request document, the pass pipeline, determinism, diagnostics — but it also carried a Cargo workspace layout and a pinned Rust dependency list declared "normative", both of which only one implementation could satisfy. This ADR moves the workspace shape and pinned crate stack into a decision record so the chapter reads as pure contract and dependency bumps become new ADRs rather than spec revisions.

---

## Context

The compiler chapter specifies the compiler's observable contracts: the request document, the sequential pass pipeline with typed inputs and outputs, determinism, diagnostics, and the output artifacts. Alongside those contracts it carried two sections of pure mechanism: [§14.3](../../03%20platform/14-compiler-architecture.md#143-crate-layout) (the Cargo workspace layout — `clean-compiler-types`, `clean-compiler`, and the per-module dependency table) and [§14.13](../../03%20platform/14-compiler-architecture.md#1413-rust-library-stack) (the pinned Rust crate stack, declared "normative" in the chapter itself).

Under [SDD-02](../03-spec-driven-design.md) a specification states what is observable from outside, never the mechanism; a crate layout and a dependency pin list are exactly the kind of statement only one implementation could satisfy. The same reasoning produced [ADR-0002](0002-clean-server-reference-stack.md) for `clean-server`: the stack is worth recording — the contracts were designed and validated against these specific crates — but as a dated, revisable decision, not as spec text.

## Options considered

**A — Keep the stack in the spec chapter.** Readers treat `wasmtime ^38` and the three-crate workspace as requirements of *the compiler* rather than of *our compiler*, and every dependency bump looks like a spec revision. Rejected — same grounds as ADR-0002 option A.

**B — Delete it.** Discards real provenance: knowing the determinism contract was validated against wasmtime's epoch interruption, or that the parser strategy was chosen for LSP-grade recovery, matters when judging an alternative implementation. Rejected.

**C — Record it as a decision.** The chapter keeps contracts; the stack lives here and the chapter cites it. Chosen.

**On parsing specifically**, the chapter had already written its options analysis in ADR form ([14 §14.13.3](../../03%20platform/14-compiler-architecture.md#14133-parsing)); it is adopted here as decided. Parser-combinator libraries — **`chumsky`** and **`nom`** — were evaluated and rejected: LSP-quality per-production error recovery is the top priority and fights the combinator model; the Clean grammar is fixed by the language spec, so combinator agility buys nothing; and spans plus comments must be preserved verbatim for diagnostics and future formatter work. `rustc`, `swiftc`, `roslyn`, `rust-analyzer`, and `tsc` made the same call for the same reasons.

## Decision

**Option C.** The reference compiler is a Cargo workspace of **three crates** — `clean-compiler-types` (spans, diagnostics, error codes, request-document types: the cheap, stable surface external tools depend on), `clean-compiler` (the pipeline behind the canonical `compile()` API), and a thin binary adapter crate — with pass modules layered one-directionally and each IR owned by exactly one module. Its dependency stack:

- **`wasm-encoder`, `wit-component`, `wit-parser`, `wasmparser`** at `^0.220`, pinned together (one `wasm-tools` workspace) — emission, componentization, WIT resolution, and self-validation. `wit-bindgen` is deliberately excluded (it generates guest bindings; this compiler *is* the guest author).
- **`wasmtime`** at `^38` — the compile-time block-handler sandbox ([ADR-0004](0004-block-handler-execution-model.md)). *Corrected 2026-08-18:* the original entry also listed `wasmtime-wasi`, but implementing the sandbox showed that everything WASI provides (clocks, randomness, file descriptors, environment) is exactly the [`BLOCK006`](../../03%20platform/09-error-codes.md#315-block-handler-codes-block) forbidden list — [21 §21.7](../../04%20language/21-block-handlers.md#217-compile-time-execution-environment) stubs *all* host imports, so the sandbox links no WASI provider at all (`clean-language-compiler/docs/DISCOVERIES-M5.md`, item 3).
- **Hand-written recursive-descent lexer and parser** — no parser library (see Options).
- **`ena`** `^0.14` — union-find for type inference, confined to the typecheck module.
- **`clap`** `^4` and **`toml`** `^0.8` — confined to the binary adapter; the compiler library never sees TOML or argv.
- **`serde`/`serde_json`** `^1`, **`sha2`** `^0.10`, **`thiserror`** `^2`, **`indexmap`** `^2` — serialization, hashing, typed errors, deterministic iteration. Excluded from v1: `anyhow`, `tracing`, `rayon`.

As with [ADR-0002](0002-clean-server-reference-stack.md): **none of these names is normative.** An alternative compiler with a different workspace shape, engine, or parser strategy is conformant as long as it satisfies the observable contracts in the compiler chapter — the request document, the pass contracts, determinism, and the diagnostic surface.

## Consequences

**Easier.** Dependency bumps and workspace refactors become new ADRs, not spec revisions. A reader can distinguish the compiler's contract from our Rust choices at a glance.

**Harder.** Two documents to keep coherent: any contract that is only satisfiable because of a specific crate capability (e.g. epoch-based handler timeouts) must be stated as an observable contract in the spec, not left implicit in this stack.

**Now required (DOC-07).** 14 §14.3 and §14.13 are reduced to pointers here, dropping the "each entry below is normative" framing; the observable pass contracts, limits, and determinism rules stay in the chapter.

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Date:** 2026-08-01
- **Supersedes:** None
- **Spec impact:** [14 — Compiler Architecture §14.3, §14.13](../../03%20platform/14-compiler-architecture.md#143-crate-layout) (mechanism moves here; the chapter keeps the observable contracts)
