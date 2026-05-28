// Compare C and Rust implementations of my_pow via FFI by loading both
// shared libraries with libloading and asserting byte-identical results.

use libloading::{Library, Symbol};
use std::path::PathBuf;

type MyPowFn = unsafe extern "C" fn(f64, f64) -> f64;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_lib_path() -> PathBuf {
    project_root().join("c_src/build/libpow.so")
}

fn rust_lib_path() -> PathBuf {
    // Cargo always builds dylibs into target/<profile>/. The integration
    // test harness builds the cdylib for us.
    let target_dir = project_root().join("target");
    // Try several locations: debug then release.
    for candidate in &["debug/libpow.so", "release/libpow.so"] {
        let p = target_dir.join(candidate);
        if p.exists() {
            return p;
        }
    }
    target_dir.join("debug/libpow.so")
}

fn assert_bit_equal(a: f64, b: f64, base: f64, exp: f64) {
    let abits = a.to_bits();
    let bbits = b.to_bits();
    assert_eq!(
        abits, bbits,
        "mismatch for my_pow({}, {}): C=0x{:016x} ({}), Rust=0x{:016x} ({})",
        base, exp, abits, a, bbits, b
    );
}

#[test]
fn compare_my_pow_across_inputs() {
    let c_lib = unsafe { Library::new(c_lib_path()) }
        .expect("failed to load C libpow.so; build it with cmake first");
    let rust_lib = unsafe { Library::new(rust_lib_path()) }
        .expect("failed to load Rust libpow.so; build it with cargo first");

    let c_pow: Symbol<MyPowFn> = unsafe { c_lib.get(b"my_pow\0") }.expect("C my_pow");
    let rust_pow: Symbol<MyPowFn> =
        unsafe { rust_lib.get(b"my_pow\0") }.expect("Rust my_pow");

    let cases: &[(f64, f64)] = &[
        // Normal cases
        (0.0, 0.0),
        (1.0, 0.0),
        (0.0, 1.0),
        (2.0, 3.0),
        (3.0, 2.0),
        (10.0, 5.0),
        (2.5, 4.0),
        (4.0, 0.5),
        (-2.0, 3.0),
        (-2.0, 4.0),
        // Negative base, fractional exponent => domain error
        (-2.0, 0.5),
        (-1.5, 1.5),
        // Overflow / underflow => range error
        (2.0, 1024.0),
        (10.0, 400.0),
        (0.5, 2000.0),
        (1.0e300, 10.0),
        // Special values
        (0.0, -1.0),     // pole error, may set ERANGE
        (-0.0, -1.0),
        (1.0, f64::INFINITY),
        (f64::INFINITY, 0.0),
        (f64::NAN, 0.0),
        (0.0, f64::NAN),
        (1.0, f64::NAN),
        (f64::NAN, f64::NAN),
        (-1.0, f64::INFINITY),
        // Edge powers
        (2.0, -3.0),
        (2.0, 0.5),
        (-3.0, 0.0),
    ];

    for &(base, exp) in cases {
        let c_result = unsafe { c_pow(base, exp) };
        let rust_result = unsafe { rust_pow(base, exp) };
        assert_bit_equal(c_result, rust_result, base, exp);
    }
}
