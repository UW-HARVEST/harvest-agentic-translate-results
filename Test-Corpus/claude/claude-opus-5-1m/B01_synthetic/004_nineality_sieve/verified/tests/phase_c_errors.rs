//! Phase C — error-path differential tests.
//!
//! One test per row of ERRORS.md. Every test runs the real C binary and the
//! real Rust binary through the process boundary and asserts they reject (or
//! accept!) identically — same stdout bytes, same exit status. Rows 20..31 pin
//! the inputs that *look* invalid but that the C code accepts, so the Rust
//! translation cannot "helpfully" add validation the C never had.

mod common;

use common::*;
use std::time::Duration;

// ===========================================================================
// site 1: argc != 2
// ===========================================================================

/// ERRORS.md #1 — argc == 0 via a raw execve with argv = {NULL}.
#[test]
fn err_01_argc_zero_raw_execve() {
    let (c_out, c_code) = run_argc_zero(&c_bin());
    let (r_out, r_code) = run_argc_zero(&rust_bin());
    assert_eq!(
        c_out, E_ARGC,
        "C did not print the argc error for argc==0: {c_out:?}"
    );
    assert_eq!(c_code, 1, "C exit code for argc==0");
    assert_eq!(r_out, c_out, "argc==0 stdout differs");
    assert_eq!(r_code, c_code, "argc==0 exit code differs");
}

/// ERRORS.md #2 — argc == 1 (no operand at all).
#[test]
fn err_02_argc_one_no_operand() {
    let spec = Spec::new(Vec::<Vec<u8>>::new());
    assert_c_result(&spec, E_ARGC, 1);
    assert_same(&spec);
}

/// ERRORS.md #3 — argc == 3.
#[test]
fn err_03_argc_three() {
    let spec = Spec::new([b"1".to_vec(), b"2".to_vec()]);
    assert_c_result(&spec, E_ARGC, 1);
    assert_same(&spec);
}

/// ERRORS.md #4 — the argc check runs BEFORE any parsing: a first operand that
/// would be perfectly valid still yields the argc error.
#[test]
fn err_04_argc_precedes_parse() {
    for extra in [b"".to_vec(), b"9".to_vec(), b"abc".to_vec()] {
        let spec = Spec::new([b"5".to_vec(), extra]);
        assert_c_result(&spec, E_ARGC, 1);
        assert_same(&spec);
    }
}

/// ERRORS.md #5 — many operands.
#[test]
fn err_05_argc_many() {
    let args: Vec<Vec<u8>> = (0..12).map(|i| format!("{i}").into_bytes()).collect();
    let spec = Spec::new(args);
    assert_c_result(&spec, E_ARGC, 1);
    assert_same(&spec);
}

/// ERRORS.md #6 — argc error wins even when every operand is unparsable.
#[test]
fn err_06_argc_wins_over_bad_parse() {
    let spec = Spec::new([b"abc".to_vec(), b"def".to_vec()]);
    assert_c_result(&spec, E_ARGC, 1);
    assert_same(&spec);
}

// ===========================================================================
// site 2: end == argv[1]  (strtol performed no conversion)
// ===========================================================================

fn expect_int_error(arg: &[u8]) {
    let spec = Spec::one(arg);
    assert_c_result(&spec, E_INT, 1);
    assert_same(&spec);
}

/// ERRORS.md #7 — zero-length operand.
#[test]
fn err_07_empty_operand() {
    expect_int_error(b"");
}

/// ERRORS.md #8 — whitespace only, each C-locale space character and all six.
#[test]
fn err_08_whitespace_only() {
    for ws in C_SPACES {
        expect_int_error(&[ws]);
        expect_int_error(&[ws, ws, ws]);
    }
    expect_int_error(&C_SPACES);
    expect_int_error(b" \t\n\x0b\x0c\r \t\n");
}

/// ERRORS.md #9 — sign with no digits.
#[test]
fn err_09_sign_only() {
    expect_int_error(b"-");
    expect_int_error(b"+");
    expect_int_error(b"--");
    expect_int_error(b"++");
}

/// ERRORS.md #10 — whitespace then a lone sign.
#[test]
fn err_10_ws_then_sign_only() {
    for ws in C_SPACES {
        expect_int_error(&[ws, b'-']);
        expect_int_error(&[ws, b'+']);
        expect_int_error(&[ws, ws, b'-']);
    }
}

