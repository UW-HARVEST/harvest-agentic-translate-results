use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // cargo builds cdylib into target/<profile>/
    p.push("target/debug/libget_predict_func_lib.so");
    if !p.exists() {
        p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/release/libget_predict_func_lib.so");
    }
    p
}

#[test]
fn test_get_predict_func_matches_c() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("failed to load C .so");
        let rust_lib = Library::new(rust_lib_path()).expect("failed to load Rust .so");

        let c_fn: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            c_lib.get(b"get_predict_func").expect("C symbol not found");
        let r_fn: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            rust_lib.get(b"get_predict_func").expect("Rust symbol not found");

        // Test pfcn values: valid range 0-15, plus edge cases
        for pfcn in -5..=20 {
            let c_result = c_fn(pfcn);
            let r_result = r_fn(pfcn);
            assert_eq!(
                c_result, r_result,
                "mismatch for pfcn={pfcn}: C={c_result}, Rust={r_result}"
            );
        }
    }
}
