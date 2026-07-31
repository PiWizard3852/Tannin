use compiler::lexer::Scanner;
use compiler::parser::{Parser,ast::*};

fn parse(source: &str) -> Program {
    let mut scanner = Scanner::new(source);

    let tokens = scanner
        .scan_tokens()
        .into_iter()
        .collect();

    let mut parser = Parser::new(tokens);
    parser.parse().expect("parsing failed")
}

mod declarations {
    use super::*;

    #[test]
    fn parses_function() {
        let program = parse(
            r#"
            pub fn add(a: i32, b: i32) -> i32 {
                return a + b;
            }
            "#,
        );

        assert_eq!(program.declarations.len(), 1);

        match &program.declarations[0] {
            Declaration::Function(function) => {
                assert_eq!(function.name, "add");
                assert_eq!(function.pub_, true);
                assert_eq!(function.parameters.len(), 2);
                assert!(function.return_type.is_some());
                assert_eq!(function.body.statements.len(), 1);
            }

            _ => panic!("expected function declaration"),
        }
    }

    #[test]
    fn parses_function_without_return_type() {
        let program = parse(
            r#"
            fn hello() {
                return;
            }
            "#,
        );

        match &program.declarations[0] {
            Declaration::Function(function) => {
                assert_eq!(function.name, "hello");
                assert_eq!(function.pub_, false);
                assert!(function.parameters.is_empty());
                assert!(function.return_type.is_none());
            }

            _ => panic!("expected function declaration"),
        }
    }

    #[test]
    fn parses_struct() {
        let program = parse(
            r#"
            struct Point {
                x: i32,
                y: i32,
            }
            "#,
        );

        match &program.declarations[0] {
            Declaration::Struct(struct_decl) => {
                assert_eq!(struct_decl.name, "Point");
                assert_eq!(struct_decl.fields.len(), 2);

                assert_eq!(struct_decl.fields[0].name, "x");
                assert_eq!(struct_decl.fields[1].name, "y");
            }

            _ => panic!("expected struct declaration"),
        }
    }

    #[test]
    fn parses_enum() {
        let program = parse(
            r#"
            enum Direction {
                North,
                South,
                East,
                West,
            }
            "#,
        );

        match &program.declarations[0] {
            Declaration::Enum(enum_decl) => {
                assert_eq!(enum_decl.name, "Direction");
                assert_eq!(enum_decl.variants.len(), 4);
                assert_eq!(enum_decl.variants[0].name, "North");
                assert_eq!(enum_decl.variants[3].name, "West");
            }

            _ => panic!("expected enum declaration"),
        }
    }

    #[test]
    fn parses_const() {
        let program = parse(
            r#"
            const ANSWER: i32 = 42;
            "#,
        );

        match &program.declarations[0] {
            Declaration::Const(const_decl) => {
                assert_eq!(const_decl.name, "ANSWER");

                match &const_decl.type_ {
                    Type::Named { name, .. } => assert_eq!(name, "i32"),
                    _ => panic!("expected named type"),
                }

                match &const_decl.value {
                    Expression::Literal {
                        value: Literal::Integer(value),
                        ..
                    } => {
                        assert_eq!(value, "42");
                    }

                    _ => panic!("expected integer literal"),
                }
            }

            _ => panic!("expected const declaration"),
        }
    }

    #[test]
    fn parses_multiple_declarations() {
        let program = parse(
            r#"
            const X: i32 = 5;

            struct Point {
                x: i32,
                y: i32,
            }

            enum Direction {
                North,
                South,
            }

            fn main() {
                return;
            }
            "#,
        );

        assert_eq!(program.declarations.len(), 4);

        assert!(matches!(
            program.declarations[0],
            Declaration::Const(_)
        ));

        assert!(matches!(
            program.declarations[1],
            Declaration::Struct(_)
        ));

        assert!(matches!(
            program.declarations[2],
            Declaration::Enum(_)
        ));

        assert!(matches!(
            program.declarations[3],
            Declaration::Function(_)
        ));
    }
}

