//! Diagnostic: replicate the C body of `c2CapsuletoPolyManifold` using each
//! library's *exported* primitives and print every intermediate, to locate
//! which primitive diverges. Not part of the verification matrix.

#![allow(non_snake_case)]

mod common;

use common::*;
use std::ffi::{c_int, c_void};

struct P {
    dot: FnFvv,
    dist: FnFhv,
    sub: FnVvv,
    add: FnVvv,
    mulvs: FnVvf,
    norm: FnVv,
    neg: FnVv,
    ccw: FnVv,
    skew: FnVv,
    mulxv: FnVxv,
    mulxvt: FnVxv,
    support: FnSupport,
    plane_at: FnHpi,
    gjk: FnGJK,
    intersect: FnIntersect,
    bbverts: FnBBVerts,
    norms: FnNorms,
}

macro_rules! load {
    ($lib:expr, $($f:ident : $t:ty = $n:literal),* $(,)?) => {
        P { $($f: unsafe { *$lib.get::<$t>(concat!($n, "\0").as_bytes()).unwrap() }),* }
    };
}

fn prims(lib: &'static libloading::Library) -> P {
    load!(lib,
        dot: FnFvv = "c2Dot",
        dist: FnFhv = "c2Dist",
        sub: FnVvv = "c2Sub",
        add: FnVvv = "c2Add",
        mulvs: FnVvf = "c2Mulvs",
        norm: FnVv = "c2Norm",
        neg: FnVv = "c2Neg",
        ccw: FnVv = "c2CCW90",
        skew: FnVv = "c2Skew",
        mulxv: FnVxv = "c2Mulxv",
        mulxvt: FnVxv = "c2MulxvT",
        support: FnSupport = "c2Support",
        plane_at: FnHpi = "c2PlaneAt",
        gjk: FnGJK = "c2GJK",
        intersect: FnIntersect = "c2Intersect",
        bbverts: FnBBVerts = "c2BBVerts",
        norms: FnNorms = "c2Norms",
    )
}

/// Replica of the static `c2Clip`.
unsafe fn clip(p: &P, seg: &mut [c2v; 2], h: c2h) -> (i32, String) {
    let mut out = [c2v::default(); 3];
    let mut sp = 0usize;
    let d0 = unsafe { (p.dist)(h, seg[0]) };
    if d0 < 0.0 {
        out[sp] = seg[0];
        sp += 1;
    }
    let d1 = unsafe { (p.dist)(h, seg[1]) };
    if d1 < 0.0 {
        out[sp] = seg[1];
        sp += 1;
    }
    if d0 == 0.0 && d1 == 0.0 {
        out[sp] = seg[0];
        sp += 1;
        out[sp] = seg[1];
        sp += 1;
    } else if d0 * d1 <= 0.0 {
        out[sp] = unsafe { (p.intersect)(seg[0], seg[1], d0, d1) };
        sp += 1;
    }
    let log = format!("d0={} d1={} d0*d1={} sp={sp}", fs(d0), fs(d1), fs(d0 * d1));
    seg[0] = out[0];
    seg[1] = out[1];
    (sp as i32, log)
}

/// Replica of the static `c2SidePlanes`.
unsafe fn side_planes(
    p: &P,
    seg: &mut [c2v; 2],
    ra: c2v,
    rb: c2v,
    h: &mut c2h,
    log: &mut String,
) -> i32 {
    unsafe {
        let inn = (p.norm)((p.sub)(rb, ra));
        let left = c2h {
            n: (p.neg)(inn),
            d: (p.dot)((p.neg)(inn), ra),
        };
        let right = c2h {
            n: inn,
            d: (p.dot)(inn, rb),
        };
        *log += &format!("\n    in={} left.d={} right.d={}", vs(inn), fs(left.d), fs(right.d));
        let (c0, l0) = clip(p, seg, left);
        *log += &format!("\n    clip(left): {l0} -> seg={} {}", vs(seg[0]), vs(seg[1]));
        if c0 < 2 {
            return 0;
        }
        let (c1, l1) = clip(p, seg, right);
        *log += &format!("\n    clip(right): {l1} -> seg={} {}", vs(seg[0]), vs(seg[1]));
        if c1 < 2 {
            return 0;
        }
        h.n = (p.ccw)(inn);
        h.d = (p.dot)((p.ccw)(inn), ra);
        1
    }
}

unsafe fn keep_deep(p: &P, seg: &[c2v; 2], h: c2h, m: &mut c2Manifold, log: &mut String) {
    let mut cp = 0usize;
    for i in 0..2 {
        let pt = seg[i];
        let d = unsafe { (p.dist)(h, pt) };
        *log += &format!("\n    keepdeep[{i}] p={} d={}", vs(pt), fs(d));
        if d <= 0.0 {
            m.contact_points[cp] = pt;
            m.depths[cp] = -d;
            cp += 1;
        }
    }
    m.count = cp as c_int;
    m.n = h.n;
}

