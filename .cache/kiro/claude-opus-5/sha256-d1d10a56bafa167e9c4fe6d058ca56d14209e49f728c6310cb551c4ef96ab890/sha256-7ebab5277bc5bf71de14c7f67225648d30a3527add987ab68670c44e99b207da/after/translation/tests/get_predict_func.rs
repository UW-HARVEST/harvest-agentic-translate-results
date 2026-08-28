//! Differential test of the one symbol the C library exports:
//! `int get_predict_func(int pfcn)`.
//!
//! Both libraries are loaded with `libloading` and called purely through their
//! dynamic symbols, so the Rust `#[no_mangle]` wrapper is exercised exactly as
//! an external C caller would exercise it.

mod common;

use libloading::{Library, Symbol};

type GetPredictFunc = unsafe extern "C" fn(i32) -> i32;

struct Pair {
    _c_lib: Library,
    _rust_lib: Library,
    c: GetPredictFunc,
    rust: GetPredictFunc,
}

impl Pair {
    fn load() -> Pair {
        unsafe {
            let c_lib = Library::new(common::c_so()).expect("load C .so");
            let rust_lib = Library::new(common::rust_so()).expect("load Rust .so");
            let c: Symbol<GetPredictFunc> = c_lib
                .get(b"get_predict_func\0")
                .expect("C get_predict_func");
            let rust: Symbol<GetPredictFunc> = rust_lib
                .get(b"get_predict_func\0")
                .expect("Rust get_predict_func");
            let c = *c;
            let rust = *rust;
            Pair {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c,
                rust,
            }
        }
    }

    fn check(&self, pfcn: i32) {
        let c = unsafe { (self.c)(pfcn) };
        let r = unsafe { (self.rust)(pfcn) };
        assert_eq!(
            c, r,
            "get_predict_func({pfcn}) mismatch: C returned {c}, Rust returned {r}"
        );
    }
}

/// Every predictor number that has a dedicated specialization, plus the
/// immediate neighbours of the valid range.
#[test]
fn matches_over_selector_range() {
    let pair = Pair::load();
    for pfcn in -8..=32 {
        pair.check(pfcn);
    }
}

/// The C `switch` has no arms for these, so the selector falls through to the
/// generic `BTAC1C2_PredictSample` and the second `switch` leaves `result` at 0.
#[test]
fn matches_on_out_of_range_and_extremes() {
    let pair = Pair::load();
    let interesting = [
        i32::MIN,
        i32::MIN + 1,
        -1_000_000,
        -65_536,
        -256,
        -16,
        -12,
        -1,
        0,
        11,
        12,
        15,
        16,
        256,
        65_536,
        1_000_000,
        i32::MAX - 1,
        i32::MAX,
    ];
    for pfcn in interesting {
        pair.check(pfcn);
    }
}

/// Exhaustive sweep across a wide band of inputs, then a strided sweep across
/// the whole `int` domain, to catch any divergent boundary handling.
#[test]
fn matches_exhaustively_and_strided() {
    let pair = Pair::load();
    for pfcn in -4096..=4096 {
        pair.check(pfcn);
    }
    let mut pfcn: i64 = i32::MIN as i64;
    while pfcn <= i32::MAX as i64 {
        pair.check(pfcn as i32);
        pfcn += 7_919; // prime stride, ~542k samples
    }
}

/// Repeated calls must be stable: the selector compares function addresses, so
/// a translation that accidentally folded two predictors together could still
/// pass a single call but drift under load ordering changes.
#[test]
fn results_are_stable_across_repeated_calls() {
    let pair = Pair::load();
    for _ in 0..64 {
        for pfcn in -2..=18 {
            pair.check(pfcn);
        }
    }
}

/// Sanity anchor independent of the C library: predictor numbers 0..=11 each
/// have a dedicated specialization and must report a match; everything else
/// falls through to the generic routine and must report 0.
#[test]
fn absolute_expected_values() {
    let pair = Pair::load();
    for pfcn in -8..=32 {
        let expected = if (0..=11).contains(&pfcn) { 1 } else { 0 };
        let c = unsafe { (pair.c)(pfcn) };
        let r = unsafe { (pair.rust)(pfcn) };
        assert_eq!(c, expected, "C get_predict_func({pfcn}) unexpected");
        assert_eq!(r, expected, "Rust get_predict_func({pfcn}) unexpected");
    }
}