mod expressions {
    use super::*;

    #[test]
    fn parses_integer_literal() {
        let program = parse(
            r#"
            fn main() {
                42;
            }
            "#,
        );

        match &program.declarations[0] {
            Declaration::Function(function) => {
                match &function.body.statements[0] {
                    Statement::Expression { expression, .. } => {
                        assert!(matches!(
                            expression,
                            Expression::Literal {
                                value: Literal::Integer(value),
                                ..
                            } if value == "42"
                        ));
                    }

                    _ => panic!("expected expression statement"),
                }
            }

            _ => panic!("expected function"),
        }
    }

    #[test]
    fn parses_binary_expression() {
        let program = parse(
            r#"
            fn main() {
                1 + 2 * 3;
            }
            "#,
        );

        match &program.declarations[0] {
            Declaration::Function(function) => {
                match &function.body.statements[0] {
                    Statement::Expression { expression, .. } => {
                        match expression {
                            Expression::Binary {
                                operator: BinaryOperator::Add,
                                left,
                                right,
                                ..
                            } => {
                                assert!(matches!(
                                    **left,
                                    Expression::Literal {
                                        value: Literal::Integer(_),
                                        ..
                                    }
                                ));

                                assert!(matches!(
                                    **right,
                                    Expression::Binary {
                                        operator: BinaryOperator::Multiply,
                                        ..
                                    }
                                ));
                            }

                            _ => panic!("expected addition"),
                        }
                    }

                    _ => panic!("expected expression statement"),
                }
            }

            _ => panic!("expected function"),
        }
    }

    #[test]
    fn respects_operator_precedence() {
        let program = parse(
            r#"
            fn main() {
                1 + 2 * 3;
            }
            "#,
        );

        match &program.declarations[0] {
            Declaration::Function(function) => {
                match &function.body.statements[0] {
                    Statement::Expression { expression, .. } => {
                        let Expression::Binary {
                            operator,
                            right,
                            ..
                        } = expression
                        else {
                            panic!("expected binary expression");
                        };

                        assert_eq!(*operator, BinaryOperator::Add);

                        assert!(matches!(
                            **right,
                            Expression::Binary {
                                operator: BinaryOperator::Multiply,
                                ..
                            }
                        ));
                    }

                    _ => panic!("expected expression statement"),
                }
            }

            _ => panic!("expected function"),
        }
    }

    #[test]
    fn parses_parenthesized_expression() {
        let program = parse(
            r#"
            fn main() {
                (1 + 2) * 3;
            }
            "#,
        );

        match &program.declarations[0] {
            Declaration::Function(function) => {
                match &function.body.statements[0] {
                    Statement::Expression { expression, .. } => {
                        match expression {
                            Expression::Binary {
                                operator: BinaryOperator::Multiply,
                                left,
                                ..
                            } => {
                                assert!(matches!(
                                    **left,
                                    Expression::Binary {
                                        operator: BinaryOperator::Add,
                                        ..
                                    }
                                ));
                            }

                            _ => panic!("expected multiplication"),
                        }
                    }

                    _ => panic!("expected expression statement"),
                }
            }

            _ => panic!("expected function"),
        }
    }

    #[test]
    fn parses_unary_expression() {
        let program = parse(
            r#"
            fn main() {
                -x;
            }
            "#,
        );

        match &program.declarations[0] {
            Declaration::Function(function) => {
                match &function.body.statements[0] {
                    Statement::Expression { expression, .. } => {
                        assert!(matches!(
                            expression,
                            Expression::Unary {
                                operator: UnaryOperator::Negate,
                                ..
                            }
                        ));
                    }

                    _ => panic!("expected expression statement"),
                }
            }

            _ => panic!("expected function"),
        }
    }

