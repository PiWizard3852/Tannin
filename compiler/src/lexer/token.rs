#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

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

    PlusPlus,
    MinusMinus,

    Equal,
    EqualEqual,
    NotEqual,

    Less,
    Greater,
    LessEqual,
    GreaterEqual,

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

    Eof,
}