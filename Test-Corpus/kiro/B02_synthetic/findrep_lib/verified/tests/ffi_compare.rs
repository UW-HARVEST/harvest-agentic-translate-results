use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    // The cdylib is built alongside the test artifacts
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug");
    dir.join("libfindrep_lib.so")
}

/// Load a fresh copy of the library by copying it to a temp file.
/// This ensures each test gets fresh static state.
fn load_fresh(src: &std::path::Path, tag: &str) -> Library {
    let tmp = std::env::temp_dir().join(format!("test_{}_{}.so", tag, std::process::id()));
    std::fs::copy(src, &tmp).unwrap();
    unsafe { Library::new(&tmp).unwrap() }
}

// ---- Leaf-level tests (no global state) ----

#[test]
fn test_validate_and_normalize() {
    let c = load_fresh(&c_lib_path(), "c_van");
    let r = load_fresh(&rust_lib_path(), "r_van");

    type Fn = unsafe extern "C" fn(c_int) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { c.get(b"validate_and_normalize").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r.get(b"validate_and_normalize").unwrap() };

    let cases = [0, 1, -1, 63, 64, 65, 100, 510, 511, 512, 1000, -100, i32::MAX, i32::MIN];
    for &v in &cases {
        let c_res = unsafe { c_fn(v) };
        let r_res = unsafe { r_fn(v) };
        assert_eq!(c_res, r_res, "validate_and_normalize({v}): C={c_res} Rust={r_res}");
    }
}

#[test]
fn test_process_octal_string() {
    let c = load_fresh(&c_lib_path(), "c_pos");
    let r = load_fresh(&rust_lib_path(), "r_pos");

    type Fn = unsafe extern "C" fn(*mut u8, c_int);
    let c_fn: Symbol<Fn> = unsafe { c.get(b"process_octal_string").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r.get(b"process_octal_string").unwrap() };

    for &val in &[0, 1, 0o123, 255, 511, 1000] {
        let mut c_buf = [0u8; 100];
        let mut r_buf = [0u8; 100];
        unsafe {
            c_fn(c_buf.as_mut_ptr(), val);
            r_fn(r_buf.as_mut_ptr(), val);
        }
        assert_eq!(c_buf, r_buf, "process_octal_string({val})");
    }
}

#[test]
fn test_find_and_replace_char() {
    let c = load_fresh(&c_lib_path(), "c_farc");
    let r = load_fresh(&rust_lib_path(), "r_farc");

    type Fn = unsafe extern "C" fn(*mut u8, c_int);
    let c_fn: Symbol<Fn> = unsafe { c.get(b"find_and_replace_char").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r.get(b"find_and_replace_char").unwrap() };

    let cases: &[(&[u8], i32)] = &[
        (b"hello world\0", b'o' as i32),
        (b"hello world\0", b'z' as i32),
        (b"aaa\0", b'a' as i32),
        (b"\0", b'x' as i32),
        (b"Octal: 0123\0", b'O' as i32),
    ];

    for (input, ch) in cases {
        let mut c_buf = [0u8; 100];
        let mut r_buf = [0u8; 100];
        c_buf[..input.len()].copy_from_slice(input);
        r_buf[..input.len()].copy_from_slice(input);
        unsafe {
            c_fn(c_buf.as_mut_ptr(), *ch);
            r_fn(r_buf.as_mut_ptr(), *ch);
        }
        assert_eq!(c_buf, r_buf, "find_and_replace_char({:?}, {})", std::str::from_utf8(input).unwrap_or("?"), ch);
    }
}

// ---- Stateful functions: test in identical call sequences ----

#[test]
fn test_accumulator_ops() {
    let c = load_fresh(&c_lib_path(), "c_acc");
    let r = load_fresh(&rust_lib_path(), "r_acc");

    type Fn2 = unsafe extern "C" fn(c_int, c_int) -> c_int;
    let c_add: Symbol<Fn2> = unsafe { c.get(b"add_to_accumulator").unwrap() };
    let r_add: Symbol<Fn2> = unsafe { r.get(b"add_to_accumulator").unwrap() };
    let c_sub: Symbol<Fn2> = unsafe { c.get(b"subtract_from_accumulator").unwrap() };
    let r_sub: Symbol<Fn2> = unsafe { r.get(b"subtract_from_accumulator").unwrap() };

    let calls: &[(bool, i32, i32)] = &[
        (true, 10, 20),
        (true, 5, 3),
        (false, 100, 40),
        (true, 0, 0),
        (false, 7, 2),
    ];

    for &(is_add, a, b) in calls {
        let (c_res, r_res) = if is_add {
            (unsafe { c_add(a, b) }, unsafe { r_add(a, b) })
        } else {
            (unsafe { c_sub(a, b) }, unsafe { r_sub(a, b) })
        };
        assert_eq!(c_res, r_res, "accumulator op({is_add}, {a}, {b}): C={c_res} Rust={r_res}");
    }
}

