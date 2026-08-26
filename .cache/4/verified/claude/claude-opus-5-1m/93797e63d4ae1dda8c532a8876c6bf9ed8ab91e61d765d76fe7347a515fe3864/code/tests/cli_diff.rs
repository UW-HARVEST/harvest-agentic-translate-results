//! Phase B — differential tests for the stdin-driven entry points:
//!   * the exported `main` symbol of both shared objects (called via `dlsym`
//!     in a forked child so that stdio buffer state cannot leak between calls),
//!   * the two executables (`c_src/build/driver` vs `target/<profile>/driver`),
//!     comparing stdout, stderr, exit status and terminating signal.
//!
//! Rows M01–M21 of CONFIGS.md.

mod common;
use common::*;

fn v(s: &str) -> Vec<u8> {
    s.as_bytes().to_vec()
}

fn check_all(inputs: &[Vec<u8>], ctx: &str) {
    for (i, inp) in inputs.iter().enumerate() {
        assert_input_matches(inp, &format!("{} #{}", ctx, i));
    }
}

/// Executable-only comparison (used for the very large inputs, where spawning
/// is cheaper than forking twice).
fn check_exe_only(inputs: &[Vec<u8>], ctx: &str) {
    for (i, inp) in inputs.iter().enumerate() {
        assert_exe_matches(inp, &format!("{} #{}", ctx, i));
    }
}

// ---------------------------------------------------------------------------
// M01 — plain non-negative decimals
// ---------------------------------------------------------------------------
#[test]
fn cfg_m01_plain_decimals() {
    let inputs: Vec<Vec<u8>> = ["0\n", "1\n", "7\n", "9\n", "10\n", "12345\n", "2147483646\n"]
        .iter()
        .map(|s| v(s))
        .collect();
    check_all(&inputs, "M01");
}

// ---------------------------------------------------------------------------
// M02 — explicit signs
// ---------------------------------------------------------------------------
#[test]
fn cfg_m02_signs() {
    let inputs: Vec<Vec<u8>> = ["-7\n", "+7\n", "-0\n", "+0\n", "-1\n", "+1\n", "-12345\n"]
        .iter()
        .map(|s| v(s))
        .collect();
    check_all(&inputs, "M02");
}

// ---------------------------------------------------------------------------
// M03 — every whitespace class strtol skips
// ---------------------------------------------------------------------------
#[test]
fn cfg_m03_leading_whitespace() {
    let inputs: Vec<Vec<u8>> = [
        " 7\n",
        "\t7\n",
        "\x0b7\n",
        "\x0c7\n",
        "\r7\n",
        "  \t \x0b\x0c\r 7\n",
        " -7\n",
        "\t+7\n",
        "   \t-2147483648\n",
        "\n7\n",
        " \n",
    ]
    .iter()
    .map(|s| v(s))
    .collect();
    check_all(&inputs, "M03");
}

// ---------------------------------------------------------------------------
// M04 — leading zeros
// ---------------------------------------------------------------------------
#[test]
fn cfg_m04_leading_zeros() {
    let mut inputs: Vec<Vec<u8>> = ["007\n", "0000000000000000000000000007\n", "-007\n", "+000\n"]
        .iter()
        .map(|s| v(s))
        .collect();
    inputs.push(v(&format!("{}7\n", "0".repeat(90))));
    inputs.push(v(&format!("-{}7\n", "0".repeat(90))));
    inputs.push(v(&format!("{}2147483647\n", "0".repeat(80))));
    check_all(&inputs, "M04");
}

// ---------------------------------------------------------------------------
// M05 — trailing garbage after a valid prefix
// ---------------------------------------------------------------------------
#[test]
fn cfg_m05_trailing_garbage() {
    let inputs: Vec<Vec<u8>> = [
        "7abc\n",
        "7 8\n",
        "7.9\n",
        "0x10\n",
        "0X10\n",
        "7e3\n",
        "12,34\n",
        "5-\n",
        "5+\n",
        "-5-5\n",
        "1_000\n",
        "42\t\n",
        "42 \n",
        "8/2\n",
        "3;\n",
    ]
    .iter()
    .map(|s| v(s))
    .collect();
    check_all(&inputs, "M05");
}

