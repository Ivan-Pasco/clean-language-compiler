# Control Flow — Grammar

Derived from foundation `specs/language/syntax/control-flow.md`. `if` /
`else if` / `else`, `iterate` over a collection or a range with an optional
`step`, and `while`. Every body is an indented `block`. `step` is a
contextual keyword. The loop exits `break` and `continue` are named in the
prose of the specs but no example uses them, so they have no production
yet.

```lark
%extend compound_statement: if_stmt | iterate_stmt | while_stmt

if_stmt: "if" expression _NL block else_clause?
else_clause: "else" _NL block
    | "else" if_stmt

iterate_stmt: "iterate" NAME "in" expression ("to" expression ("step" expression)?)? _NL block
while_stmt: "while" expression _NL block
```
