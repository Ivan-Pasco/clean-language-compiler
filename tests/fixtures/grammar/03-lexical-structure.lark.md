# 03 lexical-structure — Grammar

**Title:** 03 lexical-structure — Grammar

**Id:** `LEXG-`

**Purpose:** Companion grammar file for 03 — Lexical Structure. It defines, in Lark, every terminal the lexer produces and later grammar files consume, with the small rules that name the literal forms: the character classes drawn over the decoded byte stream, line terminators and the NEWLINE, INDENT and DEDENT events, comments, identifiers and the reserved words, numeric, string, bytes, boolean, none, list and matrix literals, the punctuation and operator tokens, and the rule that every token is matched by its exact character sequence. It states only the shape of what the lexer produces; the semantic rules attached to these productions (LEX-01..LEX-09) live in the companion chapter, and what is an expression — interpolation bodies, operator precedence, list elements — belongs to 06-expressions.lark.md. It is written for implementors of the lexer and parser and for authors of the other grammar files, and rests on 03 — Lexical Structure and the text encoding rules of TXT-01 and TXT-02.

---

### LEXG-01 — 1. Character classes

**Id:** LEXG-01

**Kind:** rule

**Name:** 1. Character classes

**Question:** Over which character classes — which ASCII letters and digits, which hex, octal and binary digits, which Unicode scalars and which whitespace and line characters — are every later production's terminals drawn from the decoded byte stream?

**Norm:**

```lark
// Terminals drawn from the file's byte stream, after UTF-8 decoding per
// TXT-01 / TXT-02.  See LEX-02 for the ASCII/Unicode split.  These are
// fragments: the later terminals are spelled with them, and none of
// them reaches the parser on its own.

ASCII_LETTER: /[A-Za-z]/
ASCII_DIGIT: /[0-9]/
HEX_DIGIT: ASCII_DIGIT | /[a-fA-F]/
OCTAL_DIGIT: /[0-7]/
BINARY_DIGIT: "0" | "1"
// Any Unicode scalar value permitted by TXT-01.
UNICODE_CHAR: /[\s\S]/
TAB: "\t"
SPACE: " "
LF: "\n"
CR: "\r"
```

**Why:** every later production spells its terminals in these classes, so they are named once against the decoded byte stream of TXT-01 / TXT-02, and the ASCII/Unicode split of LEX-02 has a single place where it is drawn.

---

### LEXG-02 — 2. Line terminators and whitespace

**Id:** LEXG-02

**Kind:** rule

**Name:** 2. Line terminators and whitespace

**Question:** Which line-ending sequences does the lexer accept, and which terminals does it emit for a line break, for added and reduced indentation, and for inline whitespace that downstream grammar files consume?

**Parent:** LANG-05

**Norm:**

```lark
// LEX-07: \n or \r\n; a lone CR is SYN001.  The lexer normalises CRLF
// to LF before anything else, so downstream rules see only the
// _NEWLINE below.

LINE_TERMINATOR: LF | CR LF

// _NEWLINE, _INDENT and _DEDENT are the physical tokens the lexer emits
// after normalisation.  Grammar files elsewhere use these three
// terminals, never LINE_TERMINATOR or TAB directly.  One _NEWLINE
// spans the line end together with any blank lines and comment-only
// lines after it, so a comment never breaks a sequence of lines; the
// indentation it ends with is measured by the indenter (LEX-01), which
// turns it into _INDENT and _DEDENT.

_NEWLINE: (LINE_TERMINATOR (TAB | SPACE)* | LINE_COMMENT | BLOCK_COMMENT INLINE_SPACE? /(?=\n|\/\/)/)+

%declare _INDENT _DEDENT

// The order of the events at a line end is fixed: the _NEWLINE of a
// line is emitted first, then one _INDENT if the next line's
// indentation rose by one tab, or one _DEDENT per tab it fell; at end
// of file a _NEWLINE closes an unterminated last line and one _DEDENT
// closes every block still open.  Between an opening "(" and its
// closing ")" no _NEWLINE, _INDENT or _DEDENT is emitted, whatever the
// lines inside look like (EXP-02): a parenthesis carries an expression
// across lines, and a "(" still open at end of file is SYN004.

// Inline whitespace inside a line — spaces or tabs — is not a token;
// it separates other tokens.  A tab in indentation position is
// structural, not whitespace: it is part of the _NEWLINE above.

INLINE_SPACE: (SPACE | TAB)+
%ignore INLINE_SPACE
```

