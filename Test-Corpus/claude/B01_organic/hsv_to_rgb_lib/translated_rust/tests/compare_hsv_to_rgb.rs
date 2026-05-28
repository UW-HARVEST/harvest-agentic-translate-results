use libloading::{Library, Symbol};
use std::path::PathBuf;

type HsvToRgbFn = unsafe extern "C" fn(*mut f32, *const f32);

fn c_lib_path() -> PathBuf {
    // tests/.. -> translated_rust/c_src/build/libtranslated_rust.so
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src");
    p.push("build");
    p.push("libtranslated_rust.so");
    p
}

fn rust_lib_path() -> PathBuf {
    // The cdylib is built into target/<profile>/libhsv_to_rgb_lib.so.
    // Tests are typically built in dev profile. Try both.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for profile in &["debug", "release"] {
        let mut p = manifest.clone();
        p.push("target");
        p.push(profile);
        p.push("libhsv_to_rgb_lib.so");
        if p.exists() {
            return p;
        }
    }
    let mut p = manifest;
    p.push("target");
    p.push("debug");
    p.push("libhsv_to_rgb_lib.so");
    p
}

fn run_pair(input: [f32; 3]) -> ([u32; 3], [u32; 3]) {
    let c_path = c_lib_path();
    let rust_path = rust_lib_path();

    let c_lib = unsafe { Library::new(&c_path).expect("failed to load C .so") };
    let rust_lib = unsafe { Library::new(&rust_path).expect("failed to load Rust .so") };

    let c_fn: Symbol<HsvToRgbFn> = unsafe { c_lib.get(b"hsv_to_rgb").unwrap() };
    let rust_fn: Symbol<HsvToRgbFn> = unsafe { rust_lib.get(b"hsv_to_rgb").unwrap() };

    let mut c_out = [0.0f32; 3];
    let mut rust_out = [0.0f32; 3];
    unsafe {
        c_fn(c_out.as_mut_ptr(), input.as_ptr());
        rust_fn(rust_out.as_mut_ptr(), input.as_ptr());
    }
    let c_bits = [c_out[0].to_bits(), c_out[1].to_bits(), c_out[2].to_bits()];
    let r_bits = [rust_out[0].to_bits(), rust_out[1].to_bits(), rust_out[2].to_bits()];
    (c_bits, r_bits)
}

fn assert_match(input: [f32; 3]) {
    let (c, r) = run_pair(input);
    assert_eq!(c, r, "mismatch for input {:?}: C={:?} Rust={:?}", input, c, r);
}

#[test]
fn s_zero_branch() {
    // s == 0 returns (v, v, v)
    assert_match([0.0, 0.0, 0.0]);
    assert_match([123.0, 0.0, 0.5]);
    assert_match([359.999, 0.0, 1.0]);
    assert_match([-50.0, 0.0, 0.25]);
}

#[test]
fn each_sector_basic() {
    // Hues that fall into each switch sector at i = 0..5
    for h in [30.0, 90.0, 150.0, 210.0, 270.0, 330.0] {
        assert_match([h, 1.0, 1.0]);
        assert_match([h, 0.5, 0.75]);
        assert_match([h, 0.25, 0.1]);
    }
}

#[test]
fn boundaries() {
    // Exact boundaries at 0, 60, 120, 180, 240, 300, 360 to make sure floor/casts agree
    for h in [0.0, 60.0, 120.0, 180.0, 240.0, 300.0, 360.0] {
        assert_match([h, 1.0, 1.0]);
        assert_match([h, 0.7, 0.4]);
    }
}

#[test]
fn out_of_range_hue() {
    // Hues outside [0, 360) — switch default branch handles i >= 5 via default
    for h in [400.0, 500.0, 720.0, 1000.0] {
        assert_match([h, 1.0, 1.0]);
        assert_match([h, 0.3, 0.6]);
    }
    // Negative hues exercise floor() with negative numbers
    for h in [-1.0, -30.0, -60.0, -120.0, -360.0] {
        assert_match([h, 1.0, 1.0]);
        assert_match([h, 0.5, 0.5]);
    }
}

#[test]
fn fuzz_grid() {
    let hues: &[f32] = &[
        -360.0, -180.0, -1.0, 0.0, 0.5, 30.0, 59.999, 60.0, 60.001, 120.0,
        179.5, 200.0, 270.0, 359.0, 360.0, 540.0,
    ];
    let sats: &[f32] = &[0.0, 0.001, 0.1, 0.333333, 0.5, 0.75, 1.0];
    let vals: &[f32] = &[0.0, 0.1, 0.5, 0.99999, 1.0, 2.5];
    for &h in hues {
        for &s in sats {
            for &v in vals {
                assert_match([h, s, v]);
            }
        }
    }
}

#[test]
fn special_values() {
    // NaN/Inf — C and Rust should still exhibit identical bit patterns for the
    // s==0 branch (NaN s won't equal 0). We just want byte-exact agreement.
    // Note: NaN != 0 in both C and Rust IEEE comparisons.
    assert_match([f32::NAN, 0.5, 0.5]);
    assert_match([60.0, f32::NAN, 0.5]);
    assert_match([60.0, 0.5, f32::NAN]);
    assert_match([f32::INFINITY, 0.5, 0.5]);
    // s==0 branch with NaN v -> all three become NaN with same bits
    assert_match([60.0, 0.0, f32::NAN]);
}
