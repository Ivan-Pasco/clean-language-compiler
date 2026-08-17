//! The scanner: bytes → tokens, one file at a time.
//!
//! Layout discipline (LEX-01/LEX-07): CRLF normalises to LF; indentation is
//! tabs only — spaces there are SYN003, a jump of more than one level is
//! SYN006. Blank and comment-only lines produce no layout tokens. Errors
//! recover: the offending character is skipped and scanning continues, so
//! one file reports every lexical finding in a single run (§14.4).

use clean_compiler_types::{codes, Diagnostic, Level};

use crate::diag::{render_cli, DiagnosticSink};
use crate::source::{ByteSpan, LineMap};

use super::token::{Kw, StrPart, Token, TokenKind, TokenStream};

pub fn lex(path: &str, content: &str, sink: &mut DiagnosticSink) -> TokenStream {
    let mut scanner = Scanner {
        path,
        content,
        bytes: content.as_bytes(),
        pos: 0,
        tokens: Vec::new(),
        comments: Vec::new(),
        line_map: LineMap::new(content),
        indent: 0,
        brackets: 0,
    };
    scanner.run(sink);
    TokenStream {
        path: path.to_string(),
        content: content.to_string(),
        tokens: scanner.tokens,
        comments: scanner.comments,
        line_map: scanner.line_map,
    }
}

struct Scanner<'src> {
    path: &'src str,
    content: &'src str,
    bytes: &'src [u8],
    pos: usize,
    tokens: Vec<Token>,
    comments: Vec<ByteSpan>,
    line_map: LineMap,
    indent: u32,
    /// Open `(`/`[` pairs; line ends are transparent while positive.
    brackets: u32,
}

impl<'src> Scanner<'src> {
    fn run(&mut self, sink: &mut DiagnosticSink) {
        while self.pos < self.bytes.len() {
            self.scan_line(sink);
        }
        // Close the file: dangling indentation unwinds, then EOF.
        let end = ByteSpan::new(self.bytes.len() as u32, self.bytes.len() as u32);
        if !matches!(
            self.tokens.last().map(|t| &t.kind),
            Some(TokenKind::Newline) | None
        ) {
            self.tokens.push(Token {
                kind: TokenKind::Newline,
                span: end,
            });
        }
        for _ in 0..self.indent {
            self.tokens.push(Token {
                kind: TokenKind::Dedent,
                span: end,
            });
        }
        self.tokens.push(Token {
            kind: TokenKind::Eof,
            span: end,
        });
    }

