// Phase B — valid-path differential tests for the top-level `doubleneg` entry
// point (CONFIGS.md rows 31..38), including a byte-for-byte comparison of
// everything the two libraries print on stdout.
//
// This binary intentionally contains exactly ONE `#[test]`: `doubleneg` writes to
// the process-global fd 1, which the harness must redirect, and that is only
// safe when no other test in the same binary runs concurrently.

mod common;

use common::{assert_bytes_eq, assert_i32_eq, c, capture_stdout, rs, silence_stdout, Rng};

/// Compares only the return value. MUST be called inside `silence_stdout`.
fn cmp_value(row: &str, p: (i32, i32, i32, i32)) {
    let cv = unsafe { (c().doubleneg)(p.0, p.1, p.2, p.3) };
    let rv = unsafe { (rs().doubleneg)(p.0, p.1, p.2, p.3) };
    assert_i32_eq(
        cv,
        rv,
        &format!("[{row}] doubleneg({}, {}, {}, {})", p.0, p.1, p.2, p.3),
    );
}

/// Compares the return value AND the exact bytes written to stdout.
/// MUST NOT be called inside `silence_stdout`.
fn cmp_value_and_stdout(row: &str, p: (i32, i32, i32, i32)) {
    let ctx = format!("[{row}] doubleneg({}, {}, {}, {})", p.0, p.1, p.2, p.3);
    let (cv, cout) = capture_stdout(|| unsafe { (c().doubleneg)(p.0, p.1, p.2, p.3) });
    let (rv, rout) = capture_stdout(|| unsafe { (rs().doubleneg)(p.0, p.1, p.2, p.3) });
    assert_i32_eq(cv, rv, &ctx);
    // Guard against a silently broken capture mechanism: the C output must
    // really be doubleneg's output, not an empty file or harness noise.
    for marker in [
        &b"=== Starting foo() execution ==="[..],
        &b"--- Integer Negation Test ---"[..],
        &b"--- Double to Int Conversion Test ---"[..],
        &b"--- Memchr Search Test ---"[..],
        &b"--- Combined Feature Test ---"[..],
        &b"--- Special Double Values ---"[..],
        &b"Accumulated result: "[..],
    ] {
        assert!(
            cout.windows(marker.len()).any(|w| w == marker),
            "{ctx}: captured C stdout is missing {:?} — capture is broken.\nGot: {}",
            String::from_utf8_lossy(marker),
            String::from_utf8_lossy(&cout)
        );
    }
    assert_bytes_eq(&cout, &rout, &format!("{ctx} stdout"));
}