**Why:** the lexer normalises CRLF to LF and emits NEWLINE, INDENT and DEDENT as physical tokens, so downstream grammar files name those three terminals and never LineTerminator or Tab; the order of the events and their suppression inside parentheses are fixed here because every block production and the multi-line expression depend on them, and inline whitespace separates tokens without being one, while a tab in indentation position is structural.

---

### LEXG-03 — 3. Comments

**Id:** LEXG-03

**Kind:** rule

**Name:** 3. Comments

**Question:** What productions shape a line comment and a block comment, and does a block comment nest within itself or end at the first close delimiter?

**Norm:**

```lark
// LEX-09: line comments run to the end of the line; block comments
// nest, tracking depth.  A block comment still open at EOF is SYN004.

LINE_COMMENT: "//" (/(?!\n)/ UNICODE_CHAR)*

// The text of a block comment is any character that does not open
// "/*" or close "*/" a comment.  A regular expression cannot count, so
// the nesting is written out level by level: BLOCK_COMMENT holds
// comments nested eight deep, each level holding the one below it.

BLOCK_COMMENT_TEXT: /(?!\/\*|\*\/)/ UNICODE_CHAR
BLOCK_COMMENT_1: "/*" BLOCK_COMMENT_TEXT* "*/"
BLOCK_COMMENT_2: "/*" (BLOCK_COMMENT_TEXT | BLOCK_COMMENT_1)* "*/"
BLOCK_COMMENT_3: "/*" (BLOCK_COMMENT_TEXT | BLOCK_COMMENT_2)* "*/"
BLOCK_COMMENT_4: "/*" (BLOCK_COMMENT_TEXT | BLOCK_COMMENT_3)* "*/"
BLOCK_COMMENT_5: "/*" (BLOCK_COMMENT_TEXT | BLOCK_COMMENT_4)* "*/"
BLOCK_COMMENT_6: "/*" (BLOCK_COMMENT_TEXT | BLOCK_COMMENT_5)* "*/"
BLOCK_COMMENT_7: "/*" (BLOCK_COMMENT_TEXT | BLOCK_COMMENT_6)* "*/"
BLOCK_COMMENT: "/*" (BLOCK_COMMENT_TEXT | BLOCK_COMMENT_7)* "*/"

%ignore LINE_COMMENT
%ignore BLOCK_COMMENT
```

**Why:** line comments end with the line and block comments nest by depth, so the productions state both shapes with the nesting written into BlockCommentBody, which is what lets a block comment still open at EOF be reported as SYN004.

---

### LEXG-04 — 4. Identifiers and keywords

**Id:** LEXG-04

**Kind:** rule

**Name:** 4. Identifiers and keywords

**Question:** What character shape may an identifier take, and which exact words are enumerated as hard keywords, contextual keywords, type keywords and reserved-but-unused words?

**Parent:** LANG-02

**Norm:**

