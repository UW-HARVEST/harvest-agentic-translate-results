//! Phase B, CONFIGS.md rows 21-29: support mapping, edge normals, proxy building.
#![allow(non_snake_case)]
#![allow(clippy::unnecessary_cast, clippy::needless_range_loop, clippy::let_and_return)]
#![allow(clippy::field_reassign_with_default)]

mod common;
use common::*;
use std::ffi::c_void;

const N: usize = 4_000;

// ---------------------------------------------------------------------------
// Rows 21-23: c2Support
// ---------------------------------------------------------------------------

#[test]
fn row21_support_random_counts() {
    let l = libs();
    let (cf, rf) = l.get::<FnSupport>("c2Support");
    let mut rng = Rng::new(21);
    for &count in [1i32, 2, 3, 4, 8].iter() {
        for _ in 0..N {
            let mut verts = [c2v::default(); 8];
            for k in 0..8 {
                verts[k] = rng.vec_norm(100.0);
            }
            let d = rng.vec_norm(10.0);
            let (c, r) = unsafe { (cf(verts.as_ptr(), count, d), rf(verts.as_ptr(), count, d)) };
            eq_i32("c2Support", &format!("count={count} d={d:?} verts={verts:?}"), c, r);
        }
    }
}

#[test]
fn row22_support_ties() {
    let l = libs();
    let (cf, rf) = l.get::<FnSupport>("c2Support");
    let mut rng = Rng::new(22);
    // Lattice coordinates + axis-aligned directions make exact ties common; C uses
    // `>` not `>=`, so the FIRST maximal index must win.
    for &count in [1i32, 2, 4, 8].iter() {
        for _ in 0..N {
            let mut verts = [c2v::default(); 8];
            for k in 0..8 {
                verts[k] = rng.vec_lattice(3);
            }
            let d = match rng.below(5) {
                0 => v(1.0, 0.0),
                1 => v(0.0, 1.0),
                2 => v(-1.0, 0.0),
                3 => v(0.0, -1.0),
                _ => rng.vec_lattice(2),
            };
            let (c, r) = unsafe { (cf(verts.as_ptr(), count, d), rf(verts.as_ptr(), count, d)) };
            eq_i32("c2Support ties", &format!("count={count} d={d:?} verts={verts:?}"), c, r);
        }
    }
    // All vertices identical -> every dot equal -> index 0.
    for &count in [1i32, 2, 4, 8].iter() {
        let verts = [v(2.0, 3.0); 8];
        let d = v(1.0, 1.0);
        let (c, r) = unsafe { (cf(verts.as_ptr(), count, d), rf(verts.as_ptr(), count, d)) };
        eq_i32("c2Support all-equal", &format!("count={count}"), c, r);
        assert_eq!(c, 0);
    }
}

