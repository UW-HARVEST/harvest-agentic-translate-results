use libloading::{Library, Symbol};
use std::os::raw::c_uint;

#[test]
fn test_hdr_bitrate_c_vs_rust() {
    let c_lib = unsafe {
        Library::new(concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libtranslated_rust.so"))
    }
    .expect("Failed to load C .so");

    let c_fn: Symbol<unsafe extern "C" fn(*const u8) -> c_uint> =
        unsafe { c_lib.get(b"hdr_bitrate") }.expect("Failed to find hdr_bitrate in C .so");

    let rust_lib = unsafe {
        Library::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("target/debug/libhdr_bitrate_lib.so"),
        )
    }
    .expect("Failed to load Rust .so");

    let rust_fn: Symbol<unsafe extern "C" fn(*const u8) -> c_uint> =
        unsafe { rust_lib.get(b"hdr_bitrate") }.expect("Failed to find hdr_bitrate in Rust .so");

    let mut mismatches = Vec::new();
    let mut tested = 0u32;

    for h1 in 0u8..=255 {
        let layer_idx = (h1 >> 1) & 3;
        if layer_idx == 0 {
            continue;
        }
        for h2_hi in 0u8..15 {
            let h2 = h2_hi << 4;
            let input: [u8; 3] = [0, h1, h2];

            let c_result = unsafe { c_fn(input.as_ptr()) };
            let rust_result = unsafe { rust_fn(input.as_ptr()) };

            if c_result != rust_result {
                mismatches.push((h1, h2, c_result, rust_result));
            }
            tested += 1;
        }
    }

    if !mismatches.is_empty() {
        for (h1, h2, c_val, r_val) in &mismatches[..mismatches.len().min(20)] {
            eprintln!(
                "MISMATCH h=[0x00, 0x{:02x}, 0x{:02x}]: C={}, Rust={}",
                h1, h2, c_val, r_val
            );
        }
        panic!(
            "{} mismatches out of {} tests (showing first {})",
            mismatches.len(),
            tested,
            mismatches.len().min(20)
        );
    }
    eprintln!("All {} test cases passed", tested);
}
