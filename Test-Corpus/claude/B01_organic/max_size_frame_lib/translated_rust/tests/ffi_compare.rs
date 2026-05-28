use libloading::{Library, Symbol};
use std::path::PathBuf;

type MaxSizeFrameFn = unsafe extern "C" fn(u32, u32, u32) -> u32;

fn c_lib_path() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    // The cdylib is placed in target/<profile>/libmax_size_frame_lib.so.
    // Tests run with the same profile that built the artifact, but for
    // safety check both possible locations.
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        manifest_dir.join("target/debug/libmax_size_frame_lib.so"),
        manifest_dir.join("target/release/libmax_size_frame_lib.so"),
    ];
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    candidates[0].clone()
}

fn load_fn(lib: &Library) -> Symbol<MaxSizeFrameFn> {
    unsafe { lib.get(b"max_size_frame").expect("symbol max_size_frame not found") }
}

fn run_compare(blocksize: u32, channels: u32, bitdepth: u32) {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("failed to load C lib") };
    let rust_lib = unsafe { Library::new(rust_lib_path()).expect("failed to load Rust lib") };

    let c_fn = load_fn(&c_lib);
    let rust_fn = load_fn(&rust_lib);

    let c_val = unsafe { c_fn(blocksize, channels, bitdepth) };
    let rust_val = unsafe { rust_fn(blocksize, channels, bitdepth) };
    assert_eq!(
        c_val, rust_val,
        "Mismatch for blocksize={}, channels={}, bitdepth={}: C={}, Rust={}",
        blocksize, channels, bitdepth, c_val, rust_val
    );
}

#[test]
fn test_basic_cases() {
    // Common, plausible FLAC parameter combinations
    let blocksizes: [u32; 6] = [1, 16, 192, 4096, 16384, 65535];
    let channels_list: [u32; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    let bitdepths: [u32; 6] = [8, 12, 16, 20, 24, 32];

    for &b in &blocksizes {
        for &c in &channels_list {
            for &d in &bitdepths {
                run_compare(b, c, d);
            }
        }
    }
}

#[test]
fn test_zero_inputs() {
    run_compare(0, 0, 0);
    run_compare(0, 1, 16);
    run_compare(1, 0, 16);
    run_compare(1, 1, 0);
}

#[test]
fn test_channels_two_special_case() {
    // channels==2 has special handling vs channels!=2
    for &b in &[1u32, 100, 4096, 65535] {
        for &d in &[1u32, 8, 16, 24, 32, 33, 64] {
            run_compare(b, 2, d);
        }
    }
}

#[test]
fn test_bitdepth_32_special_case() {
    // bitdepth==32 vs bitdepth!=32 only matters when channels==2
    run_compare(4096, 2, 32);
    run_compare(4096, 2, 31);
    run_compare(4096, 2, 33);
    run_compare(4096, 1, 32);
}

#[test]
fn test_large_values() {
    // Max u32 inputs to exercise wrapping arithmetic
    run_compare(u32::MAX, 1, 1);
    run_compare(u32::MAX, 2, 1);
    run_compare(u32::MAX, 2, 32);
    run_compare(1, u32::MAX, 1);
    run_compare(1, 1, u32::MAX);
    run_compare(u32::MAX, u32::MAX, u32::MAX);
}

#[test]
fn test_random_combinations() {
    // Deterministic pseudo-random sweep using a simple LCG
    let mut state: u32 = 0x1234_5678;
    for _ in 0..2000 {
        state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        let b = state & 0xFFFF;
        state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        let c = (state >> 8) & 0x3F;
        state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        let d = (state >> 16) & 0x3F;
        run_compare(b, c, d);
    }
}
