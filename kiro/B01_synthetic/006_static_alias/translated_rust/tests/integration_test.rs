use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libstatic_alias.so")
}

fn rust_lib_path() -> PathBuf {
    // cargo puts cdylib in target/debug/ or target/release/
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target/debug/libstatic_alias.so");
    p
}

/// Test static_alias with a sequence of calls, comparing C vs Rust outputs.
/// Because static_alias uses a static variable, we must test sequences
/// (the static state accumulates across calls).
#[test]
fn test_static_alias_sequence() {
    // We need to build the Rust cdylib first — cargo test builds it as a dependency
    let c_lib = unsafe { Library::new(c_lib_path()).expect("Failed to load C .so") };
    let rust_lib = unsafe { Library::new(rust_lib_path()).expect("Failed to load Rust .so") };

    type StaticAliasFn = unsafe extern "C" fn(*mut i32) -> *mut i32;

    let c_fn: Symbol<StaticAliasFn> =
        unsafe { c_lib.get(b"static_alias").expect("C static_alias not found") };
    let rust_fn: Symbol<StaticAliasFn> =
        unsafe { rust_lib.get(b"static_alias").expect("Rust static_alias not found") };

    // Test case 1: initial_value=5, 5 iterations (like calling driver 5 5)
    {
        let mut c_outer: i32 = 5;
        let mut r_outer: i32 = 5;
        let mut c_ptr: *mut i32 = &mut c_outer;
        let mut r_ptr: *mut i32 = &mut r_outer;

        for i in 0..5 {
            c_ptr = unsafe { c_fn(c_ptr) };
            r_ptr = unsafe { rust_fn(r_ptr) };
            let c_val = unsafe { *c_ptr };
            let r_val = unsafe { *r_ptr };
            assert_eq!(
                c_val, r_val,
                "Mismatch at iteration {} (initial=5): C={}, Rust={}",
                i, c_val, r_val
            );
        }
    }
}

/// Test with initial_value=1, 10 iterations
#[test]
fn test_static_alias_small() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("Failed to load C .so") };
    let rust_lib = unsafe { Library::new(rust_lib_path()).expect("Failed to load Rust .so") };

    type StaticAliasFn = unsafe extern "C" fn(*mut i32) -> *mut i32;

    let c_fn: Symbol<StaticAliasFn> =
        unsafe { c_lib.get(b"static_alias").expect("C static_alias not found") };
    let rust_fn: Symbol<StaticAliasFn> =
        unsafe { rust_lib.get(b"static_alias").expect("Rust static_alias not found") };

    let mut c_outer: i32 = 1;
    let mut r_outer: i32 = 1;
    let mut c_ptr: *mut i32 = &mut c_outer;
    let mut r_ptr: *mut i32 = &mut r_outer;

    for i in 0..10 {
        c_ptr = unsafe { c_fn(c_ptr) };
        r_ptr = unsafe { rust_fn(r_ptr) };
        let c_val = unsafe { *c_ptr };
        let r_val = unsafe { *r_ptr };
        assert_eq!(
            c_val, r_val,
            "Mismatch at iteration {} (initial=1): C={}, Rust={}",
            i, c_val, r_val
        );
    }
}

/// Test with initial_value=0, 5 iterations (tests the else branch first)
#[test]
fn test_static_alias_zero() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("Failed to load C .so") };
    let rust_lib = unsafe { Library::new(rust_lib_path()).expect("Failed to load Rust .so") };

    type StaticAliasFn = unsafe extern "C" fn(*mut i32) -> *mut i32;

    let c_fn: Symbol<StaticAliasFn> =
        unsafe { c_lib.get(b"static_alias").expect("C static_alias not found") };
    let rust_fn: Symbol<StaticAliasFn> =
        unsafe { rust_lib.get(b"static_alias").expect("Rust static_alias not found") };

    let mut c_outer: i32 = 0;
    let mut r_outer: i32 = 0;
    let mut c_ptr: *mut i32 = &mut c_outer;
    let mut r_ptr: *mut i32 = &mut r_outer;

    for i in 0..5 {
        c_ptr = unsafe { c_fn(c_ptr) };
        r_ptr = unsafe { rust_fn(r_ptr) };
        let c_val = unsafe { *c_ptr };
        let r_val = unsafe { *r_ptr };
        assert_eq!(
            c_val, r_val,
            "Mismatch at iteration {} (initial=0): C={}, Rust={}",
            i, c_val, r_val
        );
    }
}
