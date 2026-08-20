# 03 lexical-structure — Grammar

Companion grammar file for [03 — Lexical Structure](../03-lexical-structure.md). Defines every lexical terminal (tokens, keywords, literals, indentation events, line terminators) that later grammar files consume. Semantic rules attached to these productions (LEX-01..LEX-09) live in the companion chapter; this file states only the shape of what the lexer produces.

---

## 1. Character classes

```ebnf
(* Terminals defined by the file's byte stream, after UTF-8 decoding per
   TXT-01 / TXT-02. See LEX-02 for the ASCII/Unicode split. *)

AsciiLetter    = ? one character in A-Z or a-z ? ;
AsciiDigit     = ? one character in 0-9 ? ;
HexDigit       = AsciiDigit | ? one character in a-f or A-F ? ;
OctalDigit     = ? one character in 0-7 ? ;
BinaryDigit    = "0" | "1" ;
UnicodeChar    = ? any Unicode scalar value permitted by TXT-01 ? ;
Tab            = ? one U+0009 character ? ;
Space          = ? one U+0020 character ? ;
LF             = ? one U+000A character ? ;
CR             = ? one U+000D character ? ;
```

## 2. Line terminators and whitespace

```ebnf
(* LEX-07: \n or \r\n; a lone CR is SYN001. The lexer normalises
   CRLF to LF before anything else, so downstream productions see
   only NEWLINE below. *)

LineTerminator = LF | CR, LF ;

(* NEWLINE and INDENT/DEDENT are the physical tokens the lexer emits
   after normalisation. Grammar files elsewhere use these three
   terminals, not LineTerminator/Tab directly. *)

NEWLINE        = ? one line terminator event ? ;
INDENT         = ? one tab of additional indentation, per LEX-01 ? ;
DEDENT         = ? one tab of reduced indentation, per LEX-01 ? ;

(* Inline whitespace inside a line — spaces or tabs — is not a token;
   it separates other tokens. Tab in indentation position is
   structural, not whitespace. *)

InlineSpace    = { Space | Tab } ;
```

## 3. Comments

```ebnf
(* LEX-09: line comments run to the end of the line; block comments
   nest, tracking depth. A block comment still open at EOF is SYN004. *)

LineComment    = "//", { UnicodeChar - LF }, ( LineTerminator | ? EOF ? ) ;

BlockComment   = "/*", BlockCommentBody, "*/" ;

BlockCommentBody = { UnicodeChar - "/*" - "*/" | BlockComment } ;
```

## 4. Identifiers and keywords

```ebnf
(* LEX-03: identifiers are ASCII, start with a letter, camelCase.
   The camelCase convention is a naming rule (LEX-03 prose), not
   enforced by the grammar. *)

Identifier     = AsciiLetter, { AsciiLetter | AsciiDigit | "_" } ;

(* LEX-04: hard keywords — reserved everywhere. Identifier positions
   holding one of these are SYN002. The keyword tables are the single
   source per LEX-04; this production enumerates them for the parser. *)

HardKeyword    = "after"    | "always"      | "and"         | "assert"
               | "background"| "base"       | "before"      | "block"
               | "break"    | "can"         | "class"       | "compiletime"
               | "constant" | "constructor" | "continue"    | "default"
               | "else"     | "error"       | "false"       | "function"
               | "handles"  | "host"        | "if"          | "import"
               | "in"       | "intent"      | "is"          | "iterate"
               | "later"    | "none"        | "not"         | "onError"
               | "or"       | "print"       | "public"      | "reset"
               | "result"   | "return"      | "returns"     | "spec"
               | "start"    | "this"        | "to"          | "true"
               | "while"    | "with" ;

(* LEX-04: contextual keywords — reserved only as block headers
   (word followed by ":"). In any other position they are ordinary
   identifiers. The grammar accepts these as identifiers everywhere;
   the parser specialises them when a ":" immediately follows.
   This matches how Python, Rust, and Kotlin handle contextual
   keywords. *)

ContextualKeyword = "build"    | "computed"    | "description" | "functions"
                  | "guard"    | "input"       | "source"      | "state"
                  | "step"     | "test"        | "tests"       | "watch" ;

TypeKeyword    = "any"      | "boolean"     | "bytes"       | "datetime"
               | "integer"  | "list"        | "matrix"      | "number"
               | "pairs"    | "string"      | "void" ;

ReservedUnused = "for" | "from" | "unit" ;
```