#[test]
fn doubleneg_all_configurations() {
    // -------------------------------------------------------------------
    // Rows 31..37 — return value only (stdout sent to /dev/null in one batch).
    // -------------------------------------------------------------------
    silence_stdout(|| {
        // Row 31: all-zero parameters (every `!!` false, `b == 0` guard taken).
        cmp_value("row31", (0, 0, 0, 0));

        // Row 32: full 2^4 zero / non-zero truth table, with several distinct
        // non-zero representatives so the pattern is not confounded with a value.
        let mut rng = Rng::new(0x2020);
        let nonzeros: [i32; 8] = [1, -1, 2, 7, 255, 256, i32::MAX, i32::MIN];
        for mask in 0u32..16 {
            for &nz in &nonzeros {
                let pick = |bit: u32| if mask & (1 << bit) != 0 { nz } else { 0 };
                cmp_value("row32", (pick(0), pick(1), pick(2), pick(3)));
            }
            for _ in 0..40 {
                let mut pick = |bit: u32| {
                    if mask & (1 << bit) != 0 {
                        loop {
                            let v = rng.interesting_i32();
                            if v != 0 {
                                return v;
                            }
                        }
                    } else {
                        0
                    }
                };
                cmp_value("row32", (pick(0), pick(1), pick(2), pick(3)));
            }
        }

        // Row 33: p2 == 0 -> the `if (b != 0)` divisor guard inside the pipeline.
        for _ in 0..500 {
            cmp_value(
                "row33",
                (
                    rng.interesting_i32(),
                    0,
                    rng.interesting_i32(),
                    rng.interesting_i32(),
                ),
            );
        }

        // Row 34: p3 realising every `c % 10` remainder in -9..=9.
        for r in -9i32..=9 {
            for mult in 0..12i32 {
                let p3 = r + if r >= 0 { 10 * mult } else { -10 * mult };
                cmp_value("row34", (1, 1, p3, 1));
                cmp_value("row34", (355, -113, p3, 42));
                cmp_value("row34", (0, 3, p3, 0));
            }
        }
        for p3 in [i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1] {
            cmp_value("row34", (17, 5, p3, 9));
        }

        // Row 35: p1 over every buffer-seed residue (each rotates the byte
        // permutation, changing every search position).
        for p1 in -256i32..=256 {
            cmp_value("row35", (p1, 3, 5, 7));
        }
        for _ in 0..300 {
            let p1 = rng.range_i32(-100_000, 100_000);
            cmp_value("row35", (p1, 1, 1, 1));
        }

        // Row 36: extremes, one parameter at a time and all together.
        let ext = [i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1, -1, 1];
        for &e in &ext {
            cmp_value("row36", (e, 3, 5, 7));
            cmp_value("row36", (3, e, 5, 7));
            cmp_value("row36", (3, 5, e, 7));
            cmp_value("row36", (3, 5, 7, e));
            cmp_value("row36", (e, e, e, e));
        }
        for &a in &ext {
            for &b in &ext {
                cmp_value("row36", (a, b, a, b));
                cmp_value("row36", (a, b, b, a));
            }
        }

        // Row 37: randomized over the full int domain.
        let mut rng37 = Rng::new(0x3737);
        for _ in 0..3000 {
            cmp_value(
                "row37",
                (
                    rng37.interesting_i32(),
                    rng37.interesting_i32(),
                    rng37.interesting_i32(),
                    rng37.interesting_i32(),
                ),
            );
        }
        for _ in 0..2000 {
            cmp_value(
                "row37",
                (
                    rng37.next_i32(),
                    rng37.next_i32(),
                    rng37.next_i32(),
                    rng37.next_i32(),
                ),
            );
        }
    });

    // -------------------------------------------------------------------
    // Row 38 — stdout bytes compared exactly (covers `%d`, `%e`, `%ld`
    // formatting and GCC's `printf("lit\n")` -> `puts("lit")` rewrite).
    // -------------------------------------------------------------------
    let named: [(i32, i32, i32, i32); 20] = [
        (0, 0, 0, 0),
        (1, 1, 1, 1),
        (-1, -1, -1, -1),
        (1, 0, 0, 0),
        (0, 1, 0, 0),
        (0, 0, 1, 0),
        (0, 0, 0, 1),
        (42, 42, 42, 42),
        (1, 3, 0, 0),
        (1, 3, 5, 7),
        (1, 3, -5, 7),
        (100, 7, 9, 255),
        (255, 256, 257, 258),
        (i32::MAX, i32::MAX, i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN, i32::MIN, i32::MIN),
        (i32::MAX, -1, 9, i32::MIN),
        (i32::MIN, 1, -9, i32::MAX),
        (-2147483648, 3, 0, -1),
        (7, -13, 123456789, -987654321),
        (1000000, 3, 1, 2),
    ];
    for p in named {
        cmp_value_and_stdout("row38-named", p);
    }

    let mut rng = Rng::new(0x3838);
    for _ in 0..250 {
        cmp_value_and_stdout(
            "row38-random",
            (
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
            ),
        );
    }
    for _ in 0..150 {
        cmp_value_and_stdout(
            "row38-random-full",
            (
                rng.next_i32(),
                rng.next_i32(),
                rng.next_i32(),
                rng.next_i32(),
            ),
        );
    }
}
