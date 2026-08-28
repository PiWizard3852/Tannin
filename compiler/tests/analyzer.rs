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

#[test]
fn checks_updates_and_returning_branches() {
    let program = parse(
        r#"
        fn f(flag: bool) -> i32 {
            let mut n: i32 = 10;
            n <<= 1;

            while n > 0 {
                n--;
            }

            if flag { return 1; } else { ++n }
        }

        fn g(flag: bool) -> i32 {
            if flag { return 2; } else { return 3; };
        }
        "#,
    );

    Analyzer::analyze(&program).expect("analysis failed");
}

#[test]
fn rejects_invalid_updates_and_control_flow() {
    for (source, message) in [
        ("fn f() { let n = 1; n++; }", "cannot assign to immutable binding 'n'"),
        ("fn f() { 1 += 2; }", "expected an assignment target"),
        ("fn f() { while 1 {} }", "expected bool, found isize"),
        ("fn f(x: bool) -> i32 { if x { return 1; }; }", "missing return value"),
        ("fn f() { let n = if true { 1 }; }", "branches must produce the same type"),
    ] {
        assert_eq!(analyze_error(source).message, message);
    }
}

#[test]
fn checks_named_struct_fields_and_mutability() {
    for (source, message) in [
        ("struct Point { x: i32, } fn f() { Point {}; }", "missing initializer for 'x'"),
        (
            "struct A { x: i32, } struct B { x: i32, } fn f(a: A) -> B { a }",
            "expected B, found A",
        ),
        (
            "struct A { x: i32, } fn f(a: A) { a.x = 1; }",
            "cannot assign to immutable binding 'a'",
        ),
    ] {
        assert_eq!(analyze_error(source).message, message);
    }
}

#[test]
fn checks_array_element_and_index_types() {
    for (source, message) in [
        ("fn f() { let xs: [i32] = [1, true]; }", "expected i32, found bool"),
        ("fn f() { let xs = []; }", "empty array needs an element type"),
        ("fn f(xs: [i32]) { xs[false]; }", "array index must be an integer"),
        (
            "fn f(xs: [i32]) { xs[0]++; }",
            "cannot assign to immutable binding 'xs'",
        ),
    ] {
        assert_eq!(analyze_error(source).message, message);
    }
}

#[test]
fn checks_enum_variant_names_and_types() {
    for (source, message) in [
        ("enum A { One, One, }", "duplicate variant 'One'"),
        ("enum A { One, } fn f() -> A { A::Two }", "'A' has no variant 'Two'"),
        (
            "enum A { One, } enum B { One, } fn f() -> A { B::One }",
            "expected A, found B",
        ),
    ] {
        assert_eq!(analyze_error(source).message, message);
    }
}
