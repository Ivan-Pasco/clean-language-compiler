# 12 control-flow — Grammar

Companion grammar file for [12 — Control Flow](../12-control-flow.md). Defines the shape of `if` / `else if` / `else` conditionals, the `iterate` loop in both element-iteration and range forms, the `while` loop, and the `break` / `continue` statements. Semantic rules FLW-01..FLW-03 live in the companion chapter. There is no `for` (reserved), no `switch`, and no labelled form of `break` or `continue`.

The productions here are the concrete forms referenced by `ControlFlowStatement`, `BreakStatement`, and `ContinueStatement` in [07-statements.ebnf.md](./07-statements.ebnf.md).

---

## 1. Conditional

```ebnf
(* FLW-01: if / else if / else.  Each branch has an indented body.
   The else-if chain is right-nested — an `else if` is really an
   `else` whose body is a single `if`.  Grammar encodes this by
   allowing the `else` branch to optionally start with `if`. *)

IfStatement     = "if", Expression, NEWLINE, INDENT, StatementSequence, DEDENT,
                  { ElseIfClause },
                  [ ElseClause ] ;

ElseIfClause    = "else", "if", Expression, NEWLINE, INDENT,
                  StatementSequence, DEDENT ;

ElseClause      = "else", NEWLINE, INDENT, StatementSequence, DEDENT ;
```

## 2. `iterate` loop

```ebnf
(* FLW-02: iterate has two related forms — element iteration and
   range iteration.  Both use the same syntactic frame:
     iterate <binder> in <source> [ step <expr> ]
   The source is what distinguishes them: a Range expression
   (a to b) is the range form; anything else is the element form. *)

IterateStatement = "iterate", Identifier, "in", IterateSource,
                   [ StepClause ], NEWLINE,
                   INDENT, StatementSequence, DEDENT ;

IterateSource   = RangeExpression | Expression ;
                  (* A RangeExpression is `a to b` (see below).  Any
                     other expression is an iterable value — a list,
                     a string, a matrix, or the rows of a matrix. *)

RangeExpression = Expression, "to", Expression ;
                  (* `to` is a hard keyword per LEX-04.  RangeExpression
                     is iterate-only — it does NOT appear as a general
                     Expression form.  Every example in the chapter uses
                     it in iterate source position; treating it as a
                     general expression would require thinking through
                     precedence, associativity, and interactions with
                     list/matrix types that the chapter does not address.
                     If a future spec wants `list<integer> r = 1 to 10`,
                     that is a new form to add, not an implicit one. *)

StepClause      = "step", Expression ;
                  (* Step may be negative (e.g. `step -2` for a
                     descending range). *)
```

## 3. `while` loop

```ebnf
(* FLW-02 §While: condition-first, indented body.  Condition MUST
   evaluate to boolean (type-checker rule, not grammar). *)

WhileStatement  = "while", Expression, NEWLINE,
                  INDENT, StatementSequence, DEDENT ;
```

## 4. `break` and `continue`

```ebnf
(* FLW-03: each stands alone on its own line.  Takes no operand,
   carries no condition and no label.  Producing no value means
   the parser must NOT admit `break` or `continue` as an operand
   inside an Expression. *)

BreakStatement    = "break" ;

ContinueStatement = "continue" ;

(* FLW-03 boundary rules — a break/continue without an enclosing
   loop in the same body is SEM025 — are semantic, not syntactic.
   Grammar admits either statement anywhere Statement is admitted;
   the checker verifies enclosure. *)
```

## 5. Aggregate: `ControlFlowStatement`

```ebnf
(* The alternation referenced from 07-statements.ebnf.md. *)

ControlFlowStatement = IfStatement
                     | IterateStatement
                     | WhileStatement ;
```

---

## Changelog

- 2026-08-07 (afternoon) — Resolved the §2 `⚠` marker: `RangeExpression` stays iterate-only, NOT admissible as a general Expression form. Every chapter example uses it in iterate source position; treating it as a general expression would require thinking through precedence, associativity, and interactions with list/matrix types that the chapter does not address. A future spec that wants general ranges would add a new form explicitly. No production change.
- 2026-08-07 — File minted. Productions derived from FLW-01..FLW-03 in [12-control-flow.md](../12-control-flow.md) Accepted 2026-08-01.

---

## Metadata

- **Status:** Accepted (2026-08-07)
- **Audience:** Parser implementers; anyone writing conditionals and loops
- **Notation:** EBNF (ISO/IEC 14977)
- **Part of:** [04 language / grammar / README.md](./README.md)
- **Rules referencing this grammar:** [12-control-flow.md](../12-control-flow.md) (FLW-01..FLW-03)
- **References:** [03-lexical-structure.ebnf.md](./03-lexical-structure.ebnf.md), [06-expressions.ebnf.md](./06-expressions.ebnf.md), [07-statements.ebnf.md](./07-statements.ebnf.md) (StatementSequence)
