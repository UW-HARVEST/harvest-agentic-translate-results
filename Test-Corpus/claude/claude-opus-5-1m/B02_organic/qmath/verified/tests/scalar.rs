//! Differential tests for the scalar entry points of `q_math.c`
//! (CONFIGS.md rows 1-16).
//!
//! Every call goes through `dlsym` on both shared objects.

mod harness;

use core::ffi::{c_int, c_uint};
use harness::*;

type F1 = unsafe extern "C" fn(f32) -> f32;
type F2 = unsafe extern "C" fn(f32, f32) -> f32;
type F3 = unsafe extern "C" fn(f32, f32, f32) -> f32;

// ---------------------------------------------------------------------------
// Q_rand / Q_random / Q_crandom  -- stateful through an int* seed
// ---------------------------------------------------------------------------

#[test]
fn q_rand_sequences() {
    type F = unsafe extern "C" fn(*mut c_int) -> c_int;
    let (c, r): (F, F) = both("Q_rand");

    // every interesting seed, then long random sequences: `69069 * seed + 1`
    // overflows a signed int, which is UB in C but wraps with gcc.
    let mut seeds: Vec<i32> = vec![
        0,
        1,
        -1,
        2,
        -2,
        1234,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        0x5f3759df,
        -0x5f3759df,
    ];
    let mut rng = Rng::new(0x9E3779B97F4A7C15);
    for _ in 0..64 {
        seeds.push(rng.i32_any());
    }

    for s in seeds {
        let mut sc = s;
        let mut sr = s;
        for step in 0..200 {
            let vc = unsafe { c(&mut sc) };
            let vr = unsafe { r(&mut sr) };
            assert_int(&format!("Q_rand(seed={s}) step {step} result"), vc, vr);
            assert_int(&format!("Q_rand(seed={s}) step {step} *seed"), sc, sr);
        }
    }
}

#[test]
fn q_random_and_crandom_sequences() {
    type F = unsafe extern "C" fn(*mut c_int) -> f32;
    let (crandom_c, crandom_r): (F, F) = both("Q_crandom");
    let (random_c, random_r): (F, F) = both("Q_random");

    let mut rng = Rng::new(2);
    let mut seeds: Vec<i32> = vec![0, 1, -1, i32::MAX, i32::MIN, 42];
    for _ in 0..64 {
        seeds.push(rng.i32_any());
    }

    for s in seeds {
        let (mut sc, mut sr) = (s, s);
        for step in 0..100 {
            let vc = unsafe { random_c(&mut sc) };
            let vr = unsafe { random_r(&mut sr) };
            assert_f32(&format!("Q_random(seed={s}) step {step}"), vc, vr);
            assert_int(&format!("Q_random(seed={s}) step {step} *seed"), sc, sr);
        }
        let (mut sc, mut sr) = (s, s);
        for step in 0..100 {
            let vc = unsafe { crandom_c(&mut sc) };
            let vr = unsafe { crandom_r(&mut sr) };
            assert_f32(&format!("Q_crandom(seed={s}) step {step}"), vc, vr);
            assert_int(&format!("Q_crandom(seed={s}) step {step} *seed"), sc, sr);
        }
    }
}

// ---------------------------------------------------------------------------
// ClampChar / ClampShort  -- full boundary sweep
// ---------------------------------------------------------------------------

#[test]
fn clamp_char() {
    type F = unsafe extern "C" fn(c_int) -> i8;
    let (c, r): (F, F) = both("ClampChar");

    let mut vals: Vec<i32> = (-300..=300).collect();
    vals.extend([i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1, -129, -128, 127, 128]);
    let mut rng = Rng::new(3);
    for _ in 0..2000 {
        vals.push(rng.i32_any());
    }
    for v in vals {
        assert_int(&format!("ClampChar({v})"), unsafe { c(v) }, unsafe { r(v) });
    }
}

