#![allow(non_snake_case)]
//! NaN-OPERAND-ORDER differential tests.
//!
//! IEEE-754 leaves it unspecified WHICH input NaN a binary operation
//! propagates, so the answer is fixed by the emitted instruction sequence.
//! On x86-64 `OPss dst, src` returns the quieted `dst` when `dst` is a NaN,
//! otherwise the quieted `src` — so the operand order GCC chose for each
//! expression in `c_src/src/lib.c` is observable whenever two operands are
//! NaNs with DIFFERENT sign or payload.
//!
//! These tests feed distinct NaN bit patterns into every float slot of every
//! exported function and compare the results bit-for-bit. A single NaN value
//! (as `f32::NAN` alone) cannot detect an ordering mistake; distinct payloads
//! can.

mod common;
use common::*;

fn v(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

/// Distinct NaN payloads + signs, quiet and signalling, plus infinities and
/// one ordinary value so that "real op NaN" is covered as well as "NaN op NaN".
const POOL: [u32; 8] = [
    0x7fc0_0001, // +QNaN payload 1
    0xffc0_0002, // -QNaN payload 2
    0x7fc0_0003, // +QNaN payload 3
    0x7f80_0001, // +SNaN  (must be quieted to 0x7fc00001)
    0xff80_0002, // -SNaN
    0x7f80_0000, // +inf
    0xff80_0000, // -inf
    0x4000_0000, // 2.0
];

fn p(i: usize) -> f32 {
    f32::from_bits(POOL[i])
}
const NP: usize = POOL.len();

// ------------------------------------------------------- 2-float functions
#[test]
fn nan_order_unary_vec() {
    let (cn, rn) = pair::<FnV_v>("c2Neg");
    let (cs, rs) = pair::<FnV_v>("c2Skew");
    let (cw, rw) = pair::<FnV_v>("c2CCW90");
    let (cl, rl) = pair::<FnF_v>("c2Len");
    let (cnm, rnm) = pair::<FnV_v>("c2Norm");
    for i in 0..NP {
        for j in 0..NP {
            let a = v(p(i), p(j));
            same("c2Neg", cn(a), rn(a));
            same("c2Skew", cs(a), rs(a));
            same("c2CCW90", cw(a), rw(a));
            same("c2Len", cl(a), rl(a));
            same("c2Norm", cnm(a), rnm(a));
        }
    }
}

// ------------------------------------------------------- 3-float functions
#[test]
fn nan_order_vec_scalar() {
    let (cm, rm) = pair::<FnV_vf>("c2Mulvs");
    let (cd, rd) = pair::<FnV_vf>("c2Div");
    for i in 0..NP {
        for j in 0..NP {
            for k in 0..NP {
                let a = v(p(i), p(j));
                let b = p(k);
                same("c2Mulvs", cm(a, b), rm(a, b));
                same("c2Div", cd(a, b), rd(a, b));
            }
        }
    }
}

// ------------------------------------------------------- 4-float functions
#[test]
fn nan_order_vec_binary() {
    let (cd, rd) = pair::<FnF_vv>("c2Dot");
    let (ct, rt) = pair::<FnF_vv>("c2Det2");
    let (ca, ra) = pair::<FnV_vv>("c2Add");
    let (cs, rs) = pair::<FnV_vv>("c2Sub");
    let (cx, rx) = pair::<FnV_vv>("c2Maxv");
    let (ci, ri) = pair::<FnV_vv>("c2Minv");
    for i in 0..NP {
        for j in 0..NP {
            for k in 0..NP {
                for l in 0..NP {
                    let a = v(p(i), p(j));
                    let b = v(p(k), p(l));
                    same("c2Dot", cd(a, b), rd(a, b));
                    same("c2Det2", ct(a, b), rt(a, b));
                    same("c2Add", ca(a, b), ra(a, b));
                    same("c2Sub", cs(a, b), rs(a, b));
                    same("c2Maxv", cx(a, b), rx(a, b));
                    same("c2Minv", ci(a, b), ri(a, b));
                }
            }
        }
    }
}

#[test]
fn nan_order_rotations() {
    let (cm, rm) = pair::<FnV_rv>("c2Mulrv");
    let (ct, rt) = pair::<FnV_rv>("c2MulrvT");
    for i in 0..NP {
        for j in 0..NP {
            for k in 0..NP {
                for l in 0..NP {
                    let r = c2r { c: p(i), s: p(j) };
                    let b = v(p(k), p(l));
                    same("c2Mulrv", cm(r, b), rm(r, b));
                    same("c2MulrvT", ct(r, b), rt(r, b));
                }
            }
        }
    }
}

// ------------------------------------------------------- 6-float functions
#[test]
fn nan_order_clamp_and_xform() {
    let (cc, rc) = pair::<FnV_vvv>("c2Clampv");
    let (cx, rx) = pair::<FnV_xv>("c2Mulxv");
    let mut g = Rng::new(0x5001);
    for _ in 0..60_000 {
        let idx = |g: &mut Rng| p(g.below(NP as u32) as usize);
        let a = v(idx(&mut g), idx(&mut g));
        let lo = v(idx(&mut g), idx(&mut g));
        let hi = v(idx(&mut g), idx(&mut g));
        same("c2Clampv", cc(a, lo, hi), rc(a, lo, hi));
        let x = c2x {
            p: v(idx(&mut g), idx(&mut g)),
            r: c2r {
                c: idx(&mut g),
                s: idx(&mut g),
            },
        };
        same("c2Mulxv", cx(x, a), rx(x, a));
    }
}

// ------------------------------------------------------- boolean predicates
#[test]
fn nan_order_predicates() {
    let (ccc, rcc) = pair::<FnI_Cir_Cir>("c2CircletoCircle");
    let (cca, rca) = pair::<FnI_Cir_AABB>("c2CircletoAABB");
    let (ccp, rcp) = pair::<FnI_Cir_Cap>("c2CircletoCapsule");
    let (caa, raa) = pair::<FnI_AABB_AABB>("c2AABBtoAABB");
    let (cac, rac) = pair::<FnI_AABB_Cap>("c2AABBtoCapsule");
    let (cpp, rpp) = pair::<FnI_Cap_Cap>("c2CapsuletoCapsule");
    let mut g = Rng::new(0x5002);
    let idx = |g: &mut Rng| p(g.below(NP as u32) as usize);
    for _ in 0..40_000 {
        let c1 = c2Circle {
            p: v(idx(&mut g), idx(&mut g)),
            r: idx(&mut g),
        };
        let c2 = c2Circle {
            p: v(idx(&mut g), idx(&mut g)),
            r: idx(&mut g),
        };
        let b1 = c2AABB {
            min: v(idx(&mut g), idx(&mut g)),
            max: v(idx(&mut g), idx(&mut g)),
        };
        let b2 = c2AABB {
            min: v(idx(&mut g), idx(&mut g)),
            max: v(idx(&mut g), idx(&mut g)),
        };
        let k1 = c2Capsule {
            a: v(idx(&mut g), idx(&mut g)),
            b: v(idx(&mut g), idx(&mut g)),
            r: idx(&mut g),
        };
        let k2 = c2Capsule {
            a: v(idx(&mut g), idx(&mut g)),
            b: v(idx(&mut g), idx(&mut g)),
            r: idx(&mut g),
        };
        same("c2CircletoCircle", ccc(c1, c2), rcc(c1, c2));
        same("c2CircletoAABB", cca(c1, b1), rca(c1, b1));
        same("c2CircletoCapsule", ccp(c1, k1), rcp(c1, k1));
        same("c2AABBtoAABB", caa(b1, b2), raa(b1, b2));
        same("c2AABBtoCapsule", cac(b1, k1), rac(b1, k1));
        same("c2CapsuletoCapsule", cpp(k1, k2), rpp(k1, k2));
    }
}

// ------------------------------------------------------- simplex machinery
#[test]
fn nan_order_simplex() {
    let (c22c, c22r) = pair::<FnSimplexVoid>("c22");
    let (c23c, c23r) = pair::<FnSimplexVoid>("c23");
    let (cmc, cmr) = pair::<FnSimplexF>("c2GJKSimplexMetric");
    let (cdc, cdr) = pair::<FnSimplexV>("c2D");
    let (clc, clr) = pair::<FnSimplexV>("c2L");
    let (cwc, cwr) = pair::<FnWitness>("c2Witness");
    let mut g = Rng::new(0x5003);
    let idx = |g: &mut Rng| p(g.below(NP as u32) as usize);
    for count in [1i32, 2, 3] {
        for _ in 0..20_000 {
            let mut s = c2Simplex::default();
            for k in 0..4 {
                s.verts[k] = c2sv {
                    sA: v(idx(&mut g), idx(&mut g)),
                    sB: v(idx(&mut g), idx(&mut g)),
                    p: v(idx(&mut g), idx(&mut g)),
                    u: idx(&mut g),
                    iA: g.below(4) as i32,
                    iB: g.below(4) as i32,
                };
            }
            s.div = idx(&mut g);
            s.count = count;

            let (mut a, mut b) = (s, s);
            unsafe {
                c22c(&mut a);
                c22r(&mut b);
            }
            same("c22", a, b);

            let (mut a, mut b) = (s, s);
            unsafe {
                c23c(&mut a);
                c23r(&mut b);
            }
            same("c23", a, b);

            let (mut a, mut b) = (s, s);
            same("c2GJKSimplexMetric", unsafe { cmc(&mut a) }, unsafe {
                cmr(&mut b)
            });
            let (mut a, mut b) = (s, s);
            same("c2D", unsafe { cdc(&mut a) }, unsafe { cdr(&mut b) });
            let (mut a, mut b) = (s, s);
            same("c2L", unsafe { clc(&mut a) }, unsafe { clr(&mut b) });

            let (mut a, mut b) = (s, s);
            let mut ca = v(0.0, 0.0);
            let mut cb = ca;
            let mut ra = ca;
            let mut rb = ca;
            unsafe {
                cwc(&mut a, &mut ca, &mut cb);
                cwr(&mut b, &mut ra, &mut rb);
            }
            same("c2Witness", (ca, cb, a), (ra, rb, b));
        }
    }
}

// ------------------------------------------------------- c2Support / c2GJK
#[test]
fn nan_order_support() {
    let (cf, rf) = pair::<FnSupport>("c2Support");
    let mut g = Rng::new(0x5004);
    let idx = |g: &mut Rng| p(g.below(NP as u32) as usize);
    for count in [1i32, 2, 4, 8] {
        for _ in 0..10_000 {
            let mut verts = [c2v::default(); 8];
            for x in verts.iter_mut() {
                *x = v(idx(&mut g), idx(&mut g));
            }
            let d = v(idx(&mut g), idx(&mut g));
            same(
                &format!("c2Support count={count}"),
                unsafe { cf(verts.as_ptr(), count, d) },
                unsafe { rf(verts.as_ptr(), count, d) },
            );
        }
    }
}

#[test]
fn nan_order_gjk() {
    let (cf, rf) = gjk_pair();
    let mut g = Rng::new(0x5005);
    let idx = |g: &mut Rng| p(g.below(NP as u32) as usize);
    // Blend NaN-pool floats with ordinary geometry so GJK actually iterates.
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            for ur in [0i32, 1] {
                for k in 0..1500 {
                    let f = |g: &mut Rng| {
                        if g.below(3) == 0 {
                            idx(g)
                        } else {
                            g.coord()
                        }
                    };
                    let mk = |t: C2_TYPE, g: &mut Rng| match t {
                        C2_TYPE_CIRCLE => Blob::of_circle(c2Circle {
                            p: v(f(g), f(g)),
                            r: f(g),
                        }),
                        C2_TYPE_AABB => Blob::of_aabb(c2AABB {
                            min: v(f(g), f(g)),
                            max: v(f(g), f(g)),
                        }),
                        _ => Blob::of_capsule(c2Capsule {
                            a: v(f(g), f(g)),
                            b: v(f(g), f(g)),
                            r: f(g),
                        }),
                    };
                    let a = mk(ta, &mut g);
                    let b = mk(tb, &mut g);
                    // exercise transforms and the cache under NaN too
                    let ax = c2x {
                        p: v(f(&mut g), f(&mut g)),
                        r: c2r {
                            c: f(&mut g),
                            s: f(&mut g),
                        },
                    };
                    let cache = if k % 2 == 0 {
                        Some(c2GJKCache::default())
                    } else {
                        None
                    };
                    let axr = if k % 3 == 0 { Some(&ax) } else { None };
                    let co = call_gjk(cf, &a, ta, axr, &b, tb, None, ur, cache);
                    let ro = call_gjk(rf, &a, ta, axr, &b, tb, None, ur, cache);
                    same(&format!("c2GJK NaN ta={ta} tb={tb} ur={ur}"), co, ro);
                }
            }
        }
    }
}

