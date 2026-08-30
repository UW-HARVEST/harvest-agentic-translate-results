//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test drives BOTH shared objects through `dlopen`/`dlsym` and compares
//! the stdout bytes. `driver` is simultaneously the highest and the lowest
//! level entry point of this library (there is nothing beneath it and no
//! options to set), so calling it directly IS the low-level path.

mod harness;
use harness::*;

// --- C1: identity input -----------------------------------------------------

#[test]
fn c1_zero() {
    assert_same("C1", &[0]);
}

// --- C2 / C3: small magnitudes, exhaustive ----------------------------------

#[test]
fn c2_small_positive_exhaustive() {
    assert_same("C2", &inclusive(1, 1000));
}

#[test]
fn c3_small_negative_exhaustive() {
    assert_same("C3", &inclusive(-1000, -1));
}

// --- C4: exact zero-crossing of the result ----------------------------------

#[test]
fn c4_result_zero_crossing() {
    // y = 2x + 300 == 0  <=>  x == -150
    let xs = around(-150, 8);
    assert_same("C4", &xs);
    // Pin the semantics of the crossing explicitly.
    assert_eq!(expected_line(-150), "0\n");
    assert_eq!(expected_line(-151), "-2\n");
    assert_eq!(expected_line(-149), "2\n");
}

// --- C5 / C6 / C7: printf field-width transitions ---------------------------

#[test]
fn c5_one_digit_results() {
    // y in [0, 9]  <=>  x in [-150, -145] (y even, so 0,2,4,6,8)
    assert_same("C5", &inclusive(-150, -145));
}

#[test]
fn c6_positive_digit_width_transitions() {
    // For each power of ten, the x that lands y just below / at / above it.
    let mut xs = Vec::new();
    let mut p: i64 = 1;
    while p <= 1_000_000_000 {
        for delta in [-2i64, -1, 0, 1, 2] {
            let y = p + delta;
            // x = (y - 300) / 2, keeping only exactly representable results
            let num = y - 300;
            if num % 2 == 0 {
                let x = num / 2;
                if x >= i32::MIN as i64 && x <= i32::MAX as i64 {
                    xs.push(x as i32);
                }
            }
        }
        p *= 10;
    }
    xs.sort_unstable();
    xs.dedup();
    assert_same("C6", &xs);
}

#[test]
fn c7_negative_digit_width_transitions() {
    let mut xs = Vec::new();
    let mut p: i64 = 1;
    while p <= 1_000_000_000 {
        for delta in [-2i64, -1, 0, 1, 2] {
            let y = -(p + delta);
            let num = y - 300;
            if num % 2 == 0 {
                let x = num / 2;
                if x >= i32::MIN as i64 && x <= i32::MAX as i64 {
                    xs.push(x as i32);
                }
            }
        }
        p *= 10;
    }
    xs.sort_unstable();
    xs.dedup();
    assert_same("C7", &xs);
}

// --- C8 / C9: the two extremes ----------------------------------------------

#[test]
fn c8_int_max() {
    assert_same("C8", &[i32::MAX]);
    // 2*INT_MAX wraps to -2, then +300 -> 298
    assert_eq!(expected_line(i32::MAX), "298\n");
}

#[test]
fn c9_int_min() {
    assert_same("C9", &[i32::MIN]);
    // 2*INT_MIN wraps to 0, then +300 -> 300
    assert_eq!(expected_line(i32::MIN), "300\n");
}

// --- C10 / C11: the extremal wrap regions, exhaustive -----------------------

#[test]
fn c10_upper_wrap_region() {
    let mut xs = inclusive(i32::MAX - 32, i32::MAX);
    let mut rng = Rng::with_seed(0xC10);
    for _ in 0..500 {
        xs.push(rng.range_i32(i32::MAX - 100_000, i32::MAX));
    }
    assert_same("C10", &xs);
}

#[test]
fn c11_lower_wrap_region() {
    let mut xs = inclusive(i32::MIN, i32::MIN + 32);
    let mut rng = Rng::with_seed(0xC11);
    for _ in 0..500 {
        xs.push(rng.range_i32(i32::MIN, i32::MIN + 100_000));
    }
    assert_same("C11", &xs);
}

