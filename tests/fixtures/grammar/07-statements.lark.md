# 07 statements — Grammar

**Title:** 07 statements — Grammar

**Id:** `STMG-`

**Purpose:** Companion grammar file for 07 — Statements. Defines the shape of every statement form — variable declaration (type-first), assignment with its three admissible targets, `return`, the expression statement whose value is discarded, the `print(...)` call with its one named argument, and the `print:` block — and the general statement sequence that is the body of every block, whose alternation is completed by reference with the control-flow, `break` and `continue` forms of `12-control-flow.lark.md`. Semantic rules STM-01..STM-03 live in the companion chapter. Assignment is a statement, never an expression (STM-02); this file encodes that by keeping `assignment` in the `statement` production and out of `06-expressions.lark.md`. It is written for compiler implementors building the parser and for anyone who needs the exact form of a statement; it rests on 07 — Statements, on `04-type-system.lark.md` for the `typed_declaration` it reuses, and on `06-expressions.lark.md` for `expression` and `postfix_expression`.

---

### STMG-01 — 1. Statement sequences

**Id:** STMG-01

**Kind:** rule

**Name:** 1. Statement sequences

**Question:** Of what is the body of every block composed, and which statement forms may stand in one — including the control-flow, `break` and `continue` forms defined in another grammar file?

**Parent:** LANG-05

**Norm:**

```lark
// A statement sequence is the body of any block — start:, functions:,
// if/else branches, iterate bodies, contract clauses, etc.  A simple
// statement stands on its own line and is followed by its _NEWLINE.  A
// compound statement owns an indented body and ends with the _DEDENT
// that closes it: the _NEWLINE that ends its last line is emitted
// before that _DEDENT (LEXG-02), so no _NEWLINE follows a compound
// statement in the sequence.  Nested blocks use _INDENT/_DEDENT per
// LEX-01.
//
// statement_sequence is also where a parser reading a run of
// statements outside any file starts — the snippets of the chapters —
// so it admits the line ends a leading comment leaves.  block is the
// indented sequence every block form ends with; it holds at least one
// statement because the lexer emits no _INDENT for an empty body.

statement_sequence: _NEWLINE* statement*

block: _NEWLINE _INDENT statement+ _DEDENT

statement: simple_statement _NEWLINE
         | compound_statement

simple_statement: variable_declaration
                | assignment
                | return_statement
                | print_statement
                | expression_statement
                | error_statement
                | diagnostic_emission
                | break_statement
                | continue_statement
                | later_binding
                | reset_statement

compound_statement: print_block
                  | control_flow_statement
                  | apply_block
                  | on_error_block
                  | handled_declaration
                  | handled_assignment
                  | background_statement
                  | match_statement
                  | block_valued_declaration
                  | block_valued_return

// control_flow_statement, break_statement and continue_statement live
// in 12-control-flow.lark.md; error_statement, on_error_block,
// handled_declaration and handled_assignment in
// 13-error-handling.lark.md; apply_block in 05-apply-blocks.lark.md;
// later_binding and background_statement in 18-async.lark.md;
// reset_statement in 20-state-management.lark.md; block_valued_declaration, block_valued_return,
// match_statement and
// diagnostic_emission in 21-block-handlers.lark.md.  They are
// referenced here so the alternation is complete for any parser
// walking a statement sequence.  background_statement stands among the
// compound statements because it ends its own line: its tail may be a
// block.
```

**Why:** every block body — start:, functions:, branches, iterate bodies, contract clauses — is the same sequence, so it is defined once; a simple statement is followed by its NEWLINE and a compound one ends with the DEDENT of its body, because the lexer emits the NEWLINE of a line before the DEDENTs that close it, and the alternation is completed with the forms of 12-control-flow.lark.md and 13-error-handling.lark.md so a parser walking a sequence has the whole set.

---

### STMG-02 — 2. Variable declaration

**Id:** STMG-02

**Kind:** rule

**Name:** 2. Variable declaration

**Question:** From which production does a variable declaration statement take its shape, and must it carry an initialiser?

**Parent:** LANG-08

**Norm:**

```lark
// STM-01: type-first declaration.  The declaration may have an
// initialiser (expression) or stand uninitialised.
// explicit_declaration is defined in 04-type-system.lark.md and used
// here directly.  The inferred form `name = expr` is not a
// variable_declaration: it has the shape of an assignment (STMG-03)
// and is read as one — it declares the name when none is in scope
// (TYP-11) — so a statement never has two readings.  TYP-05: a list
// declaration may carry behavior suffixes on its type
// (list_behavior_type, 04-type-system.lark.md); this is the one
// position that admits them.

variable_declaration: explicit_declaration
                    | list_behavior_type IDENTIFIER ("=" expression)?
```

**Why:** a variable declaration is the type-first ExplicitDeclaration of 04-type-system.lark.md, initialised or not; the inferred form is left to Assignment, whose shape it has, so that `count = 0` is read one way and the name it declares is the checker's to notice.

