use libloading::{Library, Symbol};
use std::ffi::c_int;

fn c_lib_path() -> String {
    std::env::var("C_LIB_PATH").unwrap_or_else(|_| {
        let manifest = env!("CARGO_MANIFEST_DIR");
        format!("{}/c_src/build/libtranslated_rust.so", manifest)
    })
}

fn rust_lib_path() -> String {
    std::env::var("RUST_LIB_PATH").unwrap_or_else(|_| {
        let manifest = env!("CARGO_MANIFEST_DIR");
        format!("{}/target/debug/libcall_predict_lib.so", manifest)
    })
}

#[test]
fn test_call_predict_all_pfcn() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("Failed to load C .so");
        let rust_lib = Library::new(rust_lib_path()).expect("Failed to load Rust .so");

        let c_fn: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            c_lib.get(b"call_predict").expect("C call_predict not found");
        let rust_fn: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            rust_lib.get(b"call_predict").expect("Rust call_predict not found");

        // Test pfcn 0-11 (should return 1 — pointer matches)
        // Test pfcn 12-20 and negative values (should return 0 — default case)
        let test_values: Vec<c_int> = (-5..=20).collect();

        for pfcn in test_values {
            let c_result = c_fn(pfcn);
            let rust_result = rust_fn(pfcn);
            assert_eq!(
                c_result, rust_result,
                "Mismatch for pfcn={}: C={}, Rust={}",
                pfcn, c_result, rust_result
            );
        }
    }
}
