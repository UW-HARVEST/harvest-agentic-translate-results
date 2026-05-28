// Integration test that loads BOTH the C .so and the Rust .so via libloading
// and compares their behavior through the FFI boundary.
//
// The only public C API is `void arr_ins(int num)`. It mutates no observable
// state (no globals, no allocations leak, no return value). It internally
// asserts certain invariants. So the only way to compare behavior is to call
// both implementations with a representative set of inputs and ensure neither
// panics/aborts and both return normally.

use libloading::{Library, Symbol};
use std::path::PathBuf;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    manifest_dir().join("c_src/build/libtranslated_rust.so")
}

fn rust_so_path() -> PathBuf {
    // The Rust crate is built with crate-type = ["cdylib"] and lib.name =
    // "arr_ins_lib", so the produced shared library is libarr_ins_lib.so.
    manifest_dir().join("target/release/libarr_ins_lib.so")
}

unsafe fn load_arr_ins(lib: &Library) -> Symbol<unsafe extern "C" fn(i32)> {
    unsafe { lib.get(b"arr_ins").expect("arr_ins symbol not found") }
}

#[test]
fn arr_ins_matches_c() {
    // Inputs to exercise: cover the documented behaviour (insert at indices
    // 0..5 of [1,2,3,4]) for a variety of numeric values, including edge
    // cases (i32::MIN, i32::MAX, 0, ±small numbers).
    let inputs = [
        0i32,
        1,
        -1,
        4,
        100,
        -100,
        i32::MIN,
        i32::MAX,
        i32::MIN + 1,
        i32::MAX - 1,
        12345,
        -12345,
    ];

    let c_lib = unsafe { Library::new(c_so_path()).expect("failed to load C .so") };
    let rust_lib = unsafe { Library::new(rust_so_path()).expect("failed to load Rust .so") };

    let c_arr_ins = unsafe { load_arr_ins(&c_lib) };
    let rust_arr_ins = unsafe { load_arr_ins(&rust_lib) };

    for &num in &inputs {
        // Both should return normally without panicking. If either panics
        // (or aborts via assert), this test fails immediately. The C
        // version's STBDS_ASSERT will abort the process if violated; Vec's
        // assert! in Rust panics. Both indicate a mismatch from expected
        // behaviour.
        unsafe { c_arr_ins(num) };
        unsafe { rust_arr_ins(num) };
    }
}
