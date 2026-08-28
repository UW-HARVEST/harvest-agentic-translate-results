//! Level 1: leaf functions with no dependencies on the rest of the library.
//!   classify_mode, apply_multiplier, convert_time_factor,
//!   convert_negative_overflow, hash_time_value

mod common;

use common::*;
use std::ffi::CString;

#[test]
fn classify_mode_matches() {
    let pair = Pair::load();
    let (c, rs) = pair.classify_mode();

    let mut inputs: Vec<String> = vec![
        "standard".into(),
        "enhanced".into(),
        "turbo".into(),
        "extreme".into(),
        "".into(),
        " ".into(),
        "STANDARD".into(),
        "Standard".into(),
        "standar".into(),
        "standardx".into(),
        "standard ".into(),
        " standard".into(),
        "enhance".into(),
        "enhancedd".into(),
        "turb".into(),
        "turboo".into(),
        "extrem".into(),
        "extremee".into(),
        "s".into(),
        "e".into(),
        "t".into(),
        "x".into(),
        "mode".into(),
        "\u{1}\u{2}\u{3}".into(),
        "0123456789".into(),
    ];
    // every single-byte string (excluding NUL)
    for b in 1u8..=127 {
        inputs.push((b as char).to_string());
    }
    // prefixes of each keyword, to exercise strcmp's early exit
    for kw in ["standard", "enhanced", "turbo", "extreme"] {
        for n in 0..=kw.len() {
            inputs.push(kw[..n].to_string());
        }
    }

    for s in &inputs {
        let cs = CString::new(s.as_bytes()).unwrap();
        let a = unsafe { c(cs.as_ptr()) };
        let b = unsafe { rs(cs.as_ptr()) };
        assert_eq!(a, b, "classify_mode({s:?}): C={a:#x} Rust={b:#x}");
    }

    // non-UTF-8 / high-bit bytes
    for hi in [0x80u8, 0xC3, 0xFF, 0xFE] {
        let cs = CString::new(vec![hi, b'a', hi]).unwrap();
        let a = unsafe { c(cs.as_ptr()) };
        let b = unsafe { rs(cs.as_ptr()) };
        assert_eq!(a, b, "classify_mode(high-bytes {hi:#x})");
    }
}

#[test]
fn apply_multiplier_matches() {
    let pair = Pair::load();
    let (c, rs) = pair.apply_multiplier();

    let bases = interesting_ints();
    // exercise the fall-through chain plus the `default` arm
    let levels: Vec<i32> = (-8..=12).chain([100, -100, i32::MAX, i32::MIN]).collect();

    for &base in &bases {
        for &level in &levels {
            let a = unsafe { c(base, level) };
            let b = unsafe { rs(base, level) };
            assert_eq!(a, b, "apply_multiplier({base}, {level})");
        }
    }
}

#[test]
fn convert_time_factor_matches() {
    let pair = Pair::load();
    let (c, rs) = pair.convert_time_factor();

    for &d in &interesting_doubles() {
        let a = unsafe { c(d) };
        let b = unsafe { rs(d) };
        assert_eq!(a, b, "convert_time_factor({d:e})");
    }

    // sweep across the int boundary after the 1e12 scaling
    let mut k = -2_100_000_000i64;
    while k <= 2_100_000_000 {
        let d = k as f64 / 1e12;
        let a = unsafe { c(d) };
        let b = unsafe { rs(d) };
        assert_eq!(a, b, "convert_time_factor({d:e}) [k={k}]");
        k += 3_700_000;
    }
    for k in [
        -2147483649i64,
        -2147483648,
        -2147483647,
        -1,
        0,
        1,
        2147483646,
        2147483647,
        2147483648,
        2147483649,
        4294967296,
    ] {
        let d = k as f64 / 1e12;
        let a = unsafe { c(d) };
        let b = unsafe { rs(d) };
        assert_eq!(a, b, "convert_time_factor boundary k={k}");
    }
}

#[test]
fn convert_negative_overflow_matches() {
    let pair = Pair::load();
    let (c, rs) = pair.convert_negative_overflow();

    for &d in &interesting_doubles() {
        let a = unsafe { c(d) };
        let b = unsafe { rs(d) };
        assert_eq!(a, b, "convert_negative_overflow({d:e})");
    }

    for k in [
        -2147483649i64,
        -2147483648,
        -2147483647,
        -1,
        0,
        1,
        2147483646,
        2147483647,
        2147483648,
        2147483649,
    ] {
        // value * -1e15, so pre-divide by -1e15 to land on `k`
        let d = k as f64 / -1e15;
        let a = unsafe { c(d) };
        let b = unsafe { rs(d) };
        assert_eq!(a, b, "convert_negative_overflow boundary k={k}");
    }
}

#[test]
fn hash_time_value_matches() {
    let pair = Pair::load();
    let (c, rs) = pair.hash_time_value();

    let mut inputs: Vec<i64> = vec![
        0,
        1,
        -1,
        2,
        -2,
        255,
        256,
        -255,
        0x7F,
        0x80,
        0xFF,
        0x100,
        0x5A5A5A5A,
        -0x5A5A5A5A,
        0x1F,
        i64::MAX,
        i64::MIN,
        i64::MAX - 1,
        i64::MIN + 1,
        0x0102030405060708,
        -0x0102030405060708,
        0x8080808080808080u64 as i64,
        0xFFFFFFFFFFFFFFFFu64 as i64,
        0x00000000FFFFFFFF,
        0xFFFFFFFF00000000u64 as i64,
        1_700_000_000,
        1_700_000_000 >> 29,
        -1_700_000_000,
    ];
    // every distinctive byte in every byte position (covers the `<< 24` sign issue)
    for pos in 0..8 {
        for b in [1u8, 0x7F, 0x80, 0xAA, 0xFF] {
            inputs.push((b as i64) << (pos * 8));
        }
    }
    // deterministic pseudo-random spread
    let mut x: u64 = 0x243F6A8885A308D3;
    for _ in 0..5000 {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        inputs.push(x as i64);
    }

    for &t in &inputs {
        let a = unsafe { c(t) };
        let b = unsafe { rs(t) };
        assert_eq!(a, b, "hash_time_value({t}) C={a:#x} Rust={b:#x}");
    }
}
