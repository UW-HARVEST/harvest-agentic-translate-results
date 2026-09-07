//! Phase B — CONFIGS.md rows 15..38: `c2MakeProxy`, `c2Support` and the
//! low-level simplex routines (`c22`, `c23`, `c2D`, `c2L`, `c2Witness`,
//! `c2GJKSimplexMetric`).  These are the entry points the convenience wrappers
//! never reach directly, so they are driven here by hand.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_int, c_void};

const N: usize = 4000;

/// Calls `c2MakeProxy` on both libs with the same pre-dirtied output buffer and
/// byte-compares the whole 72-byte `c2Proxy`.
fn proxy_case(shape: *const c_void, ty: c_int, dirty: u8, ctx: &dyn std::fmt::Debug) {
    let p = apis();
    let mut pc: c2Proxy = unsafe { std::mem::zeroed() };
    unsafe {
        std::ptr::write_bytes(&mut pc as *mut c2Proxy as *mut u8, dirty, 1);
    }
    let mut pr = pc;
    unsafe {
        (p.c.c2MakeProxy)(shape, ty, &mut pc);
        (p.r.c2MakeProxy)(shape, ty, &mut pr);
    }
    eq_bytes("c2MakeProxy", ctx, &pc, &pr);
}

// ----------------------------------------------------------------- rows 15-18
#[test]
fn row15_makeproxy_circle() {
    let mut rng = Rng::new(0x1500);
    for _ in 0..n_cases(N) {
        let c = rng.circle();
        for &d in &[0x00u8, 0xAA, 0xFF] {
            proxy_case(&c as *const c2Circle as *const c_void, C2_TYPE_CIRCLE, d, &c);
        }
    }
    for &x in SPECIAL_F32 {
        for &r in SPECIAL_F32 {
            let c = c2Circle {
                p: c2v { x, y: -x },
                r,
            };
            proxy_case(&c as *const c2Circle as *const c_void, C2_TYPE_CIRCLE, 0x5A, &c);
        }
    }
}

#[test]
fn row16_makeproxy_aabb() {
    let mut rng = Rng::new(0x1600);
    for _ in 0..n_cases(N) {
        let bb = rng.aabb();
        for &d in &[0x00u8, 0xAA, 0xFF] {
            proxy_case(&bb as *const c2AABB as *const c_void, C2_TYPE_AABB, d, &bb);
        }
    }
    for &x in SPECIAL_F32 {
        for &y in SPECIAL_F32 {
            // includes inverted and degenerate boxes
            let bb = c2AABB {
                min: c2v { x, y },
                max: c2v { x: y, y: x },
            };
            proxy_case(&bb as *const c2AABB as *const c_void, C2_TYPE_AABB, 0x5A, &bb);
        }
    }
}

#[test]
fn row17_makeproxy_capsule() {
    let mut rng = Rng::new(0x1700);
    for _ in 0..n_cases(N) {
        let c = rng.capsule();
        for &d in &[0x00u8, 0xAA, 0xFF] {
            proxy_case(
                &c as *const c2Capsule as *const c_void,
                C2_TYPE_CAPSULE,
                d,
                &c,
            );
        }
    }
    for &x in SPECIAL_F32 {
        for &r in SPECIAL_F32 {
            let c = c2Capsule {
                a: c2v { x, y: r },
                b: c2v { x: r, y: x },
                r,
            };
            proxy_case(
                &c as *const c2Capsule as *const c_void,
                C2_TYPE_CAPSULE,
                0x5A,
                &c,
            );
        }
    }
}

#[test]
fn row18_makeproxy_preserves_untouched_tail() {
    // A CIRCLE proxy writes radius, count and verts[0] only: verts[1..8] must
    // keep the caller's dirty bytes, identically in both libraries.
    let mut rng = Rng::new(0x1800);
    for _ in 0..n_cases(500) {
        let c = rng.circle();
        for d in 0..=255u8 {
            if d % 37 != 0 {
                continue;
            }
            proxy_case(&c as *const c2Circle as *const c_void, C2_TYPE_CIRCLE, d, &(c, d));
        }
    }
}

