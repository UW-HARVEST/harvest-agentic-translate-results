//! Phase B — valid-path differential tests through the **executables**. Covers
//! `CONFIGS.md` rows 12–17 and 23–39.
//!
//! This channel is what can observe process death, so it owns the rows where the
//! C code's unchecked `buffer[data] = 1` corrupts a live return address.

mod common;
use common::{exe, Rng, SEED};

/// stdin for the whole program: line 1 feeds `goodB2G`, line 2 feeds `bad`.
fn two(a: &str, b: &str) -> Vec<u8> {
    format!("{a}\n{b}\n").into_bytes()
}

// ---------------------------------------------------------------------------
// Rows 12–17: the `bad()` index classes, end to end
// ---------------------------------------------------------------------------

/// Row 12 — exhaustive over every index from -8 up to the probed deterministic
/// limit (at least 28, typically 200+), repeated, so that a nondeterministic
/// outcome cannot slip through as an accidental pass.
#[test]
fn row12_bad_index_exhaustive_reproducible() {
    let hi = common::deterministic_benign_limit().min(200);
    for k in -8i64..=hi {
        let stdin = two("0", &k.to_string());
        let (c0, r0) = exe::both(&stdin);
        assert_eq!(
            c0, r0,
            "\nDIVERGENCE [bad idx {k}]\n  C    = {c0:?}\n  Rust = {r0:?}\n"
        );
        // Both sides must also be stable across runs for these indices; the C
        // binary was measured to be 100% reproducible over 0..=1300.
        for rep in 1..8 {
            let (c, r) = exe::both(&stdin);
            assert_eq!(c, c0, "C binary nondeterministic at idx {k}, rep {rep}");
            assert_eq!(r, r0, "Rust binary nondeterministic at idx {k}, rep {rep}");
        }
    }
}

/// Row 13 — indices 16..=19 overwrite `bad()`'s saved rbp / return address.
/// The C process dies from SIGSEGV *before* stdio is ever flushed, so stdout is
/// empty. Both sides must show exactly that, not merely "both failed somehow".
#[test]
fn row13_bad_index_corrupts_own_frame() {
    for k in 16..=19 {
        let stdin = two("0", &k.to_string());
        let (c, r) = exe::both(&stdin);
        assert_eq!(c, r, "\nDIVERGENCE [bad idx {k}]\n C={c:?}\n R={r:?}\n");
        assert_eq!(
            c.status,
            Err(libc::SIGSEGV),
            "expected C to die from SIGSEGV at idx {k}, got {c:?}"
        );
        assert!(
            c.stdout.is_empty(),
            "expected C to lose its buffered stdout at idx {k}, got {c:?}"
        );
    }
}

/// Row 14 — indices 20..=25 land in `main()`'s argc/argv/saved-rbp: benign.
#[test]
fn row14_bad_index_caller_frame_benign() {
    for k in 20..=25 {
        let stdin = two("0", &k.to_string());
        let (c, r) = exe::both(&stdin);
        assert_eq!(c, r, "\nDIVERGENCE [bad idx {k}]\n C={c:?}\n R={r:?}\n");
        assert_eq!(c.status, Ok(0), "expected benign exit at idx {k}, got {c:?}");
    }
}

/// Row 15 — indices 26..=27 overwrite `main()`'s return address.
#[test]
fn row15_bad_index_corrupts_main_return_address() {
    for k in 26..=27 {
        let stdin = two("0", &k.to_string());
        let (c, r) = exe::both(&stdin);
        assert_eq!(c, r, "\nDIVERGENCE [bad idx {k}]\n C={c:?}\n R={r:?}\n");
        assert_eq!(
            c.status,
            Err(libc::SIGSEGV),
            "expected C to die from SIGSEGV at idx {k}, got {c:?}"
        );
        assert!(c.stdout.is_empty(), "expected empty stdout at idx {k}");
    }
}