    #[test]
    fn parses_assignment() {
        let program = parse(
            r#"
            fn main() {
                x += 5;
            }
            "#,
        );

        match &program.declarations[0] {
            Declaration::Function(function) => {
                match &function.body.statements[0] {
                    Statement::Expression { expression, .. } => {
                        match expression {
                            Expression::Assignment {
                                operator,
                                target,
                                value,
                                ..
                            } => {
                                assert_eq!(*operator, AssignmentOperator::AddAssign);

                                assert!(matches!(
                                    **target,
                                    Expression::Identifier { .. }
                                ));

                                assert!(matches!(
                                    **value,
                                    Expression::Literal { .. }
                                ));
                            }

                            _ => panic!("expected assignment"),
                        }
                    }

                    _ => panic!("expected expression statement"),
                }
            }

            _ => panic!("expected function"),
        }
    }

    #[test]
    fn parses_right_associative_assignment() {
        let program = parse(
            r#"
            fn main() {
                a = b = 5;
            }
            "#,
        );

        match &program.declarations[0] {
            Declaration::Function(function) => {
                match &function.body.statements[0] {
                    Statement::Expression { expression, .. } => {
                        match expression {
                            Expression::Assignment {
                                target,
                                value,
                                ..
                            } => {
                                assert!(matches!(
                                    **target,
                                    Expression::Identifier { .. }
                                ));

                                assert!(matches!(
                                    **value,
                                    Expression::Assignment { .. }
                                ));
                            }

                            _ => panic!("expected assignment"),
                        }
                    }

                    _ => panic!("expected expression statement"),
                }
            }

            _ => panic!("expected function"),
        }
    }

    #[test]
    fn parses_function_call() {
        let program = parse(
            r#"
            fn main() {
                add(1, 2);
            }
            "#,
        );

        match &program.declarations[0] {
            Declaration::Function(function) => {
                match &function.body.statements[0] {
                    Statement::Expression { expression, .. } => {
                        match expression {
                            Expression::Call {
                                callee,
                                arguments,
                                ..
                            } => {
                                assert!(matches!(
                                    **callee,
                                    Expression::Identifier { .. }
                                ));

                                assert_eq!(arguments.len(), 2);
                            }

                            _ => panic!("expected call"),
                        }
                    }

                    _ => panic!("expected expression statement"),
                }
            }

            _ => panic!("expected function"),
        }
    }

    #[test]
    fn parses_member_access() {
        let program = parse(
            r#"
            fn main() {
                point.x;
            }
            "#,
        );

        match &program.declarations[0] {
            Declaration::Function(function) => {
                match &function.body.statements[0] {
                    Statement::Expression { expression, .. } => {
                        match expression {
                            Expression::Member {
                                object,
                                member,
                                ..
                            } => {
                                assert_eq!(member, "x");

                                assert!(matches!(
                                    **object,
                                    Expression::Identifier { .. }
                                ));
                            }

                            _ => panic!("expected member access"),
                        }
                    }

                    _ => panic!("expected expression statement"),
                }
            }

            _ => panic!("expected function"),
        }
    }

    #[test]
    fn parses_index_access() {
        let program = parse(
            r#"
            fn main() {
                array[5];
            }
            "#,
        );

        match &program.declarations[0] {
            Declaration::Function(function) => {
                match &function.body.statements[0] {
                    Statement::Expression { expression, .. } => {
                        assert!(matches!(
                            expression,
                            Expression::Index { .. }
                        ));
                    }

                    _ => panic!("expected expression statement"),
                }
            }

            _ => panic!("expected function"),
        }
    }

    #[test]
    fn parses_chained_postfix_expressions() {
        let program = parse(
            r#"
            fn main() {
                foo.bar[0](x);
            }
            "#,
        );

        match &program.declarations[0] {
            Declaration::Function(function) => {
                match &function.body.statements[0] {
                    Statement::Expression { expression, .. } => {
                        assert!(matches!(
                            expression,
                            Expression::Call { .. }
                        ));
                    }

                    _ => panic!("expected expression statement"),
                }
            }

            _ => panic!("expected function"),
        }
    }

