use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn c_driver() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../c_src/build/driver")
}

fn run_with_input(program: &Path, input_path: &Path) -> Output {
    let stdin = File::open(input_path).expect("open test input");
    Command::new(program)
        .stdin(Stdio::from(stdin))
        .output()
        .unwrap_or_else(|error| panic!("run {}: {error}", program.display()))
}

fn assert_matches_c(case_name: &str, input: &[u8]) {
    let input_path =
        std::env::temp_dir().join(format!("driver-differential-{}-{case_name}", std::process::id()));
    fs::write(&input_path, input).expect("write test input");

    let c_output = run_with_input(&c_driver(), &input_path);
    let rust_output = run_with_input(Path::new(env!("CARGO_BIN_EXE_driver")), &input_path);
    fs::remove_file(&input_path).expect("remove test input");

    assert_eq!(rust_output.stdout, c_output.stdout, "{case_name}: stdout");
    assert_eq!(rust_output.stderr, c_output.stderr, "{case_name}: stderr");
    assert_eq!(rust_output.status, c_output.status, "{case_name}: exit status");
}

#[test]
fn empty_input() {
    assert_matches_c("empty", b"");
}

#[test]
fn single_item() {
    assert_matches_c("single-item", b"x");
}

#[test]
fn multiline_input() {
    assert_matches_c("multiline", b"first\nsecond\n");
}

#[test]
fn large_input() {
    assert_matches_c("large", &vec![b'x'; 1024 * 1024]);
}
