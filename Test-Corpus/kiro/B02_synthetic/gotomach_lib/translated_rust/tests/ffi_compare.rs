use libloading::{Library, Symbol};
use std::os::raw::c_int;
use std::ffi::c_void;

const C_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libtranslated_rust.so");

fn rust_lib_path() -> String {
    // Find the Rust .so in target/debug
    let manifest = env!("CARGO_MANIFEST_DIR");
    format!("{}/target/debug/libgotomach_lib.so", manifest)
}

type OpFn = unsafe extern "C" fn(c_int, c_int, *mut c_void) -> c_int;
type GotomachFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

// --- Low-level operation functions ---

#[test]
fn test_process_value() {
    unsafe {
        let c = Library::new(C_LIB).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<OpFn> = c.get(b"process_value").unwrap();
        let r_fn: Symbol<OpFn> = r.get(b"process_value").unwrap();
        for v in [-100, -1, 0, 1, 5, 100, 1000, i32::MAX / 2] {
            let c_res = c_fn(v, 0, std::ptr::null_mut());
            let r_res = r_fn(v, 0, std::ptr::null_mut());
            assert_eq!(c_res, r_res, "process_value({v}) mismatch: C={c_res} Rust={r_res}");
        }
    }
}

#[test]
fn test_double_value() {
    unsafe {
        let c = Library::new(C_LIB).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<OpFn> = c.get(b"double_value").unwrap();
        let r_fn: Symbol<OpFn> = r.get(b"double_value").unwrap();
        for v in [-50, -1, 0, 1, 7, 500, 10000] {
            let c_res = c_fn(v, 0, std::ptr::null_mut());
            let r_res = r_fn(v, 0, std::ptr::null_mut());
            assert_eq!(c_res, r_res, "double_value({v}) mismatch: C={c_res} Rust={r_res}");
        }
    }
}

#[test]
fn test_triple_value() {
    unsafe {
        let c = Library::new(C_LIB).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<OpFn> = c.get(b"triple_value").unwrap();
        let r_fn: Symbol<OpFn> = r.get(b"triple_value").unwrap();
        for v in [-33, -1, 0, 1, 3, 333, 9999] {
            let c_res = c_fn(v, 0, std::ptr::null_mut());
            let r_res = r_fn(v, 0, std::ptr::null_mut());
            assert_eq!(c_res, r_res, "triple_value({v}) mismatch: C={c_res} Rust={r_res}");
        }
    }
}

// --- High-level gotomach function ---

#[test]
fn test_gotomach_error_cases() {
    unsafe {
        let c = Library::new(C_LIB).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<GotomachFn> = c.get(b"gotomach").unwrap();
        let r_fn: Symbol<GotomachFn> = r.get(b"gotomach").unwrap();

        // Negative iterations
        assert_eq!(c_fn(-1, 0, 0, 0), r_fn(-1, 0, 0, 0), "negative iterations");
        // iterations > UINT16_MAX
        assert_eq!(c_fn(70000, 0, 0, 0), r_fn(70000, 0, 0, 0), "iterations > 65535");
        // Negative seed
        assert_eq!(c_fn(1, -1, 0, 0), r_fn(1, -1, 0, 0), "negative seed");
        // seed > UINT16_MAX
        assert_eq!(c_fn(1, 70000, 0, 0), r_fn(1, 70000, 0, 0), "seed > 65535");
        // Zero iterations (edge case - valid)
        assert_eq!(c_fn(0, 0, 0, 0), r_fn(0, 0, 0, 0), "zero iterations");
    }
}

#[test]
fn test_gotomach_modes() {
    unsafe {
        let c = Library::new(C_LIB).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<GotomachFn> = c.get(b"gotomach").unwrap();
        let r_fn: Symbol<GotomachFn> = r.get(b"gotomach").unwrap();

        // Test all modes with various inputs
        for mode in 0..=3 {
            for &(iters, seed, thresh) in &[
                (1, 0, 100),
                (5, 10, 50),
                (10, 100, 500),
                (20, 50, 200),
                (100, 1, 1000),
                (50, 999, 100),
                (10, 0, 0),       // threshold 0 means nothing passes
                (10, 5, 100000),  // high threshold means everything passes
            ] {
                let c_res = c_fn(iters, seed, mode, thresh);
                let r_res = r_fn(iters, seed, mode, thresh);
                assert_eq!(
                    c_res, r_res,
                    "gotomach({iters}, {seed}, {mode}, {thresh}) mismatch: C={c_res} Rust={r_res}"
                );
            }
        }
    }
}

#[test]
fn test_gotomach_boundary_values() {
    unsafe {
        let c = Library::new(C_LIB).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<GotomachFn> = c.get(b"gotomach").unwrap();
        let r_fn: Symbol<GotomachFn> = r.get(b"gotomach").unwrap();

        // Max valid iterations/seed
        for &(iters, seed, mode, thresh) in &[
            (65535, 0, 0, 100),
            (1, 65535, 1, 100000),
            (100, 500, 2, 1500),
        ] {
            let c_res = c_fn(iters, seed, mode, thresh);
            let r_res = r_fn(iters, seed, mode, thresh);
            assert_eq!(
                c_res, r_res,
                "gotomach({iters}, {seed}, {mode}, {thresh}) mismatch: C={c_res} Rust={r_res}"
            );
        }
    }
}
