//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test drives both the C `.so` and the
//! Rust `.so` through their exported `driver` symbol and asserts the captured
//! `stdout` bytes match exactly. Randomized rows use a fixed seed.

mod common;

use common::{Rng, assert_same, assert_same_floats};

/// Row C1 — positive normal floats, random mantissa and exponent.
#[test]
fn b_c1_positive_normals() {
    let mut rng = Rng::new(0x0000_0001);
    let mut bits = Vec::with_capacity(4096);
    for _ in 0..4096 {
        // exponent 1..=254 keeps it normal (0 = subnormal/zero, 255 = inf/NaN),
        // sign bit clear.
        let exp = 1 + rng.below(254);
        let mant = rng.next_u32() & 0x007F_FFFF;
        bits.push((exp << 23) | mant);
    }
    assert_same("C1 positive normals", &bits);
}

/// Row C2 — negative normal floats. The sign bit makes byte 3 land in
/// `0x80..=0xff`, which is where a sign-extending translation of the
/// `unsigned char` element would print `ffffffXX` instead of `XX`.
#[test]
fn b_c2_negative_normals() {
    let mut rng = Rng::new(0x0000_0002);
    let mut bits = Vec::with_capacity(4096);
    for _ in 0..4096 {
        let exp = 1 + rng.below(254);
        let mant = rng.next_u32() & 0x007F_FFFF;
        bits.push(0x8000_0000 | (exp << 23) | mant);
    }
    assert_same("C2 negative normals", &bits);
}

/// Row C3 — `+0.0` (all bytes zero) and `-0.0` (sign bit only).
#[test]
fn b_c3_zeroes() {
    assert_same("C3 zeroes", &[0x0000_0000, 0x8000_0000]);
}

/// Row C4 — subnormals of both signs: zero exponent, non-zero mantissa.
#[test]
fn b_c4_subnormals() {
    let mut rng = Rng::new(0x0000_0004);
    let mut bits = Vec::with_capacity(4096);
    for _ in 0..4096 {
        let mut mant = rng.next_u32() & 0x007F_FFFF;
        if mant == 0 {
            mant = 1; // keep it subnormal rather than zero
        }
        let sign = (rng.next_u32() & 1) << 31;
        bits.push(sign | mant);
    }
    assert_same("C4 subnormals", &bits);
}

/// Row C5 — both infinities.
#[test]
fn b_c5_infinities() {
    assert_same("C5 infinities", &[0x7F80_0000, 0xFF80_0000]);
}

/// Row C6 — quiet NaNs, both signs, random payloads. The payload must be printed
/// verbatim; nothing in the C code inspects or canonicalises it.
#[test]
fn b_c6_quiet_nans() {
    let mut rng = Rng::new(0x0000_0006);
    let mut bits = Vec::with_capacity(4096);
    for _ in 0..4096 {
        let payload = rng.next_u32() & 0x003F_FFFF; // low 22 bits
        let sign = (rng.next_u32() & 1) << 31;
        // exponent all ones + mantissa MSB set => quiet NaN
        bits.push(sign | 0x7FC0_0000 | payload);
    }
    assert_same("C6 quiet NaNs", &bits);
}

/// Row C7 — signalling NaNs (mantissa MSB clear, remaining mantissa non-zero),
/// both signs, random payloads.
#[test]
fn b_c7_signalling_nans() {
    let mut rng = Rng::new(0x0000_0007);
    let mut bits = Vec::with_capacity(4096);
    for _ in 0..4096 {
        let mut payload = rng.next_u32() & 0x003F_FFFF;
        if payload == 0 {
            payload = 1; // a zero payload with the MSB clear would be infinity
        }
        let sign = (rng.next_u32() & 1) << 31;
        bits.push(sign | 0x7F80_0000 | payload);
    }
    assert_same("C7 signalling NaNs", &bits);
}

/// Row C8 — the IEEE-754 extremes a real caller is most likely to hit.
#[test]
fn b_c8_ieee_extremes() {
    let values: [f32; 12] = [
        f32::MAX,
        f32::MIN,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::EPSILON,
        -f32::EPSILON,
        f32::from_bits(0x0000_0001),  // smallest positive subnormal
        f32::from_bits(0x8000_0001),  // smallest negative subnormal
        f32::from_bits(0x007F_FFFF),  // largest subnormal
        f32::from_bits(0x0080_0000),  // smallest normal
        1.0,
        -1.0,
    ];
    assert_same_floats("C8 IEEE extremes", &values);
}

