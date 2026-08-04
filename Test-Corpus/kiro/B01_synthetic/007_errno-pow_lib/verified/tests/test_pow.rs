use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libpow.so")
}

fn rust_lib_path() -> PathBuf {
    // cargo puts cdylib in target/<profile>/libpow.so
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target/debug/libpow.so");
    p
}

type MyPowFn = unsafe extern "C" fn(f64, f64) -> f64;

fn load_my_pow(lib: &Library) -> Symbol<MyPowFn> {
    unsafe { lib.get(b"my_pow") }.expect("symbol my_pow not found")
}

fn compare(base: f64, exponent: f64) {
    let c_lib = unsafe { Library::new(c_lib_path()) }.expect("load C lib");
    let r_lib = unsafe { Library::new(rust_lib_path()) }.expect("load Rust lib");
    let c_fn = load_my_pow(&c_lib);
    let r_fn = load_my_pow(&r_lib);

    let c_res: f64 = unsafe { c_fn(base, exponent) };
    let r_res: f64 = unsafe { r_fn(base, exponent) };

    assert!(
        c_res.to_bits() == r_res.to_bits(),
        "mismatch for my_pow({base}, {exponent}): C={c_res} (bits {:016x}), Rust={r_res} (bits {:016x})",
        c_res.to_bits(),
        r_res.to_bits(),
    );
}

#[test]
fn test_basic_powers() {
    for &(b, e) in &[
        (2.0, 10.0),
        (3.0, 3.0),
        (10.0, 0.0),
        (0.0, 0.0),
        (1.0, 1000.0),
        (0.0, 5.0),
        (5.0, 1.0),
        (-2.0, 3.0),
        (-2.0, 4.0),
        (2.0, -1.0),
        (0.5, 2.0),
    ] {
        compare(b, e);
    }
}

#[test]
fn test_special_values() {
    compare(f64::INFINITY, 2.0);
    compare(f64::NEG_INFINITY, 3.0);
    compare(f64::NAN, 2.0);
    compare(2.0, f64::NAN);
    compare(f64::NAN, f64::NAN);
    compare(0.0, -1.0);       // inf
    compare(-0.0, -1.0);      // -inf
    compare(1.0, f64::INFINITY);
    compare(1.0, f64::NEG_INFINITY);
}

#[test]
fn test_large_exponents() {
    // may trigger ERANGE in C
    compare(1e308, 2.0);
    compare(2.0, 1024.0);
    compare(0.5, -1074.0);
}

#[test]
fn test_negative_base_fractional_exp() {
    // may trigger EDOM in C
    compare(-1.0, 0.5);
}
