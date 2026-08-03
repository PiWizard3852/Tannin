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
