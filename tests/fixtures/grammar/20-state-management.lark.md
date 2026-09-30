# 20 state-management — Grammar

**Title:** 20 state-management — Grammar

**Id:** `SMGG-`

**Purpose:** Companion grammar file for 20 — State Management. Defines the shape of every state-related construct: the top-level `state:` block and the interleaving of its three member kinds, state variable declarations with their mandatory initialiser and stacked guard clauses, the `computed:` and `rules:` sub-blocks, `watch:` observers in single- and multi-variable form, and the `reset` statement for one variable or the whole state; it also exports the `state_body` and `watch_body` aliases other grammar files cite, `state_body` being what 08-file-structure.lark.md reaches through `state_section`. Only shape is decided here: that a guard or a rule is boolean, or that a computed body returns its declared type, is the checker's (STATE001, STATE005, SEM018), and the semantic rules SMG-01..SMG-05 live in the companion chapter. It is written for whoever implements or reads the parser of these constructs; it rests on 20 — State Management and on the lexical terminals of 03 — Lexical Structure (LEX-04).

---

### SMGG-01 — 1. `state:` block

**Id:** SMGG-01

**Kind:** rule

**Name:** 1. `state:` block

**Question:** In what order may state variable declarations, the `computed:` sub-block and the `rules:` sub-block stand inside the body of a `state:` block — a fixed sequence, or any interleaving of the three member kinds?

**Parent:** LANG-14

**Norm:**

```lark
// SMG-01: state: block.  Contains state variable declarations, an
// optional rules: sub-block, and an optional computed: sub-block.  The
// chapter's examples show declarations first and `computed:` and
// `rules:` after them, but no rule of the chapter fixes an order among
// the three, so the body admits any interleaving of its members.

state_block: _STATE ":" _NEWLINE _INDENT state_body _DEDENT

state_body: state_body_member*

state_body_member: state_variable_declaration
                 | computed_block
                 | rules_block
```

**Why:** the chapter's examples place `rules:` and `computed:` at various positions, so the body is encoded as any interleaving of its three member kinds rather than an order the examples do not support, since no rule of the chapter fixes one.

---

### SMGG-02 — 2. State variable declaration + guard clauses

**Id:** SMGG-02

**Kind:** rule

**Name:** 2. State variable declaration + guard clauses

**Question:** Must a state variable declaration carry an initialiser expression, and how many `guard <expr> else <string>` clauses may follow it and in what position?

**Parent:** LANG-14

**Norm:**

```lark
// SMG-01: state variables require initial values (chapter Rules).
// Grammar-wise this is a typed declaration with a required
// initialiser.

state_variable_declaration: type_expression IDENTIFIER "=" expression _NEWLINE guard_clauses?

// SMG-02: guard clauses are indented lines directly beneath a
// declaration.  A declaration may carry more than one — evaluated in
// written order, first-failure-wins semantics.

guard_clauses: _INDENT (guard_clause _NEWLINE)+ _DEDENT

// The expression MUST be a pure boolean expression (STATE001 semantic
// check).  The message MUST be a string literal — chapter §Rules
// requires the `else "<message>"` clause be mandatory.  `guard` is a
// contextual keyword (LEX-04): the parser state after the _INDENT under
// a state variable is the only one that offers it.

guard_clause: _GUARD expression _ELSE string_literal
```

**Why:** a state variable requires an initial value, so the initialiser is mandatory in the production, and guards are indented lines directly beneath their declaration so several may stack in written order; the message is a string literal because the chapter makes the `else` clause mandatory.

---

### SMGG-03 — 3. `computed:` sub-block

**Id:** SMGG-03

**Kind:** rule

**Name:** 3. `computed:` sub-block

**Question:** What written shape does an entry of the `computed:` sub-block take — a typed name followed by an indented multi-line statement body, or a single-line expression?

**Parent:** LANG-14

**Norm:**

```lark
// SMG-05: computed: — read-only derived values.  Each entry is a typed
// name with an indented body that returns the computed value.  The body
// may span multiple lines; each entry ends with the _DEDENT of its body
// (LEXG-02), so no _NEWLINE follows it.

computed_block: _COMPUTED ":" _NEWLINE _INDENT computed_declaration+ _DEDENT

// The body's return type must match the declared type (SEM018 checker
// rule).

computed_declaration: type_expression IDENTIFIER block
```

**Why:** a computed value is a typed name with an indented body rather than a one-line expression, because the body may span several lines; whether that body returns the declared type is the checker's SEM018, not the grammar's.

