//! Phase B — valid-path differential tests for the exported `int main()`
//! symbol, i.e. the whole `scanf("%d") -> driver -> print_hex` pipeline driven
//! through the `.so` export.  Rows 10-26 and 29 of CONFIGS.md.

mod common;

use common::*;

const WS: [u8; 6] = [b' ', b'\t', b'\n', 0x0b, 0x0c, b'\r'];
const TERMINATORS: [&str; 12] = [
    "", "\n", " ", "\t", "\r\n", "a", "Z", ".", "-", "+", "x", "\0",
];

/// CONFIGS row 10 — empty (zero-byte) seekable stdin.
#[test]
fn cfg_10_main_empty() {
    assert_main_eq(b"", Stdin::File);
    // sanity anchor: the C program prints the untouched `x == 0`
    let c = call_main(c_impl(), b"", Stdin::File);
    assert_eq!(c.stdout, b"00000000\n");
    assert_eq!(c.status, 0);
}

/// CONFIGS row 11 — bare digit runs, no sign, no trailing newline.
#[test]
fn cfg_11_main_plain_digits() {
    let mut rng = Rng::new();
    for _ in 0..200 {
        let v = rng.range_i64(0, i32::MAX as i64);
        assert_main_eq(format!("{v}").as_bytes(), Stdin::File);
    }
    for v in [0i64, 1, 9, 10, 99, 100, 12345, 2147483646, 2147483647] {
        assert_main_eq(format!("{v}").as_bytes(), Stdin::File);
    }
}

/// CONFIGS row 12 — every terminator class right after the digits.
#[test]
fn cfg_12_main_terminators() {
    let mut rng = Rng::new();
    for t in TERMINATORS {
        for _ in 0..12 {
            let v = rng.range_i64(i32::MIN as i64, i32::MAX as i64);
            let mut input = format!("{v}").into_bytes();
            input.extend_from_slice(t.as_bytes());
            assert_main_eq(&input, Stdin::File);
        }
    }
    // digits followed by more digits after whitespace, and by a second sign
    assert_main_eq(b"12 34", Stdin::File);
    assert_main_eq(b"12-34", Stdin::File);
    assert_main_eq(b"12+34", Stdin::File);
    assert_main_eq(b"0x10", Stdin::File);
    assert_main_eq(b"1e5", Stdin::File);
    assert_main_eq(b"3.14", Stdin::File);
}

/// CONFIGS row 13 — explicit `-` sign.
#[test]
fn cfg_13_main_negative() {
    let mut rng = Rng::new();
    for _ in 0..200 {
        let m = rng.range_i64(0, 2147483648);
        assert_main_eq(format!("-{m}").as_bytes(), Stdin::File);
    }
    for m in [0i64, 1, 2147483647, 2147483648, 2147483649] {
        assert_main_eq(format!("-{m}").as_bytes(), Stdin::File);
        assert_main_eq(format!("-{m}\n").as_bytes(), Stdin::File);
    }
}

/// CONFIGS row 14 — explicit `+` sign.
#[test]
fn cfg_14_main_plus_sign() {
    let mut rng = Rng::new();
    for _ in 0..200 {
        let m = rng.range_i64(0, 4294967296);
        assert_main_eq(format!("+{m}").as_bytes(), Stdin::File);
    }
    assert_main_eq(b"+0", Stdin::File);
    assert_main_eq(b"+0000000042", Stdin::File);
}

/// CONFIGS row 15 — leading whitespace: each class alone, and random mixes.
#[test]
fn cfg_15_main_whitespace_classes() {
    let mut rng = Rng::new();
    for w in WS {
        for reps in [1usize, 2, 7] {
            let mut input = vec![w; reps];
            input.extend_from_slice(b"-12345");
            assert_main_eq(&input, Stdin::File);
        }
    }
    for _ in 0..200 {
        let n = rng.below(12) as usize;
        let mut input: Vec<u8> = (0..n).map(|_| *rng.pick(&WS)).collect();
        let v = rng.range_i64(i32::MIN as i64, i32::MAX as i64);
        input.extend_from_slice(format!("{v}").as_bytes());
        // random trailing whitespace as well
        let m = rng.below(4) as usize;
        input.extend((0..m).map(|_| *rng.pick(&WS)));
        assert_main_eq(&input, Stdin::File);
    }
}

/// CONFIGS row 16 — leading zeros (long digit run, small value).
#[test]
fn cfg_16_main_leading_zeros() {
    let mut rng = Rng::new();
    for _ in 0..200 {
        let zeros = 1 + rng.below(64) as usize;
        let v = rng.range_i64(0, i32::MAX as i64);
        let sign = if rng.below(2) == 0 { "" } else { "-" };
        let input = format!("{sign}{}{v}", "0".repeat(zeros));
        assert_main_eq(input.as_bytes(), Stdin::File);
    }
    assert_main_eq(b"0", Stdin::File);
    assert_main_eq(b"-0", Stdin::File);
    assert_main_eq(b"+0", Stdin::File);
    assert_main_eq("0".repeat(200).as_bytes(), Stdin::File);
    assert_main_eq(format!("-{}", "0".repeat(200)).as_bytes(), Stdin::File);
}

