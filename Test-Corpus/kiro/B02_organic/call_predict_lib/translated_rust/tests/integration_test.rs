use libloading::{Library, Symbol};
use std::os::raw::c_int;

fn c_lib_path() -> std::path::PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    std::path::PathBuf::from(manifest).join("c_src/build/libcall_predict_lib.so")
}

fn rust_lib_path() -> std::path::PathBuf {
    // cdylib output goes to target/debug/ or target/release/
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let base = std::path::PathBuf::from(manifest).join("target/debug");
    base.join("libcall_predict_lib.so")
}

#[test]
fn test_call_predict_matches_c() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("Failed to load C .so");
        let r_lib = Library::new(rust_lib_path()).expect("Failed to load Rust .so");

        let c_fn: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            c_lib.get(b"call_predict").expect("C: call_predict not found");
        let r_fn: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            r_lib.get(b"call_predict").expect("Rust: call_predict not found");

        for pfcn in -1..=100 {
            let c_result = c_fn(pfcn);
            let rust_result = r_fn(pfcn);
            assert_eq!(
                c_result, rust_result,
                "call_predict({pfcn}): C={c_result}, Rust={rust_result}"
            );
        }
    }
}

#[test]
fn test_get_predict_func_wrapper() {
    unsafe {
        let r_lib = Library::new(rust_lib_path()).expect("Failed to load Rust .so");
        let r_call: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            r_lib.get(b"call_predict").expect("Rust: call_predict not found");
        let r_get: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            r_lib.get(b"get_predict_func").expect("Rust: get_predict_func not found");

        for pfcn in -1..=20 {
            let cp = r_call(pfcn);
            let gpf = r_get(pfcn);
            assert_eq!(cp, gpf, "get_predict_func({pfcn})={gpf} != call_predict({pfcn})={cp}");
        }
    }
}
