//! Differential tests for the top-level `charinbuf` entry point.
//! Both the return value and everything written to stdout must match.

mod common;

use common::*;
use std::ffi::c_int;

type Charinbuf = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

fn check(mode: c_int, value: c_int, opt1: c_int, opt2: c_int) {
    let (c, r) = sym::<Charinbuf>("charinbuf");
    let (cv, cout) = capture(|| unsafe { c(mode, value, opt1, opt2) });
    let (rv, rout) = capture(|| unsafe { r(mode, value, opt1, opt2) });
    assert_eq!(
        show(&cout),
        show(&rout),
        "charinbuf({mode}, {value}, {opt1}, {opt2}) stdout mismatch"
    );
    assert_eq!(
        cout, rout,
        "charinbuf({mode}, {value}, {opt1}, {opt2}) stdout bytes mismatch"
    );
    assert_eq!(
        cv, rv,
        "charinbuf({mode}, {value}, {opt1}, {opt2}) return mismatch"
    );
}

#[test]
fn mode0_uint16_validation() {
    let _g = lock();
    for v in [
        c_int::MIN,
        c_int::MIN + 1,
        -100000,
        -65536,
        -1,
        0,
        1,
        255,
        256,
        32767,
        32768,
        65534,
        65535,
        65536,
        65537,
        1000000,
        c_int::MAX - 1,
        c_int::MAX,
    ] {
        check(0, v, 0, 0);
        check(0, v, 12345, -999);
    }
}

#[test]
fn mode1_string_empty() {
    let _g = lock();
    for v in [0, 1, -1, c_int::MAX, c_int::MIN, 65535] {
        check(1, v, v, v);
    }
}

#[test]
fn mode2_alloc_free() {
    let _g = lock();
    for v in [0, 7, -7, c_int::MAX, c_int::MIN] {
        check(2, v, 3, 4);
    }
}

#[test]
fn mode3_function_pointers() {
    let _g = lock();
    let interesting = [
        0,
        1,
        -1,
        2,
        -2,
        5,
        10,
        -10,
        100,
        65535,
        65536,
        c_int::MAX,
        c_int::MIN,
        c_int::MAX - 1,
        c_int::MIN + 1,
        46341,
        -46341,
    ];
    for &v in &interesting {
        for &o1 in &interesting {
            for &o2 in &[0, 1, -1, 2, 3, -3, c_int::MAX, c_int::MIN] {
                check(3, v, o1, o2);
            }
        }
    }
}

#[test]
fn mode4_memchr() {
    let _g = lock();
    for v in [0, 1, -1, 88, c_int::MAX, c_int::MIN] {
        check(4, v, v, v);
    }
}

#[test]
fn invalid_modes() {
    let _g = lock();
    for m in [
        -1,
        -2,
        -100,
        5,
        6,
        7,
        100,
        65536,
        c_int::MAX,
        c_int::MIN,
        c_int::MAX - 1,
    ] {
        check(m, 1, 2, 3);
    }
}

/// Consecutive calls in one process: `charinbuf` zeroes the shared counter on
/// entry, so results must not drift between invocations.
#[test]
fn repeated_and_interleaved_calls() {
    let _g = lock();
    let mut x: u32 = 0xfeed_face;
    for _ in 0..400 {
        x = x.wrapping_mul(1664525).wrapping_add(1013904223);
        let mode = (x % 8) as c_int - 2;
        let value = (x >> 3) as c_int;
        let opt1 = (x >> 7).wrapping_mul(2654435761) as c_int;
        let opt2 = (x >> 11) as c_int;
        check(mode, value, opt1, opt2);
    }
}

/// Guards against a vacuous suite: if stdout capture silently returned nothing,
/// every byte comparison above would trivially pass.
#[test]
fn capture_actually_observes_stdout() {
    let _g = lock();
    let (c, r) = sym::<Charinbuf>("charinbuf");
    for mode in 0..5 {
        let (_, cout) = capture(|| unsafe { c(mode, 7, 3, 2) });
        let (_, rout) = capture(|| unsafe { r(mode, 7, 3, 2) });
        assert!(
            !cout.is_empty(),
            "C mode {mode} produced no captured output — capture is broken"
        );
        assert!(
            !rout.is_empty(),
            "Rust mode {mode} produced no captured output — capture is broken"
        );
        assert!(
            cout.contains(&b'\n'),
            "mode {mode} output lacks a newline: {:?}",
            show(&cout)
        );
    }
    // And a known-good line from the C side.
    let (_, cout) = capture(|| unsafe { c(0, 7, 0, 0) });
    assert!(
        show(&cout).contains("Mode 0: UINT16_MAX validation"),
        "unexpected C output: {}",
        show(&cout)
    );
    let (_, rout) = capture(|| unsafe { r(0, 7, 0, 0) });
    assert_eq!(show(&cout), show(&rout));
}
