# M3 discoveries — spec gaps found while building the front-end

Findings that belong to the foundation specs, recorded here per the working
rule (spec gaps become task briefs in foundation `work/`, written from a
foundation session — never patched silently from this repo).

## Status

All six items were taken to foundation on 2026-08-17 from a foundation
session; **foundation-side commit pending** (hashes to be recorded here
once it lands):

- [x] Item 1 — erratum in `21-block-handlers.md`: example corrected to
  `expandDataBlock(BlockAST ast)` in both real occurrences (§21.1 and
  §21.5 — this file originally miscited the second as §21.6; the
  foundation changelog records the discrepancy).
- [x] Item 5 — erratum in `03-lexical-structure.ebnf.md` §8: `Caret = "^"`
  row added, pointing at EXP-01 level 4.
- [x] Item 2 — brief `work/2026-08-17-block-ast-statement-classification.md`.
- [x] Item 3 — brief `work/2026-08-17-block-attribute-recognition.md`.
- [x] Item 4 — brief `work/2026-08-17-library-block-header-grammar.md`.
- [x] Item 6 — brief `work/2026-08-17-fatal-path-info-code.md`.

Side finding from that session, foundation-owned: `check-docs-compliance.py`
reports 859 pre-existing warn-only hard failures across the tree, which
contradicts the recorded "repo at zero since 2026-08-01" — likely the
checker hardened later; to be investigated from a foundation session.

## 1. Chapter 21's canonical handler example is unparseable under its own rules

`21-block-handlers.md` §21.1 and §21.5 both show (this file first cited
the second occurrence as §21.6; the real home is §21.5, Span
Preservation):

```clean
compiletime function expandDataBlock(block BlockAST) returns IR
```

Two conflicts with Accepted specs:

1. **Parameter order.** The parameter is written name-first
   (`block BlockAST`), but `ParameterList` (`09-functions.ebnf.md`,
   referenced by `21-block-handlers.ebnf.md` §1) is type-first:
   `Parameter = TypeExpression, Identifier, …` — i.e. `BlockAST block`.
   DOC-15 makes the grammar authoritative, so the compiler parses
   type-first; the chapter's examples need erratum.
2. **`block` as a parameter name.** `block` is a LEX-04 hard keyword;
   using it as an identifier is SYN002. Even reordered, the example's
   parameter name is illegal. The chapter needs a different name
   (`ast`, `input`, …) or LEX-04 needs to drop `block` — the former looks
   intended, since `handles block` is the only grammar use of the keyword.

## 2. BlockAST `Statement` body nodes cannot be produced at parse time

`schema/block-ast.md` says a `BlockNode` may be a `Statement` — "a normal
Clean statement, **already typed**". At parse time (pass [3]) nothing is
typed yet, and the parser cannot know which lines of a DSL body are Clean
statements versus handler-tokenised `BlockLine`s without asking the handler
(which only runs in pass [6]). M3 therefore parses every non-block body
line as a `BlockLine` and defers `Statement` materialisation to expansion
(M5). The schema should state which component performs that classification
and when.

## 3. `BlockAttribute` recognition is unspecified

`schema/block-ast.md` defines attributes as "a keyword-prefixed line at the
top of the block body" and shows `deprecated "Use ExtendedUserData
instead"`. There is no grammar rule distinguishing an attribute line from
an ordinary DSL `BlockLine` (both are identifier-headed token runs), and no
registry of attribute keywords. M3 leaves `attributes` empty and preserves
such lines as `BlockLine`s; the schema needs either a closed attribute
keyword set or a syntactic marker.

## 4. Block-header arguments outside parentheses

`schema/block-ast.md` says block arguments are "passed in parentheses",
but the chapter's own example `data UserData:` passes `UserData` bare in
the header (no parentheses), and `08-file-structure.ebnf.md` §3's
`LibraryBlock` production shows only `Identifier ":"` with no argument
surface at all. M3 parses both parenthesized argument lists and bare
header expressions as `arguments`; the grammar file should pin the header
shape down.

## 5. `03-lexical-structure.ebnf.md` §8 omits the `^` operator token

The token list in §8 has no `Caret`/`^` entry, yet `06-expressions.ebnf.md`
level 4 uses `"^"` for exponentiation. The lexer recognises `^`; §8 needs
the missing row.

## 6. The §10.4 fatal-path `info` diagnostic has no registered code

Platform 13 §10.4 requires that after a fatal diagnostic the compiler
"emits one final `info` diagnostic explaining that further checking was
skipped". DIA-01 requires every diagnostic to carry a registered code, and
Platform 09 registers no code for this notice — so the fatal path as
specified cannot be implemented without inventing a code. M3's parser
treats every syntax error as recoverable (13 §10.1's parse-fatal case,
"syntax error in the first three lines that prevents identifying the file
as Clean source", is also heuristically under-specified); the fatal path
needs a registered code and a testable trigger condition before the
compiler can honour it.
