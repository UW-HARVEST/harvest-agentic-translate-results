//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test drives BOTH the C `.so` and the
//! Rust `.so` through their exported `driver` symbol and compares the captured
//! stdout byte-for-byte.

mod common;

use common::{
    capture, capture_via_pipe, compare_batch, compare_one, c_driver, rust_driver, Rng, EXTREMES,
};

/// Row 1 — both operands zero; `0 | ~0 == -1`.
#[test]
fn row01_both_zero() {
    compare_one("row01", 0, 0);
}

/// Row 2 — the unique pair whose result is `0`.
#[test]
fn row02_result_zero() {
    compare_one("row02", 0, -1);
}

/// Row 3 — `x == -1` saturates the OR; result is `-1` for every `y`.
#[test]
fn row03_x_all_ones() {
    let mut rng = Rng::new(0x0301);
    let mut pairs = vec![(-1, 0), (-1, -1), (-1, i32::MIN), (-1, i32::MAX)];
    for _ in 0..500 {
        pairs.push((-1, rng.next_i32()));
    }
    compare_batch("row03", &pairs);
}

/// Row 4 — `y == -1` makes `~y == 0`, so `result == x` over the full `x` range.
#[test]
fn row04_y_all_ones() {
    let mut rng = Rng::new(0x0401);
    let mut pairs = vec![(0, -1), (-1, -1), (i32::MIN, -1), (i32::MAX, -1)];
    for _ in 0..500 {
        pairs.push((rng.next_i32(), -1));
    }
    compare_batch("row04", &pairs);
}

/// Row 5 — `x == 0` isolates the complement: `result == ~y`.
#[test]
fn row05_x_zero_complement_only() {
    let mut rng = Rng::new(0x0501);
    let mut pairs = vec![(0, 0), (0, -1), (0, i32::MIN), (0, i32::MAX)];
    for _ in 0..500 {
        pairs.push((0, rng.next_i32()));
    }
    compare_batch("row05", &pairs);
}

/// Row 6 — `y == x`, so `x | ~x == -1` always.
#[test]
fn row06_y_equals_x() {
    let mut rng = Rng::new(0x0601);
    let mut pairs = vec![(0, 0), (-1, -1), (i32::MIN, i32::MIN), (i32::MAX, i32::MAX)];
    for _ in 0..500 {
        let x = rng.next_i32();
        pairs.push((x, x));
    }
    compare_batch("row06", &pairs);
}

/// Row 7 — `y == !x`, so `x | ~(~x) == x`.
#[test]
fn row07_y_is_complement_of_x() {
    let mut rng = Rng::new(0x0701);
    let mut pairs = vec![(0, !0), (-1, 0), (i32::MIN, !i32::MIN), (i32::MAX, !i32::MAX)];
    for _ in 0..500 {
        let x = rng.next_i32();
        pairs.push((x, !x));
    }
    compare_batch("row07", &pairs);
}

/// Row 8 — result forced non-negative: `x >= 0` and `y < 0` clears both sign
/// bits, so `%d` never emits a `-`.
#[test]
fn row08_result_positive() {
    let mut rng = Rng::new(0x0801);
    let mut pairs = Vec::new();
    for _ in 0..600 {
        let x = rng.next_nonneg();
        let y = rng.next_neg();
        assert!(x | !y >= 0, "row08 setup: expected non-negative result");
        pairs.push((x, y));
    }
    compare_batch("row08", &pairs);
}

/// Row 9 — result forced negative: `y >= 0` sets the sign bit via `~y`.
#[test]
fn row09_result_negative() {
    let mut rng = Rng::new(0x0901);
    let mut pairs = Vec::new();
    for _ in 0..600 {
        let x = rng.next_i32();
        let y = rng.next_nonneg();
        assert!(x | !y < 0, "row09 setup: expected negative result");
        pairs.push((x, y));
    }
    compare_batch("row09", &pairs);
}

/// Row 10 — both operands non-negative.
#[test]
fn row10_both_nonnegative() {
    let mut rng = Rng::new(0x1001);
    let mut pairs = Vec::new();
    for _ in 0..600 {
        pairs.push((rng.next_nonneg(), rng.next_nonneg()));
    }
    compare_batch("row10", &pairs);
}

/// Row 11 — both operands negative.
#[test]
fn row11_both_negative() {
    let mut rng = Rng::new(0x1101);
    let mut pairs = Vec::new();
    for _ in 0..600 {
        pairs.push((rng.next_neg(), rng.next_neg()));
    }
    compare_batch("row11", &pairs);
}

/// Row 12 — `x < 0 <= y`.
#[test]
fn row12_x_negative_y_nonnegative() {
    let mut rng = Rng::new(0x1201);
    let mut pairs = Vec::new();
    for _ in 0..600 {
        pairs.push((rng.next_neg(), rng.next_nonneg()));
    }
    compare_batch("row12", &pairs);
}

