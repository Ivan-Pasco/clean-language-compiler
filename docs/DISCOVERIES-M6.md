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
6a. **No `sizeof(T)` table exists.** MMD §3.4.1 sizes elements by
   `sizeof(T)` but no chapter tabulates widths. **Local adoption**
   (pinned in `mir::elem_layout` and the `tests/lists.rs` layout probe):
   `integer`/`integer:u64` and `number` are 8 bytes; narrower boundary
   integers, `boolean` and enum discriminants are 4; `string`/`bytes`/
   `list` element slots are 4-byte pointers.
6b. **No record/class layout exists anywhere.** Platform 14 (line 257)
   and chapter 14 both delegate object layout to Platform 03, which
   defines only `list`/`pairs`/`bytes`; the class chapter's changelog
   even records removing its 4-byte class-id header *in favour of* §03 —
   which never received it. Blocks object codegen. **Local adoption for
   record-valued list elements only:** fields packed in declaration
   order at natural alignment, element stride rounded to the widest
   leaf.
6c. **RUN013 is specified as catchable (`onError`) but error lowering
   does not exist yet** — an out-of-range index currently traps
   (`unreachable`). To revisit with the error-handling stage.
6d. **Conversion semantics under-specified in 15 §Conversions.** (a)
   `number.toString()` — "literal form" names no formatting contract
   (shortest round-trip vs fixed precision); lowering declines until a
   ruling. (b) `string.toBoolean()` is accepted by the M4 checker but the
   table defines `toBoolean` only from `integer`/`number`; lowering
   declines. (c) `toInteger`/`toNumber` failures are RUN003, specified
   catchable — they trap until error lowering (same shape as 6c). (d)
   `string.toNumber` parses by naive f64 accumulation; the spec names no
   rounding contract for decimal parsing (matters for RUN007/L5 later).
6e. **`math.round` rounding mode unstated.** 15 says "round nearest";
   wasm `f64.nearest` ties to even (2.5 → 2.0), most languages' `round`
   ties away from zero. **Local adoption:** ties-to-even (the wasm
   instruction), pinned in `tests/stdlib_math_conversions.rs`. Also:
   `math` domain errors (`sqrt(-1)` → NaN?) remain unspecified — wasm
   semantics (NaN) adopted; and `math.max/min` NaN behaviour follows the
   wasm instructions.
