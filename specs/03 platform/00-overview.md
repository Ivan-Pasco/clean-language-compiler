# Platform 00. Runtime Architecture Overview

This section is the one-page synthesis of how a Clean program runs. It stitches together the three architectural pillars — the layer model, the WebAssembly Component Model, and the bridge — that are defined in detail elsewhere. If you're new to the platform, read this first, then follow the links. This chapter states no rules of its own; every rule it mentions is owned by the chapter it links to.

---

## 1. The six execution layers

Clean code runs across six layers (L0–L5). Each layer has one job and calls only downward.

```
L5   Application            (user .cln files)
L4   Libraries              (auth, canvas, client, data, jobs, locale,
                             mcp, server, storage, ui — all Clean source;
                             plus the implicit `core` library)
L3   Host World             (server-only, browser-only, or CLI-only interfaces)
L2   Portable Host Bridge   (wasi:* + clean:bridge/*)
L1   WASM Runtime           (pure computation, memory intrinsics — no I/O)
L0   Compiler               (build-machine only; emits imports, never implementations)
```

L0 and L1 have no observable side effects; anything that touches the outside world lives at L2 or above — the rule and its full definition live in [1 — Execution Layers](./01-execution-layers.md).

---

## 2. The Component Model — the mechanism

V2 targets the **WebAssembly Component Model**, not raw core WASM. Every Clean program compiles to a **component** that fulfills a specific **WIT world**.

- **WIT** (Wasm Interface Types) is the text-format interface language. Rich types — `string`, `list<T>`, `record`, `variant`, `option`, `result<T, E>`, `resource` — cross the bridge with generated marshalling. No hand-written `(ptr, len)` pairs.
- **Package** — a namespaced group of interfaces, e.g. `clean:bridge@1.0.0`, `wasi:filesystem@0.3.0`, `clean:host@0.1.0`.
- **Interface** — a named group of typed functions and resources inside a package.
- **World** — a WIT declaration listing which interfaces a component **imports** and which it **exports**. A Clean program targets exactly one world.
- **Host** — any runtime that instantiates the component: `clean-server`, browsers (via `jco`), `clean-cli`, third-party embedders. A host is defined by the world it fulfills.

**Single-source-of-truth guarantee:** every bridge signature is declared exactly once, in a WIT file. Compiler expected-imports, host registered-exports, library manifests, and MCP tool schemas are all generated from those WIT files. Signature drift becomes a compile-time error, not a runtime bug. Full model in [15 — Component Model Architecture](15-component-model-architecture.md).

---

## 3. The three shipping worlds

A component targets exactly one world; the compiler rejects programs whose imports don't fit.

| World     | Imports (in addition to L2 portable bridge)                                                                                             | Exports                      |
| --------- | --------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- |
| `server`  | `clean:host/routing`, `clean:host/sse`, `clean:host/ws`, `clean:host/session`, `clean:host/jobs`, `clean:host/email`, `clean:host/i18n` | `wasi:http/service@0.3.0`    |
| `browser` | `clean:host/dom`, `clean:host/nav`, `clean:host/storage`, `clean:host/toast`                                                            | Browser-runtime entry point  |
| `cli`     | `clean:host/prompt` (interactive stdin)                                                                                                 | `wasi:cli/run`               |

