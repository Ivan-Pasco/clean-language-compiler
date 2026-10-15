# File Structure — Grammar

Derived from foundation `specs/language/syntax/file-structure.md` and the
`SYN007` rule of `specs/language/semantics.md`. Defines the entry `file`:
the top-level sections, each optional, in their fixed order. A section out
of order does not parse (`SYN007`); loose statements at the top level do not
parse either.

Library blocks and a library's own declarations (`compiletime function`,
`handles block`, `host function`) sit between `functions:` and `screen`.
Each section is defined by the chapter that owns it: `libraries:` and
`import:` in modules, `source:` in ai, `constant:` in apply-blocks,
`state:`, `screen` and `watch` in state, `can` and `class` in classes,
`functions:` and `start:` in functions, `tests:` in testing, library
blocks in block-handlers.

```lark
file: _NL? libraries_section? import_section? source_section? constant_section? state_section? capability_decl* class_decl* functions_section? library_item* screen_decl* watch_decl* tests_section? start_section?

library_item: generic_block
```
