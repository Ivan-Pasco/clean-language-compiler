//! ISO/IEC 14977 EBNF reader for the vendored grammar files
//! (`tests/fixtures/grammar/*.ebnf.md`, copies of foundation
//! `04 language/grammar/` — DOC-15's source of truth for syntax).
//!
//! Reads exactly the repo's notation per the grammar README: `=` defines,
//! `,` concatenates, `|` alternates, `[x]` optional, `{x}` repetition,
//! `(x)` grouping, `"lit"`/`'lit'` terminals, `? description ?` special
//! sequences, `(* comments *)` (nesting), `;` optional rule terminator,
//! and the ISO exception operator `-` (chained in this repo:
//! `UnicodeChar - '"' - "\" - LF`).

// Shared by several test binaries; not every binary uses every item.
#![allow(dead_code)]

use indexmap::IndexMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Concatenation: `a, b, c`.
    Seq(Vec<Expr>),
    /// Alternation: `a | b`.
    Alt(Vec<Expr>),
    /// `[ x ]` — zero or one.
    Opt(Box<Expr>),
    /// `{ x }` — zero or more.
    Rep(Box<Expr>),
    /// `"literal"` or `'literal'`.
    Terminal(String),
    /// `ProductionName` reference.
    NonTerm(String),
    /// `? informal description ?`.
    Special(String),
    /// `base - exception - exception…`.
    Except(Box<Expr>, Vec<Expr>),
}

pub struct Rule {
    pub expr: Expr,
    /// File the definition came from, for duplicate reporting.
    pub file: String,
}

pub struct Grammar {
    pub rules: IndexMap<String, Rule>,
    /// (name, first file, shadowed file) — DOC-15 calls a production
    /// defined in more than one place a defect; recorded, not resolved.
    pub duplicates: Vec<(String, String, String)>,
}

impl Grammar {
    /// Loads every ` ```ebnf ` fenced block from the files, in the given
    /// order. A rule whose whole body is one `? see … ?` special is a
    /// cross-reference stub, skipped so the real definition wins wherever
    /// it loads. Otherwise the first definition wins and later ones are
    /// recorded in `duplicates`.
    pub fn load(files: &[(String, String)]) -> Grammar {
        let mut grammar = Grammar {
            rules: IndexMap::new(),
            duplicates: Vec::new(),
        };
        for (file, markdown) in files {
            for block in ebnf_blocks(markdown) {
                for (name, expr) in parse_block(&block, file) {
                    if matches!(&expr, Expr::Special(s) if s.starts_with("see ")) {
                        continue; // cross-reference stub
                    }
                    if let Some(existing) = grammar.rules.get(&name) {
                        grammar
                            .duplicates
                            .push((name, existing.file.clone(), file.clone()));
                    } else {
                        grammar.rules.insert(
                            name,
                            Rule {
                                expr,
                                file: file.clone(),
                            },
                        );
                    }
                }
            }
        }
        grammar
    }
}