// ----------------------------------------------------------------- rows 19-22
fn support_case(verts: &[c2v], count: c_int, d: c2v, ctx: &dyn std::fmt::Debug) {
    let p = apis();
    unsafe {
        let cv = (p.c.c2Support)(verts.as_ptr(), count, d);
        let rv = (p.r.c2Support)(verts.as_ptr(), count, d);
        eq_int("c2Support", ctx, cv, rv);
    }
}

#[test]
fn row19_row22_c2Support_counts() {
    let mut rng = Rng::new(0x1900);
    for &count in &[1i32, 2, 4, 8] {
        for _ in 0..n_cases(N) {
            let mut verts = [c2v::default(); 8];
            for v in verts.iter_mut() {
                *v = rng.v();
            }
            let d = rng.any_v();
            support_case(&verts, count, d, &(verts, count, d));
        }
    }
    // exact ties: identical vertices -> the FIRST index must win
    let verts = [c2v { x: 1.0, y: 1.0 }; 8];
    for &count in &[1i32, 2, 4, 8] {
        for _ in 0..64 {
            let d = rng.v();
            support_case(&verts, count, d, &(count, d, "tie"));
        }
    }
    // 8 axis/diagonal directions against an AABB-shaped 4-vertex proxy
    let bb = c2AABB {
        min: c2v { x: -3.0, y: -5.0 },
        max: c2v { x: 7.0, y: 2.5 },
    };
    let quad = [
        bb.min,
        c2v { x: bb.max.x, y: bb.min.y },
        bb.max,
        c2v { x: bb.min.x, y: bb.max.y },
        c2v::default(),
        c2v::default(),
        c2v::default(),
        c2v::default(),
    ];
    for &(dx, dy) in &[
        (1.0f32, 0.0f32),
        (-1.0, 0.0),
        (0.0, 1.0),
        (0.0, -1.0),
        (1.0, 1.0),
        (1.0, -1.0),
        (-1.0, 1.0),
        (-1.0, -1.0),
    ] {
        let d = c2v { x: dx, y: dy };
        support_case(&quad, 4, d, &(d, "octant"));
    }
    // NaN / Inf directions: `dot > dmax` is never true for NaN -> index 0
    for &s in SPECIAL_F32 {
        for &t in SPECIAL_F32 {
            let d = c2v { x: s, y: t };
            support_case(&quad, 4, d, &(d, "special-dir"));
        }
    }
    // NaN / Inf vertices
    for &s in SPECIAL_F32 {
        let verts = [
            c2v { x: s, y: s },
            c2v { x: 1.0, y: 2.0 },
            c2v { x: -s, y: s },
            c2v { x: 3.0, y: -4.0 },
            c2v { x: s, y: -s },
            c2v { x: 0.0, y: 0.0 },
            c2v { x: 9.0, y: 9.0 },
            c2v { x: -s, y: -s },
        ];
        for &count in &[1i32, 2, 4, 8] {
            let d = c2v { x: 0.5, y: -0.25 };
            support_case(&verts, count, d, &(s, count, "special-verts"));
        }
    }
}

// -------------------------------------------------------------------- row 23
#[test]
fn row23_c2GJKSimplexMetric() {
    let p = apis();
    let mut rng = Rng::new(0x2300);
    for &count in &[1i32, 2, 3] {
        for _ in 0..n_cases(N) {
            let mut sc = rng.simplex(count);
            let mut sr = sc;
            unsafe {
                let cv = (p.c.c2GJKSimplexMetric)(&mut sc);
                let rv = (p.r.c2GJKSimplexMetric)(&mut sr);
                eq_f32("c2GJKSimplexMetric", &(count, sc.verts[0].p), cv, rv);
            }
            eq_simplex("c2GJKSimplexMetric/simplex-untouched", &count, &sc, &sr);
        }
    }
    // NaN / Inf `p` fields
    for &s in SPECIAL_F32 {
        for &count in &[1i32, 2, 3] {
            let mut sc: c2Simplex = unsafe { std::mem::zeroed() };
            sc.count = count;
            sc.div = 1.0;
            sc.verts[0].p = c2v { x: s, y: -s };
            sc.verts[1].p = c2v { x: 1.0, y: s };
            sc.verts[2].p = c2v { x: -s, y: 2.0 };
            let mut sr = sc;
            unsafe {
                let cv = (p.c.c2GJKSimplexMetric)(&mut sc);
                let rv = (p.r.c2GJKSimplexMetric)(&mut sr);
                eq_f32("c2GJKSimplexMetric/sp", &(s, count), cv, rv);
            }
        }
    }
}