```lark
// LEX-03: identifiers are ASCII, start with a letter, camelCase.  The
// camelCase convention is a naming rule (LEX-03 prose), not enforced
// by the grammar.

IDENTIFIER: ASCII_LETTER (ASCII_LETTER | ASCII_DIGIT | "_")*

// LEX-04: hard keywords — reserved everywhere.  An identifier position
// holding one of these is SYN002.  The keyword tables are the single
// source per LEX-04; RESERVED enumerates them for the parser, together
// with the reserved-but-unused words `for`, `from` and `unit`.
// RESERVED is offered to the lexer in every parser state, so a
// reserved word never passes for a name: where no rule admits it, the
// parser receives RESERVED and rejects it.  Each hard keyword a rule
// uses has a terminal of its own below, with a priority above
// RESERVED, so it wins in the states where a rule admits it.  `host`
// has none: no rule of the language uses it (the host interface
// belongs to the library-authoring grammar).

RESERVED.3: /(after|always|and|assert|background|base|before|block|break|can|case|class|compiletime|constant|constructor|continue|default|else|error|false|function|handles|host|if|import|in|intent|is|iterate|later|match|none|not|onError|or|print|public|reset|result|return|returns|spec|start|this|to|true|while|with|for|from|unit)(?![A-Za-z0-9_])/

_AFTER.4: /after(?![A-Za-z0-9_])/
_ALWAYS.4: /always(?![A-Za-z0-9_])/
_AND.4: /and(?![A-Za-z0-9_])/
_ASSERT.4: /assert(?![A-Za-z0-9_])/
_BACKGROUND.4: /background(?![A-Za-z0-9_])/
_BASE.4: /base(?![A-Za-z0-9_])/
_BEFORE.4: /before(?![A-Za-z0-9_])/
_BLOCK.4: /block(?![A-Za-z0-9_])/
_BREAK.4: /break(?![A-Za-z0-9_])/
_CAN.4: /can(?![A-Za-z0-9_])/
_CASE.4: /case(?![A-Za-z0-9_])/
_CLASS.4: /class(?![A-Za-z0-9_])/
_COMPILETIME.4: /compiletime(?![A-Za-z0-9_])/
_CONSTANT.4: /constant(?![A-Za-z0-9_])/
_CONSTRUCTOR.4: /constructor(?![A-Za-z0-9_])/
_CONTINUE.4: /continue(?![A-Za-z0-9_])/
_DEFAULT.4: /default(?![A-Za-z0-9_])/
_ELSE.4: /else(?![A-Za-z0-9_])/
// `error` is the raise keyword only where "(" follows it (ERHG-03).
_ERROR.4: /error(?=[ \t]*\()/
_FALSE.4: /false(?![A-Za-z0-9_])/
_FUNCTION.4: /function(?![A-Za-z0-9_])/
_HANDLES.4: /handles(?![A-Za-z0-9_])/
_IF.4: /if(?![A-Za-z0-9_])/
_IMPORT.4: /import(?![A-Za-z0-9_])/
_IN.4: /in(?![A-Za-z0-9_])/
_INTENT.4: /intent(?![A-Za-z0-9_])/
_IS.4: /is(?![A-Za-z0-9_])/
_ITERATE.4: /iterate(?![A-Za-z0-9_])/
_LATER.4: /later(?![A-Za-z0-9_])/
_MATCH.4: /match(?![A-Za-z0-9_])/
_NONE.4: /none(?![A-Za-z0-9_])/
_NOT.4: /not(?![A-Za-z0-9_])/
_ONERROR.4: /onError(?![A-Za-z0-9_])/
_OR.4: /or(?![A-Za-z0-9_])/
_PRINT.4: /print(?![A-Za-z0-9_])/
_PUBLIC.4: /public(?![A-Za-z0-9_])/
_RESET.4: /reset(?![A-Za-z0-9_])/
_RESULT.4: /result(?![A-Za-z0-9_])/
_RETURN.4: /return(?![A-Za-z0-9_])/
_RETURNS.4: /returns(?![A-Za-z0-9_])/
_SPEC.4: /spec(?![A-Za-z0-9_])/
_START.4: /start(?![A-Za-z0-9_])/
_THIS.4: /this(?![A-Za-z0-9_])/
_TO.4: /to(?![A-Za-z0-9_])/
_TRUE.4: /true(?![A-Za-z0-9_])/
_WHILE.4: /while(?![A-Za-z0-9_])/
_WITH.4: /with(?![A-Za-z0-9_])/

// LEX-04: contextual keywords — keywords only in the position a rule
// gives them, ordinary identifiers everywhere else.  A block-header
// word is specialised only when ":" follows it; `description` only
// when a string literal follows it and `input` only at the end of its
// line, the two positions FNCG-02 gives them; `guard`, `step` and
// `watch` only in the parser states whose rules admit them (the
// contextual lexer offers them in no other state).  This matches how
// Python, Rust, and Kotlin handle contextual keywords.  `build` and
// `test` are in the table but no rule of the language uses them yet,
// so they have no terminal: they are identifiers.

_COMPUTED.1: /computed(?=[ \t]*:)/
_DESCRIPTION.1: /description(?=[ \t]+")/
_FUNCTIONS.1: /functions(?=[ \t]*:)/
_GUARD.1: /guard(?![A-Za-z0-9_])/
_INPUT.1: /input(?=[ \t]*(\/\/|\n))/
_SOURCE.1: /source(?=[ \t]*:)/
_STATE.1: /state(?=[ \t]*:)/
_STEP.1: /step(?![A-Za-z0-9_])/
_TESTS.1: /tests(?=[ \t]*:)/
_WATCH.1: /watch(?![A-Za-z0-9_])/

// The type keywords.  A type keyword is never an identifier (LEX-04);
// `list` has its own terminal because only it takes the behavior
// suffixes of TYPG-03.

PRIMITIVE_TYPE.3: /(any|boolean|bytes|datetime|integer|number|string|void)(?![A-Za-z0-9_])/
LIST_NAME.3: /list(?![A-Za-z0-9_])/
GENERIC_NAME.3: /(matrix|pairs)(?![A-Za-z0-9_])/

type_keyword: PRIMITIVE_TYPE
            | LIST_NAME
            | GENERIC_NAME
```

