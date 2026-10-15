# Apply-Blocks — Grammar

Derived from foundation `specs/language/syntax/apply-blocks.md`. A call
target followed by `:` applies it to each indented expression
(`items.add:`); a core type followed by `:` declares one variable of that
type per indented line (`integer:`). `print:` is its own construct
(Statements). The `constant:` section holds one typed declaration per line.

```lark
%extend compound_statement: apply_block | typed_apply_block
apply_block: postfix ":" _NL _INDENT (expression _NL)+ _DEDENT
typed_apply_block: core_type ":" _NL _INDENT (NAME "=" expression _NL)+ _DEDENT

constant_section: "constant" ":" _NL _INDENT (var_decl _NL)+ _DEDENT
```
