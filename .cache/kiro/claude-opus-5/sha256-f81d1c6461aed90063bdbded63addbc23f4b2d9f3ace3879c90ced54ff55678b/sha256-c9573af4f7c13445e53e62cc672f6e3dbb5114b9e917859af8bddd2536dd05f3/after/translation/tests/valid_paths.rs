//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test loads both shared objects and
//! compares captured `stdout` byte-for-byte. `sieve` is the *lowest-level*
//! entry point (it is the only one the C library exports), so there is no
//! convenience wrapper being tested in place of the real thing.

mod common;

use common::*;

/// Row 1 — residue 9, positive: the loop body runs exactly once because the
/// print happens before the check.
#[test]
fn row01_residue_nine_single_iteration() {
    let (c, r) = both();
    for val in [9, 19, 29, 109, 999_999_999, 2_147_483_639 /* == (INT_MAX/10)*10+9 - 10, the largest int ending in 9 */] {
        assert_same(&c, &r, val, Buffering::Default);
        // Sanity: the C really does emit exactly one line here.
        let out = run_capture(&c, val, Buffering::Default);
        assert_eq!(out, format!("{val}\n").into_bytes());
    }
}

/// Row 2 — randomized positive values with residue 9, spread over the whole
/// positive range.
#[test]
fn row02_residue_nine_randomized() {
    let (c, r) = both();
    let mut rng = Rng::new();
    for _ in 0..300 {
        // Largest k with k*10+9 <= i32::MAX is 214748363.
        let k = rng.range_i32(0, 214_748_363);
        let val = k * 10 + 9;
        assert_same(&c, &r, val, Buffering::Default);
    }
}

/// Row 3 — residue 0: a full ten-line run.
#[test]
fn row03_residue_zero_full_run() {
    let (c, r) = both();
    for val in [0, 10, 100, 1000, 1_000_000, 2_147_483_630] {
        assert_same(&c, &r, val, Buffering::Default);
        assert_eq!(line_count(val), Some(10));
    }
}

/// Row 4 — every residue 1..=8, giving every partial run length 2..=9.
#[test]
fn row04_every_residue_partial_runs() {
    let (c, r) = both();
    for val in 0..=19 {
        assert_same(&c, &r, val, Buffering::Default);
    }
    for base in [0, 10, 20, 100, 1230, 45_670] {
        for d in 0..10 {
            assert_same(&c, &r, base + d, Buffering::Default);
        }
    }
}

/// Row 5 — randomized positive values in a small range (residue uniform).
#[test]
fn row05_randomized_small_positive() {
    let (c, r) = both();
    let mut rng = Rng::new();
    for _ in 0..400 {
        let val = rng.range_i32(1, 1_000_000);
        assert_same(&c, &r, val, Buffering::Default);
    }
}

/// Row 6 — decade boundaries and mid-run `printf` width changes.
///
/// Note derived from the C, not assumed: a *positive* run can never change
/// width, because it always ends at the next value ending in 9, which is
/// inside the same decade (`95` → `99`, not `109`). Width only changes mid-run
/// for negative starts (`-100` → `-99`, `-1` → `0`) and across the overflow
/// wrap. Both cases are asserted here.
#[test]
fn row06_digit_width_transitions() {
    let (c, r) = both();
    // Decade-boundary starts (positive): each stays inside one decade.
    for val in [
        5, 8, 95, 98, 995, 998, 9995, 99_995, 999_995, 9_999_995, 99_999_995, 999_999_995,
        2_147_483_635,
    ] {
        assert_same(&c, &r, val, Buffering::Default);
    }
    assert_eq!(
        run_capture(&c, 95, Buffering::Default),
        b"95\n96\n97\n98\n99\n",
        "a positive run must stay inside its decade"
    );

    // Negative starts: these really do change width mid-run.
    for val in [-1, -10, -100, -1000, -105, -1005, -10_005] {
        assert_same(&c, &r, val, Buffering::Default);
    }
    let out = run_capture(&c, -12, Buffering::Default);
    let expected: Vec<u8> = (-12..=9)
        .flat_map(|v: i32| format!("{v}\n").into_bytes())
        .collect();
    assert_eq!(out, expected);
    assert!(
        out.windows(6).any(|w| w == b"-10\n-9"),
        "expected a 3->2 char width change mid-run"
    );
    assert!(
        out.windows(5).any(|w| w == b"-1\n0\n"),
        "expected the -1 -> 0 width change mid-run"
    );
}

