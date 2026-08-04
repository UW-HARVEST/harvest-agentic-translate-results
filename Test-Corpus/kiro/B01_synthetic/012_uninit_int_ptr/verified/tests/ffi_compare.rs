use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_lib/libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/libdriver.so")
}

/// Capture stdout from a function called via .so by forking a child process.
/// We fork because println!/printf write to the process stdout.
fn capture_fn_output(lib_path: &std::path::Path, fn_name: &str) -> String {
    let lib_str = lib_path.to_str().unwrap();
    // Use a small helper binary via Command to load and call the function
    let output = Command::new("python3")
        .arg("-c")
        .arg(format!(
            r#"
import ctypes, sys
lib = ctypes.CDLL("{lib_str}")
fn = getattr(lib, "{fn_name}")
fn.restype = None
fn.argtypes = []
fn()
sys.stdout.flush()
"#
        ))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("failed to run python3");
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn capture_print_int_ptr_line(lib_path: &std::path::Path, value: i32) -> String {
    let lib_str = lib_path.to_str().unwrap();
    let output = Command::new("python3")
        .arg("-c")
        .arg(format!(
            r#"
import ctypes, sys
lib = ctypes.CDLL("{lib_str}")
fn = lib.printIntPtrLine
fn.restype = None
fn.argtypes = [ctypes.POINTER(ctypes.c_int)]
val = ctypes.c_int({value})
fn(ctypes.byref(val))
sys.stdout.flush()
"#
        ))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("failed to run python3");
    String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
fn test_print_int_ptr_line_matches() {
    let c_lib = c_lib_path();
    let rust_lib = rust_lib_path();
    for val in [0, 1, -1, 5, 42, i32::MAX, i32::MIN] {
        let c_out = capture_print_int_ptr_line(&c_lib, val);
        let r_out = capture_print_int_ptr_line(&rust_lib, val);
        assert_eq!(c_out, r_out, "printIntPtrLine mismatch for value {val}");
    }
}

#[test]
fn test_good_matches() {
    let c_out = capture_fn_output(&c_lib_path(), "good");
    let r_out = capture_fn_output(&rust_lib_path(), "good");
    assert_eq!(c_out, r_out, "good() output mismatch");
}

#[test]
fn test_symbol_parity() {
    // Verify all C-exported symbols exist in Rust .so
    let c_lib = c_lib_path();
    let rust_lib = rust_lib_path();
    unsafe {
        let c = Library::new(&c_lib).expect("load C lib");
        let r = Library::new(&rust_lib).expect("load Rust lib");
        for sym in ["printIntPtrLine", "bad", "good", "main"] {
            let _c_fn: Symbol<unsafe extern "C" fn()> =
                c.get(sym.as_bytes()).unwrap_or_else(|_| panic!("C missing {sym}"));
            let _r_fn: Symbol<unsafe extern "C" fn()> =
                r.get(sym.as_bytes()).unwrap_or_else(|_| panic!("Rust missing {sym}"));
        }
    }
}
