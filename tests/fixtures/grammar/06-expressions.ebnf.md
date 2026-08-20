# 06 expressions — Grammar

Companion grammar file for [06 — Expressions](../06-expressions.md). Defines the syntactic surface of every operator, the precedence-and-associativity structure that resolves ambiguous readings, the multi-line-parenthesized form, string interpolation, and function/method call shapes. Semantic rules EXP-01..EXP-03 live in the companion chapter, along with the operator-meaning-per-type table.

Precedence is encoded here in the standard EBNF way: each level is one production, calling the next-tighter level as its operand. `Expression` is the entry point at the loosest level.

---

## 1. The precedence ladder

Level numbering follows [EXP-01](../06-expressions.md#exp-01--operator-precedence-and-associativity) — level 1 is postfix (tightest), level 13 is `onError` (loosest). Assignment (level 12 in the EXP-01 table) is [STM-02](../07-statements.md#stm-02--assignment-is-a-statement-never-an-expression) a statement, not an expression, and does not appear in this ladder.

```ebnf
(* Entry point — an expression's loosest form. *)

Expression      = OnErrorExpression ;

(* Level 13: onError — failure fallback, left-associative.
   Grammar delegated to 13-error-handling.ebnf.md.
   Concrete shape referenced here so the ladder is complete. *)

OnErrorExpression = DefaultExpression, { "onError", DefaultExpression } ;

(* Level 11: default — none-coalescing, left-associative.
   EXP-03: value default fallback. *)

DefaultExpression = OrExpression, { "default", OrExpression } ;

(* Level 10: or — logical, left-associative. *)

OrExpression    = AndExpression, { "or", AndExpression } ;

(* Level 9: and — logical, left-associative. *)

AndExpression   = EqualityExpression, { "and", EqualityExpression } ;

(* Level 8: equality and identity — left-associative.
   `not` in binary position (a not b) sits here per EXP prose:
   "position, not lookahead" distinguishes unary and binary not.
   Grammar encodes "not" in both levels of the ladder (unary at
   level 3, binary at level 8); the parser dispatches on operand-
   vs-operator position, which is a standard parser technique
   (Pratt parser or similar). *)

EqualityExpression = ComparisonExpression,
                     { EqualityOp, ComparisonExpression } ;

EqualityOp      = "==" | "!=" | "is" | "not" ;

(* Level 7: comparison — left-associative. *)

ComparisonExpression = AdditiveExpression,
                       { ComparisonOp, AdditiveExpression } ;

ComparisonOp    = "<" | ">" | "<=" | ">=" ;

(* Level 6: additive — left-associative. *)

AdditiveExpression = MultiplicativeExpression,
                     { AdditiveOp, MultiplicativeExpression } ;

AdditiveOp      = "+" | "-" ;

(* Level 5: multiplicative — left-associative. *)

MultiplicativeExpression = ExponentiationExpression,
                           { MultiplicativeOp, ExponentiationExpression } ;

MultiplicativeOp = "*" | "/" | "%" ;

(* Level 4: exponentiation — RIGHT-associative per EXP-01.
   Encoded by recursion on the right: a right-associative operator
   nests its own production, not the next-tighter one, on its
   right-hand side. *)

ExponentiationExpression = UnaryExpression,
                           [ "^", ExponentiationExpression ] ;

(* Level 3: unary — prefix operators. `not` here is the UNARY form.
   `-` here is unary minus. Per EXP-01, unary binds tighter than
   arithmetic — so unary applies to a Postfix operand. *)

UnaryExpression = { UnaryOp }, PostfixExpression ;

UnaryOp         = "not" | "-" ;

(* Level 1-2 combined: postfix and primary.
   Postfix `!` (EXP-03 required-assertion) is written immediately
   after the primary it applies to. Multiple postfix `!`s could
   in principle be written but each requires an optional on the
   left; grammar admits zero-or-more, semantic checker restricts. *)

PostfixExpression = PrimaryExpression, { PostfixOp } ;

PostfixOp       = "!"
                | MemberAccess
                | Call
                | IndexAccess ;

MemberAccess    = ".", Identifier ;

Call            = "(", [ ArgumentList ], ")" ;

IndexAccess     = "[", Expression, "]" ;

ArgumentList    = Expression, { ",", Expression } ;
```

## 2. Primary expressions (level 2)

```ebnf
PrimaryExpression = Literal
                  | Identifier
                  | ParenthesizedExpression
                  | MultiLineParenthesized ;

Literal         = IntegerLiteral
                | NumberLiteral
                | StringLiteral
                | BooleanLiteral
                | NoneLiteral
                | ListLiteral
                | MatrixLiteral ;

ParenthesizedExpression = "(", Expression, ")" ;

(* EXP-02: a multi-line expression is wrapped in parentheses;
   the enclosing pair carries the expression across line breaks,
   so indentation is never what joins the lines.  Grammar-wise,
   MultiLineParenthesized is the same shape as ParenthesizedExpression
   except NEWLINE and inline whitespace are permitted between tokens
   inside.  A parser reading whitespace-sensitively distinguishes
   the two; a whitespace-insensitive parser inside "(...)" needs no
   separate production. *)

MultiLineParenthesized = "(", { NEWLINE | InlineSpace }, Expression,
                         { NEWLINE | InlineSpace }, ")" ;
```

## 3. String interpolation

```ebnf
(* LEX-06: single-line string literals may contain {expr}.
   \{ and \} escape literal braces.  Inside {...} the content is
   an Expression — grammar-wise unrestricted; the "no method calls
   in interpolation" restriction from EXP prose is enforced by a
   semantic checker, not by the grammar.  Same "grammar admits,
   checker restricts" pattern as list behaviors (04-type-system).
   Keeps open the option of relaxing the restriction later without
   a grammar change. *)

InterpolatedString = '"', { StringCharacter | EscapeSequence | Interpolation }, '"' ;

Interpolation      = "{", Expression, "}" ;

(* Grammatically InterpolatedString is a specialisation of the
   SingleLineString production in 03-lexical-structure.ebnf.md;
   this file's version admits Interpolation as an additional
   alternative inside the body. *)
```

## 4. `onError` shape (referenced by 13-error-handling.ebnf.md)

```ebnf
(* Level 13 in EXP-01. Full grammar (including the {ErrorBinding}
   forms) lives in 13-error-handling.ebnf.md; this file publishes
   only the binary-infix shape for the precedence ladder above. *)
```

---

## Changelog

- 2026-08-07 (afternoon) — Resolved both `⚠` markers: (a) unary-vs-binary `not` stays as parser-position dispatch — grammar has `not` in both levels of the ladder, parser dispatches (standard Pratt-parser technique); (b) interpolation restriction on method calls stays as a semantic check, not a grammar restriction — keeps open the option of relaxing the rule later without a grammar change. No production change.
- 2026-08-07 — File minted. Productions derived from EXP-01..EXP-03 in [06-expressions.md](../06-expressions.md) Accepted 2026-08-01.

---

## Metadata

- **Status:** Accepted (2026-08-07)
- **Audience:** Parser implementers; anyone reasoning about operator precedence
- **Notation:** EBNF (ISO/IEC 14977)
- **Part of:** [04 language / grammar / README.md](./README.md)
- **Rules referencing this grammar:** [06-expressions.md](../06-expressions.md) (EXP-01..EXP-03)
- **References:** [03-lexical-structure.ebnf.md](./03-lexical-structure.ebnf.md), forward reference to [13-error-handling.ebnf.md](./13-error-handling.ebnf.md) for `onError`
