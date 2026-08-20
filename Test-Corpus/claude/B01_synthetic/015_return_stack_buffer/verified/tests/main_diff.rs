//! Phase B/C differential tests for the `main` export, i.e. the whole program:
//! `scanf("%d", &x)` followed by `if (x) good(); else bad();`.
//!
//! Every invocation `dlopen()`s one of the two shared objects in a **fresh
//! process** (`src/bin/so_main_runner.rs`) and calls its exported `main`.  A
//! fresh process is required because glibc's `FILE *stdin` and Rust's
//! `std::io::stdin()` both read ahead into a process-wide buffer, so reusing a
//! process would give the second call a different byte stream than the first.
//!
//! The Rust side is reached exclusively through the `#[no_mangle] extern "C"`
//! `main` export of `libdriver.so`; it is never called directly.

mod common;

use common::{assert_same, c_exe, c_so, exe_run, rust_exe, rust_so, show, so_main, Rng, SEED};

/// The six characters `isspace()` recognizes in the "C" locale.
const SPACES: [u8; 6] = [b' ', b'\t', b'\n', 0x0b, 0x0c, b'\r'];

#[track_caller]
fn diff_main(label: &str, input: &[u8]) {
    let c = so_main(&c_so(), input);
    let r = so_main(&rust_so(), input);
    assert_same(label, input, &c, &r);
    assert_eq!(
        c.code,
        Some(0),
        "the C main() has a single `return 0;`, so the status must be 0 for \"{}\"",
        show(input)
    );
}

#[track_caller]
fn diff_main_all(label: &str, inputs: &[Vec<u8>]) {
    for (i, input) in inputs.iter().enumerate() {
        diff_main(&format!("{label} #{i}"), input);
    }
}

/// Assert both objects took the `bad()` branch (no output at all).
#[track_caller]
fn diff_main_expect_empty(label: &str, input: &[u8]) {
    let c = so_main(&c_so(), input);
    let r = so_main(&rust_so(), input);
    assert_same(label, input, &c, &r);
    assert!(
        c.stdout.is_empty(),
        "expected the C reference to print nothing for {label} (\"{}\"), got \"{}\"",
        show(input),
        show(&c.stdout)
    );
    assert_eq!(c.code, Some(0));
}

/// Assert both objects took the `good()` branch.
#[track_caller]
fn diff_main_expect_good(label: &str, input: &[u8]) {
    let c = so_main(&c_so(), input);
    let r = so_main(&rust_so(), input);
    assert_same(label, input, &c, &r);
    assert_eq!(
        c.stdout,
        b"helperGood1 string\n",
        "expected the C reference to take the good() branch for {label} (\"{}\")",
        show(input)
    );
    assert_eq!(c.code, Some(0));
}

// ===========================================================================
// Corpora — each also feeds CONFIGS.md row 26 (executable end-to-end)
// ===========================================================================

fn corpus_single_digit() -> Vec<Vec<u8>> {
    let mut v = Vec::new();
    for d in b'0'..=b'9' {
        v.push(vec![d]);
        v.push(vec![d, b'\n']);
        v.push(vec![d, b' ']);
    }
    v
}

fn corpus_whitespace_prefix(rng: &mut Rng) -> Vec<Vec<u8>> {
    let mut v = Vec::new();
    // Every single space character on its own, before 0 and before 1.
    for s in SPACES {
        v.push(vec![s, b'0']);
        v.push(vec![s, b'1']);
    }
    for _ in 0..48 {
        let n = rng.range(1, 8) as usize;
        let mut input: Vec<u8> = (0..n).map(|_| *rng.pick(&SPACES)).collect();
        let value = rng.next_u64() as i32;
        input.extend_from_slice(value.to_string().as_bytes());
        if rng.bool() {
            input.push(b'\n');
        }
        v.push(input);
    }
    v
}

fn corpus_plus_sign(rng: &mut Rng) -> Vec<Vec<u8>> {
    let mut v = vec![b"+0".to_vec(), b"+1".to_vec(), b"+2147483647".to_vec()];
    for _ in 0..48 {
        let m = rng.below(i32::MAX as u64 + 1);
        v.push(format!("+{m}").into_bytes());
    }
    v
}

