# 03. Lexical Structure

The lexical structure is the layer between the raw bytes of a `.cln` file and the tokens the parser sees. It fixes the alphabet the language reads code in (ASCII), the alphabet it lets programs carry text in (full Unicode), how comments and literals are formed, what counts as an identifier, and which words are reserved. Every rule here is enforced by the lexer before any type-checking runs; getting any of them wrong is a syntax error, never a semantic one.

### Comments

A comment is not code and carries no restriction on its alphabet ([LEX-02](#lex-02--keywords-and-identifiers-are-ascii-text-is-unicode)).

```clean
// Single line comment

/* 
   Multi-line
   comment
*/
```

### LEX-09 — Block comments nest

`/*` opens a block comment and `*/` closes it, and they **nest**: the lexer tracks depth, raising it at every `/*` and lowering it at every `*/`, and the comment ends when the depth returns to zero. A block comment still open at end of file is [`SYN004`](../03%20platform/09-error-codes.md#31-syntax-codes-syn).

`//` runs to the end of the line. There is nothing to nest, and a `/*` inside it opens nothing.

Comment delimiters have no meaning inside a string literal: `"/*"` is a two-character string, not the start of a comment. Conversely, a quotation mark inside a block comment is an ordinary character and does not open a literal — a comment is not code ([LEX-02](#lex-02--keywords-and-identifiers-are-ascii-text-is-unicode)) and nothing inside it is tokenised.

**Why:** the principal use of a block comment is disabling a region of code while working. Without nesting, that use fails exactly when the region is well documented — the comment ends at the *first* `*/` it meets, the lines after it become code again, and the final `*/` becomes a syntax error at a location unrelated to anything the developer did. C behaves this way, and the fifty-year-old advice not to comment out code with `/* */` is the workaround for it. Nesting costs a counter.

**Check:** a block comment containing a complete block comment compiles as a single comment; removing its final `*/` reports `SYN004`.

### LEX-01 — Indentation is one tab per level

Clean Language uses **tab-based indentation** for code structure:

- **Indentation**: Uses tabs only. Each tab represents one block level
- **Spaces**: May be used within expressions for alignment and formatting, but not for indentation
- **Block Structure**: Indentation defines code blocks (no braces `{}`)
- **Whitespace**: Spaces and tabs. A carriage return is *not* whitespace — it is part of a line terminator, or it is an error ([LEX-07](#lex-07--line-terminators))

**Example:**
```clean
start:
⇥⇥⇥⇥integer x = 5    // Tab indentation
⇥⇥⇥⇥if x > 0
⇥⇥⇥⇥⇥⇥⇥⇥print("positive")    // Nested tab indentation
⇥⇥⇥⇥else
⇥⇥⇥⇥⇥⇥⇥⇥print("zero or negative")
```

**Indentation Rules:**
- Each indentation level must use exactly one tab character
- Mixing tabs and spaces for indentation is [`SYN006`](../03%20platform/09-error-codes.md#31-syntax-codes-syn); using spaces where a tab belongs is [`SYN003`](../03%20platform/09-error-codes.md#31-syntax-codes-syn)
- Spaces within expressions are permitted for readability:
  ```clean
  result = function(arg1,  arg2,  arg3)    // Spaces for alignment
  value  = x + y                           // Spaces around operators
  ```

### LEX-02 — Keywords and identifiers are ASCII; text is Unicode

Clean Language separates the alphabet of its **code** from the alphabet of its **text**:

| | Alphabet |
|---|---|
| Keywords | **ASCII only** |
| Identifiers — variables, parameters, functions, classes, modules | **ASCII only** |
| String literals | **Full Unicode** |
| Comments | **Full Unicode** |

- **Keywords are ASCII.** The four keyword tables below contain only characters in `A-Z` and `a-z`. The language will not add a keyword outside ASCII, and no keyword has an accented, localized, or otherwise non-ASCII spelling.
- **Identifiers are ASCII.** Every name a program declares is written in `A-Z`, `a-z`, digits and `_` (see §Identifiers). A non-ASCII character in a name is [`SYN001`](../03%20platform/09-error-codes.md#31-syntax-codes-syn).
- **String literals carry full Unicode.** A `string` is UTF-8 text ([4 — Type System](./04-type-system.md)) and a literal may contain any Unicode character directly — any script, any punctuation, emoji.
- **Comments carry full Unicode.** A developer documents code in whatever language they think in; comments are never parsed as code.

The split is what makes genuine multilingual work possible without making the language itself ambiguous. Everything a machine must match exactly — the keywords a parser recognises, the names a tool resolves, greps for, and renames — stays in one fixed ASCII vocabulary that every developer and every tool spells identically. Everything written for humans — the content an application shows its users, and the notes its authors leave each other — is unrestricted.

**Source file encoding.** A `.cln` file is UTF-8, and a byte sequence that is not well-formed UTF-8 is not a Clean source file — [TXT-01](../03%20platform/17-text-encoding.md#txt-01--every-ecosystem-text-file-is-utf-8), which governs every file the toolchain reads, not just source. The validation happens where the raw bytes are, in whichever component reads the file ([TXT-02](../03%20platform/17-text-encoding.md#txt-02--the-reader-validates-at-the-moment-it-reads)); the compiler receives text that has already been decoded and performs no encoding check of its own. This is what gives "string literals carry full Unicode" a mechanism.


### LEX-03 — Identifier form

Identifiers must:
- Start with an ASCII letter (`A-Z`, `a-z`)
- Contain only ASCII letters, ASCII digits (`0-9`), and underscores
- Follow camelCase conventions (e.g. `myVariable`, `calculateSum`)

"Letter" and "digit" here mean ASCII, per §Alphabet. A non-ASCII character anywhere in an identifier is [`SYN001`](../03%20platform/09-error-codes.md#31-syntax-codes-syn), even where it would be a letter in Unicode: `café` and `número` are not valid identifiers. Write names in ASCII and put the localized text in string literals and comments, which are unrestricted.

**Valid Examples:**
```clean
x
count
myVariable
value1
calculateSum
```

**Invalid Examples** — each is [`SYN001`](../03%20platform/09-error-codes.md#31-syntax-codes-syn):
```clean
1value      // Cannot start with a digit
my-var      // Hyphens are not identifier characters
$name       // Neither are other punctuation marks
café        // Non-ASCII letters are not identifier characters
```

### LEX-04 — The reserved words are a disjoint, exhaustive partition

Clean Language partitions its reserved words into four disjoint categories. **Every reserved word appears in exactly one of the four tables below**; no word is in two, and no word used as a keyword by any chapter of this specification is missing.

#### Hard Keywords

Reserved everywhere. Using one as an identifier — variable, parameter, function, class, or module name — is [`SYN002`](../03%20platform/09-error-codes.md#31-syntax-codes-syn).

```
after       always      and         assert      background  base
before      block       break       can         class       compiletime
constant    constructor continue    default     else        error
false       function    handles     host        if          import
in          intent      is          iterate     later       none
not         onError     or          print       public      reset
result      return      returns     spec        start       this
to          true        while       with
```

Notes on the less obvious entries:

- `assert` — the assertion form inside a `tests:` block ([11 — Testing](./11-testing.md)).
- `background`, `later` — the asynchronous execution keywords ([18 — Asynchronous Programming](./18-async.md)).
- `base` — calls the parent constructor ([14 — Classes and Objects](./14-classes-and-objects.md)).
- `handles`, `block`, `with` — the block-handler registration `handles block "name" with handlerName` ([21 — Block Handlers](./21-block-handlers.md)).
- `result` — the return value, in scope only inside an `after` contract ([10 — Contracts](./10-contracts.md)).
- `returns` — the return-type clause of a `compiletime function` ([21 §21.1](./21-block-handlers.md#211-declaring-a-block-handler)). Ordinary function declarations use type-first syntax (`Return name(params)`); capability method signatures use arrow-return (`name(params) -> Return`). See [9 — Functions](./09-functions.md) and [14 — Classes and Objects](./14-classes-and-objects.md).
- `default`, `error`, `intent`, `reset`, `spec` — these appear in statement or expression position (`value default 0`, `error("…")`, `spec "path"`, `reset count`), not as block headers, so they are hard rather than contextual.

#### Contextual Keywords

Reserved only as a **block header** — the word followed by `:` (optionally with a name, as in `watch total:`). In any other position they are ordinary identifiers, so `string rules = ""` is valid while `rules:` is still recognized as a block header.

"Any other position" means every position that takes a name, without exception: a variable, a parameter, a function, a class, or a module may all be called `state`, `test` or `build`.

```clean
functions:
	integer test(integer input)      // a function named `test`, a parameter named `input`
		return input
```

The reservation is positional, so it does not depend on what kind of thing is being named. Making it depend on that would be an exception a reader has to memorise with no cause behind it. Nothing is ambiguous either way: a declaration names its type first, a class is introduced by `class`, and a block header is only a block header because of the `:` that follows the word.

```
build       computed    description functions   guard       input
source      state       step        test        tests       watch
```

#### Type Keywords

The names of the built-in types ([4 — Type System](./04-type-system.md)). Reserved everywhere, like hard keywords, and listed separately because they occupy type position rather than statement position.

```
any         boolean     bytes       datetime    integer     list
matrix      number      pairs       string      void
```

#### Reserved Keywords (not yet used)

Reserved for future language versions. They currently have no meaning, but are reserved so that adding them later does not break existing code. Using one as an identifier is [`SYN002`](../03%20platform/09-error-codes.md#31-syntax-codes-syn).

```
for        from       unit
```

- `for` — reserved as an alternate loop keyword. Use `iterate` today.
- `from` — formerly the source clause of a `host function` declaration. That grammar was withdrawn in favour of [LBS-02](../02%20components/framework/09-libraries-specification.md); the word stays reserved so the old form is rejected rather than silently reinterpreted.
- `unit` — reserved for a possible unit / void marker type distinct from `void`.

### LEX-05 — A library may not claim a reserved name

A library registers a block name with `handles block` ([21 §21.2](./21-block-handlers.md#212-block-name-resolution)). It may not claim any word from the four tables above, nor any name beginning with `core.`; attempting to do so fails at library load with [`BLOCK003`](../03%20platform/09-error-codes.md#315-block-handler-codes-block). This table is the single source of that reserved set — [21 §21.2](./21-block-handlers.md#212-block-name-resolution) cites it rather than maintaining its own list.

### LEX-06 — Literal forms

The literal forms below are the complete set; a token that matches none of them is [`SYN001`](../03%20platform/09-error-codes.md#31-syntax-codes-syn).

#### Numeric Literals

**Integers:**
```clean
42          // Decimal
0xff        // Hexadecimal
0b1010      // Binary
0o777       // Octal
```

**Floating-Point:**
```clean
3.14        // Standard decimal
.5          // Leading zero optional
6.02e23     // Scientific notation
```

**A numeric literal has no sign.** `-17` is the unary minus operator ([6 — Expressions](./06-expressions.md)) applied to the literal `17`; the `-` is a token of its own and never part of the number. Both spellings that a reader might expect to differ — `-17` and `- 17` — are therefore the same two tokens, and `a - 1` is unambiguously a subtraction.

The alternative, folding the sign into the literal, would need a rule to tell `a - 1` from `a` followed by `-1`. Only two exist: making the meaning depend on spacing, which contradicts [LEX-01](#lex-01--indentation-is-one-tab-per-level)'s allowance of spaces inside expressions for readability, or having the parser tell the lexer what it expects, which contradicts the compiler's requirement that every pass be a pure function of its input ([Platform 14 §14.4.1](../03%20platform/14-compiler-architecture.md#1441-pass-contracts)). Unary minus would still be needed for `-x` regardless, so the sign-bearing literal adds a second meaning for `-` rather than removing one.

Where this leaves the range of a signed value is [TYP-01](./04-type-system.md#typ-01--the-core-types-and-their-ranges): the range is measured after the sign is applied, which is what keeps the documented minimum writable.

**Where a numeric literal ends.** A `.` is part of a numeric literal **only if an ASCII digit follows it immediately.** Otherwise the `.` is the member-access operator and the literal has already ended.

| Written | Read as | Because |
|---------|---------|---------|
| `.5` | the literal `0.5` | a digit follows the dot |
| `3.14` | one literal | a digit follows the dot |
| `3.14.toInteger()` | the literal `3.14`, then member access | `t` follows the second dot |
| `3.toInteger()` | the literal `3`, then member access | `t` follows the dot |
| `3.` | the literal `3`, then a member access with no member | there is no digit, so the dot is not part of the number |

The rule is needed because a leading dot already starts a literal — `.5` is valid — so after a complete literal a dot is ambiguous on its own. One condition removes the ambiguity everywhere, with no special case for the leading-dot form and no lookahead beyond a single character.

Two consequences follow rather than needing rules of their own. `3.` is not a numeric literal, so there is no "float without a fractional part" form. And `3.toInteger()` is written without parentheses around the `3`, because the dot cannot be mistaken for the start of a fraction.

**The digits are ASCII.** Every digit of every numeric literal is an ASCII character: `0`–`9` in decimal and in an exponent, `0`–`1` in binary, `0`–`7` in octal, and `0`–`9` together with `a`–`f` or `A`–`F` in hexadecimal. Unicode marks several hundred characters across dozens of scripts as decimal digits — `٤٢`, `४२`, `๔๒` all denote forty-two — and none of them is a Clean numeric literal. A digit from any other script is [`SYN001`](../03%20platform/09-error-codes.md#31-syntax-codes-syn).

**There are no digit separators.** `1_000_000` is not a numeric literal; the underscore is [`SYN001`](../03%20platform/09-error-codes.md#31-syntax-codes-syn) in that position. Digits run uninterrupted.

The choice is deliberately the reversible one. Accepting separators later compiles every program written before; rejecting them later breaks programs that already work, so the direction that can be undone is the one taken first. Accepting them would also require settling where the underscore may sit — leading, trailing, doubled, adjacent to the decimal point, inside a hexadecimal prefix, inside an exponent — and every one of those left unwritten is a place where two implementations diverge silently. Refusing them asks none of those questions.

This is [LEX-02](#lex-02--keywords-and-identifiers-are-ascii-text-is-unicode) applied to numbers: what a machine must recognise exactly stays in one fixed ASCII vocabulary, and what is written for a human is unrestricted. A number is code. The alternative would oblige the lexer to carry the Unicode character database in order to read an integer, and would let a literal that displays as `42` in a reviewer's font hold a different value — a class of defect the language is better off unable to express. Text shown to an end user is unaffected: that is a string, and strings carry all of Unicode.

#### String Literals

**Basic Strings:**
```clean
"Hello, World!"
"Line 1\nLine 2"
""          // Empty string
```

**Escape Sequences:**

Clean Language supports standard escape sequences within string literals:

| Sequence | Result | Example |
|----------|--------|---------|
| `\"` | Double quote | `"say \"hi\""` → `say "hi"` |
| `\\` | Backslash | `"path\\file"` → `path\file` |
| `\n` | Newline | `"line1\nline2"` → two lines |
| `\t` | Tab | `"col1\tcol2"` → tab-separated |
| `\r` | Carriage return | `"text\r"` → with CR |
| `\{` | Literal left brace | `"\{not interpolation\}"` → `{not interpolation}` |
| `\}` | Literal right brace | `"\{literal\}"` → `{literal}` |
| `\0` | Null character | `"text\0"` → null-terminated |
| `\uXXXXXX` | The character with that Unicode code point | `"caf\u0000E9"` → `café` |

**The Unicode escape.** `\u` is followed by **exactly six hexadecimal digits**, upper or lower case, naming the character's Unicode code point. Six is the width of the largest code point that exists (`10FFFF`), so every character is expressible and no character ever needs two escapes.

The width is fixed precisely so that no delimiter is needed. Hexadecimal digits include `a`-`f`, so a variable-length escape could not be told apart from the text that follows it: in `"\u1F600abc"` the run of hexadecimal characters is eight long, and every cut between four and eight of them would be a legal reading. Languages that allow a variable count spend a pair of braces to mark where the number ends; a fixed width buys the same certainty for nothing.

Rejected at compile time as [`SYN005`](../03%20platform/09-error-codes.md#31-syntax-codes-syn): any `\u` not followed by exactly six hexadecimal digits, and these two ranges of value —

- `00D800`-`00DFFF`, the halves of a surrogate pair. Clean has no surrogate-pair escape: a character above `00FFFF` is written as one `\u` escape, never as two.
- anything above `10FFFF`, which is outside Unicode.

Both rejections are what sustains the guarantee that no ill-formed sequence reaches memory ([03 — Memory Model](../03%20platform/03-memory-model.md)). An escape is the only way a string literal could otherwise name something that is not a character.

The escape exists for characters that cannot be seen, or cannot be told apart: a non-breaking space (`\u0000A0`), a zero-width joiner (`\u00200D`), a soft hyphen (`\u0000AD`), an en dash where a hyphen was meant (`\u002013`). A character that is simply *visible* — any emoji, any script — is written directly in the source instead: the file is UTF-8 ([TXT-01](../03%20platform/17-text-encoding.md#txt-01--every-ecosystem-text-file-is-utf-8)), so it needs no escape at all.

**Escape Sequences in JSON Strings:**

Escape sequences are particularly useful when working with JSON:

```clean
// JSON with escaped quotes
string jsonStr = "{\"name\": \"Alice\", \"age\": 25}"

// Parse JSON containing escape sequences
any data = json.textToData("{\"count\": 42}")
integer count = data.count  // Returns 42

// Nested JSON with multiple escape sequences
string nestedJson = "{\"user\": {\"name\": \"Bob\", \"active\": true}}"
any parsed = json.textToData(nestedJson)
string userName = parsed.user.name  // Returns "Bob"
```

**Multi-line Strings:**

A literal opened with `"""` runs until the next `"""`, across as many lines as needed. The opening delimiter MUST be followed by a line terminator; a single-line string is written with `"`.

```clean
string block = """
	data UserData:
		fields:
			integer id primary
	"""
```

Four rules define its content:

- **Nothing inside is interpreted.** Escape sequences and interpolation are inert: `\n` is a backslash followed by an `n`, and `{name}` is three characters and a name. A `"` is ordinary content, so JSON needs no escaping. Only `"""` ends the literal, which is therefore the one sequence a multi-line string cannot contain.
- **The opening and closing line breaks are not content.** The terminator right after the opening delimiter, and the one right before the closing delimiter, are discarded — without this every such literal would begin and end with a blank line nobody wrote.
- **The closing delimiter sets the left margin.** Its indentation is removed from every content line. In the example the closing `"""` sits at one tab, so one tab comes off each line and the content begins at column zero.
- **A content line indented less than the closing delimiter is [`SYN005`](../03%20platform/09-error-codes.md#31-syntax-codes-syn).** There is no margin to remove from it, and silently keeping it would produce content the author cannot see is wrong.

The margin rule exists because indentation is structure in Clean ([LEX-01](#lex-01--indentation-is-one-tab-per-level)). Embedded text sits wherever the surrounding code sits, so a literal that preserved every tab would hand its consumer text indented by an amount that depends on how deeply the *enclosing* code happens to be nested — for embedded Clean source, structurally broken text. Tying the margin to the closing delimiter puts the choice in view, on the line that ends the literal.

Interpretation is off for the same reason: the text being carried is usually code, and code has its own braces and backslashes. A literal that read them would consume what it exists to transport. A string that needs interpolation is an ordinary `"` string.

Line terminators inside are already `\n` whatever the file held, by [LEX-07](#lex-07--line-terminators), so the content does not vary with the platform the repository was checked out on.

**String Interpolation:**
```clean
name = "World"
greeting = "Hello, {name}!"     // Results in "Hello, World!"

// Simple property access allowed
user = User("Alice", 25)
message = "User {user.name} is {user.age} years old"

// Note: Complex method calls in strings are not supported
// ❌ "Hello {user.name}, you have {messages.count()} messages"
```

**Interpolation vs Literal Braces:**

The compiler distinguishes between interpolation and literal braces:
- `"{variable}"` → Interpolation (evaluates `variable`)
- `"{obj.prop}"` → Interpolation (evaluates `obj.prop`)
- `"{(expr)}"` → Interpolation (evaluates expression)
- `"{\"literal\"}"` → NOT interpolation (produces `{"literal"}`)
- `"\{literal\}"` → NOT interpolation (produces `{literal}`)

#### Bytes Literals

A `bytes` value ([4 — Type System](./04-type-system.md), [15 §Bytes Module](./15-standard-library.md#bytes-module)) can be written directly with the prefix `b` on the single-line string shape:

```clean
b"GET / HTTP/1.1\r\n"     // the UTF-8 bytes of the text, escapes applied
b"\x00\xFF\x7F"           // arbitrary bytes, two hex digits each
b"PNG\x89"                // text and hex bytes mix freely
```

The prefix attaches with no intervening space. The escapes are the string set (`\"`, `\\`, `\n`, `\t`, `\r`, `\{`, `\}`, `\0`) plus `\xNN` — exactly two hex digits naming one byte. There is no `\u` escape and no multi-line form: a bytes value has no code points to name, so a character outside ASCII is written directly (its UTF-8 bytes are the value) or spelled out with `\xNN`. Grammar production: [`03-lexical-structure.ebnf.md` §7](./grammar/03-lexical-structure.ebnf.md); compiler contract: [Platform 14 §14.14.2](../03%20platform/14-compiler-architecture.md#14142-first-class-bytes-type).

#### Boolean Literals
```clean
true
false
```

#### None Literal

The `none` value represents the absence of a value. It is distinct from `0`, `false`, or empty string `""`.

```clean
none        // The none value
```

**None Semantics:**
- `none` is its own type that is compatible with any optional context
- `none == none` is `true`
- `none == anything_else` is `false` (except for another none)
- Use the `default` operator to provide fallback values for none
- Use the `!` operator to assert a value is not none

#### List Literals
```clean
[1, 2, 3, 4]           // Integer list
["a", "b", "c"]        // String list
[]                     // Empty list
[true, false, true]    // Boolean list
```

#### Matrix Literals
```clean
[[1, 2], [3, 4]]                    // 2x2 matrix
[[1, 2, 3], [4, 5, 6], [7, 8, 9]]   // 3x3 matrix
[[]]                                // Empty matrix
```

### LEX-07 — Line terminators

A line is terminated by `\n` or by `\r\n`. Both are one terminator, and the lexer replaces every `\r\n` with `\n` **before doing anything else** — before measuring indentation, before producing a token, before reading the content of any literal. Everything downstream of that step sees `\n` and cannot tell which convention the file used.

A carriage return that is not immediately followed by a line feed is [`SYN001`](../03%20platform/09-error-codes.md#31-syntax-codes-syn). It is not whitespace and it does not terminate a line.

The final line of a file need not be terminated. End of file terminates it.

Because the replacement happens on the character stream rather than per construct, a literal whose content spans several lines carries `\n` regardless of how the file was stored. This is the part of the rule that is about meaning rather than parsing: git converts terminators on checkout without being asked, so a literal that preserved them would hold different text on a Windows machine than on a macOS one — for the same commit, with nothing visible on screen in either. The comparison that passes for one developer would fail for the other, and neither could see why.

Line terminators are load-bearing here in a way they are not in most languages, because indentation defines block structure ([LEX-01](#lex-01--indentation-is-one-tab-per-level)). A stray carriage return in the middle of a line is invisible in every editor; treating it as whitespace would let it sit inside indentation and change what a block contains, undetectably. That is why it is an error rather than something to skip.

This rule governs what the compiler *reads*. What the toolchain *writes* is [TXT-05](../03%20platform/17-text-encoding.md#txt-05--a-tool-that-rewrites-a-file-preserves-its-line-terminators) and [TXT-06](../03%20platform/17-text-encoding.md#txt-06--a-tool-that-creates-a-file-uses-the-projects-declared-convention-and-n-when-there-is-none): tools preserve the convention of a file they rewrite, and use the project's declared convention for one they create. The compiler never writes a source file, so the two never conflict.

### LEX-08 — Every name is case-sensitive

Every name the language recognises is matched by its exact character sequence. No case folding is performed at any point, for any category of name: identifiers, keywords, type names, namespace names, and the block names a library registers ([LEX-05](#lex-05--a-library-may-not-claim-a-reserved-name)).

`total` and `Total` are two different names and may coexist in one scope. `if` is a keyword; `If` and `IF` are not keywords and are ordinary identifiers.

**Why:** the specification already depends on this in places that would otherwise be meaningless. [LDR-05](./02-language-design-rules.md#ldr-05--namespace-names-are-lowercase) requires `math.sqrt()` and forbids `Math.sqrt()` — a distinction that exists only if the two spellings are different names. The naming conventions of [LEX-03](#lex-03--identifier-form) put a class `User` and a variable `user` in the same scope as a matter of routine, which case folding would turn into a collision.

It also follows from what [LEX-02](#lex-02--keywords-and-identifiers-are-ascii-text-is-unicode) says names are *for*: something a tool resolves, greps for and renames. Under case folding, a search for a name would have to match spellings that are not written anywhere, and a rename would have to rewrite them.

**Check:** a program declaring both `User` and `user` in one scope compiles, and the two are independent; `IF` used as a variable name compiles.

## Changelog

- 2026-08-19 — Erratum from compiler Milestone 6 (`clean-language-compiler/docs/DISCOVERIES-M6.md`, item 6h): [LEX-06](#lex-06--literal-forms) gains the **Bytes Literals** form, `b"…"` with the string escapes plus `\xNN` and no `\u` escape or multi-line form. [Platform 14 §14.14.2](../03%20platform/14-compiler-architecture.md#14142-first-class-bytes-type)'s Accepted lexer contract required the token while this chapter's "complete set" of literal forms and the authoritative grammar both omitted it, leaving the literal unimplementable; the grammar production lands in [`03-lexical-structure.ebnf.md` §7](./grammar/03-lexical-structure.ebnf.md) in the same change.
- 2026-08-07 — `screen` REMOVED entirely per [ADR-0030](../01%20governance/decisions/0030-withdraw-screen-from-language.md). Not a keyword of any kind — hard, contextual, or reserved-unused. The word `screen` is a free identifier for user code. The former `screen <Name>:` language section is withdrawn; no future language use is planned. The [ui library](../02%20components/framework/libraries/10-ui.md) also does NOT register `screen` as a block name (removed from its block-registration list in the framework spec).
- 2026-08-02 — Multi-line `"""` literals defined in [LEX-06](#lex-06--literal-forms), closing question 13 of [ADR-0014](../01%20governance/decisions/0014-source-text-encoding-and-identifier-charset.md) and with it the whole ADR. The form was already in use by [21 — Block Handlers](./21-block-handlers.md) and appeared in no literal table — syntax in service with no specification. Content is uninterpreted, the opening and closing line breaks are dropped, and the closing delimiter's indentation is the margin removed from every line; a line indented less than it is `SYN005`. The chapter's open-questions note is removed: nothing in this chapter is open.
- 2026-08-02 — [LEX-04](#lex-04--the-reserved-words-are-a-disjoint-exhaustive-partition) now states that a contextual keyword may name anything — variable, parameter, function, class or module — closing question 12 of [ADR-0014](../01%20governance/decisions/0014-source-text-encoding-and-identifier-charset.md). The chapter had committed only to the variable case, leaving the others unstated; since the reservation is positional, restricting it by the kind of thing named would have been an exception without a cause.
- 2026-08-02 — A numeric literal carries no sign, closing question 11 of [ADR-0014](../01%20governance/decisions/0014-source-text-encoding-and-identifier-charset.md). [LEX-06](#lex-06--literal-forms) had been presenting `-17` and `-2.5` as negative literals while [6 — Expressions](./06-expressions.md) made unary minus an operator — one text with two readings, in two Accepted chapters. The lexical chapter is the one corrected. Range measurement moves to its home in [TYP-01](./04-type-system.md#typ-01--the-core-types-and-their-ranges).
- 2026-08-02 — The end of a numeric literal fixed in [LEX-06](#lex-06--literal-forms), closing question 10 of [ADR-0014](../01%20governance/decisions/0014-source-text-encoding-and-identifier-charset.md): a `.` belongs to a numeric literal only when an ASCII digit follows it immediately. One condition resolves the leading-dot form, member access on a literal, and the status of `3.` — which is consequently not a literal. `literal` added to the [glossary](../01%20governance/06-glossary.md), where it was missing.
- 2026-08-02 — `LEX-09` minted, closing question 9 of [ADR-0014](../01%20governance/decisions/0014-source-text-encoding-and-identifier-charset.md): block comments nest, an unterminated one is `SYN004`, and comment delimiters are inert inside a string literal (and quotation marks inert inside a comment). This gives [`SYN004`](../03%20platform/09-error-codes.md#31-syntax-codes-syn) its first citation from any chapter — it was registered with no rule pointing at it.
- 2026-08-02 — `LEX-08` minted, closing question 8 of [ADR-0014](../01%20governance/decisions/0014-source-text-encoding-and-identifier-charset.md): every name is matched by its exact characters and nothing is case-folded. It was never stated, though [LDR-05](./02-language-design-rules.md#ldr-05--namespace-names-are-lowercase)'s ban on `Math.sqrt()` and the `User`/`user` convention of [LEX-03](#lex-03--identifier-form) both already presumed it.
- 2026-08-02 — Digit separators rejected in [LEX-06](#lex-06--literal-forms), closing question 7 of [ADR-0014](../01%20governance/decisions/0014-source-text-encoding-and-identifier-charset.md): `1_000_000` is not a literal. Taken as the reversible direction — separators can be added later without breaking any existing program, while removing them could not — and it avoids having to settle the six placement questions an underscore raises. The §Alphabet open-questions note is retired: nothing it listed is open any more. It is replaced by a chapter-level note listing the six lexical questions that genuinely remain, which live in §Comments, LEX-04 and LEX-06 rather than around the alphabet.
- 2026-08-02 — Stale open-question note removed from [LEX-03](#lex-03--identifier-form). It said the identifier character set was "the open question recorded in ADR-0014" and told the reader to write ASCII "until it is decided" — directly contradicting the two bullets above it, which state ASCII normatively, and [LEX-02](#lex-02--keywords-and-identifiers-are-ascii-text-is-unicode), which settled it. [ADR-0014](../01%20governance/decisions/0014-source-text-encoding-and-identifier-charset.md) had already recorded it as settled for identifiers; only the note was left behind. The §Alphabet note is likewise reduced to the one mechanic still open.
- 2026-08-02 — The digit character set of numeric literals fixed to ASCII in [LEX-06](#lex-06--literal-forms), closing question 6 of [ADR-0014](../01%20governance/decisions/0014-source-text-encoding-and-identifier-charset.md). Unicode decimal digits from other scripts are `SYN001`. This is [LEX-02](#lex-02--keywords-and-identifiers-are-ascii-text-is-unicode) applied to numbers, and it keeps the Unicode character database out of the lexer's number path; the homograph case — a literal that renders as `42` and is not — becomes unrepresentable.
- 2026-08-02 — `LEX-07` minted, closing the lexer half of question 4 of [ADR-0014](../01%20governance/decisions/0014-source-text-encoding-and-identifier-charset.md): `\n` and `\r\n` both terminate a line and are normalised to `\n` on the character stream before indentation is measured; a lone carriage return is `SYN001`; a final terminator is not required; and because the normalisation precedes tokenising, multi-line literal content always carries `\n` whatever git wrote to disk. **LEX-01 corrected**: it listed carriage returns as whitespace, which made an invisible stray `\r` legal inside significant indentation. The toolchain half is [TXT-05](../03%20platform/17-text-encoding.md#txt-05--a-tool-that-rewrites-a-file-preserves-its-line-terminators) / [TXT-06](../03%20platform/17-text-encoding.md#txt-06--a-tool-that-creates-a-file-uses-the-projects-declared-convention-and-n-when-there-is-none).
- 2026-08-02 — The Unicode escape added to the LEX-06 escape table, closing question 2 of [ADR-0014](../01%20governance/decisions/0014-source-text-encoding-and-identifier-charset.md): six hexadecimal digits, exactly, naming the code point. Fixed width rather than variable, because hexadecimal digits include `a`–`f` and a variable-length escape cannot be told apart from the text after it — which is the ambiguity braces exist to resolve elsewhere, and the reason none are needed here. Six is the width of the largest code point that exists, so there is no plane the escape cannot reach and no surrogate-pair form: the two halves (`00D800`–`00DFFF`) and anything above `10FFFF` are rejected as [`SYN005`](../03%20platform/09-error-codes.md#31-syntax-codes-syn), which is what sustains the memory model's guarantee that no ill-formed sequence reaches memory.
- 2026-08-02 — Source file encoding fixed, closing question 1 of [ADR-0014](../01%20governance/decisions/0014-source-text-encoding-and-identifier-charset.md): §Alphabet now states that a `.cln` file is UTF-8 and cites [TXT-01](../03%20platform/17-text-encoding.md#txt-01--every-ecosystem-text-file-is-utf-8) / [TXT-02](../03%20platform/17-text-encoding.md#txt-02--the-reader-validates-at-the-moment-it-reads) rather than restating the rule (DOC-14) — the invariant is ecosystem-wide and its home is [03 platform / 17 — Text Files](../03%20platform/17-text-encoding.md). The open-questions note is reduced accordingly; encoding and byte-order marks are no longer the same item, the latter remaining open.
- 2026-08-01 — Fase 5 (zero-debt pass): the lexical failures now cite their codes — reserved word as identifier is [`SYN002`](../03%20platform/09-error-codes.md#31-syntax-codes-syn), a non-ASCII or otherwise invalid identifier character is [`SYN001`](../03%20platform/09-error-codes.md#31-syntax-codes-syn), tab/space mixing is [`SYN006`](../03%20platform/09-error-codes.md#31-syntax-codes-syn).
- 2026-08-01 — Fase 3/4 (L3, L9, L10, alphabet): §Alphabet added — **keywords and identifiers are ASCII; string literals and comments carry full Unicode** (user decision, now written down as a normative rule); §Identifiers made explicit that "letter" and "digit" mean ASCII. The keyword tables rebuilt as a **disjoint, exhaustive partition** of four categories: eight tokens that were in two lists at once (`state`, `watch`, `reset`, `source`, `spec`, `intent`, `default`, `error`) assigned to one each; fifteen missing tokens added (`later`, `background`, `handles`, `block`, `with`, `assert`, `base`, `result`, and the ten type names); `returns` promoted from reserved-unused to a hard keyword ([21 §21.1](./21-block-handlers.md#211-declaring-a-block-handler) uses it); `from` retired to reserved-unused (the withdrawn `host function … from` clause); `libraries` removed (L4). [21 §21.2](./21-block-handlers.md#212-block-name-resolution) now cites these tables instead of keeping a second list. Rules `LEX-01`..`LEX-06` minted; prefix `LEX-` registered.

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Clean Language users learning what the compiler will and will not accept at the token level; compiler and tool authors implementing the lexer
- **Rule prefix:** `LEX-`
- **Part of:** [Clean Language Specification — Language](./README.md)
- **References:** [Type System](./04-type-system.md) (numeric ranges), [Platform 09 — Error Codes](../03%20platform/09-error-codes.md) (`SYN001`–`SYN008`), [Platform 17 — Text Encoding](../03%20platform/17-text-encoding.md), [ADR-0014](../01%20governance/decisions/0014-source-text-encoding-and-identifier-charset.md)
