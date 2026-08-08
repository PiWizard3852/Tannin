use std::mem::discriminant;

use crate::common::Span;
use crate::lexer::{Token, TokenKind};
use crate::lexer::TokenKind::Eof;
use crate::parser::ast::*;
use crate::parser::error::{ParseError, ParseResult};
use crate::parser::precedence::Precedence;

pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    pub fn parse(&mut self) -> ParseResult<Program> {
        let mut declarations = Vec::new();

        while self.peek().kind != Eof {
            declarations.push(self.parse_declaration(false)?);
        }

        Ok(Program { declarations })
    }

    fn parse_declaration(&mut self, pub_: bool) -> ParseResult<Declaration> {
        let current_token = self.peek();

        return match current_token.kind {
            TokenKind::Pub => {
                self.advance();
                Ok(self.parse_declaration(true)?)
            }

            TokenKind::Fn => {
                Ok(Declaration::Function(self.parse_function(pub_)?))
            }

            TokenKind::Struct => {
                Ok(Declaration::Struct(self.parse_struct(pub_)?))
            }

            TokenKind::Enum => {
                Ok(Declaration::Enum(self.parse_enum(pub_)?))
            }

            TokenKind::Const => {
                Ok(Declaration::Const(self.parse_const(pub_)?))
            }

            _ => Err(ParseError::new(
                "expected declaration",
                current_token.span,
            )),
        };
    }

    // ======================================================
    // Declarations
    // ======================================================

    fn parse_function(&mut self, pub_: bool) -> ParseResult<FunctionDecl> {
        let start = self.consume(TokenKind::Fn, "expected 'fn'")?.span.start;

        let (_, name) = self.consume_identifier(None)?;

        self.consume(
            TokenKind::LeftParen,
            "expected '(' after function name",
        )?;

        let parameters = self.parse_parameter_list()?;

        self.consume(
            TokenKind::RightParen,
            "expected ')' after parameter list",
        )?;

        let return_type = if self.matches(TokenKind::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };

        let body = self.parse_block()?;
        let end = body.span.end;

        Ok(FunctionDecl {
            name,
            pub_,
            parameters,
            return_type,
            body,
            span: Span {
                start,
                end,
            },
        })
    }

    fn parse_struct(&mut self, pub_: bool) -> ParseResult<StructDecl> {
        let start = self.consume(TokenKind::Struct, "expected 'struct'")?.span.start;

        let (_, name) = self.consume_identifier(None)?;

        self.consume(
            TokenKind::LeftBrace,
            "expected '{' after struct name",
        )?;

        let fields = self.parse_struct_field_list()?;

        let end = self.consume(TokenKind::RightBrace, "expected '}' after struct fields")?.span.end;

        Ok(StructDecl {
            name,
            pub_,
            fields,
            span: Span {
                start,
                end,
            },
        })
    }

    fn parse_enum(&mut self, pub_: bool) -> ParseResult<EnumDecl> {
        let start = self.consume(TokenKind::Enum, "expected 'enum'")?.span.start;

        let (_, name) = self.consume_identifier(None)?;

        self.consume(
            TokenKind::LeftBrace,
            "expected '{' after enum name",
        )?;

        let variants = self.parse_enum_variants_list()?;

        let end = self.consume(TokenKind::RightBrace, "expected '}' after enum variants")?.span.end;

        Ok(EnumDecl {
            name,
            pub_,
            variants,
            span: Span {
                start,
                end,
            },
        })
    }

    fn parse_const(&mut self, pub_: bool) -> ParseResult<ConstDecl> {
        let start = self.consume(TokenKind::Const, "expected 'const'")?.span.start;

        let (_, name) = self.consume_identifier(None)?;

        self.consume(
            TokenKind::Colon,
            "expected a type after const name",
        )?;

        let type_ = self.parse_type()?;

        self.consume(
            TokenKind::Equal,
            "expected an assignment after const type",
        )?;

        let value = self.parse_expression(Precedence::Lowest)?;

        let end = self.consume(TokenKind::Semicolon, "expected ';' after assignment expression")?.span.end;

        Ok(ConstDecl {
            name,
            pub_,
            type_,
            value,
            span: Span {
                start,
                end,
            },
        })
    }

    // ======================================================
    // Statements
    // ======================================================

    fn parse_block(&mut self) -> ParseResult<Block> {
        let start = self.consume(
            TokenKind::LeftBrace,
            "expected '{'",
        )?.span.start;

        let mut statements = Vec::new();
        let mut tail = None;

        while !self.check(TokenKind::RightBrace) {
            if self.check(Eof) {
                return Err(ParseError::new(
                    "expected '}' after block",
                    self.peek().span,
                ));
            }

            let statement = match self.peek().kind {
                TokenKind::Let => self.parse_let_statement()?,
                TokenKind::While => self.parse_while_statement()?,
                TokenKind::Return => self.parse_return_statement()?,
                _ => match self.parse_expression(Precedence::Lowest)? {
                    Expression::Block(block)
                        if !self.check(TokenKind::Semicolon)
                            && (!self.check(TokenKind::RightBrace) || block.tail.is_none()) =>
                    {
                        Statement::Block(block)
                    }

                    expression if self.check(TokenKind::RightBrace) => {
                        tail = Some(Box::new(expression));
                        break;
                    }

                    expression => self.parse_expression_statement(expression)?,
                },
            };

            statements.push(statement);
        }

        let end = self.consume(
            TokenKind::RightBrace,
            "expected '}' after block",
        )?.span.end;

        Ok(Block {
            statements,
            tail,
            span: Span {
                start,
                end,
            },
        })
    }

    fn parse_let_statement(&mut self) -> ParseResult<Statement> {
        let start = self.consume(
            TokenKind::Let,
            "expected 'let'",
        )?.span.start;

        let mutable = self.matches(TokenKind::Mut);

        let (_, name) = self.consume_identifier(
            Some("expected identifier after 'let'"),
        )?;

        let type_ = if self.matches(TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };

        let initializer = if self.matches(TokenKind::Equal) {
            Some(self.parse_expression(Precedence::Lowest)?)
        } else {
            None
        };

        let end = self.consume(
            TokenKind::Semicolon,
            "expected ';' after variable declaration",
        )?.span.end;

        Ok(Statement::Let {
            name,
            mutable,
            type_,
            initializer,
            span: Span {
                start,
                end,
            },
        })
    }

    fn parse_while_statement(&mut self) -> ParseResult<Statement> {
        let start = self.consume(
            TokenKind::While,
            "expected 'while'",
        )?.span.start;

        let condition = self.parse_expression(Precedence::Lowest)?;

        let body = self.parse_block()?;

        let end = body.span.end;

        Ok(Statement::While {
            condition,
            body,
            span: Span {
                start,
                end,
            },
        })
    }

    fn parse_return_statement(&mut self) -> ParseResult<Statement> {
        let start = self.consume(
            TokenKind::Return,
            "expected 'return'",
        )?.span.start;

        let value = if self.check(TokenKind::Semicolon) {
            None
        } else {
            Some(self.parse_expression(Precedence::Lowest)?)
        };

        let end = self.consume(
            TokenKind::Semicolon,
            "expected ';' after return statement",
        )?.span.end;

        Ok(Statement::Return {
            value,
            span: Span {
                start,
                end,
            },
        })
    }

    fn parse_expression_statement(&mut self, expression: Expression) -> ParseResult<Statement> {
        let start = expression.span().start;

        let end = self.consume(
            TokenKind::Semicolon,
            "expected ';' after expression",
        )?.span.end;

        Ok(Statement::Expression {
            expression,
            span: Span {
                start,
                end,
            },
        })
    }

    // ======================================================
    // Expressions
    // ======================================================

    fn parse_expression(
        &mut self,
        precedence: Precedence,
    ) -> ParseResult<Expression> {
        let mut left = self.parse_prefix()?;

        loop {
            let next_precedence = Precedence::from_token(&self.peek().kind);

            let should_parse = if next_precedence.is_right_associative() {
                precedence <= next_precedence
            } else {
                precedence < next_precedence
            };

            if !should_parse {
                break;
            }

            left = self.parse_infix(left)?;
        }

        Ok(left)
    }

    fn parse_prefix(&mut self) -> ParseResult<Expression> {
        let token = self.peek().clone();

        match token.kind {
            TokenKind::Minus
            | TokenKind::Not
            | TokenKind::Tilde
            | TokenKind::PlusPlus
            | TokenKind::MinusMinus => {
                let operator = match token.kind {
                    TokenKind::Minus => UnaryOperator::Negate,
                    TokenKind::Not => UnaryOperator::Not,
                    TokenKind::Tilde => UnaryOperator::BitwiseNot,
                    TokenKind::PlusPlus => UnaryOperator::PreIncrement,
                    TokenKind::MinusMinus => UnaryOperator::PreDecrement,
                    _ => unreachable!(),
                };

                let start = self.advance().span.start;

                let operand = self.parse_expression(Precedence::Unary)?;

                let end = operand.span().end;

                Ok(Expression::Unary {
                    operator,
                    operand: Box::new(operand),
                    span: Span {
                        start,
                        end,
                    },
                })
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> ParseResult<Expression> {
        let token = self.peek().clone();

        match token.kind {
            TokenKind::Integer(value) => {
                self.advance();

                Ok(Expression::Literal {
                    value: Literal::Integer(value),
                    span: token.span,
                })
            }
            TokenKind::Float(value) => {
                self.advance();

                Ok(Expression::Literal {
                    value: Literal::Float(value),
                    span: token.span,
                })
            }
            TokenKind::String(value) => {
                self.advance();

                Ok(Expression::Literal {
                    value: Literal::String(value),
                    span: token.span,
                })
            }
            TokenKind::Char(value) => {
                self.advance();

                Ok(Expression::Literal {
                    value: Literal::Char(value),
                    span: token.span,
                })
            }
            TokenKind::True => {
                self.advance();

                Ok(Expression::Literal {
                    value: Literal::Bool(true),
                    span: token.span,
                })
            }
            TokenKind::False => {
                self.advance();

                Ok(Expression::Literal {
                    value: Literal::Bool(false),
                    span: token.span,
                })
            }
            TokenKind::Identifier(name) => {
                self.advance();

                Ok(Expression::Identifier {
                    value: name,
                    span: token.span,
                })
            }
            TokenKind::LeftParen => {
                self.advance();

                let expression = self.parse_expression(Precedence::Lowest)?;

                self.consume(
                    TokenKind::RightParen,
                    "expected ')' after expression",
                )?;

                Ok(expression)
            }
            TokenKind::LeftBrace => Ok(Expression::Block(self.parse_block()?)),
            TokenKind::If => self.parse_if_expression(),
            TokenKind::Match => self.parse_match_expression(),
            _ => Err(ParseError::new(
                "expected expression",
                token.span,
            )),
        }
    }

    fn parse_infix(
        &mut self,
        left: Expression,
    ) -> ParseResult<Expression> {
        match self.peek().kind {
            TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Star
            | TokenKind::Slash
            | TokenKind::Percent

            | TokenKind::EqualEqual
            | TokenKind::NotEqual

            | TokenKind::Less
            | TokenKind::LessEqual
            | TokenKind::Greater
            | TokenKind::GreaterEqual

            | TokenKind::AndAnd
            | TokenKind::OrOr

            | TokenKind::And
            | TokenKind::Or
            | TokenKind::Caret

            | TokenKind::LessLess
            | TokenKind::GreaterGreater
            => self.parse_binary_expression(left),

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
            | TokenKind::GreaterGreaterEqual
            => self.parse_assignment_expression(left),

            TokenKind::LeftParen => self.parse_call_expression(left),
            TokenKind::Dot => self.parse_member_access_expression(left),
            TokenKind::LeftBracket => self.parse_index_expression(left),

            TokenKind::PlusPlus => self.parse_post_increment_expression(left),
            TokenKind::MinusMinus => self.parse_post_decrement_expression(left),

            _ => Err(ParseError::new(
                "expected infix expression",
                self.peek().span,
            )),
        }
    }

    fn parse_binary_expression(
        &mut self,
        left: Expression,
    ) -> ParseResult<Expression> {
        let operator_token = self.advance();

        let precedence = Precedence::from_token(&operator_token.kind);

        let right = self.parse_expression(precedence)?;

        let operator = match operator_token.kind {
            TokenKind::Plus => BinaryOperator::Add,
            TokenKind::Minus => BinaryOperator::Subtract,
            TokenKind::Star => BinaryOperator::Multiply,
            TokenKind::Slash => BinaryOperator::Divide,
            TokenKind::Percent => BinaryOperator::Modulo,

            TokenKind::EqualEqual => BinaryOperator::Equal,
            TokenKind::NotEqual => BinaryOperator::NotEqual,

            TokenKind::Less => BinaryOperator::Less,
            TokenKind::LessEqual => BinaryOperator::LessEqual,
            TokenKind::Greater => BinaryOperator::Greater,
            TokenKind::GreaterEqual => BinaryOperator::GreaterEqual,

            TokenKind::AndAnd => BinaryOperator::LogicalAnd,
            TokenKind::OrOr => BinaryOperator::LogicalOr,

            TokenKind::And => BinaryOperator::BitwiseAnd,
            TokenKind::Or => BinaryOperator::BitwiseOr,
            TokenKind::Caret => BinaryOperator::BitwiseXor,

            TokenKind::LessLess => BinaryOperator::LeftShift,
            TokenKind::GreaterGreater => BinaryOperator::RightShift,

            _ => unreachable!("expected binary operator"),
        };

        let start = left.span().start;
        let end = right.span().end;

        Ok(Expression::Binary {
            left: Box::new(left),
            operator,
            right: Box::new(right),
            span: Span {
                start,
                end,
            },
        })
    }

    fn parse_assignment_expression(
        &mut self,
        target: Expression,
    ) -> ParseResult<Expression> {
        let operator_token = self.advance();

        let operator = match operator_token.kind {
            TokenKind::Equal => AssignmentOperator::Assign,
            TokenKind::PlusEqual => AssignmentOperator::AddAssign,
            TokenKind::MinusEqual => AssignmentOperator::SubAssign,
            TokenKind::StarEqual => AssignmentOperator::MulAssign,
            TokenKind::SlashEqual => AssignmentOperator::DivAssign,
            TokenKind::PercentEqual => AssignmentOperator::ModAssign,
            TokenKind::AndEqual => AssignmentOperator::AndAssign,
            TokenKind::OrEqual => AssignmentOperator::OrAssign,
            TokenKind::CaretEqual => AssignmentOperator::XorAssign,
            TokenKind::LessLessEqual => AssignmentOperator::LeftShiftAssign,
            TokenKind::GreaterGreaterEqual => AssignmentOperator::RightShiftAssign,

            _ => unreachable!("expected assignment operator"),
        };

        let value = self.parse_expression(Precedence::Assignment)?;

        let start = target.span().start;
        let end = value.span().end;

        Ok(Expression::Assignment {
            target: Box::new(target),
            operator,
            value: Box::new(value),
            span: Span {
                start,
                end,
            },
        })
    }

    fn parse_call_expression(
        &mut self,
        callee: Expression,
    ) -> ParseResult<Expression> {
        self.consume(
            TokenKind::LeftParen,
            "expected '('",
        )?;

        let arguments = self.parse_argument_list()?;

        let start = callee.span().start;

        let end = self.consume(
            TokenKind::RightParen,
            "expected ')' after arguments",
        )?.span.end;

        Ok(Expression::Call {
            callee: Box::new(callee),
            arguments,
            span: Span {
                start,
                end,
            },
        })
    }

    fn parse_member_access_expression(
        &mut self,
        object: Expression,
    ) -> ParseResult<Expression> {
        self.consume(TokenKind::Dot, "expected '.'")?;

        let (member_token, member) =
            self.consume_identifier(Some("expected member name after '.'"))?;

        let start = object.span().start;
        let end = member_token.span.end;

        Ok(Expression::Member {
            object: Box::new(object),
            member,
            span: Span {
                start,
                end,
            },
        })
    }

    fn parse_index_expression(
        &mut self,
        object: Expression,
    ) -> ParseResult<Expression> {
        self.consume(
            TokenKind::LeftBracket,
            "expected '['",
        )?;

        let index = self.parse_expression(Precedence::Lowest)?;

        let start = object.span().start;

        let end = self.consume(
            TokenKind::RightBracket,
            "expected ']' after index expression",
        )?.span.end;

        Ok(Expression::Index {
            object: Box::new(object),
            index: Box::new(index),
            span: Span {
                start,
                end,
            },
        })
    }

    fn parse_post_increment_expression(
        &mut self,
        operand: Expression,
    ) -> ParseResult<Expression> {
        let end = self.consume(
            TokenKind::PlusPlus,
            "expected '++'",
        )?.span.end;

        Ok(Expression::Unary {
            operator: UnaryOperator::PostIncrement,
            operand: Box::new(operand.clone()),
            span: Span {
                start: operand.span().start,
                end,
            },
        })
    }

    fn parse_post_decrement_expression(
        &mut self,
        operand: Expression,
    ) -> ParseResult<Expression> {
        let end = self.consume(
            TokenKind::MinusMinus,
            "expected '--'",
        )?.span.end;

        Ok(Expression::Unary {
            operator: UnaryOperator::PostDecrement,
            operand: Box::new(operand.clone()),
            span: Span {
                start: operand.span().start,
                end,
            },
        })
    }

    fn parse_if_expression(&mut self) -> ParseResult<Expression> {
        let start = self.consume(
            TokenKind::If,
            "expected 'if'",
        )?.span.start;

        let condition = self.parse_expression(Precedence::Lowest)?;

        let then_branch = Box::new(
            Expression::Block(self.parse_block()?)
        );

        let else_branch = if self.matches(TokenKind::Else) {
            if self.check(TokenKind::If) {
                Some(Box::new(self.parse_if_expression()?))
            } else {
                Some(Box::new(
                    Expression::Block(self.parse_block()?)
                ))
            }
        } else {
            None
        };

        let end = match &else_branch {
            Some(expression) => expression.span().end,
            None => then_branch.span().end,
        };

        Ok(Expression::If {
            condition: Box::new(condition),
            then_branch,
            else_branch,
            span: Span {
                start,
                end,
            },
        })
    }

    fn parse_match_expression(&mut self) -> ParseResult<Expression> {
        let start = self.consume(
            TokenKind::Match,
            "expected 'match'",
        )?.span.start;

        let expression = self.parse_expression(Precedence::Lowest)?;

        self.consume(
            TokenKind::LeftBrace,
            "expected '{' after match expression",
        )?;

        let mut arms = Vec::new();

        while !self.check(TokenKind::RightBrace) {
            arms.push(self.parse_match_arm()?);
        }

        let end = self.consume(
            TokenKind::RightBrace,
            "expected '}' after match arms",
        )?.span.end;

        Ok(Expression::Match {
            expression: Box::new(expression),
            arms,
            span: Span {
                start,
                end,
            },
        })
    }

    fn parse_match_arm(&mut self) -> ParseResult<MatchArm> {
        let pattern = self.parse_pattern()?;

        let start = pattern.span().start;

        self.consume(
            TokenKind::FatArrow,
            "expected '=>' after pattern",
        )?;

        let value = self.parse_expression(Precedence::Lowest)?;

        let end = self.consume(
            TokenKind::Comma,
            "expected ',' after match arm",
        )?.span.end;

        Ok(MatchArm {
            pattern,
            value,
            span: Span {
                start,
                end,
            },
        })
    }

    // ======================================================
    // Utility parsers
    // ======================================================

    fn parse_pattern(&mut self) -> ParseResult<Pattern> {
        let token = self.peek().clone();

        match token.kind {
            TokenKind::Identifier(value) => {
                self.advance();

                Ok(Pattern::Identifier {
                    value,
                    span: token.span,
                })
            }

            TokenKind::Integer(value) => {
                self.advance();

                Ok(Pattern::Literal {
                    value: Literal::Integer(value),
                    span: token.span,
                })
            }

            TokenKind::Float(value) => {
                self.advance();

                Ok(Pattern::Literal {
                    value: Literal::Float(value),
                    span: token.span,
                })
            }

            TokenKind::String(value) => {
                self.advance();

                Ok(Pattern::Literal {
                    value: Literal::String(value),
                    span: token.span,
                })
            }

            TokenKind::Char(value) => {
                self.advance();

                Ok(Pattern::Literal {
                    value: Literal::Char(value),
                    span: token.span,
                })
            }

            TokenKind::True => {
                self.advance();

                Ok(Pattern::Literal {
                    value: Literal::Bool(true),
                    span: token.span,
                })
            }

            TokenKind::False => {
                self.advance();

                Ok(Pattern::Literal {
                    value: Literal::Bool(false),
                    span: token.span,
                })
            }

            TokenKind::Underscore => {
                self.advance();

                Ok(Pattern::Wildcard {
                    span: token.span,
                })
            }

            _ => Err(ParseError::new(
                "expected pattern",
                token.span,
            )),
        }
    }

    fn parse_type(&mut self) -> ParseResult<Type> {
        if self.check(TokenKind::LeftBracket) {
            let start = self.advance().span.start;

            let element_type = self.parse_type()?;

            let end = self.consume(
                TokenKind::RightBracket,
                "expected ']' after array type",
            )?.span.end;

            return Ok(Type::Array {
                element: Box::new(element_type),
                span: Span {
                    start,
                    end,
                },
            });
        }

        let (token, name) = self.consume_identifier(
            Some("expected type"),
        )?;

        Ok(Type::Named {
            name,
            span: token.span,
        })
    }

    fn parse_parameter_list(&mut self) -> ParseResult<Vec<Parameter>> {
        let mut parameters = Vec::new();

        if self.check(TokenKind::RightParen) {
            return Ok(parameters);
        }

        parameters.push(self.parse_parameter()?);

        while self.matches(TokenKind::Comma) {
            parameters.push(self.parse_parameter()?);
        }

        Ok(parameters)
    }

    fn parse_parameter(&mut self) -> ParseResult<Parameter> {
        let (name_token, name) = self.consume_identifier(
            Some("expected parameter name"),
        )?;

        self.consume(
            TokenKind::Colon,
            "expected ':' after parameter name",
        )?;

        let type_ = self.parse_type()?;

        let start = name_token.span.start;
        let end = type_.span().end;

        Ok(Parameter {
            name,
            type_,
            span: Span {
                start,
                end,
            },
        })
    }

    fn parse_argument_list(&mut self) -> ParseResult<Vec<Expression>> {
        let mut arguments = Vec::new();

        if self.check(TokenKind::RightParen) {
            return Ok(arguments);
        }

        arguments.push(self.parse_expression(Precedence::Lowest)?);

        while self.matches(TokenKind::Comma) {
            arguments.push(self.parse_expression(Precedence::Lowest)?);
        }

        Ok(arguments)
    }

    fn parse_struct_field_list(&mut self) -> ParseResult<Vec<Field>> {
        let mut fields = Vec::new();

        while !self.check(TokenKind::RightBrace) {
            if self.check(Eof) {
                return Err(ParseError::new(
                    "expected '}' after struct fields",
                    self.peek().span,
                ));
            }

            fields.push(self.parse_struct_field()?);
        }

        Ok(fields)
    }

    fn parse_struct_field(&mut self) -> ParseResult<Field> {
        let (name_token, name) = self.consume_identifier(
            Some("expected field name"),
        )?;

        self.consume(
            TokenKind::Colon,
            "expected ':' after field name",
        )?;

        let type_ = self.parse_type()?;

        let end = self.consume(
            TokenKind::Comma,
            "expected ',' after struct field",
        )?.span.end;

        Ok(Field {
            name,
            type_,
            span: Span {
                start: name_token.span.start,
                end,
            },
        })
    }

    fn parse_enum_variants_list(&mut self) -> ParseResult<Vec<EnumVariant>> {
        let mut variants = Vec::new();

        while !self.check(TokenKind::RightBrace) {
            if self.check(Eof) {
                return Err(ParseError::new(
                    "expected '}' after enum variants",
                    self.peek().span,
                ));
            }

            variants.push(self.parse_enum_variant()?);
        }

        Ok(variants)
    }

    fn parse_enum_variant(&mut self) -> ParseResult<EnumVariant> {
        let (name_token, name) = self.consume_identifier(
            Some("expected enum variant name"),
        )?;

        let end = self.consume(
            TokenKind::Comma,
            "expected ',' after enum variant",
        )?.span.end;

        Ok(EnumVariant {
            name,
            span: Span {
                start: name_token.span.start,
                end,
            },
        })
    }

    // ======================================================
    // Helpers
    // ======================================================

    fn peek(&self) -> &Token {
        &self.tokens[self.position]
    }

    fn advance(&mut self) -> Token {
        let token = self.peek().clone();

        if token.kind != Eof {
            self.position += 1;
        }

        token
    }

    fn check(&self, expected: TokenKind) -> bool {
        discriminant(&self.peek().kind) == discriminant(&expected)
    }

    fn matches(&mut self, expected: TokenKind) -> bool {
        if self.check(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn consume(
        &mut self,
        expected: TokenKind,
        message: &str,
    ) -> ParseResult<Token> {
        if self.check(expected) {
            Ok(self.advance().clone())
        } else {
            Err(ParseError::new(message, self.peek().span))
        }
    }

    fn consume_identifier(
        &mut self,
        message: Option<&str>,
    ) -> ParseResult<(Token, String)> {
        match &self.peek().kind {
            TokenKind::Identifier(name) => {
                let name = name.clone();
                let token = self.advance();

                Ok((token, name))
            }

            _ => Err(ParseError::new(
                message.unwrap_or("expected identifier"),
                self.peek().span,
            )),
        }
    }
}