// ---------------------------------------------------------------------------
// M06 — no trailing newline
// ---------------------------------------------------------------------------
#[test]
fn cfg_m06_no_trailing_newline() {
    let inputs: Vec<Vec<u8>> = ["7", "-7", "+7", "abc", "0", " 12", "2147483648", ""]
        .iter()
        .map(|s| v(s))
        .collect();
    check_all(&inputs, "M06");
}

// ---------------------------------------------------------------------------
// M07 — empty stdin
// ---------------------------------------------------------------------------
#[test]
fn cfg_m07_empty_stdin() {
    check_all(&[Vec::new()], "M07");
}

// ---------------------------------------------------------------------------
// M08 — blank first line
// ---------------------------------------------------------------------------
#[test]
fn cfg_m08_blank_first_line() {
    let inputs: Vec<Vec<u8>> = ["\n", "\n\n", "\n7\n", "\r\n", "\r\n7\n"]
        .iter()
        .map(|s| v(s))
        .collect();
    check_all(&inputs, "M08");
}

// ---------------------------------------------------------------------------
// M09 — int boundaries
// ---------------------------------------------------------------------------
#[test]
fn cfg_m09_int_boundaries() {
    let inputs: Vec<Vec<u8>> = [
        "2147483647\n",
        "2147483646\n",
        "-2147483648\n",
        "-2147483647\n",
        "+2147483647\n",
        "-2147483648",
        "2147483647",
    ]
    .iter()
    .map(|s| v(s))
    .collect();
    check_all(&inputs, "M09");
}

// ---------------------------------------------------------------------------
// M10 — between int and long
// ---------------------------------------------------------------------------
#[test]
fn cfg_m10_between_int_and_long() {
    let inputs: Vec<Vec<u8>> = [
        "2147483648\n",
        "2147483649\n",
        "-2147483649\n",
        "-2147483650\n",
        "4294967295\n",
        "4294967296\n",
        "9223372036854775807\n",
        "-9223372036854775808\n",
        "1000000000000\n",
    ]
    .iter()
    .map(|s| v(s))
    .collect();
    check_all(&inputs, "M10");
}

// ---------------------------------------------------------------------------
// M11 — beyond long (ERANGE)
// ---------------------------------------------------------------------------
#[test]
fn cfg_m11_beyond_long() {
    let mut inputs: Vec<Vec<u8>> = [
        "9223372036854775808\n",
        "9223372036854775809\n",
        "-9223372036854775809\n",
        "-9223372036854775810\n",
        "18446744073709551616\n",
    ]
    .iter()
    .map(|s| v(s))
    .collect();
    for n in [30usize, 60, 98, 99] {
        inputs.push(v(&format!("{}\n", "9".repeat(n))));
        inputs.push(v(&format!("-{}\n", "9".repeat(n))));
    }
    inputs.push(v(&format!("1{}\n", "0".repeat(98))));
    check_all(&inputs, "M11");
}

// ---------------------------------------------------------------------------
// M12 — line length vs the 99-byte fgets cap
// ---------------------------------------------------------------------------
#[test]
fn cfg_m12_length_boundaries() {
    let mut inputs: Vec<Vec<u8>> = Vec::new();
    for len in [96usize, 97, 98, 99, 100, 101, 150] {
        // digits only, with and without a trailing newline
        inputs.push(v(&format!("{}\n", "1".repeat(len))));
        inputs.push(v(&"1".repeat(len)));
        // small number followed by padding that crosses the cap
        inputs.push(v(&format!("7{}\n", "x".repeat(len))));
        inputs.push(v(&format!("{}7\n", " ".repeat(len))));
        // zeros then a digit: truncation may cut the digit off
        inputs.push(v(&format!("{}7\n", "0".repeat(len))));
    }
    check_all(&inputs, "M12");
}

