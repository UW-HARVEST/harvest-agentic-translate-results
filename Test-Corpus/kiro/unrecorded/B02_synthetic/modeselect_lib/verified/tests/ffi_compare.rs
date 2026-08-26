use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, CString};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    // The cdylib is built in the deps dir or directly in target/debug
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug");
    dir.join("libmodeselect_lib.so")
}

struct Libs {
    _c: Library,
    _r: Library,
}

// Keep libraries alive while symbols are in use
static mut C_LIB: Option<Library> = None;
static mut R_LIB: Option<Library> = None;

fn load_libs() -> (&'static Library, &'static Library) {
    unsafe {
        if C_LIB.is_none() {
            C_LIB = Some(Library::new(c_lib_path()).expect("load C .so"));
        }
        if R_LIB.is_none() {
            R_LIB = Some(Library::new(rust_lib_path()).expect("load Rust .so"));
        }
        (C_LIB.as_ref().unwrap(), R_LIB.as_ref().unwrap())
    }
}

// ---- classify_mode ----
#[test]
fn test_classify_mode() {
    let (c, r) = load_libs();
    type Fn = unsafe extern "C" fn(*const c_char) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { c.get(b"classify_mode") }.unwrap();
    let r_fn: Symbol<Fn> = unsafe { r.get(b"classify_mode") }.unwrap();

    for mode in &["standard", "enhanced", "turbo", "extreme", "unknown", ""] {
        let cs = CString::new(*mode).unwrap();
        let c_res = unsafe { c_fn(cs.as_ptr()) };
        let r_res = unsafe { r_fn(cs.as_ptr()) };
        assert_eq!(c_res, r_res, "classify_mode({mode:?}): C={c_res} Rust={r_res}");
    }
}

// ---- apply_multiplier ----
#[test]
fn test_apply_multiplier() {
    let (c, r) = load_libs();
    type Fn = unsafe extern "C" fn(c_int, c_int) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { c.get(b"apply_multiplier") }.unwrap();
    let r_fn: Symbol<Fn> = unsafe { r.get(b"apply_multiplier") }.unwrap();

    for base in [0, 0xA0, -1, i32::MAX, i32::MIN] {
        for level in -1..=6 {
            let c_res = unsafe { c_fn(base, level) };
            let r_res = unsafe { r_fn(base, level) };
            assert_eq!(c_res, r_res, "apply_multiplier({base}, {level}): C={c_res} Rust={r_res}");
        }
    }
}

// ---- convert_time_factor ----
#[test]
fn test_convert_time_factor() {
    let (c, r) = load_libs();
    type Fn = unsafe extern "C" fn(f64) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { c.get(b"convert_time_factor") }.unwrap();
    let r_fn: Symbol<Fn> = unsafe { r.get(b"convert_time_factor") }.unwrap();

    for factor in [0.0, 0.001, 1.0, -1.0, 1e8, -1e8, 5e8, 1e-15, -0.0] {
        let c_res = unsafe { c_fn(factor) };
        let r_res = unsafe { r_fn(factor) };
        assert_eq!(c_res, r_res, "convert_time_factor({factor}): C={c_res} Rust={r_res}");
    }
}

// ---- convert_negative_overflow ----
#[test]
fn test_convert_negative_overflow() {
    let (c, r) = load_libs();
    type Fn = unsafe extern "C" fn(f64) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { c.get(b"convert_negative_overflow") }.unwrap();
    let r_fn: Symbol<Fn> = unsafe { r.get(b"convert_negative_overflow") }.unwrap();

    for value in [0.0, 1.0, -1.0, 3.0, -3.0, 1e7, -1e7, 0.001, -0.0] {
        let c_res = unsafe { c_fn(value) };
        let r_res = unsafe { r_fn(value) };
        assert_eq!(c_res, r_res, "convert_negative_overflow({value}): C={c_res} Rust={r_res}");
    }
}

// ---- hash_time_value ----
#[test]
fn test_hash_time_value() {
    let (c, r) = load_libs();
    type Fn = unsafe extern "C" fn(i64) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { c.get(b"hash_time_value") }.unwrap();
    let r_fn: Symbol<Fn> = unsafe { r.get(b"hash_time_value") }.unwrap();

    for t in [0i64, 1, -1, 12345678, i64::MAX, i64::MIN, 86400, 1000000] {
        let c_res = unsafe { c_fn(t) };
        let r_res = unsafe { r_fn(t) };
        assert_eq!(c_res, r_res, "hash_time_value({t}): C={c_res} Rust={r_res}");
    }
}

// ---- get_modified_time ----
#[test]
fn test_get_modified_time() {
    let (c, r) = load_libs();
    type Fn = unsafe extern "C" fn(c_int, c_int) -> i64;
    let c_fn: Symbol<Fn> = unsafe { c.get(b"get_modified_time") }.unwrap();
    let r_fn: Symbol<Fn> = unsafe { r.get(b"get_modified_time") }.unwrap();

    // Both call time(NULL) internally, so call them back-to-back
    // and allow a small tolerance for the time shift.
    // Actually, time(NULL) >> 29 changes very rarely, so they should match exactly
    // if called within the same second.
    for (days, hours) in [(0, 0), (1, 2), (-1, 0), (365, 23), (0, -5)] {
        let c_res = unsafe { c_fn(days, hours) };
        let r_res = unsafe { r_fn(days, hours) };
        assert_eq!(c_res, r_res, "get_modified_time({days}, {hours}): C={c_res} Rust={r_res}");
    }
}

// ---- modeselect (top-level) ----
#[test]
fn test_modeselect() {
    let (c, r) = load_libs();
    type Fn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { c.get(b"modeselect") }.unwrap();
    let r_fn: Symbol<Fn> = unsafe { r.get(b"modeselect") }.unwrap();

    // modeselect calls get_modified_time which uses time(NULL), so results
    // should match if called close together.
    for (ms, to, cx, sd) in [
        (0, 0, 0, 0),
        (1, 1, 1, 1),
        (2, 3, 4, 5),
        (3, 0, 2, 23),
        (0, -1, 3, 10),
        (7, 100, 9, 0),
    ] {
        let c_res = unsafe { c_fn(ms, to, cx, sd) };
        let r_res = unsafe { r_fn(ms, to, cx, sd) };
        assert_eq!(c_res, r_res, "modeselect({ms}, {to}, {cx}, {sd}): C={c_res} Rust={r_res}");
    }
}
