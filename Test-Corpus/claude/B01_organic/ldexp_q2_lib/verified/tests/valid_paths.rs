//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every row drives BOTH shared objects
//! (C and Rust) through their exported `ldexp_q2` symbol and compares the
//! returned `float` bit-for-bit, over many randomized inputs with a fixed seed.

mod common;

use common::{check, exp_boundary_panel, y_panel, Rng, SEED};

/// C1 — `exp_q2 == 0`: one iteration, `e&3 == 0`, `cnt == 0`, `product == 1.0`.
#[test]
fn c01_exp_zero_all_y_classes() {
    let mut rng = Rng::new(SEED ^ 1);
    for y in y_panel() {
        check("C1", y, 0);
    }
    for _ in 0..4096 {
        check("C1", rng.any_f32(), 0);
    }
}

/// C2 — `exp_q2 ∈ {1,2,3}`: the three non-unit table entries, `cnt == 0`.
#[test]
fn c02_exp_one_two_three() {
    let mut rng = Rng::new(SEED ^ 2);
    for e in 1..=3 {
        for y in y_panel() {
            check("C2", y, e);
        }
        for _ in 0..4096 {
            check("C2", rng.any_f32(), e);
        }
    }
}

/// C3 — `exp_q2 ∈ [4,115]`: one iteration, `cnt ∈ [1,28]`, all four residues.
#[test]
fn c03_single_iteration_mid_range() {
    let mut rng = Rng::new(SEED ^ 3);
    for e in 4..=115 {
        for y in y_panel() {
            check("C3", y, e);
        }
    }
    for _ in 0..40_000 {
        let e = rng.range_i32(4, 115);
        check("C3", rng.any_f32(), e);
        check("C3", rng.normal_f32(), e);
    }
}

/// C4 — `exp_q2 ∈ [116,119]`: `cnt == 29`, still on the `exp_q2 < 120` side.
#[test]
fn c04_shift_count_29() {
    let mut rng = Rng::new(SEED ^ 4);
    for e in 116..=119 {
        for y in y_panel() {
            check("C4", y, e);
        }
        for _ in 0..4096 {
            check("C4", rng.any_f32(), e);
        }
    }
}

/// C5 — `exp_q2 == 120`: clamp boundary (`>` not `>=`), `cnt == 30`, `shifted == 1`.
#[test]
fn c05_clamp_boundary_120() {
    let mut rng = Rng::new(SEED ^ 5);
    for y in y_panel() {
        check("C5", y, 119);
        check("C5", y, 120);
        check("C5", y, 121);
    }
    for _ in 0..8192 {
        let y = rng.any_f32();
        check("C5", y, 119);
        check("C5", y, 120);
        check("C5", y, 121);
    }
}

/// C6 — `exp_q2 ∈ [121,240]`: exactly two loop iterations.
#[test]
fn c06_two_iterations() {
    let mut rng = Rng::new(SEED ^ 6);
    for e in 121..=240 {
        for y in y_panel() {
            check("C6", y, e);
        }
    }
    for _ in 0..40_000 {
        let e = rng.range_i32(121, 240);
        check("C6", rng.any_f32(), e);
    }
}

/// C7 — `exp_q2 == 240` vs `241`: two vs three iterations.
#[test]
fn c07_two_vs_three_iterations() {
    let mut rng = Rng::new(SEED ^ 7);
    for e in [239, 240, 241, 242, 360, 361] {
        for y in y_panel() {
            check("C7", y, e);
        }
        for _ in 0..2048 {
            check("C7", rng.any_f32(), e);
        }
    }
}

/// C8 — `exp_q2 ∈ [242,20000]`: 3..167 iterations, accumulating underflow.
#[test]
fn c08_many_iterations() {
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..20_000 {
        let e = rng.range_i32(242, 20_000);
        check("C8", rng.any_f32(), e);
    }
    for y in y_panel() {
        for _ in 0..64 {
            let e = rng.range_i32(242, 20_000);
            check("C8", y, e);
        }
    }
}