#[test]
fn test_multiplier_ops() {
    let c = load_fresh(&c_lib_path(), "c_mul");
    let r = load_fresh(&rust_lib_path(), "r_mul");

    type Fn2 = unsafe extern "C" fn(c_int, c_int) -> c_int;
    let c_mul: Symbol<Fn2> = unsafe { c.get(b"multiply_with_multiplier").unwrap() };
    let r_mul: Symbol<Fn2> = unsafe { r.get(b"multiply_with_multiplier").unwrap() };
    let c_div: Symbol<Fn2> = unsafe { c.get(b"divide_multiplier").unwrap() };
    let r_div: Symbol<Fn2> = unsafe { r.get(b"divide_multiplier").unwrap() };

    let calls: &[(bool, i32, i32)] = &[
        (true, 3, 4),
        (true, 2, 1),
        (false, 999, 3),
        (false, 0, 0),  // divide by zero - should be no-op
        (true, 1, 5),
    ];

    for &(is_mul, a, b) in calls {
        let (c_res, r_res) = if is_mul {
            (unsafe { c_mul(a, b) }, unsafe { r_mul(a, b) })
        } else {
            (unsafe { c_div(a, b) }, unsafe { r_div(a, b) })
        };
        assert_eq!(c_res, r_res, "multiplier op({is_mul}, {a}, {b}): C={c_res} Rust={r_res}");
    }
}

// ---- Top-level function ----

#[test]
fn test_findrep() {
    let c = load_fresh(&c_lib_path(), "c_fr");
    let r = load_fresh(&rust_lib_path(), "r_fr");

    type Fn4 = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
    let c_fn: Symbol<Fn4> = unsafe { c.get(b"findrep").unwrap() };
    let r_fn: Symbol<Fn4> = unsafe { r.get(b"findrep").unwrap() };

    let cases: &[(i32, i32, i32, i32)] = &[
        (0, 0, 0, 0),
        (1, 0, 0, 0),
        (1, 1, 0, 0),
        (1, 1, 1, 0),
        (1, 1, 1, 1),
        (10, 20, 30, 40),
        (100, 200, 300, 400),
        (-1, -2, -3, -4),
        (0, 1, 0, 1),
        (511, 511, 511, 511),
        (1000, 1000, 1000, 1000),
    ];

    // Each call mutates global state, so we must call in the same order.
    // Load fresh libs for each test case to isolate state.
    for &(a, b, c_val, d) in cases {
        let c_lib = load_fresh(&c_lib_path(), &format!("c_fr_{}_{}_{}_{}", a, b, c_val, d));
        let r_lib = load_fresh(&rust_lib_path(), &format!("r_fr_{}_{}_{}_{}", a, b, c_val, d));
        let c_f: Symbol<Fn4> = unsafe { c_lib.get(b"findrep").unwrap() };
        let r_f: Symbol<Fn4> = unsafe { r_lib.get(b"findrep").unwrap() };

        let c_res = unsafe { c_f(a, b, c_val, d) };
        let r_res = unsafe { r_f(a, b, c_val, d) };
        assert_eq!(c_res, r_res, "findrep({a}, {b}, {c_val}, {d}): C={c_res} Rust={r_res}");
    }
}

#[test]
fn test_findrep_sequential_calls() {
    // Test that sequential calls (with accumulating state) also match
    let c = load_fresh(&c_lib_path(), "c_frseq");
    let r = load_fresh(&rust_lib_path(), "r_frseq");

    type Fn4 = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
    let c_fn: Symbol<Fn4> = unsafe { c.get(b"findrep").unwrap() };
    let r_fn: Symbol<Fn4> = unsafe { r.get(b"findrep").unwrap() };

    let calls: &[(i32, i32, i32, i32)] = &[
        (1, 2, 3, 4),
        (10, 20, 30, 40),
        (0, 0, 0, 0),
        (100, 200, 300, 400),
        (-5, 10, -15, 20),
    ];

    for &(a, b, c_val, d) in calls {
        let c_res = unsafe { c_fn(a, b, c_val, d) };
        let r_res = unsafe { r_fn(a, b, c_val, d) };
        assert_eq!(c_res, r_res, "findrep seq({a}, {b}, {c_val}, {d}): C={c_res} Rust={r_res}");
    }
}
