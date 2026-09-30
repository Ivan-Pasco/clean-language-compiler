# 08 file-structure — Grammar

**Title:** 08 file-structure — Grammar

**Id:** `FILG-`

**Purpose:** Companion grammar file for 08 — File Structure, written for the parser implementer and the grammar author. It defines the top-level shape of a `.cln` source file: which sections may appear at the top level, the fixed order they take when present, the header shape of each section with its body delegated to the grammar file of the chapter that owns it, where a library-registered block sits among them, and that `public:` is a wrapper inside a section rather than a section of its own. The semantic rules FIL-01 (order) and FIL-02 (only the listed forms) live in the companion chapter; the content of each section belongs to its owning grammar — functions, classes, tests, state, modules — and library blocks to 21-block-handlers.lark.md, so this file states only what may appear at the top level and in what order.

---

### FILG-01 — 1. File shape

**Id:** FILG-01

**Kind:** rule

**Name:** 1. File shape

**Question:** Which sections named in the FIL-01 table may stand at the top level of a `.cln` source file, in what fixed order must they appear when present, and may a loose statement, call, assignment or a `screen` block stand there?

**Parent:** LANG-07

**Norm:**

```lark
// FIL-01: sections are optional but must appear in this order.
// FIL-02: nothing else appears at the top level — no loose statements,
// calls, or assignments outside a section.  source_file is where a
// parser reading a whole .cln file starts; the comments before the
// first section are line ends to it (LEXG-02).

source_file: _NEWLINE? file_body

file_body: import_part? source_section? constant_section? state_section? class_or_capability_declaration* callable_section? library_block* watch_block* tests_section? start_section?

// The import slot holds the `import:` section and the standalone
// file-path imports that stand beside it at the top level
// (17-modules-and-imports.lark.md, MODG-03), in any order around it.

import_part: file_path_imports
           | file_path_imports? import_section file_path_imports?

file_path_imports: file_path_import _NEWLINE
                 | file_path_imports file_path_import _NEWLINE

// A library-registered block stands between the callable section and
// the watch blocks, where FILG-03 places it when library.toml does not;
// its rule is library_block (21-block-handlers.lark.md).
//
// ScreenBlock was removed per ADR-0030 — see the lexical grammar's
// history in git.  `screen` is not a keyword and no library registers
// it as a block name; it is a free identifier for user code.
```

**Why:** the top level of a `.cln` file is a fixed sequence of optional sections and nothing else, so FIL-01's order and FIL-02's only-listed-forms are both encoded in one FileBody production; the comment records why ScreenBlock is gone, so nobody restores it from an older reading.

---

### FILG-02 — 2. Section-level slots

**Id:** FILG-02

**Kind:** rule

**Name:** 2. Section-level slots

**Question:** What header shape does each named top-level section (`import:`, `source:`, `constant:`, `state:`, the callable slot, `tests:`, `start:`) take in this file, and is its body production restated here or delegated to the grammar of the chapter that owns it?

**Parent:** LANG-07

**Norm:**

Each named section is a block-form the owning chapter defines in detail. This grammar file publishes the header shape and delegates the body to the referenced companion grammar file.

