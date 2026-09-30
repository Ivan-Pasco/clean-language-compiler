# 11 testing — Grammar

**Title:** 11 testing — Grammar

**Id:** `TSTG-`

**Purpose:** Companion grammar file for 11 — Testing, written for the implementors of the parser and for readers of the testing chapter who need the exact shape behind its examples. It defines the `tests:` block — whose interior is the body 08-file-structure.lark.md's TestsSection delegates to, with at least one declaration — and its three test forms: the single-line named test, the single-line anonymous test, and the multi-statement block test with its `assert` statement and its requirement of at least one assert. It fixes what tells the forms apart — the `:` after the description in the named form, absent in the block form — and why the assertion admits any expression rather than only `==`, leaving the check of the top-level operator to the checker. The semantic rules of tests live in the companion chapter (`TST-`), not here.

---

### TSTG-01 — 1. `tests:` block

**Id:** TSTG-01

**Kind:** rule

**Name:** 1. `tests:` block

**Question:** What productions give the `tests:` block and the interior that 08-file-structure.lark.md's TestsSection delegates to their shape, and how many test declarations must that interior hold?

**Parent:** LANG-17

**Norm:**

```lark
// TST-01: tests: block.  Body is a sequence of test declarations (any
// of the three forms below), one per line for single-line tests, or a
// description-plus-indented-body for block tests.

tests_block: _TESTS ":" _NEWLINE _INDENT tests_body _DEDENT

// tests_body is the block's interior — the body
// 08-file-structure.lark.md's tests_section delegates to.  At least
// one test declaration: an empty tests: section is dead weight.  A
// single-line test is followed by its _NEWLINE; a block test ends with
// the _DEDENT of its body, the _NEWLINE of its last line coming before
// that _DEDENT (LEXG-02), so no _NEWLINE follows it.

tests_body: test_declaration+

test_declaration: named_test _NEWLINE
                | anonymous_test _NEWLINE
                | block_test
```

**Why:** the tests: block is a sequence of test declarations in any of the three forms, and TestsBody is published on its own because 08-file-structure.lark.md's TestsSection delegates to it; at least one declaration is required since an empty tests: section is dead weight.

---

### TSTG-02 — 2. Single-line tests

**Id:** TSTG-02

**Kind:** rule

**Name:** 2. Single-line tests

**Question:** What productions give the single-line named and anonymous test forms their shape — what token separates the named form from a block test, and which expressions may stand as a test assertion?

**Parent:** LANG-17

**Norm:**

```lark
// TST-01 form 1: named single-line test.
//     "description": expression == expected
// The colon after the string literal distinguishes this form from the
// block_test, which has no colon.

named_test: string_literal ":" test_assertion

// TST-01 form 2: anonymous single-line test.
//     expression == expected

anonymous_test: test_assertion

// The grammar admits any expression as a test assertion; the checker
// validates that the top-level operator is a comparison (== is
// canonical per TST-01, but !=, is, not are also used in the chapter's
// Best Practices examples).  Tightening to expression "==" expression
// would reject valid tests.

test_assertion: expression
```

**Why:** the colon after the string literal is what distinguishes a named single-line test from a block test, so the production carries it; the assertion admits any Expression because !=, is and not appear in the chapter's own examples, and tightening to `==` would reject valid tests.

---

### TSTG-03 — 3. Block tests

**Id:** TSTG-03

**Kind:** rule

**Name:** 3. Block tests

**Question:** What production gives the multi-statement block test its shape — how is its description line written and how many `assert` statements must its indented body contain?

**Parent:** LANG-17

**Norm:**

```lark
// TST-01 form 3: block test.  Description WITHOUT a colon on the
// header line, then an indented body containing any number of
// statements and at least one `assert` line.  Test passes when every
// assert holds.

block_test: BLOCK_TEST_DESCRIPTION _NEWLINE _INDENT block_test_body _DEDENT

// The description is a single-line string literal ending its line.  A
// string literal alone on a line of tests_body is therefore always a
// block test's description, and the parser never needs a second token
// of lookahead to tell it from an anonymous test (a bare string is not
// a comparison, so no anonymous test is lost).

BLOCK_TEST_DESCRIPTION.2: STRING_LITERAL /(?=[ \t]*(\/\/[^\n]*)?\n)/

// At least one assert_statement is required.  A block test with zero
// asserts is a defect — almost always a maintenance mistake where the
// assertion was deleted and the test forgotten.  If a placeholder is
// genuinely wanted, write `assert true`.  Each statement carries its
// own line end (07-statements.lark.md, STMG-01).

block_test_body: statement* assert_statement _NEWLINE (statement | assert_statement _NEWLINE)*

// `assert` is a hard keyword per LEX-04.  Takes exactly one boolean
// expression.

assert_statement: _ASSERT expression
```

**Why:** a block test has a description without a colon and an indented body of statements with at least one `assert`, so BlockTestBody requires one AssertStatement: a test with zero asserts is almost always a deleted assertion, and a genuine placeholder is written `assert true`.

---
