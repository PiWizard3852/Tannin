use crate::common::Span;
use crate::lexer::{LexError, LexResult, Token, TokenKind};

pub struct Scanner<'a> {
    source: &'a str,
    position: usize,
}

impl<'a> Scanner<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            position: 0,
        }
    }

    pub fn scan_tokens(&mut self) -> Vec<Token> {
        self.try_scan_tokens().unwrap_or_else(|error| panic!("{}", error.message))
    }

    pub fn try_scan_tokens(&mut self) -> LexResult<Vec<Token>> {
        let mut result = Vec::new();

        loop {
            let start = self.position;
            let current_char = self.advance();

            match current_char {
                Some('+') => {
                    let next_char = self.peek();

                    if let Some('=') = next_char {
                        self.advance();

                        result.push(Token::new(TokenKind::PlusEqual, Span::new(start, self.position)));
                        continue;
                    }

                    if let Some('+') = next_char {
                        self.advance();

                        result.push(Token::new(TokenKind::PlusPlus, Span::new(start, self.position)));
                        continue;
                    }

                    result.push(Token::new(TokenKind::Plus, Span::new(start, self.position)));
                }
                Some('-') => {
                    let next_char = self.peek();

                    if let Some('=') = next_char {
                        self.advance();

                        result.push(Token::new(TokenKind::MinusEqual, Span::new(start, self.position)));
                        continue;
                    }

                    if let Some('-') = next_char {
                        self.advance();

                        result.push(Token::new(TokenKind::MinusMinus, Span::new(start, self.position)));
                        continue;
                    }

                    if let Some('>') = next_char {
                        self.advance();

                        result.push(Token::new(TokenKind::Arrow, Span::new(start, self.position)));
                        continue;
                    }

                    result.push(Token::new(TokenKind::Minus, Span::new(start, self.position)))
                }
                Some('*') => {
                    let next_char = self.peek();

                    if let Some('=') = next_char {
                        self.advance();

                        result.push(Token::new(TokenKind::StarEqual, Span::new(start, self.position)));
                        continue;
                    }

                    result.push(Token::new(TokenKind::Star, Span::new(start, self.position)))
                }
                Some('/') => {
                    let next_char = self.peek();

                    if let Some('=') = next_char {
                        self.advance();

                        result.push(Token::new(TokenKind::SlashEqual, Span::new(start, self.position)));
                        continue;
                    }

                    if let Some('/') = next_char {
                        self.advance();

                        while let Some(c) = self.peek() {
                            self.advance();

                            if c == '\n' {
                                break;
                            }
                        }

                        continue;
                    }

                    if let Some('*') = next_char {
                        self.advance();

                        loop {
                            match self.advance() {
                                Some('*') if self.peek() == Some('/') => {
                                    self.advance();
                                    break;
                                }

                                Some(_) => {}

                                None => return Err(self.error("unterminated block comment", start)),
                            }
                        }

                        continue;
                    }

                    result.push(Token::new(TokenKind::Slash, Span::new(start, self.position)))
                }
                Some('%') => {
                    let next_char = self.peek();

                    if let Some('=') = next_char {
                        self.advance();

                        result.push(Token::new(TokenKind::PercentEqual, Span::new(start, self.position)));
                        continue;
                    }

                    result.push(Token::new(TokenKind::Percent, Span::new(start, self.position)))
                }

                Some('=') => {
                    let next_char = self.peek();

                    if let Some('=') = next_char {
                        self.advance();

                        result.push(Token::new(TokenKind::EqualEqual, Span::new(start, self.position)));
                        continue;
                    }

                    if let Some('>') = next_char {
                        self.advance();

                        result.push(Token::new(TokenKind::FatArrow, Span::new(start, self.position)));
                        continue;
                    }

                    result.push(Token::new(TokenKind::Equal, Span::new(start, self.position)))
                }
                Some('!') => {
                    let next_char = self.peek();

                    if let Some('=') = next_char {
                        self.advance();

                        result.push(Token::new(TokenKind::NotEqual, Span::new(start, self.position)));
                        continue;
                    }

                    result.push(Token::new(TokenKind::Not, Span::new(start, self.position)))
                }

                Some('<') => {
                    let next_char = self.peek();

                    if let Some('=') = next_char {
                        self.advance();

                        result.push(Token::new(TokenKind::LessEqual, Span::new(start, self.position)));
                        continue;
                    }

                    if let Some('<') = next_char {
                        self.advance();

                        if let Some('=') = self.peek() {
                            self.advance();

                            result.push(Token::new(TokenKind::LessLessEqual, Span::new(start, self.position)));
                            continue;
                        }

                        result.push(Token::new(TokenKind::LessLess, Span::new(start, self.position)));
                        continue;
                    }

                    result.push(Token::new(TokenKind::Less, Span::new(start, self.position)))
                }
                Some('>') => {
                    let next_char = self.peek();

                    if let Some('=') = next_char {
                        self.advance();

                        result.push(Token::new(TokenKind::GreaterEqual, Span::new(start, self.position)));
                        continue;
                    }

                    if let Some('>') = next_char {
                        self.advance();

                        if let Some('=') = self.peek() {
                            self.advance();

                            result.push(Token::new(TokenKind::GreaterGreaterEqual, Span::new(start, self.position)));
                            continue;
                        }

                        result.push(Token::new(TokenKind::GreaterGreater, Span::new(start, self.position)));
                        continue;
                    }

                    result.push(Token::new(TokenKind::Greater, Span::new(start, self.position)))
                }

                Some('&') => {
                    let next_char = self.peek();

                    if let Some('=') = next_char {
                        self.advance();

                        result.push(Token::new(TokenKind::AndEqual, Span::new(start, self.position)));
                        continue;
                    }

                    if let Some('&') = next_char {
                        self.advance();

                        result.push(Token::new(TokenKind::AndAnd, Span::new(start, self.position)));
                        continue;
                    }

                    result.push(Token::new(TokenKind::And, Span::new(start, self.position)))
                }
                Some('|') => {
                    let next_char = self.peek();

                    if let Some('=') = next_char {
                        self.advance();

                        result.push(Token::new(TokenKind::OrEqual, Span::new(start, self.position)));
                        continue;
                    }

                    if let Some('|') = next_char {
                        self.advance();

                        result.push(Token::new(TokenKind::OrOr, Span::new(start, self.position)));
                        continue;
                    }

                    result.push(Token::new(TokenKind::Or, Span::new(start, self.position)))
                }
                Some('^') => {
                    let next_char = self.peek();

                    if let Some('=') = next_char {
                        self.advance();

                        result.push(Token::new(TokenKind::CaretEqual, Span::new(start, self.position)));
                        continue;
                    }

                    result.push(Token::new(TokenKind::Caret, Span::new(start, self.position)))
                }
                Some('~') => result.push(Token::new(TokenKind::Tilde, Span::new(start, self.position))),

                Some(',') => result.push(Token::new(TokenKind::Comma, Span::new(start, self.position))),
                Some(';') => result.push(Token::new(TokenKind::Semicolon, Span::new(start, self.position))),
                Some(':') => {
                    let next_char = self.peek();

                    if let Some(':') = next_char {
                        self.advance();

                        result.push(Token::new(TokenKind::ColonColon, Span::new(start, self.position)));
                        continue;
                    }

                    result.push(Token::new(TokenKind::Colon, Span::new(start, self.position)))
                }

                Some('.') => {
                    let next_char = self.peek();

                    if let Some('.') = next_char {
                        self.advance();

                        result.push(Token::new(TokenKind::DotDot, Span::new(start, self.position)));
                        continue;
                    }

                    result.push(Token::new(TokenKind::Dot, Span::new(start, self.position)))
                }

                Some('(') => result.push(Token::new(TokenKind::LeftParen, Span::new(start, self.position))),
                Some(')') => result.push(Token::new(TokenKind::RightParen, Span::new(start, self.position))),

                Some('{') => result.push(Token::new(TokenKind::LeftBrace, Span::new(start, self.position))),
                Some('}') => result.push(Token::new(TokenKind::RightBrace, Span::new(start, self.position))),

                Some('[') => result.push(Token::new(TokenKind::LeftBracket, Span::new(start, self.position))),
                Some(']') => result.push(Token::new(TokenKind::RightBracket, Span::new(start, self.position))),

                Some('\'') => result.push(self.parse_char()?),
                Some('"') => result.push(self.parse_string()?),

                None => {
                    result.push(Token::new(TokenKind::Eof, Span::new(start, start)));
                    break;
                }

                Some(unwrapped_char) => {
                    if unwrapped_char.is_whitespace() {
                        continue;
                    }

                    if unwrapped_char.is_ascii_alphabetic() || unwrapped_char == '_' {
                        result.push(self.parse_identifier(unwrapped_char));
                        continue;
                    }

                    if unwrapped_char.is_ascii_digit() {
                        result.push(self.parse_numeric(unwrapped_char));
                        continue;
                    }

                    return Err(self.error(
                        format!("unexpected character '{unwrapped_char}'"),
                        start,
                    ));
                }
            }
        }

        Ok(result)
    }

    fn peek(&self) -> Option<char> {
        self.source[self.position..].chars().next()
    }

    fn advance(&mut self) -> Option<char> {
        let char = self.peek()?;

        self.position += char.len_utf8();

        Some(char)
    }

    fn error(&self, message: impl Into<String>, start: usize) -> LexError {
        LexError::new(message, Span::new(start, self.position))
    }

    fn parse_escape(&mut self, start: usize) -> LexResult<char> {
        match self.advance() {
            Some('n') => Ok('\n'),
            Some('r') => Ok('\r'),
            Some('t') => Ok('\t'),
            Some('\\') => Ok('\\'),
            Some('"') => Ok('"'),
            Some('\'') => Ok('\''),
            Some(c) => Err(self.error(format!("invalid escape sequence: \\{c}"), start)),
            None => Err(self.error("unterminated escape sequence", start)),
        }
    }

    fn parse_char(&mut self) -> LexResult<Token> {
        let start = self.position - 1;

        let value = match self.advance() {
            Some('\\') => self.parse_escape(start)?,
            Some('\'') => return Err(self.error("empty character literal", start)),
            Some(c) => c,
            None => return Err(self.error("unterminated character literal", start)),
        };

        match self.advance() {
            Some('\'') => Ok(Token::new(
                TokenKind::Char(value),
                Span::new(start, self.position),
            )),
            _ => Err(self.error("character literals must contain exactly one character", start)),
        }
    }

    fn parse_string(&mut self) -> LexResult<Token> {
        let mut value = String::new();
        let start = self.position - 1;

        while let Some(next_char) = self.peek() {
            if next_char == '"' {
                self.advance();

                return Ok(Token::new(
                    TokenKind::String(value),
                    Span::new(start, self.position),
                ));
            }

            if next_char == '\\' {
                self.advance();

                value.push(self.parse_escape(start)?);
                continue;
            }

            value.push(next_char);
            self.advance();
        }

        Err(self.error("unterminated string literal", start))
    }

    fn parse_identifier(&mut self, first_char: char) -> Token {
        let mut value = first_char.to_string();
        let start = self.position - 1;

        while let Some(next_char) = self.peek() {
            if !(next_char.is_ascii_alphanumeric() || next_char == '_') {
                return Token::new(
                    self.parse_keyword(value),
                    Span::new(start, self.position),
                );
            }

            value.push(next_char);
            self.advance();
        }

        Token::new(
            self.parse_keyword(value),
            Span::new(start, self.position),
        )
    }

    fn parse_keyword(&self, value: String) -> TokenKind {
        return match value.as_str() {
            "fn" => TokenKind::Fn,
            "struct" => TokenKind::Struct,
            "enum" => TokenKind::Enum,
            "impl" => TokenKind::Impl,
            "const" => TokenKind::Const,
            "let" => TokenKind::Let,
            "mut" => TokenKind::Mut,
            "import" => TokenKind::Import,
            "pub" => TokenKind::Pub,

            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "match" => TokenKind::Match,
            "return" => TokenKind::Return,

            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "self" => TokenKind::Self_,

            "_" => TokenKind::Underscore,

            _ => TokenKind::Identifier(value)
        };
    }

    fn parse_numeric(&mut self, first_char: char) -> Token {
        let mut value = first_char.to_string();
        let start = self.position - 1;

        while let Some(next_char) = self.peek() {
            if !(next_char.is_ascii_digit() || next_char == '_' || (!value.contains('.') && next_char == '.')) {
                return if value.contains('.') {
                    Token::new(
                        TokenKind::Float(value),
                        Span::new(start, self.position),
                    )
                } else {
                    Token::new(
                        TokenKind::Integer(value),
                        Span::new(start, self.position),
                    )
                };
            }

            value.push(next_char);
            self.advance();
        }

        return if value.contains('.') {
            Token::new(
                TokenKind::Float(value),
                Span::new(start, self.position),
            )
        } else {
            Token::new(
                TokenKind::Integer(value),
                Span::new(start, self.position),
            )
        };
    }
}
