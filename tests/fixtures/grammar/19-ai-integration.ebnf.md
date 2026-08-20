# 19 ai-integration — Grammar

Companion grammar file for [19 — AI Integration](../19-ai-integration.md). Defines the shape of the three provenance constructs: the `spec` statement (function-level, links to a specification file), the `intent` statement (function-level, natural-language purpose), and the `source:` block (file-level, marks the file as generated from a specification with a version). Semantic rules AIM-01..AIM-03 live in the companion chapter.

These constructs carry no runtime behaviour — they are metadata read by tooling. The MCP-server surface that consumes them is specified in `02 components/framework/10-mcp-server-architecture.md` and does not appear here.

---

## 1. `spec` statement

```ebnf
(* AIM-01: `spec "path"` — links a function to its specification
   document.  Appears only inside function or method bodies (a
   placement rule enforced by hosting FunctionBody, not grammar).
   Must appear before other statements except `intent`; also a
   placement rule.  Multiple `spec` declarations allowed. *)

SpecStatement   = "spec", StringLiteral ;
```

## 2. `intent` statement

```ebnf
(* AIM-02: `intent "description"` — natural-language purpose.
   Placement rules same as `spec` (before other statements
   except spec/intent siblings).  Multiple `intent` allowed. *)

IntentStatement = "intent", StringLiteral ;
```

## 3. Metadata prelude in function bodies

```ebnf
(* Order (chapter §Best Practices #2 and AIM-01/AIM-02 prose):
     intent (0..*) → spec (0..*) → contract prelude → statements
   Grammar admits any interleaving of `intent` and `spec` lines
   at the top of a function body; the CTR-01/CTR-02 ordering
   applies from the first non-metadata statement.  The relative
   order between `intent` and `spec` is deliberately not fixed —
   the chapter's Best Practices §2 shows `intent` first in one
   example and `spec` first in another. *)

AIMetadataPrelude = { SpecStatement, NEWLINE | IntentStatement, NEWLINE } ;
```

## 4. `source:` block (file-level)

```ebnf
(* AIM-03: `source:` block at the top of a file.  Must appear
   before any other declarations.  Two required fields: spec
   and version, both string literals. *)

SourceBlock     = "source", ":", NEWLINE, INDENT,
                  SourceField, NEWLINE,
                  SourceField, NEWLINE,
                  { SourceField, NEWLINE },
                  DEDENT ;
                  (* At least two entries required (spec, version).
                     Checker enforces exactly-one-of-each and both
                     required. *)

SourceField     = ( "spec" | "version" ), ":", StringLiteral ;
                  (* Only `spec` and `version` are allowed — closed
                     schema per DOC-18.  If future metadata is
                     needed, add it to AIM-03 explicitly; don't let
                     arbitrary fields slip in silently. *)

(* SourceBlock is the body of the SourceSection referenced by
   08-file-structure.ebnf.md's FileBody production. *)

SourceBody      = SourceField, NEWLINE, SourceField, NEWLINE ;
                  (* Alias for the body content that
                     08-file-structure delegates to.  Same shape
                     as SourceBlock's body. *)
```

---

## Changelog

- 2026-08-07 (afternoon) — Resolved both `⚠` markers: (a) relative order between `intent` and `spec` is deliberately unfixed — the chapter's Best Practices shows both orderings; (b) `SourceBlock` admits only `spec` and `version` — closed schema per DOC-18; future metadata must be added to AIM-03 explicitly. No production change.
- 2026-08-07 — File minted. Productions derived from AIM-01..AIM-03 in [19-ai-integration.md](../19-ai-integration.md) Accepted 2026-08-01.

---

## Metadata

- **Status:** Accepted (2026-08-07)
- **Audience:** Parser implementers; tooling that reads provenance metadata (MCP server, IDE, review agents)
- **Notation:** EBNF (ISO/IEC 14977)
- **Part of:** [04 language / grammar / README.md](./README.md)
- **Rules referencing this grammar:** [19-ai-integration.md](../19-ai-integration.md) (AIM-01..AIM-03)
- **References:** [03-lexical-structure.ebnf.md](./03-lexical-structure.ebnf.md), [08-file-structure.ebnf.md](./08-file-structure.ebnf.md) (SourceSection)