/// Row 16 — above `main()`'s frame but still inside the mapped stack: benign and
/// deterministic in C. The upper bound is *probed at runtime* because it depends
/// on the size of the environment block (see `deterministic_benign_limit`).
#[test]
fn row16_bad_index_far_but_mapped() {
    let limit = common::deterministic_benign_limit();
    assert!(limit >= 28, "probe produced a useless limit: {limit}");
    let mut rng = Rng::new(SEED ^ 16);
    let mut ks: Vec<i64> = (28..=64.min(limit)).collect();
    for step in [100i64, 200, 400, 800, 1600, 3200] {
        if step <= limit {
            ks.push(step);
        }
    }
    for _ in 0..60 {
        ks.push(rng.in_range(28, limit));
    }
    for k in ks {
        let stdin = two("0", &k.to_string());
        let (c, r) = exe::both(&stdin);
        assert_eq!(c.status, Ok(0), "expected benign C exit at idx {k}, got {c:?}");
        assert_eq!(c, r, "\nDIVERGENCE [bad idx {k}]\n C={c:?}\n R={r:?}\n");
    }
}

/// Row 17 — indices far past the top of the stack mapping always fault.
///
/// Two sub-regions, because C is only *partly* deterministic here:
///   * `FAR_FATAL_MIN ..= SIGNAL_KIND_UNSTABLE_MIN` — reproducibly `SIGSEGV`
///     (240/240 random samples, and 20/20 per fixed index under empty, minimal
///     and inherited environments), so exact equality of signal *and* stdout is
///     asserted.
///   * above that — ASLR decides whether the fault is `SIGSEGV` or `SIGBUS`
///     (one `SIGBUS` in 120 samples already at k ≈ 2.8e7, rising to 33% at
///     `i32::MAX`), so only "died from a fatal signal with no output" is
///     asserted. Pinning the signal number there would be asserting against C's
///     own coin flip.
#[test]
fn row17_bad_index_off_stack_always_faults() {
    use common::{FAR_FATAL_MIN, SIGNAL_KIND_UNSTABLE_MIN};
    let mut rng = Rng::new(SEED ^ 17);

    // Sub-region A: signal kind is reproducible.
    let mut ks: Vec<i64> = vec![FAR_FATAL_MIN, 60_000, 100_000, 500_000, 999_999];
    for _ in 0..30 {
        ks.push(rng.in_range(FAR_FATAL_MIN, SIGNAL_KIND_UNSTABLE_MIN));
    }
    for k in ks {
        let stdin = two("0", &k.to_string());
        let (c, r) = exe::both(&stdin);
        assert_eq!(
            c.status,
            Err(libc::SIGSEGV),
            "expected reproducible C SIGSEGV for far idx {k}, got {c:?}"
        );
        assert_eq!(c, r, "\nDIVERGENCE [bad far idx {k}]\n C={c:?}\n R={r:?}\n");
    }

    // Sub-region B: only the fact of death is reproducible.
    let mut ks: Vec<i64> = vec![1_000_000_000, 2_000_000_000, i32::MAX as i64];
    for _ in 0..20 {
        ks.push(rng.in_range(SIGNAL_KIND_UNSTABLE_MIN, i32::MAX as i64));
    }
    for k in ks {
        let stdin = two("0", &k.to_string());
        let (c, r) = exe::both(&stdin);
        assert!(
            c.status.is_err(),
            "expected C to die for huge idx {k}, got {c:?}"
        );
        assert!(
            r.status.is_err(),
            "expected Rust to die for huge idx {k}, got {r:?}"
        );
        assert!(
            c.stdout.is_empty() && r.stdout.is_empty(),
            "expected both to lose buffered stdout at idx {k}: C={c:?} Rust={r:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 23–26: line-count shapes
// ---------------------------------------------------------------------------

#[test]
fn row23_two_lines_cross_product_of_classes() {
    let limit = common::deterministic_benign_limit();
    // One representative per class for goodB2G's line (its sink is bounds-checked,
    // so any i32 is safe there), crossed with the *deterministic* classes for
    // bad()'s line, plus randomized values inside each class.
    let a_classes: [i64; 14] = [
        i32::MIN as i64, -1, 0, 5, 9, 10, 11, 14, 15, 20, 26, 100, 100000, i32::MAX as i64,
    ];
    let b_classes: Vec<i64> = [-1i64, 0, 5, 9, 10, 11, 14, 15, 16, 18, 20, 25, 26, 27]
        .into_iter()
        .chain([100i64, 1000].into_iter().filter(|&k| k <= limit))
        .collect();
    for a in a_classes {
        for b in &b_classes {
            let stdin = two(&a.to_string(), &b.to_string());
            exe::assert_same(&stdin, &format!("cross a={a} b={b}"));
        }
    }
    let mut rng = Rng::new(SEED ^ 23);
    for _ in 0..300 {
        let a = rng.in_range(i32::MIN as i64, i32::MAX as i64);
        // Keep bad()'s index inside the probed deterministic region.
        let b = rng.in_range(-100, limit);
        let stdin = two(&a.to_string(), &b.to_string());
        exe::assert_same(&stdin, &format!("cross rnd a={a} b={b}"));
    }
}

#[test]
fn row24_zero_lines() {
    exe::assert_same(b"", "no input at all");
}

#[test]
fn row25_one_line_only() {
    for s in ["0", "5", "-1", "10", "abc", ""] {
        exe::assert_same(format!("{s}\n").as_bytes(), &format!("single line {s:?}"));
        exe::assert_same(s.as_bytes(), &format!("single line no newline {s:?}"));
    }
}

#[test]
fn row26_more_than_two_lines() {
    exe::assert_same(b"1\n2\n3\n4\n5\n", "five lines");
    exe::assert_same(b"1\n2\nignored garbage\n", "third line ignored");
    let mut rng = Rng::new(SEED ^ 26);
    for _ in 0..60 {
        let n = rng.in_range(3, 8) as usize;
        let mut s = String::new();
        for _ in 0..n {
            s.push_str(&format!("{}\n", rng.in_range(-20, 30)));
        }
        exe::assert_same(s.as_bytes(), "many lines");
    }
}

// ---------------------------------------------------------------------------
// Rows 27–29: the fgets(.,14,.) truncation boundary
// ---------------------------------------------------------------------------

#[test]
fn row27_line_exactly_13_bytes() {
    // 13 bytes exactly fills inputBuffer; the newline is NOT consumed and is
    // therefore left for the next fgets.
    exe::assert_same(b"1234567890123\n5\n", "13-byte line then 5");
    exe::assert_same(b"0000000000007\n3\n", "13-byte leading zeros");
    exe::assert_same(b"             \n3\n", "13 spaces");
    let mut rng = Rng::new(SEED ^ 27);
    for _ in 0..80 {
        let mut s: String = (0..13)
            .map(|_| char::from(b'0' + rng.in_range(0, 9) as u8))
            .collect();
        s.push('\n');
        s.push_str("2\n");
        exe::assert_same(s.as_bytes(), "random 13-digit line");
    }
}

#[test]
fn row28_line_longer_than_13_bytes_truncates() {
    // The remainder of an over-long first line is what bad()'s fgets receives.
    // Constructing the tail out of leading zeros keeps bad()'s index small and
    // therefore fully deterministic, so these can be asserted strictly.
    exe::assert_same(b"00000000000000000005\n", "20 bytes, tail parses to 5");
    exe::assert_same(b"12345678901230000000\n", "tail is all zeros");
    exe::assert_same(b"9999999999999000009\n", "13 nines then tail 9");
    exe::assert_same(b"1234567890123-0000005\n", "tail negative");
    let mut rng = Rng::new(SEED ^ 28);
    let mut compared = 0;
    for _ in 0..60 {
        // 13 leading digits, then a tail whose value is a controlled small index.
        let head: String = (0..13)
            .map(|_| char::from(b'0' + rng.in_range(0, 9) as u8))
            .collect();
        let idx = rng.in_range(0, 15);
        let tail = format!("{:0>6}", idx); // leading zeros keep it small
        let s = format!("{head}{tail}\n");
        exe::assert_same(s.as_bytes(), &format!("truncated, tail idx {idx}"));
        compared += 1;
    }
    // Fully random over-long lines: the resulting index is uncontrolled and may
    // land in C's nondeterministic band, so signal deaths are skipped.
    let mut soup_compared = 0;
    for _ in 0..120 {
        let n = rng.in_range(14, 40) as usize;
        let mut s: String = (0..n)
            .map(|_| char::from(b'0' + rng.in_range(0, 9) as u8))
            .collect();
        s.push('\n');
        if exe::assert_same_if_no_crash(s.as_bytes(), &format!("over-long line len {n}")) {
            soup_compared += 1;
        }
    }
    assert!(compared == 60, "controlled truncation cases did not all run");
    assert!(
        soup_compared > 0,
        "every random over-long line crashed -- this row became vacuous"
    );
}

#[test]
fn row29_single_long_digit_run_spans_both_fgets() {
    // Runs of zeros keep both indices deterministic, so assert strictly.
    for n in [14usize, 15, 20, 26, 27, 30, 39, 40] {
        let s = "0".repeat(n) + "\n";
        exe::assert_same(s.as_bytes(), &format!("{n} zeros"));
        // A run of digits ending in a small value: the second fgets sees the tail.
        let s = "0".repeat(n - 1) + "7\n";
        exe::assert_same(s.as_bytes(), &format!("{n} zeros ending in 7"));
    }
    // Runs of 1s / 9s produce huge indices, which fall in the unmatchable band.
    let mut compared = 0;
    for n in [14usize, 15, 20, 26, 27, 30, 39, 40] {
        for d in ['1', '9'] {
            let s: String = std::iter::repeat(d).take(n).chain(['\n']).collect();
            if exe::assert_same_if_no_crash(s.as_bytes(), &format!("{n} x {d}")) {
                compared += 1;
            }
        }
    }
    assert!(compared > 0, "row 29 large-digit cases became vacuous");
}

// ---------------------------------------------------------------------------
// Rows 30–37: atoi input spellings
// ---------------------------------------------------------------------------

#[test]
fn row30_leading_whitespace_forms() {
    for ws in [" ", "\t", "\x0b", "\x0c", "\r", "  \t ", "\t\t\t"] {
        for v in ["0", "5", "9", "12", "-3"] {
            let s = format!("{ws}{v}\n{ws}{v}\n");
            exe::assert_same(s.as_bytes(), &format!("ws {ws:?} val {v}"));
        }
    }
    // Whitespace only -- atoi sees no digits and returns 0.
    for ws in [" ", "\t", "   ", "\x0b\x0c"] {
        let s = format!("{ws}\n{ws}\n");
        exe::assert_same(s.as_bytes(), &format!("ws only {ws:?}"));
    }
}

#[test]
fn row31_explicit_signs() {
    for s in [
        "+5\n+5\n", "-5\n-5\n", "+0\n+0\n", "-0\n-0\n", "+\n+\n", "-\n-\n",
        "+ 5\n+ 5\n", " +5\n +5\n", " -5\n -5\n", "++5\n++5\n", "--5\n--5\n",
        "+-5\n+-5\n", "-+5\n-+5\n",
    ] {
        exe::assert_same(s.as_bytes(), &format!("sign {s:?}"));
    }
}

#[test]
fn row32_leading_zeros() {
    for s in ["0000000000007\n0000000000003\n", "007\n003\n", "0\n0\n", "00\n00\n"] {
        exe::assert_same(s.as_bytes(), &format!("leading zeros {s:?}"));
    }
}

#[test]
fn row33_non_numeric_and_mixed() {
    for s in [
        "abc", "3abc", "abc3", "0x10", "0X10", "1e3", "1.9", ".5", "5.", "e5",
        "x", "z", "--5", "++5", "5-", "5+", "1 2", "1\t2", " a5", "9a9",
    ] {
        let t = format!("{s}\n{s}\n");
        exe::assert_same(t.as_bytes(), &format!("nonnumeric {s:?}"));
    }
}

#[test]
fn row34_crlf_line_endings() {
    for v in ["0", "7", "9", "10", "-1"] {
        let s = format!("{v}\r\n{v}\r\n");
        exe::assert_same(s.as_bytes(), &format!("crlf {v}"));
    }
    exe::assert_same(b"\r\n\r\n", "crlf empty lines");
}

#[test]
fn row35_no_trailing_newline() {
    exe::assert_same(b"3\n5", "second line unterminated");
    exe::assert_same(b"3", "first line unterminated, no second");
    exe::assert_same(b"-1", "unterminated negative");
    exe::assert_same(b"1234567890123", "unterminated 13 bytes");
}

#[test]
fn row36_embedded_nul_bytes() {
    for s in [
        &b"5\x006\n5\x006\n"[..],
        b"\x005\n\x005\n",
        b"\x00\n\x00\n",
        b"12\x0034\n12\x0034\n",
        b"-\x001\n-\x001\n",
    ] {
        exe::assert_same(s, "embedded NUL");
    }
}

#[test]
fn row37_values_straddling_int_and_long_limits() {
    for s in [
        "2147483647",   // INT_MAX
        "2147483646",
        "2147483648",   // INT_MAX + 1
        "-2147483648",  // INT_MIN
        "-2147483647",
        "-2147483649",  // INT_MIN - 1
        "4294967295",   // UINT_MAX -> -1 after truncation
        "4294967296",   // 2^32 -> 0
        "4294967306",   // 2^32 + 10
        "9999999999999",  // 13 digits, > INT_MAX, < LONG_MAX
        "1000000000000",
        "1234567890123",
        "-999999999999",
        "-000000000001",
        "0000000000000",
    ] {
        // As the first line only, bad() sees a fresh EOF -> fully deterministic.
        let t = format!("{s}\n");
        exe::assert_same(t.as_bytes(), &format!("limit first-only {s}"));
        // As both lines, bad()'s index is the (possibly huge) truncated value,
        // which may fall in C's nondeterministic band, so tolerate a crash.
        let t = format!("{s}\n{s}\n");
        exe::assert_same_if_no_crash(t.as_bytes(), &format!("limit {s}"));
        // Paired with a small, deterministic index for bad().
        let t = format!("{s}\n3\n");
        exe::assert_same(t.as_bytes(), &format!("limit {s} then idx 3"));
    }
}

// ---------------------------------------------------------------------------
// Rows 38–39: randomized soup
// ---------------------------------------------------------------------------

/// Row 38 — random byte soup. The resulting `bad()` index is whatever `atoi`
/// makes of the second 13-byte chunk, so it is uncontrolled; signal deaths are
/// skipped (they can only come from the unchecked store, whose outcome is
/// nondeterministic in C for large indices). Every normal exit is compared byte
/// for byte, which is what exercises the atoi/fgets surface.
#[test]
fn row38_random_byte_soup() {
    let alpha: &[u8] = b"0123456789+- \t\n.eExX\x00abz\r";
    let mut rng = Rng::new(SEED ^ 38);
    let mut compared = 0;
    let mut skipped = 0;
    for _ in 0..800 {
        let n = rng.in_range(0, 30) as usize;
        let s: Vec<u8> = (0..n)
            .map(|_| alpha[(rng.next_u64() % alpha.len() as u64) as usize])
            .collect();
        if exe::assert_same_if_no_crash(&s, "random byte soup") {
            compared += 1;
        } else {
            skipped += 1;
        }
    }
    // Guard against the row silently becoming vacuous.
    assert!(
        compared >= 600,
        "only {compared}/800 soup cases were comparable ({skipped} crashed) -- \
         the skip rule is masking too much"
    );
}

#[test]
fn row39_random_i32_pairs() {
    let limit = common::deterministic_benign_limit();
    let mut rng = Rng::new(SEED ^ 39);
    for _ in 0..300 {
        // goodB2G's line can be any i32 (its sink is bounds-checked).
        let a = rng.i32_any() as i64;
        // bad()'s line is kept inside the probed deterministic region, including
        // the fatal classes 16..19 / 26..27, which are environment-independent.
        let b = match rng.next_u64() % 4 {
            0 => rng.in_range(-30, 30),
            1 => rng.in_range(-limit, limit),
            2 => *rng.pick(&common::FATAL_INDICES),
            _ => rng.in_range(0, 15),
        };
        let stdin = two(&a.to_string(), &b.to_string());
        exe::assert_same(&stdin, &format!("rnd pair {a},{b}"));
    }
}