// ----------------------------------------------------------------- rows 24-27
/// Runs `c22` on both libs, comparing the full 152-byte simplex afterwards.
fn c22_case(s: c2Simplex, ctx: &dyn std::fmt::Debug) {
    let p = apis();
    let mut sc = s;
    let mut sr = s;
    unsafe {
        (p.c.c22)(&mut sc);
        (p.r.c22)(&mut sr);
    }
    eq_simplex("c22", ctx, &sc, &sr);
}

fn simplex_from_p(ps: &[c2v], count: c_int, div: f32) -> c2Simplex {
    let mut s: c2Simplex = unsafe { std::mem::zeroed() };
    for (i, v) in ps.iter().enumerate().take(4) {
        s.verts[i].p = *v;
        s.verts[i].sA = c2v { x: v.x, y: -v.y };
        s.verts[i].sB = c2v { x: -v.x, y: v.y };
        s.verts[i].u = 1.0;
        s.verts[i].iA = i as c_int;
        s.verts[i].iB = (3 - i) as c_int;
    }
    s.div = div;
    s.count = count;
    s
}

#[test]
fn row24_c22_vertex_a_region() {
    // v = dot(a, a-b) <= 0  =>  A is the closest feature.
    // Pick b "beyond" a along the same ray: a = t*u, b = k*u with k > t > 0.
    let mut rng = Rng::new(0x2400);
    let mut hits = 0;
    for _ in 0..n_cases(N) {
        let ang = rng.f32_in(-3.15, 3.15);
        let u = c2v { x: ang.cos(), y: ang.sin() };
        let t = rng.f32_in(0.01, 50.0);
        let k = t + rng.f32_in(0.01, 50.0);
        let a = c2v { x: u.x * t, y: u.y * t };
        let b = c2v { x: u.x * k, y: u.y * k };
        // v = dot(a, a-b) = t*(t-k) <= 0
        if a.x * (a.x - b.x) + a.y * (a.y - b.y) <= 0.0 {
            hits += 1;
        }
        c22_case(simplex_from_p(&[a, b], 2, rng.f32_in(-4.0, 4.0)), &(a, b));
    }
    assert!(hits > n_cases(N) / 2, "vertex-A region not exercised enough");
}

#[test]
fn row25_c22_vertex_b_region() {
    // u = dot(b, b-a) <= 0 (and v > 0)  =>  B closest; triggers `s->a = s->b`.
    let mut rng = Rng::new(0x2500);
    for _ in 0..n_cases(N) {
        let ang = rng.f32_in(-3.15, 3.15);
        let u = c2v { x: ang.cos(), y: ang.sin() };
        let k = rng.f32_in(0.01, 50.0);
        let t = k + rng.f32_in(0.01, 50.0);
        let a = c2v { x: u.x * t, y: u.y * t };
        let b = c2v { x: u.x * k, y: u.y * k };
        c22_case(simplex_from_p(&[a, b], 2, rng.f32_in(-4.0, 4.0)), &(a, b));
    }
}

#[test]
fn row26_c22_interior_edge_region() {
    // Origin projects strictly inside the segment: u>0 and v>0.
    let mut rng = Rng::new(0x2600);
    for _ in 0..n_cases(N) {
        let ang = rng.f32_in(-3.15, 3.15);
        let (dx, dy) = (ang.cos(), ang.sin());
        let off = rng.f32_in(-20.0, 20.0);
        let (nx, ny) = (-dy * off, dx * off);
        let ta = -rng.f32_in(0.01, 40.0);
        let tb = rng.f32_in(0.01, 40.0);
        let a = c2v { x: nx + dx * ta, y: ny + dy * ta };
        let b = c2v { x: nx + dx * tb, y: ny + dy * tb };
        c22_case(simplex_from_p(&[a, b], 2, rng.f32_in(-4.0, 4.0)), &(a, b));
    }
}