/// ERRORS.md #11 — a space between the sign and the digits breaks the numeral.
#[test]
fn err_11_sign_space_digits() {
    expect_int_error(b"- 5");
    expect_int_error(b"+ 5");
    expect_int_error(b"-\t7");
    expect_int_error(b"+\n7");
    expect_int_error(b"   -   5");
}

/// ERRORS.md #12 — doubled or mixed signs.
#[test]
fn err_12_double_sign() {
    for s in [
        &b"--5"[..],
        b"++5",
        b"+-5",
        b"-+5",
        b"---9",
        b"  --0",
        b"+-+-1",
    ] {
        expect_int_error(s);
    }
}

/// ERRORS.md #13 — leading alphabetic text.
#[test]
fn err_13_leading_alpha() {
    for s in [
        &b"abc"[..],
        b"x9",
        b"e5",
        b"E5",
        b"inf",
        b"infinity",
        b"nan",
        b"NaN",
        b"NULL",
        b"null",
        b"O9",
        b"l1",
        b"true",
        b"0",
    ] {
        if s == b"0" {
            continue; // "0" is valid; kept out of the rejection set on purpose
        }
        expect_int_error(s);
    }
}

/// ERRORS.md #14 — leading punctuation, including the ASCII neighbours of the
/// digit range ('/' = 0x2f, ':' = 0x3a).
#[test]
fn err_14_leading_punct() {
    for s in [
        &b"."[..], b",", b"/", b":", b"_5", b"'5", b"#9", b".5", b",5", b"/5", b":5", b"$5", b"(5)",
        b"[5]", b"*", b"~9", b"=9", b"%9", b"\\9", b"\"5\"", b"`5", b"!5", b"?5", b";5", b"<5",
        b">5", b"|5", b"^5", b"&5", b"@5", b"{5}",
    ] {
        expect_int_error(s);
    }
}

/// ERRORS.md #15 — leading non-ASCII / invalid-UTF-8 bytes (raw argv bytes).
#[test]
fn err_15_leading_high_byte() {
    expect_int_error(b"\xff9");
    expect_int_error(b"\x80");
    expect_int_error(b"\xc3\x28 5");
    expect_int_error(b"\xfe\xff\x01");
    expect_int_error(b"\xa0 5"); // NBSP-as-latin1, not isspace in the C locale
}

/// ERRORS.md #16 — Unicode look-alikes for sign / digits / space.
#[test]
fn err_16_unicode_lookalikes() {
    expect_int_error("−5".as_bytes()); // U+2212 MINUS SIGN
    expect_int_error("５".as_bytes()); // U+FF15 FULLWIDTH FIVE
    expect_int_error("\u{a0}5".as_bytes()); // U+00A0 NO-BREAK SPACE
    expect_int_error("\u{2007}9".as_bytes()); // U+2007 FIGURE SPACE
    expect_int_error("٥".as_bytes()); // U+0665 ARABIC-INDIC FIVE
    expect_int_error("½".as_bytes());
    expect_int_error("😀9".as_bytes());
}

/// ERRORS.md #17 — exhaustive single-byte sweep: every byte that is neither an
/// ASCII digit, nor a sign, nor C-locale whitespace must be rejected, and the
/// ones that are digits must be accepted. 255 differential runs.
#[test]
fn err_17_exhaustive_single_byte_sweep() {
    for b in 1u16..=255 {
        let byte = b as u8;
        let spec = Spec::one([byte]).cap(1 << 16);
        let is_digit = byte.is_ascii_digit();
        let is_sign = byte == b'+' || byte == b'-';
        let is_space = C_SPACES.contains(&byte);
        if !is_digit && !is_sign && !is_space {
            assert_c_result(&spec, E_INT, 1);
        } else if is_sign || is_space {
            assert_c_result(&spec, E_INT, 1);
        }
        assert_same(&spec);
    }
}