#[test]
fn clamp_short() {
    type F = unsafe extern "C" fn(c_int) -> i16;
    let (c, r): (F, F) = both("ClampShort");

    let mut vals: Vec<i32> = (-40000..=-32000).step_by(7).collect();
    vals.extend((32000..=40000).step_by(7));
    vals.extend([
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
        -32769,
        -32768,
        32767,
        32768,
        0,
    ]);
    let mut rng = Rng::new(4);
    for _ in 0..2000 {
        vals.push(rng.i32_any());
    }
    for v in vals {
        assert_int(&format!("ClampShort({v})"), unsafe { c(v) }, unsafe { r(v) });
    }
}

// ---------------------------------------------------------------------------
// Q_rsqrt / Q_fabs
// ---------------------------------------------------------------------------

#[test]
fn q_rsqrt_all_shapes() {
    let (c, r): (F1, F1) = both("Q_rsqrt");
    let mut rng = Rng::new(5);

    for &v in INTERESTING {
        assert_f32(&format!("Q_rsqrt({v:?})"), unsafe { c(v) }, unsafe { r(v) });
    }
    for _ in 0..20000 {
        let v = rng.f32_any();
        assert_f32(&format!("Q_rsqrt(0x{:08x})", v.to_bits()), unsafe { c(v) }, unsafe {
            r(v)
        });
    }
    // exhaustive-ish sweep of the exponent range, mantissa 0 and mantissa max
    for exp in 0u32..=255 {
        for mant in [0u32, 1, 0x400000, 0x7fffff] {
            for sign in [0u32, 1] {
                let v = f32::from_bits((sign << 31) | (exp << 23) | mant);
                assert_f32(
                    &format!("Q_rsqrt(0x{:08x})", v.to_bits()),
                    unsafe { c(v) },
                    unsafe { r(v) },
                );
            }
        }
    }
}

#[test]
fn q_fabs_all_shapes() {
    let (c, r): (F1, F1) = both("Q_fabs");
    let mut rng = Rng::new(6);

    for &v in INTERESTING {
        assert_f32(&format!("Q_fabs({v:?})"), unsafe { c(v) }, unsafe { r(v) });
    }
    // Q_fabs is pure bit manipulation, so any bit pattern (including every
    // NaN payload) must round-trip identically.
    for _ in 0..20000 {
        let v = f32::from_bits(rng.next_u32());
        assert_f32(&format!("Q_fabs(0x{:08x})", v.to_bits()), unsafe { c(v) }, unsafe {
            r(v)
        });
    }
}

// ---------------------------------------------------------------------------
// angle helpers
// ---------------------------------------------------------------------------

#[test]
fn lerp_angle() {
    let (c, r): (F3, F3) = both("LerpAngle");
    let mut rng = Rng::new(7);

    let interesting = [
        0.0f32, 1.0, -1.0, 180.0, -180.0, 180.000_01, -180.000_01, 179.999_98, 360.0, -360.0,
        90.0, 270.0, 1e30, -1e30, f32::INFINITY, f32::NEG_INFINITY, f32::NAN, -0.0,
    ];
    for &from in &interesting {
        for &to in &interesting {
            for &frac in &[0.0f32, 0.5, 1.0, -1.0, 2.0, f32::NAN, f32::INFINITY] {
                check_f32(
                    &format!("LerpAngle({from:?},{to:?},{frac:?})"),
                    &[from, to, frac],
                    unsafe { c(from, to, frac) },
                    unsafe { r(from, to, frac) },
                );
            }
        }
    }
    for _ in 0..20000 {
        let (from, to, frac) = (rng.f32_any(), rng.f32_any(), rng.f32_any());
        check_f32(
            &format!("LerpAngle({from:?},{to:?},{frac:?})"),
            &[from, to, frac],
            unsafe { c(from, to, frac) },
            unsafe { r(from, to, frac) },
        );
    }
}

