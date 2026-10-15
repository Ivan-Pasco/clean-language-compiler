# State — Grammar

Derived from foundation `specs/language/state.md`. The `state:` section with
its `computed:` sub-section, `screen Name:` sections with their own state,
functions and watchers, `watch` sections on one or several state variables,
and the `reset` statement. `computed` and `screen` are contextual keywords.
That a state variable has an initial value is the checker's rule.

```lark
state_section: "state" ":" _NL _INDENT state_item+ _DEDENT
?state_item: var_decl _NL
    | computed_section
computed_section: "computed" ":" _NL _INDENT computed_item+ _DEDENT
computed_item: type NAME _NL block

screen_decl: "screen" NAME ":" _NL _INDENT screen_item+ _DEDENT
?screen_item: state_section
    | functions_section
    | watch_decl

watch_decl: "watch" watch_target ":" _NL block
watch_target: NAME
    | "(" NAME ("," NAME)* ")"

%extend simple_statement: reset_stmt
reset_stmt: "reset" NAME
    | "reset" "state"
```
