# 06 expressions — Grammar

**Title:** 06 expressions — Grammar

**Id:** `EXPG-`

**Purpose:** This is the companion grammar of 06 — Expressions: the productions that give every operator of Clean its syntactic surface and settle how an ambiguous reading resolves. It writes precedence and associativity as a ladder — one production per level, each calling the next-tighter as its operand, exponentiation recursing on the right, `not` admitted in both unary and binary position — from `expression`, the entry point at the loosest level, down to the postfix forms (member access, call, index, the `!` assertion) and the atoms: literals, identifiers, and the parenthesized form that carries an expression across lines. It also gives the interpolated string its production, admitting an unrestricted expression inside the braces. What an operator means per type, and every restriction the grammar admits but the checker refuses, belongs to the Expressions chapter; the full `onError` grammar belongs to the error-handling grammar, of which only the binary shape appears here to complete the ladder; and the body of a string belongs to the lexical grammar. It is written for the implementer of a Clean parser, and rests on 06 — Expressions (EXP-), the lexical grammar (LEX-) and the error-handling grammar.

---

### EXPG-01 — 1. The precedence ladder

**Id:** EXPG-01

**Kind:** rule

**Name:** 1. The precedence ladder

**Question:** By what grammar shape are Clean's operator precedence levels and their associativity fixed — one production per level taking the next-tighter level as its operand, exponentiation recursing on its own right, `not` written at both the unary and the equality level — and does assignment appear as one of those levels?

**Norm:**

Level numbering follows EXP-01 — level 1 is postfix (tightest), level 13 is `onError` (loosest). Assignment (level 12 in the EXP-01 table) is STM-02 a statement, not an expression, and does not appear in this ladder.

```lark
// Entry point — an expression's loosest form.

expression: on_error_expression

// Level 13: onError — failure fallback, left-associative.  The full
// onError grammar belongs to 13-error-handling.lark.md; this concrete
// shape is here so the ladder is complete.  It is written
// left-recursive, which is left-associative too, so that the block form
// of 13 — an on_error_expression followed by `onError :` — is decided
// by the token after `onError` alone.

on_error_expression: default_expression
                   | on_error_expression _ONERROR default_expression

// Level 11: default — none-coalescing, left-associative.
// EXP-03: value default fallback.

default_expression: or_expression (_DEFAULT or_expression)*

// Level 10: or — logical, left-associative.

or_expression: and_expression (_OR and_expression)*

// Level 9: and — logical, left-associative.

and_expression: equality_expression (_AND equality_expression)*

// Level 8: equality and identity — left-associative.  `not` in binary
// position (a not b) sits here per EXP prose: "position, not
// lookahead" distinguishes unary and binary not.  The grammar writes
// `not` at both levels of the ladder (unary at level 3, binary at
// level 8); an LALR(1) parser tells them apart by the state it is in —
// operand expected or operator expected — with no lookahead beyond
// the `not` itself.

equality_expression: comparison_expression (equality_op comparison_expression)*

equality_op: "=="  -> op_eq
           | "!="  -> op_ne
           | _IS   -> op_is
           | _NOT  -> op_is_not

// Level 7: comparison — left-associative.

comparison_expression: additive_expression (comparison_op additive_expression)*

comparison_op: "<"   -> op_lt
             | ">"   -> op_gt
             | "<="  -> op_le
             | ">="  -> op_ge

// Level 6: additive — left-associative.

additive_expression: multiplicative_expression (additive_op multiplicative_expression)*

additive_op: "+"  -> op_add
           | "-"  -> op_sub

// Level 5: multiplicative — left-associative.

multiplicative_expression: exponentiation_expression (multiplicative_op exponentiation_expression)*

multiplicative_op: "*"  -> op_mul
                 | "/"  -> op_div
                 | "%"  -> op_mod

// Level 4: exponentiation — RIGHT-associative per EXP-01.  Encoded by
// recursion on the right: a right-associative operator nests its own
// rule, not the next-tighter one, on its right-hand side.

exponentiation_expression: unary_expression ("^" exponentiation_expression)?

// Level 3: unary — prefix operators.  `not` here is the UNARY form.
// `-` here is unary minus.  Per EXP-01, unary binds tighter than
// arithmetic — so unary applies to a postfix operand.

unary_expression: unary_op* postfix_expression

unary_op: _NOT  -> op_not
        | "-"   -> op_neg

// Levels 1-2 combined: postfix and primary.  Postfix `!` (EXP-03
// required-assertion) is written immediately after the operand it
// applies to.  Several postfix `!`s could in principle be written, but
// each requires an optional on the left; the grammar admits
// zero-or-more, the semantic checker restricts.  The rule is
// left-recursive — each postfix operation applies to everything before
// it — which also lets the parser decide with one token of lookahead
// whether `a.b` is an access or the target of `a.b = v`.

postfix_expression: primary_expression
                  | postfix_expression "!"                     -> op_assert
                  | postfix_expression "." IDENTIFIER          -> member_access
                  | postfix_expression "(" argument_list? ")"  -> call
                  | postfix_expression "[" expression "]"      -> index_access

argument_list: expression ("," expression)*
```

