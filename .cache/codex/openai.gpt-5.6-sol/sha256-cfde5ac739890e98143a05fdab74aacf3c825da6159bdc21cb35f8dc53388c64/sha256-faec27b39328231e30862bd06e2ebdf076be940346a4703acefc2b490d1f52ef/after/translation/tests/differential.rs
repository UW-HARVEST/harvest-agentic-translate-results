use std::fs::{self, File};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_FILE_ID: AtomicU64 = AtomicU64::new(0);

unsafe extern "C" {
    fn signal(signal: i32, handler: usize) -> usize;
}

fn c_driver() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../c_src/build/driver")
        .canonicalize()
        .expect("C driver is missing; build c_src before running cargo test")
}

fn run(program: &Path, args: &[&str]) -> Output {
    Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .unwrap_or_else(|error| panic!("failed to run {}: {error}", program.display()))
}

fn run_with_file_limit(program: &Path, args: &[&str], label: &str) -> Output {
    let id = TEMP_FILE_ID.fetch_add(1, Ordering::Relaxed);
    let output_path =
        std::env::temp_dir().join(format!("nineality-{label}-{}-{id}.out", std::process::id()));
    let output_file = File::create(&output_path).expect("failed to create limited stdout file");

    let mut command = Command::new("sh");
    command
        .args(["-c", "ulimit -f 8; exec \"$@\"", "sh"])
        .arg(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::from(output_file))
        .stderr(Stdio::piped());

    // Cargo may ignore SIGXFSZ; reset it in the child before applying RLIMIT_FSIZE.
    unsafe {
        command.pre_exec(|| {
            signal(25, 0);
            Ok(())
        });
    }

    let mut output = command
        .output()
        .unwrap_or_else(|error| panic!("failed to run {}: {error}", program.display()));

    output.stdout = fs::read(&output_path).expect("failed to read limited stdout file");
    fs::remove_file(output_path).expect("failed to remove limited stdout file");
    output
}

fn assert_same(case: &str, args: &[&str]) {
    let c = run(&c_driver(), args);
    let rust = run(Path::new(env!("CARGO_BIN_EXE_driver")), args);

    assert_eq!(
        c.stdout, rust.stdout,
        "{case}: stdout differs\nC: {:?}\nRust: {:?}",
        c.stdout, rust.stdout
    );
    assert_eq!(
        c.stderr, rust.stderr,
        "{case}: stderr differs\nC: {:?}\nRust: {:?}",
        c.stderr, rust.stderr
    );
    assert_eq!(
        c.status, rust.status,
        "{case}: exit status differs (C: {}, Rust: {})",
        c.status, rust.status
    );
}

fn assert_same_with_file_limit(case: &str, args: &[&str]) {
    let c = run_with_file_limit(&c_driver(), args, "c");
    let rust = run_with_file_limit(Path::new(env!("CARGO_BIN_EXE_driver")), args, "rust");

    assert_eq!(c.stdout.len(), 4096, "{case}: file limit was not reached");
    assert_eq!(
        c.stdout, rust.stdout,
        "{case}: stdout differs\nC: {:?}\nRust: {:?}",
        c.stdout, rust.stdout
    );
    assert_eq!(
        c.stderr, rust.stderr,
        "{case}: stderr differs\nC: {:?}\nRust: {:?}",
        c.stderr, rust.stderr
    );
    assert_eq!(
        c.status, rust.status,
        "{case}: exit status differs (C: {}, Rust: {})",
        c.status, rust.status
    );
}

#[test]
fn rejects_missing_argument() {
    assert_same("missing argument", &[]);
}

#[test]
fn rejects_extra_argument() {
    assert_same("extra argument", &["9", "10"]);
}

#[test]
fn rejects_extra_arguments_before_parsing() {
    assert_same(
        "argument count is validated before parsing",
        &["not-a-number", "extra"],
    );
}

#[test]
fn rejects_empty_argument() {
    assert_same("empty argument", &[""]);
}

#[test]
fn rejects_non_numeric_argument() {
    assert_same("non-numeric argument", &["not-a-number"]);
}

#[test]
fn rejects_whitespace_only_argument() {
    assert_same("whitespace-only argument", &[" \t\n"]);
}

#[test]
fn stops_after_single_item() {
    assert_same("single item", &["9"]);
}

#[test]
fn counts_until_a_value_ending_in_nine() {
    assert_same("ordinary count", &["7"]);
}

#[test]
fn accepts_a_numeric_prefix() {
    assert_same("numeric prefix", &["8trailing-text"]);
}

#[test]
fn accepts_leading_whitespace_sign_and_zeroes() {
    assert_same("strtol syntax", &[" \t+008suffix"]);
}

#[test]
fn negative_nine_does_not_satisfy_the_remainder_check() {
    assert_same("negative remainder", &["-9"]);
}

#[test]
fn handles_the_largest_int_that_stops_before_overflow() {
    assert_same("largest terminating int", &["2147483639"]);
}

#[test]
fn wraps_after_the_maximum_int() {
    assert_same_with_file_limit("maximum int", &["2147483647"]);
}

#[test]
fn narrows_long_to_int() {
    assert_same("long-to-int narrowing", &["4294967295"]);
}

#[test]
fn saturates_positive_strtol_overflow() {
    assert_same("positive strtol overflow", &["9223372036854775808"]);
}

#[test]
fn saturates_negative_strtol_overflow() {
    assert_same("negative strtol overflow", &["-9223372036854775809"]);
}