fn corpus_minus_sign(rng: &mut Rng) -> Vec<Vec<u8>> {
    let mut v = vec![
        b"-0".to_vec(),
        b"-1".to_vec(),
        b"-2147483647".to_vec(),
        b"-2147483648".to_vec(),
    ];
    for _ in 0..48 {
        let m = rng.below(2_147_483_649u64);
        v.push(format!("-{m}").into_bytes());
    }
    v
}

fn corpus_leading_zeros(rng: &mut Rng) -> Vec<Vec<u8>> {
    let mut v = Vec::new();
    for n in 1..=12usize {
        v.push(vec![b'0'; n]);
        let mut s = vec![b'0'; n];
        s.push(b'7');
        v.push(s);
        let mut s = b"-".to_vec();
        s.extend(vec![b'0'; n]);
        s.push(b'7');
        v.push(s);
    }
    for _ in 0..48 {
        let zeros = rng.range(1, 12) as usize;
        let value = rng.next_u64() as u32;
        let sign: &[u8] = if rng.bool() {
            b""
        } else if rng.bool() {
            b"+"
        } else {
            b"-"
        };
        let mut input = sign.to_vec();
        input.extend(vec![b'0'; zeros]);
        input.extend_from_slice(value.to_string().as_bytes());
        v.push(input);
    }
    v
}

fn corpus_zero_x_prefix(rng: &mut Rng) -> Vec<Vec<u8>> {
    let mut v = vec![
        b"0x10".to_vec(),
        b"0X10".to_vec(),
        b"0x".to_vec(),
        b"0X".to_vec(),
        b"0xy".to_vec(),
        b"-0x10".to_vec(),
        b"+0X1F".to_vec(),
        b"00x10".to_vec(),
        b"0x0".to_vec(),
        b"0x00000001".to_vec(),
    ];
    let hex = b"0123456789abcdefABCDEF";
    for _ in 0..48 {
        let n = rng.range(0, 8) as usize;
        let tail = rng.bytes(n, hex);
        let mut input: Vec<u8> = Vec::new();
        if rng.below(3) == 0 {
            input.push(if rng.bool() { b'+' } else { b'-' });
        }
        input.push(b'0');
        input.push(if rng.bool() { b'x' } else { b'X' });
        input.extend_from_slice(&tail);
        v.push(input);
    }
    v
}

fn corpus_random_i32(rng: &mut Rng) -> Vec<Vec<u8>> {
    let mut v = Vec::new();
    for &n in &[
        0i64,
        1,
        -1,
        2,
        -2,
        i32::MAX as i64,
        i32::MIN as i64,
        (i32::MAX as i64) - 1,
        (i32::MIN as i64) + 1,
    ] {
        v.push(n.to_string().into_bytes());
    }
    for _ in 0..64 {
        let n = rng.next_u64() as i32;
        v.push(n.to_string().into_bytes());
    }
    // Values chosen to land on both sides of the `if (x)` test.
    for _ in 0..16 {
        v.push(format!("{}", rng.range(1, 1000)).into_bytes());
        v.push(format!("-{}", rng.range(1, 1000)).into_bytes());
    }
    v
}

fn corpus_i64_narrowing(rng: &mut Rng) -> Vec<Vec<u8>> {
    let mut v = Vec::new();
    // Deterministic narrowing landmines: low 32 bits zero -> `bad()`.
    for s in [
        "4294967296",
        "-4294967296",
        "8589934592",
        "-8589934592",
        "4294967295",
        "4294967297",
        "-4294967295",
        "-4294967297",
        "9223372036854775807",
        "-9223372036854775808",
        "9223372036854775806",
        "-9223372036854775807",
        "1099511627776",
        "281474976710656",
    ] {
        v.push(s.as_bytes().to_vec());
    }
    for _ in 0..48 {
        // Outside int range but inside long range.
        let mag = rng.range(i32::MAX as u64 + 1, i64::MAX as u64);
        if rng.bool() {
            v.push(format!("{mag}").into_bytes());
        } else {
            v.push(format!("-{mag}").into_bytes());
        }
    }
    for _ in 0..24 {
        // Force the low 32 bits to zero so `(int) num.l == 0`.
        let hi = rng.range(1, 0x7FFF_FFFF);
        let mag = hi << 32;
        if rng.bool() {
            v.push(format!("{mag}").into_bytes());
        } else {
            v.push(format!("-{mag}").into_bytes());
        }
    }
    for _ in 0..24 {
        // Non-zero low 32 bits, guaranteed `good()`.
        let hi = rng.range(1, 0x7FFF_FFFF);
        let lo = rng.range(1, 0xFFFF_FFFF);
        let mag = (hi << 32) | lo;
        if rng.bool() {
            v.push(format!("{mag}").into_bytes());
        } else {
            v.push(format!("-{mag}").into_bytes());
        }
    }
    v
}

