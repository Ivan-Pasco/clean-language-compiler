//! AST for the Milestone 1 surface. Every node carries a real source span
//! (§14.4.2[3]: no synthetic spans without a source anchor).

use crate::lexer::Token;
use crate::source::ByteSpan;

#[derive(Debug)]
pub struct SourceFile {
    pub path: String,
    pub items: Vec<Item>,
}

#[derive(Debug)]
pub enum Item {
    /// `import:` block (17 §1) — module-name entries.
    Imports(Vec<ImportEntry>),
    /// Standalone `import "path"` (17 §3).
    FileImport { path: String, span: ByteSpan },
    /// `source:` provenance block (19 §4 / AIM-03).
    Source(SourceSection),
    /// `constant:` section (08 §2) — TypedDeclaration items.
    Constants(Vec<ConstantDecl>),
    /// `state:` block (20 §1).
    State(StateSection),
    /// LBS-02 `host interface` block.
    HostInterface(HostInterface),
    /// `functions:` block (08-file-structure `FunctionsBlock`).
    Functions(Vec<Function>),
    /// Bare top-level `FunctionDeclaration` (08 §2 TopLevelCallable).
    Function(Function),
    /// `constant function` (09 §4).
    ConstantFunction(ConstantFunction),
    /// `compiletime function` (21 §1).
    CompiletimeFunction(CompiletimeFunction),
    /// `handles block "<name>" with <handler>` (21 §1).
    HandlesBlock(HandlesBlock),
    /// `class` declaration (14 §1).
    Class(ClassDecl),
    /// `can Name:` capability declaration (14 §4).
    Capability(CapabilityDecl),
    /// `watch <target>:` observer (20 §5).
    Watch(WatchBlock),
    /// `tests:` section (11 §1).
    Tests(Vec<TestDecl>),
    /// `start:` section.
    Start(Block),
    /// A library-registered block (08 §3) — the typed BlockAST node the
    /// pass-6 handler receives (21 §21.3, schema/block-ast.md). Expansion
    /// stays pass-through until M5.
    LibraryBlock(BlockAst),
}

/// The parsed block a handler receives (schema/block-ast.md). `span` is
/// anchored to the block header per the schema.
#[derive(Debug)]
pub struct BlockAst {
    /// Qualified identifier (`data`, `data.query`).
    pub name: String,
    pub arguments: Vec<BlockArg>,
    pub body: Vec<BlockNode>,
    /// Modifier annotations. Distinguishing attribute lines from DSL body
    /// lines is under-specified (see docs/DISCOVERIES-M3.md); the parser
    /// leaves this empty until the schema pins the shape down.
    pub attributes: Vec<BlockAttribute>,
    pub span: ByteSpan,
}

/// Sum type over block-body children (schema/block-ast.md). The
/// `Statement` variant materialises during pass-6 expansion (M5); at parse
/// time every non-block line is preserved as a `BlockLine`.
#[derive(Debug)]
pub enum BlockNode {
    Block(BlockAst),
    Line(BlockLine),
}

/// A structured DSL line — the handler tokenises it itself.
#[derive(Debug)]
pub struct BlockLine {
    pub tokens: Vec<Token>,
    pub span: ByteSpan,
}

/// Positional or keyword block argument (schema/block-ast.md).
#[derive(Debug)]
pub enum BlockArg {
    Positional(Expr),
    Keyword {
        name: String,
        value: Expr,
        span: ByteSpan,
    },
}

/// Block modifier annotation (schema/block-ast.md).
#[derive(Debug)]
pub struct BlockAttribute {
    pub name: String,
    pub arguments: Vec<Expr>,
    pub span: ByteSpan,
}

/// One entry in an `import:` block (17 §2): dotted module path with an
/// optional alias. `math.sqrt as s` keeps the whole path; resolution is
/// the framework's job (MOD-03).
#[derive(Debug)]
pub struct ImportEntry {
    pub path: Vec<String>,
    pub alias: Option<String>,
    pub span: ByteSpan,
}

/// `source:` block fields (AIM-03): exactly `spec` and `version`, both
/// strings. The grammar admits repeats; the checker enforces one-of-each.
#[derive(Debug)]
pub struct SourceSection {
    pub fields: Vec<SourceField>,
    pub span: ByteSpan,
}

