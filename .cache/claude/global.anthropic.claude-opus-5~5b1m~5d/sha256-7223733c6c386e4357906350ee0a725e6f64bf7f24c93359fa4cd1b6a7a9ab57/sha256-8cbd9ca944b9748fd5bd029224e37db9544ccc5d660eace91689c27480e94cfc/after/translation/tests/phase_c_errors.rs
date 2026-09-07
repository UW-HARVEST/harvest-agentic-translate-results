//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. Each test constructs the exact invalid
//! input, calls BOTH the C `.so` and the Rust `.so`, and asserts they return the
//! SAME sentinel (`1`) *and* print the SAME diagnostic — not merely that both
//! "failed somehow".

mod harness;

use harness::*;

/// Asserts both libraries agree AND that they produced exactly `1` + `msg`.
fn expect_error(row: &str, call: &Call, msg: &[u8]) {
    assert_same_and(row, call, 1, msg);
}

/// Asserts both libraries agree AND that they succeeded (`0`).
fn expect_ok(row: &str, call: &Call, out: &[u8]) {
    assert_same_and(row, call, 0, out);
}

// ---------------------------------------------------------------------------
// Row 1 — start_ptr != NULL and *start_ptr > len
// ---------------------------------------------------------------------------

fn err01_start_past_end() {
    let mut rng = Rng::new(0x00001);
    for _ in 0..ITERS {
        let len = rng.range(0, 64);
        let s = gen_string(&mut rng, len, Flavor::Ascii);
        // Anything strictly greater than len, staying inside i32.
        let over = rng.range(len + 1, len + 4096) as i32;
        expect_error("err01", &Call::new(s, Some(over), None), ERR_START);
    }
    expect_error("err01", &Call::new(b"hello", Some(6), None), ERR_START);
    expect_error("err01", &Call::new(b"hello", Some(6), Some(2)), ERR_START);
}

// ---------------------------------------------------------------------------
// Row 2 — negative *start_ptr sign-extends and trips the *start* check
// ---------------------------------------------------------------------------

fn err02_negative_start_sign_extends() {
    let mut rng = Rng::new(0x00002);
    for _ in 0..ITERS {
        let len = rng.range(0, 64);
        let s = gen_string(&mut rng, len, Flavor::Ascii);
        let neg = -(rng.range(1, 100_000) as i64) as i32;
        // Crucially the *start* message, NOT "stop must come after start".
        expect_error("err02", &Call::new(s, Some(neg), None), ERR_START);
    }
    for neg in [-1i32, -2, -7, -1000, -65536] {
        expect_error("err02", &Call::new(b"hello", Some(neg), None), ERR_START);
        expect_error("err02", &Call::new(b"hello", Some(neg), Some(3)), ERR_START);
        expect_error("err02", &Call::new(b"", Some(neg), None), ERR_START);
    }
}

// ---------------------------------------------------------------------------
// Row 3 — *start_ptr == INT_MIN
// ---------------------------------------------------------------------------

