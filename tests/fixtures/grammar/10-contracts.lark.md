# 10 contracts — Grammar

**Title:** 10 contracts — Grammar

**Id:** `CTRG-`

**Purpose:** Companion grammar file for 10 — Contracts. Defines the shape of the three contract blocks — `before:`, `after:`, `always:` — that carry preconditions, postconditions, and invariants: each is an indented body of expressions, one per line, and the three differ in where they may appear (function body vs. class body) and in evaluation semantics, not in surface shape. It also defines `contract_prelude`, the optional before-then-after pair that 09-functions.lark.md and the class method body of 14-classes-and-objects.lark.md open with, so the order is fixed once; where `always:` sits in a class body is the class grammar's. Only shape is decided here: that every line is boolean, and that `result` is in scope only inside `after:`, is the checker's, and the semantic rules CTR-01..CTR-03 and the strippability regime (`--strip-checks`) live in the companion chapter. It is written for whoever implements or reads the parser of these constructs; it rests on 10 — Contracts.

---

### CTRG-01 — 1. Contract blocks

**Id:** CTRG-01

**Kind:** rule

**Name:** 1. Contract blocks

**Question:** What production shape do the `before:`, `after:` and `always:` contract blocks take, and is their placement, their boolean restriction and the scope of `result` fixed by these productions themselves or left to the hosting productions and the checker?

**Parent:** LANG-13

**Norm:**

```lark
// CTR-01: before: — precondition block.  Every line inside is one
// boolean expression.  CTR-01 also requires it be at the top of a
// function body before any other statement, and only inside a function
// or class method — placement rules enforced by the function_body rule
// that hosts it (through contract_prelude), not by before_block itself.

before_block: _BEFORE ":" _NEWLINE _INDENT (contract_expression _NEWLINE)+ _DEDENT

// CTR-02: after: — postcondition block.  Same shape as before:.
// Placement rule (must appear after any before_block, before other
// statements) enforced by the hosting function_body rule.

after_block: _AFTER ":" _NEWLINE _INDENT (contract_expression _NEWLINE)+ _DEDENT

// CTR-03: always: — invariant block.  Only inside a class body.  Sits
// after field declarations per CTR-03 (grammar enforcement lives in the
// class_body rule in 14-classes-and-objects.lark.md).  At most one per
// class.

always_block: _ALWAYS ":" _NEWLINE _INDENT (contract_expression _NEWLINE)+ _DEDENT

// Each line is one boolean expression.  The grammar accepts any
// expression; the type checker restricts to boolean (CLASS006 for
// always:, SEM001 for before:/after:).

contract_expression: expression

// Contract expressions may reference the special identifier `result`
// per CTR-02.  `result` is a hard keyword (LEX-04), so it never
// arrives as an IDENTIFIER: the expression grammar admits it as an
// operand here.  That it is in scope only inside after_block is a
// scope rule, not grammar.

%extend primary_expression: _RESULT -> result_reference
```

**Why:** before:, after: and always: share one surface shape — an indented body of one expression per line — and differ only in placement and evaluation, so the three productions are identical and placement stays with the hosting productions; the boolean restriction and the `result` scope are the checker's.

---

### CTRG-02 — 2. Aggregate: the contract-prelude in a function body

**Id:** CTRG-02

**Kind:** rule

**Name:** 2. Aggregate: the contract-prelude in a function body

**Question:** Is the optional before-then-after pair that a function body and a class method body open with defined once as a shared `ContractPrelude` production that fixes their order, or written out by each hosting production?

**Parent:** LANG-13

**Norm:**

```lark
// Used by function_body in 09-functions.lark.md, which also hosts
// every class method (a method is a function_declaration inside the
// class's functions: block).  Order is fixed per CTR ordering rule:
// before_block (if any) precedes after_block (if any).  The prelude
// holds at least one of the two; function_body makes it optional.

contract_prelude: before_block after_block?
                | after_block
```

**Why:** 09-functions.lark.md and ClassMethodBody both need the same optional before-then-after prelude, so it is defined once here with the CTR ordering rule fixed in the production rather than left to each host to get right.

---
