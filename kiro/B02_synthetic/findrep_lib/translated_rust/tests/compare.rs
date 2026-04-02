use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let debug = dir.join("target/debug/libfindrep_lib.so");
    if debug.exists() {
        return debug;
    }
    dir.join("target/release/libfindrep_lib.so")
}

// ---- validate_and_normalize (pure function, no state) ----

#[test]
fn test_validate_and_normalize() {
    let c_lib = unsafe { Library::new(c_lib_path()) }.expect("load C lib");
    let r_lib = unsafe { Library::new(rust_lib_path()) }.expect("load Rust lib");

    let c_fn: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
        unsafe { c_lib.get(b"validate_and_normalize") }.expect("c sym");
    let r_fn: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
        unsafe { r_lib.get(b"validate_and_normalize") }.expect("r sym");

    let test_vals = [
        -1000, -1, 0, 1, 32, 63, 64, 65, 100, 200, 510, 511, 512, 1000,
    ];
    for &v in &test_vals {
        let c_res = unsafe { c_fn(v) };
        let r_res = unsafe { r_fn(v) };
        assert_eq!(c_res, r_res, "validate_and_normalize({v}): C={c_res} Rust={r_res}");
    }
}

// ---- add_to_accumulator (stateful) ----

#[test]
fn test_add_to_accumulator() {
    let c_lib = unsafe { Library::new(c_lib_path()) }.expect("load C lib");
    let r_lib = unsafe { Library::new(rust_lib_path()) }.expect("load Rust lib");

    let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int) -> c_int> =
        unsafe { c_lib.get(b"add_to_accumulator") }.expect("c sym");
    let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int) -> c_int> =
        unsafe { r_lib.get(b"add_to_accumulator") }.expect("r sym");

    let cases = [(1, 2), (10, 20), (-5, 3), (0, 0), (100, -100)];
    for &(a, b) in &cases {
        let c_res = unsafe { c_fn(a, b) };
        let r_res = unsafe { r_fn(a, b) };
        assert_eq!(c_res, r_res, "add_to_accumulator({a},{b}): C={c_res} Rust={r_res}");
    }
}

// ---- multiply_with_multiplier (stateful) ----

#[test]
fn test_multiply_with_multiplier() {
    let c_lib = unsafe { Library::new(c_lib_path()) }.expect("load C lib");
    let r_lib = unsafe { Library::new(rust_lib_path()) }.expect("load Rust lib");

    let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int) -> c_int> =
        unsafe { c_lib.get(b"multiply_with_multiplier") }.expect("c sym");
    let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int) -> c_int> =
        unsafe { r_lib.get(b"multiply_with_multiplier") }.expect("r sym");

    let cases = [(2, 3), (1, 1), (-1, 5), (0, 10), (4, 4)];
    for &(a, b) in &cases {
        let c_res = unsafe { c_fn(a, b) };
        let r_res = unsafe { r_fn(a, b) };
        assert_eq!(c_res, r_res, "multiply_with_multiplier({a},{b}): C={c_res} Rust={r_res}");
    }
}

// ---- subtract_from_accumulator (stateful) ----

#[test]
fn test_subtract_from_accumulator() {
    let c_lib = unsafe { Library::new(c_lib_path()) }.expect("load C lib");
    let r_lib = unsafe { Library::new(rust_lib_path()) }.expect("load Rust lib");

    let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int) -> c_int> =
        unsafe { c_lib.get(b"subtract_from_accumulator") }.expect("c sym");
    let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int) -> c_int> =
        unsafe { r_lib.get(b"subtract_from_accumulator") }.expect("r sym");

    let cases = [(10, 3), (0, 0), (5, 10), (-3, -7), (100, 50)];
    for &(a, b) in &cases {
        let c_res = unsafe { c_fn(a, b) };
        let r_res = unsafe { r_fn(a, b) };
        assert_eq!(c_res, r_res, "subtract_from_accumulator({a},{b}): C={c_res} Rust={r_res}");
    }
}

// ---- divide_multiplier (stateful) ----

#[test]
fn test_divide_multiplier() {
    let c_lib = unsafe { Library::new(c_lib_path()) }.expect("load C lib");
    let r_lib = unsafe { Library::new(rust_lib_path()) }.expect("load Rust lib");

    let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int) -> c_int> =
        unsafe { c_lib.get(b"divide_multiplier") }.expect("c sym");
    let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int) -> c_int> =
        unsafe { r_lib.get(b"divide_multiplier") }.expect("r sym");

    // First multiply to get multiplier > 1, then divide
    let c_mul: Symbol<unsafe extern "C" fn(c_int, c_int) -> c_int> =
        unsafe { c_lib.get(b"multiply_with_multiplier") }.expect("c mul");
    let r_mul: Symbol<unsafe extern "C" fn(c_int, c_int) -> c_int> =
        unsafe { r_lib.get(b"multiply_with_multiplier") }.expect("r mul");

    unsafe { c_mul(10, 10) };
    unsafe { r_mul(10, 10) };

    let cases = [(0, 2), (0, 5), (0, 0), (0, 3)];
    for &(a, b) in &cases {
        let c_res = unsafe { c_fn(a, b) };
        let r_res = unsafe { r_fn(a, b) };
        assert_eq!(c_res, r_res, "divide_multiplier({a},{b}): C={c_res} Rust={r_res}");
    }
}

