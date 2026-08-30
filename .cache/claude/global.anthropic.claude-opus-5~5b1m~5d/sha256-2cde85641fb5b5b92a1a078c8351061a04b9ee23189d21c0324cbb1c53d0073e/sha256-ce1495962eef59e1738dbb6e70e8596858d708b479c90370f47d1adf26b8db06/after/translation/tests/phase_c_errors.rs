//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! Every public function in `driver.c` returns `void`: there is no error code,
//! no `errno`, no sentinel. The library reports a rejected input by printing a
//! fixed diagnostic line and skipping the work. "Same error" therefore means
//! **the exact same diagnostic bytes on stdout, and the same absence of the
//! output the accepted path would have produced** — which is a stronger check
//! than "both failed somehow".
//!
//! Each test asserts the specific expected text, not just C-vs-Rust equality,
//! so a translation that made *both* sides silently wrong would still fail.

mod common;

use common::{assert_same_clean, run_both, Op, Rng};

const NEGATIVE_MSG: &[u8] = b"ERROR: Array index is negative.";
const OOB_MSG: &[u8] = b"ERROR: Array index is out-of-bounds";

/// Convenience: run both, require equality, and hand back the (shared) lines.
fn agreed_lines(label: &str, ops: &[Op]) -> Vec<Vec<u8>> {
    assert_same_clean(label, ops);
    let (c, _r) = run_both(ops);
    c.lines().into_iter().map(|s| s.to_vec()).collect()
}

// ===========================================================================
// Row 1 / G1 — printLine(NULL)
// ===========================================================================

/// The guard `if (line != NULL)` at driver.c:31. Rejection is *silence*: not a
/// blank line, not a diagnostic, nothing at all.
#[test]
fn err_01_print_line_null() {
    let lines = agreed_lines("err_01 printLine(NULL) alone", &[Op::PrintLineNull]);
    assert!(
        lines.is_empty(),
        "printLine(NULL) must emit nothing, got {lines:?}"
    );

    // Sandwiched, to prove no stray newline is produced between the neighbours.
    let (c, r) = run_both(&[
        Op::print_line("a"),
        Op::PrintLineNull,
        Op::print_line("b"),
    ]);
    assert_eq!(c.stdout, r.stdout);
    assert_eq!(c.stdout, b"a\nb\n", "printLine(NULL) leaked output");

    // A long run of NULLs must still be completely silent.
    let many: Vec<Op> = (0..500).map(|_| Op::PrintLineNull).collect();
    let (c, r) = run_both(&many);
    assert_eq!(c.stdout, r.stdout);
    assert!(c.stdout.is_empty());
}

// ===========================================================================
// Row 2 / G2 — printLine("") : accepted, degenerate
// ===========================================================================

/// The zero-length string passes the NULL guard, so `printf("%s\n", "")` runs
/// and emits exactly one newline. Distinguishing this from row 1 is the whole
/// point: NULL and `""` must NOT behave alike.
#[test]
fn err_02_print_line_empty() {
    let (c, r) = run_both(&[Op::PrintLine(vec![])]);
    assert_eq!(c.stdout, r.stdout);
    assert_eq!(c.stdout, b"\n", "printLine(\"\") must emit exactly one newline");

    let (c_null, _) = run_both(&[Op::PrintLineNull]);
    assert_ne!(
        c.stdout, c_null.stdout,
        "printLine(\"\") and printLine(NULL) must differ"
    );
}

// ===========================================================================
// Row 3 — bad(data < 0)
// ===========================================================================

#[test]
fn err_03_bad_negative() {
    let lines = agreed_lines("err_03 bad(-1)", &[Op::Bad(-1)]);
    assert_eq!(
        lines,
        vec![NEGATIVE_MSG.to_vec()],
        "bad(-1) must print exactly the negative diagnostic and no buffer dump"
    );

    // Randomised over the whole negative half of the domain.
    let mut rng = Rng::new(0xE003);
    let ops: Vec<Op> = (0..300).map(|_| Op::Bad(rng.range(i32::MIN, -1))).collect();
    let lines = agreed_lines("err_03 bad(random negative)", &ops);
    assert_eq!(lines.len(), 300);
    assert!(lines.iter().all(|l| l == NEGATIVE_MSG));
}

