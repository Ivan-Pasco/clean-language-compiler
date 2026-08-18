//! Wire-output intake: the handler's `{"ir", "diagnostics"}` envelope
//! (docs/adr/0003-handler-wire-abi.md) parsed and lowered onto the surface
//! AST, where the spliced program re-enters resolve/type-check so the
//! returned IR is "validated against the surrounding compilation unit"
//! (chapter 21 §21.4) by the same machinery that checks written code.
//!
//! Every lowered node carries the block header's span by default (§21.5);
//! `with_span` re-anchors to a real sub-span of the block. A shape the
//! closed §21.4 builder surface does not produce is malformed IR
//! (`BLOCK004`); more than 500 000 nodes in one envelope is the library's
//! resource limit (`LIB014`, chapter 21 §21.7).

use serde_json::Value;

use crate::parser::ast;
use crate::source::ByteSpan;

/// Chapter 21 §21.7: IR nodes emitted by a single invocation.
pub const MAX_IR_NODES: u64 = 500_000;

/// ICE prevention, implementation-defined (DISCOVERIES-M5): the lowerer is
/// recursive, so a pathologically deep tree would overflow the stack long
/// before the node budget fires. No legitimate §21.4 composition nests this
/// deep. Sized so the guard fires within a 2 MiB thread stack even with
/// debug-build frames.
pub const MAX_IR_DEPTH: u32 = 128;

/// A BLK-03 diagnostic collected by the handler and returned in the
/// envelope.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HandlerDiag {
    pub severity: String,
    pub code: String,
    pub message: String,
    pub span: WireSpan,
}

/// The schema's `Span` as serialized on the wire.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireSpan {
    pub file: String,
    pub line: u32,
    pub column: u32,
    #[serde(rename = "byteRange")]
    pub byte_range: (u32, u32),
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Envelope {
    pub ir: Value,
    #[serde(default)]
    pub diagnostics: Vec<HandlerDiag>,
}

pub fn parse_envelope(text: &str) -> Result<Envelope, String> {
    serde_json::from_str(text).map_err(|e| e.to_string())
}

/// Why a lowering stopped: the IR is malformed (`BLOCK004`) or the node
/// budget is exhausted (`LIB014`).
pub enum LowerError {
    Malformed(String),
    NodeLimit,
}

pub struct Lowerer {
    /// Default anchor: the block header (§21.5); the range `with_span` may
    /// re-anchor into is the block's full extent, header and body.
    block_extent: ByteSpan,
    /// The block's file — the only file `with_span` may re-anchor into.
    file: String,
    nodes: u64,
    depth: u32,
}

type Lower<T> = Result<T, LowerError>;

impl Lowerer {
    pub fn new(block_extent: ByteSpan, file: &str) -> Self {
        Self {
            block_extent,
            file: file.to_string(),
            nodes: 0,
            depth: 0,
        }
    }

    fn tick(&mut self) -> Lower<()> {
        self.nodes += 1;
        if self.nodes > MAX_IR_NODES {
            return Err(LowerError::NodeLimit);
        }
        Ok(())
    }

    /// Guards every recursive descent; an error is terminal for the whole
    /// lowering, so the counter only unwinds on success.
    fn descend<T>(&mut self, f: impl FnOnce(&mut Self) -> Lower<T>) -> Lower<T> {
        self.depth += 1;
        if self.depth > MAX_IR_DEPTH {
            return Err(malformed(&format!(
                "nesting exceeds the depth limit of {MAX_IR_DEPTH}"
            )));
        }
        let result = f(self);
        self.depth -= 1;
        result
    }

