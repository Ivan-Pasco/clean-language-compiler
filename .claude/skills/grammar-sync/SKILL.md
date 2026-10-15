---
name: grammar-sync
description: Bring the derived Lark grammar (grammar/*.md), its corpus and the hand-written parser up to date with the ```clean examples of foundation specs. Use after foundation's language examples change, or when asked to sync, check or extend the grammar.
---

# grammar-sync

The examples in `../clean-language-foundation/specs` are the truth; the
grammar in `grammar/` is derived from them, and the parser in
`crates/clean-compiler/src/{lexer,parser}` follows the grammar. Read
`grammar/README.md` first.

Foundation is never edited here. An example that is ambiguous, or that
contradicts another, is not settled in the grammar: it becomes a question
for the human, with two candidate examples (✅ / ❌).

Never commit. Other sessions work in this repository: never `git add -A`,
`git add .`, `git stash`, `git checkout -- .` or `git reset --hard`.

## Setup (once per session)

```sh
python3 -m venv "$TMPDIR/grammar-venv"
"$TMPDIR/grammar-venv/bin/pip" install -q -r tools/grammar/requirements.txt
PY="$TMPDIR/grammar-venv/bin/python"
SPECS=../clean-language-foundation/specs
```

## Steps

1. **What changed.** `python3 -I tools/grammar/extract.py $SPECS grammar/corpus.json`.
   It lists, per foundation file, the cases new, changed and gone. If it
   says `manifest: up to date` and steps 3 and 5 are green, stop: nothing to
   do. Otherwise keep that list for the report, then record the manifest
   with `--write`: `check.py` and the Rust conformance test read the cases
   from `grammar/corpus.json`.
2. **Edit the grammar.** For each new or changed case, read the example and
   the prose around it in foundation, then edit the chapter of `grammar/`
   that owns the construct (the case's `chapter`, or the chapter of the
   construct for files without one). Keep the rules of `grammar/README.md`:
   productions in ```` ```lark ```` fences, `%extend` into earlier chapters,
   one production per precedence level, no semantic predicates, every
   production backed by an example. Remove a production whose last example
   is gone.
3. **Grammar until green.** `"$PY" -I tools/grammar/check.py` (add
   `--verbose` to see the code of each mismatch). Fix conflicts and
   mismatches. A mismatch caused by the example itself (a reserved word used
   as a name, a template, code that contradicts the prose) is not fixed in
   the grammar: record it as a question.
4. **Adjust the parser.** Run step 5, read
   `tests/snapshots/grammar/parser_disagreements.snap`, and change the
   lexer and parser so that each construct the grammar now accepts or
   rejects is handled the same way. Follow the hand-written recursive
   descent style of `crates/clean-compiler/src/parser/`; never add a
   parser-combinator dependency (implementation.md).
5. **Tests until green.** `cargo test -p clean-compiler --test grammar_conformance`,
   then `cargo test --workspace`. When the list of disagreements changes
   because the parser learned a construct, review the snapshot diff and
   accept it (`cargo insta review`, or `INSTA_UPDATE=always`); never accept
   a new disagreement without saying why.
6. **Confirm and report.** `python3 -I tools/grammar/extract.py $SPECS grammar/corpus.json --write`
   must report `manifest: up to date` (foundation did not move meanwhile);
   run steps 3 and 5 once more. End with three lists:
   - **Done** — chapters and parser constructs changed, with the cases they
     turned green.
   - **Pending** — cases still red in `check.py`, and disagreements left in
     the snapshot, grouped by construct.
   - **Questions for the human** — each with the foundation file:line and
     two candidate examples, ✅ and ❌.

Run again with nothing changed in foundation, the skill changes nothing:
the manifest is up to date, `check.py` and the conformance test give the
same result, and no file is written.
