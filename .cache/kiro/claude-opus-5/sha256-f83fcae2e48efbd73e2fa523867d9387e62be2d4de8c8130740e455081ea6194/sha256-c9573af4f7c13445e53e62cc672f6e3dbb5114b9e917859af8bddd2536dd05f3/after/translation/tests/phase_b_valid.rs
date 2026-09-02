//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test loads the C `.so` and the Rust `.so` with `libloading` and calls
//! `colourblind` in both; nothing is called directly.

mod common;

use common::*;
use std::ffi::c_int;

// ---------------------------------------------------------------------------
// Generic row drivers
// ---------------------------------------------------------------------------

/// Run one `CONFIGS.md` row: `n` randomized draws of `class` through
/// `colourblind(imp, ...)` with three distinct pointers (shape A1).
fn cfg_row(row: &str, imp: c_int, class: VClass, n: usize) {
    // Row-specific seed derived from the fixed global SEED, so each row has its
    // own reproducible stream.
    let mut rng = Rng::new(SEED ^ (row.len() as u64).wrapping_mul(0x9E37_79B9) ^ hash_row(row));
    let mut compared = 0u64;
    for i in 0..n {
        let v = draw_triple(&mut rng, class);
        compared += assert_same(&format!("{row} draw#{i}"), imp, v);
    }
    expect_comparisons(row, compared, n);
}

/// Non-vacuity guard: every row must actually have compared `n` inputs against
/// every Rust `.so`, otherwise a passing row proves nothing.
#[track_caller]
fn expect_comparisons(row: &str, compared: u64, draws: usize) {
    let expected = draws as u64 * rust_impls().len() as u64;
    assert!(draws > 0, "[{row}] has zero draws");
    assert_eq!(
        compared, expected,
        "[{row}] performed {compared} comparisons, expected {expected}"
    );
}

fn hash_row(row: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in row.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    h
}

/// Run one row with a caller-chosen aliasing shape (`CONFIGS.md` Axis 4).
/// Both `.so`s get identical backing storage and identical pointer patterns.
#[track_caller]
fn cfg_alias_row(row: &str, imp: c_int, idx: [usize; 3], n: usize) {
    let mut rng = Rng::new(SEED ^ hash_row(row));
    let mut compared = 0u64;
    for i in 0..n {
        let init = draw_triple(&mut rng, VClass::Any);

        let mut c_slots = init;
        c_impl().apply_aliased(imp, &mut c_slots, idx);

        for r in rust_impls() {
            let mut r_slots = init;
            r.apply_aliased(imp, &mut r_slots, idx);
            compared += 1;
            assert_eq!(
                bits(r_slots),
                bits(c_slots),
                "\n[{row} draw#{i}] aliasing divergence in {}\n  impairment: {} ({imp})\n  ptr idx   : {:?}\n  initial   : {}\n  C   slots : {}\n  Rust slots: {}\n",
                r.label,
                impairment_name(imp),
                idx,
                show(init),
                show(c_slots),
                show(r_slots),
            );
        }
    }
    expect_comparisons(row, compared, n);
}

// ---------------------------------------------------------------------------
// C1..C3 — in-gamut colours, the intended domain
// ---------------------------------------------------------------------------

#[test]
fn c1_protanopia_in_gamut() {
    cfg_row("C1", CB_PROTANOPIA, VClass::InGamut, 20_000);
}

#[test]
fn c2_deuteranopia_in_gamut() {
    cfg_row("C2", CB_DEUTERANOPIA, VClass::InGamut, 20_000);
}

#[test]
fn c3_tritanopia_in_gamut() {
    cfg_row("C3", CB_TRITANOPIA, VClass::InGamut, 20_000);
}

// ---------------------------------------------------------------------------
// C4..C6 — arbitrary finite normals across the whole exponent range
// ---------------------------------------------------------------------------

#[test]
fn c4_protanopia_finite_normals() {
    cfg_row("C4", CB_PROTANOPIA, VClass::FiniteNormal, 20_000);
}

#[test]
fn c5_deuteranopia_finite_normals() {
    cfg_row("C5", CB_DEUTERANOPIA, VClass::FiniteNormal, 20_000);
}

#[test]
fn c6_tritanopia_finite_normals() {
    cfg_row("C6", CB_TRITANOPIA, VClass::FiniteNormal, 20_000);
}

// ---------------------------------------------------------------------------
// C7..C9 — signed zeros, exhaustive over all 8 sign combinations
// ---------------------------------------------------------------------------

fn signed_zero_triples() -> Vec<[f32; 3]> {
    let z = [0.0f32, -0.0f32];
    let mut out = Vec::new();
    for &r in &z {
        for &g in &z {
            for &b in &z {
                out.push([r, g, b]);
            }
        }
    }
    out
}