#[test]
fn row27_c22_unbiased_and_special() {
    let mut rng = Rng::new(0x2700);
    for _ in 0..n_cases(N) {
        c22_case(rng.simplex(2), &"random");
        c22_case(rng.simplex(1), &"random-count1");
        c22_case(rng.simplex(3), &"random-count3");
    }
    // NaN / Inf `p` (all comparisons false -> else branch)
    for &s in SPECIAL_F32 {
        for &t in SPECIAL_F32 {
            let a = c2v { x: s, y: t };
            let b = c2v { x: t, y: s };
            c22_case(simplex_from_p(&[a, b], 2, 1.0), &(a, b, "sp"));
            c22_case(simplex_from_p(&[a, b], 2, 0.0), &(a, b, "sp/div0"));
        }
    }
    // exactly coincident points -> u == v == 0 -> first branch
    for _ in 0..256 {
        let a = rng.v();
        c22_case(simplex_from_p(&[a, a], 2, 1.0), &(a, "coincident"));
    }
    // origin exactly on a vertex
    for _ in 0..256 {
        let b = rng.v();
        let z = c2v { x: 0.0, y: 0.0 };
        c22_case(simplex_from_p(&[z, b], 2, 1.0), &(b, "a==origin"));
        c22_case(simplex_from_p(&[b, z], 2, 1.0), &(b, "b==origin"));
    }
}

// ----------------------------------------------------------------- rows 28-35
fn c23_case(s: c2Simplex, ctx: &dyn std::fmt::Debug) {
    let p = apis();
    let mut sc = s;
    let mut sr = s;
    unsafe {
        (p.c.c23)(&mut sc);
        (p.r.c23)(&mut sr);
    }
    eq_simplex("c23", ctx, &sc, &sr);
}

/// Builds a triangle whose three vertices are `centre + radius * unit(angle_i)`,
/// which lets us place the origin in any Voronoi region of the triangle.
fn tri(rng: &mut Rng, cx: f32, cy: f32, rad: f32) -> [c2v; 3] {
    let base = rng.f32_in(-3.15, 3.15);
    let mut out = [c2v::default(); 3];
    for (i, o) in out.iter_mut().enumerate() {
        let a = base + (i as f32) * 2.0943951;
        *o = c2v {
            x: cx + rad * a.cos(),
            y: cy + rad * a.sin(),
        };
    }
    out
}

#[test]
fn row28_row34_c23_all_seven_branches() {
    // A wide sweep of triangle placements: the origin ends up in each of the 3
    // vertex regions, each of the 3 edge regions and the interior. We classify
    // afterwards to assert every branch really was reached.
    let mut rng = Rng::new(0x2800);
    let mut branch_hits = [0usize; 8];
    let n = n_cases(N).max(2000);
    for i in 0..n {
        let rad = rng.f32_in(0.5, 40.0);
        // distance of the origin from the triangle centre: 0 => interior,
        // >> rad => vertex/edge regions
        let dist = match i % 5 {
            0 => 0.0,
            1 => rng.f32_in(0.0, rad * 0.4),
            2 => rng.f32_in(rad * 0.8, rad * 1.3),
            3 => rng.f32_in(rad, rad * 4.0),
            _ => rng.f32_in(0.0, rad * 8.0),
        };
        let ang = rng.f32_in(-3.15, 3.15);
        let t = tri(&mut rng, -dist * ang.cos(), -dist * ang.sin(), rad);
        branch_hits[classify_c23(t[0], t[1], t[2])] += 1;
        c23_case(simplex_from_p(&t, 3, rng.f32_in(-4.0, 4.0)), &t);
    }
    for b in 1..=7 {
        assert!(
            branch_hits[b] > 0,
            "c23 branch {b} never exercised: {branch_hits:?}"
        );
    }
}

