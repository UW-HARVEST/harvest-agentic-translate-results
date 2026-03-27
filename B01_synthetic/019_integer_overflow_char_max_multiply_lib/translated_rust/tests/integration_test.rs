use std::ffi::{c_char, c_int, CString};
use std::process::Command;

fn project_dir() -> String {
    std::env::current_dir().unwrap().to_str().unwrap().to_string()
}

fn c_lib_path() -> String {
    format!("{}/c_src/build/libdriver.so", project_dir())
}

fn rust_lib_path() -> String {
    // Find the built cdylib - check deps dir first, then direct
    let target_dir = format!("{}/target/debug", project_dir());
    let deps_path = format!("{}/deps/libdriver.so", target_dir);
    let direct_path = format!("{}/libdriver.so", target_dir);
    if std::path::Path::new(&direct_path).exists() {
        direct_path
    } else {
        deps_path
    }
}

/// Run a small C program that loads the given .so and calls a function, capturing stdout
fn call_via_helper(lib_path: &str, func: &str, args: &[&str]) -> String {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_test_helper"));
    cmd.arg(lib_path).arg(func);
    for a in args {
        cmd.arg(a);
    }
    cmd.env("LD_LIBRARY_PATH", format!("{}/c_src/build", project_dir()));
    let output = cmd.output().expect("failed to run test_helper");
    String::from_utf8(output.stdout).unwrap()
}

// ---- Low-level: printHexCharLine ----

#[test]
fn test_print_hex_char_line_positive() {
    let c_out = call_via_helper(&c_lib_path(), "printHexCharLine", &["4"]);
    let r_out = call_via_helper(&rust_lib_path(), "printHexCharLine", &["4"]);
    assert_eq!(c_out, r_out, "printHexCharLine(4)");
}

#[test]
fn test_print_hex_char_line_negative() {
    let c_out = call_via_helper(&c_lib_path(), "printHexCharLine", &["-2"]);
    let r_out = call_via_helper(&rust_lib_path(), "printHexCharLine", &["-2"]);
    assert_eq!(c_out, r_out, "printHexCharLine(-2)");
}

#[test]
fn test_print_hex_char_line_zero() {
    let c_out = call_via_helper(&c_lib_path(), "printHexCharLine", &["0"]);
    let r_out = call_via_helper(&rust_lib_path(), "printHexCharLine", &["0"]);
    assert_eq!(c_out, r_out, "printHexCharLine(0)");
}

// ---- Low-level: printLine ----

#[test]
fn test_print_line() {
    let c_out = call_via_helper(&c_lib_path(), "printLine", &["hello test"]);
    let r_out = call_via_helper(&rust_lib_path(), "printLine", &["hello test"]);
    assert_eq!(c_out, r_out, "printLine");
}

#[test]
fn test_print_line_null() {
    let c_out = call_via_helper(&c_lib_path(), "printLine", &["__NULL__"]);
    let r_out = call_via_helper(&rust_lib_path(), "printLine", &["__NULL__"]);
    assert_eq!(c_out, r_out, "printLine(NULL)");
}

// ---- Mid-level: bad, good ----

#[test]
fn test_bad() {
    let c_out = call_via_helper(&c_lib_path(), "bad", &[]);
    let r_out = call_via_helper(&rust_lib_path(), "bad", &[]);
    assert_eq!(c_out, r_out, "bad()");
}

#[test]
fn test_good() {
    let c_out = call_via_helper(&c_lib_path(), "good", &[]);
    let r_out = call_via_helper(&rust_lib_path(), "good", &[]);
    assert_eq!(c_out, r_out, "good()");
}

// ---- Top-level: driver ----

#[test]
fn test_driver_bad_path() {
    let c_out = call_via_helper(&c_lib_path(), "driver", &["0"]);
    let r_out = call_via_helper(&rust_lib_path(), "driver", &["0"]);
    assert_eq!(c_out, r_out, "driver(0)");
}

#[test]
fn test_driver_good_path() {
    let c_out = call_via_helper(&c_lib_path(), "driver", &["1"]);
    let r_out = call_via_helper(&rust_lib_path(), "driver", &["1"]);
    assert_eq!(c_out, r_out, "driver(1)");
}
