// Phase C — error-path / rejection differential tests.
//
// One test per row of ERRORS.md. `sieve` returns `void` and contains no
// rejection construct at all (see ERRORS.md for the mechanical grep), so the
// "same error/sentinel" assertion becomes: for every boundary or degenerate
// input, C and Rust must produce the *same* observable result — identical stdout
// bytes AND identical termination (same child exit code / signal), not merely
// "both did something".

mod common;
use common::*;

/// Row 1 — the library has no rejection path at all. Assert positively that
/// `sieve` accepts inputs from every class and always returns normally (never
/// aborts, never traps, never hangs) in both implementations.
#[test]
fn err_row01_no_rejection_path_exists() {
    // one representative from every terminating input class
    let reps: [i32; 12] = [
        9,
        0,
        8,
        -1,
        -9,
        -10,
        1_000_000_000,
        i32::MAX - 8, // last value that terminates
        -123_456,
        19,
        -19,
        2_000_000_000,
    ];
    for v in reps {
        // returns normally, exit code 7 reached => no abort/trap/hang
        let c = run_with_closed_stdout(Impl::C, v);
        let r = run_with_closed_stdout(Impl::Rust, v);
        assert!(!c.timed_out && !r.timed_out, "sieve({v}) hung");
        assert_eq!(
            c.exit_code,
            Some(7),
            "C sieve({v}) did not return normally: {c:?}"
        );
        assert_eq!(c, r, "termination of sieve({v}) differs (C {c:?} vs Rust {r:?})");
        // and the output agrees byte for byte
        assert_match(v);
    }
}

/// Row 2 — immediate-break degenerate case: `val % 10 == 9` already true, so the
/// loop body runs once and `val++` never executes.
#[test]
fn err_row02_immediate_break_positive() {
    let mut rng = Rng::new(SEED ^ 0xE02);
    let mut vals = vec![9, 19, 109, 1_000_009, 2_147_483_639];
    for _ in 0..200 {
        vals.push((rng.range(0, 214_748_363) * 10 + 9) as i32);
    }
    for v in vals {
        let c = run(Impl::C, v);
        let r = run(Impl::Rust, v);
        assert_same_bytes(&format!("[err02] sieve({v})"), &c, &r);
        assert_eq!(
            c,
            format!("{v}\n").into_bytes(),
            "[err02] C must emit exactly one line for {v}"
        );
    }
}

/// Row 3 — `val == 0`, the zero / "closest thing to a null argument" boundary.
#[test]
fn err_row03_zero_argument() {
    let c = run(Impl::C, 0);
    let r = run(Impl::Rust, 0);
    assert_same_bytes("[err03] sieve(0)", &c, &r);
    assert_eq!(c, b"0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n".to_vec());
    assert_eq!(c.iter().filter(|&&b| b == b'\n').count(), 10);
}

/// Row 4 — negatives can never satisfy `val % 10 == 9` (C truncated modulo), so
/// the loop is *not* short-circuited; it counts all the way up to 9.
#[test]
fn err_row04_negative_never_matches() {
    let mut rng = Rng::new(SEED ^ 0xE04);
    let mut vals: Vec<i32> = (-30..=-1).collect();
    for _ in 0..150 {
        vals.push(rng.range(-5000, -1) as i32);
    }
    for v in vals {
        let c = run(Impl::C, v);
        let r = run(Impl::Rust, v);
        assert_same_bytes(&format!("[err04] sieve({v})"), &c, &r);
        // the C behaviour: exactly (-v + 10) lines, last line is "9"
        let expected_lines = (-(v as i64) + 10) as usize;
        assert_eq!(
            c.iter().filter(|&&b| b == b'\n').count(),
            expected_lines,
            "[err04] line count for {v}"
        );
        assert!(c.ends_with(b"\n9\n"), "[err04] sieve({v}) must end at 9");
        // if Rust had used Euclidean modulo it would break early; verify it did not
        assert!(
            c.len() > format!("{v}\n").len(),
            "[err04] sieve({v}) must not stop on the first line"
        );
    }
}