    /// Scans one physical line: indentation, then tokens, then the line end.
    fn scan_line(&mut self, sink: &mut DiagnosticSink) {
        let indent_start = self.pos;
        let mut tabs: u32 = 0;
        let mut saw_space = false;
        while let Some(&b) = self.bytes.get(self.pos) {
            match b {
                b'\t' => {
                    tabs += 1;
                    self.pos += 1;
                }
                b' ' => {
                    saw_space = true;
                    self.pos += 1;
                }
                _ => break,
            }
        }
        let indent_span = ByteSpan::new(indent_start as u32, self.pos as u32);

        // Blank or comment-only lines leave the layout untouched.
        if self.line_is_effectively_empty(sink) {
            return;
        }

        if saw_space {
            self.error(
                sink,
                codes::SYN003,
                "indentation uses spaces; Clean indents with tabs only".to_string(),
                indent_span,
            );
        }

        match tabs.cmp(&self.indent) {
            std::cmp::Ordering::Greater => {
                if tabs > self.indent + 1 {
                    self.error(
                        sink,
                        codes::SYN006,
                        format!(
                            "indentation jumps from level {} to level {}; blocks nest one tab at a time",
                            self.indent, tabs
                        ),
                        indent_span,
                    );
                }
                for _ in self.indent..tabs {
                    self.tokens.push(Token {
                        kind: TokenKind::Indent,
                        span: indent_span,
                    });
                }
            }
            std::cmp::Ordering::Less => {
                for _ in tabs..self.indent {
                    self.tokens.push(Token {
                        kind: TokenKind::Dedent,
                        span: indent_span,
                    });
                }
            }
            std::cmp::Ordering::Equal => {}
        }
        self.indent = tabs;

        // Token loop until the physical line ends. Inside an open bracket
        // pair, line ends are transparent (EXP-02 multi-line parenthesized
        // form): no NEWLINE is emitted and the next line's indentation is
        // plain whitespace, so the layout stays balanced.
        while let Some(&b) = self.bytes.get(self.pos) {
            match b {
                b'\n' => {
                    if self.brackets > 0 {
                        self.pos += 1;
                        continue;
                    }
                    self.push_char_token(TokenKind::Newline);
                    return;
                }
                b'\r' => {
                    if self.bytes.get(self.pos + 1) == Some(&b'\n') {
                        if self.brackets > 0 {
                            self.pos += 2;
                            continue;
                        }
                        let span = ByteSpan::new(self.pos as u32, self.pos as u32 + 2);
                        self.pos += 2;
                        self.tokens.push(Token {
                            kind: TokenKind::Newline,
                            span,
                        });
                        return;
                    }
                    self.error(
                        sink,
                        codes::SYN001,
                        "lone carriage return; line endings are \\n or \\r\\n".to_string(),
                        ByteSpan::new(self.pos as u32, self.pos as u32 + 1),
                    );
                    self.pos += 1;
                }
                b' ' | b'\t' => self.pos += 1,
                b'/' if self.bytes.get(self.pos + 1) == Some(&b'/') => {
                    self.line_comment();
                }
                b'/' if self.bytes.get(self.pos + 1) == Some(&b'*') => {
                    self.block_comment(sink);
                }
                b'"' => self.string(sink),
                b'0'..=b'9' => self.number(sink),
                b'.' if matches!(self.bytes.get(self.pos + 1), Some(b'0'..=b'9')) => {
                    self.number(sink)
                }
                b'A'..=b'Z' | b'a'..=b'z' => self.word(sink),
                _ => self.punctuation(sink),
            }
        }
    }

    /// From the current position (after indentation), is the rest of this
    /// physical line only whitespace and/or comments? Consumes it if so.
    fn line_is_effectively_empty(&mut self, sink: &mut DiagnosticSink) -> bool {
        let mut probe = self.pos;
        loop {
            match self.bytes.get(probe) {
                None => {
                    self.pos = probe;
                    return true;
                }
                Some(b'\n') => {
                    self.pos = probe + 1;
                    return true;
                }
                Some(b'\r') if self.bytes.get(probe + 1) == Some(&b'\n') => {
                    self.pos = probe + 2;
                    return true;
                }
                Some(b' ') | Some(b'\t') => probe += 1,
                Some(b'/') if self.bytes.get(probe + 1) == Some(&b'/') => {
                    self.pos = probe;
                    self.line_comment();
                    probe = self.pos;
                }
                Some(b'/') if self.bytes.get(probe + 1) == Some(&b'*') => {
                    self.pos = probe;
                    // A block comment may span lines; if content follows its
                    // close on the same line, the line is not empty.
                    self.block_comment(sink);
                    probe = self.pos;
                }
                _ => return false,
            }
        }
    }

    fn line_comment(&mut self) {
        let start = self.pos;
        while let Some(&b) = self.bytes.get(self.pos) {
            if b == b'\n' || (b == b'\r' && self.bytes.get(self.pos + 1) == Some(&b'\n')) {
                break;
            }
            self.pos += 1;
        }
        self.comments
            .push(ByteSpan::new(start as u32, self.pos as u32));
    }

