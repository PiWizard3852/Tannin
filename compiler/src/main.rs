use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use compiler::analyzer::Analyzer;
use compiler::codegen::emit_llvm;
use compiler::common::Span;
use compiler::lexer::Scanner;
use compiler::parser::Parser;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = std::env::args_os().skip(1);
    let path = match (args.next(), args.next()) {
        (Some(path), None) => PathBuf::from(path),
        _ => return Err("usage: compiler <file.tn>".to_owned()),
    };
    let source = std::fs::read_to_string(&path)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    let tokens = Scanner::new(&source).try_scan_tokens()
        .map_err(|error| diagnostic(&path, &source, &error.message, Some(error.span)))?;
    let program = Parser::new(tokens).parse()
        .map_err(|error| diagnostic(&path, &source, &error.message, Some(error.span)))?;
    let program = Analyzer::analyze(&program)
        .map_err(|error| diagnostic(&path, &source, &error.message, Some(error.span)))?;
    let ir = emit_llvm(&program)
        .map_err(|error| diagnostic(&path, &source, &error.message, error.span))?;

    io::stdout().lock().write_all(ir.as_bytes())
        .map_err(|error| format!("could not write LLVM IR: {error}"))
}

fn diagnostic(path: &Path, source: &str, message: &str, span: Option<Span>) -> String {
    let Some(before) = span.and_then(|span| source.get(..span.start)) else {
        return format!("{}: {message}", path.display());
    };

    let line = before.bytes().filter(|&byte| byte == b'\n').count() + 1;
    let column = before.rsplit('\n').next().unwrap_or("").chars().count() + 1;

    format!("{}:{line}:{column}: {message}", path.display())
}
