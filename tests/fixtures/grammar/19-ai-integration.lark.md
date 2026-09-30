# 19 ai-integration — Grammar

**Title:** 19 ai-integration — Grammar

**Id:** `AIMG-`

**Purpose:** Companion grammar file for 19 — AI Integration, written for parser implementors and for readers of that chapter who need the exact shape of a provenance construct. It defines the three constructs: the `spec` statement (function-level, links to a specification file), the `intent` statement (function-level, natural-language purpose), and the `source:` block (file-level, marks the file as generated from a specification with a version, with a closed schema of exactly those two fields); it also fixes the metadata prelude of a function body — `spec` and `intent` lines in any interleaving before the contract prelude and the statements — and the body that the file-structure grammar delegates to for the `source:` section. Placement rules stay with the hosting productions, and the semantic rules (AIM-) live in the companion chapter. These constructs carry no runtime behaviour — they are metadata read by tooling; the MCP-server surface that consumes them is specified elsewhere and does not appear here.

---

### AIMG-01 — 1. `spec` statement

**Id:** AIMG-01

**Kind:** rule

**Name:** 1. `spec` statement

**Question:** What written form must a `spec` statement take in the grammar — keyword plus string literal — and does its production itself encode where in a function body it may stand?

**Parent:** LANG-18

**Norm:**

```lark
// AIM-01: `spec "path"` — links a function to its specification
// document.  Appears only inside function or method bodies (a placement
// rule enforced by the hosting function_body, through
// ai_metadata_prelude, not by this rule).  Must appear before other
// statements except `intent`; also a placement rule.  Multiple `spec`
// declarations allowed.

spec_statement: _SPEC string_literal
```

**Why:** the statement is a keyword and a string literal, nothing more, and its placement — function bodies only, before other statements — belongs to the hosting `FunctionBody`, so the production stays minimal and does not encode position.

---

### AIMG-02 — 2. `intent` statement

**Id:** AIMG-02

**Kind:** rule

**Name:** 2. `intent` statement

**Question:** What written form must an `intent` statement take, and does it get a production of its own distinct from the `spec` statement's?

**Parent:** LANG-18

**Norm:**

```lark
// AIM-02: `intent "description"` — natural-language purpose.  Placement
// rules same as `spec` (before other statements except spec/intent
// siblings).  Multiple `intent` allowed.

intent_statement: _INTENT string_literal
```

**Why:** `intent` mirrors `spec` in shape and placement, so it gets a production of its own only because it is a different keyword carrying a different meaning, not because its form differs.

---

### AIMG-03 — 3. Metadata prelude in function bodies

**Id:** AIMG-03

**Kind:** rule

**Name:** 3. Metadata prelude in function bodies

**Question:** In what order may `intent` and `spec` lines stand in a function body's metadata prelude, and from which statement does the contract-prelude ordering take over?

**Norm:**

```lark
// Order (chapter §Best Practices #2 and AIM-01/AIM-02 prose):
//     intent (zero or more) -> spec (zero or more) -> contract prelude
//     -> statements
// The grammar admits any interleaving of `intent` and `spec` lines at
// the top of a function body; the CTR-01/CTR-02 ordering applies from
// the first non-metadata statement.  The relative order between
// `intent` and `spec` is deliberately not fixed — the chapter's Best
// Practices §2 shows `intent` first in one example and `spec` first in
// another.  The prelude holds at least one line; function_body
// (09-functions.lark.md) makes it optional.

ai_metadata_prelude: (spec_statement _NEWLINE | intent_statement _NEWLINE)+
```

**Why:** the chapter shows `intent` before `spec` in one example and `spec` first in another, so the prelude admits any interleaving of the two rather than fixing an order the chapter never fixed, and hands the contract ordering to CTR-01/CTR-02 from the first non-metadata statement.

---

### AIMG-04 — 4. `source:` block (file-level)

**Id:** AIMG-04

**Kind:** rule

**Name:** 4. `source:` block (file-level)

**Question:** What shape must the file-level `source:` block take — which field names it admits, how many entries its production requires, and what body does 08-file-structure delegate to it?

**Parent:** LANG-18

**Norm:**

```lark
// AIM-03: `source:` block at the top of a file.  Must appear before
// any other declarations.  Two required fields: spec and version, both
// string literals.  A third field, generator, is required in generated
// files and absent in hand-written ones (LANG-18: a generated file MUST
// declare its generator as well as its source).

source_block: _SOURCE ":" _NEWLINE _INDENT source_body _DEDENT

// source_block's body is the body 08-file-structure.lark.md's
// source_section reaches through it.  At least two entries required
// (spec, version); generated files carry a third (generator).  The
// checker enforces exactly-one-of-each, spec and version required
// always, and generator required whenever the file is generated.

source_body: source_field _NEWLINE source_field _NEWLINE (source_field _NEWLINE)*

// Only `spec`, `version` and `generator` are allowed — closed schema
// per DOC-18.  If future metadata is needed, add it to AIM-03
// explicitly; don't let arbitrary fields slip in silently.

source_field: _SPEC ":" string_literal       -> spec_field
            | _VERSION ":" string_literal    -> version_field
            | _GENERATOR ":" string_literal  -> generator_field

// `version` and `generator` are words of this position only: the lexer
// offers them in the one parser state that admits a field name, and
// they are identifiers everywhere else.  `spec` is the hard keyword.

_VERSION.1: /version(?![A-Za-z0-9_])/
_GENERATOR.1: /generator(?![A-Za-z0-9_])/
```

**Why:** the block has exactly two fields and no room for more, so its schema is closed — a new field is an amendment to AIM-03, not something that slips in — while exactly-one-of-each is the checker's, and `SourceBody` lets 08-file-structure.lark.md delegate instead of restating.

---