/// C9 — `exp_q2 ∈ [-4,-1]`: negative shift count masks to 31 → `product == 0`.
#[test]
fn c09_negative_shift_product_zero() {
    let mut rng = Rng::new(SEED ^ 9);
    for e in -4..=-1 {
        for y in y_panel() {
            check("C9", y, e);
        }
        for _ in 0..4096 {
            check("C9", rng.any_f32(), e);
        }
    }
}

/// C10 — `exp_q2 ∈ [-124,-5]`: negative shift regime, `cnt ∈ [1,30]`.
#[test]
fn c10_negative_shift_amplifying() {
    let mut rng = Rng::new(SEED ^ 10);
    for e in -124..=-5 {
        for y in y_panel() {
            check("C10", y, e);
        }
    }
    for _ in 0..40_000 {
        let e = rng.range_i32(-124, -5);
        check("C10", rng.any_f32(), e);
        check("C10", rng.normal_f32(), e);
    }
}

/// C11 — `exp_q2 ∈ [-128,-125]`: `cnt` wraps to 0 → `shifted == 2^30`.
#[test]
fn c11_shift_wrap_to_zero_count() {
    let mut rng = Rng::new(SEED ^ 11);
    for e in -128..=-125 {
        for y in y_panel() {
            check("C11", y, e);
        }
        for _ in 0..4096 {
            check("C11", rng.any_f32(), e);
        }
    }
}

/// C12 — `exp_q2 ∈ [-132,-129]`: second mod-32 period, `cnt == 31` again.
#[test]
fn c12_second_period_product_zero() {
    let mut rng = Rng::new(SEED ^ 12);
    for e in -132..=-129 {
        for y in y_panel() {
            check("C12", y, e);
        }
        for _ in 0..4096 {
            check("C12", rng.any_f32(), e);
        }
    }
}

/// C13 — exhaustive `exp_q2 ∈ [-4096,4096]` × the full `y` class panel.
#[test]
fn c13_exhaustive_exponent_sweep() {
    let panel = y_panel();
    for e in -4096..=4096i32 {
        for &y in &panel {
            check("C13", y, e);
        }
    }
}

/// C14 — deeply negative `exp_q2`, far past one mod-128 period.
#[test]
fn c14_deeply_negative() {
    let mut rng = Rng::new(SEED ^ 14);
    for _ in 0..60_000 {
        let e = rng.range_i32(-2_000_000, -1);
        check("C14", rng.any_f32(), e);
    }
    let panel = y_panel();
    for _ in 0..512 {
        let e = rng.range_i32(i32::MIN, -1);
        for &y in &panel {
            check("C14", y, e);
        }
    }
}

/// C15 — `INT_MIN` and its neighbourhood (extreme negative residues/wraps).
#[test]
fn c15_int_min_neighbourhood() {
    let mut rng = Rng::new(SEED ^ 15);
    let mut exps: Vec<i32> = (0..=8).map(|k| i32::MIN + k).collect();
    exps.extend((120..=132).map(|k| i32::MIN + k));
    let panel = y_panel();
    for &e in &exps {
        for &y in &panel {
            check("C15", y, e);
        }
        for _ in 0..1024 {
            check("C15", rng.any_f32(), e);
        }
    }
}

/// C16 — `INT_MAX` neighbourhood: maximum loop trip count (~1.8e7 iterations).
#[test]
fn c16_int_max_neighbourhood() {
    let ys = [
        1.0f32,
        -1.0f32,
        f32::from_bits(0x7F80_0000), // +inf
        f32::from_bits(0xFF80_0000), // -inf
        f32::from_bits(0x7FC0_0001), // qNaN
        f32::from_bits(0x8000_0000), // -0
    ];
    for e in [i32::MAX, i32::MAX - 1, i32::MAX - 119, i32::MAX - 120] {
        for &y in &ys {
            check("C16", y, e);
        }
    }
}

