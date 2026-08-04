// Integration tests comparing the Rust .so output against the C .so output
// through the FFI boundary using libloading.

use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::ffi::c_int;
use std::os::raw::c_void;
use std::path::PathBuf;

type CleanupFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
type PrintResultFn = unsafe extern "C" fn(*const c_char, c_int);
type CleanupResourcesFn = unsafe extern "C" fn(*mut c_char);

fn c_lib_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src");
    p.push("build");
    p.push("libtranslated_rust.so");
    p
}

fn rust_lib_path() -> PathBuf {
    // The integration test runs after the lib is built. Locate the .so
    // alongside the test binary.
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    p.push("debug");
    p.push("libcleanup_lib.so");
    p
}

unsafe fn load_libs() -> (Library, Library) {
    unsafe {
        let c = Library::new(c_lib_path()).expect("failed to load C .so");
        let r = Library::new(rust_lib_path()).expect("failed to load Rust .so");
        (c, r)
    }
}

#[test]
fn cleanup_returns_match_for_various_inputs() {
    unsafe {
        let (c_lib, r_lib) = load_libs();
        let c_fn: Symbol<CleanupFn> = c_lib.get(b"cleanup").expect("C cleanup");
        let r_fn: Symbol<CleanupFn> = r_lib.get(b"cleanup").expect("Rust cleanup");

        // Inputs cover all switch cases plus default cases and combinations.
        let inputs: &[(c_int, c_int, c_int, c_int)] = &[
            (0, 0, 0, 0),
            (10, 20, 30, 40),
            (40, 30, 20, 10),
            (10, 10, 10, 10),
            (20, 20, 20, 20),
            (30, 30, 30, 30),
            (40, 40, 40, 40),
            (1, 2, 3, 4),
            (5, 15, 25, 35),
            (100, 200, 300, 400),
            (-1, -2, -3, -4),
            (0, 10, 0, 30),
            (10, 30, 20, 40),
            (10, 10, 30, 30),
            (i32::MIN, i32::MAX, 0, 1),
            (10, 0, 0, 0),
            (30, 0, 0, 0),
            (20, 0, 0, 0),
            (40, 0, 0, 0),
            (10, 30, 10, 30),
        ];

        for (a, b, c, d) in inputs.iter().copied() {
            let cr = c_fn(a, b, c, d);
            let rr = r_fn(a, b, c, d);
            assert_eq!(
                cr, rr,
                "cleanup({}, {}, {}, {}) mismatch: C={} Rust={}",
                a, b, c, d, cr, rr
            );
        }
    }
}

#[test]
fn print_result_callable_via_both_libs() {
    // We can't easily compare stdout output across libraries within the same
    // process without a lot of plumbing, but we can at least exercise both
    // exported symbols to ensure they exist and don't crash.
    unsafe {
        let (c_lib, r_lib) = load_libs();
        let c_fn: Symbol<PrintResultFn> = c_lib.get(b"print_result").expect("C print_result");
        let r_fn: Symbol<PrintResultFn> = r_lib.get(b"print_result").expect("Rust print_result");

        let label = b"label\0".as_ptr() as *const c_char;
        c_fn(label, 42);
        r_fn(label, 42);
    }
}

#[test]
fn cleanup_resources_handles_null_and_alloc() {
    unsafe {
        let (c_lib, r_lib) = load_libs();
        let c_fn: Symbol<CleanupResourcesFn> =
            c_lib.get(b"cleanup_resources").expect("C cleanup_resources");
        let r_fn: Symbol<CleanupResourcesFn> = r_lib
            .get(b"cleanup_resources")
            .expect("Rust cleanup_resources");

        // Both must be safe to call with a NULL pointer.
        c_fn(std::ptr::null_mut());
        r_fn(std::ptr::null_mut());

        // Both must be safe with a valid heap-allocated pointer (one each;
        // each call frees its own allocation).
        unsafe extern "C" {
            fn malloc(size: usize) -> *mut c_void;
        }
        let p1 = malloc(50) as *mut c_char;
        c_fn(p1);
        let p2 = malloc(50) as *mut c_char;
        r_fn(p2);
    }
}
