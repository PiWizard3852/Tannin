use crate::analyzer::types::{FunctionType, ValueType};
use crate::common::Span;
use crate::parser::ast::{AssignmentOperator, BinaryOperator, UnaryOperator};

#[derive(Debug)]
pub struct CheckedProgram {
    pub(crate) functions: Vec<CheckedFunction>,
    pub(crate) structs: Vec<CheckedStruct>,
}

#[derive(Debug)]
pub(crate) struct CheckedStruct {
    pub name: String,
    pub fields: Vec<(String, ValueType)>,
}

#[derive(Debug)]
pub(crate) struct CheckedFunction {
    pub name: String,
    pub pub_: bool,
    pub type_: FunctionType,
    pub locals: Vec<ValueType>,
    pub body: CheckedBlock,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Flow {
    Value(ValueType),
    NoValue,
    Returns,
}

impl Flow {
    pub(crate) fn value(&self) -> Option<&ValueType> {
        match self {
            Self::Value(type_) => Some(type_),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub(crate) struct CheckedBlock {
    pub statements: Vec<CheckedStatement>,
    pub tail: Option<Box<CheckedExpression>>,
    pub flow: Flow,
}

#[derive(Debug)]
pub(crate) enum CheckedStatement {
    Block(CheckedBlock),

    Let {
        index: usize,
        initializer: CheckedExpression,
    },

    While {
        condition: CheckedExpression,
        body: CheckedBlock,
    },

    Return(Option<CheckedExpression>),
    Expression(CheckedExpression),
}

impl CheckedStatement {
    pub(crate) fn returns(&self) -> bool {
        match self {
            Self::Block(block) => block.flow == Flow::Returns,
            Self::Return(_) => true,
            Self::Expression(expression) => expression.flow == Flow::Returns,
            Self::Let { .. } | Self::While { .. } => false,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum CheckedLiteral {
    Integer(i128),
    Float(f64),
    Bool(bool),
    Char(char),
    String(String),
}

#[derive(Debug)]
pub(crate) struct CheckedExpression {
    pub kind: ExpressionKind,
    pub flow: Flow,
    pub span: Span,
}

impl CheckedExpression {
    pub(crate) fn new(kind: ExpressionKind, flow: Flow, span: Span) -> Self {
        Self { kind, flow, span }
    }

    pub(crate) fn value_type(&self) -> &ValueType {
        self.flow.value().expect("checked expression has a value")
    }
}

#[derive(Debug)]
pub(crate) enum ExpressionKind {
    Literal(CheckedLiteral),
    Local(usize),
    Block(CheckedBlock),

    Call {
        function: String,
        arguments: Vec<CheckedExpression>,
    },

    Unary {
        operator: UnaryOperator,
        operand: Box<CheckedExpression>,
    },

    Binary {
        left: Box<CheckedExpression>,
        operator: BinaryOperator,
        right: Box<CheckedExpression>,
    },

    Assignment {
        target: Box<CheckedExpression>,
        operator: AssignmentOperator,
        value: Box<CheckedExpression>,
    },

    If {
        condition: Box<CheckedExpression>,
        then_branch: Box<CheckedExpression>,
        else_branch: Option<Box<CheckedExpression>>,
    },

    Struct {
        name: String,
        fields: Vec<(usize, CheckedExpression)>,
    },

    Member {
        object: Box<CheckedExpression>,
        index: usize,
    },
}
