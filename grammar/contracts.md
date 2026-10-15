# Contracts — Grammar

Derived from foundation `specs/language/types/contracts.md`. `before:` and
`after:` are blocks of one expression per line inside a function body;
`always` is a line of a class body. Where they may appear and in which
order, and that each line is a boolean, are the checker's rules
(`CLASS005`, `CLASS006`).

```lark
%extend compound_statement: before_block | after_block
before_block: "before" ":" _NL _INDENT (expression _NL)+ _DEDENT
after_block: "after" ":" _NL _INDENT (expression _NL)+ _DEDENT

%extend class_member: always_line _NL
always_line: "always" expression
```
