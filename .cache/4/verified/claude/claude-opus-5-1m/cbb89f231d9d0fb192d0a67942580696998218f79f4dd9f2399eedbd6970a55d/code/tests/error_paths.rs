//! Phase C -- error-path differential tests, one test per `ERRORS.md` row
//! (rows 1-19; rows 20-22 are FFI-only and live in `ffi_driver_diff.rs`).
//!
//! Every rejection in this program has the same observable sentinel: `scanf`
//! does not assign, so `x` keeps its initialiser `0.f` and the program prints
//! `00000000` and exits 0.  Each test therefore asserts *both* that C and Rust
//! agree *and* that the shared result is exactly that sentinel -- never merely
//! "both failed somehow".
//!
//! Note the sentinel is *positive* zero even for inputs that started with '-':
//! glibc keeps the sign out of the buffer it hands to `strtof` and only applies
//! it after a successful conversion.

mod common;

use common::{diff_all, diff_and_expect, Rng};

/// The value of `float x = 0.f;` printed by `driver`, i.e. what every rejected
/// input produces.
const PLUS_ZERO: &str = "00000000\n";
const MINUS_ZERO: &str = "00000080\n";
const PLUS_INF: &str = "0000807f\n";
const MINUS_INF: &str = "000080ff\n";

/// ERRORS.md rows 1-2 -- EOF before any character, and whitespace-only input.
#[test]
fn eof_and_whitespace_only() {
    let cases: Vec<&[u8]> = vec![
        b"",
        b" ",
        b"\t",
        b"\n",
        b"\x0b",
        b"\x0c",
        b"\r",
        b" \t\n\x0b\x0c\r",
        b"\n\n\n\n\n",
        b"                                ",
    ];
    diff_and_expect("errors01_02", cases, PLUS_ZERO);
}

/// Generic boundary: fd 0 closed, so the read fails with EBADF rather than
/// reporting EOF.
#[test]
fn closed_stdin() {
    let c = common::run_with_closed_stdin(&common::c_exe());
    let r = common::run_with_closed_stdin(&common::rust_exe());
    assert_eq!(c, r, "closed stdin");
    assert_eq!(
        String::from_utf8_lossy(&c.stdout),
        PLUS_ZERO,
        "closed stdin should still print the +0 sentinel"
    );
}

/// ERRORS.md row 3 -- EOF immediately after a leading sign.
#[test]
fn eof_after_sign() {
    let cases: Vec<&[u8]> = vec![b"-", b"+", b"   -", b"\n\n+", b" \t -"];
    diff_and_expect("errors03", cases, PLUS_ZERO);
}

/// ERRORS.md row 4 -- the first non-space character cannot start a number.
#[test]
fn first_char_cannot_start_a_number() {
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for s in ["", "+", "-", "  ", " -"] {
        for bad in [
            "z", "Z", "q", "_", ",", "(", ")", "*", "/", "@", "#", "%", "&", "'", "\"", "[", "]",
            "{", "}", ":", ";", "<", ">", "?", "!", "~", "^", "|", "\\", "`", "$", "=", "e", "E",
            "e5", "E5", "p", "P", "p1", "x", "X", "x1", "g", "G", "b", "B", "c", "C", "d", "D",
            "f", "F", "a", "A",
        ] {
            cases.push(format!("{s}{bad}").into_bytes());
        }
    }
    diff_and_expect("errors04", cases, PLUS_ZERO);
}

/// ERRORS.md row 5 -- a second sign directly after the first.
#[test]
fn double_sign() {
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for a in ["-", "+"] {
        for b in ["-", "+"] {
            for tail in ["", "1", "1.5", "0x1", "inf", "nan"] {
                cases.push(format!("{a}{b}{tail}").into_bytes());
            }
        }
    }
    diff_and_expect("errors05", cases, PLUS_ZERO);
}

