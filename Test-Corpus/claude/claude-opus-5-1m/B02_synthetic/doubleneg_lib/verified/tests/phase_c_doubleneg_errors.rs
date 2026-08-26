// Phase C — error / rejection-path differential tests for `doubleneg`
// (ERRORS.md rows 25..29).
//
// `doubleneg` writes to the process-global fd 1, so this binary deliberately
// contains exactly ONE `#[test]`: the harness redirects fd 1, which is only safe
// when no other test in the same binary runs concurrently.

mod common;

use common::{assert_bytes_eq, assert_i32_eq, c, capture_stdout, rs, silence_stdout, Rng};

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

/// Runs both implementations, compares the return value and the stdout bytes,
/// and hands the C output back for branch-reachability assertions.
fn run_both(row: &str, p: (i32, i32, i32, i32)) -> Vec<u8> {
    let ctx = format!("[{row}] doubleneg({}, {}, {}, {})", p.0, p.1, p.2, p.3);
    let (cv, cout) = capture_stdout(|| unsafe { (c().doubleneg)(p.0, p.1, p.2, p.3) });
    let (rv, rout) = capture_stdout(|| unsafe { (rs().doubleneg)(p.0, p.1, p.2, p.3) });
    assert_i32_eq(cv, rv, &ctx);
    assert_bytes_eq(&cout, &rout, &format!("{ctx} stdout"));
    cout
}

fn cmp_value(row: &str, p: (i32, i32, i32, i32)) {
    let cv = unsafe { (c().doubleneg)(p.0, p.1, p.2, p.3) };
    let rv = unsafe { (rs().doubleneg)(p.0, p.1, p.2, p.3) };
    assert_i32_eq(
        cv,
        rv,
        &format!("[{row}] doubleneg({}, {}, {}, {})", p.0, p.1, p.2, p.3),
    );
}

