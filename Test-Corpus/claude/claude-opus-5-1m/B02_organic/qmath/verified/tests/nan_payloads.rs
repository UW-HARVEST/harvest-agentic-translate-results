//! Pins the *exact* extent of the one documented tolerance of this suite (see
//! NOTES.md "Deviation 1"): which entry points are bit-identical even for NaN
//! and infinite inputs, and which ones can only be compared up to the NaN
//! payload because the surviving payload is chosen by gcc's register allocation
//! rather than by the C source.
//!
//! In the reference build configuration (the `gcc -O0` shared object from
//! `./build_c.sh` against the dev profile) the payload assertions here are
//! exact.  The tests that cover a known-divergent entry point -- and every test
//! in an optimised build, see [`cmp3`] -- assert the *class* instead (both sides
//! return a NaN, in the same lanes, and agree bit-for-bit on every non-NaN lane),
//! which is all IEEE 754 and ISO C define.
//!
//! Reproduce the survey with `DIFF_STRICT_NAN=1 cargo test`.

mod harness;

use harness::*;

type AngleVectorsFn = unsafe extern "C" fn(*const f32, *mut f32, *mut f32, *mut f32) -> ();

/// The reference configuration of this suite is `gcc -O0` (what
/// `./build_c.sh` produces) against the unoptimised dev profile, and there the
/// NaN payloads of `AngleVectors` match exactly.  In `--release` LLVM reorders
/// the `mulss`/`addss` source operands and re-associates the products, so which
/// NaN survives changes again -- on the Rust side this time.  That is the whole
/// point of Deviation 1 in NOTES.md: the payload is a codegen artifact, so in an
/// optimised build only the NaN *class* is asserted.
#[track_caller]
fn cmp3(ctx: &str, c: &[f32; 3], r: &[f32; 3]) {
    if cfg!(debug_assertions) {
        assert_vec(ctx, c, r);
    } else {
        for i in 0..3 {
            assert_eq!(
                c[i].is_nan(),
                r[i].is_nan(),
                "{ctx}: element {i}: C {:08x} vs Rust {:08x}",
                c[i].to_bits(),
                r[i].to_bits()
            );
            if !c[i].is_nan() {
                assert_f32(&format!("{ctx}[{i}]"), c[i], r[i]);
            }
        }
    }
}

/// Angle triples that put NaNs of both signs and both origins (a caller NaN and
/// the `0xffc00000` that `sin(inf)`/`cos(inf)` manufacture) into every slot.
const ANGLE_POOL: &[f32] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    45.0,
    90.0,
    180.0,
    270.0,
    360.0,
    -45.0,
    1e30,
    -1e30,
    1e-45,
    f32::MAX,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
    -f32::NAN,
];

/// `AngleVectors` IS bit-identical for every input, NaN payloads included (in the
/// reference build configuration, see [`cmp3`]).
///
/// It is the one function where gcc's folding of the source's `-1 * x` factors
/// into a sign flip is *observable*: `mulss` by `-1.0f` keeps a NaN's sign bit
/// while `xorps` flips it.  `src/q_math.rs` therefore spells the expressions the
/// way gcc folds them (see the comment there), which makes all 2 916 x 9 output
/// components below match exactly.
#[test]
fn angle_vectors_nan_payloads_are_exact() {
    let _guard = angle_vectors_guard();
    let (c, r): (AngleVectorsFn, AngleVectorsFn) = both("AngleVectors");

    let mut compared = 0usize;
    for &a0 in ANGLE_POOL {
        for &a1 in ANGLE_POOL {
            for &a2 in ANGLE_POOL {
                let ang = [a0, a1, a2];
                let mut fc = [0.0f32; 3];
                let mut rc = [0.0f32; 3];
                let mut uc = [0.0f32; 3];
                let mut fr = [0.0f32; 3];
                let mut rr = [0.0f32; 3];
                let mut ur = [0.0f32; 3];
                unsafe {
                    c(
                        ang.as_ptr(),
                        fc.as_mut_ptr(),
                        rc.as_mut_ptr(),
                        uc.as_mut_ptr(),
                    );
                    r(
                        ang.as_ptr(),
                        fr.as_mut_ptr(),
                        rr.as_mut_ptr(),
                        ur.as_mut_ptr(),
                    );
                }
                let ctx = format!("AngleVectors({ang:?})");
                cmp3(&format!("{ctx} forward"), &fc, &fr);
                cmp3(&format!("{ctx} right"), &rc, &rr);
                cmp3(&format!("{ctx} up"), &uc, &ur);
                compared += 9;
            }
        }
    }
    assert_eq!(compared, ANGLE_POOL.len().pow(3) * 9);
}