/// C17 — signed zeros through 1..N multiplications.
#[test]
fn c17_signed_zero_preservation() {
    for y in [f32::from_bits(0x0000_0000), f32::from_bits(0x8000_0000)] {
        for e in exp_boundary_panel() {
            check("C17", y, e);
        }
    }
}

/// C18 — infinities, incl. the `inf * 0` invalid-operation combination.
#[test]
fn c18_infinities() {
    for y in [f32::from_bits(0x7F80_0000), f32::from_bits(0xFF80_0000)] {
        for e in exp_boundary_panel() {
            check("C18", y, e);
        }
        for e in -600..=600 {
            check("C18", y, e);
        }
    }
}

/// C19 — NaN payload/sign propagation and sNaN quieting.
#[test]
fn c19_nan_payloads() {
    let mut rng = Rng::new(SEED ^ 19);
    for quiet in [true, false] {
        for _ in 0..512 {
            let y = rng.nan_f32(quiet);
            let e = rng.range_i32(-4096, 4096);
            check("C19", y, e);
            check("C19", y, 0);
            check("C19", y, -1);
            check("C19", y, 120);
            check("C19", y, 241);
        }
    }
}

/// C20 — subnormal `y`: gradual underflow and round-to-nearest-even ties.
#[test]
fn c20_subnormals() {
    let mut rng = Rng::new(SEED ^ 20);
    let fixed = [
        f32::from_bits(0x0000_0001),
        f32::from_bits(0x8000_0001),
        f32::from_bits(0x0000_0002),
        f32::from_bits(0x0000_0003),
        f32::from_bits(0x0040_0000),
        f32::from_bits(0x007F_FFFF),
        f32::from_bits(0x807F_FFFF),
    ];
    for e in 0..=240 {
        for &y in &fixed {
            check("C20", y, e);
        }
    }
    for _ in 0..40_000 {
        let e = rng.range_i32(0, 240);
        check("C20", rng.subnormal_f32(), e);
    }
}

/// C21 — extremes of the dynamic range.
#[test]
fn c21_dynamic_range_extremes() {
    let ys = [
        f32::from_bits(0x7F7F_FFFF), // +FLT_MAX
        f32::from_bits(0xFF7F_FFFF), // -FLT_MAX
        f32::from_bits(0x0080_0000), // +FLT_MIN (min normal)
        f32::from_bits(0x8080_0000), // -FLT_MIN
        1.0,
        -1.0,
        f32::from_bits(0x0000_0001), // 2^-149
        f32::from_bits(0x8000_0001),
    ];
    for &y in &ys {
        for e in [-128, -1, 0, 1, 120, 121, 240] {
            check("C21", y, e);
        }
        for e in exp_boundary_panel() {
            check("C21", y, e);
        }
    }
}

/// C22 — broad random fuzz (positive side bounded to keep the runtime sane).
#[test]
fn c22_random_fuzz() {
    let mut rng = Rng::new(SEED ^ 22);
    for _ in 0..200_000 {
        let y = rng.any_f32();
        let e = rng.range_i64(i32::MIN as i64, 20_000) as i32;
        check("C22", y, e);
    }
}

/// C23 — random fuzz over the huge-trip-count half of the `i32` domain.
#[test]
fn c23_random_fuzz_huge_trip_counts() {
    let mut rng = Rng::new(SEED ^ 23);
    for _ in 0..8 {
        let y = rng.any_f32();
        let e = rng.range_i32(0, i32::MAX);
        check("C23", y, e);
    }
    for _ in 0..64 {
        let y = rng.any_f32();
        let e = rng.range_i32(0, 4_000_000);
        check("C23", y, e);
    }
}