/// Row 7 — negative magnitudes ending in 9. C's `%` truncates toward zero, so
/// `-9 % 10 == -9`, which never equals 9: the loop must keep counting up to +9.
/// A Euclidean-remainder translation would stop immediately and print one line.
#[test]
fn row07_negative_magnitude_ends_in_nine() {
    let (c, r) = both();
    for val in [-9, -19, -29, -129, -999] {
        assert_same(&c, &r, val, Buffering::Default);
    }
    let out = run_capture(&c, -9, Buffering::Default);
    let expected: Vec<u8> = (-9..=9)
        .flat_map(|v: i32| format!("{v}\n").into_bytes())
        .collect();
    assert_eq!(out, expected, "C must not stop early on -9");
}

/// Row 8 — negative starts whose runs cross zero (and the `-1` → `0` width
/// change).
#[test]
fn row08_negative_crossing_zero() {
    let (c, r) = both();
    for val in [-1, -2, -5, -8, -10, -11, -100, -101, -123, -1000, -10_000] {
        assert_same(&c, &r, val, Buffering::Default);
    }
}

/// Row 9 — randomized negatives.
#[test]
fn row09_randomized_negative() {
    let (c, r) = both();
    let mut rng = Rng::new();
    for _ in 0..300 {
        let val = rng.range_i32(-1_000_000, -1);
        if is_cheap(val) {
            assert_same(&c, &r, val, Buffering::Default);
        } else {
            assert_same_prefix(&c, &r, val, 4096);
        }
    }
}

/// Row 10 — randomized over the entire 32-bit domain (any bit pattern a caller
/// can push across the FFI boundary), with the comparison strategy chosen
/// mechanically from the predicted run length.
#[test]
fn row10_randomized_full_range() {
    let (c, r) = both();
    let mut rng = Rng::new();
    let mut cheap = 0;
    let mut expensive = 0;
    for _ in 0..220 {
        let val = rng.next_i32();
        if is_cheap(val) {
            cheap += 1;
            assert_same(&c, &r, val, Buffering::Default);
        } else {
            expensive += 1;
            assert_same_prefix(&c, &r, val, 4096);
        }
    }
    assert!(cheap > 0 && expensive > 0, "row 10 must hit both strategies");
}

/// Row 11 — the extreme low end. `INT_MIN % 10 == -8`, so the run climbs all
/// the way to +9 (~2.1e9 lines); compared as a bounded output prefix.
#[test]
fn row11_int_min_prefix() {
    let (c, r) = both();
    for val in [i32::MIN, i32::MIN + 1, i32::MIN + 8, i32::MIN + 9] {
        assert_same_prefix(&c, &r, val, 8192);
    }
    let out = run_prefix(&c, i32::MIN, 64);
    assert!(
        out.starts_with(b"-2147483648\n-2147483647\n"),
        "unexpected C prefix for INT_MIN: {:?}",
        String::from_utf8_lossy(&out)
    );
}

/// Row 12 — the signed-overflow region. `INT_MAX % 10 == 7`, so `val++`
/// overflows; the C compiled at -O0 wraps to `INT_MIN`, and the Rust must
/// reproduce that wrap rather than panicking or saturating.
#[test]
fn row12_int_max_overflow_prefix() {
    let (c, r) = both();
    for val in [i32::MAX, i32::MAX - 1, i32::MAX - 2, i32::MAX - 6] {
        assert_none_cheap(val);
        assert_same_prefix(&c, &r, val, 8192);
    }
    let out = run_prefix(&c, i32::MAX, 48);
    assert!(
        out.starts_with(b"2147483647\n-2147483648\n-2147483647\n"),
        "C did not wrap as expected at INT_MAX; got {:?}",
        String::from_utf8_lossy(&out)
    );
}

fn assert_none_cheap(val: i32) {
    assert_eq!(
        line_count(val),
        None,
        "sieve({val}) was expected to overflow into a long run"
    );
}

/// Row 13/14/15 — the three stdio buffering modes. The observable bytes must
/// not depend on how `stdout` is buffered, and must match between the two
/// libraries in every mode.
#[test]
fn row13_15_buffering_modes() {
    let (c, r) = both();
    let mut rng = Rng::with_seed(0xDEAD_BEEF_0000_0001);
    for mode in [
        Buffering::Default,
        Buffering::Full,
        Buffering::Line,
        Buffering::None,
    ] {
        for val in [0, 7, 9, 95, -1, -9, -123] {
            assert_same(&c, &r, val, mode);
        }
        for _ in 0..40 {
            let val = rng.range_i32(-5000, 5000);
            assert_same(&c, &r, val, mode);
        }
        // The bytes are mode-independent.
        assert_eq!(
            run_capture(&c, 95, mode),
            run_capture(&c, 95, Buffering::Default)
        );
    }
}

