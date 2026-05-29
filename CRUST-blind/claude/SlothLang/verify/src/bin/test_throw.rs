use SlothLang::throw;
use std::process::Command;

// throw::math_err and throw::op_err call process::exit, so we cannot run them
// directly inside the test process. Instead we spawn the `throw_helper`
// binary, which calls them with arguments specifying which function to run.
//
// The C version uses raise(SIGFPE) for math_err and raise(SIGILL) for op_err.
// The Rust translation matches that behavior by exiting with codes 128+8=136
// and 128+4=132, respectively (the standard shell exit codes for signals).

fn helper_path() -> std::path::PathBuf {
    // The throw_helper binary lives next to the current test executable.
    let mut p = std::env::current_exe().unwrap();
    // Strip the test-binary basename (which usually has a hash suffix); we
    // know the helper is at deps/../throw_helper(.exe).
    p.pop(); // remove test binary basename
    // The test binary is at target/debug/deps/test_throw-XXXX so we go up to
    // target/debug.
    if p.ends_with("deps") {
        p.pop();
    }
    let helper = p.join("throw_helper");
    if helper.exists() {
        helper
    } else {
        helper.with_extension("exe")
    }
}

fn run_helper(arg: &str) -> std::process::Output {
    Command::new(helper_path())
        .arg(arg)
        .output()
        .expect("failed to run throw_helper child")
}

#[test]
fn test_math_err_function_exists() {
    // Take a function pointer to ensure the signature compiles.
    let _: fn(&str) = throw::math_err;
}

#[test]
fn test_op_err_function_exists() {
    let _: fn(&str, u8) = throw::op_err;
}

#[test]
fn test_math_err_exit_code() {
    let out = run_helper("math_err");
    let code = out.status.code().unwrap_or(-1);
    assert_eq!(code, 136, "math_err should exit with code 136 (128+SIGFPE)");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("[ERROR] division by zero"), "stderr was: {}", stderr);
}

#[test]
fn test_op_err_exit_code() {
    let out = run_helper("op_err");
    let code = out.status.code().unwrap_or(-1);
    assert_eq!(code, 132, "op_err should exit with code 132 (128+SIGILL)");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("[ERROR] invalid operation code: 0x0a"),
        "stderr was: {}", stderr);
}

#[test]
fn test_math_err_empty_message_no_print() {
    let out = run_helper("math_err_empty");
    let code = out.status.code().unwrap_or(-1);
    assert_eq!(code, 136);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("[ERROR]"), "stderr was: {}", stderr);
}

#[test]
fn test_op_err_empty_type_no_print() {
    let out = run_helper("op_err_empty");
    let code = out.status.code().unwrap_or(-1);
    assert_eq!(code, 132);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("[ERROR]"), "stderr was: {}", stderr);
}

#[test]
fn test_op_err_format() {
    // Verify the code is formatted with two-digit lower-case hex and 0x prefix.
    let out = run_helper("op_err_99");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("0x99"), "stderr was: {}", stderr);
    assert!(stderr.contains("invalid input type code"), "stderr was: {}", stderr);
}

fn main() {}
