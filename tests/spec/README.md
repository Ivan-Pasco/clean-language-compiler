# Per-chapter spec suite (M3 parse, M4 typecheck)

One directory per EBNF grammar chapter in
`../clean-language-foundation/04 language/grammar/`; each `.cln` fixture
runs through the whole front-end (lex, parse, resolve, typecheck against
an empty world) by `crates/clean-compiler/tests/spec_suite.rs` and is
pinned as an AST + diagnostics + unsupported-frontier snapshot under
`tests/snapshots/spec/`. Since M4, chapters 04/06/09/10/12/13/14/17
type-check clean (deliberate-error fixtures aside); the remaining
UNSUPPORTED entries are the pre-M6 lowering frontier, not typing gaps.
CI fails on drift (spec snapshot-drift gate); regenerate deliberately
with `INSTA_UPDATE=always cargo test --test spec_suite` and review every
diff — a surprising snapshot is a design question.

Chapter 16 (method-style syntax) has no directory by design: its grammar
file defines no productions — method calls are `PostfixExpression` shapes
covered by `06-expressions/`.