    /// LEX-09: block comments nest, tracking depth; still open at EOF is
    /// SYN004.
    fn block_comment(&mut self, sink: &mut DiagnosticSink) {
        let start = self.pos;
        self.pos += 2;
        let mut depth = 1u32;
        while depth > 0 {
            match (self.bytes.get(self.pos), self.bytes.get(self.pos + 1)) {
                (Some(b'/'), Some(b'*')) => {
                    depth += 1;
                    self.pos += 2;
                }
                (Some(b'*'), Some(b'/')) => {
                    depth -= 1;
                    self.pos += 2;
                }
                (Some(_), _) => self.pos += 1,
                (None, _) => {
                    self.error(
                        sink,
                        codes::SYN004,
                        "block comment is still open at end of file".to_string(),
                        ByteSpan::new(start as u32, self.pos as u32),
                    );
                    break;
                }
            }
        }
        self.comments
            .push(ByteSpan::new(start as u32, self.pos as u32));
    }

    fn word(&mut self, sink: &mut DiagnosticSink) {
        let start = self.pos;
        while let Some(&b) = self.bytes.get(self.pos) {
            if b.is_ascii_alphanumeric() || b == b'_' {
                self.pos += 1;
            } else {
                break;
            }
        }
        let text = &self.content[start..self.pos];
        let span = ByteSpan::new(start as u32, self.pos as u32);
        // LEX-04 ReservedUnused: `for`/`from`/`unit` have no meaning yet, so
        // any use is use-as-identifier — SYN002. The token survives as an
        // identifier so parsing continues.
        if matches!(text, "for" | "from" | "unit") {
            self.error(
                sink,
                codes::SYN002,
                format!("'{text}' is reserved for a future language version and cannot be used as an identifier"),
                span,
            );
        }
        let kind = match Kw::from_word(text) {
            Some(kw) => TokenKind::Keyword(kw),
            None => TokenKind::Ident(text.to_string()),
        };
        self.tokens.push(Token { kind, span });
    }

    fn number(&mut self, sink: &mut DiagnosticSink) {
        let start = self.pos;
        let radix_parse = |scanner: &mut Self, radix: u32, digits: fn(u8) -> bool| {
            scanner.pos += 2;
            let digit_start = scanner.pos;
            while let Some(&b) = scanner.bytes.get(scanner.pos) {
                if digits(b) {
                    scanner.pos += 1;
                } else {
                    break;
                }
            }
            (digit_start, radix)
        };

        if self.bytes.get(self.pos) == Some(&b'0')
            && matches!(self.bytes.get(self.pos + 1), Some(b'x' | b'b' | b'o'))
        {
            let (digit_start, radix) = match self.bytes[self.pos + 1] {
                b'x' => radix_parse(self, 16, |b| b.is_ascii_hexdigit()),
                b'b' => radix_parse(self, 2, |b| b == b'0' || b == b'1'),
                _ => radix_parse(self, 8, |b| (b'0'..=b'7').contains(&b)),
            };
            let span = ByteSpan::new(start as u32, self.pos as u32);
            let digits = &self.content[digit_start..self.pos];
            match u128::from_str_radix(digits, radix) {
                Ok(value) if !digits.is_empty() => self.tokens.push(Token {
                    kind: TokenKind::Int(value),
                    span,
                }),
                _ => self.error(
                    sink,
                    codes::SYN005,
                    format!(
                        "malformed numeric literal '{}'",
                        &self.content[start..self.pos]
                    ),
                    span,
                ),
            }
            return;
        }

        // Decimal: integer part, optional fraction (dot-boundary rule:
        // "." belongs to the number only when a digit follows), optional
        // exponent. Any fraction or exponent makes it a NumberLiteral.
        let mut is_number = false;
        while matches!(self.bytes.get(self.pos), Some(b'0'..=b'9')) {
            self.pos += 1;
        }
        if self.bytes.get(self.pos) == Some(&b'.')
            && matches!(self.bytes.get(self.pos + 1), Some(b'0'..=b'9'))
        {
            is_number = true;
            self.pos += 1;
            while matches!(self.bytes.get(self.pos), Some(b'0'..=b'9')) {
                self.pos += 1;
            }
        }
        if matches!(self.bytes.get(self.pos), Some(b'e' | b'E')) {
            let mut probe = self.pos + 1;
            if matches!(self.bytes.get(probe), Some(b'+' | b'-')) {
                probe += 1;
            }
            if matches!(self.bytes.get(probe), Some(b'0'..=b'9')) {
                is_number = true;
                self.pos = probe;
                while matches!(self.bytes.get(self.pos), Some(b'0'..=b'9')) {
                    self.pos += 1;
                }
            }
        }

        let span = ByteSpan::new(start as u32, self.pos as u32);
        let text = &self.content[start..self.pos];
        if is_number {
            self.tokens.push(Token {
                kind: TokenKind::Number(text.to_string()),
                span,
            });
        } else {
            match text.parse::<u128>() {
                Ok(value) => self.tokens.push(Token {
                    kind: TokenKind::Int(value),
                    span,
                }),
                Err(_) => self.error(
                    sink,
                    codes::SYN005,
                    format!("integer literal '{text}' does not fit any integer width"),
                    span,
                ),
            }
        }
    }