/// Mirrors the C branch ladder in `c23` (l.222..261) so the test can assert
/// coverage. Returns 1..7 for the seven branches.
fn classify_c23(a: c2v, b: c2v, c: c2v) -> usize {
    fn dot(p: c2v, q: c2v) -> f32 {
        p.x * q.x + p.y * q.y
    }
    fn sub(p: c2v, q: c2v) -> c2v {
        c2v {
            x: p.x - q.x,
            y: p.y - q.y,
        }
    }
    fn det(p: c2v, q: c2v) -> f32 {
        p.x * q.y - p.y * q.x
    }
    let uAB = dot(b, sub(b, a));
    let vAB = dot(a, sub(a, b));
    let uBC = dot(c, sub(c, b));
    let vBC = dot(b, sub(b, c));
    let uCA = dot(a, sub(a, c));
    let vCA = dot(c, sub(c, a));
    let area = det(sub(b, a), sub(c, a));
    let uABC = det(b, c) * area;
    let vABC = det(c, a) * area;
    let wABC = det(a, b) * area;
    if vAB <= 0.0 && uCA <= 0.0 {
        1
    } else if uAB <= 0.0 && vBC <= 0.0 {
        2
    } else if uBC <= 0.0 && vCA <= 0.0 {
        3
    } else if uAB > 0.0 && vAB > 0.0 && wABC <= 0.0 {
        4
    } else if uBC > 0.0 && vBC > 0.0 && uABC <= 0.0 {
        5
    } else if uCA > 0.0 && vCA > 0.0 && vABC <= 0.0 {
        6
    } else {
        7
    }
}

#[test]
fn row35_c23_degenerate_and_special() {
    let mut rng = Rng::new(0x3500);
    for _ in 0..n_cases(N) {
        c23_case(rng.simplex(3), &"random");
        c23_case(rng.simplex(1), &"random-count1");
        c23_case(rng.simplex(2), &"random-count2");
        // collinear -> area == 0 -> uABC == vABC == wABC == 0
        let o = rng.v();
        let d = rng.v();
        let (t1, t2, t3) = (
            rng.f32_in(-9.0, 9.0),
            rng.f32_in(-9.0, 9.0),
            rng.f32_in(-9.0, 9.0),
        );
        let l = |t: f32| c2v {
            x: o.x + d.x * t,
            y: o.y + d.y * t,
        };
        c23_case(
            simplex_from_p(&[l(t1), l(t2), l(t3)], 3, rng.f32_in(-4.0, 4.0)),
            &"collinear",
        );
        // all three coincident
        let a = rng.v();
        c23_case(simplex_from_p(&[a, a, a], 3, 1.0), &"coincident");
    }
    for &s in SPECIAL_F32 {
        for &t in SPECIAL_F32 {
            let a = c2v { x: s, y: t };
            let b = c2v { x: t, y: s };
            let c = c2v { x: -s, y: -t };
            c23_case(simplex_from_p(&[a, b, c], 3, 1.0), &(a, b, c, "sp"));
            c23_case(simplex_from_p(&[a, b, c], 3, 0.0), &(a, b, c, "sp/div0"));
        }
    }
}

// -------------------------------------------------------------------- row 36
#[test]
fn row36_c2D() {
    let p = apis();
    let mut rng = Rng::new(0x3600);
    for &count in &[1i32, 2] {
        for _ in 0..n_cases(N) {
            let mut sc = rng.simplex(count);
            let mut sr = sc;
            unsafe {
                let cv = (p.c.c2D)(&mut sc);
                let rv = (p.r.c2D)(&mut sr);
                eq_v("c2D", &(count, sc.verts[0].p, sc.verts[1].p), cv, rv);
            }
            eq_simplex("c2D/untouched", &count, &sc, &sr);
        }
    }
    // count==2 with the determinant deliberately >0 and <=0
    for _ in 0..n_cases(N) {
        let a = rng.v();
        let b = rng.v();
        for &(x, y) in &[(a, b), (b, a)] {
            let mut sc = simplex_from_p(&[x, y], 2, 1.0);
            let mut sr = sc;
            unsafe {
                let cv = (p.c.c2D)(&mut sc);
                let rv = (p.r.c2D)(&mut sr);
                eq_v("c2D/det", &(x, y), cv, rv);
            }
        }
        // exactly zero determinant (collinear with the origin)
        let k = rng.f32_in(-5.0, 5.0);
        let ap = c2v { x: a.x * k, y: a.y * k };
        let mut sc = simplex_from_p(&[a, ap], 2, 1.0);
        let mut sr = sc;
        unsafe {
            let cv = (p.c.c2D)(&mut sc);
            let rv = (p.r.c2D)(&mut sr);
            eq_v("c2D/det0", &(a, ap), cv, rv);
        }
    }
    for &s in SPECIAL_F32 {
        for &t in SPECIAL_F32 {
            let a = c2v { x: s, y: t };
            let b = c2v { x: t, y: s };
            for &count in &[1i32, 2] {
                let mut sc = simplex_from_p(&[a, b], count, 1.0);
                let mut sr = sc;
                unsafe {
                    let cv = (p.c.c2D)(&mut sc);
                    let rv = (p.r.c2D)(&mut sr);
                    eq_v("c2D/sp", &(a, b, count), cv, rv);
                }
            }
        }
    }
}