// ===========================================================================
// Row 4 — bad(INT_MIN)
// ===========================================================================

/// `INT_MIN` is the extreme of the rejected range. Note it is also the value for
/// which `-data` would overflow, so a translation that normalised the index
/// before checking (or used a `usize` cast) would diverge here specifically.
#[test]
fn err_04_bad_int_min() {
    for data in [i32::MIN, i32::MIN + 1, -2_000_000_000, -1] {
        let lines = agreed_lines(&format!("err_04 bad({data})"), &[Op::Bad(data)]);
        assert_eq!(lines, vec![NEGATIVE_MSG.to_vec()], "bad({data})");
    }
}

// ===========================================================================
// Row 5 — good(data < 0)  ->  goodB2G rejects
// ===========================================================================

/// `good()` runs `goodG2B()` first (always 10 lines, `data` fixed at 7), *then*
/// `goodB2G(data)`, which rejects. So the output is 10 lines + 1 diagnostic —
/// the ordering matters and is part of the contract.
#[test]
fn err_05_good_negative() {
    let lines = agreed_lines("err_05 good(-1)", &[Op::Good(-1)]);
    assert_eq!(lines.len(), 11, "good(-1) must emit goodG2B's dump then the error");
    // goodG2B: data == 7, so index 7 is the only 1.
    let expected_g2b: Vec<Vec<u8>> = (0..10)
        .map(|i| if i == 7 { b"1".to_vec() } else { b"0".to_vec() })
        .collect();
    assert_eq!(&lines[..10], &expected_g2b[..]);
    assert_eq!(lines[10], OOB_MSG.to_vec());

    let mut rng = Rng::new(0xE005);
    let ops: Vec<Op> = (0..200).map(|_| Op::Good(rng.range(i32::MIN, -1))).collect();
    let lines = agreed_lines("err_05 good(random negative)", &ops);
    assert_eq!(lines.len(), 200 * 11);
    assert!(lines.chunks(11).all(|c| c[10] == OOB_MSG));
}

// ===========================================================================
// Row 6 — good(10): exactly one step past the valid range
// ===========================================================================

/// The `data < (10)` half of the guard. `good(9)` is accepted and `good(10)` is
/// rejected; testing both sides of the boundary is what catches an off-by-one.
#[test]
fn err_06_good_at_bound() {
    let lines = agreed_lines("err_06 good(10)", &[Op::Good(10)]);
    assert_eq!(lines.len(), 11);
    assert_eq!(lines[10], OOB_MSG.to_vec());

    // The accepted side of the very same boundary: 20 lines, no diagnostic.
    let lines9 = agreed_lines("err_06 good(9) accepted", &[Op::Good(9)]);
    assert_eq!(lines9.len(), 20, "good(9) must be accepted");
    assert!(
        !lines9.iter().any(|l| l == OOB_MSG || l == NEGATIVE_MSG),
        "good(9) must not produce a diagnostic"
    );
    assert_eq!(lines9[19], b"1".to_vec(), "good(9) sets the last element");
}

// ===========================================================================
// Row 7 — good(INT_MAX)
// ===========================================================================

#[test]
fn err_07_good_int_max() {
    for data in [10, 11, 1_000_000, i32::MAX - 1, i32::MAX] {
        let lines = agreed_lines(&format!("err_07 good({data})"), &[Op::Good(data)]);
        assert_eq!(lines.len(), 11, "good({data})");
        assert_eq!(lines[10], OOB_MSG.to_vec(), "good({data})");
    }
}

// ===========================================================================
// Row 8 — goodG2B's own else branch is dead code
// ===========================================================================

