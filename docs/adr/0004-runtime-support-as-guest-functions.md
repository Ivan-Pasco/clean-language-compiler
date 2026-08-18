# ADR 0004 — Runtime support is emitted as guest functions; the BRG-05 always-on imports are deferred

Status: Accepted (2026-08-18)

## Context

Platform 03 (memory model) obligates the compiler to emit a bump allocator
over `__heap_ptr` (MMD-01/02), arena save-points (MMD-03), and the string
layout plus its operations (MMD-04). Platform 02 §2.6 (BRG-05) separately
obligates the compiler to always emit imports for `print`
(`clean:bridge/console`), `arena-push`/`arena-pop`/`mem-alloc`
(`clean:bridge/mem`) and `concat`/`substring`/`compare`
(`clean:bridge/string`).

The two chapters cannot both be satisfied today:

1. **The WIT does not exist.** No `clean:bridge` package ships anywhere the
   request can deliver it: foundation `03 platform/wit/` holds only a
   README, and CMP-01/CCMP-03 forbid the compiler conjuring interfaces the
   request did not carry.
2. **No host provides the interfaces.** The vendored `clean:host` server
   world restates neither `clean:bridge/mem` nor `clean:bridge/string`;
   clean-host-core implements no `clean:bridge/*` interface (its only
   mention of the namespace is a version-validation test). A component
   importing them would never instantiate.
3. **The signatures cannot cross the component boundary.** §3.2's
   `arena-push() -> i32` returns a guest heap address and `mem-alloc`
   returns pointers into guest linear memory, while §3.7 states bridge
   functions never expose raw guest pointers and there is no shared memory
   between guest and host. As component-level imports these functions are
   unimplementable; they only make sense as core-module shims composed
   into the same instance — a mechanism no spec chapter defines.

The same blocker applies to `clean:bridge/math` (Platform 02 §2.2.1 routes
transcendentals and `^` through the host) — which additionally conflicts
with chapter 15's stance that math is guest computation precisely so
results cannot be host-dependent.

## Decision

The compiler emits the runtime support that MMD-01..04 requires as
**guest functions inside the emitted core module**:

- the bump allocator (`alloc(size, align) -> i32`, exported to hosts only
  through the Canonical ABI `cabi_realloc` shape), growing memory per
  TIER-02 and trapping — never returning a failure value — when the tier
  limit is exceeded (MMD-02);
- arena save/restore around the boundaries §3.2.3 names (per-request on
  the `handle` entry point), inlined as `__heap_ptr` reads/writes (MMD-03
  semantics, O(1) pop, no destructors);
- string operations (`concat`, `compare`, `eq`, boundary lift) over the
  MMD-04 layout.

No `clean:bridge/*` import is emitted. Observable semantics — the
`__heap_start`/`__heap_ptr` globals, the fixed layout constants, trap on
failed growth, per-request reset — follow Platform 03 exactly, so a later
switch to composed bridge shims changes plumbing, not behaviour.

The string comparison primitive's convention is fixed at its definition
site: **`string_compare(a, b)` returns 0 iff the strings are equal**, with
lexicographic sign otherwise. `==`, `!=` and any ordering all derive from
that one convention (KNOWLEDGE §2: polarity inversions are silent).

## Consequences

- BLOCKED ON SPEC: BRG-05 conformance. The conflict (BRG-05 vs
  MMD-01/§3.7 vs missing `clean:bridge` WIT) is recorded as
  DISCOVERIES-M6 item 1 for a foundation brief.
- `math` transcendentals (`sin`..`tanh`, `ln`/`log*`, `exp*`, number `^`)
  stay on the Unsupported channel until foundation resolves guest-vs-
  bridge (DISCOVERIES-M6 item 2); the wasm-native subset (`sqrt`, `abs`,
  `floor`, `ceil`, `round`, `trunc`, `min`, `max`, `sign`, constants)
  lowers as guest instructions.
- Reachability elision of the runtime helpers is deliberately not
  attempted: the helper set is small, fixed, and always emitted, which
  keeps emission deterministic and mirrors BRG-05's always-on intent.
