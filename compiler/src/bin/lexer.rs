use std::io::{self, Write};

use compiler::lexer::Scanner;

fn main() {
    println!("Tannin Lexer");
    println!("Enter Tannin source code.");
    println!("Type 'exit' or 'quit' to leave.\n");

    loop {
        print!("lexer> ");
        io::stdout().flush().unwrap();

        let mut source = String::new();

        if !read_source(&mut source) {
            break;
        }

        if source.trim().is_empty() {
            continue;
        }

        let mut scanner = Scanner::new(&source);
        let tokens = scanner.scan_tokens();

        for token in tokens {
            println!("{:?}", token);
        }
    }
}

fn read_source(source: &mut String) -> bool {
    let mut brace_depth = 0;
    let mut first_line = true;

    loop {
        if !first_line {
            print!("....> ");
            io::stdout().flush().unwrap();
        }

        let mut line = String::new();

        if io::stdin().read_line(&mut line).unwrap() == 0 {
            return false;
        }

        let trimmed = line.trim();

        if first_line && (trimmed == "exit" || trimmed == "quit") {
            return false;
        }

        brace_depth += count_open_braces(&line);
        brace_depth -= count_close_braces(&line);

        source.push_str(&line);

        first_line = false;

        if brace_depth <= 0 {
            break;
        }
    }

    true
}

fn count_open_braces(line: &str) -> i32 {
    line.chars().filter(|&c| c == '{').count() as i32
}

fn count_close_braces(line: &str) -> i32 {
    line.chars().filter(|&c| c == '}').count() as i32
}