/// ERRORS.md row 6 -- NUL bytes and bytes >= 0x80.
#[test]
fn non_ascii_and_nul_bytes() {
    let mut cases: Vec<Vec<u8>> = vec![
        vec![0x00],
        vec![0x00, b'1', b'.', b'5'],
        vec![0x01],
        vec![0x1f],
        vec![0x7f],
        vec![0x80],
        vec![0xff],
        vec![0xc3, 0xa9],       // "e-acute" in UTF-8
        vec![0xcf, 0x80],       // "pi" in UTF-8
        vec![0xef, 0xbb, 0xbf], // UTF-8 BOM
        b"-\x80".to_vec(),
        b" \xff1.5".to_vec(),
    ];
    for b in 0x80u16..=0xff {
        cases.push(vec![b as u8, b'1', b'.', b'5']);
    }
    diff_and_expect("errors06", cases, PLUS_ZERO);
}

/// ERRORS.md rows 7-8 -- a truncated or misspelled `nan`.
#[test]
fn nan_word_truncated() {
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for s in ["", "+", "-", "  ", " -"] {
        for w in [
            "n", "N", "nx", "Nx", "n5", "n.", "n-", "n(", "na", "NA", "nA", "Na", "nax", "naX",
            "na5", "na.", "na-", "na(", "nab", "nam", "nao",
        ] {
            cases.push(format!("{s}{w}").into_bytes());
        }
    }
    diff_and_expect("errors07_08", cases, PLUS_ZERO);
}

/// ERRORS.md rows 9-10 -- a truncated or misspelled `inf`.
#[test]
fn inf_word_truncated() {
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for s in ["", "+", "-", "  ", " -"] {
        for w in [
            "i", "I", "ix", "Ix", "i5", "i.", "i-", "in", "IN", "iN", "In", "inx", "inX", "in5",
            "in.", "in-", "ina", "ine", "ino",
        ] {
            cases.push(format!("{s}{w}").into_bytes());
        }
    }
    diff_and_expect("errors09_10", cases, PLUS_ZERO);
}

/// ERRORS.md row 11 -- once `inf` is followed by an 'i', glibc commits to the
/// long spelling `infinity` and rejects everything else.
#[test]
fn inf_commits_to_infinity() {
    let mut rejected: Vec<Vec<u8>> = Vec::new();
    let mut accepted_pos: Vec<Vec<u8>> = Vec::new();
    let mut accepted_neg: Vec<Vec<u8>> = Vec::new();
    // Every strict prefix of "infinity" that is longer than "inf" is rejected,
    // as is any wrong letter after the 'i'.
    let word = "infinity";
    for s in ["", "+", "-"] {
        for len in 4..word.len() {
            rejected.push(format!("{s}{}", &word[..len]).into_bytes());
        }
        // wrong letter after the committed 'i', or the word cut short
        for wrong in ["infix", "infinit1", "infiXty", "infin1ty", "infini", "infinit"] {
            rejected.push(format!("{s}{wrong}").into_bytes());
        }
        // uppercase variants of the same truncations
        for len in 4..word.len() {
            rejected.push(format!("{s}{}", word[..len].to_uppercase()).into_bytes());
        }
    }
    diff_and_expect("errors11_rejected", rejected, PLUS_ZERO);

    // The complementary accepted cases: exactly "inf", "inf"+non-'i', and the
    // full "infinity" (with or without trailing junk).
    for w in [
        "inf", "INF", "infz", "inf5", "inf.", "inf-", "infinity", "INFINITY", "infinityy",
        "infinity!", "infinIty", "iNfInItY", "infinIty2",
    ] {
        accepted_pos.push(w.as_bytes().to_vec());
        accepted_pos.push(format!("+{w}").into_bytes());
        accepted_neg.push(format!("-{w}").into_bytes());
    }
    diff_and_expect("errors11_accepted_pos", accepted_pos, PLUS_INF);
    diff_and_expect("errors11_accepted_neg", accepted_neg, MINUS_INF);
}

/// ERRORS.md row 12 -- the `0x` prefix with nothing after it.
#[test]
fn hex_prefix_only() {
    let cases: Vec<&[u8]> = vec![
        b"0x", b"0X", b"-0x", b"-0X", b"+0x", b"+0X", b"  0x", b" -0X", b"\n-0x",
    ];
    diff_and_expect("errors12", cases, PLUS_ZERO);
}