**Why:** the parser needs the reserved words enumerated in one place: hard keywords excluded from identifier positions (SYN002), contextual keywords accepted as identifiers and specialised only when a ":" follows, type keywords, and the words held back unused; the camelCase convention stays prose.

---

### LEXG-05 — 5. Numeric literals

**Id:** LEXG-05

**Kind:** rule

**Name:** 5. Numeric literals

**Question:** What shapes may an integer and a number literal take — which bases, whether a sign or digit separators are part of the literal, which exponent form, and when does a `.` belong to the number rather than to member access?

**Norm:**

```lark
// LEX-06: numeric literals carry no sign.  Sign is unary minus,
// applied elsewhere.  Digits are ASCII.  No digit separators.

integer_literal: HEX_LITERAL              -> hex_literal
               | BINARY_LITERAL           -> binary_literal
               | OCTAL_LITERAL            -> octal_literal
               | DECIMAL_INTEGER_LITERAL  -> decimal_literal

HEX_LITERAL.2: "0x" HEX_DIGIT+
BINARY_LITERAL.2: "0b" BINARY_DIGIT+
OCTAL_LITERAL.2: "0o" OCTAL_DIGIT+
DECIMAL_INTEGER_LITERAL: ASCII_DIGIT+

NUMBER_LITERAL.2: DECIMAL_INTEGER_LITERAL "." DECIMAL_INTEGER_LITERAL EXPONENT?
                | "." DECIMAL_INTEGER_LITERAL EXPONENT?
                | DECIMAL_INTEGER_LITERAL EXPONENT

EXPONENT: ("e" | "E") ("+" | "-")? DECIMAL_INTEGER_LITERAL

// LEX-06 dot-boundary rule: a "." is part of a NUMBER_LITERAL only if
// an ASCII digit follows immediately.  The terminals above encode
// this: NUMBER_LITERAL requires a digit on both sides of the dot (or on
// the right side if the number leads with ".").  "3." is therefore not
// a NUMBER_LITERAL — it is the integer literal "3" followed by "." as
// the member-access operator.
```

