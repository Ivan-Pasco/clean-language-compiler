# Platform 14. Compiler Architecture

The compiler is L0 in the layer model — it has no observable side effects at runtime and does one thing: consume source and produce a WebAssembly component. This chapter is the authoritative specification of the compiler itself: the shape of the compilation request it accepts, the pipeline it runs, the artifacts it emits, and the contracts each pass must uphold. Every other platform chapter describes something the *running* program must satisfy; this one describes something the *compiler* must satisfy, and is the bridge between "what Clean means" (the language spec) and "what runs on the host runtime" (the platform contract).

---

## 14.1 Inputs and Outputs


The compiler is a pure function of its inputs. The same inputs must always produce byte-identical outputs; nondeterminism in the pipeline is a bug, not a feature.

### 14.1.1 Inputs

The compiler accepts **one request document**, a JSON payload that packages every source file and every configuration value required to produce a component. This shape exists so the compiler can be driven equally well by the toolchain (`cln` dispatching through Clean Framework), by the LSP, by AI agents, and by CI, without any of them having to touch the filesystem in an ambient way.

```json
{
  "spec_version": "1",
  "project": {
    "name": "my-app",
    "version": "0.1.0"
  },
  "build": {
    "target": "wasm32-server",
    "optimization": "release",
    "memory": { "tier": "standard" },
    "strip": true,
    "component_model": true,
    "memory64": false
  },
  "folders": {
    "app/data":   ["data"],
    "app/server": ["server"]
  },
  "dependencies": {
    "data":   { "version": "1.4.0", "resolved_from": "registry" },
    "server": { "version": "1.4.0", "resolved_from": "registry" }
  },
  "compile_limits": {
    "handler_timeout_ms": 5000,
    "handler_memory_mb": 128,
    "total_timeout_min": 10,
    "max_file_size_mb": 4,
    "max_import_depth": 32,
    "max_nesting_depth": 256
  },
  "telemetry": { "consent_level": "error-with-code" },
  "target_world": {
    "host": "clean-server",
    "version": "0.1.0",
    "world": "server",
    "sha256": "9f2b1c...",
    "wit": "package clean:host@0.1.0;\n\ninterface routing { ... }\n\nworld server {\n  import routing;\n}\n"
  },
  "sources": [
    {
      "path": "app/main.cln",
      "sha256": "e3b0c44...",
      "content": "start()\n  console.print(\"hello\")\n"
    },
    {
      "path": "app/data/User.cln",
      "sha256": "8fa16c...",
      "content": "..."
    }
  ],
  "library_manifests": [
    {
      "name": "data",
      "version": "1.4.0",
      "wit": "package clean:library/data@0.1.0;\n...",
      "handles_blocks": ["data", "endpoints"],
      "compiletime_wasm_sha256": "d41d8c..."
    }
  ],
  "overrides": [
    { "path": "build.optimization", "value": "debug", "source": "cli" }
  ]
}
```

**Rules.**