fn corpus_erange(rng: &mut Rng) -> Vec<Vec<u8>> {
    let mut v = Vec::new();
    for s in [
        "9223372036854775808",
        "-9223372036854775809",
        "18446744073709551615",
        "18446744073709551616",
        "-18446744073709551616",
        "99999999999999999999",
        "-99999999999999999999",
        "340282366920938463463374607431768211456",
    ] {
        v.push(s.as_bytes().to_vec());
    }
    let digits = b"0123456789";
    for _ in 0..48 {
        let n = rng.range(20, 40) as usize;
        let mut input: Vec<u8> = Vec::new();
        if rng.bool() {
            input.push(if rng.bool() { b'+' } else { b'-' });
        }
        // Leading digit 1..9 so the magnitude really exceeds LONG_MAX.
        input.push(rng.range(b'1' as u64, b'9' as u64) as u8);
        input.extend_from_slice(&rng.bytes(n - 1, digits));
        v.push(input);
    }
    v
}

fn corpus_digit_run_lengths(rng: &mut Rng) -> Vec<Vec<u8>> {
    let mut v = Vec::new();
    let digits = b"0123456789";
    for len in [
        1usize, 2, 8, 9, 10, 11, 17, 18, 19, 20, 21, 39, 40, 100, 1000, 4094, 4095, 4096, 4097,
        4098, 4200, 8200,
    ] {
        // All-zero run: converts to 0 -> `bad()`.
        v.push(vec![b'0'; len]);
        // Random run.
        let mut s = vec![rng.range(b'1' as u64, b'9' as u64) as u8];
        if len > 1 {
            s.extend_from_slice(&rng.bytes(len - 1, digits));
        }
        v.push(s);
        // Random run behind a sign and a leading zero, then a trailing token.
        let mut s = b"-0".to_vec();
        s.extend_from_slice(&rng.bytes(len, digits));
        s.extend_from_slice(b" trailing");
        v.push(s);
    }
    v
}

fn corpus_trailing_garbage(rng: &mut Rng) -> Vec<Vec<u8>> {
    let mut v = vec![
        b"5abc".to_vec(),
        b"0nonsense".to_vec(),
        b"12.".to_vec(),
        b"12.5".to_vec(),
        b"0,0".to_vec(),
        b"7e9".to_vec(),
        b"0e0".to_vec(),
        b"1_000".to_vec(),
        b"3:4".to_vec(),
        b"-0junk".to_vec(),
    ];
    let alphabet: Vec<u8> = (0x21u8..=0x7e).filter(|b| !b.is_ascii_digit()).collect();
    for _ in 0..48 {
        let value = rng.next_u64() as i32;
        let mut input = value.to_string().into_bytes();
        let n = rng.range(1, 12) as usize;
        input.extend_from_slice(&rng.bytes(n, &alphabet));
        v.push(input);
    }
    v
}

fn corpus_multiple_tokens(rng: &mut Rng) -> Vec<Vec<u8>> {
    let mut v = vec![
        b"0 1".to_vec(),
        b"1 0".to_vec(),
        b"0\n1\n".to_vec(),
        b"1\n0\n".to_vec(),
        b"  0   7  ".to_vec(),
    ];
    for _ in 0..48 {
        let count = rng.range(2, 5) as usize;
        let mut input: Vec<u8> = Vec::new();
        for i in 0..count {
            if i > 0 {
                let n = rng.range(1, 3) as usize;
                for _ in 0..n {
                    input.push(*rng.pick(&SPACES));
                }
            }
            let value = rng.next_u64() as i32;
            input.extend_from_slice(value.to_string().as_bytes());
        }
        v.push(input);
    }
    v
}

