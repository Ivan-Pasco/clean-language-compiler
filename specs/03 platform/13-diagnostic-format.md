# Platform 13. Diagnostic Format

Diagnostics are the primary UX of the compiler. This chapter defines the **shape** of a diagnostic — what fields it carries, how it renders in the CLI, how it is serialized for tools, how it links to `cln explain`, and the style rules every message must follow. It does not enumerate individual codes ([09 — Error Codes](./09-error-codes.md) does) or the rule bodies ([10 — Semantic Rules](./10-semantic-rules.md) does); its remit is anatomy, wire format, and style. A well-formed diagnostic tells the developer three things in one glance: where the problem is, what the compiler thinks it means, and what to do next.

---

## 1. Design Goals


Diagnostics are UX. A good compiler error tells the developer three things in one glance: **where the problem is, what the compiler thinks it means, and what to do next.** The format is designed so that:

- A developer reading a hover tooltip understands the failure without opening the terminal.
- A developer reading the terminal sees enough context to fix the failure without opening the editor.
- An IDE Quick Fix can apply the suggested edit without asking the user.
- An LLM agent can consume the JSON output, apply high-confidence suggestions, and defer low-confidence ones to a human turn.
- A blog post or Stack Overflow answer can link directly to the docs for a single code.

Every field below exists to serve one of those five readers.

---

## 2. The Diagnostic Value


Every diagnostic emitted by the compiler is a value with this shape:

```
Diagnostic {
    level          : Level              // error | warning | info | help
    code           : Code               // e.g. SEM001, LIB011 — see 09-error-codes.md
    message        : string             // one-line headline; see §4
    primary_span   : Span               // where the problem is
    primary_label  : string?            // optional in-line label at the caret; see §4.3
    secondary      : list<Annotation>   // additional labelled spans; see §4.4
    notes          : list<string>       // "note:" lines — context; see §4.5
    helps          : list<string>       // "help:" lines — actionable guidance
    suggestions    : list<Suggestion>   // structured edits; see §5
    doc_url        : string             // https://errors.cleanlanguage.dev/E/<CODE>
}
```

`Span` and `Annotation`:

```
Span {
    file  : string       // relative path from the project root, forward-slashed
    start : Position     // 1-based line, 1-based column, both counted in characters
    end   : Position     // exclusive end; end.line ≥ start.line
}

Annotation {
    span  : Span
    label : string       // short, ≤ 80 chars, describes what this span contributes
}
```

There is exactly one `primary_span`. There may be zero or more `secondary` annotations. All spans must resolve to real characters in a file the compiler has read; a span pointing into generated code from a `compiletime function` uses the source-language span the library attached via the diagnostic API (never the generated-code position).

### DIA-01 — Every diagnostic carries a registry code


Every diagnostic the compiler or language server emits MUST carry a `code` that exists in the [09 — Error Codes](./09-error-codes.md) registry, in `PREFIX###` form (09 §1). A diagnostic without a registered code — a stringly-typed message — MUST NOT be emitted. Check: for every emitted diagnostic, `code` resolves to a row in 09 §3.

---

## 3. Levels


| Level | Semantic | When to use |
|-------|----------|-------------|
| `error` | Compilation fails. | The code cannot be compiled or the runtime rule violated invariants that must hold. |
| `warning` | Compilation succeeds. Behavior may be unintended. | Dead code, shadowed variables, unused imports, non-void functions with a missing return on some paths. |
| `info` | Compilation succeeds. Purely advisory. | Style suggestions, spec-companion links, deprecation notices. |
| `help` | Not a top-level diagnostic. | Attached to another diagnostic to explain how to fix it. Never appears in isolation. |

A build is considered failed if any diagnostic has `level = error`.

---

## 4. Message Anatomy


### 4.1 Headline

The `message` field is a **single line, ≤ 100 characters, no trailing punctuation**. It must stand alone in a 40-column hover tooltip.

### DIA-02 — The headline is mandatory and bounded