// ---- process_octal_string ----

#[test]
fn test_process_octal_string() {
    let c_lib = unsafe { Library::new(c_lib_path()) }.expect("load C lib");
    let r_lib = unsafe { Library::new(rust_lib_path()) }.expect("load Rust lib");

    let c_fn: Symbol<unsafe extern "C" fn(*mut u8, c_int)> =
        unsafe { c_lib.get(b"process_octal_string") }.expect("c sym");
    let r_fn: Symbol<unsafe extern "C" fn(*mut u8, c_int)> =
        unsafe { r_lib.get(b"process_octal_string") }.expect("r sym");

    for &val in &[0, 1, 7, 8, 63, 64, 83, 255, 511, 512] {
        let mut c_buf = [0u8; 100];
        let mut r_buf = [0u8; 100];
        unsafe { c_fn(c_buf.as_mut_ptr(), val) };
        unsafe { r_fn(r_buf.as_mut_ptr(), val) };
        assert_eq!(c_buf, r_buf, "process_octal_string({val}): C={:?} Rust={:?}",
            String::from_utf8_lossy(&c_buf), String::from_utf8_lossy(&r_buf));
    }
}

// ---- find_and_replace_char ----

#[test]
fn test_find_and_replace_char() {
    let c_lib = unsafe { Library::new(c_lib_path()) }.expect("load C lib");
    let r_lib = unsafe { Library::new(rust_lib_path()) }.expect("load Rust lib");

    let c_fn: Symbol<unsafe extern "C" fn(*mut u8, c_int)> =
        unsafe { c_lib.get(b"find_and_replace_char") }.expect("c sym");
    let r_fn: Symbol<unsafe extern "C" fn(*mut u8, c_int)> =
        unsafe { r_lib.get(b"find_and_replace_char") }.expect("r sym");

    let test_cases: &[(&[u8], i32)] = &[
        (b"hello world\0", b'o' as i32),
        (b"abcdef\0", b'z' as i32),
        (b"test\0", b't' as i32),
        (b"\0", b'a' as i32),
    ];
    for &(src, ch) in test_cases {
        let mut c_buf = [0u8; 100];
        let mut r_buf = [0u8; 100];
        c_buf[..src.len()].copy_from_slice(src);
        r_buf[..src.len()].copy_from_slice(src);
        unsafe { c_fn(c_buf.as_mut_ptr(), ch) };
        unsafe { r_fn(r_buf.as_mut_ptr(), ch) };
        assert_eq!(c_buf, r_buf, "find_and_replace_char({:?}, {})",
            String::from_utf8_lossy(src), ch as u8 as char);
    }
}

// ---- findrep (top-level, fresh load each case to reset globals) ----

#[test]
fn test_findrep() {
    let cases = [
        (1, 2, 3, 4),
        (0, 0, 0, 0),
        (100, 200, 300, 400),
        (-1, -2, -3, -4),
        (1, 0, 0, 0),
        (10, 20, 0, 0),
        (0, 0, 50, 60),
        (511, 511, 511, 511),
        (512, 512, 512, 512),
        (1, 1, 1, 1),
    ];
    for &(a, b, c, d) in &cases {
        // Fresh load each time to reset static globals
        let c_lib = unsafe { Library::new(c_lib_path()) }.expect("load C lib");
        let r_lib = unsafe { Library::new(rust_lib_path()) }.expect("load Rust lib");

        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            unsafe { c_lib.get(b"findrep") }.expect("c sym");
        let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            unsafe { r_lib.get(b"findrep") }.expect("r sym");

        let c_res = unsafe { c_fn(a, b, c, d) };
        let r_res = unsafe { r_fn(a, b, c, d) };
        assert_eq!(c_res, r_res, "findrep({a},{b},{c},{d}): C={c_res} Rust={r_res}");
    }
}

// ---- findrep sequential calls (globals accumulate) ----

#[test]
fn test_findrep_sequential() {
    let c_lib = unsafe { Library::new(c_lib_path()) }.expect("load C lib");
    let r_lib = unsafe { Library::new(rust_lib_path()) }.expect("load Rust lib");

    let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
        unsafe { c_lib.get(b"findrep") }.expect("c sym");
    let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
        unsafe { r_lib.get(b"findrep") }.expect("r sym");

    let calls = [
        (1, 2, 3, 4),
        (10, 20, 30, 40),
        (5, 5, 5, 5),
        (0, 0, 0, 0),
        (100, 200, 300, 400),
    ];
    for &(a, b, c, d) in &calls {
        let c_res = unsafe { c_fn(a, b, c, d) };
        let r_res = unsafe { r_fn(a, b, c, d) };
        assert_eq!(c_res, r_res, "findrep_seq({a},{b},{c},{d}): C={c_res} Rust={r_res}");
    }
}
