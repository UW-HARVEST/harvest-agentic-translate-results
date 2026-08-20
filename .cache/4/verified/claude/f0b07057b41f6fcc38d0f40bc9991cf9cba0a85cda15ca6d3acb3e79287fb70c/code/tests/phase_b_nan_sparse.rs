//! Phase B (blind-spot closure) — SPARSE NaN INJECTION.
//!
//! The dense NaN test in `phase_b_nan_payload.rs` fills every input with a NaN,
//! which turns out to MASK operand-order bugs: in `c2Mulvs(sA, mulp(u, den))`,
//! if `sA.x` is itself a NaN it wins and the `mulp(u, den)` vs `mulp(den, u)`
//! choice becomes unobservable.
//!
//! So this file does the opposite: it injects NaNs into exactly ONE or TWO
//! input slots and keeps every other slot FINITE, for every slot pair. That way
//! each individual `ADDSS`/`MULSS` has its two NaN candidates isolated, and the
//! destination-operand choice is directly observable in the result.
//!
//! (Verified to be a real discriminator: swapping the operand order of any
//! single `mulp`/`addp` in `src/lib.rs` makes a test in this file fail.)

#![allow(non_snake_case)]

#[macro_use]
mod common;

use common::*;
use std::ffi::c_void;

/// Two NaNs with clearly distinct payloads, plus sign/quiet variants.
const NAN_A: f32 = f32::from_bits(0x7fc0_0001);
const NAN_B: f32 = f32::from_bits(0x7fd2_3456);
const NAN_C: f32 = f32::from_bits(0xffc0_00ff);
const NAN_D: f32 = f32::from_bits(0x7f80_0001); // signalling
const NAN_E: f32 = f32::from_bits(0xff81_0000); // -signalling

fn nans() -> [f32; 5] {
    [NAN_A, NAN_B, NAN_C, NAN_D, NAN_E]
}

/// Distinct finite fillers so an accidental slot swap is also visible.
fn finite_base(n: usize, g: &mut Rng) -> Vec<f32> {
    (0..n).map(|k| 1.0 + k as f32 * 0.5 + g.range(-0.25, 0.25)).collect()
}

/// Every (single slot) and (ordered pair of distinct slots) NaN assignment.
fn injections(n: usize) -> Vec<Vec<(usize, f32)>> {
    let ns = nans();
    let mut out = Vec::new();
    for i in 0..n {
        for &na in ns.iter() {
            out.push(vec![(i, na)]);
        }
    }
    for i in 0..n {
        for j in 0..n {
            if i == j {
                continue;
            }
            // the interesting case: two DIFFERENT payloads in two slots
            out.push(vec![(i, NAN_A), (j, NAN_B)]);
            out.push(vec![(i, NAN_B), (j, NAN_A)]);
            out.push(vec![(i, NAN_C), (j, NAN_D)]);
            out.push(vec![(i, NAN_D), (j, NAN_E)]);
        }
    }
    out
}

fn apply(base: &[f32], inj: &[(usize, f32)]) -> Vec<f32> {
    let mut v = base.to_vec();
    for &(i, x) in inj {
        v[i] = x;
    }
    v
}

// ---------------------------------------------------------------------------
// Level 0
// ---------------------------------------------------------------------------

#[test]
fn sparse_nan_dot_det2() {
    let l = libs();
    let (cdot, rdot) = l.get::<FnVVf>("c2Dot");
    let (cdet, rdet) = l.get::<FnVVf>("c2Det2");
    let mut g = Rng::new(0xD07);
    for trial in 0..4 {
        let base = finite_base(4, &mut g); // a.x a.y b.x b.y
        for inj in injections(4) {
            let p = apply(&base, &inj);
            let a = V::new(p[0], p[1]);
            let b = V::new(p[2], p[3]);
            ck_f32!("c2Dot/sparse", unsafe { cdot(a, b) }, unsafe { rdot(a, b) },
                    "trial={trial} inj={inj:x?} a={:#010x?} b={:#010x?}", a.bits(), b.bits());
            ck_f32!("c2Det2/sparse", unsafe { cdet(a, b) }, unsafe { rdet(a, b) },
                    "trial={trial} inj={inj:x?} a={:#010x?} b={:#010x?}", a.bits(), b.bits());
        }
    }
}

