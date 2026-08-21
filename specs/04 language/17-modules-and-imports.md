# 17. Modules and Imports

A Clean program is one or more `.cln` files. Each file is a module named after its filename, and `import:` brings other modules and their public declarations into scope. This chapter defines the `import:` block, the two forms of it (module-name and direct-file-path), the resolution model (which is a compilation-request property, not a filesystem search), and the `public:` wrapper that controls what a module exports. Libraries are a separate question — they normally come into scope via `clean.toml [folders]`, and `import` only overrides that for explicit disambiguation.

**Clean Language supports multi-file programs through a module system.** Each `.cln` file is a module that can import and use code from other modules.

**Libraries are a separate question from modules.** A library (`data`, `server`, `ui`, …) is normally in scope without any statement in the file: the project's `clean.toml [folders]` maps folders to libraries, and every file under a mapped folder gets them — see [LBS-01](../02%20components/framework/09-libraries-specification.md) and [FRM-01](../02%20components/framework/01-framework-specification.md), which own that model.

`import` still applies to a library in one case: **an explicit import overrides folder scope and wins ties.** Writing `import data.experimental` selects that library's block handlers over whatever the manifest would have brought in, which is how two libraries claiming the same block name are disambiguated ([21 §21.2](./21-block-handlers.md#212-block-name-resolution)).

So `import` covers both: other `.cln` files, which the rest of this chapter describes, and the explicit-library override above. There is no per-file `libraries:` block; the manifest is the only source of implicit scope.

### Module Definition

Every `.cln` file is implicitly a module. The module name is derived from the filename (without the `.cln` extension).

```clean
// file: utils.cln
// This file defines the "utils" module

functions:
	public:
		integer add(integer a, integer b)
			return a + b

		integer multiply(integer a, integer b)
			return a * b
```