## 5. Numeric literals

```ebnf
(* LEX-06: numeric literals carry no sign. Sign is unary minus,
   applied elsewhere. Digits are ASCII. No digit separators. *)

IntegerLiteral = HexLiteral | BinaryLiteral | OctalLiteral | DecimalIntegerLiteral ;

HexLiteral        = "0x", HexDigit, { HexDigit } ;
BinaryLiteral     = "0b", BinaryDigit, { BinaryDigit } ;
OctalLiteral      = "0o", OctalDigit, { OctalDigit } ;
DecimalIntegerLiteral = AsciiDigit, { AsciiDigit } ;

NumberLiteral  = DecimalIntegerLiteral, ".", DecimalIntegerLiteral, [ Exponent ]
               | ".", DecimalIntegerLiteral, [ Exponent ]
               | DecimalIntegerLiteral, Exponent ;

Exponent       = ( "e" | "E" ), [ "+" | "-" ], DecimalIntegerLiteral ;

(* LEX-06 dot-boundary rule: a "." is part of a NumberLiteral only if
   an ASCII digit follows immediately. The productions above encode this:
   NumberLiteral requires a digit on both sides of the dot (or on the
   right side if the number leads with ".").  "3." is therefore not
   a NumberLiteral — it is the IntegerLiteral "3" followed by "." as
   the member-access operator. *)
```

## 6. String literals

```ebnf
(* LEX-06: single-line strings use ", multi-line strings use """.
   Escapes are recognised inside "; nothing is interpreted inside """. *)

StringLiteral       = SingleLineString | MultiLineString ;

SingleLineString    = '"', { StringCharacter | EscapeSequence }, '"' ;

StringCharacter     = UnicodeChar - '"' - "\" - LF ;

EscapeSequence      = SimpleEscape | UnicodeEscape ;

SimpleEscape        = "\", ( '"' | "\" | "n" | "t" | "r" | "{" | "}" | "0" ) ;

UnicodeEscape       = "\u", HexDigit, HexDigit, HexDigit, HexDigit, HexDigit, HexDigit ;
                    (* LEX-06: exactly six hex digits. Values in
                       00D800-00DFFF or above 10FFFF are SYN005. *)

MultiLineString     = '"""', NEWLINE, { MultiLineContent }, MultiLineClose ;

MultiLineContent    = ? any line whose content is not exactly the close
                        delimiter — text is uninterpreted inside ? ;

MultiLineClose      = { InlineSpace }, '"""' ;
                    (* LEX-06: the close delimiter's indentation sets
                       the margin removed from every content line;
                       a content line indented less is SYN005. *)

(* Interpolation inside SingleLineString: {expr} evaluates expr.
   \{ and \} escape literal braces. Interpolation productions live
   in 06-expressions.ebnf.md because their body is an expression. *)
```

## 7. Other literals

```ebnf
(* Bytes literal — the compiler contract of Platform 14 §14.14.2.
   The prefix "b" attaches with no intervening space; the payload is
   the UTF-8 bytes of the text with escapes applied. String escapes
   are recognised plus \xNN for an arbitrary byte; there is no \u
   escape (a bytes value has no code points to name — encode the
   character in the text or spell its bytes with \xNN). No multi-line
   form. *)

BytesLiteral   = "b", '"', { BytesCharacter | BytesEscape }, '"' ;

BytesCharacter = UnicodeChar - '"' - "\" - LF ;

BytesEscape    = SimpleEscape | HexByteEscape ;

HexByteEscape  = "\x", HexDigit, HexDigit ;

BooleanLiteral = "true" | "false" ;

NoneLiteral    = "none" ;

(* LEX-06 list and matrix literal shapes. The element expressions are
   defined in 06-expressions.ebnf.md. *)

ListLiteral    = "[", [ Expression, { ",", Expression } ], "]" ;

MatrixLiteral  = "[", [ ListLiteral, { ",", ListLiteral } ], "]" ;
                (* A matrix literal is a list of row literals.
                   [[]] is an empty matrix. *)
```

## 8. Punctuation and operator tokens