/// ERRORS.md #18 — randomized non-numeric garbage (fixed seed, 512 cases).
#[test]
fn err_18_random_nonnumeric() {
    let mut rng = Rng::new(0xC0FFEE_1234_5678);
    let mut done = 0;
    let mut attempts = 0;
    while done < 512 {
        attempts += 1;
        assert!(attempts < 20_000, "generator failed to produce cases");
        let len = 1 + rng.below(12) as usize;
        let mut s: Vec<u8> = Vec::with_capacity(len);
        for _ in 0..len {
            // bias towards "numeral-ish" bytes so the generator explores the
            // boundary between accept and reject
            let c = match rng.below(6) {
                0 => *rng.pick(&C_SPACES),
                1 => *rng.pick(b"+-"),
                2 => b'0' + rng.below(10) as u8,
                3 => *rng.pick(b"abcxXeE.,_/:"),
                4 => rng.below(256) as u8,
                _ => *rng.pick(b"oOlL$#%\xff\x80"),
            };
            if c == 0 {
                continue; // NUL cannot appear in argv
            }
            s.push(c);
        }
        if s.is_empty() {
            continue;
        }
        // keep only the strings the C rejects: first byte after optional
        // whitespace+sign must not be a digit
        let mut i = 0;
        while i < s.len() && C_SPACES.contains(&s[i]) {
            i += 1;
        }
        if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
            i += 1;
        }
        let rejected = i >= s.len() || !s[i].is_ascii_digit();
        if !rejected {
            continue;
        }
        let spec = Spec::one(&s[..]);
        assert_c_result(&spec, E_INT, 1);
        assert_same(&spec);
        done += 1;
    }
}

/// ERRORS.md #19 — oversized rejected operand (100 000 bytes).
#[test]
fn err_19_oversized_nonnumeric() {
    let s = vec![b'x'; 100_000];
    let spec = Spec::one(&s[..]);
    assert_c_result(&spec, E_INT, 1);
    assert_same(&spec);
}

// ===========================================================================
// accepted-but-odd inputs (the mirror of the rejection table)
// ===========================================================================

/// ERRORS.md #20 — trailing garbage is accepted; only the leading numeral parses.
#[test]
fn err_20_trailing_garbage_accepted() {
    assert_c_result(&Spec::one(b"12abc"), b"12\n13\n14\n15\n16\n17\n18\n19\n", 0);
    assert_c_result(&Spec::one(b"0x1f"), b"0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n", 0);
    assert_c_result(&Spec::one(b"0x"), b"0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n", 0);
    assert_c_result(&Spec::one(b"1e3"), b"1\n2\n3\n4\n5\n6\n7\n8\n9\n", 0);
    for s in [
        &b"12abc"[..],
        b"0x1f",
        b"0x",
        b"0X10",
        b"1e3",
        b"1,000",
        b"9 9",
        b"7-3",
        b"7+3",
        b"5.9",
        b"08/09",
        b"3\n",
        b"3\t\t",
        b"6 ",
        b"9abc",
        b"019xyz",
    ] {
        assert_same(&Spec::one(s));
    }
}

/// ERRORS.md #21 — ERANGE on overflow is ignored; LONG_MAX truncates to -1.
#[test]
fn err_21_erange_positive_clamp() {
    let expect = b"-1\n0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n";
    for s in [
        &b"9223372036854775807"[..], // LONG_MAX
        b"9223372036854775808",      // LONG_MAX + 1 -> clamp
        b"99999999999999999999",
        b"18446744073709551615", // 2^64-1
        b"18446744073709551616", // 2^64
    ] {
        assert_c_result(&Spec::one(s), expect, 0);
        assert_same(&Spec::one(s));
    }
    let long_num = vec![b'7'; 400];
    assert_c_result(&Spec::one(&long_num[..]), expect, 0);
    assert_same(&Spec::one(&long_num[..]));
}

/// ERRORS.md #22 — negative overflow clamps to LONG_MIN, which truncates to 0.
#[test]
fn err_22_erange_negative_clamp() {
    let expect = b"0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n";
    for s in [
        &b"-9223372036854775808"[..], // LONG_MIN exactly
        b"-9223372036854775809",      // LONG_MIN - 1 -> clamp
        b"-99999999999999999999",
        b"-18446744073709551616",
    ] {
        assert_c_result(&Spec::one(s), expect, 0);
        assert_same(&Spec::one(s));
    }
    let mut long_num = vec![b'-'];
    long_num.extend(std::iter::repeat(b'8').take(400));
    assert_c_result(&Spec::one(&long_num[..]), expect, 0);
    assert_same(&Spec::one(&long_num[..]));
}

