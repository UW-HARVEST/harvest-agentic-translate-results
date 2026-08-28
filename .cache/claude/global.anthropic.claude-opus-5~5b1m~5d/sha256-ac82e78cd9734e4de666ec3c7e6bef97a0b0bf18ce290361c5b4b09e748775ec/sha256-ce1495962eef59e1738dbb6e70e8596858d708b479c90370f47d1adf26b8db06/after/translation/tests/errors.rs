//! Phase C — error / rejection-path differential tests, one test per
//! `ERRORS.md` row (rows that share a construction are grouped, and every row
//! id appears in an assertion message).
//!
//! `synth_pair` returns `void`, so the "error result" being compared is the
//! sentinel the C substitutes: the saturated `int16_t` from the two guards in
//! `mp3d_scale_pcm`, the `0` produced by the un-guarded NaN conversion, and the
//! destination index the out-of-range `nch` values resolve to.

mod common;

use common::*;
use std::ffi::c_int;

const PCM_LEN: usize = 16 * 8 + 16;
const FILL: i16 = 0x5A5A_u16 as i16;

/// Runs both implementations for a lane-0 accumulator of exactly `a` and
/// asserts the shared result equals `expect`.
fn expect_lane0(row: &str, a: f32, expect: i16) {
    let z = z_for_lane0_exact(a)
        .unwrap_or_else(|| panic!("{row}: cannot construct lane-0 accumulator {a:e}"));
    let got = model_lane0(&z);
    assert_eq!(
        got.to_bits(),
        a.to_bits(),
        "{row}: constructed accumulator 0x{:08x} != requested 0x{:08x}",
        got.to_bits(),
        a.to_bits()
    );
    let out = diff_call(row, PCM_LEN, 0, FILL, 1, &z, 0);
    assert_eq!(
        out[0], expect,
        "{row}: C and Rust agree but on the wrong value for a = {a:e} \
         (0x{:08x}); expected {expect}",
        a.to_bits()
    );
}

/// Same for lane 1 (the `pcm[16 * nch]` store); lane 0 stays at `0.0`.
fn expect_lane1(row: &str, a: f32, expect: i16) {
    let z = z_for_lane1_exact(a)
        .unwrap_or_else(|| panic!("{row}: cannot construct lane-1 accumulator {a:e}"));
    let got = model_lane1(&z);
    assert_eq!(
        got.to_bits(),
        a.to_bits(),
        "{row}: constructed accumulator 0x{:08x} != requested 0x{:08x}",
        got.to_bits(),
        a.to_bits()
    );
    assert_eq!(model_lane0(&z), 0.0, "{row}: lane 0 should stay zero");
    let nch: c_int = 2;
    let out = diff_call(row, PCM_LEN, 0, FILL, nch, &z, 0);
    assert_eq!(
        out[16 * nch as usize], expect,
        "{row}: C and Rust agree but on the wrong value for a = {a:e}",
    );
    // Lane 0 wrote `mp3d_scale_pcm(0.0) == 0`.
    assert_eq!(out[0], 0, "{row}: lane 0 unexpectedly non-zero");
}

// ---------------------------------------------------------------------------
// E1 / E3 — high clamp on both lanes.  E2 / E4 — low clamp on both lanes.
// ---------------------------------------------------------------------------

#[test]
fn err_e1_e2_saturate_high() {
    let mut rng = Rng::new(0xE001);
    // E1: lane 0.
    for a in [32_766.5f32, 32_767.0, 32_768.0, 40_000.0, 1e30, f32::MAX] {
        expect_lane0(&format!("E1 a={a:e}"), a, 32767);
    }
    // E3: lane 1.
    for a in [32_766.5f32, 32_767.0, 32_768.0, 40_000.0, 1e30, f32::MAX] {
        expect_lane1(&format!("E3 a={a:e}"), a, 32767);
    }
    // Randomised sweep strictly above the threshold.
    for _ in 0..2_000 {
        let a = 32_766.5f32 + rng.unit() * 1e6 + f32::EPSILON;
        let z = z_for_lane0_exact(a);
        if let Some(z) = z {
            if model_lane0(&z).to_bits() == a.to_bits() {
                let out = diff_call("E1 rand", PCM_LEN, 0, FILL, 1, &z, 0);
                assert_eq!(out[0], 32767, "E1: a={a:e} did not clamp high");
            }
        }
    }
}

