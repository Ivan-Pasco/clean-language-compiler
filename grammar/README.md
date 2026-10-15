# The Clean grammar

An EBNF grammar of Clean, in the Lark dialect, **derived** from the
examples of the language specification. It is not a source of truth: the
specification in the sibling repository `clean-language-foundation`
(`specs/`) is. When the two disagree, the specification wins, and the
grammar (or a question to the specification's owner) follows.

## What is here

| Path | What it is |
|------|------------|
| `grammar/<chapter>.md` | One chapter of the grammar per chapter of foundation, its productions in ```` ```lark ```` fences, with the prose that ties them to the specification. |
| `grammar/corpus.json` | The manifest: every ```` ```clean ```` example of foundation, sealed by content, with the result the grammar must give. |
| `tools/grammar/extract.py` | Reads foundation, rebuilds the manifest, reports what changed. |
| `tools/grammar/check.py` | Composes the grammar, builds it LALR(1), parses every case. |
| `crates/clean-compiler/tests/grammar_conformance.rs` | Runs every case through the real lexer and parser; its disagreements are pinned in `tests/snapshots/grammar/parser_disagreements.snap`. |
| `.claude/skills/grammar-sync/SKILL.md` | The procedure that brings all of it up to date after foundation changes. |

The chapters compose in this fixed order; a later chapter adds
alternatives to an earlier rule with `%extend`:

lexical, file-structure, expressions, statements, control-flow, functions,
classes, contracts, errors, state, testing, ai, modules, async,
apply-blocks, method-syntax, block-handlers.

Rules of the grammar:

- Terminals are in `lexical.md` (and a few chapter-local ones: `SIZE_SUFFIX`,
  `SPEC_PATH`, `IMPORT_PATH`). Indentation is one tab per level; the
  indenter in `check.py` turns it into `_INDENT` / `_DEDENT` and rejects
  spaces (`SYN003`).
- `expressions.md` has one production per level of the precedence table.
- There are no semantic predicates. What the parser accepts and the checker
  rejects (a `SEM…`, `FUNC…`, `CLASS…` rule) is accepted here.
- Every production is backed by at least one example that parses through
  it; `check.py` reports a production no example uses, which is surplus.

## Entries

| Entry | Start rule | Used for |
|-------|------------|----------|
| `file` | `file` | A whole file: sections in their order. |
| `fragment` | `fragment` | A sequence of statements: an example that is not a whole file. |
| `expression` | `expression_entry` | One expression on one line. |
| `outer_block` | `outer_block` | One generic library block, its body as token lines (examples under `components/`). |
| `identifier` | `identifier_entry` | One name (the lists of valid and invalid identifiers). |

`extract.py` picks the entry of each case and records it in the manifest;
its heuristic is documented next to `entry_of`.

## The manifest

`corpus.json` holds one object per case:

| Field | Meaning |
|-------|---------|
| `file`, `line` | Where the example is in foundation `specs/` (the line of its first code line). |
| `sha256` | The seal: SHA-256 of the example's code as written. An edited example is a seal that disappears and one that is born in the same file. |
| `expected` | `passes`, `fails:SYNxxx` (`fails:SYN` when the specification names no code) or `illustrative`. |
| `chapter` | The grammar chapter the example belongs to (`-` for files without a chapter: type system, semantic rules, standard library). |
| `entry` | The entry it is parsed with. |
| `code` | The code that is parsed: the example with tabs restored (rule R8 of `extract.py`); for `outer_block`, its first block. |
| `checker` | When the example is a failing example of a checker rule (`SEM001`, `FUNC006`, …): that code. The grammar must accept it. |
| `reason` | Why an `illustrative` case is not checked: `ellipsis`, `mixed`, `metavariable`, `components`, `outside language/`. |
| `part` | When one example holds several files (`// file:`), the index of this file. |

What each `expected` means:

- **passes** — the grammar and the parser must accept it.
- **fails:SYNxxx** — the grammar and the parser must reject it; any syntax
  error will do.
- **illustrative** — not checked: a template, a fragment with code left
  out, a block that mixes valid and invalid lines, or a `components/` block
  that is not a library block.

The relevance rules R1–R9 that decide this are at the top of `extract.py`;
they live in the script, never in foundation.

## Regenerating

```sh
python3 -m venv "$TMPDIR/grammar-venv"
"$TMPDIR/grammar-venv/bin/pip" install -r tools/grammar/requirements.txt

python3 -I tools/grammar/extract.py ../clean-language-foundation/specs grammar/corpus.json           # what changed
python3 -I tools/grammar/extract.py ../clean-language-foundation/specs grammar/corpus.json --write   # record it
"$TMPDIR/grammar-venv/bin/python" -I tools/grammar/check.py                                          # grammar vs corpus
cargo test -p clean-compiler --test grammar_conformance                                              # parser vs corpus
```

`extract.py --check` exits non-zero when the manifest is not up to date
with foundation. `check.py` exits non-zero on any LALR conflict, any case whose result
differs from `expected`, and any production no example backs. Both run in
the `grammar` workflow (`.github/workflows/grammar.yml`), informational for
now.
