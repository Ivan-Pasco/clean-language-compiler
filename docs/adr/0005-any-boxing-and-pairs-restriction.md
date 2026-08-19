# ADR 0005 — The `any` box, JSON number fidelity, and the pairs subset

Status: Accepted (2026-08-18)

## Context

Chapter 15's JSON module maps objects to `pairs<string, any>`, arrays to
`list<any>` and scalars to `any`-held values, and chapter 4 gives `any`
TYP-02 semantics — but no chapter defines a runtime representation for
`any` (Platform 03 §3.4 stops at `list`/`pairs`/`bytes`; DISCOVERIES-M6
records the family of missing layouts). The JSON round-trip regime
(quality playbook §1.9 layer 2) additionally requires
`parse ∘ serialize ∘ parse` fidelity, which naive f64 re-rendering
cannot deliver without a shortest-round-trip formatter the spec has not
contracted (DISCOVERIES-M6 item 6d).

## Decision

**The `any` box.** An `any` value is a single `i32` pointer to a
16-byte, 8-aligned heap object: `[tag: u32 @0][pad @4][payload @8]`.
Tags: 0 `none`, 1 `boolean` (i32), 2 `integer` (i64), 3 `number` (f64),
4 `string` (object pointer), 5 `bytes`, 6 `list` (object pointer,
element type `any`), 7 `pairs<string, any>` (object pointer), 8
`number-with-source` — a 24-byte box whose f64 payload sits at +8 and
whose original JSON text (a string object pointer) sits at +16. The
shared `none` box is a static constant at `NONE_BOX_ADDR`, seeded right
after the empty-string constant; `none` never allocates.

**JSON number fidelity.** The parser produces tag-8 boxes carrying the
source text; the serializer re-emits that text verbatim, making the
corpus round-trip exact by construction. Numbers computed in Clean (tag
3) serialize through the integral fast path or a 17-significant-digit
fallback — round-trip-correct, not shortest; the pretty formatter is
withheld from conformance claims until the formatting contract exists.

**Coercions.** `T → any` boxes at the site pass [5] accepted the fit
(assignment, argument); `any → T` unboxes with a tag check that traps
on mismatch (the RUN005 family until error lowering). `integer`/`number`
unboxing accepts either numeric tag (truncating toward zero or widening
respectively). `data.name` is typed as `data["name"]` (the chapter's
stated equivalence); lookups on missing keys, out-of-range indexes, or
non-container boxes yield the `none` box, never a trap.

**Pairs subset.** The §3.4.2 layout (`[count][cap][entries: key-ptr,
value-ptr]`) stores entries by pointer, so this milestone implements
`pairs` only where both sides are pointer-shaped — in practice
`pairs<string, any>`, the JSON object shape. Standalone `pairs<K, V>`
surfaces stay on the Unsupported channel. Aggregates built inside the
JSON parser use the element-tag sentinel `u32::MAX` (tags are
compiler-local and never read back at runtime).

## Consequences

- The box layout is a local adoption on top of the missing-layout family
  (DISCOVERIES-M6): when Platform 03 receives real `any`/record layouts,
  this ADR is superseded and the change is mechanical.
- RUN006–RUN010 surface as traps from guest code with their conditions
  exactly as Platform 10 states them; their catchable (`onError`) form
  arrives with error lowering, like RUN003/RUN013.
