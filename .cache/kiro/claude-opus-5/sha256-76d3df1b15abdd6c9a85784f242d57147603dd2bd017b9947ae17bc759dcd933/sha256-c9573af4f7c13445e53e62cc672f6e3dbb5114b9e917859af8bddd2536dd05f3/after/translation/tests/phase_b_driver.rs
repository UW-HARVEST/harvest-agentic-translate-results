//! Phase B, part 2 — `CONFIGS.md` rows C16..C33: the `driver` entry point and
//! the composed `driver -> call_fma -> fma_array` pipeline.
//!
//! `driver` reports its result by `printf`-ing to stdout, so verifying it means
//! capturing fd 1 and comparing the bytes the C `.so` and the Rust `.so` each
//! wrote. fd 1 is process-global: if libtest's own progress output ("test
//! c18_... ok") is written by another thread while the redirect is in place it
//! lands in the capture file and shows up as a spurious diff. That is why every
//! row here is a plain function invoked from a SINGLE `#[test]`, and why this is
//! its own test binary — with one test, libtest writes its progress line before
//! the run starts and after it finishes, never during a capture. Cargo runs
//! test binaries one at a time, so no other binary competes either.

mod common;

use common::*;

/// Per-row iteration count.
const ITERS: usize = 200;

/// Every C16..C33 row, run in sequence inside one test.
#[test]
fn phase_b_driver_rows() {
    c16_driver_single_int_random();
    c17_driver_few_ints_space_random();
    c18_driver_mixed_whitespace_random();
    c19_driver_leading_whitespace_random();
    c20_driver_trailing_whitespace_random();
    c21_driver_explicit_plus_signs_random();
    c22_driver_leading_zeros_random();
    c23_driver_exactly_99();
    c24_driver_exactly_100();
    c25_driver_past_cap_random();
    c26_driver_ints_then_garbage_random();
    c27_driver_int_glued_to_garbage_random();
    c28_driver_baseish_tokens();
    c29_driver_nonspace_separators_random();
    c30_driver_out_of_range_numerals_random();
    c31_driver_last_value_extremes();
    c32_driver_fuzz_random();
    c34_driver_raw_bytes_fuzz();
    c33_pipeline_cross_check();
}

// ===========================================================================
// C16..C32 — `driver`, the top-level entry point (stdout compared byte-for-byte)
// ===========================================================================

/// Whitespace runs the C's `%d` conversion must skip.
const SEPS: &[&str] = &[" ", "  ", "\t", "\n", "\r", "\u{b}", "\u{c}", " \t\n", "\n\n  "];

/// Builds `n` random integers joined by `sep`.
fn joined(rng: &mut Rng, n: usize, sep: &str) -> (String, Vec<i32>) {
    let vals: Vec<i32> = (0..n).map(|_| rng.i32_full()).collect();
    let s = vals
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(sep);
    (s, vals)
}

/// C16 — one integer, no surrounding whitespace, full `i32` range.
fn c16_driver_single_int_random() {
    let mut rng = Rng::new(SEED ^ 16);
    for it in 0..ITERS {
        let v = rng.i32_full();
        let out = assert_driver_matches(&v.to_string(), &format!("C16 it={it} v={v}"));
        assert_eq!(
            out,
            format!("{v}\n").into_bytes(),
            "C16 it={it}: unexpected shared output"
        );
    }
    // Plus the exact boundary literals, not just random draws.
    for v in [0i32, 1, -1, i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1] {
        assert_driver_matches(&v.to_string(), &format!("C16 boundary v={v}"));
    }
}

/// C17 — 2..=10 integers, single-space separated.
fn c17_driver_few_ints_space_random() {
    let mut rng = Rng::new(SEED ^ 17);
    for it in 0..ITERS {
        let n = rng.range(2, 10);
        let (s, vals) = joined(&mut rng, n, " ");
        let out = assert_driver_matches(&s, &format!("C17 it={it} n={n}"));
        assert_eq!(
            out,
            format!("{}\n", vals[n - 1]).into_bytes(),
            "C17 it={it}: expected the last integer"
        );
    }
}

/// C18 — 2..=100 integers with a randomly chosen whitespace run in every gap.
fn c18_driver_mixed_whitespace_random() {
    let mut rng = Rng::new(SEED ^ 18);
    for it in 0..ITERS {
        let n = rng.range(2, 100);
        let vals: Vec<i32> = (0..n).map(|_| rng.i32_full()).collect();
        let mut s = String::new();
        for (i, v) in vals.iter().enumerate() {
            if i > 0 {
                s.push_str(rng.pick(SEPS));
            }
            s.push_str(&v.to_string());
        }
        let out = assert_driver_matches(&s, &format!("C18 it={it} n={n}"));
        assert_eq!(
            out,
            format!("{}\n", vals[n - 1]).into_bytes(),
            "C18 it={it}"
        );
    }
}