/// C24 — statelessness: interleaved repeated calls must be reproducible
/// (the `static const` table must not be mutated, no hidden state).
#[test]
fn c24_statelessness() {
    let h = common::harness();
    let mut rng = Rng::new(SEED ^ 24);
    for _ in 0..4096 {
        let y = rng.any_f32();
        let e = rng.range_i32(-1024, 1024);
        let c0 = h.c.ldexp_q2(y, e);
        let mut rs: Vec<f32> = h.rust.iter().map(|r| r.ldexp_q2(y, e)).collect();
        let c1 = h.c.ldexp_q2(y, e);
        for (i, r) in h.rust.iter().enumerate() {
            let again = r.ldexp_q2(y, e);
            assert_eq!(
                rs[i].to_bits(),
                again.to_bits(),
                "[C24] {} not reproducible for (0x{:08x}, {})",
                r.name,
                y.to_bits(),
                e
            );
            rs[i] = again;
        }
        let c2 = h.c.ldexp_q2(y, e);
        assert_eq!(c0.to_bits(), c1.to_bits(), "[C24] C not reproducible");
        assert_eq!(c1.to_bits(), c2.to_bits(), "[C24] C not reproducible");
        for (i, r) in h.rust.iter().enumerate() {
            assert_eq!(
                c0.to_bits(),
                rs[i].to_bits(),
                "[C24] {} diverged on repeat call for (0x{:08x}, {})",
                r.name,
                y.to_bits(),
                e
            );
        }
    }
}

/// C25 — exhaustive mantissa sweep over ALL 2^23 subnormal `y` patterns
/// (exponent field 0) for the rounding-tie-prone `exp_q2` values.
#[test]
fn c25_exhaustive_subnormal_mantissa_sweep() {
    let h = common::harness();
    for e in [1i32, 2, 3, 4, 5, 8, -1, -5, 120, 121] {
        for m in 0u32..(1 << 23) {
            let y = f32::from_bits(m);
            let expect = h.c.ldexp_q2(y, e);
            for r in &h.rust {
                let got = r.ldexp_q2(y, e);
                if expect.to_bits() != got.to_bits() {
                    panic!(
                        "[C25] {} diverged: ldexp_q2(0x{:08x}, {}) C=0x{:08x} Rust=0x{:08x}",
                        r.name,
                        y.to_bits(),
                        e,
                        expect.to_bits(),
                        got.to_bits()
                    );
                }
            }
        }
    }
}

/// C26 — exhaustive mantissa sweep over ALL 2^23 patterns for selected binades
/// (tiny normals, unit binade, largest binade) incl. the NaN/inf binade.
#[test]
fn c26_exhaustive_binade_mantissa_sweep() {
    let h = common::harness();
    for exp_field in [1u32, 126, 254, 255] {
        for e in [1i32, 4, -1, -124, 121] {
            for m in 0u32..(1 << 23) {
                let y = f32::from_bits((exp_field << 23) | m);
                let expect = h.c.ldexp_q2(y, e);
                for r in &h.rust {
                    let got = r.ldexp_q2(y, e);
                    if expect.to_bits() != got.to_bits() {
                        panic!(
                            "[C26] {} diverged: ldexp_q2(0x{:08x}, {}) C=0x{:08x} Rust=0x{:08x}",
                            r.name,
                            y.to_bits(),
                            e,
                            expect.to_bits(),
                            got.to_bits()
                        );
                    }
                }
            }
        }
    }
}

/// C27 — exhaustive mantissa sweep with the sign bit set (negative binades),
/// covering sign propagation through 1 and 2 loop iterations.
#[test]
fn c27_exhaustive_negative_binade_sweep() {
    let h = common::harness();
    for exp_field in [0u32, 1, 127, 255] {
        for e in [3i32, -2, 240] {
            for m in 0u32..(1 << 23) {
                let y = f32::from_bits(0x8000_0000 | (exp_field << 23) | m);
                let expect = h.c.ldexp_q2(y, e);
                for r in &h.rust {
                    let got = r.ldexp_q2(y, e);
                    if expect.to_bits() != got.to_bits() {
                        panic!(
                            "[C27] {} diverged: ldexp_q2(0x{:08x}, {}) C=0x{:08x} Rust=0x{:08x}",
                            r.name,
                            y.to_bits(),
                            e,
                            expect.to_bits(),
                            got.to_bits()
                        );
                    }
                }
            }
        }
    }
}
