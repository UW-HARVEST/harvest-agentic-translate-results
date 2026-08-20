//! Differential tests for the angle / axis / vector-basis entry points of
//! `q_math.c` (CONFIGS.md rows 32-38 and 45-47):
//!
//! `AngleVectors` `AnglesToAxis` `AxisClear` `AxisCopy` `ProjectPointOnPlane`
//! `MakeNormalVectors` `PerpendicularVector` `vectoangles`
//! `RotatePointAroundVector` `RotateAroundDirection`.
//!
//! Every call goes through `dlsym` on both shared objects; every output byte is
//! compared with `f32::to_bits()`.  Output buffers are always pre-filled with
//! the SAME sentinel on both sides, so "not written at all" (a NULL output of
//! `AngleVectors`, the skipped rotation of `RotateAroundDirection`) is detected
//! as well as a wrong value.
//!
//! Two properties of this group of functions decide how they are compared:
//!
//! * They all manufacture NaNs internally -- `sin`/`cos` of a non-finite angle,
//!   `1.0f / 0.0f` for a zero `normal`/`src`/`dir`, `0 * inf` -- and the source
//!   itself negates intermediates (`right[1] = -forward[0]`, `-1*sr*sp*cy`,
//!   `-d`, `-sy`), which flips the sign bit of a NaN.  Two different NaN
//!   patterns therefore meet inside single expressions such as
//!   `right[2]*forward[0] - right[0]*forward[2]`, where the surviving payload is
//!   whatever operand order the compiler picked for `mulss`/`addss`.  Results
//!   are therefore compared with `check_vec`, which tolerates a NaN payload
//!   difference only when both sides produced a NaN *and* an input was
//!   non-finite; NaN-ness, every finite value, every signed zero and every
//!   infinity are still compared bit for bit.  For all-finite inputs the
//!   comparison is fully exact (`assert_vec` semantics), and every routine here
//!   is NaN-free for finite inputs.
//! * `AngleVectors` keeps its six sines/cosines in `static float`s, so the C
//!   implementation is not reentrant and `cargo test`'s parallel threads would
//!   corrupt each other's results.  `angle_vectors_guard()` serialises the two
//!   tests that reach it.

mod harness;

use core::ptr;
use harness::*;

type AngleVectorsFn = unsafe extern "C" fn(*const f32, *mut f32, *mut f32, *mut f32);
type AnglesToAxisFn = unsafe extern "C" fn(*const f32, *mut [f32; 3]);
type AxisClearFn = unsafe extern "C" fn(*mut [f32; 3]);
type AxisCopyFn = unsafe extern "C" fn(*mut [f32; 3], *mut [f32; 3]);
type ProjectFn = unsafe extern "C" fn(*mut f32, *const f32, *const f32);
type MakeNormalFn = unsafe extern "C" fn(*const f32, *mut f32, *mut f32);
type PerpFn = unsafe extern "C" fn(*mut f32, *const f32);
type VecToAnglesFn = unsafe extern "C" fn(*const f32, *mut f32);
type RotatePointFn = unsafe extern "C" fn(*mut f32, *const f32, *const f32, f32);
type RotateDirFn = unsafe extern "C" fn(*mut [f32; 3], f32);

// ---------------------------------------------------------------------------
// sentinels: values no routine here can plausibly produce
// ---------------------------------------------------------------------------

const SENT_F: [f32; 3] = [-1.5e-8, 12345.678, -9.876_543e21];
const SENT_R: [f32; 3] = [0.333_333_34, -7.0, 4.25e-13];
const SENT_U: [f32; 3] = [-0.0, 2.5e9, -33.25];
const SENT_D: [f32; 3] = [7.5e-11, -654321.0, 1.234_567e18];
const SENT_AXIS: [[f32; 3]; 3] = [SENT_F, SENT_R, SENT_U];

fn flat9(a: &[[f32; 3]; 3]) -> [f32; 9] {
    [
        a[0][0], a[0][1], a[0][2], a[1][0], a[1][1], a[1][2], a[2][0], a[2][1], a[2][2],
    ]
}

// ---------------------------------------------------------------------------
// shared input pools
// ---------------------------------------------------------------------------

/// The angle values `AngleVectors`/`AnglesToAxis` branch on: the multiples of
/// 90 degrees, their neighbours, huge/tiny magnitudes, both infinities and NaN.
const ANGLE_VALS: &[f32] = &[
    0.0,
    -0.0,
    90.0,
    -90.0,
    180.0,
    -180.0,
    270.0,
    -270.0,
    360.0,
    -360.0,
    45.0,
    -45.0,
    1.0,
    89.999_99,
    90.000_01,
    179.999_98,
    1e30,
    -1e30,
    f32::MAX,
    1e-30,
    1e-45,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
];