// --- C12 / C13: the 2*x overflow thresholds ---------------------------------

#[test]
fn c12_mul_overflow_threshold_positive() {
    // x > 2^30 - 1 makes 2*x exceed INT_MAX.
    assert_same("C12", &around(1 << 30, 32));
}

#[test]
fn c13_mul_overflow_threshold_negative() {
    // x < -2^30 makes 2*x fall below INT_MIN.
    assert_same("C13", &around(-(1 << 30), 32));
}

// --- C14: the ADD is what overflows (distinct from C12) ---------------------

#[test]
fn c14_add_overflow() {
    // x = 0x3FFFFFFF -> 2*x = 0x7FFFFFFE (just under INT_MAX); +300 wraps.
    let xs = around(0x3FFF_FFFF, 32);
    assert_same("C14", &xs);

    // Pin the arithmetic: 2 * 0x3FFFFFFF == 0x7FFFFFFE (== INT_MAX - 1), which
    // does NOT overflow the multiply; the subsequent `+ 300` is what wraps,
    // landing 298 past INT_MIN - 1.
    let doubled = 0x3FFF_FFFFi32.wrapping_mul(2);
    assert_eq!(doubled, i32::MAX - 1);
    assert_eq!(
        expected_line(0x3FFF_FFFF),
        format!("{}\n", doubled.wrapping_add(300))
    );
}

// --- C15: results exactly at the printable extremes -------------------------

#[test]
fn c15_result_at_extremes() {
    // Find x giving y == INT_MAX / INT_MIN under wrapping arithmetic.
    let mut xs = Vec::new();
    for x in [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
        xs.push(x);
    }
    // y == INT_MAX  =>  2x == INT_MAX - 300 (odd, unreachable) ; check the
    // nearest reachable pair instead, plus y == INT_MIN.
    for x in 0..64i64 {
        let cand = ((i32::MAX as i64 - 300 - x) / 2) as i32;
        xs.push(cand);
        let cand2 = ((i32::MIN as i64 - 300 + x) / 2) as i32;
        xs.push(cand2);
    }
    xs.sort_unstable();
    xs.dedup();
    assert_same("C15", &xs);
}

// --- C16: full-range randomized sweep ---------------------------------------

#[test]
fn c16_full_range_random() {
    let mut rng = Rng::new();
    let xs: Vec<i32> = (0..20_000).map(|_| rng.next_i32()).collect();
    assert_same("C16", &xs);
}

// --- C17: powers of two and their neighbours --------------------------------

#[test]
fn c17_powers_of_two_and_neighbours() {
    let mut xs = Vec::new();
    for k in 0..32u32 {
        let p = 1i64 << k;
        for base in [p, -p] {
            for d in [-1i64, 0, 1] {
                let v = base + d;
                if v >= i32::MIN as i64 && v <= i32::MAX as i64 {
                    xs.push(v as i32);
                }
            }
        }
    }
    xs.sort_unstable();
    xs.dedup();
    assert_same("C17", &xs);
}

// --- C18: statelessness across many interleaved calls -----------------------

#[test]
fn c18_many_calls_interleaved() {
    let mut rng = Rng::with_seed(0xC18);
    let xs: Vec<i32> = (0..5_000).map(|_| rng.next_i32()).collect();
    // Batch form (each impl runs all 5000 back to back).
    assert_same("C18/batch", &xs);
    // Interleaved form (C, Rust, C, Rust, ... in ONE captured stream).
    assert_interleaved("C18/interleaved", &xs);
}

// --- C19: harness self-check ------------------------------------------------

#[test]
fn c19_zero_calls_capture_is_empty() {
    // Guards against the harness reporting vacuous "matches": if capture were
    // broken so it always returned empty, this is the only test that should
    // see empty output.
    assert!(
        capture_nothing().is_empty(),
        "capturing zero calls must yield zero bytes"
    );
    // And a single call must yield NON-empty output.
    assert_same("C19", &[7]);
}

// --- C20: fresh dlopen per call ---------------------------------------------

#[test]
fn c20_fresh_handles() {
    for x in [0, 1, -1, -150, i32::MAX, i32::MIN, 1 << 30, -(1 << 30)] {
        assert_same_fresh_handles("C20", x);
    }
}
