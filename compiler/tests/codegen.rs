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

#[test]
fn compiles_branches_loops_and_short_circuiting() {
    let status = compile_and_run(
        r#"
        fn main() -> i32 {
            let mut n: i32 = 0;
            let mut i: i32 = 6;
            while i > 0 {
                n += i--;
            }

            if false && fail() { return 1; };
            if true || fail() { return twice(n); };
            return 0;
        }

        fn twice(n: i32) -> i32 {
            if n == 0 { return 0; } else {
                if n > 20 { n + n } else { n * 2 }
            }
        }

        fn fail() -> bool {
            let zero: i32 = 0;
            1 / zero == 0
        }
        "#,
    );

    assert_eq!(status.code(), Some(42));
}

#[test]
fn compiles_struct_construction_and_shared_fields() {
    let status = compile_and_run(
        r#"
        struct Point {
            x: i32,
            y: i32,
        }

        struct Pair {
            item: Point,
        }

        fn make_pair(x: i32) -> Pair {
            let mut n = x;
            Pair { item: Point { y: n++, x: ++n, }, }
        }

        fn main() -> i32 {
            let mut p = make_pair(18);
            let alias = p;
            p.item.x += 4;
            if alias.item.x == 24 {
                make_pair(18).item.y + 24
            } else {
                0
            }
        }
        "#,
    );

    assert_eq!(status.code(), Some(42));
}

#[test]
fn compiles_chains_and_evaluates_index_updates_once() {
    let status = compile_and_run(
        r#"
        struct Item { value: i32, }
        struct Bag { items: [Item], }

        fn make_bag() -> Bag {
            Bag { items: [Item { value: 13, }, Item { value: 25, }], }
        }

        fn main() -> i32 {
            let mut bag = make_bag();
            let alias = bag.items;
            let mut i: u8 = 0;
            bag.items[i++].value += 4;

            if i == 1 { alias[0].value + make_bag().items[1].value } else { 0 }
        }
        "#,
    );

    assert_eq!(status.code(), Some(42));
}

#[test]
fn traps_out_of_bounds_array_reads() {
    for source in [
        "fn main() -> i32 { let xs: [i32] = []; xs[0] }",
        "fn main() -> i32 { let xs: [i32] = [42]; xs[-1] }",
    ] {
        assert_eq!(compile_and_run(source).code(), None);
    }
}

#[test]
fn compiles_enum_values_and_equality() {
    let status = compile_and_run(
        r#"
        enum Mode { Off, On, }
        const DEFAULT: Mode = Mode::Off;

        fn enable(mode: Mode) -> Mode {
            if mode == Mode::On { mode } else { Mode::On }
        }

        fn main() -> i32 {
            if DEFAULT != Mode::On && enable(DEFAULT) == Mode::On { 42 } else { 0 }
        }
        "#,
    );

    assert_eq!(status.code(), Some(42));
}

#[test]
fn compiles_matches_with_chained_expressions_and_mutation() {
    let status = compile_and_run(
        r#"
        enum Choice { First, Second, }
        struct Item { value: i32, }
        struct Bag { items: [Item], }

        fn make_bag() -> Bag {
            Bag { items: [Item { value: 17, }, Item { value: 25, },], }
        }

        fn choose(index: u8) -> Choice {
            if index == 0 { Choice::Second } else { Choice::First }
        }

        fn main() -> i32 {
            let mut index: u8 = 0;
            let mut total = make_bag().items[0].value;
            let picked = match choose(index++) {
                Choice::First => { return 0; },
                Choice::Second => make_bag().items[index].value,
            };
            total += picked;

            if index != 1 { return 0; };

            match total {
                0 => 0,
                value => value,
            }
        }
        "#,
    );

    assert_eq!(status.code(), Some(42));
}

#[test]
fn compiles_boolean_and_string_patterns() {
    let status = compile_and_run(
        r#"
        fn main() -> i32 {
            let text = match true {
                true => "café",
                false => "",
            };

            match text {
                "cafe" => 1,
                "café" => 42,
                _ => 0,
            }
        }
        "#,
    );

    assert_eq!(status.code(), Some(42));
}
