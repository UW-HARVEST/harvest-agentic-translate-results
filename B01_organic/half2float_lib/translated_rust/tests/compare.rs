use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so")
}

#[test]
fn test_half2float_exhaustive() {
    let lib = unsafe { Library::new(c_lib_path()) }.expect("Failed to load C .so");
    let c_half2float: Symbol<unsafe extern "C" fn(u16) -> f32> =
        unsafe { lib.get(b"half2float") }.expect("Failed to find half2float in C .so");

    let mut mismatches = Vec::new();
    for h in 0u16..=u16::MAX {
        let c_result = unsafe { c_half2float(h) };
        let rust_result = half2float_lib::half2float(h);
        if c_result.to_bits() != rust_result.to_bits() {
            mismatches.push((h, c_result.to_bits(), rust_result.to_bits()));
            if mismatches.len() >= 20 {
                break;
            }
        }
    }
    if !mismatches.is_empty() {
        for (h, c_bits, r_bits) in &mismatches {
            eprintln!(
                "MISMATCH h=0x{:04x}: C=0x{:08x} Rust=0x{:08x}",
                h, c_bits, r_bits
            );
        }
        panic!("{} mismatches found (showing up to 20)", mismatches.len());
    }
}
