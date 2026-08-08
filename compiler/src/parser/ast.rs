use crate::common::Span;

#[derive(Debug, Clone)]
pub struct Program {
    pub declarations: Vec<Declaration>,
}

#[derive(Debug, Clone)]
pub enum Declaration {
    Function(FunctionDecl),
    Struct(StructDecl),
    Enum(EnumDecl),
    Const(ConstDecl),
}

#[derive(Debug, Clone)]
pub struct FunctionDecl {
    pub name: String,
    pub pub_: bool,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<Type>,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct StructDecl {
    pub name: String,
    pub pub_: bool,
    pub fields: Vec<Field>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct EnumDecl {
    pub name: String,
    pub pub_: bool,
    pub variants: Vec<EnumVariant>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ConstDecl {
    pub name: String,
    pub pub_: bool,
    pub type_: Type,
    pub value: Expression,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub type_: Type,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub type_: Type,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub statements: Vec<Statement>,
    pub tail: Option<Box<Expression>>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Statement {
    Block(Block),

    Let {
        name: String,
        mutable: bool,
        type_: Option<Type>,
        initializer: Option<Expression>,
        span: Span,
    },

    While {
        condition: Expression,
        body: Block,
        span: Span,
    },

    Return {
        value: Option<Expression>,
        span: Span,
    },

    Expression {
        expression: Expression,
        span: Span,
    },
}

#[derive(Debug, Clone)]
pub enum Expression {
    Literal {
        value: Literal,
        span: Span,
    },

    Identifier {
        value: String,
        span: Span,
    },

    Unary {
        operator: UnaryOperator,
        operand: Box<Expression>,
        span: Span,
    },

    Binary {
        left: Box<Expression>,
        operator: BinaryOperator,
        right: Box<Expression>,
        span: Span,
    },

    Assignment {
        target: Box<Expression>,
        operator: AssignmentOperator,
        value: Box<Expression>,
        span: Span,
    },

    Call {
        callee: Box<Expression>,
        arguments: Vec<Expression>,
        span: Span,
    },

    Member {
        object: Box<Expression>,
        member: String,
        span: Span,
    },

    Index {
        object: Box<Expression>,
        index: Box<Expression>,
        span: Span,
    },

    Match {
        expression: Box<Expression>,
        arms: Vec<MatchArm>,
        span: Span,
    },

    If {
        condition: Box<Expression>,
        then_branch: Box<Expression>,
        else_branch: Option<Box<Expression>>,
        span: Span,
    },

    Block(Block),
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub value: Expression,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Identifier {
        value: String,
        span: Span,
    },

    Literal {
        value: Literal,
        span: Span,
    },

    EnumVariant {
        value: String,
        span: Span,
    },

    Wildcard {
        span: Span,
    },
}

#[derive(Debug, Clone)]
pub enum Literal {
    Integer(String),
    Float(String),
    String(String),
    Char(char),
    Bool(bool),
}

#[derive(Debug, Clone)]
pub enum Type {
    Named {
        name: String,
        span: Span,
    },

    Array {
        element: Box<Type>,
        span: Span,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignmentOperator {
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    ModAssign,
    AndAssign,
    OrAssign,
    XorAssign,
    LeftShiftAssign,
    RightShiftAssign,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    Negate,
    Not,
    BitwiseNot,
    PreIncrement,
    PreDecrement,
    PostIncrement,
    PostDecrement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,

    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,

    LogicalAnd,
    LogicalOr,

    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,

    LeftShift,
    RightShift,
}

impl Expression {
    pub fn span(&self) -> Span {
        match self {
            Expression::Literal { span, .. } => *span,
            Expression::Identifier { span, .. } => *span,
            Expression::Unary { span, .. } => *span,
            Expression::Binary { span, .. } => *span,
            Expression::Assignment { span, .. } => *span,
            Expression::Call { span, .. } => *span,
            Expression::Member { span, .. } => *span,
            Expression::Index { span, .. } => *span,
            Expression::Match { span, .. } => *span,
            Expression::If { span, .. } => *span,
            Expression::Block(block) => block.span,
        }
    }
}

impl Pattern {
    pub fn span(&self) -> Span {
        match self {
            Pattern::Identifier { span, .. } => *span,
            Pattern::Literal { span, .. } => *span,
            Pattern::EnumVariant { span, .. } => *span,
            Pattern::Wildcard { span } => *span,
        }
    }
}

impl Type {
    pub fn span(&self) -> Span {
        match self {
            Type::Named { span, .. } => *span,
            Type::Array { span, .. } => *span,
        }
    }
}
