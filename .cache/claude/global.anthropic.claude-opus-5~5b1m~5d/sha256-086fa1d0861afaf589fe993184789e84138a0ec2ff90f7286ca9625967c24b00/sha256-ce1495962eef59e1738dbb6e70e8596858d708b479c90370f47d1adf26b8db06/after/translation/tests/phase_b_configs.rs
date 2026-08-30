//! Phase B — valid-path differential tests.
//!
//! One `#[test]` per row of `CONFIGS.md`. Both `.so`s are driven through their
//! exported symbols only; stdout is compared byte-for-byte.

mod common;

use common::*;
use std::ffi::c_int;

// ---------------------------------------------------------------------------
// Row 1 — run(0), isolates the floors/bathrooms mutation.
// ---------------------------------------------------------------------------
#[test]
fn row01_run_zero() {
    for _ in 0..20 {
        let out = assert_same(&Call::Run(0));
        assert_eq!(line_count(&out), RUN_LINES, "run() must print 4 lines");
    }
}

// ---------------------------------------------------------------------------
// Row 2 — small positive extra_bedrooms.
// ---------------------------------------------------------------------------
#[test]
fn row02_run_small_positive() {
    let mut rng = Rng::new();
    for _ in 0..200 {
        let x = rng.range_i64(1, 1000) as c_int;
        let out = assert_same(&Call::Run(x));
        assert_eq!(line_count(&out), RUN_LINES);
    }
}

// ---------------------------------------------------------------------------
// Row 3 — small negative extra_bedrooms (negative %d output).
// ---------------------------------------------------------------------------
#[test]
fn row03_run_small_negative() {
    let mut rng = Rng::with_seed(Rng::SEED ^ 3);
    for _ in 0..200 {
        let x = rng.range_i64(-1000, -1) as c_int;
        assert_same(&Call::Run(x));
    }
}

// ---------------------------------------------------------------------------
// Row 4 — uniform random over all 2^32 int patterns (random wraparound).
// ---------------------------------------------------------------------------
#[test]
fn row04_run_full_range_random() {
    let mut rng = Rng::with_seed(Rng::SEED ^ 4);
    for _ in 0..400 {
        assert_same(&Call::Run(rng.next_i32()));
    }
}

// ---------------------------------------------------------------------------
// Row 5 — boundary int arguments.
// ---------------------------------------------------------------------------
#[test]
fn row05_run_boundaries() {
    let vals: [c_int; 9] = [
        0,
        1,
        -1,
        c_int::MAX,
        c_int::MIN,
        c_int::MAX - 1,
        c_int::MIN + 1,
        1 << 30,
        -(1 << 30),
    ];
    for _round in 0..3 {
        for &v in &vals {
            assert_same(&Call::Run(v));
        }
    }
}

// ---------------------------------------------------------------------------
// Row 6 — state accumulation over 300 consecutive run() calls.
// ---------------------------------------------------------------------------
#[test]
fn row06_state_accumulation() {
    let mut rng = Rng::with_seed(Rng::SEED ^ 6);
    for i in 0..300 {
        let x = match i % 4 {
            0 => 0,
            1 => rng.range_i64(-50, 50) as c_int,
            2 => rng.next_i32(),
            _ => rng.range_i64(1, 1_000_000) as c_int,
        };
        assert_same(&Call::Run(x));
    }
}

// ---------------------------------------------------------------------------
// Row 7 — deep state: bathrooms grows large, exercising %.1f of a big
// half-integer double.
// ---------------------------------------------------------------------------
#[test]
fn row07_deep_state_bathrooms_growth() {
    for _ in 0..500 {
        assert_same(&Call::Run(0));
    }
    // Confirm we really did drive bathrooms into the hundreds and that the
    // %.1f rendering still agrees.
    let out = assert_same(&Call::Run(0));
    let text = String::from_utf8_lossy(&out).to_string();
    let last = text.lines().last().unwrap();
    let frac = last.rsplit(' ').nth(1).unwrap();
    assert!(
        frac.ends_with(".5"),
        "bathrooms should stay a half-integer, got {frac:?}"
    );
    let whole: f64 = frac.parse().unwrap();
    assert!(whole > 100.0, "expected deep state, bathrooms={whole}");
}

// ---------------------------------------------------------------------------
// Row 8 — repeated INT_MAX additions, wrapping bedrooms several times.
// ---------------------------------------------------------------------------
#[test]
fn row08_repeated_int_max_wrap() {
    for _ in 0..10 {
        assert_same(&Call::Run(c_int::MAX));
    }
    for _ in 0..10 {
        assert_same(&Call::Run(c_int::MIN));
    }
}

// ---------------------------------------------------------------------------
// Helpers for the driver-string rows
// ---------------------------------------------------------------------------
const WS: [&str; 6] = [" ", "\t", "\n", "\u{b}", "\u{c}", "\r"];
const GARBAGE: [&str; 10] = ["abc", " 43", ".5", "e3", ",000", "x1A", "%", "\n", "zzz", "-7"];