/// Row 13 — `y < 0 <= x`.
#[test]
fn row13_y_negative_x_nonnegative() {
    let mut rng = Rng::new(0x1301);
    let mut pairs = Vec::new();
    for _ in 0..600 {
        pairs.push((rng.next_nonneg(), rng.next_neg()));
    }
    compare_batch("row13", &pairs);
}

/// Row 14 — unconstrained full `int` range, 4000 randomized pairs.
#[test]
fn row14_full_range_random() {
    let mut rng = Rng::new(0xDEADBEEF);
    let mut pairs = Vec::with_capacity(4000);
    for _ in 0..4000 {
        pairs.push((rng.next_i32(), rng.next_i32()));
    }
    compare_batch("row14", &pairs);
}

/// Row 15 — `result == INT_MAX`, the widest positive rendering.
#[test]
fn row15_result_int_max() {
    assert_eq!(i32::MAX | !-1, i32::MAX);
    compare_one("row15", i32::MAX, -1);
}

/// Row 16 — `result == INT_MIN`, whose magnitude has no positive `int`.
#[test]
fn row16_result_int_min() {
    assert_eq!(0 | !i32::MAX, i32::MIN);
    compare_one("row16", 0, i32::MAX);
}

/// Row 17 — cross-product of the nine extreme values (81 pairs).
#[test]
fn row17_extreme_cross_product() {
    let mut pairs = Vec::with_capacity(81);
    for &x in &EXTREMES {
        for &y in &EXTREMES {
            pairs.push((x, y));
        }
    }
    assert_eq!(pairs.len(), 81);
    compare_batch("row17", &pairs);
}

/// Row 18 — single-bit operands across all 32 bit positions (sign bit included).
#[test]
fn row18_single_bit_sweep() {
    let mut pairs = Vec::with_capacity(1024);
    for i in 0..32u32 {
        for j in 0..32u32 {
            pairs.push(((1u32 << i) as i32, (1u32 << j) as i32));
        }
    }
    assert_eq!(pairs.len(), 1024);
    compare_batch("row18", &pairs);
}

/// Row 19 — one clear bit in a field of ones, all 32 positions.
#[test]
fn row19_single_clear_bit_sweep() {
    let mut pairs = Vec::with_capacity(1024);
    for i in 0..32u32 {
        for j in 0..32u32 {
            pairs.push((!((1u32 << i) as i32), !((1u32 << j) as i32)));
        }
    }
    compare_batch("row19", &pairs);
}

/// Row 20 — disjoint bit sets: `x & ~y == 0`, so `x` contributes nothing.
#[test]
fn row20_disjoint_bits() {
    let mut rng = Rng::new(0x2001);
    let mut pairs = Vec::new();
    for _ in 0..500 {
        let y = rng.next_i32();
        // Every set bit of x must also be set in y, so x & ~y == 0.
        let x = rng.next_i32() & y;
        assert_eq!(x & !y, 0, "row20 setup");
        pairs.push((x, y));
    }
    compare_batch("row20", &pairs);
}

/// Row 21 — positive results of every decimal width from 1 to 10 digits.
#[test]
fn row21_positive_digit_widths() {
    // y = -1 makes result == x, so x directly selects the digit width.
    let mut pairs = Vec::new();
    let mut p: i64 = 1;
    for _ in 0..10 {
        for cand in [p, p + 1, p * 9, p * 9 + 8, p * 10 - 1] {
            if cand <= i32::MAX as i64 {
                pairs.push((cand as i32, -1));
            }
        }
        p *= 10;
    }
    pairs.push((i32::MAX, -1));
    compare_batch("row21", &pairs);
}

/// Row 22 — negative results of every decimal width, i.e. widths 1..10 plus the
/// `-` sign byte.
#[test]
fn row22_negative_digit_widths() {
    // x = 0 makes result == ~y, so y = ~want selects the exact result.
    let mut pairs = Vec::new();
    let mut p: i64 = 1;
    for _ in 0..10 {
        for cand in [p, p + 1, p * 9, p * 10 - 1] {
            if cand <= i32::MAX as i64 {
                let want = -(cand as i32);
                pairs.push((0, !want));
            }
        }
        p *= 10;
    }
    pairs.push((0, !i32::MIN));
    pairs.push((0, i32::MAX)); // result INT_MIN
    compare_batch("row22", &pairs);
}

/// Row 23 — dense sweep of every small pair in `[-40, 40]^2` (6561 pairs).
#[test]
fn row23_dense_small_sweep() {
    let mut pairs = Vec::with_capacity(81 * 81);
    for x in -40..=40 {
        for y in -40..=40 {
            pairs.push((x, y));
        }
    }
    assert_eq!(pairs.len(), 6561);
    compare_batch("row23", &pairs);
}

