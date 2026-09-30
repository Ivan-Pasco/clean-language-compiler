# 04 type-system — Grammar

**Title:** 04 type-system — Grammar

**Id:** `TYPG-`

**Purpose:** Companion grammar file of 04 — Type System per DOC-15. It defines the syntactic surface of type expressions — how a primitive, generic, class or compile-time type name is written in any type position, how generics are parameterised, the single optional marker `?`, and the behavior suffixes a `list<T>` declaration may carry — and publishes the two shapes other grammar files reference rather than restate: the type-first `typed_declaration` shared by variables, parameters and fields, and the `return_type` alias both return-type syntaxes accept. Where a form is admitted by the grammar but restricted by the type checker — `T??`, two removal disciplines at once — the production says so and names the semantic diagnostic. It is written for parser implementers and for the authors of the grammar files that reference it; the semantic rules (TYP-01..TYP-07 — ranges, conversions, string equality, behavior interactions) live in the parent chapter, and the fields of the compile-time types in 21 — Block Handlers.

---

### TYPG-01 — 1. Type expressions

**Id:** TYPG-01

**Kind:** rule

**Name:** 1. Type expressions

**Question:** Which written forms may stand in any type position — which primitive, generic, class and compile-time type names — and how many optional markers `?` may one type expression carry?

**Parent:** LANG-08

**Norm:**

```lark
// A type_expression is what appears in every type position: variable
// declaration, parameter, return type, field, and type argument.
// Optional (T?), generic (list<T>), and behavior-suffixed forms are all
// type expressions.

type_expression: base_type OPTIONAL_MARKER?

// Behavior suffixes are lexically part of the type only on the
// left-hand side of a variable declaration for list<T> — see TYPG-03
// below.  For every non-list type, type_expression is base_type with
// optional "?".

// PrimitiveType is the terminal PRIMITIVE_TYPE — boolean, integer,
// number, string, bytes, datetime, any, void — and GenericName the
// terminals LIST_NAME (list) and GENERIC_NAME (matrix, pairs), all
// three defined with the type keywords in 03-lexical-structure.lark.md.
//
// Generic types — list<T>, matrix<T>, pairs<K,V>.  The grammar admits
// any number of type arguments on any generic name; the type checker
// restricts which names take how many.
//
// A class_type is any user-defined class or capability name.
// Grammar-wise it is an IDENTIFIER used in type position; the type
// checker verifies it resolves to a declared class or capability.
//
// Compile-time types are the compiler-side values passed to and
// returned from `compiletime function` bodies — BlockAST, BlockNode,
// BlockArg, BlockAttribute, BlockLine, Token, IR, Span, Diagnostic.
// Their fields live in 21-block-handlers.md; here they are just their
// names in type position.  Each of those names is an IDENTIFIER, so a
// compile-time type is written, and read, as a class_type: a separate
// alternative spelling the same words would give every one of them two
// readings.  Which name denotes a compile-time type is the checker's.

base_type: PRIMITIVE_TYPE                          -> primitive_type
         | LIST_NAME "<" type_argument_list ">"     -> generic_type
         | GENERIC_NAME "<" type_argument_list ">"  -> generic_type
         | IDENTIFIER                              -> class_type

type_argument_list: type_expression ("," type_expression)*

// TYP-03: "?" (OPTIONAL_MARKER, 03-lexical-structure.lark.md) turns T
// into T?.  TYP-03 forbids T?? in source (SEM009); the grammar accepts a
// single "?" only.  A parser attempting to read a second "?" fails the
// type_expression rule and reports SEM009 as a semantic error.
```

**Why:** one production serves every type position — declaration, parameter, return, field, type argument — so optional, generic, class and compile-time forms all read as a TypeExpression; the grammar admits a single "?" and leaves T?? to the checker as SEM009.

---

### TYPG-02 — 2. Type-first declaration form (referenced by other files)

**Id:** TYPG-02

**Kind:** rule

**Name:** 2. Type-first declaration form (referenced by other files)

