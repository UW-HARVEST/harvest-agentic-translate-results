use libloading::{Library, Symbol};
use std::process::Command;

/// Path to the C shared library.
const C_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver.so");

/// Returns the path to the Rust shared library built by cargo.
fn rust_lib_path() -> String {
    let dir = env!("CARGO_MANIFEST_DIR");
    format!("{}/target/debug/libdriver.so", dir)
}

/// Helper: run a function that prints to stdout in a subprocess so we can capture output.
/// `lib_path` is the .so to load, `func` is the symbol name, `stdin_data` is fed to stdin.
fn capture_void_fn(lib_path: &str, func: &str, stdin_data: &str) -> String {
    // We use a small helper binary approach: call ourselves with env vars
    // to indicate which lib/func to call.
    // Instead, use a simpler approach: write a small script that uses python to dlopen.
    let script = format!(
        r#"
import ctypes, sys, os
lib = ctypes.CDLL("{lib_path}")
func = getattr(lib, "{func}")
func.restype = None
func.argtypes = []
func()
"#,
        lib_path = lib_path,
        func = func,
    );
    let output = Command::new("python3")
        .arg("-c")
        .arg(&script)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            if !stdin_data.is_empty() {
                if let Some(ref mut si) = child.stdin {
                    let _ = si.write_all(stdin_data.as_bytes());
                }
            }
            drop(child.stdin.take());
            child.wait_with_output()
        })
        .expect("failed to run python3");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Helper for printLine: needs a const char* argument.
fn capture_print_line(lib_path: &str, arg: &str) -> String {
    let script = format!(
        r#"
import ctypes
lib = ctypes.CDLL("{lib_path}")
func = lib.printLine
func.restype = None
func.argtypes = [ctypes.c_char_p]
func({arg})
"#,
        lib_path = lib_path,
        arg = if arg == "NULL" {
            "None".to_string()
        } else {
            format!("b\"{}\"", arg.replace('\\', "\\\\").replace('"', "\\\""))
        },
    );
    let output = Command::new("python3")
        .arg("-c")
        .arg(&script)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .expect("failed to run python3");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Helper for printIntLine: needs an int argument.
fn capture_print_int_line(lib_path: &str, val: i32) -> String {
    let script = format!(
        r#"
import ctypes
lib = ctypes.CDLL("{lib_path}")
func = lib.printIntLine
func.restype = None
func.argtypes = [ctypes.c_int]
func({val})
"#,
        lib_path = lib_path,
        val = val,
    );
    let output = Command::new("python3")
        .arg("-c")
        .arg(&script)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .expect("failed to run python3");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

// ---- Tests ----

#[test]
fn test_print_line_basic() {
    let c_lib = C_LIB;
    let r_lib = rust_lib_path();
    for input in &["hello", "world", "", "line with spaces", "123"] {
        let c_out = capture_print_line(c_lib, input);
        let r_out = capture_print_line(&r_lib, input);
        assert_eq!(c_out, r_out, "printLine mismatch for input {:?}", input);
    }
}

#[test]
fn test_print_line_null() {
    let c_lib = C_LIB;
    let r_lib = rust_lib_path();
    let c_out = capture_print_line(c_lib, "NULL");
    let r_out = capture_print_line(&r_lib, "NULL");
    assert_eq!(c_out, r_out, "printLine(NULL) mismatch");
}

#[test]
fn test_print_int_line() {
    let c_lib = C_LIB;
    let r_lib = rust_lib_path();
    for val in &[0, 1, -1, 42, 100, -2147483648, 2147483647, 50] {
        let c_out = capture_print_int_line(c_lib, *val);
        let r_out = capture_print_int_line(&r_lib, *val);
        assert_eq!(c_out, r_out, "printIntLine mismatch for {}", val);
    }
}

#[test]
fn test_good_with_input() {
    let c_lib = C_LIB;
    let r_lib = rust_lib_path();
    // good() calls goodG2B() (no stdin) then goodB2G() (reads stdin).
    // goodG2B prints (int)(100.0/2.0) = 50
    // goodB2G reads a float, if |data| > 0.000001 prints (int)(100.0/data), else prints message
    for input in &["5.0\n", "0.0\n", "2.0\n", "-3.0\n", "0.0000001\n"] {
        let c_out = capture_void_fn(c_lib, "good", input);
        let r_out = capture_void_fn(&r_lib, "good", input);
        assert_eq!(c_out, r_out, "good() mismatch for stdin={:?}", input);
    }
}

#[test]
fn test_good_eof() {
    // goodB2G with EOF on stdin: fgets returns NULL -> prints "fgets() failed."
    // then data=0.0 so prints "This would result in a divide by zero"
    let c_lib = C_LIB;
    let r_lib = rust_lib_path();
    let c_out = capture_void_fn(c_lib, "good", "");
    let r_out = capture_void_fn(&r_lib, "good", "");
    assert_eq!(c_out, r_out, "good() mismatch for EOF stdin");
}

#[test]
fn test_bad_with_input() {
    let c_lib = C_LIB;
    let r_lib = rust_lib_path();
    for input in &["5.0\n", "2.0\n", "-4.0\n", "10.0\n"] {
        let c_out = capture_void_fn(c_lib, "bad", input);
        let r_out = capture_void_fn(&r_lib, "bad", input);
        assert_eq!(c_out, r_out, "bad() mismatch for stdin={:?}", input);
    }
}

#[test]
fn test_bad_eof() {
    let c_lib = C_LIB;
    let r_lib = rust_lib_path();
    // bad() with EOF: fgets fails, data=0.0, divides 100.0/0.0 -> UB in C (inf -> INT_MIN on x86)
    let c_out = capture_void_fn(c_lib, "bad", "");
    let r_out = capture_void_fn(&r_lib, "bad", "");
    assert_eq!(c_out, r_out, "bad() mismatch for EOF stdin");
}

#[test]
fn test_symbol_parity() {
    // Verify all C symbols exist in Rust .so
    let c_lib = C_LIB;
    let r_lib = rust_lib_path();
    let expected = ["printLine", "printIntLine", "bad", "good", "main"];
    unsafe {
        let c = Library::new(c_lib).expect("load C lib");
        let r = Library::new(&r_lib).expect("load Rust lib");
        for sym in &expected {
            let _c_sym: Symbol<unsafe extern "C" fn()> =
                c.get(sym.as_bytes()).unwrap_or_else(|_| panic!("C lib missing {}", sym));
            let _r_sym: Symbol<unsafe extern "C" fn()> =
                r.get(sym.as_bytes()).unwrap_or_else(|_| panic!("Rust lib missing {}", sym));
        }
    }
}
