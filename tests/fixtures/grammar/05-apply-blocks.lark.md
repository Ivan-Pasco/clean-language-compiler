# 05 apply-blocks — Grammar

**Title:** 05 apply-blocks — Grammar

**Id:** `APBG-`

**Purpose:** Companion grammar file for 05 — Apply-Blocks, per DOC-15. It defines the one production for `identifier:`-headed blocks that apply their header to each indented item — the header any expression that resolves to a one-argument callable, the body a union the parser dispatches on the header's kind (a callable, a bare type keyword for grouped declarations, or `constant:`), with callability and arity left to the type checker — and the `constant_body` alias that `08-file-structure.lark.md` delegates here for a file's `constant:` section, with the initializer required. The single semantic rule attached to the construct (APB-01) lives in the companion chapter; `print:` looks like an apply-block but is a separate construct with its own semantic (SYN008), and its grammar lives in `07-statements.lark.md`. It is written for parser implementers and for the spec authors who cite its productions by name; it rests on 05 — Apply-Blocks and 08 — File Structure.

---

### APBG-01 — 1. Apply-block form

**Id:** APBG-01

**Kind:** rule

**Name:** 1. Apply-block form

**Question:** In Lark, what form must the apply-block production take — one production whose header is any expression and whose body is a union of item shapes the parser dispatches on the header's kind, or separate productions per kind of header?

**Parent:** LANG-06

**Norm:**

```lark
// APB-01: an apply-block applies its header to each indented item.
// The header is a callable expression that takes one argument; each
// line in the body is treated as one argument to that expression.
//
// One rule, apply_block.  Its body is a union of item shapes, and the
// parser dispatches on the header's kind — which is why each
// alternative pairs a kind of header with the item shape it takes:
//   - callable-style header (items.add:)     -> each item an expression
//   - type keyword header (integer:, string:) -> each item name-with-init
//   - constant: header                       -> each item a typed_declaration
// The constructs look identical to the reader (header ":" and an
// indented body) and the chapter treats them as one; splitting them
// into separate rules would invite the reader to wonder why.  The
// items are one per line, at least one.

apply_block: callable_expression ":" _NEWLINE _INDENT (expression_item _NEWLINE)+ _DEDENT
           | type_keyword ":" _NEWLINE _INDENT (typed_declaration_item _NEWLINE)+ _DEDENT
           | _CONSTANT ":" _NEWLINE _INDENT (typed_declaration _NEWLINE)+ _DEDENT

// A callable_expression is any expression that resolves to a callable
// accepting one argument.  The grammar admits any expression here; the
// type checker verifies callability and arity.  Common cases: a plain
// function name (`items.add`), a method chain
// (`report.messages.append`).  A type keyword (`integer`, `string`) is
// never an expression (LEX-04), so the grouped-declaration header is
// type_keyword (03-lexical-structure.lark.md), and `constant` is the
// hard keyword.

callable_expression: expression

expression_item: expression

// When the header is a bare type keyword (integer:, string:, etc.),
// each item is one variable in that type: name ("=" expression)?.
// When the header is `constant:`, each item is a full typed_declaration
// (04-type-system.lark.md).

typed_declaration_item: IDENTIFIER ("=" expression)?
```

**Why:** to the reader an apply-block is one construct — `identifier:` and an indented body — whatever its header is, so the grammar states it as one production whose body is a union the parser dispatches on the header's kind; splitting it into three productions would invite the reader to wonder why, and the comments carry the semantic of APB-01 that the type checker, not the grammar, enforces.

---

### APBG-02 — 2. `ConstantBody` alias (referenced from 08-file-structure.lark.md)

**Id:** APBG-02

**Kind:** rule

**Name:** 2. `ConstantBody` alias (referenced from 08-file-structure.lark.md)

**Question:** What form must the `ConstantBody` production that `08-file-structure.lark.md` delegates here take — at least one full typed declaration per line with the initializer required, or may a constant declaration be admitted without a value?

**Parent:** LANG-06

**Norm:**

```lark
// The body 08-file-structure.lark.md's constant_section delegates
// here — the `constant:` case of the apply_block above, made explicit.
// One full declaration per line, initializer REQUIRED: a constant
// without a value has no meaning, so the grammar does not admit the
// initializer-less declaration here.  At least one declaration,
// matching apply_block — an empty `constant:` section is dead weight.

constant_body: (constant_declaration _NEWLINE)+

constant_declaration: type_expression IDENTIFIER "=" expression
```

**Why:** the `constant:` section of a file is the apply-block's `constant:` case, so 08-file-structure.lark.md delegates here rather than restating the shape; it is made explicit with the initializer required, because a constant without a value has no meaning and the initializer-less form must not be admitted there.

---