/// ERRORS.md row 13 -- `0x` followed by something that is neither a hex digit
/// nor a '.'.
#[test]
fn hex_prefix_then_non_hex() {
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for s in ["", "+", "-"] {
        for p in ["0x", "0X"] {
            for bad in [
                "g", "G", "h", "z", "Z", "_", ",", "-", "+", " ", "\t", "\n", "(", ")", "*", "/",
                "x", "X", "q", "w", "y", "!", "#", "'", ":", "[", "]", "{", "}", "\\", "|", "~",
            ] {
                cases.push(format!("{s}{p}{bad}").into_bytes());
                cases.push(format!("{s}{p}{bad}5").into_bytes());
            }
        }
    }
    diff_and_expect("errors13", cases, PLUS_ZERO);
}

/// ERRORS.md row 14 -- `0x` followed directly by the exponent character: `p` is
/// only accepted after a digit, so the buffer is still exactly "0x".
#[test]
fn hex_prefix_then_exponent_char() {
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for s in ["", "+", "-"] {
        for p in ["0x", "0X"] {
            for tail in ["p", "P", "p1", "P1", "p+1", "p-1", "p+", "p-", "pp"] {
                cases.push(format!("{s}{p}{tail}").into_bytes());
            }
        }
    }
    diff_and_expect("errors14", cases, PLUS_ZERO);

    // Complement: 'e'/'E' are *hex digits*, not the exponent character, in the
    // hexadecimal grammar -- so "0xe" is 14.0, not a rejection.
    let mut fourteen: Vec<Vec<u8>> = Vec::new();
    for s in ["", "+"] {
        for p in ["0x", "0X"] {
            for tail in ["e", "E"] {
                fourteen.push(format!("{s}{p}{tail}").into_bytes());
            }
        }
    }
    diff_and_expect("errors14_hex_digit_e", fourteen, "00006041\n");
}

/// ERRORS.md row 15 -- the collected buffer is exactly "." (no digit anywhere),
/// so `strtof` consumes nothing.
#[test]
fn lone_decimal_point() {
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for s in ["", "+", "-", "  ", " -"] {
        for tail in [
            ".", ".e", ".e5", ".E5", ".x", ".z", ".p", ".p1", "..", "...", ".,", ".-", ".+", ". ",
            ".\t", ".\n", ".(",
        ] {
            cases.push(format!("{s}{tail}").into_bytes());
        }
    }
    diff_and_expect("errors15", cases, PLUS_ZERO);
}

/// ERRORS.md row 16 -- overflow assigns +-HUGE_VALF (and sets ERANGE, which the
/// C code ignores).
#[test]
fn overflow_to_infinity() {
    let pos = [
        "1e39",
        "3.4028236e38",
        "3.5e38",
        "0x1p128",
        "0x1.ffffffp127",
        "1e400",
        "1e1000000",
        "1e99999999999999999999",
        "340282356779733661637539395458142568449",
        "0x1p2000",
    ];
    diff_and_expect("errors16_pos", pos.iter().map(|s| s.as_bytes()), PLUS_INF);
    let neg: Vec<String> = pos.iter().map(|s| format!("-{s}")).collect();
    diff_and_expect("errors16_neg", neg, MINUS_INF);
}

/// ERRORS.md row 17 -- underflow assigns +-0 (and sets ERANGE).
#[test]
fn underflow_to_zero() {
    let pos = [
        "1e-46",
        "7e-46",
        "7.00649232162408534e-46",
        "0x1p-150",
        "0x0.8p-149",
        "1e-400",
        "1e-1000000",
        "1e-99999999999999999999",
        "0x1p-2000",
    ];
    diff_and_expect("errors17_pos", pos.iter().map(|s| s.as_bytes()), PLUS_ZERO);
    let neg: Vec<String> = pos.iter().map(|s| format!("-{s}")).collect();
    diff_and_expect("errors17_neg", neg, MINUS_ZERO);
}

/// ERRORS.md row 18 -- an exponent character with no digits is dropped, but the
/// mantissa still converts.
#[test]
fn exponent_without_digits() {
    // "1e", "1e+", "1e-" all convert to 1.0f
    let one = ["1e", "1e+", "1e-", "1E", "1E+", "1E-", "1e+x", "1e-z", "0x1p", "0x1p+", "0x1p-", "0x1P"];
    diff_and_expect("errors18_one", one.iter().map(|s| s.as_bytes()), "0000803f\n");
    let minus_one: Vec<String> = one.iter().map(|s| format!("-{s}")).collect();
    diff_and_expect("errors18_minus_one", minus_one, "000080bf\n");
    // and the same shape around zero keeps the sign
    let zero = ["0e", "0e+", "0e-", "0x0p", "0x0p+", "0x0p-", "0.e", "0.0e+"];
    diff_and_expect("errors18_zero", zero.iter().map(|s| s.as_bytes()), PLUS_ZERO);
    let minus_zero: Vec<String> = zero.iter().map(|s| format!("-{s}")).collect();
    diff_and_expect("errors18_minus_zero", minus_zero, MINUS_ZERO);
}

