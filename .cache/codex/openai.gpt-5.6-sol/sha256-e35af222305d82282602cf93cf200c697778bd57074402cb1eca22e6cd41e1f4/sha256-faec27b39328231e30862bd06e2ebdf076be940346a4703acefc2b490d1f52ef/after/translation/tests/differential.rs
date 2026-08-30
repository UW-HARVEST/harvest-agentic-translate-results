use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

static INPUT_ID: AtomicU64 = AtomicU64::new(0);

fn c_driver() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/driver")
}

fn rust_driver() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn run(binary: &Path, args: &[&str], input: &[u8]) -> Output {
    let input_id = INPUT_ID.fetch_add(1, Ordering::Relaxed);
    let input_path = std::env::temp_dir().join(format!(
        "driver-differential-input-{}-{input_id}",
        std::process::id()
    ));

    fs::write(&input_path, input).expect("failed to create subprocess input");
    let input_file = File::open(&input_path).expect("failed to open subprocess input");
    let output = Command::new(binary)
        .args(args)
        .stdin(Stdio::from(input_file))
        .output()
        .unwrap_or_else(|error| panic!("failed to run {}: {error}", binary.display()));
    fs::remove_file(&input_path).expect("failed to remove subprocess input");

    output
}

fn assert_programs_match(case: &str, args: &[&str], input: &[u8]) {
    let c_output = run(&c_driver(), args, input);
    let rust_output = run(&rust_driver(), args, input);

    assert_eq!(
        c_output.stdout, rust_output.stdout,
        "{case}: stdout differs"
    );
    assert_eq!(
        c_output.stderr, rust_output.stderr,
        "{case}: stderr differs"
    );
    assert_eq!(
        c_output.status, rust_output.status,
        "{case}: exit status differs"
    );
}

#[test]
fn empty_input() {
    assert_programs_match("empty input", &[], b"");
}

#[test]
fn single_item_input() {
    assert_programs_match("single item", &[], b"x");
}

#[test]
fn multiline_input() {
    assert_programs_match("multiline input", &[], b"first\nsecond\nthird\n");
}

#[test]
fn large_input() {
    // The C program never reads stdin, so it has no finite input-size maximum.
    let input = vec![b'x'; 1024 * 1024];
    assert_programs_match("1 MiB input", &[], &input);
}

#[test]
fn arbitrary_binary_input() {
    assert_programs_match("arbitrary binary input", &[], b"\0\xff\x80\n");
}

#[test]
fn command_line_arguments() {
    assert_programs_match(
        "command-line arguments",
        &["single-item", "", "--unknown-option"],
        b"",
    );
}