    fn string(&mut self, sink: &mut DiagnosticSink) {
        if self.content[self.pos..].starts_with("\"\"\"") {
            self.multiline_string(sink);
            return;
        }
        let start = self.pos;
        self.pos += 1;
        let mut parts: Vec<StrPart> = Vec::new();
        let mut text = String::new();
        loop {
            match self.bytes.get(self.pos) {
                None | Some(b'\n') => {
                    self.error(
                        sink,
                        codes::SYN004,
                        "string literal is not closed before the end of the line".to_string(),
                        ByteSpan::new(start as u32, self.pos as u32),
                    );
                    break;
                }
                Some(b'\r') if self.bytes.get(self.pos + 1) == Some(&b'\n') => {
                    self.error(
                        sink,
                        codes::SYN004,
                        "string literal is not closed before the end of the line".to_string(),
                        ByteSpan::new(start as u32, self.pos as u32),
                    );
                    break;
                }
                Some(b'"') => {
                    self.pos += 1;
                    break;
                }
                Some(b'\\') => self.escape(sink, &mut text),
                Some(b'{') => {
                    // Interpolation: the interior is a full Expression
                    // (06-expressions §3), tokenized here so the parser
                    // receives real tokens with file-accurate spans.
                    if !text.is_empty() {
                        parts.push(StrPart::Text(std::mem::take(&mut text)));
                    }
                    let (span, tokens, closed) = self.interpolation(sink);
                    parts.push(StrPart::Interp { span, tokens });
                    if !closed {
                        // The line (or file) ended inside `{…}`; the string
                        // itself is unterminated too, reported once here.
                        self.error(
                            sink,
                            codes::SYN004,
                            "string literal is not closed before the end of the line".to_string(),
                            ByteSpan::new(start as u32, self.pos as u32),
                        );
                        break;
                    }
                }
                Some(_) => {
                    let ch = self.content[self.pos..].chars().next().expect("in bounds");
                    text.push(ch);
                    self.pos += ch.len_utf8();
                }
            }
        }
        if !text.is_empty() {
            parts.push(StrPart::Text(text));
        }
        self.tokens.push(Token {
            kind: TokenKind::Str { parts },
            span: ByteSpan::new(start as u32, self.pos as u32),
        });
    }

    /// Scans one `{expr}` interpolation from its opening brace, tokenizing
    /// the interior with the ordinary token rules (nested strings included).
    /// Returns the full `{…}` span, the interior tokens (Eof-terminated),
    /// and whether the closing `}` was found before the line ended.
    fn interpolation(&mut self, sink: &mut DiagnosticSink) -> (ByteSpan, Vec<Token>, bool) {
        let start = self.pos;
        self.pos += 1; // '{'
        let saved = std::mem::take(&mut self.tokens);
        let mut depth = 0u32;
        let mut closed = false;
        while let Some(&b) = self.bytes.get(self.pos) {
            match b {
                b'}' if depth == 0 => {
                    self.pos += 1;
                    closed = true;
                    break;
                }
                b'\n' => break,
                b'\r' if self.bytes.get(self.pos + 1) == Some(&b'\n') => break,
                b' ' | b'\t' => self.pos += 1,
                b'{' => {
                    depth += 1;
                    self.punctuation(sink);
                }
                b'}' => {
                    depth -= 1;
                    self.punctuation(sink);
                }
                b'"' => self.string(sink),
                b'0'..=b'9' => self.number(sink),
                b'.' if matches!(self.bytes.get(self.pos + 1), Some(b'0'..=b'9')) => {
                    self.number(sink)
                }
                b'A'..=b'Z' | b'a'..=b'z' => self.word(sink),
                _ => self.punctuation(sink),
            }
        }
        let end = self.pos;
        let mut tokens = std::mem::replace(&mut self.tokens, saved);
        tokens.push(Token {
            kind: TokenKind::Eof,
            span: ByteSpan::new(end as u32, end as u32),
        });
        (ByteSpan::new(start as u32, end as u32), tokens, closed)
    }

