# 07 statements — Grammar

Companion grammar file for [07 — Statements](../07-statements.md). Defines the shape of statement forms — variable declaration (type-first), assignment, `return`, the `print:` block, and the general statement sequence used by every block body. Semantic rules STM-01..STM-03 live in the companion chapter.

Assignment is a statement, never an expression ([STM-02](../07-statements.md#stm-02--assignment-is-a-statement-never-an-expression)); this file encodes that by keeping `Assignment` in the `Statement` production and out of `06-expressions.ebnf.md`.

---

## 1. Statement sequences

```ebnf
(* A statement sequence is the body of any block — start:, functions:,
   if/else branches, iterate bodies, contract clauses, etc.  Each
   Statement stands on its own line, followed by NEWLINE.  Nested
   blocks use INDENT/DEDENT per LEX-01. *)

StatementSequence = { Statement, NEWLINE } ;

Statement       = VariableDeclaration
                | Assignment
                | ReturnStatement
                | ExpressionStatement
                | PrintBlock
                | ControlFlowStatement
                | BreakStatement
                | ContinueStatement ;

(* ControlFlowStatement, BreakStatement, and ContinueStatement
   grammars live in 12-control-flow.ebnf.md; referenced here so
   the Statement alternation is complete for any parser walking
   a statement sequence. *)
```

## 2. Variable declaration

```ebnf
(* STM-01: type-first declaration.  The declaration may have an
   initialiser (Expression) or stand uninitialised.
   TypedDeclaration is defined in 04-type-system.ebnf.md and used
   here directly. *)

VariableDeclaration = TypedDeclaration ;
```

## 3. Assignment

```ebnf
(* STM-02: assignment is a statement.  The target is one of exactly
   three forms — grammar restricts these directly so an invalid
   target (a call, a postfix `!`, an arbitrary expression) fails at
   parse time with a clean message rather than needing a semantic
   pass to reject.  *)

Assignment      = AssignmentTarget, "=", Expression ;

AssignmentTarget = SimpleAssignmentTarget
                 | IndexedAssignmentTarget
                 | MemberAssignmentTarget ;

SimpleAssignmentTarget  = Identifier ;

IndexedAssignmentTarget = PostfixExpression, "[", Expression, "]" ;
                          (* e.g. arr[0] = value, obj.field[k] = v *)

MemberAssignmentTarget  = PostfixExpression, ".", Identifier ;
                          (* e.g. obj.property = val, a.b.c = val *)

(* The PostfixExpression on the LHS of an index or member target
   may itself resolve to an identifier, a chain of members, or a
   chain of indices — the grammar admits the recursive structure
   through PostfixExpression, but the FINAL postfix operation
   MUST be Index or Member.  This restricts targets to the three
   observable shapes without excluding chained access. *)
```

## 4. Return

```ebnf
(* STM-03: three forms.
    return              // void return
    return value        // return a variable
    return expression   // return an expression result
   Grammatically the second and third are one production — Expression
   subsumes bare identifiers. *)

ReturnStatement = "return", [ Expression ] ;
```

## 5. Expression statement

```ebnf
(* An Expression whose result is discarded — a function call whose
   return value is not stored.  The parser accepts any Expression
   here; the checker MAY warn if the call returns non-void and the
   value is unused (currently not a specified diagnostic). *)

ExpressionStatement = Expression ;
```

## 6. `print:` block

```ebnf
(* STM prose: print: is a BLOCK, not an apply-block.  Each indented
   line is one expression to print on its own line.  An empty body
   or a body containing a statement (rather than an expression) is
   SYN008. *)

PrintBlock      = "print", ":", NEWLINE, INDENT,
                  PrintItem, NEWLINE,
                  { PrintItem, NEWLINE },
                  DEDENT ;

PrintItem       = Expression ;
                  (* A non-Expression item here is SYN008.  Grammar
                     accepts Expression only; a Statement or empty
                     line fails the production. *)
```

---

## Changelog

- 2026-08-07 (afternoon) — Resolved the §3 `⚠` marker: grammar now restricts `AssignmentTarget` to exactly three shapes — `SimpleAssignmentTarget` (Identifier), `IndexedAssignmentTarget` (PostfixExpression `[` Expression `]`), and `MemberAssignmentTarget` (PostfixExpression `.` Identifier). Invalid targets like `foo() = 42` or `x! = 42` now fail at parse time with a clean message instead of needing a semantic pass to reject. Production change: replaced `AssignmentTarget = Identifier | PostfixExpression` with the three-alternative form.
- 2026-08-07 — File minted. Productions derived from STM-01..STM-03 in [07-statements.md](../07-statements.md) Accepted 2026-08-01.

---

## Metadata

- **Status:** Accepted (2026-08-07)
- **Audience:** Parser implementers; downstream grammar files (functions, control flow) that consume StatementSequence
- **Notation:** EBNF (ISO/IEC 14977)
- **Part of:** [04 language / grammar / README.md](./README.md)
- **Rules referencing this grammar:** [07-statements.md](../07-statements.md) (STM-01..STM-03)
- **References:** [03-lexical-structure.ebnf.md](./03-lexical-structure.ebnf.md), [04-type-system.ebnf.md](./04-type-system.ebnf.md), [06-expressions.ebnf.md](./06-expressions.ebnf.md), forward reference to [12-control-flow.ebnf.md](./12-control-flow.ebnf.md)
