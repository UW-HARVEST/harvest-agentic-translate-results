use libloading::{Library, Symbol};

const C_LIB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libpow.so");

type MyPowFn = unsafe extern "C" fn(f64, f64) -> f64;

fn load_c_my_pow() -> (Library, MyPowFn) {
    let lib = unsafe { Library::new(C_LIB_PATH) }.expect("Failed to load C libpow.so");
    let func: Symbol<MyPowFn> = unsafe { lib.get(b"my_pow") }.expect("Failed to find my_pow");
    let func = *func;
    (lib, func)
}

fn compare(base: f64, exponent: f64, c_fn: MyPowFn) {
    let c_result = unsafe { c_fn(base, exponent) };
    let rust_result = pow::my_pow(base, exponent);
    assert_eq!(
        c_result.to_bits(),
        rust_result.to_bits(),
        "Mismatch for my_pow({}, {}): C={}, Rust={}",
        base, exponent, c_result, rust_result
    );
}

#[test]
fn test_my_pow_normal_cases() {
    let (_lib, c_fn) = load_c_my_pow();
    let cases: &[(f64, f64)] = &[
        (2.0, 3.0),
        (10.0, 0.0),
        (1.0, 100.0),
        (0.0, 5.0),
        (2.0, -1.0),
        (3.0, 2.0),
        (0.5, 3.0),
        (100.0, 0.5),
        (-2.0, 3.0),
        (-2.0, 2.0),
    ];
    for &(base, exp) in cases {
        compare(base, exp, c_fn);
    }
}

#[test]
fn test_my_pow_edge_cases() {
    let (_lib, c_fn) = load_c_my_pow();
    let cases: &[(f64, f64)] = &[
        (0.0, 0.0),
        (1.0, f64::INFINITY),
        (f64::INFINITY, 2.0),
        (f64::INFINITY, -1.0),
        (0.0, -1.0),
        (f64::NAN, 2.0),
        (2.0, f64::NAN),
    ];
    for &(base, exp) in cases {
        compare(base, exp, c_fn);
    }
}

#[test]
fn test_my_pow_error_cases() {
    let (_lib, c_fn) = load_c_my_pow();
    // Cases that may trigger errno on some platforms
    let cases: &[(f64, f64)] = &[
        (-1.0, 0.5),       // negative base, fractional exponent -> may EDOM
        (1e308, 1e308),     // huge overflow -> may ERANGE
    ];
    for &(base, exp) in cases {
        compare(base, exp, c_fn);
    }
}
