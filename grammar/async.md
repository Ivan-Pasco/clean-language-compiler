# Async — Grammar

Derived from foundation `specs/language/async.md`. `start` before a call is
an expression that begins it in the background; `later` declares the
future that holds its result; `background` runs a call and discards its
result. The `background` mark of a function is in functions. The `start:`
section is told apart by position: only a top-level `start` is followed by
`:`.

```lark
%extend expression: start_expr
start_expr: "start" postfix

%extend simple_statement: later_decl | background_stmt
later_decl: "later" type NAME "=" expression
background_stmt: "background" postfix
```
