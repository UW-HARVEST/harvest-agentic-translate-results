//! Phase B (blind-spot closure) — DISTINCT NaN PAYLOADS.
//!
//! `ADDSS`/`MULSS` propagate the *destination* operand's NaN in preference to
//! the source operand's, so for a commutative `a + b` / `a * b` the compiler's
//! choice of which operand lands in the destination register decides WHICH NaN
//! payload survives. That is invisible unless the two operands are NaNs with
//! DIFFERENT payloads — a single `f32::NAN` everywhere cannot see it.
//!
//! `src/lib.rs` models this explicitly with `addp(dst, src)` / `mulp(dst, src)`,
//! so these tests are what actually pins that model down against the C build.

#![allow(non_snake_case)]

#[macro_use]
mod common;

use common::*;
use std::ffi::c_void;

/// A pool of NaNs with pairwise-distinct payloads (quiet, signalling, and both
/// signs), plus the non-NaN specials for good measure.
fn nan_pool() -> Vec<f32> {
    let mut v = Vec::new();
    for payload in [
        0x0000_0001u32,
        0x0000_0002,
        0x0000_1234,
        0x0003_ffff,
        0x0020_0000,
        0x0030_0000,
        0x003f_ffff,
    ] {
        // quiet NaN, positive and negative
        v.push(f32::from_bits(0x7fc0_0000 | payload));
        v.push(f32::from_bits(0xffc0_0000 | payload));
        // signalling NaN, positive and negative
        v.push(f32::from_bits(0x7f80_0000 | payload));
        v.push(f32::from_bits(0xff80_0000 | payload));
    }
    v.push(f32::INFINITY);
    v.push(f32::NEG_INFINITY);
    v.push(0.0);
    v.push(-0.0);
    v.push(1.0);
    v.push(-1.0);
    v
}

fn nan_v(pool: &[f32], i: usize, j: usize) -> V {
    V::new(pool[i % pool.len()], pool[j % pool.len()])
}

/// `c2Dot` — two multiplies feeding one add: three chances to pick the wrong
/// destination operand.
#[test]
fn nan_dot() {
    let l = libs();
    let (c, r) = l.get::<FnVVf>("c2Dot");
    let pool = nan_pool();
    let n = pool.len();
    for i in 0..n {
        for j in 0..n {
            for k in 0..n {
                let a = nan_v(&pool, i, j);
                let b = nan_v(&pool, k, i + 1);
                ck_f32!("c2Dot/nan", unsafe { c(a, b) }, unsafe { r(a, b) },
                        "a=({:#010x},{:#010x}) b=({:#010x},{:#010x})",
                        a.x.to_bits(), a.y.to_bits(), b.x.to_bits(), b.y.to_bits());
            }
        }
    }
}

/// `c2Det2` — multiply/multiply/subtract.
#[test]
fn nan_det2() {
    let l = libs();
    let (c, r) = l.get::<FnVVf>("c2Det2");
    let pool = nan_pool();
    let n = pool.len();
    for i in 0..n {
        for j in 0..n {
            for k in 0..n {
                let a = nan_v(&pool, i, j);
                let b = nan_v(&pool, k, j + 2);
                ck_f32!("c2Det2/nan", unsafe { c(a, b) }, unsafe { r(a, b) },
                        "a=({:#010x},{:#010x}) b=({:#010x},{:#010x})",
                        a.x.to_bits(), a.y.to_bits(), b.x.to_bits(), b.y.to_bits());
            }
        }
    }
}

