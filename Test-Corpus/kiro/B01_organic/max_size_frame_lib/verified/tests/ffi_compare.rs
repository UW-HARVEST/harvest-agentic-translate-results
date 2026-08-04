use libloading::{Library, Symbol};
use std::path::PathBuf;

type MaxSizeFrameFn = unsafe extern "C" fn(u32, u32, u32) -> u32;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn load_c_lib() -> Library {
    let path = project_root().join("c_src/build/libtranslated_rust.so");
    unsafe { Library::new(&path) }.expect("Failed to load C .so")
}

fn load_rust_lib() -> Library {
    let path = project_root().join("target/debug/libmax_size_frame_lib.so");
    unsafe { Library::new(&path) }.expect("Failed to load Rust .so")
}

#[test]
fn test_max_size_frame_matches() {
    let c_lib = load_c_lib();
    let rs_lib = load_rust_lib();

    let c_fn: Symbol<MaxSizeFrameFn> = unsafe { c_lib.get(b"max_size_frame") }.unwrap();
    let rs_fn: Symbol<MaxSizeFrameFn> = unsafe { rs_lib.get(b"max_size_frame") }.unwrap();

    // Test cases: (blocksize, channels, bitdepth)
    let cases: &[(u32, u32, u32)] = &[
        // Basic cases
        (0, 0, 0),
        (1, 1, 1),
        (1, 2, 16),
        (1, 2, 32),
        (4096, 2, 16),
        (4096, 2, 24),
        (4096, 2, 32),
        (4096, 1, 16),
        (4096, 6, 24),
        // Edge cases
        (0, 2, 16),
        (1, 0, 16),
        (1, 1, 0),
        (u32::MAX, 1, 1),
        (u32::MAX, 2, 32),
        (u32::MAX, u32::MAX, u32::MAX),
        (1, u32::MAX, 1),
        (1, 1, u32::MAX),
        // Typical FLAC parameters
        (256, 2, 16),
        (512, 2, 16),
        (1024, 2, 16),
        (2048, 2, 24),
        (4608, 2, 16),
        (4608, 6, 24),
        (8192, 8, 24),
        // channels == 2 vs != 2 boundary
        (100, 1, 16),
        (100, 2, 16),
        (100, 3, 16),
        // bitdepth == 32 vs != 32 boundary
        (100, 2, 31),
        (100, 2, 32),
        (100, 2, 33),
    ];

    for &(bs, ch, bd) in cases {
        let c_val = unsafe { c_fn(bs, ch, bd) };
        let rs_val = unsafe { rs_fn(bs, ch, bd) };
        assert_eq!(
            c_val, rs_val,
            "MISMATCH for blocksize={bs}, channels={ch}, bitdepth={bd}: C={c_val}, Rust={rs_val}"
        );
    }
}
