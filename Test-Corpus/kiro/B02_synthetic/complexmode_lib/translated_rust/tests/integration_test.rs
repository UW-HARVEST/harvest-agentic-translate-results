use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, CStr, CString};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target/debug/libcomplexmode_lib.so");
    // If debug doesn't exist try release
    if !p.exists() {
        p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("target/release/libcomplexmode_lib.so");
    }
    p
}

#[test]
fn test_check_permissions() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let r_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    type Fn = unsafe extern "C" fn(c_int, c_int) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"check_permissions").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"check_permissions").unwrap() };

    let cases = [
        (0o644, 0o400),  // has read
        (0o644, 0o200),  // has write
        (0o644, 0o100),  // no exec
        (0o755, 0o100),  // has exec
        (0o000, 0o400),  // no perms
        (0o644, 0o600),  // read+write
    ];
    for (perms, req) in cases {
        let c_res = unsafe { c_fn(perms, req) };
        let r_res = unsafe { r_fn(perms, req) };
        assert_eq!(c_res, r_res, "check_permissions({:#o}, {:#o}): C={} Rust={}", perms, req, c_res, r_res);
    }
}

#[test]
fn test_create_result_string() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let r_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    type Fn = unsafe extern "C" fn(*const c_char, c_int) -> *mut c_char;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"create_result_string").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"create_result_string").unwrap() };

    let ops = ["add", "multiply", "test_op"];
    let vals = [42, 0, -7, 999];
    for op in &ops {
        for &val in &vals {
            let c_op = CString::new(*op).unwrap();
            let c_ptr = unsafe { c_fn(c_op.as_ptr(), val) };
            let r_ptr = unsafe { r_fn(c_op.as_ptr(), val) };
            assert!(!c_ptr.is_null());
            assert!(!r_ptr.is_null());
            let c_str = unsafe { CStr::from_ptr(c_ptr) };
            let r_str = unsafe { CStr::from_ptr(r_ptr) };
            assert_eq!(c_str, r_str, "create_result_string(\"{}\", {}): C={:?} Rust={:?}", op, val, c_str, r_str);
            unsafe { libc::free(c_ptr as *mut _); libc::free(r_ptr as *mut _); }
        }
    }
}

#[test]
fn test_safe_add() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let r_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    type Fn = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"safe_add").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"safe_add").unwrap() };

    let cases = [
        (3, 4, 0o644),   // has rw
        (3, 4, 0o000),   // no perms
        (10, -5, 0o600), // has rw
        (0, 0, 0o755),   // has rwx
    ];
    for (a, b, perms) in cases {
        let c_res = unsafe { c_fn(a, b, perms) };
        let r_res = unsafe { r_fn(a, b, perms) };
        assert_eq!(c_res, r_res, "safe_add({}, {}, {:#o}): C={} Rust={}", a, b, perms, c_res, r_res);
    }
}

#[test]
fn test_multiply_with_log() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let r_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    type Fn = unsafe extern "C" fn(c_int, c_int, *mut *mut c_char) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"multiply_with_log").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"multiply_with_log").unwrap() };

    let cases = [(3, 4), (0, 5), (-2, 7), (100, 100)];
    for (a, b) in cases {
        let mut c_msg: *mut c_char = std::ptr::null_mut();
        let mut r_msg: *mut c_char = std::ptr::null_mut();
        let c_res = unsafe { c_fn(a, b, &mut c_msg) };
        let r_res = unsafe { r_fn(a, b, &mut r_msg) };
        assert_eq!(c_res, r_res, "multiply_with_log({}, {}): C={} Rust={}", a, b, c_res, r_res);
        if !c_msg.is_null() && !r_msg.is_null() {
            let c_str = unsafe { CStr::from_ptr(c_msg) };
            let r_str = unsafe { CStr::from_ptr(r_msg) };
            assert_eq!(c_str, r_str, "multiply_with_log({}, {}) log: C={:?} Rust={:?}", a, b, c_str, r_str);
            unsafe { libc::free(c_msg as *mut _); libc::free(r_msg as *mut _); }
        }
    }
}

#[test]
fn test_copy_and_sum() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let r_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    type Fn = unsafe extern "C" fn(*mut c_int, c_int) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"copy_and_sum").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"copy_and_sum").unwrap() };

    // Normal case
    let mut vals = [1, 2, 3, 4, 5];
    let c_res = unsafe { c_fn(vals.as_mut_ptr(), 5) };
    let r_res = unsafe { r_fn(vals.as_mut_ptr(), 5) };
    assert_eq!(c_res, r_res, "copy_and_sum([1..5], 5): C={} Rust={}", c_res, r_res);

    // NULL case
    let c_null = unsafe { c_fn(std::ptr::null_mut(), 0) };
    let r_null = unsafe { r_fn(std::ptr::null_mut(), 0) };
    assert_eq!(c_null, r_null, "copy_and_sum(NULL, 0): C={} Rust={}", c_null, r_null);
}

#[test]
fn test_compare_operations() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let r_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    type Fn = unsafe extern "C" fn(*const c_char, *const c_char) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"compare_operations").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"compare_operations").unwrap() };

    let a = CString::new("add").unwrap();
    let b = CString::new("multiply").unwrap();
    let c = CString::new("add").unwrap();

    // Equal
    let c_res = unsafe { c_fn(a.as_ptr(), c.as_ptr()) };
    let r_res = unsafe { r_fn(a.as_ptr(), c.as_ptr()) };
    assert_eq!(c_res, r_res, "compare_operations(add, add): C={} Rust={}", c_res, r_res);

    // Different
    let c_res = unsafe { c_fn(a.as_ptr(), b.as_ptr()) };
    let r_res = unsafe { r_fn(a.as_ptr(), b.as_ptr()) };
    assert_eq!(c_res, r_res, "compare_operations(add, multiply): C={} Rust={}", c_res, r_res);

    // NULL
    let c_res = unsafe { c_fn(std::ptr::null(), b.as_ptr()) };
    let r_res = unsafe { r_fn(std::ptr::null(), b.as_ptr()) };
    assert_eq!(c_res, r_res, "compare_operations(NULL, multiply): C={} Rust={}", c_res, r_res);
}

#[test]
fn test_complexmode() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let r_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    type Fn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"complexmode").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"complexmode").unwrap() };

    let cases = [
        (1, 10, 20, 0),    // addition mode
        (2, 5, 6, 0),      // multiplication mode
        (3, 1, 2, 3),      // array sum mode
        (4, 3, 4, 5),      // complex mode
        (99, 0, 0, 0),     // invalid mode
    ];
    for (mode, v1, v2, v3) in cases {
        let c_res = unsafe { c_fn(mode, v1, v2, v3) };
        let r_res = unsafe { r_fn(mode, v1, v2, v3) };
        assert_eq!(c_res, r_res, "complexmode({}, {}, {}, {}): C={} Rust={}", mode, v1, v2, v3, c_res, r_res);
    }
}