#[test]
fn sparse_nan_add_sub_minmax() {
    let l = libs();
    let (cadd, radd) = l.get::<FnVVV>("c2Add");
    let (csub, rsub) = l.get::<FnVVV>("c2Sub");
    let (cmax, rmax) = l.get::<FnVVV>("c2Maxv");
    let (cmin, rmin) = l.get::<FnVVV>("c2Minv");
    let mut g = Rng::new(0xADD);
    for trial in 0..4 {
        let base = finite_base(4, &mut g);
        for inj in injections(4) {
            let p = apply(&base, &inj);
            let a = V::new(p[0], p[1]);
            let b = V::new(p[2], p[3]);
            ck_v!("c2Add/sparse", unsafe { cadd(a, b) }, unsafe { radd(a, b) }, "trial={trial} inj={inj:x?}");
            ck_v!("c2Sub/sparse", unsafe { csub(a, b) }, unsafe { rsub(a, b) }, "trial={trial} inj={inj:x?}");
            ck_v!("c2Maxv/sparse", unsafe { cmax(a, b) }, unsafe { rmax(a, b) }, "trial={trial} inj={inj:x?}");
            ck_v!("c2Minv/sparse", unsafe { cmin(a, b) }, unsafe { rmin(a, b) }, "trial={trial} inj={inj:x?}");
        }
    }
}

#[test]
fn sparse_nan_mulvs_div_len_norm() {
    let l = libs();
    let (cmul, rmul) = l.get::<FnVsV>("c2Mulvs");
    let (cdiv, rdiv) = l.get::<FnVsV>("c2Div");
    let (clen, rlen) = l.get::<FnVf>("c2Len");
    let (cnorm, rnorm) = l.get::<FnVV>("c2Norm");
    let mut g = Rng::new(0x111);
    for trial in 0..4 {
        let base = finite_base(3, &mut g); // a.x a.y s
        for inj in injections(3) {
            let p = apply(&base, &inj);
            let a = V::new(p[0], p[1]);
            let s = p[2];
            ck_v!("c2Mulvs/sparse", unsafe { cmul(a, s) }, unsafe { rmul(a, s) }, "trial={trial} inj={inj:x?}");
            ck_v!("c2Div/sparse", unsafe { cdiv(a, s) }, unsafe { rdiv(a, s) }, "trial={trial} inj={inj:x?}");
            ck_f32!("c2Len/sparse", unsafe { clen(a) }, unsafe { rlen(a) }, "trial={trial} inj={inj:x?}");
            ck_v!("c2Norm/sparse", unsafe { cnorm(a) }, unsafe { rnorm(a) }, "trial={trial} inj={inj:x?}");
        }
    }
}

#[test]
fn sparse_nan_clampv() {
    let l = libs();
    let (c, r) = l.get::<FnVVVV>("c2Clampv");
    let mut g = Rng::new(0xC1A);
    for trial in 0..3 {
        let base = finite_base(6, &mut g); // a lo hi
        for inj in injections(6) {
            let p = apply(&base, &inj);
            let a = V::new(p[0], p[1]);
            let lo = V::new(p[2], p[3]);
            let hi = V::new(p[4], p[5]);
            ck_v!("c2Clampv/sparse", unsafe { c(a, lo, hi) }, unsafe { r(a, lo, hi) },
                  "trial={trial} inj={inj:x?}");
        }
    }
}

#[test]
fn sparse_nan_rotations() {
    let l = libs();
    let (c, r) = l.get::<FnRVV>("c2Mulrv");
    let (ct, rt) = l.get::<FnRVV>("c2MulrvT");
    let (cx, rx) = l.get::<FnXVV>("c2Mulxv");
    let mut g = Rng::new(0xB07);
    for trial in 0..3 {
        let base = finite_base(6, &mut g); // r.c r.s v.x v.y p.x p.y
        for inj in injections(6) {
            let p = apply(&base, &inj);
            let rot = R { c: p[0], s: p[1] };
            let v = V::new(p[2], p[3]);
            ck_v!("c2Mulrv/sparse", unsafe { c(rot, v) }, unsafe { r(rot, v) }, "trial={trial} inj={inj:x?}");
            ck_v!("c2MulrvT/sparse", unsafe { ct(rot, v) }, unsafe { rt(rot, v) }, "trial={trial} inj={inj:x?}");
            let x = X { p: V::new(p[4], p[5]), r: rot };
            ck_v!("c2Mulxv/sparse", unsafe { cx(x, v) }, unsafe { rx(x, v) }, "trial={trial} inj={inj:x?}");
        }
    }
}

// ---------------------------------------------------------------------------
// Level 2 — simplex functions
// ---------------------------------------------------------------------------

