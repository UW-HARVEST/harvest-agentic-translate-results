//! Phase C — error/boundary-path differential tests, one per `ERRORS.md` row.
//!
//! The C library has no error returns, no sentinels, no asserts and no pointer
//! or length parameters (see `ERRORS.md` for the mechanical derivation), so
//! each row asserts that C and Rust agree **bit-for-bit** on the value/output
//! produced for the boundary input — that being the entirety of this API's
//! observable accept/reject behaviour.

mod harness;

use harness::{boundary_i32s, Pair, Rng};

/// Row 1 — `update == INT_MAX` on a fresh instance.
#[test]
fn err01_static_sum_int_max_fresh() {
    let p = Pair::fresh();
    let got = p.check_static_sum(i32::MAX, "err01");
    assert_eq!(got, i32::MAX, "C did not simply return INT_MAX");
}

/// Row 2 — `update == INT_MIN` on a fresh instance.
#[test]
fn err02_static_sum_int_min_fresh() {
    let p = Pair::fresh();
    let got = p.check_static_sum(i32::MIN, "err02");
    assert_eq!(got, i32::MIN);
}

/// Row 3 — positive signed overflow: INT_MAX then +1 must wrap to INT_MIN.
#[test]
fn err03_static_sum_positive_overflow() {
    let p = Pair::fresh();
    assert_eq!(p.check_static_sum(i32::MAX, "err03 a"), i32::MAX);
    let got = p.check_static_sum(1, "err03 b");
    assert_eq!(got, i32::MIN, "positive overflow did not wrap to INT_MIN");
}

/// Row 4 — negative signed overflow: INT_MIN then -1 must wrap to INT_MAX.
#[test]
fn err04_static_sum_negative_overflow() {
    let p = Pair::fresh();
    assert_eq!(p.check_static_sum(i32::MIN, "err04 a"), i32::MIN);
    let got = p.check_static_sum(-1, "err04 b");
    assert_eq!(got, i32::MAX, "negative overflow did not wrap to INT_MAX");
}

/// Row 5 — `update == 0` is the identity on any accumulated state.
#[test]
fn err05_static_sum_zero_is_identity() {
    let mut rng = Rng::new(0x0500_0005);
    for i in 0..128 {
        let p = Pair::fresh();
        let seed = rng.i32_any();
        let s = p.check_static_sum(seed, &format!("err05 #{i} seed"));
        for k in 0..4 {
            let z = p.check_static_sum(0, &format!("err05 #{i} zero {k}"));
            assert_eq!(z, s, "zero update changed the accumulator");
        }
    }
}

/// Row 6 — INT_MAX applied repeatedly: many consecutive overflows.
#[test]
fn err06_static_sum_repeated_int_max() {
    let p = Pair::fresh();
    for i in 0..64 {
        p.check_static_sum(i32::MAX, &format!("err06 step {i}"));
    }
    let q = Pair::fresh();
    for i in 0..64 {
        q.check_static_sum(i32::MIN, &format!("err06 min step {i}"));
    }
}

/// Row 7 — one step past the extremes from an already-extreme state.
#[test]
fn err07_static_sum_one_past_range() {
    let p = Pair::fresh();
    assert_eq!(p.check_static_sum(i32::MAX - 1, "err07 a"), i32::MAX - 1);
    let got = p.check_static_sum(2, "err07 b");
    assert_eq!(got, i32::MIN, "INT_MAX-1 + 2 did not wrap to INT_MIN");

    let q = Pair::fresh();
    assert_eq!(q.check_static_sum(i32::MIN + 1, "err07 c"), i32::MIN + 1);
    let got = q.check_static_sum(-2, "err07 d");
    assert_eq!(got, i32::MAX, "INT_MIN+1 - 2 did not wrap to INT_MAX");
}

/// Row 8 — `driver(INT_MAX)`: `i * stride` overflows for every `i >= 2`.
#[test]
fn err08_driver_stride_int_max() {
    let p = Pair::fresh();
    let out = p.check_driver(i32::MAX, "err08");
    assert_eq!(out.iter().filter(|&&b| b == b'\n').count(), 10);
}

/// Row 9 — `driver(INT_MIN)`.
#[test]
fn err09_driver_stride_int_min() {
    let p = Pair::fresh();
    let out = p.check_driver(i32::MIN, "err09");
    assert_eq!(out.iter().filter(|&&b| b == b'\n').count(), 10);
}

