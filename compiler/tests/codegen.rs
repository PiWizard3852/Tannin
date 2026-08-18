use std::fs;
use std::path::PathBuf;
use std::process::{Command, ExitStatus};
use std::sync::atomic::{AtomicUsize, Ordering};

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);

        let path = std::env::temp_dir().join(format!(
            "tannin-codegen-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
        ));
        fs::create_dir(&path).expect("could not create test directory");

        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn compile_and_run(source: &str) -> ExitStatus {
    let scratch = Scratch::new();
    let input = scratch.0.join("test.tn");
    let ir = scratch.0.join("test.ll");
    let executable = scratch.0.join("test");
    fs::write(&input, source).expect("could not write Tannin source");

    let compiler = Command::new(env!("CARGO_BIN_EXE_compiler"))
        .arg(&input)
        .output()
        .expect("could not run compiler");
    assert!(compiler.status.success(), "{}", String::from_utf8_lossy(&compiler.stderr));
    fs::write(&ir, compiler.stdout).expect("could not write LLVM IR");

    let clang = std::env::var_os("CLANG").unwrap_or_else(|| "clang".into());
    let linker = Command::new(clang)
        .args(["-Wno-override-module", "-x", "ir", "-o"])
        .arg(&executable)
        .arg(&ir)
        .output()
        .expect("could not run clang");
    assert!(linker.status.success(), "{}", String::from_utf8_lossy(&linker.stderr));

    Command::new(&executable).status().expect("could not run generated executable")
}

#[test]
fn compiles_scalar_functions_and_updates() {
    let status = compile_and_run(
        r#"
        const BASE: i32 = 10;

        fn main() -> i32 {
            check("café", 1.5, 2.0);
            let mut n = twice(BASE);
            n <<= 1;
            let before = n++;
            let after = ++n;
            after + before - 40
        }

        fn twice(n: i32) -> i32 {
            return n * 2;
        }

        fn check(s: string, a: f32, b: f32) {
            (s == "café") == (a < b);
            return;
        }
        "#,
    );

    assert_eq!(status.code(), Some(42));
}
