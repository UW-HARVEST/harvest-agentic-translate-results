//! Level 3: `void driver(const char *in)` — the public API from `driver.h`.
//!
//! `driver` parses up to 100 integers with `sscanf(in, "%d%zn", ...)` and
//! `printf`s a single result, so these tests compare the captured stdout bytes.

mod common;

use common::*;

fn empty_and_whitespace_only() {
    for s in ["", " ", "   ", "\t", "\n", "\r", "\x0b", "\x0c", " \t\n\r\x0b\x0c "] {
        compare_driver(s);
    }
}

fn single_numbers() {
    for s in ["0", "1", "-1", "+1", "7", "-7", "  42", "42  ", "\t\n-99\r\n"] {
        compare_driver(s);
    }
}

fn multiple_numbers() {
    compare_driver("1 2 3");
    compare_driver("1 2 3 4 5 6 7 8 9 10");
    compare_driver("-1 -2 -3");
    compare_driver("+1 +2 +3");
    compare_driver("1\t2\n3\r4\x0b5\x0c6");
    compare_driver("   1   2   3   ");
    compare_driver("1  -2  +3  -4");
}

fn stops_at_first_non_number() {
    for s in [
        "5abc",
        "abc",
        "abc 5",
        "1 2 x 3 4",
        "1,2,3",
        "1;2",
        "1.5",
        "1.5 2.5",
        "-",
        "+",
        "--5",
        "+-5",
        "- 5",
        "+ 5",
        "1 - 2",
        "1 -",
        "1 +",
        ".",
        "..1",
        "e5",
        "0x10",
        "0X10",
        "10x",
        "1e5",
        "#",
        "/1",
    ] {
        compare_driver(s);
    }
}

fn leading_zeros_and_octal_looking_input() {
    // `%d` is always base 10, so "010" is ten, and "0x10" stops after the "0".
    for s in [
        "0", "00", "000", "010", "0010", "0x", "0x1", "00x10", "-0", "+0", "-00", "0 0x10",
        "007 010 0",
    ] {
        compare_driver(s);
    }
}

fn int_boundary_values() {
    for s in [
        "2147483647",
        "-2147483648",
        "2147483648",
        "-2147483649",
        "4294967295",
        "4294967296",
        "4294967297",
        "-4294967296",
        "2147483647 -2147483648",
        "-2147483648 2147483647",
    ] {
        compare_driver(s);
    }
}

/// glibc converts the digit run with `strtol` (saturating at `LONG_MAX` /
/// `LONG_MIN`) and then truncates the `long` to `int`.
fn long_boundary_and_overflow_saturation() {
    for s in [
        "9223372036854775806",
        "9223372036854775807",
        "9223372036854775808",
        "9223372036854775809",
        "-9223372036854775807",
        "-9223372036854775808",
        "-9223372036854775809",
        "18446744073709551615",
        "18446744073709551616",
        "99999999999999999999",
        "-99999999999999999999",
        "123456789012345678901234567890",
        "-123456789012345678901234567890",
        "9223372036854775807 1",
        "1 9223372036854775808",
        "0000000000000000000009223372036854775808",
        "-0000000000000000000009223372036854775809",
    ] {
        compare_driver(s);
    }
}

fn very_long_digit_runs() {
    for n in [30usize, 64, 100, 512, 1000] {
        let digits: String = std::iter::repeat_n('9', n).collect();
        compare_driver(&digits);
        compare_driver(&format!("-{digits}"));
        compare_driver(&format!("+{digits}"));
        let zeros: String = std::iter::repeat_n('0', n).collect();
        compare_driver(&zeros);
        compare_driver(&format!("{zeros}5"));
        compare_driver(&format!("-{zeros}123"));
    }
}

fn exactly_at_and_over_the_hundred_element_limit() {
    for count in [1usize, 2, 99, 100, 101, 150, 300] {
        let s = (0..count)
            .map(|i| (i as i64 - 50).to_string())
            .collect::<Vec<_>>()
            .join(" ");
        compare_driver(&s);
    }
    // 100 valid numbers followed by junk: the loop already ended at i == 100.
    let s = (0..100).map(|i| i.to_string()).collect::<Vec<_>>().join(" ");
    compare_driver(&format!("{s} junk"));
    compare_driver(&format!("{s} 12345"));
}

fn trailing_and_interior_whitespace_forms() {
    compare_driver("1 ");
    compare_driver("1\n");
    compare_driver("1\n\n2");
    compare_driver("\n\n\n1");
    compare_driver("1\r\n2\r\n3\r\n");
    compare_driver("\x0b\x0c1\x0c\x0b2");
    let padded = format!("{}{}{}", " ".repeat(200), "-5", " ".repeat(200));
    compare_driver(&padded);
}

fn embedded_nul_terminates_input() {
    // `driver` takes a `const char *`, so parsing stops at the NUL.
    compare_driver("1 2\0 3 4");
    compare_driver("\0 5");
}

