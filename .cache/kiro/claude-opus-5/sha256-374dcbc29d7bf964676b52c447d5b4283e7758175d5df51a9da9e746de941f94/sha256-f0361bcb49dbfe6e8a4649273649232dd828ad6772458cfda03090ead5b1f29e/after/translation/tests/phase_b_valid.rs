//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md` (C1..C27). Every test drives BOTH shared
//! objects through their exported symbols and compares results byte-for-byte;
//! most also compare against an independent reference `Model` of the C code.

mod common;

use common::{harness, Model, Rng};

// ---------------------------------------------------------------------------
// C1..C3 — single low-level call, canonical value classes.
// ---------------------------------------------------------------------------

#[test]
fn c1_static_sum_zero_update() {
    let mut h = harness();
    let before = h.peek();
    let got = h.static_sum(0);
    assert_eq!(got, before, "update=0 must leave the accumulator unchanged");
}

#[test]
fn c2_static_sum_plus_one() {
    let mut h = harness();
    let before = h.peek();
    let got = h.static_sum(1);
    assert_eq!(got, before.wrapping_add(1));
}

#[test]
fn c3_static_sum_minus_one() {
    let mut h = harness();
    let before = h.peek();
    let got = h.static_sum(-1);
    assert_eq!(got, before.wrapping_sub(1));
}

// ---------------------------------------------------------------------------
// C4..C6 — randomized runs over the three value classes.
// ---------------------------------------------------------------------------

fn randomized_static_sum(seed: u64, lo: i32, hi: i32, iters: usize) {
    let mut h = harness();
    let mut rng = Rng::new(seed);
    let mut model = Model::new(h.peek());
    for _ in 0..iters {
        let update = rng.range_i32(lo, hi);
        let got = h.static_sum(update);
        assert_eq!(
            got,
            model.static_sum(update),
            "static_sum({update}) disagreed with the reference model"
        );
    }
}

#[test]
fn c4_static_sum_random_small_positive() {
    randomized_static_sum(0xC4_0000_0001, 1, 1_000_000, 500);
}

#[test]
fn c5_static_sum_random_small_negative() {
    randomized_static_sum(0xC5_0000_0002, -1_000_000, -1, 500);
}

#[test]
fn c6_static_sum_random_full_range() {
    randomized_static_sum(0xC6_0000_0003, i32::MIN, i32::MAX, 1000);
}

// ---------------------------------------------------------------------------
// C7..C10 — accumulator arithmetic edges.
// ---------------------------------------------------------------------------

/// Moves the (shared) accumulator to exactly `target`, mirrored into both libs.
fn drive_to(h: &mut common::Harness, target: i32) {
    let cur = h.peek();
    let delta = target.wrapping_sub(cur);
    let got = h.static_sum(delta);
    assert_eq!(got, target, "failed to position the accumulator at {target}");
}

#[test]
fn c7_accumulator_positive_overflow() {
    let mut h = harness();
    drive_to(&mut h, i32::MAX - 3);
    let mut model = Model::new(i32::MAX - 3);
    for update in [1, 1, 1, 1, 7, i32::MAX] {
        assert_eq!(h.static_sum(update), model.static_sum(update));
    }
}

#[test]
fn c8_accumulator_negative_overflow() {
    let mut h = harness();
    drive_to(&mut h, i32::MIN + 3);
    let mut model = Model::new(i32::MIN + 3);
    for update in [-1, -1, -1, -1, -7, i32::MIN] {
        assert_eq!(h.static_sum(update), model.static_sum(update));
    }
}

#[test]
fn c9_static_sum_int_max_twice() {
    let mut h = harness();
    drive_to(&mut h, 0);
    let mut model = Model::new(0);
    assert_eq!(h.static_sum(i32::MAX), model.static_sum(i32::MAX));
    assert_eq!(h.static_sum(i32::MAX), model.static_sum(i32::MAX));
}

#[test]
fn c10_static_sum_int_min_twice() {
    let mut h = harness();
    drive_to(&mut h, 0);
    let mut model = Model::new(0);
    assert_eq!(h.static_sum(i32::MIN), model.static_sum(i32::MIN));
    assert_eq!(h.static_sum(i32::MIN), model.static_sum(i32::MIN));
}

// ---------------------------------------------------------------------------
// C11 — long cumulative run.
// ---------------------------------------------------------------------------

