//! Recursive-descent parser (ADR-0006) for the Milestone 1 surface:
//! `host interface` blocks (LBS-02), `functions:` blocks, `class` field
//! declarations, `start:`, statements (declaration, assignment, return,
//! expression, `if`/`else if`/`else`, `print:`), and the EXP-01 precedence
//! ladder. Error-recovering: a bad statement reports SYN and re-syncs at
//! the next line so one run reports every finding (§14.4.2[3]).

use clean_compiler_types::{codes, Diagnostic, Level};

use crate::diag::{render_cli, DiagnosticSink};
use crate::lexer::{Kw, Token, TokenKind, TokenStream};
use crate::source::ByteSpan;

use super::ast::*;

pub fn parse(stream: &TokenStream, sink: &mut DiagnosticSink) -> SourceFile {
    let mut parser = Parser {
        stream,
        pos: 0,
        paren_depth: 0,
    };
    parser.source_file(sink)
}

struct Parser<'a> {
    stream: &'a TokenStream,
    pos: usize,
    paren_depth: u32,
}

impl<'a> Parser<'a> {
    // ----- cursor -----------------------------------------------------

    fn peek(&self) -> &'a TokenKind {
        &self.stream.tokens[self.effective_pos()].kind
    }

    fn peek2(&self) -> &'a TokenKind {
        let last = self.stream.tokens.len() - 1;
        let mut i = (self.effective_pos() + 1).min(last);
        if self.paren_depth > 0 {
            while i < last
                && matches!(
                    self.stream.tokens[i].kind,
                    TokenKind::Newline | TokenKind::Indent | TokenKind::Dedent
                )
            {
                i += 1;
            }
        }
        &self.stream.tokens[i].kind
    }

    fn effective_pos(&self) -> usize {
        let mut i = self.pos;
        if self.paren_depth > 0 {
            // EXP-02: inside parentheses, line breaks do not end the
            // expression — layout tokens are transparent.
            while matches!(
                self.stream.tokens[i].kind,
                TokenKind::Newline | TokenKind::Indent | TokenKind::Dedent
            ) {
                i += 1;
            }
        }
        i
    }

    fn span(&self) -> ByteSpan {
        self.stream.tokens[self.effective_pos()].span
    }

    fn prev_span(&self) -> ByteSpan {
        self.stream.tokens[self.pos.saturating_sub(1)].span
    }

    /// Advances past the current token — except at `Eof`, which is sticky so
    /// error recovery can never run off the end of the stream.
    fn bump(&mut self) -> &'a Token {
        let i = self.effective_pos();
        let token = &self.stream.tokens[i];
        if !matches!(token.kind, TokenKind::Eof) {
            self.pos = i + 1;
        }
        token
    }

    fn at(&self, kind: &TokenKind) -> bool {
        self.peek() == kind
    }

    fn at_kw(&self, kw: Kw) -> bool {
        matches!(self.peek(), TokenKind::Keyword(k) if *k == kw)
    }

    fn eat(&mut self, kind: &TokenKind) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn eat_kw(&mut self, kw: Kw) -> bool {
        if self.at_kw(kw) {
            self.bump();
            true
        } else {
            false
        }
    }

    /// Contextual keywords lex as identifiers; this matches one by text.
    fn at_word(&self, word: &str) -> bool {
        matches!(self.peek(), TokenKind::Ident(name) if name == word)
    }

    fn eat_word(&mut self, word: &str) -> bool {
        if self.at_word(word) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: &TokenKind, what: &str, sink: &mut DiagnosticSink) -> bool {
        if self.eat(kind) {
            true
        } else {
            self.error_here(sink, format!("expected {what}"));
            false
        }
    }

    fn ident(&mut self, what: &str, sink: &mut DiagnosticSink) -> Option<(String, ByteSpan)> {
        match self.peek() {
            TokenKind::Ident(name) => {
                let name = name.clone();
                let span = self.span();
                self.bump();
                Some((name, span))
            }
            TokenKind::Keyword(_) => {
                // LEX-04: a hard keyword in identifier position is SYN002.
                self.error_here(sink, format!("keyword cannot be used as {what}"));
                None
            }
            _ => {
                self.error_here(sink, format!("expected {what}"));
                None
            }
        }
    }

    fn string_literal(&mut self, what: &str, sink: &mut DiagnosticSink) -> Option<String> {
        match self.peek() {
            TokenKind::Str { value, .. } => {
                let value = value.clone();
                self.bump();
                Some(value)
            }
            _ => {
                self.error_here(sink, format!("expected {what}"));
                None
            }
        }
    }

    // ----- diagnostics and recovery -----------------------------------

    fn error_here(&self, sink: &mut DiagnosticSink, message: String) {
        self.error_at(sink, codes::SYN002, message, self.span());
    }

    fn error_at(&self, sink: &mut DiagnosticSink, code: &str, message: String, span: ByteSpan) {
        let mut diagnostic = Diagnostic {
            level: Level::Error,
            code: code.to_string(),
            message,
            primary_span: self.stream.diag_span(span),
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

    /// Skip to just after the next NEWLINE at the current nesting depth,
    /// consuming any indented sub-block that follows a broken line.
    fn sync_line(&mut self) {
        let mut depth = 0i32;
        loop {
            match self.peek() {
                TokenKind::Eof => return,
                TokenKind::Indent => {
                    depth += 1;
                    self.bump();
                }
                TokenKind::Dedent => {
                    if depth == 0 {
                        return;
                    }
                    depth -= 1;
                    self.bump();
                }
                TokenKind::Newline if depth == 0 => {
                    self.bump();
                    if self.at(&TokenKind::Indent) {
                        // The broken line opened a block: swallow it whole.
                        self.bump();
                        depth += 1;
                        continue;
                    }
                    return;
                }
                _ => {
                    self.bump();
                }
            }
        }
    }

    // ----- top level ---------------------------------------------------

    fn source_file(&mut self, sink: &mut DiagnosticSink) -> SourceFile {
        let mut items = Vec::new();
        loop {
            match self.peek() {
                TokenKind::Eof => break,
                TokenKind::Newline | TokenKind::Indent | TokenKind::Dedent => {
                    // Stray layout (e.g. after recovery) is consumed, never
                    // looped on.
                    self.bump();
                }
                TokenKind::Keyword(Kw::Host) => {
                    if let Some(item) = self.host_interface(sink) {
                        items.push(Item::HostInterface(item));
                    }
                }
                TokenKind::Ident(name) if name == "functions" => {
                    if let Some(functions) = self.functions_block(sink) {
                        items.push(Item::Functions(functions));
                    }
                }
                TokenKind::Keyword(Kw::Class) => {
                    if let Some(class) = self.class_decl(sink) {
                        items.push(Item::Class(class));
                    }
                }
                TokenKind::Keyword(Kw::Start) => {
                    if let Some(block) = self.start_section(sink) {
                        items.push(Item::Start(block));
                    }
                }
                _ => {
                    // SYN009 — not a permitted top-level form.
                    let span = self.span();
                    let construct = self.describe_current();
                    self.error_at(
                        sink,
                        codes::SYN009,
                        format!("'{construct}' cannot appear at the top level of a file"),
                        span,
                    );
                    self.sync_line();
                }
            }
        }
        SourceFile {
            path: self.stream.path.clone(),
            items,
        }
    }

    fn describe_current(&self) -> String {
        match self.peek() {
            TokenKind::Ident(name) => name.clone(),
            TokenKind::Keyword(kw) => format!("{kw:?}").to_lowercase(),
            TokenKind::Int(v) => v.to_string(),
            other => format!("{other:?}"),
        }
    }

    /// `host interface <kebab> version "<x.y.z>":` … (LBS-02).
    fn host_interface(&mut self, sink: &mut DiagnosticSink) -> Option<HostInterface> {
        let start = self.span();
        self.bump(); // host
        if !self.eat_word("interface") {
            self.error_here(sink, "expected 'interface' after 'host'".to_string());
            self.sync_line();
            return None;
        }
        let name = self.kebab_name(sink)?;
        if !self.eat_word("version") {
            self.error_here(
                sink,
                "expected 'version' in host interface header".to_string(),
            );
        }
        let version = self
            .string_literal("version string", sink)
            .unwrap_or_default();
        self.expect(&TokenKind::Colon, "':'", sink);
        self.expect(&TokenKind::Newline, "end of line", sink);
        if !self.expect(&TokenKind::Indent, "indented host interface body", sink) {
            return None;
        }

        let mut worlds = Vec::new();
        // `requires host worlds ["server", …]`
        if self.eat_word("requires") {
            let ok = self.eat_kw(Kw::Host) && self.eat_word("worlds");
            if !ok {
                self.error_here(sink, "expected 'host worlds' after 'requires'".to_string());
            }
            self.expect(&TokenKind::LBracket, "'['", sink);
            while let Some(world) = self.string_literal("world name", sink) {
                worlds.push(world);
                if !self.eat(&TokenKind::Comma) {
                    break;
                }
            }
            self.expect(&TokenKind::RBracket, "']'", sink);
            self.expect(&TokenKind::Newline, "end of line", sink);
        }

        let mut functions = Vec::new();
        loop {
            match self.peek() {
                TokenKind::Newline => {
                    self.bump();
                }
                TokenKind::Keyword(Kw::Host) => {
                    if let Some(f) = self.host_function(sink) {
                        functions.push(f);
                    }
                }
                TokenKind::Dedent | TokenKind::Eof => break,
                _ => {
                    self.error_here(
                        sink,
                        "expected a 'host function' declaration in host interface body".to_string(),
                    );
                    self.sync_line();
                }
            }
        }
        self.eat(&TokenKind::Dedent);
        Some(HostInterface {
            name,
            version,
            worlds,
            functions,
            span: start.merge(self.prev_span()),
        })
    }

    /// `host function name(p: type, …) [returns type]` + indented
    /// `description "…"` (LBS-02: description is mandatory).
    fn host_function(&mut self, sink: &mut DiagnosticSink) -> Option<HostFunction> {
        let start = self.span();
        self.bump(); // host
        if !self.eat_kw(Kw::Function) {
            self.error_here(sink, "expected 'function' after 'host'".to_string());
            self.sync_line();
            return None;
        }
        let (name, _) = self.ident("host function name", sink)?;
        self.expect(&TokenKind::LParen, "'('", sink);
        self.paren_depth += 1;
        let mut params = Vec::new();
        if !self.at(&TokenKind::RParen) {
            loop {
                let param_start = self.span();
                let Some((param_name, _)) = self.ident("parameter name", sink) else {
                    break;
                };
                self.expect(
                    &TokenKind::Colon,
                    "':' between parameter name and type",
                    sink,
                );
                let ty = self.type_expr(true, sink);
                params.push(HostParam {
                    name: param_name,
                    ty,
                    span: param_start.merge(self.prev_span()),
                });
                if !self.eat(&TokenKind::Comma) {
                    break;
                }
            }
        }
        self.paren_depth -= 1;
        self.expect(&TokenKind::RParen, "')'", sink);
        let ret = if self.eat_kw(Kw::Returns) {
            Some(self.type_expr(true, sink))
        } else {
            None
        };
        self.expect(&TokenKind::Newline, "end of line", sink);

        let mut description = String::new();
        if self.eat(&TokenKind::Indent) {
            if self.eat_word("description") {
                description = self
                    .string_literal("description string", sink)
                    .unwrap_or_default();
                self.expect(&TokenKind::Newline, "end of line", sink);
            } else {
                self.error_here(
                    sink,
                    "host function body admits only a 'description' line".to_string(),
                );
                self.sync_line();
            }
            while self.eat(&TokenKind::Newline) {}
            self.eat(&TokenKind::Dedent);
        }
        if description.is_empty() {
            self.error_at(
                sink,
                codes::SYN005,
                format!("host function '{name}' is missing its mandatory description"),
                start,
            );
        }
        Some(HostFunction {
            name,
            params,
            ret,
            description,
            span: start.merge(self.prev_span()),
        })
    }

    /// Interface names are lowercase-kebab; the lexer splits them at `-`,
    /// so this re-joins contiguous `ident - ident` runs.
    fn kebab_name(&mut self, sink: &mut DiagnosticSink) -> Option<String> {
        let (mut name, mut last_span) = self.ident("interface name", sink)?;
        while self.at(&TokenKind::Minus) {
            let minus_span = self.span();
            if minus_span.start != last_span.end {
                break;
            }
            self.bump();
            let Some((part, part_span)) = self.ident("interface name part", sink) else {
                break;
            };
            if part_span.start != minus_span.end {
                self.error_at(
                    sink,
                    codes::SYN005,
                    "kebab-case name parts must be contiguous".to_string(),
                    part_span,
                );
            }
            name.push('-');
            name.push_str(&part);
            last_span = part_span;
        }
        Some(name)
    }

    /// `functions:` block containing FNC-02 type-first declarations.
    fn functions_block(&mut self, sink: &mut DiagnosticSink) -> Option<Vec<Function>> {
        self.bump(); // functions
        self.expect(&TokenKind::Colon, "':'", sink);
        self.expect(&TokenKind::Newline, "end of line", sink);
        if !self.expect(&TokenKind::Indent, "indented functions body", sink) {
            return None;
        }
        let mut functions = Vec::new();
        loop {
            match self.peek() {
                TokenKind::Newline => {
                    self.bump();
                }
                TokenKind::Dedent | TokenKind::Eof => break,
                _ => {
                    if let Some(f) = self.function_decl(sink) {
                        functions.push(f);
                    }
                }
            }
        }
        self.eat(&TokenKind::Dedent);
        Some(functions)
    }

    /// `ReturnType name(params)` + indented body (FNC-02/FNC-03).
    fn function_decl(&mut self, sink: &mut DiagnosticSink) -> Option<Function> {
        let start = self.span();
        let ret = self.type_expr(false, sink);
        let Some((name, _)) = self.ident("function name", sink) else {
            self.sync_line();
            return None;
        };
        if !self.expect(&TokenKind::LParen, "'(' after function name", sink) {
            self.sync_line();
            return None;
        }
        self.paren_depth += 1;
        let mut params = Vec::new();
        if !self.at(&TokenKind::RParen) {
            loop {
                let param_start = self.span();
                let ty = self.type_expr(false, sink);
                let Some((param_name, _)) = self.ident("parameter name", sink) else {
                    break;
                };
                let default = if self.eat(&TokenKind::Assign) {
                    Some(self.expression(sink))
                } else {
                    None
                };
                params.push(Param {
                    ty,
                    name: param_name,
                    default,
                    span: param_start.merge(self.prev_span()),
                });
                if !self.eat(&TokenKind::Comma) {
                    break;
                }
            }
        }
        self.paren_depth -= 1;
        self.expect(&TokenKind::RParen, "')'", sink);
        self.expect(&TokenKind::Newline, "end of line", sink);
        let body = self.indented_block(sink);
        Some(Function {
            ret,
            name,
            params,
            body,
            span: start.merge(self.prev_span()),
        })
    }

    /// `class Name` + indented field declarations (M1 record subset).
    fn class_decl(&mut self, sink: &mut DiagnosticSink) -> Option<ClassDecl> {
        let start = self.span();
        self.bump(); // class
        let (name, _) = self.ident("class name", sink)?;
        self.expect(&TokenKind::Newline, "end of line", sink);
        if !self.expect(&TokenKind::Indent, "indented class body", sink) {
            return None;
        }
        let mut fields = Vec::new();
        loop {
            match self.peek() {
                TokenKind::Newline => {
                    self.bump();
                }
                TokenKind::Dedent | TokenKind::Eof => break,
                _ => {
                    let field_start = self.span();
                    let ty = self.type_expr(false, sink);
                    match self.ident("field name", sink) {
                        Some((field_name, _)) => {
                            fields.push(Field {
                                ty,
                                name: field_name,
                                span: field_start.merge(self.prev_span()),
                            });
                            self.expect(&TokenKind::Newline, "end of line", sink);
                        }
                        None => self.sync_line(),
                    }
                }
            }
        }
        self.eat(&TokenKind::Dedent);
        Some(ClassDecl {
            name,
            fields,
            span: start.merge(self.prev_span()),
        })
    }

    fn start_section(&mut self, sink: &mut DiagnosticSink) -> Option<Block> {
        self.bump(); // start
        self.expect(&TokenKind::Colon, "':'", sink);
        self.expect(&TokenKind::Newline, "end of line", sink);
        Some(self.indented_block(sink))
    }

    // ----- statements --------------------------------------------------

    fn indented_block(&mut self, sink: &mut DiagnosticSink) -> Block {
        if !self.expect(&TokenKind::Indent, "indented block", sink) {
            return Vec::new();
        }
        let mut statements = Vec::new();
        loop {
            match self.peek() {
                TokenKind::Newline => {
                    self.bump();
                }
                TokenKind::Dedent | TokenKind::Eof => break,
                _ => {
                    if let Some(statement) = self.statement(sink) {
                        statements.push(statement);
                    }
                }
            }
        }
        self.eat(&TokenKind::Dedent);
        statements
    }

    fn statement(&mut self, sink: &mut DiagnosticSink) -> Option<Stmt> {
        let start = self.span();
        match self.peek() {
            TokenKind::Keyword(Kw::Return) => {
                self.bump();
                let value = if self.at(&TokenKind::Newline) {
                    None
                } else {
                    Some(self.expression(sink))
                };
                self.expect(&TokenKind::Newline, "end of line", sink);
                Some(Stmt::Return {
                    value,
                    span: start.merge(self.prev_span()),
                })
            }
            TokenKind::Keyword(Kw::If) => self.if_statement(sink),
            TokenKind::Keyword(Kw::Print) => self.print_block(sink),
            _ if self.starts_type_first_declaration() => {
                let ty = self.type_expr(false, sink);
                let Some((name, _)) = self.ident("variable name", sink) else {
                    self.sync_line();
                    return None;
                };
                let init = if self.eat(&TokenKind::Assign) {
                    Some(self.expression(sink))
                } else {
                    None
                };
                self.expect(&TokenKind::Newline, "end of line", sink);
                Some(Stmt::VarDecl {
                    ty,
                    name,
                    init,
                    span: start.merge(self.prev_span()),
                })
            }
            _ => {
                let expr = self.expression(sink);
                if self.eat(&TokenKind::Assign) {
                    if !matches!(expr, Expr::Ident { .. } | Expr::Member { .. }) {
                        // STM-02: targets are identifier, member, or index.
                        self.error_at(
                            sink,
                            codes::SYN005,
                            "assignment target must be a variable, member, or index".to_string(),
                            expr.span(),
                        );
                    }
                    let value = self.expression(sink);
                    self.expect(&TokenKind::Newline, "end of line", sink);
                    return Some(Stmt::Assign {
                        target: expr,
                        value,
                        span: start.merge(self.prev_span()),
                    });
                }
                self.expect(&TokenKind::Newline, "end of line", sink);
                Some(Stmt::Expr(expr))
            }
        }
    }

    /// Type-first declaration starts with a type keyword, or with an
    /// identifier immediately followed by another identifier (`User u`).
    fn starts_type_first_declaration(&self) -> bool {
        match self.peek() {
            TokenKind::Ident(name) => {
                if is_type_word(name) {
                    return true;
                }
                matches!(self.peek2(), TokenKind::Ident(_))
            }
            _ => false,
        }
    }

    fn if_statement(&mut self, sink: &mut DiagnosticSink) -> Option<Stmt> {
        let start = self.span();
        self.bump(); // if
        let cond = self.expression(sink);
        self.expect(&TokenKind::Newline, "end of line", sink);
        let then = self.indented_block(sink);
        let mut else_ifs = Vec::new();
        let mut els = None;
        while self.at_kw(Kw::Else) {
            self.bump();
            if self.eat_kw(Kw::If) {
                let elif_cond = self.expression(sink);
                self.expect(&TokenKind::Newline, "end of line", sink);
                let body = self.indented_block(sink);
                else_ifs.push((elif_cond, body));
            } else {
                self.expect(&TokenKind::Newline, "end of line", sink);
                els = Some(self.indented_block(sink));
                break;
            }
        }
        Some(Stmt::If {
            cond,
            then,
            else_ifs,
            els,
            span: start.merge(self.prev_span()),
        })
    }

    fn print_block(&mut self, sink: &mut DiagnosticSink) -> Option<Stmt> {
        let start = self.span();
        self.bump(); // print
        self.expect(&TokenKind::Colon, "':' after 'print'", sink);
        self.expect(&TokenKind::Newline, "end of line", sink);
        let mut items = Vec::new();
        if self.expect(&TokenKind::Indent, "indented print body", sink) {
            loop {
                match self.peek() {
                    TokenKind::Newline => {
                        self.bump();
                    }
                    TokenKind::Dedent | TokenKind::Eof => break,
                    _ => {
                        items.push(self.expression(sink));
                        self.expect(&TokenKind::Newline, "end of line", sink);
                    }
                }
            }
            self.eat(&TokenKind::Dedent);
        }
        if items.is_empty() {
            self.error_at(
                sink,
                codes::SYN008,
                "print: block must contain at least one expression".to_string(),
                start,
            );
        }
        Some(Stmt::Print {
            items,
            span: start.merge(self.prev_span()),
        })
    }

    // ----- types --------------------------------------------------------

    /// `host_position` admits the LBS-02 width suffixes (`integer:32`,
    /// `integer:u32`) that are invalid in surface-language positions.
    fn type_expr(&mut self, host_position: bool, sink: &mut DiagnosticSink) -> TypeExpr {
        let start = self.span();
        let base = match self.peek().clone() {
            TokenKind::Ident(name) => {
                self.bump();
                match name.as_str() {
                    "boolean" => BaseType::Boolean,
                    "integer" => {
                        let width = if host_position && self.at(&TokenKind::Colon) {
                            self.bump();
                            self.int_width(sink)
                        } else {
                            None
                        };
                        BaseType::Integer(width)
                    }
                    "number" => BaseType::Number,
                    "string" => BaseType::String_,
                    "bytes" => BaseType::Bytes,
                    "datetime" => BaseType::Datetime,
                    "any" => BaseType::Any,
                    "void" => BaseType::Void,
                    "list" => {
                        self.expect(&TokenKind::Lt, "'<' after 'list'", sink);
                        let element = self.type_expr(host_position, sink);
                        self.expect(&TokenKind::Gt, "'>'", sink);
                        BaseType::List(Box::new(element))
                    }
                    "pairs" => {
                        self.expect(&TokenKind::Lt, "'<' after 'pairs'", sink);
                        let key = self.type_expr(host_position, sink);
                        self.expect(&TokenKind::Comma, "','", sink);
                        let value = self.type_expr(host_position, sink);
                        self.expect(&TokenKind::Gt, "'>'", sink);
                        BaseType::Pairs(Box::new(key), Box::new(value))
                    }
                    _ => BaseType::Named(name),
                }
            }
            _ => {
                self.error_here(sink, "expected a type".to_string());
                BaseType::Named("<error>".to_string())
            }
        };
        // TYP-03: a single `?` only; the grammar rejects `??` here and the
        // second `?` falls out as an unexpected token.
        let optional = self.eat(&TokenKind::Question);
        TypeExpr {
            base,
            optional,
            span: start.merge(self.prev_span()),
        }
    }

    fn int_width(&mut self, sink: &mut DiagnosticSink) -> Option<IntWidth> {
        match self.peek().clone() {
            TokenKind::Int(32) => {
                self.bump();
                Some(IntWidth::S32)
            }
            TokenKind::Ident(word) => {
                let width = match word.as_str() {
                    "u8" => Some(IntWidth::U8),
                    "u16" => Some(IntWidth::U16),
                    "u32" => Some(IntWidth::U32),
                    "u64" => Some(IntWidth::U64),
                    _ => None,
                };
                if width.is_some() {
                    self.bump();
                } else {
                    self.error_here(
                        sink,
                        "integer width must be one of 32, u8, u16, u32, u64".to_string(),
                    );
                }
                width
            }
            _ => {
                self.error_here(
                    sink,
                    "integer width must be one of 32, u8, u16, u32, u64".to_string(),
                );
                None
            }
        }
    }

    // ----- expressions (EXP-01 ladder) ----------------------------------

    fn expression(&mut self, sink: &mut DiagnosticSink) -> Expr {
        self.default_expr(sink)
    }

    /// Level 11: `default` — none-coalescing, left-associative (EXP-03).
    /// `onError` (level 13) sits above this and is outside the M1 surface.
    fn default_expr(&mut self, sink: &mut DiagnosticSink) -> Expr {
        let mut lhs = self.or_expr(sink);
        while self.at_kw(Kw::Default) {
            self.bump();
            let rhs = self.or_expr(sink);
            lhs = binary(BinOp::Default, lhs, rhs);
        }
        lhs
    }

    fn or_expr(&mut self, sink: &mut DiagnosticSink) -> Expr {
        let mut lhs = self.and_expr(sink);
        while self.at_kw(Kw::Or) {
            self.bump();
            let rhs = self.and_expr(sink);
            lhs = binary(BinOp::Or, lhs, rhs);
        }
        lhs
    }

    fn and_expr(&mut self, sink: &mut DiagnosticSink) -> Expr {
        let mut lhs = self.equality_expr(sink);
        while self.at_kw(Kw::And) {
            self.bump();
            let rhs = self.equality_expr(sink);
            lhs = binary(BinOp::And, lhs, rhs);
        }
        lhs
    }

    fn equality_expr(&mut self, sink: &mut DiagnosticSink) -> Expr {
        let mut lhs = self.comparison_expr(sink);
        loop {
            let op = match self.peek() {
                TokenKind::Eq => BinOp::Eq,
                TokenKind::NEq => BinOp::NEq,
                _ => break,
            };
            self.bump();
            let rhs = self.comparison_expr(sink);
            lhs = binary(op, lhs, rhs);
        }
        lhs
    }

    fn comparison_expr(&mut self, sink: &mut DiagnosticSink) -> Expr {
        let mut lhs = self.additive_expr(sink);
        loop {
            let op = match self.peek() {
                TokenKind::Lt => BinOp::Lt,
                TokenKind::LtEq => BinOp::LtEq,
                TokenKind::Gt => BinOp::Gt,
                TokenKind::GtEq => BinOp::GtEq,
                _ => break,
            };
            self.bump();
            let rhs = self.additive_expr(sink);
            lhs = binary(op, lhs, rhs);
        }
        lhs
    }

    fn additive_expr(&mut self, sink: &mut DiagnosticSink) -> Expr {
        let mut lhs = self.multiplicative_expr(sink);
        loop {
            let op = match self.peek() {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => break,
            };
            self.bump();
            let rhs = self.multiplicative_expr(sink);
            lhs = binary(op, lhs, rhs);
        }
        lhs
    }

    fn multiplicative_expr(&mut self, sink: &mut DiagnosticSink) -> Expr {
        let mut lhs = self.power_expr(sink);
        loop {
            let op = match self.peek() {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                TokenKind::Percent => BinOp::Rem,
                _ => break,
            };
            self.bump();
            let rhs = self.power_expr(sink);
            lhs = binary(op, lhs, rhs);
        }
        lhs
    }

    /// `^` is right-associative (EXP-01 level 4).
    fn power_expr(&mut self, sink: &mut DiagnosticSink) -> Expr {
        let lhs = self.unary_expr(sink);
        if self.at(&TokenKind::Caret) {
            self.bump();
            let rhs = self.power_expr(sink);
            return binary(BinOp::Pow, lhs, rhs);
        }
        lhs
    }

    fn unary_expr(&mut self, sink: &mut DiagnosticSink) -> Expr {
        if self.at_kw(Kw::Not) {
            let start = self.span();
            self.bump();
            let operand = self.unary_expr(sink);
            let span = start.merge(operand.span());
            return Expr::Unary {
                op: UnOp::Not,
                operand: Box::new(operand),
                span,
            };
        }
        if self.at(&TokenKind::Minus) {
            let start = self.span();
            self.bump();
            let operand = self.unary_expr(sink);
            let span = start.merge(operand.span());
            return Expr::Unary {
                op: UnOp::Neg,
                operand: Box::new(operand),
                span,
            };
        }
        self.postfix_expr(sink)
    }

    fn postfix_expr(&mut self, sink: &mut DiagnosticSink) -> Expr {
        let mut expr = self.primary_expr(sink);
        loop {
            match self.peek() {
                TokenKind::Dot => {
                    self.bump();
                    match self.ident("member name", sink) {
                        Some((name, name_span)) => {
                            let span = expr.span().merge(name_span);
                            expr = Expr::Member {
                                receiver: Box::new(expr),
                                name,
                                span,
                            };
                        }
                        None => break,
                    }
                }
                TokenKind::LParen => {
                    self.bump();
                    self.paren_depth += 1;
                    let mut args = Vec::new();
                    if !self.at(&TokenKind::RParen) {
                        loop {
                            args.push(self.expression(sink));
                            if !self.eat(&TokenKind::Comma) {
                                break;
                            }
                        }
                    }
                    self.paren_depth -= 1;
                    let end = self.span();
                    self.expect(&TokenKind::RParen, "')'", sink);
                    let span = expr.span().merge(end);
                    expr = Expr::Call {
                        callee: Box::new(expr),
                        args,
                        span,
                    };
                }
                _ => break,
            }
        }
        expr
    }

    fn primary_expr(&mut self, sink: &mut DiagnosticSink) -> Expr {
        let span = self.span();
        match self.peek().clone() {
            TokenKind::Int(value) => {
                self.bump();
                Expr::Int { value, span }
            }
            TokenKind::Number(text) => {
                self.bump();
                Expr::Number { text, span }
            }
            TokenKind::Str {
                value,
                interpolations,
            } => {
                self.bump();
                Expr::Str {
                    value,
                    interpolations,
                    span,
                }
            }
            TokenKind::Keyword(Kw::True) => {
                self.bump();
                Expr::Bool { value: true, span }
            }
            TokenKind::Keyword(Kw::False) => {
                self.bump();
                Expr::Bool { value: false, span }
            }
            TokenKind::Keyword(Kw::None) => {
                self.bump();
                Expr::NoneLit { span }
            }
            TokenKind::Ident(name) => {
                self.bump();
                Expr::Ident { name, span }
            }
            TokenKind::LParen => {
                self.bump();
                self.paren_depth += 1;
                let inner = self.expression(sink);
                self.paren_depth -= 1;
                self.expect(&TokenKind::RParen, "')'", sink);
                inner
            }
            TokenKind::LBracket => {
                self.bump();
                self.paren_depth += 1;
                let mut items = Vec::new();
                if !self.at(&TokenKind::RBracket) {
                    loop {
                        items.push(self.expression(sink));
                        if !self.eat(&TokenKind::Comma) {
                            break;
                        }
                    }
                }
                self.paren_depth -= 1;
                let end = self.span();
                self.expect(&TokenKind::RBracket, "']'", sink);
                Expr::List {
                    items,
                    span: span.merge(end),
                }
            }
            other => {
                self.error_here(sink, format!("expected an expression, found {other:?}"));
                self.bump();
                Expr::Ident {
                    name: "<error>".to_string(),
                    span,
                }
            }
        }
    }
}

fn binary(op: BinOp, lhs: Expr, rhs: Expr) -> Expr {
    let span = lhs.span().merge(rhs.span());
    Expr::Binary {
        op,
        lhs: Box::new(lhs),
        rhs: Box::new(rhs),
        span,
    }
}

fn is_type_word(word: &str) -> bool {
    matches!(
        word,
        "boolean"
            | "integer"
            | "number"
            | "string"
            | "bytes"
            | "datetime"
            | "any"
            | "void"
            | "list"
            | "pairs"
    )
}
