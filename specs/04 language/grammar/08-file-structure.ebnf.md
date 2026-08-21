# 08 file-structure — Grammar

Companion grammar file for [08 — File Structure](../08-file-structure.md). Defines the top-level shape of a `.cln` source file: which sections may appear at the top level, and the fixed order they must appear in when present. Semantic rules FIL-01 (order) and FIL-02 (only-listed-forms) live in the companion chapter.

The content of each section is defined by the grammar file for the chapter that owns it (functions, classes, tests, etc.); this file states only which sections may appear and the order between them.

---

## 1. File shape

```ebnf
(* FIL-01: sections are optional but must appear in this order.
   FIL-02: nothing else appears at the top level — no loose
   statements, calls, or assignments outside a section. *)

SourceFile     = { LineComment | BlockComment }, [ FileBody ] ;

FileBody       = [ ImportSection ],
                 [ SourceSection ],
                 [ ConstantSection ],
                 [ StateSection ],
                 { ClassOrCapabilityDeclaration },
                 [ CallableSection ],
                 { WatchBlock },
                 [ TestsSection ],
                 [ StartSection ] ;
                 (* ScreenBlock was removed per ADR-0030 — see
                    ../03-lexical-structure.ebnf.md changelog.
                    `screen` is not a keyword and no library
                    registers it as a block name; it is a free
                    identifier for user code. *)
```

## 2. Section-level slots

Each named section is a block-form the owning chapter defines in detail. This grammar file publishes the header shape and delegates the body to the referenced companion grammar file.

```ebnf
ImportSection  = "import", ":", NEWLINE, INDENT, ImportBody, DEDENT ;
                (* Body defined in 17-modules-and-imports.ebnf.md *)

SourceSection  = "source", ":", NEWLINE, INDENT, SourceBody, DEDENT ;
                (* Body defined in 19-ai-integration.ebnf.md *)

ConstantSection = "constant", ":", NEWLINE, INDENT, ConstantBody, DEDENT ;
                (* ConstantBody defined in 05-apply-blocks.ebnf.md §2 —
                   one ConstantDeclaration per line, initializer
                   required.  Semantics of constant-ness are in the
                   companion chapter. *)

StateSection   = "state", ":", NEWLINE, INDENT, StateBody, DEDENT ;
                (* Body defined in 20-state-management.ebnf.md *)

(* Class and capability declarations are top-level (not inside a
   wrapping section). Grammar in 14-classes-and-objects.ebnf.md. *)

ClassOrCapabilityDeclaration = ClassDeclaration | CapabilityDeclaration ;

(* The callable section groups four different callable forms.  Each
   has its own grammar in its owning file.  Both shapes are allowed:
     - all callables nested inside a single `functions:` block, OR
     - each callable standing at the top level with its own header keyword
   Existing library code uses both shapes — libraries/data/host_bridge.cln
   has bare `host function` at the top level, while regular functions
   nest inside `functions:` blocks. Forcing one shape would invalidate
   existing library code. *)

CallableSection = FunctionsBlock
                | { TopLevelCallable } ;

FunctionsBlock  = "functions", ":", NEWLINE, INDENT,
                  { FunctionDeclaration }, DEDENT ;
                  (* Body in 09-functions.ebnf.md *)

TopLevelCallable = FunctionDeclaration
                 | CompileTimeFunctionDeclaration
                 | HandlesBlockDeclaration
                 | HostFunctionDeclaration ;
                 (* Bodies:
                    - FunctionDeclaration              → 09-functions.ebnf.md
                    - CompileTimeFunctionDeclaration   → 21-block-handlers.ebnf.md
                    - HandlesBlockDeclaration          → 21-block-handlers.ebnf.md
                    - HostFunctionDeclaration          → 02 components/framework/grammar/host-bridge.ebnf.md
                    The last path is where the LBS-02 grammar will
                    live once its companion file is created (currently
                    inline in 09-libraries-specification.md §8.3). *)

(* WatchBlock — header and body defined in
   20-state-management.ebnf.md §5.  SMG-04 admits both watch
   targets — a single identifier (`watch total:`) and a
   parenthesized identifier list (`watch (a, b, c):`) — so the
   production is NOT restated here: an earlier restatement carried
   only the single-identifier form, making it a diverging duplicate
   definition, which is a defect per DOC-15. *)

TestsSection   = "tests", ":", NEWLINE, INDENT, TestsBody, DEDENT ;
                (* TestsBody defined in 11-testing.ebnf.md §1 *)

StartSection   = "start", ":", NEWLINE, INDENT, StatementSequence, DEDENT ;
                (* Body is a sequence of statements, grammar in
                   07-statements.ebnf.md.  FIL-01: `start:` MUST be
                   the last section — enforced by the FileBody
                   production placing it last with no repetition. *)
```