/// Rows 5 & 6 — the `-9` trap: `-9 % 10 == -9`, not `9`, so negative values that
/// "end in 9" do NOT break.
#[test]
fn err_row05_negative_nine_does_not_break() {
    let vals: [i32; 8] = [-9, -19, -29, -99, -109, -1009, -100_009, -999_999_999];
    for v in vals {
        // -999999999 would be ~1e9 lines; use a prefix comparison for the huge one
        if line_count(v).unwrap_or(u64::MAX) > 200_000 {
            const LIMIT: usize = 128 * 1024;
            let c = capture_prefix(Impl::C, v, LIMIT);
            let r = capture_prefix(Impl::Rust, v, LIMIT);
            assert_eq!(c.prefix.len(), LIMIT, "[err05] C prefix short for {v}");
            assert_eq!(r.prefix.len(), LIMIT, "[err05] Rust prefix short for {v}");
            assert_same_bytes(&format!("[err05] sieve({v}) prefix"), &c.prefix, &r.prefix);
            // it kept going instead of breaking on line 1
            assert!(
                c.prefix.starts_with(format!("{v}\n{}\n", v + 1).as_bytes()),
                "[err05] sieve({v}) broke early in C"
            );
            continue;
        }
        let c = run(Impl::C, v);
        let r = run(Impl::Rust, v);
        assert_same_bytes(&format!("[err05] sieve({v})"), &c, &r);
        assert_ne!(
            c,
            format!("{v}\n").into_bytes(),
            "[err05] sieve({v}) must NOT break immediately"
        );
        assert!(c.ends_with(b"\n9\n"), "[err05] sieve({v}) must run up to 9");
    }
    // spot-check the exact expected byte stream for -9
    let expect: String = (-9..=9).map(|i| format!("{i}\n")).collect();
    assert_eq!(run(Impl::C, -9), expect.as_bytes().to_vec());
    assert_eq!(run(Impl::Rust, -9), expect.as_bytes().to_vec());
}

/// Row 7 — `val == INT_MAX`: C executes `val++` on INT_MAX (signed overflow,
/// UB in C). Compare a bounded prefix of the resulting unbounded stream.
#[test]
fn err_row07_int_max_overflow_prefix() {
    const LIMIT: usize = 256 * 1024;
    let v = i32::MAX;
    let c = capture_prefix(Impl::C, v, LIMIT);
    let r = capture_prefix(Impl::Rust, v, LIMIT);
    assert_eq!(c.prefix.len(), LIMIT, "[err07] C prefix short (vacuous test)");
    assert_eq!(r.prefix.len(), LIMIT, "[err07] Rust prefix short");
    assert_same_bytes("[err07] sieve(INT_MAX) prefix", &c.prefix, &r.prefix);
    // document/lock in the observed C behaviour: INT_MAX then wrap to INT_MIN
    assert!(
        c.prefix.starts_with(b"2147483647\n-2147483648\n-2147483647\n"),
        "[err07] unexpected C overflow behaviour: {:?}",
        String::from_utf8_lossy(&c.prefix[..40.min(c.prefix.len())])
    );
    assert!(!c.finished, "[err07] C is expected not to terminate quickly");
    assert_eq!(c.finished, r.finished, "[err07] termination differs");
}

/// Row 8 — all eight values in the overflow window behave identically.
#[test]
fn err_row08_overflow_window_all_eight() {
    const LIMIT: usize = 64 * 1024;
    for v in (i32::MAX - 7)..=i32::MAX {
        assert!(
            line_count(v).is_none(),
            "[err08] {v} should be in the unbounded/overflow class"
        );
        let c = capture_prefix(Impl::C, v, LIMIT);
        let r = capture_prefix(Impl::Rust, v, LIMIT);
        assert_eq!(c.prefix.len(), LIMIT, "[err08] C prefix short for {v}");
        assert_eq!(r.prefix.len(), LIMIT, "[err08] Rust prefix short for {v}");
        assert_same_bytes(&format!("[err08] sieve({v}) prefix"), &c.prefix, &r.prefix);
        assert_eq!(c.finished, r.finished, "[err08] termination differs for {v}");
    }
}

/// Row 9 — `INT_MAX - 8`: one step *inside* the range, breaks instead of
/// overflowing. Boundary partner of row 8.
#[test]
fn err_row09_last_safe_value() {
    let v = i32::MAX - 8;
    let c = run(Impl::C, v);
    let r = run(Impl::Rust, v);
    assert_same_bytes("[err09] sieve(INT_MAX-8)", &c, &r);
    assert_eq!(c, b"2147483639\n".to_vec());
    // and its neighbour one step out is unbounded (row 8) — the boundary is sharp
    assert!(line_count(v).is_some() && line_count(v + 1).is_none());
}

/// Row 10 — `val == INT_MIN`, the smallest representable argument.
#[test]
fn err_row10_int_min_prefix() {
    const LIMIT: usize = 256 * 1024;
    let v = i32::MIN;
    let c = capture_prefix(Impl::C, v, LIMIT);
    let r = capture_prefix(Impl::Rust, v, LIMIT);
    assert_eq!(c.prefix.len(), LIMIT, "[err10] C prefix short (vacuous test)");
    assert_eq!(r.prefix.len(), LIMIT, "[err10] Rust prefix short");
    assert_same_bytes("[err10] sieve(INT_MIN) prefix", &c.prefix, &r.prefix);
    assert!(
        c.prefix.starts_with(b"-2147483648\n-2147483647\n"),
        "[err10] unexpected C output: {:?}",
        String::from_utf8_lossy(&c.prefix[..40.min(c.prefix.len())])
    );
}