    #[test]
    fn parses_post_increment() {
        let program = parse(
            r#"
            fn main() {
                x++;
            }
            "#,
        );

        match &program.declarations[0] {
            Declaration::Function(function) => {
                match &function.body.statements[0] {
                    Statement::Expression { expression, .. } => {
                        assert!(matches!(
                            expression,
                            Expression::Unary {
                                operator: UnaryOperator::PostIncrement,
                                ..
                            }
                        ));
                    }

                    _ => panic!("expected expression statement"),
                }
            }

            _ => panic!("expected function"),
        }
    }
}

mod statements {
    use super::*;

    #[test]
    fn parses_let_with_initializer() {
        let program = parse(
            r#"
            fn main() {
                let x = 5;
            }
            "#,
        );

        match &program.declarations[0] {
            Declaration::Function(function) => {
                match &function.body.statements[0] {
                    Statement::Let {
                        name,
                        mutable,
                        type_,
                        initializer,
                        ..
                    } => {
                        assert_eq!(name, "x");
                        assert!(!mutable);
                        assert!(type_.is_none());
                        assert!(initializer.is_some());
                    }

                    _ => panic!("expected let statement"),
                }
            }

            _ => panic!("expected function"),
        }
    }

    #[test]
    fn parses_mutable_typed_let() {
        let program = parse(
            r#"
            fn main() {
                let mut x: i32 = 5;
            }
            "#,
        );

        match &program.declarations[0] {
            Declaration::Function(function) => {
                match &function.body.statements[0] {
                    Statement::Let {
                        name,
                        mutable,
                        type_,
                        initializer,
                        ..
                    } => {
                        assert_eq!(name, "x");
                        assert!(*mutable);
                        assert!(type_.is_some());
                        assert!(initializer.is_some());
                    }

                    _ => panic!("expected let statement"),
                }
            }

            _ => panic!("expected function"),
        }
    }

    #[test]
    fn parses_let_without_initializer() {
        let program = parse(
            r#"
            fn main() {
                let x: i32;
            }
            "#,
        );

        match &program.declarations[0] {
            Declaration::Function(function) => {
                match &function.body.statements[0] {
                    Statement::Let {
                        initializer,
                        ..
                    } => {
                        assert!(initializer.is_none());
                    }

                    _ => panic!("expected let statement"),
                }
            }

            _ => panic!("expected function"),
        }
    }

    #[test]
    fn parses_while() {
        let program = parse(
            r#"
            fn main() {
                while x < 10 {
                    x++;
                }
            }
            "#,
        );

        match &program.declarations[0] {
            Declaration::Function(function) => {
                match &function.body.statements[0] {
                    Statement::While {
                        condition,
                        body,
                        ..
                    } => {
                        assert!(matches!(
                            condition,
                            Expression::Binary {
                                operator: BinaryOperator::Less,
                                ..
                            }
                        ));

                        assert!(matches!(
                            body,
                            Block { .. }
                        ));
                    }

                    _ => panic!("expected while statement"),
                }
            }

            _ => panic!("expected function"),
        }
    }

    #[test]
    fn parses_return_with_value() {
        let program = parse(
            r#"
            fn main() -> i32 {
                return 42;
            }
            "#,
        );

        match &program.declarations[0] {
            Declaration::Function(function) => {
                match &function.body.statements[0] {
                    Statement::Return { value, .. } => {
                        assert!(value.is_some());
                    }

                    _ => panic!("expected return statement"),
                }
            }

            _ => panic!("expected function"),
        }
    }

