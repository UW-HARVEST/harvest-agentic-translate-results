//! Broad randomized differential fuzzing over the composed pipeline.
//!
//! The per-row tests in `phase_b_*` pin down the configurations enumerated in
//! `CONFIGS.md`.  This file is the safety net for everything in between: shape
//! fields are drawn from *arbitrary float bit patterns* (so NaNs, infinities,
//! denormals and huge magnitudes all appear), every option is randomized
//! independently, and the whole `c2GJK` / `c2Collided` pipeline is compared
//! bit-for-bit including the cache image carried across successive calls.
//!
//! Only VALID `C2_TYPE` values are used, because an out-of-range type makes the
//! C read an uninitialised `c2Proxy` (see `ERRORS.md` rows 6-7).

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_int, c_void};

/// A float built from arbitrary bits, biased so that ordinary magnitudes and
/// the special values both appear often.
fn any_float(r: &Rng) -> f32 {
    match r.below(12) {
        0 => f32::from_bits(r.next_u32()),          // truly arbitrary bits
        1 => f32::NAN,
        2 => -f32::NAN,
        3 => f32::from_bits(0x7f80_0001 | (r.next_u32() & 0x003f_ffff)), // signalling NaNs
        4 => f32::INFINITY,
        5 => f32::NEG_INFINITY,
        6 => 0.0,
        7 => -0.0,
        8 => f32::from_bits(r.next_u32() & 0x007f_ffff), // denormals
        9 => (r.below(65) as i32 - 32) as f32,
        _ => r.sym(200.0),
    }
}

fn any_v(r: &Rng) -> c2v {
    c2v { x: any_float(r), y: any_float(r) }
}

/// A 32-byte scratch buffer holding a shape of the requested type, so that no
/// read the C performs can leave the buffer even if we mis-sized a struct.
struct Blob {
    buf: [u8; 32],
    ty: c_int,
    desc: String,
}

impl Blob {
    fn ptr(&self) -> *const c_void {
        self.buf.as_ptr() as *const c_void
    }
}

fn any_shape(r: &Rng, ty: c_int) -> Blob {
    let mut buf = [0u8; 32];
    let desc;
    match ty {
        C2_TYPE_CIRCLE => {
            let c = c2Circle { p: any_v(r), r: any_float(r) };
            buf[..12].copy_from_slice(bytes_of(&c));
            desc = format!("{c:?}");
        }
        C2_TYPE_AABB => {
            let b = c2AABB { min: any_v(r), max: any_v(r) };
            buf[..16].copy_from_slice(bytes_of(&b));
            desc = format!("{b:?}");
        }
        _ => {
            let c = c2Capsule { a: any_v(r), b: any_v(r), r: any_float(r) };
            buf[..20].copy_from_slice(bytes_of(&c));
            desc = format!("{c:?}");
        }
    }
    Blob { buf, ty, desc }
}

fn any_xform(r: &Rng) -> Option<c2x> {
    match r.below(5) {
        0 => None,
        1 => Some(c2x { p: c2v { x: 0.0, y: 0.0 }, r: c2r { c: 1.0, s: 0.0 } }),
        2 => Some(c2x { p: r.geo_v(), r: r.rot() }),
        3 => Some(c2x { p: any_v(r), r: c2r { c: any_float(r), s: any_float(r) } }),
        _ => Some(c2x { p: r.geo_v(), r: c2r { c: r.sym(3.0), s: r.sym(3.0) } }),
    }
}

const TYPES: [c_int; 3] = [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE];

