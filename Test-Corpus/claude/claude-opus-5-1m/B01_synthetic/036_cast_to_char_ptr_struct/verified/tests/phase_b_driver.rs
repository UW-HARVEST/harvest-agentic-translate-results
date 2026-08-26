//! Phase B — valid-path differential tests for the lowest-level exported
//! entry point, `void driver(int floors)`.
//!
//! Rows C1–C9 of CONFIGS.md. Every test calls the symbol in **both** shared
//! libraries through `dlopen`/`dlsym` and compares the bytes written to
//! stdout exactly, and additionally checks the C output against an independent
//! model of the struct's object representation so that the comparison cannot
//! pass vacuously.

mod common;

use common::*;

/// Independent model of the C output:
/// `floors` (LE) ++ `bedrooms = 3` (LE) ++ `bathrooms = 2.0` (LE bits) ++ '\n',
/// each byte as exactly two lowercase hex digits.
fn expected_line(floors: i32) -> String {
    let mut s = String::new();
    for b in floors.to_le_bytes() {
        s.push_str(&format!("{b:02x}"));
    }
    s.push_str("03000000"); // bedrooms = 3
    s.push_str("0000000000000040"); // bathrooms = 2.0
    s.push('\n');
    s
}

#[track_caller]
fn check_batch(values: &[i32], ctx: &str) {
    let wide: Vec<i64> = values.iter().map(|&v| v as i64).collect();
    let out = assert_driver_batch_same(&wide, false, ctx);
    for (i, &v) in values.iter().enumerate() {
        assert_eq!(
            String::from_utf8_lossy(&out[i]),
            expected_line(v),
            "{ctx}: C output does not match the independent model for driver({v})"
        );
    }
}

#[track_caller]
fn check(floors: i32, ctx: &str) {
    check_batch(&[floors], ctx);
}

/// C1 — `floors = 0`: every byte of the field is `0x00`.
#[test]
fn c1_floors_zero() {
    check(0, "C1");
}

/// C2 — all bytes in `0x01..=0x0f`: exercises the zero padding of `%02x`
/// (a plain `%x` would print "1234" instead of "01020304").
#[test]
fn c2_low_nibble_bytes_zero_padded() {
    let mut values: Vec<i32> = [
        0x0102_0304u32,
        0x0f0f_0f0f,
        0x0101_0101,
        0x0001_0002,
        0x0908_0706,
        0x0000_000f,
        0x0f00_0000,
    ]
    .iter()
    .map(|&v| v as i32)
    .collect();
    // Every single low byte value in isolation, at every byte offset.
    for byte in 0x00u32..=0x0f {
        for shift in [0, 8, 16, 24] {
            values.push((byte << shift) as i32);
        }
    }
    check_batch(&values, "C2");
}

/// C3 — positive values whose bytes are all `>= 0x10`, high bit clear.
#[test]
fn c3_positive_high_bytes() {
    let values: Vec<i32> = [
        0x1234_5678u32,
        0x7f7f_7f7f,
        0x1010_1010,
        0x7edc_ba98,
        0x4040_4040,
        0x1111_1111,
        0x7fff_ffff,
    ]
    .iter()
    .map(|&v| v as i32)
    .collect();
    check_batch(&values, "C3");
}

/// C4 — high bit set: `unsigned char` promotion must not sign-extend
/// (`%02x` of `0xff` is "ff", never "ffffffff").
#[test]
fn c4_negative_high_bit_bytes() {
    let mut values: Vec<i32> = [
        0x8000_0000u32,
        0xdead_beef,
        0xffff_ffff,
        0x8080_8080,
        0xfefe_fefe,
        0xf0f0_f0f0,
        0xcafe_babe,
    ]
    .iter()
    .map(|&v| v as i32)
    .collect();
    // Every high byte value in isolation, at every byte offset.
    for byte in [0x80u32, 0x81, 0xa5, 0xf0, 0xfe, 0xff] {
        for shift in [0, 8, 16, 24] {
            values.push((byte << shift) as i32);
        }
    }
    check_batch(&values, "C4");
}

/// C5 — the `int` extremes.
#[test]
fn c5_int_boundaries() {
    check_batch(
        &[
            i32::MIN,
            i32::MIN + 1,
            -2,
            -1,
            0,
            1,
            2,
            i32::MAX - 1,
            i32::MAX,
            i16::MIN as i32,
            i16::MAX as i32,
            u16::MAX as i32,
            u8::MAX as i32,
        ],
        "C5",
    );
}

/// C6 — embedded `0x00` bytes inside a non-zero value.
#[test]
fn c6_embedded_zero_bytes() {
    let values: Vec<i32> = [
        0x00ff_00ffu32,
        0xff00_00ff,
        0x0000_0001,
        0x0100_0000,
        0x00ff_ff00,
        0xff00_ff00,
        0x0000_ff00,
    ]
    .iter()
    .map(|&v| v as i32)
    .collect();
    check_batch(&values, "C6");
}

/// C7 — bytes that are ASCII control characters: they must appear as hex
/// digits, never as literal newlines/tabs in the output stream.
#[test]
fn c7_control_character_bytes() {
    let values: Vec<i32> = [
        0x0a0d_0920u32,
        0x0a0a_0a0a,
        0x0d0d_0d0d,
        0x0000_000a,
        0x0a00_0000,
        0x0908_0b0c,
        0x1b1b_1b1b,
    ]
    .iter()
    .map(|&v| v as i32)
    .collect();
    check_batch(&values, "C7");

    // Each line must contain exactly one newline, at its very end.
    let wide: Vec<i64> = values.iter().map(|&v| v as i64).collect();
    for (i, line) in assert_driver_batch_same(&wide, false, "C7 newlines")
        .iter()
        .enumerate()
    {
        assert_eq!(
            line.iter().filter(|&&b| b == b'\n').count(),
            1,
            "control byte leaked into the output stream for {:#010x}",
            values[i] as u32
        );
        assert_eq!(*line.last().unwrap(), b'\n');
    }
}

/// C8 — uniform random over the whole 32-bit range (fixed seed).
#[test]
fn c8_random_full_range() {
    let mut rng = Rng::new(0xC8);
    let values: Vec<i32> = (0..4096).map(|_| rng.next_i32()).collect();
    check_batch(&values, "C8 randomized");
}

/// C9 — many calls into one loaded instance: `house_t house = {0}` must be
/// re-initialized on every call, so the output must depend only on the
/// argument and never on the call history.
#[test]
fn c9_repeated_calls_no_state_leak() {
    let mut rng = Rng::new(0xC9);
    let values: Vec<i32> = (0..64).map(|_| rng.next_i32()).collect();
    let wide: Vec<i64> = values.iter().map(|&v| v as i64).collect();

    // Baseline: one call per value, in order, in a single loaded instance.
    let baseline = assert_driver_batch_same(&wide, false, "C9 baseline");

    // Reversed order, with every value repeated three times in a row, plus a
    // few values interleaved: each output must still equal its baseline.
    let mut order: Vec<i64> = Vec::new();
    let mut expect: Vec<usize> = Vec::new();
    for (i, &v) in wide.iter().enumerate().rev() {
        for _ in 0..3 {
            order.push(v);
            expect.push(i);
        }
        order.push(wide[0]);
        expect.push(0);
    }
    let replay = assert_driver_batch_same(&order, false, "C9 replay");
    for (k, &i) in expect.iter().enumerate() {
        assert_eq!(
            replay[k], baseline[i],
            "C9: driver({}) output changed across calls (state leak)",
            values[i]
        );
    }
}
