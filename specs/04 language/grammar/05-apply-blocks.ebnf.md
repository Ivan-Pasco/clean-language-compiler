# 05 apply-blocks — Grammar

Companion grammar file for [05 — Apply-Blocks](../05-apply-blocks.md). Defines the shape of `identifier:`-headed blocks that apply their header to each indented item. The single semantic rule (APB-01) attached to this construct lives in the companion chapter.

`print:` looks like an apply-block but is a separate construct with its own semantic (SYN008); its grammar lives in [07-statements.ebnf.md](./07-statements.ebnf.md).

---

## 1. Apply-block form

```ebnf
(* APB-01: an apply-block applies its header to each indented item.
   The header is a callable expression that takes one argument; each
   line in the body is treated as one argument to that expression. *)

ApplyBlock     = ApplyHeader, ":", NEWLINE, INDENT, ApplyBody, DEDENT ;

ApplyHeader    = CallableExpression ;

(* CallableExpression is any expression that resolves to a callable
   accepting one argument. Grammar admits any expression here; the
   type checker verifies callability and arity.
   Common cases: a plain function name (`items.add`), a method chain
   (`report.messages.append`), a type keyword for grouped
   declarations (`integer`, `string`), or `constant`. *)

CallableExpression = Expression ;

ApplyBody      = ApplyItem, NEWLINE, { ApplyItem, NEWLINE } ;

(* Each ApplyItem is one call's argument. Union of two shapes, with
   the parser dispatching based on the header's kind:
     - Callable-style header (items.add:)     → ApplyItem is an Expression
     - TypeKeyword header (integer:, string:) → ApplyItem is name-with-init
     - constant: header                       → ApplyItem is a TypedDeclaration
   The two constructs look identical to the reader (identifier: + body)
   and the chapter treats them as one; splitting them in grammar would
   invite the reader to wonder why. *)

ApplyItem      = TypedDeclarationItem | ExpressionItem ;

ExpressionItem = Expression ;

(* When the header is a bare TypeKeyword (integer:, string:, etc.),
   each item is one variable in that type: name [ "=" Expression ].
   When the header is `constant:`, each item is a full TypedDeclaration
   (TypeExpression Identifier "=" Expression). *)

TypedDeclarationItem = Identifier, [ "=", Expression ]
                     | TypedDeclaration ;
```

## 2. `ConstantBody` alias (referenced from 08-file-structure.ebnf.md)

```ebnf
(* The body 08-file-structure.ebnf.md's ConstantSection delegates
   here — the `constant:` case of the ApplyBody shape above, made
   explicit.  One full declaration per line, initializer REQUIRED:
   a constant without a value has no meaning, so the grammar does
   not admit the initializer-less TypedDeclaration form here.
   At least one declaration, matching ApplyBody — an empty
   `constant:` section is dead weight. *)

ConstantBody        = ConstantDeclaration, NEWLINE,
                      { ConstantDeclaration, NEWLINE } ;

ConstantDeclaration = TypeExpression, Identifier, "=", Expression ;
```

---

## Changelog

- 2026-08-20 — Erratum from the compiler's Milestone 9 (`clean-language-compiler/docs/DISCOVERIES-M9.md` §1, item 1c): 08-file-structure.ebnf.md's `ConstantSection` referenced a `ConstantBody` no grammar file defined (its comment pointed here, but this file only defined the generic `ApplyBody`). New §2 defines `ConstantBody` = one or more `ConstantDeclaration` lines, where `ConstantDeclaration = TypeExpression, Identifier, "=", Expression` — the "full TypedDeclaration" case this file's §1 comment already described for `constant:` headers, with the initializer made grammatically mandatory.
- 2026-08-07 (afternoon) — Resolved the ApplyItem-shape `⚠` marker: apply-blocks and grouped-declarations remain a single unified construct with a union body; parser dispatches on the header's kind. Splitting them in grammar would invite the reader to wonder why since they look identical (`identifier:` + body). No production change.
- 2026-08-07 — File minted. Productions derived from APB-01 in [05-apply-blocks.md](../05-apply-blocks.md) Accepted 2026-08-01.

---

## Metadata

- **Status:** Accepted (2026-08-07)
- **Audience:** Parser implementers; downstream grammar-file authors referencing ApplyBlock
- **Notation:** EBNF (ISO/IEC 14977)
- **Part of:** [04 language / grammar / README.md](./README.md)
- **Rules referencing this grammar:** [05-apply-blocks.md](../05-apply-blocks.md) (APB-01)
- **References:** [03-lexical-structure.ebnf.md](./03-lexical-structure.ebnf.md) (Identifier, NEWLINE, INDENT, DEDENT, TypeKeyword), [04-type-system.ebnf.md](./04-type-system.ebnf.md) (TypedDeclaration, TypeExpression), [06-expressions.ebnf.md](./06-expressions.ebnf.md) (Expression), [08-file-structure.ebnf.md](./08-file-structure.ebnf.md) (ConstantSection — delegates to ConstantBody)
