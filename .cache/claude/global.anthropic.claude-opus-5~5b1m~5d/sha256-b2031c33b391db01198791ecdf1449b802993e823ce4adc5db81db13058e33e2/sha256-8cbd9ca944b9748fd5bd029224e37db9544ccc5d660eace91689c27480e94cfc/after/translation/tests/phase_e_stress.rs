//! Extra stress: high-volume randomized and structured exhaustive sweeps.
//! Guards against value-dependent divergences that per-row sampling can miss.

mod common;
use common::{check, Rng};

/// 20 million uniform random tuples, fixed seed.
#[test]
fn stress_uniform_20m() {
    let mut r = Rng::new(0xA5A5_5A5A_1234_9876);
    for _ in 0..20_000_000 {
        check(r.next_i32(), r.next_i32(), r.next_i32(), r.next_i32());
    }
}

/// Exhaustive sweep of the low 24 bits of `a` (subnormals + small normals +
/// every mantissa pattern), with the other args fixed.
#[test]
fn stress_exhaustive_a_low24() {
    for a in 0..(1i64 << 24) {
        check(a as i32, 1, 2, 3);
    }
}

/// Exhaustive sweep of every float exponent field of `a`, both signs, with a
/// spread of mantissas — covers all IEEE-754 exponent classes of the type pun.
#[test]
fn stress_exhaustive_a_exponents() {
    let mantissas: [u32; 9] = [
        0,
        1,
        0x0000_00FF,
        0x0000_FFFF,
        0x0040_0000,
        0x0055_5555,
        0x007F_FFFE,
        0x007F_FFFF,
        0x002A_AAAA,
    ];
    for exp in 0u32..256 {
        for &m in &mantissas {
            for sign in [0u32, 0x8000_0000] {
                let a = (sign | (exp << 23) | m) as i32;
                check(a, 1, 2, 3);
                check(a, 0, 0, 0);
                check(a, -1, 0xFF, i32::MIN);
                check(a, i32::MAX, i32::MIN, -1);
            }
        }
    }
}

/// Exhaustive over all 2^24 combinations of the low bytes of b, c, d (the exact
/// bytes `interpret_as_int` reinterprets) with `a` cycling through classes.
#[test]
fn stress_exhaustive_low_bytes_bcd() {
    let a_classes: [i32; 4] = [
        0,
        123.456f32.to_bits() as i32,
        0x7FC0_0000u32 as i32,
        i32::MIN,
    ];
    for b in 0..256i32 {
        for c in 0..256i32 {
            for d in 0..256i32 {
                let a = a_classes[((b ^ c ^ d) & 3) as usize];
                check(a, b, c, d);
            }
        }
    }
}

/// Sweep every decimal magnitude of every argument independently, densely.
#[test]
fn stress_decimal_magnitudes() {
    let mut r = Rng::new(0x3333_7777);
    // For each power of ten, sample many values in that decade, both signs.
    let mut lo: i64 = 1;
    while lo <= i32::MAX as i64 {
        let hi = ((lo * 10) - 1).min(i32::MAX as i64);
        fn signed(r: &mut Rng, v: i32) -> i32 {
            if r.next_u64() & 1 == 0 {
                v
            } else {
                -v
            }
        }
        for _ in 0..40_000 {
            let m0 = r.i32_in(lo as i32, hi as i32);
            let a = signed(&mut r, m0);
            let m1 = r.i32_in(lo as i32, hi as i32);
            let b = signed(&mut r, m1);
            let m2 = r.i32_in(lo as i32, hi as i32);
            let c = signed(&mut r, m2);
            let m3 = r.i32_in(lo as i32, hi as i32);
            let d = signed(&mut r, m3);
            check(a, b, c, d);
            // also mix decades across positions
            let x = r.next_i32();
            let y = r.next_i32();
            check(a, x, c, y);
        }
        lo *= 10;
    }
}
