//! Phase D — NaN/infinity propagation, regression pins, and cross-profile
//! parity.
//!
//! Uniform 32-bit fuzzing only produces a NaN in ~0.8% of slots, so triples
//! with two or three NaNs are effectively never sampled by `phase_b`'s
//! `c26_unrestricted_bit_pattern_fuzz`. That is exactly where the C's SSE
//! operand ordering is observable (float multiply is commutative in value but
//! not in NaN sign/payload propagation), so those cases get their own dense
//! coverage here.

mod common;

use common::{Libs, Rng, SEED};

fn libs() -> Libs {
    Libs::load()
}

/// A spread of NaN encodings: both signs, quiet and signalling, min/max/typical
/// payloads.
const NANS: &[u32] = &[
    0x7FC0_0000, // canonical quiet NaN
    0xFFC0_0000, // negative quiet NaN (== x86 "real indefinite")
    0x7FC0_0001,
    0xFFC0_0001,
    0x7FC0_1234,
    0xFFC0_1234,
    0x7FFF_FFFF, // max payload, quiet
    0xFFFF_FFFF,
    0x7F80_0001, // min payload, signalling
    0xFF80_0001,
    0x7FBF_FFFF, // max payload, signalling
    0xFFBF_FFFF,
    0x7F80_4321,
    0xFF80_4321,
];

const INFS: &[u32] = &[0x7F80_0000, 0xFF80_0000];

/// Zeros and a few ordinary values, so NaNs are mixed with non-NaNs too.
const PLAIN: &[u32] = &[
    0x0000_0000, // +0.0
    0x8000_0000, // -0.0
    0x3F80_0000, // 1.0
    0xBF80_0000, // -1.0
    0x3F00_0000, // 0.5
    0x0000_0001, // min subnormal
    0x7F7F_FFFF, // f32::MAX
];

/// Hues covering every `switch` arm plus the indefinite-conversion paths.
const HUES: &[f32] = &[
    30.0, 90.0, 150.0, 210.0, 270.0, 330.0, 400.0, -30.0, 0.0, 60.0, 1e30, f32::INFINITY,
];

// ---------------------------------------------------------------------------
// D1 — dense cross-product of NaN encodings across all three argument slots.
// ---------------------------------------------------------------------------

#[test]
fn d1_nan_cross_product_all_slots() {
    let l = libs();
    let mut pool: Vec<u32> = Vec::new();
    pool.extend_from_slice(NANS);
    pool.extend_from_slice(INFS);
    pool.extend_from_slice(PLAIN);

    let mut n = 0u64;
    for &hb in &pool {
        for &sb in &pool {
            for &vb in &pool {
                let src = [
                    f32::from_bits(hb),
                    f32::from_bits(sb),
                    f32::from_bits(vb),
                ];
                l.check("D1", src);
                n += 1;
            }
        }
    }
    assert!(n >= 12_000, "expected a dense sweep, only ran {n} cases");
}

// ---------------------------------------------------------------------------
// D2 — NaN in one slot crossed with every sector and with the other two slots
// swept over the plain values. Catches sector-specific propagation bugs.
// ---------------------------------------------------------------------------

