# 16 method-style-syntax — Grammar

Companion grammar file for [16 — Method-Style Syntax](../16-method-style-syntax.md). The single semantic rule (CALL-01) chosen by this chapter is a **convention for which existing call shape applies to which operation** — it does not introduce a new syntactic form. Every call shape it references is already defined in other grammar files:

| Shape | Home |
|---|---|
| Method style — `value.operation(args)` | [06-expressions.ebnf.md](./06-expressions.ebnf.md) — `PostfixExpression` with `MemberAccess` then `Call` |
| Namespace style — `module.operation(a, b)` | [06-expressions.ebnf.md](./06-expressions.ebnf.md) — same `MemberAccess`+`Call` pattern, distinguished only by the LHS being a namespace name |
| Operator form — `a + b`, `a == b`, `a and b` | [06-expressions.ebnf.md](./06-expressions.ebnf.md) — the precedence ladder |

There is no CALL-specific grammar production. This file exists to satisfy [DOC-15](../../01%20governance/00-documentation-principles.md#doc-15--grammar-is-the-source-of-truth-for-syntax-specs-cite-it)'s companion-file requirement and to explicitly state "no new productions" so a future author looking for a `MethodCall` production knows to find it in `06-expressions.ebnf.md` under `PostfixExpression`.

---

## 1. Nothing to define here

CALL-01 is a call-site *choice* rule enforced by the language's `LDR-08` "one way to do things" principle plus the type system. Grammar-wise, `text.length()` and `math.max(10, 20)` are identical shapes — both are `Expression "." Identifier "(" [ ArgumentList ] ")"`. The distinction between method-style and namespace-style is:

- **Semantic**, not syntactic — the LHS being an instance vs. a module.
- **Convention**, not enforcement — CALL-01's "one name" rule is a style guide backed by the standard library's design (no aliases exported).

No `⚠` markers — nothing is under-specified because nothing is defined here.

---

## Changelog

- 2026-08-07 — File minted as a companion-file placeholder per DOC-15. States explicitly that no new grammar productions are required; refers to [06-expressions.ebnf.md](./06-expressions.ebnf.md) for the call shapes CALL-01 applies to.

---

## Metadata

- **Status:** Accepted (2026-08-07)
- **Audience:** Anyone looking for method-call grammar (redirected to `06-expressions.ebnf.md`); spec editors verifying DOC-15 companion-file coverage
- **Notation:** EBNF (ISO/IEC 14977) — no productions defined
- **Part of:** [04 language / grammar / README.md](./README.md)
- **Rules referencing this grammar:** [16-method-style-syntax.md](../16-method-style-syntax.md) (CALL-01)
- **References:** [06-expressions.ebnf.md](./06-expressions.ebnf.md) (`PostfixExpression`, `MemberAccess`, `Call`)
