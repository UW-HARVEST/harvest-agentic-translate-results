//! Differential test of the one and only symbol the C library exports:
//! `int call_predict(int pfcn)`.
//!
//! Both libraries are loaded with `libloading`; the Rust side is never called
//! directly, so the `#[no_mangle]` wrapper is exercised as an external caller
//! would exercise it.

mod support;

use libloading::{Library, Symbol};

type CallPredict = unsafe extern "C" fn(i32) -> i32;

struct Pair {
    _c: Library,
    _r: Library,
    c: CallPredict,
    r: CallPredict,
}

fn load() -> Pair {
    let c_path = support::c_shared_lib();
    let r_path = support::rust_shared_lib();
    unsafe {
        let c_lib = Library::new(&c_path).expect("load C .so");
        let r_lib = Library::new(&r_path).expect("load Rust .so");
        let c: Symbol<CallPredict> = c_lib.get(b"call_predict\0").expect("C call_predict");
        let r: Symbol<CallPredict> = r_lib.get(b"call_predict\0").expect("Rust call_predict");
        let (c, r) = (*c, *r);
        Pair {
            _c: c_lib,
            _r: r_lib,
            c,
            r,
        }
    }
}

fn check(p: &Pair, pfcn: i32) {
    let (c, r) = unsafe { ((p.c)(pfcn), (p.r)(pfcn)) };
    assert_eq!(
        c, r,
        "call_predict({pfcn}) mismatch: C returned {c}, Rust returned {r}"
    );
}

#[test]
fn call_predict_matches_for_documented_range() {
    let p = load();
    // 0..=11 select a dedicated `_PfnN` entry point; 12..=15 and everything
    // else fall through to the generic `BTAC1C2_PredictSample`.
    for pfcn in -4..=32 {
        check(&p, pfcn);
    }
}

#[test]
fn call_predict_matches_for_wide_range() {
    let p = load();
    for pfcn in -5000..=5000 {
        check(&p, pfcn);
    }
}

#[test]
fn call_predict_matches_at_integer_extremes() {
    let p = load();
    for pfcn in [
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 11,
        -1_000_000,
        -65536,
        -256,
        -12,
        -1,
        0,
        11,
        12,
        15,
        16,
        256,
        65535,
        65536,
        1_000_000,
        i32::MAX - 11,
        i32::MAX - 1,
        i32::MAX,
    ] {
        check(&p, pfcn);
    }
}

#[test]
fn call_predict_matches_on_pseudo_random_inputs() {
    let p = load();
    // xorshift32, deterministic; no external rng dependency.
    let mut s: u32 = 0x1234_5678;
    for _ in 0..200_000 {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        check(&p, s as i32);
    }
}

#[test]
fn call_predict_is_stateless_and_repeatable() {
    let p = load();
    for _ in 0..3 {
        for pfcn in -20..=40 {
            check(&p, pfcn);
        }
    }
}
