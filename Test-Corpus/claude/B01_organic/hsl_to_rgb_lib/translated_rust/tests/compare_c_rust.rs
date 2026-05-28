use libloading::{Library, Symbol};
use std::path::PathBuf;

type HslToRgbFn = unsafe extern "C" fn(*mut f32, *const f32);

fn c_lib_path() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    // Find the cdylib that cargo built. Default is target/release or target/debug
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Try debug first (since `cargo test` uses debug profile)
    let debug = manifest_dir.join("target/debug/libhsl_to_rgb_lib.so");
    if debug.exists() {
        return debug;
    }
    let release = manifest_dir.join("target/release/libhsl_to_rgb_lib.so");
    if release.exists() {
        return release;
    }
    panic!(
        "could not find Rust .so at {:?} or {:?}",
        debug, release
    );
}

fn run_one(h: f32, s: f32, l: f32) -> ([f32; 3], [f32; 3]) {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("failed to load C lib");
        let rust_lib = Library::new(rust_lib_path()).expect("failed to load Rust lib");

        let c_fn: Symbol<HslToRgbFn> = c_lib.get(b"hsl_to_rgb").expect("no hsl_to_rgb in C lib");
        let rust_fn: Symbol<HslToRgbFn> =
            rust_lib.get(b"hsl_to_rgb").expect("no hsl_to_rgb in Rust lib");

        let src: [f32; 3] = [h, s, l];
        let mut c_out: [f32; 3] = [0.0; 3];
        let mut rust_out: [f32; 3] = [0.0; 3];

        c_fn(c_out.as_mut_ptr(), src.as_ptr());
        rust_fn(rust_out.as_mut_ptr(), src.as_ptr());

        (c_out, rust_out)
    }
}

fn assert_bitwise_eq(c: [f32; 3], r: [f32; 3], h: f32, s: f32, l: f32) {
    for i in 0..3 {
        let cb = c[i].to_bits();
        let rb = r[i].to_bits();
        assert_eq!(
            cb, rb,
            "mismatch at index {} for input (h={}, s={}, l={}): c={} (0x{:08x}) rust={} (0x{:08x})",
            i, h, s, l, c[i], cb, r[i], rb
        );
    }
}

#[test]
fn test_zero_saturation() {
    for &l in &[0.0f32, 0.25, 0.5, 0.75, 1.0] {
        let (c, r) = run_one(0.0, 0.0, l);
        assert_bitwise_eq(c, r, 0.0, 0.0, l);
        let (c, r) = run_one(180.0, 0.0, l);
        assert_bitwise_eq(c, r, 180.0, 0.0, l);
    }
}

#[test]
fn test_hue_sweep() {
    // Sweep hues through all branches: 0..360 plus boundaries and out-of-range.
    let hues: &[f32] = &[
        -10.0, 0.0, 30.0, 59.999, 60.0, 90.0, 119.999, 120.0, 150.0, 179.999, 180.0, 210.0,
        239.999, 240.0, 270.0, 299.999, 300.0, 330.0, 359.999, 360.0, 400.0,
    ];
    let sats: &[f32] = &[0.1, 0.5, 1.0];
    let lits: &[f32] = &[0.1, 0.3, 0.5, 0.7, 0.9];

    for &h in hues {
        for &s in sats {
            for &l in lits {
                let (c, r) = run_one(h, s, l);
                assert_bitwise_eq(c, r, h, s, l);
            }
        }
    }
}

#[test]
fn test_pure_colors() {
    // Pure red, green, blue, etc.
    let cases = &[
        (0.0f32, 1.0f32, 0.5f32),   // red
        (60.0, 1.0, 0.5),           // yellow
        (120.0, 1.0, 0.5),          // green
        (180.0, 1.0, 0.5),          // cyan
        (240.0, 1.0, 0.5),          // blue
        (300.0, 1.0, 0.5),          // magenta
    ];
    for &(h, s, l) in cases {
        let (c, r) = run_one(h, s, l);
        assert_bitwise_eq(c, r, h, s, l);
    }
}

#[test]
fn test_edge_extremes() {
    // l=0 and l=1 with non-zero saturation
    for &h in &[0.0f32, 90.0, 200.0, 350.0] {
        for &s in &[0.5f32, 1.0] {
            for &l in &[0.0f32, 1.0] {
                let (c, r) = run_one(h, s, l);
                assert_bitwise_eq(c, r, h, s, l);
            }
        }
    }
}
