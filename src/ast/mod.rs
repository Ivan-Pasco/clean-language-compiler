use std::fmt;





#[derive(Debug, Clone, PartialEq, Default)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
    pub file: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Integer(i64),        // Default integer (platform optimal)
    Number(f64),          // Default number (platform optimal)
    Boolean(bool),
    String(String),
    Matrix(Vec<Vec<f64>>),
    Void,
    // Advanced sized types
    Integer8(i8),
    Integer8u(u8),
    Integer16(i16),
    Integer16u(u16),
    Integer32(i32),
    Integer64(i64),
    Number32(f32),
    Number64(f64),
    
    // List (replaces Array)
    List(Vec<Value>),
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum ListBehavior {
    Default,    // Standard list behavior
    Line,       // Queue behavior (FIFO)
    Pile,       // Stack behavior (LIFO)
    Unique,     // Set behavior (no duplicates)
    LinePile,   // Combined line + pile (not typical, but allowed)
    LineUnique, // Queue with unique elements
    PileUnique, // Stack with unique elements
    LineUniquePile, // All three combined
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    // Core types from specification
    Boolean,
    Integer,             // Default integer
    Number,             // Default number (floating point)
    String,
    Void,
    
    // Advanced sized types
    IntegerSized { bits: u8, unsigned: bool },
    NumberSized { bits: u8 },
    
    // Composite types
    List(Box<Type>),
    Matrix(Box<Type>),
    Pairs(Box<Type>, Box<Type>),
    
    // Generic types
    Generic(Box<Type>, Vec<Type>),
    TypeParameter(String),
    
    // Object types
    Object(String),
    Class { name: String, type_args: Vec<Type> },
    Function(Vec<Type>, Box<Type>),
    
    // Async types
    Future(Box<Type>),
    
    Any,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Power,
    
    // Comparison
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    Is,
    Not,
    
    // Logical
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOperator {
    Negate,
    Not,
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub type_: Type,
    pub default_value: Option<Expression>,
}

impl Parameter {
    pub fn new(name: String, type_: Type) -> Self {
        Self { name, type_, default_value: None }
    }
    
    pub fn new_with_default(name: String, type_: Type, default_value: Expression) -> Self {
        Self { name, type_, default_value: Some(default_value) }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Literal(Value),
    Variable(String),
    Binary(Box<Expression>, BinaryOperator, Box<Expression>),
    Unary(UnaryOperator, Box<Expression>),
    Call(String, Vec<Expression>),
    
    // Property and method access
    PropertyAccess {
        object: Box<Expression>,
        property: String,
        location: SourceLocation,
    },
    
    // Property assignment (for list.type = behavior)
    PropertyAssignment {
        object: Box<Expression>,
        property: String,
        value: Box<Expression>,
        location: SourceLocation,
    },
    MethodCall {
        object: Box<Expression>,
        method: String,
        arguments: Vec<Expression>,
        location: SourceLocation,
    },
    
    // Static method call (ClassName.method())
    StaticMethodCall {
        class_name: String,
        method: String,
        arguments: Vec<Expression>,
        location: SourceLocation,
    },
    
    // Array and Matrix access
    ArrayAccess(Box<Expression>, Box<Expression>),
    MatrixAccess(Box<Expression>, Box<Expression>, Box<Expression>),
    
    // String interpolation
    StringInterpolation(Vec<StringPart>),
    
    // Object creation
    ObjectCreation {
        class_name: String,
        arguments: Vec<Expression>,
        location: SourceLocation,
    },
    
    // Error handling
    OnError {
        expression: Box<Expression>,
        fallback: Box<Expression>,
        location: SourceLocation,
    },
    
    // Error handling with block
    OnErrorBlock {
        expression: Box<Expression>,
        error_handler: Vec<Statement>,
        location: SourceLocation,
    },
    
    // Error variable access (only valid in error handling contexts)
    ErrorVariable {
        location: SourceLocation,
    },
    
    // Conditional expressions: if condition then value else value
    Conditional {
        condition: Box<Expression>,
        then_expr: Box<Expression>,
        else_expr: Box<Expression>,
        location: SourceLocation,
    },
    
    // Base constructor call: base(args...)
    BaseCall {
        arguments: Vec<Expression>,
        location: SourceLocation,
    },
    
    // Async expressions
    StartExpression {
        expression: Box<Expression>,
        location: SourceLocation,
    },
    
    // Later assignment (for async)
    LaterAssignment {
        variable: String,
        expression: Box<Expression>,
        location: SourceLocation,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum StringPart {
    Text(String),
    Interpolation(Expression),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchCase {
    pub pattern: Pattern,
    pub guard: Option<Expression>,  // Optional when condition
    pub body: Vec<Statement>,
    pub location: SourceLocation,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    // Literal patterns: 42, "hello", true
    Literal(Value),
    
    // Variable patterns: x (binds to variable)
    Variable(String),
    
    // Wildcard pattern: _
    Wildcard,
    
    // Constructor patterns: Some(x), Point(x, y)
    Constructor {
        name: String,
        patterns: Vec<Pattern>,
    },
    
    // Array patterns: [x, y, z] or [head, ...tail]
    Array {
        patterns: Vec<Pattern>,
        rest: Option<String>,  // For spread patterns like [x, ...rest]
    },
    
    // Object patterns: { x, y } or { x: pattern }
    Object {
        fields: Vec<FieldPattern>,
    },
    
    // Or patterns: pattern1 | pattern2
    Or(Vec<Pattern>),
    
    // Range patterns: 1..10
    Range {
        start: Box<Expression>,
        end: Box<Expression>,
        inclusive: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldPattern {
    pub name: String,
    pub pattern: Option<Pattern>,  // None means shorthand { x } instead of { x: pattern }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    // Variable declarations (type-first)
    VariableDecl {
        name: String,
        type_: Type,
        initializer: Option<Expression>,
        location: Option<SourceLocation>,
    },
    
    // Apply blocks - Three types as per specification
    TypeApplyBlock {
        type_: Type,
        assignments: Vec<VariableAssignment>,
        location: Option<SourceLocation>,
    },
    
    FunctionApplyBlock {
        function_name: String,
        expressions: Vec<Expression>,
        location: Option<SourceLocation>,
    },
    
    MethodApplyBlock {
        object_name: String,
        method_chain: Vec<String>,
        expressions: Vec<Expression>,
        location: Option<SourceLocation>,
    },
    
    ConstantApplyBlock {
        constants: Vec<ConstantAssignment>,
        location: Option<SourceLocation>,
    },
    
    // Assignment
    Assignment {
        target: String,
        value: Expression,
        location: Option<SourceLocation>,
    },
    
    // Print statements
    Print {
        expression: Expression,
        newline: bool,
        location: Option<SourceLocation>,
    },
    
    // Print block (multiple expressions)
    PrintBlock {
        expressions: Vec<Expression>,
        newline: bool,
        location: Option<SourceLocation>,
    },
    
    // Return
    Return {
        value: Option<Expression>,
        location: Option<SourceLocation>,
    },
    
    // Control flow
    If {
        condition: Expression,
        then_branch: Vec<Statement>,
        else_branch: Option<Vec<Statement>>,
        location: Option<SourceLocation>,
    },
    
    // Iteration
    Iterate {
        iterator: String,
        collection: Expression,
        body: Vec<Statement>,
        location: Option<SourceLocation>,
    },
    
    RangeIterate {
        iterator: String,
        start: Expression,
        end: Expression,
        step: Option<Expression>,
        body: Vec<Statement>,
        location: Option<SourceLocation>,
    },
    
    // Test
    Test {
        name: String,
        body: Vec<Statement>,
        location: Option<SourceLocation>,
    },
    
    // Tests block
    TestsBlock {
        tests: Vec<TestCase>,
        location: Option<SourceLocation>,
    },
    
    // Expression statement
    Expression {
        expr: Expression,
        location: Option<SourceLocation>,
    },
    
    // Error statement (throw error)
    Error {
        message: Expression,
        location: Option<SourceLocation>,
    },
    
    // Module imports
    Import {
        imports: Vec<ImportItem>,
        location: Option<SourceLocation>,
    },
    
    // Async statements
    LaterAssignment {
        variable: String,
        expression: Expression,
        location: Option<SourceLocation>,
    },
    
    Background {
        expression: Expression,
        location: Option<SourceLocation>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct VariableAssignment {
    pub name: String,
    pub initializer: Option<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstantAssignment {
    pub type_: Type,
    pub name: String,
    pub value: Expression,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportItem {
    pub name: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TestCase {
    pub description: Option<String>,  // None for anonymous tests
    pub test_expression: Expression,
    pub expected_value: Expression,
    pub location: Option<SourceLocation>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FunctionModifier {
    None,
    Background,
}

#[derive(Debug, Clone)]
pub enum FunctionSyntax {
    Simple,      // function integer add() ...
    Detailed,    // function integer add() with description/input blocks
    Block,       // functions: block
}

#[derive(Debug, Clone)]
pub struct TypeConstraint {
    pub type_parameter: String,
    pub constraint_type: Type,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub type_parameters: Vec<String>,
    pub type_constraints: Vec<TypeConstraint>,
    pub parameters: Vec<Parameter>,
    pub return_type: Type,
    pub body: Vec<Statement>,
    pub description: Option<String>,
    pub syntax: FunctionSyntax,
    pub visibility: Visibility,
    pub modifier: FunctionModifier,
    pub location: Option<SourceLocation>,
}

impl Function {
    pub fn new(
        name: String,
        parameters: Vec<Parameter>,
        return_type: Type,
        body: Vec<Statement>,
        location: Option<SourceLocation>,
    ) -> Self {
        Self {
            name,
            type_parameters: Vec::new(),
            type_constraints: Vec::new(),
            parameters,
            return_type,
            body,
            description: None,
            syntax: FunctionSyntax::Simple,
            visibility: Visibility::Public,
            modifier: FunctionModifier::None,
            location,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Visibility {
    Public,
    Private,
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub type_: Type,
    pub visibility: Visibility,
    pub is_static: bool,
    pub default_value: Option<Expression>,
}

#[derive(Debug, Clone)]
pub struct Constructor {
    pub parameters: Vec<Parameter>,
    pub body: Vec<Statement>,
    pub location: Option<SourceLocation>,
}

impl Constructor {
    pub fn new(parameters: Vec<Parameter>, body: Vec<Statement>, location: Option<SourceLocation>) -> Self {
        Self {
            parameters,
            body,
            location,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Class {
    pub name: String,
    pub type_parameters: Vec<String>,
    pub description: Option<String>,
    pub base_class: Option<String>,  // Using "is" inheritance
    pub base_class_type_args: Vec<Type>,
    pub fields: Vec<Field>,
    pub methods: Vec<Function>,
    pub constructor: Option<Constructor>,
    pub location: Option<SourceLocation>,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub imports: Vec<ImportItem>,
    pub functions: Vec<Function>,
    pub classes: Vec<Class>,
    pub start_function: Option<Function>,
    pub tests: Vec<TestCase>,
}

// Display implementations for better error messages
impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Type::Boolean => f.write_str("boolean"),
            Type::Integer => f.write_str("integer"),
            Type::Number => f.write_str("number"),
            Type::String => f.write_str("string"),
            Type::Void => f.write_str("void"),
            Type::IntegerSized { bits, unsigned } => {
                if *unsigned {
                    write!(f, "integer:{}u", bits)
                } else {
                    write!(f, "integer:{}", bits)
                }
            },
            Type::NumberSized { bits } => write!(f, "number:{}", bits),
            // Type::Array removed - now using Type::List
            Type::List(inner) => write!(f, "list<{}>", inner),
            Type::Matrix(inner) => write!(f, "matrix<{}>", inner),
            Type::Pairs(key, value) => write!(f, "pairs<{}, {}>", key, value),
            Type::Function(params, ret) => {
                write!(f, "function(")?;
                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", param)?;
                }
                write!(f, ") returns {}", ret)
            },
            Type::Object(name) => write!(f, "{}", name),
            Type::Class { name, type_args } => {
                if type_args.is_empty() {
                    write!(f, "{}", name)
                } else {
                    write!(f, "{}<", name)?;
                    for (i, arg) in type_args.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", arg)?;
                    }
                    write!(f, ">")
                }
            },
            Type::Generic(base, args) => {
                write!(f, "{}<", base)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ">")
            },
            Type::TypeParameter(name) => write!(f, "{}", name),
            Type::Future(inner) => write!(f, "Future<{}>", inner),
            Type::Any => f.write_str("any"),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Integer(i) => write!(f, "{}", i),
            Value::Number(n) => write!(f, "{}", n),
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::List(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            },
            Value::Matrix(rows) => {
                write!(f, "[")?;
                for (i, row) in rows.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "[")?;
                    for (j, value) in row.iter().enumerate() {
                        if j > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", value)?;
                    }
                    write!(f, "]")?;
                }
                write!(f, "]")
            },
            Value::Void => write!(f, "()"),
            Value::Integer8(i) => write!(f, "{}:8", i),
            Value::Integer8u(u) => write!(f, "{}:8u", u),
            Value::Integer16(i) => write!(f, "{}:16", i),
            Value::Integer16u(u) => write!(f, "{}:16u", u),
            Value::Integer32(i) => write!(f, "{}:32", i),
            Value::Integer64(i) => write!(f, "{}:64", i),
            Value::Number32(f_val) => write!(f, "{}:32", f_val),
            Value::Number64(f_val) => write!(f, "{}:64", f_val),
        }
    }
}

impl Type {
    pub fn as_ref(&self) -> &Type {
        self
    }
} 