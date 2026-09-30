# 12 control-flow — Grammar

**Title:** 12 control-flow — Grammar

**Id:** `FLWG-`

**Purpose:** Companion grammar file for 12 — Control Flow. Defines the shape of `if` / `else if` / `else` conditionals, the `iterate` loop in both element-iteration and range forms — the `a to b` range and its `step` clause exist only as an `iterate` source, never as a general expression — the `while` loop, and the `break` / `continue` statements, each a bare keyword with no operand, condition or label. Semantic rules FLW-01..FLW-03 live in the companion chapter; the boolean condition and the enclosing-loop check are the type checker's, not the grammar's. The productions here are the concrete forms referenced by `control_flow_statement`, `break_statement`, and `continue_statement` in 07-statements.lark.md; it is written for parser implementors and for readers of 12 — Control Flow who need the exact form.

---

### FLWG-01 — 1. Conditional

**Id:** FLWG-01

**Kind:** rule

**Name:** 1. Conditional

**Question:** What written form must an `if` / `else if` / `else` conditional take — is each branch's body an indented block, and is an `else if` a third kind of branch or an `else` whose body starts with `if`?

**Parent:** LANG-05

**Norm:**

```lark
// FLW-01: if / else if / else.  Each branch has an indented body
// (block, 07-statements.lark.md).  The else-if chain is right-nested —
// an `else if` is really an `else` whose body is a single `if`.  The
// grammar encodes this by allowing the `else` branch to optionally
// start with `if`.

if_statement: _IF expression block else_if_clause* else_clause?

else_if_clause: _ELSE _IF expression block

else_clause: _ELSE block
```

**Why:** an `else if` is only an `else` whose body is a single `if`, so the grammar encodes the chain as right-nested clauses instead of inventing a third kind of branch; one frame — keyword, expression, indented body — serves every branch.

---

### FLWG-02 — 2. `iterate` loop

**Id:** FLWG-02

**Kind:** rule

**Name:** 2. `iterate` loop

**Question:** What written form must an `iterate` loop take — do its element and range forms share one production distinguished only by the source, where does a `step` clause attach, and may `a to b` stand anywhere but in an iterate source?

**Parent:** LANG-05

**Norm:**

```lark
// FLW-02: iterate has two related forms — element iteration and range
// iteration.  Both use the same syntactic frame:
//     iterate <binder> in <source> [ step <expr> ]
// The source is what distinguishes them: a range_expression (a to b)
// is the range form; anything else is the element form.

iterate_statement: _ITERATE IDENTIFIER _IN iterate_source block

// A range_expression is `a to b` (see below).  Any other expression is
// an iterable value — a list, a string, a matrix, or the rows of a
// matrix.  2026-08-22: step_clause moved inside the range alternative —
// `step` belongs to ranges only (12 §FLW-02).  Decidable in the parser
// because range_expression is syntactic and iterate-only.

iterate_source: range_expression step_clause?  -> range_source
              | expression                     -> element_source

// `to` is a hard keyword per LEX-04.  range_expression is iterate-only
// — it does NOT appear as a general expression form.  Every example in
// the chapter uses it in iterate source position; treating it as a
// general expression would require thinking through precedence,
// associativity, and interactions with list/matrix types that the
// chapter does not address.  If a future spec wants
// `list<integer> r = 1 to 10`, that is a new form to add, not an
// implicit one.

range_expression: expression _TO expression

// Step may be negative (e.g. `step -2` for a descending range).
// `step` is a contextual keyword (LEX-04): the lexer offers it only in
// the parser state after a range, the one position this rule gives it.

step_clause: _STEP expression
```

**Why:** the two forms of `iterate` differ only in their source, so one production with a source alternation holds both; `step` sits inside the range alternative because it belongs to ranges only, and `a to b` stays iterate-only so its precedence never has to be settled.

---

### FLWG-03 — 3. `while` loop

**Id:** FLWG-03

**Kind:** rule

**Name:** 3. `while` loop

**Question:** What written form must a `while` loop take, and does its production itself constrain the condition to a boolean or state form alone?

**Parent:** LANG-05

**Norm:**

```lark
// FLW-02 §While: condition-first, indented body.  Condition MUST
// evaluate to boolean (type-checker rule, not grammar).

while_statement: _WHILE expression block
```

**Why:** the loop is the smallest shape the language admits — a condition and an indented body — and the boolean requirement on that condition is the type checker's, so the production states form alone and does not pretend to check types.

---

### FLWG-04 — 4. `break` and `continue`

**Id:** FLWG-04

**Kind:** rule

**Name:** 4. `break` and `continue`

**Question:** What written form must `break` and `continue` take — may either carry an operand, condition or label, may either stand as an operand inside an expression, and does the grammar itself require an enclosing loop?

**Norm:**

```lark
// FLW-03: each stands alone on its own line.  Takes no operand,
// carries no condition and no label.  Producing no value means the
// parser must NOT admit `break` or `continue` as an operand inside an
// expression: the two are statements only.

break_statement: _BREAK

continue_statement: _CONTINUE

// FLW-03 boundary rules — a break/continue without an enclosing loop
// in the same body is SEM025 — are semantic, not syntactic.  The
// grammar admits either statement anywhere a statement is admitted;
// the checker verifies enclosure.
```

**Why:** each statement is one keyword and nothing more — no operand, no condition, no label — and because it produces no value it must not be admitted as an operand; the enclosing-loop rule (SEM025) is the checker's, so the grammar admits either wherever a statement stands.

---

### FLWG-05 — 5. Aggregate: `ControlFlowStatement`

**Id:** FLWG-05

**Kind:** rule

**Name:** 5. Aggregate: `ControlFlowStatement`

**Question:** Which productions does the `ControlFlowStatement` alternation referenced by `07-statements.lark.md` gather, and is that name defined here or restated in the statement grammar?

**Norm:**

```lark
// The alternation referenced from 07-statements.lark.md.

control_flow_statement: if_statement
                      | iterate_statement
                      | while_statement
```

**Why:** 07-statements.lark.md needs one name to refer to, so the three forms are gathered under `ControlFlowStatement` here, where they are defined, rather than listed again in the statement grammar.

---
