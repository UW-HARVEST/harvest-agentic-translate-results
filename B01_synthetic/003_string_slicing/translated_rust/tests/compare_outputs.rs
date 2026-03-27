use std::process::Command;

fn c_bin() -> String {
    std::env::current_dir()
        .unwrap()
        .join("c_src/build/driver")
        .to_string_lossy()
        .to_string()
}

fn rust_bin() -> String {
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    std::env::current_dir()
        .unwrap()
        .join(format!("target/{}/driver", profile))
        .to_string_lossy()
        .to_string()
}

fn run(bin: &str, args: &[&str]) -> (Vec<u8>, Vec<u8>, i32) {
    let out = Command::new(bin).args(args).output().expect("failed to run");
    let code = out.status.code().unwrap_or(-1);
    (out.stdout, out.stderr, code)
}

fn compare(args: &[&str]) {
    let (c_out, c_err, c_code) = run(&c_bin(), args);
    let (r_out, r_err, r_code) = run(&rust_bin(), args);
    assert_eq!(
        c_out, r_out,
        "stdout mismatch for args {:?}\nC:    {:?}\nRust: {:?}",
        args,
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out)
    );
    // Compare exit codes (normalize non-zero to 1 since Rust process::exit(1))
    let c_nz = c_code != 0;
    let r_nz = r_code != 0;
    assert_eq!(
        c_nz, r_nz,
        "exit code mismatch for args {:?}: C={} Rust={}",
        args, c_code, r_code
    );
}

#[test]
fn no_args() {
    compare(&[]);
}

#[test]
fn string_only() {
    compare(&["hello"]);
}

#[test]
fn string_with_start() {
    compare(&["hello", "2"]);
}

#[test]
fn string_with_start_stop() {
    compare(&["hello", "1", "4"]);
}

#[test]
fn full_range() {
    compare(&["abcdef", "0", "6"]);
}

#[test]
fn start_zero() {
    compare(&["world", "0", "3"]);
}

#[test]
fn single_char() {
    compare(&["test", "2", "3"]);
}

#[test]
fn start_equals_len_error() {
    // start > len should error
    compare(&["hi", "10"]);
}

#[test]
fn stop_greater_than_len_error() {
    compare(&["hi", "0", "10"]);
}

#[test]
fn stop_before_start_error() {
    compare(&["hello", "3", "1"]);
}

#[test]
fn too_many_args() {
    compare(&["a", "b", "c", "d"]);
}

#[test]
fn non_integer_start() {
    compare(&["hello", "abc"]);
}

#[test]
fn non_integer_stop() {
    // C bug: the end pointer from argv[2] parse is checked against argv[3],
    // so this won't actually trigger the "Third argument must be an integer" error
    compare(&["hello", "1", "abc"]);
}

#[test]
fn empty_string() {
    compare(&[""]);
}

#[test]
fn empty_string_with_zero_zero() {
    compare(&["", "0", "0"]);
}

#[test]
fn negative_start() {
    compare(&["hello", "-1"]);
}

#[test]
fn spaces_in_string() {
    compare(&["hello world", "3", "8"]);
}
