//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md` (rows 1 and 22 live in their own test
//! binaries because they require pristine global state). Every randomized row
//! uses the fixed `SEED` so failures reproduce exactly.

mod common;
use common::*;

const INT_MAX: i32 = i32::MAX;
const INT_MIN: i32 = i32::MIN;

/// Row 2 — `extra_bedrooms = 0` repeated: isolates the floors/bathrooms
/// accumulation from the bedrooms accumulator.
#[test]
fn row2_run_zero_repeated() {
    let mut h = lock();
    let before = h.bedrooms();
    for i in 0..64 {
        h.run(0, &format!("row2 i={i}"));
    }
    assert_eq!(h.bedrooms(), before, "zero adds must not change bedrooms");
}

/// Row 3 — small positive `extra_bedrooms` in 1..=100, randomized.
#[test]
fn row3_run_small_positive() {
    let mut h = lock();
    let mut rng = Rng::new(SEED ^ 3);
    for i in 0..200 {
        let v = rng.range_i32(1, 100);
        h.run(v, &format!("row3 i={i} v={v}"));
    }
}

/// Row 4 — small negative `extra_bedrooms` in -100..=-1: drives `bedrooms`
/// negative so `%d` must render a leading `-`.
#[test]
fn row4_run_small_negative() {
    let mut h = lock();
    let mut rng = Rng::new(SEED ^ 4);
    // Push the accumulator negative first, then keep it there.
    let arg = (-5000i32).wrapping_sub(h.bedrooms());
    h.run(arg, "row4 seed-negative");
    assert!(h.bedrooms() < 0);
    for i in 0..200 {
        let v = rng.range_i32(-100, -1);
        let out = h.run(v, &format!("row4 i={i} v={v}"));
        assert!(
            String::from_utf8_lossy(&out).contains("-"),
            "negative bedrooms must print a minus sign"
        );
    }
}

/// Row 5 — `+1` / `-1` boundary steps, alternating.
#[test]
fn row5_run_plus_minus_one_alternating() {
    let mut h = lock();
    for i in 0..100 {
        let v = if i % 2 == 0 { 1 } else { -1 };
        h.run(v, &format!("row5 i={i} v={v}"));
    }
}

/// Row 6 — uniformly random full-range `i32` (every bit pattern reachable).
#[test]
fn row6_run_uniform_full_range() {
    let mut h = lock();
    let mut rng = Rng::new(SEED ^ 6);
    for i in 0..400 {
        let v = rng.next_i32();
        h.run(v, &format!("row6 i={i} v={v}"));
    }
}

/// Row 7 — `extra_bedrooms = INT_MAX`, repeated (accumulator wraps each call).
#[test]
fn row7_run_int_max_repeated() {
    let mut h = lock();
    for i in 0..40 {
        let before = h.bedrooms();
        h.run(INT_MAX, &format!("row7 i={i}"));
        assert_eq!(h.bedrooms(), before.wrapping_add(INT_MAX));
    }
}

/// Row 8 — `extra_bedrooms = INT_MIN`, repeated.
#[test]
fn row8_run_int_min_repeated() {
    let mut h = lock();
    for i in 0..40 {
        let before = h.bedrooms();
        h.run(INT_MIN, &format!("row8 i={i}"));
        assert_eq!(h.bedrooms(), before.wrapping_add(INT_MIN));
    }
}

/// Row 9 — halves and a full powers-of-two sweep (k = 0..31), both signs.
#[test]
fn row9_run_halves_and_power_of_two_sweep() {
    let mut h = lock();
    h.run(INT_MAX / 2, "row9 INT_MAX/2");
    h.run(INT_MIN / 2, "row9 INT_MIN/2");
    for k in 0..32u32 {
        let v = 1i32.wrapping_shl(k); // includes k=31 -> INT_MIN
        h.run(v, &format!("row9 +2^{k} = {v}"));
        h.run(v.wrapping_neg(), &format!("row9 -2^{k} = {}", v.wrapping_neg()));
    }
}

