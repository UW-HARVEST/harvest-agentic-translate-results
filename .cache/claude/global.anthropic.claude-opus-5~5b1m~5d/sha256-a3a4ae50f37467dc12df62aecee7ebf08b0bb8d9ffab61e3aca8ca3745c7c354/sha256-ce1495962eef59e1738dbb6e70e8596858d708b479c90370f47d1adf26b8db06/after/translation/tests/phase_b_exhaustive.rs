//! Phase B, continued — near-exhaustive sweeps.
//!
//! `CONFIGS.md` rows 10-12 cover NaN payload/sign propagation with randomised
//! inputs. These tests push the same axes to *exhaustion* over the interesting
//! subspaces, because the operand order transcribed in `src/lib.rs` is only
//! observable through NaN ties: a sampling test can miss a single payload, an
//! exhaustive one cannot.
//!
//! In `--release` the NaN sweeps run at stride 1, i.e. genuinely exhaustive over
//! all 2^24 NaN encodings (~604M + ~75M compared calls, about 15s). Debug builds
//! are far slower through `dlsym`, so they coarsen automatically. Override with
//! `CB_NAN_STRIDE` / `CB_EXP_STRIDE`:
//!
//! ```text
//! CB_NAN_STRIDE=64 cargo test --release -- --nocapture   # quick pass
//! ```

mod common;

use common::*;

fn stride(var: &str, default: u32) -> u32 {
    std::env::var(var)
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(default)
}

/// Every NaN bit pattern is `s 11111111 mmmmmmmmmmmmmmmmmmmmmmm` with
/// `m != 0`: 2^24 - 2 of them. Sweep them all (or every `stride`-th) in each
/// channel position, for every impairment.
#[test]
fn cfg_row25_exhaustive_nan_payload_and_sign_in_each_channel() {
    let (c, rust) = both();
    // Debug builds are ~50x slower through the FFI, so coarsen there.
    let default = if cfg!(debug_assertions) { 8 } else { 1 };
    let st = stride("CB_NAN_STRIDE", default);

    // Partners chosen so that each of the three sub-expressions sees a
    // different mix: a normal (never NaN), a NaN of the OPPOSITE sign with a
    // distinctive payload (so a tie must be resolved), and an infinity.
    let partners: [(f32, f32); 4] = [
        (0.375, 0.625),
        (f32::from_bits(0xFFC1_2345), 0.5),
        (f32::INFINITY, f32::NEG_INFINITY),
        (f32::from_bits(0x7F80_0001), f32::from_bits(0xFFD5_5555)),
    ];

    let mut checked: u64 = 0;
    for &imp in &VALID {
        for pos in 0usize..3 {
            for &(p0, p1) in &partners {
                let mut m: u32 = 1;
                while m < 0x0080_0000 {
                    for sign in [0u32, 0x8000_0000] {
                        let nan = f32::from_bits(sign | 0x7F80_0000 | m);
                        let mut v = [0f32; 3];
                        let mut it = [p0, p1].into_iter();
                        for (i, slot) in v.iter_mut().enumerate() {
                            *slot = if i == pos { nan } else { it.next().unwrap() };
                        }
                        diff(&c, &rust, "exhaustive NaN sweep", imp, v);
                        checked += 1;
                    }
                    m += st;
                }
            }
        }
    }
    // Exact expected count: this both proves the sweep really visited every
    // intended pattern and catches off-by-one errors in the loop above.
    let m_values = u64::from((0x0080_0000u32 - 1).div_ceil(st));
    let expected = m_values * 2 /* signs */ * 4 /* partners */ * 3 /* positions */ * 3 /* impairments */;
    println!("exhaustive NaN sweep: {checked} inputs (stride {st}, expected {expected})");
    assert_eq!(checked, expected, "NaN sweep did not cover the intended space");
    assert!(m_values >= 0x0080_0000 / 64, "stride {st} is too coarse to be meaningful");
}

