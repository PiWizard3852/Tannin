use crate::common::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Integer(String),
    Float(String),
    String(String),
    Char(char),

    Identifier(String),

    Fn,
    Struct,
    Enum,
    Impl,
    Const,
    Let,
    Mut,
    Import,
    Pub,

    If,
    Else,
    While,
    Match,
    Return,

    True,
    False,
    Self_,

    Plus,
    Minus,
    Star,
    Slash,
    Percent,

    PlusEqual,
    MinusEqual,
    StarEqual,
    SlashEqual,
    PercentEqual,

    PlusPlus,
    MinusMinus,

    Equal,
    EqualEqual,
    NotEqual,

    Less,
    Greater,
    LessEqual,
    GreaterEqual,

    GreaterGreater,
    LessLess,

    GreaterGreaterEqual,
    LessLessEqual,

    And,
    Or,
    Caret,
    Tilde,

    AndEqual,
    OrEqual,
    CaretEqual,

    AndAnd,
    OrOr,
    Not,

    Comma,
    Semicolon,
    Colon,

    Arrow,      // ->
    FatArrow,   // =>
    Dot,
    ColonColon,
    DotDot,

    LeftParen,
    RightParen,

    LeftBrace,
    RightBrace,

    LeftBracket,
    RightBracket,

    Underscore,

    Eof,
}