/// Row 10 — values that push `bedrooms` past `INT_MAX` (positive -> negative wrap).
#[test]
fn row10_run_overflow_positive_to_negative() {
    let mut h = lock();
    let mut rng = Rng::new(SEED ^ 10);
    for i in 0..100 {
        // Park the accumulator just below INT_MAX, then step over it.
        let margin = rng.range_i32(0, 1000);
        let park = INT_MAX - margin;
        let to_park = park.wrapping_sub(h.bedrooms());
        h.run(to_park, &format!("row10 park i={i}"));
        assert_eq!(h.bedrooms(), park);

        let push = margin + rng.range_i32(1, 1000);
        h.run(push, &format!("row10 overflow i={i} push={push}"));
        assert!(
            h.bedrooms() < 0,
            "i={i}: expected wrap to negative, got {}",
            h.bedrooms()
        );
    }
}

/// Row 11 — values that push `bedrooms` below `INT_MIN` (negative -> positive wrap).
#[test]
fn row11_run_underflow_negative_to_positive() {
    let mut h = lock();
    let mut rng = Rng::new(SEED ^ 11);
    for i in 0..100 {
        let margin = rng.range_i32(0, 1000);
        let park = INT_MIN + margin;
        let to_park = park.wrapping_sub(h.bedrooms());
        h.run(to_park, &format!("row11 park i={i}"));
        assert_eq!(h.bedrooms(), park);

        let push = -(margin + rng.range_i32(1, 1000));
        h.run(push, &format!("row11 underflow i={i} push={push}"));
        assert!(
            h.bedrooms() > 0,
            "i={i}: expected wrap to positive, got {}",
            h.bedrooms()
        );
    }
}

/// Row 12 — `driver(0)`: the wrapper applies the identity add twice, 8 lines.
#[test]
fn row12_driver_zero() {
    let mut h = lock();
    let before = h.bedrooms();
    let f0 = h.floors();
    let b0 = h.bathrooms();
    let out = h.driver(0, "row12");
    assert_eq!(h.bedrooms(), before);
    assert_eq!(h.floors(), f0.wrapping_add(2), "driver adds two floors");
    assert_eq!(h.bathrooms(), b0 + 2.0, "driver adds two bathrooms");
    assert_eq!(
        String::from_utf8_lossy(&out).lines().count(),
        8,
        "driver prints 8 lines"
    );
}

/// Row 13 — `driver` with small positive / small negative randomized args.
#[test]
fn row13_driver_small_magnitudes() {
    let mut h = lock();
    let mut rng = Rng::new(SEED ^ 13);
    for i in 0..150 {
        let v = if i % 2 == 0 {
            rng.range_i32(1, 100)
        } else {
            rng.range_i32(-100, -1)
        };
        let before = h.bedrooms();
        h.driver(v, &format!("row13 i={i} v={v}"));
        assert_eq!(
            h.bedrooms(),
            before.wrapping_add(v).wrapping_add(v),
            "driver applies the value twice"
        );
    }
}

/// Row 14 — `driver` with uniformly random full-range `i32`.
#[test]
fn row14_driver_uniform_full_range() {
    let mut h = lock();
    let mut rng = Rng::new(SEED ^ 14);
    for i in 0..300 {
        let v = rng.next_i32();
        h.driver(v, &format!("row14 i={i} v={v}"));
    }
}

/// Row 15 — `driver(INT_MAX)` / `driver(INT_MIN)`: double wrap in one call.
#[test]
fn row15_driver_extremes() {
    let mut h = lock();
    for i in 0..30 {
        let before = h.bedrooms();
        h.driver(INT_MAX, &format!("row15 max i={i}"));
        assert_eq!(h.bedrooms(), before.wrapping_add(INT_MAX).wrapping_add(INT_MAX));

        let before = h.bedrooms();
        h.driver(INT_MIN, &format!("row15 min i={i}"));
        assert_eq!(h.bedrooms(), before.wrapping_add(INT_MIN).wrapping_add(INT_MIN));
    }
}

/// Row 16 — `driver` powers-of-two sweep.
#[test]
fn row16_driver_power_of_two_sweep() {
    let mut h = lock();
    for k in 0..32u32 {
        let v = 1i32.wrapping_shl(k);
        h.driver(v, &format!("row16 +2^{k}"));
        h.driver(v.wrapping_neg(), &format!("row16 -2^{k}"));
    }
}

