//! Reader for the Lark grammar the law writes (FS-01, foundation
//! `governance/product/language-principles/*.lark.md`, vendored byte for
//! byte under `tests/fixtures/grammar/`).
//!
//! Reads the subset of Lark those companions use, from the ```` ```lark ````
//! fences of each file in filename order — the order FS-01 composes them
//! in:
//!
//! - definitions `name: …` (rules) and `NAME: …` (terminals), with the
//!   `?` / `!` rule prefixes and `.n` priorities; continuation lines are
//!   indented or start with `|`;
//! - alternatives `|`, groups `( … )`, optional `[ … ]` and `x?`,
//!   repetition `x*` / `x+`, string literals `"…"` (optional `i` flag),
//!   regular expressions `/…/flags`, aliases `-> name` (dropped: they
//!   name tree nodes, not syntax);
//! - `%declare`, `%ignore`, `%import` (recorded) and `%extend` (appends
//!   alternatives to an earlier definition);
//! - `//` comments, whole-line or trailing.
//!
//! Anything outside that subset (`~` repetition ranges, `..` character
//! ranges, other directives) fails loudly: the reader grows with the
//! grammar deliberately, never by guessing.

// Shared by several test binaries; not every binary uses every item.
#![allow(dead_code)]

use indexmap::IndexMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Juxtaposition: `a b c` (possibly empty: an empty alternative).
    Seq(Vec<Expr>),
    /// Alternation: `a | b`.
    Alt(Vec<Expr>),
    /// `[ x ]` or `x?` — zero or one.
    Opt(Box<Expr>),
    /// `x*` — zero or more.
    Star(Box<Expr>),
    /// `x+` — one or more.
    Plus(Box<Expr>),
    /// `"literal"`, unescaped.
    Literal(String),
    /// `/pattern/flags` — the pattern text as written (flags dropped).
    Regex(String),
    /// A rule or terminal reference.
    Name(String),
}

/// Lark's naming convention: a terminal's name is upper case once its
/// leading `_` (inlined / filtered) is stripped; a rule's is lower case.
pub fn is_terminal_name(name: &str) -> bool {
    name.trim_start_matches('_')
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_uppercase())
}

pub struct Definition {
    pub expr: Expr,
    /// File the definition came from, for duplicate reporting.
    pub file: String,
    /// The `.n` priority (0 when absent).
    pub priority: i32,
}

pub struct Grammar {
    /// Rules (lower-case names), in definition order.
    pub rules: IndexMap<String, Definition>,
    /// Terminals (upper-case names), in definition order.
    pub terminals: IndexMap<String, Definition>,
    /// `%declare`d terminals: emitted by the postlex indenter, never by
    /// the lexer (`_INDENT`, `_DEDENT`).
    pub declared: Vec<String>,
    /// `%ignore`d terminals.
    pub ignored: Vec<String>,
    /// `%import` lines, verbatim (the language grammar has none; a
    /// library companion's would name the `clean` module).
    pub imports: Vec<String>,
    /// (name, first file, redefining file) — Lark refuses a name defined
    /// twice, and DOC-15 calls it a defect; recorded, not resolved.
    pub duplicates: Vec<(String, String, String)>,
}

impl Grammar {
    /// Loads every ```` ```lark ```` fence of the files, in the given
    /// order, into one grammar.
    pub fn load(files: &[(String, String)]) -> Grammar {
        let mut grammar = Grammar {
            rules: IndexMap::new(),
            terminals: IndexMap::new(),
            declared: Vec::new(),
            ignored: Vec::new(),
            imports: Vec::new(),
            duplicates: Vec::new(),
        };
        for (file, markdown) in files {
            for block in lark_blocks(markdown, file) {
                for statement in statements(&block, file) {
                    grammar.apply(&statement, file);
                }
            }
        }
        grammar
    }

    pub fn definition(&self, name: &str) -> Option<&Definition> {
        if is_terminal_name(name) {
            self.terminals.get(name)
        } else {
            self.rules.get(name)
        }
    }