#[derive(Debug)]
pub struct SourceField {
    /// `spec` or `version` — the closed field set (DOC-18).
    pub key: String,
    pub value: String,
    pub span: ByteSpan,
}

/// One item of the `constant:` section — a TypedDeclaration.
#[derive(Debug)]
pub struct ConstantDecl {
    pub ty: TypeExpr,
    pub name: String,
    pub init: Option<Expr>,
    pub span: ByteSpan,
}

/// `state:` block body (20 §1): declarations, computed:, rules: — freely
/// interleaved per the chapter's resolved ordering decision.
#[derive(Debug)]
pub struct StateSection {
    pub members: Vec<StateMember>,
    pub span: ByteSpan,
}

#[derive(Debug)]
pub enum StateMember {
    Var(StateVar),
    Computed(Vec<ComputedDecl>),
    Rules(Vec<Expr>),
}

/// A state variable (SMG-01) — initialiser required — with its guard
/// clauses (SMG-02) in written order.
#[derive(Debug)]
pub struct StateVar {
    pub ty: TypeExpr,
    pub name: String,
    pub init: Expr,
    pub guards: Vec<GuardClause>,
    pub span: ByteSpan,
}

/// `guard <expr> else "<message>"` (SMG-02).
#[derive(Debug)]
pub struct GuardClause {
    pub cond: Expr,
    pub message: String,
    pub span: ByteSpan,
}

/// One `computed:` entry (SMG-05): typed name with an indented body.
#[derive(Debug)]
pub struct ComputedDecl {
    pub ty: TypeExpr,
    pub name: String,
    pub body: Block,
    pub span: ByteSpan,
}

/// `watch <target>:` (20 §5) — one identifier or a parenthesized list.
#[derive(Debug)]
pub struct WatchBlock {
    pub targets: Vec<(String, ByteSpan)>,
    pub body: Block,
    pub span: ByteSpan,
}

/// One `tests:` entry (11 §1–§3).
#[derive(Debug)]
pub enum TestDecl {
    /// `"description": expression`
    Named {
        description: String,
        assertion: Expr,
        span: ByteSpan,
    },
    /// `expression`
    Anonymous { assertion: Expr, span: ByteSpan },
    /// `"description"` + indented body with at least one `assert`.
    Block {
        description: String,
        body: Block,
        span: ByteSpan,
    },
}

/// `constant function name(params) [returns T]` + body (09 §4).
#[derive(Debug)]
pub struct ConstantFunction {
    pub name: String,
    pub params: Vec<Param>,
    pub ret: Option<TypeExpr>,
    pub body: FunctionBody,
    pub span: ByteSpan,
}

/// `compiletime function name(params) returns T` + body (21 §1 / BLK-01).
#[derive(Debug)]
pub struct CompiletimeFunction {
    pub name: String,
    pub params: Vec<Param>,
    pub ret: Option<TypeExpr>,
    pub body: FunctionBody,
    pub span: ByteSpan,
}

/// `handles block "<name>" with <handler>` (21 §1).
#[derive(Debug)]
pub struct HandlesBlock {
    pub block_name: String,
    pub handler: String,
    pub span: ByteSpan,
}

/// `can Name:` — a named contract of arrow-return signatures (14 §4).
#[derive(Debug)]
pub struct CapabilityDecl {
    pub name: String,
    pub signatures: Vec<CapabilitySig>,
    pub span: ByteSpan,
}

/// `name(params) -> ReturnType` (09 §5 / FNC-03). No body (CLS-03).
#[derive(Debug)]
pub struct CapabilitySig {
    pub name: String,
    pub params: Vec<Param>,
    pub ret: TypeExpr,
    /// An indented body under the signature — illegal (CLS-03: pure
    /// contracts); parsed and discarded so the checker can report SEM014.
    pub body_span: Option<ByteSpan>,
    pub span: ByteSpan,
}

/// A `before:` / `after:` / `always:` contract block (10 §1): one boolean
/// expression per line.
#[derive(Debug)]
pub struct ContractBlock {
    pub exprs: Vec<Expr>,
    pub span: ByteSpan,
}

