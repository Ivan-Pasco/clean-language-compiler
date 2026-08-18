//! Wire-input serialization: the typed `BlockAST` as the JSON document the
//! handler receives (docs/adr/0003-handler-wire-abi.md). Field names follow
//! `schema/block-ast.md` verbatim (`byteRange` included); `BlockNode` is
//! tagged `{"kind": "line" | "block"}` — the `Statement` variant is not
//! producible at parse time (M3 discovery 2, brief still Ready).
//!
//! Determinism: objects are built in fixed key order and serde_json's
//! `preserve_order` keeps them that way (CMP-02).

use serde_json::{json, Value};

use crate::lexer::{plain_text, TokenKind, TokenStream};
use crate::parser::ast;
use crate::source::ByteSpan;

fn span_json(stream: &TokenStream, span: ByteSpan) -> Value {
    let diag = stream.diag_span(span);
    json!({
        "file": diag.file,
        "line": diag.start.line,
        "column": diag.start.column,
        "byteRange": [span.start, span.end],
    })
}

fn slice(stream: &TokenStream, span: ByteSpan) -> &str {
    stream
        .content
        .get(span.start as usize..span.end as usize)
        .unwrap_or("")
}

/// One lexed token per the schema's `Token` sum type. Layout tokens
/// (newline/indent/dedent/eof) are lexer artifacts, not DSL content, and
/// serialize to `None`.
fn token_json(stream: &TokenStream, token: &crate::lexer::Token) -> Option<Value> {
    let text = || slice(stream, token.span);
    let value = match &token.kind {
        TokenKind::Ident(name) => json!({"kind": "identifier", "text": name}),
        TokenKind::Keyword(_) => json!({"kind": "keyword", "text": text()}),
        TokenKind::Int(value) => match u64::try_from(*value) {
            Ok(fits) => json!({"kind": "integer", "value": fits}),
            // Out of JSON's integer range — carry the source text.
            Err(_) => json!({"kind": "integer", "text": text()}),
        },
        TokenKind::Number(raw) => match raw.parse::<f64>() {
            Ok(fits) => json!({"kind": "number", "value": fits}),
            Err(_) => json!({"kind": "number", "text": raw}),
        },
        TokenKind::Str { parts } => {
            let text = plain_text(parts).unwrap_or_else(|| slice(stream, token.span).to_string());
            json!({"kind": "string", "text": text})
        }
        TokenKind::Newline | TokenKind::Indent | TokenKind::Dedent | TokenKind::Eof => return None,
        // Punctuation (§8) — the schema's `Symbol`.
        _ => json!({"kind": "symbol", "text": text()}),
    };
    let mut object = value;
    object
        .as_object_mut()
        .expect("token json is an object")
        .insert("span".to_string(), span_json(stream, token.span));
    Some(object)
}

/// Argument expressions reach the handler "already parsed" — literals and
/// names structurally, anything richer as its source text (`opaque`).
fn expr_json(stream: &TokenStream, expr: &ast::Expr) -> Value {
    let mut value = match expr {
        ast::Expr::Int { value, .. } => match u64::try_from(*value) {
            Ok(fits) => json!({"kind": "literal_integer", "value": fits}),
            Err(_) => json!({"kind": "opaque", "text": slice(stream, expr.span())}),
        },
        ast::Expr::Number { text, .. } => match text.parse::<f64>() {
            Ok(fits) => json!({"kind": "literal_number", "value": fits}),
            Err(_) => json!({"kind": "opaque", "text": text}),
        },
        ast::Expr::Str { segments, .. } => {
            let plain: Option<String> = segments
                .iter()
                .map(|seg| match seg {
                    ast::StrSeg::Text(text) => Some(text.as_str()),
                    ast::StrSeg::Interp { .. } => None,
                })
                .collect();
            match plain {
                Some(text) => json!({"kind": "literal_string", "value": text}),
                None => json!({"kind": "opaque", "text": slice(stream, expr.span())}),
            }
        }
        ast::Expr::Bool { value, .. } => json!({"kind": "literal_boolean", "value": value}),
        ast::Expr::Ident { name, .. } => json!({"kind": "variable", "name": name}),
        _ => json!({"kind": "opaque", "text": slice(stream, expr.span())}),
    };
    value
        .as_object_mut()
        .expect("expr json is an object")
        .insert("span".to_string(), span_json(stream, expr.span()));
    value
}

fn arg_json(stream: &TokenStream, arg: &ast::BlockArg) -> Value {
    match arg {
        ast::BlockArg::Positional(expr) => {
            json!({"kind": "positional", "value": expr_json(stream, expr)})
        }
        ast::BlockArg::Keyword { name, value, span } => json!({
            "kind": "keyword",
            "name": name,
            "value": expr_json(stream, value),
            "span": span_json(stream, *span),
        }),
    }
}

fn node_json(stream: &TokenStream, node: &ast::BlockNode) -> Value {
    match node {
        ast::BlockNode::Line(line) => json!({
            "kind": "line",
            "tokens": line
                .tokens
                .iter()
                .filter_map(|t| token_json(stream, t))
                .collect::<Vec<Value>>(),
            "span": span_json(stream, line.span),
        }),
        ast::BlockNode::Block(nested) => {
            let mut object = block_json(stream, nested);
            let map = object.as_object_mut().expect("block json is an object");
            // Tag nested blocks; the shift preserves `kind`-first ordering
            // only informally — key presence is what the ABI fixes.
            map.insert("kind".to_string(), Value::String("block".to_string()));
            object
        }
    }
}

/// The complete wire document for one block occurrence.
pub fn block_json(stream: &TokenStream, block: &ast::BlockAst) -> Value {
    json!({
        "name": block.name,
        "arguments": block
            .arguments
            .iter()
            .map(|a| arg_json(stream, a))
            .collect::<Vec<Value>>(),
        "body": block
            .body
            .iter()
            .map(|n| node_json(stream, n))
            .collect::<Vec<Value>>(),
        "attributes": Vec::<Value>::new(),
        "span": span_json(stream, block.span),
    })
}
