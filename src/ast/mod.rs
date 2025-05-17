use std::fmt;
use crate::parser::StringPart;
use std::collections::HashMap;
use crate::types::WasmType;
use wasm_encoder::ValType;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
    pub file: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f64),
    String(String),
    Boolean(bool),
    Array(Vec<Value>),
    Matrix(Vec<Vec<f64>>),
    Integer(i32),
    Byte(u8),
    Unsigned(u32),
    Long(i64),
    ULong(u64),
    Big(String),
    UBig(String),
    Float(f64),
    Unit,
    Null,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Integer,
    Number,
    Boolean,
    String,
    Byte,
    Unsigned,
    Long,
    ULong,
    Float,
    Array(Box<Type>),
    Matrix(Box<Type>),
    Unit,
    Object(String),
    Any,
    TypeParameter(String),
    Big,
    UBig,
    Generic(Box<Type>, Vec<Type>),
    Map(Box<Type>, Box<Type>),
    Function(Vec<Type>, Box<Type>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Operator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    LessThan,
    LessEqual,
    GreaterThan,
    GreaterEqual,
    Equal,
    NotEqual,
    And,
    Or,
    Not,
}

impl std::fmt::Display for Operator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operator::Add => write!(f, "+"),
            Operator::Subtract => write!(f, "-"),
            Operator::Multiply => write!(f, "*"),
            Operator::Divide => write!(f, "/"),
            Operator::Modulo => write!(f, "%"),
            Operator::LessThan => write!(f, "<"),
            Operator::LessEqual => write!(f, "<="),
            Operator::GreaterThan => write!(f, ">"),
            Operator::GreaterEqual => write!(f, ">="),
            Operator::Equal => write!(f, "=="),
            Operator::NotEqual => write!(f, "!="),
            Operator::And => write!(f, "&&"),
            Operator::Or => write!(f, "||"),
            Operator::Not => write!(f, "!"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOperator {
    Negate,
    Not,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MatrixOperator {
    Add,
    Subtract,
    Multiply,
    Transpose,
    Inverse,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ComparisonOperator {
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEquals,
    GreaterEquals,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    And,
    Or,
}

impl From<ComparisonOperator> for BinaryOperator {
    fn from(op: ComparisonOperator) -> Self {
        match op {
            ComparisonOperator::Equal => BinaryOperator::Equal,
            ComparisonOperator::NotEqual => BinaryOperator::NotEqual,
            ComparisonOperator::Less => BinaryOperator::Less,
            ComparisonOperator::Greater => BinaryOperator::Greater,
            ComparisonOperator::LessEquals => BinaryOperator::LessEqual,
            ComparisonOperator::GreaterEquals => BinaryOperator::GreaterEqual,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub type_: Type,
}

impl Parameter {
    pub fn new(name: String, type_: Type) -> Self {
        Self { name, type_ }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Literal(Value),
    Variable(String),
    Binary(Box<Expression>, BinaryOperator, Box<Expression>),
    Unary(UnaryOperator, Box<Expression>),
    Call(String, Vec<Expression>),
    ArrayAccess(Box<Expression>, Box<Expression>),
    MatrixAccess(Box<Expression>, Box<Expression>, Box<Expression>),
    StringConcat(Vec<Expression>),
    FieldAccess {
        object: Box<Expression>,
        field: String,
        location: SourceLocation,
    },
    MethodCall {
        object: Box<Expression>,
        method: String,
        arguments: Vec<Expression>,
        location: SourceLocation,
    },
    ObjectCreation {
        class_name: String,
        arguments: Vec<Expression>,
        location: SourceLocation,
    },
    MatrixOperation(Box<Expression>, MatrixOperator, Box<Expression>, SourceLocation),
}

#[derive(Debug, Clone)]
pub enum Statement {
    VariableDecl {
        name: String,
        type_: Option<Type>,
        initializer: Option<Expression>,
        location: Option<SourceLocation>,
    },
    Assignment {
        target: String,
        value: Expression,
        location: Option<SourceLocation>,
    },
    Print {
        expression: Expression,
        newline: bool,
        location: Option<SourceLocation>,
    },
    Return {
        value: Option<Expression>,
        location: Option<SourceLocation>,
    },
    If {
        condition: Expression,
        then_branch: Vec<Statement>,
        else_branch: Option<Vec<Statement>>,
        location: Option<SourceLocation>,
    },
    Iterate {
        iterator: String,
        collection: Expression,
        body: Vec<Statement>,
        location: Option<SourceLocation>,
    },
    FromTo {
        start: Expression,
        end: Expression,
        step: Option<Expression>,
        body: Vec<Statement>,
        location: Option<SourceLocation>,
    },
    ErrorHandler {
        stmt: Box<Statement>,
        handler: Vec<Statement>,
        location: Option<SourceLocation>,
    },
    Test {
        name: String,
        description: Option<String>,
        body: Vec<Statement>,
        location: Option<SourceLocation>,
    },
    Expression {
        expr: Expression,
        location: Option<SourceLocation>,
    },
    Constructor {
        params: Vec<Parameter>,
        body: Vec<Statement>,
        location: Option<SourceLocation>,
    }
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub type_parameters: Vec<String>,
    pub parameters: Vec<Parameter>,
    pub return_type: Type,
    pub body: Vec<Statement>,
    pub description: Option<String>,
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
            parameters,
            return_type,
            body,
            description: None,
            location,
        }
    }
}

#[derive(Debug, Clone)]
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
    pub base_class: Option<String>,
    pub base_class_type_args: Vec<Type>,
    pub fields: Vec<Field>,
    pub methods: Vec<Function>,
    pub constructor: Option<Constructor>,
    pub location: Option<SourceLocation>,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub functions: Vec<Function>,
    pub classes: Vec<Class>,
}

impl Program {
    pub fn new(functions: Vec<Function>, classes: Vec<Class>) -> Self {
        Self { functions, classes }
    }
}

// Display implementations for better error messages
impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Type::Integer => f.write_str("integer"),
            Type::Boolean => f.write_str("boolean"),
            Type::String => f.write_str("string"),
            Type::Float => f.write_str("float"),
            Type::Array(inner) => write!(f, "array<{}>", inner),
            Type::Matrix(inner) => write!(f, "matrix<{}>", inner),
            Type::Map(key, value) => write!(f, "map<{}, {}>", key, value),
            Type::Function(params, ret) => {
                write!(f, "fn(")?;
                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", param)?;
                }
                write!(f, ") -> {}", ret)
            },
            Type::Object(name) => write!(f, "{}", name),
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
            Type::Any => f.write_str("any"),
            Type::Unit => f.write_str("unit"),
            Type::Number => f.write_str("number"),
            Type::Byte => f.write_str("byte"),
            Type::Unsigned => f.write_str("unsigned"),
            Type::Long => f.write_str("long"),
            Type::ULong => f.write_str("ulong"),
            Type::Big => f.write_str("big"),
            Type::UBig => f.write_str("ubig"),
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
            Value::Array(items) => {
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
            Value::Null => write!(f, "null"),
            Value::Unit => write!(f, "()"),
            Value::Byte(b) => write!(f, "{}b", b),
            Value::Unsigned(u) => write!(f, "{}u", u),
            Value::Long(l) => write!(f, "{}L", l),
            Value::ULong(ul) => write!(f, "{}UL", ul),
            Value::Big(s) => write!(f, "{}n", s),
            Value::UBig(s) => write!(f, "{}un", s),
            Value::Float(f_val) => write!(f, "{}f", f_val),
        }
    }
}

impl Type {
    pub fn as_ref(&self) -> &Type {
        self
    }
} 