//! Phase C — error-path / boundary differential tests.
//!
//! One test per row of `ERRORS.md`. The C library has no error-return channel
//! (`driver` is `void` and contains no checks), so each test asserts the two
//! libraries agree on the *absence* of rejection: identical bytes on stdout and
//! a normal return from both, with no panic, abort, or trap on the Rust side.
//! Rows 16 and 17 assert the inexpressibility of null/length inputs from the C
//! signature itself.

mod common;

use common::{capture, compare_batch, compare_one, c_driver, rust_driver};

/// A call must produce output and return normally on both sides — the "no
/// rejection" assertion shared by every row of `ERRORS.md`.
fn assert_no_rejection(row: &str, x: i32, y: i32) {
    let c = capture(|| unsafe { c_driver()(x, y) });
    let r = capture(|| unsafe { rust_driver()(x, y) });
    assert!(
        !c.is_empty(),
        "{row}: C produced no output for x={x} y={y} (would indicate a rejection)"
    );
    assert!(
        !r.is_empty(),
        "{row}: Rust produced no output for x={x} y={y} (would indicate a rejection)"
    );
    assert_eq!(
        *c.last().unwrap(),
        b'\n',
        "{row}: C output not newline-terminated for x={x} y={y}"
    );
    assert_eq!(
        c,
        r,
        "{row}: C and Rust disagree for x={x} (0x{x:08x}) y={y} (0x{y:08x}): C={:?} Rust={:?}",
        String::from_utf8_lossy(&c),
        String::from_utf8_lossy(&r)
    );
}

/// Row 1 — degenerate all-zero input.
#[test]
fn err01_both_zero() {
    assert_no_rejection("err01", 0, 0);
}

/// Row 2 — the only input pair yielding the falsy-looking result `0`.
#[test]
fn err02_result_zero_sentinel() {
    assert_no_rejection("err02", 0, -1);
    let c = capture(|| unsafe { c_driver()(0, -1) });
    assert_eq!(c, b"0\n", "err02: C must print a bare zero");
}

/// Row 3 — extreme low `x`.
#[test]
fn err03_x_int_min() {
    assert_no_rejection("err03", i32::MIN, 0);
}

/// Row 4 — extreme high `x`.
#[test]
fn err04_x_int_max() {
    assert_no_rejection("err04", i32::MAX, 0);
}

/// Row 5 — `y == INT_MIN`, so `~y == INT_MAX`.
#[test]
fn err05_y_int_min() {
    let mut pairs = vec![(0, i32::MIN), (-1, i32::MIN), (i32::MAX, i32::MIN)];
    let mut rng = common::Rng::new(0xE05);
    for _ in 0..200 {
        pairs.push((rng.next_i32(), i32::MIN));
    }
    compare_batch("err05", &pairs);
    assert_no_rejection("err05", 0, i32::MIN);
}

/// Row 6 — `y == INT_MAX`, so `~y == INT_MIN`; the result is always negative.
#[test]
fn err06_y_int_max() {
    let mut pairs = vec![(0, i32::MAX), (-1, i32::MAX), (i32::MAX, i32::MAX)];
    let mut rng = common::Rng::new(0xE06);
    for _ in 0..200 {
        let x = rng.next_i32();
        assert!(x | !i32::MAX < 0, "err06 setup");
        pairs.push((x, i32::MAX));
    }
    compare_batch("err06", &pairs);
}

/// Row 7 — `result == INT_MIN`: the magnitude has no positive `int`
/// counterpart, the classic `%d` negation-overflow case.
#[test]
fn err07_result_int_min_unnegatable() {
    assert_no_rejection("err07", 0, i32::MAX);
    let c = capture(|| unsafe { c_driver()(0, i32::MAX) });
    assert_eq!(c, b"-2147483648\n", "err07: C must print the full INT_MIN");
    let r = capture(|| unsafe { rust_driver()(0, i32::MAX) });
    assert_eq!(c, r);
}

/// Row 8 — both operands extreme low.
#[test]
fn err08_both_int_min() {
    assert_no_rejection("err08", i32::MIN, i32::MIN);
}

/// Row 9 — both operands extreme high.
#[test]
fn err09_both_int_max() {
    assert_no_rejection("err09", i32::MAX, i32::MAX);
}

/// Row 10 — one step past the top of the signed range: `0x80000000` passed as a
/// raw 32-bit pattern, which is `INT_MIN` once reinterpreted as `int`.
#[test]
fn err10_one_past_int_max_bit_pattern() {
    let past = 0x8000_0000u32 as i32;
    assert_eq!(past, i32::MIN);
    assert_no_rejection("err10", past, 0);
    assert_no_rejection("err10", 0, past);
    assert_no_rejection("err10", past, past);
}

