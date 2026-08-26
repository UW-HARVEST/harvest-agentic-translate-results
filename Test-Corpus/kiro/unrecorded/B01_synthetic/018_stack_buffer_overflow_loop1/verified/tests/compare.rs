use libloading::{Library, Symbol};
use std::process::Command;

fn c_lib_path() -> String {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    format!("{}/c_src/libdriver_c.so", manifest)
}

fn rust_lib_path() -> String {
    // Find the Rust cdylib in target/debug
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    format!("{}/target/debug/libdriver.so", manifest)
}

/// Capture stdout of a void() function by forking a helper process
fn capture_void_fn(lib_path: &str, fn_name: &str) -> String {
    let out = Command::new("python3")
        .arg("-c")
        .arg(format!(
            r#"
import ctypes, sys
lib = ctypes.CDLL("{lib_path}")
fn = getattr(lib, "{fn_name}")
fn.restype = None
fn()
"#
        ))
        .output()
        .expect("failed to run python3");
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn capture_print_int_line(lib_path: &str, val: i32) -> String {
    let out = Command::new("python3")
        .arg("-c")
        .arg(format!(
            r#"
import ctypes
lib = ctypes.CDLL("{lib_path}")
fn = lib.printIntLine
fn.restype = None
fn.argtypes = [ctypes.c_int]
fn({val})
"#
        ))
        .output()
        .expect("failed to run python3");
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn capture_print_line(lib_path: &str, s: Option<&str>) -> String {
    let arg = match s {
        Some(v) => format!("ctypes.c_char_p(b\"{}\")", v),
        None => "ctypes.c_char_p(None)".to_string(),
    };
    let out = Command::new("python3")
        .arg("-c")
        .arg(format!(
            r#"
import ctypes
lib = ctypes.CDLL("{lib_path}")
fn = lib.printLine
fn.restype = None
fn.argtypes = [ctypes.c_char_p]
fn({arg})
"#
        ))
        .output()
        .expect("failed to run python3");
    String::from_utf8_lossy(&out.stdout).to_string()
}

#[test]
fn test_print_int_line() {
    let c = c_lib_path();
    let r = rust_lib_path();
    for val in [0, 1, -1, 42, i32::MAX, i32::MIN] {
        let c_out = capture_print_int_line(&c, val);
        let r_out = capture_print_int_line(&r, val);
        assert_eq!(c_out, r_out, "printIntLine({val}) mismatch");
    }
}

#[test]
fn test_print_line() {
    let c = c_lib_path();
    let r = rust_lib_path();
    // Non-null string
    for s in ["hello", "", "test 123"] {
        let c_out = capture_print_line(&c, Some(s));
        let r_out = capture_print_line(&r, Some(s));
        assert_eq!(c_out, r_out, "printLine(\"{s}\") mismatch");
    }
    // NULL pointer
    let c_out = capture_print_line(&c, None);
    let r_out = capture_print_line(&r, None);
    assert_eq!(c_out, r_out, "printLine(NULL) mismatch");
}

#[test]
fn test_bad() {
    let c_out = capture_void_fn(&c_lib_path(), "bad");
    let r_out = capture_void_fn(&rust_lib_path(), "bad");
    assert_eq!(c_out, r_out, "bad() output mismatch");
}

#[test]
fn test_good() {
    let c_out = capture_void_fn(&c_lib_path(), "good");
    let r_out = capture_void_fn(&rust_lib_path(), "good");
    assert_eq!(c_out, r_out, "good() output mismatch");
}

#[test]
fn test_exports_match() {
    let c = c_lib_path();
    let r = rust_lib_path();
    unsafe {
        let c_lib = Library::new(&c).expect("load C lib");
        let r_lib = Library::new(&r).expect("load Rust lib");
        for sym in ["printLine", "printIntLine", "bad", "good", "main"] {
            let _c: Symbol<unsafe extern "C" fn()> =
                c_lib.get(sym.as_bytes()).unwrap_or_else(|_| panic!("C missing {sym}"));
            let _r: Symbol<unsafe extern "C" fn()> =
                r_lib.get(sym.as_bytes()).unwrap_or_else(|_| panic!("Rust missing {sym}"));
        }
    }
}
