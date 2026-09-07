//! Phase B — valid-path differential tests, one test per row of CONFIGS.md.
//!
//! Both libraries are exercised exclusively through their `.so` exports.

mod harness;

use harness::*;
use std::ffi::c_int;

const SEED: u64 = 0x00C0FFEE_D15EA5E5;

fn i32_text(v: i32) -> String {
    format!("{v}")
}

// --------------------------------------------------------------------------
// C1..C7 — the low-level `run` entry point, single call, value classes
// --------------------------------------------------------------------------

#[test]
fn cfg_c1_run_zero() {
    // NOTE: the absolute pristine-state anchor for this row lives in
    // tests/phase_b_initial_state.rs (its own process). Here we assert the
    // state-independent shape: 4 report lines, floors +1, bathrooms +1.0,
    // bedrooms unchanged.
    let out = assert_same_one("C1 run(0)", run_op(0));
    let text = String::from_utf8_lossy(&out);
    let reports: Vec<_> = text.lines().filter_map(parse_line).collect();
    assert_eq!(reports.len(), 4, "run() prints 4 lines: {text:?}");
    assert_eq!(reports[3].0, reports[0].0 + 1, "floors must advance by 1");
    assert_eq!(bedrooms_delta(&out), 0, "run(0) must not change bedrooms");
    assert_ne!(reports[0].2, reports[3].2, "bathrooms must advance by 1.0");
}

#[test]
fn cfg_c2_run_small_positive() {
    let mut rng = Rng::new(SEED ^ 2);
    for v in [1, 2, 7, 999, 1000].into_iter().chain((0..64).map(|_| rng.range_i32(1, 1000))) {
        assert_same_one(&format!("C2 run({v})"), run_op(v as c_int));
    }
}

#[test]
fn cfg_c3_run_small_negative() {
    let mut rng = Rng::new(SEED ^ 3);
    for v in [-1, -5, -6, -999, -1000]
        .into_iter()
        .chain((0..64).map(|_| rng.range_i32(-1000, -1)))
    {
        assert_same_one(&format!("C3 run({v})"), run_op(v as c_int));
    }
}

#[test]
fn cfg_c4_run_int_max() {
    // 5 + INT_MAX overflows a signed int; the C build must be matched exactly.
    assert_same_one("C4 run(INT_MAX)", run_op(i32::MAX));
    assert_same("C4 run(INT_MAX) x2", &[run_op(i32::MAX), run_op(i32::MAX)]);
}

#[test]
fn cfg_c5_run_int_min() {
    assert_same_one("C5 run(INT_MIN)", run_op(i32::MIN));
    assert_same("C5 run(INT_MIN) x2", &[run_op(i32::MIN), run_op(i32::MIN)]);
}

#[test]
fn cfg_c6_run_full_i32_range() {
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..256 {
        let v = rng.next_i32();
        assert_same_one(&format!("C6 run({v})"), run_op(v));
    }
}

#[test]
fn cfg_c7_run_boundary_landings() {
    // bedrooms starts at 5; pick `extra` so the sum lands on notable values.
    let interesting: [i32; 10] = [
        i32::MAX - 5,      // -> INT_MAX
        i32::MAX - 4,      // -> wraps to INT_MIN
        i32::MIN.wrapping_sub(5), // (wrapping) -> INT_MIN
        -5,                // -> 0
        -6,                // -> -1
        -4,                // -> 1
        i32::MAX / 2,
        i32::MIN / 2,
        1 << 30,
        -(1 << 30),
    ];
    for v in interesting {
        assert_same_one(&format!("C7 run({v})"), run_op(v));
    }
}

// --------------------------------------------------------------------------
// C8, C9, C23 — persistent state accumulation across `run` calls
// --------------------------------------------------------------------------

#[test]
fn cfg_c8_run_repeated_accumulation() {
    let mut rng = Rng::new(SEED ^ 8);
    for n in [1usize, 2, 3, 5, 10, 64] {
        for _trial in 0..8 {
            let ops: Vec<Op> = (0..n).map(|_| run_op(rng.next_i32())).collect();
            assert_same(&format!("C8 run x{n}"), &ops);
        }
    }
}

#[test]
fn cfg_c9_run_long_sequence_float_fmt() {
    // 300 runs -> bathrooms walks 2.5 .. 302.5, floors 2 .. 302: exercises
    // `%.1f` and `%d` across many magnitudes in one composed pipeline.
    let mut rng = Rng::new(SEED ^ 9);
    let ops: Vec<Op> = (0..300).map(|_| run_op(rng.range_i32(-3, 3))).collect();
    let out = assert_same("C9 run x300", &ops);
    assert_eq!(out.iter().filter(|&&b| b == b'\n').count(), 300 * 4);
    // `bathrooms` advances by exactly 1.0 per run and always renders with a
    // ".5" fractional part under %.1f; check that across all 300 iterations.
    let text = String::from_utf8_lossy(&out);
    let baths: Vec<String> = text.lines().filter_map(parse_line).map(|r| r.2).collect();
    assert!(
        baths.iter().all(|b| b.ends_with(".5")),
        "every %.1f rendering should end in .5, got e.g. {:?}",
        &baths[..8.min(baths.len())]
    );
    let distinct: std::collections::BTreeSet<&String> = baths.iter().collect();
    assert_eq!(
        distinct.len(),
        301,
        "300 runs should span 301 distinct bathroom values"
    );
}

