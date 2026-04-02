use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so")
}

#[test]
fn test_pow43_matches_c() {
    let lib = unsafe { Library::new(c_lib_path()).expect("Failed to load C library") };
    let c_pow43: Symbol<unsafe extern "C" fn(i32) -> f32> =
        unsafe { lib.get(b"pow43").expect("Failed to find pow43 in C lib") };

    for x in 0..8192 {
        let c_result = unsafe { c_pow43(x) };
        let rust_result = pow43_lib::pow43(x);
        assert_eq!(
            c_result.to_bits(),
            rust_result.to_bits(),
            "pow43({}) mismatch: C={} (bits={:#010x}), Rust={} (bits={:#010x})",
            x, c_result, c_result.to_bits(), rust_result, rust_result.to_bits()
        );
    }
}