#[test]
fn fuzz_c2GJK_full_option_space() {
    let a = api();
    let mut d = Diff::new("fuzz: c2GJK over arbitrary float bit patterns x the full option space");
    let r = Rng::new(0xF0FF_1E5E);
    let mut nan_dist = 0usize;
    let mut zero_dist = 0usize;
    let mut pos_dist = 0usize;

    for iter in 0..120_000 {
        let ta = TYPES[(r.below(3)) as usize];
        let tb = TYPES[(r.below(3)) as usize];
        let A = any_shape(&r, ta);
        let B = any_shape(&r, tb);
        let xa = any_xform(&r);
        let xb = any_xform(&r);
        let ap = xa.as_ref().map(|v| v as *const c2x).unwrap_or(std::ptr::null());
        let bp = xb.as_ref().map(|v| v as *const c2x).unwrap_or(std::ptr::null());
        let use_radius = match r.below(4) {
            0 => 0,
            1 => 1,
            2 => -1,
            _ => r.next_u32() as c_int,
        };
        let want_cache = r.below(3) != 0;

        // Vertex-count bound for a legal warm cache.
        let vc = |t: c_int| match t {
            C2_TYPE_CIRCLE => 1u32,
            C2_TYPE_AABB => 4,
            _ => 2,
        };
        let mut cache = if want_cache {
            match r.below(3) {
                0 => c2GJKCache::default(),
                _ => c2GJKCache {
                    metric: any_float(&r),
                    count: (r.below(4)) as c_int, // 0..3 (>=4 crashes the C: ERRORS row 60)
                    iA: [
                        r.below(vc(ta)) as c_int,
                        r.below(vc(ta)) as c_int,
                        r.below(vc(ta)) as c_int,
                    ],
                    iB: [
                        r.below(vc(tb)) as c_int,
                        r.below(vc(tb)) as c_int,
                        r.below(vc(tb)) as c_int,
                    ],
                    div: match r.below(3) {
                        0 => 0.0,
                        1 => 1.0,
                        _ => any_float(&r),
                    },
                },
            }
        } else {
            c2GJKCache::default()
        };

        // Up to three chained calls, so a cache written by call N feeds call N+1
        // (exactly how a real consumer uses it).
        let rounds = 1 + r.below(3);
        let mut cc = cache;
        let mut rc = cache;
        for round in 0..rounds {
            let sentinel = c2v { x: -8.5, y: 7.25 };
            let (mut coa, mut cob) = (sentinel, sentinel);
            let (mut roa, mut rob) = (sentinel, sentinel);
            let (mut ci, mut ri) = (-77 as c_int, -77 as c_int);
            let want_a = r.below(8) != 0;
            let want_b = r.below(8) != 0;
            let want_i = r.below(8) != 0;

            let cd = unsafe {
                (a.c2GJK.0)(
                    A.ptr(), A.ty, ap, B.ptr(), B.ty, bp,
                    if want_a { &mut coa } else { std::ptr::null_mut() },
                    if want_b { &mut cob } else { std::ptr::null_mut() },
                    use_radius,
                    if want_i { &mut ci } else { std::ptr::null_mut() },
                    if want_cache { &mut cc } else { std::ptr::null_mut() },
                )
            };
            let rd = unsafe {
                (a.c2GJK.1)(
                    A.ptr(), A.ty, ap, B.ptr(), B.ty, bp,
                    if want_a { &mut roa } else { std::ptr::null_mut() },
                    if want_b { &mut rob } else { std::ptr::null_mut() },
                    use_radius,
                    if want_i { &mut ri } else { std::ptr::null_mut() },
                    if want_cache { &mut rc } else { std::ptr::null_mut() },
                )
            };

            let ctx = || {
                format!(
                    "iter={iter} round={round}\n  A({}) = {}\n  B({}) = {}\n  xa={xa:?} xb={xb:?}\n  \
                     use_radius={use_radius} cache_on={want_cache} outA={want_a} outB={want_b} iters={want_i}\n  \
                     dist  C={} R={}\n  outA  C={} R={}\n  outB  C={} R={}\n  iter  C={ci} R={ri}\n  \
                     cache C={cc:?}\n  cache R={rc:?}\n  start cache={cache:?}",
                    A.ty, A.desc, B.ty, B.desc,
                    fmt_f(cd), fmt_f(rd), fmt_v(coa), fmt_v(roa), fmt_v(cob), fmt_v(rob)
                )
            };
            d.check(feq(cd, rd), ctx);
            d.check(veq(coa, roa) && veq(cob, rob), ctx);
            d.check(ci == ri, ctx);
            d.check(cacheeq(&cc, &rc), ctx);

            if cd.is_nan() {
                nan_dist += 1
            } else if cd == 0.0 {
                zero_dist += 1
            } else {
                pos_dist += 1
            }
            cache = cc;
        }
    }
    eprintln!("fuzz c2GJK: nan_dist={nan_dist} zero_dist={zero_dist} pos_dist={pos_dist}");
    assert!(nan_dist > 0 && zero_dist > 0 && pos_dist > 0, "fuzz corpus too narrow");
    d.finish();
}

