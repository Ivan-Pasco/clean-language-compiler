# ADR-0004 — Block handler execution model

A library registers a compile-time function with `handles block` to turn a DSL block into typed IR, but the spec tree carried two incompatible pictures of where those handlers run (compiler vs. framework), what they return (typed IR vs. Clean source text), and how they ship (source vs. pre-built binary). This ADR settles the mechanism: handlers are distributed as Clean source, the framework compiles and caches them, and the compiler executes them inside a sandboxed wasmtime sub-instance during its block-expansion pass — one execution model that keeps the typed-IR contract intact.

---

## Context

A block handler is the compile-time function a library registers with `handles block` to turn a block into typed IR ([21 — Block Handlers](../../04%20language/21-block-handlers.md)). The specification tree carried two incompatible models of how that mechanism works, split across three questions:

- **Where do handlers run?** [15 §0.1](../../03%20platform/15-component-model-architecture.md) said "inside `clean-framework`, not inside the compiler. The compiler never executes user Clean code," and [15 §13.2](../../03%20platform/15-component-model-architecture.md#132-library-handler-contract--framework--library) added the compiler "needs no sandboxed sub-runtime." Against that, [14 §14.4 pass 6](../../03%20platform/14-compiler-architecture.md#144-pipeline--sequential-passes) and [03 §3.8](../../03%20platform/03-memory-model.md#38-compile-time-sandbox-memory) specify a sandboxed wasmtime sub-instance *inside the compiler*, [21 — Block Handlers](../../04%20language/21-block-handlers.md) says the reference compiler runs `compiletime` functions, and concern [C-08](../05-concerns.md) presupposes a "sandboxed compile-time plugin runtime" in the compiler. Governance itself was split: boundaries §2.1 gave `compiletime` execution to the compiler while §2.4 gave "running compile-time block handlers" to the framework.
- **What do handlers return?** 15 §13.2 said "vanilla `.cln` text"; 14 pass 6, [LBS §3.2](../../02%20components/framework/09-libraries-specification.md) (Accepted) and the glossary say typed IR.
- **How are handlers distributed?** LBS says "nothing about a library is pre-built" (source distribution); 14 §14.1.1 models a `compiletime_wasm_sha256` per library manifest (compiled WASM in the request); 15 §11.6 said handlers are "pre-compiled to a fixed ABI and loaded by the framework as native modules."

With the documents as they stood, two agents would have built two incompatible compilers. This is the central architectural decision of the compile pipeline, so it is recorded here rather than resolved silently by precedence.

## Options considered

**A — Text-to-text expansion in the framework.** The framework runs each handler during its orchestration step; the handler receives the block and returns vanilla Clean source; the compiler then compiles ordinary Clean and needs no sandbox (the 15 §0.1/§13.2 model). Rejected: at the point the framework runs, no type information exists, so a handler cannot validate its block against the project's actual types — the whole point of the typed `BlockAST` → typed `IR` contract. Giving the framework a type-checker to fix that would plant compiler logic in the framework, violating [C-11](../05-concerns.md) ("It never contains parser, type-checker, or codegen logic"). Text output also throws away the typed-IR contract that LBS §3.2 and the block-handler chapter already specify as Accepted.

**B — Source distribution; framework compiles and caches; compiler executes in its sandbox.** Libraries distribute handlers as Clean source (LBS intact). At install time the framework compiles each handler to WASM, caches the artifact, and — as the realization of its "block expansion ownership" under [C-11](../05-concerns.md) — assembles the compilation request naming each handler by hash (`library_manifests[].compiletime_wasm_sha256`, exactly what [14 §14.1.1](../../03%20platform/14-compiler-architecture.md#1411-inputs) models). The compiler executes the handlers during its block-expansion pass in a sandboxed wasmtime sub-instance, passing the typed `BlockAST` in and receiving typed `IR` back. This keeps 14, 03 §3.8, 21, LBS, the glossary, and [C-08](../05-concerns.md) intact, and gives each contradiction front a single answer.

## Decision

**Option B.** Block handlers are written and distributed as Clean source. Clean Framework compiles them to WASM at library-install time, caches the compiled artifacts, and assembles the compilation request that names each handler by its SHA-256 hash. The compiler executes those handlers during its block-expansion pass, inside a sandboxed wasmtime sub-instance subject to the request's compile-time limits; a handler's input is the typed `BlockAST` for its block and its output is typed `IR`, which the compiler splices into the program. The framework decides *which* handlers at *which* versions participate in a build; the compiler is the only place handler code executes.

## Consequences

**Easier.** One execution model across 14, 03, 21, LBS, and the glossary; the typed-IR handler contract survives; C-08 and C-11 stop pulling in opposite directions. Handler compilation cost is paid once per install, not per build, and the request document stays self-contained (hashes, not paths).

**Harder.** The compiler must carry a wasmtime-based compile-time sandbox forever; the framework must manage a handler-artifact cache and its invalidation; determinism now depends on the sandbox being deterministic.

**Sandbox limits.** The sandbox is not optional hardening — it is part of the contract. The budgets arrive per-request in `compile_limits` ([14 §14.1.1](../../03%20platform/14-compiler-architecture.md#1411-inputs)) and are enforced during pass [6] ([14 §14.4](../../03%20platform/14-compiler-architecture.md#144-pipeline--sequential-passes)); the reference stack that implements them is pinned by [ADR-0006](./0006-compiler-reference-stack.md). The reference configuration uses `StoreLimits` with `max_memory_size` to enforce the per-handler memory limit, epoch interruption to enforce the per-handler timeout (epoch over fuel: near-zero cost while the handler stays within budget), and deterministic execution flags (no wall clock, seeded randomness) so handler expansion preserves reproducible builds. Guest-visible sandbox memory behavior stays specified in [03 §3.8](../../03%20platform/03-memory-model.md#38-compile-time-sandbox-memory).

**Now required (DOC-07).** 15 §0.1 (toolchain-roles table and its consequence bullets), §13.2 (handler contract: typed IR, sandbox exists), and §11.6 (no "native modules") are rewritten to this model; boundaries §2.1 keeps `compiletime` execution in the compiler with the sandbox named, and §2.4 changes to "compiling and caching library block handlers; assembling the compilation request."

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Date:** 2026-08-01
- **Supersedes:** None
- **Spec impact:** [15 — Component Model Architecture §0.1, §11.6, §13.2](../../03%20platform/15-component-model-architecture.md), [Architecture Boundaries §2.1, §2.4](../01-architecture-boundaries.md), [03 — Memory Model §3.8](../../03%20platform/03-memory-model.md#38-compile-time-sandbox-memory), [14 — Compiler Architecture §14.4 pass 6](../../03%20platform/14-compiler-architecture.md#144-pipeline--sequential-passes)
