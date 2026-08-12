use compiler::analyzer::{Analyzer, AnalyzeError};
use compiler::lexer::Scanner;
use compiler::parser::Parser;
use compiler::parser::ast::Program;

fn parse(source: &str) -> Program {
    let tokens = Scanner::new(source).scan_tokens();

    Parser::new(tokens).parse().expect("parsing failed")
}

fn analyze_error(source: &str) -> AnalyzeError {
    Analyzer::analyze(&parse(source)).expect_err("analysis succeeded")
}

#[test]
fn rejects_duplicate_declarations() {
    let source = "struct Name {} fn Name() {}";
    let error = analyze_error(source);

    assert_eq!(error.message, "duplicate declaration of 'Name'");
    assert_eq!(error.span.start, source.find("fn").unwrap());
}

#[test]
fn resolves_forward_calls_and_contextual_literals() {
    let program = parse(
        r#"
        fn first() -> i8 {
            let n: i8 = -128;
            next(n + 1)
        }

        fn next(n: i8) -> i8 {
            return 1 + n;
        }
        "#,
    );

    Analyzer::analyze(&program).expect("analysis failed");
}

#[test]
fn rejects_invalid_function_bodies() {
    for (source, message) in [
        ("fn f() -> i32 { true }", "expected i32, found bool"),
        ("fn f() -> u8 { 256 }", "integer literal out of range for u8"),
        ("fn f() { let n: i32; }", "variable needs an initializer"),
        ("fn f() { return 1; }", "function does not return a value"),
        ("fn f() { f(true); }", "expected 0 arguments, found 1"),
        ("fn f() { { let n = 1; } n; }", "unknown name 'n'"),
    ] {
        assert_eq!(analyze_error(source).message, message);
    }
}