    fn escape(&mut self, sink: &mut DiagnosticSink, value: &mut String) {
        let esc_start = self.pos;
        self.pos += 1;
        match self.bytes.get(self.pos) {
            Some(b'"') => {
                value.push('"');
                self.pos += 1;
            }
            Some(b'\\') => {
                value.push('\\');
                self.pos += 1;
            }
            Some(b'n') => {
                value.push('\n');
                self.pos += 1;
            }
            Some(b't') => {
                value.push('\t');
                self.pos += 1;
            }
            Some(b'r') => {
                value.push('\r');
                self.pos += 1;
            }
            Some(b'{') => {
                value.push('{');
                self.pos += 1;
            }
            Some(b'}') => {
                value.push('}');
                self.pos += 1;
            }
            Some(b'0') => {
                value.push('\0');
                self.pos += 1;
            }
            Some(b'u') => {
                self.pos += 1;
                let hex_start = self.pos;
                for _ in 0..6 {
                    if matches!(self.bytes.get(self.pos), Some(b) if b.is_ascii_hexdigit()) {
                        self.pos += 1;
                    }
                }
                let hex = &self.content[hex_start..self.pos];
                let scalar = (hex.len() == 6)
                    .then(|| u32::from_str_radix(hex, 16).ok())
                    .flatten()
                    .filter(|v| *v <= 0x10FFFF && !(0xD800..=0xDFFF).contains(v))
                    .and_then(char::from_u32);
                match scalar {
                    Some(ch) => value.push(ch),
                    None => self.error(
                        sink,
                        codes::SYN005,
                        "\\u escape requires exactly six hex digits naming a Unicode scalar value"
                            .to_string(),
                        ByteSpan::new(esc_start as u32, self.pos as u32),
                    ),
                }
            }
            _ => {
                self.error(
                    sink,
                    codes::SYN005,
                    "unknown escape sequence in string literal".to_string(),
                    ByteSpan::new(
                        esc_start as u32,
                        (self.pos + 1).min(self.bytes.len()) as u32,
                    ),
                );
                self.pos = (self.pos + 1).min(self.bytes.len());
            }
        }
    }

