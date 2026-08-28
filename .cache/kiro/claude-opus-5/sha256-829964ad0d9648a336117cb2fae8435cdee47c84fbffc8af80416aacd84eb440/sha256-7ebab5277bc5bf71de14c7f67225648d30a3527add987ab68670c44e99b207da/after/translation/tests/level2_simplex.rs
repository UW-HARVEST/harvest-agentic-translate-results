//! Level 2: proxy construction, simplex reduction, support and witness points.

#![allow(non_snake_case)]

mod common;
use common::*;

use std::ffi::c_void;

const RANGE: f32 = 250.0;

fn rand_simplex(g: &mut Rng, count: i32) -> c2Simplex {
    let mut s = c2Simplex::default();
    for v in s.verts.iter_mut() {
        v.sA = g.v_spicy(RANGE);
        v.sB = g.v_spicy(RANGE);
        v.p = g.v_spicy(RANGE);
        v.u = g.f_spicy(RANGE);
        v.iA = g.below(8) as i32;
        v.iB = g.below(8) as i32;
    }
    s.div = g.f_spicy(RANGE);
    s.count = count;
    s
}

/// Builds a simplex whose `p` values are deliberately correlated (shared points,
/// collinear points, mirrored points) so the many `<= 0` branches in c23 are hit.
fn tricky_simplex(g: &mut Rng, count: i32) -> c2Simplex {
    let mut s = rand_simplex(g, count);
    let base = g.v(20.0);
    let dir = g.v(20.0);
    match g.below(6) {
        0 => {
            // collinear
            s.verts[0].p = base;
            s.verts[1].p = c2v {
                x: base.x + dir.x,
                y: base.y + dir.y,
            };
            s.verts[2].p = c2v {
                x: base.x + 2.0 * dir.x,
                y: base.y + 2.0 * dir.y,
            };
        }
        1 => {
            // duplicated vertices
            s.verts[0].p = base;
            s.verts[1].p = base;
            s.verts[2].p = dir;
        }
        2 => {
            // all identical
            s.verts[0].p = base;
            s.verts[1].p = base;
            s.verts[2].p = base;
        }
        3 => {
            // triangle enclosing the origin
            s.verts[0].p = c2v { x: -1.0, y: -1.0 };
            s.verts[1].p = c2v { x: 2.0, y: -1.0 };
            s.verts[2].p = c2v { x: 0.0, y: 3.0 };
        }
        4 => {
            // origin exactly on an edge
            s.verts[0].p = c2v { x: -1.0, y: 0.0 };
            s.verts[1].p = c2v { x: 1.0, y: 0.0 };
            s.verts[2].p = c2v { x: 0.0, y: dir.y };
        }
        _ => {
            // small magnitudes near the origin
            s.verts[0].p = g.v(1e-3);
            s.verts[1].p = g.v(1e-3);
            s.verts[2].p = g.v(1e-3);
        }
    }
    if g.below(4) == 0 {
        s.verts[3].p = g.v(20.0);
    }
    s
}

#[test]
fn t_c2BBVerts() {
    let (c, r) = both::<FnBBVerts>("c2BBVerts");
    let mut g = Rng::new(20);
    for _ in 0..20_000 {
        let mut bb = c2AABB {
            min: g.v_spicy(RANGE),
            max: g.v_spicy(RANGE),
        };
        // Both output buffers start from the same non-zero contents so that any
        // element the C code leaves untouched is compared too.
        let seed = [
            g.v_spicy(RANGE),
            g.v_spicy(RANGE),
            g.v_spicy(RANGE),
            g.v_spicy(RANGE),
            g.v_spicy(RANGE),
            g.v_spicy(RANGE),
        ];
        let mut co = seed;
        let mut ro = seed;
        unsafe {
            c(co.as_mut_ptr(), &mut bb);
            r(ro.as_mut_ptr(), &mut bb);
        }
        assert_bytes(&format!("c2BBVerts({bb:?})"), &co, &ro);
    }
}