#[test]
fn row23_support_with_nan() {
    let l = libs();
    let (cf, rf) = l.get::<FnSupport>("c2Support");
    let mut rng = Rng::new(23);
    for count in 1i32..=8 {
        for _ in 0..N {
            let mut verts = [c2v::default(); 8];
            for k in 0..8 {
                verts[k] = if rng.below(3) == 0 { rng.vec_special() } else { rng.vec_norm(50.0) };
            }
            let d = if rng.below(3) == 0 { rng.vec_special() } else { rng.vec_norm(10.0) };
            let (c, r) = unsafe { (cf(verts.as_ptr(), count, d), rf(verts.as_ptr(), count, d)) };
            eq_i32("c2Support nan", &format!("count={count} d={d:?} verts={verts:?}"), c, r);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 24-26: c2Norms
// ---------------------------------------------------------------------------

#[test]
fn row24_norms_convex_polygons() {
    let l = libs();
    let (cf, rf) = l.get::<FnNorms>("c2Norms");
    let mut rng = Rng::new(24);
    for count in 3i32..=8 {
        for _ in 0..N {
            let (rad, ctr) = (rng.f_pos(50.0) + 0.1, rng.vec_norm(20.0));
            let p = convex_poly(&mut rng, count, rad, ctr);
            let mut cv = p.verts;
            let mut rv = p.verts;
            // 8 output slots, poisoned, so writes past `count` would be caught
            let mut cn = [poison_v(9); 8];
            let mut rn = [poison_v(9); 8];
            unsafe {
                cf(cv.as_mut_ptr(), cn.as_mut_ptr(), count);
                rf(rv.as_mut_ptr(), rn.as_mut_ptr(), count);
            }
            let ctx = format!("count={count} verts={:?}", p.verts);
            eq("c2Norms norms", &ctx, &cn, &rn);
            eq("c2Norms verts (must not be modified)", &ctx, &cv, &rv);
        }
    }
    // clockwise winding -> inward normals
    for count in 3i32..=8 {
        for _ in 0..500 {
            let (rad, ctr) = (rng.f_pos(30.0) + 0.1, rng.vec_norm(10.0));
            let p = concave_wound_poly(&mut rng, count, rad, ctr);
            let mut cv = p.verts;
            let mut rv = p.verts;
            let mut cn = [poison_v(9); 8];
            let mut rn = [poison_v(9); 8];
            unsafe {
                cf(cv.as_mut_ptr(), cn.as_mut_ptr(), count);
                rf(rv.as_mut_ptr(), rn.as_mut_ptr(), count);
            }
            eq("c2Norms CW", &format!("count={count}"), &cn, &rn);
        }
    }
}

#[test]
fn row25_norms_degenerate_counts() {
    let l = libs();
    let (cf, rf) = l.get::<FnNorms>("c2Norms");
    let mut rng = Rng::new(25);
    // count == 2: b wraps to 0 for i == 1, so norms[1] == -norms[0]
    for _ in 0..N {
        let mut verts = [c2v::default(); 8];
        for k in 0..8 {
            verts[k] = rng.vec_norm(20.0);
        }
        for &count in [2i32, 1, 0, -3].iter() {
            let mut cv = verts;
            let mut rv = verts;
            let mut cn = [poison_v(4); 8];
            let mut rn = [poison_v(4); 8];
            unsafe {
                cf(cv.as_mut_ptr(), cn.as_mut_ptr(), count);
                rf(rv.as_mut_ptr(), rn.as_mut_ptr(), count);
            }
            let ctx = format!("count={count} verts={verts:?}");
            eq("c2Norms norms", &ctx, &cn, &rn);
            eq("c2Norms verts", &ctx, &cv, &rv);
        }
    }
}

#[test]
fn row26_norms_duplicate_vertices() {
    let l = libs();
    let (cf, rf) = l.get::<FnNorms>("c2Norms");
    let mut rng = Rng::new(26);
    for count in 3i32..=8 {
        for _ in 0..N {
            let mut p = convex_poly(&mut rng, count, 10.0, v(0.0, 0.0));
            // duplicate one vertex onto its neighbour -> zero edge -> NaN normal
            let i = rng.below(count as u32) as usize;
            let j = (i + 1) % (count as usize);
            p.verts[j] = p.verts[i];
            // sometimes also inject a special value
            if rng.below(4) == 0 {
                p.verts[rng.below(count as u32) as usize] = rng.vec_special();
            }
            let mut cv = p.verts;
            let mut rv = p.verts;
            let mut cn = [poison_v(6); 8];
            let mut rn = [poison_v(6); 8];
            unsafe {
                cf(cv.as_mut_ptr(), cn.as_mut_ptr(), count);
                rf(rv.as_mut_ptr(), rn.as_mut_ptr(), count);
            }
            eq("c2Norms dup", &format!("count={count} verts={:?}", p.verts), &cn, &rn);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 27-29: c2MakeProxy for each handled type
// ---------------------------------------------------------------------------

#[test]
fn row27_make_proxy_circle() {
    let l = libs();
    let (cf, rf) = l.get::<FnMakeProxy>("c2MakeProxy");
    let mut rng = Rng::new(27);
    for i in 0..N {
        let circ = if i % 4 == 0 {
            c2Circle { p: rng.vec_special(), r: rng.f_special() }
        } else {
            c2Circle { p: rng.vec_norm(100.0), r: rng.f_norm(10.0) }
        };
        // pre-poisoned: the untouched verts[1..8] must stay identical
        let mut cp = poison_proxy(11);
        let mut rp = poison_proxy(11);
        unsafe {
            cf(&circ as *const c2Circle as *const c_void, C2_TYPE_CIRCLE, &mut cp);
            rf(&circ as *const c2Circle as *const c_void, C2_TYPE_CIRCLE, &mut rp);
        }
        eq("c2MakeProxy CIRCLE", &format!("circ={circ:?}"), &cp, &rp);
    }
}

#[test]
fn row28_make_proxy_aabb() {
    let l = libs();
    let (cf, rf) = l.get::<FnMakeProxy>("c2MakeProxy");
    let mut rng = Rng::new(28);
    for i in 0..N {
        let bb = match i % 4 {
            0 => {
                let min = rng.vec_norm(100.0);
                c2AABB { min, max: v(min.x + rng.f_pos(50.0), min.y + rng.f_pos(50.0)) }
            }
            1 => {
                let max = rng.vec_norm(100.0);
                c2AABB { min: v(max.x + rng.f_pos(50.0), max.y + rng.f_pos(50.0)), max } // inverted
            }
            2 => {
                let p = rng.vec_norm(100.0);
                c2AABB { min: p, max: p } // zero extent
            }
            _ => c2AABB { min: rng.vec_special(), max: rng.vec_special() },
        };
        let mut cp = poison_proxy(12);
        let mut rp = poison_proxy(12);
        let (mut cbb, mut rbb) = (bb, bb);
        unsafe {
            cf(&mut cbb as *mut c2AABB as *const c_void, C2_TYPE_AABB, &mut cp);
            rf(&mut rbb as *mut c2AABB as *const c_void, C2_TYPE_AABB, &mut rp);
        }
        eq("c2MakeProxy AABB", &format!("bb={bb:?}"), &cp, &rp);
        eq("c2MakeProxy AABB input", &format!("bb={bb:?}"), &cbb, &rbb);
    }
}

#[test]
fn row29_make_proxy_capsule() {
    let l = libs();
    let (cf, rf) = l.get::<FnMakeProxy>("c2MakeProxy");
    let mut rng = Rng::new(29);
    for i in 0..N {
        let cap = match i % 4 {
            0 => c2Capsule { a: rng.vec_norm(100.0), b: rng.vec_norm(100.0), r: rng.f_pos(10.0) },
            1 => {
                let p = rng.vec_norm(100.0);
                c2Capsule { a: p, b: p, r: rng.f_pos(10.0) } // degenerate
            }
            2 => c2Capsule { a: rng.vec_norm(100.0), b: rng.vec_norm(100.0), r: rng.f_norm(10.0) },
            _ => c2Capsule { a: rng.vec_special(), b: rng.vec_special(), r: rng.f_special() },
        };
        let mut cp = poison_proxy(13);
        let mut rp = poison_proxy(13);
        unsafe {
            cf(&cap as *const c2Capsule as *const c_void, C2_TYPE_CAPSULE, &mut cp);
            rf(&cap as *const c2Capsule as *const c_void, C2_TYPE_CAPSULE, &mut rp);
        }
        eq("c2MakeProxy CAPSULE", &format!("cap={cap:?}"), &cp, &rp);
    }
}
