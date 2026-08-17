# Per-chapter spec suite (M3)

One directory per EBNF grammar chapter in
`../clean-language-foundation/04 language/grammar/`; each `.cln` fixture is
parsed by `crates/clean-compiler/tests/spec_suite.rs` and pinned as an AST +
diagnostics snapshot under `tests/snapshots/spec/`. CI fails on drift
(spec snapshot-drift gate); regenerate deliberately with
`INSTA_UPDATE=always cargo test --test spec_suite` and review every diff —
a surprising snapshot is a design question.

Chapter 16 (method-style syntax) has no directory by design: its grammar
file defines no productions — method calls are `PostfixExpression` shapes
covered by `06-expressions/`.
