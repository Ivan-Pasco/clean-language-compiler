# Discoveries — Milestone 6 (stdlib + memory model)

Spec gaps and under-specifications found while implementing M6. Each item
either becomes a task brief in foundation `work/` (written from a
foundation session — this repo never writes there) or records a local
adoption that stays in force until foundation resolves it.

## Status

Open — milestone in progress (started 2026-08-18). Foundation HEAD at
start: `af0e34c` (the M5 round-trip closure; none of the 16 open briefs
from M3–M5 executed since).

## Items

1. **BRG-05 always-on imports are unimplementable today** (Platform 02
   §2.6 vs Platform 03 §3.2/§3.7). Three independent blockers: no
   `clean:bridge` WIT package exists anywhere a request could deliver it
   (foundation `03 platform/wit/` holds only a README; `mem.wit` is a
   planned path in 15 §237); no host provides the interfaces (the vendored
   server world restates none of them, clean-host-core implements no
   `clean:bridge/*`); and the §3.2 signatures traffic in guest heap
   addresses (`arena-push() -> i32`), which §3.7 forbids crossing the
   component boundary — they only make sense as composed core-module
   shims, a mechanism no chapter defines. Also: §3.2.1 names the allocator
   `alloc(size, align)` while BRG-05 calls it `mem-alloc` with no written
   signature. **Local adoption in force:** ADR 0004 — runtime support
   (allocator, arenas, string ops) is emitted as guest functions with
   MMD-observable semantics; no `clean:bridge/*` import is emitted.
2. **`math` lowering home contradicts itself.** Chapter 15 (§Math, and
   §Matrix line 868 for the rationale) mandates guest computation with no
   bridge crossing, precisely so results cannot be host-dependent;
   Platform 02 §2.2.1 routes transcendentals and the `^` operator through
   `clean:bridge/math` ("host implementation of `^`"). Same missing-WIT
   blocker as item 1 besides the direct conflict. **Local adoption:** the
   wasm-native subset lowers as guest instructions; transcendentals and
   number `^` stay on the Unsupported channel pending a ruling.
3. **`embedded` tier cannot allocate.** MMD-01 fixes `HEAP_START` at
   1 MiB; TIER-01 caps `embedded` at exactly 1 MiB, leaving a zero-byte
   heap — every allocation trips the tier ceiling. Latent (the tier is
   reserved, not shipped in V2), but the two Accepted numbers are jointly
   unusable.
4. **No rule covers static data larger than the fixed heap start.**
   MMD-01 says `HEAP_START` is 1 MiB "regardless of how much data the
   compiler emitted"; a program with ≥ ~1 MiB of string literals therefore
   has no conforming layout, and no diagnostic owns the rejection (COM003
   is a RUL-03 stub with no template). **Local adoption:** the compiler
   surfaces the overflow as a COM013 internal-invariant failure until a
   code and template exist.
5. **`memory64` requests are accepted by schema but unimplementable as
   specified for 32-bit codegen** — §3.6 makes V2 *hosts* accept memory64
   guests but no chapter says what the *compiler* emits for
   `build.memory64 = true` (pointer width changes every layout in §3).
   **Local adoption:** the flag lands on the Unsupported channel at
   intake.
6. **Range `iterate` semantics are example-specified only** (chapter 12,
   FLW-02). The worked examples fix inclusivity and signed steps, but
   three cases have no normative sentence: (a) the default step when
   `from > to` — **local adoption:** descend by −1, mirroring
   `list.range(5, 1)` (pinned by
   `tests/control_flow.rs::iterate_descending_without_step_mirrors_list_range`);
   (b) evaluation cardinality of `from`/`to`/`step` — **local adoption:**
   each evaluates exactly once, before the first test; (c) `step 0` with
   `from ≠ to` — no text forbids it and the natural semantics never
   terminate; no code exists to reject it (the M4 brief
   `2026-08-17-iterate-step-non-range.md` covers step on non-range
   sources, not this). Extends that brief's questions.