fn expect_accept(payload: &[u8]) -> Vec<u8> {
    let out = assert_same(&Call::Driver(payload.to_vec()));
    assert_eq!(
        line_count(&out),
        DRIVER_OK_LINES,
        "expected an accepted input (8 lines) for {:?}, got {:?}",
        String::from_utf8_lossy(payload),
        String::from_utf8_lossy(&out)
    );
    assert!(
        !out.windows(ERR_MSG.len()).any(|w| w == ERR_MSG),
        "unexpected rejection for {:?}",
        String::from_utf8_lossy(payload)
    );
    out
}

// ---------------------------------------------------------------------------
// Row 9 — plain decimal digits.
// ---------------------------------------------------------------------------
#[test]
fn row09_driver_plain_decimal() {
    let mut rng = Rng::with_seed(Rng::SEED ^ 9);
    for _ in 0..300 {
        let v = rng.next_i32();
        expect_accept(v.to_string().as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Row 10 — leading whitespace.
// ---------------------------------------------------------------------------
#[test]
fn row10_driver_leading_whitespace() {
    let mut rng = Rng::with_seed(Rng::SEED ^ 10);
    for _ in 0..250 {
        let v = rng.next_i32();
        let n = 1 + rng.below(5);
        let mut s = String::new();
        for _ in 0..n {
            s.push_str(rng.pick(&WS));
        }
        s.push_str(&v.to_string());
        expect_accept(s.as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Row 11 — explicit '+' sign.
// ---------------------------------------------------------------------------
#[test]
fn row11_driver_plus_sign() {
    let mut rng = Rng::with_seed(Rng::SEED ^ 11);
    for _ in 0..250 {
        let v = rng.range_i64(0, c_int::MAX as i64);
        expect_accept(format!("+{v}").as_bytes());
    }
    expect_accept(b"+0");
    expect_accept(b"+2147483647");
}

// ---------------------------------------------------------------------------
// Row 12 — leading zeros.
// ---------------------------------------------------------------------------
#[test]
fn row12_driver_leading_zeros() {
    let mut rng = Rng::with_seed(Rng::SEED ^ 12);
    for _ in 0..250 {
        let v = rng.next_i32();
        let zeros = "0".repeat(1 + rng.below(8));
        let s = if v < 0 {
            format!("-{zeros}{}", v.unsigned_abs())
        } else if rng.bool() {
            format!("+{zeros}{v}")
        } else {
            format!("{zeros}{v}")
        };
        expect_accept(s.as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Row 13 — trailing garbage after a convertible prefix (accepted).
// ---------------------------------------------------------------------------
#[test]
fn row13_driver_trailing_garbage() {
    let mut rng = Rng::with_seed(Rng::SEED ^ 13);
    for _ in 0..300 {
        let v = rng.next_i32();
        let g = rng.pick(&GARBAGE);
        expect_accept(format!("{v}{g}").as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Row 14 — all decorations combined.
// ---------------------------------------------------------------------------
#[test]
fn row14_driver_all_decorations() {
    let mut rng = Rng::with_seed(Rng::SEED ^ 14);
    for _ in 0..400 {
        let v = rng.next_i32();
        let mut s = String::new();
        for _ in 0..rng.below(4) {
            s.push_str(rng.pick(&WS));
        }
        let neg = v < 0;
        if neg {
            s.push('-');
        } else if rng.bool() {
            s.push('+');
        }
        s.push_str(&"0".repeat(rng.below(6)));
        s.push_str(&v.unsigned_abs().to_string());
        if rng.bool() {
            s.push_str(rng.pick(&GARBAGE));
        }
        // Must still be accepted: at least one digit is always present.
        expect_accept(s.as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Row 15 — boundary strings that must be accepted.
// ---------------------------------------------------------------------------
#[test]
fn row15_driver_accepted_boundaries() {
    for s in [
        "2147483647",
        "-2147483648",
        "0",
        "-0",
        "+0",
        "1",
        "-1",
        "2147483646",
        "-2147483647",
        "+2147483647",
        "0000000002147483647",
        "   -2147483648   ",
    ] {
        expect_accept(s.as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Row 16 — base is hard-coded to 10.
// ---------------------------------------------------------------------------
#[test]
fn row16_driver_base10_only() {
    // Each of these converts a leading '0' (or plain digits) and stops.
    for s in ["0x1A", "0b11", "0o17", "08", "09", "0xFFFFFFFFFFFFFFFF", "00x9"] {
        let out = expect_accept(s.as_bytes());
        assert_eq!(line_count(&out), DRIVER_OK_LINES);
    }
}

// ---------------------------------------------------------------------------
// Row 17 — very long but valid inputs.
// ---------------------------------------------------------------------------
#[test]
fn row17_driver_long_valid_inputs() {
    // 4096 leading zeros in front of a real value.
    let s = format!("{}{}", "0".repeat(4096), 12345);
    expect_accept(s.as_bytes());

    let s = format!("-{}{}", "0".repeat(4096), 2147483648u64 - 1);
    expect_accept(s.as_bytes());

    // 1 MiB of trailing garbage after a converted prefix.
    let mut v = b"777".to_vec();
    v.extend(std::iter::repeat(b'q').take(1 << 20));
    expect_accept(&v);

    // Whitespace-only prefix of substantial length.
    let s = format!("{}{}", " ".repeat(10_000), -42);
    expect_accept(s.as_bytes());
}

// ---------------------------------------------------------------------------
// Row 18 — a rejected call must leave the global state untouched in BOTH
// implementations; a following valid call reveals any divergence.
// ---------------------------------------------------------------------------
#[test]
fn row18_rejection_does_not_mutate_state() {
    let rejecting = [
        "", "abc", "  ", "+", "-", "99999999999999999999999", "2147483648", "-2147483649",
    ];
    for bad in rejecting {
        // Establish a known point, reject, then observe.
        let before = assert_same(&Call::Run(0));
        let rej = assert_same(&Call::Driver(bad.as_bytes().to_vec()));
        assert_eq!(rej, ERR_MSG, "expected rejection for {bad:?}");
        let after = assert_same(&Call::Run(0));
        // The rejected call must not have advanced the state, so `after`
        // differs from `before` only by the single intervening run(0).
        assert_ne!(before, after);
        assert_eq!(line_count(&after), RUN_LINES);
    }
}

// ---------------------------------------------------------------------------
// Row 19 — randomized interleaving of run / driver / rejecting-driver.
// ---------------------------------------------------------------------------
#[test]
fn row19_interleaved_pipeline() {
    let mut rng = Rng::with_seed(Rng::SEED ^ 19);
    let bad = [
        "", "abc", " ", "+", "-", "--1", "++1", "x", "9223372036854775808", "-2147483649",
        "2147483648", ".", "e", "\t\n",
    ];
    for _ in 0..200 {
        match rng.below(5) {
            0 => {
                assert_same(&Call::Run(rng.next_i32()));
            }
            1 => {
                assert_same(&Call::Run(rng.range_i64(-20, 20) as c_int));
            }
            2 => {
                let v = rng.next_i32();
                expect_accept(v.to_string().as_bytes());
            }
            3 => {
                let s = *rng.pick(&bad);
                let out = assert_same(&Call::Driver(s.as_bytes().to_vec()));
                assert_eq!(out, ERR_MSG, "input {s:?}");
            }
            _ => {
                assert_same(&Call::RunTwice(rng.next_i32()));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 20 — caller's errno is pre-poisoned; parse_val must neutralise it.
// ---------------------------------------------------------------------------
#[test]
fn row20_preexisting_errno_is_reset() {
    let mut rng = Rng::with_seed(Rng::SEED ^ 20);
    for pre in [ERANGE, EINVAL, 1, 4, 11, 99] {
        for _ in 0..15 {
            let v = rng.next_i32();
            let out = assert_same_errno(&Call::Driver(v.to_string().into_bytes()), pre);
            assert_eq!(
                line_count(&out),
                DRIVER_OK_LINES,
                "pre-existing errno={pre} must not cause a rejection"
            );
        }
        // ... and a genuinely invalid input still rejects.
        let out = assert_same_errno(&Call::Driver(b"nope".to_vec()), pre);
        assert_eq!(out, ERR_MSG);
        // ... and `run` is unaffected by errno entirely.
        assert_same_errno(&Call::Run(rng.next_i32()), pre);
    }
}

// ---------------------------------------------------------------------------
// Row 21 — the low-level `run(x); run(x)` pattern entered directly.
// ---------------------------------------------------------------------------
#[test]
fn row21_run_twice_low_level() {
    let mut rng = Rng::with_seed(Rng::SEED ^ 21);
    for _ in 0..150 {
        let x = rng.next_i32();
        let out = assert_same(&Call::RunTwice(x));
        assert_eq!(line_count(&out), 2 * RUN_LINES);
    }
    for &x in &[0, 1, -1, c_int::MAX, c_int::MIN] {
        assert_same(&Call::RunTwice(x));
    }
}

// ---------------------------------------------------------------------------
// Row 22 — call-hierarchy equivalence: C's `driver(s)` must equal Rust's
// `run(x); run(x)` (and vice versa) for the same parsed x.
// ---------------------------------------------------------------------------
#[test]
fn row22_driver_equals_two_runs() {
    let mut rng = Rng::with_seed(Rng::SEED ^ 22);
    for i in 0..150 {
        let v = rng.next_i32();
        let s = match i % 3 {
            0 => v.to_string(),
            1 => format!("  {v}junk"),
            _ => format!("{}{}", if v < 0 { "-000" } else { "+000" }, v.unsigned_abs()),
        };
        let x = c_parse_val(s.as_bytes()).expect("must parse");
        assert_eq!(x, v);
        if i % 2 == 0 {
            // C: driver(s)   vs   Rust: run(x); run(x)
            assert_cross(&Call::Driver(s.into_bytes()), &Call::RunTwice(x));
        } else {
            // C: run(x); run(x)   vs   Rust: driver(s)
            assert_cross(&Call::RunTwice(x), &Call::Driver(s.into_bytes()));
        }
    }
}
