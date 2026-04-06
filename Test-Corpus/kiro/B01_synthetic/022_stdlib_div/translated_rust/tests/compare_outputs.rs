use std::process::Command;

fn run_c_binary(input: &str) -> String {
    let c_bin = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/driver");
    let out = Command::new(c_bin)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
            child.wait_with_output()
        })
        .expect("failed to run C binary");
    String::from_utf8(out.stdout).unwrap()
}

fn run_rust_binary(input: &str) -> String {
    let rust_bin = env!("CARGO_BIN_EXE_driver");
    let out = Command::new(rust_bin)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
            child.wait_with_output()
        })
        .expect("failed to run Rust binary");
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn test_positive_division() {
    let input = "10 3\n";
    assert_eq!(run_c_binary(input), run_rust_binary(input));
}

#[test]
fn test_exact_division() {
    let input = "12 4\n";
    assert_eq!(run_c_binary(input), run_rust_binary(input));
}

#[test]
fn test_negative_numerator() {
    let input = "-10 3\n";
    assert_eq!(run_c_binary(input), run_rust_binary(input));
}

#[test]
fn test_negative_denominator() {
    let input = "10 -3\n";
    assert_eq!(run_c_binary(input), run_rust_binary(input));
}

#[test]
fn test_both_negative() {
    let input = "-10 -3\n";
    assert_eq!(run_c_binary(input), run_rust_binary(input));
}

#[test]
fn test_zero_numerator() {
    let input = "0 5\n";
    assert_eq!(run_c_binary(input), run_rust_binary(input));
}

#[test]
fn test_one_one() {
    let input = "1 1\n";
    assert_eq!(run_c_binary(input), run_rust_binary(input));
}

#[test]
fn test_large_values() {
    let input = "2147483647 2\n";
    assert_eq!(run_c_binary(input), run_rust_binary(input));
}