/// C19 — randomised leading whitespace before the first integer.
fn c19_driver_leading_whitespace_random() {
    let mut rng = Rng::new(SEED ^ 19);
    for it in 0..ITERS {
        let n = rng.range(1, 12);
        let (body, vals) = joined(&mut rng, n, " ");
        let mut s = String::new();
        for _ in 0..rng.range(1, 6) {
            s.push_str(rng.pick(SEPS));
        }
        s.push_str(&body);
        let out = assert_driver_matches(&s, &format!("C19 it={it} n={n}"));
        assert_eq!(
            out,
            format!("{}\n", vals[n - 1]).into_bytes(),
            "C19 it={it}"
        );
    }
}

/// C20 — randomised trailing whitespace: the loop's final `sscanf` returns EOF
/// (an input failure) rather than 0, but the printed value is unchanged.
fn c20_driver_trailing_whitespace_random() {
    let mut rng = Rng::new(SEED ^ 20);
    for it in 0..ITERS {
        let n = rng.range(1, 12);
        let (mut s, vals) = joined(&mut rng, n, " ");
        for _ in 0..rng.range(1, 6) {
            s.push_str(rng.pick(SEPS));
        }
        let out = assert_driver_matches(&s, &format!("C20 it={it} n={n}"));
        assert_eq!(
            out,
            format!("{}\n", vals[n - 1]).into_bytes(),
            "C20 it={it}"
        );
    }
}

/// C21 — explicit `+` on a random subset, mixed with `-` and bare digits.
fn c21_driver_explicit_plus_signs_random() {
    let mut rng = Rng::new(SEED ^ 21);
    for it in 0..ITERS {
        let n = rng.range(1, 20);
        let mut s = String::new();
        let mut vals = Vec::new();
        for i in 0..n {
            if i > 0 {
                s.push(' ');
            }
            // Keep magnitudes in range so an explicit '+' cannot overflow.
            let mag = (rng.next_u32() % (i32::MAX as u32)) as i32;
            let neg = rng.bool();
            let v = if neg { -mag } else { mag };
            vals.push(v);
            if neg {
                s.push_str(&format!("-{mag}"));
            } else if rng.bool() {
                s.push_str(&format!("+{mag}"));
            } else {
                s.push_str(&format!("{mag}"));
            }
        }
        let out = assert_driver_matches(&s, &format!("C21 it={it} n={n}"));
        assert_eq!(
            out,
            format!("{}\n", vals[n - 1]).into_bytes(),
            "C21 it={it}"
        );
    }
}

/// C22 — randomised leading zeros. `%d` is decimal-only, so `007` is `7`, and
/// the `%zn` byte count the C uses to advance must include the zeros.
fn c22_driver_leading_zeros_random() {
    let mut rng = Rng::new(SEED ^ 22);
    for it in 0..ITERS {
        let n = rng.range(1, 20);
        let mut s = String::new();
        let mut vals = Vec::new();
        for i in 0..n {
            if i > 0 {
                s.push(' ');
            }
            let mag = (rng.next_u32() % 100_000) as i32;
            let neg = rng.bool();
            let zeros = "0".repeat(rng.range(0, 8));
            vals.push(if neg { -mag } else { mag });
            if neg {
                s.push('-');
            }
            s.push_str(&zeros);
            s.push_str(&mag.to_string());
        }
        let out = assert_driver_matches(&s, &format!("C22 it={it} n={n}"));
        assert_eq!(
            out,
            format!("{}\n", vals[n - 1]).into_bytes(),
            "C22 it={it}"
        );
    }
}

/// C23 — exactly 99 integers, one below the `i < 100` cap.
fn c23_driver_exactly_99() {
    let mut rng = Rng::new(SEED ^ 23);
    for it in 0..60 {
        let (s, vals) = joined(&mut rng, 99, " ");
        let out = assert_driver_matches(&s, &format!("C23 it={it}"));
        assert_eq!(out, format!("{}\n", vals[98]).into_bytes(), "C23 it={it}");
    }
}