**Question:** In what order do the type and the name stand in a declaration of a variable, parameter or class field, and may an initializer expression follow that pair?

**Parent:** LANG-08

**Norm:**

```lark
// Referenced by 07-statements.lark.md, 05-apply-blocks.lark.md and
// 14-classes-and-objects.lark.md.  The declaration form is
// type_expression IDENTIFIER ("=" expression)?, which is the general
// shape for variables, parameters, and class fields.  The
// type_expression is omitted only when an initializer is present and
// its type is inferred unambiguously, giving IDENTIFIER "=" expression.

typed_declaration: explicit_declaration
                 | inferred_declaration

explicit_declaration: type_expression IDENTIFIER ("=" expression)?

inferred_declaration: IDENTIFIER "=" expression

// In statement position the inferred form has the shape of an
// assignment (07-statements.lark.md); the parser reads it as one, and
// it declares the name when none is in scope (TYP-11).  The two names
// exist so that a position which admits only the typed form — a
// variable declaration statement — can say so.
```

**Why:** variables, parameters and class fields share one declaration shape, <TypeExpression> <Identifier> [ "=" Expression ], so 07-statements.lark.md and 09-functions.lark.md reference TypedDeclaration here instead of each restating it.

---

### TYPG-03 — 3. List behaviors

**Id:** TYPG-03

**Kind:** rule

**Name:** 3. List behaviors

**Question:** In which position may a `list<T>` declaration carry behavior suffixes, are those suffixes part of the type itself, and does the grammar or the type checker reject a chain combining two removal disciplines?

**Parent:** LANG-12

**Norm:**

```lark
// TYP-05: a behavior suffix chain is written on the left side of a
// variable declaration only (07-statements.lark.md), immediately after
// list<T>.  It is part of the type; list<T> and list<T>.line are
// different types.  The chain holds at least one suffix here: list<T>
// with none is the generic_type of TYPG-01.

list_behavior_type: LIST_NAME "<" type_argument_list ">" behavior_suffix+

behavior_suffix: "." BEHAVIOR_NAME

// A behavior name is a word of this position only; the parser state
// after the "." of a suffix is the only one that offers it, and
// everywhere else line, pile and unique are identifiers.
BEHAVIOR_NAME.1: /(line|pile|unique)(?![A-Za-z0-9_])/

// TYP-05 forbids .line.pile and .line.unique.pile — two removal
// disciplines at once.  The grammar does not reject these; the type
// checker does, reporting SEM009.  This matches the "grammar admits,
// checker restricts" pattern used throughout the repo: the checker
// produces a semantic message explaining why two removal disciplines
// conflict, which is more useful than a raw parse error.  The same
// holds for a list with more than one type argument.
```

**Why:** a behavior suffix chain is part of the type — list<T> and list<T>.line differ — and is written only in a variable declaration; the grammar admits any chain and leaves two removal disciplines at once to the checker, whose SEM009 message beats a parse error.

---

### TYPG-04 — 4. Return-type positions (referenced by other files)

**Id:** TYPG-04

**Kind:** rule

**Name:** 4. Return-type positions (referenced by other files)

**Question:** Which type expressions may fill the return-type slot of both the type-first and the arrow-return signature, and is that slot defined once as a shared alias or restated by each return-type production?

**Parent:** LANG-08

**Norm:**

```lark
// Two return-type surface syntaxes exist per LEX-04 note on `returns`:
//   - Type-first (ordinary functions):
//       return_type IDENTIFIER "(" parameter_list? ")"
//   - Arrow-return (capability signatures):
//       IDENTIFIER "(" parameter_list? ")" "->" return_type
// Both accept any type_expression as return_type.  The concrete rules
// live in 09-functions.lark.md and 14-classes-and-objects.lark.md
// respectively; this file only publishes the return_type alias.

return_type: type_expression
```

**Why:** two return-type syntaxes exist, type-first for ordinary functions and arrow-return for capability signatures, and both take any TypeExpression; publishing the ReturnType alias here lets 09-functions and 14-classes-and-objects share it while each keeps its own concrete production.

---