**Why:** literals carry no sign and admit no separators, and the productions encode the dot-boundary rule of LEX-06 by requiring a digit after every ".", so "3." lexes as the integer followed by the member-access operator without a special case in the lexer.

---

### LEXG-06 — 6. String literals

**Id:** LEXG-06

**Kind:** rule

**Name:** 6. String literals

**Question:** Which delimiters, escape sequences, `\u` digit count and closing-delimiter margin rule define the single-line and the multi-line string literal, and is interpolation's body defined here?

**Norm:**

```lark
// LEX-06: single-line strings use ", multi-line strings use """.
// Escapes are recognised inside "; nothing is interpreted inside """.

string_literal: STRING_LITERAL
              | MULTI_LINE_STRING

// A "{" in a single-line string opens an interpolation (EXPG-03), so
// the single-line string of this terminal is the one without any: its
// characters are those of LEX-06 — anything but '"', "\" and LF — less
// "{", which is written \{.  A "}" is an ordinary character.

STRING_LITERAL: "\"" (STRING_CHARACTER | ESCAPE_SEQUENCE)* "\""

STRING_CHARACTER: /(?!["\\\n{])/ UNICODE_CHAR

ESCAPE_SEQUENCE: SIMPLE_ESCAPE | UNICODE_ESCAPE

SIMPLE_ESCAPE: "\\" /["\\ntr{}0]/

// LEX-06: exactly six hex digits.  Values in 00D800-00DFFF or above
// 10FFFF are SYN005: the digits this terminal admits already leave
// them out.
UNICODE_ESCAPE: "\\u" /00(?:[0-9A-Ca-cE-Fe-f][0-9A-Fa-f]{3}|[Dd][0-7][0-9A-Fa-f]{2})|0[1-9A-Fa-f][0-9A-Fa-f]{4}|10[0-9A-Fa-f]{4}/

// The opening """ is followed by its line terminator; the content runs,
// uninterpreted, to the first """ (which it therefore cannot contain).
// LEX-06: the close delimiter's indentation sets the margin removed
// from every content line; a content line indented less is SYN005 —
// a rule on the content the terminal delivers, not on its shape.
MULTI_LINE_STRING.1: "\"\"\"" INLINE_SPACE? LF /(?:(?!""")[\s\S])*/ "\"\"\""

// Interpolation inside a single-line string: {expr} evaluates expr.
// \{ and \} escape literal braces.  The interpolation rules live in
// 06-expressions.lark.md because their body is an expression.
```

**Why:** the two string forms differ in what is interpreted: escapes inside ", nothing inside """, whose close delimiter sets the margin; the productions fix the six-digit \u escape and the margin rule, and leave interpolation to 06-expressions.lark.md because its body is an expression.

---

### LEXG-07 — 7. Other literals

**Id:** LEXG-07

**Kind:** rule

**Name:** 7. Other literals

**Question:** What shapes do the bytes, boolean, none and list literals take — a matrix literal being a list literal of list literals — and which escapes does the bytes form admit or refuse?

**Norm:**

```lark
// Bytes literal — the compiler contract of Platform 14 §14.14.2.  The
// prefix "b" attaches with no intervening space; the payload is the
// UTF-8 bytes of the text with escapes applied.  String escapes are
// recognised plus \xNN for an arbitrary byte; there is no \u escape (a
// bytes value has no code points to name — encode the character in the
// text or spell its bytes with \xNN).  No multi-line form, and no
// interpolation: "{" is an ordinary character here.

bytes_literal: BYTES_LITERAL

BYTES_LITERAL.2: "b\"" (BYTES_CHARACTER | BYTES_ESCAPE)* "\""

BYTES_CHARACTER: /(?!["\\\n])/ UNICODE_CHAR

BYTES_ESCAPE: SIMPLE_ESCAPE | HEX_BYTE_ESCAPE

HEX_BYTE_ESCAPE: "\\x" HEX_DIGIT HEX_DIGIT

boolean_literal: _TRUE   -> true_literal
               | _FALSE  -> false_literal

none_literal: _NONE

// LEX-06 list and matrix literal shapes.  The element expressions are
// defined in 06-expressions.lark.md.  A matrix literal has no rule of
// its own: it is a list_literal whose elements are list_literals —
// [[1, 2], [3, 4]], and [[]] for an empty matrix — and whether a
// literal is a list of lists or a matrix is the type its context
// expects (4 — Type System), never a second reading of the same
// brackets.

list_literal: "[" (expression ("," expression)*)? "]"
```

