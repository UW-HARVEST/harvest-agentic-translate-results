use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/libcolourblind_lib.so")
}

type ColourblindFn = unsafe extern "C" fn(i32, *mut f32, *mut f32, *mut f32);

fn call_colourblind(lib: &Library, impairment: i32, r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    unsafe {
        let func: Symbol<ColourblindFn> = lib.get(b"colourblind").unwrap();
        let (mut rv, mut gv, mut bv) = (r, g, b);
        func(impairment, &mut rv, &mut gv, &mut bv);
        (rv, gv, bv)
    }
}

fn bits_eq(a: f32, b: f32) -> bool {
    if a.is_nan() && b.is_nan() { return true; }
    a.to_bits() == b.to_bits()
}

fn assert_identical(c: (f32, f32, f32), rs: (f32, f32, f32), label: &str) {
    assert!(bits_eq(c.0, rs.0), "{label}: R mismatch: C={} Rust={}", c.0, rs.0);
    assert!(bits_eq(c.1, rs.1), "{label}: G mismatch: C={} Rust={}", c.1, rs.1);
    assert!(bits_eq(c.2, rs.2), "{label}: B mismatch: C={} Rust={}", c.2, rs.2);
}

const TEST_INPUTS: &[(f32, f32, f32)] = &[
    (0.0, 0.0, 0.0),
    (1.0, 1.0, 1.0),
    (1.0, 0.0, 0.0),
    (0.0, 1.0, 0.0),
    (0.0, 0.0, 1.0),
    (0.5, 0.5, 0.5),
    (0.25, 0.75, 0.1),
    (255.0, 128.0, 64.0),
    (-1.0, 0.5, 2.0),
    (f32::MAX, f32::MIN, 0.0),
    (f32::INFINITY, f32::NEG_INFINITY, f32::NAN),
];

#[test]
fn test_colourblind_all_modes() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let rs_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    for impairment in 0..=2 {
        let name = ["Protanopia", "Deuteranopia", "Tritanopia"][impairment as usize];
        for &(r, g, b) in TEST_INPUTS {
            let c_out = call_colourblind(&c_lib, impairment, r, g, b);
            let rs_out = call_colourblind(&rs_lib, impairment, r, g, b);
            let label = format!("{name}({r}, {g}, {b})");
            assert_identical(c_out, rs_out, &label);
        }
    }
}
