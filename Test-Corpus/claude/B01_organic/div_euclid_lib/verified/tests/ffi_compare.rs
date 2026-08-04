use libloading::{Library, Symbol};
use std::os::raw::c_int;

type DivEuclidFn = unsafe extern "C" fn(c_int, c_int) -> c_int;

fn c_lib_path() -> &'static str {
    "c_src/build/libtranslated_rust.so"
}

fn rust_lib_path() -> &'static str {
    // The Rust crate-type=cdylib output. Cargo places it in target/<profile>/.
    // We rely on cargo running tests with CARGO_TARGET_DIR or default target/debug.
    let candidates = [
        "target/debug/libdiv_euclid_lib.so",
        "target/release/libdiv_euclid_lib.so",
    ];
    for c in &candidates {
        if std::path::Path::new(c).exists() {
            // Leak the &str so we can return &'static str.
            return Box::leak(c.to_string().into_boxed_str());
        }
    }
    panic!("Could not find Rust .so. Run `cargo build` first.");
}

unsafe fn call(lib: &Library, v1: c_int, v2: c_int) -> c_int {
    let f: Symbol<DivEuclidFn> = lib.get(b"div_euclid").expect("symbol div_euclid");
    f(v1, v2)
}

fn run_pair(v1: c_int, v2: c_int) {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C .so");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust .so");
        let c = call(&c_lib, v1, v2);
        let r = call(&r_lib, v1, v2);
        assert_eq!(c, r, "Mismatch for div_euclid({}, {}): C={}, Rust={}", v1, v2, c, r);
    }
}

#[test]
fn test_divide_by_zero() {
    run_pair(0, 0);
    run_pair(1, 0);
    run_pair(-1, 0);
    run_pair(i32::MAX, 0);
    run_pair(i32::MIN, 0);
}

#[test]
fn test_basic_positive() {
    for a in 0..20 {
        for b in 1..20 {
            run_pair(a, b);
        }
    }
}

#[test]
fn test_negatives() {
    for a in -20..20 {
        for b in -20..20 {
            if b == 0 { continue; }
            run_pair(a, b);
            run_pair(-a, b);
            run_pair(a, -b);
            run_pair(-a, -b);
        }
    }
}

#[test]
fn test_extremes() {
    let extremes = [i32::MIN, i32::MIN + 1, -2, -1, 0, 1, 2, i32::MAX - 1, i32::MAX];
    for &a in &extremes {
        for &b in &extremes {
            run_pair(a, b);
        }
    }
}

#[test]
fn test_random_sample() {
    use std::num::Wrapping;
    // Deterministic LCG sample
    let mut s: u64 = 0xDEAD_BEEF_CAFE_BABE;
    for _ in 0..5000 {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let a = (Wrapping(s as u32).0) as i32;
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let b = (Wrapping(s as u32).0) as i32;
        run_pair(a, b);
    }
}

#[test]
fn test_min_intentional_overflow_paths() {
    let imin = i32::MIN;
    // Cases that exercise the special branches in C: v2 == INT_MIN, etc.
    run_pair(imin, imin);
    run_pair(imin, 1);
    run_pair(imin, -1);
    run_pair(1, imin);
    run_pair(-1, imin);
    run_pair(imin, 2);
    run_pair(imin, -2);
    run_pair(2, imin);
    run_pair(-2, imin);
    run_pair(imin + 1, imin);
    run_pair(imin, imin + 1);
}
