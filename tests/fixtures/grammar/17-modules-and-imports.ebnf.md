# 17 modules-and-imports — Grammar

Companion grammar file for [17 — Modules and Imports](../17-modules-and-imports.md). Defines the shape of the `import:` block (both module-name and direct-file-path forms), import variations (whole module / single symbol / aliases), and the `public:` wrapper that controls what a module exports. Semantic rules MOD-01..MOD-03 live in the companion chapter.

The chapter also states that "libraries are a separate question from modules" — folder scope (defined by `clean.toml [folders]`) is the normal way libraries come into scope; `import` only intervenes for explicit disambiguation of block-name conflicts. That decision surface belongs to the framework spec (LBS-01, FRM-01); this grammar file covers only the language-side `import:` block.

---

## 1. `import:` block

```ebnf
(* MOD-01: import: block.  Body is an indented list of import
   entries.  Each entry names one module or file path, optionally
   with an alias, and takes exactly one line. *)

ImportBlock     = "import", ":", NEWLINE, INDENT,
                  ImportEntry, NEWLINE,
                  { ImportEntry, NEWLINE },
                  DEDENT ;

ImportEntry     = ModuleImport
                | FilePathImport ;
```

## 2. Module imports

```ebnf
(* Module import — resolves by name within the compilation request
   (MOD-03).  Four variations per chapter §Import Variations:
     - whole module          import: math
     - single symbol         import: math.sqrt
     - module alias          import: utils as u
     - symbol alias          import: json.decode as jd *)

ModuleImport    = QualifiedModuleName, [ "as", Identifier ] ;

QualifiedModuleName = Identifier, { ".", Identifier } ;
                  (* Nested paths like `data.models` resolve to
                     nested file paths (`data/models.cln`) — that
                     resolution is done by the framework before
                     the compiler runs (MOD-03), not by this
                     grammar. *)
```

## 3. File path imports

```ebnf
(* Direct file path import — path is a string literal relative to
   the importing file.  Distinguished from ModuleImport by the
   string-literal syntax (no bare Identifier).  It is a standalone
   top-level statement, NOT an entry inside an `import:` block:
   block-form is for module names, file-path form is for direct
   paths.  Mixing them inside one block would confuse readers
   about resolution order.
   A file may contain BOTH an import: block AND standalone
   `import "path"` lines — they serve different purposes and no
   reason to force a project to pick one style. *)

FilePathImport  = "import", StringLiteral ;
                  (* Placed at the top level, not inside a block. *)
```

## 4. Aggregate: the import body used by 08-file-structure

```ebnf
(* ImportBody is what 08-file-structure.ebnf.md's ImportSection
   consumes as its DSL-body.  The section is the indented body
   of the `import:` header.  A file may have both an `import:`
   block AND standalone `import "path"` file-path imports; the
   two forms are distinct and both admitted at the top level. *)

ImportBody      = { ModuleImport, NEWLINE } ;
                  (* Bare ModuleImport entries inside the block —
                     no `import` keyword per entry, indentation
                     alone binds them to the block header. *)
```

## 5. `public:` wrapper

```ebnf
(* MOD-02: private by default.  A name inside a `public:` wrapper
   is exported; outside it is module-local.  There is no `private`
   keyword.
   `public:` appears INSIDE a section (functions:, class body, etc.)
   marking what that section exports.  It is NOT a top-level
   section itself (per FIL-01 chapter prose).  Grammar-wise,
   PublicWrapper is a block that hosts declarations of the same
   shape as the section it lives in. *)

PublicWrapper   = "public", ":", NEWLINE, INDENT,
                  PublicBody, DEDENT ;

PublicBody      = { PublicDeclaration } ;

(* PublicDeclaration is any declaration that would legally appear
   at the current nesting level: a FunctionDeclaration, a
   FunctionsBlock, a ClassDeclaration, a FieldDeclaration in a
   class body, etc.  Grammar admits the union; the checker
   verifies the declaration is valid in the outer context. *)

PublicDeclaration = FunctionsBlock
                  | FunctionDeclaration
                  | ClassOrCapabilityDeclaration
                  | FieldDeclaration ;
                  (* Extend this union if more forms become
                     public-wrappable. *)
```

---

## Changelog

- 2026-08-07 (afternoon) — Resolved both `⚠` markers: (a) `import "path"` file-path form is a standalone top-level statement, NOT an entry inside an `import:` block — chapter examples consistently show it that way, and mixing them inside one block would confuse readers about resolution order; (b) a file may contain BOTH an `import:` block AND standalone `import "path"` lines — they serve different purposes (block-form for module names, standalone for direct paths). No production change.
- 2026-08-07 — File minted. Productions derived from MOD-01..MOD-03 in [17-modules-and-imports.md](../17-modules-and-imports.md) Accepted 2026-08-01.

---

## Metadata

- **Status:** Accepted (2026-08-07)
- **Audience:** Parser implementers; module-resolution tooling
- **Notation:** EBNF (ISO/IEC 14977)
- **Part of:** [04 language / grammar / README.md](./README.md)
- **Rules referencing this grammar:** [17-modules-and-imports.md](../17-modules-and-imports.md) (MOD-01..MOD-03)
- **References:** [03-lexical-structure.ebnf.md](./03-lexical-structure.ebnf.md), [08-file-structure.ebnf.md](./08-file-structure.ebnf.md), [09-functions.ebnf.md](./09-functions.ebnf.md), [14-classes-and-objects.ebnf.md](./14-classes-and-objects.ebnf.md)
