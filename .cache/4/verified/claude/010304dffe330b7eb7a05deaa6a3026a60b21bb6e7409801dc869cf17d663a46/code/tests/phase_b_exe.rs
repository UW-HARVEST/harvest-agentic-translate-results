//! Phase B — end-to-end differential tests of the two real programs
//! (`c_src/build/driver` built by CMake vs. `target/<profile>/driver` built by
//! cargo).  Rows 27-28 of CONFIGS.md.
//!
//! This is the composed pipeline a real consumer runs: stdin is a pipe, stdout
//! is a pipe, and both the emitted bytes and the exit status are compared.

mod common;

use common::*;

const WS: [u8; 6] = [b' ', b'\t', b'\n', 0x0b, 0x0c, b'\r'];

/// CONFIGS row 27 — every input shape of rows 10-26, end to end.
#[test]
fn cfg_27_exe_end_to_end() {
    let fixed: Vec<&[u8]> = vec![
        b"",
        b" ",
        b"\n",
        b"\t\t\t",
        b"\x0b\x0c\r ",
        b"0",
        b"-0",
        b"+0",
        b"1",
        b"-1",
        b"+1",
        b"9",
        b"10",
        b"15",
        b"16",
        b"255",
        b"256",
        b"2147483646",
        b"2147483647",
        b"2147483648",
        b"-2147483647",
        b"-2147483648",
        b"-2147483649",
        b"4294967295",
        b"4294967296",
        b"9223372036854775807",
        b"9223372036854775808",
        b"-9223372036854775808",
        b"-9223372036854775809",
        b"99999999999999999999999999999999",
        b"-99999999999999999999999999999999",
        b"0000000000000000000000000000000042",
        b"abc",
        b"-",
        b"+",
        b"--5",
        b"-+5",
        b"- 5",
        b"+ 5",
        b".5",
        b"0x10",
        b"1e5",
        b"3.14",
        b"12 34",
        b"12-34",
        b"  42  ",
        b"\n\n\n7\n\n",
        b"42abc",
        b"\0 5",
        b"5\0",
        b"\xff\xfe5",
    ];
    for input in fixed {
        assert_exe_eq(input);
    }

    let mut rng = Rng::new();

    // random in-range values, with and without sign / newline
    for _ in 0..120 {
        let v = rng.range_i64(i32::MIN as i64, i32::MAX as i64);
        assert_exe_eq(format!("{v}").as_bytes());
        assert_exe_eq(format!("{v}\n").as_bytes());
    }
    // random out-of-int-range but in-long-range values
    for _ in 0..80 {
        let v = rng.range_i64(i64::MIN, i64::MAX);
        assert_exe_eq(format!("{v}\n").as_bytes());
    }
    // random over-long values (saturation)
    for _ in 0..60 {
        let len = 20 + rng.below(40) as usize;
        let mut s = String::new();
        s.push(char::from(b'1' + rng.below(9) as u8));
        for _ in 1..len {
            s.push(char::from(b'0' + rng.below(10) as u8));
        }
        assert_exe_eq(s.as_bytes());
        assert_exe_eq(format!("-{s}").as_bytes());
    }
    // random whitespace prefixes, including past the 4096-byte stdio block
    for _ in 0..20 {
        let pad = rng.below(9000) as usize;
        let mut input: Vec<u8> = (0..pad).map(|_| *rng.pick(&WS)).collect();
        let v = rng.range_i64(i32::MIN as i64, i32::MAX as i64);
        input.extend_from_slice(format!("{v}\n").as_bytes());
        assert_exe_eq(&input);
    }
    // long digit run straddling the stdio block boundary
    for zeros in [4095usize, 4096, 4097] {
        assert_exe_eq(format!("{}{}", "0".repeat(zeros), 1234567).as_bytes());
        assert_exe_eq(format!("-{}{}", "0".repeat(zeros), 1234567).as_bytes());
    }
}

/// CONFIGS row 28 — random raw byte blobs, end to end.
#[test]
fn cfg_28_exe_random_blobs() {
    const ALPHABET: &[u8] = b"0123456789+- \t\n\r\x0b\x0cabxXzZ.,;:/*#\0\x01\xff\x80eE";
    let mut rng = Rng::new();
    for _ in 0..1200 {
        let n = rng.below(48) as usize;
        let input: Vec<u8> = (0..n).map(|_| *rng.pick(ALPHABET)).collect();
        assert_exe_eq(&input);
    }
    const NUMERIC: &[u8] = b"0123456789+-0123456789 0123456789";
    for _ in 0..600 {
        let n = rng.below(32) as usize;
        let input: Vec<u8> = (0..n).map(|_| *rng.pick(NUMERIC)).collect();
        assert_exe_eq(&input);
    }
}
