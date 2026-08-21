# Platform 01. Execution Layers

Every function in a Clean program executes on exactly one of six layers, from the compiler that emits the WASM component down to the application code that runs inside it. This chapter fixes those layers and the boundaries between them — it is the portability contract that lets one `.wasm` component run on any host that provides the layers the program depends on, with no code changes. The layer model answers two questions authoritatively: when you write `print(x)`, whose code prints it, and when you add a dependency on `data`, what does the runtime need to provide.

---

## 1.1 The Layer Model

Clean has **six execution layers** (L0–L5). Each layer has a fixed responsibility and a strict boundary with the layer above and below it.

| Layer | Name | Runs where | Owns |
|-------|------|-----------|------|
| **L0** | Compiler | Build machine | Parsing, type checking, IR, WASM component emission. Emits imports, never implementations. |
| **L1** | WASM Runtime | Guest sandbox | Pure computation: arithmetic, control flow, memory ops. Every host provides this. |
| **L2** | Host Bridge | Host process | Portable I/O available to every Clean program on every host: console, math intrinsics, string ops, file I/O, HTTP client, database, crypto, memory arena. |
| **L3** | Host World | Host process | Host-specific capabilities beyond the portable set: HTTP server routing (`server` world), DOM patching (`browser` world), interactive prompts (`cli` world). |
| **L4** | Libraries | Guest sandbox | Compile-time expansion of blocks (`data:`, `endpoints:`, `component:`) and runtime helpers. Libraries are Clean source, resolved through dependency resolution, and run at compile time via [block handlers](../04%20language/21-block-handlers.md). |
| **L5** | Application | Guest sandbox | The user's Clean program. Uses L1–L4, cannot reach the host directly except through declared imports. |

### 1.1.1 What each layer MAY call