/// `c2Add` / `c2Sub` / `c2Mulvs` / `c2Div` — one op per component.
#[test]
fn nan_add_sub_mul_div() {
    let l = libs();
    let (cadd, radd) = l.get::<FnVVV>("c2Add");
    let (csub, rsub) = l.get::<FnVVV>("c2Sub");
    let (cmul, rmul) = l.get::<FnVsV>("c2Mulvs");
    let (cdiv, rdiv) = l.get::<FnVsV>("c2Div");
    let pool = nan_pool();
    let n = pool.len();
    for i in 0..n {
        for j in 0..n {
            let a = nan_v(&pool, i, j);
            let b = nan_v(&pool, j, i);
            ck_v!("c2Add/nan", unsafe { cadd(a, b) }, unsafe { radd(a, b) }, "a={:#010x?} b={:#010x?}", a.bits(), b.bits());
            ck_v!("c2Sub/nan", unsafe { csub(a, b) }, unsafe { rsub(a, b) }, "a={:#010x?} b={:#010x?}", a.bits(), b.bits());
            for k in 0..n {
                let s = pool[k];
                ck_v!("c2Mulvs/nan", unsafe { cmul(a, s) }, unsafe { rmul(a, s) },
                      "a={:#010x?} s={:#010x}", a.bits(), s.to_bits());
                ck_v!("c2Div/nan", unsafe { cdiv(a, s) }, unsafe { rdiv(a, s) },
                      "a={:#010x?} s={:#010x}", a.bits(), s.to_bits());
            }
        }
    }
}

/// `c2Mulrv` / `c2MulrvT` / `c2Mulxv` — four multiplies plus adds each.
#[test]
fn nan_rotations() {
    let l = libs();
    let (c, r) = l.get::<FnRVV>("c2Mulrv");
    let (ct, rt) = l.get::<FnRVV>("c2MulrvT");
    let (cx, rx) = l.get::<FnXVV>("c2Mulxv");
    let pool = nan_pool();
    let n = pool.len();
    for i in 0..n {
        for j in 0..n {
            for k in 0..n {
                let rot = R { c: pool[i], s: pool[j] };
                let v = nan_v(&pool, k, i + j + 1);
                ck_v!("c2Mulrv/nan", unsafe { c(rot, v) }, unsafe { r(rot, v) },
                      "rot=({:#010x},{:#010x}) v={:#010x?}", rot.c.to_bits(), rot.s.to_bits(), v.bits());
                ck_v!("c2MulrvT/nan", unsafe { ct(rot, v) }, unsafe { rt(rot, v) },
                      "rot=({:#010x},{:#010x}) v={:#010x?}", rot.c.to_bits(), rot.s.to_bits(), v.bits());
                let x = X { p: nan_v(&pool, j, k), r: rot };
                ck_v!("c2Mulxv/nan", unsafe { cx(x, v) }, unsafe { rx(x, v) },
                      "x={x:?} v={:#010x?}", v.bits());
            }
        }
    }
}

/// `c2Len` / `c2Norm` — `sqrtf` of a NaN must keep the payload identically.
#[test]
fn nan_len_norm() {
    let l = libs();
    let (cl, rl) = l.get::<FnVf>("c2Len");
    let (cn, rn) = l.get::<FnVV>("c2Norm");
    let pool = nan_pool();
    let n = pool.len();
    for i in 0..n {
        for j in 0..n {
            let a = nan_v(&pool, i, j);
            ck_f32!("c2Len/nan", unsafe { cl(a) }, unsafe { rl(a) }, "a={:#010x?}", a.bits());
            ck_v!("c2Norm/nan", unsafe { cn(a) }, unsafe { rn(a) }, "a={:#010x?}", a.bits());
        }
    }
}

/// `c2Maxv` / `c2Minv` / `c2Clampv` — comparison-based, so a NaN operand makes
/// the ternary fall to its else-branch. Asymmetric and easy to get backwards.
#[test]
fn nan_minmax_clamp() {
    let l = libs();
    let (cx, rx) = l.get::<FnVVV>("c2Maxv");
    let (cn, rn) = l.get::<FnVVV>("c2Minv");
    let (cc, rc) = l.get::<FnVVVV>("c2Clampv");
    let pool = nan_pool();
    let n = pool.len();
    for i in 0..n {
        for j in 0..n {
            let a = nan_v(&pool, i, j);
            let b = nan_v(&pool, j, i);
            ck_v!("c2Maxv/nan", unsafe { cx(a, b) }, unsafe { rx(a, b) }, "a={:#010x?} b={:#010x?}", a.bits(), b.bits());
            ck_v!("c2Minv/nan", unsafe { cn(a, b) }, unsafe { rn(a, b) }, "a={:#010x?} b={:#010x?}", a.bits(), b.bits());
            for k in 0..n {
                let hi = nan_v(&pool, k, k + 1);
                ck_v!("c2Clampv/nan", unsafe { cc(a, b, hi) }, unsafe { rc(a, b, hi) },
                      "a={:#010x?} lo={:#010x?} hi={:#010x?}", a.bits(), b.bits(), hi.bits());
            }
        }
    }
}