## 3. Framework-contributed sections

Library-registered blocks (`endpoints:`, `data:`, `component:`, and the rest) do not appear in the FIL-01 table but do appear at the top level. Their grammar depends on the block handler that owns them — see [21-block-handlers.ebnf.md](./21-block-handlers.ebnf.md).

```ebnf
(* A library-registered block appears wherever the library's
   library.toml specifies it in the section order. When unspecified,
   between FunctionsBlock and WatchBlock per FIL-01 prose.
   Grammar-wise the block is an ApplyBlock (or the generalised
   block form the block handler expects); the actual shape depends
   on the handler and is not fixed by this file. *)

LibraryBlock   = Identifier, ":", NEWLINE, INDENT,
                 ? handler-defined body ?, DEDENT ;
```

## 4. `public:` wrapper

Per FIL-01 prose: `public:` is not a section, it is a wrapper appearing *inside* a section to mark what that section exports. Grammar for `public:` is in [17-modules-and-imports.ebnf.md](./17-modules-and-imports.ebnf.md).

---

## Changelog

- 2026-08-20 — Three errata from the compiler's Milestone 9 (`clean-language-compiler/docs/DISCOVERIES-M9.md` §1, items 1c, 1d, 1f). (a) `ConstantSection`'s body reference now points at the real production: `ConstantBody` is defined in [05-apply-blocks.ebnf.md §2](./05-apply-blocks.ebnf.md) (it was referenced here but defined nowhere). (b) `TestsSection`'s comment now names `TestsBody`, newly defined in [11-testing.ebnf.md §1](./11-testing.ebnf.md) (the comment said "Body in 11-testing.ebnf.md" but 11 only defined `TestsBlock`, header included). (c) The inline `WatchBlock` production is **removed**: it duplicated [20-state-management.ebnf.md §5](./20-state-management.ebnf.md) with a diverging, narrower shape (`watch Identifier :` only), and a production defined in more than one place is a defect per DOC-15. The definition in 20 is authoritative — SMG-04 explicitly admits the parenthesized multi-variable target (`watch (a, b, c):`), so the single-identifier restatement here was wrong, not just redundant. `FileBody` still references `WatchBlock`; it now resolves to 20's definition.
- 2026-08-07 (afternoon, third pass) — `ScreenBlock` production removed and dropped from `FileBody` per [ADR-0030](../../01%20governance/decisions/0030-withdraw-screen-from-language.md). `screen` is not a keyword and no library registers it as a block name; it is a free identifier.
- 2026-08-07 (afternoon) — Resolved both `⚠` markers: (a) `functions:` block and bare top-level callables both remain allowed — existing library code uses both shapes and forcing one would invalidate `host_bridge.cln` files; (b) `HostFunctionDeclaration` grammar home stays at `02 components/framework/grammar/host-bridge.ebnf.md` (LBS-02 lives in the framework spec, so its companion grammar file belongs in the framework grammar folder per DOC-15). No production change.
- 2026-08-07 — File minted. Productions derived from FIL-01 and FIL-02 in [08-file-structure.md](../08-file-structure.md) Accepted 2026-08-01.

---

## Metadata

- **Status:** Accepted (2026-08-07)
- **Audience:** Parser implementers building the top-level file layout; tool authors that walk `.cln` files
- **Notation:** EBNF (ISO/IEC 14977)
- **Part of:** [04 language / grammar / README.md](./README.md)
- **Rules referencing this grammar:** [08-file-structure.md](../08-file-structure.md) (FIL-01, FIL-02)
- **References:** [03-lexical-structure.ebnf.md](./03-lexical-structure.ebnf.md), [05-apply-blocks.ebnf.md](./05-apply-blocks.ebnf.md), forward references to `07-statements.ebnf.md`, `09-functions.ebnf.md`, `11-testing.ebnf.md`, `14-classes-and-objects.ebnf.md`, `17-modules-and-imports.ebnf.md`, `19-ai-integration.ebnf.md`, `20-state-management.ebnf.md`, `21-block-handlers.ebnf.md`