    /// LEX-06 multi-line strings: `"""` opens, a line whose content is only
    /// `"""` closes; the close delimiter's indentation is the margin removed
    /// from every content line; nothing inside is interpreted.
    fn multiline_string(&mut self, sink: &mut DiagnosticSink) {
        let start = self.pos;
        self.pos += 3;
        // Consume to end of the opening line.
        while let Some(&b) = self.bytes.get(self.pos) {
            self.pos += 1;
            if b == b'\n' {
                break;
            }
        }
        let mut lines: Vec<&str> = Vec::new();
        let close_margin;
        loop {
            if self.pos >= self.bytes.len() {
                self.error(
                    sink,
                    codes::SYN004,
                    "multi-line string is still open at end of file".to_string(),
                    ByteSpan::new(start as u32, self.pos as u32),
                );
                close_margin = "";
                break;
            }
            let line_start = self.pos;
            while let Some(&b) = self.bytes.get(self.pos) {
                if b == b'\n' {
                    break;
                }
                self.pos += 1;
            }
            let line = &self.content[line_start..self.pos];
            if self.pos < self.bytes.len() {
                self.pos += 1; // consume '\n'
            }
            let trimmed = line.trim_start_matches(['\t', ' ']);
            if trimmed == "\"\"\"" {
                close_margin = &line[..line.len() - 3];
                break;
            }
            lines.push(line);
        }
        let mut value = String::new();
        for (i, line) in lines.iter().enumerate() {
            let content = match line.strip_prefix(close_margin) {
                Some(rest) => rest,
                None => {
                    self.error(
                        sink,
                        codes::SYN005,
                        "multi-line string content is indented less than its closing delimiter"
                            .to_string(),
                        ByteSpan::new(start as u32, self.pos as u32),
                    );
                    line
                }
            };
            if i > 0 {
                value.push('\n');
            }
            value.push_str(content);
        }
        self.tokens.push(Token {
            kind: TokenKind::Str {
                parts: vec![StrPart::Text(value)],
            },
            span: ByteSpan::new(start as u32, self.pos as u32),
        });
    }

    fn punctuation(&mut self, sink: &mut DiagnosticSink) {
        use TokenKind::*;
        let two = |s: &Scanner| s.bytes.get(s.pos + 1).copied();
        let (kind, len) = match self.bytes[self.pos] {
            b'-' if two(self) == Some(b'>') => (Arrow, 2),
            b'=' if two(self) == Some(b'=') => (Eq, 2),
            b'!' if two(self) == Some(b'=') => (NEq, 2),
            b'<' if two(self) == Some(b'=') => (LtEq, 2),
            b'>' if two(self) == Some(b'=') => (GtEq, 2),
            b'(' => (LParen, 1),
            b')' => (RParen, 1),
            b'[' => (LBracket, 1),
            b']' => (RBracket, 1),
            b'{' => (LBrace, 1),
            b'}' => (RBrace, 1),
            b',' => (Comma, 1),
            b':' => (Colon, 1),
            b'.' => (Dot, 1),
            b'=' => (Assign, 1),
            b'?' => (Question, 1),
            b'!' => (Bang, 1),
            b'+' => (Plus, 1),
            b'-' => (Minus, 1),
            b'*' => (Star, 1),
            b'/' => (Slash, 1),
            b'%' => (Percent, 1),
            b'^' => (Caret, 1),
            b'<' => (Lt, 1),
            b'>' => (Gt, 1),
            _ => {
                let ch = self.content[self.pos..].chars().next().expect("in bounds");
                self.error(
                    sink,
                    codes::SYN001,
                    format!("character '{}' is not valid in any lexical context", ch),
                    ByteSpan::new(self.pos as u32, (self.pos + ch.len_utf8()) as u32),
                );
                self.pos += ch.len_utf8();
                return;
            }
        };
        match kind {
            LParen | LBracket => self.brackets += 1,
            RParen | RBracket => self.brackets = self.brackets.saturating_sub(1),
            _ => {}
        }
        let span = ByteSpan::new(self.pos as u32, (self.pos + len) as u32);
        self.pos += len;
        self.tokens.push(Token { kind, span });
    }

    fn push_char_token(&mut self, kind: TokenKind) {
        let span = ByteSpan::new(self.pos as u32, self.pos as u32 + 1);
        self.pos += 1;
        self.tokens.push(Token { kind, span });
    }

    fn error(&self, sink: &mut DiagnosticSink, code: &str, message: String, span: ByteSpan) {
        let mut diagnostic = Diagnostic {
            level: Level::Error,
            code: code.to_string(),
            message,
            primary_span: self.line_map.span(self.content, self.path, span),
            primary_label: None,
            secondary: Vec::new(),
            notes: Vec::new(),
            helps: Vec::new(),
            suggestions: Vec::new(),
            doc_url: Diagnostic::doc_url_for(code),
            rendered: String::new(),
        };
        diagnostic.rendered = render_cli(&diagnostic, &crate::diag::SourceCache::empty());
        sink.push(diagnostic);
    }
}
