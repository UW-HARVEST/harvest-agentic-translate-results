//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test calls BOTH the C `.so` and the
//! Rust `.so` through `libloading` and compares the returned `int`
//! byte-for-byte. All randomized rows use a fixed seed so failures reproduce.

mod common;

use common::{assert_same, Rng, SAMPLES};

const INT_MIN: i32 = i32::MIN;
const INT_MAX: i32 = i32::MAX;

/// bit pattern of `1.0f`
const F_ONE: u32 = 0x3F80_0000;
/// bit pattern of `1000.0f` — the first value EXCLUDED by `f < 1000.0f`
const F_1000: u32 = 0x447A_0000;
/// bit pattern of `+inf`
const F_PINF: u32 = 0x7F80_0000;

// ---------------------------------------------------------------------------
// Row 1 — degenerate all-zero shape
// ---------------------------------------------------------------------------
#[test]
fn cfg_row01_all_zero() {
    assert_same(0, 0, 0, 0);
}

// ---------------------------------------------------------------------------
// Row 2 — a == 0, single-digit non-negative b, c, d
// ---------------------------------------------------------------------------
#[test]
fn cfg_row02_zero_a_small_bcd() {
    let mut rng = Rng::new(0x0000_0002);
    for _ in 0..SAMPLES {
        let b = rng.range_i32(0, 9);
        let c = rng.range_i32(0, 9);
        let d = rng.range_i32(0, 9);
        assert_same(0, b, c, d);
    }
    // exhaustive over the 10^3 space as well
    for b in 0..10 {
        for c in 0..10 {
            for d in 0..10 {
                assert_same(0, b, c, d);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 3 — a is a float subnormal (1 .. 0xFFFF)
// ---------------------------------------------------------------------------
#[test]
fn cfg_row03_subnormal_a() {
    let mut rng = Rng::new(0x0000_0003);
    for _ in 0..SAMPLES {
        let a = rng.range_u32_as_i32(1, 0xFFFF);
        assert_same(a, rng.next_i32(), rng.next_i32(), rng.next_i32());
    }
    for a in 1..=64 {
        assert_same(a, 1, 2, 3);
    }
}

// ---------------------------------------------------------------------------
// Row 4 — 0 < f < 1  ->  (int)f == 0
// ---------------------------------------------------------------------------
#[test]
fn cfg_row04_float_between_zero_and_one() {
    let mut rng = Rng::new(0x0000_0004);
    for _ in 0..SAMPLES {
        let a = rng.range_u32_as_i32(0x0001_0000, F_ONE - 1);
        assert_same(a, rng.next_i32(), rng.next_i32(), rng.next_i32());
    }
}

// ---------------------------------------------------------------------------
// Row 5 — 1 <= f < 1000  ->  non-zero float contribution
// ---------------------------------------------------------------------------
#[test]
fn cfg_row05_float_one_to_thousand() {
    let mut rng = Rng::new(0x0000_0005);
    for _ in 0..SAMPLES * 4 {
        let a = rng.range_u32_as_i32(F_ONE, F_1000 - 1);
        assert_same(a, rng.next_i32(), rng.next_i32(), rng.next_i32());
    }
}

// ---------------------------------------------------------------------------
// Row 6 — pinned float values inside the accepted window
// ---------------------------------------------------------------------------
#[test]
fn cfg_row06_pinned_float_values() {
    let mut rng = Rng::new(0x0000_0006);
    let floats: [f32; 12] = [
        1.0, 1.5, 1.9999999, 2.0, 9.99, 10.0, 99.5, 100.0, 255.5, 512.0, 999.9375, 999.99994,
    ];
    for f in floats {
        let a = f.to_bits() as i32;
        assert_same(a, 0, 0, 0);
        for _ in 0..32 {
            assert_same(a, rng.next_i32(), rng.next_i32(), rng.next_i32());
        }
    }
}

// ---------------------------------------------------------------------------
// Row 7 — f >= 1000 (finite)
// ---------------------------------------------------------------------------
#[test]
fn cfg_row07_float_ge_thousand() {
    let mut rng = Rng::new(0x0000_0007);
    for _ in 0..SAMPLES {
        let a = rng.range_u32_as_i32(F_1000, F_PINF - 1);
        assert_same(a, rng.next_i32(), rng.next_i32(), rng.next_i32());
    }
}

// ---------------------------------------------------------------------------
// Row 8 — exact 1000.0 boundary pair
// ---------------------------------------------------------------------------
#[test]
fn cfg_row08_thousand_boundary() {
    let mut rng = Rng::new(0x0000_0008);
    for a in [
        (F_1000 - 2) as i32,
        (F_1000 - 1) as i32,
        F_1000 as i32,
        (F_1000 + 1) as i32,
        (F_ONE - 1) as i32,
        F_ONE as i32,
        (F_ONE + 1) as i32,
    ] {
        assert_same(a, 0, 0, 0);
        for _ in 0..64 {
            assert_same(a, rng.next_i32(), rng.next_i32(), rng.next_i32());
        }
    }
}

// ---------------------------------------------------------------------------
// Row 9 — +inf / -inf bit patterns
// ---------------------------------------------------------------------------
#[test]
fn cfg_row09_infinities() {
    let mut rng = Rng::new(0x0000_0009);
    for a in [F_PINF as i32, 0xFF80_0000u32 as i32] {
        assert_same(a, 0, 0, 0);
        for _ in 0..SAMPLES {
            assert_same(a, rng.next_i32(), rng.next_i32(), rng.next_i32());
        }
    }
}

// ---------------------------------------------------------------------------
// Row 10 — NaN bit patterns, both signs, quiet and signalling
// ---------------------------------------------------------------------------
#[test]
fn cfg_row10_nans() {
    let mut rng = Rng::new(0x0000_000A);
    let nans: [u32; 8] = [
        0x7F80_0001,
        0x7FBF_FFFF,
        0x7FC0_0000,
        0x7FFF_FFFF,
        0xFF80_0001,
        0xFFBF_FFFF,
        0xFFC0_0000,
        0xFFFF_FFFF,
    ];
    for bits in nans {
        let a = bits as i32;
        assert_same(a, 0, 0, 0);
        for _ in 0..64 {
            assert_same(a, rng.next_i32(), rng.next_i32(), rng.next_i32());
        }
    }
}

// ---------------------------------------------------------------------------
// Row 11 — a negative (negative float), b, c, d non-negative
// ---------------------------------------------------------------------------
#[test]
fn cfg_row11_negative_a_only() {
    let mut rng = Rng::new(0x0000_000B);
    for _ in 0..SAMPLES {
        let a = rng.range_i32(INT_MIN, -1);
        let b = rng.range_i32(0, INT_MAX);
        let c = rng.range_i32(0, INT_MAX);
        let d = rng.range_i32(0, INT_MAX);
        assert_same(a, b, c, d);
    }
}

// ---------------------------------------------------------------------------
// Row 12 — all 16 sign combinations -> dash_count 3..7
// ---------------------------------------------------------------------------
#[test]
fn cfg_row12_sign_combinations() {
    let mut rng = Rng::new(0x0000_000C);
    for mask in 0u32..16 {
        for _ in 0..SAMPLES / 2 {
            let mut v = [0i32; 4];
            for (k, slot) in v.iter_mut().enumerate() {
                let neg = (mask >> k) & 1 == 1;
                *slot = if neg {
                    rng.range_i32(INT_MIN, -1)
                } else {
                    rng.range_i32(0, INT_MAX)
                };
            }
            assert_same(v[0], v[1], v[2], v[3]);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 13 — uniform decimal-width sweep, positive
// ---------------------------------------------------------------------------
#[test]
fn cfg_row13_width_sweep_positive() {
    let mut rng = Rng::new(0x0000_000D);
    let mut lo: i64 = 0;
    let mut hi: i64 = 9;
    for _width in 1..=10 {
        let h = hi.min(INT_MAX as i64);
        for _ in 0..SAMPLES / 4 {
            let a = rng.range_i64(lo, h) as i32;
            let b = rng.range_i64(lo, h) as i32;
            let c = rng.range_i64(lo, h) as i32;
            let d = rng.range_i64(lo, h) as i32;
            assert_same(a, b, c, d);
        }
        lo = h + 1;
        hi = hi * 10 + 9;
        if lo > INT_MAX as i64 {
            break;
        }
    }
}

// ---------------------------------------------------------------------------
// Row 14 — uniform decimal-width sweep, negative
// ---------------------------------------------------------------------------
#[test]
fn cfg_row14_width_sweep_negative() {
    let mut rng = Rng::new(0x0000_000E);
    let mut lo: i64 = 1;
    let mut hi: i64 = 9;
    for _width in 1..=10 {
        let h = hi.min(-(INT_MIN as i64 + 1));
        for _ in 0..SAMPLES / 4 {
            let a = -(rng.range_i64(lo, h) as i32);
            let b = -(rng.range_i64(lo, h) as i32);
            let c = -(rng.range_i64(lo, h) as i32);
            let d = -(rng.range_i64(lo, h) as i32);
            assert_same(a, b, c, d);
        }
        lo = h + 1;
        hi = hi * 10 + 9;
        if lo > -(INT_MIN as i64 + 1) {
            break;
        }
    }
}

// ---------------------------------------------------------------------------
// Row 15 — mixed widths, maximal strlen asymmetry
// ---------------------------------------------------------------------------
#[test]
fn cfg_row15_mixed_widths() {
    let mut rng = Rng::new(0x0000_000F);
    for _ in 0..SAMPLES {
        let a = rng.range_i32(0, 9);
        let b = rng.range_i32(1_000_000_000, INT_MAX);
        let c = -rng.range_i32(10_000, 99_999);
        let d = rng.range_i32(10, 99);
        assert_same(a, b, c, d);
    }
    assert_same(7, 2_000_000_000, -12345, 42);
}

// ---------------------------------------------------------------------------
// Row 16 — INT_MIN everywhere: longest possible snprintf output (51 bytes)
// ---------------------------------------------------------------------------
#[test]
fn cfg_row16_int_min_all() {
    assert_same(INT_MIN, INT_MIN, INT_MIN, INT_MIN);
    assert_same(INT_MIN, 0, 0, 0);
    assert_same(0, INT_MIN, 0, 0);
    assert_same(0, 0, INT_MIN, 0);
    assert_same(0, 0, 0, INT_MIN);
    assert_same(INT_MIN + 1, INT_MIN + 1, INT_MIN + 1, INT_MIN + 1);
}

// ---------------------------------------------------------------------------
// Row 17 — INT_MAX everywhere and in each position
// ---------------------------------------------------------------------------
#[test]
fn cfg_row17_int_max() {
    assert_same(INT_MAX, INT_MAX, INT_MAX, INT_MAX);
    assert_same(INT_MAX, 0, 0, 0);
    assert_same(0, INT_MAX, 0, 0);
    assert_same(0, 0, INT_MAX, 0);
    assert_same(0, 0, 0, INT_MAX);
    assert_same(INT_MAX - 1, INT_MAX, INT_MIN, 0);
}

// ---------------------------------------------------------------------------
// Row 18 — low byte of b == 0 (interpret_as_int LSB zero)
// ---------------------------------------------------------------------------
#[test]
fn cfg_row18_b_low_byte_zero() {
    let mut rng = Rng::new(0x0000_0012);
    for _ in 0..SAMPLES {
        let a = rng.next_i32();
        let b = (rng.next_u32() & 0xFFFF_FF00) as i32;
        let c = (rng.next_u32() | 0x1) as i32;
        let d = (rng.next_u32() | 0x1) as i32;
        assert_same(a, b, c, d);
    }
}

// ---------------------------------------------------------------------------
// Row 19 — all three low bytes zero -> interpret_as_int == 0
// ---------------------------------------------------------------------------
#[test]
fn cfg_row19_all_low_bytes_zero() {
    let mut rng = Rng::new(0x0000_0013);
    for _ in 0..SAMPLES {
        let a = rng.next_i32();
        let b = (rng.next_u32() & 0xFFFF_FF00) as i32;
        let c = (rng.next_u32() & 0xFFFF_FF00) as i32;
        let d = (rng.next_u32() & 0xFFFF_FF00) as i32;
        assert_same(a, b, c, d);
    }
    assert_same(0, 0x100, 0x200, 0x300);
}

// ---------------------------------------------------------------------------
// Row 20 — all three low bytes 0xFF -> interpret_as_int == 0x00FFFFFF
// ---------------------------------------------------------------------------
#[test]
fn cfg_row20_all_low_bytes_ff() {
    let mut rng = Rng::new(0x0000_0014);
    for _ in 0..SAMPLES {
        let a = rng.next_i32();
        let b = (rng.next_u32() | 0xFF) as i32;
        let c = (rng.next_u32() | 0xFF) as i32;
        let d = (rng.next_u32() | 0xFF) as i32;
        assert_same(a, b, c, d);
    }
    assert_same(0, 0xFF, 0xFF, 0xFF);
    assert_same(0, -1, -1, -1);
}

// ---------------------------------------------------------------------------
// Row 21 — sweep of (b&0xFF, c&0xFF, d&0xFF) with randomized high bits
// ---------------------------------------------------------------------------
#[test]
fn cfg_row21_low_byte_sweep() {
    let mut rng = Rng::new(0x0000_0015);
    for lb in 0u32..256 {
        let hi_b = rng.next_u32() & 0xFFFF_FF00;
        let hi_c = rng.next_u32() & 0xFFFF_FF00;
        let hi_d = rng.next_u32() & 0xFFFF_FF00;
        let a = rng.next_i32();
        assert_same(
            a,
            (hi_b | lb) as i32,
            (hi_c | (255 - lb)) as i32,
            (hi_d | lb.rotate_left(3) & 0xFF) as i32,
        );
        // and a purely-low-byte variant
        assert_same(0, lb as i32, (255 - lb) as i32, lb as i32);
    }
}

// ---------------------------------------------------------------------------
// Row 22 — complex_iteration XOR fold == 0
// ---------------------------------------------------------------------------
#[test]
fn cfg_row22_xor_fold_zero() {
    let mut rng = Rng::new(0x0000_0016);
    for _ in 0..SAMPLES {
        // pick low bytes so that x_a ^ x_b ^ x_c ^ x_d == 0
        let la = rng.next_u32() & 0xFF;
        let lb = rng.next_u32() & 0xFF;
        let lc = rng.next_u32() & 0xFF;
        let ld = la ^ lb ^ lc;
        let a = ((rng.next_u32() & 0xFFFF_FF00) | la) as i32;
        let b = ((rng.next_u32() & 0xFFFF_FF00) | lb) as i32;
        let c = ((rng.next_u32() & 0xFFFF_FF00) | lc) as i32;
        let d = ((rng.next_u32() & 0xFFFF_FF00) | ld) as i32;
        assert_same(a, b, c, d);
    }
}

// ---------------------------------------------------------------------------
// Row 23 — complex_iteration XOR fold == 0xFF
// ---------------------------------------------------------------------------
#[test]
fn cfg_row23_xor_fold_ff() {
    let mut rng = Rng::new(0x0000_0017);
    for _ in 0..SAMPLES {
        let la = rng.next_u32() & 0xFF;
        let lb = rng.next_u32() & 0xFF;
        let lc = rng.next_u32() & 0xFF;
        let ld = la ^ lb ^ lc ^ 0xFF;
        let a = ((rng.next_u32() & 0xFFFF_FF00) | la) as i32;
        let b = ((rng.next_u32() & 0xFFFF_FF00) | lb) as i32;
        let c = ((rng.next_u32() & 0xFFFF_FF00) | lc) as i32;
        let d = ((rng.next_u32() & 0xFFFF_FF00) | ld) as i32;
        assert_same(a, b, c, d);
    }
}

// ---------------------------------------------------------------------------
// Row 24 — positive overflow of safe_sum_array
// ---------------------------------------------------------------------------
#[test]
fn cfg_row24_sum_overflow_positive() {
    let mut rng = Rng::new(0x0000_0018);
    for _ in 0..SAMPLES {
        let a = rng.range_i32(INT_MAX - 1000, INT_MAX);
        let b = rng.range_i32(INT_MAX - 1000, INT_MAX);
        let c = rng.range_i32(INT_MAX - 1000, INT_MAX);
        let d = rng.range_i32(INT_MAX - 1000, INT_MAX);
        assert_same(a, b, c, d);
    }
    assert_same(INT_MAX, INT_MAX, INT_MAX, INT_MAX);
    assert_same(INT_MAX, 1, 0, 0);
}

// ---------------------------------------------------------------------------
// Row 25 — negative overflow of safe_sum_array
// ---------------------------------------------------------------------------
#[test]
fn cfg_row25_sum_overflow_negative() {
    let mut rng = Rng::new(0x0000_0019);
    for _ in 0..SAMPLES {
        let a = rng.range_i32(INT_MIN, INT_MIN + 1000);
        let b = rng.range_i32(INT_MIN, INT_MIN + 1000);
        let c = rng.range_i32(INT_MIN, INT_MIN + 1000);
        let d = rng.range_i32(INT_MIN, INT_MIN + 1000);
        assert_same(a, b, c, d);
    }
    assert_same(INT_MIN, -1, 0, 0);
}

// ---------------------------------------------------------------------------
// Row 26 — shortest vs longest buffer (buf_sum extremes)
// ---------------------------------------------------------------------------
#[test]
fn cfg_row26_buffer_length_extremes() {
    // shortest: "test0-0-0-0" = 11 bytes
    assert_same(0, 0, 0, 0);
    // longest: "test-2147483648-...-..." = 51 bytes
    assert_same(INT_MIN, INT_MIN, INT_MIN, INT_MIN);
    // near-longest with mixed widths
    assert_same(INT_MIN, INT_MAX, INT_MIN, INT_MAX);
    assert_same(-999999999, -999999999, -999999999, -999999999);
}

// ---------------------------------------------------------------------------
// Row 27 — search for inputs where buf_sum % 256 == 0
// ---------------------------------------------------------------------------
#[test]
fn cfg_row27_bufsum_residue_zero() {
    fn buf_sum_mod_256(a: i32, b: i32, c: i32, d: i32) -> i32 {
        let s = format!("test{}-{}-{}-{}", a, b, c, d);
        let sum: i32 = s.bytes().map(|x| x as i8 as i32).sum();
        sum % 256
    }
    let mut rng = Rng::new(0x0000_001B);
    let mut hits = 0usize;
    let mut tried = 0usize;
    while hits < 64 && tried < 2_000_000 {
        tried += 1;
        let a = rng.next_i32();
        let b = rng.next_i32();
        let c = rng.next_i32();
        let d = rng.next_i32();
        if buf_sum_mod_256(a, b, c, d) == 0 {
            hits += 1;
            assert_same(a, b, c, d);
        }
    }
    assert!(hits > 0, "expected to find inputs with buf_sum % 256 == 0");
}

// ---------------------------------------------------------------------------
// Row 28 — full unconstrained random sweep
// ---------------------------------------------------------------------------
#[test]
fn cfg_row28_full_random() {
    let mut rng = Rng::new(0xDEAD_BEEF_CAFE_1234);
    for _ in 0..100_000 {
        assert_same(
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
        );
    }
}

// ---------------------------------------------------------------------------
// Row 29 — exhaustive boundary-value cross-product (13^4)
// ---------------------------------------------------------------------------
#[test]
fn cfg_row29_boundary_cross_product() {
    let vals: [i32; 13] = [
        INT_MIN,
        INT_MIN + 1,
        -256,
        -255,
        -1,
        0,
        1,
        255,
        256,
        F_ONE as i32,
        F_1000 as i32,
        F_PINF as i32,
        INT_MAX,
    ];
    for &a in &vals {
        for &b in &vals {
            for &c in &vals {
                for &d in &vals {
                    assert_same(a, b, c, d);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 30 — statelessness / repeated calls
// ---------------------------------------------------------------------------
#[test]
fn cfg_row30_statelessness() {
    let mut rng = Rng::new(0x0000_001E);
    let tuples: Vec<(i32, i32, i32, i32)> = (0..64)
        .map(|_| {
            (
                rng.next_i32(),
                rng.next_i32(),
                rng.next_i32(),
                rng.next_i32(),
            )
        })
        .collect();
    let mut first = Vec::new();
    for &(a, b, c, d) in &tuples {
        first.push(assert_same(a, b, c, d));
    }
    for _round in 0..3 {
        for (idx, &(a, b, c, d)) in tuples.iter().enumerate() {
            // interleave an unrelated call to perturb any hidden state
            assert_same(idx as i32, 1, 2, 3);
            let v = assert_same(a, b, c, d);
            assert_eq!(v, first[idx], "memchra2 is not stateless for {a},{b},{c},{d}");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 31 — sweep a over every float exponent boundary
// ---------------------------------------------------------------------------
#[test]
fn cfg_row31_float_exponent_boundaries() {
    let mut rng = Rng::new(0x0000_001F);
    for k in 0u32..256 {
        for sign in [0u32, 0x8000_0000] {
            let bits = sign | (k << 23);
            for delta in [0u32, 1, 0x7F_FFFF] {
                let a = bits.wrapping_add(delta) as i32;
                assert_same(a, rng.next_i32(), rng.next_i32(), rng.next_i32());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 32 — digit-content sweep (all-nines)
// ---------------------------------------------------------------------------
#[test]
fn cfg_row32_all_nines() {
    let nines: [i32; 9] = [
        9, 99, 999, 9999, 99999, 999999, 9999999, 99999999, 999999999,
    ];
    for &v in &nines {
        assert_same(v, v, v, v);
        assert_same(-v, -v, -v, -v);
        assert_same(v, -v, v, -v);
    }
    for &a in &nines {
        for &b in &nines {
            assert_same(a, b, -a, -b);
        }
    }
}