fn exhaustive_row(row: &str, imp: c_int, triples: &[[f32; 3]]) {
    let mut compared = 0u64;
    for (i, v) in triples.iter().enumerate() {
        compared += assert_same(&format!("{row} case#{i}"), imp, *v);
    }
    expect_comparisons(row, compared, triples.len());
}

#[test]
fn c7_protanopia_signed_zeros() {
    exhaustive_row("C7", CB_PROTANOPIA, &signed_zero_triples());
}

#[test]
fn c8_deuteranopia_signed_zeros() {
    exhaustive_row("C8", CB_DEUTERANOPIA, &signed_zero_triples());
}

#[test]
fn c9_tritanopia_signed_zeros() {
    exhaustive_row("C9", CB_TRITANOPIA, &signed_zero_triples());
}

// ---------------------------------------------------------------------------
// C10..C12 — subnormals
// ---------------------------------------------------------------------------

#[test]
fn c10_protanopia_subnormals() {
    cfg_row("C10", CB_PROTANOPIA, VClass::Subnormal, 20_000);
}

#[test]
fn c11_deuteranopia_subnormals() {
    cfg_row("C11", CB_DEUTERANOPIA, VClass::Subnormal, 20_000);
}

#[test]
fn c12_tritanopia_subnormals() {
    cfg_row("C12", CB_TRITANOPIA, VClass::Subnormal, 20_000);
}

// ---------------------------------------------------------------------------
// C13..C15 — extremes that overflow / underflow
// ---------------------------------------------------------------------------

#[test]
fn c13_protanopia_extremes() {
    cfg_row("C13", CB_PROTANOPIA, VClass::Extreme, 20_000);
}

#[test]
fn c14_deuteranopia_extremes() {
    cfg_row("C14", CB_DEUTERANOPIA, VClass::Extreme, 20_000);
}

#[test]
fn c15_tritanopia_extremes() {
    cfg_row("C15", CB_TRITANOPIA, VClass::Extreme, 20_000);
}

// ---------------------------------------------------------------------------
// C16..C18 — infinities, exhaustive over all 8 sign combinations
// ---------------------------------------------------------------------------

fn infinity_triples() -> Vec<[f32; 3]> {
    let inf = [f32::INFINITY, f32::NEG_INFINITY];
    let mut out = Vec::new();
    for &r in &inf {
        for &g in &inf {
            for &b in &inf {
                out.push([r, g, b]);
            }
        }
    }
    out
}

#[test]
fn c16_protanopia_infinities() {
    exhaustive_row("C16", CB_PROTANOPIA, &infinity_triples());
}

#[test]
fn c17_deuteranopia_infinities() {
    exhaustive_row("C17", CB_DEUTERANOPIA, &infinity_triples());
}

#[test]
fn c18_tritanopia_infinities() {
    exhaustive_row("C18", CB_TRITANOPIA, &infinity_triples());
}

// ---------------------------------------------------------------------------
// C19..C21 — quiet NaNs (sign + payload propagation)
// ---------------------------------------------------------------------------

#[test]
fn c19_protanopia_quiet_nans() {
    cfg_row("C19", CB_PROTANOPIA, VClass::QuietNan, 20_000);
}

#[test]
fn c20_deuteranopia_quiet_nans() {
    cfg_row("C20", CB_DEUTERANOPIA, VClass::QuietNan, 20_000);
}

#[test]
fn c21_tritanopia_quiet_nans() {
    cfg_row("C21", CB_TRITANOPIA, VClass::QuietNan, 20_000);
}

// ---------------------------------------------------------------------------
// C22..C24 — signalling NaNs (quieting preserves sign and payload)
// ---------------------------------------------------------------------------

#[test]
fn c22_protanopia_signalling_nans() {
    cfg_row("C22", CB_PROTANOPIA, VClass::SignallingNan, 20_000);
}

#[test]
fn c23_deuteranopia_signalling_nans() {
    cfg_row("C23", CB_DEUTERANOPIA, VClass::SignallingNan, 20_000);
}

#[test]
fn c24_tritanopia_signalling_nans() {
    cfg_row("C24", CB_TRITANOPIA, VClass::SignallingNan, 20_000);
}

// ---------------------------------------------------------------------------
// C25..C27 — exactly one special channel among two ordinary normals
// ---------------------------------------------------------------------------

fn one_special_row(row: &str, imp: c_int, n: usize) {
    let mut rng = Rng::new(SEED ^ hash_row(row));
    let mut compared = 0u64;
    for i in 0..n {
        let v = draw_one_special(&mut rng);
        compared += assert_same(&format!("{row} draw#{i}"), imp, v);
    }
    expect_comparisons(row, compared, n);
}

