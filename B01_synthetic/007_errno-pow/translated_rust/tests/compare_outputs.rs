use std::process::Command;

const C_BIN: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/driver");

fn rust_bin() -> String {
    let dir = env!("CARGO_MANIFEST_DIR");
    // Use cargo build output directory
    format!("{}/target/debug/driver", dir)
}

fn run(bin: &str, args: &[&str]) -> (String, String, i32) {
    let out = Command::new(bin).args(args).output().expect("failed to run");
    (
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
        out.status.code().unwrap_or(-1),
    )
}

fn compare(args: &[&str]) {
    let (c_out, _c_err, c_rc) = run(C_BIN, args);
    let (r_out, _r_err, r_rc) = run(&rust_bin(), args);
    assert_eq!(c_rc, r_rc, "exit code mismatch for args {:?}", args);
    assert_eq!(c_out, r_out, "stdout mismatch for args {:?}", args);
}

#[test]
fn test_normal_pow() {
    compare(&["2", "10"]);
    compare(&["2.5", "3.5"]);
    compare(&["100", "0.5"]);
    compare(&["0.5", "0.5"]);
    compare(&["10", "-1"]);
    compare(&["2", "-3"]);
    compare(&["1.5", "2.5"]);
}

#[test]
fn test_edge_cases() {
    compare(&["0", "0"]);
    compare(&["0", "1"]);
    compare(&["1", "0"]);
    compare(&["-1", "2"]);
    compare(&["-1", "3"]);
    compare(&["10", "0"]);
}

#[test]
fn test_error_cases() {
    // These should all exit with rc=1
    compare(&["1e308", "2"]);
    compare(&["-2", "0.5"]);
    compare(&["0", "-1"]);
    compare(&["abc", "2"]);
    compare(&["2", "abc"]);
}

#[test]
fn test_usage_error() {
    // Wrong number of args - rc should match (stderr differs only in argv[0])
    let (c_out, _, c_rc) = run(C_BIN, &["2"]);
    let (r_out, _, r_rc) = run(&rust_bin(), &["2"]);
    assert_eq!(c_rc, r_rc);
    assert_eq!(c_out, r_out); // both empty stdout

    let (c_out, _, c_rc) = run(C_BIN, &[]);
    let (r_out, _, r_rc) = run(&rust_bin(), &[]);
    assert_eq!(c_rc, r_rc);
    assert_eq!(c_out, r_out);
}
