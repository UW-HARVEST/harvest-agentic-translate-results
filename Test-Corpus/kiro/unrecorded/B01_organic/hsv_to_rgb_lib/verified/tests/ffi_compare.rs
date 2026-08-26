use libloading::{Library, Symbol};
use std::path::PathBuf;

type HsvToRgb = unsafe extern "C" fn(*mut f32, *const f32);

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn load_c_lib() -> Library {
    let path = project_root().join("c_src/build/libtranslated_rust.so");
    unsafe { Library::new(&path).expect("failed to load C .so") }
}

fn load_rust_lib() -> Library {
    let path = project_root().join("target/debug/libhsv_to_rgb_lib.so");
    unsafe { Library::new(&path).expect("failed to load Rust .so") }
}

fn call_hsv_to_rgb(lib: &Library, hsv: &[f32; 3]) -> [f32; 3] {
    unsafe {
        let func: Symbol<HsvToRgb> = lib.get(b"hsv_to_rgb").unwrap();
        let mut rgb = [0.0f32; 3];
        func(rgb.as_mut_ptr(), hsv.as_ptr());
        rgb
    }
}

#[test]
fn hsv_to_rgb_matches() {
    let c_lib = load_c_lib();
    let rust_lib = load_rust_lib();

    // Test cases: (h, s, v) covering all switch branches and edge cases
    let cases: &[[f32; 3]] = &[
        // s == 0 (grayscale)
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0],
        [180.0, 0.0, 0.5],
        // i == 0 (h in [0, 60))
        [0.0, 1.0, 1.0],
        [30.0, 0.5, 0.8],
        [59.0, 1.0, 1.0],
        // i == 1 (h in [60, 120))
        [60.0, 1.0, 1.0],
        [90.0, 0.5, 0.8],
        // i == 2 (h in [120, 180))
        [120.0, 1.0, 1.0],
        [150.0, 0.5, 0.8],
        // i == 3 (h in [180, 240))
        [180.0, 1.0, 1.0],
        [210.0, 0.5, 0.8],
        // i == 4 (h in [240, 300))
        [240.0, 1.0, 1.0],
        [270.0, 0.5, 0.8],
        // i == 5 / default (h in [300, 360))
        [300.0, 1.0, 1.0],
        [330.0, 0.5, 0.8],
        [359.0, 1.0, 1.0],
        // boundary / extreme values
        [0.0, 1.0, 0.0],
        [360.0, 1.0, 1.0],
        [0.0, 0.01, 0.99],
        [123.456, 0.789, 0.321],
    ];

    for (idx, hsv) in cases.iter().enumerate() {
        let c_rgb = call_hsv_to_rgb(&c_lib, hsv);
        let r_rgb = call_hsv_to_rgb(&rust_lib, hsv);
        assert_eq!(
            c_rgb.map(f32::to_bits),
            r_rgb.map(f32::to_bits),
            "Mismatch at case {idx}: hsv={hsv:?} c_rgb={c_rgb:?} rust_rgb={r_rgb:?}"
        );
    }
}
