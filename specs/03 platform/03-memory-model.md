# Platform 03. Memory Model

This chapter defines how a Clean program's memory is laid out and managed — the linear-memory layout for programs targeting Component Model core memory, the string and collection representations that cross the bridge, the arena discipline that keeps allocations balanced, and the observable contract every host's memory backing must satisfy. The layout is a language-level guarantee: a Clean program running on any conformant host observes the same shape, the same allocator behavior, and the same scope semantics. Hosts choose how to *back* the linear memory (see [§3.5](#35-host-backing--observable-contract)), but the guest-visible shape is fixed here.

---

## 3.1 Linear Memory Layout

### MMD-01 — Layout and guest-visible constants


Each guest component instance has one linear memory. Its layout is:

```
Byte offset     Region                     Purpose
─────────────────────────────────────────────────────────────────────
[0            .. 1024)         Reserved   Null-pointer guard (unmapped; any access traps)
[1024         .. HEAP_START)   Data       Empty-string constant + static data emitted by the compiler
[HEAP_START   .. current_end)  Heap       Runtime allocations
[current_end  .. max_end)      Unmapped   Available for growth up to the tier limit
```

**Constants (part of the guest-visible contract):**

| Symbol | Value | Meaning |
|--------|-------|---------|
| `NULL_GUARD_BYTES` | 1024 | Prefix left unallocated so that a null-pointer dereference traps immediately. |
| `EMPTY_STRING_ADDR` | 1024 | The 4-byte length-prefixed representation of `""` lives here — the first bytes of the data section, immediately after the null guard (`= NULL_GUARD_BYTES`). All empty strings share this address. |
| `DATA_SECTION_START` | 1024 | The compiler places static data starting here (the empty-string constant occupies the first 4 bytes). |
| `HEAP_START` | 1_048_576 (1 MiB) | The bump allocator begins here. Fixed regardless of how much data the compiler emitted. |
| `WASM_PAGE_SIZE` | 65_536 (64 KiB) | Enforced by the WASM specification. |
| `ALIGNMENT` | 8 bytes | All heap allocations are 8-byte aligned. |

`HEAP_START` is exposed to the runtime as the WASM global `__heap_start` (immutable). The current heap pointer is the WASM global `__heap_ptr` (mutable), initialized to `HEAP_START`.

**Rule:** hosts and libraries MUST NOT assume the heap begins at 64 KiB or at any address other than the value in `__heap_start`. Hardcoding a heap offset is a bug.

**Rule:** the static data the compiler emits MUST fit the data region `[DATA_SECTION_START, HEAP_START)`. `HEAP_START` is fixed and never moves to accommodate more data — a program-dependent heap base would make `__heap_start` vary per build and re-open the hardcoded-offset bug class this section closes. A program whose emitted static data does not fit (≈1 MiB of string constants and other compiler-emitted data) therefore has no conforming layout, and the compiler MUST reject it at code generation with [`COM003`](./09-error-codes.md#310-compilation-codes-com) (`MemoryLayoutError` — rule entry with template: [10 §COM003](./10-semantic-rules.md)). This is a user-program condition, never an internal compiler error.

---

## 3.2 Allocation


Clean uses a **bump allocator** paired with **scope-based reclamation**. There is no garbage collector, no free list, and no runtime ARC.

### 3.2.1 Bump allocator

### MMD-02 — `alloc` traps on failed growth; no failure value


The runtime exposes:

```
alloc(size: i32, align: i32) -> i32     // Returns aligned address; traps if memory cannot grow
```

`alloc` advances `__heap_ptr` by `size` (rounded up to `align`) and returns the previous value. If satisfying the allocation requires a `memory.grow` that cannot be satisfied within the tier limit, the grow MUST **trap** at the call site with [`MEM001`](./09-error-codes.md#314-memory-codes-mem) (`TierExceeded` — see [§05.3](./05-memory-policy.md#53-enforcement)); there is no observable failure value.

There is no matching `free`. Individual allocations are never released back to the allocator.

### 3.2.2 Arena scopes

Memory is reclaimed by **scope, not by individual allocation**. The runtime exposes:

```
arena-push() -> i32                     // Returns a save-point (current __heap_ptr)
arena-pop(save-point: i32) -> ()        // Resets __heap_ptr to save-point, O(1)
```

Reclamation is fully deterministic:

- Every allocation between `arena-push()` and `arena-pop(sp)` is invalidated by the pop.
- The pop is O(1) — a single global write.
- No destructors run. Clean's value types are POD-shaped by design (see [§04 Type System](../04%20language/04-type-system.md)); anything that needs cleanup (open files, DB connections) is a WIT resource and is closed by the host, not by scope pop.

### 3.2.3 Where scopes come from

The compiler inserts arena scopes at these boundaries:

| Boundary | Scope behavior |
|----------|---------------|
| Top-level function call | Optional (opt-in via compiler flag; default off — the function's callees may hold references). |
| `iterate` body iteration | New scope per iteration if the iteration variable is not captured beyond it. Enables efficient loops that don't retain intermediates. |
| HTTP request handler | New scope per request. Popped after the response is sent. Required on the `server` world. |
| Canvas frame | New scope per frame. Popped after the frame commits. Required on hosts that implement the canvas library's synthesized interface, `clean:library/canvas` ([LBS §8.1](../02%20components/framework/09-libraries-specification.md#81-the-model) — library host imports are `clean:library/<name>`, not `clean:host/*` vocabulary entries). |
| Async task | New scope on task start. Popped on task completion. |
| Async suspension point (`await`) | Scope survives the suspension. See §3.2.4. |
| Stream producer (`stream<T>`) | Scope survives while any consumer holds the stream handle. See §3.2.4. |

Scopes are the language's answer to "how do I run a request handler 10 000 times without leaking?" — the per-request scope invalidates all allocations at once.

### 3.2.4 Arenas across async suspension and stream lifetimes


WASI 0.3 (released 2026-06-11) introduces native `future<T>` and `stream<T>` as canonical-ABI types. Clean's server world imports them; handlers can `await` and producers can emit chunks into streams whose consumers live in another component. This changes the arena-lifetime picture in two ways:

**Suspension does not pop the scope.** When an async handler `await`s a `future<T>`, the request-scoped arena stays alive across the suspension point. The pending future holds guest pointers into that arena; popping the scope would invalidate them. The scope is popped only when the top-level task completes (successfully, with an error, or by cancellation). This means a slow `await` extends the effective memory lifetime of the request — operators should size the tier accordingly.

**Streams may outlive the emitting scope.** A `stream<T>` producer writes chunks into the request-scoped arena, but the consumer may read them from a different task with a different scope. To preserve the O(1) pop invariant, chunk payloads that cross a stream boundary MUST be copied out at the canonical-ABI lift — the guest never hands the host a pointer into the producer's live arena. Copies happen at the bridge (see §3.7); the producer's arena remains free to pop when its task ends.

Cancellation follows the same rule as any other completion: if the runtime cancels a task (WASI 0.3 cancellation tokens), the associated scope is popped, and any `future<T>` / `stream<T>` handle held by another party observes the cancellation via its result channel — it never observes freed guest memory.

### MMD-03 — Arena discipline: every push balanced by exactly one pop


Libraries and user code MAY call `arena-push` / `arena-pop` directly (via the `clean:bridge/mem` interface — see [§02.2.1](./02-host-bridge.md#221-portable-l2-in-every-world)) but MUST balance every push with exactly one pop, and MUST NOT pop past a save-point they did not receive. A pop without a balanced push, or a pop past a save-point the caller did not receive, traps at runtime with [`MEM003`](./09-error-codes.md#314-memory-codes-mem) (`ArenaImbalance`). Scopes never popped do not trap; they are observable on instance drop as `clean_wasm_arena_leak_bytes` (§3.9).

---

## 3.3 String Representation

### MMD-04 — String layout and semantics


Every Clean `string` in linear memory has the layout:

```
Offset  Bytes  Field
────────────────────────────────────────
+0      4      length     (u32 little-endian, byte count of UTF-8 payload)
+4      N      payload    (UTF-8 bytes, N == length, no NUL terminator)
```

The address of a string is the address of the length field. The empty string is the single fixed address `EMPTY_STRING_ADDR = 1024`.

**Rules:**

- Strings are always UTF-8. Codegen guarantees no invalid sequence enters memory.
- Strings are immutable. Any operation that "modifies" a string allocates a new one.
- String comparison compares (length, payload) — not pointer equality. Two strings with identical content at different addresses are equal. The payload comparison is byte-exact and never normalizes; the observable semantics, including the case of two canonically equivalent strings that compare unequal, is [TYP-07](../04%20language/04-type-system.md#typ-07--string-equality-is-byte-exact-nothing-normalizes).
- String hashing (for pairs, sets) uses xxhash64 of the payload with the length as seed.

**At the bridge:** WIT `string` values marshal to and from this layout via the canonical ABI. The guest does not manually construct length prefixes; `wit-bindgen` handles it.

---

## 3.4 Collection Representations


### 3.4.1 `list<T>`

```
Offset  Bytes                    Field
─────────────────────────────────────────────────
+0      4                        length      (u32)
+4      4                        capacity    (u32)
+8      4                        elem_type   (u32, type tag)
+12     4                        _padding
+16     length * sizeof(T)       elements
```

Lists grow by the 1.5× amortized policy specified in [§05.2](./05-memory-policy.md#52-growth-strategy). `capacity` is always a multiple of the platform's minimum growth unit.

**`elem_type` is a compiler-assigned tag, not part of the ABI.** Tag values are stable within a single compilation but not across compilations, compiler versions, or programs. Do not persist a `list<T>` header to disk, wire, or shared storage and expect to read it back — the tag will disagree. Serialize the elements explicitly (JSON, protobuf, Clean's own encoders) instead.

**`sizeof(T)` — element widths.** The `elements` region sizes each slot by `sizeof(T)`:

| Element type `T` | `sizeof(T)` | Slot holds |
|------------------|-------------|------------|
| `integer`, `number` | 8 | The value itself (s64 / f64) |
| `boolean` | 4 | 0 or 1 (i32) |
| Boundary-width integers narrower than 64 bits (`integer:32` — [LBS-02](../02%20components/framework/09-libraries-specification.md#lbs-02--host-bridge-declaration-grammar) declarations only) | 4 | The value itself (i32) |
| Enum discriminants | 4 | The tag (u32) |
| `string`, `bytes`, `list<…>`, `pairs<…>` | 4 | A pointer to the object |

Widths are the 32-bit layouts of this chapter (§3.6: the V2 compiler emits 32-bit components only). `sizeof(T)` for record/class-valued and `any`-valued elements is **not yet defined** — those types have no layout in this chapter; the gap is under decision (see the composite-layouts brief in `work/`), and until it closes a list of such elements has no conforming layout.

### 3.4.2 `pairs<K, V>` (Clean's ordered map)

```
Offset  Bytes                    Field
─────────────────────────────────────────────────
+0      4                        count       (u32)
+4      4                        capacity    (u32)
+8      capacity * 8             entries     (4-byte key ptr + 4-byte value ptr)
```

Keys and values are stored by pointer; the payload lives elsewhere in the heap. Lookup is O(n) — pairs preserve insertion order. (Resolved 2026-08-01: **`pairs<K, V>` is Clean's map type** — owner: [04 language / 04 — Type System](../04%20language/04-type-system.md). No separate `map<K, V>` type exists or is planned, and the formerly referenced `clean:bridge/collections` package is definitively retired.)

### 3.4.3 `bytes`

Layout is identical to `string` except no UTF-8 constraint on the payload. WIT `list<u8>` marshals directly to this shape.

---

## 3.5 Host Backing — Observable Contract

### MMD-05 — Observable host-backing contract


How a host backs guest linear memory is an implementation choice; the guest-observable behavior is not. Every host MUST satisfy this contract. The reference configuration that realizes it (engine flags, reservation sizes, pooling) is recorded in [ADR-0007 — Host Memory Backing](../01%20governance/decisions/0007-host-memory-backing.md).

- **Out-of-bounds accesses trap.** The host MUST back linear memory with guard regions (or equivalent bounds enforcement) so that any access outside committed memory — including a null-pointer dereference into the `NULL_GUARD_BYTES` region (§3.1) — traps immediately with [`RUN001`](./09-error-codes.md#312-runtime-codes-run) (`MemoryViolation`). No out-of-bounds read or write may ever return data.
- **Addresses are stable.** The linear memory's base address MUST NOT move for the lifetime of the instance. The bump allocator's stable-address invariant (§3.2.1) depends on it; a moving memory would invalidate every pointer held by the guest between grow operations.
- **Tiers are respected.** Committed memory MUST be capped at the tier limit from [§05 Memory Policy](./05-memory-policy.md). A `memory.grow` beyond the tier traps at the offending call site with [`MEM001`](./09-error-codes.md#314-memory-codes-mem) (`TierExceeded` — see [§05.3](./05-memory-policy.md#53-enforcement)); no failure value reaches the guest.
- **Runaway guests are interruptible.** Request- and frame-scoped instances MUST be bound to a wall-clock budget whose exhaustion traps with [`RUN012`](./09-error-codes.md#312-runtime-codes-run) (`TimeBudgetExceeded`); the budget defaults per invocation are owned by [§07 `[runtime]`](./07-build-config.md). In the compile-time sandbox, deterministic accounting takes precedence over throughput ([ADR-0004](../01%20governance/decisions/0004-block-handler-execution-model.md), [§21.7](../04%20language/21-block-handlers.md#217-compile-time-execution-environment)).
- **Pool-slot reuse is opaque to the guest.** Hosts MAY reuse a linear-memory slot across instances (via a pooling allocator or equivalent). Before a slot is handed to a new instance, the host MUST zero the entire linear memory — including the `NULL_GUARD_BYTES` region (§3.1) and any prior heap contents. No byte written by a prior instance may be readable by a subsequent one. This closes the class of pool-slot data-leakage bugs (cf. Wasmtime advisories `GHSA-wh6w-3828-g9qf`, `GHSA-44mr-8vmm-wjhg`) at the language contract level rather than leaving it to reference-host discipline.
- **Guest memories are non-shared.** Every Clean guest declares its linear memory as non-shared. Hosts MUST reject a Clean component whose core memory is declared `shared` — this is a build error, not a runtime one. Non-shared memories are what makes pooling and the stable-base-address invariant possible; the shared-everything-threads route is a separate concurrency model tracked as a non-goal (§3.10).

Instantiation-cost amortization, virtual-memory reservation strategy, and every engine-specific flag are reference configuration: see [ADR-0007](../01%20governance/decisions/0007-host-memory-backing.md).

---

## 3.6 Memory64


Memory64 was ratified as part of WebAssembly 3.0 (Sept 2025) and is shipped in Chrome 119+, Firefox 120+, and Safari 18.2+. Guests targeting Component Model core memory MAY declare a **memory64** memory (64-bit addresses). V2 hosts MUST accept memory64 guests and back them consistently with the observable contract above:

- The effective ceiling is the tier limit (memory64 does **not** grant more memory). No tier in [§05 Memory Policy](./05-memory-policy.md) currently permits >4 GiB, so memory64 is **shape-compatible, ceiling-limited** in V2: the wider index space is legal, but the tier limit binds first.
- On browser targets, the engine caps memory64 at 16 GiB regardless of tier — the browser cap always binds before any Clean tier could.
- The same static-address requirement applies.
- Hosts MUST cap memory64 guests at the tier limit regardless of engine defaults (reference configuration: [ADR-0007](../01%20governance/decisions/0007-host-memory-backing.md)).

**Guidance:** prefer 32-bit unless the program's working set genuinely exceeds 4 GiB. Memory64 costs roughly 15–20% throughput on pointer-heavy code (i64 indexing, larger pointer size, less cache density) and offers no benefit under the current tier ceiling. Memory64 is opt-in per program via `clean.toml` (see [§07 Build Config §7.3](./07-build-config.md#73-memory--full-schema)). The default is 32-bit.

**Compiler contract (V2).** Every layout in this chapter — pointer slots, collection headers, `sizeof(T)` — is specified in 32-bit widths, and the V2 compiler emits 32-bit components only. `build.memory64 = true` is schema-valid ([§07.3](./07-build-config.md#73-memory--full-schema)) but **reserved**: the V2 compiler MUST refuse a request carrying it with [`COM005`](./09-error-codes.md#310-compilation-codes-com) (`TargetFeatureUnsupported`) rather than emit 32-bit layouts under the flag. The host obligations above are unchanged — they bind V2 hosts against memory64 guests from foreign toolchains. The 64-bit widths for every §3 layout are deferred together with the flag; specifying them is a prerequisite for any memory64-emitting compiler.

---

## 3.7 Interaction with the Component Model


Under the Component Model, "linear memory" is a property of the underlying **core module**, not of the component itself. Components composed from multiple core modules each have their own linear memory. Bridge values (`string`, `list`, `record`) crossing between components are marshalled via the canonical ABI — there is no shared memory between the guest and the host.

**Rule:** a host implementation of a bridge function receives WIT values that have been copied out of guest memory. Mutating those values does not affect the guest, and freeing them is the host runtime's job. Bridge functions never expose raw guest pointers to the host.

---

## 3.8 Compile-Time Sandbox Memory


Block handlers ([§21](../04%20language/21-block-handlers.md)) run in a sandboxed sub-instance of wasmtime inside the compiler, per [ADR-0004 — Block Handler Execution Model](../01%20governance/decisions/0004-block-handler-execution-model.md). Their memory model is identical to §3.1–§3.4, but:

- Memory is capped at 128 MiB (configurable via `clean.toml [compile.limits] handler-memory-mb`).
- Every arena scope is discarded when the handler returns.
- No `mem-alloc` may exceed 16 MiB in a single call (guards against pathological IR construction). This cap binds the Clean memory-model allocator *inside* handler code, not the sandbox boundary: the sandbox observes only linear-memory growth, and individual `mem-alloc` calls are invisible in a hand-written or foreign-toolchain artifact. It is therefore enforceable only where the handler was compiled by a toolchain that emits the Clean allocator; for any other artifact the 128 MiB memory cap above is the enforced boundary.

Diagnostics from the sandbox reference the user's source span (see [§21.6](../04%20language/21-block-handlers.md#216-diagnostics-from-compile-time-functions)), never the sandbox's internal addresses.

---

## 3.9 Debugging and Observability


Every host exposes per-instance memory metrics:

| Metric | Type | Meaning |
|--------|------|---------|
| `clean_wasm_memory_current_bytes` | gauge | Bytes currently committed |
| `clean_wasm_memory_peak_bytes` | gauge | Maximum committed since instance start |
| `clean_wasm_memory_grows_total` | counter | Number of `memory.grow` calls |
| `clean_wasm_memory_oom_total` | counter | Number of grow failures that trapped |
| `clean_wasm_arena_pushes_total` | counter | Scope opens |
| `clean_wasm_arena_leak_bytes` | gauge | Bytes allocated in scopes never popped (detected on instance drop) |
| `clean_wasm_memory_grow_bytes_total` | histogram | Distribution of `memory.grow` request sizes in bytes — surfaces whether the §05.2 1.5× policy is producing the expected shape |
| `clean_wasm_instance_reuse_total` | counter | Number of times a pooled linear-memory slot was reused for a new instance (0 on non-pooling backends) |

The browser host exposes the same metrics under `window.__cleanRuntime.memoryStats()`.

Hosts SHOULD support capturing a core dump on trap for post-mortem inspection; how the reference host enables this is recorded in [ADR-0007](../01%20governance/decisions/0007-host-memory-backing.md).

---

## 3.10 Non-Goals


- **Garbage collection (WasmGC).** WasmGC was ratified in WebAssembly 3.0 (Sept 2025) and is available in every conformant engine (Chrome 119+, Firefox 120+, Safari 18.2+, Node 22+, Deno 1.38+). Clean does not use it. WasmGC targets languages with heap-shaped object graphs and cyclic references (Java, Kotlin, Dart, Scala); Clean's value types are POD-shaped by design (see [§04 Type System](../04%20language/04-type-system.md)) and its scope-based reclamation is O(1). Adopting WasmGC would trade a predictable memory shape for engine-managed collection, violating [PERF-06](../01%20governance/09-performance-principles.md). This is a permanent stance, not a deferred one. V3 may revisit only if a new use case emerges (e.g. hosting a GC'd guest language inside a Clean sandbox).
- **Manual `free`.** The bump-and-arena model is the reclamation strategy. Adding per-allocation free would break the O(1) scope-pop invariant.
- **Shared memory between instances.** Not supported. Cross-instance state goes through WIT resources.
- **Intra-instance threading (shared-everything-threads).** The shared-everything-threads proposal (draft, 2026) extends WASM with shared tables/globals/functions and is compatible with WasmGC. Clean does not adopt it in V2. Clean's concurrency story is WASI 0.3 native async (`future<T>`, `stream<T>`) — a message-passing model that composes with arena scopes (§3.2.4). Shared-memory threading would require abandoning the stable-base-address + non-shared-memory invariants that make pooling safe (§3.5); the tradeoff is not worth it for the workloads Clean targets.
- **Stack switching.** The stack-switching proposal (prototype in Wasmtime, not in WASM 3.0) enables typed continuations, green threads, and coroutine-shaped iterators. Clean does not use it. If it eventually ships, V3 may reconsider `iterate` codegen, but the current arena model already delivers the loop-local reclamation that would motivate it.
- **Fine-grained ARC.** Reference counting is not part of the memory model. The bump-and-arena strategy is the only reclamation mechanism.

---

## 3.11 Deferred Refinements


1. **Custom allocators per scope class.** Long-lived arenas (page lifetime in the browser) use the same bump allocator as per-request arenas. V2 does not introduce a second allocator (dlmalloc-style or otherwise). If measured fragmentation on long-lived instances becomes a problem, the model may be revisited.
2. **`memory.grow` cost accounting.** Grow calls do not consume epoch time proportional to bytes copied. The host's `StoreLimits` is the sole gate on memory growth.

---

## Changelog

- 2026-08-19 — Three errata from the compiler's Milestone 6 (`clean-language-compiler/docs/DISCOVERIES-M6.md`, items 4, 6a, 5). **§3.1**: new rule — static data MUST fit `[DATA_SECTION_START, HEAP_START)`; MMD-01's "fixed regardless of how much data the compiler emitted" left a program with ≥ ~1 MiB of literals with no conforming layout and no owning diagnostic (the compiler surfaced it as a `COM013` internal error). The rejection is now [`COM003`](./09-error-codes.md#310-compilation-codes-com), whose stub rule in [10](./10-semantic-rules.md) gains its condition and template; the interim COM013 is superseded. **§3.4.1**: the `sizeof(T)` table §3.4.1 always priced elements by is now written down (integer/number 8; boolean, narrow boundary integers, enum discriminants 4; string/bytes/list/pairs are 4-byte pointers — ratifies the compiler's adoption verbatim); record/class and `any` element widths are explicitly left open with the rest of the missing composite layouts. **§3.6**: the compiler side of memory64 stated — V2 emits 32-bit only, `build.memory64 = true` is reserved and refused with [`COM005`](./09-error-codes.md#310-compilation-codes-com); §3.6 had bound only *hosts*, leaving a schema-valid flag no chapter said how to compile.
- 2026-08-18 — §3.8's per-call 16 MiB `mem-alloc` cap gains an observability note, from the compiler's Milestone 5 (`clean-language-compiler/docs/DISCOVERIES-M5.md`, item 9): the cap binds the Clean allocator inside handler code, but the sandbox boundary observes only linear-memory growth, so for hand-written or foreign-toolchain artifacts the rule is unenforceable and the 128 MiB memory cap is the enforced boundary. Limit value and rule unchanged — the note records where enforcement is possible.
- 2026-08-05 — WASM 3.0 / WASI 0.3 landscape refresh. **§3.10** GC non-goal rewritten: WasmGC is now standard (ratified 2025-09, shipped in all major engines) but Clean rejects it on language-design grounds (POD-shaped values, PERF-06), not "wait and see" — the stance is now permanent. Added non-goals for shared-everything-threads (draft, 2026) and stack switching (prototype), citing why the concurrency route is WASI 0.3 async. **§3.6 Memory64** updated: now standard in WASM 3.0, framed as "shape-compatible, ceiling-limited" with 16 GiB browser cap and 15–20% throughput guidance for 64-bit indexing. **§3.5** two new observable-contract rules added: (a) pooled slots MUST be zeroed before reuse (closes the pool-slot data-leakage class at the language level, cf. Wasmtime advisories `GHSA-wh6w-3828-g9qf` / `GHSA-44mr-8vmm-wjhg`); (b) guest memories MUST be declared non-shared (hosts reject `shared` memory at build time). **§3.2.3** table gained rows for async-suspension and stream-producer scope behavior; new **§3.2.4** specifies that `await` does not pop the request scope and that `stream<T>` chunks crossing a component boundary are copied at the canonical-ABI lift, preserving O(1) pop. **§3.9** two new metrics: `clean_wasm_memory_grow_bytes_total` (histogram) and `clean_wasm_instance_reuse_total` (counter for pooled backends). **§3.4.1** clarified that `elem_type` is a compiler-assigned tag, not part of the ABI, and must not be persisted. No rule renumbering, no layout change, no bridge-ABI change.
- 2026-08-02 — The string-comparison rule now says the payload comparison is **byte-exact and never normalizes**, and cites [TYP-07](../04%20language/04-type-system.md#typ-07--string-equality-is-byte-exact-nothing-normalizes) for the observable semantics. It previously said only "compares (length, payload)", which left open whether the payload comparison normalized — the gap recorded as question 3 of [ADR-0014](../01%20governance/decisions/0014-source-text-encoding-and-identifier-charset.md). Representation stays here; what `==` means stays in the language tree. No change to the layout.
- 2026-08-01 — Technical-debt closure pass (lote X, approved 2026-08-01): the two "(pending)" diagnostic markers resolved with formally registered codes — arena-discipline violations (MMD-03) now trap as [`MEM003`](./09-error-codes.md#314-memory-codes-mem) `ArenaImbalance` (unbalanced pop / pop past a foreign save-point; never-popped scopes remain the non-trapping `clean_wasm_arena_leak_bytes` metric), and wall-clock budget exhaustion (MMD-05) now traps as [`RUN012`](./09-error-codes.md#312-runtime-codes-run) `TimeBudgetExceeded`. §3.4.2 map question closed: `pairs<K, V>` **is** Clean's map (owner: 04 language/04-type-system); no separate `map<K,V>` type; the `clean:bridge/collections` ghost is definitively retired.
- 2026-08-01 — Conflict-log remediation (Fase 3): §3.5 replaced by the guest-observable host-backing contract, with the wasmtime `Config`/pooling/`StoreLimits`/epoch mechanism extracted to [ADR-0007](../01%20governance/decisions/0007-host-memory-backing.md); §3.6 now refers to that contract instead of the extracted mechanism. **Incoherence correction (P13b):** `EMPTY_STRING_ADDR` moved from `4` to `1024` (`= NULL_GUARD_BYTES`) — the previous text simultaneously required the first 1024 bytes to be an unmapped trap region and placed the empty-string constant at address 4; the two statements were never implementable together, so no real ABI is broken. §3.2.1 `alloc` contract rewritten per P13a: a failed grow traps (05 §5.3), there is no observable `-1`. §3.8 now cites [ADR-0004](../01%20governance/decisions/0004-block-handler-execution-model.md) for the compiler's handler sandbox. `arena_push`/`arena_pop` renamed to WIT kebab-case `arena-push`/`arena-pop` per 02 §2.5; `map<K,V>`/`clean:bridge/collections` removed (open design question, no home); `clean:host/canvas` marked pending [ADR-0005](../01%20governance/decisions/0005-server-world-interface-additions.md); world names normalized per 15 §0.3; stale `#24x` anchors repointed to the real 21.x headings.

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Compiler and host implementors backing guest linear memory; library authors that touch arena scopes
- **Rule prefix:** `MMD-`
- **Part of:** [Clean Language Specification — Platform](./README.md)
- **References:** [Execution Layers](./01-execution-layers.md), [Host Bridge](./02-host-bridge.md), [Memory Policy](./05-memory-policy.md), [ADR-0007 — Host Memory Backing](../01%20governance/decisions/0007-host-memory-backing.md)
- **Satisfies:** LANG-01, PERF-06
