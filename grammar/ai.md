# AI Integration — Grammar

Derived from foundation `specs/language/ai.md` and the `SYN100` / `SYN102`
rules of `specs/language/semantics.md`. The `spec` header line takes a
string literal (`SYN100` without one) holding a path relative to the
project root: a path that starts with `/` does not lex as `SPEC_PATH`
(`SYN102`). The `source:` section holds the `spec:` and `version:` fields;
`version` is a contextual keyword.

```lark
%extend simple_statement: spec_line
spec_line: "spec" SPEC_PATH
SPEC_PATH: /"(?!\/)(\\.|[^"\\\n])*"/

source_section: "source" ":" _NL _INDENT source_field+ _DEDENT
source_field: "spec" ":" STRING _NL
    | "version" ":" STRING _NL
```