/// ERRORS.md #23 — `int val = strtol(...)` narrows modulo 2^32.
#[test]
fn err_23_int_truncation() {
    // 2^32 -> 0, 2^32+5 -> 5, 2^32+9 -> 9 (immediate break)
    assert_c_result(&Spec::one(b"4294967296"), b"0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n", 0);
    assert_c_result(&Spec::one(b"4294967301"), b"5\n6\n7\n8\n9\n", 0);
    assert_c_result(&Spec::one(b"4294967305"), b"9\n", 0);
    assert_c_result(&Spec::one(b"-4294967287"), b"9\n", 0);
    for s in [
        &b"4294967296"[..],
        b"4294967301",
        b"4294967305",
        b"-4294967287",
        b"8589934601",
        b"-4294967296",
    ] {
        assert_same(&Spec::one(s).cap(1 << 15));
    }
    // 2147483648 == INT_MIN after truncation: ~2^31 lines, bounded compare
    assert_same(&Spec::one(b"2147483648").cap(1 << 15));
}

/// ERRORS.md #24 — one step past every documented bound, in both directions.
#[test]
fn err_24_one_past_every_bound() {
    let bounds: [i128; 10] = [
        i32::MAX as i128,
        i32::MIN as i128,
        i64::MAX as i128,
        i64::MIN as i128,
        1i128 << 31,
        1i128 << 32,
        1i128 << 63,
        -(1i128 << 31),
        -(1i128 << 32),
        -(1i128 << 63),
    ];
    for b in bounds {
        for d in [-1i128, 0, 1] {
            let s = format!("{}", b + d);
            assert_same(&Spec::one(s.as_bytes()).cap(1 << 15));
        }
    }
}

/// ERRORS.md #25 — C's truncating `%` means a negative value never satisfies
/// `val % 10 == 9`, so the loop keeps counting up to +9.
#[test]
fn err_25_negative_mod_never_nine() {
    assert_c_result(
        &Spec::one(b"-19"),
        b"-19\n-18\n-17\n-16\n-15\n-14\n-13\n-12\n-11\n-10\n-9\n-8\n-7\n-6\n-5\n-4\n-3\n-2\n-1\n0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n",
        0,
    );
    assert_c_result(&Spec::one(b"-9"), b"-9\n-8\n-7\n-6\n-5\n-4\n-3\n-2\n-1\n0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n", 0);
    for s in [&b"-9"[..], b"-19", b"-29", b"-99", b"-109", b"-1009", b"-2000000009"] {
        assert_same(&Spec::one(s).cap(1 << 16));
    }
}

/// ERRORS.md #26 — signed overflow at INT_MAX: `val++` wraps to INT_MIN and the
/// loop continues (undefined in C, but this is what the binary does).
#[test]
fn err_26_int_max_overflow_wrap() {
    let spec = Spec::one(b"2147483647").cap(1 << 14);
    let c = run(&c_bin(), &spec);
    assert!(
        c.stdout.starts_with(b"2147483647\n-2147483648\n-2147483647\n"),
        "C did not wrap at INT_MAX: {:?}",
        String::from_utf8_lossy(&c.stdout[..60.min(c.stdout.len())])
    );
    assert_same(&spec);
}

/// ERRORS.md #27 — write failures are never checked: exit status stays 0.
#[test]
fn err_27_closed_stdout() {
    for arg in [&b"5"[..], b"9", b"-3", b"abc", b""] {
        let spec = Spec::one(arg).stdout(StdoutTarget::Closed);
        let c = run(&c_bin(), &spec);
        let r = run(&rust_bin(), &spec);
        assert_eq!(c.code, r.code, "closed-stdout exit code differs for {arg:?}");
        assert_eq!(c.signal, r.signal, "closed-stdout signal differs for {arg:?}");
        assert_eq!(c.stderr, r.stderr, "closed-stdout stderr differs");
    }
    // argc error path with fd 1 closed, too
    let spec = Spec::new([b"1".to_vec(), b"2".to_vec()]).stdout(StdoutTarget::Closed);
    assert_same(&spec);
}

/// ERRORS.md #27 — a reader that closes the pipe early must kill both binaries
/// the same way (SIGPIPE, i.e. Rust must not keep its default SIG_IGN).
#[test]
fn err_28_sigpipe_on_early_close() {
    let arg = b"-2000000000"; // ~2e9 lines: still writing when we close
    let (c_bytes, c_code, c_sig) = run_then_close_pipe(&c_bin(), arg, 4096);
    let (r_bytes, r_code, r_sig) = run_then_close_pipe(&rust_bin(), arg, 4096);
    assert_eq!(
        c_bytes.len().min(4096),
        r_bytes.len().min(4096),
        "prefix length differs"
    );
    assert_eq!(
        &c_bytes[..4096.min(c_bytes.len())],
        &r_bytes[..4096.min(r_bytes.len())],
        "prefix bytes differ"
    );
    assert_eq!(c_sig, Some(13), "C should die from SIGPIPE, got {c_sig:?}/{c_code:?}");
    assert_eq!(r_sig, c_sig, "SIGPIPE parity: C={c_sig:?} RUST={r_sig:?}");
    assert_eq!(r_code, c_code, "exit code parity after early close");
}

