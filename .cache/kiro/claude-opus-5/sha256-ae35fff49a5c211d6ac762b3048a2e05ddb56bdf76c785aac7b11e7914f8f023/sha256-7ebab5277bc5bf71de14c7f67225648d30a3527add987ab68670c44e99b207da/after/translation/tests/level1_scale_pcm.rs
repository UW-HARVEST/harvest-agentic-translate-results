//! Level 1: `mp3d_scale_pcm` semantics.
//!
//! `mp3d_scale_pcm` is `static` in C, so it can only be exercised through
//! `synth_pair`. With all of `z` zeroed except a single tap we get exact
//! control over the accumulator:
//!
//! * `pcm[0]`        <- scale(fl(z[448] * 75038))   (tap `z[7*64]`, weight 75038)
//! * `pcm[16*nch]`   <- scale(fl(z[2]   * -5))      (tap `z[0*64]` after `z += 2`)
//!
//! All other terms contribute exact `0.0`, so no extra rounding creeps in and
//! the two halves are independent (index 448 is not a second-half tap and
//! index 2 is not a first-half tap).

mod common;
use common::*;

/// Drive `pcm[0]` so the accumulator equals `fl(v * 75038)`.
fn z_for_first(v: f32) -> Vec<f32> {
    let mut z = vec![0.0f32; Z_LEN];
    z[448] = v;
    z
}

/// Drive `pcm[16*nch]` so the accumulator equals `fl(v * -5)`.
fn z_for_second(v: f32) -> Vec<f32> {
    let mut z = vec![0.0f32; Z_LEN];
    z[2] = v;
    z
}

#[test]
fn scale_pcm_all_zero() {
    let p = Pair::load();
    let z = vec![0.0f32; Z_LEN];
    for nch in [1, 2] {
        p.check(&z, nch, "all zero");
    }
}

/// Dense sweep of the accumulator across the whole `int16_t` range and beyond,
/// hitting every `x.5` rounding point and both clipping thresholds.
#[test]
fn scale_pcm_dense_integer_and_half_sweep() {
    let p = Pair::load();
    // Steps of 1/4 across [-33000, 33000] => 264001 probes per half.
    let mut n = 0u32;
    let mut t = -33000.0f64;
    while t <= 33000.0 {
        let v1 = (t / 75038.0) as f32;
        p.check(&z_for_first(v1), 1, "sweep first");
        let v2 = (t / -5.0) as f32;
        p.check(&z_for_second(v2), 1, "sweep second");
        t += 0.25;
        n += 1;
    }
    assert!(n > 200_000, "sweep too coarse: {n}");
}

/// ULP-level sweep around the two hard clipping thresholds (`32766.5` and
/// `-32767.5`) plus around zero and `±0.5`, where `(int16_t)(sample + .5f)`
/// and the `s -= (s < 0)` fixup interact.
#[test]
fn scale_pcm_ulp_sweep_around_thresholds() {
    let p = Pair::load();
    let interesting: [f64; 12] = [
        32766.5, -32767.5, 32767.0, -32768.0, 0.0, 0.5, -0.5, 1.0, -1.0, 32766.0,
        -32767.0, 16383.5,
    ];
    for &target in &interesting {
        for (weight, idx) in [(75038.0f64, 448usize), (-5.0f64, 2usize)] {
            let base = (target / weight) as f32;
            let bits = base.to_bits();
            for delta in -3000i64..=3000 {
                let b = (bits as i64 + delta) as u32;
                let v = f32::from_bits(b);
                if !v.is_finite() {
                    continue;
                }
                let mut z = vec![0.0f32; Z_LEN];
                z[idx] = v;
                p.check(&z, 1, "ulp sweep");
            }
        }
    }
}

/// Every possible `i16` result must be produced by at least one probe, and both
/// implementations must agree; this doubles as a coverage assertion.
#[test]
fn scale_pcm_covers_full_i16_range() {
    let p = Pair::load();
    let mut seen_min = i16::MAX;
    let mut seen_max = i16::MIN;
    let mut t = -32800.0f64;
    while t <= 32800.0 {
        let z = z_for_first((t / 75038.0) as f32);
        let (c, r) = p.run(&z, 1);
        assert_eq!(c, r, "mismatch at accumulator target {t}");
        seen_min = seen_min.min(c[0]);
        seen_max = seen_max.max(c[0]);
        t += 0.5;
    }
    assert_eq!(seen_min, i16::MIN, "clip-low path never reached");
    assert_eq!(seen_max, i16::MAX, "clip-high path never reached");
}