// ---------------------------------------------------------------------------
// M13 — multi-line stdin (only the first line may be consumed)
// ---------------------------------------------------------------------------
#[test]
fn cfg_m13_multiline() {
    let mut inputs: Vec<Vec<u8>> = ["7\n9\n", "abc\n7\n", "\n\n\n", "-1\n-2\n-3\n", "7\n\n"]
        .iter()
        .map(|s| v(s))
        .collect();
    let mut many = String::new();
    for i in 0..100 {
        many.push_str(&format!("{}\n", i));
    }
    inputs.push(v(&many));
    check_all(&inputs, "M13");
}

// ---------------------------------------------------------------------------
// M14 — embedded NUL bytes
// ---------------------------------------------------------------------------
#[test]
fn cfg_m14_embedded_nul() {
    let inputs: Vec<Vec<u8>> = vec![
        b"\0".to_vec(),
        b"\0\n".to_vec(),
        b"\x007\n".to_vec(),
        b"7\0 9\n".to_vec(),
        b"12\0".to_vec(),
        b"\0\0\0\0\n".to_vec(),
        b" \0 7\n".to_vec(),
        b"-\0 7\n".to_vec(),
    ];
    check_all(&inputs, "M14");
}

// ---------------------------------------------------------------------------
// M15 — non-UTF-8 / high bytes
// ---------------------------------------------------------------------------
#[test]
fn cfg_m15_high_bytes() {
    let mut inputs: Vec<Vec<u8>> = vec![
        vec![0xff, b'\n'],
        vec![0x80, b'7', b'\n'],
        vec![0xc3, 0x28, b'\n'],
        vec![b'7', 0xff, b'\n'],
        vec![0xef, 0xbb, 0xbf, b'7', b'\n'], // UTF-8 BOM then 7
        vec![0xa0, 0xa1, 0xa2],
    ];
    // one input per byte value, prefixed to a valid number
    for b in 0u16..=255 {
        inputs.push(vec![b as u8, b'4', b'2', b'\n']);
    }
    check_all(&inputs, "M15");
}

// ---------------------------------------------------------------------------
// M16 — CRLF and lone CR
// ---------------------------------------------------------------------------
#[test]
fn cfg_m16_crlf() {
    let inputs: Vec<Vec<u8>> = ["7\r\n", "7\r", "\r7\n", "-7\r\n", "\r\r\r7\n"]
        .iter()
        .map(|s| v(s))
        .collect();
    check_all(&inputs, "M16");
}

// ---------------------------------------------------------------------------
// M17 — oversized stdin
// ---------------------------------------------------------------------------
#[test]
fn cfg_m17_oversized() {
    let big_digits = format!("{}\n", "1".repeat(100 * 1024));
    let big_no_nl = "9".repeat(100 * 1024);
    let big_ws = format!("{}7\n", " ".repeat(100 * 1024));
    let big_valid_prefix = format!("42{}\n", "0".repeat(100 * 1024));
    let inputs = vec![
        v(&big_digits),
        v(&big_no_nl),
        v(&big_ws),
        v(&big_valid_prefix),
    ];
    check_all(&inputs, "M17 ffi+exe");
}

// ---------------------------------------------------------------------------
// M18 — randomised byte strings
// ---------------------------------------------------------------------------
fn random_bytes(rng: &mut Rng, max_len: usize) -> Vec<u8> {
    const ALPHABET: &[u8] = b"0123456789+-  \t\n\r\x0b\x0cxXeE.,_abcfz\0\xff\x80";
    let len = rng.below(max_len as u64 + 1) as usize;
    (0..len)
        .map(|_| ALPHABET[rng.below(ALPHABET.len() as u64) as usize])
        .collect()
}