#[test]
fn c11_long_run_cumulative_identity() {
    let mut h = harness();
    let mut rng = Rng::new(0xC11_0000_0004);
    let start = h.peek();
    let mut model = Model::new(start);
    for _ in 0..2000 {
        let update = rng.next_i32();
        assert_eq!(h.static_sum(update), model.static_sum(update));
    }
    assert_eq!(h.peek(), model.sum, "final accumulator state diverged");
}

// ---------------------------------------------------------------------------
// C12..C14 — driver with canonical strides.
// ---------------------------------------------------------------------------

/// Runs `driver(stride)` on both libs and against the model; returns the bytes.
fn check_driver(h: &mut common::Harness, stride: i32) -> Vec<u8> {
    let before = h.peek();
    let mut model = Model::new(before);
    let expected = model.driver(stride);
    let got = h.driver(stride);
    assert_eq!(
        String::from_utf8_lossy(&got),
        String::from_utf8_lossy(&expected),
        "driver({stride}) stdout disagreed with the reference model (accumulator was {before})"
    );
    assert_eq!(h.peek(), model.sum, "driver({stride}) left a different accumulator state");
    got
}

#[test]
fn c12_driver_zero_stride() {
    let mut h = harness();
    let before = h.peek();
    let out = check_driver(&mut h, 0);
    // All ten addends are 0, so the accumulator never moves.
    let expected: String = std::iter::repeat(format!("{before}\n")).take(10).collect();
    assert_eq!(out, expected.as_bytes());
    assert_eq!(h.peek(), before);
}

#[test]
fn c13_driver_stride_one() {
    let mut h = harness();
    let before = h.peek();
    let out = check_driver(&mut h, 1);
    // Addends are 0..9, so the printed values are `before + i*(i+1)/2`.
    let mut expected = String::new();
    let mut acc = before;
    for i in 0..10i32 {
        acc = acc.wrapping_add(i);
        expected.push_str(&format!("{acc}\n"));
    }
    assert_eq!(out, expected.as_bytes());
    assert_eq!(h.peek(), before.wrapping_add(45));
}

#[test]
fn c14_driver_stride_minus_one() {
    let mut h = harness();
    let before = h.peek();
    check_driver(&mut h, -1);
    assert_eq!(h.peek(), before.wrapping_sub(45));
}

// ---------------------------------------------------------------------------
// C15..C17 — randomized strides.
// ---------------------------------------------------------------------------

#[test]
fn c15_driver_random_small_positive_stride() {
    let mut h = harness();
    let mut rng = Rng::new(0xC15_0000_0005);
    for _ in 0..100 {
        check_driver(&mut h, rng.range_i32(1, 100_000));
    }
}

#[test]
fn c16_driver_random_small_negative_stride() {
    let mut h = harness();
    let mut rng = Rng::new(0xC16_0000_0006);
    for _ in 0..100 {
        check_driver(&mut h, rng.range_i32(-100_000, -1));
    }
}

#[test]
fn c17_driver_random_full_range_stride() {
    let mut h = harness();
    let mut rng = Rng::new(0xC17_0000_0007);
    for _ in 0..150 {
        check_driver(&mut h, rng.next_i32());
    }
}

// ---------------------------------------------------------------------------
// C18..C20 — multiplication-overflow boundary for `i * stride`.
// ---------------------------------------------------------------------------

#[test]
fn c18_driver_largest_non_overflowing_stride() {
    let mut h = harness();
    let stride = i32::MAX / 9; // 9 * stride still fits in i32
    assert!(9i64 * stride as i64 <= i32::MAX as i64);
    check_driver(&mut h, stride);
}

#[test]
fn c19_driver_first_overflowing_stride() {
    let mut h = harness();
    let stride = i32::MAX / 9 + 1; // 9 * stride no longer fits
    assert!(9i64 * stride as i64 > i32::MAX as i64);
    check_driver(&mut h, stride);
    check_driver(&mut h, -(i32::MAX / 9) - 1);
}

#[test]
fn c20_driver_extreme_strides() {
    let mut h = harness();
    for stride in [i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1] {
        check_driver(&mut h, stride);
    }
}

// ---------------------------------------------------------------------------
// C21..C24 — state carried across whole pipeline runs, and interleavings.
// ---------------------------------------------------------------------------