#[test]
fn fuzz_c2Collided_and_predicates() {
    let a = api();
    let mut d = Diff::new("fuzz: c2Collided + all six predicates over arbitrary float bit patterns");
    let r = Rng::new(0xC0111DED);
    let mut yes = 0usize;
    let mut no = 0usize;
    for _ in 0..150_000 {
        let ta = TYPES[(r.below(3)) as usize];
        let tb = TYPES[(r.below(3)) as usize];
        let A = any_shape(&r, ta);
        let B = any_shape(&r, tb);
        let (c, s) = unsafe {
            ((a.c2Collided.0)(A.ptr(), ta, B.ptr(), tb), (a.c2Collided.1)(A.ptr(), ta, B.ptr(), tb))
        };
        d.check(c == s, || format!("c2Collided({ta}, {tb}) A={} B={} C={c} R={s}", A.desc, B.desc));
        if c != 0 { yes += 1 } else { no += 1 }

        // and the same inputs through the direct predicates
        let circle = c2Circle { p: any_v(&r), r: any_float(&r) };
        let circle2 = c2Circle { p: any_v(&r), r: any_float(&r) };
        let bb = c2AABB { min: any_v(&r), max: any_v(&r) };
        let bb2 = c2AABB { min: any_v(&r), max: any_v(&r) };
        let cap = c2Capsule { a: any_v(&r), b: any_v(&r), r: any_float(&r) };
        let cap2 = c2Capsule { a: any_v(&r), b: any_v(&r), r: any_float(&r) };

        d.check((a.c2CircletoCircle.0)(circle, circle2) == (a.c2CircletoCircle.1)(circle, circle2),
                || format!("c2CircletoCircle({circle:?}, {circle2:?})"));
        d.check((a.c2CircletoAABB.0)(circle, bb) == (a.c2CircletoAABB.1)(circle, bb),
                || format!("c2CircletoAABB({circle:?}, {bb:?})"));
        d.check((a.c2CircletoCapsule.0)(circle, cap) == (a.c2CircletoCapsule.1)(circle, cap),
                || format!("c2CircletoCapsule({circle:?}, {cap:?})"));
        d.check((a.c2AABBtoAABB.0)(bb, bb2) == (a.c2AABBtoAABB.1)(bb, bb2),
                || format!("c2AABBtoAABB({bb:?}, {bb2:?})"));
        d.check((a.c2AABBtoCapsule.0)(bb, cap) == (a.c2AABBtoCapsule.1)(bb, cap),
                || format!("c2AABBtoCapsule({bb:?}, {cap:?})"));
        d.check((a.c2CapsuletoCapsule.0)(cap, cap2) == (a.c2CapsuletoCapsule.1)(cap, cap2),
                || format!("c2CapsuletoCapsule({cap:?}, {cap2:?})"));
    }
    eprintln!("fuzz c2Collided: collide={yes} miss={no}");
    assert!(yes > 0 && no > 0, "fuzz corpus too narrow: yes={yes} no={no}");
    d.finish();
}

