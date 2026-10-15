# Expressions — Grammar

Derived from foundation `specs/language/syntax/expressions.md`. One
production per level of the precedence table, highest last. All binary
operators are left-associative except `^`, which is right-associative and
binds tighter than a leading minus, while a minus in the exponent applies
to the exponent (`2 ^ -n`).

After a dot, a member may be a reserved name (a core type or module name):
the examples call `input.integer()` and `input.number()` and read the JSON
key `data.matrix`. Keywords are never members.

The table's levels 1 (primary: calls, method calls, property access) and 2
(postfix `!`) are one chain, `postfix`, because the examples chain them both
ways: `getText()!.toUpperCase()`. Assignment (level 12) is a statement
(Statements). An expression spans lines only inside parentheses, brackets
or braces (the indenter emits no line event there).

```lark
?expression: default_expr

?default_expr: or_expr ("default" or_expr)*
?or_expr: and_expr ("or" and_expr)*
?and_expr: equality ("and" equality)*
?equality: comparison (equality_op equality_operand)*
!equality_op: "==" | "!=" | "is" | "isnt"
?equality_operand: comparison | type_name
!type_name: "boolean" | "integer" | "number" | "string" | "bytes" | "datetime" | "list" | "matrix" | "pairs"
?comparison: additive (comparison_op additive)*
!comparison_op: "<" | ">" | "<=" | ">="
?additive: multiplicative (additive_op multiplicative)*
!additive_op: "+" | "-"
?multiplicative: unary (multiplicative_op unary)*
!multiplicative_op: "*" | "/" | "%"
?unary: unary_op unary | power
!unary_op: "not" | "-"
?power: postfix ("^" unary)?

?postfix: primary
    | postfix "(" arguments? ")"   -> call
    | postfix "." member_name      -> member
    | postfix "[" expression "]"   -> index
    | postfix "!"                  -> required

!member_name: NAME | "boolean" | "bytes" | "datetime" | "file" | "http" | "input" | "integer" | "json" | "list" | "math" | "matrix" | "number" | "pairs" | "string" | "time" | "void"

arguments: argument ("," argument)*
argument: expression
    | NAME ":" expression          -> named_argument
```

## Primary expressions

A `type_name` on the right of an equality operator makes a type test (`data is pairs`); a
class name or `none` there is an ordinary operand. That only `is` and
`isnt` take a type is the checker's rule, not the grammar's. A built-in module name
is reached only through a member (`math.sqrt`, `list.range`), since a
module name is never a value.

```lark
?primary: INT | FLOAT | STRING
    | "true" | "false" | "none"
    | NAME
    | "error"
    | "base"
    | "(" expression ")"
    | list_literal
    | pairs_literal
    | module_member

module_member: module_name "." member_name
!module_name: "math" | "string" | "list" | "file" | "http" | "time" | "json" | "input"

list_literal: "[" (expression ("," expression)*)? "]"
pairs_literal: "{" (pair ("," pair)*)? "}"
pair: NAME ":" expression

expression_entry: _NL? expression _NL
```