Every diagnostic MUST have a `message` headline that is a single line of at most 100 characters with no trailing punctuation, obeying the style rules of §9. The headline is never optional (the `primary_label` is — see §4.3). Check: the §12 snapshot tests assert the exact rendered headline for every code; a headline over 100 characters, multi-line, or ending in punctuation fails review against §9.

- Start with the phenomenon, not the cause. `"type mismatch"` beats `"expected integer"`.
- Use plain nouns and verbs. `"variable name"` beats `"identifier"`; `"function has no return"` beats `"missing return in non-void function body"`.
- First-person is allowed and encouraged when it improves clarity: `"I cannot find a variable named `count` in scope"`.
- Never say "illegal." Use `"invalid"`.
- Never say "you must." State the rule in the indicative: `"before must appear before other statements"`.
- Quote identifiers in backticks: `` `foo` ``. Quote user-authored strings with real curly quotes: `"the value “hello” is not a valid integer"`.
- Never surface compiler-internal terms in the message: no `HIR`, `MIR`, `resolver`, `type checker`, `codegen`, `IR node`, `AST`. The category taxonomy in [`09-error-codes.md`](./09-error-codes.md) is for specification readers only; user-facing messages describe the source, not the compiler.

### 4.2 Line format (CLI)

The rendered diagnostic block:

```
<level>[<CODE>]: <message>
  --> <file>:<line>:<column>
   |
LN | <source line 1>
   | <caret run>  <primary_label>
LN | <source line 2>
   | <caret run>  <secondary label>
   |
   = note: <context>
   = help: <guidance>
   = suggestion: <replacement snippet>
   = docs: https://errors.cleanlanguage.dev/E/<CODE>
```

**Example:**

```
error[SEM001]: type mismatch in assignment
  --> app/orders/checkout.cln:42:5
   |
42 |     integer total = subtotal + shipping
   |     ^^^^^^^         ------------------- this expression has type `number`
   |     |
   |     `total` is declared with type `integer`
   |
   = help: either declare `total` as `number`, or convert with `subtotal.toInteger()`
   = docs: https://errors.cleanlanguage.dev/E/SEM001
```

The renderer preserves 2-space left indent, uses ASCII box-drawing (`|`, `-->`, `^`, `=`) so it renders in any terminal, and emphasizes the primary span with `^` while marking secondary spans with `-`.

### 4.3 Primary label

The `primary_label` is a short phrase (≤ 80 characters) that appears beneath the caret. It answers "what is wrong at this exact spot?" and is complementary to the headline — the headline names the phenomenon, the label localizes it.

- Headline: `"type mismatch in assignment"`
- Label: `` "`total` is declared with type `integer`" ``

The label is optional; the headline is not (DIA-02). *Informative guidance:* omit the label only when the headline already localizes the problem and no additional information helps — a judgment made visible by the §12 snapshot review, not a mechanically checkable condition.

### 4.4 Secondary annotations

Secondary annotations point to related code that clarifies *why* the primary span is wrong. Common uses:

- The declaration site of a symbol referenced at the primary span.
- The other branch of a type-inference mismatch (`"expected because of this bound"`).
- A prior `before` or `after` clause whose position makes the current one invalid.
- The `state:` declaration whose guard rejected an assignment.

A diagnostic may carry any number of secondary annotations but should carry as few as possible. If the same information can be conveyed with a `note:`, prefer the note — extra spans compete for the reader's attention.

### 4.5 Notes and helps

- `note:` explains context that isn't itself a fix. "This function is declared `private` at `foo.cln:12`." "Contracts run at every return, including implicit ones."
- `help:` proposes a concrete change in prose. "Change the declared type to `number`." "Move the `after` clause above the assignment."

Each is a short paragraph (≤ 200 characters). Multiple notes and multiple helps are allowed and are rendered in the order provided. `note:` always precedes `help:` in the render.

---

## 5. Structured Suggestions


A `Suggestion` is a machine-applicable edit. It exists so that IDEs, `cln fix`, and AI agents can apply a fix without a round-trip to the user.