fn corpus_trailing_whitespace(rng: &mut Rng) -> Vec<Vec<u8>> {
    let mut v = Vec::new();
    for s in SPACES {
        v.push(vec![b'0', s]);
        v.push(vec![b'1', s]);
        v.push(vec![b'4', s, s, s]);
    }
    for _ in 0..32 {
        let value = rng.next_u64() as i32;
        let mut input = value.to_string().into_bytes();
        let n = rng.range(0, 5) as usize;
        for _ in 0..n {
            input.push(*rng.pick(&SPACES));
        }
        v.push(input);
    }
    v
}

fn corpus_byte_soup(rng: &mut Rng) -> Vec<Vec<u8>> {
    let mut v = Vec::new();
    // Whole 0x00..0xFF alphabet, including NUL and the high half.
    let all: Vec<u8> = (0x00u8..=0xff).collect();
    for _ in 0..96 {
        let len = rng.range(0, 24) as usize;
        v.push(rng.bytes(len, &all));
    }
    // Biased soup: mostly digits/signs/spaces so the parser gets deep more
    // often, with the occasional wild byte.
    let biased: Vec<u8> = b"0123456789+-  \t\n\r\x0b\x0cxX.,\x00\x80\xff".to_vec();
    for _ in 0..96 {
        let len = rng.range(0, 16) as usize;
        v.push(rng.bytes(len, &biased));
    }
    v
}

fn corpus_structured_random(rng: &mut Rng) -> Vec<Vec<u8>> {
    let digits = b"0123456789";
    let junk: Vec<u8> = (0x21u8..=0x7e).filter(|b| !b.is_ascii_digit()).collect();
    let mut v = Vec::new();
    for _ in 0..160 {
        let mut input: Vec<u8> = Vec::new();
        // Optional leading whitespace run.
        let ws = rng.below(4) as usize;
        for _ in 0..ws {
            input.push(*rng.pick(&SPACES));
        }
        // Optional sign (sometimes doubled, which is a matching failure).
        match rng.below(5) {
            0 => input.push(b'+'),
            1 => input.push(b'-'),
            2 => {
                input.push(b'-');
                input.push(b'-');
            }
            _ => {}
        }
        // Optional leading zeros.
        let zeros = rng.below(4) as usize;
        input.extend(vec![b'0'; zeros]);
        // Optional inert `x`.
        if rng.below(6) == 0 {
            input.push(if rng.bool() { b'x' } else { b'X' });
        }
        // Digit run of a random length (0 means a matching failure).
        let n = rng.below(24) as usize;
        input.extend_from_slice(&rng.bytes(n, digits));
        // Optional tail.
        match rng.below(4) {
            0 => {
                let k = rng.range(1, 6) as usize;
                input.extend_from_slice(&rng.bytes(k, &junk));
            }
            1 => input.push(*rng.pick(&SPACES)),
            2 => {
                input.push(*rng.pick(&SPACES));
                let k = rng.range(1, 4) as usize;
                input.extend_from_slice(&rng.bytes(k, digits));
            }
            _ => {}
        }
        v.push(input);
    }
    v
}

// ===========================================================================
// CONFIGS.md rows 11..25
// ===========================================================================

#[test]
fn cfg_11_main_single_digit() {
    diff_main_all("single digit", &corpus_single_digit());
    // Explicit branch checks, so a mutual "prints nothing" bug cannot hide.
    diff_main_expect_empty("digit 0", b"0");
    diff_main_expect_good("digit 1", b"1");
}

#[test]
fn cfg_12_main_whitespace_prefix() {
    let mut rng = Rng::new(SEED ^ 0x12);
    diff_main_all("whitespace prefix", &corpus_whitespace_prefix(&mut rng));
}

#[test]
fn cfg_13_main_plus_sign() {
    let mut rng = Rng::new(SEED ^ 0x13);
    diff_main_all("plus sign", &corpus_plus_sign(&mut rng));
}

#[test]
fn cfg_14_main_minus_sign() {
    let mut rng = Rng::new(SEED ^ 0x14);
    diff_main_all("minus sign", &corpus_minus_sign(&mut rng));
}