/// Row C9 — for each of the 4 byte positions, sweep values that place a low
/// nibble-only byte (`0x00..=0x0f`) there. This is what `%02x`'s zero padding
/// depends on: a translation using `{:x}` without width/fill would emit one
/// digit and desynchronise the whole line.
#[test]
fn b_c9_zero_padding_each_position() {
    let mut rng = Rng::new(0x0000_0009);
    let mut bits = Vec::new();
    for pos in 0..4u32 {
        for low in 0x00..=0x0Fu32 {
            // Fill the other three bytes randomly, so the padded byte is checked
            // in many surrounding contexts rather than just one.
            for _ in 0..16 {
                let mut w = rng.next_u32();
                w &= !(0xFF << (8 * pos));
                w |= low << (8 * pos);
                bits.push(w);
            }
        }
    }
    assert_same("C9 %02x zero padding per byte position", &bits);
}

/// Row C10 — for each byte position, sweep the high half (`0x80..=0xff`). This is
/// the zero-extension check: `p[i]` is `unsigned char`, so it widens to a
/// positive `int` and prints as two digits.
#[test]
fn b_c10_high_byte_each_position() {
    let mut rng = Rng::new(0x0000_000A);
    let mut bits = Vec::new();
    for pos in 0..4u32 {
        for hi in 0x80..=0xFFu32 {
            let mut w = rng.next_u32();
            w &= !(0xFF << (8 * pos));
            w |= hi << (8 * pos);
            bits.push(w);
        }
    }
    assert_same("C10 high byte per position", &bits);
}

/// Row C11 — exhaustive per-position sweep: every byte value `0x00..=0xff` at
/// every one of the 4 positions. Together with C12 this covers each byte lane's
/// full domain.
#[test]
fn b_c11_full_per_position_sweep() {
    let mut rng = Rng::new(0x0000_000B);
    let mut bits = Vec::with_capacity(4 * 256);
    for pos in 0..4u32 {
        for byte in 0x00..=0xFFu32 {
            let mut w = rng.next_u32();
            w &= !(0xFF << (8 * pos));
            w |= byte << (8 * pos);
            bits.push(w);
        }
    }
    assert_same("C11 full per-position byte sweep", &bits);
}

/// Row C12 — uniformly random raw bit patterns. Unbiased across all IEEE classes
/// and all byte values simultaneously, which is what catches value-dependent
/// formatting bugs that per-class rows can miss.
#[test]
fn b_c12_random_bit_patterns() {
    let mut rng = Rng::new(0x0000_000C);
    let bits: Vec<u32> = (0..65536).map(|_| rng.next_u32()).collect();
    assert_same("C12 random bit patterns", &bits);
}

/// Row C13 — small integer-valued floats, the ordinary real-world shape.
#[test]
fn b_c13_integral_values() {
    let values: Vec<f32> = (-1024..=1024).map(|i| i as f32).collect();
    assert_same_floats("C13 integral values", &values);
}

/// Row C14 — exponent sweep: every representable power of two, both signs,
/// including the subnormal powers.
#[test]
fn b_c14_exponent_sweep() {
    let mut values = Vec::new();
    for e in -149i32..=127 {
        let v = (2.0f64).powi(e) as f32;
        values.push(v);
        values.push(-v);
    }
    assert_same_floats("C14 exponent sweep", &values);
}

/// Row C15 — many calls in one capture window. Confirms `driver` is stateless and
/// that line framing is identical across a long run (no missing or doubled
/// newline that a single-call test would not reveal).
#[test]
fn b_c15_repeated_calls_sequence() {
    let mut rng = Rng::new(0x0000_000F);
    for _ in 0..8 {
        let bits: Vec<u32> = (0..1024).map(|_| rng.next_u32()).collect();
        assert_same("C15 repeated calls sequence", &bits);
    }
}

/// Row C16 — both `.so`s loaded at once and called alternately on the *same*
/// `stdout` stream, then the two interleaved transcripts compared against each
/// other's expected halves. This checks the Rust library shares the process
/// `stdout` FILE (rather than owning a private buffer) — if it did not, the
/// alternating lines would come out in the wrong order.
#[test]
fn b_c16_interleaved_same_stream() {
    let mut rng = Rng::new(0x0000_0010);
    let bits: Vec<u32> = (0..1024).map(|_| rng.next_u32()).collect();

    let c = common::c_driver();
    let r = common::rust_driver();

    // Alternate C, Rust, C, Rust, ... on the shared stream.
    let interleaved = common::capture_stdout(|| {
        for &b in &bits {
            unsafe {
                c(f32::from_bits(b));
                r(f32::from_bits(b));
            }
        }
    });

    let lines: Vec<&[u8]> = interleaved
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .collect();
    assert_eq!(
        lines.len(),
        bits.len() * 2,
        "expected two lines per iteration on the shared stream"
    );
    for (i, pair) in lines.chunks(2).enumerate() {
        assert_eq!(
            pair[0],
            pair[1],
            "C/Rust divergence on shared stream at iteration {i}, input 0x{:08x}",
            bits[i]
        );
    }
}