#[test]
fn c25_protanopia_one_special_channel() {
    one_special_row("C25", CB_PROTANOPIA, 20_000);
}

#[test]
fn c26_deuteranopia_one_special_channel() {
    one_special_row("C26", CB_DEUTERANOPIA, 20_000);
}

#[test]
fn c27_tritanopia_one_special_channel() {
    one_special_row("C27", CB_TRITANOPIA, 20_000);
}

// ---------------------------------------------------------------------------
// C28..C30 — unit basis vectors recover every matrix coefficient bit-exactly
// ---------------------------------------------------------------------------

const BASIS: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

#[test]
fn c28_protanopia_basis_vectors() {
    exhaustive_row("C28", CB_PROTANOPIA, &BASIS);
}

#[test]
fn c29_deuteranopia_basis_vectors() {
    exhaustive_row("C29", CB_DEUTERANOPIA, &BASIS);
}

#[test]
fn c30_tritanopia_basis_vectors() {
    exhaustive_row("C30", CB_TRITANOPIA, &BASIS);
}

/// The basis-vector responses ARE the matrix columns. Recording them makes a
/// coefficient regression obvious rather than showing up as an opaque bit diff.
#[test]
fn c28_30_matrix_columns_match_exactly() {
    for &imp in &VALID_IMPAIRMENTS {
        for (j, v) in BASIS.iter().enumerate() {
            let c = c_impl().apply(imp, *v);
            for r in rust_impls() {
                let got = r.apply(imp, *v);
                assert_eq!(
                    bits(got),
                    bits(c),
                    "matrix column {j} of {} differs in {}: C {} vs Rust {}",
                    impairment_name(imp),
                    r.label,
                    show(c),
                    show(got)
                );
            }
            println!(
                "{:<13} column {j}: {}",
                impairment_name(imp),
                show(c)
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C31 — everything mixed, drawn per channel independently
// ---------------------------------------------------------------------------

#[test]
fn c31_all_impairments_mixed_values() {
    let mut rng = Rng::new(SEED ^ hash_row("C31"));
    let mut compared = 0u64;
    for i in 0..20_000 {
        for &imp in &VALID_IMPAIRMENTS {
            let v = draw_triple(&mut rng, VClass::Any);
            compared += assert_same(&format!("C31 draw#{i}"), imp, v);
        }
    }
    expect_comparisons("C31", compared, 20_000 * 3);
}

// ---------------------------------------------------------------------------
// C32..C43 — pointer aliasing shapes A2..A5, per impairment
// ---------------------------------------------------------------------------

#[test]
fn c32_protanopia_alias_r_g() {
    cfg_alias_row("C32", CB_PROTANOPIA, [0, 0, 1], 20_000);
}

#[test]
fn c33_protanopia_alias_r_b() {
    cfg_alias_row("C33", CB_PROTANOPIA, [0, 1, 0], 20_000);
}

#[test]
fn c34_protanopia_alias_g_b() {
    cfg_alias_row("C34", CB_PROTANOPIA, [0, 1, 1], 20_000);
}

#[test]
fn c35_protanopia_alias_all() {
    cfg_alias_row("C35", CB_PROTANOPIA, [0, 0, 0], 20_000);
}

#[test]
fn c36_deuteranopia_alias_r_g() {
    cfg_alias_row("C36", CB_DEUTERANOPIA, [0, 0, 1], 20_000);
}

#[test]
fn c37_deuteranopia_alias_r_b() {
    cfg_alias_row("C37", CB_DEUTERANOPIA, [0, 1, 0], 20_000);
}

#[test]
fn c38_deuteranopia_alias_g_b() {
    cfg_alias_row("C38", CB_DEUTERANOPIA, [0, 1, 1], 20_000);
}

#[test]
fn c39_deuteranopia_alias_all() {
    cfg_alias_row("C39", CB_DEUTERANOPIA, [0, 0, 0], 20_000);
}

#[test]
fn c40_tritanopia_alias_r_g() {
    cfg_alias_row("C40", CB_TRITANOPIA, [0, 0, 1], 20_000);
}

#[test]
fn c41_tritanopia_alias_r_b() {
    cfg_alias_row("C41", CB_TRITANOPIA, [0, 1, 0], 20_000);
}

#[test]
fn c42_tritanopia_alias_g_b() {
    cfg_alias_row("C42", CB_TRITANOPIA, [0, 1, 1], 20_000);
}

#[test]
fn c43_tritanopia_alias_all() {
    cfg_alias_row("C43", CB_TRITANOPIA, [0, 0, 0], 20_000);
}

// ---------------------------------------------------------------------------
// C44 — three separate heap allocations (no adjacency assumption)
// ---------------------------------------------------------------------------

#[test]
fn c44_separate_heap_allocations() {
    let mut rng = Rng::new(SEED ^ hash_row("C44"));
    let mut compared = 0u64;
    for i in 0..10_000 {
        for &imp in &VALID_IMPAIRMENTS {
            let v = draw_triple(&mut rng, VClass::Any);

            let call = |im: &Impl| -> [f32; 3] {
                // Deliberately non-contiguous, individually-allocated cells.
                let mut br = Box::new(v[0]);
                let _pad1 = Box::new([0u8; 64]);
                let mut bg = Box::new(v[1]);
                let _pad2 = Box::new([0u8; 128]);
                let mut bb = Box::new(v[2]);
                unsafe {
                    (im.call)(
                        imp,
                        &mut *br as *mut f32,
                        &mut *bg as *mut f32,
                        &mut *bb as *mut f32,
                    )
                };
                [*br, *bg, *bb]
            };

            let expect = call(c_impl());
            for r in rust_impls() {
                let got = call(r);
                compared += 1;
                assert_eq!(
                    bits(got),
                    bits(expect),
                    "\n[C44 draw#{i}] heap-pointer divergence in {}\n  impairment: {} ({imp})\n  input     : {}\n  C   : {}\n  Rust: {}\n",
                    r.label,
                    impairment_name(imp),
                    show(v),
                    show(expect),
                    show(got),
                );
            }
        }
    }
    expect_comparisons("C44", compared, 10_000 * 3);
}

// ---------------------------------------------------------------------------
// C45 — statelessness: no hidden state, order-independent
// ---------------------------------------------------------------------------

#[test]
fn c45_stateless_and_order_independent() {
    // (a) the same call repeated on fresh buffers must be identical every time
    for &imp in &VALID_IMPAIRMENTS {
        let v = [0.25f32, 0.5, 0.75];
        let first_c = c_impl().apply(imp, v);
        for _ in 0..64 {
            assert_eq!(
                bits(c_impl().apply(imp, v)),
                bits(first_c),
                "C .so is not stateless for {}",
                impairment_name(imp)
            );
            for r in rust_impls() {
                assert_eq!(
                    bits(r.apply(imp, v)),
                    bits(first_c),
                    "{} is not stateless / diverges for {}",
                    r.label,
                    impairment_name(imp)
                );
            }
        }
    }

    // (b) interleaving impairments must not perturb later results
    let mut rng = Rng::new(SEED ^ hash_row("C45"));
    let mut compared = 0u64;
    for i in 0..10_000 {
        let imp = VALID_IMPAIRMENTS[rng.below(3) as usize];
        let v = draw_triple(&mut rng, VClass::Any);
        compared += assert_same(&format!("C45 interleaved#{i}"), imp, v);
    }
    expect_comparisons("C45", compared, 10_000);
}

// ---------------------------------------------------------------------------
// C46 — exhaustive over the top 16 bits of the f32 space, one channel varying
// ---------------------------------------------------------------------------

#[test]
fn c46_exhaustive_high_16_bits() {
    // Every sign/exponent/high-mantissa pattern. The low 16 mantissa bits are
    // pinned to a fixed non-zero value so subnormal and NaN encodings stay
    // inside their class.
    const LOW: u32 = 0x0000_ABCD;
    let pinned: [f32; 3] = [0.375, -1.5, 0.875];
    let mut compared: u64 = 0;

    for &imp in &VALID_IMPAIRMENTS {
        for channel in 0..3usize {
            for hi in 0u32..=0xFFFF {
                let x = f32::from_bits((hi << 16) | LOW);
                let mut v = pinned;
                v[channel] = x;

                let expect = c_impl().apply(imp, v);
                for r in rust_impls() {
                    let got = r.apply(imp, v);
                    compared += 1;
                    if bits(got) != bits(expect) {
                        panic!(
                            "\n[C46] divergence in {}\n  impairment : {} ({imp})\n  channel    : {channel}\n  hi16       : {hi:#06x} -> {:#010x}\n  input      : {}\n  C   output : {}\n  Rust output: {}\n",
                            r.label,
                            impairment_name(imp),
                            x.to_bits(),
                            show(v),
                            show(expect),
                            show(got),
                        );
                    }
                }
            }
        }
    }

    // Guard against a silently-empty loop: 3 impairments x 3 channels x 65536
    // high-bit patterns x one comparison per Rust .so under test.
    let expected = 3u64 * 3 * 65536 * rust_impls().len() as u64;
    assert_eq!(
        compared, expected,
        "C46 performed {compared} comparisons but should have performed {expected}"
    );
    println!("C46: {compared} bit-exact comparisons across {} Rust .so(s)", rust_impls().len());
}