#[derive(Debug)]
pub struct HostInterface {
    /// Kebab-case interface name as written (`routing`, `session-envelope`).
    pub name: String,
    pub version: String,
    pub worlds: Vec<String>,
    pub functions: Vec<HostFunction>,
    pub span: ByteSpan,
}

#[derive(Debug)]
pub struct HostFunction {
    /// camelCase Clean name; kebab-cased when matched against WIT.
    pub name: String,
    pub params: Vec<HostParam>,
    /// `None` means no `returns` clause — a void host function.
    pub ret: Option<TypeExpr>,
    pub description: String,
    pub span: ByteSpan,
}

#[derive(Debug)]
pub struct HostParam {
    pub name: String,
    pub ty: TypeExpr,
    pub span: ByteSpan,
}

#[derive(Debug)]
pub struct Function {
    pub ret: TypeExpr,
    pub name: String,
    pub params: Vec<Param>,
    /// ASY-02 `background` modifier — postfix, after the parameter list.
    pub background: bool,
    /// True when declared inside a `public:` wrapper (MOD-02).
    pub public: bool,
    pub body: FunctionBody,
    pub span: ByteSpan,
}

/// A function body (09 §2 + 19 §3 + 10 §2): optional metadata prelude,
/// then the statement sequence. The parser is permissive about prelude
/// ordering; the checker owns the placement rules.
#[derive(Debug, Default)]
pub struct FunctionBody {
    pub description: Option<String>,
    /// `input` block parameters — equivalent to ParameterList entries
    /// (FNC-04).
    pub input: Vec<Param>,
    /// `intent "…"` lines (AIM-02).
    pub intents: Vec<(String, ByteSpan)>,
    /// `spec "…"` lines (AIM-01).
    pub specs: Vec<(String, ByteSpan)>,
    /// `before:` precondition block (CTR-01).
    pub before: Option<ContractBlock>,
    /// `after:` postcondition block (CTR-02).
    pub after: Option<ContractBlock>,
    pub statements: Block,
}

#[derive(Debug)]
pub struct Param {
    pub ty: TypeExpr,
    pub name: String,
    pub default: Option<Expr>,
    pub span: ByteSpan,
}

#[derive(Debug)]
pub struct ClassDecl {
    pub name: String,
    /// `is Parent` (CLS-02, single inheritance).
    pub parent: Option<(String, ByteSpan)>,
    /// `can C1, C2, …` claim clause (CLS-03).
    pub capabilities: Vec<(String, ByteSpan)>,
    pub fields: Vec<Field>,
    /// `always:` invariant block (CTR-03) — at most one.
    pub always: Option<ContractBlock>,
    pub constructors: Vec<Constructor>,
    /// `functions:` block members.
    pub functions: Vec<Function>,
    pub span: ByteSpan,
}

#[derive(Debug)]
pub struct Field {
    pub ty: TypeExpr,
    pub name: String,
    pub init: Option<Expr>,
    /// True when declared inside a `public:` wrapper (MOD-02).
    pub public: bool,
    pub span: ByteSpan,
}

/// `constructor(params)` + body (14 §3). Overloading allowed.
#[derive(Debug)]
pub struct Constructor {
    pub params: Vec<Param>,
    pub body: Block,
    pub span: ByteSpan,
}

pub type Block = Vec<Stmt>;

/// The source of an `iterate` loop (FLW-02): a range (`a to b`) or any
/// iterable expression. `to` is iterate-only, never a general operator.
#[derive(Debug)]
pub enum IterateSource {
    Range { from: Expr, to: Expr },
    Expr(Expr),
}

