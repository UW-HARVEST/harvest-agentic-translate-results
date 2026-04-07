use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, CStr, CString};
use std::path::PathBuf;
use std::ptr;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    // The cdylib is built in target/<profile>/
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target/debug/libcomplexmode_lib.so");
    p
}

struct Libs {
    c: Library,
    rs: Library,
}

impl Libs {
    fn load() -> Self {
        let c = unsafe { Library::new(c_lib_path()) }.expect("load C .so");
        let rs = unsafe { Library::new(rust_lib_path()) }.expect("load Rust .so");
        Libs { c, rs }
    }
}

// ── check_permissions ──

#[test]
fn test_check_permissions() {
    let libs = Libs::load();
    type Fn = unsafe extern "C" fn(c_int, c_int) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"check_permissions") }.unwrap();
    let rs_fn: Symbol<Fn> = unsafe { libs.rs.get(b"check_permissions") }.unwrap();

    let cases = [
        (0o644, 0o400 | 0o200), // rw required, have rw-r--r--
        (0o644, 0o100),          // exec required, don't have
        (0o755, 0o100),          // exec required, have
        (0, 0),
        (0o777, 0o777),
    ];
    for (perms, req) in cases {
        let c_r = unsafe { c_fn(perms, req) };
        let rs_r = unsafe { rs_fn(perms, req) };
        assert_eq!(c_r, rs_r, "check_permissions({perms:#o}, {req:#o})");
    }
}

// ── create_result_string ──

#[test]
fn test_create_result_string() {
    let libs = Libs::load();
    type Fn = unsafe extern "C" fn(*const c_char, c_int) -> *mut c_char;
    let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"create_result_string") }.unwrap();
    let rs_fn: Symbol<Fn> = unsafe { libs.rs.get(b"create_result_string") }.unwrap();

    let ops = [b"add\0" as &[u8], b"multiply\0", b"\0"];
    let vals = [0, 42, -1, 999999];

    for op in &ops {
        for &val in &vals {
            let c_ptr = unsafe { c_fn(op.as_ptr() as *const c_char, val) };
            let rs_ptr = unsafe { rs_fn(op.as_ptr() as *const c_char, val) };
            assert!(!c_ptr.is_null());
            assert!(!rs_ptr.is_null());
            let c_str = unsafe { CStr::from_ptr(c_ptr) };
            let rs_str = unsafe { CStr::from_ptr(rs_ptr) };
            assert_eq!(c_str, rs_str, "create_result_string({op:?}, {val})");
            unsafe {
                libc::free(c_ptr as *mut libc::c_void);
                libc::free(rs_ptr as *mut libc::c_void);
            }
        }
    }
}

// ── safe_add ──

#[test]
fn test_safe_add() {
    let libs = Libs::load();
    type Fn = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"safe_add") }.unwrap();
    let rs_fn: Symbol<Fn> = unsafe { libs.rs.get(b"safe_add") }.unwrap();

    let cases = [
        (3, 4, 0o644),  // has rw perms
        (3, 4, 0o100),  // no rw perms
        (0, 0, 0o600),
        (-5, 10, 0o644),
    ];
    for (a, b, p) in cases {
        let c_r = unsafe { c_fn(a, b, p) };
        let rs_r = unsafe { rs_fn(a, b, p) };
        assert_eq!(c_r, rs_r, "safe_add({a}, {b}, {p:#o})");
    }
}

// ── multiply_with_log ──

#[test]
fn test_multiply_with_log() {
    let libs = Libs::load();
    type Fn = unsafe extern "C" fn(c_int, c_int, *mut *mut c_char) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"multiply_with_log") }.unwrap();
    let rs_fn: Symbol<Fn> = unsafe { libs.rs.get(b"multiply_with_log") }.unwrap();

    let cases = [(3, 4), (0, 100), (-2, 5), (0, 0)];
    for (a, b) in cases {
        let mut c_log: *mut c_char = ptr::null_mut();
        let mut rs_log: *mut c_char = ptr::null_mut();
        let c_r = unsafe { c_fn(a, b, &mut c_log) };
        let rs_r = unsafe { rs_fn(a, b, &mut rs_log) };
        assert_eq!(c_r, rs_r, "multiply_with_log return ({a}, {b})");
        if !c_log.is_null() && !rs_log.is_null() {
            let c_s = unsafe { CStr::from_ptr(c_log) };
            let rs_s = unsafe { CStr::from_ptr(rs_log) };
            assert_eq!(c_s, rs_s, "multiply_with_log log ({a}, {b})");
            unsafe {
                libc::free(c_log as *mut libc::c_void);
                libc::free(rs_log as *mut libc::c_void);
            }
        }
    }
}