/// Row 11 — one step past the bottom, produced by wrapping arithmetic
/// (`INT_MAX + 1` and `INT_MIN - 1`).
#[test]
fn err11_wrapped_out_of_range() {
    let over = i32::MAX.wrapping_add(1);
    let under = i32::MIN.wrapping_sub(1);
    assert_eq!(over, i32::MIN);
    assert_eq!(under, i32::MAX);
    for &(x, y) in &[(over, under), (under, over), (over, over), (under, under)] {
        assert_no_rejection("err11", x, y);
    }
}

/// Row 12 — out-of-range "enum-like" integers: 81 combinations of values that
/// would match no valid variant if the parameters were C enums.
#[test]
fn err12_out_of_range_enum_like_values() {
    const VALS: [i32; 9] = [
        i32::MIN, -1_000_000, -2, 3, 42, 255, 256, 65_536, i32::MAX,
    ];
    let mut pairs = Vec::with_capacity(81);
    for &x in &VALS {
        for &y in &VALS {
            pairs.push((x, y));
        }
    }
    assert_eq!(pairs.len(), 81);
    compare_batch("err12", &pairs);
    // And individually, asserting neither side rejects.
    for &(x, y) in &pairs {
        assert_no_rejection("err12", x, y);
    }
}

/// Row 13 — sign-bit-only operands.
#[test]
fn err13_sign_bit_only() {
    let sb = 0x8000_0000u32 as i32;
    assert_no_rejection("err13", sb, sb);
    let c = capture(|| unsafe { c_driver()(sb, sb) });
    assert_eq!(c, b"-1\n");
}

/// Row 14 — single-bit sweep over all 32 positions in both operands, asserting
/// no combination is rejected by either side.
#[test]
fn err14_single_bit_sweep_no_rejection() {
    let mut pairs = Vec::with_capacity(1024);
    for i in 0..32u32 {
        for j in 0..32u32 {
            pairs.push(((1u32 << i) as i32, (1u32 << j) as i32));
        }
    }
    compare_batch("err14", &pairs);

    // Spot-check the "no rejection" property on the sign-bit rows, which are the
    // ones an over-eager validity check would most plausibly reject.
    for i in 0..32u32 {
        assert_no_rejection("err14", (1u32 << 31) as i32, (1u32 << i) as i32);
        assert_no_rejection("err14", (1u32 << i) as i32, (1u32 << 31) as i32);
    }
}

/// Row 15 — repeated invocation without an intervening flush: 256 calls must
/// concatenate identically, one line per call, on both sides.
#[test]
fn err15_repeated_invocation_state() {
    let mut rng = common::Rng::new(0xE15);
    let pairs: Vec<(i32, i32)> = (0..256).map(|_| (rng.next_i32(), rng.next_i32())).collect();
    let c = capture(|| {
        let f = c_driver();
        for &(x, y) in &pairs {
            unsafe { f(x, y) };
        }
    });
    let r = capture(|| {
        let f = rust_driver();
        for &(x, y) in &pairs {
            unsafe { f(x, y) };
        }
    });
    assert_eq!(c, r, "err15: 256 repeated calls diverge");
    assert_eq!(
        c.iter().filter(|&&b| b == b'\n').count(),
        pairs.len(),
        "err15: one newline per call expected"
    );
}

/// Row 16 — no null-pointer input is expressible: the C API declares no pointer
/// parameters, so the closest analogue is the all-zero bit pattern.
#[test]
fn err16_no_pointer_parameters_exist() {
    let header = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../c_src/include/driver.h"
    ))
    .expect("read driver.h");
    let decl = header
        .lines()
        .find(|l| l.contains("driver(") && l.contains(';'))
        .expect("driver declaration in driver.h");
    assert_eq!(
        decl.trim(),
        "void driver(int x, int y);",
        "the public signature changed; re-derive ERRORS.md"
    );
    assert!(
        !decl.contains('*'),
        "driver takes a pointer after all -- a real null-pointer row is required"
    );
    // Nearest expressible analogue of a null argument.
    compare_one("err16", 0, 0);
}

/// Row 17 — no length/size input is expressible either; the nearest analogues
/// are the zero and extreme-magnitude scalars.
#[test]
fn err17_no_length_parameters_exist() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../c_src/src/driver.c"
    ))
    .expect("read driver.c");
    // The C body must remain the branchless two-statement function that
    // ERRORS.md was derived from; if it grows a check, the table is stale.
    for forbidden in [
        "assert", "errno", "return -", "return NULL", "exit(", "abort(",
    ] {
        assert!(
            !src.contains(forbidden),
            "driver.c now contains `{forbidden}` -- ERRORS.md must be re-derived"
        );
    }
    for &(x, y) in &[(0, 0), (i32::MIN, 0), (i32::MAX, 0), (0, i32::MIN), (0, i32::MAX)] {
        assert_no_rejection("err17", x, y);
    }
}