| Layer | May call |
|-------|----------|
| **L0** (Compiler) | Itself only. Never runs at execution time. |
| **L1** (WASM Runtime) | Itself only. Pure computation, no imports. |
| **L2** (Host Bridge) | L1 (uses runtime intrinsics for arithmetic). Provided by the host, not called from the host — always called *by* the guest. |
| **L3** (Host World) | L1, L2. Provided by the host. |
| **L4** (Libraries) | L1, L2, L3 (only via declared `host function`s the library imports). L4 code runs either at compile time (block handlers, IR builders — see [§1.2](#12-the-boundary-between-compile-time-and-runtime)) or at runtime (helpers linked into the guest). |
| **L5** (Application) | L1, L2, L3, L4. |

### 1.1.2 What each layer MUST NOT do

### LAY-01 — Layer boundary prohibitions

- **L0** MUST NOT embed a WASM runtime *except* the sandboxed compile-time runtime in which block handlers execute ([ADR-0004](../01%20governance/decisions/0004-block-handler-execution-model.md)). There is no "run at compile time" that reaches out to the host: in the sandbox, all host imports are stubbed to error — see [§21.7](../04%20language/21-block-handlers.md#217-compile-time-execution-environment).
- **L1** MUST NOT access memory outside the guest sandbox. This is enforced by the WASM runtime (reference engine: [ADR-0002](../01%20governance/decisions/0002-clean-server-reference-stack.md)), not by the language.
- **L2** MUST NOT depend on which host is running. If a function is not available on **every** host that ships Clean, it belongs in L3, not L2.
- **L3** MUST NOT smuggle Clean-specific behavior into WASI packages. Clean-specific interfaces live under `clean:host/*` packages, never inside `wasi:*` packages.
- **L4** MUST NOT implement host functions. A library declares host functions with `host function` (see [LBS §8](../02%20components/framework/09-libraries-specification.md#8-host-bridge-as-typed-host-function-declarations)); the host provides them. A library that ships a Rust or JavaScript implementation of its host function is a bug.
- **L5** MUST NOT call L0 tooling from within a running program. Compile-time and runtime are disjoint.

---

## 1.2 The Boundary Between Compile Time and Runtime

### LAY-02 — Exactly two execution eras

Clean has exactly two execution eras:

1. **Compile time** — L0 runs. Libraries (L4) contribute block handlers that transform the user's source into IR. The host has no involvement. `now()`, `random()`, `file.read()`, and every `host function` call are unavailable — they raise [`BLOCK006`](./09-error-codes.md#315-block-handler-codes-block) if called from a `compiletime` context (see [§21.7](../04%20language/21-block-handlers.md#217-compile-time-execution-environment)).
2. **Runtime** — L1–L5 run in a WASM runtime instance (reference engine: wasmtime, per [ADR-0002](../01%20governance/decisions/0002-clean-server-reference-stack.md)). The compiler is not present.

There is no third era. Implementations MUST NOT introduce a "load-time initialization" that runs partly in the compiler and partly in the runtime. What the compiler emits is what the runtime executes.

### 1.2.1 Why this matters

Every deployment target — clean-server, browser via jco, `clean-cli`, embedded — instantiates the same `.wasm` component. The layer model is what makes that possible. If a function's layer is wrong (e.g. a compiler that hardcodes an HTTP client instead of importing one), the same binary stops working on the target where that shortcut doesn't exist.

---

## 1.3 Reachability and Dead-Import Elision

### LAY-03 — Dead-import elision

The compiler MUST emit WASM component imports **only for the L2, L3, and L4 host functions reachable from the entry point**. Dead imports defeat the layer boundary — a program that imports `wasi:filesystem` but never opens a file grants itself filesystem authority for no reason (see [P7 in §15](15-component-model-architecture.md#3-architectural-principles)).

**Rules:**

1. Reachability is computed after tree-shaking, on the post-block-expansion IR.
2. Imports that are only reachable through a code path guarded by a compile-time-false condition (e.g. `if false: ...`) MUST be elided.
3. A small always-on set is exempt from reachability gating because runtime code paths depend on them even when the user program does not name them: `print`, the L1 memory intrinsics documented in [§03 Memory Model](./03-memory-model.md), and the string/list runtime helpers named in [§02 Host Bridge](./02-host-bridge.md#26-always-on-runtime-support).
4. Elision is a **security property**, not just an optimization. Removing dead imports is what makes least-privilege capability granting (see [§15 P7](15-component-model-architecture.md#3-architectural-principles)) meaningful.

---

## 1.4 Host Worlds (L3 Detail)

Every host implements exactly one **world** — a WIT declaration of the interfaces it provides. The compiler rejects programs whose imports cannot be satisfied by the target world.

| World | Host | Provides beyond L2 |
|-------|------|--------------------|
| `server` | clean-server | HTTP server routing, request context, SSE, WebSocket, sessions, JWT, job scheduling, email, i18n |
| `browser` | Browser (via jco) | DOM patching, event delegation, clipboard, focus, navigation, toasts, IndexedDB |
| `cli` | clean-cli | Interactive stdin (`input`, `input-integer`, `input-yesno`, `input-range`) |

Adding a world is a spec change, not a config change. A new host category needs a spec section that lists which L3 interfaces it exposes and which it explicitly does not. See [§15.6](15-component-model-architecture.md) for world declaration mechanics.

---

## 1.5 What Belongs Where — Design Heuristics

These are design heuristics, not decidable rules: "small," "heavy," and "portable" carry no objective thresholds, so no checker can apply them mechanically ([DOC-13](../01%20governance/00-documentation-principles.md)). The final layer placement of a new function is decided in spec review; the decision becomes checkable only once the function is recorded in its layer's catalog (L2/L3: [02 §2.2 — Interface Catalog](./02-host-bridge.md#22-interface-catalog)).

When adding a new function to the platform:

1. **Is it pure computation** (arithmetic, string manipulation, sorting, hashing without keys)? → **L1** or **L2 stdlib helper**. If the algorithm is small and inlinable, L1. If the algorithm is heavy and portable, L2.
2. **Does it perform I/O that every host can do** (file, HTTP client, DB via connection string, console)? → **L2**.
3. **Does it require a specific host's environment** (HTTP request context, DOM, terminal stdin)? → **L3**, in the appropriate world.
4. **Is it a compile-time transformation of user syntax**? → **L4**, as a [block handler](../04%20language/21-block-handlers.md).
5. **Is it a runtime helper only relevant to programs that opt in** (an ORM query builder, a template renderer)? → **L4**, as ordinary library code compiled into the guest.
6. **Would putting it in L2 make L2 non-portable** (e.g. "run this shell command")? → **L3**, and only in worlds where it is meaningful.

If the answer is unclear, the function is placed in L3 by default and promoted to L2 later if it turns out to be portable. The reverse move (L2 → L3) is a breaking change and is avoided.

---

## 1.6 Enforcement

- The compiler's link stage verifies that every emitted import is present in the target world's WIT (see [§15.4](15-component-model-architecture.md)). Missing imports fail with [`COM010`](./09-error-codes.md#310-compilation-codes-com) (`BridgeLinkError`), naming the missing interface.
- Contract tests (see [§15.10](15-component-model-architecture.md)) verify each host exports every function its world declares, with matching WIT signatures. A host that fails conformance does not ship.
- Host-parity checking in CI is provided by `cln mcp`'s `strict_check` (which absorbed the earlier standalone parity script) — see the [Quality Playbook](../01%20governance/02-quality-playbook.md). It runs for every host that ships in-tree and blocks merge on drift.

---

## 1.7 Non-Goals

- **Layer-level access control at runtime.** The layer model is a design and review tool, enforced at the compile boundary. At runtime, capability restriction is done by wasmtime's Component Model instance-linking (see [§15 P7](15-component-model-architecture.md#3-architectural-principles)), not by inspecting call sites.
- **Layer-per-thread isolation.** Layers describe *what code does*, not *what thread it runs on*. Multi-threading is out of scope for V2.
- **Cross-layer polymorphism.** A function belongs to exactly one layer. If two hosts implement "the same" function differently (e.g. `print`), that function is an L2 function with two host implementations, not two functions.

---

## 1.8 Deferred Refinements

The following refinements are outside V2 scope. They do not block adoption of §01 and may be reconsidered if concrete demand appears.

1. **Layer 3 world composition.** A host implements exactly one world in V2. A single host binary providing both the `server` and `cli` worlds (e.g. embedded scripting alongside HTTP serving) is not supported.
2. **Layer 4 runtime split.** Libraries may contribute both compile-time handlers and runtime helpers. The `compiletime function` keyword on individual functions is sufficient; V2 does not introduce module-level `compiletime module` vs `runtime module` distinctions.

---

## Changelog

- 2026-08-01 — Conflict-log remediation (Fase 3): L0 rule harmonized with [ADR-0004](../01%20governance/decisions/0004-block-handler-execution-model.md) — L0 embeds no WASM runtime *except* the sandboxed compile-time runtime for block handlers; engine names (wasmtime) made informative via [ADR-0002](../01%20governance/decisions/0002-clean-server-reference-stack.md). "Five layers" corrected to six (L0–L5). Stale anchors to 21-block-handlers (`#24x`) repointed to the real 21.x headings; the LBS host-bridge cite fixed from "§09.9" to LBS §8. World names normalized to the bare canonical form of 15 §0.3 (resolution 0.1); `input_*` prompt functions renamed to kebab-case (resolution 0.3); "wasmtime CLI" unified to `wasmtime_runner` (since retired in favor of `clean-cli`, 2026-08-12); the nonexistent `check_host_parity.py` replaced by a cite to the Quality Playbook (`cln mcp` `strict_check`).

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Platform maintainers, host implementors, and library authors deciding where a new function belongs
- **Rule prefix:** `LAY-`
- **Part of:** [Clean Language Specification — Platform](./README.md)
- **References:** [Host Bridge](./02-host-bridge.md), [Component Model Architecture](./15-component-model-architecture.md), [Memory Model](./03-memory-model.md)
