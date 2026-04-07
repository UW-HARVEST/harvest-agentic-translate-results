use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    // cdylib output goes to target/<profile>/
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target/debug/libconfusion_lib.so");
    p
}

/// Helper: load both libraries
fn load_libs() -> (Library, Library) {
    unsafe {
        let c = Library::new(c_lib_path()).expect("load C .so");
        let r = Library::new(rust_lib_path()).expect("load Rust .so");
        (c, r)
    }
}

// ── confusion (top-level, exercises all sub-functions) ──────────────

#[test]
fn test_confusion_basic_inputs() {
    let (c, r) = load_libs();
    type ConfusionFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
    unsafe {
        let c_fn: Symbol<ConfusionFn> = c.get(b"confusion").unwrap();
        let r_fn: Symbol<ConfusionFn> = r.get(b"confusion").unwrap();

        let cases: &[(c_int, c_int, c_int, c_int)] = &[
            (0, 0, 0, 0),
            (1, 2, 3, 4),
            (42, 7, 5, 1),
            (100, 15, 0, 2),
            (255, 31, 9, 3),
            (-1, 0, 0, 0),
            (1078530011, 0xFF, 3, 0),
            (0, 0, 0, 1),
            (0, 0, 0, 2),
            (0, 0, 0, 3),
            (10, 63, 7, 0),
        ];

        for &(a, b, c_val, d) in cases {
            let c_res = c_fn(a, b, c_val, d);
            let r_res = r_fn(a, b, c_val, d);
            assert_eq!(
                c_res, r_res,
                "confusion({}, {}, {}, {}): C={} Rust={}",
                a, b, c_val, d, c_res, r_res
            );
        }
    }
}

// ── create_state / destroy_state ────────────────────────────────────

#[test]
fn test_create_destroy_state() {
    let (c, r) = load_libs();
    type CreateFn = unsafe extern "C" fn(c_int, c_int) -> *mut c_void;
    type DestroyFn = unsafe extern "C" fn(*mut c_void);
    unsafe {
        let c_create: Symbol<CreateFn> = c.get(b"create_state").unwrap();
        let r_create: Symbol<CreateFn> = r.get(b"create_state").unwrap();
        let c_destroy: Symbol<DestroyFn> = c.get(b"destroy_state").unwrap();
        let r_destroy: Symbol<DestroyFn> = r.get(b"destroy_state").unwrap();

        for &(init, cap) in &[(0, 64), (42, 128), (999, 256)] {
            let cs = c_create(init, cap);
            let rs = r_create(init, cap);
            assert!(!cs.is_null(), "C create_state returned null for ({}, {})", init, cap);
            assert!(!rs.is_null(), "Rust create_state returned null for ({}, {})", init, cap);
            c_destroy(cs);
            r_destroy(rs);
        }

        // null destroy should not crash
        c_destroy(std::ptr::null_mut());
        r_destroy(std::ptr::null_mut());
    }
}

// ── process_buffer ──────────────────────────────────────────────────

#[test]
fn test_process_buffer() {
    let (c, r) = load_libs();
    type CreateFn = unsafe extern "C" fn(c_int, c_int) -> *mut c_void;
    type DestroyFn = unsafe extern "C" fn(*mut c_void);
    type ProcessFn = unsafe extern "C" fn(*mut c_void, c_char) -> c_int;
    unsafe {
        let c_create: Symbol<CreateFn> = c.get(b"create_state").unwrap();
        let r_create: Symbol<CreateFn> = r.get(b"create_state").unwrap();
        let c_destroy: Symbol<DestroyFn> = c.get(b"destroy_state").unwrap();
        let r_destroy: Symbol<DestroyFn> = r.get(b"destroy_state").unwrap();
        let c_proc: Symbol<ProcessFn> = c.get(b"process_buffer").unwrap();
        let r_proc: Symbol<ProcessFn> = r.get(b"process_buffer").unwrap();

        // Buffer will be "State:<init>:Mode:3"
        for &init_val in &[0, 42, 100, 999] {
            let cs = c_create(init_val, 128);
            let rs = r_create(init_val, 128);
            // Search for various chars
            for ch in b"0123456789:SMtaode" {
                let c_res = c_proc(cs, *ch as c_char);
                let r_res = r_proc(rs, *ch as c_char);
                assert_eq!(
                    c_res, r_res,
                    "process_buffer(state({}), '{}' / 0x{:02x}): C={} Rust={}",
                    init_val, *ch as char, ch, c_res, r_res
                );
            }
            c_destroy(cs);
            r_destroy(rs);
        }

        // null state
        let c_null = c_proc(std::ptr::null_mut(), b'a' as c_char);
        let r_null = r_proc(std::ptr::null_mut(), b'a' as c_char);
        assert_eq!(c_null, r_null, "process_buffer(null): C={} Rust={}", c_null, r_null);
    }
}

// ── update_flags ────────────────────────────────────────────────────

#[test]
fn test_update_flags_via_confusion() {
    // update_flags is tested indirectly through confusion with various param2 values
    let (c, r) = load_libs();
    type ConfusionFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
    unsafe {
        let c_fn: Symbol<ConfusionFn> = c.get(b"confusion").unwrap();
        let r_fn: Symbol<ConfusionFn> = r.get(b"confusion").unwrap();

        // param2 controls flags: flag1=bit0, flag2=bit1, flag3=bit2, mode=bits3-5
        for param2 in [0, 1, 2, 3, 4, 7, 8, 15, 16, 31, 63, 127, 255] {
            let c_res = c_fn(10, param2, 0, 0);
            let r_res = r_fn(10, param2, 0, 0);
            assert_eq!(c_res, r_res, "confusion(10, {}, 0, 0): C={} Rust={}", param2, c_res, r_res);
        }
    }
}

// ── confuse_types ───────────────────────────────────────────────────

#[test]
fn test_confuse_types_all_operations() {
    let (c, r) = load_libs();
    type ConfusionFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
    unsafe {
        let c_fn: Symbol<ConfusionFn> = c.get(b"confusion").unwrap();
        let r_fn: Symbol<ConfusionFn> = r.get(b"confusion").unwrap();

        // param4 % 4 selects the confuse_types operation
        for op in 0..4 {
            for &p1 in &[0, 1, 42, 1078530011, -1] {
                let c_res = c_fn(p1, 0, 0, op);
                let r_res = r_fn(p1, 0, 0, op);
                assert_eq!(
                    c_res, r_res,
                    "confusion({}, 0, 0, {}): C={} Rust={}",
                    p1, op, c_res, r_res
                );
            }
        }
    }
}