**Why:** bytes, boolean, none, list and matrix literals share no shape with strings or numbers, so they stand together here; the bytes form is spelled out because it borrows string escapes, adds \xNN, and deliberately has no \u escape and no multi-line form.

---

### LEXG-08 — 8. Punctuation and operator tokens

**Id:** LEXG-08

**Kind:** rule

**Name:** 8. Punctuation and operator tokens

**Question:** Which punctuation and operator character sequences does the lexer emit as raw tokens, and is precedence or associativity fixed here or left to the expression grammar?

**Parent:** LANG-02

**Norm:**

```lark
// The punctuation tokens the lexer recognises.  Operators with semantic
// behaviour (precedence, associativity) are grouped in
// 06-expressions.lark.md; this section lists only the raw tokens.  The
// rules write each token as its quoted string, which resolves to the
// terminal named here.
//
// The symbols enumerated here are limited to structural delimiters the
// language commits to and to well-established mathematical operators.
// Any meaning a pronounceable English word carries is spelled with that
// word, not with punctuation (LANG-02); the three symbol tokens below
// that are not operators each mark a form the language commits to: the
// optional type (TYP-03), the non-none assertion (EXP-03) and the arrow
// return of a capability method signature (LEX-04).
//
// "{" and "}" are not tokens of their own: they open and close an
// interpolation, and belong to the string pieces of EXPG-03.

LPAR: "("
RPAR: ")"
LSQB: "["
RSQB: "]"
COMMA: ","
COLON: ":"
DOT: "."               // Member access; also the decimal point of LEXG-05
EQUAL: "="
OPTIONAL_MARKER: "?"   // Optional type marker T? — TYPG-01
BANG: "!"              // Postfix non-none assertion — EXPG-01
ARROW: "->"            // Arrow return of a capability method — 14

// Arithmetic and comparison operators — enumerated here so the lexer's
// token vocabulary is complete; precedence lives with the expression
// grammar.

PLUS: "+"
MINUS: "-"
STAR: "*"
SLASH: "/"
PERCENT: "%"
CIRCUMFLEX: "^"        // Exponentiation — level 4 in 06-expressions.lark.md, per EXP-01
EQUAL_EQUAL: "=="
NOT_EQUAL: "!="
LESS_THAN: "<"
LESS_EQUAL: "<="
GREATER_THAN: ">"
GREATER_EQUAL: ">="
```

**Why:** the lexer's token vocabulary must be complete before any parser consumes it, so every punctuation and operator token is enumerated as a raw terminal here, while precedence and associativity stay with the expression grammar that gives the operators their meaning.

---

### LEXG-09 — 9. Case sensitivity note

**Id:** LEXG-09

**Kind:** rule

**Name:** 9. Case sensitivity note

**Question:** Is every token of this grammar matched by its exact character sequence, or may the lexer fold case when matching a keyword?

**Norm:**

```lark
// LEX-08: every token above is matched by its exact character
// sequence.  No case folding at any point.  `if` matches only "if", not
// "If" or "IF"; the latter two are ordinary IDENTIFIER tokens.
```

**Why:** a token is matched by its exact character sequence with no case folding anywhere, so `If` and `IF` are ordinary identifiers; stating this once over every token above forecloses a lexer that quietly folds case for keywords.

---