    /// Names referenced somewhere but defined nowhere (nor declared).
    /// Lark refuses such a grammar; the reader reports it.
    pub fn undefined(&self) -> Vec<String> {
        let mut missing = Vec::new();
        let defs = self.rules.values().chain(self.terminals.values());
        for def in defs {
            collect_names(&def.expr, &mut |name| {
                if self.definition(name).is_none()
                    && !self.declared.iter().any(|d| d == name)
                    && !missing.iter().any(|m| m == name)
                {
                    missing.push(name.to_string());
                }
            });
        }
        for name in &self.ignored {
            if self.definition(name).is_none() && !missing.contains(name) {
                missing.push(name.clone());
            }
        }
        missing
    }

    fn apply(&mut self, statement: &str, file: &str) {
        let toks = tokenize(statement, file);
        let mut p = Parser {
            toks: &toks,
            pos: 0,
            file,
        };
        if let Some(Tok::Directive(directive)) = p.peek().cloned() {
            p.pos += 1;
            match directive.as_str() {
                "declare" => {
                    while let Some(Tok::Name(name)) = p.peek().cloned() {
                        p.pos += 1;
                        self.declared.push(name);
                    }
                }
                "ignore" => match p.next() {
                    Tok::Name(name) => self.ignored.push(name),
                    other => panic!("{file}: %ignore expects a terminal name, found {other:?}"),
                },
                "import" => {
                    self.imports.push(statement.trim().to_string());
                    p.pos = toks.len();
                }
                "extend" => {
                    let name = match p.next() {
                        Tok::Name(name) => name,
                        other => panic!("{file}: %extend expects a name, found {other:?}"),
                    };
                    assert_eq!(
                        p.next(),
                        Tok::Colon,
                        "{file}: expected ':' after %extend {name}"
                    );
                    let extra = p.expansions();
                    let target = if is_terminal_name(&name) {
                        self.terminals.get_mut(&name)
                    } else {
                        self.rules.get_mut(&name)
                    }
                    .unwrap_or_else(|| panic!("{file}: %extend of undefined {name}"));
                    let mut alts = match std::mem::replace(&mut target.expr, Expr::Seq(vec![])) {
                        Expr::Alt(alts) => alts,
                        single => vec![single],
                    };
                    match extra {
                        Expr::Alt(more) => alts.extend(more),
                        single => alts.push(single),
                    }
                    target.expr = Expr::Alt(alts);
                }
                other => panic!("{file}: unsupported directive %{other}"),
            }
            assert!(
                p.pos == toks.len(),
                "{file}: trailing tokens after directive %{directive}: {:?}",
                &toks[p.pos..]
            );
            return;
        }
        // `?name` inlines a single-child node, `!name` keeps its tokens:
        // both shape the tree only, so both are read and dropped.
        if matches!(p.peek(), Some(Tok::Question) | Some(Tok::Bang)) {
            p.pos += 1;
        }
        let name = match p.next() {
            Tok::Name(name) => name,
            other => panic!("{file}: expected a definition name, found {other:?}"),
        };
        let mut priority = 0;
        if p.peek() == Some(&Tok::Dot) {
            p.pos += 1;
            priority = match p.next() {
                Tok::Number(n) => n,
                other => panic!("{file}: expected a priority after {name}., found {other:?}"),
            };
        }
        assert_eq!(p.next(), Tok::Colon, "{file}: expected ':' after {name}");
        let expr = p.expansions();
        assert!(
            p.pos == toks.len(),
            "{file}: unparsed tokens at the end of {name}: {:?}",
            &toks[p.pos..]
        );
        let table = if is_terminal_name(&name) {
            &mut self.terminals
        } else {
            &mut self.rules
        };
        if let Some(existing) = table.get(&name) {
            self.duplicates
                .push((name, existing.file.clone(), file.to_string()));
        } else {
            table.insert(
                name,
                Definition {
                    expr,
                    file: file.to_string(),
                    priority,
                },
            );
        }
    }
}

/// Calls `f` on every name the expression references.
pub fn collect_names(expr: &Expr, f: &mut impl FnMut(&str)) {
    match expr {
        Expr::Seq(items) | Expr::Alt(items) => {
            for item in items {
                collect_names(item, f);
            }
        }
        Expr::Opt(inner) | Expr::Star(inner) | Expr::Plus(inner) => collect_names(inner, f),
        Expr::Name(name) => f(name),
        Expr::Literal(_) | Expr::Regex(_) => {}
    }
}

