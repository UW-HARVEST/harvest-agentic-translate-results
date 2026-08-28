use std::fs::{remove_file, OpenOptions};
use std::io::{ErrorKind, Seek, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_INPUT_FILE: AtomicU64 = AtomicU64::new(0);

fn c_driver() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/driver")
}

fn run_driver(path: &Path, input: &[u8]) -> Output {
    let (input_path, mut input_file) = loop {
        let sequence = NEXT_INPUT_FILE.fetch_add(1, Ordering::Relaxed);
        let input_path = std::env::temp_dir().join(format!(
            "driver-differential-{}-{sequence}.stdin",
            std::process::id()
        ));
        match OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&input_path)
        {
            Ok(input_file) => break (input_path, input_file),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(error) => panic!("failed to create {}: {error}", input_path.display()),
        }
    };

    input_file
        .write_all(input)
        .unwrap_or_else(|error| panic!("failed to write {}: {error}", input_path.display()));
    input_file
        .rewind()
        .unwrap_or_else(|error| panic!("failed to rewind {}: {error}", input_path.display()));

    let result = Command::new(path)
        .stdin(Stdio::from(input_file))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    remove_file(&input_path)
        .unwrap_or_else(|error| panic!("failed to remove {}: {error}", input_path.display()));
    result.unwrap_or_else(|error| panic!("failed to run {}: {error}", path.display()))
}

fn assert_matches_c(input: &[u8]) {
    let c_output = run_driver(&c_driver(), input);
    let rust_output = run_driver(Path::new(env!("CARGO_BIN_EXE_driver")), input);

    assert_eq!(
        rust_output.status.code(),
        c_output.status.code(),
        "exit status differs"
    );
    assert_eq!(rust_output.stdout, c_output.stdout, "stdout differs");
    assert_eq!(rust_output.stderr, c_output.stderr, "stderr differs");
}

#[test]
fn empty_input() {
    assert_matches_c(b"");
}

#[test]
fn single_item_input_is_ignored() {
    assert_matches_c(b"1");
}

#[test]
fn multiline_input_is_ignored() {
    assert_matches_c(b"first\nsecond\nthird\n");
}

#[test]
fn max_data_length_sized_input_is_ignored() {
    assert_matches_c(&vec![b'x'; 256]);
}

#[test]
fn malformed_input_is_ignored_while_builtin_error_paths_run() {
    assert_matches_c(b"\0not-a-number\n18446744073709551616\n");
}