/// Row 24 — 512 back-to-back calls compared as a single accumulated stream,
/// then re-compared call-by-call. Catches a wrong flush point, a swapped
/// number/newline order, or a missing or duplicated newline.
#[test]
fn row24_back_to_back_stream() {
    let mut rng = Rng::new(0x2401);
    let pairs: Vec<(i32, i32)> = (0..512).map(|_| (rng.next_i32(), rng.next_i32())).collect();

    let c_stream = capture(|| {
        let f = c_driver();
        for &(x, y) in &pairs {
            unsafe { f(x, y) };
        }
    });
    let r_stream = capture(|| {
        let f = rust_driver();
        for &(x, y) in &pairs {
            unsafe { f(x, y) };
        }
    });
    assert_eq!(
        c_stream, r_stream,
        "row24: accumulated 512-call streams differ"
    );

    // The concatenation of the individual captures must equal the batch stream,
    // proving no per-call trailing/leading bytes are lost or added.
    let mut concat = Vec::new();
    for &(x, y) in &pairs {
        concat.extend_from_slice(&capture(|| unsafe { c_driver()(x, y) }));
    }
    assert_eq!(
        concat, c_stream,
        "row24: C per-call concatenation != C batch stream"
    );

    // Exactly one newline-terminated line per call, and no other newlines.
    assert_eq!(
        c_stream.iter().filter(|&&b| b == b'\n').count(),
        pairs.len(),
        "row24: unexpected newline count"
    );
    assert_eq!(*c_stream.last().unwrap(), b'\n', "row24: no trailing newline");
}

/// Row 25 — a single call into a freshly `dlopen`ed pair of libraries: nothing
/// may be emitted at load time, and no pre-existing `printf` state may be
/// assumed.
#[test]
fn row25_fresh_load_single_call() {
    let mut rng = Rng::new(0x2501);
    let (x, y) = (rng.next_i32(), rng.next_i32());

    let c_path = std::env::var("C_DRIVER_SO").unwrap_or_else(|_| {
        format!(
            "{}/../c_src/build/libdriver.so",
            env!("CARGO_MANIFEST_DIR")
        )
    });
    let r_path = std::env::var("RUST_DRIVER_SO").unwrap_or_else(|_| {
        format!("{}/target/release/libdriver.so", env!("CARGO_MANIFEST_DIR"))
    });

    // Load inside the capture so any load-time output is captured too.
    let c_out = capture(|| unsafe {
        let lib = libloading::Library::new(&c_path).expect("dlopen C .so");
        let f: libloading::Symbol<common::DriverFn> = lib.get(b"driver\0").expect("driver");
        f(x, y);
    });
    let r_out = capture(|| unsafe {
        let lib = libloading::Library::new(&r_path).expect("dlopen Rust .so");
        let f: libloading::Symbol<common::DriverFn> = lib.get(b"driver\0").expect("driver");
        f(x, y);
    });
    assert_eq!(
        c_out, r_out,
        "row25: fresh-load single call differs for x={x} y={y}"
    );
}

/// Row 26 — the same input with fd 1 as a pipe (glibc line/full-buffering
/// differs from a regular file). Both libraries must emit the same bytes, and
/// the same bytes as in the regular-file capture.
#[test]
fn row26_pipe_vs_file_buffering() {
    let cases = [
        (0, 0),
        (0, -1),
        (0, i32::MAX),
        (i32::MAX, -1),
        (i32::MIN, i32::MIN),
        (12345, -6789),
    ];
    for &(x, y) in &cases {
        let c_pipe = capture_via_pipe(|| unsafe { c_driver()(x, y) });
        let r_pipe = capture_via_pipe(|| unsafe { rust_driver()(x, y) });
        assert_eq!(
            c_pipe, r_pipe,
            "row26: pipe-buffered output differs for x={x} y={y}"
        );

        let c_file = capture(|| unsafe { c_driver()(x, y) });
        assert_eq!(
            c_pipe, c_file,
            "row26: C output depends on the fd type for x={x} y={y}"
        );
        let r_file = capture(|| unsafe { rust_driver()(x, y) });
        assert_eq!(
            r_pipe, r_file,
            "row26: Rust output depends on the fd type for x={x} y={y}"
        );
    }
}

/// Harness self-check: confirms the capture machinery really observes the C
/// library's bytes, by checking the C output against an independent rendering of
/// `x | ~y`. The C remains the ground truth; this only guards against a capture
/// that silently returns nothing (which would make every comparison vacuous).
#[test]
fn harness_capture_is_not_vacuous() {
    for &(x, y) in &[(0i32, 0i32), (7, 3), (i32::MIN, i32::MAX), (0, i32::MAX)] {
        let c = capture(|| unsafe { c_driver()(x, y) });
        assert!(!c.is_empty(), "capture returned no bytes for x={x} y={y}");
        assert_eq!(
            String::from_utf8_lossy(&c),
            format!("{}\n", x | !y),
            "capture mismatch against independent rendering for x={x} y={y}"
        );
        let r = capture(|| unsafe { rust_driver()(x, y) });
        assert_eq!(c, r);
    }
}
