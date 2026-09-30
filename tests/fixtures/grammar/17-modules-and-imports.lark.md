# 17 modules-and-imports — Grammar

**Title:** 17 modules-and-imports — Grammar

**Id:** `MODG-`

**Purpose:** Companion grammar file for 17 — Modules and Imports. Defines the shape of the `import:` block and its bare entries — a qualified module name with an optional alias, covering the whole-module, single-symbol and alias variations — the standalone file-path `import "path"` statement that stands at the top level beside the block rather than inside it, the `import_body` production that 08 — File Structure's grammar consumes as the block's body, and the `public:` wrapper that marks what a section exports. Semantic rules MOD-01..MOD-04 — including how a dotted name resolves to a file — live in the companion chapter; how libraries come into scope is the framework's question (LBS-01, FRM-01) and appears here nowhere. It is written for parser implementors and for the authors of the grammar files that consume its productions.

---

### MODG-01 — 1. `import:` block

**Id:** MODG-01

**Kind:** rule

**Name:** 1. `import:` block

**Question:** What shape must the standalone `ImportBlock` production take — header, indent/dedent, and how many one-line entries at minimum?

**Parent:** LANG-05

**Norm:**

```lark
// MOD-01: import: block.  Body is an indented list of import entries.
// Each entry names one module, optionally with an alias, and takes
// exactly one line.  File-path imports are not entries: they stand at
// the top level beside the block.  The block's body is import_body
// (MODG-04), at least one entry, one per line.

import_block: _IMPORT ":" _NEWLINE _INDENT import_body _DEDENT

import_entry: module_import
```

**Why:** the block is an indented list of one-line entries, so the production fixes exactly that — at least one entry, one per line — and hands the entry's two shapes to their own productions below rather than inlining both here.

---

### MODG-02 — 2. Module imports

**Id:** MODG-02

**Kind:** rule

**Name:** 2. Module imports

**Question:** In what written form must a module import be produced — one qualified dotted name with an optional `as` alias covering all four import variations, and does this grammar itself fix how a dotted name maps to a nested file path?

**Norm:**

```lark
// Module import — resolves by name within the compilation request
// (MOD-03).  Four variations per chapter §Import Variations:
//     - whole module          import: math
//     - single symbol         import: math.sqrt
//     - module alias          import: utils as u
//     - symbol alias          import: json.decode as jd

module_import: qualified_module_name (_AS IDENTIFIER)?

// Nested paths like `data.models` resolve to nested file paths
// (`data/models.cln`) by the deterministic textual rule of MOD-04 —
// shared by framework and compiler over the request's sources[] — not
// by this grammar.

qualified_module_name: IDENTIFIER ("." IDENTIFIER)*

// `as` is not a reserved word: it is a word of this position only,
// offered by the lexer in the one parser state that admits it, after
// a module name; everywhere else it is an identifier.

_AS.1: /as(?![A-Za-z0-9_])/
```

**Why:** the chapter's four import variations collapse into one production — a qualified name and an optional alias — and the mapping of a dotted name to a nested file path is left to MOD-04, so the grammar does not repeat a resolution rule that framework and compiler share.

---

### MODG-03 — 3. File path imports

**Id:** MODG-03

**Kind:** rule

**Name:** 3. File path imports

**Question:** What tells a file-path import from a module import in the grammar, and may the `import "path"` form stand as an entry inside an `import:` block or only as a standalone top-level statement beside it?

**Norm:**

```lark
// Direct file path import — path is a string literal relative to the
// importing file.  Distinguished from module_import by the
// string-literal syntax (no bare IDENTIFIER).  It is a standalone
// top-level statement, NOT an entry inside an `import:` block:
// block-form is for module names, file-path form is for direct paths.
// Mixing them inside one block would confuse readers about resolution
// order.  A file may contain BOTH an import: block AND standalone
// `import "path"` lines — they serve different purposes and no reason
// to force a project to pick one style.  Placed at the top level, not
// inside a block: file_body holds it in its import slot
// (08-file-structure.lark.md).

file_path_import: _IMPORT string_literal
```

**Why:** the string literal is what tells a file path from a module name, and keeping this form out of the `import:` block spares the reader a block whose entries resolve two different ways; a file may still carry both, because they serve different purposes.

---

### MODG-04 — 4. Aggregate: the import body used by 08-file-structure

**Id:** MODG-04

**Kind:** rule

**Name:** 4. Aggregate: the import body used by 08-file-structure

**Question:** What may the `ImportBody` production that 08 — File Structure's `ImportSection` consumes as its DSL body admit, and must each entry carry its own `import` keyword?

**Parent:** LANG-05

**Norm:**

```lark
// import_body is what 08-file-structure.lark.md's import_section
// consumes, through import_block, as its body.  The section is the
// indented body of the `import:` header.  A file may have both an
// `import:` block AND standalone `import "path"` file-path imports;
// the two forms are distinct and both admitted at the top level.
// Bare module imports inside the block — the import_entry of MODG-01,
// no `import` keyword per entry, indentation alone binding them to the
// block header.  At least one: the lexer emits no _INDENT for an empty
// body.

import_body: (import_entry _NEWLINE)+
```

**Why:** 08-file-structure.lark.md consumes the block's body as a DSL body, so it needs one named production of bare entries — no `import` keyword per line, indentation alone binding them to the header — with the reminder that file-path imports stand beside the block, not inside it.

---

### MODG-05 — 5. `public:` wrapper

**Id:** MODG-05

**Kind:** rule

**Name:** 5. `public:` wrapper

**Question:** What production shape must the `public:` export wrapper take, and which declaration forms may stand inside its body?

**Norm:**

```lark
// MOD-02: private by default.  A name inside a `public:` wrapper is
// exported; outside it is module-local.  There is no `private` keyword.
// `public:` appears INSIDE a section (functions:, class body) marking
// what that section exports.  It is NOT a top-level section itself
// (per FIL-01 chapter prose).  Grammar-wise, public_wrapper is a block
// that hosts declarations of the same shape as the section it lives
// in.

public_wrapper: _PUBLIC ":" _NEWLINE _INDENT public_body _DEDENT

public_body: public_declaration*

// public_declaration is any declaration that would legally appear at
// the current nesting level: a function_declaration inside functions:,
// a field_declaration in a class body.  The grammar admits the union;
// the checker verifies the declaration is valid in the outer context.

public_declaration: function_declaration
                  | field_declaration

// 2026-08-22: class and capability declarations retired from this
// union — classes and capabilities are top-level declarations (08
// §FIL-01) with no legal position inside a public: wrapper, and they
// travel with the module without an export marker (MOD-02).  The
// functions: block likewise removed: the wrapper lives INSIDE
// functions:/class bodies, so the members are function_declaration /
// field_declaration.  Extend this union only if a form actually
// becomes public-wrappable.
```

**Why:** there is no `private` keyword, so exporting is a wrapper inside a section rather than a section of its own, and its body union names only what can stand inside `functions:` or a class body — classes and capabilities left it because they travel with the module without one.

---