/// Row 17 — the COMPOSED pipeline: randomized interleaving of both public
/// entry points over the shared global, with randomized args.
#[test]
fn row17_interleaved_run_and_driver_random() {
    let mut h = lock();
    let mut rng = Rng::new(SEED ^ 17);
    for i in 0..300 {
        let v = rng.next_i32();
        if rng.below(2) == 0 {
            h.run(v, &format!("row17 run i={i} v={v}"));
        } else {
            h.driver(v, &format!("row17 driver i={i} v={v}"));
        }
    }
}

/// Row 18 — interleaving with adversarial args so wraps land at arbitrary
/// points in the call sequence.
#[test]
fn row18_interleaved_adversarial_args() {
    let mut h = lock();
    let mut rng = Rng::new(SEED ^ 18);
    let args = [0i32, 1, -1, INT_MAX, INT_MIN, INT_MAX / 2, INT_MIN / 2, 2, -2];
    for i in 0..300 {
        let v = args[rng.below(args.len() as u64) as usize];
        if rng.below(2) == 0 {
            h.run(v, &format!("row18 run i={i} v={v}"));
        } else {
            h.driver(v, &format!("row18 driver i={i} v={v}"));
        }
    }
}

/// Row 19 — land the accumulator on exact boundary values.
#[test]
fn row19_exact_boundary_landings() {
    let mut h = lock();
    for &target in &[0i32, INT_MAX, INT_MIN, -1, 1, INT_MAX - 1, INT_MIN + 1] {
        let arg = target.wrapping_sub(h.bedrooms());
        h.run(arg, &format!("row19 land on {target} via {arg}"));
        assert_eq!(h.bedrooms(), target, "should land exactly on {target}");
        // And take one step past the boundary from exactly there.
        for &step in &[1i32, -1] {
            let before = h.bedrooms();
            h.run(step, &format!("row19 step {step} from {before}"));
            assert_eq!(h.bedrooms(), before.wrapping_add(step));
            h.run(step.wrapping_neg(), "row19 restore");
        }
    }
}

/// Row 20 — long endurance sequence: `floors` and `bathrooms` grow to
/// multi-digit / larger magnitude; verifies no drift in `%d` width or `%.1f`.
#[test]
fn row20_endurance_mixed_sequence() {
    let mut h = lock();
    let mut rng = Rng::new(SEED ^ 20);
    let f0 = h.floors();
    let b0 = h.bathrooms();
    let mut floors_added = 0i64;
    for i in 0..2000 {
        let v = rng.next_i32();
        if rng.below(3) == 0 {
            h.driver(v, &format!("row20 driver i={i}"));
            floors_added += 2;
        } else {
            h.run(v, &format!("row20 run i={i}"));
            floors_added += 1;
        }
    }
    assert_eq!(h.floors() as i64, f0 as i64 + floors_added);
    assert_eq!(h.bathrooms(), b0 + floors_added as f64);
    // The accumulated state really did reach multi-digit widths.
    assert!(h.floors() > 1000, "floors grew: {}", h.floors());
    assert!(h.bathrooms() > 1000.0, "bathrooms grew: {}", h.bathrooms());
}

/// Row 21 — output-shape sweep: force `bedrooms` to render 1-, 2-, 5- and
/// 10-digit forms, positive and negative.
#[test]
fn row21_printed_width_sweep() {
    let mut h = lock();
    let targets = [
        7i32,
        -7,
        42,
        -42,
        12345,
        -12345,
        1_234_567_890,
        -1_234_567_890,
        INT_MAX,
        INT_MIN,
        0,
    ];
    for &t in &targets {
        let arg = t.wrapping_sub(h.bedrooms());
        let out = h.run(arg, &format!("row21 target={t}"));
        assert_eq!(h.bedrooms(), t);
        // The last of the four printed lines shows the post-add value.
        let s = String::from_utf8_lossy(&out);
        let last = s.lines().last().unwrap();
        assert!(
            last.contains(&format!("{t} bedrooms")),
            "expected `{t} bedrooms` in {last:?}"
        );
    }
}