#[test]
fn t_c2MakeProxy() {
    let (c, r) = both::<FnMakeProxy>("c2MakeProxy");
    let mut g = Rng::new(21);

    // Pre-fill the proxies identically: C only writes some fields per type.
    let fill = |g: &mut Rng| {
        let mut p = c2Proxy {
            radius: g.f_spicy(RANGE),
            count: g.below(100) as i32,
            verts: Default::default(),
        };
        for v in p.verts.iter_mut() {
            *v = g.v_spicy(RANGE);
        }
        p
    };

    for _ in 0..20_000 {
        let circle = c2Circle {
            p: g.v_spicy(RANGE),
            r: g.f_spicy(RANGE),
        };
        let aabb = c2AABB {
            min: g.v_spicy(RANGE),
            max: g.v_spicy(RANGE),
        };
        let cap = c2Capsule {
            a: g.v_spicy(RANGE),
            b: g.v_spicy(RANGE),
            r: g.f_spicy(RANGE),
        };
        let seed = fill(&mut g);

        let cases: [(&str, *const c_void, i32); 3] = [
            (
                "circle",
                &circle as *const c2Circle as *const c_void,
                C2_TYPE_CIRCLE,
            ),
            ("aabb", &aabb as *const c2AABB as *const c_void, C2_TYPE_AABB),
            (
                "capsule",
                &cap as *const c2Capsule as *const c_void,
                C2_TYPE_CAPSULE,
            ),
        ];
        for (label, ptr, ty) in cases {
            let mut cp = seed;
            let mut rp = seed;
            unsafe {
                c(ptr, ty, &mut cp);
                r(ptr, ty, &mut rp);
            }
            assert_bytes(&format!("c2MakeProxy/{label}"), &cp, &rp);
        }
    }

    // Types outside the enum: the C `switch` has no default, so nothing is
    // written and the caller's buffer must be left completely untouched.
    for ty in [-2i32, -1, 3, 4, 99, i32::MIN, i32::MAX] {
        let circle = c2Circle {
            p: c2v { x: 1.0, y: 2.0 },
            r: 3.0,
        };
        let seed = fill(&mut g);
        let mut cp = seed;
        let mut rp = seed;
        unsafe {
            c(&circle as *const c2Circle as *const c_void, ty, &mut cp);
            r(&circle as *const c2Circle as *const c_void, ty, &mut rp);
        }
        assert_bytes(&format!("c2MakeProxy/type={ty}"), &cp, &rp);
        assert_bytes(&format!("c2MakeProxy/type={ty} untouched"), &seed, &cp);
    }
}

#[test]
fn t_c2Support() {
    let (c, r) = both::<FnSupport>("c2Support");
    let mut g = Rng::new(22);
    for _ in 0..40_000 {
        let n = 1 + g.below(8) as usize;
        let mut verts = [c2v::default(); 8];
        for v in verts.iter_mut().take(n) {
            *v = g.v_spicy(RANGE);
        }
        // Duplicate values sometimes, to exercise the strict `>` tie-break.
        if g.below(3) == 0 && n > 1 {
            let src = g.below(n as u32) as usize;
            let dst = g.below(n as u32) as usize;
            verts[dst] = verts[src];
        }
        let d = g.v_spicy(RANGE);
        unsafe {
            let cv = c(verts.as_ptr(), n as i32, d);
            let rv = r(verts.as_ptr(), n as i32, d);
            assert_eq!(cv, rv, "c2Support(n={n}, d={d:?}, verts={verts:?})");
        }
    }
    // count <= 1: the C loop body never runs but verts[0] is still read.
    for count in [0i32, 1, -1, -7] {
        let verts = [c2v { x: 4.0, y: -5.0 }; 8];
        let d = c2v { x: 1.0, y: 1.0 };
        unsafe {
            assert_eq!(
                c(verts.as_ptr(), count, d),
                r(verts.as_ptr(), count, d),
                "c2Support(count={count})"
            );
        }
    }
}

#[test]
fn t_c2GJKSimplexMetric() {
    let (c, r) = both::<FnSimplexF>("c2GJKSimplexMetric");
    let mut g = Rng::new(23);
    for _ in 0..40_000 {
        let count = match g.below(8) {
            0 => 0,
            1 => 4,
            2 => -1,
            3 => 7,
            n => (n - 3) as i32, // 1, 2, 3, 4
        };
        let mut cs = if g.below(2) == 0 {
            rand_simplex(&mut g, count)
        } else {
            tricky_simplex(&mut g, count)
        };
        let mut rs = cs;
        unsafe {
            let cv = c(&mut cs);
            let rv = r(&mut rs);
            assert_f32(&format!("c2GJKSimplexMetric(count={count})"), cv, rv);
        }
        assert_bytes("c2GJKSimplexMetric must not mutate", &cs, &rs);
    }
}

