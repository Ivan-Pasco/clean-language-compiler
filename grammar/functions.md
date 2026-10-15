# Functions — Grammar

Derived from foundation `specs/language/functions.md`. The `functions:`
section with its `public:` sub-section, type-first function declarations
with default parameter values and the `background` mark, the `start:`
section, the `description` header line, and the keyword-prefixed `host
function` form.

A function body is optional: a declaration whose body a library supplies
has none (`User? findById(integer userId)`). The header lines
(`description`, `spec`, `before:`, `after:`) are statements; their order is
the checker's rule (`CLASS005`). `start` with parameters parses; that it
takes none is the checker's rule (`FUNC006`).

```lark
functions_section: "functions" ":" _NL _INDENT function_item+ _DEDENT
?function_item: function_decl
    | public_functions
public_functions: "public" ":" _NL _INDENT function_decl+ _DEDENT

function_decl: type NAME "(" parameters? ")" "background"? _NL block?
parameters: parameter ("," parameter)*
parameter: type NAME ("=" expression)?

start_section: "start" ("(" parameters? ")")? ":" _NL block

%extend simple_statement: description_line
description_line: "description" STRING
```

## Keyword-prefixed forms

`host function` declares a function the host provides: no Clean body, only
header lines. Its placement (only in a library's `host_bridge.cln`) is the
checker's rule (`LIB019`). `compiletime function` is in block-handlers.

```lark
%extend library_item: host_function
host_function: "host" "function" type NAME "(" parameters? ")" ("fails" "with" NAME)? _NL block?
```