/// Row 10 — degenerate `driver(0)` on both fresh and dirty state.
#[test]
fn err10_driver_stride_zero() {
    let p = Pair::fresh();
    assert_eq!(p.check_driver(0, "err10 fresh"), b"0\n".repeat(10));

    let mut rng = Rng::new(0x1000_0010);
    for i in 0..64 {
        let q = Pair::fresh();
        let s = q.check_static_sum(rng.i32_any(), &format!("err10 #{i} seed"));
        let out = q.check_driver(0, &format!("err10 #{i} dirty"));
        assert_eq!(out, format!("{s}\n").repeat(10).into_bytes());
    }
}

/// Row 11 — `driver(-1)` / `driver(1)`, one step either side of 0.
#[test]
fn err11_driver_stride_pm_one() {
    for s in [-1i32, 1] {
        let p = Pair::fresh();
        let out = p.check_driver(s, &format!("err11 stride={s}"));
        assert_eq!(out.iter().filter(|&&b| b == b'\n').count(), 10);
    }
}

/// Row 12 — strides where the *accumulated sum* overflows mid-loop.
#[test]
fn err12_driver_sum_overflows_midloop() {
    for s in [
        i32::MAX / 8,
        0x2000_0000i32,
        0x1555_5555,
        -(i32::MAX / 8),
        -0x2000_0000,
    ] {
        let p = Pair::fresh();
        let out = p.check_driver(s, &format!("err12 stride={s}"));
        assert!(
            out.iter().filter(|&&b| b == b'\n').count() == 10,
            "driver did not print 10 lines"
        );
    }
}

/// Row 13 — `driver` from an extreme pre-existing state.
#[test]
fn err13_driver_from_extreme_state() {
    for seed in [i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1] {
        for stride in [0i32, 1, -1, i32::MAX, i32::MIN, 7, -7] {
            let p = Pair::fresh();
            p.check_static_sum(seed, &format!("err13 seed={seed}"));
            p.check_driver(stride, &format!("err13 seed={seed} stride={stride}"));
        }
    }
}

/// Row 14 — there is no out-of-range representation to pass.
///
/// Neither public function takes an enum, a pointer, a length or a buffer:
/// `int static_sum(int)` and `void driver(int)`.  Every one of the 2^32 `int`
/// bit patterns is therefore a valid, in-range argument, and null-pointer /
/// zero-length / oversized-length boundaries are structurally inapplicable.
/// This test pins that fact to the actual header text (so it fails loudly if
/// the C API ever grows a pointer/enum parameter) and then sweeps every
/// interesting bit pattern of the domain through both entry points.
#[test]
fn err14_no_out_of_range_representation() {
    let hdr = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("c_src/include/staticloop.h");
    let text = std::fs::read_to_string(&hdr).expect("read staticloop.h");
    let decls: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| l.contains('(') && l.ends_with(';') && !l.starts_with("//"))
        .collect();
    assert_eq!(
        decls,
        vec!["int static_sum(int update);", "void driver(int update);"],
        "the public API changed; ERRORS.md row 14 must be re-derived"
    );
    for d in &decls {
        assert!(!d.contains('*'), "a pointer parameter appeared in {d:?}");
        assert!(!d.contains("enum"), "an enum parameter appeared in {d:?}");
        assert!(!d.contains("size_t"), "a length parameter appeared in {d:?}");
    }

    // Exhaustive interesting-bit-pattern sweep through both entry points.
    for &v in boundary_i32s().iter() {
        let p = Pair::fresh();
        p.check_static_sum(v, &format!("err14 static_sum({v})"));
        let q = Pair::fresh();
        q.check_driver(v, &format!("err14 driver({v})"));
    }
}

/// Row 15 — full-domain adversarial sweep.
#[test]
fn err15_full_int_domain_sweep() {
    // Long shared-state sequence over the full domain.
    let mut rng = Rng::new(0x1500_0015);
    let p = Pair::fresh();
    for i in 0..5000 {
        let v = match rng.below(4) {
            0 => *boundary_i32s()
                .get(rng.below(boundary_i32s().len() as u64) as usize)
                .unwrap(),
            1 => rng.i32_in(-3, 3),
            _ => rng.i32_any(),
        };
        p.check_static_sum(v, &format!("err15 step {i} value {v}"));
    }

    // And the same domain fed to `driver` as strides, fresh instance each.
    let mut rng = Rng::new(0x1500_0016);
    for i in 0..128 {
        let s = rng.i32_any();
        let q = Pair::fresh();
        q.check_driver(s, &format!("err15 driver #{i} stride={s}"));
    }
}
