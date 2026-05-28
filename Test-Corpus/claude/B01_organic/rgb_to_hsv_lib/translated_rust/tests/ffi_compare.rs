use libloading::{Library, Symbol};
use std::path::PathBuf;

type RgbToHsvFn = unsafe extern "C" fn(*mut f32, *const f32);

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    // Walk to the built cdylib (same crate). We expect debug build.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        manifest.join("target/debug/librgb_to_hsv_lib.so"),
        manifest.join("target/release/librgb_to_hsv_lib.so"),
    ];
    for p in &candidates {
        if p.exists() {
            return p.clone();
        }
    }
    panic!(
        "Could not find Rust cdylib at {:?}. Run `cargo build` first.",
        candidates
    );
}

unsafe fn call(lib: &Library, src: [f32; 3]) -> [f32; 3] {
    let func: Symbol<RgbToHsvFn> = lib.get(b"rgb_to_hsv").expect("missing rgb_to_hsv");
    let mut dest = [0.0f32; 3];
    func(dest.as_mut_ptr(), src.as_ptr());
    dest
}

fn assert_bytes_eq(a: [f32; 3], b: [f32; 3], inp: [f32; 3]) {
    let ab = a.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
    let bb = b.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
    assert_eq!(
        ab, bb,
        "input={:?}, c={:?}, rust={:?}",
        inp, a, b
    );
}

fn run_cases(cases: &[[f32; 3]]) {
    let c_lib = unsafe { Library::new(c_lib_path()) }.expect("load C .so");
    let r_lib = unsafe { Library::new(rust_lib_path()) }.expect("load Rust .so");
    for &inp in cases {
        let c_out = unsafe { call(&c_lib, inp) };
        let r_out = unsafe { call(&r_lib, inp) };
        assert_bytes_eq(c_out, r_out, inp);
    }
}

#[test]
fn test_basic_colors() {
    let cases = [
        [0.0f32, 0.0, 0.0],     // black
        [1.0, 1.0, 1.0],        // white
        [1.0, 0.0, 0.0],        // pure red
        [0.0, 1.0, 0.0],        // pure green
        [0.0, 0.0, 1.0],        // pure blue
        [1.0, 1.0, 0.0],        // yellow
        [1.0, 0.0, 1.0],        // magenta
        [0.0, 1.0, 1.0],        // cyan
        [0.5, 0.5, 0.5],        // gray
        [0.25, 0.75, 0.5],
        [0.9, 0.1, 0.1],
        [0.1, 0.9, 0.1],
        [0.1, 0.1, 0.9],
    ];
    run_cases(&cases);
}

#[test]
fn test_extreme_values() {
    let cases = [
        [f32::MIN_POSITIVE, 0.0, 0.0],
        [1e-30, 1e-30, 1e-30],
        [1e10, 5e9, 0.0],
        [-1.0, 0.5, 0.5],     // negative inputs (still well-defined per code)
        [-0.5, -0.25, -0.75],
        [0.0, 0.0, f32::MIN_POSITIVE],
    ];
    run_cases(&cases);
}

#[test]
fn test_random_values() {
    // Deterministic LCG for reproducibility
    let mut state: u64 = 0x123456789abcdef0;
    let mut next = || {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((state >> 33) as u32) as f32 / (u32::MAX as f32)
    };
    let mut cases = Vec::new();
    for _ in 0..1000 {
        cases.push([next(), next(), next()]);
    }
    run_cases(&cases);
}

#[test]
fn test_two_equal_max() {
    // r == max && g == max but b is smaller -> should pick r branch
    let cases = [
        [1.0, 1.0, 0.0],
        [0.5, 0.5, 0.25],
        [1.0, 0.0, 1.0],   // r==b==max
        [0.0, 1.0, 1.0],   // g==b==max
        [0.7, 0.7, 0.7],
    ];
    run_cases(&cases);
}