```
Suggestion {
    message       : string          // short — appears next to the code action
    replacements  : list<Replacement>
    applicability : Applicability
}

Replacement {
    span         : Span              // exact range to replace
    replacement  : string            // new text
}

Applicability = MachineApplicable
              | HasPlaceholders
              | MaybeIncorrect
              | Unspecified
```

### 5.1 Applicability levels

The applicability tag is the single most important field in the diagnostic value. It gates whether tools may apply the suggestion automatically.

| Level | Meaning | Who may apply automatically |
|-------|---------|------------------------------|
| `MachineApplicable` | The suggestion is correct and complete. Applying it fixes the diagnostic without introducing new ones. | `cln fix`, IDE Quick Fix on save, LLM agents in autonomous mode. |
| `HasPlaceholders` | The suggestion contains one or more placeholders (`__VALUE__`) that a human must fill in. | IDE snippet insertion (with cursor at the first placeholder). Never fully automatic. |
| `MaybeIncorrect` | The suggestion is one of several plausible fixes and may not be the one the user intended. | IDE Quick Fix menu, prompted "did you mean?" — never applied without confirmation. |
| `Unspecified` | Applicability has not been assessed. Treat as `MaybeIncorrect`. | Never automatic. |

A single diagnostic may carry multiple suggestions. If it does, they represent *alternative* fixes, not a sequence — the user (or tool) picks one.

### DIA-03 — Every suggestion carries an applicability gate


Every `Suggestion` MUST carry an `applicability` value from the §5.1 table. Tools (`cln fix`, IDE Quick Fix on save, autonomous agents) MUST NOT apply any suggestion automatically unless its applicability is `MachineApplicable`; `Unspecified` MUST be treated as `MaybeIncorrect`. Check: given a diagnostic whose only suggestion is `MaybeIncorrect`, `cln fix` leaves the source unchanged.

### 5.2 Splitting rule for applicability

If a compile-time function generating a suggestion cannot decide between `MachineApplicable` and `MaybeIncorrect`, it must emit two suggestions: the safest one as `MachineApplicable` and the alternatives as `MaybeIncorrect`. Never demote the safe suggestion to `MaybeIncorrect` just because a less-safe alternative exists.

### 5.3 Placeholder syntax

Placeholders in `HasPlaceholders` suggestions use double-underscore delimiters: `__NAME__`. The compiler chooses names that describe what the human should fill in (`__CONDITION__`, `__RETURN_VALUE__`, `__PATH__`). Editors treat every `__X__` occurrence with the same name as one tab-stop group.

### 5.4 Example: `SEM002 UndefinedVariable` with ranked suggestions

```
error[SEM002]: I cannot find a variable named `lenght` in scope
  --> app/reports/summary.cln:18:17
   |
18 |     integer n = lenght(users)
   |                 ^^^^^^ no variable with this name exists here
   |
   = help: closest known names are `length`, `lengthOf`

Suggestions (alternatives — pick one):
  1. [MaybeIncorrect] Replace `lenght` with `length`
       app/reports/summary.cln:18:17..18:23  →  `length`
  2. [MaybeIncorrect] Replace `lenght` with `lengthOf`
       app/reports/summary.cln:18:17..18:23  →  `lengthOf`
```

Neither is `MachineApplicable` because the compiler cannot verify intent from a name alone. An IDE renders these as a chooser; an agent asks the user.

---

## 6. JSON Output (NDJSON)


The compiler emits one JSON object per line when invoked with `--diagnostic-format=json`. The stream is newline-delimited so tools can parse it incrementally without buffering the whole compilation.

### 6.1 Schema