/// `goodG2B` hard-codes `data = 7`, so its `data >= 0` guard can never fail and
/// its `else` branch (which prints the *negative* diagnostic) is unreachable.
/// A translation that accidentally routed `good`'s argument into `goodG2B` would
/// make that branch live — this test is what detects it.
#[test]
fn err_08_goodg2b_else_is_dead() {
    let mut rng = Rng::new(0xE008);
    let mut ops = vec![
        Op::Good(i32::MIN),
        Op::Good(-1),
        Op::Good(0),
        Op::Good(9),
        Op::Good(10),
        Op::Good(i32::MAX),
    ];
    for _ in 0..200 {
        ops.push(Op::Good(rng.i32_any()));
    }
    let lines = agreed_lines("err_08 good(*) never reports 'negative'", &ops);
    assert!(
        !lines.iter().any(|l| l == NEGATIVE_MSG),
        "good() must never print {:?} — goodG2B's else branch is dead code",
        String::from_utf8_lossy(NEGATIVE_MSG)
    );
    // And goodG2B's dump is present, identical, at the head of every call.
    let expected_g2b: Vec<Vec<u8>> = (0..10)
        .map(|i| if i == 7 { b"1".to_vec() } else { b"0".to_vec() })
        .collect();
    assert_eq!(&lines[..10], &expected_g2b[..]);
}

// ===========================================================================
// Row 9 — bad(10): the MISSING upper-bound check
// ===========================================================================

/// The defect itself. `bad(10)` is *accepted* — no diagnostic — and the write
/// goes past the end of `buffer`. This is the row that distinguishes `bad` from
/// `goodB2G`: the identical input is rejected by one and accepted by the other.
#[test]
fn err_09_bad_one_past_end() {
    let bad_lines = agreed_lines("err_09 bad(10)", &[Op::Bad(10)]);
    assert_eq!(bad_lines.len(), 10, "bad(10) is accepted, so it dumps the buffer");
    assert!(
        !bad_lines.iter().any(|l| l == OOB_MSG || l == NEGATIVE_MSG),
        "bad(10) must NOT be rejected — that is the vulnerability"
    );
    assert_eq!(bad_lines, vec![b"0".to_vec(); 10]);

    // The same index through the guarded sink IS rejected.
    let good_lines = agreed_lines("err_09 good(10) contrast", &[Op::Good(10)]);
    assert_eq!(good_lines[10], OOB_MSG.to_vec());
}

// ===========================================================================
// Row 10 — driver with badData < 0
// ===========================================================================

#[test]
fn err_10_driver_bad_negative() {
    let lines = agreed_lines("err_10 driver(0, -1)", &[Op::Driver(0, -1)]);
    let expected: Vec<Vec<u8>> = {
        let mut v: Vec<Vec<u8>> = vec![b"Calling good()...".to_vec()];
        // goodG2B: index 7
        for i in 0..10 {
            v.push(if i == 7 { b"1".to_vec() } else { b"0".to_vec() });
        }
        // goodB2G(0): index 0
        for i in 0..10 {
            v.push(if i == 0 { b"1".to_vec() } else { b"0".to_vec() });
        }
        v.push(b"Finished good()".to_vec());
        v.push(b"Calling bad()...".to_vec());
        v.push(NEGATIVE_MSG.to_vec());
        v.push(b"Finished bad()".to_vec());
        v
    };
    assert_eq!(lines, expected, "driver(0, -1) full pipeline output");
}

// ===========================================================================
// Row 11 — driver with goodData out of range
// ===========================================================================

#[test]
fn err_11_driver_good_out_of_range() {
    for good_data in [-1, i32::MIN, 10, i32::MAX] {
        let lines = agreed_lines(
            &format!("err_11 driver({good_data}, 3)"),
            &[Op::Driver(good_data, 3)],
        );
        let mut expected: Vec<Vec<u8>> = vec![b"Calling good()...".to_vec()];
        for i in 0..10 {
            expected.push(if i == 7 { b"1".to_vec() } else { b"0".to_vec() });
        }
        expected.push(OOB_MSG.to_vec()); // goodB2G rejects
        expected.push(b"Finished good()".to_vec());
        expected.push(b"Calling bad()...".to_vec());
        for i in 0..10 {
            expected.push(if i == 3 { b"1".to_vec() } else { b"0".to_vec() });
        }
        expected.push(b"Finished bad()".to_vec());
        assert_eq!(lines, expected, "driver({good_data}, 3)");
    }
}