/// ERRORS.md row 19 -- trailing garbage stops the scan; the prefix converts.
#[test]
fn trailing_garbage_stops_scan() {
    let one_point_five = [
        "1.5abc", "1.5_", "1.5,", "1.5;", "1.5 9", "1.5\t9", "1.5\n9", "1.5(", "1.5)", "1.5.5",
        "1.5.", "1.5e", "1.5x", "1.5X", "1.5q", "1.5!", "1.5-2", "1.5+2", "1.5e5e5",
    ];
    // "1.5e5e5" collects "1.5e5" -> 150000, so keep it out of the 1.5 group.
    let plain: Vec<&str> = one_point_five
        .iter()
        .filter(|s| **s != "1.5e5e5" && **s != "1.5e")
        .copied()
        .collect();
    diff_and_expect("errors19_1p5", plain.iter().map(|s| s.as_bytes()), "0000c03f\n");
    // "1.5e" drops the empty exponent -> still 1.5
    diff_and_expect("errors19_1p5e", ["1.5e", "1.5e+", "1.5e-"].iter().map(|s| s.as_bytes()), "0000c03f\n");
    // a second exponent character terminates the token: "1.5e5" -> 150000.0f
    diff_and_expect(
        "errors19_second_exp",
        ["1.5e5e5", "1.5e5E5", "1.5e5.5", "1.5e5p5", "1.5e5x"].iter().map(|s| s.as_bytes()),
        "007c1248\n",
    );
    // "1_000" is 1.0, not 1000
    diff_and_expect(
        "errors19_underscore",
        ["1_000", "1_", "1,000"].iter().map(|s| s.as_bytes()),
        "0000803f\n",
    );
}

/// Generic boundary sweep: every single byte on its own, and every byte
/// followed by a valid number.  Catches any character class that the two
/// implementations disagree about.
#[test]
fn every_single_byte_boundary() {
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for b in 0u16..=255 {
        cases.push(vec![b as u8]);
        cases.push(vec![b as u8, b'1']);
        cases.push(vec![b'1', b as u8]);
        cases.push(vec![b'-', b as u8]);
        cases.push(vec![b'0', b'x', b as u8]);
        cases.push(vec![b'1', b'.', b as u8]);
        cases.push(vec![b'1', b'e', b as u8]);
        cases.push(vec![b'i', b'n', b'f', b as u8]);
        cases.push(vec![b'n', b'a', b'n', b as u8]);
    }
    diff_all("errors_every_byte", cases);
}

/// Generic boundary: values one step past each documented range end, in both
/// directions, for the decimal and the hexadecimal grammar alike.
#[test]
fn one_step_past_range_ends() {
    let rng = Rng::new(9_999);
    let mut cases: Vec<String> = vec![
        // smallest subnormal and one step below it
        "0x1p-149".into(),
        "0x1p-150".into(),
        "0x1.0000001p-150".into(),
        "0x0.ffffffp-149".into(),
        // largest subnormal / smallest normal
        "0x7fffffp-149".into(),
        "0x800000p-149".into(),
        "0x800001p-149".into(),
        // largest finite and one step past
        "0xffffffp104".into(),
        "0x1000000p104".into(),
        "0xffffff8p101".into(),
        "0xffffff7p101".into(),
        // 24-bit integer boundary
        "16777215".into(),
        "16777216".into(),
        "16777217".into(),
        "16777218".into(),
        "16777219".into(),
    ];
    for c in cases.clone() {
        cases.push(format!("-{c}"));
    }
    for _ in 0..200 {
        let e = rng.range_i32(-152, -145);
        let m = 1 + rng.below(0x100_0000);
        cases.push(format!("0x{m:x}p{e}"));
        cases.push(format!("-0x{m:x}p{e}"));
    }
    diff_all("errors_range_ends", cases);
}
