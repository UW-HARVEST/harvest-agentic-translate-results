//! Phase B -- valid-path differential tests.
//!
//! One test per row of `CONFIGS.md` (rows 1-26). Every test drives BOTH the C
//! `.so` and the Rust `.so` through `libloading` and compares the captured
//! stdout bytes. Randomized rows use a fixed seed so failures reproduce.

mod common;

use common::*;
use std::ffi::{c_char, c_int};

const PHCL: &str = "printHexCharLine";
const DRV: &str = "driver";

fn rng(tag: u64) -> Rng {
    // Fixed, per-row seeds -> reproducible.
    Rng::new(0x5EED_0000_0000_0000 ^ tag)
}

// ---------------------------------------------------------------------------
// Rows 1-9: the low-level entry point `printHexCharLine`, called directly.
// ---------------------------------------------------------------------------

/// Row 1: `0x00` -- zero, non-negative, zero-padded.
#[test]
fn row01_phcl_zero() {
    assert_same_char(PHCL, 0x00u8 as c_char);
}

/// Row 2: randomized `0x01..=0x0f` -- one significant hex digit, zero-padded.
#[test]
fn row02_phcl_low_nibble_padded() {
    let mut r = rng(2);
    for _ in 0..SAMPLES {
        assert_same_char(PHCL, r.range_u8(0x01, 0x0f) as c_char);
    }
}

/// Row 3: `0x10` -- the zero-padding boundary.
#[test]
fn row03_phcl_pad_boundary() {
    assert_same_char(PHCL, 0x10u8 as c_char);
}

/// Row 4: randomized `0x11..=0x7e` -- two digits, no padding.
#[test]
fn row04_phcl_two_digit_positive() {
    let mut r = rng(4);
    for _ in 0..SAMPLES {
        assert_same_char(PHCL, r.range_u8(0x11, 0x7e) as c_char);
    }
}

/// Row 5: `0x7f` == `CHAR_MAX`.
#[test]
fn row05_phcl_char_max() {
    assert_same_char(PHCL, 0x7fu8 as c_char);
}

/// Row 6: `0x80` == `CHAR_MIN`; sign extension makes this print `ffffff80`.
#[test]
fn row06_phcl_char_min() {
    assert_same_char(PHCL, 0x80u8 as c_char);
}

/// Row 7: randomized `0x81..=0xfe` -- negative, sign-extended 8-digit output.
#[test]
fn row07_phcl_negative_range() {
    let mut r = rng(7);
    for _ in 0..SAMPLES {
        assert_same_char(PHCL, r.range_u8(0x81, 0xfe) as c_char);
    }
}

/// Row 8: `0xff` == `-1`.
#[test]
fn row08_phcl_minus_one() {
    assert_same_char(PHCL, 0xffu8 as c_char);
}

/// Row 9: exhaustive sweep of all 256 `char` bit patterns.
#[test]
fn row09_phcl_exhaustive() {
    for v in 0u16..=0xff {
        assert_same_char(PHCL, v as u8 as c_char);
    }
}

// ---------------------------------------------------------------------------
// Rows 10-20: the wrapper entry point `driver`.
// ---------------------------------------------------------------------------

/// Row 10: `0x00` -> result `0x01`.
#[test]
fn row10_driver_zero() {
    assert_same_char(DRV, 0x00u8 as c_char);
}

/// Row 11: randomized `0x01..=0x0e` -> result still zero-padded.
#[test]
fn row11_driver_low_nibble() {
    let mut r = rng(11);
    for _ in 0..SAMPLES {
        assert_same_char(DRV, r.range_u8(0x01, 0x0e) as c_char);
    }
}

/// Row 12: `0x0f` -> result `0x10` crosses the padding boundary.
#[test]
fn row12_driver_crosses_pad_boundary() {
    assert_same_char(DRV, 0x0fu8 as c_char);
}

/// Row 13: randomized `0x10..=0x7d` -> two-digit non-negative result.
#[test]
fn row13_driver_two_digit_positive() {
    let mut r = rng(13);
    for _ in 0..SAMPLES {
        assert_same_char(DRV, r.range_u8(0x10, 0x7d) as c_char);
    }
}

/// Row 14: `0x7e` -> result `0x7f` == `CHAR_MAX`, still non-negative.
#[test]
fn row14_driver_result_char_max() {
    assert_same_char(DRV, 0x7eu8 as c_char);
}

/// Row 15: `0x7f` -> `data + 1` overflows `char`; truncates to `-128`.
#[test]
fn row15_driver_signed_overflow() {
    assert_same_char(DRV, 0x7fu8 as c_char);
}

/// Row 16: `0x80` == `CHAR_MIN` -> result `0x81`.
#[test]
fn row16_driver_char_min() {
    assert_same_char(DRV, 0x80u8 as c_char);
}

/// Row 17: randomized `0x81..=0xfd` -- negative in, negative out.
#[test]
fn row17_driver_negative_range() {
    let mut r = rng(17);
    for _ in 0..SAMPLES {
        assert_same_char(DRV, r.range_u8(0x81, 0xfd) as c_char);
    }
}

/// Row 18: `0xfe` -> result `-1`.
#[test]
fn row18_driver_result_minus_one() {
    assert_same_char(DRV, 0xfeu8 as c_char);
}

/// Row 19: `0xff` -> result `0x00`, crosses the sign boundary downward.
#[test]
fn row19_driver_wraps_to_zero() {
    assert_same_char(DRV, 0xffu8 as c_char);
}

