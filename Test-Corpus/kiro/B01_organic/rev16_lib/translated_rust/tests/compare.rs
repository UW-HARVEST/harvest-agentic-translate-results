use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/librev16_lib.so")
}

#[test]
fn test_rev16_matches_c() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C .so") };
    let c_rev16: Symbol<unsafe extern "C" fn(u32) -> u32> =
        unsafe { c_lib.get(b"rev16").expect("find rev16 in C .so") };

    // Test a range of values including edge cases
    let test_values: Vec<u32> = vec![
        0, 1, 0xFFFF, 0xFFFFFFFF, 0xAAAA, 0x5555, 0xDEAD, 0xBEEF,
        0x1234, 0x8000, 0x0001, 0x00FF, 0xFF00, 42, 255, 256, 65535,
    ];

    for &val in &test_values {
        let c_result = unsafe { c_rev16(val) };
        let rust_result = rev16_lib::rev16(val);
        assert_eq!(
            c_result, rust_result,
            "Mismatch for rev16(0x{:08X}): C=0x{:08X}, Rust=0x{:08X}",
            val, c_result, rust_result
        );
    }
}