/// Slot layout for the simplex tests:
/// 0 = div, 1..4 = u0..u2, 4..10 = p0..p2 (x,y), 10..16 = sA0..sA2, 16..22 = sB0..sB2
const SLOTS: usize = 22;

fn build_simplex(p: &[f32], count: i32) -> Simplex {
    let mut s = Simplex::default();
    s.count = count;
    s.div = p[0];
    for k in 0..3 {
        s.verts[k].u = p[1 + k];
        s.verts[k].p = V::new(p[4 + 2 * k], p[5 + 2 * k]);
        s.verts[k].sA = V::new(p[10 + 2 * k], p[11 + 2 * k]);
        s.verts[k].sB = V::new(p[16 + 2 * k], p[17 + 2 * k]);
        s.verts[k].iA = k as i32;
        s.verts[k].iB = (2 - k) as i32;
    }
    s
}

#[test]
fn sparse_nan_witness() {
    let l = libs();
    let (c, r) = l.get::<FnWitness>("c2Witness");
    let mut g = Rng::new(0x717);
    for count in 1..=3i32 {
        let base = finite_base(SLOTS, &mut g);
        for inj in injections(SLOTS) {
            let p = apply(&base, &inj);
            let mut cs = build_simplex(&p, count);
            let mut rs = cs;
            let mut ca = V::default();
            let mut cb = V::default();
            let mut ra = V::default();
            let mut rb = V::default();
            unsafe {
                c(&mut cs, &mut ca, &mut cb);
                r(&mut rs, &mut ra, &mut rb);
            }
            ck_v!("c2Witness/sparse a", ca, ra, "count={count} inj={inj:x?}");
            ck_v!("c2Witness/sparse b", cb, rb, "count={count} inj={inj:x?}");
        }
    }
}

#[test]
fn sparse_nan_c2l() {
    let l = libs();
    let (c, r) = l.get::<FnSimplexV>("c2L");
    let mut g = Rng::new(0x2C2);
    for count in 0..=4i32 {
        let base = finite_base(SLOTS, &mut g);
        for inj in injections(SLOTS) {
            let p = apply(&base, &inj);
            let mut cs = build_simplex(&p, count);
            let mut rs = cs;
            let (cv, rv) = unsafe { (c(&mut cs), r(&mut rs)) };
            ck_v!("c2L/sparse", cv, rv, "count={count} inj={inj:x?}");
        }
    }
}

#[test]
fn sparse_nan_c22_c23_d_metric() {
    let l = libs();
    let (c22c, c22r) = l.get::<FnSimplexVoid>("c22");
    let (c23c, c23r) = l.get::<FnSimplexVoid>("c23");
    let (cdc, cdr) = l.get::<FnSimplexV>("c2D");
    let (cmc, cmr) = l.get::<FnSimplexF>("c2GJKSimplexMetric");
    let mut g = Rng::new(0x2223);
    for &count in &[2i32, 3] {
        let base = finite_base(SLOTS, &mut g);
        for inj in injections(SLOTS) {
            let p = apply(&base, &inj);
            let mut cs = build_simplex(&p, count);
            let mut rs = cs;

            let (cm, rm) = unsafe { (cmc(&mut cs), cmr(&mut rs)) };
            ck_f32!("c2GJKSimplexMetric/sparse", cm, rm, "count={count} inj={inj:x?}");

            let (cd, rd) = unsafe { (cdc(&mut cs), cdr(&mut rs)) };
            ck_v!("c2D/sparse", cd, rd, "count={count} inj={inj:x?}");

            unsafe {
                if count == 2 {
                    c22c(&mut cs);
                    c22r(&mut rs);
                } else {
                    c23c(&mut cs);
                    c23r(&mut rs);
                }
            }
            ck_bytes!("c22/c23 sparse", cs, rs, "count={count} inj={inj:x?}");
        }
    }
}

// ---------------------------------------------------------------------------
// Level 3/4 — end to end
// ---------------------------------------------------------------------------