/// Row 20: exhaustive sweep of all 256 `char` bit patterns.
#[test]
fn row20_driver_exhaustive() {
    for v in 0u16..=0xff {
        assert_same_char(DRV, v as u8 as c_char);
    }
}

// ---------------------------------------------------------------------------
// Rows 21-22: axis A6 -- over-wide `int` argument across the ABI boundary.
// ---------------------------------------------------------------------------

const WIDE: [c_int; 14] = [
    0, 1, 0x7f, 0x80, 0xff, 0x100, 0x1ff, 0x7fff, -1, -128, -129, -1000, c_int::MIN, c_int::MAX,
];

/// Row 21: `printHexCharLine` called as `void f(int)`.
#[test]
fn row21_phcl_overwide_int() {
    let mut r = rng(21);
    for &v in WIDE.iter() {
        assert_same_int(PHCL, v);
    }
    for _ in 0..SAMPLES {
        assert_same_int(PHCL, r.next_i32());
    }
}

/// Row 22: `driver` called as `void f(int)`.
#[test]
fn row22_driver_overwide_int() {
    let mut r = rng(22);
    for &v in WIDE.iter() {
        assert_same_int(DRV, v);
    }
    for _ in 0..SAMPLES {
        assert_same_int(DRV, r.next_i32());
    }
}

// ---------------------------------------------------------------------------
// Rows 23-26: axis A7/A8 -- sequencing, interleaving, buffering.
// ---------------------------------------------------------------------------

/// Row 23: a long randomized sequence alternating both entry points, compared
/// as one captured byte stream per implementation.
#[test]
fn row23_long_mixed_sequence() {
    let mut r = rng(23);
    let mut calls: Vec<(&str, c_char)> = Vec::new();
    for i in 0..512 {
        let name = if i % 2 == 0 { DRV } else { PHCL };
        calls.push((name, r.range_u8(0, 0xff) as c_char));
    }
    assert_same_sequence(&calls);
}

/// Row 24: C and Rust calls interleaved 1:1 inside a *single* capture; every
/// even line (C) must equal the following odd line (Rust).
#[test]
fn row24_interleaved_single_capture() {
    let mut r = rng(24);
    let args: Vec<c_char> = (0..256).map(|_| r.range_u8(0, 0xff) as c_char).collect();

    for &name in SYMBOLS.iter() {
        let fc = sym_char(Impl::C, name);
        let fr = sym_char(Impl::Rust, name);
        let out = capture(|| {
            for &a in &args {
                unsafe {
                    fc(a);
                    fr(a);
                }
            }
        });
        let lines: Vec<&[u8]> = out.split(|&b| b == b'\n').collect();
        // trailing empty element after the final '\n'
        assert_eq!(
            lines.last().map(|l| l.len()),
            Some(0),
            "{name}: capture did not end with a newline"
        );
        let lines = &lines[..lines.len() - 1];
        assert_eq!(
            lines.len(),
            args.len() * 2,
            "{name}: expected {} lines, got {}",
            args.len() * 2,
            lines.len()
        );
        for (i, &a) in args.iter().enumerate() {
            assert_eq!(
                lines[2 * i],
                lines[2 * i + 1],
                "{name}(0x{:02x}): C line {:?} != Rust line {:?}",
                a as u8,
                String::from_utf8_lossy(lines[2 * i]),
                String::from_utf8_lossy(lines[2 * i + 1])
            );
        }
    }
}

/// Row 25: composed pipeline -- `driver(x)` must equal `printHexCharLine(x+1)`
/// in each library, *and* across libraries (Rust `driver` vs C `printHexCharLine`).
#[test]
fn row25_composition_driver_equals_phcl_plus_one() {
    let mut r = rng(25);
    let mut args: Vec<c_char> = (0u16..=0xff).map(|v| v as u8 as c_char).collect();
    args.extend((0..SAMPLES).map(|_| r.range_u8(0, 0xff) as c_char));

    for a in args {
        let next = (a as i8).wrapping_add(1) as c_char;
        let mut outs = Vec::new();
        for imp in [Impl::C, Impl::Rust] {
            let d = sym_char(imp, DRV);
            let p = sym_char(imp, PHCL);
            outs.push(capture(|| unsafe { d(a) }));
            outs.push(capture(|| unsafe { p(next) }));
        }
        for (i, o) in outs.iter().enumerate() {
            assert_eq!(
                &outs[0],
                o,
                "composition mismatch for 0x{:02x} (variant {i}): {:?} vs {:?}",
                a as u8,
                String::from_utf8_lossy(&outs[0]),
                String::from_utf8_lossy(o)
            );
        }
    }
}

/// Row 26: many calls with no intervening flush -- fully buffered `stdout`
/// (regular file), flushed once at the end of the capture.
#[test]
fn row26_buffered_no_intermediate_flush() {
    let mut r = rng(26);
    for &name in SYMBOLS.iter() {
        let args: Vec<c_char> = (0..1024).map(|_| r.range_u8(0, 0xff) as c_char).collect();
        let run = |imp: Impl| {
            let f = sym_char(imp, name);
            capture(|| {
                for &a in &args {
                    unsafe { f(a) }
                }
            })
        };
        let oc = run(Impl::C);
        let or = run(Impl::Rust);
        assert_eq!(
            oc.len(),
            or.len(),
            "{name}: buffered stream lengths differ ({} vs {})",
            oc.len(),
            or.len()
        );
        assert_eq!(oc, or, "{name}: buffered byte streams differ");
    }
}