// ===========================================================================
// Row 12 — driver with both arguments invalid
// ===========================================================================

/// Both diagnostics must appear, in the fixed order `driver` prints them: the
/// good half's `out-of-bounds` first, then the bad half's `negative`.
#[test]
fn err_12_driver_both_invalid() {
    for (g, b) in [(-1, -1), (i32::MIN, i32::MIN), (10, -5), (i32::MAX, i32::MIN)] {
        let lines = agreed_lines(
            &format!("err_12 driver({g}, {b})"),
            &[Op::Driver(g, b)],
        );
        let mut expected: Vec<Vec<u8>> = vec![b"Calling good()...".to_vec()];
        for i in 0..10 {
            expected.push(if i == 7 { b"1".to_vec() } else { b"0".to_vec() });
        }
        expected.push(OOB_MSG.to_vec());
        expected.push(b"Finished good()".to_vec());
        expected.push(b"Calling bad()...".to_vec());
        expected.push(NEGATIVE_MSG.to_vec());
        expected.push(b"Finished bad()".to_vec());
        assert_eq!(lines, expected, "driver({g}, {b})");
    }
}

// ===========================================================================
// Generic FFI boundary rows
// ===========================================================================

/// G3 — oversized `printLine` input: 64 KiB, well past the stdio buffer and any
/// plausible internal fixed buffer.
#[test]
fn err_g3_print_line_oversized() {
    let mut rng = Rng::new(0xE003_0003);
    for len in [4096usize, 65_536, 262_144] {
        let payload = rng.ascii(len);
        let (c, r) = run_both(&[Op::PrintLine(payload.clone())]);
        assert_eq!(c.stdout, r.stdout, "printLine of {len} bytes diverged");
        assert_eq!(c.stdout.len(), len + 1, "must be the payload plus one \\n");
        assert_eq!(&c.stdout[..len], &payload[..]);
        assert_eq!(c.stdout[len], b'\n');
    }
}

/// G4 — `printf` conversion specifiers in the *data*. If either side treated
/// `line` as a format string, `%s`/`%n` would read or write wild pointers, so
/// this doubles as a format-string-vulnerability check.
#[test]
fn err_g4_print_line_format_specifiers() {
    for s in [
        "%s", "%n", "%d", "%p", "%%", "%99999999d", "%s%s%s%s%s%s%s%s%s%s", "%n%n%n%n%n%n",
        "%.2147483647f", "%1$n", "AAAA%08x.%08x.%08x.%08x.%n",
    ] {
        let (c, r) = run_both(&[Op::print_line(s)]);
        assert!(!c.crashed(), "C crashed on printLine({s:?})");
        assert!(!r.crashed(), "Rust crashed on printLine({s:?})");
        assert_eq!(c.stdout, r.stdout, "printLine({s:?}) diverged");
        assert_eq!(
            c.stdout,
            format!("{s}\n").into_bytes(),
            "printLine({s:?}) must print the bytes literally"
        );
    }
}

/// G5 — non-UTF-8 bytes and embedded newlines.
#[test]
fn err_g5_print_line_non_utf8() {
    let cases: Vec<Vec<u8>> = vec![
        vec![0xFF],
        vec![0xFE, 0xFF],
        vec![0x80],
        vec![0xC0, 0xAF],             // overlong encoding of '/'
        vec![0xED, 0xA0, 0xBD],       // lone surrogate
        vec![0xF5, 0x80, 0x80, 0x80], // beyond U+10FFFF
        (1u8..=255).rev().collect(),
        b"line1\nline2\r\nline3\t".to_vec(),
    ];
    for payload in cases {
        let (c, r) = run_both(&[Op::PrintLine(payload.clone())]);
        assert_eq!(
            c.stdout, r.stdout,
            "printLine({payload:02x?}) diverged"
        );
        let mut expected = payload.clone();
        expected.push(b'\n');
        assert_eq!(c.stdout, expected, "bytes must pass through untouched");
    }
}