/// `AngleSubtract` loops `while (a > 180) a -= 360;`, so it only terminates for
/// arguments whose difference stays small enough that subtracting 360 changes
/// the value (see ERRORS.md row 13).  Everything in range is compared here.
#[test]
fn angle_subtract() {
    let (c, r): (F2, F2) = both("AngleSubtract");
    let mut rng = Rng::new(8);

    let interesting = [
        0.0f32, -0.0, 1.0, -1.0, 180.0, -180.0, 180.000_01, -180.000_01, 360.0, -360.0, 359.0,
        -359.0, 540.0, -540.0, 720.0, 1e6, -1e6, f32::NAN,
    ];
    for &a1 in &interesting {
        for &a2 in &interesting {
            if !(a1 - a2).is_nan() && (a1 - a2).abs() > 1e7 {
                continue; // would spin for a very long time in BOTH impls
            }
            assert_f32(
                &format!("AngleSubtract({a1:?},{a2:?})"),
                unsafe { c(a1, a2) },
                unsafe { r(a1, a2) },
            );
        }
    }
    for _ in 0..20000 {
        let (a1, a2) = (rng.f32_mag(1e5), rng.f32_mag(1e5));
        assert_f32(
            &format!("AngleSubtract({a1:?},{a2:?})"),
            unsafe { c(a1, a2) },
            unsafe { r(a1, a2) },
        );
    }
}

#[test]
fn angles_subtract() {
    type F = unsafe extern "C" fn(*mut f32, *mut f32, *mut f32) -> ();
    let (c, r): (F, F) = both("AnglesSubtract");
    let mut rng = Rng::new(9);

    for _ in 0..5000 {
        let v1 = rng.vec3_mag(1e5);
        let v2 = rng.vec3_mag(1e5);
        let mut oc = [0.0f32; 3];
        let mut or_ = [0.0f32; 3];
        let (mut a, mut b) = (v1, v2);
        unsafe { c(a.as_mut_ptr(), b.as_mut_ptr(), oc.as_mut_ptr()) };
        let (mut a, mut b) = (v1, v2);
        unsafe { r(a.as_mut_ptr(), b.as_mut_ptr(), or_.as_mut_ptr()) };
        assert_vec(&format!("AnglesSubtract({v1:?},{v2:?})"), &oc, &or_);
    }
}

#[test]
fn angle_mod_normalize_delta() {
    let (mod_c, mod_r): (F1, F1) = both("AngleMod");
    let (n360_c, n360_r): (F1, F1) = both("AngleNormalize360");
    let (n180_c, n180_r): (F1, F1) = both("AngleNormalize180");
    let (delta_c, delta_r): (F2, F2) = both("AngleDelta");
    let mut rng = Rng::new(10);

    // The `(int)` cast inside these functions overflows for |angle| >~ 1.18e7,
    // which is exactly where gcc's cvttsd2si returns 0x80000000; the sweep and
    // the INTERESTING pool both cross that boundary.
    let mut vals: Vec<f32> = INTERESTING.to_vec();
    vals.extend([
        11796480.0,   // 2^31 / 182.044...
        11796479.0,
        11796481.0,
        -11796480.0,
        -11796481.0,
        1.1796e7,
        2.0e7,
        -2.0e7,
        359.999_97,
        360.000_03,
        180.0,
        180.000_01,
        -180.0,
        1e-7,
    ]);
    for _ in 0..20000 {
        vals.push(match rng.below(3) {
            0 => rng.f32_any(),
            1 => rng.f32_mag(720.0),
            _ => rng.f32_mag(2.0e7),
        });
    }

    for v in vals {
        let ctx = format!("({v:?} / 0x{:08x})", v.to_bits());
        assert_f32(&format!("AngleMod{ctx}"), unsafe { mod_c(v) }, unsafe { mod_r(v) });
        assert_f32(
            &format!("AngleNormalize360{ctx}"),
            unsafe { n360_c(v) },
            unsafe { n360_r(v) },
        );
        assert_f32(
            &format!("AngleNormalize180{ctx}"),
            unsafe { n180_c(v) },
            unsafe { n180_r(v) },
        );
    }

    for _ in 0..20000 {
        let (a, b) = (rng.f32_any(), rng.f32_any());
        assert_f32(
            &format!("AngleDelta({a:?},{b:?})"),
            unsafe { delta_c(a, b) },
            unsafe { delta_r(a, b) },
        );
    }
}