/// `c2Witness` / `c2L` — `den * u` where BOTH can be distinct NaNs. This is the
/// exact case a single shared `f32::NAN` cannot distinguish.
#[test]
fn nan_witness_and_l() {
    let l = libs();
    let (cw, rw) = l.get::<FnWitness>("c2Witness");
    let (cl, rl) = l.get::<FnSimplexV>("c2L");
    let pool = nan_pool();
    let n = pool.len();

    for count in 1..=3i32 {
        for i in 0..n {
            for j in 0..n {
                for k in 0..n {
                    let mut cs = Simplex::default();
                    cs.count = count;
                    cs.div = pool[i]; // -> den = 1.0 / div
                    for (vi, v) in cs.verts.iter_mut().enumerate() {
                        v.u = pool[(j + vi) % n];
                        v.sA = nan_v(&pool, k + vi, i + vi);
                        v.sB = nan_v(&pool, i + vi, j + vi);
                        v.p = nan_v(&pool, j + vi, k + vi);
                    }
                    let mut rs = cs;

                    let mut ca = V::default();
                    let mut cb = V::default();
                    let mut ra = V::default();
                    let mut rb = V::default();
                    unsafe {
                        cw(&mut cs, &mut ca, &mut cb);
                        rw(&mut rs, &mut ra, &mut rb);
                    }
                    ck_v!("c2Witness/nan a", ca, ra,
                          "count={count} div={:#010x} u0={:#010x}", cs.div.to_bits(), cs.verts[0].u.to_bits());
                    ck_v!("c2Witness/nan b", cb, rb,
                          "count={count} div={:#010x} u0={:#010x}", cs.div.to_bits(), cs.verts[0].u.to_bits());

                    let (cv, rv) = unsafe { (cl(&mut cs), rl(&mut rs)) };
                    ck_v!("c2L/nan", cv, rv,
                          "count={count} div={:#010x} u0={:#010x}", cs.div.to_bits(), cs.verts[0].u.to_bits());
                }
            }
        }
    }
}

/// `c22` / `c23` / `c2D` / `c2GJKSimplexMetric` with NaN-payload vertices.
#[test]
fn nan_simplex_reduction() {
    let l = libs();
    let (c22c, c22r) = l.get::<FnSimplexVoid>("c22");
    let (c23c, c23r) = l.get::<FnSimplexVoid>("c23");
    let (cdc, cdr) = l.get::<FnSimplexV>("c2D");
    let (cmc, cmr) = l.get::<FnSimplexF>("c2GJKSimplexMetric");
    let pool = nan_pool();
    let n = pool.len();

    for i in 0..n {
        for j in 0..n {
            for k in 0..n {
                for &count in &[2i32, 3] {
                    let mut cs = Simplex::default();
                    cs.count = count;
                    cs.div = pool[(i + j) % n];
                    cs.verts[0].p = nan_v(&pool, i, j);
                    cs.verts[1].p = nan_v(&pool, j, k);
                    cs.verts[2].p = nan_v(&pool, k, i);
                    cs.verts[3].p = nan_v(&pool, i + 1, k + 1);
                    for (vi, v) in cs.verts.iter_mut().enumerate() {
                        v.u = pool[(k + vi) % n];
                        v.iA = vi as i32;
                        v.iB = (3 - vi) as i32;
                    }

                    let mut rs = cs;
                    unsafe {
                        if count == 2 {
                            c22c(&mut cs);
                            c22r(&mut rs);
                        } else {
                            c23c(&mut cs);
                            c23r(&mut rs);
                        }
                    }
                    ck_bytes!("c2x reduce/nan", cs, rs,
                              "count={count} p0={:#010x?} p1={:#010x?} p2={:#010x?}",
                              cs.verts[0].p.bits(), cs.verts[1].p.bits(), cs.verts[2].p.bits());

                    let (cv, rv) = unsafe { (cdc(&mut cs), cdr(&mut rs)) };
                    ck_v!("c2D/nan", cv, rv, "count={count}");
                    let (cm, rm) = unsafe { (cmc(&mut cs), cmr(&mut rs)) };
                    ck_f32!("c2GJKSimplexMetric/nan", cm, rm, "count={count}");
                }
            }
        }
    }
}

