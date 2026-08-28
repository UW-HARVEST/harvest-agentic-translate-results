//! Level 3: the public entry point `modeselect` — return value *and* the exact
//! bytes it writes to stdout.
//!
//! `mode_selector % 4` is negative whenever `mode_selector` is negative and not
//! a multiple of 4; the C code then indexes `modes[]` out of bounds and passes
//! whatever stack garbage it finds to `strcmp`. That is genuine undefined
//! behaviour with no defined value to match, so those selectors are excluded.
//! Every in-bounds selector (all non-negative values, plus negative multiples
//! of 4, which yield index 0) is covered.
//!
//! Everything lives in a single `#[test]`: comparing stdout means redirecting
//! the process-wide fd 1, and libtest's own per-test progress lines would
//! otherwise be interleaved into the captured bytes by sibling test threads.

mod common;

use common::*;

fn check(pair: &Pair, group: &str, a: i32, b: i32, c_: i32, d: i32) {
    let (cf, rf) = pair.modeselect();

    let (cr, cout) = capture_stdout(|| unsafe { cf(a, b, c_, d) });
    let (rr, rout) = capture_stdout(|| unsafe { rf(a, b, c_, d) });

    assert_eq!(
        cr, rr,
        "[{group}] modeselect({a}, {b}, {c_}, {d}) return: C={cr} ({cr:#x}) Rust={rr} ({rr:#x})"
    );
    assert_eq!(
        cout,
        rout,
        "[{group}] modeselect({a}, {b}, {c_}, {d}) stdout differs\n--- C ---\n{:?}\n--- Rust ---\n{:?}",
        show(&cout),
        show(&rout)
    );
}

#[test]
fn modeselect_matches_c() {
    let pair = Pair::load();

    // -- shape check: the capture harness really does see printf output -------
    {
        let (cf, rf) = pair.modeselect();
        let (_, cout) = capture_stdout(|| unsafe { cf(1, 2, 3, 4) });
        let (_, rout) = capture_stdout(|| unsafe { rf(1, 2, 3, 4) });
        let text = show(&cout);
        assert!(!text.is_empty(), "no stdout captured from the C library");
        for needle in [
            "Selected mode: enhanced (0x20)",
            "Complexity level: 3",
            "Modified time: ",
            "Result 1: ",
            "Result 2: ",
            "Final result: ",
        ] {
            assert!(text.contains(needle), "missing {needle:?} in:\n{text}");
        }
        assert_eq!(cout, rout);
    }

    // -- small dense grid: every mode_index and every complexity_level -------
    for a in 0..8 {
        for b in -3..=3 {
            for c_ in 0..10 {
                for d in -3..=3 {
                    check(&pair, "grid", a, b, c_, d);
                }
            }
        }
    }

    // -- negative complexity: `complexity % 5` < 0 hits the `default` arm ----
    for a in 0..4 {
        for c_ in -12..0 {
            for d in [-1, -23, -24, -25, -100, -86400] {
                check(&pair, "neg-complexity", a, 0, c_, d);
            }
        }
    }

    // -- negative multiples of 4: mode_index == 0, still in bounds ----------
    for k in 1..=16 {
        let a = -4 * k;
        for b in [-2, 0, 5] {
            for c_ in [0, 3, 7, -3] {
                for d in [0, 11, -11] {
                    check(&pair, "neg-mult4", a, b, c_, d);
                }
            }
        }
    }

    // -- extremes: int overflow in the day/hour maths and the double casts --
    let sel = [
        0,
        1,
        2,
        3,
        4,
        255,
        256,
        1000,
        i32::MAX,
        i32::MIN,
        i32::MIN + 4,
    ];
    let rest = [
        0,
        1,
        -1,
        23,
        24,
        25,
        -24,
        127,
        -128,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        86400,
        -86400,
        24855,
        -24855,
        100000,
        -100000,
        123456789,
        -123456789,
    ];
    for &a in &sel {
        for &b in &rest {
            for &c_ in &[0, 1, 2, 3, 4, 5, -1, -5, i32::MAX, i32::MIN] {
                for &d in &[0, 1, -1, 23, -23, i32::MAX, i32::MIN, 100000] {
                    check(&pair, "extremes", a, b, c_, d);
                }
            }
        }
    }

    // -- pseudo-random sweep -------------------------------------------------
    let mut x: u64 = 0xB5026F5AA96619E9;
    let mut next = || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x as u32 as i32
    };
    for _ in 0..600 {
        // keep mode_selector in bounds (see module docs)
        let a = next().wrapping_abs().max(0);
        check(&pair, "random", a, next(), next(), next());
    }
}