/// CONFIGS row 17 — digit runs of 1..=19 digits (always representable as
/// `long`), so the `long -> int` truncation is what is under test.
#[test]
fn cfg_17_main_digit_run_lengths() {
    let mut rng = Rng::new();
    for len in 1..=19usize {
        for _ in 0..10 {
            let mut s = String::new();
            s.push(char::from(b'1' + (rng.below(9) as u8)));
            for _ in 1..len {
                s.push(char::from(b'0' + (rng.below(10) as u8)));
            }
            assert_main_eq(s.as_bytes(), Stdin::File);
            assert_main_eq(format!("-{s}").as_bytes(), Stdin::File);
        }
    }
}

/// CONFIGS row 18 — digit runs of 20..=80 digits: forces `strtol` `ERANGE`
/// saturation to `LONG_MAX` / `LONG_MIN` before the truncation to `int`.
#[test]
fn cfg_18_main_long_digit_runs() {
    let mut rng = Rng::new();
    for _ in 0..150 {
        let len = 20 + rng.below(61) as usize;
        let mut s = String::new();
        s.push(char::from(b'1' + (rng.below(9) as u8)));
        for _ in 1..len {
            s.push(char::from(b'0' + (rng.below(10) as u8)));
        }
        assert_main_eq(s.as_bytes(), Stdin::File);
        assert_main_eq(format!("-{s}").as_bytes(), Stdin::File);
        assert_main_eq(format!("+{s}\n").as_bytes(), Stdin::File);
    }
    // pathological: 5000 digits
    let long = "9".repeat(5000);
    assert_main_eq(long.as_bytes(), Stdin::File);
    assert_main_eq(format!("-{long}").as_bytes(), Stdin::File);
    // 5000 leading zeros then a small value (long run, tiny value)
    let padded = format!("{}{}", "0".repeat(5000), 12345);
    assert_main_eq(padded.as_bytes(), Stdin::File);
}

/// CONFIGS row 19 — values straddling the `int` range boundaries.
#[test]
fn cfg_19_main_int_boundary() {
    let mut rng = Rng::new();
    for base in [i32::MAX as i64, i32::MIN as i64, 0i64, 4294967296i64] {
        for d in -8i64..=8 {
            let v = base + d;
            assert_main_eq(format!("{v}").as_bytes(), Stdin::File);
            assert_main_eq(format!("{v}\n").as_bytes(), Stdin::File);
        }
    }
    for _ in 0..100 {
        let v = rng.range_i64(i32::MAX as i64 - 1000, i32::MAX as i64 + 1000);
        assert_main_eq(format!("{v}").as_bytes(), Stdin::File);
        let v = rng.range_i64(i32::MIN as i64 - 1000, i32::MIN as i64 + 1000);
        assert_main_eq(format!("{v}").as_bytes(), Stdin::File);
    }
}

/// CONFIGS row 20 — values straddling the `long` range boundaries (the
/// saturation branch of glibc's `%d`).
#[test]
fn cfg_20_main_long_boundary() {
    // exact boundaries, written out as decimal strings
    for s in [
        "9223372036854775806",
        "9223372036854775807",
        "9223372036854775808",
        "9223372036854775809",
        "-9223372036854775807",
        "-9223372036854775808",
        "-9223372036854775809",
        "-9223372036854775810",
        "18446744073709551615",
        "18446744073709551616",
        "99999999999999999999",
        "-99999999999999999999",
    ] {
        assert_main_eq(s.as_bytes(), Stdin::File);
        assert_main_eq(format!("{s}\n").as_bytes(), Stdin::File);
    }
    let mut rng = Rng::new();
    for _ in 0..100 {
        let d = rng.range_i64(-8, 8);
        let v = (i64::MAX as i128 + d as i128) as u128;
        assert_main_eq(format!("{v}").as_bytes(), Stdin::File);
        let v = (i64::MIN as i128 + d as i128).unsigned_abs();
        assert_main_eq(format!("-{v}").as_bytes(), Stdin::File);
    }
}

/// CONFIGS row 21 — several numbers on stdin: only the first is converted.
#[test]
fn cfg_21_main_multiple_numbers() {
    let mut rng = Rng::new();
    for _ in 0..150 {
        let a = rng.range_i64(i32::MIN as i64, i32::MAX as i64);
        let b = rng.range_i64(i32::MIN as i64, i32::MAX as i64);
        let c = rng.range_i64(i32::MIN as i64, i32::MAX as i64);
        let sep = *rng.pick(&WS);
        let input = format!(
            "{a}{s}{b}{s}{c}\n",
            s = char::from(sep),
            a = a,
            b = b,
            c = c
        );
        assert_main_eq(input.as_bytes(), Stdin::File);
    }
}