#[derive(Debug)]
pub enum Stmt {
    /// `TypeExpression Identifier [= Expression]` (STM-01). `on_error`
    /// carries the ERH-02 block form terminating the statement
    /// (`T x = expr onError:` + indented handler).
    VarDecl {
        ty: TypeExpr,
        name: String,
        name_span: ByteSpan,
        init: Option<Expr>,
        on_error: Option<Block>,
        span: ByteSpan,
    },
    /// `target = value` (STM-02 — statement, never expression).
    Assign {
        target: Expr,
        value: Expr,
        on_error: Option<Block>,
        span: ByteSpan,
    },
    /// `return [expr]` (STM-03).
    Return { value: Option<Expr>, span: ByteSpan },
    /// Expression whose result is discarded; `on_error` is the ERH-02
    /// block form (`expr onError:` + indented handler).
    Expr { expr: Expr, on_error: Option<Block> },
    /// `if` / `else if` / `else` (FLW-01).
    If {
        cond: Expr,
        then: Block,
        else_ifs: Vec<(Expr, Block)>,
        els: Option<Block>,
        span: ByteSpan,
    },
    /// `iterate <binder> in <source> [step <expr>]` (FLW-02).
    Iterate {
        binder: String,
        binder_span: ByteSpan,
        source: IterateSource,
        step: Option<Expr>,
        body: Block,
        span: ByteSpan,
    },
    /// `while <condition>` (FLW-02 §While).
    While {
        cond: Expr,
        body: Block,
        span: ByteSpan,
    },
    /// `break` (FLW-03).
    Break { span: ByteSpan },
    /// `continue` (FLW-03).
    Continue { span: ByteSpan },
    /// `print:` block (STM prose; SYN008 checked at parse).
    Print { items: Vec<Expr>, span: ByteSpan },
    /// `assert <expr>` (11 §3) — block-test bodies; the checker owns the
    /// placement rule.
    Assert { expr: Expr, span: ByteSpan },
    /// Apply-block (APB-01): `header:` + one item per indented line.
    Apply {
        header: ApplyHeader,
        items: Vec<ApplyItem>,
        span: ByteSpan,
    },
    /// `later T name = start f()` deferred binding (ASY-01).
    Later {
        ty: TypeExpr,
        name: String,
        name_span: ByteSpan,
        /// The call following `start` — the only legal RHS shape.
        call: Expr,
        span: ByteSpan,
    },
    /// `background f() [onError …]` (ASY-01/ASY-03). A suffix handler
    /// folds into `call` as `Expr::OnError`; the block form is `on_error`.
    Background {
        call: Expr,
        on_error: Option<Block>,
        span: ByteSpan,
    },
    /// `reset <target>` (20 §6).
    Reset { target: ResetTarget, span: ByteSpan },
}

/// Apply-block header kinds (05 §1) — the parser dispatches the body's
/// item shape on this.
#[derive(Debug)]
pub enum ApplyHeader {
    /// `items.add:` — any callable expression.
    Callable(Expr),
    /// `integer:` — grouped declarations in that type.
    TypeKeyword(TypeExpr),
    /// `constant:` — each item a full TypedDeclaration.
    Constant { span: ByteSpan },
}

/// One apply-block body line (05 §1).
#[derive(Debug)]
pub enum ApplyItem {
    /// Callable-style header: one call argument.
    Expr(Expr),
    /// Declaration-style headers: `name [= expr]` (type-keyword header,
    /// `ty` empty) or a full TypedDeclaration (`constant:` header).
    Binding {
        ty: Option<TypeExpr>,
        name: String,
        init: Option<Expr>,
        span: ByteSpan,
    },
}

/// `reset` target (20 §6): one variable, or the whole state in scope.
#[derive(Debug)]
pub enum ResetTarget {
    State,
    Var { name: String, span: ByteSpan },
}

/// One segment of a string literal (06-expressions §3): literal text or a
/// `{expr}` interpolation, in source order.
#[derive(Debug)]
pub enum StrSeg {
    Text(String),
    Interp { expr: Expr, span: ByteSpan },
}

