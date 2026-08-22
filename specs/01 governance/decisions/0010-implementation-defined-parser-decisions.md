# ADR-0010 — Home and ownership of the implementation-defined decisions document

Any parser for an external grammar hits inputs the grammar leaves open — numbers too large for the target type, duplicate object keys, lone surrogates — and an undocumented accept/reject choice is how two builds silently diverge. The testing chapter named a file path for these decisions that never existed and no directory to hold it, and four documents delegated observable behaviour to it. This ADR resolves that the document is not needed at all: the accept/reject boundary is the condition of the diagnostic that rejects each case, and it lives in the semantic-rules chapter that already owns those diagnostics.

---

## Context

Any parser for an external grammar — JSON first, others later — meets inputs the grammar itself leaves open: numbers too large for the target type, duplicate object keys, lone surrogates, deeply nested structures. JSONTestSuite calls these the `i_*` cases: *implementation-defined*. Each one forces a choice between accept, reject, and accept-with-normalisation, and an undocumented choice is how two builds of the same parser silently diverge.

[11 — Testing](../../04%20language/11-testing.md) already requires that these choices be pinned in writing, one file per format, and names the path `foundation/spec/stdlib/json/implementation-defined.md`. **That file does not exist, and neither does a `foundation/` directory in this repository.**

Four documents nonetheless delegate observable behaviour to it: [15 — Standard Library](../../04%20language/15-standard-library.md) ("fixed by the pinned decisions document"), and the rule bodies of `RUN007`, `RUN009` and `RUN010` in [10 — Semantic Rules](../../03%20platform/10-semantic-rules.md) ("the exact list of accepted vs. rejected number forms", "duplicate object keys under the strict pinned decision"). The accept/reject boundary of three registered diagnostics is therefore undefined anywhere in the repository — a gap in the error path ([SDD-04](../03-spec-driven-design.md)) and a fact with no home at the same time.

What must be decided is *where the document lives* and *who owns it*, not what the individual decisions are.

## Options considered

**A — A section inside [15 — Standard Library](../../04%20language/15-standard-library.md), one per parser module.** The decisions sit beside the module they govern, and the spec stays self-contained. Cost: the chapter grows a long table of edge cases that most readers never need, and every new format adds to it.

**B — One ADR per format.** Fits the nature of the content: these are genuine choices between viable alternatives, they must not be re-litigated silently, and ADRs are append-only, which is exactly the property "pinned" is asking for. Cost: the accept/reject boundary of a diagnostic would live outside the spec tree, so `RUN007` would cite an ADR rather than a spec section.

**C — A conformance repository, alongside the driver and host conformance suites.** Puts the decisions next to the corpus that tests them, which is where a maintainer actually works. Cost: the observable boundary of a shipped diagnostic would live in a different repository from the specification that names it, which the single-home discipline and [SDD-09](../03-spec-driven-design.md) both push against.

**D — A dedicated numbered chapter in `04 language/`.** One home for all formats, indexed, versioned with the spec. Cost: a new chapter whose content is almost entirely tables.

## Decision

**None of the four — the document does not need to exist.** The decisions live in the **condition of the rule that rejects each case**, in [Platform 10 — Semantic Rules](../../03%20platform/10-semantic-rules.md), which is already the declared home of `RUN007`, `RUN009` and `RUN010`.

All four options assume the answer is a separate artefact and then argue about where to put it. It is not one. An accept/reject boundary is precisely what a diagnostic's condition states, and [RUL-01](../../03%20platform/10-semantic-rules.md#rul-01--mandatory-entry-format) already requires every rule in that catalogue to carry a condition, a message template and worked examples. The three rules had been written as summaries that deferred to a file, which is a fact with no home wearing the shape of a fact with two.

Locating it this way also settles what the options traded against each other. There is no long table in a language chapter that most readers skip (option A), because the material sits with the diagnostic a reader arrived by. Nothing moves outside the specification tree (options B and C), which is what [SDD-09](../03-spec-driven-design.md) pushed against. No new chapter of tables is created (option D). And the "one file per format" concern answers itself: a second parser brings its own codes, and its cases become their conditions.

**The two genuinely open cases are settled in the same change**, since the ADR's premise — that only the location was in question — turned out to leave a smaller residue than expected:

- **`-0` is accepted**, yielding binary64 negative zero. It is a representable value of the target type and rejecting it would refuse input that round-trips.
- **Duplicate object keys are rejected** with `RUN009`. Last-wins and first-wins are equally arbitrary, and both discard data the input carried without the program ever learning two values were offered. Rejection is the only resolution that loses nothing silently, and it is the stance the ecosystem already takes on ambiguous input — `RQD002` refuses an unknown key rather than ignoring it.

Nesting depth needed no decision: it was already fixed at 1000 levels.

## Consequences

The path `foundation/spec/stdlib/json/implementation-defined.md` is retired, along with the `foundation/` directory it implied. [11 — Testing](../../04%20language/11-testing.md) had named it as a live location for a file that has never existed.

The accept/reject boundary of three registered diagnostics becomes conformance-testable, which was the gap [SDD-04](../03-spec-driven-design.md) recorded. A corpus asserts against the rule conditions directly, with no second document to keep in step.

Uniformity across hosts holds for a reason already decided rather than asserted: the parser is compiled to WASM once and is deliberately not routed through the bridge, so no host can widen or narrow the boundary.

---

## Metadata

- **Status:** Accepted (2026-08-02)
- **Date:** 2026-08-01
- **Supersedes:** None
- **Spec impact:** [03 platform / 10 — Semantic Rules](../../03%20platform/10-semantic-rules.md) (`RUN007`, `RUN009`, `RUN010` conditions written in full) · [04 language / 15 — Standard Library](../../04%20language/15-standard-library.md) (§JSON Module) · [04 language / 11 — Testing](../../04%20language/11-testing.md)