/// C24 — exactly 100 integers: the loop exits on the bound, not on a parse
/// failure, so the 100th `sscanf` is the last one attempted.
fn c24_driver_exactly_100() {
    let mut rng = Rng::new(SEED ^ 24);
    for it in 0..60 {
        let (s, vals) = joined(&mut rng, 100, " ");
        let out = assert_driver_matches(&s, &format!("C24 it={it}"));
        assert_eq!(out, format!("{}\n", vals[99]).into_bytes(), "C24 it={it}");
    }
}

/// C25 — 101..=250 integers: everything past the 100th is ignored.
fn c25_driver_past_cap_random() {
    let mut rng = Rng::new(SEED ^ 25);
    for it in 0..60 {
        let n = rng.range(101, 250);
        let (s, vals) = joined(&mut rng, n, " ");
        let out = assert_driver_matches(&s, &format!("C25 it={it} n={n}"));
        assert_eq!(
            out,
            format!("{}\n", vals[99]).into_bytes(),
            "C25 it={it}: expected the 100th integer, not the last"
        );
    }
}

/// Non-numeric tokens that make `sscanf` report a matching failure. Every entry
/// must start with a character `%d` cannot begin a conversion with — note that
/// `"0x"` does NOT belong here, because `%d` happily consumes its leading `0`
/// and returns 1. Base-prefixed tokens are covered by C28 instead.
const GARBAGE: &[&str] = &[
    "abc", "xyz", "-", "+", ".", ",", ";", "!", "?", "--", "++", "-+", "/", "*", "#", "e", "E",
    "inf", "nan", "NULL", "@@", "[]", "()",
];

/// C26 — `k` integers then non-numeric garbage: the mid-stream `break` path.
fn c26_driver_ints_then_garbage_random() {
    let mut rng = Rng::new(SEED ^ 26);
    for it in 0..ITERS {
        let k = rng.range(1, 100);
        let (mut s, vals) = joined(&mut rng, k, " ");
        s.push(' ');
        s.push_str(rng.pick(GARBAGE));
        if rng.bool() {
            // More integers after the garbage: they must never be reached.
            s.push_str(&format!(" {} {}", rng.i32_full(), rng.i32_full()));
        }
        let out = assert_driver_matches(&s, &format!("C26 it={it} k={k}"));
        assert_eq!(
            out,
            format!("{}\n", vals[k - 1]).into_bytes(),
            "C26 it={it}: expected the last integer before the garbage"
        );
    }
}

/// C27 — integer glued to letters with no separator (`"5abc"`): `%d` stops at
/// the first non-digit, `%zn` reports only the digits consumed, and the next
/// `sscanf` then fails on the letters.
fn c27_driver_int_glued_to_garbage_random() {
    let mut rng = Rng::new(SEED ^ 27);
    for it in 0..ITERS {
        let n = rng.range(1, 10);
        let (mut s, vals) = joined(&mut rng, n, " ");
        s.push_str(rng.pick(&["abc", "xyz", "q", "_", "e", "E", "z9", "!7"]));
        let out = assert_driver_matches(&s, &format!("C27 it={it} n={n}"));
        assert_eq!(
            out,
            format!("{}\n", vals[n - 1]).into_bytes(),
            "C27 it={it}"
        );
    }
}

/// C28 — tokens that look like another base or a float. `%d` takes the leading
/// decimal run only, so `0x10` yields `0` and stops at `x`.
fn c28_driver_baseish_tokens() {
    let cases: &[&str] = &[
        "0x10",
        "0X1F",
        "1e5",
        "1E5",
        "1.5",
        "-1.5",
        "0b1",
        "0o7",
        "3.14159",
        "1_000",
        "12e-3",
        ".5",
        "+.5",
        "1 0x10 2",
        "7 1.5 9",
        "0x10 7",
        "00x1",
        "1e",
        "5 6 0b11 7",
    ];
    for (i, s) in cases.iter().enumerate() {
        assert_driver_matches(s, &format!("C28 case={i} {s:?}"));
    }
    // Randomised: a base-ish token dropped at a random position in a valid run.
    let mut rng = Rng::new(SEED ^ 28);
    for it in 0..ITERS {
        let n = rng.range(1, 12);
        let pos = rng.range(0, n);
        let mut parts: Vec<String> = Vec::new();
        for i in 0..n {
            if i == pos {
                parts.push((*rng.pick(&["0x10", "1e5", "2.5", "0b1", "3.0"])).to_string());
            }
            parts.push(rng.i32_full().to_string());
        }
        if pos == n {
            parts.push((*rng.pick(&["0x10", "1e5", "2.5"])).to_string());
        }
        assert_driver_matches(&parts.join(" "), &format!("C28 rand it={it}"));
    }
}