```ebnf
(* Single-character punctuation tokens the lexer recognises.
   Operators with semantic behaviour (precedence, associativity)
   are grouped in 06-expressions.ebnf.md; this section lists only the
   raw tokens. *)

LParen         = "(" ;
RParen         = ")" ;
LBracket       = "[" ;
RBracket       = "]" ;
LBrace         = "{" ;   (* Interpolation and (per parser rule) member access grouping *)
RBrace         = "}" ;
Comma          = "," ;
Colon          = ":" ;
Dot            = "." ;   (* Member access; also numeric-literal decimal per §5 *)
Assign         = "=" ;
Question       = "?" ;   (* Optional-type marker per TYP-03 *)
Bang           = "!" ;   (* Non-none assertion per TYP-03 / EXP *)
Arrow          = "->" ;  (* Capability return per CLS *)

(* Arithmetic and comparison operators — enumerated here so the
   lexer's token vocabulary is complete; precedence lives with the
   expression grammar. *)

Plus           = "+" ;
Minus          = "-" ;
Star           = "*" ;
Slash          = "/" ;
Percent        = "%" ;
Caret          = "^" ;   (* Exponentiation — level 4 in 06-expressions.ebnf.md, per EXP-01 *)
Eq             = "==" ;
NEq            = "!=" ;
Lt             = "<" ;
LtEq           = "<=" ;
Gt             = ">" ;
GtEq           = ">=" ;
```

## 9. Case sensitivity note

```ebnf
(* LEX-08: every token above is matched by its exact character sequence.
   No case folding at any point.  `if` matches only "if", not "If"
   or "IF"; the latter two are ordinary Identifier tokens. *)
```

---

## Changelog

- 2026-08-19 — Erratum from compiler Milestone 6 (`clean-language-compiler/docs/DISCOVERIES-M6.md`, item 6h): §7 gains the `BytesLiteral` production. [Platform 14 §14.14.2](../../03%20platform/14-compiler-architecture.md#14142-first-class-bytes-type)'s Accepted lexer contract requires `b"..."` and hex-escaped forms, but this file — the syntax authority per DOC-15 — had `bytes` only as a `TypeKeyword`, so no literal was implementable. The production admits the single-line string shape with `SimpleEscape` plus `\xNN` and no `\u` escape; prose home: [03 §LEX-06](../03-lexical-structure.md).
- 2026-08-17 — Erratum from compiler Milestone 3 (`clean-language-compiler/docs/DISCOVERIES-M3.md`, item 5): §8 gains the missing `Caret = "^" ;` row. [06-expressions.ebnf.md](./06-expressions.ebnf.md) uses `"^"` at level 4 (`ExponentiationExpression`) and the lexer recognises it, but the §8 token vocabulary — which claims completeness for the lexer — omitted it. Registration only; precedence and associativity stay with the expression grammar.
- 2026-08-07 (afternoon, third pass) — `screen` REMOVED entirely per [ADR-0030](../../01%20governance/decisions/0030-withdraw-screen-from-language.md). Not a keyword of any kind — not `HardKeyword`, not `ContextualKeyword`, not `ReservedUnused`. The word `screen` is a free identifier for user code. The former `screen <Name>:` language section is withdrawn and no future language use is planned; the [ui library](../../02%20components/framework/libraries/10-ui.md) also does not register `screen` as a block name.
- 2026-08-07 (afternoon) — Resolved the §4 `⚠` marker: contextual-keyword handling stays as a parser-policy responsibility (grammar lists the words, parser dispatches on the following `":"`). Matches Python/Rust/Kotlin convention. No production change.
- 2026-08-07 — File minted. Productions derived from prose rules LEX-01..LEX-09 in [03-lexical-structure.md](../03-lexical-structure.md) Accepted 2026-08-01.

---

## Metadata

- **Status:** Accepted (2026-08-07)
- **Audience:** Lexer implementers; downstream grammar-file authors who reference these terminals
- **Notation:** EBNF (ISO/IEC 14977)
- **Part of:** [04 language / grammar / README.md](./README.md)
- **Rules referencing this grammar:** [03-lexical-structure.md](../03-lexical-structure.md) (LEX-01..LEX-09)
- **References:** [LEX-01..LEX-09](../03-lexical-structure.md), [TXT-01, TXT-02](../../03%20platform/17-text-encoding.md)