#[test]
fn fuzz_leaf_helpers() {
    let a = api();
    let mut d = Diff::new("fuzz: every leaf helper over arbitrary float bit patterns");
    let r = Rng::new(0x1EAF);
    for _ in 0..200_000 {
        let u = any_v(&r);
        let v = any_v(&r);
        let f = any_float(&r);
        let rot = c2r { c: any_float(&r), s: any_float(&r) };
        let xf = c2x { p: any_v(&r), r: rot };

        d.check(veq((a.c2V.0)(u.x, u.y), (a.c2V.1)(u.x, u.y)), || format!("c2V {}", fmt_v(u)));
        d.check(veq((a.c2Mulvs.0)(u, f), (a.c2Mulvs.1)(u, f)), || format!("c2Mulvs {} {}", fmt_v(u), fmt_f(f)));
        d.check(veq((a.c2Maxv.0)(u, v), (a.c2Maxv.1)(u, v)), || format!("c2Maxv {} {}", fmt_v(u), fmt_v(v)));
        d.check(veq((a.c2Minv.0)(u, v), (a.c2Minv.1)(u, v)), || format!("c2Minv {} {}", fmt_v(u), fmt_v(v)));
        d.check(veq((a.c2Sub.0)(u, v), (a.c2Sub.1)(u, v)), || format!("c2Sub {} {}", fmt_v(u), fmt_v(v)));
        d.check(veq((a.c2Add.0)(u, v), (a.c2Add.1)(u, v)), || format!("c2Add {} {}", fmt_v(u), fmt_v(v)));
        d.check(feq((a.c2Dot.0)(u, v), (a.c2Dot.1)(u, v)), || format!("c2Dot {} {}", fmt_v(u), fmt_v(v)));
        d.check(feq((a.c2Det2.0)(u, v), (a.c2Det2.1)(u, v)), || format!("c2Det2 {} {}", fmt_v(u), fmt_v(v)));
        d.check(feq((a.c2Len.0)(u), (a.c2Len.1)(u)), || format!("c2Len {}", fmt_v(u)));
        d.check(veq((a.c2Neg.0)(u), (a.c2Neg.1)(u)), || format!("c2Neg {}", fmt_v(u)));
        d.check(veq((a.c2Skew.0)(u), (a.c2Skew.1)(u)), || format!("c2Skew {}", fmt_v(u)));
        d.check(veq((a.c2CCW90.0)(u), (a.c2CCW90.1)(u)), || format!("c2CCW90 {}", fmt_v(u)));
        d.check(veq((a.c2Div.0)(u, f), (a.c2Div.1)(u, f)), || format!("c2Div {} {}", fmt_v(u), fmt_f(f)));
        d.check(veq((a.c2Norm.0)(u), (a.c2Norm.1)(u)), || format!("c2Norm {}", fmt_v(u)));
        d.check(veq((a.c2Mulrv.0)(rot, u), (a.c2Mulrv.1)(rot, u)), || format!("c2Mulrv {rot:?} {}", fmt_v(u)));
        d.check(veq((a.c2MulrvT.0)(rot, u), (a.c2MulrvT.1)(rot, u)), || format!("c2MulrvT {rot:?} {}", fmt_v(u)));
        d.check(veq((a.c2Mulxv.0)(xf, u), (a.c2Mulxv.1)(xf, u)), || format!("c2Mulxv {xf:?} {}", fmt_v(u)));

        let lo = any_v(&r);
        let hi = any_v(&r);
        d.check(veq((a.c2Clampv.0)(u, lo, hi), (a.c2Clampv.1)(u, lo, hi)),
                || format!("c2Clampv {} {} {}", fmt_v(u), fmt_v(lo), fmt_v(hi)));

        // aabb() with arbitrary bits
        let (p, q, s2, t) = (any_float(&r), any_float(&r), any_float(&r), any_float(&r));
        d.check((a.aabb.0)(p, q, s2, t) == (a.aabb.1)(p, q, s2, t),
                || format!("aabb({}, {}, {}, {})", fmt_f(p), fmt_f(q), fmt_f(s2), fmt_f(t)));
    }
    d.finish();
}

