use libloading::{Library, Symbol};
use std::path::PathBuf;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn load_c_lib() -> Library {
    let path = project_root().join("c_src/build/libtranslated_rust.so");
    unsafe { Library::new(&path) }.expect("failed to load C .so")
}

fn load_rust_lib() -> Library {
    let path = project_root().join("target/debug/librev16_lib.so");
    unsafe { Library::new(&path) }.expect("failed to load Rust .so")
}

#[test]
fn rev16_matches_c() {
    let c_lib = load_c_lib();
    let rs_lib = load_rust_lib();

    let c_rev16: Symbol<unsafe extern "C" fn(u32) -> u32> =
        unsafe { c_lib.get(b"rev16") }.expect("C rev16 not found");
    let rs_rev16: Symbol<unsafe extern "C" fn(u32) -> u32> =
        unsafe { rs_lib.get(b"rev16") }.expect("Rust rev16 not found");

    let test_values: Vec<u32> = vec![
        0, 1, 0xFFFF, 0xFFFF_FFFF, 0x0001, 0x8000, 0x00FF, 0xFF00,
        0xAAAA, 0x5555, 0xDEAD, 0xBEEF, 0xDEAD_BEEF, 0x1234_5678,
        0x8000_0000, 0x0000_0001, 0x00FF_00FF, 0xFF00_FF00,
        0xA5A5_A5A5, 0x5A5A_5A5A, 0x0F0F_0F0F, 0xF0F0_F0F0,
    ];

    for &val in &test_values {
        let c_result = unsafe { c_rev16(val) };
        let rs_result = unsafe { rs_rev16(val) };
        assert_eq!(
            c_result, rs_result,
            "mismatch for input {:#010X}: C={:#010X} Rust={:#010X}",
            val, c_result, rs_result
        );
    }
}
