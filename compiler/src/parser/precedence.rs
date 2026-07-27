use crate::lexer::TokenKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Precedence {
    None,
    Assignment,
    LogicalOr,
    LogicalAnd,
    BitwiseOr,
    BitwiseXor,
    BitwiseAnd,
    Equality,
    Comparison,
    Shift,
    Addition,
    Multiplication,
    Unary,
    Postfix,
    Call,
    Primary,
}

impl Precedence {
    pub fn next(self) -> Self {
        match self {
            Precedence::None => Precedence::Assignment,
            Precedence::Assignment => Precedence::LogicalOr,
            Precedence::LogicalOr => Precedence::LogicalAnd,
            Precedence::LogicalAnd => Precedence::BitwiseOr,
            Precedence::BitwiseOr => Precedence::BitwiseXor,
            Precedence::BitwiseXor => Precedence::BitwiseAnd,
            Precedence::BitwiseAnd => Precedence::Equality,
            Precedence::Equality => Precedence::Comparison,
            Precedence::Comparison => Precedence::Shift,
            Precedence::Shift => Precedence::Addition,
            Precedence::Addition => Precedence::Multiplication,
            Precedence::Multiplication => Precedence::Unary,
            Precedence::Unary => Precedence::Postfix,
            Precedence::Postfix => Precedence::Call,
            Precedence::Call => Precedence::Primary,
            Precedence::Primary => Precedence::Primary,
        }
    }

    pub fn from_token(token: &TokenKind) -> Self {
        match token {
            // Assignment
            TokenKind::Equal
            | TokenKind::PlusEqual
            | TokenKind::MinusEqual
            | TokenKind::StarEqual
            | TokenKind::SlashEqual
            | TokenKind::PercentEqual
            | TokenKind::AndEqual
            | TokenKind::OrEqual
            | TokenKind::CaretEqual
            | TokenKind::LessLessEqual
            | TokenKind::GreaterGreaterEqual => Precedence::Assignment,

            // Logical
            TokenKind::OrOr => Precedence::LogicalOr,
            TokenKind::AndAnd => Precedence::LogicalAnd,

            // Bitwise
            TokenKind::Or => Precedence::BitwiseOr,
            TokenKind::Caret => Precedence::BitwiseXor,
            TokenKind::And => Precedence::BitwiseAnd,

            // Equality
            TokenKind::EqualEqual
            | TokenKind::NotEqual => Precedence::Equality,

            // Comparison
            TokenKind::Less
            | TokenKind::LessEqual
            | TokenKind::Greater
            | TokenKind::GreaterEqual => Precedence::Comparison,

            // Shift
            TokenKind::LessLess
            | TokenKind::GreaterGreater => Precedence::Shift,

            // Arithmetic
            TokenKind::Plus
            | TokenKind::Minus => Precedence::Addition,

            TokenKind::Star
            | TokenKind::Slash
            | TokenKind::Percent => Precedence::Multiplication,

            // Postfix
            TokenKind::PlusPlus
            | TokenKind::MinusMinus => Precedence::Postfix,

            // Call / indexing / member access
            TokenKind::LeftParen
            | TokenKind::LeftBracket
            | TokenKind::Dot => Precedence::Call,

            _ => Precedence::None,
        }
    }
}