```json
{
  "level": "error",
  "code": "SEM001",
  "message": "type mismatch in assignment",
  "primary_span": {
    "file": "app/orders/checkout.cln",
    "start": { "line": 42, "column": 5 },
    "end":   { "line": 42, "column": 12 }
  },
  "primary_label": "`total` is declared with type `integer`",
  "secondary": [
    {
      "span": {
        "file": "app/orders/checkout.cln",
        "start": { "line": 42, "column": 21 },
        "end":   { "line": 42, "column": 40 }
      },
      "label": "this expression has type `number`"
    }
  ],
  "notes": [],
  "helps": [
    "either declare `total` as `number`, or convert with `subtotal.toInteger()`"
  ],
  "suggestions": [
    {
      "message": "Change the declared type to `number`",
      "replacements": [
        {
          "span": {
            "file": "app/orders/checkout.cln",
            "start": { "line": 42, "column": 5 },
            "end":   { "line": 42, "column": 12 }
          },
          "replacement": "number"
        }
      ],
      "applicability": "MachineApplicable"
    }
  ],
  "doc_url": "https://errors.cleanlanguage.dev/E/SEM001",
  "rendered": "error[SEM001]: type mismatch in assignment\n  --> app/orders/checkout.cln:42:5\n   |\n42 |     integer total = subtotal + shipping\n   |     ^^^^^^^         ------------------- this expression has type `number`\n..."
}
```

### 6.2 Stability

The JSON schema is versioned via the compiler's minor version.

### DIA-04 — The JSON schema is stable for tools


The NDJSON schema of §6.1 evolves under the compiler's semver: adding an optional field MUST be at most a minor bump; removing a field or narrowing a type MUST be a major bump. Tools that consume the JSON MUST ignore unknown fields. Check: a consumer written against version N parses version N+0.1 output without error.

### 6.3 The `rendered` field

Every JSON diagnostic carries a `rendered` field containing the exact CLI text the compiler would print. This lets tools display the pretty form without re-implementing the renderer. Editors that render their own diagnostics ignore `rendered`; agents that pipe compiler output to a chat surface use it verbatim.

---

## 7. LSP Diagnostic Mapping


The language server maps every `Diagnostic` value into an LSP `PublishDiagnosticsParams` entry:

| Compiler field | LSP field |
|----------------|-----------|
| `level` | `severity` (`error → 1`, `warning → 2`, `info → 3`, `help → 4`) |
| `code` | `code` (string, e.g. `"SEM001"`) |
| `message` + `primary_label` | `message` (joined with a newline if a label is present) |
| `primary_span.file` + `primary_span.start`/`end` | `range` (LSP is 0-based; the compiler converts on emission) |
| `doc_url` | `codeDescription.href` |
| `secondary` | `relatedInformation` (each entry becomes a `DiagnosticRelatedInformation`) |
| `suggestions` | Not on `Diagnostic`. Delivered via `textDocument/codeAction` when the editor requests actions at the diagnostic's range. |
| `notes`, `helps` | Appended to `message` when the editor does not fetch code actions. |

The server also populates LSP's `data` field with the untouched compiler `Diagnostic` so a subsequent `codeAction` request can retrieve the structured suggestions without re-running analysis. The full round-trip contract — including the editor-side preservation requirement — is specified in [`04-ide-lsp-architecture.md`](./04-ide-lsp-architecture.md) §4.1.1.

---

## 8. `cln explain <CODE>`


Every diagnostic code has a stable, browsable long-form explanation.

### DIA-05 — Every code is explainable, offline and by URL


For every code in the [09](./09-error-codes.md) registry, `cln explain <CODE>` MUST print the §8.1 content from material embedded in the compiler binary (no network), and the canonical URL `https://errors.cleanlanguage.dev/E/<CODE>` MUST render the four §8.3 sections. URLs are permanent: a deprecated code stays reachable and points to its successor. Check: `cln explain` succeeds with networking disabled for every registered code; no registered code yields a 404 at its canonical URL.

### 8.1 CLI

```
$ cln explain SEM001
```

Prints:

- The diagnostic name (`type mismatch`).
- One paragraph explaining the rule in plain language.
- A minimal failing example.
- A minimal fixed example.
- A pointer to the semantic rule in [`10-semantic-rules.md`](./10-semantic-rules.md).
- The doc URL.

The content is embedded in the compiler binary — `cln explain` works offline and requires no network.

### 8.2 URL scheme

Every code has a canonical URL: `https://errors.cleanlanguage.dev/E/<CODE>`, where `<CODE>` is a registry code in `PREFIX###` form ([09 §1](./09-error-codes.md)). Examples:
- `https://errors.cleanlanguage.dev/E/SEM001`
- `https://errors.cleanlanguage.dev/E/LIB011`
- `https://errors.cleanlanguage.dev/E/SYN003`

