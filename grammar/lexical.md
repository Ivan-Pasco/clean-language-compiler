# Lexical Structure — Grammar

Derived from foundation `specs/language/syntax/lexical.md`. Defines the
terminals every later chapter uses, the line and indentation events, and
the entry `identifier_entry` for the lists of valid and invalid names.

## Layout: lines, tabs, comments

Tabs only; one tab per level. `_NL` is one line end together with the blank
lines and comment-only lines after it, so a comment never breaks a block.
The indenter of `tools/grammar/check.py` turns the indentation `_NL` ends
with into `_INDENT` / `_DEDENT`. A space in that indentation, or a rise of
more than one tab, is `SYN003`. Inside `()`, `[]` and `{}` no `_NL`,
`_INDENT` or `_DEDENT` is emitted (Expressions — Multi-Line Expressions).

```lark
_NL: /((\/\/[^\n]*)?\r?\n[\t ]*(\/\*[\s\S]*?\*\/[\t ]*)?)+/
%declare _INDENT _DEDENT

LINE_COMMENT: /\/\/[^\n]*/
BLOCK_COMMENT: /\/\*[\s\S]*?\*\//
INLINE_SPACE: /[ \t]+/
%ignore INLINE_SPACE
%ignore LINE_COMMENT
%ignore BLOCK_COMMENT
```

## Identifiers and keywords

A name starts with a letter and holds letters, digits and underscores. A
reserved keyword, a reserved name (core type and built-in module names) and
a reserved-but-unused word is never a name (`SYN009`): `NAME` excludes them.
Contextual keywords (`block`, `computed`, `description`, `interface`,
`requires`, `screen`, `source`, `step`, `version`, `worlds`) are names
everywhere except the one position whose production spells them.

```lark
NAME: /(?!(after|always|and|any|assert|background|base|before|break|can|class|compiletime|constant|constructor|continue|default|else|error|fails|false|function|functions|handles|host|if|import|in|is|isnt|iterate|later|libraries|namespace|none|not|onError|or|print|public|reset|return|spec|start|state|tests|to|true|watch|while|with|boolean|bytes|datetime|file|http|input|integer|json|list|math|matrix|number|pairs|string|time|void|for|returns|this|unit)(?![A-Za-z0-9_]))[A-Za-z][A-Za-z0-9_]*/

identifier_entry: NAME _NL
```

## Literals

Integers in decimal, hexadecimal, binary and octal; floating-point with a
fraction, a leading dot or an exponent. A sign is the unary minus of
Expressions, not part of the literal. Strings are double-quoted, on one
line, with backslash escapes; `{name}` and `{obj.prop}` interpolations are
part of the string token.

```lark
FLOAT.2: /(\d+\.\d+|\.\d+)([eE][+-]?\d+)?|\d+[eE][+-]?\d+/
INT: /0x[0-9a-fA-F]+|0b[01]+|0o[0-7]+|\d+/
STRING: /"(\\.|[^"\\\n])*"/
```