- Every field in `build`, `memory`, `folders`, `dependencies`, `compile_limits`, and `telemetry` mirrors [§07](./07-build-config.md) exactly. This document is the JSON projection of a fully-resolved `clean.toml`, not a second config schema. A `clean.toml` is *lowered* to this shape by the caller (Clean Framework, dispatched by Clean Manager, or an AI harness). The compiler never reads `.toml` — it reads this JSON.
- `sources` is the *complete* set of `.cln` files the caller wants compiled, addressed by their project-relative POSIX path. The compiler does no filesystem discovery. If the caller forgot a file, the compilation fails at import resolution, not at read time.
- Every `sources[].content` is UTF-8 text — guaranteed by the caller under [TXT-02](./17-text-encoding.md#txt-02--the-reader-validates-at-the-moment-it-reads), which validates the raw bytes at the moment it reads them and refuses the file with `CFG005` otherwise. The compiler does not re-derive this: it never sees the bytes on disk (`CMP-01`), and a file decoded with the wrong table produces text that is well-formed UTF-8 and therefore undetectable downstream. Every `sources[].sha256` is the hex-lowercase SHA-256 of the *decoded* content. The compiler verifies each hash and refuses the request on mismatch (`RQD001`).
- `library_manifests` is the caller-resolved dependency closure — one entry per direct-or-transitive library. The compiler does no registry lookup. Version-solving happens outside the compiler; the compiler validates the manifest it was given against `dependencies`.
- `target_world` is the host contract the component is compiled against, carried **by value** ([ADR-0033](../01%20governance/decisions/0033-target-world-in-compilation-request.md)). It is **required**: a request without it is refused with `RQD002`, and the compiler neither fetches a world nor derives one from `build.target`. Five string fields, all required:
  - `wit` — the target host's `host.wit`, **verbatim as published**. Not a path, URL, or cache reference (`CMP-01`), and not an extract: the caller transmits the declaration unmodified so the request records what the host shipped. A `host.wit` may be a multi-package WIT document — the root package declaration unbraced, every additional package (e.g. a composed bridge interface the world re-exports) in braced `package … { … }` form, the standard WIT encoding for several packages in one document. The compiler resolves an interface's fully-qualified package from this one document; there is no second delivery channel for auxiliary packages.
  - `world` — the name of the world *within* `wit` to validate against, selected by the caller from `build.target` per the mapping in [07 §7.2](./07-build-config.md#72-schema--top-level). A `host.wit` may declare more than one world; without this field the compiler would have to resolve the target-to-world mapping from its own built-in table, making the compiler binary the authority on what a host provides — the coupling [BVER-03](./08-bridge-versioning.md#84-host-declaration) exists to prevent. The compiler MUST refuse with `RQD002` if `world` does not name a world present in `wit`.
  - `host` — the host name from the project's `[target]` block, e.g. `"clean-server"`.
  - `version` — the **resolved** host version the caller pinned, never the `clean.toml` semver constraint. A constraint here would let two byte-identical requests denote different host versions, breaking `CMP-02`.
  - `sha256` — hex-lowercase SHA-256 of `wit`, as recorded in the project lockfile per [BVER-03](./08-bridge-versioning.md#84-host-declaration). Redundant with `wit` for cache-keying and deliberately so: it is the forensic link back to `.cln/lock.toml`, and it makes a disagreement between the pinned hash and the inlined text detectable rather than silent.

  Who populates it: Clean Framework, from the `host.wit` it already fetched for its Moment 1 check ([16 §16.4](./16-host-contract-validation.md#164-the-three-check-moments)). The compiler's use of it is the World Import Check ([§14.4.2](#1442-detailed-pass-responsibilities) pass [9], `COM012`).
- `overrides` is a flat audit trail of every value that came from a source other than `clean.toml` (CLI flag, env var, programmatic override). The compiler records this verbatim in the build manifest ([§14.8](#148-build-manifest)) so the exact build is reproducible.
- Unknown top-level keys are a hard error (`RQD002`). Unknown keys inside a well-known section are a hard error scoped to that section. A **missing required field** and a malformed value are the same error — see [RQD002](./10-semantic-rules.md#rqd002--request-schema-violation) for the rule as owned. There is no "ignore-and-continue" for schema drift.

The request document has a `spec_version` field. Incrementing it is a breaking change to the compiler's API and is governed by the ADR process, like bridge versioning ([§08](./08-bridge-versioning.md)).

### CMP-01 — The request document is self-contained; the compiler touches nothing else


The compiler MUST obtain every input — sources, configuration, library manifests, target world WIT — from the request document alone: no filesystem discovery, no network access, no registry lookup. It MUST verify every `sources[].sha256` against the decoded content and refuse the request with `RQD001` on mismatch, and MUST reject unknown top-level keys (and unknown keys inside well-known sections) with `RQD002` — there is no ignore-and-continue for schema drift. Check: a compilation succeeds or fails identically when run with no filesystem beyond the request document and no network.

### 14.1.2 Outputs

On success, the compiler produces:

- **`component.wasm`** — the WebAssembly component conforming to the target world's WIT ([§15.2](15-component-model-architecture.md)).
- **`build-manifest.json`** — the reproducibility record ([§14.8](#148-build-manifest)).
- **`diagnostics.json`** — every warning and info diagnostic (any error would have prevented success). Shape defined by [§13](./13-diagnostic-format.md).
- **`source-map.json`** — optional, present when `optimization` is `debug` or `release` (not `size`). Maps WASM offsets back to `sources[].path` and byte ranges.

On failure, the compiler produces `diagnostics.json` and exits with code `1`. No partial `component.wasm` is written; the caller sees an all-or-nothing result.

The compiler writes outputs to a caller-specified directory (library API) or to stdout as a single tarball (process adapter, with `--stdout-tar`).

### CMP-05 — Outputs are all-or-nothing, and land only where the caller pointed


On failure the compiler MUST produce `diagnostics.json` and exit non-zero, and MUST NOT write a partial `component.wasm` — the caller sees an all-or-nothing result. The compiler MUST NOT mutate the input directory and MUST NOT write anywhere other than the caller-specified output directory (or stdout with `--stdout-tar`). Check: after any failed compilation, no `component.wasm` exists in the output directory and nothing outside it changed.

---

## 14.2 Invocation Surface


The compiler ships as a library API with a thin process adapter. Neither is user-facing: the user surface is owned by [Manager §00.3](../02%20components/manager/00-manager.md#003-command-surface).

### 14.2.1 Library API — the canonical entry point

```rust
pub struct CompileRequest { /* deserialized 14.1.1 JSON */ }
pub struct CompileArtifact {
    pub wasm: Vec<u8>,
    pub manifest: BuildManifest,
    pub diagnostics: Vec<Diagnostic>,
    pub source_map: Option<SourceMap>,
}

pub fn compile(request: CompileRequest) -> Result<CompileArtifact, CompileError>;
```

`CompileError` carries the failing diagnostics (never a stringly-typed message). All types are serializable so any caller — the framework, the LSP, an MCP server, an AI harness — uses the same surface without re-implementing the JSON layer.

Every other entry point is a wrapper around `compile`.

### 14.2.2 Process adapter

The compiler is invocable as a standalone process (`clean-compiler`, invoked by Clean Framework when the user runs `cln build`):

1. Receives the complete compilation request document from Clean Framework — sources inline, configuration resolved, library manifests included ([§14.1.1](#1411-inputs)). Clean Framework has already read `clean.toml`, walked the project, and lowered everything to the request shape.
2. Calls `compile(request)`.
3. Writes outputs to the caller-specified directory.

The process adapter is a thin transport step. It has no compilation logic of its own and reads nothing that is not in the request document. If a bug appears only when invoked as a process and not from the library API, the bug is in the adapter, not in the compiler.

### 14.2.3 JSON-RPC / MCP adapter

An adapter exposes `compile` over JSON-RPC and MCP so AI agents and remote hosts can call it without linking against the library. The wire format is the request document unchanged. In the internal build order this adapter comes after the library API stabilizes; it is a wrapper, not a rewrite, and it is part of v1 (see §14.14 — none of the v1 surface is deferred).

---

## 14.3 Implementation Packaging


The compiler is invocable in two ways, and both consume the same request document:

- **As a library** — the canonical `compile(request)` entry point ([§14.2.1](#1421-library-api--the-canonical-entry-point)).
- **As a process** — a standalone binary that reads a request document and writes the outputs ([§14.2.2](#1422-process-adapter)), so any orchestrator (Clean Framework, CI, an AI harness) can drive it without linking against it.

The internal crate layout, module structure, and per-module dependency allocation of the reference implementation are recorded in [ADR-0006 — Compiler reference stack](../01%20governance/decisions/0006-compiler-reference-stack.md); they are implementation decisions, not part of this contract. What is contractual: the request document in ([§14.1.1](#1411-inputs)), the outputs out ([§14.1.2](#1412-outputs)), the pass contracts ([§14.4.1](#1441-pass-contracts)), and the determinism invariant ([§14.5](#145-determinism-and-reproducibility)).

---

## 14.4 Pipeline — Sequential Passes


Compilation is a strictly sequential sequence of passes. Each pass takes the previous pass's output as its input, produces its own output, and does not run again. Incremental compilation is out of scope for v1 (see [§14.11](#1411-non-goals)); the pipeline is designed to accommodate it later by having every pass be a pure function of its input.

```
Request JSON
    │
    ▼
[1] Request Validation      request/   → ValidatedRequest
    │
    ▼
[2] Lex                     lexer/     → per-file TokenStream
    │
    ▼
[3] Parse                   parser/    → per-file AST
    │
    ▼
[4] Resolve                 resolver/  → ResolvedAST (module graph + every ident bound)
    │
    ▼
[5] Type Check              typecheck/ → TypedAST (includes capability/contract checks)
    │
    ▼
[6] Block Handler Expansion blocks/    → TypedAST' (block handlers expand Blocks)
    │
    ▼
[7] HIR Lowering            hir/       → HIR
    │
    ▼
[8] MIR Lowering + Optimize mir/       → MIR
    │
    ▼
[9] World Import Check      codegen/   → verified against target WIT world
    │
    ▼
[10] Codegen + Assembly     codegen/   → component.wasm (core WASM + component wrap)
    │
    ▼
CompileArtifact  (component.wasm + build-manifest.json emitted by the driver)
```

Each pass runs to completion and *collects* all its diagnostics before deciding whether the compilation can continue. A single syntax error does not abort parsing — the parser recovers, records the diagnostic, and continues so the developer sees every syntax error in one run. Passes downstream of an error-producing pass are skipped; the compiler reports "compilation aborted after phase N" so the user knows what did and did not run.

### 14.4.1 Pass contracts

Every pass has a stable contract:

| Contract | Requirement |
|----------|-------------|
| **Input type** | Exactly one predecessor IR type. No pass takes two. |
| **Output type** | Exactly one successor IR type or `()` for terminal passes. |
| **Determinism** | Given the same input, produces byte-identical output. No `HashMap` iteration in ordered emission paths — use `BTreeMap` or explicit sort. |
| **Diagnostics** | May append to a `DiagnosticSink` but never read it. Passes never branch on prior diagnostics. |
| **Error mode** | Recoverable errors emit a diagnostic and continue with a best-effort placeholder. Unrecoverable errors return `Err(CompileError)`. |
| **No I/O** | Passes call no host functions, open no files, make no network calls. Everything they need is in the input. |

These contracts are enforced by convention in v1 and by a dedicated pipeline-contract integration suite (reference layout: [ADR-0006](../01%20governance/decisions/0006-compiler-reference-stack.md)).

### 14.4.2 Detailed pass responsibilities

Each pass is described here in one paragraph — enough to build against, not so much that it duplicates the language spec or the block-handler spec.

**[1] Request Validation** — Deserializes the JSON request, validates every field against the schema in [§14.1.1](#1411-inputs), verifies every `sources[].sha256`, applies `overrides`, and produces a `ValidatedRequest`. Failure mode: `RQD###` diagnostics; the pipeline stops immediately.

**[2] Lex** — Runs `lexer::lex` on each `sources[]` entry independently. Produces a `TokenStream` with byte-accurate spans anchored to the file id from `source::SourceMap`. Comment tokens are preserved for the LSP.

**[3] Parse** — `parser::parse` consumes tokens, produces an `ast::File`. The parser is error-recovering: an unclosed brace does not stop parsing subsequent items. Every AST node has a span; there are no synthetic spans without a real source anchor.

**[4] Resolve** — Builds the module graph by following `import` statements (detecting cycles per [§10 `IMPORT001`](./10-semantic-rules.md#import001--circular-dependency)), applies the folder-to-library mapping from `request.folders` ([§07.6](./07-build-config.md#76-folder-to-library-mapping-folders)) to determine which library block names are in scope for each file, and walks each file's AST to resolve every identifier to a binding: a local, a parameter, an imported symbol, a class member, a library block name, or a stdlib name. Produces `ResolvedAST`. Missing files, missing libraries, depth-limit violations, and undefined names all report here; unresolved identifiers become `Error` bindings so type checking can continue. Splitting this pass in two (imports first, then names) is an implementation choice — the *pipeline* sees one input and one output.

**[5] Type Check** — Runs bidirectional inference over `ResolvedAST`. Produces `TypedAST` where every expression has a resolved type, method-style syntax is desugared ([§16](../04%20language/16-method-style-syntax.md)), and every capability declaration on a class ([§14 Classes and Objects](../04%20language/14-classes-and-objects.md)) is verified — companion types exist where required, contract signatures are satisfied, companion-access rules are respected. Type errors become `SEM###` diagnostics; the offending expression's type becomes `Error` which absorbs further errors along that path (no cascading noise). **When the program declares library blocks, this pass's findings are provisional:** the pass still runs in full — its `TypedAST` is pass [6]'s input — but its diagnostics are held back, because handlers are about to emit the companions and functions the user's code already refers to ([§21 Block Handlers](../04%20language/21-block-handlers.md)); a pass that hard-failed on those symbols would make every block-using program uncompilable. Pass [6]'s re-validation of the expanded program is authoritative. When no library block is present, the findings are final and report here.

**[6] Block Handler Expansion** — For every block whose keyword is a library-declared block name, load the library's `compiletime` function (the framework-compiled, cached WASM identified by `library_manifests[].compiletime_wasm_sha256`, instantiated in the compiler's sandboxed runtime with the compile-time limits from `request.compile_limits`), pass it the typed AST subtree, receive typed IR back, and splice it in. This is the mechanism from [§21 Block Handlers](../04%20language/21-block-handlers.md), under the execution model decided in [ADR-0004](../01%20governance/decisions/0004-block-handler-execution-model.md). Handler timeouts and memory-limit breaches produce [`BLOCK005`](./09-error-codes.md#315-block-handler-codes-block) diagnostics with the offending library named — `BLD001` covers whole-build limits, not per-handler budgets. Capability checks are re-run over any newly-introduced companions. **After splicing, the pass re-validates the expanded program; this re-validation is authoritative over pass [5]'s held-back findings** (see pass [5]): an error anchored inside an expanded block is [`BLOCK004`](./09-error-codes.md#315-block-handler-codes-block) — §21.4's malformed-IR case, reported at the user's block span — while an error anchored in user code keeps its own code and span; a held pass-[5] finding that the expansion resolved is discharged, and one it did not resolve reports here unchanged.

**[7] HIR Lowering** — Erases method-style sugar, desugars string interpolation to concatenation, canonicalizes control flow, and produces a smaller, more uniform tree. HIR is the last representation where source spans are the primary way of addressing nodes.

**[8] MIR Lowering + Optimization** — Converts HIR to a linear, SSA-shaped IR suitable for direct WASM emission. Runs the optimization profile from `request.build.optimization` ([§07.5](./07-build-config.md#75-optimization-profiles)). `debug` runs no optimizations; `release` runs inlining, dead-code elimination, and tree-shaking; `size` runs the size-first subset. Optimization is required to be semantics-preserving — a `debug` and a `release` build of the same source must produce byte-identical behavior for every test in the conformance suite.

**[9] World Import Check** — Walks every `host function` call site in MIR and verifies its signature exists in the target world — the world named by `target_world.world` within the WIT delivered as `target_world.wit` ([§14.1.1](#1411-inputs)); for library-declared imports, `library_manifests[].wit`. The compiler never fetches a host's WIT; the scope split with framework and host validation is defined in [16 §16.10](./16-host-contract-validation.md). Any call site whose imported function is not in the world produces `COM012` and aborts before codegen. This is the invariant from [§00.5](./00-overview.md#5-end-to-end-compile--instantiate--run).

**[10] Codegen + Assembly** — Emits a core WASM module (string, list, and record layouts follow [§03 Memory Model](./03-memory-model.md); every host import becomes an `(import …)` entry with the WIT-derived name; debug info follows the optimization profile), then wraps the core module in a component (emitter and componentizer tooling: [ADR-0006](../01%20governance/decisions/0006-compiler-reference-stack.md)), attaching the target world's WIT so the runtime can verify conformance at instantiation time. The pipeline driver then serializes the `BuildManifest` ([§14.8](#148-build-manifest)) — this is a straightforward struct-to-JSON step, not a pass with its own IR.

---

## 14.5 Determinism and Reproducibility


### CMP-02 — Same request in, byte-identical outputs out


Two invocations with the same request document MUST produce byte-identical `component.wasm`, byte-identical `build-manifest.json`, and byte-identical `diagnostics.json` — across runs, hosts, and platforms declared as reproducible. This is not aspirational; it is a testable invariant, enforced by a determinism suite that compiles a fixture twice and hashes both outputs (reference layout: [ADR-0006](../01%20governance/decisions/0006-compiler-reference-stack.md)). Check: the determinism suite of §14.7 passes; `cln repro build` (§14.14.6) reproduces any historical build from its manifest.

Sources of nondeterminism the compiler must guard against:

- **Hash-map iteration order.** Any pass that emits ordered output uses ordered collections or sorts explicitly; hash-order iteration in emitting paths is denied by lint (reference stack: [ADR-0006](../01%20governance/decisions/0006-compiler-reference-stack.md)).
- **Wallclock timestamps in outputs.** The manifest records the compiler version, not the wall time. Timestamps that must appear in outputs (e.g. debug info) use `SOURCE_DATE_EPOCH` if set, otherwise `0`.
- **Absolute paths.** Sources are addressed by their project-relative path from the request; the compiler never records an absolute path anywhere.
- **Parallelism.** Passes may internally parallelize (e.g. per-file lex/parse) but must reduce results in a deterministic order — always by `sources[]` index, never by completion order.
- **Rustc / library version drift.** The manifest records the exact compiler binary version; a change in codegen output between compiler versions is a semver event for the compiler.

---

## 14.6 Diagnostics and Error Handling


The compiler is a diagnostic-emitting machine that happens to also produce a WASM component when nothing was wrong. Every diagnostic follows [§13 Diagnostic Format](./13-diagnostic-format.md) and carries a code from the registry in [§09 Error Codes](./09-error-codes.md).

Code ranges the compiler emits from (registry home: [§09 §1](./09-error-codes.md); the `RQD` range is a new range whose formal registration is pending per 09 §1.2):

| Range | Category |
|-------|----------|
| `RQD###` | Malformed request document, integrity failures (`RQD001`, `RQD002`) |
| `CFG###` | Config lowering issues surfaced by the caller before invocation (also reported here for direct-library callers) |
| `SYN###` | Lexer and parser |
| `SEM###` | Semantic passes (resolver, typecheck, capability check) |
| `IMPORT###` | Import graph resolution (cycles, missing modules) |
| `COM###` | World import check (`COM012`), codegen and component assembly invariants (`COM013`) |
| `BLD###` | Compile-time limits from §07.8 (`BLD001`) |

Diagnostics from every pass are accumulated in a `DiagnosticSink`. Errors do not immediately abort — the pass finishes so the user gets one report for every finding it can produce.

### CMP-03 — Every import is verified against the world in the request


The compiler MUST verify every `host function` call site against the target world — the world named by `target_world.world` within the WIT delivered as `target_world.wit` ([§14.1.1](#1411-inputs)); for library-declared imports, `library_manifests[].wit`. Any call site whose imported function is not in the world MUST produce `COM012` and abort before codegen (pass [9], [§14.4.2](#1442-detailed-pass-responsibilities)). The compiler MUST NOT fetch a host's WIT — the scope split with framework and host validation is [16 §16.10](./16-host-contract-validation.md). Check: a program importing `clean:host/dom` compiled against the `server` world fails with `COM012` and produces no `component.wasm`.

### CMP-04 — Internal failures are `COM013`, never a user error


Every self-produced artifact MUST be validated before it leaves the compiler; a validation failure — or any broken internal invariant — MUST surface as `COM013` (internal invariant), presented as a compiler bug rather than a user error, never as a stringly-typed message. Check: an induced codegen invariant breach yields a `COM013` diagnostic conforming to [§13](./13-diagnostic-format.md), not a panic message.

**Recovery contract.** Every pass declares up-front which errors it recovers from (with a placeholder) and which it does not. Recovery is not opportunistic; it is a documented decision per error class. This is what keeps error cascades from drowning the real error under 200 downstream noise diagnostics.

---

## 14.7 Testing Strategy


The compiler ships with five categories of tests.

| Category | Purpose |
|----------|---------|
| **Unit tests** | Per-module logic (lexer, resolver, individual codegen instructions). Fast. |
| **Pipeline contract tests** | Enforces §14.4.1 (pass isolation, determinism, no I/O). |
| **Golden tests** | Small `.cln` snippets compiled to WASM; the emitted module (in WAT form) is diffed against a checked-in expected output. Catches accidental codegen changes. |
| **Conformance tests** | Every language and platform spec section has at least one conformance test. A `debug` and `release` build must both pass. |
| **Determinism tests** | Compiles fixtures twice, asserts byte-identical outputs. |

Suite locations in the reference implementation follow [ADR-0006](../01%20governance/decisions/0006-compiler-reference-stack.md).

Every bug fix ships with a regression test in the appropriate category. A bug that could have been caught by a pipeline contract test but wasn't triggers a contract-test expansion in the same commit.

---

## 14.8 Build Manifest


`build-manifest.json` is the reproducibility record for a single compilation. Its shape:

```json
{
  "spec_version": "1",
  "compiler": {
    "version": "1.4.0",
    "sha256": "…"
  },
  "request_sha256": "…",
  "inputs": {
    "sources": [
      { "path": "app/main.cln", "sha256": "…" }
    ],
    "library_manifests": [
      { "name": "data", "version": "1.4.0", "wit_sha256": "…", "compiletime_wasm_sha256": "…" }
    ]
  },
  "resolved_config": { /* the request's build/memory/folders/dependencies/compile_limits/telemetry, verbatim */ },
  "overrides": [
    { "path": "build.optimization", "value": "debug", "source": "cli" }
  ],
  "outputs": {
    "wasm_sha256": "…",
    "source_map_sha256": "…"
  },
  "diagnostics": [ /* every warning and info emitted, in emission order */ ],
  "timings": {
    "lex_ms": 12,
    "parse_ms": 34,
    "…": "…"
  }
}
```

The manifest is a first-class output, not a debug convenience. CI systems, dashboards, and AI harnesses use it to compare builds. A downstream tool that wants to know "did this build change anything?" compares `outputs.wasm_sha256`, not the file bytes.

---

## 14.9 Performance Targets


These are v1 targets on a modern developer machine (M-series Mac, Linux x86_64 laptop). They exist so we notice regressions, not so we ship a benchmark suite.

| Project size | Cold compile (`release`) | Cold compile (`debug`) |
|--------------|--------------------------|------------------------|
| Small (< 1k LOC, < 3 libraries) | < 1.5 s | < 500 ms |
| Medium (< 20k LOC, < 10 libraries) | < 10 s | < 3 s |
| Large (< 100k LOC, < 25 libraries) | < 60 s | < 15 s |

Incremental compilation is out of scope for v1. Watch-mode recompilation (the operation behind `cln dev`; user surface: [Manager §00.3](../02%20components/manager/00-manager.md#003-command-surface)) achieves interactive speeds by keeping the compiler process warm and re-running the full pipeline over the changed file set; it is not incremental in the compiler-internal sense.

---

## 14.10 AI-Assisted Development Notes


This spec is designed to be built by AI agents working against it. The following properties support that:

- **The pipeline is sequential and typed.** Every pass has one input type and one output type. An agent can be pointed at "implement pass N" without needing to hold the whole compiler in mind.
- **The request document is one JSON schema.** An agent generating a test fixture generates one file, not a directory.
- **Determinism is a testable invariant.** An agent's change either preserves it or breaks the determinism test — the loop is tight.
- **Diagnostics have machine-readable codes.** An agent iterating on a failing conformance test parses `diagnostics.json` rather than scraping stderr.
- **Passes are pure functions.** An agent can write a unit test for pass N by constructing an input of the predecessor's output type — no filesystem, no environment.

Each pass in [§14.4](#144-pipeline--sequential-passes) is a natural task boundary for an implementing agent.

---

## 14.11 Non-Goals


- **Incremental compilation inside the compiler.** V1 recompiles from scratch every invocation. The pipeline is designed to make incrementality possible later (every pass is a pure function of its input) but the compiler itself carries no query engine, no on-disk cache, no persistent DB — every request is answered from the request document alone. An external, opt-in artifact cache lives around the compiler at the framework layer; see [§14.15](#1415-external-build-cache).
- **Multi-language frontends.** The compiler is Clean-only. There is no plan for a shared IR consumed by other frontends.
- **Native code targets.** The only backend is WebAssembly component model. No x86, no ARM, no LLVM. If a future need arises, it is a new backend added to `codegen/`, not a replacement.
- **Runtime plugins to the compiler.** Libraries influence compilation via block handlers ([§21](../04%20language/21-block-handlers.md)), which are Clean code run in the compiler's sandboxed runtime ([ADR-0004](../01%20governance/decisions/0004-block-handler-execution-model.md)). There is no native plugin ABI, no dynamic library loading, no compiler extension mechanism outside the block-handler contract.
- **Compiler as a hosted service.** The compiler is a library and a process. Any hosted-service story (cloud CI, agent harness) is a wrapper around the library, not a distinct thing.

---

## 14.12 Deferred Refinements


1. **Query-graph incremental compilation.** Once the sequential pipeline is stable and the conformance suite is comprehensive, the pipeline may be re-expressed as a demand-driven query graph (parse-per-file, resolve-per-module, typecheck-per-item, codegen-per-item) with fine-grained internal caching. The pass-purity discipline in [§14.4.1](#1441-pass-contracts) is the precondition; internal packaging evolution alongside that migration is an [ADR-0006](../01%20governance/decisions/0006-compiler-reference-stack.md) concern. Doing it before we feel the pain would slow down v1 for no gain. The external, whole-artifact build cache in [§14.15](#1415-external-build-cache) is a separate, smaller mechanism that does not depend on this refinement.
2. **Distributed compilation.** For very large projects, dispatching per-file lex/parse and per-item codegen across a build farm. Only meaningful once incremental compilation is in place.
3. **Alternative optimization backends.** Post-codegen optimizer integration under a new `size-max` profile. Excluded from v1 to keep the shipped binary self-contained.

---

## 14.13 Reference Stack


The reference implementation's dependency choices — component-model authoring and validation tooling, the block-handler sandbox runtime, the parser strategy, type-inference support, serialization and hashing — are recorded with their version pins and rationale in [ADR-0006 — Compiler reference stack](../01%20governance/decisions/0006-compiler-reference-stack.md). They are implementation decisions, not part of the compiler's contract.

What remains contractual here, independent of the chosen stack:

- **Determinism** ([§14.5](#145-determinism-and-reproducibility)): same request document in, byte-identical outputs out — including across block-handler execution. The handler sandbox exposes no non-deterministic host imports: wall clock is replaced with `SOURCE_DATE_EPOCH` (or `0`), and randomness is seeded from a request-derived hash ([ADR-0004](../01%20governance/decisions/0004-block-handler-execution-model.md)).
- **No network, no ambient filesystem.** The compiler performs no network access and reads nothing that is not in the request document.
- **Sandboxed block-handler execution** with the memory and timeout limits from `request.compile_limits` enforced as hard failures ([`BLOCK005`](./09-error-codes.md#315-block-handler-codes-block), naming the offending library), per [ADR-0004](../01%20governance/decisions/0004-block-handler-execution-model.md). Handler WASM is validated before instantiation.
- **Self-validation.** Every self-produced artifact is validated before it leaves the compiler; a validation failure is `COM013` (internal invariant) and is a compiler bug, not a user error.

---

## 14.14 Compiler API Operations — v1 Requirements


The compiler is not judged only by whether it produces correct WASM. It is judged by what a developer feels in the seconds between saving a file and understanding what happened. This section is the v1 contract for the operations the compiler exposes to its callers — Clean Framework and Clean Manager. Every item below is a v1 shipping requirement; none are deferred.

**None of these operations is a user-facing command.** The user surface is owned by [Manager §00.3](../02%20components/manager/00-manager.md#003-command-surface) ([MGR-01](../02%20components/manager/00-manager.md): component binaries MUST NOT be documented or invoked as user-facing commands); where an operation backs a `cln` verb, the verb is named here only as a pointer to that table. Two former subsections are retired entirely because their responsibilities do not belong to the compiler: project scaffolding (`cln new`) is a Framework responsibility ([Architecture Boundaries §2.4](../01%20governance/01-architecture-boundaries.md)), and dependency addition (`cln add`) is a Manager built-in ([Manager §00.3](../02%20components/manager/00-manager.md#003-command-surface)).

The design of the pipeline (deterministic, staged, typed diagnostics), the library-first packaging (JSON in / JSON out), and the request document (self-contained, hash-verified) exist in part to make the following operations cheap to build. This section is where that investment is spent. Every operation is a framing of the same `compile()` library API ([§14.2.1](#1421-library-api--the-canonical-entry-point)) over the same request document; none is a special code path. This is what keeps the surface honest: a bug in the watch-mode loop cannot exist without also being a bug in a full build.

### 14.14.1 Diagnostic re-projection (behind `cln why`)

The re-projection operation explains why the compiler rejected a specific call site, walking the developer back through the layer chain that produced the rejection. It is the counterpart of the code lookup behind `cln explain` ([§13 §8](./13-diagnostic-format.md)): `explain` is "what does this code mean in general," re-projection is "what happened at this exact location."

Example output shape (illustrative, not normative — the diagnostic text follows [§13](./13-diagnostic-format.md)):

```
$ cln why app/main.cln:42
Call `file.read("/etc/passwd")` was rejected.
  ├─ Resolves to host function: wasi:filesystem/types.descriptor.read
  ├─ Your build target: wasm32-browser
  ├─ The wasm32-browser world does not import wasi:filesystem
  └─ Suggested alternatives:
      • Change target to wasm32-server (permitted by every WASI package above)
      • Use `storage.get(key)` — available in the browser world, backed by clean:bridge/storage
```

The operation reads `diagnostics.json` from the most recent build and re-projects the diagnostic at the requested location with expanded provenance: which pass rejected it ([§14.4](#144-pipeline--sequential-passes)), which world was targeted, which capability was missing, which alternatives satisfy the same intent in this world.

All the data required is already in the diagnostic — re-projection is a re-presentation, not a re-compilation. Latency target: under 100 ms from cold *(informative target — no measurement procedure defined; see §14.9)*.

### 14.14.2 First-class `bytes` type

Clean grows a primitive `bytes` type in v1. This is a language-spec change with a compiler consequence; the compiler-side contract is:

- **Lexer** — accepts byte-literal syntax `b"..."` (UTF-8 bytes) and `b"\x00\xFF..."` (hex-escaped bytes). Tokens produced in [pass 2](#144-pipeline--sequential-passes).
- **Type checker** — treats `bytes` as an opaque primitive with the following operations, all typechecked in [pass 5](#144-pipeline--sequential-passes):

| Operation | Signature |
|-----------|-----------|
| Length | `data.length() -> integer` |
| Index | `data[i] -> integer` (single-byte read; out of range is [`RUN013`](./09-error-codes.md#312-runtime-codes-run)) |
| Slice | `data.slice(start, end) -> bytes` (indices clamp like `string.substring` — [15 §Bytes Module](../04%20language/15-standard-library.md#bytes-module)) |
| From string | `bytes.fromText(text) -> bytes` — the UTF-8 encoding of `text` ([15 §Bytes Module](../04%20language/15-standard-library.md#bytes-module)) |
| To string | `bytes.toText(data) -> string?` — `none` when `data` is not well-formed UTF-8 ([15 §Bytes Module](../04%20language/15-standard-library.md#bytes-module)) |
| Concatenation | `bytes + bytes -> bytes` (via existing `+` operator lowering) |
| Equality | `bytes == bytes -> boolean` |

The surface names and shapes are chapter 15's — the stdlib chapter owns every module surface, and this table is the compiler-side contract against it. An earlier draft of this table spelled the string conversions `string.to_bytes(encoding)` / `bytes.to_string(encoding) -> result<…>`; those forms are withdrawn: the only encoding is UTF-8, the fallible direction returns an optional, and there is no encoding parameter ([ADR-0021](../01%20governance/decisions/0021-time-and-bytes-namespaces.md), [15 §Bytes Module](../04%20language/15-standard-library.md#bytes-module)).

- **HIR** ([pass 7](#144-pipeline--sequential-passes)) — `bytes` lowers to the same representation as `list<u8>` for memory-layout purposes; the distinction between `bytes` and `list<integer>` is preserved at the type level to prevent silent conversions.
- **Codegen** ([pass 10](#144-pipeline--sequential-passes)) — at the WIT boundary, `bytes` maps to `list<u8>`. This is what makes every WASI interface that speaks in `list<u8>` (HTTP bodies, file contents, crypto payloads, database blobs) directly consumable from Clean without stdlib helpers.

Without this type, every developer touching binary data must either round-trip through `string` (unsafe for non-UTF-8) or fall back to per-library helpers that hide the byte layer. The type unlocks composable byte-piping (`crypto.hash(file.read(path))`) that libraries otherwise have to enumerate combinatorially.

### 14.14.3 Watch-mode recompilation (behind `cln dev`)

The developer loop behind `cln dev` combines source watching, fast rebuild, and live component swap. The compiler contributes only the rebuild half; hot component swap is a host capability whose home is [hosts/01-server §1.9 — Reload and Hot-Swap](../02%20components/hosts/clean-server/01-server.md).

Compiler-side contract for v1:

- Watching is the caller's loop, not a compiler mode. `[dev] watch = true` and `[dev] watch-exclude = […]` ([§07](./07-build-config.md#72-schema--top-level)) configure Clean Framework, which reads `clean.toml`, watches the project tree, and lowers a fresh request document ([§14.1.1](#1411-inputs)) for every change. The request schema carries no `[dev]` projection, and the compiler watches nothing ([CMP-01](#cmp-01--the-request-document-is-self-contained-the-compiler-touches-nothing-else)): there is no watch API and no persistent watch state. A warm compiler process serving successive rebuilds is observationally stateless — cycling back to an earlier request reproduces its earlier bytes.
- Rebuild latency target: **under 500 ms for a single-file change on a medium project** (per [§14.9](#149-performance-targets) definitions) *(informative target — see §14.9)*.
- A watch-mode rebuild produces the same `component.wasm` bytes as a full `debug` build — hot-reload is a deployment mechanism, not a compilation mode.

### 14.14.4 Diagnostics-only build (behind `cln check`)

The check operation runs [passes 1 through 9](#144-pipeline--sequential-passes) — every pass except codegen and component assembly — and emits `diagnostics.json`. It does not emit `component.wasm`.

- Pass [9] is the World Import Check, so a check request carries `target_world` exactly as a build request does ([§14.1.1](#1411-inputs)) and is refused with `RQD002` without it. There is no reduced request shape for checking: an IDE that type-checks against no world would go quiet on precisely the mistakes — calling a server function from a browser target — that the check exists to surface early, and `cln check` passing where `cln build` fails would make the fast path untrustworthy. The caller already holds the contract, since the framework fetches it at Moment 1 before either operation.
- Latency target: **an order of magnitude faster than a full `debug` build** on the same input. Concretely, a medium project (< 20k LOC, < 10 libraries) checks in under 300 ms cold *(informative target — see §14.9)*.
- This operation is what IDE integrations, pre-commit hooks, and CI's "does it typecheck?" gate should invoke; the full build is reserved for producing artifacts.
- Callers may request `diagnostics.json` on stdout for programmatic consumption; the default writes it to the output directory.

This mirrors `cargo check` in Rust. It exists because most of the compiler's runtime cost lives in codegen (pass 10); an IDE that runs a full build on every keystroke wastes 90% of the work. The check operation is the fast path.

### 14.14.5 Library-author testing — bridge stub components

Every `clean:bridge/*` interface ships alongside a **stub component** in the compiler distribution. Stubs implement the interface with recorded-call semantics and canned responses driven by a test fixture file.

- Location: `dist/bridges/stubs/clean-bridge-<interface>-stub.wasm`, shipped in the compiler tarball.
- Semantics: every host call is recorded to an in-memory log; responses come from a JSON fixture handed to the stub at instantiation. Non-fixture calls fail loudly.
- Consumers: library authors compose their library against the stubs to run tests without a full `clean-server`. `cln test` (see [§11 Testing](../04%20language/11-testing.md)) knows about stubs and wires them up automatically for the target world.

Stubs are not optional. Every `clean:bridge/*` interface has one, and it lives in the same repository as the interface WIT so drift is impossible. A new bridge function without a stub update is a red conformance test.

The v1 payoff is concrete: library authors get a genuine test loop without needing a running server, a database, or a bridge implementation. Third-party library authoring becomes possible for people who don't want to operate a full host.

### 14.14.6 Build reproduction and request replay (behind `cln repro build`)

The determinism invariant ([§14.5](#145-determinism-and-reproducibility)) exists at compile time. The reproduction and replay operations expose it to developers as a debugging tool; their user surface is the `cln repro` family owned by [Manager §00.3](../02%20components/manager/00-manager.md#003-command-surface) (the byte-identical rebuild is surfaced as `cln repro build`, distinct from the manager's bug-bundle `cln repro`).

**Build reproduction** (behind `cln repro build <build-manifest.json>`):

Reads a build manifest, obtains every input at its recorded SHA-256, invokes `compile()` with the identical request, and asserts the output `wasm_sha256` matches the manifest's. On match, writes the exact `component.wasm` that was originally shipped. On mismatch, reports the first byte of divergence — this is a compiler bug (`COM013`) or a manifest corruption.

Prerequisite: sources and library manifests must be reachable by their recorded hash. In v1 this means the caller (CI, Clean Manager) provides a resolver — the compiler does not embed a package manager. The resolver contract:

```rust
pub trait InputResolver {
    fn fetch_source(&self, path: &str, sha256: &str) -> Result<String>;
    fn fetch_library(&self, name: &str, version: &str, sha256: &str) -> Result<LibraryManifest>;
}
```

**Request replay:**

`clean-server` optionally captures per-request non-determinism into a request-trace JSON: the RNG seed handed to `wasi:random/random`, the values returned by `wasi:clocks/wall-clock`, the results returned by every `clean:bridge/*` call the request made. This capture is gated by `[dev] capture-traces = true` (schema home: [§07 — Build Configuration](./07-build-config.md)) and is off by default.

The replay operation reads a trace, instantiates the recorded component version, wires the runtime to a **replay host** that returns the captured values instead of live ones, and re-runs the request. The captured response must match the original response byte-for-byte. Divergence is a Clean runtime bug or a trace corruption — never a "close enough" outcome.

Combined, the two operations close the loop from "a customer complained about behavior at 14:22 on Tuesday" to "I am stepping through the exact `.wasm` that produced that response, with every non-deterministic input frozen to what actually happened." This is a debugging capability most language runtimes cannot deliver at all, because their compilers and runtimes are not deterministic. Clean's is; the `cln repro` family is the surface that makes it useful.

Compiler-side contract for v1:

- The reproduction operation is reached through Clean Manager and implemented by the compiler. The resolver is pluggable (default resolver reads from the local `~/.cln/` cache).
- The replay host is a small component that composes with the compiler's sandbox runtime (reference layout: [ADR-0006](../01%20governance/decisions/0006-compiler-reference-stack.md)).
- Request-trace format is JSON, versioned by `spec_version` mirroring [§14.1.1](#1411-inputs), and defined in the clean-server spec (compiler ships the schema for validation but does not define capture semantics).

---

## 14.15 External Build Cache


An external, opt-in **build cache** short-circuits `compile()` calls whose inputs are identical to a prior invocation whose outputs were persisted. It exists to make the cold-compile path in [§14.9](#149-performance-targets) fast on unchanged projects — the common case during development, CI, and comita STEP 6 rebuilds — without introducing intra-compiler state, incremental passes, or nondeterminism.

The cache lives at the **framework layer**, not inside the compiler. The compiler's `compile()` contract in [§14.2.1](#1421-library-api--the-canonical-entry-point) and its self-containment invariant in [CMP-01](#cmp-01--the-request-document-is-self-contained-the-compiler-touches-nothing-else) are unchanged. Framework computes a request hash before calling `compile()`, looks up prior outputs, and — on hit — returns them directly without invoking the compiler at all.

### CMP-06 — A cache hit MUST be byte-identical to a cache miss


For any request document `R`:

```
cache_hit(R).artifacts == compile(R).artifacts   (byte-identical, all three: component.wasm, build-manifest.json, diagnostics.json)
```

If this invariant is ever violated, the cache is considered poisoned and MUST be evicted. The determinism suite of [§14.7](#147-testing-strategy) is extended with a cache-parity test: compile twice, cache-hit twice, assert all four artifact tuples hash-equal.

This is the guardrail that lets [CMP-02](#cmp-02--same-request-in-byte-identical-outputs-out) coexist with caching — the cache does not weaken determinism, it exploits it.

### 14.15.1 Key structure


The cache key is the SHA-256 of a canonically-serialized tuple:

```
key = sha256(
    canonical_json({
        "spec_version":     R.spec_version,
        "compiler_version": exact compiler binary version (as recorded in the manifest),
        "compiler_hash":    sha256 of the compiler binary,
        "request":          canonical_json(R),      // includes sources, world WIT, block-handler manifest, compile_limits, target
        "environment":      canonical_json({SOURCE_DATE_EPOCH: <value or "unset">}),
    })
)
```

Canonical JSON: keys sorted lexicographically, no insignificant whitespace, no trailing newline, `\uNNNN` escapes for non-ASCII. This mirrors the manifest-canonicalization discipline already used by `cln repro build` ([§14.14.6](#14146-build-reproduction-and-request-replay-behind-cln-repro-build)).

**Rationale for each key component:**

- `compiler_version` + `compiler_hash` — a bit-level compiler change invalidates every prior entry. Version alone is not enough (dev builds share versions).
- `request` (in full) — captures every source byte, the world WIT, block-handler source hashes, and compile limits. Any material input change misses the cache.
- `environment.SOURCE_DATE_EPOCH` — the sole environment variable the compiler observes for determinism ([§14.5](#145-determinism-and-reproducibility)).

Nothing else enters the key. No wall clock, no absolute paths, no host name. If the compiler is deterministic (CMP-02), this key is sufficient.

### 14.15.2 Location and layout


The cache lives under `~/.cln/build-cache/` — inside the on-disk envelope declared by [ADR-0022 §7](../01%20governance/decisions/0022-foundational-technology-stack.md), so it does not introduce a new user-visible directory.

Layout:

```
~/.cln/build-cache/
├── index.json                    # LRU metadata, entries indexed by key
└── entries/
    └── <key[0:2]>/<key>/
        ├── component.wasm        # exactly the bytes compile() emitted
        ├── build-manifest.json
        └── diagnostics.json
```

Framework MUST NOT store partial outputs (e.g. only `component.wasm` without the manifest) — an entry is written atomically after all three artifacts are produced, or not at all.

### 14.15.3 Opt-in and configuration


Caching is **off by default**. Enable per project via `clean.toml`:

```toml
[dev]
cache-builds = true         # Default: false. When true, framework consults ~/.cln/build-cache/ before invoking the compiler.
```

CI can force-disable with the environment variable `CLN_NO_CACHE=1`. Framework MUST honor it even when `cache-builds = true`.

Cache size is capped:

```toml
[dev]
cache-builds        = true
cache-max-bytes     = "2G"      # Default: 2G. LRU eviction when exceeded.
cache-max-entries   = 500       # Default: 500. LRU eviction when exceeded.
```

Both caps are advisory ceilings, not budgets — the eviction sweep runs after each successful write, not before.

### 14.15.4 What the cache does not do


- **Not a per-pass cache.** If parsing succeeds but typechecking fails, the parse output is not cached — the entry-level unit is the whole `compile()` call. Per-pass caching is [§14.12](#1412-deferred-refinements) territory.
- **Not shared across machines.** No content-addressed remote store, no S3 backend. A machine warms its own cache.
- **Not aware of source-file mtimes.** Filesystem mtime is a source of nondeterminism (touching a file without changing content). The cache keys on content hash only.
- **Not compiler-owned.** The compiler binary knows nothing about the cache. Framework can be swapped for one that caches differently, or not at all, without touching the compiler.
- **Not authoritative for reproducibility.** `cln repro build` (§14.14.6) MUST bypass the cache and drive the compiler directly. Reproducibility proofs come from the compiler, not from an artifact fetched from disk.

### 14.15.5 Failure modes


- **Cache miss:** framework invokes `compile()` as it does today. Zero behavioral difference.
- **Corrupted entry:** if any of the three artifacts fails to load or fails hash verification against `entries/<key>/`'s canonical bytes, framework evicts the entry and falls back to a cache miss. No user-visible error.
- **I/O failure on the cache directory:** framework logs a warning and falls back to a cache miss. Compilation MUST succeed as long as the compiler itself can run.
- **Poisoned entry (CMP-06 violated):** the cache-parity test in [§14.7](#147-testing-strategy) is the guard rail. If it ever fires in a shipped compiler, the fix is a compiler bug (`COM013`), and framework flushes the entire cache on next start.

### 14.15.6 Observability


Framework surfaces cache activity through the diagnostic sink already used by the compiler:

- `cln build --verbose` prints one line per compile: `cache hit key=abc123... 45ms` or `cache miss key=abc123... 8.2s`.
- The build manifest gains an optional `cache_hit: bool` field (framework-populated, compiler ignores it).
- No new metrics interface is introduced.

---

## Changelog

- 2026-08-20 — §14.1.1's `compile_limits` example gains `"max_nesting_depth": 256`, mirroring the new [07 §7.8](./07-build-config.md#78-compile-time-limits) limit on structural nesting (from the compiler's Milestone 9, `clean-language-compiler/docs/DISCOVERIES-M9.md` §2, via [work/2026-08-20-structural-nesting-limit.md](../work/2026-08-20-structural-nesting-limit.md)). Counting rule and enforcement point: [10 §BLD001](./10-semantic-rules.md#bld001--build-limit-exceeded). The limit is what keeps deep-but-legal input from turning into a stack-overflow abort — an outcome neither [CMP-04](#cmp-04--internal-failures-are-com013-never-a-user-error) nor [CMP-05](#cmp-05--outputs-are-all-or-nothing-and-land-only-where-the-caller-pointed) can absorb, since both assume the process can still answer.
- 2026-08-19 — Erratum from the compiler's Milestone 8 (`clean-language-compiler/docs/DISCOVERIES-M8.md`, item 3). **§14.14.3**: the first compiler-side bullet said watch-mode rebuilds "respect `[dev] watch = true` and `[dev] watch-exclude = […]` from §07" — but the [§14.1.1](#1411-inputs) request schema carries no `[dev]` projection, and watching files is filesystem discovery [CMP-01](#cmp-01--the-request-document-is-self-contained-the-compiler-touches-nothing-else) forbids: both halves of that sentence describe the caller. The bullet now attributes watching and the `[dev]` settings to Clean Framework (which lowers a fresh request document per change) and states the compiler-side obligations that were only implicit — no watch API, no persistent watch state, a warm rebuild loop observationally stateless (cycling back to an earlier request reproduces its earlier bytes). The latency-target and byte-identity bullets are unchanged. Ratifies the compiler's M8 adoption (pinned by its `watch_rebuild.rs` contract: warm rebuild ≡ cold full build, stateless loop).
- 2026-08-19 — Two errata from the compiler's Milestone 6 (`clean-language-compiler/docs/DISCOVERIES-M6.md`, items 6i, 6l). **§14.14.2**: the bytes table's string conversions realigned to chapter 15's surface — `bytes.fromText(text) -> bytes` / `bytes.toText(data) -> string?`, no encoding parameter, optional not `result<…>` (the snake_case `to_bytes`/`to_string(encoding)` rows predated [ADR-0021](../01%20governance/decisions/0021-time-and-bytes-namespaces.md) and contradicted the stdlib chapter that owns the surface); index out-of-range named as [`RUN013`](./09-error-codes.md#312-runtime-codes-run) instead of "panics"; `slice` clamping cross-referenced to 15. **§14.1.1**: the `target_world.wit` bullet states the multi-package document form (root package unbraced, auxiliary packages braced) so composed-bridge interfaces resolve their qualified package from the one delivered document — the request schema had never said how more than one package travels. Ratifies the compiler's M6 adoptions.
- 2026-08-18 — §14.4.2 passes [5]/[6] now state what the "TypedAST′" arrow only implied, from the compiler's Milestone 5 (`clean-language-compiler/docs/DISCOVERIES-M5.md`, item 12): when the program declares library blocks, pass [5]'s findings are provisional (the pass still runs; its `TypedAST` is pass [6]'s input) and pass [6]'s re-validation of the expanded program is authoritative — handlers emit the companions the user's code already refers to, so a hard-failing pass [5] would make every block-using program uncompilable. Re-validation errors anchored inside an expanded block are [`BLOCK004`](./09-error-codes.md#315-block-handler-codes-block); errors anchored in user code keep their own codes and spans. The compiler's M5 adoption is ratified verbatim. Pass order, pass inputs, and diagnostics surface unchanged.
- 2026-08-11 — §14.1.1 gains **`target_world`**, a required top-level object carrying the target host contract by value ([ADR-0033](../01%20governance/decisions/0033-target-world-in-compilation-request.md), Accepted). Closes a gap that made two Accepted rules unimplementable: [CMP-01](#cmp-01--the-request-document-is-self-contained-the-compiler-touches-nothing-else) names "target world WIT" among the inputs the compiler must take from the request and forbids fetching it, [CMP-03](#cmp-03--every-import-is-verified-against-the-world-in-the-request) requires validating every call site against it — and the schema carried no field for one, so pass [9] could not be written and `COM012` could not fire. Five required string fields: `wit` (the host's `host.wit` verbatim, not an extract), `world` (which world inside it to check against — a `host.wit` may declare several, and resolving that from `build.target` compiler-side would make the compiler binary the authority on what a host provides, contra [BVER-03](./08-bridge-versioning.md#84-host-declaration)), `host`, `version` (resolved, never the `clean.toml` constraint — a constraint would break [CMP-02](#cmp-02--same-request-in-byte-identical-outputs-out)), and `sha256` (the lockfile pin). Populated by Clean Framework from the `host.wit` it already fetches at Moment 1; the compiler still fetches nothing. Pass [9] and CMP-03 now name the field instead of saying "as delivered in the compilation request". `spec_version` stays `"1"` — no consumer exists to break. Also editorial: the schema-violation bullet named only unknown keys, though [RQD002](./10-semantic-rules.md#rqd002--request-schema-violation) as owned also covers missing required fields and malformed values; the bullet now matches the rule.
- 2026-08-05 — Added §14.15 **External Build Cache**: opt-in, framework-layer, whole-artifact cache under `~/.cln/build-cache/` keyed by `sha256(spec_version, compiler_version, compiler_hash, request, environment)`. New rule **[CMP-06]** — a cache hit MUST be byte-identical to a cache miss (guards [CMP-02](#cmp-02--same-request-in-byte-identical-outputs-out)). Enabled per project via `[dev] cache-builds = true` (default off); CI opt-out via `CLN_NO_CACHE=1`. `cln repro build` bypasses the cache. Non-goals list in §14.15.4 keeps this narrower than the query-graph incremental compilation in §14.12. §14.11 non-goal for "incremental compilation" retitled to "*inside* the compiler" with a pointer to §14.15; §14.12 deferred-refinements #1 retitled to "query-graph incremental compilation" for the same reason. No changes to the compiler's contract, WIT, or existing rules.
- 2026-08-02 — Link repair: a citation of [07 §7.2](./07-build-config.md#72-schema--top-level) carried the anchor `#7-schema--top-level`, one digit short. No normative change.
- 2026-08-01 — Handler timeout and memory-limit breaches now cite [`BLOCK005`](./09-error-codes.md#315-block-handler-codes-block) rather than `BLD001`, whose registered scope is whole-build limits.
- 2026-08-01 — Technical-debt closure pass (lote X, approved 2026-08-01): pass [4] cycle-detection citation updated `IMPORT005` → [`IMPORT001`](./10-semantic-rules.md#import001--circular-dependency) — `IMPORT005` is withdrawn (folded into `IMPORT001`, [09 §3.8](./09-error-codes.md#38-import-codes-import)); the dangling bracket reference also gained its link target.
- 2026-08-02 — §14.1.1's "every `sources[].content` is UTF-8 text" was an unowned precondition: it asserted an encoding with no guarantor, no failure mode and no diagnostic, in a document whose own [CMP-01](#cmp-01--the-request-document-is-self-contained-the-compiler-touches-nothing-else) makes the compiler structurally incapable of checking it. It now cites [TXT-02](./17-text-encoding.md#txt-02--the-reader-validates-at-the-moment-it-reads) as the guarantor and states why the duty cannot sit here. No change to the compiler's observable contract.
- 2026-08-01 — Conflict-log remediation pass (P3, P4, 0.4, P16; work/2026-08-01-conflict-log-platform.md). §14.3 crate layout removed in favor of the observable contract plus [ADR-0006](../01%20governance/decisions/0006-compiler-reference-stack.md); §14.13 dependency pins replaced by a citation of ADR-0006, retaining only observable restrictions (determinism, no network/ambient filesystem, sandboxed handler execution, self-validation). §14.14 rewritten as the compiler's API operations invoked by framework/manager: the eight-subcommand user-facing table retired (user surface: [Manager §00.3](../02%20components/manager/00-manager.md#003-command-surface)); `cln new` (scaffolding = Framework, boundaries §2.4) and `cln add` (Manager built-in) removed; hot reload reduced to a citation of hosts/01-server §1.9; the byte-identical replay documented as the operation behind `cln repro build`. Diagnostic codes converted per the approved mapping: `REQUEST-INTEGRITY-001`→`RQD001`, `REQUEST-SCHEMA-001`→`RQD002`, `SEM-IMPORT-CYCLE`→`IMPORT005`, `HOST-IMPORT-NOT-IN-WORLD`→`COM012`, `CODEGEN-INTERNAL-INVARIANT`→`COM013`, `BUILD-LIMIT-EXCEEDED`→`BLD001`, `CONFIG-SCHEMA-*`→`CFG001`; §14.6 range table rewritten with the real 09 §1 ranges (SYN/SEM/IMPORT/COM/BLD/CFG/RQD). Pass [6] aligned with [ADR-0004](../01%20governance/decisions/0004-block-handler-execution-model.md); pass [9] aligned with the P8 scope-split (world WIT delivered in the request; compiler never fetches host WIT). `clean:library-data@1.4.0`→`clean:library/data@0.1.0` per 15 §0.3 + 08 §8.0. Agent-name fossil removed; "Clean Studio" mentions removed; deferral wording harmonized (all v1; §14.2.3 relabeled as internal build order); `[dev] capture-traces` now cites §07 as schema home.

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Compiler maintainers; framework and manager authors invoking the compiler API
- **Rule prefix:** `CMP-`
- **Part of:** [Clean Language Specification — Platform](./README.md)
- **References:** [Execution Layers](./01-execution-layers.md), [Semantic Rules](./10-semantic-rules.md), [Diagnostic Format](./13-diagnostic-format.md), [ADR-0006 — Compiler Reference Stack](../01%20governance/decisions/0006-compiler-reference-stack.md)