The `public:` wrapper is what makes `add` and `multiply` reachable from an importing module — without it they are module-local ([MOD-02](#mod-02--module-visibility)).

### MOD-01 — Importing modules and libraries

Use the `import:` block to import other modules. All public functions and classes from the imported module become available.

```clean
// file: main.cln
import:
	utils

start:
	// Use functions from utils module
	integer sum = add(5, 3)
	integer product = multiply(4, 2)
	print(sum)
	print(product)
```

#### Import Block Syntax

The import block uses indentation to list imported modules:

```clean
import:
	utils           // Import the utils module
	mathHelpers     // Import the mathHelpers module
	data.models     // Import from nested path (data/models.cln)
```

#### Import Variations

```clean
import:
	math                // whole module
	math.sqrt           // single symbol
	utils as u          // module alias
	json.decode as jd   // symbol alias
```

#### File Path Imports

In addition to module-name imports, Clean Language supports **direct file path imports** using string literals. This is useful for importing files from specific locations without relying on module resolution.

```clean
// Import a file using its relative path
import "app/data/models.cln"
import "../lib/utils.cln"
import "./helpers.cln"
```

**Key differences from module imports:**

| Feature | Module Import | File Path Import |
|---------|---------------|------------------|
| Syntax | `import: module_name` | `import "path/to/file.cln"` |
| Resolution | By module name, within the compilation request | By path, relative to the importing file |
| Nested paths | `data.models` → `data/models.cln` | `"data/models.cln"` (explicit) |

**Path Resolution:**

File path imports are resolved **relative to the directory of the importing file**, not the project root:

```
project/
├── main.cln              # import "app/data/models.cln"
├── app/
│   ├── data/
│   │   └── models.cln    # import "../../lib/utils.cln"
│   └── services/
│       └── api.cln
└── lib/
    └── utils.cln
```

```clean
// file: main.cln
import "app/data/models.cln"  // Resolves to ./app/data/models.cln

start:
	integer result = double(21)
	print(result)
```

```clean
// file: app/data/models.cln
import "../../lib/utils.cln"  // Resolves to ./lib/utils.cln (relative to app/data/)

functions:
	public:
		integer double(integer x)
			return x * 2

		integer squareDouble(integer x)
			integer doubled = double(x)
			return square(doubled)  // From utils.cln — public there
```

**Chained Imports:**

File path imports can be chained - imported files can import other files:

```clean
// main.cln imports models.cln which imports utils.cln
// All functions from all three files are available in the final WASM
```

**When to use each import style:**

- **Module imports** (`import: utils`) name a module; the framework resolves the name to a file.
- **File path imports** (`import "path/file.cln"`) name the file directly, relative to the importing file.

### MOD-03 — Import resolution happens before the compiler runs

A Clean program is compiled from a **compilation request** — a self-contained document carrying every source file inline, already resolved ([Glossary](../01%20governance/06-glossary.md); [Platform 14 §14.1.1](../03%20platform/14-compiler-architecture.md#1411-inputs)). The compiler performs no filesystem discovery: it does not search directories, does not try filename patterns, and does not read `clean.toml`.

Resolving an import to a file is therefore the framework's work, done before the compiler is invoked. What the compiler does with the result is fixed:

- It follows `import` statements **across the `sources[]` set of the request**, and nowhere else.
- An import naming a module absent from that set is [`IMPORT002`](../03%20platform/09-error-codes.md#38-import-codes-import).
- A cycle among imports is [`IMPORT001`](../03%20platform/09-error-codes.md#38-import-codes-import), reported once per cycle; resolution continues so that one build reports every cycle it finds.

Because resolution is complete before compilation starts, two builds from the same request produce byte-identical output ([C-10](../01%20governance/05-concerns.md)).

**Building** is a single command, `cln build`, owned by [Clean Manager](../02%20components/manager/00-manager.md). It takes no source-file argument and no search-path flags: the project is what `clean.toml` describes.

```bash
cln build
```

#### Circular Dependencies

A cycle between modules is [`IMPORT001`](../03%20platform/09-error-codes.md#38-import-codes-import), reported once per cycle:

```clean
// file: a.cln
import:
	b  // a imports b

// file: b.cln
import:
	a  // b imports a — IMPORT001
```

### Built-in Modules

The standard library is built into the language and needs no import. Its modules — `console`, `math`, `string`, `list`, `file`, `http`, `json` and `validator` — are catalogued in [15 — Standard Library](./15-standard-library.md), which is their single home; this chapter does not restate the list.

Built-in modules are automatically available when imported:

```clean
import:
	math
	string
	list

start:
	number pi = math.pi
	string upper = "hello".toUpperCase()
	list<integer> nums = list.range(1, 10)
```

### MOD-02 — Module visibility

**Private by default** — functions and classes are module-local unless declared inside a `public:` block. There is no `private` keyword; the absence of `public:` is what makes a name private.

```clean
functions:
	// Private by default
	internalHelper()
		// implementation

	// Exported
	public:
		calculateTotal()
			// implementation

		formatCurrency()
			// implementation
```

The `public:` wrapper appears *inside* the section whose declarations it exports — it is not a top-level section itself ([grammar §5](./grammar/17-modules-and-imports.ebnf.md)).

A name that is not inside a `public:` block is not visible to another module; reaching for it is [`IMPORT003`](../03%20platform/09-error-codes.md#38-import-codes-import):

```clean
// file: mymodule.cln
functions:
	integer helperFunc()
		return 42

	public:
		integer publicFunc()
			return helperFunc() * 2

// file: main.cln
import:
	mymodule

start:
	integer x = publicFunc()   // OK
	integer y = helperFunc()   // ERROR: helperFunc is private
```

### Example: Multi-File Project

Here's a complete example of a multi-file Clean Language project:

```clean
// file: utils.cln
functions:
	public:
		integer add(integer a, integer b)
			return a + b

		integer multiply(integer a, integer b)
			return a * b

		integer doubleValue(integer n)
			return n * 2
```

```clean
// file: mathHelpers.cln
import:
	utils

functions:
	public:
		integer square(integer n)
			return multiply(n, n)

		integer quadruple(integer n)
			return doubleValue(doubleValue(n))
```

```clean
// file: main.cln
import:
	utils
	mathHelpers

start:
	// Use functions from utils
	integer sum = add(10, 5)
	print(sum)  // Output: 15

	// Use functions from mathHelpers
	integer sq = square(4)
	print(sq)  // Output: 16

	// Combined usage
	integer result = multiply(sq, 2)
	print(result)  // Output: 32
```

Build with:
```bash
cln build
```

## Changelog

- 2026-08-17 — Erratum: every multi-file example (§Module Definition, §File Path Imports, §Example: Multi-File Project) imported a module and called functions that were not inside a `public:` wrapper — under [MOD-02](#mod-02--module-visibility) ("private by default") every one of those calls is [`IMPORT003`](../03%20platform/09-error-codes.md#38-import-codes-import). The examples now export what they call. In the same pass, MOD-02's own examples showed a **top-level** `public:` wrapping a `functions:` block, a position the grammar forbids — `PublicWrapper` appears *inside* a section ([grammar §5](./grammar/17-modules-and-imports.ebnf.md)); per [DOC-15](../01%20governance/00-documentation-principles.md) the parser follows the grammar, and Platform 10's SEM005 examples already use the nested form. All examples normalized to `functions:` → `public:`. (`double_value` also renamed `doubleValue` — snake_case had survived the 2026-08-01 identifier pass.) How a *class* is exported remains open — see `work/2026-08-17-class-export-surface.md`. Found by the compiler's Milestone 4 (`clean-language-compiler/docs/DISCOVERIES-M4.md`, items 6 and 7).
- 2026-08-01 — Fase 5 (zero-debt pass): import cycles cite [`IMPORT001`](../03%20platform/09-error-codes.md#38-import-codes-import) and cross-module access to a non-public name cites [`IMPORT003`](../03%20platform/09-error-codes.md#38-import-codes-import).
- 2026-08-01 — Fase 3/4 (L4): **"libraries are never imported" corrected.** Folder scope is the only source of implicit scope, and an explicit `import` overrides it and wins ties — the rule [21 §21.2](./21-block-handlers.md#212-block-name-resolution) depends on for handler disambiguation. The compiler's filesystem search (`./`, `./lib/`, `./modules/`, `./src/`, `-L`) removed: it contradicts [C-08](../01%20governance/05-concerns.md) and the compilation-request model, and resolution happens before the compiler runs ([MOD-03](#mod-03--import-resolution-happens-before-the-compiler-runs)). `cln build main.cln -o … -L … -O3` replaced by `cln build`, the Manager's sole build command. The built-in module list replaced by a citation of [15](./15-standard-library.md). Uppercase and snake_case identifiers in examples corrected. Rules `MOD-01`..`MOD-03` minted.

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Clean Language users writing multi-file programs; anyone using `cln build`
- **Rule prefix:** `MOD-`
- **Part of:** [Clean Language Specification — Language](./README.md)
- **References:** [File Structure](./08-file-structure.md), [Standard Library](./15-standard-library.md) (built-in modules), [Libraries Specification](../02%20components/framework/09-libraries-specification.md) (folder scope), [Platform 09 — Error Codes](../03%20platform/09-error-codes.md), [Clean Manager](../02%20components/manager/00-manager.md) (`cln build`)