#[test]
fn nan_order_collided_and_reverse() {
    let (cc, rc) = pair::<FnCollided>("c2Collided");
    let (cr, rr) = pair::<FnReverseCollide>("reverse_collide");
    let mut g = Rng::new(0x5006);
    let idx = |g: &mut Rng| p(g.below(NP as u32) as usize);
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            for _ in 0..3000 {
                let mk = |t: C2_TYPE, g: &mut Rng| match t {
                    C2_TYPE_CIRCLE => Blob::of_circle(c2Circle {
                        p: v(idx(g), idx(g)),
                        r: idx(g),
                    }),
                    C2_TYPE_AABB => Blob::of_aabb(c2AABB {
                        min: v(idx(g), idx(g)),
                        max: v(idx(g), idx(g)),
                    }),
                    _ => Blob::of_capsule(c2Capsule {
                        a: v(idx(g), idx(g)),
                        b: v(idx(g), idx(g)),
                        r: idx(g),
                    }),
                };
                let a = mk(ta, &mut g);
                let b = mk(tb, &mut g);
                same(
                    &format!("c2Collided NaN ta={ta} tb={tb}"),
                    unsafe { cc(a.ptr(), ta, b.ptr(), tb) },
                    unsafe { rc(a.ptr(), ta, b.ptr(), tb) },
                );
            }
        }
    }
    // reverse_collide: exhaustive over the pool
    for i in 0..NP {
        for j in 0..NP {
            for k in 0..NP {
                let (x, y, r) = (p(i), p(j), p(k));
                same(
                    &format!("reverse_collide NaN ({x},{y},{r})"),
                    cr(x, y, r),
                    rr(x, y, r),
                );
            }
        }
    }
}
