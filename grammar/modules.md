# Modules — Grammar

Derived from foundation `specs/language/modules.md`. The one-line
`libraries:` section names libraries separated by commas; the `import:`
section lists one project file per line by its path from the project root,
`.cln` included. A path is one token.

```lark
libraries_section: "libraries" ":" NAME ("," NAME)* _NL
import_section: "import" ":" _NL _INDENT (IMPORT_PATH _NL)+ _DEDENT
IMPORT_PATH: /[A-Za-z0-9_\-]+(\/[A-Za-z0-9_\-.]+)+/
```
