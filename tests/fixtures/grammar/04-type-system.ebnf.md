# 04 type-system — Grammar

Companion grammar file for [04 — Type System](../04-type-system.md). Defines the syntactic surface of type expressions: how a type name is written, how generics are parameterised, the optional-type marker, and the list-behavior suffixes. Semantic rules (TYP-01..TYP-07 — ranges, conversions, string equality, behavior interactions) live in the companion chapter.

---

## 1. Type expressions

```ebnf
(* A TypeExpression is what appears in every type position: variable
   declaration, parameter, return type, field, and type argument.
   Optional (T?), generic (list<T>), and behavior-suffixed forms
   are all TypeExpressions. *)

TypeExpression = BaseType, [ OptionalMarker ] ;

(* Behavior suffixes are lexically part of the type only on the
   left-hand side of a variable declaration for list<T> — see §3
   below.  For every non-list type, TypeExpression is BaseType with
   optional "?". *)

BaseType       = PrimitiveType
               | GenericType
               | ClassType
               | CompileTimeType ;

PrimitiveType  = "boolean"  | "integer" | "number"
               | "string"   | "bytes"   | "datetime"
               | "any"      | "void" ;

(* Generic types — list<T>, matrix<T>, pairs<K,V>.
   The grammar admits arbitrary generic identifiers here; the type
   checker restricts which identifiers may take type parameters. *)

GenericType    = GenericName, "<", TypeArgumentList, ">" ;

GenericName    = "list" | "matrix" | "pairs" ;

TypeArgumentList = TypeExpression, { ",", TypeExpression } ;

(* A ClassType is any user-defined class or capability name. Grammar-
   wise it is an Identifier used in type position; the type checker
   verifies it resolves to a declared class or capability. *)

ClassType      = Identifier ;

(* Compile-time types are the compiler-side values passed to and
   returned from `compiletime function` bodies. Their fields live in
   21-block-handlers.md; here they are just their names in type
   position. *)

CompileTimeType = "BlockAST"      | "BlockNode"    | "BlockArg"
                | "BlockAttribute"| "BlockLine"    | "Token"
                | "IR"            | "Span"         | "Diagnostic" ;

OptionalMarker = "?" ;
(* TYP-03: "?" turns T into T?. TYP-03 forbids T?? in source
   (SEM009); grammar accepts a single "?" only.  A parser attempting
   to read a second "?" fails the OptionalMarker production and
   reports SEM009 as a semantic error. *)
```

## 2. Type-first declaration form (referenced by other files)

```ebnf
(* Referenced by 07-statements.ebnf.md and 09-functions.ebnf.md.
   The declaration form is <TypeExpression> <Identifier>
   [ "=" Expression ], which is the general shape for variables,
   parameters, and class fields. *)

TypedDeclaration = TypeExpression, Identifier, [ "=", Expression ] ;
```

## 3. List behaviors

```ebnf
(* TYP-05: a behavior suffix chain is written on the left side of a
   variable declaration only, immediately after list<T>.  It is part
   of the type; list<T> and list<T>.line are different types. *)

ListBehaviorType = "list", "<", TypeExpression, ">", { BehaviorSuffix } ;

BehaviorSuffix = ".", BehaviorName ;

BehaviorName   = "line" | "pile" | "unique" ;

(* TYP-05 forbids .line.pile and .line.unique.pile — two removal
   disciplines at once. The grammar does not reject these; the type
   checker does, reporting SEM009. This matches the "grammar admits,
   checker restricts" pattern used throughout the repo: the checker
   produces a semantic message explaining why two removal disciplines
   conflict, which is more useful than a raw parse error. *)
```

## 4. Return-type positions (referenced by other files)

```ebnf
(* Two return-type surface syntaxes exist per LEX-04 note on
   `returns`:
     - Type-first (ordinary functions):     ReturnType Identifier "(" Params ")"
     - Arrow-return (capability signatures): Identifier "(" Params ")" "->" ReturnType
   Both accept any TypeExpression as ReturnType. The concrete
   productions live in 09-functions.ebnf.md and 14-classes-and-
   objects.ebnf.md respectively; this file only publishes the
   ReturnType alias. *)

ReturnType     = TypeExpression ;
```

---

## Changelog

- 2026-08-07 (afternoon) — Resolved the §3 `⚠` marker: list-behavior mutual exclusion (`.line.pile` invalid) stays as a semantic-checker rule (`SEM009`), not a grammar restriction. Matches the "grammar admits, checker restricts" pattern used throughout the repo. No production change.
- 2026-08-07 — File minted. Productions derived from TYP-01..TYP-07 in [04-type-system.md](../04-type-system.md) Accepted 2026-08-01.

---

## Metadata

- **Status:** Accepted (2026-08-07)
- **Audience:** Parser and type-checker implementers; downstream grammar-file authors referencing TypeExpression, TypedDeclaration, or ReturnType
- **Notation:** EBNF (ISO/IEC 14977)
- **Part of:** [04 language / grammar / README.md](./README.md)
- **Rules referencing this grammar:** [04-type-system.md](../04-type-system.md) (TYP-01..TYP-07)
- **References:** [03-lexical-structure.ebnf.md](./03-lexical-structure.ebnf.md) (Identifier, TypeKeyword)