/// C29 — non-whitespace separators. `%d` does not skip them, so the first value
/// parses and the loop then breaks on a matching failure.
fn c29_driver_nonspace_separators_random() {
    let mut rng = Rng::new(SEED ^ 29);
    for it in 0..ITERS {
        let sep = *rng.pick(&[",", ";", "|", ":", "/", "-", "+", "&", "=", ", ", " ,"]);
        let n = rng.range(2, 12);
        let (s, _) = joined(&mut rng, n, sep);
        assert_driver_matches(&s, &format!("C29 it={it} sep={sep:?}"));
    }
}

/// Decimal texts outside the `int` range. glibc's `%d` still returns 1 for
/// these, so they are accepted (saturated/truncated), not rejected.
const OVER_RANGE: &[&str] = &[
    "2147483648",
    "2147483649",
    "-2147483649",
    "-2147483650",
    "4294967296",
    "9223372036854775807",
    "9223372036854775808",
    "-9223372036854775809",
    "99999999999999999999",
    "-99999999999999999999",
    "18446744073709551617",
    "123456789012345678901234567890",
];

/// C30 — over-range numerals interleaved at random positions among valid ones.
fn c30_driver_out_of_range_numerals_random() {
    let mut rng = Rng::new(SEED ^ 30);
    for it in 0..ITERS {
        let n = rng.range(1, 20);
        let mut parts: Vec<String> = Vec::new();
        for _ in 0..n {
            if rng.range(0, 2) == 0 {
                parts.push((*rng.pick(OVER_RANGE)).to_string());
            } else {
                parts.push(rng.i32_full().to_string());
            }
        }
        assert_driver_matches(&parts.join(" "), &format!("C30 it={it} n={n}"));
    }
    // Each over-range literal on its own, and as the final (printed) element.
    for (i, s) in OVER_RANGE.iter().enumerate() {
        assert_driver_matches(s, &format!("C30 solo={i} {s}"));
        assert_driver_matches(&format!("1 2 {s}"), &format!("C30 last={i} {s}"));
    }
}

/// C31 — the printed value is `data[i-1]`, so pin the last element to each
/// extreme in turn.
fn c31_driver_last_value_extremes() {
    let mut rng = Rng::new(SEED ^ 31);
    let lasts: &[&str] = &[
        "0",
        "-0",
        "+0",
        "1",
        "-1",
        "2147483647",
        "-2147483648",
        "2147483646",
        "-2147483647",
        "000",
        "-000",
    ];
    for (li, last) in lasts.iter().enumerate() {
        for it in 0..20 {
            let n = rng.range(0, 30);
            let mut parts: Vec<String> = (0..n).map(|_| rng.i32_full().to_string()).collect();
            parts.push((*last).to_string());
            assert_driver_matches(&parts.join(" "), &format!("C31 li={li} it={it} n={n}"));
        }
    }
}

/// C32 — broad fuzz over the whole `driver` token grammar: valid integers,
/// signs, whitespace runs, garbage words and over-range numerals mixed freely,
/// up to 300 tokens.
fn c32_driver_fuzz_random() {
    let mut rng = Rng::new(SEED ^ 32);
    for it in 0..400 {
        let n = rng.range(1, 300);
        let mut s = String::new();
        for i in 0..n {
            if i > 0 && rng.range(0, 9) != 0 {
                // Usually separate tokens; occasionally glue them together to
                // exercise `%zn`-driven advancement over a partial token.
                s.push_str(rng.pick(SEPS));
            }
            match rng.range(0, 9) {
                0..=4 => s.push_str(&rng.i32_full().to_string()),
                5 => s.push_str(&format!("+{}", rng.next_u32() % (i32::MAX as u32))),
                6 => s.push_str(&format!(
                    "{}{}",
                    "0".repeat(rng.range(1, 5)),
                    rng.next_u32() % 1000
                )),
                7 => s.push_str(rng.pick(OVER_RANGE)),
                8 => s.push_str(rng.pick(GARBAGE)),
                _ => s.push_str(rng.pick(&["0x1f", "1e9", "2.5", "1_0", "0b1"])),
            }
        }
        assert_driver_matches(&s, &format!("C32 it={it} n={n}"));
    }
}