#[test]
fn c21_driver_repeated_same_stride() {
    let mut h = harness();
    let stride = 7;
    let mut outs = Vec::new();
    for _ in 0..5 {
        outs.push(check_driver(&mut h, stride));
    }
    // The runs must differ from one another: proof the shared accumulator is
    // genuinely carried across calls (and identically in both libraries).
    assert_ne!(outs[0], outs[1], "driver output must depend on carried-in state");
}

#[test]
fn c22_driver_on_non_fresh_accumulator() {
    let mut h = harness();
    let mut rng = Rng::new(0xC22_0000_0008);
    for _ in 0..40 {
        for _ in 0..3 {
            h.static_sum(rng.next_i32());
        }
        check_driver(&mut h, rng.range_i32(-5000, 5000));
    }
}

#[test]
fn c23_interleaved_entry_points() {
    let mut h = harness();
    let mut rng = Rng::new(0xC23_0000_0009);
    let mut model = Model::new(h.peek());
    for _ in 0..120 {
        let u = rng.next_i32();
        assert_eq!(h.static_sum(u), model.static_sum(u));
        let s = rng.next_i32();
        assert_eq!(h.driver(s), model.driver(s));
    }
}

#[test]
fn c24_accumulator_state_after_driver() {
    let mut h = harness();
    let mut rng = Rng::new(0xC24_0000_000A);
    for _ in 0..60 {
        let stride = rng.next_i32();
        let before = h.peek();
        h.driver(stride);
        // Recompute what the accumulator must now be, and confirm both libs
        // report it (peek() itself asserts C == Rust).
        let mut expected = before;
        for i in 0..10i32 {
            expected = expected.wrapping_add(i.wrapping_mul(stride));
        }
        assert_eq!(h.peek(), expected, "hidden state after driver({stride}) diverged");
    }
}

// ---------------------------------------------------------------------------
// C25 — exact `"%d\n"` byte formatting.
// ---------------------------------------------------------------------------

#[test]
fn c25_driver_exact_stdout_formatting() {
    let mut h = harness();
    // Position the accumulator so that the run prints negative values, zero,
    // and positive values, exercising the sign and width behaviour of "%d".
    drive_to(&mut h, -20);
    let out = h.driver(1);
    let text = String::from_utf8(out.clone()).expect("stdout must be valid ASCII");
    let lines: Vec<&str> = text.split_terminator('\n').collect();
    assert_eq!(lines.len(), 10, "driver must print exactly 10 lines");
    assert!(text.ends_with('\n'), "each line ends with a newline");
    assert!(!text.contains("\r"), "no CR: the format string is \"%d\\n\"");
    for l in &lines {
        assert!(!l.is_empty());
        assert!(!l.starts_with(' ') && !l.ends_with(' '), "no padding in \"%d\"");
        assert!(
            l.parse::<i32>().is_ok(),
            "each line must be a bare decimal i32, got {l:?}"
        );
    }
    // -20, -20+1, ... : the first few are negative, later ones positive.
    assert_eq!(lines[0], "-20");
    assert_eq!(lines[9], format!("{}", -20 + 45));
}

// ---------------------------------------------------------------------------
// C27 — fixed-seed randomized program over both entry points.
// ---------------------------------------------------------------------------

#[test]
fn c27_randomized_program() {
    let mut h = harness();
    let mut rng = Rng::new(0xC27_0000_000B);
    let mut model = Model::new(h.peek());
    for step in 0..2000 {
        if rng.next_u64() % 4 == 0 {
            let stride = match rng.next_u64() % 8 {
                0 => 0,
                1 => 1,
                2 => -1,
                3 => i32::MAX,
                4 => i32::MIN,
                5 => i32::MAX / 9 + 1,
                6 => rng.range_i32(-1000, 1000),
                _ => rng.next_i32(),
            };
            assert_eq!(h.driver(stride), model.driver(stride), "step {step}: driver({stride})");
        } else {
            let update = match rng.next_u64() % 6 {
                0 => 0,
                1 => i32::MAX,
                2 => i32::MIN,
                3 => -1,
                4 => rng.range_i32(-1000, 1000),
                _ => rng.next_i32(),
            };
            assert_eq!(
                h.static_sum(update),
                model.static_sum(update),
                "step {step}: static_sum({update})"
            );
        }
    }
    assert_eq!(h.peek(), model.sum);
}
