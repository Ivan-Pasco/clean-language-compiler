# Smoke corpus

The 689 `.cln` fixtures harvested from `../clean-language-compiler-old/tests/cln`
(M3 task, plan item 4). They are **data only**: many were written for the
retired compiler's pre-spec surface, so parsing them clean is not a goal —
never port code or unspecified behaviour to make one pass.

`crates/clean-compiler/tests/corpus_smoke.rs` runs the M3 front-end
(lex + parse) over every file and classifies each as:

- `parse-clean` — zero diagnostics under the V2 grammar;
- `expected-diagnostic` — produces diagnostics and lives under a directory
  that marks it as a deliberate failure fixture (`fail/`);
- `out-of-surface` — produces diagnostics because it uses the retired
  compiler's surface (or genuinely invalid syntax that wasn't marked).

The classification snapshot lives at
`tests/snapshots/corpus/classification.snap`. Reclassifications are design
signals: an `out-of-surface` → `parse-clean` flip should be an intended
grammar gain, and any `parse-clean` → `out-of-surface` flip is a front-end
regression. The harness also proves no fixture panics the front-end.