/// Row 11 — INT_MIN+1 .. INT_MIN+3, just inside the low boundary.
#[test]
fn err_row11_near_int_min_prefix() {
    const LIMIT: usize = 64 * 1024;
    for k in 1..=3 {
        let v = i32::MIN + k;
        let c = capture_prefix(Impl::C, v, LIMIT);
        let r = capture_prefix(Impl::Rust, v, LIMIT);
        assert_eq!(c.prefix.len(), LIMIT, "[err11] C prefix short for {v}");
        assert_eq!(r.prefix.len(), LIMIT, "[err11] Rust prefix short for {v}");
        assert_same_bytes(&format!("[err11] sieve({v}) prefix"), &c.prefix, &r.prefix);
        assert!(c.prefix.starts_with(format!("{v}\n").as_bytes()));
    }
}

/// Row 12 — arbitrary / "out-of-range enum-like" bit patterns crossing the FFI
/// boundary. The API takes a bare `int`, so every 32-bit pattern is a legal
/// argument and neither side may reject or mis-handle one.
#[test]
fn err_row12_arbitrary_bit_patterns() {
    let patterns: [u32; 14] = [
        0x0000_0000,
        0xFFFF_FFFF, // -1
        0x8000_0000, // INT_MIN
        0x7FFF_FFFF, // INT_MAX
        0xDEAD_BEEF,
        0xCAFE_BABE,
        0x8000_0001,
        0x4000_0000,
        0xC000_0000,
        0x0000_0009,
        0xFFFF_FFF7, // -9
        0x5555_5555,
        0xAAAA_AAAA,
        0xFFFF_FF9C, // -100
    ];
    const LIMIT: usize = 64 * 1024;
    for p in patterns {
        let v = p as i32;
        match line_count(v) {
            Some(n) if n <= 200_000 => {
                let c = run(Impl::C, v);
                let r = run(Impl::Rust, v);
                assert_same_bytes(&format!("[err12] sieve(0x{p:08X} = {v})"), &c, &r);
            }
            _ => {
                let c = capture_prefix(Impl::C, v, LIMIT);
                let r = capture_prefix(Impl::Rust, v, LIMIT);
                assert_eq!(c.prefix.len(), LIMIT, "[err12] C prefix short for {v}");
                assert_eq!(r.prefix.len(), LIMIT, "[err12] Rust prefix short for {v}");
                assert_same_bytes(
                    &format!("[err12] sieve(0x{p:08X} = {v}) prefix"),
                    &c.prefix,
                    &r.prefix,
                );
            }
        }
    }
}

/// Row 13 — repeated / re-entrant-style calls with no init or teardown. There is
/// no state to corrupt; verify the Rust side has not introduced any.
#[test]
fn err_row13_repeated_calls_no_state() {
    let vals: [i32; 7] = [9, 9, 0, -3, 9, 25, -12];
    let c = run_seq(Impl::C, &vals);
    let r = run_seq(Impl::Rust, &vals);
    assert_same_bytes("[err13] repeated calls", &c, &r);

    // 200 calls of the same value must produce 200 identical copies
    let many = vec![19i32; 200];
    let c = run_seq(Impl::C, &many);
    let r = run_seq(Impl::Rust, &many);
    assert_same_bytes("[err13] 200x sieve(19)", &c, &r);
    assert_eq!(c, "19\n".repeat(200).into_bytes());
}

/// Row 14 — stdout is a closed fd: printf fails on every iteration and the C
/// code ignores the return value, so the call must still terminate normally with
/// no output. Both sides must agree on the exit status.
#[test]
fn err_row14_stdout_write_failure() {
    for v in [9i32, 0, 8, -5, 12345, 2_147_483_639] {
        let c = run_with_closed_stdout(Impl::C, v);
        let r = run_with_closed_stdout(Impl::Rust, v);
        assert!(!c.timed_out, "[err14] C hung with fd1 closed for {v}");
        assert!(!r.timed_out, "[err14] Rust hung with fd1 closed for {v}");
        assert_eq!(
            c.exit_code,
            Some(7),
            "[err14] C did not terminate normally for {v}: {c:?}"
        );
        assert_eq!(c.signal, None, "[err14] C died from a signal for {v}");
        assert_eq!(
            c, r,
            "[err14] C and Rust disagree for sieve({v}) with fd1 closed"
        );
    }
}

/// Generic FFI boundary sweep: brute-force a dense band of values around every
/// interesting boundary and require exact agreement.
#[test]
fn err_generic_boundary_sweep() {
    let mut vals: Vec<i32> = Vec::new();
    for base in [0i64, 10, 100, -10, -100, 1_000_000_000, -1_000_000_000] {
        for d in -12..=12i64 {
            let v = base + d;
            if v >= i32::MIN as i64 && v <= i32::MAX as i64 {
                let v = v as i32;
                if line_count(v).map(|n| n <= 200_000).unwrap_or(false) {
                    vals.push(v);
                }
            }
        }
    }
    vals.sort_unstable();
    vals.dedup();
    assert!(vals.len() > 100, "sweep should be dense");
    assert_match_all("boundary-sweep", &vals);
}
