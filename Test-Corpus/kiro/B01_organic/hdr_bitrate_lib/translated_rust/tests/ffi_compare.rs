use libloading::{Library, Symbol};
use std::os::raw::c_uint;
use std::path::PathBuf;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn load_libs() -> (Library, Library) {
    let c_so = project_root().join("c_src/build/libtranslated_rust.so");
    let rust_so = project_root().join("target/debug/libhdr_bitrate_lib.so");
    unsafe {
        (
            Library::new(&c_so).expect("failed to load C .so"),
            Library::new(&rust_so).expect("failed to load Rust .so"),
        )
    }
}

#[test]
fn test_hdr_bitrate_exhaustive() {
    let (c_lib, rust_lib) = load_libs();
    type HdrBitrate = unsafe extern "C" fn(*const u8) -> c_uint;
    let c_fn: Symbol<HdrBitrate> = unsafe { c_lib.get(b"hdr_bitrate").unwrap() };
    let r_fn: Symbol<HdrBitrate> = unsafe { rust_lib.get(b"hdr_bitrate").unwrap() };

    // h[1] bits: bit3 = version (0 or 1), bits[2:1] = layer (1-3 valid, 0 would underflow)
    // h[2] bits: bits[7:4] = bitrate_index (0-15, but table only has 15 entries so 0-14)
    for version_bit in 0u8..=1 {
        for layer in 1u8..=3 {
            for br_index in 0u8..=14 {
                let h: [u8; 4] = [
                    0,                                    // h[0] unused
                    (version_bit << 3) | (layer << 1),    // h[1]
                    br_index << 4,                        // h[2]
                    0,                                    // padding
                ];
                let c_val = unsafe { c_fn(h.as_ptr()) };
                let r_val = unsafe { r_fn(h.as_ptr()) };
                assert_eq!(
                    c_val, r_val,
                    "mismatch: version_bit={}, layer={}, br_index={}: C={}, Rust={}",
                    version_bit, layer, br_index, c_val, r_val
                );
            }
        }
    }
}
