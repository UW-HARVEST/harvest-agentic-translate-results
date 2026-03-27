use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libmax_size_frame_lib.so")
}

fn rust_lib_path() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // cargo builds cdylib into target/debug or target/release
    manifest.join("target/debug/libmax_size_frame_lib.so")
}

type MaxSizeFrameFn = unsafe extern "C" fn(u32, u32, u32) -> u32;

#[test]
fn test_max_size_frame_c_vs_rust() {
    // Build the Rust cdylib first
    let status = std::process::Command::new("cargo")
        .args(["build", "--lib"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("Failed to run cargo build");
    assert!(status.success(), "cargo build failed");

    let c_lib = unsafe { Library::new(c_lib_path()) }.expect("Failed to load C library");
    let c_fn: Symbol<MaxSizeFrameFn> =
        unsafe { c_lib.get(b"max_size_frame") }.expect("C: max_size_frame not found");

    let rust_lib = unsafe { Library::new(rust_lib_path()) }.expect("Failed to load Rust library");
    let rust_fn: Symbol<MaxSizeFrameFn> =
        unsafe { rust_lib.get(b"max_size_frame") }.expect("Rust: max_size_frame not found");

    let cases: &[(u32, u32, u32)] = &[
        (0, 0, 0),
        (1, 1, 1),
        (1, 2, 16),
        (1, 2, 32),
        (4096, 2, 16),
        (4096, 2, 24),
        (4096, 2, 32),
        (4096, 1, 16),
        (4096, 6, 24),
        (1024, 2, 16),
        (256, 1, 8),
        (8192, 2, 24),
        (u32::MAX, 2, 16),
        (4096, u32::MAX, 16),
        (4096, 2, u32::MAX),
        (0, 2, 16),
        (4096, 0, 16),
        (4096, 2, 0),
        (1, 1, 32),
        (1, 3, 16),
    ];

    for &(blocksize, channels, bitdepth) in cases {
        let c_result = unsafe { c_fn(blocksize, channels, bitdepth) };
        let rust_result = unsafe { rust_fn(blocksize, channels, bitdepth) };
        assert_eq!(
            c_result, rust_result,
            "MISMATCH for blocksize={}, channels={}, bitdepth={}: C={}, Rust={}",
            blocksize, channels, bitdepth, c_result, rust_result
        );
    }
}