```lark
// Each section's body lives in the grammar of the chapter that owns
// it; the section rules here name the header's home and delegate.

import_section: import_block
                // `import:` header and body: import_block in
                // 17-modules-and-imports.lark.md

source_section: source_block
                // `source:` header and body: source_block in
                // 19-ai-integration.lark.md

constant_section: _CONSTANT ":" _NEWLINE _INDENT constant_body _DEDENT
                // constant_body is defined in 05-apply-blocks.lark.md
                // APBG-02 — one constant_declaration per line,
                // initializer required.  Semantics of constant-ness
                // are in the companion chapter.

state_section: state_block
                // `state:` header and body: state_block in
                // 20-state-management.lark.md

// Class and capability declarations are top-level (not inside a
// wrapping section).  Grammar in 14-classes-and-objects.lark.md.

class_or_capability_declaration: class_declaration
                               | capability_declaration

// FIL-01, FNC-02, FNC-08 — one shape (LANG-07): every function,
// keyword-prefixed or not, is nested inside the `functions:` block; a
// `handles block` registration stands beside it; `host function`
// declarations are grouped in a `host interface` block, which only a
// library's host_bridge.cln carries (LBS-21).  The host interface
// block's rule belongs to the library-authoring grammar (framework
// 09-libraries-specification.lark.md LBSG-02), which reads a
// host_bridge.cln as a text of its own — its start `host_bridge_file`,
// one host interface after the comments that may open the file — and
// adds nothing to callable_section; the language grammar has no host
// interface, and `host` is a reserved word it leaves unused (LEX-04).

callable_section: functions_block handles_block_declaration*
                | handles_block_declaration+

functions_block: _FUNCTIONS ":" _NEWLINE _INDENT functions_block_member* _DEDENT

functions_block_member: function_declaration
                      | constant_function_declaration
                      | compile_time_function_declaration
                      | public_wrapper

// Bodies:
//   - function_declaration, constant_function_declaration
//                                      -> 09-functions.lark.md
//   - compile_time_function_declaration -> 21-block-handlers.lark.md
//   - handles_block_declaration         -> 21-block-handlers.lark.md
//   - public_wrapper, the `public:` that marks which functions the
//     module exports (FILG-04)           -> 17-modules-and-imports.lark.md

// watch_block — header and body defined in
// 20-state-management.lark.md SMGG-05.  SMG-04 admits both watch
// targets — a single identifier (`watch total:`) and a parenthesized
// identifier list (`watch (a, b, c):`) — so the rule is NOT restated
// here: an earlier restatement carried only the single-identifier
// form, making it a diverging duplicate definition, which is a defect
// per DOC-15.

tests_section: tests_block
                // `tests:` header and body: tests_block in
                // 11-testing.lark.md TSTG-01

start_section: start_block
                // `start:` header and body: start_block in
                // 09-functions.lark.md FNCG-03.  FIL-01: `start:` MUST
                // be the last section — enforced by the file_body rule
                // placing it last with no repetition.
```

**Why:** this file owns only the header shape of each section and delegates each body to the chapter's grammar, one home per production; the CallableSection makes the one-shape rule of LANG-07 concrete, and WatchBlock is deliberately not restated after an earlier restatement diverged from 20's definition.

---

### FILG-03 — 3. Framework-contributed sections

**Id:** FILG-03

**Kind:** rule

**Name:** 3. Framework-contributed sections

**Question:** At which position among the top-level sections does a library-registered block — which sits outside the FIL-01 table — appear when `library.toml` does not place it, and which grammar file defines its `LibraryBlock` production?

**Parent:** LANG-19

**Norm:**

Library-registered blocks (`endpoints:`, `data:`, `component:`, and the rest) do not appear in the FIL-01 table but do appear at the top level. Their grammar depends on the block handler that owns them — see 21-block-handlers.lark.md.

```lark
// A library-registered block appears wherever the library's
// library.toml specifies it in the section order.  When unspecified,
// between the callable section and the watch blocks per FIL-01 prose:
// file_body (FILG-01) holds it in that slot.  library_block — qualified
// block name, header argument surfaces, and body — is defined once in
// 21-block-handlers.lark.md BLKG-02 (DOC-15: one home per rule; same
// pattern as watch_block resolving to 20's definition).
```

**Why:** library-registered blocks sit outside the FIL-01 table yet appear at the top level, so this section says where they go — where library.toml places them, else between FunctionsBlock and WatchBlock — and points to 21-block-handlers.lark.md §1a for LibraryBlock rather than naming a second production here.

---

### FILG-04 — 4. `public:` wrapper

**Id:** FILG-04

**Kind:** rule

**Name:** 4. `public:` wrapper

**Question:** Is `public:` a section slot of the `FileBody` production or a wrapper appearing inside a section, and which grammar file carries its production?

**Norm:** Per FIL-01 prose: `public:` is not a section, it is a wrapper appearing *inside* a section to mark what that section exports. Grammar for `public:` is in 17-modules-and-imports.lark.md.

**Why:** `public:` looks like a section but is a wrapper inside one marking what it exports, so it is named here only to send the reader to 17-modules-and-imports.lark.md; without this note a parser author would look for a PublicSection slot in FileBody.

---
