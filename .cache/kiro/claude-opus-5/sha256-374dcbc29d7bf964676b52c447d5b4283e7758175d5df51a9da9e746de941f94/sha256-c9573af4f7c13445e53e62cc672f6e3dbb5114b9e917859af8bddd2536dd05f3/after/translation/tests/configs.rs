//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md` (C2..C22; C1 lives in `initial_state.rs`
//! because it needs a pristine process). Every test drives both `.so`s through
//! their exported symbols only.

mod common;

use common::{LANDMARKS, Rng, with_libs};
use std::ffi::c_int;

// --------------------------------------------------------------------------
// C2 — many small positive updates, lowest-level entry point only
// --------------------------------------------------------------------------
#[test]
fn c2_many_small_positive() {
    with_libs(|h| {
        for i in 1..=1000 {
            h.static_sum(i as c_int, "C2");
        }
    });
}

// --------------------------------------------------------------------------
// C3 — many small negative updates
// --------------------------------------------------------------------------
#[test]
fn c3_many_small_negative() {
    with_libs(|h| {
        for i in 1..=1000 {
            h.static_sum(-(i as c_int), "C3");
        }
    });
}

// --------------------------------------------------------------------------
// C4 — mixed-sign small updates, accumulator crosses zero repeatedly
// --------------------------------------------------------------------------
#[test]
fn c4_mixed_sign_random() {
    let mut rng = Rng::new(0x5EED_1004);
    with_libs(|h| {
        for _ in 0..3000 {
            let v = rng.next_in_range(-1000, 1000) as c_int;
            h.static_sum(v, "C4");
        }
    });
}

// --------------------------------------------------------------------------
// C5 — repeated no-op updates: accumulator must be perfectly stable
// --------------------------------------------------------------------------
#[test]
fn c5_repeated_zero() {
    with_libs(|h| {
        let first = h.static_sum(0, "C5");
        for _ in 0..200 {
            let v = h.static_sum(0, "C5");
            assert_eq!(v, first, "C5: static_sum(0) changed the accumulator");
        }
    });
}

// --------------------------------------------------------------------------
// C6 — full-range random updates (fixed seed), overflow in both directions
// --------------------------------------------------------------------------
#[test]
fn c6_full_range_random() {
    let mut rng = Rng::new(0x5EED_0001);
    with_libs(|h| {
        for _ in 0..4000 {
            let v = rng.next_c_int();
            h.static_sum(v, "C6");
        }
    });
}

// --------------------------------------------------------------------------
// C7 — extremal updates alternating: maximal wraparound pressure
// --------------------------------------------------------------------------
#[test]
fn c7_extremal_alternating() {
    with_libs(|h| {
        for i in 0..200 {
            let v = if i % 2 == 0 { c_int::MAX } else { c_int::MIN };
            h.static_sum(v, "C7");
        }
    });
}

// --------------------------------------------------------------------------
// C8 — accumulator parked near INT_MAX, then a full landmark sweep
// --------------------------------------------------------------------------
#[test]
fn c8_near_int_max_sweep() {
    with_libs(|h| {
        for park in [c_int::MAX - 2, c_int::MAX - 1, c_int::MAX] {
            for &u in LANDMARKS {
                h.park_accumulator_at(park, "C8");
                h.static_sum(u, "C8");
            }
        }
    });
}

// --------------------------------------------------------------------------
// C9 — accumulator parked near INT_MIN, then a full landmark sweep
// --------------------------------------------------------------------------
#[test]
fn c9_near_int_min_sweep() {
    with_libs(|h| {
        for park in [c_int::MIN, c_int::MIN + 1, c_int::MIN + 2] {
            for &u in LANDMARKS {
                h.park_accumulator_at(park, "C9");
                h.static_sum(u, "C9");
            }
        }
    });
}