/// True when the expression holds a regular expression directly (not
/// through a referenced terminal).
pub fn has_regex(expr: &Expr) -> bool {
    match expr {
        Expr::Seq(items) | Expr::Alt(items) => items.iter().any(has_regex),
        Expr::Opt(inner) | Expr::Star(inner) | Expr::Plus(inner) => has_regex(inner),
        Expr::Regex(_) => true,
        Expr::Literal(_) | Expr::Name(_) => false,
    }
}

/// Extracts the contents of every ```` ```lark ```` fence (the fence line
/// may be indented, as the composer allows).
fn lark_blocks(markdown: &str, file: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current: Option<String> = None;
    for line in markdown.lines() {
        let trimmed = line.trim();
        match &mut current {
            None if trimmed == "```lark" => current = Some(String::new()),
            None => {}
            Some(buf) => {
                if trimmed == "```" {
                    blocks.push(current.take().expect("fence open"));
                } else {
                    buf.push_str(line);
                    buf.push('\n');
                }
            }
        }
    }
    assert!(current.is_none(), "{file}: unterminated ```lark fence");
    blocks
}

/// Removes the `//` comment of one line, if any: `//` outside a string
/// literal or a regular expression starts one. Outside those, a single
/// `/` always opens a regular expression (Lark has no other use for it).
fn strip_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'/' if bytes.get(i + 1) == Some(&b'/') => return &line[..i],
            quote @ (b'"' | b'/') => {
                i += 1;
                while i < bytes.len() && bytes[i] != quote {
                    if bytes[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
                i += 1;
            }
            _ => i += 1,
        }
    }
    line
}