    #[test]
    fn parses_return_without_value() {
        let program = parse(
            r#"
            fn main() {
                return;
            }
            "#,
        );

        match &program.declarations[0] {
            Declaration::Function(function) => {
                match &function.body.statements[0] {
                    Statement::Return { value, .. } => {
                        assert!(value.is_none());
                    }

                    _ => panic!("expected return statement"),
                }
            }

            _ => panic!("expected function"),
        }
    }

    #[test]
    fn parses_nested_blocks() {
        let program = parse(
            r#"
            fn main() {
                {
                    {
                        42;
                    }
                }
            }
            "#,
        );

        match &program.declarations[0] {
            Declaration::Function(function) => {
                assert_eq!(function.body.statements.len(), 1);

                match &function.body.statements[0] {
                    Statement::Block(block) => {
                        assert_eq!(block.statements.len(), 1);
                    }

                    _ => panic!("expected block"),
                }
            }

            _ => panic!("expected function"),
        }
    }
}

mod patterns {
    use super::*;

    #[test]
    fn parses_identifier_pattern() {
        let program = parse(
            r#"
            fn main() {
                match x {
                    value => value,
                };
            }
            "#,
        );

        match &program.declarations[0] {
            Declaration::Function(function) => {
                match &function.body.statements[0] {
                    Statement::Expression { expression, .. } => {
                        match expression {
                            Expression::Match { arms, .. } => {
                                assert!(matches!(
                                    arms[0].pattern,
                                    Pattern::Identifier { .. }
                                ));
                            }

                            _ => panic!("expected match expression"),
                        }
                    }

                    _ => panic!("expected expression statement"),
                }
            }

            _ => panic!("expected function"),
        }
    }

    #[test]
    fn parses_literal_pattern() {
        let program = parse(
            r#"
            fn main() {
                match x {
                    5 => 10,
                };
            }
            "#,
        );

        match &program.declarations[0] {
            Declaration::Function(function) => {
                match &function.body.statements[0] {
                    Statement::Expression { expression, .. } => {
                        match expression {
                            Expression::Match { arms, .. } => {
                                assert!(matches!(
                                    arms[0].pattern,
                                    Pattern::Literal { .. }
                                ));
                            }

                            _ => panic!("expected match expression"),
                        }
                    }

                    _ => panic!("expected expression statement"),
                }
            }

            _ => panic!("expected function"),
        }
    }

    #[test]
    fn parses_wildcard_pattern() {
        let program = parse(
            r#"
            fn main() {
                match x {
                    _ => 0,
                };
            }
            "#,
        );

        match &program.declarations[0] {
            Declaration::Function(function) => {
                match &function.body.statements[0] {
                    Statement::Expression { expression, .. } => {
                        match expression {
                            Expression::Match { arms, .. } => {
                                assert!(matches!(
                                    arms[0].pattern,
                                    Pattern::Wildcard { .. }
                                ));
                            }

                            _ => panic!("expected match expression"),
                        }
                    }

                    _ => panic!("expected expression statement"),
                }
            }

            _ => panic!("expected function"),
        }
    }
}

mod types {
    use super::*;

    #[test]
    fn parses_named_type() {
        let program = parse(
            r#"
            fn main(x: i32) {
                return;
            }
            "#,
        );

        match &program.declarations[0] {
            Declaration::Function(function) => {
                match &function.parameters[0].type_ {
                    Type::Named { name, .. } => {
                        assert_eq!(name, "i32");
                    }

                    _ => panic!("expected named type"),
                }
            }

            _ => panic!("expected function"),
        }
    }

    #[test]
    fn parses_array_type() {
        let program = parse(
            r#"
            fn main(x: [i32]) {
                return;
            }
            "#,
        );

        match &program.declarations[0] {
            Declaration::Function(function) => {
                match &function.parameters[0].type_ {
                    Type::Array { element, .. } => {
                        assert!(matches!(
                            **element,
                            Type::Named { .. }
                        ));
                    }

                    _ => panic!("expected array type"),
                }
            }

            _ => panic!("expected function"),
        }
    }
}