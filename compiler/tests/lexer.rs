use compiler::lexer::{Scanner, TokenKind};

fn lex(source: &str) -> Vec<TokenKind> {
    let mut scanner = Scanner::new(source);

    scanner
        .scan_tokens()
        .into_iter()
        .map(|token| token.kind)
        .collect()
}

#[test]
fn lexes_empty_file() {
    assert_eq!(
        lex(""),
        vec![
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_identifier() {
    assert_eq!(
        lex("hello"),
        vec![
            TokenKind::Identifier("hello".into()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_identifier_with_underscore() {
    assert_eq!(
        lex("_hello_world123"),
        vec![
            TokenKind::Identifier("_hello_world123".into()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_integer() {
    assert_eq!(
        lex("12345"),
        vec![
            TokenKind::Integer("12345".into()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_float() {
    assert_eq!(
        lex("3.14159"),
        vec![
            TokenKind::Float("3.14159".into()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_string() {
    assert_eq!(
        lex("\"hello\""),
        vec![
            TokenKind::String("hello".into()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_char() {
    assert_eq!(
        lex("'a'"),
        vec![
            TokenKind::Char('a'),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_string_escape() {
    assert_eq!(
        lex("\"hello\\nworld\""),
        vec![
            TokenKind::String("hello\nworld".into()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_char_escape() {
    assert_eq!(
        lex("'\\n'"),
        vec![
            TokenKind::Char('\n'),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_keywords() {
    assert_eq!(
        lex("fn let mut return if else while match true false self"),
        vec![
            TokenKind::Fn,
            TokenKind::Let,
            TokenKind::Mut,
            TokenKind::Return,
            TokenKind::If,
            TokenKind::Else,
            TokenKind::While,
            TokenKind::Match,
            TokenKind::True,
            TokenKind::False,
            TokenKind::Self_,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_arithmetic_operators() {
    assert_eq!(
        lex("+ - * / %"),
        vec![
            TokenKind::Plus,
            TokenKind::Minus,
            TokenKind::Star,
            TokenKind::Slash,
            TokenKind::Percent,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_assignment_operators() {
    assert_eq!(
        lex("= += -= *= /="),
        vec![
            TokenKind::Equal,
            TokenKind::PlusEqual,
            TokenKind::MinusEqual,
            TokenKind::StarEqual,
            TokenKind::SlashEqual,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_increment_and_decrement() {
    assert_eq!(
        lex("++ --"),
        vec![
            TokenKind::PlusPlus,
            TokenKind::MinusMinus,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_comparison_operators() {
    assert_eq!(
        lex("== != < > <= >="),
        vec![
            TokenKind::EqualEqual,
            TokenKind::NotEqual,
            TokenKind::Less,
            TokenKind::Greater,
            TokenKind::LessEqual,
            TokenKind::GreaterEqual,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_logical_operators() {
    assert_eq!(
        lex("&& || !"),
        vec![
            TokenKind::AndAnd,
            TokenKind::OrOr,
            TokenKind::Not,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_bitwise_operators() {
    assert_eq!(
        lex("& | ^ ~"),
        vec![
            TokenKind::And,
            TokenKind::Or,
            TokenKind::Caret,
            TokenKind::Tilde,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_delimiters() {
    assert_eq!(
        lex("(){}[],:;"),
        vec![
            TokenKind::LeftParen,
            TokenKind::RightParen,
            TokenKind::LeftBrace,
            TokenKind::RightBrace,
            TokenKind::LeftBracket,
            TokenKind::RightBracket,
            TokenKind::Comma,
            TokenKind::Colon,
            TokenKind::Semicolon,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_special_operators() {
    assert_eq!(
        lex("-> => :: .. ."),
        vec![
            TokenKind::Arrow,
            TokenKind::FatArrow,
            TokenKind::ColonColon,
            TokenKind::DotDot,
            TokenKind::Dot,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_simple_variable_declaration() {
    assert_eq!(
        lex("let x = 42;"),
        vec![
            TokenKind::Let,
            TokenKind::Identifier("x".into()),
            TokenKind::Equal,
            TokenKind::Integer("42".into()),
            TokenKind::Semicolon,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_simple_function() {
    assert_eq!(
        lex("fn add(a: i32) -> i32 { return a + 1; }"),
        vec![
            TokenKind::Fn,
            TokenKind::Identifier("add".into()),
            TokenKind::LeftParen,
            TokenKind::Identifier("a".into()),
            TokenKind::Colon,
            TokenKind::Identifier("i32".into()),
            TokenKind::RightParen,
            TokenKind::Arrow,
            TokenKind::Identifier("i32".into()),
            TokenKind::LeftBrace,
            TokenKind::Return,
            TokenKind::Identifier("a".into()),
            TokenKind::Plus,
            TokenKind::Integer("1".into()),
            TokenKind::Semicolon,
            TokenKind::RightBrace,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn ignores_line_comments() {
    assert_eq!(
        lex("// hello\nlet x = 1;"),
        vec![
            TokenKind::Let,
            TokenKind::Identifier("x".into()),
            TokenKind::Equal,
            TokenKind::Integer("1".into()),
            TokenKind::Semicolon,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn ignores_block_comments() {
    assert_eq!(
        lex("/* comment */ let x = 1;"),
        vec![
            TokenKind::Let,
            TokenKind::Identifier("x".into()),
            TokenKind::Equal,
            TokenKind::Integer("1".into()),
            TokenKind::Semicolon,
            TokenKind::Eof,
        ]
    );
}