/// G6 — `printIntLine` at both ends of the `int` range and one step inside.
#[test]
fn err_g6_print_int_line_extremes() {
    let cases = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    let ops: Vec<Op> = cases.iter().map(|&n| Op::PrintIntLine(n)).collect();
    let lines = agreed_lines("err_g6 printIntLine extremes", &ops);
    let expected: Vec<Vec<u8>> = cases.iter().map(|n| n.to_string().into_bytes()).collect();
    assert_eq!(lines, expected, "%d formatting of the int extremes");
}

/// G7 — out-of-range "enum" values across the FFI boundary.
///
/// `driver.c` declares no `enum` and contains no `switch` (verified by grep in
/// `ERRORS.md`), so there is no invalid-variant case in the literal sense. The
/// structural analogue is an `int` parameter carrying a value with no valid
/// interpretation — an index outside `0..10`. C accepts any `int` there, so this
/// sweeps the whole 32-bit domain through every `int`-taking entry point and
/// requires the two builds to agree on every value.
#[test]
fn err_g7_no_enum_int_domain_sweep() {
    let mut rng = Rng::new(0xE007);

    // `good` and `printIntLine` are total: every int is well defined.
    let mut ops = Vec::new();
    for _ in 0..400 {
        ops.push(Op::Good(rng.i32_any()));
    }
    for _ in 0..400 {
        ops.push(Op::PrintIntLine(rng.i32_any()));
    }
    // Powers of two and their neighbours, both signs.
    for bit in 0..31 {
        let v = 1i32 << bit;
        for cand in [v - 1, v, v + 1, -v - 1, -v, -v + 1] {
            ops.push(Op::Good(cand));
            ops.push(Op::PrintIntLine(cand));
        }
    }
    assert_same_clean("err_g7 good/printIntLine full int domain", &ops);

    // `bad` and `driver` are only total on the rejected (negative) half; the
    // accepted half past index 11 corrupts the caller (see UB.md), so the sweep
    // there covers every negative value plus the defined 0..=11.
    let mut ops = Vec::new();
    for _ in 0..400 {
        ops.push(Op::Bad(rng.range(i32::MIN, -1)));
    }
    for bit in 0..31 {
        let v = 1i32 << bit;
        for cand in [-v - 1, -v, -v + 1] {
            if cand < 0 {
                ops.push(Op::Bad(cand));
                ops.push(Op::Driver(rng.i32_any(), cand));
            }
        }
    }
    for d in 0..=11 {
        ops.push(Op::Bad(d));
        ops.push(Op::Driver(rng.i32_any(), d));
    }
    assert_same_clean("err_g7 bad/driver defined int domain", &ops);
}

/// G8 — one step *inside* the boundary: `bad(9)` is the last in-bounds index.
/// Together with row 9 (`bad(10)`) this brackets the missing check exactly.
#[test]
fn err_g8_bad_last_in_bounds() {
    let lines = agreed_lines("err_g8 bad(9)", &[Op::Bad(9)]);
    let expected: Vec<Vec<u8>> = (0..10)
        .map(|i| if i == 9 { b"1".to_vec() } else { b"0".to_vec() })
        .collect();
    assert_eq!(lines, expected, "bad(9) must set exactly the last element");

    // ... and the whole in-bounds range writes exactly one element each.
    for d in 0..10 {
        let lines = agreed_lines(&format!("err_g8 bad({d})"), &[Op::Bad(d)]);
        let expected: Vec<Vec<u8>> = (0..10)
            .map(|i| if i == d { b"1".to_vec() } else { b"0".to_vec() })
            .collect();
        assert_eq!(lines, expected, "bad({d})");
    }
}