#[test]
fn cfg_15_main_leading_zeros() {
    let mut rng = Rng::new(SEED ^ 0x15);
    diff_main_all("leading zeros", &corpus_leading_zeros(&mut rng));
}

#[test]
fn cfg_16_main_zero_x_prefix() {
    let mut rng = Rng::new(SEED ^ 0x16);
    diff_main_all("0x prefix", &corpus_zero_x_prefix(&mut rng));
    // `%d` pins the base at 10, so "0x10" is the number 0 and nothing else.
    diff_main_expect_empty("0x10 is zero", b"0x10");
}

#[test]
fn cfg_17_main_random_i32() {
    let mut rng = Rng::new(SEED ^ 0x17);
    diff_main_all("random i32", &corpus_random_i32(&mut rng));
}

#[test]
fn cfg_18_main_i64_narrowing() {
    let mut rng = Rng::new(SEED ^ 0x18);
    diff_main_all("i64 narrowing", &corpus_i64_narrowing(&mut rng));
    diff_main_expect_empty("2^32 truncates to 0", b"4294967296");
    diff_main_expect_good("2^31 truncates to INT_MIN", b"2147483648");
}

#[test]
fn cfg_19_main_erange_random() {
    let mut rng = Rng::new(SEED ^ 0x19);
    diff_main_all("erange", &corpus_erange(&mut rng));
}

#[test]
fn cfg_20_main_digit_run_lengths() {
    let mut rng = Rng::new(SEED ^ 0x20);
    diff_main_all("digit run lengths", &corpus_digit_run_lengths(&mut rng));
}

#[test]
fn cfg_21_main_trailing_garbage() {
    let mut rng = Rng::new(SEED ^ 0x21);
    diff_main_all("trailing garbage", &corpus_trailing_garbage(&mut rng));
}

#[test]
fn cfg_22_main_multiple_tokens() {
    let mut rng = Rng::new(SEED ^ 0x22);
    diff_main_all("multiple tokens", &corpus_multiple_tokens(&mut rng));
}

#[test]
fn cfg_23_main_trailing_whitespace() {
    let mut rng = Rng::new(SEED ^ 0x23);
    diff_main_all("trailing whitespace", &corpus_trailing_whitespace(&mut rng));
}

#[test]
fn cfg_24_main_random_byte_soup() {
    let mut rng = Rng::new(SEED ^ 0x24);
    diff_main_all("byte soup", &corpus_byte_soup(&mut rng));
}

#[test]
fn cfg_25_main_structured_random() {
    let mut rng = Rng::new(SEED ^ 0x25);
    diff_main_all("structured random", &corpus_structured_random(&mut rng));
}

// ===========================================================================
// CONFIGS.md row 26 — the stand-alone executables, end to end
// ===========================================================================

#[test]
fn cfg_26_executables_end_to_end() {
    let mut rng = Rng::new(SEED ^ 0x26);
    let mut corpus: Vec<Vec<u8>> = Vec::new();
    corpus.extend(corpus_single_digit());
    corpus.extend(corpus_whitespace_prefix(&mut rng));
    corpus.extend(corpus_leading_zeros(&mut rng));
    corpus.extend(corpus_zero_x_prefix(&mut rng));
    corpus.extend(corpus_i64_narrowing(&mut rng));
    corpus.extend(corpus_erange(&mut rng));
    corpus.extend(corpus_trailing_garbage(&mut rng));
    corpus.extend(corpus_multiple_tokens(&mut rng));
    corpus.extend(corpus_byte_soup(&mut rng));
    corpus.extend(corpus_structured_random(&mut rng));
    corpus.push(Vec::new());

    let c = c_exe();
    let r = rust_exe();
    for (i, input) in corpus.iter().enumerate() {
        let cr = exe_run(&c, input);
        let rr = exe_run(&r, input);
        assert_same(&format!("executable end-to-end #{i}"), input, &cr, &rr);
    }
}

// ===========================================================================
// ERRORS.md rows 3..10 and generic rows G7, G8
// ===========================================================================

#[test]
fn err_03_scanf_eof_empty() {
    diff_main_expect_empty("empty stdin", b"");
}