6f. **String indexes and lengths have no unit** (15 §String Module).
   `length()`, `charAt`, `charCodeAt`, `indexOf`, `lastIndexOf`,
   `substring`, `padStart/End` all take or return indexes, and the
   chapter never says whether they count bytes, UTF-16 units (the
   JS-flavoured `charCodeAt` name suggests them) or code points.
   **Local adoption: code points**, the only Unicode-coherent choice
   over MMD-04's UTF-8 layout, pinned throughout
   `tests/stdlib_string.rs` with multi-byte fixtures. Also adopted with
   fixture pins, each individually undecided in the spec: `substring`
   clamps (end < start → `""`), `trim` whitespace is ASCII {space, \t,
   \n, \r}, `replace("")` and `padStart` with `""` return the receiver,
   `split("")` yields one element, and `charAt`/`charCodeAt` out of
   range trap (RUN013's catchable form waits on error lowering).
   `toUpperCase`/`toLowerCase` are typed but blocked: Unicode case
   folding is `clean:bridge/string` territory (item 1).
6g. **`list.add`/`insert` are jointly incoherent with §3.4.1 and
   aliasing.** The layout stores elements inline in one object; `add` on
   a full list must relocate it; and nothing in chapter 4 defines
   assignment semantics for lists (value copy vs reference), so after
   `b = a; a.add(x)` a relocating `add` leaves `b` pointing at the stale
   object — silently wrong data, not even a trap. Pick two of {inline
   layout, in-place growth, reference aliasing}. **Local state:**
   `add`/`insert` are typed but decline to lower; every other mutation
   (`set`, `remove(i)`, `removeLast`, behavior `remove()`) is in-place
   and relocation-free, hence aliasing-safe. Blocks: growable
   collections in user code (compiler-internal growth is unaffected —
   fresh unaliased objects can relocate freely). Also adopted with
   fixture pins: `slice` clamps like `substring`; `sort` is ascending
   over `integer`/`number`/`string` elements only (record ordering
   undefined); list search equality is scalar/string only (record
   equality undefined).
6h. **`b"..."` literals exist in §14.14.2 but not in the grammar.** The
   compiler contract's lexer row requires `b"..."` and hex-escaped
   forms; `grammar/03-lexical-structure.ebnf.md` has `bytes` only as a
   type keyword, with no literal production. DOC-15 makes the EBNF the
   syntax authority, so **no literal is implemented** — bytes values
   originate from `bytes.fromText` and host returns until the grammar
   gains the production.
6i. **§14.14.2's bytes table conflicts with chapter 15** on naming and
   shape: `string.to_bytes(encoding)`/`bytes.to_string(encoding) ->
   result<…>` vs `bytes.fromText(text)`/`bytes.toText(data) -> string?`
   (no encoding parameter, optional not result). **Adopted: chapter 15**
   (the stdlib chapter owns the surface; 15's rationale note also
   explains why `toText` is optional). `bytes.slice` clamps like
   `substring` (pin in `tests/stdlib_bytes.rs`); `toText` on well-formed
   input returns the receiver aliased (both types immutable, layouts
   identical).
6j. **The host-backed stdlib modules have no import path.** `console`
   maps to `clean:bridge/console` (blocked, item 1); `file`/`http`/
   `time` map to `wasi:*` interfaces — but the request delivers exactly
   one `target_world`, the vendored server world restates no `wasi:*`,
   pass [9] validates only `clean:host/*` call sites, and no chapter
   says how a stdlib-originated wasi import appears in the world or
   which code fires when it cannot. `datetime` additionally has no
   Platform 03 layout. All four module surfaces stay frontier notes.
6k. **`sse.start` is undeclarable from Clean.** LBS-02's name
   projection is mechanical camelCase→kebab, `start` is a reserved
   keyword (chapter 18 async), and the `host function` grammar has no
   explicit-WIT-name clause — so no Clean declaration can reach the
   `start` function of the vendored `sse` interface, and the `/events`
   acceptance route is **blocked by spec**. Needs either a wit-name
   escape in the declaration grammar or keyword-lenient identifiers
   after `host function` (both grammar changes, DOC-15). Same hazard for
   any WIT function whose kebab name round-trips to a Clean keyword.
6l. **Fallible-import adoptions** (framework 09 §8 round-trip). Ratified
   text says declarations carry the ok type and the compiler owns the
   `result<T, E>` reading; adopted mechanics, all fixture-pinned in
   `tests/fallible_calls.rs`: the world (not the declaration) decides
   fallibility; expression `onError` binds no error value and the error
   payload is never read (only payload-less enum/variant errors are
   supported — others decline); a bare fallible call traps on the error
   arm (RUN018's shape until error lowering); composed-bridge interfaces
   resolve their qualified package from the world (the `/counter`
   shape), which also fixes how a multi-package `target_world.wit` is
   delivered — main package unbraced, nested packages braced.
6m. **`any` has no runtime representation anywhere** — chapter 15's JSON
   mapping requires `any`-held scalars, `list<any>` and
   `pairs<string, any>`, and TYP-02 fixes the checking semantics, but no
   chapter gives `any` a layout (the §3.4 family gap again). **Local
   adoption: ADR 0005** — the 16-byte tagged box, the shared static
   `none` box, box/unbox coercions at TYP-02 fit sites (mismatched
   unboxing traps — the RUN005 family until error lowering), and JSON
   number fidelity via source-text-carrying boxes (which also sidesteps
   the unspecified number-formatting contract of 6d(a) for round-trip
   purposes). `pairs<K, V>` is implemented only as the JSON object shape
   (`pairs<string, any>`); the §3.4.2 by-pointer layout leaves scalar
   keys/values unrepresentable without boxing, which is its own spec
   question.
6n. **L5 conformance findings.** (a) The playbook's "y_* MUST be
   accepted" collides with RUN009's Accepted duplicate-key rejection:
   two upstream y_ files must fail here, recorded as expected
   divergences in `tests/conformance/json/SOURCE.md`. (b)
   `json.tryTextToData` cannot distinguish parsing the valid document
   `null` from a parse failure — both return `none`; chapter 15's
   wording defines them identically. Needs a ruling (a distinct sentinel
   or an errata acknowledging the conflation). (c) The L5 gate can cover
   only JSON in M6: chapter 15 specifies no TOML/URL/YAML/regex parser
   modules, so the master plan's JSON→TOML→URL→YAML→regex chain has one
   implementable link; the corpus registry rows for the others remain
   dormant (and that registry doc is itself stale post-ADR-0010,
   naming retired decisions-files and `json.parse`/`json.stringify`
   instead of the specified names).
7. **Range `iterate` semantics are example-specified only** (chapter 12,
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