/// Extracts the contents of every ```` ```ebnf ```` fence.
fn ebnf_blocks(markdown: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current: Option<String> = None;
    for line in markdown.lines() {
        let trimmed = line.trim_end();
        match &mut current {
            None if trimmed == "```ebnf" => current = Some(String::new()),
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
    assert!(
        current.is_none(),
        "unterminated ```ebnf fence in a vendored grammar file"
    );
    blocks
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Ident(String),
    Terminal(String),
    Special(String),
    Define, // =
    Comma,  // ,
    Bar,    // |
    Semi,   // ;
    Minus,  // -
    LBrack, // [
    RBrack, // ]
    LBrace, // {
    RBrace, // }
    LParen, // (
    RParen, // )
}

fn tokenize(block: &str, file: &str) -> Vec<Tok> {
    let chars: Vec<char> = block.chars().collect();
    let mut toks = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        // Nesting comments, strict ISO 14977: a `*)` in comment prose ends
        // the comment (the 19-ai-integration case was fixed by foundation's
        // 2026-08-20 erratum), so a stray one fails loudly here.
        if c == '(' && chars.get(i + 1) == Some(&'*') {
            let mut depth = 1;
            i += 2;
            while i < chars.len() && depth > 0 {
                if chars[i] == '(' && chars.get(i + 1) == Some(&'*') {
                    depth += 1;
                    i += 2;
                } else if chars[i] == '*' && chars.get(i + 1) == Some(&')') {
                    depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
            assert!(depth == 0, "{file}: unterminated (* comment *)");
            continue;
        }
        if c == '"' || c == '\'' {
            let quote = c;
            let start = i + 1;
            let mut j = start;
            while j < chars.len() && chars[j] != quote {
                j += 1;
            }
            assert!(j < chars.len(), "{file}: unterminated terminal literal");
            toks.push(Tok::Terminal(chars[start..j].iter().collect()));
            i = j + 1;
            continue;
        }
        if c == '?' {
            let start = i + 1;
            let mut j = start;
            while j < chars.len() && chars[j] != '?' {
                j += 1;
            }
            assert!(j < chars.len(), "{file}: unterminated ? special sequence ?");
            let text: String = chars[start..j].iter().collect();
            toks.push(Tok::Special(
                text.split_whitespace().collect::<Vec<_>>().join(" "),
            ));
            i = j + 1;
            continue;
        }
        if c.is_ascii_alphabetic() {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric()) {
                i += 1;
            }
            toks.push(Tok::Ident(chars[start..i].iter().collect()));
            continue;
        }
        let tok = match c {
            '=' => Tok::Define,
            ',' => Tok::Comma,
            '|' => Tok::Bar,
            ';' => Tok::Semi,
            '-' => Tok::Minus,
            '[' => Tok::LBrack,
            ']' => Tok::RBrack,
            '{' => Tok::LBrace,
            '}' => Tok::RBrace,
            '(' => Tok::LParen,
            ')' => Tok::RParen,
            other => panic!("{file}: unexpected character {other:?} in EBNF block"),
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

/// Parses one fenced block into `(name, body)` rules. The `;` terminator is
/// optional in this repo, so a rule also ends where `Ident =` begins the
/// next one.
fn parse_block(block: &str, file: &str) -> Vec<(String, Expr)> {
    let toks = tokenize(block, file);
    let mut p = Parser {
        toks: &toks,
        pos: 0,
        file,
    };
    let mut rules = Vec::new();
    while p.pos < p.toks.len() {
        let name = match p.next() {
            Tok::Ident(name) => name,
            other => panic!("{file}: expected production name, found {other:?}"),
        };
        assert_eq!(p.next(), Tok::Define, "{file}: expected '=' after {name}");
        let expr = p.alternation();
        if p.peek() == Some(&Tok::Semi) {
            p.pos += 1;
        }
        rules.push((name, expr));
    }
    rules
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }

    fn next(&mut self) -> Tok {
        let tok = self.toks[self.pos].clone();
        self.pos += 1;
        tok
    }

    /// True when the upcoming tokens start the next rule (`Ident =`).
    fn at_rule_boundary(&self) -> bool {
        matches!(
            (self.toks.get(self.pos), self.toks.get(self.pos + 1)),
            (Some(Tok::Ident(_)), Some(Tok::Define))
        )
    }

    fn alternation(&mut self) -> Expr {
        let mut alts = vec![self.sequence()];
        while self.peek() == Some(&Tok::Bar) {
            self.pos += 1;
            alts.push(self.sequence());
        }
        if alts.len() == 1 {
            alts.pop().expect("one alternative")
        } else {
            Expr::Alt(alts)
        }
    }

    // Strict ISO 14977: concatenation is explicit `,` only — juxtaposition
    // is a parse error again (the 18-async case was fixed by foundation's
    // 2026-08-20 erratum), so `primary()` fails loudly on a stray one.
    fn sequence(&mut self) -> Expr {
        let mut items = vec![self.term()];
        while self.peek() == Some(&Tok::Comma) {
            self.pos += 1;
            items.push(self.term());
        }
        if items.len() == 1 {
            items.pop().expect("one item")
        } else {
            Expr::Seq(items)
        }
    }

    fn term(&mut self) -> Expr {
        let base = self.primary();
        let mut exceptions = Vec::new();
        while self.peek() == Some(&Tok::Minus) {
            self.pos += 1;
            exceptions.push(self.primary());
        }
        if exceptions.is_empty() {
            base
        } else {
            Expr::Except(Box::new(base), exceptions)
        }
    }

    fn primary(&mut self) -> Expr {
        if self.at_rule_boundary() {
            panic!(
                "{}: dangling operator runs into the next rule at token {}",
                self.file, self.pos
            );
        }
        match self.next() {
            Tok::Ident(name) => Expr::NonTerm(name),
            Tok::Terminal(text) => Expr::Terminal(text),
            Tok::Special(text) => Expr::Special(text),
            Tok::LBrack => {
                let inner = self.alternation();
                assert_eq!(self.next(), Tok::RBrack, "{}: expected ']'", self.file);
                Expr::Opt(Box::new(inner))
            }
            Tok::LBrace => {
                let inner = self.alternation();
                assert_eq!(self.next(), Tok::RBrace, "{}: expected '}}'", self.file);
                Expr::Rep(Box::new(inner))
            }
            Tok::LParen => {
                let inner = self.alternation();
                assert_eq!(self.next(), Tok::RParen, "{}: expected ')'", self.file);
                inner
            }
            other => panic!("{}: unexpected token {other:?} in expression", self.file),
        }
    }
}
