//! Phase B — valid-path differential tests, one per row of `CONFIGS.md`.
//!
//! Every call goes through `libloading` into the C `.so` and the Rust `.so`;
//! the Rust crate is never called directly.

mod common;

use common::*;
use std::ffi::c_int;

/// Rows 1–56: IEEE-754 class of `a` × sign pattern of `(b, c, d)`.
///
/// `row_base` is the 1-based `CONFIGS.md` row number of the first (`+++`) sign
/// pattern for this class.
fn run_axis1_x_axis2(class: AClass, row_base: usize) {
    let p = Pair::load();
    for (k, &signs) in BCD_SIGNS.iter().enumerate() {
        let row = row_base + k;
        let label = format!("row{row} {} {}", class.name(), sign_label(signs));
        let mut rng = Rng::new(0x5EED_0000 + row as u64);

        // Randomized inputs for this row.
        for _ in 0..ITERS_PER_ROW {
            let a = class.sample(&mut rng);
            let b = sample_signed(&mut rng, signs.0);
            let c = sample_signed(&mut rng, signs.1);
            let d = sample_signed(&mut rng, signs.2);
            p.assert_same(&label, a, b, c, d);
        }

        // Exhaustive boundary representatives of the `a` class, crossed with
        // sign-correct extremes of b, c, d.
        let bcd_extremes: [c_int; 5] = [0, 1, 2, i32::MAX - 1, i32::MAX];
        let bcd_neg_extremes: [c_int; 5] = [-1, -2, -255, i32::MIN + 1, i32::MIN];
        let pick = |neg: bool| -> [c_int; 5] {
            if neg {
                bcd_neg_extremes
            } else {
                bcd_extremes
            }
        };
        for a in class.boundaries() {
            for &b in pick(signs.0).iter() {
                for &c in pick(signs.1).iter() {
                    for &d in pick(signs.2).iter() {
                        p.assert_same(&label, a, b, c, d);
                    }
                }
            }
        }
    }
}

#[test]
fn cfg_rows_01_08_a_zero() {
    run_axis1_x_axis2(AClass::Zero, 1);
}

#[test]
fn cfg_rows_09_16_a_pos_subnormal() {
    run_axis1_x_axis2(AClass::PosSubnormal, 9);
}

#[test]
fn cfg_rows_17_24_a_pos_norm_lt_one() {
    run_axis1_x_axis2(AClass::PosNormLtOne, 17);
}

#[test]
fn cfg_rows_25_32_a_pos_norm_in_range() {
    run_axis1_x_axis2(AClass::PosNormInRange, 25);
}

#[test]
fn cfg_rows_33_40_a_pos_ge_thousand() {
    run_axis1_x_axis2(AClass::PosGeThousand, 33);
}

#[test]
fn cfg_rows_41_48_a_pos_inf_nan() {
    run_axis1_x_axis2(AClass::PosInfNan, 41);
}

#[test]
fn cfg_rows_49_56_a_negative() {
    run_axis1_x_axis2(AClass::Negative, 49);
}

// ---------------------------------------------------------------------------
// Axis 3 — low-byte shapes feeding interpret_as_int / complex_iteration.
// ---------------------------------------------------------------------------

/// Replaces the low byte of `v` with `byte`, preserving the upper 24 bits.
fn with_low_byte(v: c_int, byte: u8) -> c_int {
    (((v as u32) & 0xFFFF_FF00) | byte as u32) as i32
}

#[test]
fn cfg_row57_low_bytes_all_zero() {
    let p = Pair::load();
    let mut rng = Rng::new(0x5EED_0000 + 57);
    for _ in 0..ITERS_PER_ROW * 2 {
        let a = rng.next_i32();
        let b = with_low_byte(rng.next_i32(), 0x00);
        let c = with_low_byte(rng.next_i32(), 0x00);
        let d = with_low_byte(rng.next_i32(), 0x00);
        p.assert_same("row57", a, b, c, d);
    }
    // Exact zeros too.
    for &a in INTERESTING.iter() {
        p.assert_same("row57", a, 0, 0, 0);
        p.assert_same("row57", a, 0x100, 0x200, 0x300);
        p.assert_same("row57", a, -256, -512, -768);
    }
}