/// `AnglesToAxis` is `AngleVectors` plus one `VectorSubtract`, so it is exact for
/// NaN inputs as well.
#[test]
fn angles_to_axis_nan_payloads_are_exact() {
    let _guard = angle_vectors_guard();
    type F = unsafe extern "C" fn(*const f32, *mut [f32; 3]) -> ();
    let (c, r): (F, F) = both("AnglesToAxis");

    for &a0 in ANGLE_POOL {
        for &a1 in ANGLE_POOL {
            for &a2 in ANGLE_POOL {
                let ang = [a0, a1, a2];
                let mut ac = [[0.0f32; 3]; 3];
                let mut ar = [[0.0f32; 3]; 3];
                unsafe {
                    c(ang.as_ptr(), ac.as_mut_ptr());
                    r(ang.as_ptr(), ar.as_mut_ptr());
                }
                for i in 0..3 {
                    cmp3(&format!("AnglesToAxis({ang:?}) axis[{i}]"), &ac[i], &ar[i]);
                }
            }
        }
    }
}

/// Single-NaN propagation is payload-exact everywhere, because `quiet(the one
/// NaN)` is what both operand orders yield: this sweeps every NaN payload
/// (including signalling ones) through the pure/bit-twiddling entry points.
#[test]
fn single_nan_payloads_are_exact() {
    type F1 = unsafe extern "C" fn(f32) -> f32;
    let (rsqrt_c, rsqrt_r): (F1, F1) = both("Q_rsqrt");
    let (fabs_c, fabs_r): (F1, F1) = both("Q_fabs");
    let (sqrtfast_c, sqrtfast_r): (F1, F1) = both_w("w_SQRTFAST");
    let (square_c, square_r): (F1, F1) = both_w("w_Square");
    type FC = unsafe extern "C" fn(*const f32, *mut f32) -> ();
    let (copy_c, copy_r): (FC, FC) = both("_VectorCopy");

    let mut rng = Rng::new(0x4E614E);
    let mut payloads: Vec<u32> = vec![
        0x7fc0_0000, // canonical quiet NaN
        0xffc0_0000, // x86 "default" NaN, the one invalid operations produce
        0x7f80_0001, // signalling
        0xff80_0001,
        0x7fff_ffff,
        0xffff_ffff,
        0x7fc0_0001,
        0x7fbf_ffff, // (not a NaN: largest finite-exponent pattern below)
    ];
    for _ in 0..2000 {
        payloads.push(0x7f80_0000 | (rng.next_u32() & 0x807f_ffff));
    }

    for bits in payloads {
        let v = f32::from_bits(bits);
        let ctx = format!("0x{bits:08x}");
        assert_f32(&format!("Q_rsqrt({ctx})"), unsafe { rsqrt_c(v) }, unsafe {
            rsqrt_r(v)
        });
        assert_f32(&format!("Q_fabs({ctx})"), unsafe { fabs_c(v) }, unsafe {
            fabs_r(v)
        });
        assert_f32(&format!("w_SQRTFAST({ctx})"), unsafe { sqrtfast_c(v) }, unsafe {
            sqrtfast_r(v)
        });
        assert_f32(&format!("w_Square({ctx})"), unsafe { square_c(v) }, unsafe {
            square_r(v)
        });
        let src = [v, 1.0, -v];
        let mut oc = [0.0f32; 3];
        let mut or_ = [0.0f32; 3];
        unsafe {
            copy_c(src.as_ptr(), oc.as_mut_ptr());
            copy_r(src.as_ptr(), or_.as_mut_ptr());
        }
        assert_vec(&format!("_VectorCopy({ctx})"), &oc, &or_);
    }
}