#[test]
fn err_04_scanf_eof_whitespace_only() {
    for s in SPACES {
        diff_main_expect_empty(&format!("only {s:#04x}"), &[s]);
        diff_main_expect_empty(&format!("only {s:#04x} x4"), &[s, s, s, s]);
    }
    diff_main_expect_empty("mixed whitespace", b" \t\n\x0b\x0c\r");
    diff_main_expect_empty("many spaces", &vec![b' '; 5000]);
}

#[test]
fn err_05_scanf_matching_failure_non_digit() {
    let cases: Vec<Vec<u8>> = vec![
        b"abc".to_vec(),
        b"x".to_vec(),
        b"X".to_vec(),
        b".".to_vec(),
        b"/".to_vec(),
        b":".to_vec(),
        b",".to_vec(),
        b"e".to_vec(),
        b"nil".to_vec(),
        b"(nil)".to_vec(),
        b"NULL".to_vec(),
        b"  \t hello".to_vec(),
        vec![0x00],
        vec![0x00, b'5'],
        vec![0x80],
        vec![0xff],
        vec![0x80, b'5'],
        vec![0xff, b'1'],
        b"\x7f".to_vec(),
        b"#5".to_vec(),
        b"*5".to_vec(),
        b"$5".to_vec(),
    ];
    for (i, c) in cases.iter().enumerate() {
        diff_main_expect_empty(&format!("non-digit #{i}"), c);
    }
    // Randomized: a leading non-digit, non-sign, non-space byte.
    let mut rng = Rng::new(SEED ^ 0x05);
    let lead: Vec<u8> = (0x00u8..=0xff)
        .filter(|b| !b.is_ascii_digit() && *b != b'+' && *b != b'-' && !SPACES.contains(b))
        .collect();
    for _ in 0..64 {
        let mut input = vec![*rng.pick(&lead)];
        let k = rng.below(6) as usize;
        input.extend_from_slice(&rng.bytes(k, b"0123456789"));
        diff_main_expect_empty("random non-digit lead", &input);
    }
}

#[test]
fn err_06_scanf_matching_failure_lone_sign() {
    let cases: Vec<Vec<u8>> = vec![
        b"+".to_vec(),
        b"-".to_vec(),
        b"+ ".to_vec(),
        b"- ".to_vec(),
        b"+\n".to_vec(),
        b"-\n".to_vec(),
        b"-x".to_vec(),
        b"+x".to_vec(),
        b"--5".to_vec(),
        b"++5".to_vec(),
        b"+-3".to_vec(),
        b"-+3".to_vec(),
        b"  -  5".to_vec(),
        b"-.5".to_vec(),
        b"+.5".to_vec(),
    ];
    for (i, c) in cases.iter().enumerate() {
        diff_main_expect_empty(&format!("lone sign #{i}"), c);
    }
}

#[test]
fn err_07_scanf_int_truncates_to_zero() {
    // Conversion succeeds but `(int) num.l` is 0, so `if (x)` fails.
    for s in [
        "4294967296",
        "-4294967296",
        "8589934592",
        "-8589934592",
        "-9223372036854775808",
        "1099511627776",
        "-1099511627776",
        "281474976710656",
        // 2^64 exceeds LONG_MAX, so strtol saturates to LONG_MAX rather than
        // truncating; its negation is what lands on 0 (see err_08).
        "-18446744073709551616",
    ] {
        diff_main_expect_empty(&format!("truncates to zero: {s}"), s.as_bytes());
    }
    // Randomized: magnitudes that are exact multiples of 2^32.
    let mut rng = Rng::new(SEED ^ 0x07);
    for _ in 0..48 {
        let hi = rng.range(1, 0x7FFF_FFFF);
        let mag = hi << 32;
        diff_main_expect_empty("multiple of 2^32", format!("{mag}").as_bytes());
        diff_main_expect_empty("negative multiple of 2^32", format!("-{mag}").as_bytes());
    }
}

