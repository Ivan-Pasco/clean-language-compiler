# Platform 02. Host Bridge

The host bridge is the set of typed interfaces every Clean host provides to the guest component running inside it. This chapter defines the surface: how those interfaces are declared in WIT, how they extend WASI without replacing it, how libraries add typed `host function` declarations of their own, and what contract every bridge function must obey — from structured error variants to trap lifting when a bridge component itself faults. The rules here fix *what exists* and *what it must guarantee*; how the packages are versioned lives in [08 — Bridge Versioning](./08-bridge-versioning.md), how they compose into a world lives in [15 — Component Model Architecture](./15-component-model-architecture.md), and how mismatches between guest and host are detected lives in [16 — Host Contract Validation](./16-host-contract-validation.md).

The bridge is defined in **WIT** ([WebAssembly Interface Type](https://component-model.bytecodealliance.org/design/wit.html)) — never in prose, never in TOML, never in hand-written glue. Every bridge function lives in exactly one `.wit` file. The compiler and every host generate their code from those files.

---

## 2.1 What "Extending WASI" Means

Clean's bridge is built by **composing three tiers of WIT packages**:

1. **`wasi:*`** — standard WASI 0.3 packages (`wasi:cli`, `wasi:filesystem`, `wasi:http`, `wasi:clocks`, `wasi:random`, `wasi:sockets`, `wasi:logging`). Used verbatim. Clean does not modify or subclass WASI. `wasi:logging` remains pinned to `@0.2.0` upstream (no 0.3 cut has shipped); every other package is at `@0.3.0`. See [Platform 08 §8.0 — V2 Baseline Versions](./08-bridge-versioning.md#80-v2-baseline-versions).
2. **`clean:bridge/*`** — Clean-specific portable interfaces that either don't exist in WASI or need typed shapes WASI does not provide (`clean:bridge/console`, `clean:bridge/db`, `clean:bridge/crypto`, `clean:bridge/mem`).
3. **`clean:host/*`** — per-host worlds that combine the above with host-specific interfaces (the `server` world adds routing and sessions, the `browser` world adds DOM patching, the `cli` world adds interactive stdin).

### BRG-01 — WASI where it fits, alongside it where it doesn't, never replacing it


If a capability exists in WASI in a shape Clean can use, Clean MUST import the WASI interface directly. If WASI's shape does not fit, Clean MUST declare a `clean:bridge/*` interface *alongside* WASI, never *replacing* it. Clean-specific behavior MUST NOT be smuggled into a `wasi:*` package.

**Example — the console** *(informative)*:

WASI provides `wasi:cli/stdout` and `wasi:cli/stderr` as byte streams. Clean programs need typed console output (`print`; the `print(x) +` suffix is a syntactic form that emits a trailing newline — see [7 — Statements](../04%20language/07-statements.md)) that respects the language's string encoding. So Clean declares:

```wit
// package: clean:bridge@1.0.0
interface console {
    print: func(text: string);
    log: func(level: log-level, message: string);
}

variant log-level { info, warn, error, debug }
```

The trailing-newline form (`print(x) +`) is lowered by the compiler to a `print` call whose argument has `"\n"` appended; there is no separate bridge function for it.

`clean:bridge/console` calls **through** to `wasi:cli/stdout` in every reference host implementation. It does not replace WASI stdio; it types it.

---

## 2.2 Interface Catalog


The full WIT source of each interface lives in the **`wit/` directory at the root of this repository** — the contract is versioned with the specification that defines it (see [§08 Bridge Versioning](./08-bridge-versioning.md) for how the packages are versioned). This section enumerates what exists and where each interface lives on the layer map.

### 2.2.1 Portable (L2, in every world)

### BRG-02 — The portable L2 catalog is closed


The table below is the complete portable (L2) bridge surface, available in every world. An interface not listed here is not part of L2, and an interface MUST NOT be treated as portable until it has a row in this table. Adding an interface to this catalog is a spec change to this section (via ADR where the addition touches the world inventory — the pattern set by [ADR-0005](../01%20governance/decisions/0005-server-world-interface-additions.md), which added `clean:bridge/files`). Whether a given instance is *granted* a capability at instantiation (preopened directories, sockets) is a per-host decision noted in the table; the catalog fixes the surface, not the grants.

| WIT interface | Provides |
|---------------|----------|
| `wasi:cli/stdout`, `wasi:cli/stderr` | Raw byte streams. Backing for `clean:bridge/console`. |
| `wasi:filesystem/preopens`, `wasi:filesystem/types` | File I/O against host-granted preopens only (no ambient access). |
| `wasi:http/outgoing-handler` | Outbound HTTP requests. |
| `wasi:clocks/wall-clock`, `wasi:clocks/monotonic-clock` | Time. |
| `wasi:random/random` | Cryptographic RNG. |
| `wasi:sockets/tcp`, `wasi:sockets/udp` | Sockets, where the host grants them. |
| `clean:bridge/console` | Typed `print`, `log`. |
| `clean:bridge/db` | `query`, `execute`, transactions, migrations. Backed by whichever driver the host loads (reference drivers: SQLite, Postgres, MySQL — [ADR-0002](../01%20governance/decisions/0002-clean-server-reference-stack.md)). |
| `clean:bridge/crypto` | Password hashing (Argon2id), SHA-2, HMAC, JWT sign/verify/decode. |
| `clean:bridge/mem` | Arena management (`arena-push`, `arena-pop`) — see [§03 Memory Model](./03-memory-model.md). |
| `clean:bridge/math` | Deterministic transcendental functions (trig, log, pow) that the WASM spec does not provide as instructions. `pow` here is the host implementation of the `^` operator; it is not exposed as a language-level `math.pow` function (see [16 — Method-Style Syntax](../04%20language/16-method-style-syntax.md)). |
| `clean:bridge/string` | UTF-8 heavyweight ops (case folding, normalization, regex when opted in) that a WASM-only implementation would ship as bloat. |
| `clean:bridge/files` | Typed file operations above the raw WASI shape, against host-granted preopens only. Added by [ADR-0005](../01%20governance/decisions/0005-server-world-interface-additions.md). |

**Why format parsers (JSON, TOML, YAML, URL) are NOT on this list.** The size argument that puts case folding, Argon2, or regex on the bridge does not apply to format parsers. A parser has no host-only capability — it is pure computation over bytes — and routing it through a bridge means the accept/reject boundary depends on which host is running (V8 vs. `serde_json` vs. wasmtime's runner). The stdlib conformance strategy in [`../11-testing.md`](../04%20language/11-testing.md) §Conformance Testing for Standard-Library Parsers only closes if the parser is compiled to WASM once and behaves identically everywhere. Format parsers therefore live in the stdlib (see [`../15-standard-library.md`](../04%20language/15-standard-library.md) §JSON Module), not the bridge — even when a native parser would be smaller and faster.

### 2.2.2 Host-specific (L3)


| World | Adds |
|-------|------|
| `server` | `clean:host/routing` (HTTP routing, request context, response building), `clean:host/sse` (Server-Sent Events), `clean:host/ws` (WebSocket), `clean:host/session` (session storage), `clean:host/jobs` (background jobs, cron), `clean:host/email`, `clean:host/i18n` |
| `browser` | `clean:host/dom` (patch, event delegation, focus, clipboard), `clean:host/nav` (URL, history), `clean:host/storage` (localStorage, IndexedDB), `clean:host/toast` |
| `cli` | `clean:host/prompt` (`input`, `input-integer`, `input-yesno`, `input-range`) |

### 2.2.3 Extension by libraries (L4)


Libraries declare their host bridge surface as **typed `host function` declarations inside a `host interface` block**, written in Clean source. The full contract, grammar, and worked example live in [09 §8 — Host Bridge as Typed `host function` Declarations](../02%20components/framework/09-libraries-specification.md#8-host-bridge-as-typed-host-function-declarations). This section covers only how the platform consumes those declarations.

**Framework responsibility:** Clean Framework reads each library's `host_bridge.cln`, synthesizes the corresponding WIT interface in package `clean:library/<library-name>@<version>`, and caches the result under `~/.cln/wit-cache/` (the `~/.cln/` layout is owned by [Clean Manager §00.2](../02%20components/manager/00-manager.md#002-on-disk-layout)). The synthesized WIT is what the compiler and host see; library authors never write WIT by hand.

### BRG-03 — Hosts export the full synthesized library WIT


**Host responsibility:** every host that satisfies a world declared in a library's `requires host worlds` list MUST export every function in the synthesized WIT interface. Missing exports fail at link time with [`COM010`](./09-error-codes.md#310-compilation-codes-com) (`BridgeLinkError`) naming the interface — never at runtime. See [15 §0.3](15-component-model-architecture.md#03-wit-package-and-world-naming) for the canonical package/world naming and [16 — Host Contract Validation](./16-host-contract-validation.md) for how conformance is verified.

---

## 2.3 Bridge Function Contract


Every bridge function — WASI, `clean:bridge/*`, `clean:host/*`, or `clean:library/*` — obeys the same contract.

### 2.3.1 Typed values, not pointer pairs

Bridge functions cross with **WIT types**: `string`, `list<T>`, `record`, `variant`, `option<T>`, `result<T, E>`, `tuple<...>`, `resource`. The compiler emits component-level calls; wasmtime and `wit-bindgen` handle the ABI marshalling.

Raw `(ptr, len)` pointer pairs are never part of a bridge signature.

### 2.3.2 Errors

### BRG-04 — Errors are structured variants, never strings


Every fallible function MUST return `result<T, error>`, where `error` is a variant declared in the same interface. Error variants MUST carry structured payloads, not stringly-typed messages — a bridge function that returns `result<T, string>` is a defect. Errors must be structured so the guest can dispatch on the variant:

```wit
variant db-error {
    connection-failed(string),
    query-failed(query-failure),
    constraint-violation(constraint-info),
    timeout,
}

record query-failure {
    sql: string,
    driver-code: option<u32>,
    driver-message: string,
}
```

### 2.3.3 Resources

Long-lived host objects (open files, DB connections, HTTP streams, prepared statements) are exposed as **WIT resources**, not integer handles. Resources are automatically closed when the guest drops them; the host cannot leak them.

```wit
interface db {
    resource connection {
        prepare: func(sql: string) -> result<statement, db-error>;
        execute-one: func(sql: string, args: list<value>) -> result<u64, db-error>;
    }

    resource statement {
        bind: func(args: list<value>);
        query: func() -> result<result-set, db-error>;
    }

    resource result-set {
        next-row: func() -> option<list<value>>;
    }
}
```

### 2.3.4 No hidden state

A bridge function's behavior is fully determined by its arguments and the resource it is called on. Global state on the host side (per-request context, session, request headers) is exposed **only** through the appropriate `clean:host/*` interface that hands the guest a typed handle to that state:

```wit
// clean:host/routing
interface routing {
    resource request {
        method: func() -> http-method;
        path: func() -> string;
        header: func(name: string) -> option<string>;
        body: func() -> stream<u8>;
    }
    // handlers receive `request` as a parameter
}
```

There is no "current request" the guest can query from anywhere — it must be passed in as a resource. This preserves reasoning and testability.

### 2.3.5 Traps and typed bridge exceptions

### BRG-06 — A bridge component's trap MUST be catchable by the caller as a declared error variant


Bridges are increasingly implemented as their own components — every reference bridge listed in [02 components / bridges](../02%20components/bridges/) is a `.wasm` composed into the host graph, and the WASI 0.3 middleware chain in [Platform 18 §1.2](./18-component-composition.md) chains multiple such components per request. When a bridge component's implementation traps mid-call (division by zero in the bridge's own code, an unreachable, a wasm-native memory violation), the WebAssembly exception-handling proposal — ratified in WASM 3.0, shipped in wasmtime and every conformant runtime as of 2026 — lets the *caller* catch the trap instead of unwinding all the way to the host.

Clean uses that mechanism to preserve the [BRG-04](#brg-04--errors-are-structured-variants-never-strings) contract even when the bridge itself faults:

1. Every bridge function's `result<T, error>` variant MUST include a case `bridge-fault(fault-info)`, where `fault-info` is a record carrying the trap's structured origin:

   ```wit
   variant db-error {
       connection-failed(string),
       query-failed(query-failure),
       constraint-violation(constraint-info),
       timeout,
       bridge-fault(fault-info),         // MUST appear on every bridge error variant
   }

   record fault-info {
       kind:            fault-kind,      // Categorized trap origin.
       source:          string,          // Bridge component name (e.g. "clean-data-postgres-sidecar").
       source-version:  string,          // Bridge component version.
       message:         string,          // Trap message, or empty if none.
   }

   variant fault-kind {
       arithmetic,                       // Division by zero, overflow trap, etc.
       memory,                           // Out-of-bounds access, alignment fault.
       unreachable,                      // The bridge executed a wasm `unreachable`.
       resource-exhausted,               // Table/memory grow failure inside the bridge.
       other(string),                    // Anything the runtime cannot classify.
   }
   ```

2. The host runtime MUST wrap every bridge invocation in the runtime's typed-catch primitive (wasmtime's `try_call` or equivalent on other runtimes). A trap inside the bridge component MUST be lifted to a `bridge-fault` variant returned from the outer bridge function.

3. A trap inside the **guest** — the user's program — is not a bridge fault and is out of scope for this rule. Guest traps continue to be terminal for the current invocation; see [Platform 15 §7.1](./15-component-model-architecture.md#71-guarantees) and the host-side handling in each host spec's "instance discard" clause.

4. `bridge-fault` MUST NOT be raised as a substitute for a proper structured error. A bridge that catches its own internal error and returns `bridge-fault(...)` when a `query-failed(...)` was possible is a defect. `bridge-fault` is reserved for the case where the bridge implementation itself faults through no ability of the bridge author to declare a specific case.

**Rationale.** Before this rule, a bridge trap surfaced to the guest as either an opaque instance-terminating error or a runtime-specific string, depending on the host. The middleware chain in Platform 18 makes this more common: a fault in an auth-middleware component would kill the entire request instead of surfacing as a typed 500-class result the router can log with its origin. This rule turns those faults into observable, dispatchable errors without changing the runtime — WASM exception handling is already in every host runtime Clean targets.

**Non-goals.**

- This rule does NOT introduce Clean-language `try`/`catch` at the bridge boundary; the guest still receives `result<T, error>` and dispatches with the existing [error-handling operators](../04%20language/13-error-handling.md).
- This rule does NOT expose the raw wasm trap to the guest. The lifted variant carries a categorized `fault-kind` and a message, not a stack trace.

---

## 2.4 Declaring a Custom Host Function (Library Path)


A library adds a bridge function using the `host function` declaration in library source, per the grammar in [LBS-02](../02%20components/framework/09-libraries-specification.md#lbs-02--host-bridge-declaration-grammar). The declaration is the single source of truth for the function's shape. Declarations live in the library's root `host_bridge.cln` — one file per library, no declarations elsewhere ([LBS §8.2](../02%20components/framework/09-libraries-specification.md#82-file-layout)).

**Source (library):**

```clean
// mylib/host_bridge.cln
host interface storage version "0.1.0":
	requires host worlds ["server"]

	host function encryptAndWrite(path: string, data: bytes, keyRef: string) returns integer
		description "Encrypts data with the key at keyRef and writes it to path. Returns bytes written."
```

There is no `from "..."` clause and no hand-written `result<>` — fallible declarations return a normal type, and the `onError` mechanism is lowered to `result<T, error>` on the WIT side ([LBS-02](../02%20components/framework/09-libraries-specification.md#lbs-02--host-bridge-declaration-grammar)).

**What the framework does:**

1. Parses the declaration. Verifies the type list contains only WIT-representable types (see [§04 Type System](../04%20language/04-type-system.md)).
2. Synthesizes the corresponding WIT interface member in the library's generated WIT package (`clean:library/mylib@0.1.0`, interface `storage`, function `encrypt-and-write`), cached per §2.2.3 ([LBS §8.1](../02%20components/framework/09-libraries-specification.md#81-the-model)).

**What the compiler does:**

The compiler only consumes the synthesized WIT handed to it in the compilation request. At link time it verifies that every host world the library supports provides `encrypt-and-write` in `clean:library/mylib@0.1.0` with a matching signature. Missing → [`COM010`](./09-error-codes.md#310-compilation-codes-com) (`BridgeLinkError`) naming the world and the interface.

**What the host does:**

1. Reads the library's synthesized WIT during instance setup.
2. Registers an implementation via `wit-bindgen` (Rust, JS, etc.).
3. Provides the implementation to wasmtime's component `Linker` when instantiating the guest.

There is no other path. A library that ships a Rust file "implementing" its own host function is a bug — implementations live in hosts, not libraries. See [§01.1.2](./01-execution-layers.md#112-what-each-layer-must-not-do).

---

## 2.5 Naming Conventions


- **WIT interface names** are `kebab-case`: `outgoing-handler`, `wall-clock`, `arena-push`.
- **WIT function names** are `kebab-case`: `encrypt-and-write`, `hash-password`.
- **In Clean source**, functions are called with the language's normal identifier conventions (`encryptAndWrite`, `hashPassword`). The compiler translates identifier casing at the WIT boundary.
- **Package names** are `namespace:name@version`: `wasi:filesystem@0.3.0`, `clean:bridge@1.0.0`, `clean:library/mylib@0.1.0`.

Identifier casing at the language boundary is translated by the compiler; there is no dual "underscore vs dot" naming convention.

---

## 2.6 Always-On Runtime Support

### BRG-05 — The always-on set is closed


A small set of L2 functions are exempt from reachability elision ([LAY-03](./01-execution-layers.md#13-reachability-and-dead-import-elision)) because runtime string/list operations may reference them regardless of whether the user program does. The compiler MUST always emit imports for exactly this set and no more:

| Function | Interface | Reason |
|----------|-----------|--------|
| `print` | `clean:bridge/console` | Runtime panic and trace paths write to stdout. |
| `arena-push`, `arena-pop` | `clean:bridge/mem` | Scope-based allocation used by string/list runtime helpers. |
| `mem-alloc` | `clean:bridge/mem` | The bump allocator when scope isn't active. |
| `concat`, `substring`, `compare` | `clean:bridge/string` | Emitted by codegen for `+` on strings, `==` on strings, slicing. |

The set is closed: extending it MUST be done by spec amendment to this section.

---

## 2.7 Host Contract Testing


Every host proves it satisfies its declared world by running a **shared conformance suite** in CI. The suite lives in its own component repository, **`clean-conformance`** (code lives outside the foundation repo — [EXE-04](../01%20governance/04-execution-model.md)), and is compiled from Clean source into a component that:

- Calls every function in the world at least once with valid inputs and asserts the expected result.
- Calls every fallible function with inputs that must produce each declared error variant.
- Verifies memory-and-resource lifetime rules (open, use, drop, verify closed).
- Exercises edge cases (empty strings, empty lists, large payloads at documented limits, boundary times).

A host that fails conformance does not ship. This is a merge gate, not a nice-to-have. See [§15.10](15-component-model-architecture.md) for the wire format the conformance runner uses.

---

## 2.8 Async and Streaming


WASI 0.3 (Preview 3, ratified 2026-06-11) exposes long-running I/O through the **`async func`, `stream<T>`, and `future<T>` types** as canonical-ABI primitives. Clean bridges to these directly:

- `stream<u8>` — request bodies, response bodies, file reads/writes, SSE, WebSocket frames.
- `future<T>` — outbound HTTP responses, scheduled job results.
- `async func` — any bridge function whose completion depends on host I/O; the guest awaits it natively.

The guest awaits streams and futures using Clean's [async syntax](../04%20language/18-async.md). The compiler translates `start`, `later`, and `background` into the corresponding `await` / stream-read / cancellation-token operations at the WIT boundary. There is no `poll`-loop shim — Preview 2's `poll`-based model is not part of V2 ([ADR-0022 §3](../01%20governance/decisions/0022-foundational-technology-stack.md)).

**Rule:** the host may not expose a callback-style bridge function (e.g. "call me back when done"). Async is expressed as streams, futures, or `async func`. This keeps the guest in control of scheduling and cancellation.

---

## 2.9 Backward Compatibility of Bridge Interfaces

This section is a summary gloss; the compatibility rules are owned by [§08 Bridge Versioning](./08-bridge-versioning.md), which wins on any disagreement.

Bridge WIT packages are versioned (`clean:bridge@1.0.0`). See [§08 Bridge Versioning](./08-bridge-versioning.md) for the full compatibility rules. Summary:

- **Adding** an interface member is a minor version bump. Old programs still link.
- **Removing or renaming** a member is a major version bump. Old programs fail to link against the new world.
- **Changing a signature** (adding a parameter, changing a type) is a major version bump.
- **Restructuring an error variant** (adding a case) is a minor bump; removing a case is major.

Hosts declare which package versions they provide in their published `host.wit` (see [§08.4](./08-bridge-versioning.md#84-host-declaration) and [16 — Host Contract Validation](./16-host-contract-validation.md)). The compiler emits against the intersection of the target world's declared minor versions and picks the highest compatible one.

---

## 2.10 What This Section Does Not Cover


- The **byte-level ABI** of each WIT type (canonical ABI, memory layout, alignment). This is defined in the Component Model specification and consumed via `wit-bindgen`.
- The **wasmtime `Config` and `Linker` calls** each host uses to register its world. See [§15 §7](15-component-model-architecture.md) and each host's own documentation.
- **Individual WASI package semantics.** Refer to the [WASI 0.3](https://github.com/WebAssembly/WASI) specifications directly.

---

## 2.11 Deferred Refinements


1. **Resource sharing across guests.** Component Model resource semantics forbid a resource from crossing between instances. For use cases like shared DB connection pools, hosts implement pooling internally and hand each guest a fresh handle. Cross-guest resource sharing is not exposed at the language level in V2.
2. **Custom sections for library metadata.** Libraries embed metadata (docs, examples, MCP schemas) using standard component custom sections. The concrete section format is defined per-library and is not part of this spec.

*(The former "WASI Preview 3" refinement has been retired: WASI 0.3 is the V2 floor. See [ADR-0022 §3](../01%20governance/decisions/0022-foundational-technology-stack.md) and §2.8 above.)*

---

## Changelog

- 2026-08-05 — Added §2.3.5 **Traps and typed bridge exceptions** and minted **BRG-06**: a wasm-level trap inside a bridge component MUST be caught by the host and lifted into a `bridge-fault(fault-info)` case on the enclosing error variant. Requires every bridge error variant to declare `bridge-fault`; introduces `fault-info` and `fault-kind` shared shapes. Rationale: preserves BRG-04 (structured errors) across bridge components (increasingly common under the Platform 18 middleware chain) without exposing raw traps to guests. Uses WASM exception handling (WASM 3.0, shipped in every runtime Clean targets). Non-goals: does not add Clean-language `try`/`catch` at the bridge boundary, does not expose stack traces. Also updated §2.8 (Async/Streaming) and §2.11 (Deferred Refinements) to reflect the WASI 0.3 baseline landing (see [ADR-0022 §3](../01%20governance/decisions/0022-foundational-technology-stack.md)); mechanical version-string sweeps in §2.1, §2.2 `clean:bridge` example, §2.5 package-names bullet, §2.10 WASI link.
- 2026-08-01 — Technical-debt closure (final pass, user-approved): `clean:bridge/files` promoted from "proposed" note to a full catalog row ([ADR-0005](../01%20governance/decisions/0005-server-world-interface-additions.md) is Accepted); the two "(location pending)" markers resolved — canonical WIT source lives in `wit/` at the root of this repository, and the host conformance suite lives in the `clean-conformance` component repository (code outside foundation, [EXE-04](../01%20governance/04-execution-model.md)).
- 2026-08-01 — Conflict-log remediation (Fase 3): §2.4 rewritten to the [LBS-02](../02%20components/framework/09-libraries-specification.md#lbs-02--host-bridge-declaration-grammar) grammar (resolution 0.2) — declaration in the root `host_bridge.cln`, no `from "..."` clause, no explicit `result<>`; WIT synthesis attributed to the framework (consistent with §2.2.3), with the compiler consuming only the synthesized WIT from the compilation request. Library package versions corrected to the `@0.1.0` baseline (08 §8.0). D1 removed from the reference driver list in favor of a cite to [ADR-0002](../01%20governance/decisions/0002-clean-server-reference-stack.md). `./wit/` and `./conformance/` marked "(location pending)" instead of naming nonexistent directories. §2.2.1 gained a note that `clean:bridge/files` is a proposed addition via [ADR-0005](../01%20governance/decisions/0005-server-world-interface-additions.md). `~/.cln/wit-cache/` now cites Manager §00.2 as the layout home. §2.9's `host.toml` mention replaced by the published `host.wit` per the approved P7 resolution. World names normalized to the bare canonical form of 15 §0.3 (resolution 0.1).

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Compiler and host implementors that expose the bridge; library authors declaring `host function`s
- **Rule prefix:** `BRG-`
- **Part of:** [Clean Language Specification — Platform](./README.md)
- **References:** [Component Model Architecture](./15-component-model-architecture.md), [Bridge Versioning](./08-bridge-versioning.md), [Host Contract Validation](./16-host-contract-validation.md), [ADR-0022 §2](../01%20governance/decisions/0022-foundational-technology-stack.md)
- **Satisfies:** INTEROP-01, INTEROP-05, INTEROP-06, SEC-01, SEC-10
