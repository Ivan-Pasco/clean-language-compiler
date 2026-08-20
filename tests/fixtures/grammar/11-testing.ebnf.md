# 11 testing — Grammar

Companion grammar file for [11 — Testing](../11-testing.md). Defines the shape of the `tests:` block and its three test forms — single-line named, single-line anonymous, and multi-statement block. Semantic rules TST-01 and TST-02 live in the companion chapter; TST-02 is a `cln test` runner rule with no grammar surface.

The distinguishing token between the single-line named form and the block-test form is the `:` — single-line named uses `"description": expression == expected`, block-test uses `"description"` on one line then an indented body.

---

## 1. `tests:` block

```ebnf
(* TST-01: tests: block.  Body is a sequence of test declarations
   (any of the three forms below), one per line for single-line
   tests, or a description-plus-indented-body for block tests. *)

TestsBlock      = "tests", ":", NEWLINE, INDENT, TestsBody, DEDENT ;

TestsBody       = TestDeclaration, NEWLINE,
                  { TestDeclaration, NEWLINE } ;
                  (* TestsBody is the block's interior — the body
                     08-file-structure.ebnf.md's TestsSection
                     delegates to.  At least one test declaration:
                     an empty tests: section is dead weight. *)

TestDeclaration = NamedTest
                | AnonymousTest
                | BlockTest ;
```

## 2. Single-line tests

```ebnf
(* TST-01 form 1: named single-line test.
     "description": expression == expected
   The colon after the string literal distinguishes this form from
   the BlockTest, which has no colon. *)

NamedTest       = StringLiteral, ":", TestAssertion ;

(* TST-01 form 2: anonymous single-line test.
     expression == expected  *)

AnonymousTest   = TestAssertion ;

(* Grammar admits any Expression as a test assertion; the checker
   validates that the top-level operator is a comparison (== is
   canonical per TST-01, but !=, is, not are also used in the
   chapter's Best Practices examples).  Tightening to
   `Expression "==" Expression` would reject valid tests. *)

TestAssertion   = Expression ;
```

## 3. Block tests

```ebnf
(* TST-01 form 3: block test.  Description WITHOUT a colon on the
   header line, then an indented body containing any number of
   statements and at least one `assert` line.  Test passes when
   every assert holds. *)

BlockTest       = StringLiteral, NEWLINE, INDENT,
                  BlockTestBody, DEDENT ;

BlockTestBody   = { Statement, NEWLINE }, AssertStatement, NEWLINE,
                  { Statement, NEWLINE | AssertStatement, NEWLINE } ;
                  (* At least one AssertStatement required.  A block
                     test with zero asserts is a defect — almost
                     always a maintenance mistake where the assertion
                     was deleted and the test forgotten.  If a
                     placeholder is genuinely wanted, write
                     `assert true`. *)

AssertStatement = "assert", Expression ;
                  (* `assert` is a hard keyword per LEX-04.
                     Takes exactly one boolean expression. *)
```

---

## Changelog

- 2026-08-20 — Erratum from the compiler's Milestone 9 (`clean-language-compiler/docs/DISCOVERIES-M9.md` §1, item 1d): 08-file-structure.ebnf.md's `TestsSection` delegates its body to a `TestsBody` this file never defined (only `TestsBlock`, header included, existed). `TestsBody` is now a named production — `TestDeclaration, NEWLINE, { TestDeclaration, NEWLINE }` — and `TestsBlock` is restated over it. Same language; no test that parsed before parses differently.
- 2026-08-07 (afternoon) — Resolved both `⚠` markers: (a) `TestAssertion` stays as any Expression with a checker validating the top-level operator (tightening to `==` only would reject valid tests using `!=`, `is`, `not`); (b) zero-assert block test is a defect — almost always a maintenance mistake where the assertion was deleted; grammar requires at least one `AssertStatement`. No production change (both were already the encoded position).
- 2026-08-07 — File minted. Productions derived from TST-01 and TST-02 in [11-testing.md](../11-testing.md) Accepted 2026-08-01.

---

## Metadata

- **Status:** Accepted (2026-08-07)
- **Audience:** Parser implementers; anyone writing `tests:` blocks
- **Notation:** EBNF (ISO/IEC 14977)
- **Part of:** [04 language / grammar / README.md](./README.md)
- **Rules referencing this grammar:** [11-testing.md](../11-testing.md) (TST-01, TST-02)
- **References:** [03-lexical-structure.ebnf.md](./03-lexical-structure.ebnf.md), [06-expressions.ebnf.md](./06-expressions.ebnf.md), [07-statements.ebnf.md](./07-statements.ebnf.md) (Statement)
