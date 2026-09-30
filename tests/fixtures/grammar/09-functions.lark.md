# 09 functions — Grammar

**Title:** 09 functions — Grammar

**Id:** `FNCG-`

**Purpose:** This is the companion grammar of 9 — Functions: the productions for what that chapter describes in prose, with the semantic rules left there. It defines the ordinary type-first function declaration — return type, name, parameter list with optional defaults, indented body — the optional description clause and input block a body may open with, the `start:` entry block, and the `constant function` form; the `compiletime` and `host` forms are cited from the grammars that own them, and the arrow-return signature a capability method uses inside a `can` block is published here so the two signature syntaxes stand side by side. The `functions:` block that groups declarations is not defined here but in the file-structure grammar. It is written for implementers of the parser and for anyone checking a function's written shape against the law; it rests on 9 — Functions and on the file-structure grammar.

---

### FNCG-01 — 1. Ordinary function declaration (type-first)

**Id:** FNCG-01

**Kind:** rule

**Name:** 1. Ordinary function declaration (type-first)

**Question:** In the production for an ordinary function declaration, in what order do return type, name and parameter list stand, and may a parameter carry a default-value expression?

**Norm:**

```lark
// FNC-02, FNC-03: type-first syntax.  The return type comes first,
// then name, then parameter list.  Body is an indented block.  The
// return type is written as the type_expression that return_type
// (TYPG-04) is: inside a `public:` wrapper a function and a field
// (public_declaration, 17-modules-and-imports.lark.md) both begin with
// a type and a name, and only the "(" after the name tells them apart,
// so the parser must not have to decide earlier that the type is a
// return type.  ASY-02: the
// `background` modifier (18-async.lark.md) stands once, after the
// parameter list and before the body's _NEWLINE.  FNC-05: every call
// carries parentheses — but that's a rule on call sites (in
// 06-expressions.lark.md, call), not on declarations.

function_declaration: type_expression IDENTIFIER "(" parameter_list? ")" background_modifier? _NEWLINE _INDENT function_body _DEDENT

parameter_list: parameter ("," parameter)*

// FNC-04: default parameter values.  A parameter with a default is
// optional at the call site; must appear after all required
// parameters (semantic rule, not grammar).

parameter: type_expression IDENTIFIER ("=" expression)?
```

**Why:** an ordinary function is written type-first — return type, name, parameter list, indented body — and a parameter may carry a default; the grammar states that shape and leaves required-before-optional ordering and the parentheses-on-every-call rule to the semantic pass and to the call site.

---

### FNCG-02 — 2. Function body

**Id:** FNCG-02

**Kind:** rule

**Name:** 2. Function body

**Question:** Which optional clauses may a function body open with before its statement sequence, and does the grammar fix the order of those clauses or admit either?

**Norm:**

```lark
// A function body may open with a description and an input block,
// admitted in either order (the chapter shows description first, input
// second).  Then come the metadata prelude of `intent` and `spec`
// lines (19-ai-integration.lark.md, AIMG-03) and the contract prelude
// of `before:` then `after:` (10-contracts.lark.md, CTRG-02), each
// optional, and then the statement sequence.

function_body: function_opening? ai_metadata_prelude? contract_prelude? statement*

function_opening: description_clause input_block?
                | input_block description_clause?

description_clause: _DESCRIPTION string_literal _NEWLINE

// Input block — declares parameters with optional defaults inside the
// function body.  Equivalent to declaring them in the parameter_list,
// per FNC-04.

input_block: _INPUT _NEWLINE _INDENT (input_parameter _NEWLINE)+ _DEDENT

input_parameter: type_expression IDENTIFIER ("=" expression)?
```

**Why:** a body may open with a description and an input block before its statements, and input parameters are equivalent to those in the ParameterList per FNC-04; the grammar admits both clauses in either order rather than fixing the order the chapter happens to show.

---

### FNCG-03 — 3. `start:` entry block

**Id:** FNCG-03

**Kind:** rule

**Name:** 3. `start:` entry block

**Question:** What production shape does the `start:` entry block take — does it admit a parameter list or a return type, and is its one-per-file limit stated in this production?

**Parent:** LANG-07

**Norm:**

```lark
// FNC-01: entry point.  One per file, at the top level, no parameters,
// no return type, block syntax with ":".  Only-one-per-file is
// enforced by the file_body rule in 08-file-structure.lark.md, which
// admits start_section a single time.

start_block: _START ":" block
```

**Why:** the entry point has no parameters and no return type, only block syntax with ":", so its production is a bare statement sequence; one-per-file is not repeated here because the FileBody production in 08-file-structure.lark.md already admits StartSection once.

---

### FNCG-04 — 4. Keyword-prefixed function forms

**Id:** FNCG-04

**Kind:** rule

**Name:** 4. Keyword-prefixed function forms

**Question:** Which of the three keyword-prefixed function forms is given a concrete production in this grammar, and from which grammars are the other two cited?

**Norm:**

```lark
// Per the "Keyword-Prefixed Function Forms" table in the chapter, three
// prefixes exist.  Each declares ONE function inside the functions:
// block like any other (FNC-08); `host function` inside the `host
// interface` block of a library's host_bridge.cln (framework grammar
// 09-libraries-specification §2).

constant_function_declaration: _CONSTANT _FUNCTION IDENTIFIER "(" parameter_list? ")" (_RETURNS return_type)? _NEWLINE _INDENT function_body _DEDENT

// Mirrors the shape of compile_time_function_declaration and of the
// host function declaration (the other two keyword-prefixed forms).
// The chapter says "Body allowed? Yes" without showing concrete
// syntax; this is the only form the repo has evidence for.
//
// The compile-time function rule, compile_time_function_declaration,
// lives in 21-block-handlers.lark.md (its natural home per LEX-04 note
// on `returns` and per 21 §21.1).  Referenced here for completeness.
//
// The host function rule lives with the library-authoring surface, in
// the framework tree (09-libraries-specification §2), and is not part
// of the language grammar; the language grammar reserves `host` and
// leaves the form to that grammar.
```

**Why:** each of the three keyword-prefixed forms declares one function like any other, so the constant form is spelled out as the only shape the repo has evidence for, while compiletime and host are cited from their natural homes, and the host form is left to the library-authoring grammar that owns it rather than guessed at a placeholder path.

---

### FNCG-05 — 5. Capability method signature (arrow-return)

**Id:** FNCG-05

**Kind:** rule

**Name:** 5. Capability method signature (arrow-return)

**Question:** What written signature shape does a capability method take inside a `can` block, and does its production admit a body?

**Norm:**

```lark
// FNC-03: capability methods use arrow-return syntax.  This form
// appears ONLY inside a `can` block (grammar in
// 14-classes-and-objects.lark.md).  Published here for reference.  No
// body — capabilities are contracts (CLS-03).

capability_method_signature: IDENTIFIER "(" parameter_list? ")" "->" return_type
```

**Why:** a capability method is a contract with no body and uses arrow-return syntax, appearing only inside a `can` block; publishing the signature here beside the type-first form keeps the two signature syntaxes of FNC-03 visible side by side.

---
