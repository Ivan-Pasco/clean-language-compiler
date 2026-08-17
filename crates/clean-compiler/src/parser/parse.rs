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
        tokens: &stream.tokens,
        pos: 0,
        paren_depth: 0,
    };
    parser.source_file(sink)
}

struct Parser<'a> {
    stream: &'a TokenStream,
    /// The token slice being parsed — the whole file, or one interpolation
    /// interior (both end in `Eof`).
    tokens: &'a [Token],
    pos: usize,
    paren_depth: u32,
}

impl<'a> Parser<'a> {
    // ----- cursor -----------------------------------------------------

    fn peek(&self) -> &'a TokenKind {
        &self.tokens[self.effective_pos()].kind
    }

    fn peek2(&self) -> &'a TokenKind {
        let last = self.tokens.len() - 1;
        let mut i = (self.effective_pos() + 1).min(last);
        if self.paren_depth > 0 {
            while i < last
                && matches!(
                    self.tokens[i].kind,
                    TokenKind::Newline | TokenKind::Indent | TokenKind::Dedent
                )
            {
                i += 1;
            }
        }
        &self.tokens[i].kind
    }

    fn effective_pos(&self) -> usize {
        let mut i = self.pos;
        if self.paren_depth > 0 {
            // EXP-02: inside parentheses, line breaks do not end the
            // expression — layout tokens are transparent.
            while matches!(
                self.tokens[i].kind,
                TokenKind::Newline | TokenKind::Indent | TokenKind::Dedent
            ) {
                i += 1;
            }
        }
        i
    }

    fn span(&self) -> ByteSpan {
        self.tokens[self.effective_pos()].span
    }

    fn prev_span(&self) -> ByteSpan {
        self.tokens[self.pos.saturating_sub(1)].span
    }

    /// Advances past the current token — except at `Eof`, which is sticky so
    /// error recovery can never run off the end of the stream.
    fn bump(&mut self) -> &'a Token {
        let i = self.effective_pos();
        let token = &self.tokens[i];
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
            TokenKind::Str { parts } => {
                let span = self.span();
                let plain = crate::lexer::plain_text(parts);
                self.bump();
                match plain {
                    Some(value) => Some(value),
                    None => {
                        // A literal-only position (version, path, description)
                        // cannot interpolate.
                        self.error_at(
                            sink,
                            codes::SYN005,
                            format!("{what} must be a plain string literal without interpolation"),
                            span,
                        );
                        None
                    }
                }
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
        // FIL-01 section-order tracking: each ranked section must not
        // appear after a higher-ranked one, and singleton sections appear
        // at most once. Violations are SYN007, recoverable (13 §10.1).
        let mut last_section: Option<(&'static str, u8)> = None;
        let mut seen_singletons: Vec<&'static str> = Vec::new();
        loop {
            match self.peek() {
                TokenKind::Eof => break,
                TokenKind::Newline | TokenKind::Indent | TokenKind::Dedent => {
                    // Stray layout (e.g. after recovery) is consumed, never
                    // looped on.
                    self.bump();
                }
                _ => {
                    let section_span = self.span();
                    let Some((item, order)) = self.top_level_item(sink) else {
                        continue;
                    };
                    if let Some(order) = order {
                        if let Some((prev_name, prev_rank)) = last_section {
                            if order.rank < prev_rank {
                                self.error_at(
                                    sink,
                                    codes::SYN007,
                                    format!(
                                        "the '{}' section cannot appear after '{prev_name}'; FIL-01 fixes the top-level section order",
                                        order.name
                                    ),
                                    section_span,
                                );
                            }
                        }
                        if order.singleton {
                            if seen_singletons.contains(&order.name) {
                                self.error_at(
                                    sink,
                                    codes::SYN007,
                                    format!("the '{}' section appears more than once", order.name),
                                    section_span,
                                );
                            } else {
                                seen_singletons.push(order.name);
                            }
                        }
                        if last_section.is_none_or(|(_, prev)| order.rank >= prev) {
                            last_section = Some((order.name, order.rank));
                        }
                    }
                    items.push(item);
                }
            }
        }
        SourceFile {
            path: self.stream.path.clone(),
            items,
        }
    }

    /// Parses one top-level form and its FIL-01 ordering slot (`None` for
    /// forms outside the FIL-01 table, like `host interface`). Returns
    /// `None` after error recovery.
    fn top_level_item(
        &mut self,
        sink: &mut DiagnosticSink,
    ) -> Option<(Item, Option<SectionOrder>)> {
        match self.peek() {
            TokenKind::Keyword(Kw::Import) => match self.peek2() {
                TokenKind::Colon => {
                    let entries = self.import_section(sink)?;
                    Some((
                        Item::Imports(entries),
                        SectionOrder::singleton("import:", 1),
                    ))
                }
                TokenKind::Str { .. } => {
                    let start = self.span();
                    self.bump(); // import
                    let path = self.string_literal("import path", sink)?;
                    let span = start.merge(self.prev_span());
                    self.expect(&TokenKind::Newline, "end of line", sink);
                    Some((
                        Item::FileImport { path, span },
                        SectionOrder::repeatable("import", 1),
                    ))
                }
                _ => {
                    self.error_here(
                        sink,
                        "expected ':' (module imports) or a path string after 'import'".to_string(),
                    );
                    self.sync_line();
                    None
                }
            },
            TokenKind::Ident(w) if w == "source" && matches!(self.peek2(), TokenKind::Colon) => {
                let section = self.source_section(sink)?;
                Some((Item::Source(section), SectionOrder::singleton("source:", 2)))
            }
            TokenKind::Keyword(Kw::Constant) => match self.peek2() {
                TokenKind::Colon => {
                    let constants = self.constant_section(sink)?;
                    Some((
                        Item::Constants(constants),
                        SectionOrder::singleton("constant:", 3),
                    ))
                }
                TokenKind::Keyword(Kw::Function) => {
                    let f = self.constant_function(sink)?;
                    Some((
                        Item::ConstantFunction(f),
                        SectionOrder::repeatable("constant function", 6),
                    ))
                }
                _ => {
                    self.error_here(
                        sink,
                        "expected ':' (constant section) or 'function' after 'constant'"
                            .to_string(),
                    );
                    self.sync_line();
                    None
                }
            },
            TokenKind::Ident(w) if w == "state" && matches!(self.peek2(), TokenKind::Colon) => {
                let section = self.state_section(sink)?;
                Some((Item::State(section), SectionOrder::singleton("state:", 4)))
            }
            TokenKind::Keyword(Kw::Class) => {
                let class = self.class_decl(sink)?;
                Some((Item::Class(class), SectionOrder::repeatable("class", 5)))
            }
            TokenKind::Keyword(Kw::Can) => {
                let capability = self.capability_decl(sink)?;
                Some((
                    Item::Capability(capability),
                    SectionOrder::repeatable("can", 5),
                ))
            }
            TokenKind::Ident(w) if w == "functions" && matches!(self.peek2(), TokenKind::Colon) => {
                let functions = self.functions_block(sink)?;
                Some((
                    Item::Functions(functions),
                    SectionOrder::singleton("functions:", 6),
                ))
            }
            TokenKind::Keyword(Kw::Compiletime) => {
                let f = self.compiletime_function(sink)?;
                Some((
                    Item::CompiletimeFunction(f),
                    SectionOrder::repeatable("compiletime function", 6),
                ))
            }
            TokenKind::Keyword(Kw::Handles) => {
                let h = self.handles_block(sink)?;
                Some((
                    Item::HandlesBlock(h),
                    SectionOrder::repeatable("handles block", 6),
                ))
            }
            // LBS-02 library surface: outside the FIL-01 table.
            TokenKind::Keyword(Kw::Host) => {
                let hi = self.host_interface(sink)?;
                Some((Item::HostInterface(hi), None))
            }
            TokenKind::Ident(w) if w == "watch" => {
                let watch = self.watch_block(sink)?;
                Some((Item::Watch(watch), SectionOrder::repeatable("watch", 7)))
            }
            TokenKind::Ident(w) if w == "tests" && matches!(self.peek2(), TokenKind::Colon) => {
                let tests = self.tests_section(sink)?;
                Some((Item::Tests(tests), SectionOrder::singleton("tests:", 8)))
            }
            TokenKind::Keyword(Kw::Start) => {
                let block = self.start_section(sink)?;
                Some((Item::Start(block), SectionOrder::singleton("start:", 9)))
            }
            // A library-registered block (08 §3): an identifier-headed line
            // ending in ':' that no earlier arm claimed. Checked before the
            // type-first arm so `data UserData:` is a block, not a
            // malformed declaration.
            TokenKind::Ident(_) if self.line_is_block_header() => {
                let block = self.library_block(sink)?;
                Some((Item::LibraryBlock(block), None))
            }
            _ if self.starts_type_first_declaration() => {
                // A type-first header at the top level is a bare
                // FunctionDeclaration (08 §2 TopLevelCallable); a variable
                // declaration here violates FIL-02.
                let start = self.span();
                let ty = self.type_expr(TypePos::Surface, sink);
                let Some((name, _)) = self.ident("function name", sink) else {
                    self.sync_line();
                    return None;
                };
                if !self.at(&TokenKind::LParen) {
                    self.error_at(
                        sink,
                        codes::SYN009,
                        "'variable declaration' cannot appear at the top level of a file"
                            .to_string(),
                        start,
                    );
                    self.sync_line();
                    return None;
                }
                let f = self.function_decl_tail(ty, name, start, false, sink)?;
                Some((Item::Function(f), SectionOrder::repeatable("function", 6)))
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
                None
            }
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
                let ty = self.type_expr(TypePos::Host, sink);
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
            Some(self.type_expr(TypePos::Host, sink))
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

    /// `functions:` block containing FNC-02 type-first declarations, with
    /// `public:` wrappers marking exports (MOD-02).
    fn functions_block(&mut self, sink: &mut DiagnosticSink) -> Option<Vec<Function>> {
        self.bump(); // functions
        self.expect(&TokenKind::Colon, "':'", sink);
        self.expect(&TokenKind::Newline, "end of line", sink);
        if !self.expect(&TokenKind::Indent, "indented functions body", sink) {
            return None;
        }
        let mut functions = Vec::new();
        self.functions_block_members(false, &mut functions, sink);
        self.eat(&TokenKind::Dedent);
        Some(functions)
    }

    fn functions_block_members(
        &mut self,
        public: bool,
        out: &mut Vec<Function>,
        sink: &mut DiagnosticSink,
    ) {
        loop {
            match self.peek() {
                TokenKind::Newline => {
                    self.bump();
                }
                TokenKind::Dedent | TokenKind::Eof => break,
                TokenKind::Keyword(Kw::Public) if matches!(self.peek2(), TokenKind::Colon) => {
                    self.bump(); // public
                    self.bump(); // ':'
                    self.expect(&TokenKind::Newline, "end of line", sink);
                    if self.expect(&TokenKind::Indent, "indented public body", sink) {
                        self.functions_block_members(true, out, sink);
                        self.eat(&TokenKind::Dedent);
                    }
                }
                _ => {
                    if let Some(f) = self.function_decl(public, sink) {
                        out.push(f);
                    }
                }
            }
        }
    }

    /// `ReturnType name(params) [background]` + body (FNC-02/FNC-03,
    /// ASY-02).
    fn function_decl(&mut self, public: bool, sink: &mut DiagnosticSink) -> Option<Function> {
        let start = self.span();
        let ret = self.type_expr(TypePos::Surface, sink);
        let Some((name, _)) = self.ident("function name", sink) else {
            self.sync_line();
            return None;
        };
        if !self.expect(&TokenKind::LParen, "'(' after function name", sink) {
            self.sync_line();
            return None;
        }
        self.function_decl_tail_inner(ret, name, start, public, sink)
    }

    /// The rest of a function declaration once `ReturnType name` is read
    /// and `(` is next (used by the top-level TopLevelCallable dispatch).
    fn function_decl_tail(
        &mut self,
        ret: TypeExpr,
        name: String,
        start: ByteSpan,
        public: bool,
        sink: &mut DiagnosticSink,
    ) -> Option<Function> {
        self.expect(&TokenKind::LParen, "'(' after function name", sink);
        self.function_decl_tail_inner(ret, name, start, public, sink)
    }

    fn function_decl_tail_inner(
        &mut self,
        ret: TypeExpr,
        name: String,
        start: ByteSpan,
        public: bool,
        sink: &mut DiagnosticSink,
    ) -> Option<Function> {
        let params = self.parameter_list(sink);
        // ASY-02: the `background` modifier is postfix-only, right after
        // the parameter list.
        let background = self.eat_kw(Kw::Background);
        self.expect(&TokenKind::Newline, "end of line", sink);
        let body = self.function_body(sink);
        Some(Function {
            ret,
            name,
            params,
            background,
            public,
            body,
            span: start.merge(self.prev_span()),
        })
    }

    /// `( [Parameter, …] )` with the opening paren already consumed.
    fn parameter_list(&mut self, sink: &mut DiagnosticSink) -> Vec<Param> {
        self.paren_depth += 1;
        let mut params = Vec::new();
        if !self.at(&TokenKind::RParen) {
            loop {
                let param_start = self.span();
                let ty = self.type_expr(TypePos::Surface, sink);
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
        params
    }

    /// Function body (09 §2, 19 §3, 10 §2): metadata prelude —
    /// `description`, `input`, `intent`/`spec` lines, `before:`/`after:`
    /// contract blocks — then the statement sequence. The parser accepts
    /// the prelude pieces in any order; placement rules are the checker's.
    fn function_body(&mut self, sink: &mut DiagnosticSink) -> FunctionBody {
        let mut body = FunctionBody::default();
        if !self.expect(&TokenKind::Indent, "indented block", sink) {
            return body;
        }
        loop {
            match self.peek() {
                TokenKind::Newline => {
                    self.bump();
                }
                TokenKind::Ident(w)
                    if w == "description" && matches!(self.peek2(), TokenKind::Str { .. }) =>
                {
                    self.bump();
                    if let Some(text) = self.string_literal("description string", sink) {
                        body.description = Some(text);
                    }
                    self.expect(&TokenKind::Newline, "end of line", sink);
                }
                TokenKind::Ident(w)
                    if w == "input" && matches!(self.peek2(), TokenKind::Newline) =>
                {
                    self.bump();
                    self.expect(&TokenKind::Newline, "end of line", sink);
                    if self.expect(&TokenKind::Indent, "indented input block", sink) {
                        loop {
                            match self.peek() {
                                TokenKind::Newline => {
                                    self.bump();
                                }
                                TokenKind::Dedent | TokenKind::Eof => break,
                                _ => {
                                    let p_start = self.span();
                                    let ty = self.type_expr(TypePos::Surface, sink);
                                    let Some((p_name, _)) = self.ident("parameter name", sink)
                                    else {
                                        self.sync_line();
                                        continue;
                                    };
                                    let default = if self.eat(&TokenKind::Assign) {
                                        Some(self.expression(sink))
                                    } else {
                                        None
                                    };
                                    body.input.push(Param {
                                        ty,
                                        name: p_name,
                                        default,
                                        span: p_start.merge(self.prev_span()),
                                    });
                                    self.expect(&TokenKind::Newline, "end of line", sink);
                                }
                            }
                        }
                        self.eat(&TokenKind::Dedent);
                    }
                }
                TokenKind::Keyword(Kw::Intent) => {
                    let start = self.span();
                    self.bump();
                    if matches!(self.peek(), TokenKind::Str { .. }) {
                        if let Some(text) = self.string_literal("intent description string", sink) {
                            body.intents.push((text, start.merge(self.prev_span())));
                        }
                        self.expect(&TokenKind::Newline, "end of line", sink);
                    } else {
                        // SYN101 — template verbatim from Platform 10 §2.
                        self.error_at(
                            sink,
                            codes::SYN101,
                            "Expected string literal after 'intent'".to_string(),
                            self.span(),
                        );
                        self.sync_line();
                    }
                }
                TokenKind::Keyword(Kw::Spec) => {
                    let start = self.span();
                    self.bump();
                    if matches!(self.peek(), TokenKind::Str { .. }) {
                        if let Some(text) = self.string_literal("spec path string", sink) {
                            body.specs.push((text, start.merge(self.prev_span())));
                        }
                        self.expect(&TokenKind::Newline, "end of line", sink);
                    } else {
                        // SYN100 — template verbatim from Platform 10 §2.
                        self.error_at(
                            sink,
                            codes::SYN100,
                            "Expected string literal after 'spec'".to_string(),
                            self.span(),
                        );
                        self.sync_line();
                    }
                }
                TokenKind::Keyword(Kw::Before) if matches!(self.peek2(), TokenKind::Colon) => {
                    body.before = self.contract_block(sink);
                }
                TokenKind::Keyword(Kw::After) if matches!(self.peek2(), TokenKind::Colon) => {
                    body.after = self.contract_block(sink);
                }
                _ => break,
            }
        }
        // Statement sequence until the block closes.
        loop {
            match self.peek() {
                TokenKind::Newline => {
                    self.bump();
                }
                TokenKind::Dedent | TokenKind::Eof => break,
                _ => {
                    if let Some(statement) = self.statement(sink) {
                        body.statements.push(statement);
                    }
                }
            }
        }
        self.eat(&TokenKind::Dedent);
        body
    }

    /// One `before:` / `after:` / `always:` block (10 §1): the keyword is
    /// at the cursor, `':'` follows; each body line is one expression.
    fn contract_block(&mut self, sink: &mut DiagnosticSink) -> Option<ContractBlock> {
        let start = self.span();
        self.bump(); // keyword
        self.bump(); // ':'
        self.expect(&TokenKind::Newline, "end of line", sink);
        if !self.expect(&TokenKind::Indent, "indented contract block", sink) {
            return None;
        }
        let mut exprs = Vec::new();
        loop {
            match self.peek() {
                TokenKind::Newline => {
                    self.bump();
                }
                TokenKind::Dedent | TokenKind::Eof => break,
                _ => {
                    exprs.push(self.expression(sink));
                    self.expect(&TokenKind::Newline, "end of line", sink);
                }
            }
        }
        self.eat(&TokenKind::Dedent);
        if exprs.is_empty() {
            self.error_at(
                sink,
                codes::SYN005,
                "a contract block must contain at least one boolean expression".to_string(),
                start,
            );
        }
        Some(ContractBlock {
            exprs,
            span: start.merge(self.prev_span()),
        })
    }

    /// `class Name [is Parent] [can C1, C2]` + body (14 §1–§3). Body order
    /// is STRICT per the grammar: fields → always: → constructors →
    /// functions:, with `public:` wrappers interleavable.
    fn class_decl(&mut self, sink: &mut DiagnosticSink) -> Option<ClassDecl> {
        let start = self.span();
        self.bump(); // class
        let (name, _) = self.ident("class name", sink)?;
        let parent = if self.eat_kw(Kw::Is) {
            self.ident("parent class name", sink)
        } else {
            None
        };
        let mut capabilities = Vec::new();
        if self.eat_kw(Kw::Can) {
            while let Some(cap) = self.ident("capability name", sink) {
                capabilities.push(cap);
                if !self.eat(&TokenKind::Comma) {
                    break;
                }
            }
        }
        self.expect(&TokenKind::Newline, "end of line", sink);
        if !self.expect(&TokenKind::Indent, "indented class body", sink) {
            return None;
        }
        let mut class = ClassDecl {
            name,
            parent,
            capabilities,
            fields: Vec::new(),
            always: None,
            constructors: Vec::new(),
            functions: Vec::new(),
            span: start,
        };
        self.class_members(false, &mut class, sink);
        self.eat(&TokenKind::Dedent);
        class.span = start.merge(self.prev_span());
        Some(class)
    }

    /// Class-body members at one nesting level; `public` marks members
    /// inside a `public:` wrapper (17 §5). The strict CLS body order is
    /// enforced: a member of an earlier stage after a later one is SYN005.
    fn class_members(&mut self, public: bool, class: &mut ClassDecl, sink: &mut DiagnosticSink) {
        loop {
            match self.peek() {
                TokenKind::Newline => {
                    self.bump();
                }
                TokenKind::Dedent | TokenKind::Eof => break,
                TokenKind::Keyword(Kw::Public) if matches!(self.peek2(), TokenKind::Colon) => {
                    self.bump(); // public
                    self.bump(); // ':'
                    self.expect(&TokenKind::Newline, "end of line", sink);
                    if self.expect(&TokenKind::Indent, "indented public body", sink) {
                        self.class_members(true, class, sink);
                        self.eat(&TokenKind::Dedent);
                    }
                }
                TokenKind::Keyword(Kw::Always) if matches!(self.peek2(), TokenKind::Colon) => {
                    let span = self.span();
                    if class.always.is_some()
                        || !class.constructors.is_empty()
                        || !class.functions.is_empty()
                    {
                        self.error_at(
                            sink,
                            codes::SYN005,
                            "the always: block appears once, after fields and before constructors"
                                .to_string(),
                            span,
                        );
                    }
                    if let Some(block) = self.contract_block(sink) {
                        if class.always.is_none() {
                            class.always = Some(block);
                        }
                    }
                }
                TokenKind::Keyword(Kw::Constructor) => {
                    let c_start = self.span();
                    if !class.functions.is_empty() {
                        self.error_at(
                            sink,
                            codes::SYN005,
                            "constructors appear before the functions: block in a class body"
                                .to_string(),
                            c_start,
                        );
                    }
                    self.bump(); // constructor
                    if !self.expect(&TokenKind::LParen, "'(' after 'constructor'", sink) {
                        self.sync_line();
                        continue;
                    }
                    let params = self.parameter_list(sink);
                    self.expect(&TokenKind::Newline, "end of line", sink);
                    let body = self.indented_block(sink);
                    class.constructors.push(Constructor {
                        params,
                        body,
                        span: c_start.merge(self.prev_span()),
                    });
                }
                TokenKind::Ident(w)
                    if w == "functions" && matches!(self.peek2(), TokenKind::Colon) =>
                {
                    self.bump(); // functions
                    self.bump(); // ':'
                    self.expect(&TokenKind::Newline, "end of line", sink);
                    if self.expect(&TokenKind::Indent, "indented functions body", sink) {
                        self.functions_block_members(public, &mut class.functions, sink);
                        self.eat(&TokenKind::Dedent);
                    }
                }
                _ => {
                    let field_start = self.span();
                    if class.always.is_some()
                        || !class.constructors.is_empty()
                        || !class.functions.is_empty()
                    {
                        self.error_at(
                            sink,
                            codes::SYN005,
                            "fields appear first in a class body, before always:, constructors, and functions:"
                                .to_string(),
                            field_start,
                        );
                    }
                    let ty = self.type_expr(TypePos::Surface, sink);
                    match self.ident("field name", sink) {
                        Some((field_name, _)) => {
                            let init = if self.eat(&TokenKind::Assign) {
                                Some(self.expression(sink))
                            } else {
                                None
                            };
                            class.fields.push(Field {
                                ty,
                                name: field_name,
                                init,
                                public,
                                span: field_start.merge(self.prev_span()),
                            });
                            self.expect(&TokenKind::Newline, "end of line", sink);
                        }
                        None => self.sync_line(),
                    }
                }
            }
        }
    }

    /// `can Name:` capability declaration (14 §4): arrow-return signatures
    /// only, no bodies (SEM014 is the checker's).
    fn capability_decl(&mut self, sink: &mut DiagnosticSink) -> Option<CapabilityDecl> {
        let start = self.span();
        self.bump(); // can
        let (name, _) = self.ident("capability name", sink)?;
        self.expect(&TokenKind::Colon, "':'", sink);
        self.expect(&TokenKind::Newline, "end of line", sink);
        if !self.expect(&TokenKind::Indent, "indented capability body", sink) {
            return None;
        }
        let mut signatures = Vec::new();
        loop {
            match self.peek() {
                TokenKind::Newline => {
                    self.bump();
                }
                TokenKind::Dedent | TokenKind::Eof => break,
                _ => {
                    let sig_start = self.span();
                    let Some((sig_name, _)) = self.ident("method name", sink) else {
                        self.sync_line();
                        continue;
                    };
                    if !self.expect(&TokenKind::LParen, "'(' after method name", sink) {
                        self.sync_line();
                        continue;
                    }
                    let params = self.parameter_list(sink);
                    if !self.expect(&TokenKind::Arrow, "'->' before the return type", sink) {
                        self.sync_line();
                        continue;
                    }
                    let ret = self.type_expr(TypePos::Surface, sink);
                    signatures.push(CapabilitySig {
                        name: sig_name,
                        params,
                        ret,
                        span: sig_start.merge(self.prev_span()),
                    });
                    self.expect(&TokenKind::Newline, "end of line", sink);
                }
            }
        }
        self.eat(&TokenKind::Dedent);
        if signatures.is_empty() {
            self.error_at(
                sink,
                codes::SYN005,
                "a capability declares at least one method signature".to_string(),
                start,
            );
        }
        Some(CapabilityDecl {
            name,
            signatures,
            span: start.merge(self.prev_span()),
        })
    }

    // ----- sections (08-file-structure) ---------------------------------

    /// `import:` block (17 §1): one module entry per line.
    fn import_section(&mut self, sink: &mut DiagnosticSink) -> Option<Vec<ImportEntry>> {
        self.bump(); // import
        self.bump(); // ':'
        self.expect(&TokenKind::Newline, "end of line", sink);
        if !self.expect(&TokenKind::Indent, "indented import body", sink) {
            return None;
        }
        let mut entries = Vec::new();
        loop {
            match self.peek() {
                TokenKind::Newline => {
                    self.bump();
                }
                TokenKind::Dedent | TokenKind::Eof => break,
                _ => {
                    let start = self.span();
                    let Some((first, _)) = self.ident("module name", sink) else {
                        self.sync_line();
                        continue;
                    };
                    let mut path = vec![first];
                    while self.eat(&TokenKind::Dot) {
                        match self.ident("module path segment", sink) {
                            Some((segment, _)) => path.push(segment),
                            None => break,
                        }
                    }
                    // `as` is not a keyword — contextual word (17 §2).
                    let alias = if self.eat_word("as") {
                        self.ident("import alias", sink).map(|(alias, _)| alias)
                    } else {
                        None
                    };
                    entries.push(ImportEntry {
                        path,
                        alias,
                        span: start.merge(self.prev_span()),
                    });
                    self.expect(&TokenKind::Newline, "end of line", sink);
                }
            }
        }
        self.eat(&TokenKind::Dedent);
        Some(entries)
    }

    /// `source:` block (19 §4): closed field set `spec` / `version`.
    fn source_section(&mut self, sink: &mut DiagnosticSink) -> Option<SourceSection> {
        let start = self.span();
        self.bump(); // source
        self.bump(); // ':'
        self.expect(&TokenKind::Newline, "end of line", sink);
        if !self.expect(&TokenKind::Indent, "indented source body", sink) {
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
                    let key = if self.eat_kw(Kw::Spec) {
                        Some("spec".to_string())
                    } else if self.eat_word("version") {
                        Some("version".to_string())
                    } else {
                        // DOC-18 closed schema: only spec and version.
                        self.error_at(
                            sink,
                            codes::SYN005,
                            "a source: block admits only 'spec' and 'version' fields".to_string(),
                            field_start,
                        );
                        self.sync_line();
                        None
                    };
                    let Some(key) = key else { continue };
                    self.expect(&TokenKind::Colon, "':'", sink);
                    let value = self
                        .string_literal("field value string", sink)
                        .unwrap_or_default();
                    fields.push(SourceField {
                        key,
                        value,
                        span: field_start.merge(self.prev_span()),
                    });
                    self.expect(&TokenKind::Newline, "end of line", sink);
                }
            }
        }
        self.eat(&TokenKind::Dedent);
        Some(SourceSection {
            fields,
            span: start.merge(self.prev_span()),
        })
    }

    /// `constant:` section (08 §2): TypedDeclaration items.
    fn constant_section(&mut self, sink: &mut DiagnosticSink) -> Option<Vec<ConstantDecl>> {
        self.bump(); // constant
        self.bump(); // ':'
        self.expect(&TokenKind::Newline, "end of line", sink);
        if !self.expect(&TokenKind::Indent, "indented constant body", sink) {
            return None;
        }
        let mut constants = Vec::new();
        loop {
            match self.peek() {
                TokenKind::Newline => {
                    self.bump();
                }
                TokenKind::Dedent | TokenKind::Eof => break,
                _ => {
                    let start = self.span();
                    let ty = self.type_expr(TypePos::Surface, sink);
                    let Some((name, _)) = self.ident("constant name", sink) else {
                        self.sync_line();
                        continue;
                    };
                    let init = if self.eat(&TokenKind::Assign) {
                        Some(self.expression(sink))
                    } else {
                        None
                    };
                    constants.push(ConstantDecl {
                        ty,
                        name,
                        init,
                        span: start.merge(self.prev_span()),
                    });
                    self.expect(&TokenKind::Newline, "end of line", sink);
                }
            }
        }
        self.eat(&TokenKind::Dedent);
        Some(constants)
    }

    /// `state:` block (20 §1): variable declarations with guards,
    /// `computed:` and `rules:` sub-blocks, freely interleaved.
    fn state_section(&mut self, sink: &mut DiagnosticSink) -> Option<StateSection> {
        let start = self.span();
        self.bump(); // state
        self.bump(); // ':'
        self.expect(&TokenKind::Newline, "end of line", sink);
        if !self.expect(&TokenKind::Indent, "indented state body", sink) {
            return None;
        }
        let mut members = Vec::new();
        loop {
            match self.peek() {
                TokenKind::Newline => {
                    self.bump();
                }
                TokenKind::Dedent | TokenKind::Eof => break,
                TokenKind::Ident(w)
                    if w == "computed" && matches!(self.peek2(), TokenKind::Colon) =>
                {
                    self.bump(); // computed
                    self.bump(); // ':'
                    self.expect(&TokenKind::Newline, "end of line", sink);
                    let mut computed = Vec::new();
                    if self.expect(&TokenKind::Indent, "indented computed body", sink) {
                        loop {
                            match self.peek() {
                                TokenKind::Newline => {
                                    self.bump();
                                }
                                TokenKind::Dedent | TokenKind::Eof => break,
                                _ => {
                                    let c_start = self.span();
                                    let ty = self.type_expr(TypePos::Surface, sink);
                                    let Some((c_name, _)) = self.ident("computed value name", sink)
                                    else {
                                        self.sync_line();
                                        continue;
                                    };
                                    self.expect(&TokenKind::Newline, "end of line", sink);
                                    let body = self.indented_block(sink);
                                    computed.push(ComputedDecl {
                                        ty,
                                        name: c_name,
                                        body,
                                        span: c_start.merge(self.prev_span()),
                                    });
                                }
                            }
                        }
                        self.eat(&TokenKind::Dedent);
                    }
                    members.push(StateMember::Computed(computed));
                }
                TokenKind::Ident(w) if w == "rules" && matches!(self.peek2(), TokenKind::Colon) => {
                    self.bump(); // rules
                    self.bump(); // ':'
                    self.expect(&TokenKind::Newline, "end of line", sink);
                    let mut rules = Vec::new();
                    if self.expect(&TokenKind::Indent, "indented rules body", sink) {
                        loop {
                            match self.peek() {
                                TokenKind::Newline => {
                                    self.bump();
                                }
                                TokenKind::Dedent | TokenKind::Eof => break,
                                _ => {
                                    rules.push(self.expression(sink));
                                    self.expect(&TokenKind::Newline, "end of line", sink);
                                }
                            }
                        }
                        self.eat(&TokenKind::Dedent);
                    }
                    members.push(StateMember::Rules(rules));
                }
                _ => {
                    let v_start = self.span();
                    let ty = self.type_expr(TypePos::DeclLhs, sink);
                    let Some((name, _)) = self.ident("state variable name", sink) else {
                        self.sync_line();
                        continue;
                    };
                    // SMG-01: state variables require initial values.
                    if !self.eat(&TokenKind::Assign) {
                        self.error_at(
                            sink,
                            codes::SYN005,
                            format!("state variable '{name}' requires an initial value"),
                            v_start,
                        );
                        self.sync_line();
                        continue;
                    }
                    let init = self.expression(sink);
                    self.expect(&TokenKind::Newline, "end of line", sink);
                    // SMG-02: guard clauses are indented lines directly
                    // beneath the declaration.
                    let mut guards = Vec::new();
                    if self.at(&TokenKind::Indent)
                        && matches!(self.peek2(), TokenKind::Ident(w) if w == "guard")
                    {
                        self.bump(); // indent
                        loop {
                            match self.peek() {
                                TokenKind::Newline => {
                                    self.bump();
                                }
                                TokenKind::Dedent | TokenKind::Eof => break,
                                _ => {
                                    let g_start = self.span();
                                    if !self.eat_word("guard") {
                                        self.error_here(
                                            sink,
                                            "expected a 'guard' clause".to_string(),
                                        );
                                        self.sync_line();
                                        continue;
                                    }
                                    let cond = self.expression(sink);
                                    if !self.eat_kw(Kw::Else) {
                                        self.error_here(
                                            sink,
                                            "expected 'else \"<message>\"' after the guard condition"
                                                .to_string(),
                                        );
                                    }
                                    let message = self
                                        .string_literal("guard message string", sink)
                                        .unwrap_or_default();
                                    guards.push(GuardClause {
                                        cond,
                                        message,
                                        span: g_start.merge(self.prev_span()),
                                    });
                                    self.expect(&TokenKind::Newline, "end of line", sink);
                                }
                            }
                        }
                        self.eat(&TokenKind::Dedent);
                    }
                    members.push(StateMember::Var(StateVar {
                        ty,
                        name,
                        init,
                        guards,
                        span: v_start.merge(self.prev_span()),
                    }));
                }
            }
        }
        self.eat(&TokenKind::Dedent);
        Some(StateSection {
            members,
            span: start.merge(self.prev_span()),
        })
    }

    /// `watch <target>:` observer (20 §5).
    fn watch_block(&mut self, sink: &mut DiagnosticSink) -> Option<WatchBlock> {
        let start = self.span();
        self.bump(); // watch
        let mut targets = Vec::new();
        if self.eat(&TokenKind::LParen) {
            self.paren_depth += 1;
            while let Some(target) = self.ident("watched variable name", sink) {
                targets.push(target);
                if !self.eat(&TokenKind::Comma) {
                    break;
                }
            }
            self.paren_depth -= 1;
            self.expect(&TokenKind::RParen, "')'", sink);
        } else {
            match self.ident("watched variable name", sink) {
                Some(target) => targets.push(target),
                None => {
                    self.sync_line();
                    return None;
                }
            }
        }
        self.expect(&TokenKind::Colon, "':'", sink);
        self.expect(&TokenKind::Newline, "end of line", sink);
        let body = self.indented_block(sink);
        Some(WatchBlock {
            targets,
            body,
            span: start.merge(self.prev_span()),
        })
    }

    /// `tests:` section (11 §1): named, anonymous, and block tests.
    fn tests_section(&mut self, sink: &mut DiagnosticSink) -> Option<Vec<TestDecl>> {
        self.bump(); // tests
        self.bump(); // ':'
        self.expect(&TokenKind::Newline, "end of line", sink);
        if !self.expect(&TokenKind::Indent, "indented tests body", sink) {
            return None;
        }
        let mut tests = Vec::new();
        loop {
            match self.peek() {
                TokenKind::Newline => {
                    self.bump();
                }
                TokenKind::Dedent | TokenKind::Eof => break,
                TokenKind::Str { .. } if matches!(self.peek2(), TokenKind::Colon) => {
                    // Named single-line test: "description": expression.
                    let start = self.span();
                    let description = self
                        .string_literal("test description string", sink)
                        .unwrap_or_default();
                    self.bump(); // ':'
                    let assertion = self.expression(sink);
                    self.expect(&TokenKind::Newline, "end of line", sink);
                    tests.push(TestDecl::Named {
                        description,
                        assertion,
                        span: start.merge(self.prev_span()),
                    });
                }
                TokenKind::Str { .. } if matches!(self.peek2(), TokenKind::Newline) => {
                    // Block test: description line + indented body with at
                    // least one assert.
                    let start = self.span();
                    let description = self
                        .string_literal("test description string", sink)
                        .unwrap_or_default();
                    self.expect(&TokenKind::Newline, "end of line", sink);
                    let body = self.indented_block(sink);
                    if !body.iter().any(|s| matches!(s, Stmt::Assert { .. })) {
                        self.error_at(
                            sink,
                            codes::SYN005,
                            format!("block test \"{description}\" has no assert; a test asserts at least once"),
                            start,
                        );
                    }
                    tests.push(TestDecl::Block {
                        description,
                        body,
                        span: start.merge(self.prev_span()),
                    });
                }
                _ => {
                    // Anonymous single-line test: a bare assertion.
                    let start = self.span();
                    let assertion = self.expression(sink);
                    self.expect(&TokenKind::Newline, "end of line", sink);
                    tests.push(TestDecl::Anonymous {
                        assertion,
                        span: start.merge(self.prev_span()),
                    });
                }
            }
        }
        self.eat(&TokenKind::Dedent);
        Some(tests)
    }

    /// `constant function name(params) [returns T]` + body (09 §4).
    fn constant_function(&mut self, sink: &mut DiagnosticSink) -> Option<ConstantFunction> {
        let start = self.span();
        self.bump(); // constant
        self.bump(); // function
        let (name, _) = self.ident("function name", sink)?;
        if !self.expect(&TokenKind::LParen, "'(' after function name", sink) {
            self.sync_line();
            return None;
        }
        let params = self.parameter_list(sink);
        let ret = if self.eat_kw(Kw::Returns) {
            Some(self.type_expr(TypePos::Surface, sink))
        } else {
            None
        };
        self.expect(&TokenKind::Newline, "end of line", sink);
        let body = self.function_body(sink);
        Some(ConstantFunction {
            name,
            params,
            ret,
            body,
            span: start.merge(self.prev_span()),
        })
    }

    /// `compiletime function name(params) returns T` + body (21 §1). The
    /// `returns` clause is not optional; its absence is a parse error.
    fn compiletime_function(&mut self, sink: &mut DiagnosticSink) -> Option<CompiletimeFunction> {
        let start = self.span();
        self.bump(); // compiletime
        if !self.eat_kw(Kw::Function) {
            self.error_here(sink, "expected 'function' after 'compiletime'".to_string());
            self.sync_line();
            return None;
        }
        let (name, _) = self.ident("function name", sink)?;
        if !self.expect(&TokenKind::LParen, "'(' after function name", sink) {
            self.sync_line();
            return None;
        }
        let params = self.parameter_list(sink);
        let ret = if self.eat_kw(Kw::Returns) {
            Some(self.type_expr(TypePos::Surface, sink))
        } else {
            self.error_here(
                sink,
                "expected 'returns' in a compiletime function signature".to_string(),
            );
            None
        };
        self.expect(&TokenKind::Newline, "end of line", sink);
        let body = self.function_body(sink);
        Some(CompiletimeFunction {
            name,
            params,
            ret,
            body,
            span: start.merge(self.prev_span()),
        })
    }

    /// `handles block "<name>" with <handler>` (21 §1).
    fn handles_block(&mut self, sink: &mut DiagnosticSink) -> Option<HandlesBlock> {
        let start = self.span();
        self.bump(); // handles
        if !self.eat_kw(Kw::Block) {
            self.error_here(sink, "expected 'block' after 'handles'".to_string());
            self.sync_line();
            return None;
        }
        let block_name = self.string_literal("block name string", sink)?;
        if !self.eat_kw(Kw::With) {
            self.error_here(sink, "expected 'with' after the block name".to_string());
        }
        let (handler, _) = self.ident("handler function name", sink)?;
        self.expect(&TokenKind::Newline, "end of line", sink);
        Some(HandlesBlock {
            block_name,
            handler,
            span: start.merge(self.prev_span()),
        })
    }

    fn start_section(&mut self, sink: &mut DiagnosticSink) -> Option<Block> {
        self.bump(); // start
        self.expect(&TokenKind::Colon, "':'", sink);
        self.expect(&TokenKind::Newline, "end of line", sink);
        Some(self.indented_block(sink))
    }

    // ----- library blocks (21 §21.3) ------------------------------------

    /// Does the line at the cursor look like a block header — an
    /// identifier-headed line whose last token is `':'` and that opens an
    /// indented body?
    fn line_is_block_header(&self) -> bool {
        let mut i = self.effective_pos();
        if !matches!(self.tokens[i].kind, TokenKind::Ident(_)) {
            return false;
        }
        let mut last_was_colon = false;
        while i < self.tokens.len() {
            match &self.tokens[i].kind {
                TokenKind::Newline | TokenKind::Eof => break,
                TokenKind::Colon => {
                    last_was_colon = true;
                    i += 1;
                }
                _ => {
                    last_was_colon = false;
                    i += 1;
                }
            }
        }
        last_was_colon
            && matches!(
                self.tokens.get(i + 1).map(|t| &t.kind),
                Some(TokenKind::Indent)
            )
    }

    /// One library block: `qualified.name [args…]:` + indented body of DSL
    /// lines and nested blocks (schema/block-ast.md). Body lines are
    /// preserved as token lists — the handler tokenises them itself; the
    /// `Statement` variant materialises during expansion (M5).
    fn library_block(&mut self, sink: &mut DiagnosticSink) -> Option<BlockAst> {
        let start = self.span();
        let (mut name, mut last_span) = self.ident("block name", sink)?;
        while self.at(&TokenKind::Dot) {
            self.bump();
            let Some((segment, seg_span)) = self.ident("block name segment", sink) else {
                break;
            };
            name.push('.');
            name.push_str(&segment);
            last_span = seg_span;
        }
        let _ = last_span;
        let mut arguments = Vec::new();
        if self.eat(&TokenKind::LParen) {
            self.paren_depth += 1;
            if !self.at(&TokenKind::RParen) {
                loop {
                    arguments.push(self.block_arg(sink));
                    if !self.eat(&TokenKind::Comma) {
                        break;
                    }
                }
            }
            self.paren_depth -= 1;
            self.expect(&TokenKind::RParen, "')'", sink);
        }
        // Bare header arguments (`data UserData:`): expressions up to the
        // ':' that closes the header.
        while !matches!(
            self.peek(),
            TokenKind::Colon | TokenKind::Newline | TokenKind::Eof
        ) {
            let before = self.pos;
            arguments.push(self.block_arg(sink));
            if self.pos == before {
                break;
            }
        }
        self.expect(&TokenKind::Colon, "':'", sink);
        self.expect(&TokenKind::Newline, "end of line", sink);
        let mut body = Vec::new();
        if self.expect(&TokenKind::Indent, "indented block body", sink) {
            // Raw-line regions indented deeper than the block's own level
            // are flattened into this block's line list (nesting for raw
            // lines is under-specified — docs/DISCOVERIES-M3.md).
            let mut depth = 0u32;
            loop {
                match self.peek() {
                    TokenKind::Newline => {
                        self.bump();
                    }
                    TokenKind::Indent => {
                        depth += 1;
                        self.bump();
                    }
                    TokenKind::Dedent => {
                        if depth == 0 {
                            break;
                        }
                        depth -= 1;
                        self.bump();
                    }
                    TokenKind::Eof => break,
                    _ => {
                        if depth == 0 && self.line_is_block_header() {
                            if let Some(nested) = self.library_block(sink) {
                                body.push(BlockNode::Block(nested));
                            }
                        } else {
                            body.push(BlockNode::Line(self.block_line()));
                        }
                    }
                }
            }
            self.eat(&TokenKind::Dedent);
        }
        Some(BlockAst {
            name,
            arguments,
            body,
            attributes: Vec::new(),
            span: start.merge(self.prev_span()),
        })
    }

    /// One block argument: `name = value` is a keyword argument, anything
    /// else positional (schema/block-ast.md).
    fn block_arg(&mut self, sink: &mut DiagnosticSink) -> BlockArg {
        if let TokenKind::Ident(name) = self.peek() {
            if matches!(self.peek2(), TokenKind::Assign) {
                let name = name.clone();
                let start = self.span();
                self.bump(); // name
                self.bump(); // '='
                let value = self.expression(sink);
                let span = start.merge(self.prev_span());
                return BlockArg::Keyword { name, value, span };
            }
        }
        BlockArg::Positional(self.expression(sink))
    }

    /// One raw DSL line: every token up to the line end, preserved for the
    /// handler.
    fn block_line(&mut self) -> BlockLine {
        let start = self.span();
        let mut tokens = Vec::new();
        while !matches!(
            self.peek(),
            TokenKind::Newline | TokenKind::Dedent | TokenKind::Eof
        ) {
            tokens.push(self.bump().clone());
        }
        let span = start.merge(self.prev_span());
        self.eat(&TokenKind::Newline);
        BlockLine { tokens, span }
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
            TokenKind::Keyword(Kw::While) => self.while_statement(sink),
            TokenKind::Keyword(Kw::Iterate) => self.iterate_statement(sink),
            TokenKind::Keyword(Kw::Break) => {
                self.bump();
                self.expect(&TokenKind::Newline, "end of line", sink);
                Some(Stmt::Break { span: start })
            }
            TokenKind::Keyword(Kw::Continue) => {
                self.bump();
                self.expect(&TokenKind::Newline, "end of line", sink);
                Some(Stmt::Continue { span: start })
            }
            TokenKind::Keyword(Kw::Print) => self.print_block(sink),
            TokenKind::Keyword(Kw::Assert) => {
                self.bump();
                let expr = self.expression(sink);
                self.expect(&TokenKind::Newline, "end of line", sink);
                Some(Stmt::Assert {
                    expr,
                    span: start.merge(self.prev_span()),
                })
            }
            TokenKind::Keyword(Kw::Later) => self.later_binding(sink),
            TokenKind::Keyword(Kw::Background) => {
                self.bump();
                let call = self.expression(sink);
                let on_error = self.on_error_block_tail(sink);
                if on_error.is_none() {
                    self.expect(&TokenKind::Newline, "end of line", sink);
                }
                Some(Stmt::Background {
                    call,
                    on_error,
                    span: start.merge(self.prev_span()),
                })
            }
            TokenKind::Keyword(Kw::Start) => {
                // ASY-01 boundary rule: `start` appears only on the RHS of
                // a `later` binding or after `background`.
                self.error_here(
                    sink,
                    "'start' runs a call in the background only in 'later T x = start f()' or 'background f()' positions"
                        .to_string(),
                );
                self.sync_line();
                None
            }
            TokenKind::Keyword(Kw::Reset) => {
                self.bump();
                let target = if self.eat_word("state") {
                    ResetTarget::State
                } else {
                    match self.ident("variable name to reset", sink) {
                        Some((name, span)) => ResetTarget::Var { name, span },
                        None => {
                            self.sync_line();
                            return None;
                        }
                    }
                };
                self.expect(&TokenKind::Newline, "end of line", sink);
                Some(Stmt::Reset {
                    target,
                    span: start.merge(self.prev_span()),
                })
            }
            TokenKind::Keyword(Kw::Constant) if matches!(self.peek2(), TokenKind::Colon) => {
                self.bump(); // constant
                self.apply_block(ApplyHeader::Constant { span: start }, start, sink)
            }
            TokenKind::Ident(w)
                if is_type_word(w)
                    && matches!(self.peek2(), TokenKind::Colon)
                    && !matches!(w.as_str(), "list" | "matrix" | "pairs") =>
            {
                // Grouped declarations: a bare TypeKeyword header (05 §1).
                let ty = self.type_expr(TypePos::Surface, sink);
                self.apply_block(ApplyHeader::TypeKeyword(ty), start, sink)
            }
            _ if self.starts_type_first_declaration() => {
                let ty = self.type_expr(TypePos::DeclLhs, sink);
                let Some((name, _)) = self.ident("variable name", sink) else {
                    self.sync_line();
                    return None;
                };
                let init = if self.eat(&TokenKind::Assign) {
                    Some(self.expression(sink))
                } else {
                    None
                };
                let on_error = self.on_error_block_tail(sink);
                if on_error.is_some() && init.is_none() {
                    self.error_at(
                        sink,
                        codes::SYN005,
                        "an `onError:` handler needs an initialising expression to guard"
                            .to_string(),
                        start,
                    );
                }
                if on_error.is_none() {
                    self.expect(&TokenKind::Newline, "end of line", sink);
                }
                Some(Stmt::VarDecl {
                    ty,
                    name,
                    init,
                    on_error,
                    span: start.merge(self.prev_span()),
                })
            }
            _ => {
                let expr = self.expression(sink);
                if self.at(&TokenKind::Colon) {
                    // Callable-headed apply-block (APB-01).
                    return self.apply_block(ApplyHeader::Callable(expr), start, sink);
                }
                if self.eat(&TokenKind::Assign) {
                    if !matches!(
                        expr,
                        Expr::Ident { .. } | Expr::Member { .. } | Expr::Index { .. }
                    ) {
                        // STM-02: targets are identifier, member, or index.
                        self.error_at(
                            sink,
                            codes::SYN005,
                            "assignment target must be a variable, member, or index".to_string(),
                            expr.span(),
                        );
                    }
                    let value = self.expression(sink);
                    let on_error = self.on_error_block_tail(sink);
                    if on_error.is_none() {
                        self.expect(&TokenKind::Newline, "end of line", sink);
                    }
                    return Some(Stmt::Assign {
                        target: expr,
                        value,
                        on_error,
                        span: start.merge(self.prev_span()),
                    });
                }
                let on_error = self.on_error_block_tail(sink);
                if on_error.is_none() {
                    self.expect(&TokenKind::Newline, "end of line", sink);
                }
                Some(Stmt::Expr { expr, on_error })
            }
        }
    }

    /// Apply-block body (APB-01): `':'` at the cursor; each indented line
    /// is one item, shaped by the header kind.
    fn apply_block(
        &mut self,
        header: ApplyHeader,
        start: ByteSpan,
        sink: &mut DiagnosticSink,
    ) -> Option<Stmt> {
        self.bump(); // ':'
        self.expect(&TokenKind::Newline, "end of line", sink);
        let mut items = Vec::new();
        if self.expect(&TokenKind::Indent, "indented apply-block body", sink) {
            loop {
                match self.peek() {
                    TokenKind::Newline => {
                        self.bump();
                    }
                    TokenKind::Dedent | TokenKind::Eof => break,
                    _ => {
                        let item_start = self.span();
                        match &header {
                            ApplyHeader::Callable(_) => {
                                items.push(ApplyItem::Expr(self.expression(sink)));
                            }
                            ApplyHeader::TypeKeyword(_) => {
                                // `name [= expr]` — one variable per line.
                                let Some((name, _)) = self.ident("variable name", sink) else {
                                    self.sync_line();
                                    continue;
                                };
                                let init = if self.eat(&TokenKind::Assign) {
                                    Some(self.expression(sink))
                                } else {
                                    None
                                };
                                items.push(ApplyItem::Binding {
                                    ty: None,
                                    name,
                                    init,
                                    span: item_start.merge(self.prev_span()),
                                });
                            }
                            ApplyHeader::Constant { .. } => {
                                // A full TypedDeclaration per line.
                                let ty = self.type_expr(TypePos::Surface, sink);
                                let Some((name, _)) = self.ident("constant name", sink) else {
                                    self.sync_line();
                                    continue;
                                };
                                let init = if self.eat(&TokenKind::Assign) {
                                    Some(self.expression(sink))
                                } else {
                                    None
                                };
                                items.push(ApplyItem::Binding {
                                    ty: Some(ty),
                                    name,
                                    init,
                                    span: item_start.merge(self.prev_span()),
                                });
                            }
                        }
                        self.expect(&TokenKind::Newline, "end of line", sink);
                    }
                }
            }
            self.eat(&TokenKind::Dedent);
        }
        if items.is_empty() {
            self.error_at(
                sink,
                codes::SYN005,
                "an apply-block applies its header to at least one item".to_string(),
                start,
            );
        }
        Some(Stmt::Apply {
            header,
            items,
            span: start.merge(self.prev_span()),
        })
    }

    /// `later T name = start f()` (ASY-01). The RHS must be a
    /// StartExpression — any other RHS is SYN002 per the boundary rule.
    fn later_binding(&mut self, sink: &mut DiagnosticSink) -> Option<Stmt> {
        let start = self.span();
        self.bump(); // later
        let ty = self.type_expr(TypePos::Surface, sink);
        let Some((name, name_span)) = self.ident("deferred binding name", sink) else {
            self.sync_line();
            return None;
        };
        if !self.expect(&TokenKind::Assign, "'='", sink) {
            self.sync_line();
            return None;
        }
        if !self.eat_kw(Kw::Start) {
            self.error_here(
                sink,
                "the right-hand side of a 'later' binding must be 'start f()'".to_string(),
            );
            self.sync_line();
            return None;
        }
        let call = self.expression(sink);
        self.expect(&TokenKind::Newline, "end of line", sink);
        Some(Stmt::Later {
            ty,
            name,
            name_span,
            call,
            span: start.merge(self.prev_span()),
        })
    }

    /// ERH-02 block form: `… onError:` NEWLINE INDENT handler DEDENT. The
    /// expression ladder leaves `onError ':'` unconsumed for this tail.
    fn on_error_block_tail(&mut self, sink: &mut DiagnosticSink) -> Option<Block> {
        if !(self.at_kw(Kw::OnError) && matches!(self.peek2(), TokenKind::Colon)) {
            return None;
        }
        self.bump(); // onError
        self.bump(); // ':'
        self.expect(&TokenKind::Newline, "end of line", sink);
        Some(self.indented_block(sink))
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

    /// `while <condition>` + indented body (FLW-02 §While).
    fn while_statement(&mut self, sink: &mut DiagnosticSink) -> Option<Stmt> {
        let start = self.span();
        self.bump(); // while
        let cond = self.expression(sink);
        self.expect(&TokenKind::Newline, "end of line", sink);
        let body = self.indented_block(sink);
        Some(Stmt::While {
            cond,
            body,
            span: start.merge(self.prev_span()),
        })
    }

    /// `iterate <binder> in <source> [step <expr>]` + indented body
    /// (FLW-02). A `to` after the source expression makes it the range
    /// form — `to` is iterate-only, never a general operator.
    fn iterate_statement(&mut self, sink: &mut DiagnosticSink) -> Option<Stmt> {
        let start = self.span();
        self.bump(); // iterate
        let Some((binder, binder_span)) = self.ident("iteration variable name", sink) else {
            self.sync_line();
            return None;
        };
        if !self.eat_kw(Kw::In) {
            self.error_here(
                sink,
                "expected 'in' after the iteration variable".to_string(),
            );
            self.sync_line();
            return None;
        }
        let first = self.expression(sink);
        let source = if self.eat_kw(Kw::To) {
            let to = self.expression(sink);
            IterateSource::Range { from: first, to }
        } else {
            IterateSource::Expr(first)
        };
        // `step` is a contextual keyword (03 §4) — identifier text here.
        let step = if self.eat_word("step") {
            Some(self.expression(sink))
        } else {
            None
        };
        self.expect(&TokenKind::Newline, "end of line", sink);
        let body = self.indented_block(sink);
        Some(Stmt::Iterate {
            binder,
            binder_span,
            source,
            step,
            body,
            span: start.merge(self.prev_span()),
        })
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

    /// Parses one TypeExpression (04-type-system.ebnf.md §1).
    fn type_expr(&mut self, pos: TypePos, sink: &mut DiagnosticSink) -> TypeExpr {
        let start = self.span();
        // Inside generic arguments the special powers of the outer position
        // (behavior chains) do not apply; host width suffixes do (a host
        // signature is host throughout).
        let element_pos = match pos {
            TypePos::Host => TypePos::Host,
            _ => TypePos::Surface,
        };
        let base = match self.peek().clone() {
            TokenKind::Ident(name) => {
                self.bump();
                match name.as_str() {
                    "boolean" => BaseType::Boolean,
                    "integer" => {
                        let width = if pos == TypePos::Host && self.at(&TokenKind::Colon) {
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
                        let element = self.type_expr(element_pos, sink);
                        self.expect(&TokenKind::Gt, "'>'", sink);
                        BaseType::List(Box::new(element))
                    }
                    "matrix" => {
                        self.expect(&TokenKind::Lt, "'<' after 'matrix'", sink);
                        let element = self.type_expr(element_pos, sink);
                        self.expect(&TokenKind::Gt, "'>'", sink);
                        BaseType::Matrix(Box::new(element))
                    }
                    "pairs" => {
                        self.expect(&TokenKind::Lt, "'<' after 'pairs'", sink);
                        let key = self.type_expr(element_pos, sink);
                        self.expect(&TokenKind::Comma, "','", sink);
                        let value = self.type_expr(element_pos, sink);
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
        // TYP-05: behavior suffix chain — declaration LHS, list<T> only.
        // The grammar admits any chain; the checker restricts combinations.
        let mut behaviors = Vec::new();
        if pos == TypePos::DeclLhs && matches!(base, BaseType::List(_)) {
            while self.at(&TokenKind::Dot) {
                let name = match self.peek2() {
                    TokenKind::Ident(word) => match word.as_str() {
                        "line" => BehaviorName::Line,
                        "pile" => BehaviorName::Pile,
                        "unique" => BehaviorName::Unique,
                        _ => break,
                    },
                    _ => break,
                };
                let dot = self.span();
                self.bump(); // '.'
                let word_span = self.span();
                self.bump(); // behavior word
                behaviors.push(Behavior {
                    name,
                    span: dot.merge(word_span),
                });
            }
        }
        // TYP-03: a single `?` only. Extra markers are recorded for the
        // checker's SEM009 (grammar admits, checker restricts).
        let optional = self.eat(&TokenKind::Question);
        let mut extra_optionals = Vec::new();
        while self.at(&TokenKind::Question) {
            extra_optionals.push(self.span());
            self.bump();
        }
        TypeExpr {
            base,
            optional,
            extra_optionals,
            behaviors,
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
        self.on_error_expr(sink)
    }

    /// Level 13: `onError` suffix — failure fallback, left-associative
    /// (13-error-handling §2). `onError ':'` is the block form, a statement
    /// tail — the expression ladder leaves it for the statement parser.
    fn on_error_expr(&mut self, sink: &mut DiagnosticSink) -> Expr {
        let mut lhs = self.default_expr(sink);
        while self.at_kw(Kw::OnError) && !matches!(self.peek2(), TokenKind::Colon) {
            self.bump();
            let rhs = self.default_expr(sink);
            let span = lhs.span().merge(rhs.span());
            lhs = Expr::OnError {
                value: Box::new(lhs),
                fallback: Box::new(rhs),
                span,
            };
        }
        lhs
    }

    /// Level 11: `default` — none-coalescing, left-associative (EXP-03).
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

    /// Level 8: equality and identity. `not` here is the BINARY form —
    /// operator position after an operand distinguishes it from unary `not`
    /// (06-expressions §1, position dispatch).
    fn equality_expr(&mut self, sink: &mut DiagnosticSink) -> Expr {
        let mut lhs = self.comparison_expr(sink);
        loop {
            let op = match self.peek() {
                TokenKind::Eq => BinOp::Eq,
                TokenKind::NEq => BinOp::NEq,
                TokenKind::Keyword(Kw::Is) => BinOp::Is,
                TokenKind::Keyword(Kw::Not) => BinOp::NotIs,
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
                TokenKind::LBracket => {
                    // IndexAccess (06 §1).
                    self.bump();
                    self.paren_depth += 1;
                    let index = self.expression(sink);
                    self.paren_depth -= 1;
                    let end = self.span();
                    self.expect(&TokenKind::RBracket, "']'", sink);
                    let span = expr.span().merge(end);
                    expr = Expr::Index {
                        receiver: Box::new(expr),
                        index: Box::new(index),
                        span,
                    };
                }
                TokenKind::Bang => {
                    // Postfix `!` (EXP-03 required-assertion).
                    let bang = self.span();
                    self.bump();
                    let span = expr.span().merge(bang);
                    expr = Expr::NonNone {
                        operand: Box::new(expr),
                        span,
                    };
                }
                _ => break,
            }
        }
        expr
    }

    /// Parses one `{…}` interpolation interior (tokenized by the lexer,
    /// Eof-terminated) as a full Expression (06-expressions §3).
    fn parse_interpolation(&self, tokens: &[Token], sink: &mut DiagnosticSink) -> Expr {
        let mut sub = Parser {
            stream: self.stream,
            tokens,
            pos: 0,
            paren_depth: 0,
        };
        let expr = sub.expression(sink);
        if !matches!(sub.peek(), TokenKind::Eof) {
            sub.error_here(sink, "expected the end of the interpolation".to_string());
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
            TokenKind::Str { parts } => {
                self.bump();
                let mut segments = Vec::new();
                for part in parts {
                    match part {
                        crate::lexer::StrPart::Text(text) => segments.push(StrSeg::Text(text)),
                        crate::lexer::StrPart::Interp {
                            span: seg_span,
                            tokens,
                        } => {
                            let expr = self.parse_interpolation(&tokens, sink);
                            segments.push(StrSeg::Interp {
                                expr,
                                span: seg_span,
                            });
                        }
                    }
                }
                Expr::Str { segments, span }
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
            TokenKind::Keyword(Kw::This) => {
                self.bump();
                Expr::This { span }
            }
            TokenKind::Keyword(Kw::Base) => {
                self.bump();
                Expr::Base { span }
            }
            TokenKind::Keyword(Kw::Error) => {
                // 13 §3: `error(` raise / `error.` member / `error` binding —
                // one primary; the postfix loop builds the rest.
                self.bump();
                Expr::ErrorRef { span }
            }
            TokenKind::Keyword(Kw::Result) => {
                self.bump();
                Expr::ResultRef { span }
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

/// Which position a TypeExpression is being read in — the grammar is the
/// same, but host signatures admit width suffixes (LBS-02) and declaration
/// LHS admits TYP-05 behavior chains.
#[derive(Clone, Copy, PartialEq, Eq)]
enum TypePos {
    Surface,
    Host,
    DeclLhs,
}

/// One top-level form's slot in the FIL-01 section order. Lower ranks come
/// first; `singleton` sections appear at most once.
#[derive(Clone, Copy)]
struct SectionOrder {
    name: &'static str,
    rank: u8,
    singleton: bool,
}

impl SectionOrder {
    fn singleton(name: &'static str, rank: u8) -> Option<SectionOrder> {
        Some(SectionOrder {
            name,
            rank,
            singleton: true,
        })
    }

    fn repeatable(name: &'static str, rank: u8) -> Option<SectionOrder> {
        Some(SectionOrder {
            name,
            rank,
            singleton: false,
        })
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

/// TypeKeyword table (03 §4).
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
            | "matrix"
            | "pairs"
    )
}