---

### SMGG-04 — 4. `rules:` sub-block

**Id:** SMGG-04

**Kind:** rule

**Name:** 4. `rules:` sub-block

**Question:** What may stand on each line inside a `rules:` sub-block, and how many such lines must the block hold?

**Parent:** LANG-14

**Norm:**

```lark
// SMG-03: rules: — boolean expressions over the state in scope.  Each
// line is one boolean expression, checked when a function that
// assigned to any variable in the block returns.

rules_block: _RULES ":" _NEWLINE _INDENT (rule_expression _NEWLINE)+ _DEDENT

// Must be boolean — STATE005 checker rule.
rule_expression: expression

// `rules` is a block-header word of this position only, specialised
// like the contextual keywords of LEX-04 when ":" follows it.
_RULES.1: /rules(?=[ \t]*:)/
```

**Why:** a rule is one expression per line and nothing else, so the block is the simplest list the form allows; that each expression is boolean is STATE005, left to the checker.

---

### SMGG-05 — 5. `watch:` block

**Id:** SMGG-05

**Kind:** rule

**Name:** 5. `watch:` block

**Question:** Which target forms may follow the `watch` keyword, and are the single-variable and parenthesised multi-variable shapes written as one production with a target alternation or as two separate productions?

**Parent:** LANG-14

**Norm:**

```lark
// SMG-04: watch — react to state changes.  Two shapes:
//     watch fieldName:       (single variable)
//     watch (a, b, c):       (multiple variables)
// Its body is watch_body (SMGG-07).

watch_block: _WATCH watch_target ":" _NEWLINE _INDENT watch_body _DEDENT

watch_target: IDENTIFIER
            | "(" IDENTIFIER ("," IDENTIFIER)* ")"
```

**Why:** the two shapes of `watch` differ only in their target — one name or a parenthesised list — so a single production with a target alternation covers both and keeps the body frame shared.

---

### SMGG-06 — 6. `reset` statement

**Id:** SMGG-06

**Kind:** rule

**Name:** 6. `reset` statement

**Question:** What may follow the `reset` keyword, and how does the parser tell the whole-state form `reset state` from `state` in its contextual-keyword role?

**Parent:** LANG-14

**Norm:**

```lark
// Chapter §State Reset:
//     reset fieldName    — reset one variable to its initial value
//     reset state        — reset all state in scope
// `reset` is a hard keyword (LEX-04).

reset_statement: _RESET reset_target

// `state` here is the "reset all" word.  It clashes with `state` as a
// contextual keyword (LEX-04) — the block header, which ":" follows —
// only in this position after `reset`; the parser state after `reset`
// is the only one that offers the word, so `reset state` is one form.

reset_target: _RESET_STATE  -> reset_all_state
            | IDENTIFIER    -> reset_variable

_RESET_STATE.1: /state(?![A-Za-z0-9_])/
```

**Why:** `reset` takes either one variable or the whole state, and `state` after `reset` clashes with the contextual keyword only in that one position, so the production names `reset state` as its own form for the parser to recognise.

---

### SMGG-07 — 7. `WatchBody` alias (referenced from other files)

**Id:** SMGG-07

**Kind:** rule

**Name:** 7. `WatchBody` alias (referenced from other files)

**Question:** Under what production name may other grammar files cite the statement sequence that forms a `watch` block's body, rather than inlining `StatementSequence`?

**Parent:** LANG-14

**Norm:**

```lark
// The body of watch_block (SMGG-05): its statement sequence, at least
// one statement, since the lexer emits no _INDENT for an empty body.

watch_body: statement+
```

**Why:** other files reference the watch body by name, so the alias gives them a stable production to cite rather than an inlined `StatementSequence` that a later change could rename.

---

### SMGG-08 — 8. `StateBody` alias (referenced from 08-file-structure.lark.md)

**Id:** SMGG-08

**Kind:** rule

**Name:** 8. `StateBody` alias (referenced from 08-file-structure.lark.md)

**Question:** Where is the `StateBody` production that `08-file-structure.lark.md` reaches through `StateSection` defined — the one production of §1, or a second copy stated in this section?

**Parent:** LANG-14

**Norm:** The `state_body` production defined in §1 above is the same one 08-file-structure.lark.md references via `state_section`.

**Why:** 08-file-structure.lark.md cites `StateBody` through `StateSection`, and this section says outright that the production it cites is the one of §1, so there is one definition and no second copy to drift.

---
