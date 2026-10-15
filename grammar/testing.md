# Testing — Grammar

Derived from foundation `specs/language/testing.md`. A `tests:` section
holds one-line tests, named (`"description": expression`) or anonymous
(`expression`), and body tests (`"description":` and a block). `assert` is a
statement. That a one-line test compares with `==` is the checker's rule.

```lark
tests_section: "tests" ":" _NL _INDENT test_item+ _DEDENT
?test_item: named_test
    | anonymous_test
    | body_test
named_test: STRING ":" expression _NL
anonymous_test: expression _NL
body_test: STRING ":" _NL block

%extend simple_statement: assert_stmt
assert_stmt: "assert" expression
```