The path is case-insensitive; the canonical form is uppercase. URLs are permanent — a code that is deprecated stays reachable and points to its successor.

### 8.3 Content contract

Each code URL renders four sections:

1. **Summary** — the same one-paragraph explanation shown by `cln explain`.
2. **Failing example** — minimum reproducer, syntax-highlighted.
3. **Fixed example** — the same code with the minimum change to compile.
4. **See also** — cross-links to related codes, the semantic rule, and any spec sections referenced by the rule.

The doc site is generated from the compiler's built-in explanation content, so the CLI and the web are always in sync.

---

## 9. Style Guide


Every message, note, help, and suggestion label the compiler emits must obey these rules. New codes must be reviewed against them.

1. **Plain language, no jargon.** No `HIR`, `MIR`, `resolver`, `type checker`, `IR node`, `AST`, `parser state`, `unification`.
2. **Identifier quoting.** User-authored identifiers go in backticks. User-authored string values go in curly quotes. Types go in backticks.
3. **First person is allowed** when it improves clarity: `"I cannot find a variable named X"`. Not required, but never editorial ("you must").
4. **Verbs are indicative, not imperative in the headline.** Save imperative for `help:` lines. `"before must appear before other statements"` — not `"put before at the top"`.
5. **No `illegal`, no `forbidden`, no `bad`.** Use `invalid`, `not allowed`, `does not apply`.
6. **No trailing punctuation on headlines.** Full sentences with periods are fine inside `note:` and `help:` blocks.
7. **Never blame the developer.** `"I cannot find X"` beats `"you didn't define X"`.
8. **Never surface implementation details as the phenomenon.** `"type mismatch"` beats `"unification failed"`.
9. **Numbers and names in prose stay unformatted.** Rule codes appear in square brackets; source identifiers appear in backticks. `"rule SEM001 rejects assigning `string` to `integer`"`.
10. **Every message is testable.** The rule that produces a diagnostic must have a snapshot test that asserts the exact rendered text. Message regressions are breakage.

---

## 10. Multi-Error Collection


The compiler does not stop at the first error. Every phase (parse, name resolution, type and capability checking, codegen) collects diagnostics into a shared sink and continues past recoverable failures. (The sink is an implementation detail; only the collection behavior described here is contract.)

### 10.1 What "recoverable" means per phase

| Phase | Recoverable | Not recoverable (halts the phase) |
|-------|-------------|-----------------------------------|
| Lex | Invalid character in the middle of a file. Continue with `SYN001` and resume at the next whitespace. | Unterminated string extending past EOF (`SYN004`). |
| Parse | A statement at an indentation level that closes no open block (e.g. a body line dedented past its section), a misplaced statement, wrong section order. Recover to the next top-level keyword. | Syntax error in the first three lines that prevents identifying the file as Clean source. |
| Name resolution | Undefined symbol, redefinition. Bind to an `error` sentinel and continue type-checking with degraded types. | Missing entry point (`start:` block or `main.cln` cannot be found). |
| Type check | Mismatch, invalid operation, wrong argument count. Bind the offending expression to type `error` and continue. | Cyclic type definitions that would trigger infinite recursion. |
| Codegen | An individual function fails to lower. Skip it, emit `COM001`, continue. | The module cannot be assembled at all (linker error, missing entry). |

### 10.2 Duplicate suppression

Diagnostics deduplicate by `(code, primary_span, message)`. A single failure that surfaces at ten call sites reports once at the definition and lists the ten call sites as secondary annotations, not ten separate diagnostics.

### 10.3 Rendering order

CLI output is sorted by file path, then by line, then by column. JSON output preserves emission order (the order the compiler discovered the diagnostics) so tools can reason about causality.

### 10.4 Fatal-error behavior

When a diagnostic marked fatal per §10.1 fires, the compiler:
1. Emits the diagnostic.
2. Emits one final `info` diagnostic explaining that further checking was skipped.
3. Exits with a non-zero code.

