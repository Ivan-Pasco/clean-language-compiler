# 04 language / grammar — companion grammar files

This folder holds the source-of-truth grammar productions for the Clean Language. One file per syntax-bearing chapter of `04 language/`, at the fixed location `<chapter-slug>.ebnf.md`, per [DOC-15](../../01%20governance/00-documentation-principles.md#doc-15--grammar-is-the-source-of-truth-for-syntax-specs-cite-it). Each file wraps EBNF productions in a Markdown document with a metadata trailer per [DOC-19](../../01%20governance/00-documentation-principles.md#doc-19--doc-type-metadata-trailers-are-required); the productions themselves live in fenced `ebnf` code blocks.

Spec chapters cite productions by name — `see FunctionDeclaration in 09-functions.ebnf.md` — they do not restate the grammar in prose. A production defined in more than one place, or a syntax-bearing chapter without a companion file, is a defect per DOC-15.

---

## Notation

All grammar files use **EBNF as specified in ISO/IEC 14977**, with these repo conventions:

| Symbol | Meaning |
|---|---|
| `=` | Definition (`ProductionName = ...`) |
| `,` | Concatenation |
| `\|` | Alternation |
| `[ x ]` | Optional (zero or one) |
| `{ x }` | Repetition (zero or more) |
| `( x )` | Grouping |
| `"literal"` | Terminal string literal |
| `? description ?` | Special sequence — an informal terminal description (e.g. `? any ASCII letter ?`) |
| `(* comment *)` | Comment |
| `;` | Rule terminator (optional in this repo; used for clarity in multi-line productions) |

Line terminators end statements in the language (see [LEX-07 line-terminators](../03-lexical-structure.md#lex-07--line-terminators)), so productions describing statement-level syntax use the terminal `NEWLINE` for the physical line terminator and `INDENT` / `DEDENT` for indentation events. These three are lexical terminals produced by the lexer, not literal source characters.

Every production references only terminals defined in `03-lexical-structure.ebnf.md` or non-terminals defined earlier in the same file or in a chapter this file's companion cites as `Cites grammar:`.

---

## Files in this folder

Populated during Stage 2a of the Docs Readiness Program (see [`work/2026-08-07-docs-readiness-program.md`](../../work/2026-08-07-docs-readiness-program.md)). Files land in dependency order:

- **Batch 1 (foundational):** `03-lexical-structure`, `04-type-system`, `05-apply-blocks`, `08-file-structure`
- **Batch 2 (expressions and statements):** `06-expressions`, `07-statements`, `09-functions`, `12-control-flow`
- **Batch 3 (rest of core):** `10-contracts`, `11-testing`, `13-error-handling`, `14-classes-and-objects`, `16-method-style-syntax`
- **Batch 4 (extended):** `17-modules-and-imports`, `18-async`, `19-ai-integration`, `20-state-management`, `21-block-handlers`

Chapters that produce no grammar file (catalogue-only or purely narrative): `01-overview.md`, `02-language-design-rules.md`, `15-standard-library.md`.

---

## Judgment-call markers

An `⚠` marker inside a fenced grammar block flags a production where the source chapter left the syntax underspecified and the grammar author made a best-guess call awaiting review. Format:

```
(* ⚠ <what's ambiguous> — resolved by <the decision made> — needs review *)
```

An `⚠` marker MUST be resolved (removed or replaced with a definitive production) before the containing file's Status advances from Draft to Accepted.

---

## Metadata

- **Status:** Accepted (2026-08-07)
- **Audience:** Compiler and tool authors implementing a Clean Language parser or validator; spec editors adding or amending syntax
- **Part of:** [04 language / README.md](../README.md)
- **References:** [DOC-15](../../01%20governance/00-documentation-principles.md#doc-15--grammar-is-the-source-of-truth-for-syntax-specs-cite-it), [DOC-19](../../01%20governance/00-documentation-principles.md#doc-19--doc-type-metadata-trailers-are-required), [`work/2026-08-07-docs-readiness-program.md`](../../work/2026-08-07-docs-readiness-program.md)