#[test]
fn cfg_row58_low_bytes_all_ff() {
    let p = Pair::load();
    let mut rng = Rng::new(0x5EED_0000 + 58);
    for _ in 0..ITERS_PER_ROW * 2 {
        let a = rng.next_i32();
        let b = with_low_byte(rng.next_i32(), 0xFF);
        let c = with_low_byte(rng.next_i32(), 0xFF);
        let d = with_low_byte(rng.next_i32(), 0xFF);
        p.assert_same("row58", a, b, c, d);
    }
    for &a in INTERESTING.iter() {
        p.assert_same("row58", a, 255, 255, 255);
        p.assert_same("row58", a, -1, -1, -1);
    }
}

#[test]
fn cfg_row59_low_bytes_xor_cancel() {
    let p = Pair::load();
    let mut rng = Rng::new(0x5EED_0000 + 59);
    for _ in 0..ITERS_PER_ROW * 2 {
        let a = rng.next_i32();
        let byte = (rng.next_u32() & 0xFF) as u8;
        // b and c share a low byte so their XOR contributions cancel; d is free.
        let b = with_low_byte(rng.next_i32(), byte);
        let c = with_low_byte(rng.next_i32(), byte);
        let d = rng.next_i32();
        p.assert_same("row59", a, b, c, d);
        // All four sharing the same low byte: full cancellation.
        let a2 = with_low_byte(a, byte);
        let d2 = with_low_byte(d, byte);
        p.assert_same("row59", a2, b, c, d2);
    }
}

#[test]
fn cfg_row60_low_bytes_char_sign_boundary() {
    let p = Pair::load();
    let bytes: [u8; 6] = [0x00, 0x01, 0x7E, 0x7F, 0x80, 0xFF];
    let mut rng = Rng::new(0x5EED_0000 + 60);
    for &bb in bytes.iter() {
        for &cb in bytes.iter() {
            for &db in bytes.iter() {
                for _ in 0..8 {
                    let a = rng.next_i32();
                    let b = with_low_byte(rng.next_i32(), bb);
                    let c = with_low_byte(rng.next_i32(), cb);
                    let d = with_low_byte(rng.next_i32(), db);
                    p.assert_same("row60", a, b, c, d);
                }
                p.assert_same("row60", 0, bb as i32, cb as i32, db as i32);
            }
        }
    }
}

#[test]
fn cfg_row61_a_low_byte_full_sweep() {
    let p = Pair::load();
    let fixed: [(c_int, c_int, c_int); 4] = [
        (0, 0, 0),
        (1, 2, 3),
        (-1, -2, -3),
        (i32::MIN, i32::MAX, -1),
    ];
    for byte in 0u16..256 {
        for &(b, c, d) in fixed.iter() {
            // Sweep the low byte with several distinct upper-24-bit patterns so
            // both the float branch and the XOR fold see different inputs.
            for &upper in [0x0000_0000u32, 0x3F80_0000, 0x4479_0000, 0xFFFF_FF00].iter() {
                let a = ((upper & 0xFFFF_FF00) | byte as u32) as i32;
                p.assert_same("row61", a, b, c, d);
            }
        }
    }
}

#[test]
fn cfg_row62_bcd_low_byte_sweep() {
    let p = Pair::load();
    let mut rng = Rng::new(0x5EED_0000 + 62);
    // Diagonal: all three low bytes identical, all 256 values.
    for byte in 0u16..256 {
        let bb = byte as u8;
        let a = 0x3F80_0000;
        p.assert_same(
            "row62-diag",
            a,
            with_low_byte(0x1000, bb),
            with_low_byte(0x2000, bb),
            with_low_byte(0x3000, bb),
        );
    }
    // Random triples of low bytes over the full 256^3 space (pruned sample).
    for _ in 0..4096 {
        let a = rng.next_i32();
        let b = with_low_byte(rng.next_i32(), (rng.next_u32() & 0xFF) as u8);
        let c = with_low_byte(rng.next_i32(), (rng.next_u32() & 0xFF) as u8);
        let d = with_low_byte(rng.next_i32(), (rng.next_u32() & 0xFF) as u8);
        p.assert_same("row62-rand", a, b, c, d);
    }
}

// ---------------------------------------------------------------------------
// Axis 4 — magnitude shapes: buffer length and sum overflow.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row63_single_digit_exhaustive() {
    let p = Pair::load();
    // Exhaustive 10^4 cross product: shortest possible formatted buffer.
    for a in 0..10 {
        for b in 0..10 {
            for c in 0..10 {
                for d in 0..10 {
                    p.assert_same("row63", a, b, c, d);
                }
            }
        }
    }
    // And the single-negative-digit variants (extra '-' characters).
    for a in -9..10 {
        for b in -9..10 {
            for c in -9..10 {
                for d in -9..10 {
                    p.assert_same("row63-neg", a, b, c, d);
                }
            }
        }
    }
}