/// Splits a fenced block into its statements: a line that starts in
/// column 0 with anything but `|` opens a statement; an indented line, or
/// one starting with `|`, continues the open one.
fn statements(block: &str, file: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for raw in block.lines() {
        let line = strip_comment(raw).trim_end();
        if line.trim().is_empty() {
            continue;
        }
        let continues = line.starts_with(char::is_whitespace) || line.starts_with('|');
        if continues {
            let open = out
                .last_mut()
                .unwrap_or_else(|| panic!("{file}: continuation line with no definition: {raw}"));
            open.push('\n');
            open.push_str(line);
        } else {
            out.push(line.to_string());
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Name(String),
    Directive(String),
    Literal(String),
    Regex(String),
    Number(i32),
    Colon,
    Bar,
    LParen,
    RParen,
    LBrack,
    RBrack,
    Question,
    Star,
    Plus,
    Bang,
    Dot,
    Comma,
    Arrow,
}

/// Unescapes a Lark string literal body the way Python reads it: `\n`,
/// `\t`, `\r`, `\\` and `\"` are single characters; any other escape
/// keeps its backslash.
fn unescape(body: &str) -> String {
    let mut out = String::new();
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('\\') => out.push('\\'),
            Some('"') => out.push('"'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

fn tokenize(statement: &str, file: &str) -> Vec<Tok> {
    let chars: Vec<char> = statement.chars().collect();
    let mut toks = Vec::new();
    let mut i = 0;
    let word = |i: &mut usize| -> String {
        let start = *i;
        while *i < chars.len() && (chars[*i].is_ascii_alphanumeric() || chars[*i] == '_') {
            *i += 1;
        }
        chars[start..*i].iter().collect()
    };
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == '"' || c == '/' {
            let start = i + 1;
            let mut j = start;
            while j < chars.len() && chars[j] != c {
                if chars[j] == '\\' {
                    j += 1;
                }
                j += 1;
            }
            assert!(
                j < chars.len(),
                "{file}: unterminated {c}…{c} in `{statement}`"
            );
            let body: String = chars[start..j].iter().collect();
            i = j + 1;
            // Flags: `i` on a string; `imslux` on a regex.
            while i < chars.len() && chars[i].is_ascii_lowercase() {
                i += 1;
            }
            toks.push(if c == '"' {
                Tok::Literal(unescape(&body))
            } else {
                Tok::Regex(body)
            });
            continue;
        }
        if c.is_ascii_alphabetic() || c == '_' {
            toks.push(Tok::Name(word(&mut i)));
            continue;
        }
        if c == '%' {
            i += 1;
            toks.push(Tok::Directive(word(&mut i)));
            continue;
        }
        if c.is_ascii_digit() || (c == '-' && chars.get(i + 1).is_some_and(char::is_ascii_digit)) {
            let start = i;
            i += 1;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            toks.push(Tok::Number(text.parse().expect("priority is an integer")));
            continue;
        }
        if c == '-' && chars.get(i + 1) == Some(&'>') {
            toks.push(Tok::Arrow);
            i += 2;
            continue;
        }
        if c == '.' && chars.get(i + 1) == Some(&'.') {
            panic!("{file}: `..` character ranges are outside the reader's Lark subset");
        }
        let tok = match c {
            ':' => Tok::Colon,
            '|' => Tok::Bar,
            '(' => Tok::LParen,
            ')' => Tok::RParen,
            '[' => Tok::LBrack,
            ']' => Tok::RBrack,
            '?' => Tok::Question,
            '*' => Tok::Star,
            '+' => Tok::Plus,
            '!' => Tok::Bang,
            '.' => Tok::Dot,
            ',' => Tok::Comma,
            other => panic!(
                "{file}: character {other:?} is outside the reader's Lark subset in `{statement}`"
            ),
        };
        toks.push(tok);
        i += 1;
    }
    toks
}

struct Parser<'a> {
    toks: &'a [Tok],
    pos: usize,
    file: &'a str,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }

    fn next(&mut self) -> Tok {
        let tok = self
            .toks
            .get(self.pos)
            .cloned()
            .unwrap_or_else(|| panic!("{}: definition ends early", self.file));
        self.pos += 1;
        tok
    }

    /// `expansions: alias ("|" alias)*`
    fn expansions(&mut self) -> Expr {
        let mut alts = vec![self.alias()];
        while self.peek() == Some(&Tok::Bar) {
            self.pos += 1;
            alts.push(self.alias());
        }
        if alts.len() == 1 {
            alts.pop().expect("one alternative")
        } else {
            Expr::Alt(alts)
        }
    }

    /// `alias: expansion ("->" NAME)?` — the alias names a tree node and
    /// is dropped.
    fn alias(&mut self) -> Expr {
        let expr = self.expansion();
        if self.peek() == Some(&Tok::Arrow) {
            self.pos += 1;
            match self.next() {
                Tok::Name(_) => {}
                other => panic!("{}: expected an alias name, found {other:?}", self.file),
            }
        }
        expr
    }

    /// `expansion: expr*`
    fn expansion(&mut self) -> Expr {
        let mut items = Vec::new();
        while let Some(tok) = self.peek() {
            if matches!(tok, Tok::Bar | Tok::RParen | Tok::RBrack | Tok::Arrow) {
                break;
            }
            items.push(self.expr());
        }
        if items.len() == 1 {
            items.pop().expect("one item")
        } else {
            Expr::Seq(items)
        }
    }

    /// `expr: atom ("?" | "*" | "+")?`
    fn expr(&mut self) -> Expr {
        let atom = self.atom();
        match self.peek() {
            Some(Tok::Question) => {
                self.pos += 1;
                Expr::Opt(Box::new(atom))
            }
            Some(Tok::Star) => {
                self.pos += 1;
                Expr::Star(Box::new(atom))
            }
            Some(Tok::Plus) => {
                self.pos += 1;
                Expr::Plus(Box::new(atom))
            }
            _ => atom,
        }
    }

    fn atom(&mut self) -> Expr {
        match self.next() {
            Tok::Name(name) => Expr::Name(name),
            Tok::Literal(text) => Expr::Literal(text),
            Tok::Regex(pattern) => Expr::Regex(pattern),
            Tok::LParen => {
                let inner = self.expansions();
                assert_eq!(self.next(), Tok::RParen, "{}: expected ')'", self.file);
                inner
            }
            Tok::LBrack => {
                let inner = self.expansions();
                assert_eq!(self.next(), Tok::RBrack, "{}: expected ']'", self.file);
                Expr::Opt(Box::new(inner))
            }
            other => panic!("{}: unexpected token {other:?} in an expansion", self.file),
        }
    }
}
