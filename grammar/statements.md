# Statements — Grammar

Derived from foundation `specs/language/syntax/statements.md`, with the
type syntax of `specs/language/types/type-system.md`, which has no grammar
chapter of its own. Defines `statement`, the indented `block`, and the
entry `fragment` (a sequence of statements: the examples that are not whole
files). Later chapters add statements with `%extend simple_statement` (one
line) and `%extend compound_statement` (a line that opens a block).

```lark
fragment: _NL? statement+

statement: simple_statement _NL
    | compound_statement

block: _INDENT statement+ _DEDENT

?simple_statement: var_decl
    | assignment
    | expression_statement
    | print_call
    | return_stmt

?compound_statement: print_block
```

## Declarations and assignment

Type first. A variable may be declared without a value. The target of an
assignment is a name, an element (`arr[0]`) or a property (`obj.property`);
which targets are valid is the checker's rule.

```lark
var_decl: type NAME ("=" expression)?
assignment: postfix "=" expression
expression_statement: expression
return_stmt: "return" expression?
```

## Print

Printing always uses parentheses (`print(x)`); several values use the
`print:` block, one expression per line. A bare `print x` does not parse
(`SYN002`), nor does a `print:` block with no expression (`SYN008`).
`printHere(x)` is an ordinary call.

```lark
print_call: "print" "(" expression ")"
print_block: "print" ":" _NL _INDENT (expression _NL)+ _DEDENT
```

## Types

A type is a class or capability name, a type parameter (one letter), a core
type, a sized numeric type (`integer:8`, `integer:32u`, `number:32`), or a
composite `list<T>` (with behavior suffixes `.line`, `.pile`, `.unique`),
`matrix<T>` or `pairs<K, V>`; `T?` is its optional form. The size suffix is
one token, written with no space.

```lark
type: base_type "?"?
?base_type: NAME
    | core_type
    | sized_type
    | list_type
    | matrix_type
    | pairs_type

!core_type: "boolean" | "integer" | "number" | "string" | "bytes" | "datetime" | "void" | "any"
sized_type: ("integer" | "number") SIZE_SUFFIX
SIZE_SUFFIX: /:(8|16|32|64)u?(?![A-Za-z0-9_])/
list_type: "list" "<" type ">" ("." NAME)*
matrix_type: "matrix" "<" type ">"
pairs_type: "pairs" "<" type "," type ">"
```