#[test]
fn doubleneg_error_paths() {
    // -------------------------------------------------------------------
    // Rows 25 & 26 — the two "not found" rejection branches inside doubleneg
    // (`if (pos >= 0)` else, and `if (direct_search != NULL)` false).
    //
    // These are provably unreachable: create_numeric_buffer fills 256 bytes with
    // (seed + 7*i) % 256, and gcd(7, 256) == 1, so the buffer is a permutation of
    // all 256 byte values and every memchr hits.  The differential requirement is
    // therefore that NEITHER implementation ever takes them — if the Rust
    // translation made them reachable (or the C did), the strings below would
    // diverge and the byte comparison in `run_both` would already fail.
    // -------------------------------------------------------------------
    let mut rng = Rng::new(0xD025);
    let mut probes: Vec<(i32, i32, i32, i32)> = vec![
        (0, 0, 0, 0),
        (1, 1, 1, 1),
        (i32::MIN, i32::MIN, i32::MIN, i32::MIN),
        (i32::MAX, i32::MAX, i32::MAX, i32::MAX),
        (100, 100, 100, 100),
        (-100, -100, -100, -100),
        (42, 0, -42, 255),
    ];
    // Every seed residue class rotates the permutation differently.
    for p1 in [0i32, 1, 99, 100, 101, 155, 156, 255, -1, -100, -255] {
        probes.push((p1, 7, 3, 5));
    }
    for _ in 0..120 {
        probes.push((
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        ));
    }

    for p in &probes {
        let out = run_both("row25/26", *p);

        // Row 25: the "Value %d not found" branch must never be taken.
        assert!(
            !contains(&out, b"not found"),
            "row25: doubleneg({}, {}, {}, {}) took the 'not found' branch, which \
             the stride-7 permutation makes unreachable:\n{}",
            p.0,
            p.1,
            p.2,
            p.3,
            String::from_utf8_lossy(&out)
        );
        // ... and all four searches must have reported a hit.
        let hits = out
            .windows(b"Found value ".len())
            .filter(|w| *w == b"Found value ")
            .count();
        assert_eq!(
            hits, 4,
            "row25: expected all 4 searches to hit for {p:?}, saw {hits}"
        );

        // Row 26: `memchr(buffer, 100, 256)` always finds byte 100.
        assert!(
            contains(&out, b"Direct memchr found byte 100 at offset: "),
            "row26: doubleneg({}, {}, {}, {}) skipped the direct-memchr branch:\n{}",
            p.0,
            p.1,
            p.2,
            p.3,
            String::from_utf8_lossy(&out)
        );

        // Row 28: converted_neg comes from -1.0 * pow(2, 40), which is always out
        // of int range, so the cvttsd2si "integer indefinite" value is always
        // printed and INT_MIN % 1000 == -648 is always folded into the result.
        assert!(
            contains(&out, b"Converted to int (UB likely): -2147483648"),
            "row28: expected the out-of-range conversion to yield 0x80000000:\n{}",
            String::from_utf8_lossy(&out)
        );
        // Row 28 (cont.): INFINITY and NAN also convert to 0x80000000.
        assert!(
            contains(&out, b"Converting INFINITY to int: -2147483648"),
            "row28: INFINITY conversion diverged:\n{}",
            String::from_utf8_lossy(&out)
        );
        assert!(
            contains(&out, b"Converting NAN to int: -2147483648"),
            "row28: NAN conversion diverged:\n{}",
            String::from_utf8_lossy(&out)
        );
        // 10 combined-feature searches, all of which must hit.
        let found1 = out
            .windows(b"found=1".len())
            .filter(|w| *w == b"found=1")
            .count();
        assert_eq!(found1, 10, "row25: expected 10 pointer-!! hits for {p:?}");
        assert!(
            !contains(&out, b"found=0"),
            "row25: a combined-feature search missed for {p:?}"
        );
    }

    assert_eq!(i32::MIN % 1000, -648, "sanity: INT_MIN % 1000");

    // -------------------------------------------------------------------
    // Rows 27, 28, 29 — value-only sweeps (stdout to /dev/null).
    // -------------------------------------------------------------------
    silence_stdout(|| {
        let mut rng = Rng::new(0xD027);

        // Row 27: param2/3/4 == INT_MIN -> `param % 256` yields a NEGATIVE search
        // value that is fed to find_value_in_buffer.
        assert_eq!(i32::MIN % 256, 0, "sanity: INT_MIN % 256");
        for &neg in &[
            i32::MIN,
            i32::MIN + 1,
            i32::MIN + 255,
            -1,
            -255,
            -256,
            -257,
            -1000,
        ] {
            cmp_value("row27", (0, neg, 0, 0));
            cmp_value("row27", (0, 0, neg, 0));
            cmp_value("row27", (0, 0, 0, neg));
            cmp_value("row27", (neg, neg, neg, neg));
            cmp_value("row27", (1, neg, neg, neg));
        }
        for _ in 0..1500 {
            let neg = -(1 + rng.below(2_000_000_000) as i32);
            cmp_value("row27", (rng.interesting_i32(), neg, neg, neg));
            cmp_value("row27", (neg, rng.interesting_i32(), neg, neg));
        }

        // Row 28: converted_int is INT_MIN whenever the computed double leaves
        // int range, so `converted_int % 1000` is also exercised at INT_MIN.
        // p1/p2 chosen so a/b * 10^(p3%10) overflows int.
        for &p3 in &[9i32, 19, -1, 0, i32::MAX, i32::MIN] {
            cmp_value("row28", (i32::MAX, 1, p3, 0));
            cmp_value("row28", (i32::MIN, 1, p3, 0));
            cmp_value("row28", (i32::MAX, -1, p3, 0));
            cmp_value("row28", (1000000, 1, p3, 0));
        }
        for _ in 0..1500 {
            // Large |a| with small |b| and a positive exponent -> out of range.
            let a = rng.next_i32();
            let b = if rng.below(2) == 0 { 1 } else { -1 };
            let p3 = 9 + 10 * (rng.below(100) as i32);
            cmp_value("row28", (a, b, p3, rng.interesting_i32()));
        }

        // Row 29: (param1 + i * param2) % 256 overflows signed int for large
        // params (i runs 0..9, so param2 near INT_MAX overflows immediately).
        for &p2 in &[
            i32::MAX,
            i32::MAX - 1,
            i32::MIN,
            i32::MIN + 1,
            0x2000_0000,
            -0x2000_0000,
            0x1000_0000,
        ] {
            for &p1 in &[0i32, 1, -1, i32::MAX, i32::MIN, 255, -255] {
                cmp_value("row29", (p1, p2, 0, 0));
            }
        }
        for _ in 0..3000 {
            cmp_value(
                "row29",
                (
                    rng.next_i32(),
                    rng.next_i32(),
                    rng.interesting_i32(),
                    rng.interesting_i32(),
                ),
            );
        }
    });
}
