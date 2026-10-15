# Error Handling — Grammar

Derived from foundation `specs/language/errors.md`. `onError` follows the
statement whose expression can fail: a declaration, an assignment or an
expression on its own line. Its one-line form takes an expression or, when
the value is not used, a statement (`saveReport(report) onError
print(error.message)`); its block form takes an indented block. Whether a
block may fall through is the checker's rule (`SEM024`). `error(...)` and
`error.message` are expressions on the keyword `error` (Expressions).

```lark
%extend statement: guarded_statement
guarded_statement: guarded_head "onError" simple_statement _NL
    | guarded_head "onError" ":" _NL block
?guarded_head: var_decl
    | assignment
    | expression_statement
```