/// Row 16 — unflushed bytes are already in the shared `stdout` FILE buffer when
/// `sieve` is called. Both libraries must append to that same buffer (not open
/// a private stream), so ordering is preserved.
#[test]
fn row16_shared_pending_buffer() {
    let (c, r) = both();
    for val in [5, 9, -3] {
        let out_c = run_capture_with_pending(&c, val, "PENDING>");
        let out_r = run_capture_with_pending(&r, val, "PENDING>");
        assert_eq!(
            out_c,
            out_r,
            "pending-buffer ordering diverged for sieve({val})"
        );
        assert!(
            out_c.starts_with(b"PENDING>"),
            "library opened its own stream instead of using the shared stdout: {:?}",
            String::from_utf8_lossy(&out_c)
        );
    }
}

/// Row 17 — statelessness. The C has no globals, so 200 sequential calls on one
/// handle must behave exactly like 200 independent first calls.
#[test]
fn row17_repeated_calls_are_stateless() {
    let (c, r) = both();
    let mut rng = Rng::with_seed(0x0BAD_C0DE_1234_5678);
    let vals: Vec<i32> = (0..200).map(|_| rng.range_i32(-300, 300)).collect();

    // Each call individually.
    for &val in &vals {
        assert_same(&c, &r, val, Buffering::Default);
    }

    // All calls in one capture, in sequence: the concatenation must match too,
    // which would fail if either side carried state between invocations.
    let seq_c = capture(|| {
        for &val in &vals {
            unsafe { (c.sieve)(val) };
        }
    });
    let seq_r = capture(|| {
        for &val in &vals {
            unsafe { (r.sieve)(val) };
        }
    });
    assert_eq!(seq_c, seq_r, "sequential-call output diverged");

    let concat: Vec<u8> = vals
        .iter()
        .flat_map(|&val| run_capture(&c, val, Buffering::Default))
        .collect();
    assert_eq!(seq_c, concat, "C is not stateless across calls");
}

/// Row 18 — ABI width. The symbol takes an `int`; called with a 64-bit
/// argument carrying garbage in the upper half, only the low 32 bits may be
/// read, and both libraries must agree on that.
#[test]
fn row18_argument_width_truncation() {
    let (c, r) = both();
    let mut rng = Rng::new();
    let mut cases: Vec<i64> = vec![
        0x0000_0001_0000_0009,
        0x7FFF_FFFF_0000_0000,
        -1i64,
        0xFFFF_FFFF_0000_0005u64 as i64,
        i64::MIN,
        i64::MAX,
    ];
    for _ in 0..40 {
        let low = rng.range_i32(0, 100_000) as u32 as u64;
        let high = (rng.next_u32() as u64) << 32;
        cases.push((high | low) as i64);
    }
    for raw in cases {
        let low = raw as i32;
        if !is_cheap(low) {
            continue; // covered by the prefix tests
        }
        let out_c = run_capture_i64(&c, raw);
        let out_r = run_capture_i64(&r, raw);
        assert_eq!(
            out_c, out_r,
            "64-bit-argument call diverged for raw={raw:#x} (low32={low})"
        );
        assert_eq!(
            out_c,
            run_capture(&c, low, Buffering::Default),
            "C read more than the low 32 bits for raw={raw:#x}"
        );
    }
}

/// Row 19 — the two libraries interleaved in one process on the same stream.
#[test]
fn row19_interleaved_c_and_rust() {
    let (c, r) = both();
    let mut rng = Rng::with_seed(0xFEED_FACE_CAFE_0001);
    let vals: Vec<i32> = (0..60).map(|_| rng.range_i32(-200, 200)).collect();

    let interleaved = capture(|| {
        for (i, &val) in vals.iter().enumerate() {
            unsafe {
                if i % 2 == 0 {
                    (c.sieve)(val)
                } else {
                    (r.sieve)(val)
                }
            }
        }
    });
    let all_c = capture(|| {
        for &val in &vals {
            unsafe { (c.sieve)(val) };
        }
    });
    assert_eq!(
        interleaved, all_c,
        "interleaving the Rust .so changed the C .so's output"
    );
}
