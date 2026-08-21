# 10 contracts — Grammar

Companion grammar file for [10 — Contracts](../10-contracts.md). Defines the shape of the three contract blocks — `before:`, `after:`, `always:` — that carry preconditions, postconditions, and invariants. Semantic rules CTR-01..CTR-03 live in the companion chapter, along with the strippability regime (`--strip-checks`), the ordering rules inside a function body, and the placement of `always:` inside a class body.

Contract blocks are indented bodies of boolean expressions, one per line. The three keywords differ in where they may appear (function body vs. class body) and in evaluation semantics, not in surface shape.

---

## 1. Contract blocks

```ebnf
(* CTR-01: before: — precondition block.  Every line inside is one
   boolean Expression.  CTR-01 also requires it be at the top of a
   function body before any other statement, and only inside a
   function or class method — placement rules enforced by the
   FunctionBody / ClassMethodBody productions that host it, not by
   the BeforeBlock production itself. *)

BeforeBlock     = "before", ":", NEWLINE, INDENT,
                  ContractExpression, NEWLINE,
                  { ContractExpression, NEWLINE },
                  DEDENT ;

(* CTR-02: after: — postcondition block.  Same shape as before:.
   Placement rule (must appear after any BeforeBlock, before other
   statements) enforced by the hosting FunctionBody production. *)

AfterBlock      = "after", ":", NEWLINE, INDENT,
                  ContractExpression, NEWLINE,
                  { ContractExpression, NEWLINE },
                  DEDENT ;

(* CTR-03: always: — invariant block.  Only inside a class body.
   Sits after field declarations per CTR-03 (grammar enforcement
   lives in the ClassBody production in 14-classes-and-objects.
   ebnf.md).  At most one per class. *)

AlwaysBlock     = "always", ":", NEWLINE, INDENT,
                  ContractExpression, NEWLINE,
                  { ContractExpression, NEWLINE },
                  DEDENT ;

(* Each line is one boolean Expression.  Grammar accepts any
   Expression; the type checker restricts to boolean (CLASS006 for
   always:, SEM001 for before:/after:).  Contract expressions may
   reference the special identifier `result` (only inside AfterBlock)
   per CTR-02 — that is a scope rule, not grammar. *)

ContractExpression = Expression ;
```

## 2. Aggregate: the contract-prelude in a function body

```ebnf
(* Used by 09-functions.ebnf.md and by ClassMethodBody in
   14-classes-and-objects.ebnf.md.  Order is fixed per CTR ordering
   rule: BeforeBlock (if any) precedes AfterBlock (if any). *)

ContractPrelude = [ BeforeBlock ], [ AfterBlock ] ;
```

---

## Changelog

- 2026-08-07 — File minted. Productions derived from CTR-01..CTR-03 in [10-contracts.md](../10-contracts.md) Accepted 2026-08-01. No `⚠` markers — the contract-block shape is uniform and unambiguous; placement rules are enforced by hosting productions.

---

## Metadata

- **Status:** Accepted (2026-08-07)
- **Audience:** Parser implementers; downstream grammar files (functions, classes) that host contract blocks
- **Notation:** EBNF (ISO/IEC 14977)
- **Part of:** [04 language / grammar / README.md](./README.md)
- **Rules referencing this grammar:** [10-contracts.md](../10-contracts.md) (CTR-01..CTR-03)
- **References:** [03-lexical-structure.ebnf.md](./03-lexical-structure.ebnf.md), [06-expressions.ebnf.md](./06-expressions.ebnf.md)