**Why:** the ladder itself settles precedence and associativity — each level one production calling the next-tighter as operand, exponentiation recursing on the right; `not` sits at levels 3 and 8 because position, not lookahead, tells unary from binary, and assignment is absent because it is a statement.

---

### EXPG-02 — 2. Primary expressions (level 2)

**Id:** EXPG-02

**Kind:** rule

**Name:** 2. Primary expressions (level 2)

**Question:** Which forms may stand as a primary expression at the tightest level — literals, identifiers, the namespace call on a type keyword, the parenthesized form — and what carries such an expression across line breaks, the enclosing parentheses or indentation?

**Norm:**

```lark
primary_expression: literal
                  | IDENTIFIER          -> identifier
                  | namespace_call
                  | "(" expression ")"  -> parenthesized

// The literal forms are those of 03-lexical-structure.lark.md, with
// the interpolated string of EXPG-03 beside the plain one and the bytes
// literal LEXG-07 defines.

literal: integer_literal
       | NUMBER_LITERAL  -> number_literal
       | string_literal
       | interpolated_string
       | bytes_literal
       | boolean_literal
       | none_literal
       | list_literal

// LEX-04, CALL-03: a type keyword is never an IDENTIFIER, but it may
// stand before "." as the namespace of the functions of its type, in a
// call — list.concat(a, b), list.range(1, 10).  The rule carries its
// own argument list, so a type keyword standing alone is not an
// expression.  type_keyword is defined in 03-lexical-structure.lark.md.
// The grammar admits any type keyword and any function name; whether
// the type has that function is the checker's.

namespace_call: type_keyword "." IDENTIFIER "(" argument_list? ")"

// EXP-02: a multi-line expression is wrapped in parentheses; the
// enclosing pair carries the expression across line breaks, so
// indentation is never what joins the lines.  The grammar needs no
// second rule for it: between an opening "(" and its closing ")" the
// lexer emits no _NEWLINE, _INDENT or _DEDENT (LEXG-02), so a
// parenthesized expression, an argument list or an index that spans
// lines reads as one line of tokens.
```

**Why:** the tightest level names the atoms — literals, identifiers, the namespace call, parenthesized forms; a type keyword is not an Identifier (LEX-04), so the call on its namespace has to be its own production rather than a Call the parser would resolve by name, and it carries its argument list so the keyword alone never becomes a value; and a multi-line expression is carried across lines by its enclosing parentheses, never by indentation, and by the lexer suppressing line events inside them (LEXG-02), so one production serves the single-line and the multi-line form alike.

---

### EXPG-03 — 3. String interpolation

**Id:** EXPG-03

**Kind:** rule

**Name:** 3. String interpolation

**Question:** What production gives an interpolated string literal its shape, and how restricted is the expression the grammar admits between its interpolation braces?

**Norm:**

```lark
// LEX-06: single-line string literals may contain {expr}.  \{ and \}
// escape literal braces.  Inside {...} the content is an expression —
// grammar-wise unrestricted; the "no method calls in interpolation"
// restriction from EXP prose is enforced by a semantic checker, not by
// the grammar.  Same "grammar admits, checker restricts" pattern as
// list behaviors (04-type-system).  Keeps open the option of relaxing
// the restriction later without a grammar change.
//
// Grammatically interpolated_string is a specialisation of the
// single-line string of 03-lexical-structure.lark.md with interpolation
// as an additional alternative inside the body.  The lexer delivers
// the text around the interpolations as pieces: the text up to the
// first "{", the text between a "}" and the next "{", and the text from
// the last "}" to the closing quote.

interpolated_string: STRING_HEAD interpolation (STRING_MIDDLE interpolation)* STRING_TAIL

interpolation: expression

STRING_HEAD: "\"" (STRING_CHARACTER | ESCAPE_SEQUENCE)* "{"
STRING_MIDDLE: "}" (STRING_CHARACTER | ESCAPE_SEQUENCE)* "{"
STRING_TAIL: "}" (STRING_CHARACTER | ESCAPE_SEQUENCE)* "\""
```

**Why:** an interpolated string is the SingleLineString of 03-lexical-structure.lark.md with Interpolation as one more alternative, and the braces hold an unrestricted Expression; the no-method-calls restriction is left to the checker so it can be relaxed later without a grammar change.

---

### EXPG-04 — 4. `onError` shape (referenced by 13-error-handling.lark.md)

**Id:** EXPG-04

**Kind:** rule

**Name:** 4. `onError` shape (referenced by 13-error-handling.lark.md)

**Question:** How much of the `onError` grammar does this expressions grammar publish — only the binary-infix shape that completes the precedence ladder — and where do its remaining `ErrorBinding` forms live?

**Norm:**

```lark
// Level 13 in EXP-01.  The full grammar (including the block form)
// lives in 13-error-handling.lark.md; this file publishes only the
// binary-infix shape, on_error_expression, for the precedence ladder
// above.
```

**Why:** the ladder above needs level 13 to be complete, but the full `onError` grammar with its {ErrorBinding} forms belongs to 13-error-handling.lark.md; this section marks that only the binary-infix shape is published here, so nobody looks for the rest in this file.

---
