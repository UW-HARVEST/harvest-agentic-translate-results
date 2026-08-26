use std::process::Command;

const C_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver.so");

fn rust_lib_path() -> String {
    // Find the Rust .so in target/debug
    let manifest = env!("CARGO_MANIFEST_DIR");
    format!("{}/target/debug/libdriver.so", manifest)
}

/// Capture stdout from a function call by running a small helper program
fn capture_fn_output(lib_path: &str, fn_name: &str, args: &str) -> String {
    // Use a small python script to dlopen and call the function
    let script = format!(
        r#"
import ctypes, sys
lib = ctypes.CDLL("{lib_path}")
fn = getattr(lib, "{fn_name}")
{args}
"#,
    );
    let out = Command::new("python3")
        .arg("-c")
        .arg(&script)
        .output()
        .expect("failed to run python3");
    String::from_utf8_lossy(&out.stdout).to_string()
}

#[test]
fn test_print_hex_char_line() {
    // Test with various char values
    for val in [-2i8, -1, 0, 1, 2, 127, -128, 42] {
        let args = format!("fn.argtypes = [ctypes.c_char]; fn(ctypes.c_char({}))", val);
        let c_out = capture_fn_output(C_LIB, "printHexCharLine", &args);
        let r_out = capture_fn_output(&rust_lib_path(), "printHexCharLine", &args);
        assert_eq!(c_out, r_out, "printHexCharLine mismatch for val={}", val);
    }
}

#[test]
fn test_print_line() {
    for s in ["hello", "", "data value is too large to perform arithmetic safely."] {
        let args = format!("fn.argtypes = [ctypes.c_char_p]; fn(b\"{}\")", s);
        let c_out = capture_fn_output(C_LIB, "printLine", &args);
        let r_out = capture_fn_output(&rust_lib_path(), "printLine", &args);
        assert_eq!(c_out, r_out, "printLine mismatch for s={:?}", s);
    }
}

#[test]
fn test_print_line_null() {
    let args = "fn.argtypes = [ctypes.c_char_p]; fn(None)";
    let c_out = capture_fn_output(C_LIB, "printLine", args);
    let r_out = capture_fn_output(&rust_lib_path(), "printLine", args);
    assert_eq!(c_out, r_out, "printLine(NULL) mismatch");
}

#[test]
fn test_bad() {
    let c_out = capture_fn_output(C_LIB, "bad", "fn()");
    let r_out = capture_fn_output(&rust_lib_path(), "bad", "fn()");
    assert_eq!(c_out, r_out, "bad() mismatch");
}

#[test]
fn test_good() {
    let c_out = capture_fn_output(C_LIB, "good", "fn()");
    let r_out = capture_fn_output(&rust_lib_path(), "good", "fn()");
    assert_eq!(c_out, r_out, "good() mismatch");
}

#[test]
fn test_symbol_exports() {
    let c_out = Command::new("nm")
        .args(["-D", C_LIB])
        .output()
        .expect("nm failed");
    let r_out = Command::new("nm")
        .args(["-D", &rust_lib_path()])
        .output()
        .expect("nm failed");

    let c_syms: Vec<&str> = std::str::from_utf8(&c_out.stdout)
        .unwrap()
        .lines()
        .filter(|l| l.contains(" T "))
        .filter_map(|l| l.split_whitespace().last())
        .filter(|s| !s.starts_with('_'))
        .collect();

    let r_syms: Vec<&str> = std::str::from_utf8(&r_out.stdout)
        .unwrap()
        .lines()
        .filter(|l| l.contains(" T "))
        .filter_map(|l| l.split_whitespace().last())
        .collect();

    for sym in &c_syms {
        assert!(
            r_syms.contains(sym),
            "C exports symbol '{}' but Rust .so does not",
            sym
        );
    }
}
