# 09 functions — Grammar

Companion grammar file for [9 — Functions](../09-functions.md). Defines the shape of function declarations, the `start:` entry block, parameter lists with default values, the keyword-prefixed forms (`constant`, `compiletime`, `host`), and the two signature syntaxes (type-first for definitions, arrow-return for capability methods). Semantic rules FNC-01..FNC-05 live in the companion chapter.

The `functions:` wrapping block that groups function declarations is defined in [08-file-structure.ebnf.md](./08-file-structure.ebnf.md) as `FunctionsBlock`. This file defines the individual `FunctionDeclaration` and related keyword-prefixed forms.

---

## 1. Ordinary function declaration (type-first)

```ebnf
(* FNC-02, FNC-03: type-first syntax.  ReturnType comes first, then
   name, then parameter list.  Body is an indented block.
   FNC-05: every call carries parentheses — but that's a rule on
   call sites (in 06-expressions.ebnf.md Call), not on declarations. *)

FunctionDeclaration = ReturnType, Identifier, "(", [ ParameterList ], ")",
                      NEWLINE, INDENT, FunctionBody, DEDENT ;

ParameterList   = Parameter, { ",", Parameter } ;

(* FNC-04: default parameter values.
   A parameter with a default is optional at the call site; must
   appear after all required parameters (semantic rule, not grammar). *)

Parameter       = TypeExpression, Identifier, [ "=", Expression ] ;
```

## 2. Function body

```ebnf
(* Function body may include optional description and input blocks
   before the statement sequence.  Grammar admits them in either
   order; the chapter shows description first, input second. *)

FunctionBody    = [ DescriptionClause ],
                  [ InputBlock ],
                  StatementSequence ;

DescriptionClause = "description", StringLiteral, NEWLINE ;

(* Input block — declares parameters with optional defaults inside
   the function body.  Equivalent to declaring them in the
   ParameterList, per FNC-04. *)

InputBlock      = "input", NEWLINE, INDENT,
                  InputParameter, NEWLINE,
                  { InputParameter, NEWLINE },
                  DEDENT ;

InputParameter  = TypeExpression, Identifier, [ "=", Expression ] ;
```

## 3. `start:` entry block

```ebnf
(* FNC-01: entry point.  One per file, at the top level, no
   parameters, no return type, block syntax with ":".  Only-one-
   per-file is enforced by the FileBody production in
   08-file-structure.ebnf.md, which admits [ StartSection ] with
   a single occurrence. *)

StartBlock      = "start", ":", NEWLINE, INDENT, StatementSequence, DEDENT ;
```

## 4. Keyword-prefixed function forms

```ebnf
(* Per the "Keyword-Prefixed Function Forms" table in the chapter,
   three prefixes exist.  Each declares ONE function outside a
   functions: block; the block form is reserved for ordinary
   functions. *)

ConstantFunctionDeclaration = "constant", "function", Identifier,
                              "(", [ ParameterList ], ")",
                              [ "returns", ReturnType ],
                              NEWLINE, INDENT, FunctionBody, DEDENT ;
                              (* Mirrors the shape of
                              CompileTimeFunctionDeclaration and
                              HostFunctionDeclaration (the other two
                              keyword-prefixed forms).  Chapter says
                              "Body allowed? Yes" without showing
                              concrete syntax; this is the only form
                              the repo has evidence for.  If the
                              chapter table entry is stale, catching
                              that is a separate cleanup. *)

(* Compile-time function grammar lives in 21-block-handlers.ebnf.md
   (its natural home per LEX-04 note on `returns` and per 21 §21.1).
   Referenced here for completeness. *)

CompileTimeFunctionDeclaration = ? see 21-block-handlers.ebnf.md ? ;

(* Host function grammar lives with the library-authoring surface,
   in the framework tree.  Referenced here for completeness. *)

HostFunctionDeclaration = ? see 02 components/framework/grammar/host-bridge.ebnf.md ? ;
                          (* ⚠ Placeholder path — same as in
                          08-file-structure.ebnf.md.  Real location
                          decided in Stage 2b. *)
```

## 5. Capability method signature (arrow-return)

```ebnf
(* FNC-03: capability methods use arrow-return syntax.  This form
   appears ONLY inside a `can` block (grammar in
   14-classes-and-objects.ebnf.md).  Published here for reference. *)

CapabilityMethodSignature = Identifier, "(", [ ParameterList ], ")",
                            "->", ReturnType ;
                            (* No body — capabilities are contracts
                               (CLS-03). *)
```

---

## Changelog

- 2026-08-07 (afternoon) — Resolved both `⚠` markers: (a) `constant function` concrete syntax stays as `constant function name(...) returns T` — mirrors the shape of the other prefixed forms (`compiletime function`, `host function`), and no other form has evidence in the repo; (b) `HostFunctionDeclaration` placeholder path stays at `02 components/framework/grammar/host-bridge.ebnf.md` (framework-scoped since LBS-02 lives in the framework spec). No production change.
- 2026-08-07 — File minted. Productions derived from FNC-01..FNC-05 in [09-functions.md](../09-functions.md) Accepted 2026-08-01.

---

## Metadata

- **Status:** Accepted (2026-08-07)
- **Audience:** Parser implementers; anyone defining or calling functions
- **Notation:** EBNF (ISO/IEC 14977)
- **Part of:** [04 language / grammar / README.md](./README.md)
- **Rules referencing this grammar:** [09-functions.md](../09-functions.md) (FNC-01..FNC-05)
- **References:** [03-lexical-structure.ebnf.md](./03-lexical-structure.ebnf.md), [04-type-system.ebnf.md](./04-type-system.ebnf.md) (TypeExpression, ReturnType), [06-expressions.ebnf.md](./06-expressions.ebnf.md), [07-statements.ebnf.md](./07-statements.ebnf.md) (StatementSequence), forward references to `14-classes-and-objects.ebnf.md`, `21-block-handlers.ebnf.md`
