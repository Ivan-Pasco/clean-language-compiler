# ADR-0030 — Withdraw `screen` from the language and framework

The language-level `screen <Name>:` construct — a top-level section holding screen-local state — is withdrawn. The word `screen` is not a keyword of any kind (not hard, not contextual, not reserved-unused) and is not registered as a library block name either. It is a free identifier available for user code.

---

## Context

While auditing the language specification for the Docs Readiness Program (see [`../../work/2026-08-07-docs-readiness-program.md`](../../work/2026-08-07-docs-readiness-program.md)), the `screen` construct was identified as a spec defect during review of Q24 in the `⚠`-marker walkthrough. The user, on being asked whether `screen` was really part of the language, replied that they did not remember adding it and confirmed:

> "we are not going to use `screen`, we don't need it for now, and it won't be part of the language."

The evidence surveyed:

- **`screen` appears in the language spec only in one chapter** — `04 language/20-state-management.md` (SMG-01 and later sections). Every other reference in `04 language/` traces to that chapter's registration or the lexical keyword table.
- **No ADR justified the language-level status of `screen`.** The changelog for `20-state-management.md` shows the chapter was promoted from a Platform-tier rules chapter on 2026-08-01 with no decision recorded.
- **The UI library never used `screen` as a language construct.** [`02 components/framework/libraries/10-ui.md`](../../02%20components/framework/libraries/10-ui.md) — 2108 lines about pages, components, layouts, forms, and hydration — does not contain the word `screen` at all.
- **The framework specification listed `screen` in the ui library's block-ownership table** ([`01-framework-specification.md`](../../02%20components/framework/01-framework-specification.md) §374), but no library documentation defined its handler.
- **The glossary itself flagged the layering tension** between the language-level `screen <Name>:` and the ui-library `screen:` block name.
- **One diagnostic depended on the construct** — `SCOPE005 ScreenStateAccess` in [`03 platform/09-error-codes.md`](../../03%20platform/09-error-codes.md) and [`10-semantic-rules.md`](../../03%20platform/10-semantic-rules.md) §SCOPE005.

The forces at play:

- **Not needed.** The user confirmed there is no application need for `screen` in the language.
- **Layering.** `screen` is inherently a UI concept. Reserving language keywords for UI-specific concerns violates the language's target-independence.
- **Precedent.** Every other block-form with UI semantics (`page`, `component`, `layout`) is a library block handler; `screen` was an unexplained exception.
- **`LDR-08` "one way to do things."** Having both a language-level `screen <Name>:` and a library-level `screen:` block would give two ways to do UI scoping.
- **Cost of removal.** Nothing in the framework, ui library, or reference-app code depends on `screen`, so removal is cheap now. The cost of leaving it would grow every session that reads the spec and wonders what to do with it.

## Decision

Withdraw `screen` completely, from both the language and the framework.

Concretely:

1. **Remove `screen` from every keyword table** in [`03-lexical-structure.md`](../../04%20language/03-lexical-structure.md). It is not a hard keyword, not a contextual keyword, and not a reserved-unused keyword. The word is a free identifier for user code.
2. **Remove `screen <Name>:` from [`08-file-structure.md`](../../04%20language/08-file-structure.md) FIL-01's top-level section-order table.** The table shrinks from 10 slots to 9.
3. **Simplify [`20-state-management.md`](../../04%20language/20-state-management.md):** SMG-01 becomes app-scoped-only; the "State in Screens" section is deleted entirely; the Complete Example is trimmed to remove its `screen Home:` block; summary tables are updated. SMG-02..SMG-05 (guard, rules, computed, watch) are unaffected — they remain features of app-scoped `state:`.
4. **Retire [`SCOPE005`](../../03%20platform/10-semantic-rules.md).** Mark the semantic rule "withdrawn". Keep the code ID `SCOPE005` registered as withdrawn per [`DOC-13`](../00-documentation-principles.md#doc-13--stable-ids-for-checkable-rules-no-ceremony-for-prose) ("IDs are never renumbered or reused"). Update [`09-error-codes.md`](../../03%20platform/09-error-codes.md) to reflect the withdrawal.
5. **Remove the `Screen` entry from [`06-glossary.md`](../06-glossary.md).**
6. **Remove `screen` from the ui library's block-ownership table** in [`01-framework-specification.md`](../../02%20components/framework/01-framework-specification.md) §374. The ui library does not register `screen` as a block name. It retains ownership of `component`, `page`, `html`, `styles`, `ui`.
7. **Update the three paired grammar files** ([`03-lexical-structure.ebnf.md`](../../04%20language/grammar/03-lexical-structure.ebnf.md), [`08-file-structure.ebnf.md`](../../04%20language/grammar/08-file-structure.ebnf.md), [`20-state-management.ebnf.md`](../../04%20language/grammar/20-state-management.ebnf.md)) to remove `screen` from every production it appears in.

**UI-local scoping**, if an application needs it, is an application-level organisational concern. It is not a language construct and not a framework block. Applications may declare an ordinary `class` that groups view state and behaviour, or use the ui library's `page` and `component` mechanisms which already exist.

## Options considered

- **Withdraw completely** (chosen). Removes the unjustified construct from both language and framework. Cheapest possible position: no keyword reservation, no library block, no orphaned diagnostics active. Preserves `screen` as an ordinary identifier so user code can freely use it.
- **Withdraw from the language but keep as a ui-library block name** (an earlier draft of this ADR). Rejected because it kept the layering ambiguity alive — the glossary would still need to explain which `screen` was meant in which context, and future spec readers would still wonder why one library got a UI-flavoured word. The user's clarification made it clear the goal is *no `screen` construct at any layer*, not just a cleaner layering.
- **Move `screen` to reserved-unused** (an earlier draft of this ADR). Rejected as inconsistent with "not needed for now" — reserving a word for a future purpose that has no plan behind it is exactly the ceremony `DOC-17` warns against ("if a piece of metadata cannot be pointed to a consumer, it is decoration, and decoration is trimmed").
- **Write the missing ADR** justifying `screen`'s language-level status. Rejected — no downstream consumer justifies the language-level treatment; the ADR would have to invent a justification post-hoc.

## Consequences

**What becomes easier:**

- The language spec loses an unjustified UI-flavoured construct. `04 language/` is now consistently non-UI.
- The framework spec loses an orphaned ui-library block name that no library implementation defined.
- The layering ambiguity disappears from every document that used to hedge around it (glossary, chapter opening notes, ui-library block table).
- User code can freely use `screen` as an identifier (variable name, class name, parameter name) without collision.
- The 9-slot FIL-01 table is easier to memorise than a 10-slot table with a UI outlier.
- Future spec authors have one fewer edge case to reason about when writing new language rules.

**What becomes harder:**

- Any project (if one existed) that used the language-level `screen <Name>:` construct must migrate. Since no framework, ui-library, or reference-app code used it, this cost is expected to be zero in practice.
- `SCOPE005` occupies an ID slot it no longer uses. This is the intended `DOC-13` behaviour (IDs never renumber or reuse) — the cost of an orphan ID is the price paid for stable citations.

**What must now be done:**

- The paired grammar files ([`03-lexical-structure.ebnf.md`](../../04%20language/grammar/03-lexical-structure.ebnf.md), [`08-file-structure.ebnf.md`](../../04%20language/grammar/08-file-structure.ebnf.md), [`20-state-management.ebnf.md`](../../04%20language/grammar/20-state-management.ebnf.md)) are updated in the same commit as this ADR — done.
- The language-compliance audit report ([`../../reports/2026-08-01-language-compliance-audit.md`](../../reports/2026-08-01-language-compliance-audit.md)) references `screen` in its historical narrative. Reports are informative artifacts and do not need to be back-edited; readers of an old report understand it was true at the time of writing.
- If a future UI need arises for a scoping construct, it may be reintroduced under a different name (or, if `screen` is genuinely the right word, brought back with a fresh ADR that justifies its language-level status against real usage evidence).

---

## Metadata

- **Status:** Accepted (2026-08-07)
- **Date:** 2026-08-07
- **Supersedes:** None
- **Spec impact:** [`04 language/03-lexical-structure.md`](../../04%20language/03-lexical-structure.md) LEX-04 (`screen` removed from all keyword tables); [`04 language/08-file-structure.md`](../../04%20language/08-file-structure.md) FIL-01 (screen row removed); [`04 language/20-state-management.md`](../../04%20language/20-state-management.md) SMG-01 (simplified to app-scoped only, screen sections deleted); [`03 platform/10-semantic-rules.md`](../../03%20platform/10-semantic-rules.md) SCOPE005 (withdrawn); [`03 platform/09-error-codes.md`](../../03%20platform/09-error-codes.md) SCOPE005 row (marked withdrawn); [`01 governance/06-glossary.md`](../06-glossary.md) Screen entry (removed); [`02 components/framework/01-framework-specification.md`](../../02%20components/framework/01-framework-specification.md) §374 (screen removed from ui library block list); the three paired grammar files updated in the same change.
