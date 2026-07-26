use crate::lexer::{Span, Token, TokenKind};

pub struct Scanner {
    chars: Vec<char>,
    position: usize,
}

impl<'a> Scanner {
    pub fn new(source: &'a str) -> Self {
        Self {
            chars: source.chars().collect(),
            position: 0,
        }
    }

    pub fn scan_tokens(&mut self) -> Vec<Token> {
        let mut result: Vec<Token> = Vec::new();

        while self.position <= self.chars.len() {
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
                },
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
                },
                Some('*') => {
                    let next_char = self.peek();

                    if let Some('=') = next_char {
                        self.advance();

                        result.push(Token::new(TokenKind::StarEqual, Span::new(start, self.position)));
                        continue;
                    }

                    result.push(Token::new(TokenKind::Star, Span::new(start, self.position)))
                },
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

                        while let Some(c) = self.peek() {
                            self.advance();

                            if c == '*' {
                                if let Some('/') = self.peek() {
                                    self.advance();

                                    break;
                                }
                            }
                        }

                        continue;
                    }

                    result.push(Token::new(TokenKind::Slash, Span::new(start, self.position)))
                }
                Some('%') => result.push(Token::new(TokenKind::Percent, Span::new(start, self.position))),

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
                },
                Some('!') => {
                    let next_char = self.peek();

                    if let Some('=') = next_char {
                        self.advance();

                        result.push(Token::new(TokenKind::NotEqual, Span::new(start, self.position)));
                        continue;
                    }

                    result.push(Token::new(TokenKind::Not, Span::new(start, self.position)))
                },

                Some('<') => {
                    let next_char = self.peek();

                    if let Some('=') = next_char {
                        self.advance();

                        result.push(Token::new(TokenKind::LessEqual, Span::new(start, self.position)));
                        continue;
                    }

                    result.push(Token::new(TokenKind::Less, Span::new(start, self.position)))
                },
                Some('>') => {
                    let next_char = self.peek();

                    if let Some('=') = next_char {
                        self.advance();

                        result.push(Token::new(TokenKind::GreaterEqual, Span::new(start, self.position)));
                        continue;
                    }

                    result.push(Token::new(TokenKind::Greater, Span::new(start, self.position)))
                },

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
                },
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
                },
                Some('^') => {
                    let next_char = self.peek();

                    if let Some('=') = next_char {
                        self.advance();

                        result.push(Token::new(TokenKind::CaretEqual, Span::new(start, self.position)));
                        continue;
                    }

                    result.push(Token::new(TokenKind::Caret, Span::new(start, self.position)))
                },
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
                },

                Some('.') => {
                    let next_char = self.peek();

                    if let Some('.') = next_char {
                        self.advance();

                        result.push(Token::new(TokenKind::DotDot, Span::new(start, self.position)));
                        continue;
                    }

                    result.push(Token::new(TokenKind::Dot, Span::new(start, self.position)))
                },

                Some('(') => result.push(Token::new(TokenKind::LeftParen, Span::new(start, self.position))),
                Some(')') => result.push(Token::new(TokenKind::RightParen, Span::new(start, self.position))),

                Some('{') => result.push(Token::new(TokenKind::LeftBrace, Span::new(start, self.position))),
                Some('}') => result.push(Token::new(TokenKind::RightBrace, Span::new(start, self.position))),

                Some('[') => result.push(Token::new(TokenKind::LeftBracket, Span::new(start, self.position))),
                Some(']') => result.push(Token::new(TokenKind::RightBracket, Span::new(start, self.position))),

                Some('\'') => result.push(self.parse_char()),
                Some('"') => result.push(self.parse_string()),

                None => result.push(Token::new(TokenKind::Eof, Span::new(start, start))),

                _ => {
                    let unwrapped_char = current_char.unwrap();

                    if unwrapped_char.is_whitespace() {
                        continue;
                    }

                    if unwrapped_char.is_ascii_alphabetic() || unwrapped_char == '_' {
                        result.push(self.parse_identifier(unwrapped_char));
                        continue;
                    }

                    if unwrapped_char.is_numeric() {
                        result.push(self.parse_numeric(unwrapped_char));
                        continue;
                    }

                    panic!("Unexpected character '{}'", unwrapped_char);
                }
            }
        }

        return result;
    }

    fn peek(&self) -> Option<char> {
        return match self.chars.get(self.position) {
            None => None,
            Some(c) => Some(*c)
        }
    }

    fn advance(&mut self) -> Option<char> {
        let char = self.peek();

        self.position += 1;

        return char;
    }

    fn parse_char(&mut self) -> Token {
        let start = self.position - 1;

        let value = match self.advance() {
            Some('\\') => match self.advance() {
                Some('n') => '\n',
                Some('r') => '\r',
                Some('t') => '\t',
                Some('\\') => '\\',
                Some('"') => '"',
                Some('\'') => '\'',
                Some(c) => panic!("Invalid escape sequence: \\{}", c),
                None => panic!("Unterminated character literal."),
            },
            Some(c) => c,
            None => panic!("Unterminated character literal."),
        };

        match self.advance() {
            Some('\'') => Token::new(
                TokenKind::Char(value),
                Span::new(start, self.position),
            ),
            _ => panic!("Character literals must contain exactly one character."),
        }
    }

    fn parse_string(&mut self) -> Token {
        let mut value = String::new();
        let start = self.position - 1;

        while let Some(next_char) = self.peek() {
            if next_char == '"' {
                self.advance();

                return Token::new(
                    TokenKind::String(value),
                    Span::new(start, self.position),
                );
            }

            if next_char == '\\' {
                self.advance();

                let escaped = match self.advance() {
                    Some('n') => '\n',
                    Some('r') => '\r',
                    Some('t') => '\t',
                    Some('\\') => '\\',
                    Some('"') => '"',
                    Some('\'') => '\'',
                    Some(c) => panic!("Invalid escape sequence: \\{}", c),
                    None => panic!("Unterminated escape sequence."),
                };

                value.push(escaped);
                continue;
            }

            value.push(next_char);
            self.advance();
        }

        panic!("Unterminated string literal.");
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

            _ => TokenKind::Identifier(value)
        }
    }

    fn parse_numeric(&mut self, first_char: char) -> Token {
        let mut value = first_char.to_string();
        let start = self.position - 1;

        while let Some(next_char) = self.peek() {
            if !(next_char.is_numeric() || next_char == '_' || (!value.contains('.') && next_char == '.')) {
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
                }
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
        }
    }
}