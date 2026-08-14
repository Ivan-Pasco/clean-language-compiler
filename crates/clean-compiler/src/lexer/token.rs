//! Token vocabulary, mirroring `03-lexical-structure.ebnf.md` §4–§8.

use crate::source::{ByteSpan, LineMap};

/// Hard keywords (LEX-04) — reserved everywhere. Contextual keywords
/// (`functions`, `tests`, …) and type keywords (`integer`, `string`, …)
/// lex as `Ident`; the parser specialises them by position, per the
/// grammar's §4 note.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kw {
    After,
    Always,
    And,
    Assert,
    Background,
    Base,
    Before,
    Block,
    Break,
    Can,
    Class,
    Compiletime,
    Constant,
    Constructor,
    Continue,
    Default,
    Else,
    Error,
    False,
    Function,
    Handles,
    Host,
    If,
    Import,
    In,
    Intent,
    Is,
    Iterate,
    Later,
    None,
    Not,
    OnError,
    Or,
    Print,
    Public,
    Reset,
    Result,
    Return,
    Returns,
    Spec,
    Start,
    This,
    To,
    True,
    While,
    With,
}

impl Kw {
    pub fn from_word(word: &str) -> Option<Kw> {
        Some(match word {
            "after" => Kw::After,
            "always" => Kw::Always,
            "and" => Kw::And,
            "assert" => Kw::Assert,
            "background" => Kw::Background,
            "base" => Kw::Base,
            "before" => Kw::Before,
            "block" => Kw::Block,
            "break" => Kw::Break,
            "can" => Kw::Can,
            "class" => Kw::Class,
            "compiletime" => Kw::Compiletime,
            "constant" => Kw::Constant,
            "constructor" => Kw::Constructor,
            "continue" => Kw::Continue,
            "default" => Kw::Default,
            "else" => Kw::Else,
            "error" => Kw::Error,
            "false" => Kw::False,
            "function" => Kw::Function,
            "handles" => Kw::Handles,
            "host" => Kw::Host,
            "if" => Kw::If,
            "import" => Kw::Import,
            "in" => Kw::In,
            "intent" => Kw::Intent,
            "is" => Kw::Is,
            "iterate" => Kw::Iterate,
            "later" => Kw::Later,
            "none" => Kw::None,
            "not" => Kw::Not,
            "onError" => Kw::OnError,
            "or" => Kw::Or,
            "print" => Kw::Print,
            "public" => Kw::Public,
            "reset" => Kw::Reset,
            "result" => Kw::Result,
            "return" => Kw::Return,
            "returns" => Kw::Returns,
            "spec" => Kw::Spec,
            "start" => Kw::Start,
            "this" => Kw::This,
            "to" => Kw::To,
            "true" => Kw::True,
            "while" => Kw::While,
            "with" => Kw::With,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Ident(String),
    Keyword(Kw),
    /// Integer literal in any base (LEX-06); value already decoded.
    Int(u128),
    /// Float-shaped literal (`NumberLiteral`); raw text kept — the `number`
    /// type is outside the Milestone 1 surface and rejects downstream.
    Number(String),
    /// Single-line string, escapes decoded. `interpolations` carries the
    /// spans of `{…}` segments; non-empty means the string interpolates
    /// (outside the Milestone 1 surface — rejected downstream, not here).
    Str {
        value: String,
        interpolations: Vec<ByteSpan>,
    },
    // Punctuation (§8).
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    Colon,
    Dot,
    Assign,
    Question,
    Bang,
    Arrow,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Caret,
    Eq,
    NEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    // Layout (§2).
    Newline,
    Indent,
    Dedent,
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: ByteSpan,
}

/// Output of pass [2] for one source file.
pub struct TokenStream {
    pub path: String,
    pub content: String,
    pub tokens: Vec<Token>,
    /// Comment spans, in source order — preserved for the LSP (§14.4.2[2]).
    pub comments: Vec<ByteSpan>,
    pub line_map: LineMap,
}

impl TokenStream {
    /// Diagnostic span for a byte span of this file.
    pub fn diag_span(&self, span: ByteSpan) -> clean_compiler_types::Span {
        self.line_map.span(&self.content, &self.path, span)
    }
}