// --------------------------------------------------------------------------
// C10 — wrapper alone, canonical stride = 1
// --------------------------------------------------------------------------
#[test]
fn c10_driver_stride_one() {
    with_libs(|h| {
        let out = h.driver(1, "C10");
        // Sanity check on the shape of the output, independent of the
        // accumulated state: exactly 10 newline-terminated decimal lines.
        assert_eq!(
            out.iter().filter(|&&b| b == b'\n').count(),
            10,
            "C10: driver must print exactly 10 lines, got {:?}",
            String::from_utf8_lossy(&out)
        );
    });
}

// --------------------------------------------------------------------------
// C11 — wrapper alone, degenerate stride = 0 (10 identical lines)
// --------------------------------------------------------------------------
#[test]
fn c11_driver_stride_zero() {
    with_libs(|h| {
        let out = h.driver(0, "C11");
        let lines: Vec<&[u8]> = out.split(|&b| b == b'\n').collect();
        // last element is the empty tail after the final '\n'
        assert_eq!(lines.len(), 11, "C11: expected 10 lines");
        assert!(
            lines[..10].windows(2).all(|w| w[0] == w[1]),
            "C11: stride 0 must print the same value 10 times, got {:?}",
            String::from_utf8_lossy(&out)
        );
    });
}

// --------------------------------------------------------------------------
// C12 — wrapper alone, small negative stride (exercises the '-' sign bytes)
// --------------------------------------------------------------------------
#[test]
fn c12_driver_small_negative() {
    with_libs(|h| {
        h.park_accumulator_at(0, "C12");
        let out = h.driver(-3, "C12");
        assert!(
            out.contains(&b'-'),
            "C12: expected negative output, got {:?}",
            String::from_utf8_lossy(&out)
        );
        for &s in &[-1, -2, -5, -100, -99991] {
            h.driver(s, "C12");
        }
    });
}

// --------------------------------------------------------------------------
// C13 — wrapper alone, small positive random strides (fixed seed)
// --------------------------------------------------------------------------
#[test]
fn c13_driver_small_positive_random() {
    let mut rng = Rng::new(0x5EED_0002);
    with_libs(|h| {
        for _ in 0..200 {
            let s = rng.next_in_range(1, 10_000) as c_int;
            h.driver(s, "C13");
        }
    });
}

// --------------------------------------------------------------------------
// C14 — wrapper alone, full-range random strides: `i * stride` overflows
// --------------------------------------------------------------------------
#[test]
fn c14_driver_full_range_random() {
    let mut rng = Rng::new(0x5EED_0003);
    with_libs(|h| {
        for _ in 0..300 {
            let s = rng.next_c_int();
            h.driver(s, "C14");
        }
    });
}

// --------------------------------------------------------------------------
// C15 — wrapper alone, extremal strides
// --------------------------------------------------------------------------
#[test]
fn c15_driver_extremal_strides() {
    with_libs(|h| {
        for &s in &[
            c_int::MAX,
            c_int::MIN,
            1,
            -1,
            c_int::MAX / 9,
            c_int::MIN / 9,
            c_int::MAX / 10,
            c_int::MIN / 10,
        ] {
            h.driver(s, "C15");
        }
    });
}

// --------------------------------------------------------------------------
// C16 — many consecutive driver calls: state carried across calls
// --------------------------------------------------------------------------
#[test]
fn c16_driver_repeated_same_stride() {
    with_libs(|h| {
        for stride in [1, 7, -7, 123_456] {
            for _ in 0..40 {
                h.driver(stride, "C16");
            }
        }
    });
}

// --------------------------------------------------------------------------
// C17 — interleaving: low-level entry point first, then the wrapper
// --------------------------------------------------------------------------
#[test]
fn c17_static_sum_then_driver() {
    let mut rng = Rng::new(0x5EED_0017);
    with_libs(|h| {
        for _ in 0..100 {
            h.static_sum(rng.next_c_int(), "C17");
            h.static_sum(rng.next_in_range(-50, 50) as c_int, "C17");
            h.driver(rng.next_in_range(-1000, 1000) as c_int, "C17");
        }
    });
}

