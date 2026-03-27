use libloading::{Library, Symbol};
use colourblind_lib::{cb_impairment, colourblind};

const C_LIB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libcolourblind_lib.so");

type ColourblindFn = unsafe extern "C" fn(i32, *mut f32, *mut f32, *mut f32);

fn call_c(impairment: i32, r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    unsafe {
        let lib = Library::new(C_LIB_PATH).expect("Failed to load C library");
        let func: Symbol<ColourblindFn> = lib.get(b"colourblind").expect("Failed to find colourblind");
        let (mut cr, mut cg, mut cb) = (r, g, b);
        func(impairment, &mut cr, &mut cg, &mut cb);
        (cr, cg, cb)
    }
}

fn call_rust(impairment: cb_impairment, r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let (mut rr, mut rg, mut rb) = (r, g, b);
    colourblind(impairment, &mut rr, &mut rg, &mut rb);
    (rr, rg, rb)
}

const TEST_INPUTS: &[(f32, f32, f32)] = &[
    (0.0, 0.0, 0.0),
    (1.0, 1.0, 1.0),
    (0.5, 0.3, 0.8),
    (1.0, 0.0, 0.0),
    (0.0, 1.0, 0.0),
    (0.0, 0.0, 1.0),
    (0.123, 0.456, 0.789),
];

fn compare_all(impairment_c: i32, impairment_rust: cb_impairment, name: &str) {
    for &(r, g, b) in TEST_INPUTS {
        let c = call_c(impairment_c, r, g, b);
        let rust = call_rust(impairment_rust, r, g, b);
        assert_eq!(
            c.0.to_bits(), rust.0.to_bits(),
            "{name}: R mismatch for input ({r},{g},{b}): C={}, Rust={}", c.0, rust.0
        );
        assert_eq!(
            c.1.to_bits(), rust.1.to_bits(),
            "{name}: G mismatch for input ({r},{g},{b}): C={}, Rust={}", c.1, rust.1
        );
        assert_eq!(
            c.2.to_bits(), rust.2.to_bits(),
            "{name}: B mismatch for input ({r},{g},{b}): C={}, Rust={}", c.2, rust.2
        );
    }
}

#[test]
fn test_protanopia() {
    compare_all(0, cb_impairment::cbProtanopia, "Protanopia");
}

#[test]
fn test_deuteranopia() {
    compare_all(1, cb_impairment::cbDeuteranopia, "Deuteranopia");
}

#[test]
fn test_tritanopia() {
    compare_all(2, cb_impairment::cbTritanopia, "Tritanopia");
}