#[test]
fn d2_nan_per_slot_per_sector() {
    let l = libs();
    for &nb in NANS {
        let nan = f32::from_bits(nb);
        for &h in HUES {
            for &ab in PLAIN {
                for &bb in PLAIN {
                    let (a, b) = (f32::from_bits(ab), f32::from_bits(bb));
                    l.check("D2-h", [nan, a, b]);
                    l.check("D2-s", [h, nan, b]);
                    l.check("D2-v", [h, a, nan]);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// D3 — NaN-biased fuzz: each slot is a NaN, an infinity, a zero or an ordinary
// value with roughly equal probability, so multi-NaN triples are common.
// ---------------------------------------------------------------------------

#[test]
fn d3_nan_biased_fuzz() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0xD3);

    let pick = |rng: &mut Rng| -> f32 {
        match rng.next_u32() % 5 {
            // Random NaN: all-ones exponent, non-zero mantissa, random sign
            // and payload (covers signalling and quiet).
            0 | 1 => {
                let sign = (rng.next_u32() & 1) << 31;
                let mant = (rng.next_u32() & 0x007F_FFFF) | 1;
                f32::from_bits(sign | 0x7F80_0000 | mant)
            }
            2 => f32::from_bits(((rng.next_u32() & 1) << 31) | 0x7F80_0000), // ±inf
            3 => f32::from_bits((rng.next_u32() & 1) << 31),                 // ±0.0
            _ => f32::from_bits(rng.next_u32()),                             // anything
        }
    };

    for _ in 0..600_000 {
        let src = [pick(&mut rng), pick(&mut rng), pick(&mut rng)];
        l.check("D3", src);
    }
}

// ---------------------------------------------------------------------------
// D4 — infinity-only combinations. These produce NaNs from non-NaN operands
// (inf*0, inf-inf), whose result is the hardware default NaN, a different
// mechanism from operand forwarding.
// ---------------------------------------------------------------------------

#[test]
fn d4_infinity_generated_nans() {
    let l = libs();
    let pool: Vec<f32> = INFS
        .iter()
        .chain(PLAIN.iter())
        .map(|&b| f32::from_bits(b))
        .chain([2.0f32, -2.0, 60.0, 360.0, 1e30, -1e30])
        .collect();
    for &h in &pool {
        for &s in &pool {
            for &v in &pool {
                l.check("D4", [h, s, v]);
            }
        }
    }
    // Explicitly force inf*0 and inf-inf inside the pipeline:
    //   s = inf, v = 0      -> p = (1-inf)*0 = -inf*0 = NaN
    //   s = inf, f = 0      -> s*f = inf*0  = NaN
    for &h in &[0.0f32, 60.0, 120.0, 180.0, 240.0, 300.0, 360.0, -60.0, 30.0, 90.0] {
        for &s in &[f32::INFINITY, f32::NEG_INFINITY, 1.0, -1.0] {
            for &v in &[0.0f32, -0.0, f32::INFINITY, f32::NEG_INFINITY] {
                l.check("D4", [h, s, v]);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// D5 — regression pins for divergences that were actually found and fixed.
// ---------------------------------------------------------------------------

#[test]
fn d5_regression_pins() {
    let l = libs();
    // Found in the debug profile: `q = v * (1 - s*f)` is emitted as
    // `(1 - s*f) * v`, so with h = +NaN and v = -NaN the C result carries the
    // sign of `(1 - s*f)` (positive), not of `v`.
    let pins: &[[f32; 3]] = &[
        [f32::from_bits(0x7FC0_0000), 0.7, f32::from_bits(0xFFC0_0000)],
        [f32::from_bits(0xFFC0_0000), 0.7, f32::from_bits(0x7FC0_0000)],
        [f32::from_bits(0x7FC0_0000), f32::from_bits(0xFFC0_0000), 0.7],
        [f32::from_bits(0x7F80_0001), 0.7, f32::from_bits(0xFFC0_1234)],
        // t = v * (1 - s*(1-f)): GCC emits `(1-f) * s`, i.e. the source's
        // operand order is reversed, so a NaN `f` wins over a NaN `s`.
        [f32::from_bits(0x7FC0_ABCD), f32::from_bits(0xFFC0_1111), 1.0],
        // Achromatic guard must copy `v` bit-for-bit, NaN payload included.
        [123.0, 0.0, f32::from_bits(0xFF80_0001)],
        [123.0, -0.0, f32::from_bits(0x7FFF_FFFF)],
        // Indefinite float->int conversions.
        [f32::INFINITY, 1.0, 1.0],
        [f32::NEG_INFINITY, 1.0, 1.0],
        [2147483648.0 * 60.0, 1.0, 1.0],
        [-2147483648.0 * 60.0, 1.0, 1.0],
        // Negative hue -> unsigned `cmpl $4 / ja` -> default arm.
        [-1.0, 1.0, 1.0],
        [-0.000_001, 1.0, 1.0],
    ];
    for &src in pins {
        l.check("D5", src);
    }
}

// ---------------------------------------------------------------------------
// D6 — the two profiles of the Rust cdylib must agree with each other as well
// as with C, so a profile-dependent codegen difference cannot hide.
// ---------------------------------------------------------------------------

#[test]
fn d6_debug_and_release_cdylib_agree() {
    use libloading::{Library, Symbol};
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut fns: Vec<(String, common::HsvToRgb)> = Vec::new();
    let mut keep: Vec<Library> = Vec::new();
    for profile in ["debug", "release"] {
        let p = root.join(format!("target/{profile}/libhsv_to_rgb_lib.so"));
        if !p.is_file() {
            continue;
        }
        unsafe {
            let lib = Library::new(&p).unwrap();
            let s: Symbol<common::HsvToRgb> = lib.get(b"hsv_to_rgb\0").unwrap();
            fns.push((profile.to_string(), *s));
            keep.push(lib);
        }
    }
    if fns.len() < 2 {
        eprintln!("only {} Rust profile(s) built; skipping cross-profile check", fns.len());
        return;
    }
    let mut rng = Rng::new(SEED ^ 0xD6);
    let call = |f: common::HsvToRgb, src: [f32; 3]| -> [u32; 3] {
        let mut d = [0.0f32; 3];
        unsafe { f(d.as_mut_ptr(), src.as_ptr()) };
        [d[0].to_bits(), d[1].to_bits(), d[2].to_bits()]
    };
    for _ in 0..300_000 {
        let src = [
            f32::from_bits(rng.next_u32()),
            f32::from_bits(rng.next_u32()),
            f32::from_bits(rng.next_u32()),
        ];
        let a = call(fns[0].1, src);
        let b = call(fns[1].1, src);
        assert_eq!(
            a, b,
            "{} vs {} disagree for src={src:?} ({:08x?})",
            fns[0].0,
            fns[1].0,
            [src[0].to_bits(), src[1].to_bits(), src[2].to_bits()]
        );
    }
    // NaN-heavy portion.
    for &hb in NANS {
        for &sb in NANS {
            for &vb in NANS {
                let src = [
                    f32::from_bits(hb),
                    f32::from_bits(sb),
                    f32::from_bits(vb),
                ];
                assert_eq!(
                    call(fns[0].1, src),
                    call(fns[1].1, src),
                    "profiles disagree for NaN triple {:08x} {:08x} {:08x}",
                    hb,
                    sb,
                    vb
                );
            }
        }
    }
}