/// Special float values. `±inf` is fully defined by the C source (the
/// comparisons catch it); NaN and out-of-range values are checked too so any
/// divergence in the generated code is caught rather than assumed away.
#[test]
fn scale_pcm_special_values() {
    let p = Pair::load();
    let specials: [f32; 16] = [
        0.0,
        -0.0,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::from_bits(1), // smallest subnormal
        f32::from_bits(0x8000_0001),
        1.0,
        -1.0,
        f32::MAX,
        f32::MIN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7FC0_1234), // another quiet NaN payload
        f32::from_bits(0x7F80_0001), // signalling NaN
    ];
    for &v in &specials {
        for nch in [1, 2] {
            p.check(&z_for_first(v), nch, "special first");
            p.check(&z_for_second(v), nch, "special second");
            // and with the special value on every tap at once
            let mut z = vec![0.0f32; Z_LEN];
            for &t in TAPS.iter().chain(TAPS2.iter()) {
                z[t] = v;
            }
            p.check(&z, nch, "special all taps");
        }
    }
}

/// Hit the clipping thresholds *exactly*, with no rounding anywhere.
///
/// The second-half accumulator is a plain chain of adds, so picking weights
/// whose products are exactly representable gives bit-exact control:
///
/// * `z[130] * 146` and `z[2] * -5` are exact for the values below, and the
///   final sum is exactly representable in `f32`.
#[test]
fn scale_pcm_exact_thresholds() {
    let p = Pair::load();

    // 146*224 = 32704 (exact); -12.5 * -5 = 62.5 (exact); 32704 + 62.5 = 32766.5
    let mut z = vec![0.0f32; Z_LEN];
    z[130] = 224.0;
    z[2] = -12.5;
    let (c, r) = p.run(&z, 1);
    assert_eq!(c, r, "exact +threshold mismatch");
    assert_eq!(c[16], 32767, "sample == 32766.5 must clip to i16::MAX");

    // 146*(-224) = -32704 (exact); 12.7 is not exact, so use -5 * 12.7 -> pick
    // -63.5: 146*(-224) = -32704, then -32767.5 - (-32704) = -63.5,
    // and -63.5 = 12.7 * -5 is inexact, so use z[2] = 12.7? Instead reach
    // -32767.5 as -32704 + (-63.5) with -63.5 = (-12.7)*5 ... use weight -45:
    // -45 * 1.4111.. is inexact. Use z[2] = 12.7 replaced by exact 63.5/-5:
    // 63.5 / -5 = -12.7 (inexact). So build -63.5 from z[130] instead:
    // 146 * x is a multiple of 146 scaled by powers of two; -63.5 is not.
    // Use the -5 tap alone with an exact quotient: -32767.5 / -5 = 6553.5,
    // and 6553.5 * -5 = -32767.5 exactly (both are exactly representable).
    let mut z = vec![0.0f32; Z_LEN];
    z[2] = 6553.5;
    let (c, r) = p.run(&z, 1);
    assert_eq!(c, r, "exact -threshold mismatch");
    assert_eq!(c[16], -32768, "sample == -32767.5 must clip to i16::MIN");

    // Exactly one ULP inside each threshold, so the non-clipping path runs.
    for &v in &[6553.5f32, -6553.3f32] {
        let bits = v.to_bits();
        for d in -4i64..=4 {
            let mut z = vec![0.0f32; Z_LEN];
            z[2] = f32::from_bits((bits as i64 + d) as u32);
            p.check(&z, 1, "exact threshold neighbourhood");
        }
    }

    // Exact half-way values across the whole range via the -5 weight:
    // (k + 0.5) / -5 is exact whenever (2k+1)/10 is representable; instead
    // sweep exact multiples of 0.5 by using z[2] = -(k as f32 + 0.5) / 5.0
    // only when the round-trip is exact.
    for k in -32770i32..=32770 {
        let target = k as f32 + 0.5;
        let x = target / -5.0;
        if x * -5.0 == target {
            let mut z = vec![0.0f32; Z_LEN];
            z[2] = x;
            p.check(&z, 1, "exact half-way");
        }
    }
}

/// Exact integer accumulator values (`.0`), where `sample + .5f` lands exactly
/// on `x.5` and truncation-toward-zero plus the `s -= (s < 0)` fixup decide the
/// result.
#[test]
fn scale_pcm_exact_integers() {
    let p = Pair::load();
    for k in -32800i32..=32800 {
        let target = k as f32;
        let x = target / -5.0;
        if x * -5.0 == target {
            let mut z = vec![0.0f32; Z_LEN];
            z[2] = x;
            p.check(&z, 1, "exact integer");
        }
        let x = target / 146.0;
        if x * 146.0 == target {
            let mut z = vec![0.0f32; Z_LEN];
            z[130] = x;
            p.check(&z, 1, "exact integer 146");
        }
    }
}