// ── copy_and_sum ──

#[test]
fn test_copy_and_sum() {
    let libs = Libs::load();
    type Fn = unsafe extern "C" fn(*const c_int, c_int) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"copy_and_sum") }.unwrap();
    let rs_fn: Symbol<Fn> = unsafe { libs.rs.get(b"copy_and_sum") }.unwrap();

    // normal arrays
    let arr1: [c_int; 3] = [1, 2, 3];
    let arr2: [c_int; 5] = [10, -20, 30, -40, 50];
    let arr3: [c_int; 1] = [42];

    for (arr, len) in [
        (arr1.as_ptr(), 3),
        (arr2.as_ptr(), 5),
        (arr3.as_ptr(), 1),
    ] {
        let c_r = unsafe { c_fn(arr, len) };
        let rs_r = unsafe { rs_fn(arr, len) };
        assert_eq!(c_r, rs_r, "copy_and_sum len={len}");
    }

    // null pointer
    let c_r = unsafe { c_fn(ptr::null(), 3) };
    let rs_r = unsafe { rs_fn(ptr::null(), 3) };
    assert_eq!(c_r, rs_r, "copy_and_sum null");
}

// ── compare_operations ──

#[test]
fn test_compare_operations() {
    let libs = Libs::load();
    type Fn = unsafe extern "C" fn(*const c_char, *const c_char) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"compare_operations") }.unwrap();
    let rs_fn: Symbol<Fn> = unsafe { libs.rs.get(b"compare_operations") }.unwrap();

    let a = CString::new("add").unwrap();
    let b = CString::new("multiply").unwrap();
    let c = CString::new("add").unwrap();

    let cases: Vec<(*const c_char, *const c_char, &str)> = vec![
        (a.as_ptr(), b.as_ptr(), "add vs multiply"),
        (b.as_ptr(), a.as_ptr(), "multiply vs add"),
        (a.as_ptr(), c.as_ptr(), "add vs add"),
        (ptr::null(), a.as_ptr(), "null vs add"),
        (a.as_ptr(), ptr::null(), "add vs null"),
        (ptr::null(), ptr::null(), "null vs null"),
    ];
    for (p1, p2, label) in cases {
        let c_r = unsafe { c_fn(p1, p2) };
        let rs_r = unsafe { rs_fn(p1, p2) };
        // strcmp returns <0, 0, >0 — compare signs
        assert_eq!(c_r.signum(), rs_r.signum(), "compare_operations {label}: c={c_r} rs={rs_r}");
    }
}

// ── complexmode ──

#[test]
fn test_complexmode() {
    let libs = Libs::load();
    type Fn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"complexmode") }.unwrap();
    let rs_fn: Symbol<Fn> = unsafe { libs.rs.get(b"complexmode") }.unwrap();

    let cases = [
        (1, 10, 20, 0),   // addition mode
        (2, 3, 7, 0),     // multiplication mode
        (3, 1, 2, 3),     // array sum mode
        (4, 5, 6, 7),     // complex calc mode
        (0, 0, 0, 0),     // invalid mode
        (99, 1, 2, 3),    // invalid mode
    ];
    for (mode, v1, v2, v3) in cases {
        let c_r = unsafe { c_fn(mode, v1, v2, v3) };
        let rs_r = unsafe { rs_fn(mode, v1, v2, v3) };
        assert_eq!(c_r, rs_r, "complexmode({mode}, {v1}, {v2}, {v3})");
    }
}
