use libloading::{Library, Symbol};
use std::path::PathBuf;

type HslToRgbFn = unsafe extern "C" fn(*mut f32, *const f32);

fn load_libs() -> (Library, Library) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_so = manifest.join("c_src/build/libtranslated_rust.so");
    let rust_so = manifest.join("target/debug/libhsl_to_rgb_lib.so");
    unsafe {
        (
            Library::new(&c_so).expect("Failed to load C .so"),
            Library::new(&rust_so).expect("Failed to load Rust .so"),
        )
    }
}

fn call_hsl_to_rgb(lib: &Library, h: f32, s: f32, l: f32) -> [f32; 3] {
    unsafe {
        let func: Symbol<HslToRgbFn> = lib.get(b"hsl_to_rgb").unwrap();
        let src = [h, s, l];
        let mut dest = [0.0f32; 3];
        func(dest.as_mut_ptr(), src.as_ptr());
        dest
    }
}

fn compare(h: f32, s: f32, l: f32, c_lib: &Library, rs_lib: &Library) {
    let c_out = call_hsl_to_rgb(c_lib, h, s, l);
    let rs_out = call_hsl_to_rgb(rs_lib, h, s, l);
    assert_eq!(
        c_out.map(f32::to_bits),
        rs_out.map(f32::to_bits),
        "Mismatch at h={h} s={s} l={l}: C={c_out:?} Rust={rs_out:?}"
    );
}

#[test]
fn test_hsl_to_rgb_branch_boundaries() {
    let (c_lib, rs_lib) = load_libs();
    // Test each branch boundary and midpoint
    let hues = [
        0.0, 30.0, 59.99, 60.0, 90.0, 119.99, 120.0, 150.0, 179.99,
        180.0, 210.0, 239.99, 240.0, 270.0, 299.99, 300.0, 330.0, 359.99,
        -1.0, 360.0, 400.0, // out-of-range
    ];
    let sats = [0.0, 0.5, 1.0];
    let lums = [0.0, 0.25, 0.5, 0.75, 1.0];

    for &h in &hues {
        for &s in &sats {
            for &l in &lums {
                compare(h, s, l, &c_lib, &rs_lib);
            }
        }
    }
}

#[test]
fn test_hsl_to_rgb_special_values() {
    let (c_lib, rs_lib) = load_libs();
    let specials: &[(f32, f32, f32)] = &[
        (f32::NAN, 0.5, 0.5),
        (0.0, f32::NAN, 0.5),
        (0.0, 0.5, f32::NAN),
        (f32::INFINITY, 0.5, 0.5),
        (f32::NEG_INFINITY, 0.5, 0.5),
        (0.0, 0.0, 0.0),
        (0.0, 0.0, 1.0),
        (0.0, 1.0, 0.5),
    ];
    for &(h, s, l) in specials {
        compare(h, s, l, &c_lib, &rs_lib);
    }
}

#[test]
fn test_hsl_to_rgb_sweep() {
    let (c_lib, rs_lib) = load_libs();
    // Sweep hue in 1-degree steps with a few s/l combos
    for hi in 0..360 {
        let h = hi as f32;
        for &s in &[0.3, 0.7, 1.0] {
            for &l in &[0.2, 0.5, 0.8] {
                compare(h, s, l, &c_lib, &rs_lib);
            }
        }
    }
}
