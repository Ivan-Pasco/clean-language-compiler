# Classes — Grammar

Derived from foundation `specs/language/types/classes.md`. Classes with
single inheritance (`is`) and claimed capabilities (`can`), fields with an
optional starting value, the `public:` field sub-section, the constructor,
and methods in a `functions:` section; capability declarations (`can Name:`)
with arrow-return method signatures.

A class body is optional (`class A is B`). A capability method with a body
parses; that a capability holds signatures only is the checker's rule
(`SEM014`).

```lark
class_decl: "class" NAME ("is" NAME)? ("can" NAME ("," NAME)*)? _NL class_body?
class_body: _INDENT class_member+ _DEDENT
?class_member: field_decl _NL
    | public_fields
    | constructor_decl
    | functions_section

field_decl: type NAME ("=" expression)?
public_fields: "public" ":" _NL _INDENT (field_decl _NL)+ _DEDENT
constructor_decl: "constructor" "(" parameters? ")" _NL block

capability_decl: "can" NAME ":" _NL _INDENT capability_method+ _DEDENT
capability_method: NAME "(" parameters? ")" "->" type _NL block?
```
