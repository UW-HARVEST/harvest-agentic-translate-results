//! Phase C, part 2 — `ERRORS.md` rows E6..E11: `driver`'s rejection paths.
//!
//! `driver` has no error return; it reports everything through `printf`. Its
//! "rejection" sentinel is therefore the exact byte string it prints, and that
//! is what these rows assert — the same bytes from both `.so`s, and for the
//! total-rejection rows the specific sentinel `"0\n"` rather than merely "both
//! printed something".
//!
//! Same single-`#[test]`-per-binary arrangement as `phase_b_driver.rs`: fd 1 is
//! process-global, so libtest must not be writing progress lines to it while a
//! capture is in place.

mod common;

use common::*;

const ZERO: &[u8] = b"0\n";

// ===========================================================================

/// E6 — `sscanf` returns `EOF` (`-1`): an INPUT failure, meaning there was no
/// non-whitespace character for `%d` to look at. The loop breaks on iteration 0,
/// `call_fma(data, 0)` takes the E1 guard, and `"0\n"` is printed.
fn e6_driver_sscanf_eof_input_failure() {
    let cases: &[&str] = &[
        "",
        " ",
        "  ",
        "\t",
        "\n",
        "\r",
        "\u{b}",
        "\u{c}",
        "   \t\n\r ",
        "\n\n\n",
        "\t\t\t\t\t\t\t\t",
        " \t\n\r\u{b}\u{c} \t\n\r\u{b}\u{c}",
    ];
    for (i, s) in cases.iter().enumerate() {
        let out = assert_driver_matches(s, &format!("E6 case={i} {s:?}"));
        assert_eq!(
            out, ZERO,
            "E6 case={i} {s:?}: whitespace-only input must print exactly \"0\\n\""
        );
    }

    // Randomised whitespace-only strings of varying length and composition.
    let mut rng = Rng::new(SEED ^ 0x106);
    let ws = [' ', '\t', '\n', '\r', '\u{b}', '\u{c}'];
    for it in 0..200 {
        let n = rng.range(0, 40);
        let s: String = (0..n).map(|_| *rng.pick(&ws)).collect();
        let out = assert_driver_matches(&s, &format!("E6 rand it={it}"));
        assert_eq!(out, ZERO, "E6 rand it={it}: expected \"0\\n\"");
    }
}

/// E7 — `sscanf` returns `0`: a MATCHING failure. A character is present but
/// cannot begin an `int`. Same `break`, different `sscanf` return value, and the
/// same `"0\n"` sentinel out the other end.
fn e7_driver_sscanf_zero_matching_failure() {
    let cases: &[&str] = &[
        "abc",
        "xyz",
        "-",
        "+",
        ".",
        ",",
        ";",
        ":",
        "!",
        "?",
        "/",
        "*",
        "#",
        "@",
        "--",
        "++",
        "-+",
        "+-",
        "---5",
        ".5",
        "-.5",
        "+.5",
        "e5",
        "E5",
        "x10",
        "  abc",
        "\n\t-",
        "\r+",
        " , 1 2 3",
        "- 5",
        "+ 5",
        "-abc",
        "NULL",
        "nan",
        "inf",
        "()",
        "[]",
        "{}",
        "\u{b}\u{c}zzz",
        "'5'",
        "\"5\"",
        "(5)",
        "$5",
        "%d",
        "~",
        "|",
        "\\",
        "^7",
        "&8",
    ];
    for (i, s) in cases.iter().enumerate() {
        let out = assert_driver_matches(s, &format!("E7 case={i} {s:?}"));
        assert_eq!(
            out, ZERO,
            "E7 case={i} {s:?}: unparsable leading token must print exactly \"0\\n\""
        );
    }

    // Randomised: leading whitespace, then a token that cannot start an int,
    // then arbitrary valid integers that must never be reached.
    let mut rng = Rng::new(SEED ^ 0x107);
    let bad = [
        "abc", "-", "+", ".", ",", "--", "-+", ".5", "e", "z", "!", "@", "#", "/", "*",
    ];
    let ws = [" ", "\t", "\n", "\r", "  ", " \t\n"];
    for it in 0..200 {
        let mut s = String::new();
        for _ in 0..rng.range(0, 4) {
            s.push_str(rng.pick(&ws));
        }
        s.push_str(rng.pick(&bad));
        for _ in 0..rng.range(0, 5) {
            s.push(' ');
            s.push_str(&rng.i32_full().to_string());
        }
        let out = assert_driver_matches(&s, &format!("E7 rand it={it}"));
        assert_eq!(out, ZERO, "E7 rand it={it}: expected \"0\\n\"");
    }
}

/// E8 — `sscanf` succeeds `k` times (`1 <= k < 100`) then fails: the mid-stream
/// `break`. This is NOT reported as an error; `driver` prints `data[k-1]`, the
/// last integer it did manage to read. Everything after the failure point must
/// be ignored, including further valid integers.
fn e8_driver_partial_parse_then_failure() {
    let mut rng = Rng::new(SEED ^ 0x108);
    // Tokens that are certain to stop the parse: none may begin with a digit.
    let stoppers = [
        "abc", "-", "+", ".", ",", ";", "--", "-+", ".5", "e", "E", "z", "!", "@", "#", "/", "*",
        "nan", "inf", "NULL", "()",
    ];

    for it in 0..300 {
        let k = rng.range(1, 99);
        let vals: Vec<i32> = (0..k).map(|_| rng.i32_full()).collect();
        let mut s = vals
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        s.push(' ');
        s.push_str(rng.pick(&stoppers));
        // Trailing valid integers that must never be consumed.
        for _ in 0..rng.range(0, 6) {
            s.push(' ');
            s.push_str(&rng.i32_full().to_string());
        }

        let out = assert_driver_matches(&s, &format!("E8 it={it} k={k}"));
        assert_eq!(
            out,
            format!("{}\n", vals[k - 1]).into_bytes(),
            "E8 it={it} k={k}: expected the last integer parsed before the stopper"
        );
    }

    // The k == 1 boundary explicitly: exactly one good integer, then garbage.
    for (i, stopper) in stoppers.iter().enumerate() {
        let out = assert_driver_matches(&format!("42 {stopper}"), &format!("E8 k=1 case={i}"));
        assert_eq!(out, b"42\n", "E8 k=1 case={i}");
    }
}