/// `c2GJK` end to end with NaN-payload shape data — the payload has to survive
/// the whole pipeline identically.
#[test]
fn nan_gjk_end_to_end() {
    let l = libs();
    let (c, r) = l.get::<FnGJK>("c2GJK");
    let pool = nan_pool();
    let n = pool.len();

    let mut checked = 0usize;
    for i in 0..n {
        for j in 0..n {
            // circle with NaN centre/radius vs AABB with NaN corners
            let circle = Circle { p: nan_v(&pool, i, j), r: pool[(i + j) % n] };
            let bb = AABB { min: nan_v(&pool, j, i), max: nan_v(&pool, i + 1, j + 1) };
            let cap = Capsule {
                a: nan_v(&pool, i, i),
                b: nan_v(&pool, j, j),
                r: pool[(i * 3 + j) % n],
            };

            let shapes: [(*const c_void, i32); 3] = [
                (&circle as *const Circle as *const c_void, C2_TYPE_CIRCLE),
                (&bb as *const AABB as *const c_void, C2_TYPE_AABB),
                (&cap as *const Capsule as *const c_void, C2_TYPE_CAPSULE),
            ];

            for &(pa, ta) in shapes.iter() {
                for &(pb, tb) in shapes.iter() {
                    for &ur in &[0i32, 1] {
                        let poison = f32::from_bits(0xA5A5_A5A5);
                        let mut ca = V::new(poison, poison);
                        let mut cb = ca;
                        let mut ra = ca;
                        let mut rb = ca;
                        let mut cit = -1i32;
                        let mut rit = -1i32;
                        let mut ccache = GJKCache::default();
                        let mut rcache = GJKCache::default();
                        let cd = unsafe {
                            c(pa, ta, std::ptr::null(), pb, tb, std::ptr::null(),
                              &mut ca, &mut cb, ur, &mut cit, &mut ccache)
                        };
                        let rd = unsafe {
                            r(pa, ta, std::ptr::null(), pb, tb, std::ptr::null(),
                              &mut ra, &mut rb, ur, &mut rit, &mut rcache)
                        };
                        ck_f32!("c2GJK/nan dist", cd, rd, "ta={ta} tb={tb} ur={ur} i={i} j={j}");
                        ck_v!("c2GJK/nan outA", ca, ra, "ta={ta} tb={tb} ur={ur} i={i} j={j}");
                        ck_v!("c2GJK/nan outB", cb, rb, "ta={ta} tb={tb} ur={ur} i={i} j={j}");
                        ck_i32!("c2GJK/nan iters", cit, rit, "ta={ta} tb={tb} ur={ur} i={i} j={j}");
                        ck_bytes!("c2GJK/nan cache", ccache, rcache, "ta={ta} tb={tb} ur={ur} i={i} j={j}");
                        checked += 1;
                    }
                }
            }
        }
    }
    eprintln!("nan_gjk_end_to_end: {checked} configurations");
}

/// `gjk` public wrapper with NaN payloads in every one of its nine floats.
#[test]
fn nan_gjk_wrapper() {
    let l = libs();
    let (c, r) = l.get::<FnGjkWrapper>("gjk");
    let pool = nan_pool();
    let n = pool.len();
    let mut g = Rng::new(0x4E_414E);
    for slot in 0..9usize {
        for &nanv in pool.iter() {
            for t in 0..40 {
                let mut p = [
                    g.grid(), g.grid(), g.grid(), g.grid(),
                    g.grid(), g.grid(), g.grid(), g.grid(),
                    g.grid().abs(),
                ];
                p[slot] = nanv;
                if t % 3 == 0 {
                    p[(slot + 4) % 9] = pool[(t + slot) % n];
                }
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
                    ck_v!("gjk/nan outA", ca, ra, "slot={slot} nan={:#010x} rev={rev} p={p:?}", nanv.to_bits());
                    ck_v!("gjk/nan outB", cb, rb, "slot={slot} nan={:#010x} rev={rev} p={p:?}", nanv.to_bits());
                }
            }
        }
    }
}