#[test]
fn sparse_nan_gjk() {
    let l = libs();
    let (c, r) = l.get::<FnGJK>("c2GJK");
    let mut g = Rng::new(0x616);

    // slots: circle(px,py,r) aabb(minx,miny,maxx,maxy) capsule(ax,ay,bx,by,r)
    //        + transforms (ax.p.x, ax.p.y, ax.r.c, ax.r.s, bx.*)
    const N: usize = 12 + 8;
    for trial in 0..2 {
        let mut base = finite_base(N, &mut g);
        // make the AABB well-formed and the rotations unit-ish
        base[3] = -2.0;
        base[4] = -2.0;
        base[5] = 3.0;
        base[6] = 4.0;
        base[14] = 1.0; // ax.r.c
        base[15] = 0.0; // ax.r.s
        base[18] = 1.0; // bx.r.c
        base[19] = 0.0; // bx.r.s

        for inj in injections(N) {
            let p = apply(&base, &inj);
            let circle = Circle { p: V::new(p[0], p[1]), r: p[2] };
            let bb = AABB { min: V::new(p[3], p[4]), max: V::new(p[5], p[6]) };
            let cap = Capsule { a: V::new(p[7], p[8]), b: V::new(p[9], p[10]), r: p[11] };
            let ax = X { p: V::new(p[12], p[13]), r: R { c: p[14], s: p[15] } };
            let bx = X { p: V::new(p[16], p[17]), r: R { c: p[18], s: p[19] } };

            let shapes: [(*const c_void, i32); 3] = [
                (&circle as *const Circle as *const c_void, C2_TYPE_CIRCLE),
                (&bb as *const AABB as *const c_void, C2_TYPE_AABB),
                (&cap as *const Capsule as *const c_void, C2_TYPE_CAPSULE),
            ];

            for (si, &(pa, ta)) in shapes.iter().enumerate() {
                let (pb, tb) = shapes[(si + 1) % 3];
                for &ur in &[0i32, 1] {
                    for with_x in [false, true] {
                        let (axp, bxp) = if with_x {
                            (&ax as *const X, &bx as *const X)
                        } else {
                            (std::ptr::null(), std::ptr::null())
                        };
                        let poison = f32::from_bits(0xA5A5_A5A5);
                        let mut ca = V::new(poison, poison);
                        let mut cb = ca;
                        let mut ra = ca;
                        let mut rb = ca;
                        let mut cit = -1i32;
                        let mut rit = -1i32;
                        let mut cc = GJKCache::default();
                        let mut rc = GJKCache::default();
                        let cd = unsafe {
                            c(pa, ta, axp, pb, tb, bxp, &mut ca, &mut cb, ur, &mut cit, &mut cc)
                        };
                        let rd = unsafe {
                            r(pa, ta, axp, pb, tb, bxp, &mut ra, &mut rb, ur, &mut rit, &mut rc)
                        };
                        ck_f32!("c2GJK/sparse dist", cd, rd,
                                "trial={trial} inj={inj:x?} ta={ta} tb={tb} ur={ur} x={with_x}");
                        ck_v!("c2GJK/sparse outA", ca, ra,
                              "trial={trial} inj={inj:x?} ta={ta} tb={tb} ur={ur} x={with_x}");
                        ck_v!("c2GJK/sparse outB", cb, rb,
                              "trial={trial} inj={inj:x?} ta={ta} tb={tb} ur={ur} x={with_x}");
                        ck_i32!("c2GJK/sparse iters", cit, rit,
                                "trial={trial} inj={inj:x?} ta={ta} tb={tb} ur={ur} x={with_x}");
                        ck_bytes!("c2GJK/sparse cache", cc, rc,
                                  "trial={trial} inj={inj:x?} ta={ta} tb={tb} ur={ur} x={with_x}");
                    }
                }
            }
        }
    }
}

#[test]
fn sparse_nan_gjk_wrapper() {
    let l = libs();
    let (c, r) = l.get::<FnGjkWrapper>("gjk");
    let mut g = Rng::new(0x9AB);
    for trial in 0..3 {
        let mut base = finite_base(9, &mut g);
        base[0] = -2.0;
        base[1] = -2.0;
        base[2] = 3.0;
        base[3] = 4.0;
        for inj in injections(9) {
            let p = apply(&base, &inj);
            for rev in [0i8, 1] {
                let poison = f32::from_bits(0xA5A5_A5A5);
                let mut ca = V::new(poison, poison);
                let mut cb = ca;
                let mut ra = ca;
                let mut rb = ca;
                unsafe {
                    c(rev, &mut ca, &mut cb, p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8]);
                    r(rev, &mut ra, &mut rb, p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8]);
                }
                ck_v!("gjk/sparse outA", ca, ra, "trial={trial} inj={inj:x?} rev={rev}");
                ck_v!("gjk/sparse outB", cb, rb, "trial={trial} inj={inj:x?} rev={rev}");
            }
        }
    }
}