fn randomized_numeric_strings() {
    let mut rng = Rng::new(0xD1CE_5EED);
    for _ in 0..iters(2000) {
        let count = rng.below(12);
        let mut s = String::new();
        for _ in 0..count {
            match rng.below(6) {
                0 => s.push_str(&(rng.next_i32() as i64).to_string()),
                1 => s.push_str(&format!("+{}", rng.next_u32() % 1_000_000)),
                2 => s.push_str(&format!("-{}", rng.next_u32() % 1_000_000)),
                3 => s.push_str(&format!("{:020}", rng.next_u32())),
                4 => s.push_str(&rng.next_u64().to_string()),
                _ => s.push_str(&format!("-{}", rng.next_u64())),
            }
            s.push(match rng.below(6) {
                0 => ' ',
                1 => '\t',
                2 => '\n',
                3 => '\r',
                4 => '\x0b',
                _ => '\x0c',
            });
        }
        compare_driver(&s);
    }
}

fn randomized_fuzz_alphabet() {
    const ALPHABET: &[u8] = b"0123456789+- \t\n\r\x0b\x0cxXeE.,;abzZ/*\x7f\x01";
    let mut rng = Rng::new(0xF0227A11_u64.wrapping_add(0x1234_5678));
    for _ in 0..iters(6000) {
        let n = rng.below(40);
        let bytes: Vec<u8> = (0..n).map(|_| ALPHABET[rng.below(ALPHABET.len())]).collect();
        let s = String::from_utf8(bytes).unwrap();
        compare_driver(&s);
    }
}

fn randomized_fuzz_full_byte_range() {
    let mut rng = Rng::new(0x0BAD_F00D);
    for _ in 0..iters(4000) {
        let n = rng.below(24);
        // Any non-NUL byte, including >= 0x80: a `char`-based `isspace`/`isdigit`
        // would treat those differently from an `unsigned char` based one.
        let bytes: Vec<u8> = (0..n).map(|_| 1 + (rng.next_u32() % 255) as u8).collect();
        compare_driver_bytes(&bytes, &format!("{bytes:?}"));
    }
}

fn high_bytes_around_digits() {
    // Bytes whose low 7 bits look like digits or signs (0x80 | '5' == 0xb5, etc.)
    for hi in [0x80u8, 0xa0, 0xc0, 0xff] {
        for lo in [b'0', b'5', b'9', b'+', b'-', b' '] {
            let b = [hi | lo, b'1', b'2'];
            compare_driver_bytes(&b, &format!("{b:?}"));
            let b = [b'1', hi | lo, b'2'];
            compare_driver_bytes(&b, &format!("{b:?}"));
            let b = [b'1', b' ', hi | lo];
            compare_driver_bytes(&b, &format!("{b:?}"));
        }
    }
    // Latin-1 NBSP and other bytes that are whitespace only in some locales.
    for b in [0xa0u8, 0x85, 0x1c, 0x1d, 0x1e, 0x1f, 0x00] {
        let s = [b, b'7'];
        compare_driver_bytes(&s, &format!("{s:?}"));
        let s = [b'7', b, b'8'];
        compare_driver_bytes(&s, &format!("{s:?}"));
    }
}

fn many_numbers_hits_the_hundred_cap_with_extremes() {
    let mut s = String::new();
    for i in 0..120 {
        let v: i64 = match i % 6 {
            0 => i64::from(i32::MAX),
            1 => i64::from(i32::MIN),
            2 => 0,
            3 => -1,
            4 => 9_223_372_036_854_775_807,
            _ => i as i64,
        };
        s.push_str(&v.to_string());
        s.push(' ');
    }
    compare_driver(&s);
}


fn main() {
    let cases: &[(&str, fn())] = &[
        ("empty_and_whitespace_only", empty_and_whitespace_only),
        ("single_numbers", single_numbers),
        ("multiple_numbers", multiple_numbers),
        ("stops_at_first_non_number", stops_at_first_non_number),
        ("leading_zeros_and_octal_looking_input", leading_zeros_and_octal_looking_input),
        ("int_boundary_values", int_boundary_values),
        ("long_boundary_and_overflow_saturation", long_boundary_and_overflow_saturation),
        ("very_long_digit_runs", very_long_digit_runs),
        ("exactly_at_and_over_the_hundred_element_limit", exactly_at_and_over_the_hundred_element_limit),
        ("trailing_and_interior_whitespace_forms", trailing_and_interior_whitespace_forms),
        ("embedded_nul_terminates_input", embedded_nul_terminates_input),
        ("randomized_numeric_strings", randomized_numeric_strings),
        ("randomized_fuzz_alphabet", randomized_fuzz_alphabet),
        ("randomized_fuzz_full_byte_range", randomized_fuzz_full_byte_range),
        ("high_bytes_around_digits", high_bytes_around_digits),
        ("many_numbers_hits_the_hundred_cap_with_extremes", many_numbers_hits_the_hundred_cap_with_extremes),
    ];
    let mut failures = 0usize;
    for (name, f) in cases {
        print!("driver::{name} ... ");
        let _ = std::io::Write::flush(&mut std::io::stdout());
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
            Ok(()) => println!("ok"),
            Err(_) => {
                println!("FAILED");
                failures += 1;
            }
        }
    }
    if failures > 0 {
        eprintln!("\n{failures} driver test group(s) failed");
        std::process::exit(1);
    }
    println!("\nall {} driver test groups passed", cases.len());
}