World and interface naming is defined in [15 §0.3](15-component-model-architecture.md#03-wit-package-and-world-naming); the contents of each server-only interface are defined in WIT in [12 — Server Extensions §13](./12-server-extensions.md#13-world-declaration).

---

## 4. WASI vs `clean:bridge/*` — the composition rule

The bridge (L2) is composed from two families, and Clean deliberately does not smuggle Clean semantics into WASI:

**WASI packages** — used where the WASI shape fits Clean's needs:

| Interface                                               | Provides                                                        |
| ------------------------------------------------------- | --------------------------------------------------------------- |
| `wasi:filesystem/preopens`, `wasi:filesystem/types`     | File I/O against host-granted preopens only (no ambient access) |
| `wasi:cli/stdout`, `wasi:cli/stderr`                    | Raw byte streams                                                |
| `wasi:clocks/wall-clock`, `wasi:clocks/monotonic-clock` | Time                                                            |
| `wasi:random/random`                                    | Cryptographic RNG                                               |
| `wasi:http/handler@0.3.0`                               | Outbound HTTP                                                   |
| `wasi:sockets/tcp`, `wasi:sockets/udp`                  | Sockets, where the host grants them                             |

**`clean:bridge/*` packages** — declared alongside WASI when the WASI shape doesn't fit:

| Interface              | Provides                                                        |
| ---------------------- | --------------------------------------------------------------- |
| `clean:bridge/console` | Typed `print`, `log` (calls through to WASI stdio)              |
| `clean:bridge/db`      | `query`, `execute`, transactions, migrations                    |
| `clean:bridge/crypto`  | Argon2id, SHA-2, HMAC, JWT sign/verify/decode                   |
| `clean:bridge/math`    | Trig, log, `pow` (the `^` operator's implementation)            |
| `clean:bridge/string`  | UTF-8 heavyweight ops (case folding, normalization)             |
| `clean:bridge/mem`     | Arena management — see [3 — Memory Model](./03-memory-model.md) |

The composition rule — WASI where its shape fits, a `clean:bridge/*` interface *alongside* WASI (never replacing it) where it doesn't — is defined in [2 — Host Bridge §2.1](./02-host-bridge.md#21-what-extending-wasi-means); the full catalog is [2 §2.2.1](./02-host-bridge.md#221-portable-l2-in-every-world).

---

## 5. End-to-end: compile → instantiate → run

```
clean.toml + .cln sources  ─────►  Clean Framework
                        │
                        │ 1. Read clean.toml; resolve libraries (L4)
                        │    and their versions
                        │ 2. Compile each library's block handlers to
                        │    WASM at install time and cache them
                        │ 3. Assemble the compilation request document —
                        │    sources inline, no filesystem discovery
                        ▼
                   Compiler (L0)   receives the request document
                        │          (14 §14.1.1 — never reads clean.toml)
                        │ 4. Parse the inline .cln sources
                        │ 5. Execute block handlers in the sandboxed
                        │    compile-time runtime — typed BlockAST in,
                        │    typed IR out (blocks like endpoints:, data:,
                        │    component: expand into ordinary Clean IR)
                        │ 6. Type-check every host-function call against
                        │    the target world's WIT from the request
                        │ 7. Reject any import not in the world
                        │ 8. Emit a .wasm component
                        ▼
                   program.wasm    (component conforming to a world)
                        │
                        │ Host instantiates via wasmtime / jco / embedder
                        │ Host hands the component only the interfaces it
                        │ declared — no ambient authority
                        ▼
                   Running component
                        │
                        │ Component calls typed bridge functions:
                        │   L5 app  → L4 library helpers
                        │           → L3 host-world extensions
                        │           → L2 portable bridge
                        │           → L1 WASM intrinsics
                        ▼
                   Host executes the bridge call and returns typed result
```

The split of responsibilities in this pipeline — the framework reads `clean.toml`, resolves libraries, and compiles/caches handlers; the compiler receives a self-contained request document and executes handlers in its sandbox — is decided in [ADR-0004 — Block Handler Execution Model](../01%20governance/decisions/0004-block-handler-execution-model.md) and specified in [14 §14.1.1](./14-compiler-architecture.md#1411-inputs).

At runtime, the component is **capability-isolated**: a component that did not import `wasi:filesystem` cannot open files, period. This is a property of the Component Model, not a Clean-specific check.

---

## 6. Libraries — Layer 4 in depth

There are **10 shipped libraries** — `auth`, `canvas`, `client`, `data`, `jobs`, `locale`, `mcp`, `server`, `storage`, `ui` — plus the implicit `core` library that provides the language surface (primitive types, stdlib namespaces like `math.*`, `string.*`, `list.*`, `time.*`, `json.*`, `http.*`, `console`).

Each shipped library:

1. **Declares blocks** the compiler otherwise wouldn't recognize (`endpoints:`, `data <T>Data:`, `component:`, `canvas:`, etc.) via a `handles block` declaration in library source ([21 §21.1](../04%20language/21-block-handlers.md#211-declaring-a-block-handler), [LBS §3.2](../02%20components/framework/09-libraries-specification.md#32-compile-time-functions)).
2. **Contributes `compiletime` functions** (block handlers) written and distributed as Clean source. The framework compiles them to WASM at install time and caches them; the compiler executes them in its sandboxed compile-time runtime during compilation — they receive a typed `BlockAST` and return typed IR ([ADR-0004](../01%20governance/decisions/0004-block-handler-execution-model.md)).
3. **Declares typed host imports** in its `host_bridge.cln` as `host function <name>(...) returns <type>` per [LBS-02](../02%20components/framework/09-libraries-specification.md#lbs-02--host-bridge-declaration-grammar). No `from "..."` clause, no hand-written `result<>`.
4. **Never ships a WIT file.** The framework synthesizes the library's WIT from its `host function` declarations ([LBS §8.1](../02%20components/framework/09-libraries-specification.md#81-the-model)) — the bridge surface is generated, not hand-transcribed (C-07: no hand-written WIT).

Capabilities on classes remain **pure contracts** — signatures only, no default method bodies. Infrastructure capabilities like `Persist`, `Cacheable`, `Auditable` sit on companion types (`UserData`, not `User`), reached through the companion-access rule in [14 — Classes and Objects](../04%20language/14-classes-and-objects.md). Full library model in [09 — Libraries Specification](../02%20components/framework/09-libraries-specification.md).

---

## 7. What each layer knows, in one line

| Layer              | Knows about                                                                  |
| ------------------ | ---------------------------------------------------------------------------- |
| L0 Compiler        | Types, syntax, block handlers, WIT worlds. No I/O.                           |
| L1 WASM Runtime    | Arithmetic, control flow, linear memory. No imports.                         |
| L2 Portable Bridge | Typed portable I/O. Runs in every world.                                     |
| L3 Host World      | HTTP routing, DOM patching, interactive prompts. Only in the matching world. |
| L4 Libraries       | Domain vocabulary. Written in Clean, run at compile time to expand blocks.   |
| L5 Application     | Your `.cln` files.                                                           |

---

## 8. Where to go next

- **The layer model in detail** → [1 — Execution Layers](./01-execution-layers.md)
- **The full bridge and world catalog** → [2 — Host Bridge](./02-host-bridge.md)
- **Memory layout under all of this** → [3 — Memory Model](./03-memory-model.md)
- **Component Model, WIT, versioning, security** → [15 — Component Model Architecture](15-component-model-architecture.md)
- **How libraries extend the compiler** → [21 — Block Handlers](../04%20language/21-block-handlers.md) + [09 — Libraries Specification](../02%20components/framework/09-libraries-specification.md)
- **Server-side WIT interfaces** → [12 — Server Extensions](./12-server-extensions.md)
- **Diagnostic and error taxonomy** → [9 — Error Codes](./09-error-codes.md) + [10 — Semantic Rules](./10-semantic-rules.md) + [13 — Diagnostic Format](./13-diagnostic-format.md)

---

[Index](./README.md) | [Next: Execution Layers](./01-execution-layers.md) →

---

## Changelog

- 2026-08-05 — §3 world table and §4 WASI-interface table: two leftover WASI 0.2 names (`wasi:http/incoming-handler`, `wasi:http/outgoing-handler`) updated to their WASI 0.3 replacements (`wasi:http/service@0.3.0`, `wasi:http/handler@0.3.0`) — Preview 3 sweep debt from [ADR-0022 §3](../01%20governance/decisions/0022-foundational-technology-stack.md) that the normative [Platform 12 server world](./12-server-extensions.md#13-world-declaration) already reflects.
- 2026-08-01 — Conflict-log remediation (Fase 3): §5 pipeline rewritten per [ADR-0004](../01%20governance/decisions/0004-block-handler-execution-model.md) and 14 §14.1.1 — the framework reads `clean.toml`, resolves libraries and compiles/caches handlers; the compiler receives the request document and executes handlers in its sandbox. §6 aligned to LBS §3.2 (`handles block` lives in library source, not `library.toml`), LBS-02 (host-function grammar without `from`), and LBS §8.1/C-07 (the framework synthesizes WIT; libraries never ship one). "Five layers" corrected to six (L0–L5); world names normalized to the canonical bare form of 15 §0.3 (resolution 0.1); inline "**Rule:**" restatements of 01 and 02 §2.1 downgraded to citations of their homes; §3 world-declaration pointer redirected to 12 §13 and 15 §0.3.

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Anyone new to the Clean runtime architecture
- **References:** [1 — Execution Layers](./01-execution-layers.md), [15 — Component Model Architecture](./15-component-model-architecture.md), [2 — Host Bridge](./02-host-bridge.md)