/// All 2^24 NaN patterns simultaneously in *all three* channels (same pattern in
/// each), which is the configuration where every add/sub in the function has two
/// NaN operands at once — the only way the destination-wins rule is visible.
#[test]
fn cfg_row26_exhaustive_nan_in_all_three_channels() {
    let (c, rust) = both();
    let default = if cfg!(debug_assertions) { 8 } else { 1 };
    let st = stride("CB_NAN_STRIDE", default);
    let mut checked: u64 = 0;
    for &imp in &VALID {
        let mut m: u32 = 1;
        while m < 0x0080_0000 {
            // Three different payloads/signs derived from `m`, so the three
            // channels never hold the same NaN and the tie-break is decisive.
            let a = f32::from_bits(0x7F80_0000 | m);
            let b = f32::from_bits(0x8000_0000 | 0x7F80_0000 | (m ^ 0x0055_5555).max(1));
            let d = f32::from_bits(0x7F80_0000 | 0x0040_0000 | (m ^ 0x002A_AAAA).max(1));
            diff(&c, &rust, "exhaustive triple-NaN sweep", imp, [a, b, d]);
            diff(&c, &rust, "exhaustive triple-NaN sweep", imp, [d, a, b]);
            diff(&c, &rust, "exhaustive triple-NaN sweep", imp, [b, d, a]);
            checked += 3;
            m += st;
        }
    }
    let m_values = u64::from((0x0080_0000u32 - 1).div_ceil(st));
    let expected = m_values * 3 /* rotations */ * 3 /* impairments */;
    println!("exhaustive triple-NaN sweep: {checked} inputs (stride {st}, expected {expected})");
    assert_eq!(checked, expected, "triple-NaN sweep did not cover the intended space");
    assert!(m_values >= 0x0080_0000 / 64, "stride {st} is too coarse to be meaningful");
}

/// Sweep every biased exponent (0..=255) against every other, with a handful of
/// mantissas, so overflow, underflow, cancellation and gradual-underflow
/// boundaries are all hit systematically rather than by luck.
#[test]
fn cfg_row27_exhaustive_exponent_cross_product() {
    let (c, rust) = both();
    let st = stride("CB_EXP_STRIDE", 1);
    let mantissas: [u32; 4] = [0x0000_0000, 0x0000_0001, 0x0055_5555, 0x007F_FFFF];
    let mut checked: u64 = 0;
    for &imp in &VALID {
        let mut er = 0u32;
        while er <= 255 {
            let mut eg = 0u32;
            while eg <= 255 {
                // The blue exponent is tied to a rotating choice rather than a
                // third nested loop, to keep this at a few million cases.
                let eb = (er * 7 + eg * 13) % 256;
                for (k, &mm) in mantissas.iter().enumerate() {
                    let sr = ((er + k as u32) & 1) << 31;
                    let sg = ((eg + k as u32) & 1) << 31;
                    let sb = ((eb + k as u32) & 1) << 31;
                    let v = [
                        f32::from_bits(sr | (er << 23) | mm),
                        f32::from_bits(sg | (eg << 23) | (mm ^ 0x0012_3456)),
                        f32::from_bits(sb | (eb << 23) | (mm ^ 0x0065_4321)),
                    ];
                    diff(&c, &rust, "exhaustive exponent cross product", imp, v);
                    checked += 1;
                }
                eg += st;
            }
            er += st;
        }
    }
    let e_values = u64::from(256u32.div_ceil(st));
    let expected = e_values * e_values * 4 /* mantissas */ * 3 /* impairments */;
    println!("exhaustive exponent cross product: {checked} inputs (stride {st}, expected {expected})");
    assert_eq!(checked, expected, "exponent sweep did not cover the intended space");
    assert_eq!(st, 1, "the exponent cross product is meant to run exhaustively");
}

/// Sweep the full 32-bit input space of one channel at a coarse stride, with the
/// other two channels cycling, so no exponent/mantissa region is unvisited.
#[test]
fn cfg_row28_strided_full_u32_channel_sweep() {
    let (c, rust) = both();
    // 2^32 / step iterations per (impairment, position).
    let step: u32 = if cfg!(debug_assertions) { 0x0001_0001 } else { 0x0000_0101 };
    let mut checked: u64 = 0;
    for &imp in &VALID {
        for pos in 0usize..3 {
            let mut rng = Rng::new(SEED ^ 0xFFFF ^ u64::from(imp) ^ pos as u64);
            let mut bits: u32 = 0;
            loop {
                let mut v = [rng.any_f32(), rng.any_f32(), rng.any_f32()];
                v[pos] = f32::from_bits(bits);
                diff(&c, &rust, "strided full-u32 channel sweep", imp, v);
                checked += 1;
                match bits.checked_add(step) {
                    Some(next) => bits = next,
                    None => break,
                }
            }
        }
    }
    let per_axis = u64::from(u32::MAX / step) + 1;
    let expected = per_axis * 3 /* positions */ * 3 /* impairments */;
    println!("strided full-u32 channel sweep: {checked} inputs (step {step:#x}, expected {expected})");
    assert_eq!(checked, expected, "u32 sweep did not cover the intended space");
    assert!(per_axis >= 65_536, "step {step:#x} leaves too much of the u32 space unvisited");
}