The fatal path is rare — most diagnostics are recoverable — so users almost always see the full set of failures in one run.

---

## 11. Splitting Rule


A diagnostic code must be split into narrower codes when any of these are true:

- `cln explain <CODE>` cannot give focused advice because the code covers multiple root causes.
- Two situations that trigger the same code need different suggestions with different applicability levels.
- The `doc_url` page needs more than one "failing example / fixed example" pair to cover the code.

This rule is retroactive. A code that grows to cover distinct situations must be split rather than left in place, following the reservation protocol in [`09-error-codes.md`](./09-error-codes.md) §5. The pre-split code is deprecated (not removed), continues to resolve to its `doc_url`, and points readers to the successor codes.

---

## 12. Testing Diagnostics


Diagnostics are first-class compiler output.

### DIA-06 — Every diagnostic is snapshot-tested


Every rule in [`10-semantic-rules.md`](./10-semantic-rules.md) MUST have three test artifacts:

1. **A minimal failing `.cln` file** in `tests/cln/diagnostics/<code>.cln`.
2. **A snapshot of the CLI output** in `tests/cln/diagnostics/<code>.stdout.txt`.
3. **A snapshot of the JSON output** in `tests/cln/diagnostics/<code>.json`.

Both snapshots are byte-exact. A change in message wording, span position, or applicability level fails CI and requires updating the snapshot in the same commit as the rule change. This turns diagnostic quality into a version-controlled artifact, not an implementation detail.

---

## 13. Summary — What This Contract Guarantees


If a code is defined in [`09-error-codes.md`](./09-error-codes.md), then:

- It has a symbolic name (`SEM001 AssignTypeMismatch`) and a stable number.
- It renders to CLI in the format defined in §4.2 and to NDJSON in §6.1.
- It maps cleanly to LSP `Diagnostic` per §7, including `codeDescription.href`.
- `cln explain <CODE>` and `errors.cleanlanguage.dev/E/<CODE>` both work.
- Any structured suggestion carries an `Applicability` from §5.1.
- The message obeys §9 style rules.
- The compiler collects it non-fatally per §10.
- The rule has snapshot tests per §12.

Diagnostics that do not meet all eight guarantees are considered incomplete and cannot ship.

---

## Changelog

- 2026-08-15 — §5.4's example block made internally consistent: the `-->` line read `app/reports/summary.cln:18:12` while the caret run and both suggestion spans place `lenght` at columns 17..23 — no single renderer can produce both from one primary span. The caret and suggestion positions are the authoritative ones; the `-->` line now reads `:18:17`. Rendered output only; no code, rule, or format change. Discovered while building the §4.2 renderer in the compiler's Milestone 2 (`clean-language-compiler/docs/DISCOVERIES-M2.md`, item 2).
- 2026-08-01 — Fase 3 remediation per the approved conflict log (P16.6, resolution 0.4): repaired stale internal citations ("fatal-per-§9.1" → §10.1, "collects non-fatally per §9" → §10, "snapshot tests per §11" → §12); §8.2 example URL `…/E/BLD-LAYOUT` (a prohibited kebab code) replaced with a registry-form code (`…/E/SYN003`) and the `PREFIX###` requirement made explicit; §13 symbolic-name example corrected to the real registry name (`SEM001 AssignTypeMismatch`); §10 neutralized compiler-internal/foreign vocabulary ("`DiagnosticSink`" → shared sink as implementation detail; "borrow / capability check" → type and capability checking — Clean has no borrow checker); §10.1 parse-recovery example rewritten without the nonexistent `end` keyword (indentation-based case instead).

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Compiler and language-server implementors emitting diagnostics; tool authors consuming them
- **Rule prefix:** `DIA-`
- **Part of:** [Clean Language Specification — Platform](./README.md)
- **References:** [09 — Error Codes](./09-error-codes.md), [10 — Semantic Rules](./10-semantic-rules.md), [06 — Error Reporting](./06-error-reporting.md), [04 — IDE / Language Server](./04-ide-lsp-architecture.md)