/// E9 / E10 — the `i < 100` loop bound, which is what stops `int data[100]` from
/// being overrun. E10 is the boundary itself (exactly 100 integers, loop exits
/// on the bound rather than on a parse failure); E9 is one and many past it.
fn e9_driver_more_than_100_truncates() {
    let mut rng = Rng::new(SEED ^ 0x109);

    // E10 — exactly 100.
    for it in 0..40 {
        let vals: Vec<i32> = (0..100).map(|_| rng.i32_full()).collect();
        let s = vals
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        let out = assert_driver_matches(&s, &format!("E10 it={it}"));
        assert_eq!(out, format!("{}\n", vals[99]).into_bytes(), "E10 it={it}");
    }

    // E9 — 101 (one past), and a spread of larger counts.
    for &n in &[101usize, 102, 128, 200, 512, 1000] {
        for it in 0..10 {
            let vals: Vec<i32> = (0..n).map(|_| rng.i32_full()).collect();
            let s = vals
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(" ");
            let out = assert_driver_matches(&s, &format!("E9 n={n} it={it}"));
            assert_eq!(
                out,
                format!("{}\n", vals[99]).into_bytes(),
                "E9 n={n} it={it}: must print the 100th integer, not the {n}th"
            );
        }
    }

    // Past the cap, with a deliberately distinctive 100th element and garbage
    // planted after it that must never be looked at.
    for (i, &sentinel) in [0i32, 1, -1, i32::MAX, i32::MIN].iter().enumerate() {
        let mut parts: Vec<String> = (0..99).map(|_| rng.i32_full().to_string()).collect();
        parts.push(sentinel.to_string());
        for _ in 0..50 {
            parts.push(rng.i32_full().to_string());
        }
        parts.push("garbage".to_string());
        let out = assert_driver_matches(&parts.join(" "), &format!("E9 sentinel={i}"));
        assert_eq!(
            out,
            format!("{sentinel}\n").into_bytes(),
            "E9 sentinel={i}: expected the planted 100th value"
        );
    }
}

/// E11 — decimal text outside the `int` range. `driver` performs no range check,
/// and glibc's `%d` still returns 1 for these, so they are ACCEPTED (saturated
/// then truncated), not rejected: parsing continues past them. The exact
/// saturated value is whatever the shared libc produces, which is why this row
/// asserts C/Rust agreement rather than a hardcoded number.
fn e11_driver_int_overflow_accepted() {
    let over: &[&str] = &[
        "2147483648",
        "2147483649",
        "-2147483649",
        "-2147483650",
        "4294967295",
        "4294967296",
        "4294967297",
        "9223372036854775807",
        "9223372036854775808",
        "-9223372036854775808",
        "-9223372036854775809",
        "18446744073709551615",
        "18446744073709551616",
        "99999999999999999999",
        "-99999999999999999999",
        "123456789012345678901234567890",
        "-123456789012345678901234567890",
        "+2147483648",
        "00000000002147483648",
        "10000000000000000000000000000000000000000",
    ];

    // Alone (so the printed value IS the saturated/truncated one). Only C/Rust
    // agreement is asserted here, not a particular value: glibc reads the
    // numeral into a `long` (saturating at `LONG_MAX`/`LONG_MIN`) and then
    // truncates to `int`, so the result can legitimately be anything — note
    // that "4294967296" is 2^32 and truncates to exactly 0, which is why the
    // printed bytes alone cannot distinguish acceptance from rejection. The
    // `cont` loop below is what proves these numerals are accepted.
    for (i, s) in over.iter().enumerate() {
        let out = assert_driver_matches(s, &format!("E11 solo={i} {s}"));
        assert!(
            out.ends_with(b"\n") && out.len() >= 2,
            "E11 solo={i} {s}: driver must still print a value and a newline, got {out:?}"
        );
    }

    // Followed by more integers, proving the parse continued rather than broke.
    let mut rng = Rng::new(SEED ^ 0x111);
    for (i, s) in over.iter().enumerate() {
        let tail = rng.i32_full();
        let out = assert_driver_matches(&format!("{s} {tail}"), &format!("E11 cont={i} {s}"));
        assert_eq!(
            out,
            format!("{tail}\n").into_bytes(),
            "E11 cont={i} {s}: parsing must continue past an over-range numeral"
        );
    }

    // Interleaved at random positions among ordinary integers.
    for it in 0..200 {
        let n = rng.range(1, 24);
        let parts: Vec<String> = (0..n)
            .map(|_| {
                if rng.bool() {
                    (*rng.pick(over)).to_string()
                } else {
                    rng.i32_full().to_string()
                }
            })
            .collect();
        assert_driver_matches(&parts.join(" "), &format!("E11 rand it={it} n={n}"));
    }
}

// ===========================================================================

/// Every E6..E11 row, run in sequence inside one test so nothing else in the
/// process writes to fd 1 while a capture is in place.
#[test]
fn phase_c_driver_error_rows() {
    e6_driver_sscanf_eof_input_failure();
    e7_driver_sscanf_zero_matching_failure();
    e8_driver_partial_parse_then_failure();
    e9_driver_more_than_100_truncates();
    e11_driver_int_overflow_accepted();
}