// --------------------------------------------------------------------------
// C10..C17 — the `driver` wrapper, input-string shapes
// --------------------------------------------------------------------------

#[test]
fn cfg_c10_driver_small_digits() {
    let mut rng = Rng::new(SEED ^ 10);
    for s in ["0", "1", "7", "42", "9999"].map(String::from).into_iter().chain(
        (0..64).map(|_| i32_text(rng.range_i32(0, 9999))),
    ) {
        assert_same_one(&format!("C10 driver({s:?})"), drv(&s));
    }
}

#[test]
fn cfg_c11_driver_plus_sign() {
    let mut rng = Rng::new(SEED ^ 11);
    for s in ["+0", "+1", "+2147483647"].map(String::from).into_iter().chain(
        (0..48).map(|_| format!("+{}", rng.range_i32(0, i32::MAX))),
    ) {
        assert_same_one(&format!("C11 driver({s:?})"), drv(&s));
    }
}

#[test]
fn cfg_c12_driver_minus_sign() {
    let mut rng = Rng::new(SEED ^ 12);
    for s in ["-0", "-1", "-2147483648"].map(String::from).into_iter().chain(
        (0..48).map(|_| format!("-{}", rng.range_i32(0, i32::MAX))),
    ) {
        assert_same_one(&format!("C12 driver({s:?})"), drv(&s));
    }
}

#[test]
fn cfg_c13_driver_leading_whitespace() {
    let ws = [" ", "\t", "\n", "\u{b}", "\u{c}", "\r", " \t\n\u{b}\u{c}\r ", "   "];
    let mut rng = Rng::new(SEED ^ 13);
    for w in ws {
        for body in ["0", "5", "-5", "+5", "2147483647", "-2147483648"] {
            let s = format!("{w}{body}");
            assert_same_one(&format!("C13 driver({s:?})"), drv(&s));
        }
    }
    for _ in 0..64 {
        let mut s = String::new();
        for _ in 0..rng.below(4) {
            s.push_str(*rng.pick(&ws[..6]));
        }
        s.push_str(&i32_text(rng.next_i32()));
        assert_same_one(&format!("C13 rand driver({s:?})"), drv(&s));
    }
}