/// CONFIGS row 22 — more than 4096 bytes of leading whitespace: crosses
/// glibc's `BUFSIZ` stdin refill boundary before the first digit.
#[test]
fn cfg_22_main_huge_ws_prefix() {
    let mut rng = Rng::new();
    for pad in [4095usize, 4096, 4097, 8191, 8192, 8193, 20000] {
        let mut input = vec![b' '; pad];
        input.extend_from_slice(b"-424242\n");
        assert_main_eq(&input, Stdin::File);
    }
    for _ in 0..20 {
        let pad = 4000 + rng.below(300) as usize;
        let mut input: Vec<u8> = (0..pad).map(|_| *rng.pick(&WS)).collect();
        let v = rng.range_i64(i32::MIN as i64, i32::MAX as i64);
        input.extend_from_slice(format!("{v}").as_bytes());
        assert_main_eq(&input, Stdin::File);
    }
    // whitespace only, longer than the buffer -> EOF after the skip
    assert_main_eq(&vec![b'\n'; 9000], Stdin::File);
}

/// CONFIGS row 23 — the digit run itself straddles offset 4096 (buffer refill
/// in the middle of the number).
#[test]
fn cfg_23_main_number_across_buffer() {
    let mut rng = Rng::new();
    for zeros in [4090usize, 4094, 4095, 4096, 4097, 4100] {
        let input = format!("{}{}", "0".repeat(zeros), "1234567");
        assert_main_eq(input.as_bytes(), Stdin::File);
        let input = format!("-{}{}", "0".repeat(zeros), "7654321");
        assert_main_eq(input.as_bytes(), Stdin::File);
    }
    for _ in 0..20 {
        let zeros = 4080 + rng.below(40) as usize;
        let v = rng.range_i64(0, i32::MAX as i64);
        let input = format!("{}{v}", "0".repeat(zeros));
        assert_main_eq(input.as_bytes(), Stdin::File);
    }
    // sign at the very end of the first buffer
    let input = format!("{}-{}", " ".repeat(4095), 987654321);
    assert_main_eq(input.as_bytes(), Stdin::File);
}

/// CONFIGS row 24 — non-seekable stdin (pipe).
#[test]
fn cfg_24_main_stdin_pipe() {
    let mut rng = Rng::new();
    for _ in 0..150 {
        let v = rng.range_i64(i32::MIN as i64, i32::MAX as i64);
        assert_main_eq(format!("{v}\n").as_bytes(), Stdin::Pipe);
    }
    assert_main_eq(b"", Stdin::Pipe);
    assert_main_eq(b"   ", Stdin::Pipe);
    assert_main_eq(b"abc", Stdin::Pipe);
    assert_main_eq(b"-", Stdin::Pipe);
    assert_main_eq(b"99999999999999999999999999\n", Stdin::Pipe);
    assert_main_eq(b"-99999999999999999999999999\n", Stdin::Pipe);
    let mut big = vec![b' '; 5000];
    big.extend_from_slice(b"777\n");
    assert_main_eq(&big, Stdin::Pipe);
}

/// CONFIGS row 25 — stdin is `/dev/null` (character device, instant EOF).
#[test]
fn cfg_25_main_stdin_devnull() {
    assert_main_eq(b"", Stdin::DevNull);
    assert_main_eq(b"12345", Stdin::DevNull); // bytes are ignored: /dev/null
}

/// CONFIGS row 26 — random raw byte blobs over the whole `%d` state machine.
#[test]
fn cfg_26_main_random_blobs() {
    const ALPHABET: &[u8] = b"0123456789+- \t\n\r\x0b\x0cabxXzZ.,;:/*#\0\x01\xff\x80eE";
    let mut rng = Rng::new();
    for _ in 0..800 {
        let n = rng.below(65) as usize;
        let input: Vec<u8> = (0..n).map(|_| *rng.pick(ALPHABET)).collect();
        assert_main_eq(&input, Stdin::File);
    }
    // blobs biased towards digits and signs
    const NUMERIC: &[u8] = b"0123456789+-0123456789 0123456789";
    for _ in 0..400 {
        let n = rng.below(40) as usize;
        let input: Vec<u8> = (0..n).map(|_| *rng.pick(NUMERIC)).collect();
        assert_main_eq(&input, Stdin::File);
    }
}

/// CONFIGS row 29 — repeated invocations with different stdin content:
/// no state must leak between calls.
#[test]
fn cfg_29_main_repeated_invocations() {
    let mut rng = Rng::with_seed(0x5DEE_CE66_D3A5_1B01);
    for _ in 0..60 {
        let v = rng.range_i64(i32::MIN as i64, i32::MAX as i64);
        assert_main_eq(format!("{v}\n").as_bytes(), Stdin::File);
        assert_main_eq(b"", Stdin::File);
        assert_main_eq(b"garbage", Stdin::File);
        assert_main_eq(format!("  +{}  ", v.unsigned_abs()).as_bytes(), Stdin::Pipe);
    }
}