#[test]
fn err_e3_e4_saturate_low() {
    let mut rng = Rng::new(0xE002);
    // E2: lane 0.
    for a in [-32_767.5f32, -32_768.0, -40_000.0, -1e30, f32::MIN] {
        expect_lane0(&format!("E2 a={a:e}"), a, -32768);
    }
    // E4: lane 1.
    for a in [-32_767.5f32, -32_768.0, -40_000.0, -1e30, f32::MIN] {
        expect_lane1(&format!("E4 a={a:e}"), a, -32768);
    }
    for _ in 0..2_000 {
        let a = -32_767.5f32 - rng.unit() * 1e6 - f32::EPSILON;
        if let Some(z) = z_for_lane0_exact(a) {
            if model_lane0(&z).to_bits() == a.to_bits() {
                let out = diff_call("E2 rand", PCM_LEN, 0, FILL, 1, &z, 0);
                assert_eq!(out[0], -32768, "E2: a={a:e} did not clamp low");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E5 / E6 — infinities are absorbed by the two guards
// ---------------------------------------------------------------------------

#[test]
fn err_e5_e6_infinities() {
    // Lane 0: `z[512] * 37489` overflows to +-inf.
    let mut z = zeros_z();
    z[512] = f32::MAX;
    assert_eq!(model_lane0(&z), f32::INFINITY);
    let out = diff_call("E5 lane0 +inf", PCM_LEN, 0, FILL, 1, &z, 0);
    assert_eq!(out[0], 32767, "E5: +inf must clamp to 32767");

    let mut z = zeros_z();
    z[512] = f32::MIN;
    assert_eq!(model_lane0(&z), f32::NEG_INFINITY);
    let out = diff_call("E6 lane0 -inf", PCM_LEN, 0, FILL, 1, &z, 0);
    assert_eq!(out[0], -32768, "E6: -inf must clamp to -32768");

    // Direct `+-inf` fed straight into a tap.
    for (v, expect) in [(f32::INFINITY, 32767i16), (f32::NEG_INFINITY, -32768i16)] {
        let mut z = zeros_z();
        z[448] = v; // weight +75038, so the sign is preserved
        let out = diff_call(&format!("E5/E6 tap {v}"), PCM_LEN, 0, FILL, 1, &z, 0);
        assert_eq!(out[0], expect, "E5/E6: tap {v} -> {expect}");
    }
    // Lane 1 too.
    for (v, expect) in [(f32::INFINITY, 32767i16), (f32::NEG_INFINITY, -32768i16)] {
        let mut z = zeros_z();
        z[514] = v; // weight +64019
        let out = diff_call(&format!("E5/E6 lane1 {v}"), PCM_LEN, 0, FILL, 2, &z, 0);
        assert_eq!(out[32], expect, "E5/E6 lane1: tap {v} -> {expect}");
    }
}

// ---------------------------------------------------------------------------
// E7 — NaN falls through *both* guards (`comiss` sets CF when unordered)
// ---------------------------------------------------------------------------

#[test]
fn err_e7_nan_falls_through_guards() {
    // A plain NaN in a tap.
    for bits in [
        0x7FC0_0000u32, // canonical quiet NaN
        0xFFC0_0000,    // negative quiet NaN
        0x7F80_0001,    // signalling NaN, smallest payload
        0xFF80_0001,
        0x7FFF_FFFF, // all-payload-bits quiet NaN
        0xFFBF_FFFF,
    ] {
        let nan = f32::from_bits(bits);
        assert!(nan.is_nan());

        let mut z = zeros_z();
        z[448] = nan;
        let out = diff_call(&format!("E7 lane0 0x{bits:08x}"), PCM_LEN, 0, FILL, 1, &z, 0);
        assert_eq!(
            out[0], 0,
            "E7: NaN 0x{bits:08x} must reach the conversion path and yield 0"
        );

        let mut z = zeros_z();
        z[514] = nan;
        let out = diff_call(&format!("E7 lane1 0x{bits:08x}"), PCM_LEN, 0, FILL, 2, &z, 0);
        assert_eq!(out[32], 0, "E7 lane1: NaN 0x{bits:08x} -> 0");
    }

    // NaN produced arithmetically (`inf - inf`) rather than supplied directly.
    let mut z = zeros_z();
    z[896] = f32::INFINITY;
    z[0] = f32::INFINITY; // (inf - inf) * 29 -> NaN
    assert!(model_lane0(&z).is_nan());
    let out = diff_call("E7 inf-inf", PCM_LEN, 0, FILL, 1, &z, 0);
    assert_eq!(out[0], 0, "E7: arithmetically produced NaN -> 0");

    // NaN produced as `0 * inf`.
    let mut z = zeros_z();
    z[514] = 0.0;
    z[386] = f32::INFINITY;
    z[642] = f32::INFINITY; // 9727*inf = inf then -9975*inf = -inf -> NaN
    assert!(model_lane1(&z).is_nan());
    let out = diff_call("E7 lane1 inf mix", PCM_LEN, 0, FILL, 2, &z, 0);
    assert_eq!(out[32], 0, "E7 lane1: NaN -> 0");
}

// ---------------------------------------------------------------------------
// E8 / E9 — the guards are inclusive (`>=` and `<=`)
// ---------------------------------------------------------------------------

#[test]
fn err_e8_e9_exact_guard_boundaries() {
    expect_lane0("E8 a == 32766.5", 32_766.5, 32767);
    expect_lane1("E8 lane1 a == 32766.5", 32_766.5, 32767);
    expect_lane0("E9 a == -32767.5", -32_767.5, -32768);
    expect_lane1("E9 lane1 a == -32767.5", -32_767.5, -32768);
}

// ---------------------------------------------------------------------------
// E10 / E11 — one ULP inside the guards (the extreme conversion-path values)
// ---------------------------------------------------------------------------

#[test]
fn err_e10_e11_one_ulp_inside_guards() {
    let hi_in = prev_f32(32_766.5); // 32766.498046875
    assert!(hi_in < 32_766.5);
    expect_lane0("E10 one ulp below 32766.5", hi_in, 32766);
    expect_lane1("E10 lane1 one ulp below 32766.5", hi_in, 32766);

    let lo_in = next_f32(-32_767.5); // -32767.498046875 (closer to zero)
    assert!(lo_in > -32_767.5);
    expect_lane0("E11 one ulp above -32767.5", lo_in, -32767);
    expect_lane1("E11 lane1 one ulp above -32767.5", lo_in, -32767);

    // One ULP *outside* each guard must clamp.
    expect_lane0("E10' one ulp above 32766.5", next_f32(32_766.5), 32767);
    expect_lane0("E11' one ulp below -32767.5", prev_f32(-32_767.5), -32768);
}

// ---------------------------------------------------------------------------
// E12 — the `s -= (s < 0)` downward bias for negative samples
// ---------------------------------------------------------------------------

#[test]
fn err_e12_negative_bias_correction() {
    // `(int16_t)(a + .5f)` then `-1` when the truncated value is negative.
    // The differential comparison is the real assertion; `c_scale_pcm_reference`
    // is a literal replay of the C used as an independent cross-check of the
    // value the two `.so`s agree on.
    let cases: [f32; 14] = [
        -0.5, -0.5001, -1.0, -1.5, -2.5, -100.5, -1.0e3, -0.25, 0.5, 1.5, 0.25, -32_767.0,
        -32_766.0, -1.0e4,
    ];
    for a in cases {
        let want = c_scale_pcm_reference(a);
        expect_lane0(&format!("E12 a={a:e}"), a, want);
    }
    // Spot-check the documented arithmetic explicitly:
    //   a = -1.5 -> (int16)(-1.0) == -1 -> -1 - 1 == -2
    assert_eq!(c_scale_pcm_reference(-1.5), -2);
    //   a = -0.5 -> (int16)(0.0)  ==  0 -> 0 is NOT < 0 -> 0
    assert_eq!(c_scale_pcm_reference(-0.5), 0);
    //   a = -0.5001 -> (int16)(-0.0001) == 0 -> 0
    assert_eq!(c_scale_pcm_reference(-0.5001), 0);
    //   a =  0.5 -> (int16)(1.0) == 1 -> 1
    assert_eq!(c_scale_pcm_reference(0.5), 1);

    // Randomised: every negative non-clamping accumulator must be biased down.
    let mut rng = Rng::new(0xE012);
    for _ in 0..3_000 {
        let a = -(rng.unit() * 32_000.0);
        if let Some(z) = z_for_lane0_exact(a) {
            if model_lane0(&z).to_bits() == a.to_bits() {
                let out = diff_call("E12 rand", PCM_LEN, 0, FILL, 1, &z, 0);
                assert_eq!(
                    out[0],
                    c_scale_pcm_reference(a),
                    "E12: bias correction wrong for a={a:e}"
                );
            }
        }
    }
}


// ---------------------------------------------------------------------------
// E13 — negative zero takes no bias correction
// ---------------------------------------------------------------------------

#[test]
fn err_e13_negative_zero() {
    // Getting `a == -0.0` all the way to `mp3d_scale_pcm` requires *every*
    // accumulated term to be `-0.0`, because IEEE-754 gives `-0.0 + +0.0 == +0.0`.
    //
    //   term 1: (z[896] - z[0]) * 29      -> (-0.0 - +0.0) * 29      == -0.0
    //   term 2: (z[64] + z[832]) * 213    -> (-0.0 + -0.0) * 213     == -0.0
    //   term 3: (z[768] - z[128]) * 459   -> (-0.0 - +0.0) * 459     == -0.0
    //   term 4: (z[192] + z[704]) * 2037  -> (-0.0 + -0.0) * 2037    == -0.0
    //   term 5: (z[640] - z[256]) * 5153  -> (-0.0 - +0.0) * 5153    == -0.0
    //   term 6: (z[320] + z[576]) * 6574  -> (-0.0 + -0.0) * 6574    == -0.0
    //   term 7: (z[512] - z[384]) * 37489 -> (-0.0 - +0.0) * 37489   == -0.0
    //   term 8: z[448] * 75038            -> -0.0 * 75038            == -0.0
    let mut z = zeros_z();
    for i in [896usize, 64, 832, 768, 192, 704, 640, 320, 576, 512, 448] {
        z[i] = -0.0;
    }
    for i in [0usize, 128, 256, 384] {
        z[i] = 0.0;
    }
    let a = model_lane0(&z);
    assert_eq!(a.to_bits(), (-0.0f32).to_bits(), "E13: expected -0.0, got {a:e}");
    let out = diff_call("E13 -0.0 lane0", PCM_LEN, 0, FILL, 1, &z, 0);
    assert_eq!(out[0], 0, "E13: -0.0 must map to 0 (s == 0 is not < 0)");

    // +0.0 for comparison.
    let out = diff_call("E13 +0.0 lane0", PCM_LEN, 0, FILL, 1, &zeros_z(), 0);
    assert_eq!(out[0], 0, "E13: +0.0 must map to 0");

    // Lane 1, same idea. The three negative weights flip the required sign:
    //   z[898]*104, z[770]*1567, z[642]*9727, z[514]*64019 need -0.0 taps,
    //   z[386]*-9975, z[258]*-45, z[2]*-5      need +0.0 taps,
    //   z[130]*146                             needs a -0.0 tap.
    let mut z = zeros_z();
    for i in [898usize, 770, 642, 514, 130] {
        z[i] = -0.0;
    }
    for i in [386usize, 258, 2] {
        z[i] = 0.0;
    }
    let a1 = model_lane1(&z);
    assert_eq!(a1.to_bits(), (-0.0f32).to_bits());
    let out = diff_call("E13 -0.0 lane1", PCM_LEN, 0, FILL, 2, &z, 0);
    assert_eq!(out[32], 0, "E13 lane1: -0.0 -> 0");
}

// ---------------------------------------------------------------------------
// E14 — subnormal / tiny accumulators
// ---------------------------------------------------------------------------

#[test]
fn err_e14_subnormals() {
    for v in [
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        1e-40f32,
        -1e-40f32,
        f32::from_bits(1),          // smallest positive subnormal
        f32::from_bits(0x8000_0001), // smallest negative subnormal
        f32::EPSILON,
        -f32::EPSILON,
    ] {
        let mut z = zeros_z();
        z[448] = v;
        let a = model_lane0(&z);
        let out = diff_call(&format!("E14 lane0 {v:e}"), PCM_LEN, 0, FILL, 1, &z, 0);
        assert_eq!(
            out[0],
            c_scale_pcm_reference(a),
            "E14: tiny accumulator {a:e} mishandled"
        );
        assert_eq!(out[0], 0, "E14: tiny accumulators truncate to 0");

        let mut z = zeros_z();
        z[514] = v;
        let out = diff_call(&format!("E14 lane1 {v:e}"), PCM_LEN, 0, FILL, 2, &z, 0);
        assert_eq!(out[32], 0, "E14 lane1: tiny accumulators truncate to 0");
    }
}

// ---------------------------------------------------------------------------
// E15 / E16 — NaN and infinity *inputs* propagating through the whole chain
// ---------------------------------------------------------------------------

#[test]
fn err_e15_nan_inputs_propagate() {
    let mut rng = Rng::new(0xE015);
    let taps = all_taps();
    for i in 0..5_000 {
        let mut z: Vec<f32> = (0..Z_MIN_LEN).map(|_| rng.signed_unit() * 0.3).collect();
        // Poison 1..=4 random taps with NaNs of random payloads.
        let n = 1 + rng.below(4);
        for _ in 0..n {
            let t = taps[rng.below(taps.len())];
            z[t] = f32::from_bits(0x7F80_0000 | (rng.next_u32() & 0x007F_FFFF) | ((rng.next_u32() & 1) << 31));
            if !z[t].is_nan() {
                z[t] = f32::NAN;
            }
        }
        let out = diff_call(&format!("E15 #{i}"), PCM_LEN, 0, FILL, 2, &z, 0);
        // Whichever lane's accumulator became NaN must have produced 0.
        if model_lane0(&z).is_nan() {
            assert_eq!(out[0], 0, "E15 #{i}: NaN lane-0 accumulator must give 0");
        }
        if model_lane1(&z).is_nan() {
            assert_eq!(out[32], 0, "E15 #{i}: NaN lane-1 accumulator must give 0");
        }
    }
}

#[test]
fn err_e16_infinity_inputs() {
    let mut rng = Rng::new(0xE016);
    let taps = all_taps();
    // Difference pairs: setting both to +inf makes `inf - inf == NaN`.
    const DIFF_PAIRS: [(usize, usize); 4] = [(896, 0), (768, 128), (640, 256), (512, 384)];
    for (a, b) in DIFF_PAIRS {
        for &v in &[f32::INFINITY, f32::NEG_INFINITY] {
            let mut z = zeros_z();
            z[a] = v;
            z[b] = v;
            assert!(model_lane0(&z).is_nan());
            let out = diff_call(&format!("E16 pair({a},{b}) {v}"), PCM_LEN, 0, FILL, 1, &z, 0);
            assert_eq!(out[0], 0, "E16: inf-inf must give NaN -> 0");
        }
    }
    // Sum pairs with opposite infinities: `inf + -inf == NaN`.
    const SUM_PAIRS: [(usize, usize); 3] = [(64, 832), (192, 704), (320, 576)];
    for (a, b) in SUM_PAIRS {
        let mut z = zeros_z();
        z[a] = f32::INFINITY;
        z[b] = f32::NEG_INFINITY;
        assert!(model_lane0(&z).is_nan());
        let out = diff_call(&format!("E16 sum({a},{b})"), PCM_LEN, 0, FILL, 1, &z, 0);
        assert_eq!(out[0], 0, "E16: inf + -inf must give NaN -> 0");
    }
    // Randomised mixtures of infinities.
    for i in 0..5_000 {
        let mut z: Vec<f32> = (0..Z_MIN_LEN).map(|_| rng.signed_unit()).collect();
        let n = 1 + rng.below(6);
        for _ in 0..n {
            let t = taps[rng.below(taps.len())];
            z[t] = if rng.next_u64() & 1 == 0 {
                f32::INFINITY
            } else {
                f32::NEG_INFINITY
            };
        }
        diff_call(&format!("E16 rand #{i}"), PCM_LEN, 0, FILL, 2, &z, 0);
    }
}

// ---------------------------------------------------------------------------
// E17 — `nch == 0`: both stores hit `pcm[0]`, lane 1 wins
// ---------------------------------------------------------------------------

#[test]
fn err_e17_nch_zero_aliasing() {
    let mut rng = Rng::new(0xE017);
    for i in 0..5_000 {
        let z: Vec<f32> = (0..Z_MIN_LEN).map(|_| rng.signed_unit() * 0.8).collect();
        let out = diff_call(&format!("E17 #{i}"), PCM_LEN, 0, FILL, 0, &z, 0);
        // Only pcm[0] may be touched.
        for (k, &v) in out.iter().enumerate().skip(1) {
            assert_eq!(v, FILL, "E17 #{i}: nch=0 wrote pcm[{k}]");
        }
        // And it must hold lane 1's value (the later store).
        let expect = c_scale_pcm_reference(model_lane1(&z));
        assert_eq!(
            out[0], expect,
            "E17 #{i}: nch=0 must leave lane 1's value in pcm[0]"
        );
    }
}

// ---------------------------------------------------------------------------
// E18 — negative `nch`
// ---------------------------------------------------------------------------

#[test]
fn err_e18_negative_nch() {
    let mut rng = Rng::new(0xE018);
    // Give the callee a pointer with headroom *before* it.
    const HEADROOM: usize = 16 * 8;
    for i in 0..4_000 {
        let nch: c_int = -1 - (i % 8) as c_int; // -1 ..= -8
        let z: Vec<f32> = (0..Z_MIN_LEN).map(|_| rng.signed_unit() * 0.8).collect();
        let out = diff_call(
            &format!("E18 #{i} nch={nch}"),
            HEADROOM + PCM_LEN,
            HEADROOM,
            FILL,
            nch,
            &z,
            0,
        );
        let lane1_at = HEADROOM as isize + 16 * nch as isize;
        assert!(lane1_at >= 0);
        assert_eq!(
            out[lane1_at as usize],
            c_scale_pcm_reference(model_lane1(&z)),
            "E18 #{i}: negative nch resolved to the wrong slot"
        );
        assert_eq!(
            out[HEADROOM],
            c_scale_pcm_reference(model_lane0(&z)),
            "E18 #{i}: lane 0 wrong"
        );
    }
}

// ---------------------------------------------------------------------------
// E19 / E20 — `16 * nch` overflowing `int`, and the `int` extremes
// ---------------------------------------------------------------------------

#[test]
fn err_e19_nch_index_wraparound() {
    let mut rng = Rng::new(0xE019);
    const HEADROOM: usize = 32;

    // `nch` values whose `int` product `16 * nch` wraps around to a small,
    // still-addressable element offset. Verified against the emitted
    // `shl $0x4,%eax; cltq` sequence:
    //   0x1000_0000 (2^28) -> 16 * 2^28 == 2^32          -> 0
    //   0x2000_0000 (2^29) -> 16 * 2^29 == 2^33          -> 0
    //   0x0FFF_FFFF        -> 16 * (2^28 - 1) == 2^32-16 -> -16
    //   0x1000_0001        -> 16 * (2^28 + 1) == 2^32+16 -> +16
    //   0x1000_0002        ->                            -> +32
    let cases: [(c_int, isize); 5] = [
        (0x1000_0000, 0),
        (0x2000_0000, 0),
        (0x0FFF_FFFF, -16),
        (0x1000_0001, 16),
        (0x1000_0002, 32),
    ];
    for (nch, expect_off) in cases {
        // Cross-check the expectation against C's `int` semantics.
        assert_eq!(
            16i32.wrapping_mul(nch) as isize,
            expect_off,
            "E19: bad expectation for nch=0x{nch:08x}"
        );
        for i in 0..200 {
            let z: Vec<f32> = (0..Z_MIN_LEN).map(|_| rng.signed_unit() * 0.8).collect();
            let out = diff_call(
                &format!("E19 nch=0x{nch:08x} #{i}"),
                HEADROOM + PCM_LEN,
                HEADROOM,
                FILL,
                nch,
                &z,
                0,
            );
            let slot = (HEADROOM as isize + expect_off) as usize;
            let lane1 = c_scale_pcm_reference(model_lane1(&z));
            assert_eq!(
                out[slot], lane1,
                "E19: nch=0x{nch:08x} must resolve the lane-1 store to \
                 pcm[{expect_off}]"
            );
            // Nothing outside {pcm[0], pcm[expect_off]} may be written.
            for (k, &v) in out.iter().enumerate() {
                if k != HEADROOM && k != slot {
                    assert_eq!(v, FILL, "E19: nch=0x{nch:08x} wrote out[{k}]");
                }
            }
        }
    }
}

#[test]
fn err_e20_nch_int_extremes() {
    let mut rng = Rng::new(0xE020);
    // `nch` in `-8..=8` needs 16*8 = 128 elements of headroom before the
    // pointer we hand over, and the same amount after it.
    const HEADROOM: usize = 16 * 8;
    const TOTAL: usize = HEADROOM + 16 * 8 + 32;

    // Every extreme / meaningless `int` that still resolves to an addressable
    // slot, together with the element offset C's `shl $4; cltq` produces.
    let cases: [(c_int, isize); 12] = [
        (i32::MIN, 0),      // -2^31 * 16 == -2^35 -> 0 (mod 2^32)
        (i32::MAX, -16),    //  (2^31-1)*16       -> -16
        (i32::MIN + 1, 16), //  (-2^31+1)*16      -> +16
        (-8, -128),
        (-3, -48),
        (-1, -16),
        (0, 0),
        (1, 16),
        (2, 32),
        (3, 48),
        (7, 112),
        (8, 128),
    ];

    for (nch, expect_off) in cases {
        // Cross-check the expectation against C's `int` semantics.
        assert_eq!(
            16i32.wrapping_mul(nch) as isize,
            expect_off,
            "E20: bad expectation for nch={nch}"
        );
        let slot = (HEADROOM as isize + expect_off) as usize;
        assert!(slot < TOTAL, "E20: slot {slot} out of the test buffer");

        for i in 0..300 {
            let z: Vec<f32> = (0..Z_MIN_LEN).map(|_| rng.signed_unit() * 0.8).collect();
            let out = diff_call(
                &format!("E20 nch={nch} #{i}"),
                TOTAL,
                HEADROOM,
                FILL,
                nch,
                &z,
                0,
            );
            let lane0 = c_scale_pcm_reference(model_lane0(&z));
            let lane1 = c_scale_pcm_reference(model_lane1(&z));
            assert_eq!(
                out[slot], lane1,
                "E20: nch={nch} must resolve the lane-1 store to pcm[{expect_off}]"
            );
            if slot != HEADROOM {
                assert_eq!(out[HEADROOM], lane0, "E20: lane 0 wrong for nch={nch}");
            }
            for (k, &v) in out.iter().enumerate() {
                if k != HEADROOM && k != slot {
                    assert_eq!(v, FILL, "E20: nch={nch} wrote out[{k}]");
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E21 — null pointers: both must fault with the same signal
// ---------------------------------------------------------------------------

#[test]
fn err_e21_null_pointers_both_segfault() {
    if let Some(mode) = child_mode() {
        // Child: perform the faulting call and never return.
        let p = pair();
        let z = zeros_z();
        let mut pcm = vec![0i16; PCM_LEN];
        match mode.as_str() {
            "c_null_pcm" => unsafe {
                (p.c.synth_pair)(std::ptr::null_mut(), 2, z.as_ptr());
            },
            "rust_null_pcm" => unsafe {
                (p.rust.synth_pair)(std::ptr::null_mut(), 2, z.as_ptr());
            },
            "c_null_z" => unsafe {
                (p.c.synth_pair)(pcm.as_mut_ptr(), 2, std::ptr::null());
            },
            "rust_null_z" => unsafe {
                (p.rust.synth_pair)(pcm.as_mut_ptr(), 2, std::ptr::null());
            },
            "c_null_both" => unsafe {
                (p.c.synth_pair)(std::ptr::null_mut(), 2, std::ptr::null());
            },
            "rust_null_both" => unsafe {
                (p.rust.synth_pair)(std::ptr::null_mut(), 2, std::ptr::null());
            },
            other => panic!("unknown child mode {other}"),
        }
        // If we get here the call did not fault; report it via a distinct code.
        println!("child {mode}: survived");
        std::process::exit(77);
    }

    let name = "err_e21_null_pointers_both_segfault";
    for (c_mode, r_mode) in [
        ("c_null_pcm", "rust_null_pcm"),
        ("c_null_z", "rust_null_z"),
        ("c_null_both", "rust_null_both"),
    ] {
        let c = run_child(name, c_mode);
        let r = run_child(name, r_mode);
        assert_eq!(
            c, r,
            "E21: {c_mode} gave (signal, code) = {c:?} but {r_mode} gave {r:?}"
        );
        assert_eq!(
            c.0,
            Some(11),
            "E21: expected SIGSEGV (11) for {c_mode}, got {c:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// E22 — the minimum legal read extent for `z` (899 floats)
// ---------------------------------------------------------------------------

#[test]
fn err_e22_minimum_legal_z_extent() {
    let mut rng = Rng::new(0xE022);
    for i in 0..4_000 {
        let base: Vec<f32> = (0..Z_MIN_LEN).map(|_| rng.signed_unit()).collect();
        // Reference run on an exactly-sized buffer.
        let tight = diff_call(&format!("E22 tight #{i}"), PCM_LEN, 0, FILL, 2, &base, 0);

        // Same 899 values, then 128 floats of arbitrary garbage past the end.
        let mut padded = base.clone();
        for _ in 0..128 {
            padded.push(rng.any_bits_f32());
        }
        let padded_out = diff_call(&format!("E22 padded #{i}"), PCM_LEN, 0, FILL, 2, &padded, 0);
        assert_eq!(
            tight, padded_out,
            "E22 #{i}: something past z[898] influenced the result"
        );
    }
}

// ---------------------------------------------------------------------------
// E23 — `pcm` aliasing the `z` buffer (no `restrict` in the C signature)
// ---------------------------------------------------------------------------

#[test]
fn err_e23_aliased_pcm_and_z() {
    let mut rng = Rng::new(0xE023);
    for i in 0..3_000 {
        // Keep the alias inside the buffer for every write: pcm[0] and
        // pcm[16*nch] must land within `Z_MIN_LEN` floats (2 i16 per float).
        let nch: c_int = 1 + (i % 2) as c_int;
        let max_i16 = Z_MIN_LEN * 2;
        let need = 16 * nch as usize + 1;
        let alias_at = rng.below((max_i16 - need) / 2);
        let z: Vec<f32> = (0..Z_MIN_LEN).map(|_| rng.signed_unit() * 0.6).collect();
        diff_call_aliased(&format!("E23 #{i} nch={nch}"), nch, &z, alias_at);
    }
}

// ---------------------------------------------------------------------------
// E24 / E25 — misaligned `z` and `pcm` pointers.
//
// The C signature promises nothing about alignment and GCC emits plain
// `movss` / 16-bit stores, which tolerate any address on x86-64. A Rust
// translation that let the compiler assume alignment (or that kept rustc's
// debug-assertion alignment check) would diverge here, so both cases are
// exercised across randomized data.
// ---------------------------------------------------------------------------

/// Calls both `.so`s with `z` deliberately misaligned by `byte_off` bytes and
/// compares the whole `pcm` buffer.
fn diff_misaligned_z(ctx: &str, nch: c_int, values: &[f32], byte_off: usize) -> Vec<i16> {
    let p = pair();
    assert!(values.len() >= Z_MIN_LEN);

    // Over-allocate bytes and place the float payload at an odd byte offset.
    let mut raw = vec![0u8; byte_off + values.len() * 4 + 8];
    for (i, v) in values.iter().enumerate() {
        raw[byte_off + i * 4..byte_off + i * 4 + 4].copy_from_slice(&v.to_le_bytes());
    }
    let z_ptr = unsafe { raw.as_ptr().add(byte_off) } as *const f32;
    assert_eq!(
        z_ptr as usize % 4,
        byte_off % 4,
        "{ctx}: expected a misaligned pointer"
    );

    let mut out_c = vec![FILL; PCM_LEN];
    let mut out_r = vec![FILL; PCM_LEN];
    unsafe {
        (p.c.synth_pair)(out_c.as_mut_ptr(), nch, z_ptr);
        (p.rust.synth_pair)(out_r.as_mut_ptr(), nch, z_ptr);
    }
    assert_eq!(out_c, out_r, "{ctx}: misaligned-z divergence");
    out_c
}

#[test]
fn err_e24_misaligned_z_pointer() {
    let mut rng = Rng::new(0xE024);
    for i in 0..3_000 {
        let byte_off = 1 + (i % 3); // 1, 2, 3 -> never 4-byte aligned
        let nch: c_int = 1 + (i % 2) as c_int;
        let values: Vec<f32> = (0..Z_MIN_LEN)
            .map(|_| match rng.below(4) {
                0 => rng.signed_unit() * 0.7,
                1 => rng.wide_exponent_f32(-20, 20),
                2 => BOUNDARY_POOL[rng.below(BOUNDARY_POOL.len())],
                _ => rng.any_bits_f32(),
            })
            .collect();
        let out = diff_misaligned_z(
            &format!("E24 #{i} off={byte_off}"),
            nch,
            &values,
            byte_off,
        );
        // The result must equal the aligned-buffer result for the same values.
        let aligned = diff_call(&format!("E24 aligned #{i}"), PCM_LEN, 0, FILL, nch, &values, 0);
        assert_eq!(
            out, aligned,
            "E24 #{i}: alignment changed the result (off={byte_off})"
        );
    }
}

#[test]
fn err_e25_misaligned_pcm_pointer() {
    let p = pair();
    let mut rng = Rng::new(0xE025);
    for i in 0..3_000 {
        let byte_off = 1; // odd address -> `mp3d_sample_t*` is misaligned
        let nch: c_int = 1 + (i % 2) as c_int;
        let z: Vec<f32> = (0..Z_MIN_LEN).map(|_| rng.signed_unit() * 0.7).collect();

        let n_bytes = PCM_LEN * 2 + 8;
        let mut raw_c = vec![0xA5u8; byte_off + n_bytes];
        let mut raw_r = vec![0xA5u8; byte_off + n_bytes];
        unsafe {
            (p.c.synth_pair)(
                raw_c.as_mut_ptr().add(byte_off) as *mut i16,
                nch,
                z.as_ptr(),
            );
            (p.rust.synth_pair)(
                raw_r.as_mut_ptr().add(byte_off) as *mut i16,
                nch,
                z.as_ptr(),
            );
        }
        assert_eq!(raw_c, raw_r, "E25 #{i}: misaligned-pcm divergence");

        // Cross-check the two stored samples against the reference replay.
        let lane0 = c_scale_pcm_reference(model_lane0(&z)).to_le_bytes();
        let lane1 = c_scale_pcm_reference(model_lane1(&z)).to_le_bytes();
        assert_eq!(&raw_c[byte_off..byte_off + 2], &lane0, "E25 #{i}: lane 0");
        let off1 = byte_off + 32 * nch as usize;
        assert_eq!(&raw_c[off1..off1 + 2], &lane1, "E25 #{i}: lane 1");
    }
}