#[test]
fn cfg_c14_driver_leading_zeros() {
    let mut cases: Vec<String> = vec![
        "0", "-0", "+0", "00", "007", "0000000000000000042", "-00012", "+00012",
        "0000000000000000000000000000000000000000000000000000000001",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let mut rng = Rng::new(SEED ^ 14);
    for _ in 0..48 {
        let zeros = "0".repeat(1 + rng.below(20));
        let sign = *rng.pick(&["", "+", "-"]);
        cases.push(format!("{sign}{zeros}{}", rng.range_i32(0, 2_000_000)));
    }
    for s in cases {
        assert_same_one(&format!("C14 driver({s:?})"), drv(&s));
    }
}

#[test]
fn cfg_c15_driver_trailing_garbage() {
    let mut cases: Vec<String> = [
        "12abc", "3 ", "5\n", "7;", "1,000", "0x1A", "0X1A", "9e9", "42.7", "-8.9",
        "1 2 3", "2147483647x", "-2147483648y", "0abc", "+3z",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let mut rng = Rng::new(SEED ^ 15);
    let tails = ["abc", " ", "\n", ".", ",", "e", "x", "-", "+", "!", "\t9"];
    for _ in 0..64 {
        cases.push(format!("{}{}", rng.range_i32(-5000, 5000), rng.pick(&tails)));
    }
    for s in cases {
        assert_same_one(&format!("C15 driver({s:?})"), drv(&s));
    }
}

#[test]
fn cfg_c16_driver_int_boundaries() {
    for s in [
        "2147483647",
        "-2147483648",
        "+2147483647",
        "2147483646",
        "-2147483647",
    ] {
        assert_same_one(&format!("C16 driver({s:?})"), drv(s));
    }
    // Boundary values twice in a row -> double wrap of `bedrooms`.
    assert_same(
        "C16 driver(INT_MAX) x2",
        &[drv("2147483647"), drv("2147483647")],
    );
    assert_same(
        "C16 driver(INT_MIN) x2",
        &[drv("-2147483648"), drv("-2147483648")],
    );
}

#[test]
fn cfg_c17_driver_full_i32_range() {
    let mut rng = Rng::new(SEED ^ 17);
    for _ in 0..256 {
        let v = rng.next_i32();
        let s = i32_text(v);
        assert_same_one(&format!("C17 driver({s:?})"), drv(&s));
    }
}

// --------------------------------------------------------------------------
// C18..C22 — composed pipelines: state carried across mixed entry points
// --------------------------------------------------------------------------

#[test]
fn cfg_c18_driver_repeated_accumulation() {
    let mut rng = Rng::new(SEED ^ 18);
    for n in [1usize, 2, 3, 8] {
        for _trial in 0..6 {
            let ops: Vec<Op> = (0..n).map(|_| drv(&i32_text(rng.next_i32()))).collect();
            assert_same(&format!("C18 driver x{n}"), &ops);
        }
    }
}

#[test]
fn cfg_c19_interleaved_run_and_driver() {
    // Fixed hand-written interleavings first...
    assert_same(
        "C19 fixed",
        &[
            run_op(1),
            drv("10"),
            run_op(-3),
            run_op(i32::MAX),
            drv("-2147483648"),
            run_op(0),
        ],
    );
    // ...then randomized ones.
    let mut rng = Rng::new(SEED ^ 19);
    for _ in 0..40 {
        let n = 1 + rng.below(12);
        let ops: Vec<Op> = (0..n)
            .map(|_| {
                if rng.next_u32() & 1 == 0 {
                    run_op(rng.next_i32())
                } else {
                    drv(&i32_text(rng.next_i32()))
                }
            })
            .collect();
        assert_same("C19 random interleave", &ops);
    }
}

#[test]
fn cfg_c20_rejected_driver_preserves_state() {
    // A rejected driver() call prints exactly one sentinel line and mutates
    // nothing: the run() before it and the run() after it must be contiguous in
    // floors/bathrooms, exactly as if the rejected call were not there.
    let out = assert_same(
        "C20 with reject",
        &[run_op(3), drv("not-a-number"), run_op(4)],
    );
    let text = String::from_utf8_lossy(&out);
    assert_eq!(
        text.lines().filter(|l| *l == "An error occurred").count(),
        1,
        "exactly one sentinel line: {text:?}"
    );
    let reports: Vec<_> = text.lines().filter_map(parse_line).collect();
    assert_eq!(reports.len(), 8, "two run()s => 8 report lines: {text:?}");
    // last line of run #1 (index 3) must equal the first line of run #2 (index 4)
    assert_eq!(
        reports[3], reports[4],
        "a rejected driver() must leave the_house untouched: {text:?}"
    );
    assert_eq!(bedrooms_delta(&out), 3 + 4);

    let mut rng = Rng::new(SEED ^ 20);
    let bad = ["", " ", "abc", "+", "-", "99999999999999999999", "2147483648"];
    for _ in 0..40 {
        let n = 1 + rng.below(10);
        let ops: Vec<Op> = (0..n)
            .map(|_| match rng.below(3) {
                0 => run_op(rng.next_i32()),
                1 => drv(&i32_text(rng.next_i32())),
                _ => drv(rng.pick(&bad)),
            })
            .collect();
        assert_same("C20 random with rejects", &ops);
    }
}

#[test]
fn cfg_c21_driver_fuzz_arbitrary_strings() {
    // Alphabet deliberately mixes digits, signs, whitespace, and letters so both
    // the accepting and the rejecting decision paths are hit constantly.
    const ALPHABET: &[u8] = b"0123456789+- \t\n\r\x0b\x0cabcxXeE.,;!0099";
    let mut rng = Rng::new(SEED ^ 21);
    let before = comparison_count();
    let mut accepted = 0usize;
    let mut rejected = 0usize;
    for _ in 0..4000 {
        let len = rng.below(25);
        let bytes: Vec<u8> = (0..len).map(|_| *rng.pick(ALPHABET)).collect();
        let out = assert_same_one(
            &format!("C21 driver({:?})", String::from_utf8_lossy(&bytes)),
            Op::Driver(bytes),
        );
        if out == b"An error occurred\n" {
            rejected += 1;
        } else {
            accepted += 1;
        }
    }
    // Anti-vacuity: the row really ran 4000 comparisons and really exercised
    // BOTH the accepting and the rejecting decision path many times.
    assert_eq!(comparison_count() - before, 4000);
    assert!(accepted > 400, "too few accepting cases: {accepted}");
    assert!(rejected > 400, "too few rejecting cases: {rejected}");
}

#[test]
fn cfg_c22_long_mixed_random_sequences() {
    let mut rng = Rng::new(SEED ^ 22);
    const ALPHABET: &[u8] = b"0123456789+- \tabc";
    let before_ops = ops_executed();
    for _seq in 0..40 {
        let ops: Vec<Op> = (0..200)
            .map(|_| match rng.below(4) {
                0 => run_op(rng.next_i32()),
                1 => run_op(rng.range_i32(-3, 3)),
                2 => drv(&i32_text(rng.next_i32())),
                _ => {
                    let len = rng.below(12);
                    Op::Driver((0..len).map(|_| *rng.pick(ALPHABET)).collect())
                }
            })
            .collect();
        assert_same("C22 long mixed sequence", &ops);
    }
    // Anti-vacuity: 40 sequences x 200 ops really went through both libraries.
    assert_eq!(ops_executed() - before_ops, 40 * 200);
}