#[test]
fn cfg_m18_random_bytes() {
    let mut rng = Rng::new(0xC0FF_EE00_1234_5678);
    let inputs: Vec<Vec<u8>> = (0..384).map(|_| random_bytes(&mut rng, 120)).collect();
    check_all(&inputs, "M18");
}

#[test]
fn cfg_m18b_random_bytes_exe_only() {
    let mut rng = Rng::new(0x5EED_0BAD_CAFE_0001);
    let inputs: Vec<Vec<u8>> = (0..768).map(|_| random_bytes(&mut rng, 260)).collect();
    check_exe_only(&inputs, "M18b");
}

// ---------------------------------------------------------------------------
// M19 — randomised decimal texts around the interesting boundaries
// ---------------------------------------------------------------------------
fn random_number_text(rng: &mut Rng) -> Vec<u8> {
    let body = match rng.below(8) {
        0 => format!("{}", rng.next_i32()),
        1 => format!("{}", rng.next_u64() as i64),
        2 => {
            // near 2^31 / -2^31
            let d = rng.below(9) as i64 - 4;
            format!("{}", 2147483648i64 + d)
        }
        3 => {
            let d = rng.below(9) as i64 - 4;
            format!("{}", -2147483648i64 + d)
        }
        4 => {
            // near 2^63
            let d = rng.below(9) as u64;
            format!("9223372036854775{}", 800 + d)
        }
        5 => {
            let d = rng.below(9) as u64;
            format!("-9223372036854775{}", 800 + d)
        }
        6 => {
            // random digit string of random length
            let n = 1 + rng.below(40) as usize;
            (0..n)
                .map(|_| (b'0' + rng.below(10) as u8) as char)
                .collect()
        }
        _ => format!("{}", rng.next_u32()),
    };
    let prefix = match rng.below(6) {
        0 => "",
        1 => " ",
        2 => "\t",
        3 => "+",
        4 => "0000",
        _ => "",
    };
    let suffix = match rng.below(6) {
        0 => "\n",
        1 => "",
        2 => "abc\n",
        3 => " 5\n",
        4 => ".5\n",
        _ => "\n",
    };
    format!("{}{}{}", prefix, body, suffix).into_bytes()
}

#[test]
fn cfg_m19_random_numbers() {
    let mut rng = Rng::new(0x9E37_79B9_0000_0001);
    let inputs: Vec<Vec<u8>> = (0..512).map(|_| random_number_text(&mut rng)).collect();
    check_all(&inputs, "M19");
}

#[test]
fn cfg_m19b_random_numbers_exe_only() {
    let mut rng = Rng::new(0x1357_9BDF_0246_8ACE);
    let inputs: Vec<Vec<u8>> = (0..1024).map(|_| random_number_text(&mut rng)).collect();
    check_exe_only(&inputs, "M19b");
}

// ---------------------------------------------------------------------------
// M20 — whitespace-only lines of every class and length
// ---------------------------------------------------------------------------
#[test]
fn cfg_m20_whitespace_only() {
    let mut inputs: Vec<Vec<u8>> = Vec::new();
    for ws in [b' ', b'\t', 0x0b, 0x0c, b'\r'] {
        for len in [1usize, 2, 98, 99, 100, 101] {
            let mut s = vec![ws; len];
            inputs.push(s.clone());
            s.push(b'\n');
            inputs.push(s);
        }
    }
    check_all(&inputs, "M20");
}

// ---------------------------------------------------------------------------
// M21 — a valid number split by the 99-byte cap
// ---------------------------------------------------------------------------
#[test]
fn cfg_m21_number_split_by_cap() {
    let mut inputs: Vec<Vec<u8>> = Vec::new();
    for pad in [88usize, 89, 90, 91, 92, 98, 99] {
        inputs.push(v(&format!("{}1234567890123\n", " ".repeat(pad))));
        inputs.push(v(&format!("{}-1234567890123\n", " ".repeat(pad))));
        inputs.push(v(&format!("{}9999999999999999999999\n", " ".repeat(pad))));
    }
    check_all(&inputs, "M21");
}
