use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so")
}

fn call_c_hsl_to_rgb(lib: &Library, src: &[f32; 3]) -> [f32; 3] {
    unsafe {
        let func: Symbol<unsafe extern "C" fn(*mut f32, *const f32)> =
            lib.get(b"hsl_to_rgb").unwrap();
        let mut dest = [0.0f32; 3];
        func(dest.as_mut_ptr(), src.as_ptr());
        dest
    }
}

fn call_rust_hsl_to_rgb(src: &[f32; 3]) -> [f32; 3] {
    let mut dest = [0.0f32; 3];
    hsl_to_rgb_lib::hsl_to_rgb(dest.as_mut_ptr(), src.as_ptr());
    dest
}

/// Test inputs covering every branch of the HSL-to-RGB conversion:
/// - s==0 (achromatic)
/// - h in [0,60), [60,120), [120,180), [180,240), [240,300), [300,360)
/// - h < 0 and h >= 360 (else branch)
/// - edge values
const TEST_INPUTS: &[[f32; 3]] = &[
    // achromatic
    [0.0, 0.0, 0.5],
    [180.0, 0.0, 0.0],
    [180.0, 0.0, 1.0],
    // h in [0, 60)
    [0.0, 1.0, 0.5],
    [30.0, 0.5, 0.25],
    [59.9, 1.0, 0.5],
    // h in [60, 120)
    [60.0, 1.0, 0.5],
    [90.0, 0.5, 0.75],
    [119.9, 1.0, 0.5],
    // h in [120, 180)
    [120.0, 1.0, 0.5],
    [150.0, 0.5, 0.25],
    [179.9, 1.0, 0.5],
    // h in [180, 240)
    [180.0, 1.0, 0.5],
    [210.0, 0.5, 0.75],
    [239.9, 1.0, 0.5],
    // h in [240, 300)
    [240.0, 1.0, 0.5],
    [270.0, 0.5, 0.25],
    [299.9, 1.0, 0.5],
    // h in [300, 360)
    [300.0, 1.0, 0.5],
    [330.0, 0.5, 0.75],
    [359.9, 1.0, 0.5],
    // else branch
    [-1.0, 1.0, 0.5],
    [360.0, 1.0, 0.5],
    [400.0, 0.8, 0.3],
];

#[test]
fn test_hsl_to_rgb_matches_c() {
    let lib = unsafe { Library::new(c_lib_path()).expect("Failed to load C library") };

    for (i, input) in TEST_INPUTS.iter().enumerate() {
        let c_out = call_c_hsl_to_rgb(&lib, input);
        let rust_out = call_rust_hsl_to_rgb(input);
        assert_eq!(
            c_out.map(f32::to_bits),
            rust_out.map(f32::to_bits),
            "Mismatch at test case {i}: input={input:?}\n  C={c_out:?}\n  Rust={rust_out:?}"
        );
    }
}