---

### STMG-03 — 3. Assignment

**Id:** STMG-03

**Kind:** rule

**Name:** 3. Assignment

**Question:** Which targets may stand to the left of `=` in an assignment statement, and is an inadmissible target rejected by the grammar at parse time or left to a semantic pass?

**Norm:**

```lark
// STM-02: assignment is a statement.  The target is one of exactly
// three forms — the grammar restricts these directly so an invalid
// target (a call, a postfix `!`, an arbitrary expression) fails at
// parse time with a clean message rather than needing a semantic pass
// to reject.

assignment: assignment_target "=" expression

assignment_target: IDENTIFIER                            -> simple_target
                 | postfix_expression "[" expression "]"  -> index_target
                 | postfix_expression "." IDENTIFIER      -> member_target

// index_target: e.g. arr[0] = value, obj.field[k] = v
// member_target: e.g. obj.property = val, a.b.c = val
//
// The postfix_expression on the left of an index or member target may
// itself resolve to an identifier, a chain of members, or a chain of
// indices — the grammar admits the recursive structure through
// postfix_expression, but the FINAL postfix operation MUST be index or
// member.  This restricts targets to the three observable shapes
// without excluding chained access.
```

**Why:** assignment is a statement whose target is one of exactly three forms, so the grammar restricts them directly: a call, a postfix `!` or an arbitrary expression fails at parse time with a clean message, while chained access still arrives through PostfixExpression ending in index or member.

---

### STMG-04 — 4. Return

**Id:** STMG-04

**Kind:** rule

**Name:** 4. Return

**Question:** How many expressions may follow the `return` keyword, and are the void, variable and expression returns written as one production or three?

**Norm:**

```lark
// STM-03: three forms.
//    return              // void return
//    return value        // return a variable
//    return expression   // return an expression result
// Grammatically the second and third are one rule — expression
// subsumes bare identifiers.

return_statement: _RETURN expression?
```

**Why:** the three return forms of STM-03 collapse to one production because Expression subsumes a bare identifier; the comment keeps the three visible so the reader does not look for a missing alternative.

---

### STMG-05 — 5. Expression statement

**Id:** STMG-05

**Kind:** rule

**Name:** 5. Expression statement

**Question:** May a bare expression stand as a statement with its value discarded, and does the grammar reject an unused non-void result or leave it to a checker warning?

**Norm:**

```lark
// An expression whose result is discarded — a function call whose
// return value is not stored.  The parser accepts any expression here;
// the checker MAY warn if the call returns non-void and the value is
// unused (currently not a specified diagnostic).

expression_statement: expression
```

**Why:** a call whose value is discarded must still be a statement, so any Expression is admitted on its own line; a warning for an unused non-void result is named as a possibility the checker may take, not a diagnostic the grammar promises.

---

### STMG-06 — 6. `print:` block

**Id:** STMG-06

**Kind:** rule

**Name:** 6. `print:` block

**Question:** What must the body of a `print:` block contain, and what becomes of an empty body or of a statement written in place of an expression there?

**Norm:**

```lark
// STM prose: print: is a BLOCK, not an apply-block.  Each indented line
// is one expression to print on its own line.  An empty body or a body
// containing a statement (rather than an expression) is SYN008.

print_block: _PRINT ":" _NEWLINE _INDENT print_item+ _DEDENT

// A non-expression item here is SYN008.  The grammar accepts an
// expression only; a statement or an empty body fails the rule.
print_item: expression _NEWLINE
```

**Why:** print: is a block, not an apply-block: each indented line is one expression printed on its own line, and the production requires at least one PrintItem and admits only expressions, so an empty body or a statement inside fails the production and is SYN008.

---

### STMG-07 — 7. `print` call

**Id:** STMG-07

**Kind:** rule

**Name:** 7. `print` call

**Question:** From which production does a `print(...)` call take its shape, given that `print` is a hard keyword and never an Identifier, and how is its one named argument written?

**Norm:**

```lark
// STD-04: print is a hard keyword (LEX-04), so the call is not a call
// applied to an IDENTIFIER and needs a rule of its own.  There is
// exactly one call form, with one optional named argument, `newline:`
// — the only named argument in the language — spelled out here rather
// than added to argument_list.  `newline` is not a reserved word: it is
// the keyword only in this position, before the `:`, and an ordinary
// identifier anywhere else.

print_statement: _PRINT "(" expression ("," NEWLINE_ARG ":" expression)? ")"

NEWLINE_ARG.1: /newline(?=[ \t]*:)/
```

**Why:** `print(value)` appears in every chapter's examples and had no production: `print` is a hard keyword, so the Identifier-based Call of 06-expressions.lark.md can never produce it, and its `newline:` argument is the one named argument the language has; giving the call its own production, with the argument spelled out, is what lets a parser read the examples and keeps ArgumentList free of a named-argument form nothing else uses.

---
