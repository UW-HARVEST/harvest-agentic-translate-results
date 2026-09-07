// CONFIGS.md row 26 -- byte-for-byte stdout comparison.
//
// `modeselect` emits eight `printf` lines (`%s`, `%d`, `%X`, `%ld`, `%.2e`).
// cargo's harness only captures Rust's print macros, so each call is made in a
// forked child with fd 1 redirected to a file, and the raw bytes are compared.

mod common;
use common::*;

fn stdout_of_modeselect(lib: &Lib, tag: &str, a: i32, b: i32, c: i32, d: i32) -> Vec<u8> {
    let (bytes, outcome) = capture_stdout(tag, || {
        let _ = lib.modeselect(a, b, c, d);
    });
    assert_eq!(
        outcome,
        Child::Exited(0),
        "{} modeselect({a}, {b}, {c}, {d}) child terminated abnormally: {outcome:?}",
        lib.name
    );
    bytes
}

#[track_caller]
fn compare_stdout(a: i32, b: i32, c: i32, d: i32) {
    let l = libs();
    let cb = stdout_of_modeselect(&l.c, "c", a, b, c, d);
    let rb = stdout_of_modeselect(&l.rust, "rust", a, b, c, d);
    if cb != rb {
        panic!(
            "STDOUT DIVERGENCE for modeselect({a}, {b}, {c}, {d})\n\
             --- C ({} bytes) ---\n{}\n--- Rust ({} bytes) ---\n{}\n",
            cb.len(),
            String::from_utf8_lossy(&cb),
            rb.len(),
            String::from_utf8_lossy(&rb),
        );
    }
    assert!(!cb.is_empty(), "expected printf output from modeselect");
}

#[test]
fn cfg_26_modeselect_stdout_byte_identical() {
    // 4x5 mode/complexity grid with a few seeds/offsets.
    for ms in 0..4 {
        for cx in 0..5 {
            compare_stdout(ms, 0, cx, 0);
        }
    }
    for seed in [0, 1, 7, 23, 24, 25, -1, -23, -24] {
        for off in [0, 1, -1, 1000, -1000] {
            compare_stdout(2, off, 3, seed);
        }
    }
    // Randomized tuples (fixed seed): exercises %.2e rounding and %X casing over
    // many magnitudes as well as the overflow sentinel formatting.
    let mut rng = Rng::new(SEED ^ 26);
    for _ in 0..96 {
        let ms = (rng.next_i32() & i32::MAX) as i32;
        let off = rng.next_i32();
        let cx = rng.next_i32();
        let seed = rng.next_i32();
        compare_stdout(ms, off, cx, seed);
    }
    // Extremes, where %.2e prints huge exponents and %X prints the sentinel.
    for &(a, b, c, d) in &[
        (0i32, i32::MAX, 0i32, i32::MAX),
        (0, i32::MIN, 0, i32::MIN),
        (3, i32::MIN + 1, 4, i32::MAX - 1),
        (1, 0, i32::MIN, 0),
    ] {
        compare_stdout(a, b, c, d);
    }
}

#[test]
fn cfg_26b_modeselect_stdout_negative_complexity_and_int_min() {
    for cx in [-1i32, -2, -3, -4, -5, i32::MIN] {
        compare_stdout(0, 0, cx, 0);
    }
    compare_stdout(i32::MIN, 0, 0, 0); // INT_MIN % 4 == 0 -> in-bounds selector
}
