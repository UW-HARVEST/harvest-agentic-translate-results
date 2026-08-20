//! Phase B — CONFIGS.md rows C1..C7: the two stateless, lowest-level exports
//! `safe_double_to_int` and `process_string`.

mod common;
use common::*;

// ------------------------------------------------------------------ C1, C2, C3

/// C1 — axis F: all hand-picked double classes.
#[test]
fn c1_safe_double_to_int_classes() {
    let p = Pair::new("C1");
    for d in double_classes() {
        p.safe_double_to_int(d);
    }
    // Explicit documented behaviour of the three guards, so a silent
    // both-wrong-the-same-way cannot hide (values read off the C source).
    assert_eq!(p.safe_double_to_int(f64::NAN), 0);
    assert_eq!(p.safe_double_to_int(f64::INFINITY), INT_MAX);
    assert_eq!(p.safe_double_to_int(f64::NEG_INFINITY), INT_MIN);
    assert_eq!(p.safe_double_to_int(INT_MAX as f64), INT_MAX);
    assert_eq!(p.safe_double_to_int(INT_MIN as f64), INT_MIN);
    assert_eq!(p.safe_double_to_int(1.9), 1);
    assert_eq!(p.safe_double_to_int(-1.9), -1);
    assert_eq!(p.safe_double_to_int(-0.0), 0);
}

/// C2 — randomized raw 64-bit patterns reinterpreted as f64. Covers every
/// exponent class including NaN, inf, subnormal and huge magnitudes.
#[test]
fn c2_safe_double_to_int_random_bit_patterns() {
    let p = Pair::new("C2");
    let mut rng = Rng::new(0x5AFE_D0B1_E70_1234);
    for _ in 0..20_000 {
        p.safe_double_to_int(rng.next_f64_bits());
    }
    // Also a spread of ordinary finite magnitudes (the bit-pattern generator is
    // dominated by huge exponents).
    for _ in 0..20_000 {
        p.safe_double_to_int(rng.next_f64_spread());
    }
}

/// C3 — randomized doubles concentrated on the INT_MIN / INT_MAX boundaries,
/// where the two range guards and the truncating cast interact.
#[test]
fn c3_safe_double_to_int_boundary_random() {
    let p = Pair::new("C3");
    let mut rng = Rng::new(0x00B0_0DA0_C3C3_C3C3);
    let anchors = [
        INT_MAX as f64,
        INT_MIN as f64,
        2147483648.0,
        -2147483648.0,
        2147483647.0,
        -2147483647.0,
        0.0,
        -1.0,
        1.0,
        4294967296.0,
    ];
    for _ in 0..4_096 {
        let a = anchors[rng.below(anchors.len() as u64) as usize];
        // nudge by a few ULPs and by fractional amounts
        let ulps = rng.range_i32(-4, 4) as i64;
        let bits = (a.to_bits() as i64).wrapping_add(ulps) as u64;
        p.safe_double_to_int(f64::from_bits(bits));

        let frac = (rng.below(2001) as f64 / 1000.0) - 1.0; // -1.0 ..= 1.0
        p.safe_double_to_int(a + frac);
        p.safe_double_to_int(a * (1.0 + frac * 1e-9));
    }
    // Every integer-valued double in a tight window around both boundaries.
    for k in -300i64..=300 {
        p.safe_double_to_int(INT_MAX as f64 + k as f64);
        p.safe_double_to_int(INT_MIN as f64 + k as f64);
        p.safe_double_to_int(INT_MAX as f64 + k as f64 + 0.5);
        p.safe_double_to_int(INT_MIN as f64 + k as f64 - 0.5);
    }
}

// ------------------------------------------------------------------ C4, C5, C6

/// C4 — axis C: lengths 0, 1, 48, 49, 50, 200 of plain ASCII.
#[test]
fn c4_process_string_lengths() {
    let p = Pair::new("C4");
    for len in [0usize, 1, 2, 47, 48, 49, 50, 51, 200] {
        let mut b = vec![b'A'; len];
        b.push(0);
        let got = p.process_string(&b);
        assert_eq!(got, (len as i32) * 65, "len={len}");
    }
    // Empty string => 0 (the `if (*str)` early exit).
    assert_eq!(p.process_string(b"\0"), 0);
    // Interior NUL: the scan stops there.
    assert_eq!(p.process_string(b"ab\0cd\0"), 97 + 98);
}

/// C5 — axis C: bytes with the high bit set. `char` is signed on x86-64, so each
/// such byte contributes a negative value; a translation that treated `char` as
/// unsigned would diverge here.
#[test]
fn c5_process_string_high_bit_bytes() {
    let p = Pair::new("C5");
    // 0x80 == -128 as signed char
    assert_eq!(p.process_string(&[0x80, 0]), -128);
    assert_eq!(p.process_string(&[0xFF, 0]), -1);
    assert_eq!(p.process_string(&[0x80, 0x80, 0x80, 0]), -384);
    assert_eq!(p.process_string(&[0x7F, 0x80, 0]), 127 - 128);

    let mut rng = Rng::new(0xC5C5_0000_1111_2222);
    for _ in 0..2_000 {
        let len = rng.below(513) as usize;
        // high-bit-only
        let mut b = rng.bytes(len, 0x80, 0xFF);
        b.push(0);
        p.process_string(&b);
        // mixed sign, never zero (zero would terminate early)
        let mut b2 = rng.bytes(len, 0x01, 0xFF);
        b2.push(0);
        p.process_string(&b2);
    }
}

/// C6 — randomized full byte range, random lengths up to 4096.
#[test]
fn c6_process_string_random() {
    let p = Pair::new("C6");
    let mut rng = Rng::new(0x0000_C6C6_DEAD_BEEF);
    for _ in 0..2_000 {
        let len = rng.below(4097) as usize;
        let mut b = rng.bytes(len, 0x01, 0xFF);
        b.push(0);
        p.process_string(&b);
    }
    // Buffers that contain interior NULs at random positions: only the prefix
    // before the first NUL may be summed.
    for _ in 0..500 {
        let len = 1 + rng.below(256) as usize;
        let mut b = rng.bytes(len, 0x00, 0xFF);
        b.push(0);
        let first_nul = b.iter().position(|&x| x == 0).unwrap();
        let expect: i32 = b[..first_nul]
            .iter()
            .fold(0i32, |a, &x| a.wrapping_add(x as i8 as i32));
        assert_eq!(p.process_string(&b), expect);
    }
}

/// C7 — oversized input whose byte sum overflows `int`. The C code does
/// `result += (int)(*str)` with no overflow check, so the accumulator wraps;
/// the Rust translation must wrap identically.
#[test]
fn c7_process_string_overflow_wraps() {
    let p = Pair::new("C7");
    const N: usize = 17_000_000; // 17e6 * 127 = 2_159_000_000 > INT_MAX
    let mut b = vec![0x7Fu8; N];
    b.push(0);
    let expect = (N as i64 * 127) as u32 as i32; // two's-complement wrap
    let got = p.process_string(&b);
    assert_eq!(got, expect, "expected wrapped sum {expect}, got {got}");
    assert!(got < 0, "the sum must have wrapped negative, got {got}");

    // Same in the negative direction.
    let mut b2 = vec![0x80u8; N];
    b2.push(0);
    let expect2 = (N as i64 * -128) as u32 as i32;
    assert_eq!(p.process_string(&b2), expect2);
}
