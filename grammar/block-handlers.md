# Block Handlers — Grammar

Derived from foundation `specs/language/syntax/block-handlers.md` (§1 the
handler declarations, §3 the `BlockAST`). A library's `compiletime
function` and its `handles block` registration; the generic library block
`identifier:` with an optional title and arguments.

The body of a generic block is a sequence of lines, each a list of tokens
(`BlockLine`, §3.2), possibly with an indented body of its own: the compiler
does not parse those lines as statements, and their meaning is the
handler's. Attributes (§3.1) are such lines too. The entry `outer_block`
parses one generic block: the form every `components/` example is checked
for.

```lark
%extend library_item: compiletime_function | handles_decl
compiletime_function: "compiletime" "function" type NAME "(" parameters ")" _NL block
handles_decl: "handles" "block" STRING "with" NAME _NL

generic_block: block_header ":" _NL block_body?
block_header: NAME ("." NAME)* NAME? ("(" arguments? ")")?
block_body: _INDENT block_line+ _DEDENT
block_line: block_token+ _NL block_body?

?block_token: NAME | INT | FLOAT | STRING
    | "(" | ")" | "[" | "]" | "{" | "}" | "," | "." | ":" | "->"
    | "=" | "==" | "!=" | "<" | ">" | "<=" | ">=" | "+" | "-" | "*" | "/" | "%" | "^" | "!" | "?"
    | "after" | "always" | "and" | "any" | "assert" | "background" | "base" | "before" | "break"
    | "can" | "class" | "compiletime" | "constant" | "constructor" | "continue" | "default"
    | "else" | "error" | "fails" | "false" | "function" | "functions" | "handles" | "host"
    | "if" | "import" | "in" | "is" | "isnt" | "iterate" | "later" | "libraries" | "namespace"
    | "none" | "not" | "onError" | "or" | "print" | "public" | "reset" | "return" | "spec"
    | "start" | "state" | "tests" | "to" | "true" | "watch" | "while" | "with"
    | "boolean" | "bytes" | "datetime" | "file" | "http" | "input" | "integer" | "json"
    | "list" | "math" | "matrix" | "number" | "pairs" | "string" | "time" | "void"
    | "for" | "returns" | "this" | "unit"

outer_block: _NL? generic_block
```