unsafe fn incident(p: &P, poly: &c2Poly, ix: c2x, rn: c2v, out: &mut [c2v; 2], log: &mut String) {
    unsafe {
        let mut index: c_int = !0;
        let mut min_dot = f32::MAX;
        for i in 0..poly.count {
            let dot = (p.dot)(rn, poly.norms[i as usize]);
            if dot < min_dot {
                min_dot = dot;
                index = i;
            }
        }
        *log += &format!("\n    incident index={index} min_dot={}", fs(min_dot));
        let vi = if index < 0 {
            // `verts[-1]` in C reads the 8 bytes before the array.
            let base = (&raw const poly.verts).cast::<c2v>();
            *base.offset(index as isize)
        } else {
            poly.verts[index as usize]
        };
        out[0] = (p.mulxv)(ix, vi);
        let nxt = if index + 1 == poly.count { 0 } else { index + 1 };
        let vn = if nxt < 0 {
            let base = (&raw const poly.verts).cast::<c2v>();
            *base.offset(nxt as isize)
        } else {
            poly.verts[nxt as usize]
        };
        out[1] = (p.mulxv)(ix, vn);
    }
}

/// Replica of `c2CapsuletoPolyManifold`, fully instrumented.
unsafe fn capsule_to_poly(p: &P, A: c2Capsule, poly: &c2Poly) -> (c2Manifold, String) {
    unsafe {
        let mut m = seeded_manifold(-4242.0);
        m.count = 0;
        let mut log = String::new();
        let mut a = c2v::default();
        let mut b = c2v::default();
        scrub_stack();
        let d = (p.gjk)(
            (&raw const A).cast::<c_void>(),
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            (poly as *const c2Poly).cast::<c_void>(),
            C2_TYPE_POLY,
            std::ptr::null(),
            &mut a,
            &mut b,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        log += &format!("gjk d={} a={} b={}", fs(d), vs(a), vs(b));
        if d < 1.0e-6f32 {
            let bx = c2x {
                p: v(0.0, 0.0),
                r: c2r { c: 1.0, s: 0.0 },
            };
            let aa = (p.mulxvt)(bx, A.a);
            let bb = (p.mulxvt)(bx, A.b);
            let ab = (p.norm)((p.sub)(aa, bb));
            log += &format!("\n  A_in_B a={} b={} ab={}", vs(aa), vs(bb), vs(ab));

            let mut ab_h0 = c2h::default();
            ab_h0.n = (p.ccw)(ab);
            ab_h0.d = (p.dot)(aa, ab_h0.n);
            let v0 = (p.support)(poly.verts.as_ptr(), poly.count, (p.neg)(ab_h0.n));
            let s0 = (p.dist)(ab_h0, poly.verts[v0.clamp(0, 7) as usize]);

            let mut ab_h1 = c2h::default();
            ab_h1.n = (p.skew)(ab);
            ab_h1.d = (p.dot)(aa, ab_h1.n);
            let v1 = (p.support)(poly.verts.as_ptr(), poly.count, (p.neg)(ab_h1.n));
            let s1 = (p.dist)(ab_h1, poly.verts[v1.clamp(0, 7) as usize]);
            log += &format!(
                "\n  ab_h0 n={} d={} v0={v0} s0={}\n  ab_h1 n={} d={} v1={v1} s1={}",
                vs(ab_h0.n),
                fs(ab_h0.d),
                fs(s0),
                vs(ab_h1.n),
                fs(ab_h1.d),
                fs(s1)
            );

            let mut index: c_int = !0;
            let mut sep = -f32::MAX;
            let mut code: c_int = 0;
            for i in 0..poly.count {
                let h = (p.plane_at)(poly, i);
                let da = (p.dot)(aa, (p.neg)(h.n));
                let db = (p.dot)(bb, (p.neg)(h.n));
                let dd = if da > db {
                    (p.dist)(h, aa)
                } else {
                    (p.dist)(h, bb)
                };
                log += &format!(
                    "\n  plane[{i}] n={} d={} da={} db={} dd={}",
                    vs(h.n),
                    fs(h.d),
                    fs(da),
                    fs(db),
                    fs(dd)
                );
                if dd > sep {
                    sep = dd;
                    index = i;
                }
            }
            if s0 > sep {
                sep = s0;
                index = v0;
                code = 1;
            }
            if s1 > sep {
                sep = s1;
                index = v1;
                code = 2;
            }
            log += &format!("\n  => sep={} index={index} code={code}", fs(sep));

            match code {
                0 => {
                    let mut seg = [A.a, A.b];
                    let mut h = c2h::default();
                    log += "\n  code0:";
                    let ra = (p.mulxv)(bx, poly.verts[index.clamp(0, 7) as usize]);
                    let nxt = if index + 1 == poly.count { 0 } else { index + 1 };
                    let rb = (p.mulxv)(bx, poly.verts[nxt.clamp(0, 7) as usize]);
                    if side_planes(p, &mut seg, ra, rb, &mut h, &mut log) == 0 {
                        log += "\n  side_planes -> 0, early return";
                        return (m, log);
                    }
                    keep_deep(p, &seg, h, &mut m, &mut log);
                    m.n = (p.neg)(m.n);
                }
                1 => {
                    let mut inc = [c2v::default(); 2];
                    log += "\n  code1:";
                    incident(p, poly, bx, ab_h0.n, &mut inc, &mut log);
                    let mut h = c2h::default();
                    if side_planes(p, &mut inc, bb, aa, &mut h, &mut log) == 0 {
                        log += "\n  side_planes -> 0, early return";
                        return (m, log);
                    }
                    keep_deep(p, &inc, h, &mut m, &mut log);
                }
                2 => {
                    let mut inc = [c2v::default(); 2];
                    log += "\n  code2:";
                    incident(p, poly, bx, ab_h1.n, &mut inc, &mut log);
                    let mut h = c2h::default();
                    if side_planes(p, &mut inc, aa, bb, &mut h, &mut log) == 0 {
                        log += "\n  side_planes -> 0, early return";
                        return (m, log);
                    }
                    keep_deep(p, &inc, h, &mut m, &mut log);
                }
                _ => return (m, log),
            }
            for i in 0..m.count {
                m.depths[i as usize] += A.r;
            }
        } else if d < A.r {
            m.count = 1;
            m.n = (p.norm)((p.sub)(b, a));
            m.contact_points[0] = (p.add)(a, (p.mulvs)(m.n, A.r));
            m.depths[0] = A.r - d;
            log += "\n  shallow branch";
        }
        (m, log)
    }
}

#[test]
#[ignore = "diagnostic only"]
fn bisect() {
    let l = libs();
    let cp = prims(&l.c);
    let rp = prims(&l.rust);

    // The two failing inputs from phase_b_collide (CAPSULE x AABB).
    let cases: [([f32; 5], [f32; 5]); 2] = [
        (
            [1.1754942e-38, 9.367953e16, 1.1754944e-38, -485.76437, -1.7242005e36],
            [725.4435, 4.926178e-25, f32::INFINITY, f32::INFINITY, 1.0],
        ),
        (
            [934.9363, 881.8736, -1.881654e-14, 1.1754944e-38, 1.3140647e22],
            [-5.7208593e-29, -4.5576037e-35, f32::NEG_INFINITY, -3.234572e-29, -1.7463647e-13],
        ),
    ];

    for (n, (pa, pb)) in cases.iter().enumerate() {
        let cap = c2Capsule {
            a: v(pa[0], pa[1]),
            b: v(pa[2], pa[3]),
            r: pa[4],
        };
        let mut bb = c2AABB {
            min: v(pb[0], pb[1]),
            max: v(pb[2], pb[3]),
        };
        // c2AABBtoCapsuleManifold builds the poly this way.
        let mut poly = c2Poly::default();
        unsafe {
            (cp.bbverts)(poly.verts.as_mut_ptr(), &mut bb);
            poly.count = 4;
            (cp.norms)(poly.verts.as_mut_ptr(), poly.norms.as_mut_ptr(), 4);
        }
        println!("\n########## case {n}");
        println!("capsule={cap:?}");
        println!("poly verts={:?}", &poly.verts[..4]);
        println!("poly norms={:?}", &poly.norms[..4]);
        let (cm, clog) = unsafe { capsule_to_poly(&cp, cap, &poly) };
        let (rm, rlog) = unsafe { capsule_to_poly(&rp, cap, &poly) };
        println!("--- C  primitives ---\n{clog}\n  => {}", ms(&cm));
        println!("--- Rust primitives ---\n{rlog}\n  => {}", ms(&rm));

        // And the real exported functions.
        let (c_f, r_f) = l.pair::<FnCapsulePoly>("c2CapsuletoPolyManifold");
        let mut cm2 = seeded_manifold(-4242.0);
        let mut rm2 = cm2;
        unsafe {
            scrub_stack();
            c_f(cap, &poly, std::ptr::null(), &mut cm2);
            scrub_stack();
            r_f(cap, &poly, std::ptr::null(), &mut rm2);
        }
        println!("real C   => {}", ms(&cm2));
        println!("real Rust=> {}", ms(&rm2));
    }
}
