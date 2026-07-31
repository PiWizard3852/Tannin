use crate::lexer::TokenKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Precedence {
    Lowest,

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

    Call,
}

impl Precedence {
    pub fn is_right_associative(self) -> bool {
        matches!(self, Precedence::Assignment)
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

            // Postfix operators
            TokenKind::LeftParen
            | TokenKind::LeftBracket
            | TokenKind::Dot
            | TokenKind::PlusPlus
            | TokenKind::MinusMinus => Precedence::Call,

            _ => Precedence::Lowest,
        }
    }
}