#[derive(Debug)]
pub enum Expr {
    Int {
        value: u128,
        span: ByteSpan,
    },
    /// Float-shaped literal; the `number` type's semantics land in M6.
    Number {
        text: String,
        span: ByteSpan,
    },
    Str {
        segments: Vec<StrSeg>,
        span: ByteSpan,
    },
    Bool {
        value: bool,
        span: ByteSpan,
    },
    NoneLit {
        span: ByteSpan,
    },
    Ident {
        name: String,
        span: ByteSpan,
    },
    /// `this` in class-method scope (CLS prose §5).
    This {
        span: ByteSpan,
    },
    /// `base` — parent-constructor callee in class scope (CLS-02).
    Base {
        span: ByteSpan,
    },
    /// `error` as the bound Error value inside a handler (ERH-04), or as
    /// the raise/emission callee when a Call wraps it (ERH-01, BLK-03) —
    /// the parser dispatches on the following token per 13 §3.
    ErrorRef {
        span: ByteSpan,
    },
    /// `result` inside an `after:` contract block (CTR-02).
    ResultRef {
        span: ByteSpan,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        span: ByteSpan,
    },
    Member {
        receiver: Box<Expr>,
        name: String,
        span: ByteSpan,
    },
    /// `receiver[index]` (06 §1 IndexAccess).
    Index {
        receiver: Box<Expr>,
        index: Box<Expr>,
        span: ByteSpan,
    },
    /// Postfix `!` — required-assertion on an optional (EXP-03).
    NonNone {
        operand: Box<Expr>,
        span: ByteSpan,
    },
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        span: ByteSpan,
    },
    Unary {
        op: UnOp,
        operand: Box<Expr>,
        span: ByteSpan,
    },
    /// `value onError fallback` — suffix form, level 13 (ERH-02). The block
    /// form is a statement tail, `Stmt`-side.
    OnError {
        value: Box<Expr>,
        fallback: Box<Expr>,
        span: ByteSpan,
    },
    List {
        items: Vec<Expr>,
        span: ByteSpan,
    },
}

impl Expr {
    pub fn span(&self) -> ByteSpan {
        match self {
            Expr::Int { span, .. }
            | Expr::Number { span, .. }
            | Expr::Str { span, .. }
            | Expr::Bool { span, .. }
            | Expr::NoneLit { span }
            | Expr::Ident { span, .. }
            | Expr::This { span }
            | Expr::Base { span }
            | Expr::ErrorRef { span }
            | Expr::ResultRef { span }
            | Expr::Call { span, .. }
            | Expr::Member { span, .. }
            | Expr::Index { span, .. }
            | Expr::NonNone { span, .. }
            | Expr::Binary { span, .. }
            | Expr::Unary { span, .. }
            | Expr::OnError { span, .. }
            | Expr::List { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Pow,
    Eq,
    NEq,
    /// `a is b` — identity comparison, level 8 (EXP-01).
    Is,
    /// `a not b` — binary `not` in operator position, level 8 (EXP-01).
    NotIs,
    Lt,
    LtEq,
    Gt,
    GtEq,
    And,
    Or,
    /// EXP-03 none-coalescing fallback (`value default fallback`), level 11.
    Default,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum UnOp {
    Not,
    Neg,
}

/// A type as written (04-type-system.ebnf.md), plus the width-suffix forms
/// valid only in host-function positions (LBS-02, ADR-0002).
#[derive(Debug, Clone, PartialEq)]
pub struct TypeExpr {
    pub base: BaseType,
    pub optional: bool,
    /// Spans of `?` markers beyond the first. TYP-03 forbids `T??`; the
    /// parser records the extras so the checker can report SEM009 (M4).
    pub extra_optionals: Vec<ByteSpan>,
    /// TYP-05 behavior suffix chain (`list<T>.line`), declaration-LHS only.
    /// The grammar admits any chain; the checker restricts combinations.
    pub behaviors: Vec<Behavior>,
    pub span: ByteSpan,
}

/// One `.line` / `.pile` / `.unique` suffix (TYP-05).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Behavior {
    pub name: BehaviorName,
    pub span: ByteSpan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BehaviorName {
    Line,
    Pile,
    Unique,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BaseType {
    Boolean,
    /// `integer`, or width-suffixed `integer:32` / `integer:u32` etc. in
    /// host-function declarations only.
    Integer(Option<IntWidth>),
    Number,
    String_,
    Bytes,
    Datetime,
    Any,
    Void,
    List(Box<TypeExpr>),
    Matrix(Box<TypeExpr>),
    Pairs(Box<TypeExpr>, Box<TypeExpr>),
    /// Class, capability, world-declared, or compile-time (TYP-04) type
    /// referenced by name.
    Named(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum IntWidth {
    S32,
    U8,
    U16,
    U32,
    U64,
}