#[test]
fn cfg_row64_all_int_min_longest_buffer() {
    let p = Pair::load();
    // "test-2147483648--2147483648--2147483648--2147483648" == 51 bytes.
    p.assert_same("row64", i32::MIN, i32::MIN, i32::MIN, i32::MIN);
    // Every position individually at INT_MIN with the rest also long+negative.
    let longs: [c_int; 4] = [i32::MIN, i32::MIN + 1, -1_000_000_000, -2_000_000_000];
    for &a in longs.iter() {
        for &b in longs.iter() {
            for &c in longs.iter() {
                for &d in longs.iter() {
                    p.assert_same("row64", a, b, c, d);
                }
            }
        }
    }
}

#[test]
fn cfg_row65_all_int_max() {
    let p = Pair::load();
    p.assert_same("row65", i32::MAX, i32::MAX, i32::MAX, i32::MAX);
    let longs: [c_int; 4] = [i32::MAX, i32::MAX - 1, 1_000_000_000, 2_000_000_000];
    for &a in longs.iter() {
        for &b in longs.iter() {
            for &c in longs.iter() {
                for &d in longs.iter() {
                    p.assert_same("row65", a, b, c, d);
                }
            }
        }
    }
}

#[test]
fn cfg_row66_sum_overflow_positive() {
    let p = Pair::load();
    let mut rng = Rng::new(0x5EED_0000 + 66);
    for _ in 0..ITERS_PER_ROW * 2 {
        // Four large positives: a+b+c+d overflows int (C wraps at -O0).
        let a = (rng.range_u32(0x4000_0000, 0x7FFF_FFFF)) as i32;
        let b = (rng.range_u32(0x4000_0000, 0x7FFF_FFFF)) as i32;
        let c = (rng.range_u32(0x4000_0000, 0x7FFF_FFFF)) as i32;
        let d = (rng.range_u32(0x4000_0000, 0x7FFF_FFFF)) as i32;
        assert!(
            (a as i64 + b as i64 + c as i64 + d as i64) > i32::MAX as i64,
            "row66 precondition"
        );
        p.assert_same("row66", a, b, c, d);
    }
    p.assert_same("row66", i32::MAX, i32::MAX, i32::MAX, i32::MAX);
    p.assert_same("row66", i32::MAX, 1, 0, 0);
    p.assert_same("row66", i32::MAX, i32::MAX, 0, 0);
}

#[test]
fn cfg_row67_sum_overflow_negative() {
    let p = Pair::load();
    let mut rng = Rng::new(0x5EED_0000 + 67);
    for _ in 0..ITERS_PER_ROW * 2 {
        // Four large negatives: a+b+c+d underflows int.
        let a = (rng.range_u32(0x8000_0000, 0xC000_0000)) as i32;
        let b = (rng.range_u32(0x8000_0000, 0xC000_0000)) as i32;
        let c = (rng.range_u32(0x8000_0000, 0xC000_0000)) as i32;
        let d = (rng.range_u32(0x8000_0000, 0xC000_0000)) as i32;
        assert!(
            (a as i64 + b as i64 + c as i64 + d as i64) < i32::MIN as i64,
            "row67 precondition"
        );
        p.assert_same("row67", a, b, c, d);
    }
    p.assert_same("row67", i32::MIN, i32::MIN, i32::MIN, i32::MIN);
    p.assert_same("row67", i32::MIN, -1, 0, 0);
}

#[test]
fn cfg_row68_int_f_hits_every_value_1_to_999() {
    let p = Pair::load();
    let mut rng = Rng::new(0x5EED_0000 + 68);
    for k in 1..1000u32 {
        let f = k as f32;
        // Exact integer, the ulp below it, and a fractional value above it —
        // all three must land on the same `(int)f` in C and Rust.
        let probes: [i32; 4] = [
            f.to_bits() as i32,
            (f.to_bits() - 1) as i32,
            (f + 0.5).to_bits() as i32,
            (f + 0.9999).to_bits() as i32,
        ];
        for &a in probes.iter() {
            p.assert_same("row68", a, 0, 0, 0);
            let b = rng.next_i32();
            let c = rng.next_i32();
            let d = rng.next_i32();
            p.assert_same("row68", a, b, c, d);
        }
    }
}
