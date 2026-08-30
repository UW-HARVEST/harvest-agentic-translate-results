use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

static INPUT_ID: AtomicU64 = AtomicU64::new(0);

fn c_driver() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/driver")
}

fn run_driver(program: &Path, stdin: &[u8], args: &[&str]) -> Output {
    let input_id = INPUT_ID.fetch_add(1, Ordering::Relaxed);
    let input_path = std::env::temp_dir().join(format!(
        "driver-differential-{}-{input_id}.stdin",
        std::process::id()
    ));

    let mut input_file = File::create(&input_path).expect("create temporary stdin file");
    input_file
        .write_all(stdin)
        .expect("write temporary stdin file");
    drop(input_file);

    let input_file = File::open(&input_path).expect("open temporary stdin file");
    let output = Command::new(program)
        .args(args)
        .stdin(Stdio::from(input_file))
        .output()
        .unwrap_or_else(|error| panic!("run {}: {error}", program.display()));

    fs::remove_file(&input_path).expect("remove temporary stdin file");
    output
}

fn assert_matches_c(case: &str, stdin: &[u8], args: &[&str]) {
    let c_program = c_driver();
    assert!(
        c_program.is_file(),
        "C driver is missing; build it at {}",
        c_program.display()
    );

    let c_output = run_driver(&c_program, stdin, args);
    let rust_output = run_driver(Path::new(env!("CARGO_BIN_EXE_driver")), stdin, args);

    assert_eq!(
        rust_output.stdout, c_output.stdout,
        "{case}: stdout differs"
    );
    assert_eq!(
        rust_output.stderr, c_output.stderr,
        "{case}: stderr differs"
    );
    assert_eq!(
        rust_output.status, c_output.status,
        "{case}: exit status differs"
    );
}

#[test]
fn matches_c_for_empty_input() {
    assert_matches_c("empty stdin", b"", &[]);
}

#[test]
fn matches_c_for_single_item() {
    assert_matches_c("single item", b"item\n", &[]);
}

#[test]
fn matches_c_for_multiline_input() {
    assert_matches_c("multiline stdin", b"first\nsecond third\n", &[]);
}

#[test]
fn matches_c_for_binary_input() {
    assert_matches_c("binary stdin", b"\0\xff\x80input\n", &[]);
}

#[test]
fn matches_c_for_large_input() {
    let input = vec![b'x'; 64 * 1024];
    assert_matches_c("large stdin", &input, &[]);
}

#[test]
fn matches_c_when_arguments_are_present() {
    assert_matches_c(
        "command-line arguments",
        b"",
        &["one", "two words", "--flag"],
    );
}