// ---------------------------------------------------------------------------
// Q_log2
// ---------------------------------------------------------------------------

/// Negative inputs make the C loop spin forever (`-1 >> 1 == -1`), see
/// ERRORS.md row 20 -- only non-negative values are compared.
#[test]
fn q_log2_non_negative() {
    type F = unsafe extern "C" fn(c_int) -> c_int;
    let (c, r): (F, F) = both("Q_log2");

    let mut vals: Vec<i32> = (0..=1024).collect();
    for b in 0..31 {
        vals.push(1 << b);
        vals.push((1 << b) - 1);
        vals.push((1 << b) + 1);
    }
    vals.push(i32::MAX);
    let mut rng = Rng::new(11);
    for _ in 0..2000 {
        vals.push((rng.next_u32() >> 1) as i32);
    }
    for v in vals {
        if v < 0 {
            continue;
        }
        assert_int(&format!("Q_log2({v})"), unsafe { c(v) }, unsafe { r(v) });
    }
}

// ---------------------------------------------------------------------------
// ColorBytes3 / ColorBytes4
// ---------------------------------------------------------------------------

/// `ColorBytes3` never initialises the top byte of its result (see ERRORS.md
/// row 6), so only the 24 bits it does write are compared.
#[test]
fn color_bytes3() {
    type F = unsafe extern "C" fn(f32, f32, f32) -> c_uint;
    let (c, r): (F, F) = both("ColorBytes3");
    let mut rng = Rng::new(12);

    for _ in 0..20000 {
        let (a, b, d) = (rng.f32_any(), rng.f32_any(), rng.f32_any());
        let vc = unsafe { c(a, b, d) } & 0x00ff_ffff;
        let vr = unsafe { r(a, b, d) } & 0x00ff_ffff;
        assert_int(&format!("ColorBytes3({a:?},{b:?},{d:?})"), vc, vr);
    }
    for &a in INTERESTING {
        for &b in &[0.0f32, 1.0, 0.5, -1.0, 1e30, f32::NAN] {
            let vc = unsafe { c(a, b, a) } & 0x00ff_ffff;
            let vr = unsafe { r(a, b, a) } & 0x00ff_ffff;
            assert_int(&format!("ColorBytes3({a:?},{b:?},{a:?})"), vc, vr);
        }
    }
}

#[test]
fn color_bytes4() {
    type F = unsafe extern "C" fn(f32, f32, f32, f32) -> c_uint;
    let (c, r): (F, F) = both("ColorBytes4");
    let mut rng = Rng::new(13);

    for _ in 0..20000 {
        let (a, b, d, e) = (rng.f32_any(), rng.f32_any(), rng.f32_any(), rng.f32_any());
        assert_int(
            &format!("ColorBytes4({a:?},{b:?},{d:?},{e:?})"),
            unsafe { c(a, b, d, e) },
            unsafe { r(a, b, d, e) },
        );
    }
    // the exact 0..255 ramp used by real callers, plus the rounding boundaries
    for i in 0..=255 {
        let v = i as f32 / 255.0;
        assert_int(
            &format!("ColorBytes4(ramp {i})"),
            unsafe { c(v, v, v, v) },
            unsafe { r(v, v, v, v) },
        );
    }
    for &v in &[
        -0.001f32, 0.0, 0.001, 0.5, 0.999, 1.0, 1.001, 1.5, 255.0, 256.0, 1e9, -1e9, f32::NAN,
        f32::INFINITY, f32::NEG_INFINITY,
    ] {
        assert_int(
            &format!("ColorBytes4({v:?})"),
            unsafe { c(v, v, v, v) },
            unsafe { r(v, v, v, v) },
        );
    }
}
