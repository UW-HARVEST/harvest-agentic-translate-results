//! Systematic (rather than sampled) sweeps: instead of trusting that the
//! hand-picked rows cover the parser, enumerate whole input spaces and compare
//! the two executables on every element.
//!
//! These complement the CONFIGS.md rows — every byte value and every short
//! combination of the "interesting" bytes is checked.

mod common;
use common::*;

/// Every byte value on its own, and every byte value followed by a newline.
#[test]
fn sweep_all_single_bytes() {
    for b in 0u16..=255 {
        let b = b as u8;
        assert_exe_matches(&[b], &format!("single byte {:#04x}", b));
        assert_exe_matches(&[b, b'\n'], &format!("byte {:#04x} + LF", b));
    }
}

/// Every byte value in front of, and behind, a valid number.
#[test]
fn sweep_byte_around_number() {
    for b in 0u16..=255 {
        let b = b as u8;
        assert_exe_matches(&[b, b'1', b'2', b'\n'], &format!("{:#04x} before", b));
        assert_exe_matches(&[b'1', b'2', b, b'\n'], &format!("{:#04x} after", b));
        assert_exe_matches(&[b'-', b, b'3', b'\n'], &format!("- {:#04x} 3", b));
    }
}

const ALPHA2: &[u8] = b"0129+- \t\n\r\x0b\x0cxX.\0\xff89aeE";
const ALPHA3: &[u8] = b"09+- \t\n\r\0\xff2x.E7";

/// All two-byte strings over a 22-character alphabet (484 inputs).
#[test]
fn sweep_all_two_byte_combinations() {
    for &a in ALPHA2 {
        for &b in ALPHA2 {
            assert_exe_matches(&[a, b], &format!("2-byte {:#04x},{:#04x}", a, b));
        }
    }
}

/// All three-byte strings over a 16-character alphabet (4096 inputs).
#[test]
fn sweep_all_three_byte_combinations() {
    for &a in ALPHA3 {
        for &b in ALPHA3 {
            for &c in ALPHA3 {
                assert_exe_matches(
                    &[a, b, c],
                    &format!("3-byte {:#04x},{:#04x},{:#04x}", a, b, c),
                );
            }
        }
    }
}

/// Numbers whose decimal length crosses every interesting width, for both
/// signs, with and without a newline: 1..=25 digits of 9s, 1s and 0s, plus the
/// exact decimal strings of the i32/i64 boundaries and their neighbours.
#[test]
fn sweep_digit_lengths_and_boundaries() {
    for len in 1..=25usize {
        for digit in [b'0', b'1', b'9'] {
            let body: String = std::iter::repeat(digit as char).take(len).collect();
            for sign in ["", "-", "+"] {
                for nl in ["\n", ""] {
                    let s = format!("{}{}{}", sign, body, nl);
                    assert_exe_matches(s.as_bytes(), &format!("len {} {:?}", len, s));
                }
            }
        }
    }
    let mut boundaries: Vec<i128> = Vec::new();
    for base in [
        0i128,
        i32::MAX as i128,
        i32::MIN as i128,
        u32::MAX as i128,
        i64::MAX as i128,
        i64::MIN as i128,
        u64::MAX as i128,
    ] {
        for d in -2i128..=2 {
            boundaries.push(base + d);
        }
    }
    for b in boundaries {
        for nl in ["\n", ""] {
            let s = format!("{}{}", b, nl);
            assert_exe_matches(s.as_bytes(), &format!("boundary {:?}", s));
        }
    }
}