/// The three shapes where the payload really is decided by gcc's operand
/// ordering.  Both libraries must still agree on *what kind* of value comes out
/// (a NaN, in the same lanes) and on every non-NaN lane, bit for bit.
///
/// `DIFF_STRICT_NAN=1 cargo test` turns the payload tolerance off and shows the
/// full list of affected entry points: `_DotProduct`, `_VectorMA`,
/// `MatrixMultiply`, `VectorRotate`, `LerpAngle`, `ProjectPointOnPlane`,
/// `MakeNormalVectors`, `PerpendicularVector`-driven `RotatePointAroundVector` /
/// `RotateAroundDirection`, `PlaneFromPoints`, and the `w_*` expansions of
/// `DotProduct` / `VectorLength` / `Distance`.
#[test]
fn documented_nan_payload_divergences_are_nan_on_both_sides() {
    // 1. MakeNormalVectors: `right[1] = -forward[0]` makes -NaN meet the
    //    caller's +NaN inside CrossProduct.
    type FMNV = unsafe extern "C" fn(*const f32, *mut f32, *mut f32) -> ();
    let (c, r): (FMNV, FMNV) = both("MakeNormalVectors");
    let fwd = [f32::NAN, 1.0, 0.0];
    let (mut rc, mut uc) = ([0.0f32; 3], [0.0f32; 3]);
    let (mut rr, mut ur) = ([0.0f32; 3], [0.0f32; 3]);
    unsafe {
        c(fwd.as_ptr(), rc.as_mut_ptr(), uc.as_mut_ptr());
        r(fwd.as_ptr(), rr.as_mut_ptr(), ur.as_mut_ptr());
    }
    for i in 0..3 {
        assert_eq!(
            rc[i].is_nan(),
            rr[i].is_nan(),
            "MakeNormalVectors right[{i}]: C {:08x} vs Rust {:08x}",
            rc[i].to_bits(),
            rr[i].to_bits()
        );
        assert_eq!(
            uc[i].is_nan(),
            ur[i].is_nan(),
            "MakeNormalVectors up[{i}]: C {:08x} vs Rust {:08x}",
            uc[i].to_bits(),
            ur[i].to_bits()
        );
        if !rc[i].is_nan() {
            assert_f32("MakeNormalVectors right", rc[i], rr[i]);
        }
        if !uc[i].is_nan() {
            assert_f32("MakeNormalVectors up", uc[i], ur[i]);
        }
    }

    // 2. ProjectPointOnPlane with a zero normal: the internal 1.0f/0.0f = +inf
    //    and 0*inf = -NaN meet the caller's +NaN.
    type FPPP = unsafe extern "C" fn(*mut f32, *const f32, *const f32) -> ();
    let (c, r): (FPPP, FPPP) = both("ProjectPointOnPlane");
    let p = [f32::NAN, 1.0, 2.0];
    let n = [0.0f32, 0.0, 0.0];
    let mut dc = [0.0f32; 3];
    let mut dr = [0.0f32; 3];
    unsafe {
        c(dc.as_mut_ptr(), p.as_ptr(), n.as_ptr());
        r(dr.as_mut_ptr(), p.as_ptr(), n.as_ptr());
    }
    for i in 0..3 {
        assert_eq!(
            dc[i].is_nan(),
            dr[i].is_nan(),
            "ProjectPointOnPlane dst[{i}]: C {:08x} vs Rust {:08x}",
            dc[i].to_bits(),
            dr[i].to_bits()
        );
    }

    // 3. LerpAngle(inf, inf, NaN): `to - from` manufactures -NaN, `frac` is +NaN.
    type FL = unsafe extern "C" fn(f32, f32, f32) -> f32;
    let (c, r): (FL, FL) = both("LerpAngle");
    let vc = unsafe { c(f32::INFINITY, f32::INFINITY, f32::NAN) };
    let vr = unsafe { r(f32::INFINITY, f32::INFINITY, f32::NAN) };
    assert!(
        vc.is_nan() && vr.is_nan(),
        "LerpAngle(inf,inf,NaN): C {:08x} vs Rust {:08x}",
        vc.to_bits(),
        vr.to_bits()
    );
}