/// C34 — arbitrary raw bytes. `driver` takes a `const char *`, so any byte
/// except NUL is legal input, and `%d`'s leading-whitespace skip goes through
/// the locale's `isspace`, which high bytes (0x80..0xFF) can reach. Comparing
/// raw byte streams here is the broadest check that the Rust is genuinely
/// delegating to libc rather than parsing anything itself.
fn c34_driver_raw_bytes_fuzz() {
    let mut rng = Rng::new(SEED ^ 34);
    for it in 0..300 {
        let n = rng.range(0, 200);
        let mut bytes: Vec<u8> = Vec::with_capacity(n);
        for _ in 0..n {
            let b = match rng.range(0, 9) {
                // Weighted towards digits and whitespace so plenty of inputs
                // actually parse, rather than degenerating to "always rejects".
                0..=3 => b'0' + (rng.next_u32() % 10) as u8,
                4 => *rng.pick(&[b' ', b'\t', b'\n', b'\r', 0x0b, 0x0c]),
                5 => *rng.pick(&[b'-', b'+']),
                6 => (rng.next_u32() % 128) as u8,
                7 => 0x80u8.wrapping_add((rng.next_u32() % 128) as u8),
                _ => (rng.next_u32() % 256) as u8,
            };
            // NUL would just terminate the string early; skip it so the whole
            // generated buffer is actually exercised.
            bytes.push(if b == 0 { b' ' } else { b });
        }
        assert_driver_matches_bytes(&bytes, &format!("C34 it={it} n={n}"));
    }

    // Explicit high-byte and control-byte cases around a valid integer.
    for (i, filler) in [
        vec![0x80u8],
        vec![0xA0],
        vec![0xFF],
        vec![0x01],
        vec![0x1f],
        vec![0x7f],
        vec![0xC2, 0xA0], // UTF-8 NO-BREAK SPACE
        vec![0xE2, 0x80, 0x82],
    ]
    .iter()
    .enumerate()
    {
        let mut b = b"7".to_vec();
        b.extend_from_slice(filler);
        b.extend_from_slice(b"8");
        assert_driver_matches_bytes(&b, &format!("C34 highbyte={i}"));

        let mut b2 = filler.clone();
        b2.extend_from_slice(b"9");
        assert_driver_matches_bytes(&b2, &format!("C34 highbyte-lead={i}"));
    }
}

// ===========================================================================
// C33 — the composed pipeline
// ===========================================================================

/// C33 — cross-check the composed `driver -> call_fma -> fma_array` pipeline
/// against a direct `call_fma` on the same parsed prefix. A per-wrapper test
/// cannot see a bug where two stages' errors cancel; this can.
fn c33_pipeline_cross_check() {
    let mut rng = Rng::new(SEED ^ 33);

    // Mirror of the C parse loop, so the expected prefix is derived
    // independently of either implementation under test.
    fn parse_prefix(s: &str) -> Vec<i32> {
        let mut out = Vec::new();
        let mut rest = s;
        while out.len() < 100 {
            let t = rest.trim_start();
            let mut end = 0;
            let b = t.as_bytes();
            if end < b.len() && (b[end] == b'+' || b[end] == b'-') {
                end += 1;
            }
            let digits_start = end;
            while end < b.len() && b[end].is_ascii_digit() {
                end += 1;
            }
            if end == digits_start {
                break; // matching failure or EOF
            }
            match t[..end].parse::<i32>() {
                Ok(v) => out.push(v),
                // Over-range: glibc saturates, which this cross-check does not
                // model, so stop rather than assert a value we did not derive.
                Err(_) => break,
            }
            rest = &t[end..];
        }
        out
    }

    for it in 0..ITERS {
        let n = rng.range(0, 150);
        // Magnitudes kept inside `i32` so `parse_prefix` never hits saturation.
        let vals: Vec<i32> = (0..n).map(|_| rng.i32_full()).collect();
        let s = vals
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(" ");

        // Top of the pipeline: both `.so`s must print the same bytes.
        let out = assert_driver_matches(&s, &format!("C33 it={it} n={n}"));

        // Mid level, driven directly with the prefix `driver` would have built.
        // `parse_prefix` stops at 100 just as the C loop does, so the expected
        // prefix is the first 100 values at most.
        let prefix = parse_prefix(&s);
        let expected: &[i32] = &vals[..vals.len().min(100)];
        assert_eq!(prefix, expected, "C33 it={it}: independent parse disagrees");
        let len = prefix.len() as i32;
        let direct = assert_call_fma_matches(&prefix, len, &format!("C33 direct it={it}"));

        assert_eq!(
            out,
            format!("{direct}\n").into_bytes(),
            "C33 it={it}: driver output does not match a direct call_fma on the same prefix"
        );
    }
}