// --------------------------------------------------------------------------
// C18 — interleaving: wrapper first, then the low-level entry point observes
//        the 10 in-loop updates it performed
// --------------------------------------------------------------------------
#[test]
fn c18_driver_then_static_sum() {
    with_libs(|h| {
        for stride in [0, 1, -1, 5, -5, 1_000_000] {
            h.park_accumulator_at(0, "C18");
            h.driver(stride, "C18");
            // After driver(stride) from 0 the accumulator must be
            // stride * (0+1+..+9) = 45*stride (wrapping); static_sum(0) reads it.
            let observed = h.static_sum(0, "C18");
            assert_eq!(
                observed,
                (stride as c_int).wrapping_mul(45),
                "C18: accumulator after driver({stride}) is wrong"
            );
        }
    });
}

// --------------------------------------------------------------------------
// C19 — fully randomized interleaving of both entry points (pipeline row)
// --------------------------------------------------------------------------
#[test]
fn c19_random_interleaved_pipeline() {
    let mut rng = Rng::new(0x5EED_0004);
    with_libs(|h| {
        for _ in 0..2000 {
            match rng.next_u32() % 4 {
                0 => {
                    h.static_sum(rng.next_c_int(), "C19");
                }
                1 => {
                    h.static_sum(rng.next_in_range(-16, 16) as c_int, "C19");
                }
                2 => {
                    h.driver(rng.next_in_range(-64, 64) as c_int, "C19");
                }
                _ => {
                    h.driver(rng.next_c_int(), "C19");
                }
            }
        }
    });
}

// --------------------------------------------------------------------------
// C20 — accumulator parked at a wrap boundary so the wrap happens *inside*
//        the wrapper's loop
// --------------------------------------------------------------------------
#[test]
fn c20_driver_wrap_inside_loop() {
    with_libs(|h| {
        let parks = [
            c_int::MAX,
            c_int::MAX - 1,
            c_int::MAX - 44,
            c_int::MAX - 45,
            c_int::MIN,
            c_int::MIN + 1,
            c_int::MIN + 44,
            c_int::MIN + 45,
            -1,
            0,
        ];
        let strides = [1, -1, 2, -2, c_int::MAX, c_int::MIN, 47_721_858];
        for &p in &parks {
            for &s in &strides {
                h.park_accumulator_at(p, "C20");
                h.driver(s, "C20");
            }
        }
    });
}

// --------------------------------------------------------------------------
// C21 — maximum-width decimal output (10 digits, with and without sign)
// --------------------------------------------------------------------------
#[test]
fn c21_driver_max_width_output() {
    with_libs(|h| {
        // 45 * 47_721_858 = 2_147_483_610, just under INT_MAX: from a zero
        // accumulator the last printed line is 10 digits wide.
        h.park_accumulator_at(0, "C21");
        let out = h.driver(47_721_858, "C21");
        let last = out
            .split(|&b| b == b'\n')
            .filter(|s| !s.is_empty())
            .next_back()
            .expect("C21: driver printed nothing");
        assert_eq!(
            last,
            b"2147483610",
            "C21: unexpected widest line {:?}",
            String::from_utf8_lossy(last)
        );

        // And the negative-sign 11-byte form.
        h.park_accumulator_at(0, "C21");
        let out = h.driver(-47_721_858, "C21");
        let last = out
            .split(|&b| b == b'\n')
            .filter(|s| !s.is_empty())
            .next_back()
            .expect("C21: driver printed nothing");
        assert_eq!(
            last,
            b"-2147483610",
            "C21: unexpected widest negative line {:?}",
            String::from_utf8_lossy(last)
        );
    });
}

// --------------------------------------------------------------------------
// C22 — engineer the return value onto every interesting landmark
// --------------------------------------------------------------------------
#[test]
fn c22_exact_return_landmarks() {
    with_libs(|h| {
        for &target in LANDMARKS {
            h.park_accumulator_at(target, "C22");
            let v = h.static_sum(0, "C22");
            assert_eq!(v, target, "C22: accumulator not parked at {target}");
        }
    });
}