/// ERRORS.md #29 — oversized *numeral* (100 000 digits) clamps like any other
/// overflow instead of being rejected.
#[test]
fn err_29_oversized_numeral() {
    let mut rng = Rng::new(0x9999_1111);
    let mut s = rng.digits(100_000);
    s[0] = b'1'; // no leading zero: definitely > LONG_MAX
    let spec = Spec::one(&s[..]);
    assert_c_result(&spec, b"-1\n0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n", 0);
    assert_same(&spec);

    let mut neg = vec![b'-'];
    neg.extend_from_slice(&s);
    let spec = Spec::one(&neg[..]);
    assert_c_result(&spec, b"0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n", 0);
    assert_same(&spec);

    // 100 000 leading zeros followed by a small value -> parses fine
    let mut zeros = vec![b'0'; 100_000];
    zeros.extend_from_slice(b"7");
    assert_c_result(&Spec::one(&zeros[..]), b"7\n8\n9\n", 0);
    assert_same(&Spec::one(&zeros[..]));
}

/// ERRORS.md #30 — operands that are not valid UTF-8 but do parse.
#[test]
fn err_30_non_utf8_but_parses() {
    assert_c_result(&Spec::one(b"5\xff"), b"5\n6\n7\n8\n9\n", 0);
    assert_c_result(&Spec::one(b"\t-7\x80"), b"-7\n-6\n-5\n-4\n-3\n-2\n-1\n0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n", 0);
    for s in [
        &b"5\xff"[..],
        b"\t-7\x80",
        b"9\xc3\x28",
        b"  +3\xfe\xff",
        b"0\xff\xff",
    ] {
        assert_same(&Spec::one(s));
    }
}

/// ERRORS.md #31 — the rejection is locale-invariant.
#[test]
fn err_31_locale_invariance() {
    let locales = [
        "C",
        "POSIX",
        "en_US.UTF-8",
        "tr_TR.UTF-8",
        "de_DE.UTF-8",
        "C.UTF-8",
        "ja_JP.UTF-8",
    ];
    for loc in locales {
        for arg in [&b"abc"[..], b"", b"-", b"\xff5", b" "] {
            let spec = Spec::one(arg).env("LC_ALL", loc).env("LANG", loc);
            assert_c_result(&spec, E_INT, 1);
            assert_same(&spec);
        }
        // and on an accepted input, where printf/strtol could in principle
        // grow separators
        for arg in [&b"1234567"[..], b"-1000", b"999999999"] {
            assert_same(&Spec::one(arg).env("LC_ALL", loc).env("LANG", loc).cap(1 << 16));
        }
    }
    // LC_NUMERIC alone
    for arg in [&b"abc"[..], b"1234567"] {
        assert_same(
            &Spec::one(arg)
                .env("LC_NUMERIC", "de_DE.UTF-8")
                .cap(1 << 16),
        );
    }
}

/// Belt-and-braces: the two error messages must be byte-exact, including the
/// trailing newline and the absence of anything on stderr.
#[test]
fn err_32_message_bytes_are_exact() {
    let no_args = Spec::new(Vec::<Vec<u8>>::new());
    let c = run(&c_bin(), &no_args);
    assert_eq!(c.stdout, E_ARGC);
    assert!(c.stderr.is_empty(), "C wrote to stderr: {:?}", c.stderr);
    let r = run(&rust_bin(), &no_args);
    assert_eq!(r.stdout, E_ARGC);
    assert!(r.stderr.is_empty());

    let bad = Spec::one(b"nope").timeout(Duration::from_secs(10));
    let c = run(&c_bin(), &bad);
    assert_eq!(c.stdout, E_INT);
    assert!(c.stderr.is_empty());
    let r = run(&rust_bin(), &bad);
    assert_eq!(r.stdout, E_INT);
    assert!(r.stderr.is_empty());
}