// -------------------------------------------------------------------- row 37
#[test]
fn row37_c2L() {
    let p = apis();
    let mut rng = Rng::new(0x3700);
    for &count in &[1i32, 2] {
        for _ in 0..n_cases(N) {
            let mut sc = rng.simplex(count);
            let mut sr = sc;
            unsafe {
                let cv = (p.c.c2L)(&mut sc);
                let rv = (p.r.c2L)(&mut sr);
                eq_v("c2L", &(count, sc.div, sc.verts[0].u, sc.verts[1].u), cv, rv);
            }
            eq_simplex("c2L/untouched", &count, &sc, &sr);
        }
    }
    // div and u driven to the extremes, incl. div == +-0
    for &div in &[0.0f32, -0.0, 1.0, -1.0, f32::MIN_POSITIVE, f32::MAX, f32::INFINITY, f32::NAN] {
        for &u in SPECIAL_F32 {
            let mut sc = simplex_from_p(&[c2v { x: 3.0, y: -4.0 }, c2v { x: -1.5, y: 8.0 }], 2, div);
            sc.verts[0].u = u;
            sc.verts[1].u = -u;
            let mut sr = sc;
            unsafe {
                let cv = (p.c.c2L)(&mut sc);
                let rv = (p.r.c2L)(&mut sr);
                eq_v("c2L/sp", &(div, u), cv, rv);
            }
        }
    }
}

// -------------------------------------------------------------------- row 38
#[test]
fn row38_c2Witness() {
    let p = apis();
    let mut rng = Rng::new(0x3800);
    for &count in &[1i32, 2, 3] {
        for _ in 0..n_cases(N) {
            let mut sc = rng.simplex(count);
            let mut sr = sc;
            let mut ac = c2v { x: 1.5, y: -2.5 };
            let mut bc = c2v { x: -9.0, y: 4.0 };
            let (mut ar, mut br) = (ac, bc);
            unsafe {
                (p.c.c2Witness)(&mut sc, &mut ac, &mut bc);
                (p.r.c2Witness)(&mut sr, &mut ar, &mut br);
            }
            eq_v("c2Witness/a", &count, ac, ar);
            eq_v("c2Witness/b", &count, bc, br);
            eq_simplex("c2Witness/untouched", &count, &sc, &sr);
        }
    }
    for &div in &[0.0f32, -0.0, 1.0, -1.0, f32::MIN_POSITIVE, f32::MAX, f32::INFINITY, f32::NAN] {
        for &u in SPECIAL_F32 {
            for &count in &[1i32, 2, 3] {
                let mut sc: c2Simplex = unsafe { std::mem::zeroed() };
                sc.count = count;
                sc.div = div;
                for i in 0..3 {
                    sc.verts[i].sA = c2v { x: u, y: -u };
                    sc.verts[i].sB = c2v { x: 1.0 + i as f32, y: u };
                    sc.verts[i].u = if i % 2 == 0 { u } else { -u };
                }
                let mut sr = sc;
                let mut ac = c2v::default();
                let mut bc = c2v::default();
                let (mut ar, mut br) = (ac, bc);
                unsafe {
                    (p.c.c2Witness)(&mut sc, &mut ac, &mut bc);
                    (p.r.c2Witness)(&mut sr, &mut ar, &mut br);
                }
                eq_v("c2Witness/sp-a", &(div, u, count), ac, ar);
                eq_v("c2Witness/sp-b", &(div, u, count), bc, br);
            }
        }
    }
}