fn angle_shapes(rng: &mut Rng) -> Vec<[f32; 3]> {
    let mut out: Vec<[f32; 3]> = vec![[0.0, 0.0, 0.0], [-0.0, -0.0, -0.0]];
    // one interesting value per slot, the other slots 0 and then 45
    for slot in 0..3 {
        for &v in ANGLE_VALS {
            let mut a = [0.0f32; 3];
            a[slot] = v;
            out.push(a);
            let mut b = [45.0f32; 3];
            b[slot] = v;
            out.push(b);
        }
    }
    // every (pitch, yaw) pair, roll fixed at 30
    for &x in ANGLE_VALS {
        for &y in ANGLE_VALS {
            out.push([x, y, 30.0]);
        }
    }
    for _ in 0..600 {
        out.push(rng.vec3_mag(720.0));
    }
    for _ in 0..400 {
        out.push(rng.vec3_any());
    }
    for _ in 0..200 {
        out.push(rng.vec3_mag(1e30));
    }
    out
}

/// Direction-vector shapes: axial, zero, negative zero, normalized, random,
/// huge, tiny, infinite, NaN.
fn dir_shapes(rng: &mut Rng, n_random: usize) -> Vec<[f32; 3]> {
    let mut out: Vec<[f32; 3]> = vec![
        [0.0, 0.0, 0.0],
        [-0.0, -0.0, -0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [-1.0, 0.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, 0.0, -1.0],
        [0.577_350_3, 0.577_350_3, 0.577_350_3],
        [-0.577_350_3, 0.577_350_3, -0.577_350_3],
        [0.6, 0.8, 0.0],
        [1.0, 1.0, 1.0],
        [2.0, 3.0, 4.0],
        [1e30, 1e30, 1e30],
        [f32::MAX, 0.0, 0.0],
        [1e-30, 1e-30, 1e-30],
        [1e-45, 0.0, 0.0],
        [f32::INFINITY, 0.0, 0.0],
        [f32::NEG_INFINITY, 1.0, 0.0],
        [f32::NAN, 0.0, 0.0],
        [0.0, f32::NAN, 0.0],
        [0.0, 0.0, f32::NAN],
        [f32::NAN, f32::NAN, f32::NAN],
    ];
    for _ in 0..n_random {
        out.push(match rng.below(4) {
            0 => rng.dir(),
            1 => rng.vec3_mag(1.0),
            2 => rng.vec3_finite(),
            _ => rng.vec3_any(),
        });
    }
    out
}

// ---------------------------------------------------------------------------
// row 32 -- AngleVectors: all eight NULL/non-NULL combinations
// ---------------------------------------------------------------------------

#[test]
fn angle_vectors() {
    // the C `AngleVectors` communicates through `static float`s -- serialise
    let _guard = angle_vectors_guard();
    let (c, r): (AngleVectorsFn, AngleVectorsFn) = both("AngleVectors");
    let mut rng = Rng::new(0x0032_0001);

    // `mask` bit 0 = forward, bit 1 = right, bit 2 = up; a clear bit passes NULL.
    let call = |f: AngleVectorsFn, a: &[f32; 3], mask: u32| -> ([f32; 3], [f32; 3], [f32; 3]) {
        let mut fwd = SENT_F;
        let mut rgt = SENT_R;
        let mut up = SENT_U;
        unsafe {
            f(
                a.as_ptr(),
                if mask & 1 != 0 {
                    fwd.as_mut_ptr()
                } else {
                    ptr::null_mut()
                },
                if mask & 2 != 0 {
                    rgt.as_mut_ptr()
                } else {
                    ptr::null_mut()
                },
                if mask & 4 != 0 {
                    up.as_mut_ptr()
                } else {
                    ptr::null_mut()
                },
            );
        }
        (fwd, rgt, up)
    };

    for a in angle_shapes(&mut rng) {
        for mask in 0u32..8 {
            let (fc, rc, uc) = call(c, &a, mask);
            let (fr, rr, ur) = call(r, &a, mask);
            let ctx = format!(
                "AngleVectors(angles={a:?} bits=[{:08x},{:08x},{:08x}], mask={mask} \
                 forward={} right={} up={})",
                a[0].to_bits(),
                a[1].to_bits(),
                a[2].to_bits(),
                if mask & 1 != 0 { "buf" } else { "NULL" },
                if mask & 2 != 0 { "buf" } else { "NULL" },
                if mask & 4 != 0 { "buf" } else { "NULL" },
            );
            check_vec(&format!("{ctx} forward"), &a, &fc, &fr);
            check_vec(&format!("{ctx} right"), &a, &rc, &rr);
            check_vec(&format!("{ctx} up"), &a, &uc, &ur);

            // a NULL output must be left completely untouched in BOTH impls
            if mask & 1 == 0 {
                assert_vec(&format!("{ctx} forward untouched (C)"), &SENT_F, &fc);
                assert_vec(&format!("{ctx} forward untouched (Rust)"), &SENT_F, &fr);
            }
            if mask & 2 == 0 {
                assert_vec(&format!("{ctx} right untouched (C)"), &SENT_R, &rc);
                assert_vec(&format!("{ctx} right untouched (Rust)"), &SENT_R, &rr);
            }
            if mask & 4 == 0 {
                assert_vec(&format!("{ctx} up untouched (C)"), &SENT_U, &uc);
                assert_vec(&format!("{ctx} up untouched (Rust)"), &SENT_U, &ur);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// row 33 -- AnglesToAxis
// ---------------------------------------------------------------------------

#[test]
fn angles_to_axis() {
    // AnglesToAxis calls AngleVectors, whose C version keeps its sines and
    // cosines in `static float`s -- serialise against `angle_vectors`
    let _guard = angle_vectors_guard();
    let (c, r): (AnglesToAxisFn, AnglesToAxisFn) = both("AnglesToAxis");
    let mut rng = Rng::new(0x0033_0002);

    let call = |f: AnglesToAxisFn, a: &[f32; 3]| -> [[f32; 3]; 3] {
        let mut axis = SENT_AXIS;
        unsafe { f(a.as_ptr(), axis.as_mut_ptr()) };
        axis
    };

    for a in angle_shapes(&mut rng) {
        let ac = call(c, &a);
        let ar = call(r, &a);
        let ctx = format!(
            "AnglesToAxis(angles={a:?} bits=[{:08x},{:08x},{:08x}])",
            a[0].to_bits(),
            a[1].to_bits(),
            a[2].to_bits()
        );
        check_vec(&ctx, &a, &flat9(&ac), &flat9(&ar));
    }
}

// ---------------------------------------------------------------------------
// row 34 -- AxisClear
// ---------------------------------------------------------------------------

#[test]
fn axis_clear() {
    let (c, r): (AxisClearFn, AxisClearFn) = both("AxisClear");
    let mut rng = Rng::new(0x0034_0003);

    // fresh buffers, arbitrary garbage (incl. NaN payloads) as the previous
    // contents, and repeated calls on the same buffer
    for iter in 0..3000 {
        let mut ac = [[0.0f32; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                ac[i][j] = match rng.below(3) {
                    0 => f32::from_bits(rng.next_u32()),
                    1 => rng.f32_any(),
                    _ => rng.f32_finite(),
                };
            }
        }
        let mut ar = ac;
        unsafe { c(ac.as_mut_ptr()) };
        unsafe { r(ar.as_mut_ptr()) };
        let ctx = format!("AxisClear(iter {iter}, prev={ac:?})");
        assert_vec(&ctx, &flat9(&ac), &flat9(&ar));
        // idempotent: a second call must reproduce the same 9 constants
        unsafe { c(ac.as_mut_ptr()) };
        unsafe { r(ar.as_mut_ptr()) };
        assert_vec(&format!("{ctx} twice"), &flat9(&ac), &flat9(&ar));
    }

    // The nine writes must land in exactly `axis[0..3]` -- run them at a
    // non-zero offset inside a larger buffer whose surroundings are sentinels,
    // which also covers a caller that hands over an interior (aliased) slice.
    for off in 0..4usize {
        let mut bufc = [0f32; 15];
        for (i, x) in bufc.iter_mut().enumerate() {
            *x = -1000.0 - i as f32;
        }
        let mut bufr = bufc;
        unsafe { c(bufc.as_mut_ptr().add(off) as *mut [f32; 3]) };
        unsafe { r(bufr.as_mut_ptr().add(off) as *mut [f32; 3]) };
        assert_vec(&format!("AxisClear(offset {off} in [f32;15])"), &bufc, &bufr);
    }
}

// ---------------------------------------------------------------------------
// row 35 -- AxisCopy
// ---------------------------------------------------------------------------

/// `AxisCopy` only copies, so every bit pattern -- including every NaN payload
/// -- has to survive verbatim; compared with `assert_vec`, never softened.
#[test]
fn axis_copy() {
    let (c, r): (AxisCopyFn, AxisCopyFn) = both("AxisCopy");
    let mut rng = Rng::new(0x0035_0004);

    for iter in 0..4000 {
        let mut src = [[0.0f32; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                src[i][j] = match rng.below(3) {
                    0 => f32::from_bits(rng.next_u32()), // arbitrary bits, all NaN payloads
                    1 => rng.f32_any(),
                    _ => rng.f32_finite(),
                };
            }
        }

        // distinct arrays: `out` starts as a sentinel, so a missed element shows
        let mut inc = src;
        let mut outc = SENT_AXIS;
        let mut inr = src;
        let mut outr = SENT_AXIS;
        unsafe { c(inc.as_mut_ptr(), outc.as_mut_ptr()) };
        unsafe { r(inr.as_mut_ptr(), outr.as_mut_ptr()) };
        let ctx = format!("AxisCopy(iter {iter}, in={:?})", flat9(&src));
        assert_vec(&format!("{ctx} out"), &flat9(&outc), &flat9(&outr));
        assert_vec(&format!("{ctx} in unchanged"), &flat9(&inc), &flat9(&inr));
        assert_vec(&format!("{ctx} out == in (C)"), &flat9(&src), &flat9(&outc));

        // in == out aliasing: a self copy must leave every bit alone
        let mut ac = src;
        let mut ar = src;
        unsafe {
            let p = ac.as_mut_ptr();
            c(p, p);
        }
        unsafe {
            let p = ar.as_mut_ptr();
            r(p, p);
        }
        assert_vec(&format!("{ctx} in==out"), &flat9(&ac), &flat9(&ar));
        assert_vec(&format!("{ctx} in==out is identity (C)"), &flat9(&src), &flat9(&ac));
    }

    // exhaustive NaN-payload sweep: every exponent-mantissa combination that is
    // a NaN must be copied bit for bit
    for k in 0..64u32 {
        let bits = 0x7f80_0000 | (k << 12) | 1;
        let v = f32::from_bits(bits);
        let src = [[v; 3], [f32::from_bits(bits | 0x8000_0000); 3], [v; 3]];
        let mut inc = src;
        let mut outc = SENT_AXIS;
        let mut inr = src;
        let mut outr = SENT_AXIS;
        unsafe { c(inc.as_mut_ptr(), outc.as_mut_ptr()) };
        unsafe { r(inr.as_mut_ptr(), outr.as_mut_ptr()) };
        assert_vec(
            &format!("AxisCopy(NaN payload 0x{bits:08x})"),
            &flat9(&outc),
            &flat9(&outr),
        );
        assert_vec(
            &format!("AxisCopy(NaN payload 0x{bits:08x}) preserved (C)"),
            &flat9(&src),
            &flat9(&outc),
        );
    }
}

// ---------------------------------------------------------------------------
// row 36 -- ProjectPointOnPlane
// ---------------------------------------------------------------------------

#[test]
fn project_point_on_plane() {
    let (c, r): (ProjectFn, ProjectFn) = both("ProjectPointOnPlane");
    let mut rng = Rng::new(0x0036_0005);

    // alias: 0 = three distinct buffers, 1 = dst == p, 2 = dst == normal
    let call = |f: ProjectFn, p: &[f32; 3], n: &[f32; 3], alias: u32| -> [f32; 3] {
        let mut pb = *p;
        let mut nb = *n;
        let mut dst = SENT_D;
        unsafe {
            match alias {
                0 => {
                    f(dst.as_mut_ptr(), pb.as_ptr(), nb.as_ptr());
                    dst
                }
                1 => {
                    let pp = pb.as_mut_ptr();
                    f(pp, pp, nb.as_ptr());
                    pb
                }
                _ => {
                    let np = nb.as_mut_ptr();
                    f(np, pb.as_ptr(), np);
                    nb
                }
            }
        }
    };

    let normals = dir_shapes(&mut rng, 0);
    let points: Vec<[f32; 3]> = vec![
        [0.0, 0.0, 0.0],
        [-0.0, -0.0, -0.0],
        [1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0],
        [1.0, 2.0, 3.0],
        [-4.5, 6.25, -8.125],
        [1e30, -1e30, 1e30],
        [f32::MAX, f32::MAX, f32::MAX],
        [1e-30, 1e-30, 1e-30],
        [f32::INFINITY, 0.0, 0.0],
        [f32::NAN, 1.0, 2.0],
        [0.0, 0.0, f32::NAN],
    ];

    for n in &normals {
        for p in &points {
            for alias in 0u32..3 {
                let vc = call(c, p, n, alias);
                let vr = call(r, p, n, alias);
                let inputs = [p[0], p[1], p[2], n[0], n[1], n[2]];
                check_vec(
                    &format!("ProjectPointOnPlane(p={p:?}, normal={n:?}, alias={alias})"),
                    &inputs,
                    &vc,
                    &vr,
                );
            }
        }
    }

    for iter in 0..6000 {
        let p = match rng.below(4) {
            0 => rng.vec3_any(),
            1 => rng.vec3_finite(),
            2 => rng.vec3_mag(1.0),
            _ => rng.vec3_mag(1e30),
        };
        let n = match rng.below(4) {
            0 => rng.vec3_any(),
            1 => rng.dir(),
            2 => rng.vec3_mag(1.0),
            _ => rng.vec3_finite(),
        };
        let alias = rng.below(3);
        let vc = call(c, &p, &n, alias);
        let vr = call(r, &p, &n, alias);
        let inputs = [p[0], p[1], p[2], n[0], n[1], n[2]];
        check_vec(
            &format!("ProjectPointOnPlane(iter {iter}, p={p:?}, normal={n:?}, alias={alias})"),
            &inputs,
            &vc,
            &vr,
        );
    }
}

// ---------------------------------------------------------------------------
// row 37 -- PerpendicularVector
// ---------------------------------------------------------------------------

#[test]
fn perpendicular_vector() {
    let (c, r): (PerpFn, PerpFn) = both("PerpendicularVector");
    let mut rng = Rng::new(0x0037_0006);

    // alias: 0 = distinct buffers, 1 = dst == src
    let call = |f: PerpFn, src: &[f32; 3], alias: u32| -> [f32; 3] {
        let mut sb = *src;
        let mut dst = SENT_D;
        unsafe {
            if alias == 0 {
                f(dst.as_mut_ptr(), sb.as_ptr());
                dst
            } else {
                let sp = sb.as_mut_ptr();
                f(sp, sp);
                sb
            }
        }
    };

    let mut srcs: Vec<[f32; 3]> = vec![
        // smallest |component| at index 0, 1 and 2
        [0.1, 0.5, 0.9],
        [0.5, 0.1, 0.9],
        [0.9, 0.5, 0.1],
        [-0.1, 0.5, -0.9],
        [0.5, -0.1, 0.9],
        [0.9, -0.5, -0.1],
        // ties: `<` is strict, so the FIRST minimum wins (pos = 0 / 1)
        [0.5, 0.5, 0.5],
        [0.9, 0.5, 0.5],
        // every component >= 1 -> `pos` stays 0 and `minelem` stays 1.0
        [1.0, 2.0, 3.0],
        [1.0, 1.0, 1.0],
        [-1.0, -2.0, -3.0],
        [1e30, 1.5, 2.5],
        // exactly 1.0 is NOT < 1.0
        [1.0, 0.999_999_94, 1.0],
        [0.999_999_94, 1.0, 1.0],
        // zero vector: DotProduct == 0 -> 1.0f/0.0f == +inf
        [0.0, 0.0, 0.0],
        [-0.0, -0.0, -0.0],
        [0.0, -0.0, 0.0],
        // denormals whose squares underflow to zero -> same inf path
        [1e-30, 1e-30, 1e-30],
        [1e-45, 0.0, 0.0],
    ];
    srcs.extend(dir_shapes(&mut rng, 0));

    for src in &srcs {
        for alias in 0u32..2 {
            let vc = call(c, src, alias);
            let vr = call(r, src, alias);
            let inputs = [src[0], src[1], src[2]];
            check_vec(
                &format!(
                    "PerpendicularVector(src={src:?} bits=[{:08x},{:08x},{:08x}], alias={alias})",
                    src[0].to_bits(),
                    src[1].to_bits(),
                    src[2].to_bits()
                ),
                &inputs,
                &vc,
                &vr,
            );
        }
    }

    for iter in 0..6000 {
        let src = match rng.below(5) {
            0 => rng.vec3_any(),
            1 => rng.dir(),
            2 => rng.vec3_mag(1.0),
            3 => rng.vec3_finite(),
            _ => {
                // deliberately steer the argmin to a random index
                let mut v = rng.vec3_mag(1.0);
                let i = rng.below(3) as usize;
                v[i] = 0.0;
                v
            }
        };
        let alias = rng.below(2);
        let vc = call(c, &src, alias);
        let vr = call(r, &src, alias);
        let inputs = [src[0], src[1], src[2]];
        check_vec(
            &format!("PerpendicularVector(iter {iter}, src={src:?}, alias={alias})"),
            &inputs,
            &vc,
            &vr,
        );
    }
}

// ---------------------------------------------------------------------------
// row 38 -- MakeNormalVectors
// ---------------------------------------------------------------------------

/// `MakeNormalVectors` writes `right[]` and then reads `forward[]` again, so
/// `right == forward` and `up == forward` are observable aliasing cases and the
/// two implementations must agree on what the interleaving produces.
#[test]
fn make_normal_vectors() {
    let (c, r): (MakeNormalFn, MakeNormalFn) = both("MakeNormalVectors");
    let mut rng = Rng::new(0x0038_0007);

    // alias: 0 = distinct, 1 = right == forward, 2 = up == forward,
    //        3 = up == right, 4 = all three the same buffer
    let call = |f: MakeNormalFn, fwd: &[f32; 3], alias: u32| -> [f32; 9] {
        let mut fb = *fwd;
        let mut rb = SENT_R;
        let mut ub = SENT_U;
        unsafe {
            match alias {
                0 => f(fb.as_ptr(), rb.as_mut_ptr(), ub.as_mut_ptr()),
                1 => {
                    let fp = fb.as_mut_ptr();
                    f(fp, fp, ub.as_mut_ptr());
                }
                2 => {
                    let fp = fb.as_mut_ptr();
                    f(fp, rb.as_mut_ptr(), fp);
                }
                3 => {
                    let rp = rb.as_mut_ptr();
                    f(fb.as_ptr(), rp, rp);
                }
                _ => {
                    let fp = fb.as_mut_ptr();
                    f(fp, fp, fp);
                }
            }
        }
        [
            fb[0], fb[1], fb[2], rb[0], rb[1], rb[2], ub[0], ub[1], ub[2],
        ]
    };

    let mut fwds: Vec<[f32; 3]> = vec![
        [0.0, 0.0, 0.0],
        [-0.0, 0.0, -0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [-1.0, 0.0, 0.0],
        [0.577_350_3, 0.577_350_3, 0.577_350_3],
        [0.6, 0.0, 0.8],
        [3.0, -4.0, 12.0],
        [1e30, 1e30, 1e30],
        [1e-30, 1e-30, 1e-30],
        [f32::INFINITY, 1.0, 0.0],
        [f32::NAN, 1.0, 0.0],
        [1.0, f32::NAN, 0.0],
        [1.0, 0.0, f32::NAN],
    ];
    fwds.extend(dir_shapes(&mut rng, 0));

    for fwd in &fwds {
        for alias in 0u32..5 {
            let vc = call(c, fwd, alias);
            let vr = call(r, fwd, alias);
            check_vec(
                &format!(
                    "MakeNormalVectors(forward={fwd:?} bits=[{:08x},{:08x},{:08x}], alias={alias})",
                    fwd[0].to_bits(),
                    fwd[1].to_bits(),
                    fwd[2].to_bits()
                ),
                fwd,
                &vc,
                &vr,
            );
        }
    }

    for iter in 0..5000 {
        let fwd = match rng.below(4) {
            0 => rng.vec3_any(),
            1 => rng.dir(),
            2 => rng.vec3_mag(1.0),
            _ => rng.vec3_finite(),
        };
        let alias = rng.below(5);
        let vc = call(c, &fwd, alias);
        let vr = call(r, &fwd, alias);
        check_vec(
            &format!("MakeNormalVectors(iter {iter}, forward={fwd:?}, alias={alias})"),
            &fwd,
            &vc,
            &vr,
        );
    }
}

// ---------------------------------------------------------------------------
// row 45 -- vectoangles
// ---------------------------------------------------------------------------

#[test]
fn vectoangles() {
    let (c, r): (VecToAnglesFn, VecToAnglesFn) = both("vectoangles");
    let mut rng = Rng::new(0x0045_0008);

    let call = |f: VecToAnglesFn, v: &[f32; 3]| -> [f32; 3] {
        let vb = *v;
        let mut out = SENT_D;
        unsafe { f(vb.as_ptr(), out.as_mut_ptr()) };
        out
    };

    let mut vals: Vec<[f32; 3]> = Vec::new();

    // x == 0 && y == 0: yaw = 0, pitch = 90 if z > 0 else 270
    for &z in &[
        1.0f32,
        -1.0,
        0.0,
        -0.0,
        1e-45,
        -1e-45,
        1e30,
        -1e30,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ] {
        vals.push([0.0, 0.0, z]);
        vals.push([-0.0, 0.0, z]);
        vals.push([0.0, -0.0, z]);
        vals.push([-0.0, -0.0, z]);
    }

    // x == 0, y != 0: yaw = 90 (y > 0) or 270 (y <= 0 / NaN)
    for &y in &[
        1.0f32,
        -1.0,
        1e-45,
        -1e-45,
        1e30,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ] {
        for &z in &[0.0f32, -0.0, 1.0, -1.0, 1e30, f32::INFINITY, f32::NAN] {
            vals.push([0.0, y, z]);
            vals.push([-0.0, y, z]);
        }
    }

    // all four atan2 quadrants (and hence the `yaw < 0` fix-up for y < 0),
    // crossed with z > 0 / z < 0 (the `pitch < 0` fix-up)
    for &x in &[1.0f32, -1.0, 3.0, -3.0, 1e-30, -1e-30, 1e30, -1e30, f32::MAX] {
        for &y in &[1.0f32, -1.0, 4.0, -4.0, 1e-30, -1e-30, 1e30, -1e30, 0.0, -0.0] {
            for &z in &[
                0.0f32,
                -0.0,
                1.0,
                -1.0,
                5.0,
                -5.0,
                1e30,
                -1e30,
                1e-45,
                f32::INFINITY,
                f32::NEG_INFINITY,
                f32::NAN,
            ] {
                vals.push([x, y, z]);
            }
        }
    }

    // NaN in every position (all comparisons false -> atan2 path)
    for slot in 0..3 {
        for &other in &[0.0f32, 1.0, -1.0, f32::INFINITY] {
            let mut v = [other; 3];
            v[slot] = f32::NAN;
            vals.push(v);
        }
    }

    for _ in 0..4000 {
        vals.push(match rng.below(5) {
            0 => rng.vec3_any(),
            1 => rng.vec3_mag(1.0),
            2 => rng.vec3_finite(),
            3 => rng.dir(),
            _ => rng.vec3_mag(1e30),
        });
    }

    for v in &vals {
        let ac = call(c, v);
        let ar = call(r, v);
        check_vec(
            &format!(
                "vectoangles(value1={v:?} bits=[{:08x},{:08x},{:08x}])",
                v[0].to_bits(),
                v[1].to_bits(),
                v[2].to_bits()
            ),
            v,
            &ac,
            &ar,
        );
    }
}

// ---------------------------------------------------------------------------
// row 46 -- RotatePointAroundVector
// ---------------------------------------------------------------------------

#[test]
fn rotate_point_around_vector() {
    let (c, r): (RotatePointFn, RotatePointFn) = both("RotatePointAroundVector");
    let mut rng = Rng::new(0x0046_0009);

    // alias: 0 = distinct, 1 = dst == dir, 2 = dst == point
    let call = |f: RotatePointFn, dir: &[f32; 3], p: &[f32; 3], deg: f32, alias: u32| -> [f32; 3] {
        let mut db = *dir;
        let mut pb = *p;
        let mut dst = SENT_D;
        unsafe {
            match alias {
                0 => {
                    f(dst.as_mut_ptr(), db.as_ptr(), pb.as_ptr(), deg);
                    dst
                }
                1 => {
                    let dp = db.as_mut_ptr();
                    f(dp, dp, pb.as_ptr(), deg);
                    db
                }
                _ => {
                    let pp = pb.as_mut_ptr();
                    f(pp, db.as_ptr(), pp, deg);
                    pb
                }
            }
        }
    };

    let degrees: &[f32] = &[
        0.0,
        -0.0,
        90.0,
        -90.0,
        180.0,
        -180.0,
        270.0,
        360.0,
        -360.0,
        45.0,
        1.0,
        1e30,
        -1e30,
        1e-30,
        f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ];
    let dirs = dir_shapes(&mut rng, 0);
    let points: Vec<[f32; 3]> = vec![
        [0.0, 0.0, 0.0],
        [-0.0, -0.0, -0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [1.0, 2.0, 3.0],
        [-7.5, 0.25, 16.0],
        [1e30, 1e30, -1e30],
        [f32::MAX, 0.0, 0.0],
        [1e-30, 0.0, 0.0],
        [f32::INFINITY, 1.0, 0.0],
        [f32::NAN, 0.0, 1.0],
    ];

    for dir in &dirs {
        for p in &points {
            for &deg in degrees {
                for alias in 0u32..3 {
                    let vc = call(c, dir, p, deg, alias);
                    let vr = call(r, dir, p, deg, alias);
                    let inputs = [dir[0], dir[1], dir[2], p[0], p[1], p[2], deg];
                    check_vec(
                        &format!(
                            "RotatePointAroundVector(dir={dir:?}, point={p:?}, \
                             degrees={deg:?} (0x{:08x}), alias={alias})",
                            deg.to_bits()
                        ),
                        &inputs,
                        &vc,
                        &vr,
                    );
                }
            }
        }
    }

    for iter in 0..6000 {
        let dir = match rng.below(4) {
            0 => rng.dir(),
            1 => rng.vec3_mag(1.0),
            2 => rng.vec3_finite(),
            _ => rng.vec3_any(),
        };
        let p = match rng.below(4) {
            0 => rng.vec3_mag(1.0),
            1 => rng.vec3_finite(),
            2 => rng.vec3_mag(1e30),
            _ => rng.vec3_any(),
        };
        let deg = match rng.below(3) {
            0 => rng.f32_mag(360.0),
            1 => rng.f32_any(),
            _ => degrees[rng.below(degrees.len() as u32) as usize],
        };
        let alias = rng.below(3);
        let vc = call(c, &dir, &p, deg, alias);
        let vr = call(r, &dir, &p, deg, alias);
        let inputs = [dir[0], dir[1], dir[2], p[0], p[1], p[2], deg];
        check_vec(
            &format!(
                "RotatePointAroundVector(iter {iter}, dir={dir:?}, point={p:?}, \
                 degrees={deg:?} (0x{:08x}), alias={alias})",
                deg.to_bits()
            ),
            &inputs,
            &vc,
            &vr,
        );
    }
}

// ---------------------------------------------------------------------------
// row 47 -- RotateAroundDirection
// ---------------------------------------------------------------------------

#[test]
fn rotate_around_direction() {
    let (c, r): (RotateDirFn, RotateDirFn) = both("RotateAroundDirection");
    let mut rng = Rng::new(0x0047_000a);

    // `axis[1]` and `axis[2]` start as sentinels: `if (yaw)` skipping the
    // rotation must still leave the raw PerpendicularVector result behind, and
    // both rows are always overwritten.
    let call = |f: RotateDirFn, a0: &[f32; 3], yaw: f32| -> [f32; 9] {
        let mut axis: [[f32; 3]; 3] = [*a0, SENT_R, SENT_U];
        unsafe { f(axis.as_mut_ptr(), yaw) };
        flat9(&axis)
    };

    let yaws: &[f32] = &[
        0.0, // `if ( yaw )` false -> rotation skipped
        -0.0, // ditto (ERRORS.md row 40)
        1e-45, // smallest non-zero: rotation IS performed
        -1e-45,
        1.0,
        -1.0,
        45.0,
        90.0,
        -90.0,
        180.0,
        -180.0,
        270.0,
        360.0,
        -360.0,
        1e30,
        f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ];

    // `yaw == 0` skips the rotation, so `axis[1]` must then be bit-for-bit what
    // `PerpendicularVector` alone produces -- checked against the exported
    // `PerpendicularVector` of the SAME library.
    let (pc, pr): (PerpFn, PerpFn) = both("PerpendicularVector");

    let a0s = dir_shapes(&mut rng, 0);
    for a0 in &a0s {
        for &yaw in yaws {
            let vc = call(c, a0, yaw);
            let vr = call(r, a0, yaw);
            let inputs = [a0[0], a0[1], a0[2], yaw];
            check_vec(
                &format!(
                    "RotateAroundDirection(axis[0]={a0:?}, yaw={yaw:?} (0x{:08x}))",
                    yaw.to_bits()
                ),
                &inputs,
                &vc,
                &vr,
            );
            if yaw == 0.0 {
                // the rotation is skipped (true for `-0.0` as well): `axis[1]`
                // must be exactly what PerpendicularVector wrote
                let mut sc = SENT_R;
                let mut sr = SENT_R;
                let ab = *a0;
                unsafe { pc(sc.as_mut_ptr(), ab.as_ptr()) };
                unsafe { pr(sr.as_mut_ptr(), ab.as_ptr()) };
                check_vec(
                    &format!("RotateAroundDirection(axis[0]={a0:?}, yaw={yaw:?}) axis[1] == PerpendicularVector"),
                    &inputs,
                    &sc,
                    &vc[3..6],
                );
                check_vec(
                    &format!("RotateAroundDirection(axis[0]={a0:?}, yaw={yaw:?}) axis[1] == PerpendicularVector (Rust)"),
                    &inputs,
                    &sr,
                    &vr[3..6],
                );
            }
        }
    }

    for iter in 0..6000 {
        let a0 = match rng.below(4) {
            0 => rng.dir(),
            1 => rng.vec3_mag(1.0),
            2 => rng.vec3_finite(),
            _ => rng.vec3_any(),
        };
        let yaw = match rng.below(3) {
            0 => rng.f32_mag(360.0),
            1 => rng.f32_any(),
            _ => yaws[rng.below(yaws.len() as u32) as usize],
        };
        let vc = call(c, &a0, yaw);
        let vr = call(r, &a0, yaw);
        let inputs = [a0[0], a0[1], a0[2], yaw];
        check_vec(
            &format!(
                "RotateAroundDirection(iter {iter}, axis[0]={a0:?}, yaw={yaw:?} (0x{:08x}))",
                yaw.to_bits()
            ),
            &inputs,
            &vc,
            &vr,
        );
    }
}