#[test]
fn err_08_scanf_erange_saturation() {
    // Positive saturation -> LONG_MAX -> (int) -1 -> good().
    for s in [
        "99999999999999999999",
        "9223372036854775808",
        "340282366920938463463374607431768211455",
        "+99999999999999999999",
        "18446744073709551615",
        "18446744073709551616",
    ] {
        diff_main_expect_good(&format!("positive ERANGE: {s}"), s.as_bytes());
    }
    // Negative saturation -> LONG_MIN -> (int) 0 -> bad().
    for s in [
        "-99999999999999999999",
        "-9223372036854775809",
        "-340282366920938463463374607431768211455",
    ] {
        diff_main_expect_empty(&format!("negative ERANGE: {s}"), s.as_bytes());
    }
    // Randomized saturation in both directions.
    let mut rng = Rng::new(SEED ^ 0x08);
    for _ in 0..48 {
        let n = rng.range(20, 60) as usize;
        let mut digits = vec![rng.range(b'1' as u64, b'9' as u64) as u8];
        digits.extend_from_slice(&rng.bytes(n - 1, b"0123456789"));
        let mut pos = digits.clone();
        if rng.bool() {
            pos.insert(0, b'+');
        }
        diff_main_expect_good("random positive ERANGE", &pos);
        let mut neg = vec![b'-'];
        neg.extend_from_slice(&digits);
        diff_main_expect_empty("random negative ERANGE", &neg);
    }
}

#[test]
fn err_09_scanf_hex_prefix_not_honored() {
    for s in ["0x10", "0X1F", "0xy", "0x", "0X", "-0x10", "+0X1F", "0xff"] {
        diff_main_expect_empty(&format!("hex prefix: {s}"), s.as_bytes());
    }
    // "00x10" also stops at the 'x': the digit loop breaks on a non-digit.
    diff_main_expect_empty("00x10", b"00x10");
    // A non-zero prefix followed by 'x' keeps the digits already read.
    diff_main_expect_good("1x10", b"1x10");
}

#[test]
fn err_10_scanf_partial_then_garbage() {
    diff_main_expect_good("5abc", b"5abc");
    diff_main_expect_empty("0nonsense", b"0nonsense");
    diff_main_expect_good("12.", b"12.");
    diff_main_expect_good("12.5", b"12.5");
    diff_main_expect_empty("0,0", b"0,0");
    diff_main_expect_good("7e9", b"7e9");
    diff_main_expect_empty("0e0", b"0e0");
    diff_main_expect_good("1_000", b"1_000");
    diff_main_expect_empty("-0junk", b"-0junk");
}

#[test]
fn err_g7_scanf_int_boundaries() {
    // One step either side of the `int` range, in both signs.
    for s in [
        "2147483646",
        "2147483647",
        "2147483648",
        "2147483649",
        "-2147483647",
        "-2147483648",
        "-2147483649",
        "-2147483650",
        "4294967294",
        "4294967295",
        "4294967296",
        "4294967297",
        "-4294967295",
        "-4294967296",
        "-4294967297",
    ] {
        diff_main(&format!("int boundary {s}"), s.as_bytes());
    }
    diff_main_expect_good("INT_MAX", b"2147483647");
    diff_main_expect_good("INT_MAX+1 wraps to INT_MIN", b"2147483648");
    diff_main_expect_good("INT_MIN", b"-2147483648");
    diff_main_expect_empty("UINT_MAX+1 truncates to 0", b"4294967296");
}

#[test]
fn err_g8_scanf_long_boundaries() {
    for s in [
        "9223372036854775806",
        "9223372036854775807",
        "9223372036854775808",
        "9223372036854775809",
        "-9223372036854775807",
        "-9223372036854775808",
        "-9223372036854775809",
        "-9223372036854775810",
        "18446744073709551614",
        "18446744073709551615",
        "18446744073709551616",
        "-18446744073709551615",
        "-18446744073709551616",
    ] {
        diff_main(&format!("long boundary {s}"), s.as_bytes());
    }
    diff_main_expect_good("LONG_MAX -> (int) -1", b"9223372036854775807");
    diff_main_expect_empty("LONG_MIN -> (int) 0", b"-9223372036854775808");
    diff_main_expect_good("LONG_MAX+1 saturates to LONG_MAX", b"9223372036854775808");
    diff_main_expect_empty("LONG_MIN-1 saturates to LONG_MIN", b"-9223372036854775809");
}