#[test]
fn fuzz_simplex_primitives() {
    let a = api();
    let mut d = Diff::new("fuzz: c22 / c23 / c2D / c2L / c2Witness / c2GJKSimplexMetric / c2Support over arbitrary bits");
    let r = Rng::new(0x5117);
    for _ in 0..80_000 {
        let mut s = c2Simplex::default();
        for i in 0..4 {
            s.verts[i].sA = any_v(&r);
            s.verts[i].sB = any_v(&r);
            s.verts[i].p = any_v(&r);
            s.verts[i].u = any_float(&r);
            s.verts[i].iA = r.next_u32() as c_int;
            s.verts[i].iB = r.next_u32() as c_int;
        }
        s.div = any_float(&r);
        s.count = match r.below(8) {
            0 => 0,
            1 => 1,
            2 => 2,
            3 => 3,
            4 => 4,
            5 => -1,
            6 => r.next_u32() as c_int,
            _ => (r.below(4)) as c_int,
        };

        // c2GJKSimplexMetric (read-only)
        let (mut cs, mut rs) = (s, s);
        let (cv, rv) = unsafe { ((a.c2GJKSimplexMetric.0)(&mut cs), (a.c2GJKSimplexMetric.1)(&mut rs)) };
        d.check(feq(cv, rv) && bytes_of(&cs) == bytes_of(&rs), || format!("c2GJKSimplexMetric {s:?} C={} R={}", fmt_f(cv), fmt_f(rv)));

        // c2D (read-only)
        let (mut cs, mut rs) = (s, s);
        let (cv, rv) = unsafe { ((a.c2D.0)(&mut cs), (a.c2D.1)(&mut rs)) };
        d.check(veq(cv, rv) && bytes_of(&cs) == bytes_of(&rs), || format!("c2D {s:?} C={} R={}", fmt_v(cv), fmt_v(rv)));

        // c2L (read-only)
        let (mut cs, mut rs) = (s, s);
        let (cv, rv) = unsafe { ((a.c2L.0)(&mut cs), (a.c2L.1)(&mut rs)) };
        d.check(veq(cv, rv) && bytes_of(&cs) == bytes_of(&rs), || format!("c2L {s:?} C={} R={}", fmt_v(cv), fmt_v(rv)));

        // c2Witness (writes two out-params)
        let (mut cs, mut rs) = (s, s);
        let (mut ca, mut cb) = (c2v { x: 5.0, y: 6.0 }, c2v { x: 7.0, y: 8.0 });
        let (mut ra, mut rb) = (ca, cb);
        unsafe {
            (a.c2Witness.0)(&mut cs, &mut ca, &mut cb);
            (a.c2Witness.1)(&mut rs, &mut ra, &mut rb);
        }
        d.check(veq(ca, ra) && veq(cb, rb) && bytes_of(&cs) == bytes_of(&rs),
                || format!("c2Witness {s:?}\n C=({},{})\n R=({},{})", fmt_v(ca), fmt_v(cb), fmt_v(ra), fmt_v(rb)));

        // c22 / c23 (mutating) — only for the counts the C actually dispatches on
        let (mut cs, mut rs) = (s, s);
        unsafe {
            (a.c22.0)(&mut cs);
            (a.c22.1)(&mut rs);
        }
        d.check(bytes_of(&cs) == bytes_of(&rs), || format!("c22 {s:?}\n C={cs:?}\n R={rs:?}"));

        let (mut cs, mut rs) = (s, s);
        unsafe {
            (a.c23.0)(&mut cs);
            (a.c23.1)(&mut rs);
        }
        d.check(bytes_of(&cs) == bytes_of(&rs), || format!("c23 {s:?}\n C={cs:?}\n R={rs:?}"));

        // c2Support with all in-range counts
        let mut verts = [c2v::default(); 8];
        for v in verts.iter_mut() {
            *v = any_v(&r);
        }
        let dir = any_v(&r);
        for count in [0i32, 1, 2, 3, 4, 5, 6, 7, 8, -1] {
            let (cv, rv) = unsafe {
                ((a.c2Support.0)(verts.as_ptr(), count, dir), (a.c2Support.1)(verts.as_ptr(), count, dir))
            };
            d.check(cv == rv, || format!("c2Support(count={count}, d={}) verts={verts:?} C={cv} R={rv}", fmt_v(dir)));
        }

        // c2BBVerts + c2MakeProxy with arbitrary bits
        let mut bb = c2AABB { min: any_v(&r), max: any_v(&r) };
        let mut co = [c2v { x: 1.0, y: 2.0 }; 4];
        let mut ro = co;
        unsafe {
            (a.c2BBVerts.0)(co.as_mut_ptr(), &mut bb);
            (a.c2BBVerts.1)(ro.as_mut_ptr(), &mut bb);
        }
        d.check(bytes_of(&co) == bytes_of(&ro), || format!("c2BBVerts {bb:?}"));

        for ty in TYPES {
            let blob = any_shape(&r, ty);
            let seed = c2Proxy { radius: 3.5, count: -9, verts: [c2v { x: 1.25, y: -2.5 }; 8] };
            let (mut cp, mut rp) = (seed, seed);
            unsafe {
                (a.c2MakeProxy.0)(blob.ptr(), ty, &mut cp);
                (a.c2MakeProxy.1)(blob.ptr(), ty, &mut rp);
            }
            d.check(bytes_of(&cp) == bytes_of(&rp), || format!("c2MakeProxy(ty={ty}) {}", blob.desc));
        }
    }
    d.finish();
}