    fn kind<'v>(&self, node: &'v Value) -> Lower<(&'v str, &'v Value)> {
        let kind = node
            .get("kind")
            .and_then(Value::as_str)
            .ok_or_else(|| malformed("an IR node has no `kind`"))?;
        Ok((kind, node))
    }

    /// `with_span` re-anchors into a real sub-span of the block
    /// (chapter 21 §21.5); anything outside the block's file or range is a
    /// synthetic span and malformed IR.
    fn anchored_span(&self, node: &Value) -> Lower<ByteSpan> {
        let wire: WireSpan = serde_json::from_value(
            node.get("span")
                .cloned()
                .ok_or_else(|| malformed("`with_span` carries no `span`"))?,
        )
        .map_err(|e| malformed(&format!("`with_span` span does not parse: {e}")))?;
        let (start, end) = wire.byte_range;
        if wire.file != self.file
            || start > end
            || start < self.block_extent.start
            || end > self.block_extent.end
        {
            return Err(malformed(
                "`with_span` does not name a real span inside the block",
            ));
        }
        Ok(ByteSpan::new(start, end))
    }

    /// Top-level lowering: declarations only. `concat` splices, `empty`
    /// contributes nothing.
    pub fn items(&mut self, node: &Value) -> Lower<Vec<ast::Item>> {
        self.items_at(node, self.block_extent)
    }

    fn items_at(&mut self, node: &Value, span: ByteSpan) -> Lower<Vec<ast::Item>> {
        self.descend(|s| s.items_at_inner(node, span))
    }

    fn items_at_inner(&mut self, node: &Value, span: ByteSpan) -> Lower<Vec<ast::Item>> {
        self.tick()?;
        let (kind, node) = self.kind(node)?;
        match kind {
            "empty" => Ok(Vec::new()),
            "concat" => {
                let mut items = Vec::new();
                for fragment in self.list(node, "fragments")? {
                    items.extend(self.items_at(fragment, span)?);
                }
                Ok(items)
            }
            "with_span" => {
                let span = self.anchored_span(node)?;
                let inner = self.field(node, "node")?;
                self.items_at(inner, span)
            }
            "class" => Ok(vec![ast::Item::Class(self.class(node, span)?)]),
            "function" => Ok(vec![ast::Item::Functions(vec![self.function(node, span)?])]),
            other => Err(malformed(&format!(
                "IR fragment `{other}` is not a declaration"
            ))),
        }
    }

    fn class(&mut self, node: &Value, span: ByteSpan) -> Lower<ast::ClassDecl> {
        let name = self.string(node, "name")?;
        let mut fields = Vec::new();
        for field in self.list(node, "fields")? {
            fields.push(self.field_decl(field, span)?);
        }
        let mut functions = Vec::new();
        for method in self.list(node, "methods")? {
            functions.push(self.method(method, span)?);
        }
        Ok(ast::ClassDecl {
            name,
            parent: None,
            capabilities: Vec::new(),
            fields,
            always: None,
            constructors: Vec::new(),
            functions,
            span,
        })
    }

    /// `Field` record or `ir.field(...)` node — the schema carries both
    /// spellings (§21.4 builder and the record type behind it).
    fn field_decl(&mut self, node: &Value, span: ByteSpan) -> Lower<ast::Field> {
        self.descend(|s| s.field_decl_inner(node, span))
    }

    fn field_decl_inner(&mut self, node: &Value, span: ByteSpan) -> Lower<ast::Field> {
        self.tick()?;
        if node.get("kind").and_then(Value::as_str) == Some("with_span") {
            let anchored = self.anchored_span(node)?;
            let inner = self.field(node, "node")?;
            return self.field_decl(inner, anchored);
        }
        Ok(ast::Field {
            ty: self.type_ref(self.field(node, "type")?, span)?,
            name: self.string(node, "name")?,
            init: None,
            // The §21.4 builder surface has no visibility parameter; a
            // companion type exists to be used, so emitted fields are
            // public (adoption, DISCOVERIES-M5).
            public: true,
            span,
        })
    }

    fn method(&mut self, node: &Value, span: ByteSpan) -> Lower<ast::Function> {
        self.tick()?;
        self.callable(node, span)
    }

    fn function(&mut self, node: &Value, span: ByteSpan) -> Lower<ast::Function> {
        self.callable(node, span)
    }

    fn callable(&mut self, node: &Value, span: ByteSpan) -> Lower<ast::Function> {
        let name = self.string(node, "name")?;
        let mut params = Vec::new();
        for param in self.list(node, "params")? {
            self.tick()?;
            let default = match param.get("default") {
                None | Some(Value::Null) => None,
                Some(value) => Some(self.expr(value, span)?),
            };
            params.push(ast::Param {
                ty: self.type_ref(self.field(param, "type")?, span)?,
                name: self.string(param, "name")?,
                default,
                span,
            });
        }
        let ret = self.type_ref(self.field(node, "return")?, span)?;
        let statements = self.stmts(self.field(node, "body")?, span)?;
        Ok(ast::Function {
            ret,
            name,
            params,
            background: false,
            public: false,
            body: ast::FunctionBody {
                statements,
                ..Default::default()
            },
            span,
        })
    }

    fn stmts(&mut self, node: &Value, span: ByteSpan) -> Lower<Vec<ast::Stmt>> {
        self.descend(|s| s.stmts_inner(node, span))
    }

    fn stmts_inner(&mut self, node: &Value, span: ByteSpan) -> Lower<Vec<ast::Stmt>> {
        self.tick()?;
        let (kind, node) = self.kind(node)?;
        match kind {
            "empty" => Ok(Vec::new()),
            "block" => {
                let mut stmts = Vec::new();
                for stmt in self.list(node, "statements")? {
                    stmts.extend(self.stmts(stmt, span)?);
                }
                Ok(stmts)
            }
            "concat" => {
                let mut stmts = Vec::new();
                for fragment in self.list(node, "fragments")? {
                    stmts.extend(self.stmts(fragment, span)?);
                }
                Ok(stmts)
            }
            "with_span" => {
                let span = self.anchored_span(node)?;
                let inner = self.field(node, "node")?;
                self.stmts(inner, span)
            }
            "return" => {
                let value = match node.get("expression") {
                    None | Some(Value::Null) => None,
                    Some(expression) => Some(self.expr(expression, span)?),
                };
                Ok(vec![ast::Stmt::Return { value, span }])
            }
            "assign" => Ok(vec![ast::Stmt::Assign {
                target: ast::Expr::Ident {
                    name: self.string(node, "name")?,
                    span,
                },
                value: self.expr(self.field(node, "expression")?, span)?,
                on_error: None,
                span,
            }]),
            "if" => {
                let cond = self.expr(self.field(node, "condition")?, span)?;
                let then = self.stmts(self.field(node, "then")?, span)?;
                let els = match node.get("else") {
                    None | Some(Value::Null) => None,
                    Some(other) => Some(self.stmts(other, span)?),
                };
                Ok(vec![ast::Stmt::If {
                    cond,
                    then,
                    else_ifs: Vec::new(),
                    els,
                    span,
                }])
            }
            // The expression builders are valid in statement position as
            // discarded-result statements.
            "call" | "literal_integer" | "literal_string" | "variable" | "field_access" => {
                Ok(vec![ast::Stmt::Expr {
                    expr: self.expr(node, span)?,
                    on_error: None,
                }])
            }
            other => Err(malformed(&format!(
                "IR fragment `{other}` is not a statement"
            ))),
        }
    }

    fn expr(&mut self, node: &Value, span: ByteSpan) -> Lower<ast::Expr> {
        self.descend(|s| s.expr_inner(node, span))
    }

    fn expr_inner(&mut self, node: &Value, span: ByteSpan) -> Lower<ast::Expr> {
        self.tick()?;
        let (kind, node) = self.kind(node)?;
        match kind {
            "with_span" => {
                let span = self.anchored_span(node)?;
                let inner = self.field(node, "node")?;
                self.expr(inner, span)
            }
            "literal_integer" => {
                let value = node
                    .get("value")
                    .and_then(Value::as_i64)
                    .ok_or_else(|| malformed("`literal_integer` has no integer `value`"))?;
                if value < 0 {
                    Ok(ast::Expr::Unary {
                        op: ast::UnOp::Neg,
                        operand: Box::new(ast::Expr::Int {
                            value: value.unsigned_abs() as u128,
                            span,
                        }),
                        span,
                    })
                } else {
                    Ok(ast::Expr::Int {
                        value: value as u128,
                        span,
                    })
                }
            }
            "literal_string" => Ok(ast::Expr::Str {
                segments: vec![ast::StrSeg::Text(self.string(node, "value")?)],
                span,
            }),
            "variable" => Ok(ast::Expr::Ident {
                name: self.string(node, "name")?,
                span,
            }),
            "field_access" => Ok(ast::Expr::Member {
                receiver: Box::new(self.expr(self.field(node, "receiver")?, span)?),
                name: self.string(node, "field")?,
                span,
            }),
            "call" => {
                let qualified = self.string(node, "qualified_name")?;
                let mut segments = qualified.split('.');
                let first = segments
                    .next()
                    .ok_or_else(|| malformed("`call` has an empty `qualified_name`"))?;
                let mut callee = ast::Expr::Ident {
                    name: first.to_string(),
                    span,
                };
                for segment in segments {
                    callee = ast::Expr::Member {
                        receiver: Box::new(callee),
                        name: segment.to_string(),
                        span,
                    };
                }
                let mut args = Vec::new();
                for arg in self.list(node, "arguments")? {
                    args.push(self.expr(arg, span)?);
                }
                Ok(ast::Expr::Call {
                    callee: Box::new(callee),
                    args,
                    span,
                })
            }
            other => Err(malformed(&format!(
                "IR fragment `{other}` is not an expression"
            ))),
        }
    }

    /// `TypeRef` constructors (schema/block-ast.md supporting types).
    fn type_ref(&mut self, node: &Value, span: ByteSpan) -> Lower<ast::TypeExpr> {
        self.descend(|s| s.type_ref_inner(node, span))
    }

    fn type_ref_inner(&mut self, node: &Value, span: ByteSpan) -> Lower<ast::TypeExpr> {
        self.tick()?;
        let (kind, node) = self.kind(node)?;
        let base = match kind {
            "boolean" => ast::BaseType::Boolean,
            "integer" => ast::BaseType::Integer(None),
            "number" => ast::BaseType::Number,
            "string" => ast::BaseType::String_,
            "bytes" => ast::BaseType::Bytes,
            "datetime" => ast::BaseType::Datetime,
            "any" => ast::BaseType::Any,
            "void" => ast::BaseType::Void,
            "list" => ast::BaseType::List(Box::new(self.type_ref(self.field(node, "of")?, span)?)),
            "matrix" => {
                ast::BaseType::Matrix(Box::new(self.type_ref(self.field(node, "of")?, span)?))
            }
            "pairs" => ast::BaseType::Pairs(
                Box::new(self.type_ref(self.field(node, "key")?, span)?),
                Box::new(self.type_ref(self.field(node, "value")?, span)?),
            ),
            "optional" => {
                let mut inner = self.type_ref(self.field(node, "of")?, span)?;
                inner.optional = true;
                return Ok(inner);
            }
            "class" => ast::BaseType::Named(self.string(node, "name")?),
            other => {
                return Err(malformed(&format!(
                    "IR fragment `{other}` is not a TypeRef"
                )))
            }
        };
        Ok(ast::TypeExpr {
            base,
            optional: false,
            extra_optionals: Vec::new(),
            behaviors: Vec::new(),
            span,
        })
    }

    fn field<'v>(&self, node: &'v Value, key: &str) -> Lower<&'v Value> {
        node.get(key)
            .ok_or_else(|| malformed(&format!("an IR node is missing `{key}`")))
    }

    fn list<'v>(&self, node: &'v Value, key: &str) -> Lower<&'v Vec<Value>> {
        self.field(node, key)?
            .as_array()
            .ok_or_else(|| malformed(&format!("`{key}` is not a list")))
    }

    fn string(&self, node: &Value, key: &str) -> Lower<String> {
        Ok(self
            .field(node, key)?
            .as_str()
            .ok_or_else(|| malformed(&format!("`{key}` is not a string")))?
            .to_string())
    }
}

fn malformed(reason: &str) -> LowerError {
    LowerError::Malformed(reason.to_string())
}