fn err03_start_int_min() {
    for extreme in [i32::MIN, i32::MIN + 1, i32::MIN + 2] {
        for s in [&b""[..], b"a", b"hello world", &[0xFFu8; 300][..]] {
            expect_error("err03", &Call::new(s, Some(extreme), None), ERR_START);
            expect_error("err03", &Call::new(s, Some(extreme), Some(1)), ERR_START);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 4 — *start_ptr == INT_MAX (len < INT_MAX)
// ---------------------------------------------------------------------------

fn err04_start_int_max() {
    for extreme in [i32::MAX, i32::MAX - 1, 1 << 30, 1 << 20] {
        for s in [&b""[..], b"a", b"hello world"] {
            expect_error("err04", &Call::new(s, Some(extreme), None), ERR_START);
            expect_error("err04", &Call::new(s, Some(extreme), Some(0)), ERR_START);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 5 — precedence: the start check wins over both stop checks
// ---------------------------------------------------------------------------

fn err05_start_check_takes_precedence() {
    // Every combination of an invalid start with an invalid stop must yield the
    // START diagnostic only (one line of output, never two).
    let bad_starts = [6i32, 100, -1, i32::MIN, i32::MAX];
    let bad_stops = [6i32, 100, -1, i32::MIN, i32::MAX, 0, 1, 5];
    for &bs in &bad_starts {
        for &bp in &bad_stops {
            let call = Call::new(b"hello", Some(bs), Some(bp));
            let (_ret, out) = assert_same("err05", &call);
            assert_eq!(
                out, ERR_START,
                "[err05] start={bs} stop={bp}: expected only the start diagnostic, got {:?}",
                String::from_utf8_lossy(&out)
            );
            assert_eq!(
                out.iter().filter(|&&b| b == b'\n').count(),
                1,
                "[err05] start={bs} stop={bp}: more than one line printed"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 6 — stop_ptr != NULL and *stop_ptr > len
// ---------------------------------------------------------------------------

fn err06_stop_past_end() {
    let mut rng = Rng::new(0x00006);
    for _ in 0..ITERS {
        let len = rng.range(0, 64);
        let s = gen_string(&mut rng, len, Flavor::Ascii);
        let over = rng.range(len + 1, len + 4096) as i32;
        // start absent
        expect_error("err06", &Call::new(&s, None, Some(over)), ERR_STOP_OFF);
        // start present and valid
        let start = rng.range(0, len) as i32;
        expect_error(
            "err06",
            &Call::new(&s, Some(start), Some(over)),
            ERR_STOP_OFF,
        );
    }
    expect_error("err06", &Call::new(b"hello", None, Some(6)), ERR_STOP_OFF);
    expect_error("err06", &Call::new(b"hello", Some(0), Some(6)), ERR_STOP_OFF);
}

// ---------------------------------------------------------------------------
// Row 7 — negative *stop_ptr sign-extends and trips the *stop-off-end* check
// ---------------------------------------------------------------------------

fn err07_negative_stop_sign_extends() {
    let mut rng = Rng::new(0x00007);
    for _ in 0..ITERS {
        let len = rng.range(0, 64);
        let s = gen_string(&mut rng, len, Flavor::Ascii);
        let neg = -(rng.range(1, 100_000) as i64) as i32;
        // "off the end", NOT "must come after start" — this is the quirk.
        expect_error("err07", &Call::new(&s, None, Some(neg)), ERR_STOP_OFF);
        let start = rng.range(0, len) as i32;
        expect_error(
            "err07",
            &Call::new(&s, Some(start), Some(neg)),
            ERR_STOP_OFF,
        );
    }
    for neg in [-1i32, -2, -7, -1000, -65536] {
        expect_error("err07", &Call::new(b"hello", None, Some(neg)), ERR_STOP_OFF);
        expect_error("err07", &Call::new(b"", None, Some(neg)), ERR_STOP_OFF);
    }
}

// ---------------------------------------------------------------------------
// Row 8 — *stop_ptr == INT_MIN
// ---------------------------------------------------------------------------

fn err08_stop_int_min() {
    for extreme in [i32::MIN, i32::MIN + 1, i32::MIN + 2] {
        for s in [&b""[..], b"a", b"hello world", &[0x80u8; 257][..]] {
            expect_error("err08", &Call::new(s, None, Some(extreme)), ERR_STOP_OFF);
            expect_error("err08", &Call::new(s, Some(0), Some(extreme)), ERR_STOP_OFF);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 9 — *stop_ptr == INT_MAX (len < INT_MAX)
// ---------------------------------------------------------------------------

fn err09_stop_int_max() {
    for extreme in [i32::MAX, i32::MAX - 1, 1 << 30, 1 << 20] {
        for s in [&b""[..], b"a", b"hello world"] {
            expect_error("err09", &Call::new(s, None, Some(extreme)), ERR_STOP_OFF);
            expect_error("err09", &Call::new(s, Some(0), Some(extreme)), ERR_STOP_OFF);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 10 — both bounds in range but *stop_ptr < *start_ptr
// ---------------------------------------------------------------------------

fn err10_stop_before_start() {
    let mut rng = Rng::new(0x00010);
    for _ in 0..ITERS {
        let len = rng.range(2, 64);
        let s = gen_string(&mut rng, len, Flavor::Ascii);
        let start = rng.range(1, len);
        let stop = rng.range(0, start - 1);
        expect_error(
            "err10",
            &Call::new(s, Some(start as i32), Some(stop as i32)),
            ERR_STOP_ORDER,
        );
    }
    expect_error(
        "err10",
        &Call::new(b"hello", Some(4), Some(2)),
        ERR_STOP_ORDER,
    );
}

// ---------------------------------------------------------------------------
// Row 11 — *stop_ptr == *start_ptr (empty half-open range is rejected)
// ---------------------------------------------------------------------------

fn err11_stop_equals_start() {
    let mut rng = Rng::new(0x00011);
    for _ in 0..ITERS {
        let len = rng.range(0, 64);
        let s = gen_string(&mut rng, len, Flavor::Ascii);
        let v = rng.range(0, len) as i32;
        expect_error(
            "err11",
            &Call::new(s, Some(v), Some(v)),
            ERR_STOP_ORDER,
        );
    }
    for v in 0..=5i32 {
        expect_error("err11", &Call::new(b"hello", Some(v), Some(v)), ERR_STOP_ORDER);
    }
}

// ---------------------------------------------------------------------------
// Row 12 — *stop_ptr == 0 with start_ptr == NULL (default start 0)
// ---------------------------------------------------------------------------

fn err12_zero_stop_with_default_start() {
    for s in [
        &b""[..],
        b"a",
        b"hello world",
        b"%s%s%s",
        &[0xC3u8, 0xA9, 0x41][..],
    ] {
        expect_error("err12", &Call::new(s, None, Some(0)), ERR_STOP_ORDER);
    }
    let mut rng = Rng::new(0x00012);
    for _ in 0..ITERS {
        let len = rng.range(0, 96);
        let s = gen_string(&mut rng, len, Flavor::AnyByte);
        expect_error("err12", &Call::new(s, None, Some(0)), ERR_STOP_ORDER);
    }
}

// ---------------------------------------------------------------------------
// Row 13 — *stop_ptr == *start_ptr == len (upper boundary, then order check)
// ---------------------------------------------------------------------------

fn err13_both_bounds_at_len() {
    let mut rng = Rng::new(0x00013);
    for _ in 0..ITERS {
        let len = rng.range(0, 128);
        let s = gen_string(&mut rng, len, Flavor::Ascii);
        expect_error(
            "err13",
            &Call::new(s, Some(len as i32), Some(len as i32)),
            ERR_STOP_ORDER,
        );
    }
    // The `stop > len` check must NOT fire here: len > len is false.
    expect_error(
        "err13",
        &Call::new(b"hello", Some(5), Some(5)),
        ERR_STOP_ORDER,
    );
}

// ---------------------------------------------------------------------------
// Row 14 — empty string with *stop_ptr == 0
// ---------------------------------------------------------------------------

fn err14_empty_string_zero_stop() {
    expect_error("err14", &Call::new(b"", None, Some(0)), ERR_STOP_ORDER);
    expect_error("err14", &Call::new(b"", Some(0), Some(0)), ERR_STOP_ORDER);
    // Every non-zero stop on an empty string is off the end instead.
    for stop in [1i32, 2, 100, i32::MAX] {
        expect_error("err14", &Call::new(b"", None, Some(stop)), ERR_STOP_OFF);
    }
}

// ---------------------------------------------------------------------------
// Row 15 — empty string with *start_ptr == 1 (one past the valid range [0,0])
// ---------------------------------------------------------------------------

fn err15_empty_string_start_one() {
    expect_error("err15", &Call::new(b"", Some(1), None), ERR_START);
    expect_error("err15", &Call::new(b"", Some(1), Some(0)), ERR_START);
    // start == 0 on an empty string is the only valid start.
    expect_ok("err15", &Call::new(b"", Some(0), None), b"\n");
}

// ---------------------------------------------------------------------------
// Row 16 — *start_ptr == len + 1 for many lengths
// ---------------------------------------------------------------------------

fn err16_start_one_past_range() {
    let mut rng = Rng::new(0x00016);
    for len in 0..=80usize {
        let s = gen_string(&mut rng, len, Flavor::Ascii);
        // len is valid, len + 1 is not: the exact boundary step.
        expect_ok("err16", &Call::new(&s, Some(len as i32), None), b"\n");
        expect_error("err16", &Call::new(&s, Some(len as i32 + 1), None), ERR_START);
    }
}

// ---------------------------------------------------------------------------
// Row 17 — *stop_ptr == len + 1 for many lengths
// ---------------------------------------------------------------------------

fn err17_stop_one_past_range() {
    let mut rng = Rng::new(0x00017);
    for len in 1..=80usize {
        let s = gen_string(&mut rng, len, Flavor::Ascii);
        // len is a valid stop (with start 0), len + 1 is not.
        let mut ok = s.clone();
        ok.push(b'\n');
        expect_ok("err17", &Call::new(&s, None, Some(len as i32)), &ok);
        expect_error(
            "err17",
            &Call::new(&s, None, Some(len as i32 + 1)),
            ERR_STOP_OFF,
        );
    }
}

// ---------------------------------------------------------------------------
// Generic boundaries G1–G6
// ---------------------------------------------------------------------------

fn g1_g3_pointer_nullness_matrix() {
    // All four null/non-null combinations over several string shapes.
    let mut rng = Rng::new(0x000A1);
    for _ in 0..ITERS {
        let len = rng.range(0, 32);
        let s = gen_string(&mut rng, len, Flavor::Ascii);
        // G1: both NULL -> whole string.
        let mut whole = s.clone();
        whole.push(b'\n');
        expect_ok("G1", &Call::new(&s, None, None), &whole);

        // G2: start NULL, stop set.
        if len >= 1 {
            let stop = rng.range(1, len) as i32;
            check("G2", &Call::new(&s, None, Some(stop)));
        }
        expect_error("G2", &Call::new(&s, None, Some(0)), ERR_STOP_ORDER);

        // G3: start set, stop NULL.
        let start = rng.range(0, len) as i32;
        check("G3", &Call::new(&s, Some(start), None));

        // Both set.
        if len >= 1 {
            check("G1-4", &Call::new(&s, Some(0), Some(len as i32)));
        }
    }
}

fn g4_zero_length_every_pointer_combination() {
    expect_ok("G4", &Call::new(b"", None, None), b"\n");
    expect_ok("G4", &Call::new(b"", Some(0), None), b"\n");
    expect_error("G4", &Call::new(b"", None, Some(0)), ERR_STOP_ORDER);
    expect_error("G4", &Call::new(b"", Some(0), Some(0)), ERR_STOP_ORDER);
    expect_error("G4", &Call::new(b"", Some(1), None), ERR_START);
    expect_error("G4", &Call::new(b"", Some(-1), None), ERR_START);
    expect_error("G4", &Call::new(b"", None, Some(1)), ERR_STOP_OFF);
    expect_error("G4", &Call::new(b"", None, Some(-1)), ERR_STOP_OFF);
}

fn g5_extreme_and_boundary_bound_values() {
    // Every "oversized length" / extreme value against several lengths.
    let extremes: [i32; 14] = [
        i32::MIN,
        i32::MIN + 1,
        -65537,
        -65536,
        -2,
        -1,
        0,
        1,
        65535,
        65536,
        1 << 30,
        (1i64 << 31) as i32, // wraps to i32::MIN
        i32::MAX - 1,
        i32::MAX,
    ];
    let mut rng = Rng::new(0x000A5);
    for len in [0usize, 1, 2, 5, 17, 64, 300] {
        let s = gen_string(&mut rng, len, Flavor::AnyByte);
        for &a in &extremes {
            check("G5", &Call::new(&s, Some(a), None));
            check("G5", &Call::new(&s, None, Some(a)));
            for &b in &extremes {
                check("G5", &Call::new(&s, Some(a), Some(b)));
            }
        }
    }
}

fn g6_unconstrained_int_domain_fuzz() {
    // `slice` has no enum parameter, so the analogous "value with no valid
    // variant" surface is an arbitrary i32 in either bound slot. Fuzz the whole
    // domain, including values that are meaningless for any string length.
    let mut rng = Rng::new(0x0BADC0DE_u64);
    for _ in 0..3000 {
        let len = rng.range(0, 40);
        let s = gen_string(&mut rng, len, Flavor::Ascii);
        let start = if rng.bool_p(1, 4) {
            None
        } else {
            Some(rng.i32_any())
        };
        let stop = if rng.bool_p(1, 4) {
            None
        } else {
            Some(rng.i32_any())
        };
        check("G6", &Call::new(s, start, stop));
    }
}

fn g6b_exhaustive_small_domain() {
    // Exhaustive over every (len, start, stop) triple in a small window,
    // including one step past both ends and the null variants.
    let mut rng = Rng::new(0x000A6);
    for len in 0..=6usize {
        let s = gen_string(&mut rng, len, Flavor::Ascii);
        for start in -2i32..=8 {
            check("G6b", &Call::new(&s, Some(start), None));
            check("G6b", &Call::new(&s, None, Some(start)));
            for stop in -2i32..=8 {
                check("G6b", &Call::new(&s, Some(start), Some(stop)));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Diagnostic text must match byte-for-byte (no trailing-space / wording drift)
// ---------------------------------------------------------------------------

fn diagnostic_texts_are_byte_exact() {
    let cases: [(&Call, &[u8]); 3] = {
        // Leak the calls so we can hold references in an array.
        let a = Box::leak(Box::new(Call::new(b"hello", Some(9), None)));
        let b = Box::leak(Box::new(Call::new(b"hello", None, Some(9))));
        let c = Box::leak(Box::new(Call::new(b"hello", Some(3), Some(1))));
        [
            (a, b"Error: start is off the end of the string!\n"),
            (b, b"Error: stop is off the end of the string!\n"),
            (c, b"Error: stop must come after start!\n"),
        ]
    };
    for (call, want) in cases {
        let (ret, out) = assert_same("diag", call);
        assert_eq!(ret, 1);
        assert_eq!(
            out, want,
            "diagnostic text drifted: got {:?}, want {:?}",
            String::from_utf8_lossy(&out),
            String::from_utf8_lossy(want)
        );
    }
}

// ---------------------------------------------------------------------------
// Sequential runner (`harness = false`): fd 1 is redirected around every
// library call, so the cases must not run concurrently.
// ---------------------------------------------------------------------------

fn main() {
    harness::run_all(
        "phase_c_errors",
        &[
            ("err01_start_past_end", err01_start_past_end as fn()),
            ("err02_negative_start_sign_extends", err02_negative_start_sign_extends as fn()),
            ("err03_start_int_min", err03_start_int_min as fn()),
            ("err04_start_int_max", err04_start_int_max as fn()),
            ("err05_start_check_takes_precedence", err05_start_check_takes_precedence as fn()),
            ("err06_stop_past_end", err06_stop_past_end as fn()),
            ("err07_negative_stop_sign_extends", err07_negative_stop_sign_extends as fn()),
            ("err08_stop_int_min", err08_stop_int_min as fn()),
            ("err09_stop_int_max", err09_stop_int_max as fn()),
            ("err10_stop_before_start", err10_stop_before_start as fn()),
            ("err11_stop_equals_start", err11_stop_equals_start as fn()),
            ("err12_zero_stop_with_default_start", err12_zero_stop_with_default_start as fn()),
            ("err13_both_bounds_at_len", err13_both_bounds_at_len as fn()),
            ("err14_empty_string_zero_stop", err14_empty_string_zero_stop as fn()),
            ("err15_empty_string_start_one", err15_empty_string_start_one as fn()),
            ("err16_start_one_past_range", err16_start_one_past_range as fn()),
            ("err17_stop_one_past_range", err17_stop_one_past_range as fn()),
            ("g1_g3_pointer_nullness_matrix", g1_g3_pointer_nullness_matrix as fn()),
            ("g4_zero_length_every_pointer_combination", g4_zero_length_every_pointer_combination as fn()),
            ("g5_extreme_and_boundary_bound_values", g5_extreme_and_boundary_bound_values as fn()),
            ("g6_unconstrained_int_domain_fuzz", g6_unconstrained_int_domain_fuzz as fn()),
            ("g6b_exhaustive_small_domain", g6b_exhaustive_small_domain as fn()),
            ("diagnostic_texts_are_byte_exact", diagnostic_texts_are_byte_exact as fn()),
        ],
    );
}