#[test]
fn t_c22() {
    let (c, r) = both::<FnSimplexVoid>("c22");
    let mut g = Rng::new(24);
    for _ in 0..60_000 {
        let mut cs = if g.below(2) == 0 {
            rand_simplex(&mut g, 2)
        } else {
            tricky_simplex(&mut g, 2)
        };
        let orig = cs;
        let mut rs = cs;
        unsafe {
            c(&mut cs);
            r(&mut rs);
        }
        assert_bytes(&format!("c22 on {orig:?}"), &cs, &rs);
    }
}

#[test]
fn t_c23() {
    let (c, r) = both::<FnSimplexVoid>("c23");
    let mut g = Rng::new(25);
    for _ in 0..120_000 {
        let mut cs = if g.below(3) == 0 {
            rand_simplex(&mut g, 3)
        } else {
            tricky_simplex(&mut g, 3)
        };
        let orig = cs;
        let mut rs = cs;
        unsafe {
            c(&mut cs);
            r(&mut rs);
        }
        assert_bytes(&format!("c23 on {orig:?}"), &cs, &rs);
    }
}

#[test]
fn t_c2D() {
    let (c, r) = both::<FnSimplexV>("c2D");
    let mut g = Rng::new(26);
    for _ in 0..60_000 {
        let count = match g.below(6) {
            0 => 0,
            1 => 3,
            2 => 4,
            3 => -3,
            n => (n - 3) as i32, // 1, 2
        };
        let mut cs = if g.below(2) == 0 {
            rand_simplex(&mut g, count)
        } else {
            tricky_simplex(&mut g, count)
        };
        let mut rs = cs;
        unsafe {
            let cv = c(&mut cs);
            let rv = r(&mut rs);
            assert_v(&format!("c2D(count={count})"), cv, rv);
        }
        assert_bytes("c2D must not mutate", &cs, &rs);
    }
}

#[test]
fn t_c2L() {
    let (c, r) = both::<FnSimplexV>("c2L");
    let mut g = Rng::new(27);
    for _ in 0..60_000 {
        let count = match g.below(7) {
            0 => 0,
            1 => 3,
            2 => 4,
            3 => -1,
            n => (n - 3) as i32,
        };
        let mut cs = rand_simplex(&mut g, count);
        // Also exercise div values that make 1/div special.
        match g.below(6) {
            0 => cs.div = 0.0,
            1 => cs.div = -0.0,
            2 => cs.div = f32::INFINITY,
            3 => cs.div = 1.0,
            _ => {}
        }
        let mut rs = cs;
        unsafe {
            let cv = c(&mut cs);
            let rv = r(&mut rs);
            assert_v(&format!("c2L(count={count}, div={})", cs.div), cv, rv);
        }
        assert_bytes("c2L must not mutate", &cs, &rs);
    }
}

#[test]
fn t_c2Witness() {
    let (c, r) = both::<FnWitness>("c2Witness");
    let mut g = Rng::new(28);
    for _ in 0..60_000 {
        let count = match g.below(8) {
            0 => 0,
            1 => 4,
            2 => -2,
            3 => 9,
            n => (n - 3) as i32, // 1, 2, 3, 4
        };
        let mut cs = rand_simplex(&mut g, count);
        match g.below(6) {
            0 => cs.div = 0.0,
            1 => cs.div = -0.0,
            2 => cs.div = f32::INFINITY,
            3 => cs.div = 1.0,
            _ => {}
        }
        let mut rs = cs;
        // Seed the outputs identically so an unwritten output is caught.
        let sentinel = (g.v_spicy(RANGE), g.v_spicy(RANGE));
        let (mut ca, mut cb) = sentinel;
        let (mut ra, mut rb) = sentinel;
        unsafe {
            c(&mut cs, &mut ca, &mut cb);
            r(&mut rs, &mut ra, &mut rb);
        }
        assert_v(&format!("c2Witness a (count={count})"), ca, ra);
        assert_v(&format!("c2Witness b (count={count})"), cb, rb);
        assert_bytes("c2Witness must not mutate the simplex", &cs, &rs);
    }
}
