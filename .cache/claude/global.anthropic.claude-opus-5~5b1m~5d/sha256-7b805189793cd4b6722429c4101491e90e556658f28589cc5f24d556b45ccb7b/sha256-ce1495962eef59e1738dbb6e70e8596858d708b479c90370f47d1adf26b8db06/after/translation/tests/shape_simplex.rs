//! Phase B — `CONFIGS.md` rows 16..41.
//!
//! Shape/proxy construction and the simplex primitives, driven **directly**
//! through the low-level exported symbols with hand-built `c2Simplex` states
//! rather than only through `c2GJK`. Every assertion compares the full struct
//! byte-for-byte (72 bytes for `c2Proxy`, 152 for `c2Simplex`), so a field that
//! the C leaves untouched must also be untouched by the Rust.

mod common;
use common::*;
use std::ffi::c_void;
use std::os::raw::c_int;

const N: usize = 4_000;

// ---------------------------------------------------------------------------
// Row 16 — c2BBVerts
// ---------------------------------------------------------------------------

#[test]
fn cfg_bbverts() {
    let (c, r) = both();
    let mut rng = Rng::new(16);
    for i in 0..N {
        for bb in [
            rng.aabb(),
            {
                let p = rng.v();
                c2AABB { min: p, max: p } // degenerate
            },
            {
                let a = rng.v();
                c2AABB {
                    min: c2v { x: a.x + 1.0, y: a.y + 1.0 },
                    max: a, // inverted
                }
            },
            c2AABB {
                min: rng.any_v(),
                max: rng.any_v(), // NaN / inf corners
            },
        ] {
            // 8 slots so an over-write past out[3] would be caught.
            let mut cout = [c2v { x: -7.5, y: 13.25 }; 8];
            let mut rout = cout;
            let mut cbb = bb;
            let mut rbb = bb;
            unsafe { (c.c2BBVerts)(cout.as_mut_ptr(), &mut cbb) };
            unsafe { (r.c2BBVerts)(rout.as_mut_ptr(), &mut rbb) };
            assert_bits_eq(&format!("c2BBVerts #{i} out {bb:?}"), &cout, &rout);
            assert_bits_eq(&format!("c2BBVerts #{i} bb unchanged"), &cbb, &rbb);
            // The C only writes out[0..3]; slots 4..7 must keep the sentinel
            // in BOTH libraries (compare the arrays, not Vec headers).
            let sentinel = [c2v { x: -7.5, y: 13.25 }; 4];
            let ctail: [c2v; 4] = cout[4..8].try_into().unwrap();
            let rtail: [c2v; 4] = rout[4..8].try_into().unwrap();
            assert_bits_eq(&format!("c2BBVerts #{i} C tail untouched"), &sentinel, &ctail);
            assert_bits_eq(
                &format!("c2BBVerts #{i} Rust tail untouched"),
                &sentinel,
                &rtail,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 17..20 — c2MakeProxy for each valid type
// ---------------------------------------------------------------------------

/// Fill a proxy with a recognisable pattern so we can see exactly which bytes
/// `c2MakeProxy` writes.
fn dirty_proxy(seed: u8) -> c2Proxy {
    let mut p = c2Proxy::default();
    p.radius = f32::from_bits(0xdead_0000 | seed as u32);
    p.count = -0x4321;
    for (k, v) in p.verts.iter_mut().enumerate() {
        v.x = f32::from_bits(0xcafe_0000 | (k as u32) << 4 | seed as u32);
        v.y = f32::from_bits(0xbeef_0000 | (k as u32) << 4 | seed as u32);
    }
    p
}

fn diff_makeproxy(ctx: &str, shape_ptr: *const c_void, ty: c_int, seed: u8) {
    let (c, r) = both();
    let mut cp = dirty_proxy(seed);
    let mut rp = dirty_proxy(seed);
    unsafe { (c.c2MakeProxy)(shape_ptr, ty, &mut cp) };
    unsafe { (r.c2MakeProxy)(shape_ptr, ty, &mut rp) };
    assert_bits_eq(ctx, &cp, &rp);
}

#[test]
fn cfg_makeproxy_circle() {
    let mut rng = Rng::new(17);
    for i in 0..N {
        let shapes = [
            rng.circle(),
            c2Circle { p: rng.v(), r: 0.0 },
            c2Circle { p: rng.v(), r: FLT_MAX },
            c2Circle {
                p: rng.any_v(),
                r: rng.any_f32(),
            },
        ];
        for (k, s) in shapes.iter().enumerate() {
            diff_makeproxy(
                &format!("c2MakeProxy CIRCLE #{i}.{k} {s:?}"),
                s as *const _ as *const c_void,
                C2_TYPE_CIRCLE,
                (i % 251) as u8,
            );
        }
    }
}

#[test]
fn cfg_makeproxy_aabb() {
    let mut rng = Rng::new(18);
    for i in 0..N {
        let p = rng.v();
        let shapes = [
            rng.aabb(),
            c2AABB { min: p, max: p },
            c2AABB {
                min: c2v { x: p.x + 5.0, y: p.y + 5.0 },
                max: p,
            },
            c2AABB {
                min: rng.any_v(),
                max: rng.any_v(),
            },
        ];
        for (k, s) in shapes.iter().enumerate() {
            diff_makeproxy(
                &format!("c2MakeProxy AABB #{i}.{k} {s:?}"),
                s as *const _ as *const c_void,
                C2_TYPE_AABB,
                (i % 251) as u8,
            );
        }
    }
}

#[test]
fn cfg_makeproxy_capsule() {
    let mut rng = Rng::new(19);
    for i in 0..N {
        let a = rng.v();
        let shapes = [
            rng.capsule(),
            c2Capsule { a, b: a, r: 0.0 },
            c2Capsule {
                a,
                b: rng.v(),
                r: FLT_MAX,
            },
            c2Capsule {
                a: rng.any_v(),
                b: rng.any_v(),
                r: rng.any_f32(),
            },
        ];
        for (k, s) in shapes.iter().enumerate() {
            diff_makeproxy(
                &format!("c2MakeProxy CAPSULE #{i}.{k} {s:?}"),
                s as *const _ as *const c_void,
                C2_TYPE_CAPSULE,
                (i % 251) as u8,
            );
        }
    }
}

/// Row 20 — which proxy bytes each type actually overwrites. `c2MakeProxy`
/// writes `verts[0]` only for a circle and `verts[0..1]` only for a capsule, so
/// the remaining slots must still hold the caller's pattern.
#[test]
fn cfg_makeproxy_partial_write() {
    let (c, r) = both();
    let mut rng = Rng::new(20);
    for i in 0..N {
        let circle = rng.circle();
        let capsule = rng.capsule();
        let aabb = rng.aabb();
        for (ty, ptr, written_verts) in [
            (
                C2_TYPE_CIRCLE,
                &circle as *const _ as *const c_void,
                1usize,
            ),
            (C2_TYPE_AABB, &aabb as *const _ as *const c_void, 4),
            (C2_TYPE_CAPSULE, &capsule as *const _ as *const c_void, 2),
        ] {
            let seed = (i % 251) as u8;
            let orig = dirty_proxy(seed);
            let mut cp = orig;
            let mut rp = orig;
            unsafe { (c.c2MakeProxy)(ptr, ty, &mut cp) };
            unsafe { (r.c2MakeProxy)(ptr, ty, &mut rp) };
            assert_bits_eq(&format!("partial write ty={ty} #{i}"), &cp, &rp);
            for k in written_verts..8 {
                assert_bits_eq(
                    &format!("ty={ty} verts[{k}] must be untouched #{i}"),
                    &orig.verts[k],
                    &cp.verts[k],
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 21..25 — c2Support
// ---------------------------------------------------------------------------

#[test]
fn cfg_support_counts() {
    let (c, r) = both();
    let mut rng = Rng::new(21);
    for count in [1i32, 2, 4, 8] {
        for i in 0..N {
            let mut verts = [c2v::default(); 8];
            for v in verts.iter_mut() {
                *v = if rng.below(5) == 0 { rng.any_v() } else { rng.v() };
            }
            let d = if rng.below(5) == 0 { rng.any_v() } else { rng.v() };
            let cv = unsafe { (c.c2Support)(verts.as_ptr(), count, d) };
            let rv = unsafe { (r.c2Support)(verts.as_ptr(), count, d) };
            assert_eq_ctx(
                &format!("c2Support count={count} #{i} d={d:?} verts={verts:?}"),
                cv,
                rv,
            );
            assert!(
                cv >= 0 && cv < count.max(1),
                "c2Support returned {cv} for count={count}"
            );
        }
    }
}

/// Row 25 — exact ties must keep the FIRST index (`dot > dmax`, not `>=`), and
/// `±0` dots must not perturb that.
#[test]
fn cfg_support_ties() {
    let (c, r) = both();
    let mut rng = Rng::new(25);
    for i in 0..N {
        let p = rng.v();
        // All eight vertices identical -> every dot ties -> must return 0.
        let verts = [p; 8];
        let d = rng.v();
        assert_eq_ctx(
            &format!("c2Support all-tie #{i}"),
            unsafe { (c.c2Support)(verts.as_ptr(), 8, d) },
            unsafe { (r.c2Support)(verts.as_ptr(), 8, d) },
        );
        // Mirrored pairs: dots are ±the same magnitude, including exact 0.
        let q = rng.v();
        let verts2 = [
            p,
            c2v { x: -p.x, y: -p.y },
            q,
            c2v { x: -q.x, y: -q.y },
            c2v { x: 0.0, y: 0.0 },
            c2v { x: -0.0, y: -0.0 },
            p,
            q,
        ];
        for d in [
            d,
            c2v { x: 0.0, y: 0.0 },
            c2v { x: -0.0, y: 0.0 },
            c2v { x: p.y, y: -p.x }, // perpendicular -> dot 0 for p
        ] {
            assert_eq_ctx(
                &format!("c2Support tie-pairs #{i} d={d:?}"),
                unsafe { (c.c2Support)(verts2.as_ptr(), 8, d) },
                unsafe { (r.c2Support)(verts2.as_ptr(), 8, d) },
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 26, 27 — c2GJKSimplexMetric with the two meaningful counts
// ---------------------------------------------------------------------------

#[test]
fn cfg_simplexmetric() {
    let mut rng = Rng::new(26);
    for i in 0..N {
        for count in [2i32, 3] {
            let mut vs = [c2sv::default(); 4];
            for v in vs.iter_mut() {
                *v = rng.sv();
            }
            // Also exercise collinear / coincident p's, where area == 0.
            if rng.below(4) == 0 {
                vs[1].p = vs[0].p;
            }
            if rng.below(4) == 0 {
                vs[2].p = c2v {
                    x: vs[0].p.x + (vs[1].p.x - vs[0].p.x) * 2.0,
                    y: vs[0].p.y + (vs[1].p.y - vs[0].p.y) * 2.0,
                };
            }
            if rng.below(6) == 0 {
                vs[rng.below(3) as usize].p = rng.any_v();
            }
            let s = simplex(count, rng.coord(), &vs);
            let (cv, rv) = diff_simplex(&format!("c2GJKSimplexMetric count={count} #{i}"), &s, |a, p| {
                unsafe { (a.c2GJKSimplexMetric)(p) }
            });
            assert_f32_bits_eq(
                &format!("c2GJKSimplexMetric count={count} #{i} {vs:?}"),
                cv,
                rv,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 28..30 — c22, with every arm proven hit
// ---------------------------------------------------------------------------

/// Which of `c22`'s three arms the C will take, computed with the C's own
/// exported `c2Dot`/`c2Sub` so the classification is exactly the library's.
fn c22_arm(api: &Api, a: c2v, b: c2v) -> usize {
    let u = unsafe { (api.c2Dot)(b, (api.c2Sub)(b, a)) };
    let v = unsafe { (api.c2Dot)(a, (api.c2Sub)(a, b)) };
    if v <= 0.0 {
        0
    } else if u <= 0.0 {
        1
    } else {
        2
    }
}

#[test]
fn cfg_c22_all_regions() {
    let (c, _) = both();
    let mut rng = Rng::new(28);
    let mut hits = [0usize; 3];
    for i in 0..N * 4 {
        let mut vs = [rng.sv(), rng.sv(), rng.sv(), rng.sv()];
        // Spread the segment around the origin so all three Voronoi regions of
        // the segment get sampled.
        vs[0].p = rng.v();
        vs[1].p = rng.v();
        if rng.below(8) == 0 {
            vs[0].p = rng.any_v();
        }
        if rng.below(8) == 0 {
            vs[1].p = rng.any_v();
        }
        let s = simplex(2, rng.coord(), &vs);
        hits[c22_arm(c, vs[0].p, vs[1].p)] += 1;
        diff_simplex(&format!("c22 #{i} {vs:?}"), &s, |a, p| unsafe { (a.c22)(p) });
    }
    assert!(
        hits.iter().all(|&h| h > 50),
        "c22 arms under-exercised: {hits:?} (need all three of v<=0 / u<=0 / edge)"
    );
    println!("c22 arm hits (vertexA, vertexB, edge) = {hits:?}");
}

/// Row 29 — degenerate segment `a.p == b.p`: `u == v == 0`, so `v <= 0` takes
/// the first arm. Also checks that the `s->a = s->b` copy in arm 1 moves the
/// whole 36-byte `c2sv` (row 30), which a p-only comparison would miss.
#[test]
fn cfg_c22_degenerate() {
    let mut rng = Rng::new(29);
    for i in 0..N {
        let mut vs = [rng.sv(), rng.sv(), rng.sv(), rng.sv()];
        vs[1].p = vs[0].p;
        let s = simplex(2, rng.coord(), &vs);
        diff_simplex(&format!("c22 degenerate #{i}"), &s, |a, p| unsafe {
            (a.c22)(p)
        });

        // Force arm 1 (u <= 0, v > 0): put the origin beyond b, i.e. pick
        // a and b so that dot(b, b-a) <= 0 < dot(a, a-b).
        let mut vs2 = [rng.sv(), rng.sv(), rng.sv(), rng.sv()];
        let dir = c2v { x: 1.0, y: 0.0 };
        vs2[0].p = c2v { x: 2.0, y: 0.0 };
        vs2[1].p = c2v {
            x: 2.0 + dir.x, // b farther out; origin is in A's region
            y: 0.0,
        };
        // Swap so that b is the closer end -> u <= 0.
        vs2.swap(0, 1);
        let s2 = simplex(2, rng.coord(), &vs2);
        diff_simplex(&format!("c22 arm1 #{i}"), &s2, |a, p| unsafe { (a.c22)(p) });

        // Zero-length at the origin: p == (0,0) for both -> u == v == 0.
        let mut vs3 = [rng.sv(), rng.sv(), rng.sv(), rng.sv()];
        vs3[0].p = c2v { x: 0.0, y: 0.0 };
        vs3[1].p = c2v { x: -0.0, y: 0.0 };
        let s3 = simplex(2, 0.0, &vs3);
        diff_simplex(&format!("c22 origin #{i}"), &s3, |a, p| unsafe { (a.c22)(p) });
    }
}

// ---------------------------------------------------------------------------
// Rows 31..33 — c23, with all seven arms proven hit
// ---------------------------------------------------------------------------

fn c23_arm(api: &Api, a: c2v, b: c2v, c: c2v) -> usize {
    let dot = |p, q| unsafe { (api.c2Dot)(p, q) };
    let sub = |p, q| unsafe { (api.c2Sub)(p, q) };
    let det = |p, q| unsafe { (api.c2Det2)(p, q) };
    let mul = |p: f32, q: f32| p * q;

    let u_ab = dot(b, sub(b, a));
    let v_ab = dot(a, sub(a, b));
    let u_bc = dot(c, sub(c, b));
    let v_bc = dot(b, sub(b, c));
    let u_ca = dot(a, sub(a, c));
    let v_ca = dot(c, sub(c, a));
    let area = det(sub(b, a), sub(c, a));
    let u_abc = mul(det(b, c), area);
    let v_abc = mul(det(c, a), area);
    let w_abc = mul(det(a, b), area);

    if v_ab <= 0.0 && u_ca <= 0.0 {
        0
    } else if u_ab <= 0.0 && v_bc <= 0.0 {
        1
    } else if u_bc <= 0.0 && v_ca <= 0.0 {
        2
    } else if u_ab > 0.0 && v_ab > 0.0 && w_abc <= 0.0 {
        3
    } else if u_bc > 0.0 && v_bc > 0.0 && u_abc <= 0.0 {
        4
    } else if u_ca > 0.0 && v_ca > 0.0 && v_abc <= 0.0 {
        5
    } else {
        6
    }
}

#[test]
fn cfg_c23_all_regions() {
    let (c, _) = both();
    let mut rng = Rng::new(31);
    let mut hits = [0usize; 7];
    for i in 0..N * 8 {
        let mut vs = [rng.sv(), rng.sv(), rng.sv(), rng.sv()];
        // A mix of triangles: some enclosing the origin (interior arm), some
        // off to one side (vertex/edge arms), both windings.
        let scale = *pick(&[0.5f32, 1.0, 3.0, 30.0], &mut rng);
        let off = if rng.below(3) == 0 {
            c2v { x: 0.0, y: 0.0 }
        } else {
            c2v {
                x: rng.unit() * scale * 2.0,
                y: rng.unit() * scale * 2.0,
            }
        };
        for k in 0..3 {
            vs[k].p = c2v {
                x: off.x + rng.unit() * scale,
                y: off.y + rng.unit() * scale,
            };
        }
        if rng.bool() {
            vs.swap(1, 2); // flip winding -> area changes sign
        }
        if rng.below(10) == 0 {
            vs[rng.below(3) as usize].p = rng.any_v();
        }
        let s = simplex(3, rng.coord(), &vs);
        hits[c23_arm(c, vs[0].p, vs[1].p, vs[2].p)] += 1;
        diff_simplex(&format!("c23 #{i} {vs:?}"), &s, |a, p| unsafe { (a.c23)(p) });
    }
    assert!(
        hits.iter().all(|&h| h > 20),
        "c23 arms under-exercised: {hits:?} \
         (need A,B,C,AB,BC,CA,interior all hit)"
    );
    println!("c23 arm hits (A,B,C,AB,BC,CA,interior) = {hits:?}");
}

/// Row 32 — collinear or duplicated points make `area == 0`, so
/// `uABC == vABC == wABC == 0` and the `<= 0` sub-conditions all pass.
#[test]
fn cfg_c23_degenerate() {
    let mut rng = Rng::new(32);
    for i in 0..N {
        let mut vs = [rng.sv(), rng.sv(), rng.sv(), rng.sv()];
        let base = rng.v();
        let dir = rng.v();
        match i % 4 {
            0 => {
                // strictly collinear
                vs[0].p = base;
                vs[1].p = c2v { x: base.x + dir.x, y: base.y + dir.y };
                vs[2].p = c2v {
                    x: base.x + dir.x * 2.0,
                    y: base.y + dir.y * 2.0,
                };
            }
            1 => {
                // two coincident
                vs[0].p = base;
                vs[1].p = base;
                vs[2].p = rng.v();
            }
            2 => {
                // all three coincident
                vs[0].p = base;
                vs[1].p = base;
                vs[2].p = base;
            }
            _ => {
                // all at the origin
                vs[0].p = c2v { x: 0.0, y: 0.0 };
                vs[1].p = c2v { x: -0.0, y: 0.0 };
                vs[2].p = c2v { x: 0.0, y: -0.0 };
            }
        }
        let s = simplex(3, rng.coord(), &vs);
        diff_simplex(&format!("c23 degenerate #{i} case{}", i % 4), &s, |a, p| {
            unsafe { (a.c23)(p) }
        });
    }
}

// ---------------------------------------------------------------------------
// Rows 34..36 — c2D
// ---------------------------------------------------------------------------

#[test]
fn cfg_c2d_counts() {
    let (c, _) = both();
    let mut rng = Rng::new(34);
    let mut skew = 0usize;
    let mut ccw = 0usize;
    for i in 0..N * 2 {
        // count == 1
        let mut vs = [rng.sv(), rng.sv(), rng.sv(), rng.sv()];
        vs[0].p = if rng.below(6) == 0 { rng.any_v() } else { rng.v() };
        let s1 = simplex(1, rng.coord(), &vs);
        let (cv, rv) = diff_simplex(&format!("c2D count=1 #{i}"), &s1, |a, p| unsafe {
            (a.c2D)(p)
        });
        assert_bits_eq(&format!("c2D count=1 #{i} {vs:?}"), &cv, &rv);

        // count == 2, both orientations of c2Det2(ab, -a.p)
        vs[1].p = if rng.below(6) == 0 { rng.any_v() } else { rng.v() };
        let s2 = simplex(2, rng.coord(), &vs);
        let ab = unsafe { (c.c2Sub)(vs[1].p, vs[0].p) };
        let det = unsafe { (c.c2Det2)(ab, (c.c2Neg)(vs[0].p)) };
        if det > 0.0 {
            skew += 1
        } else {
            ccw += 1
        }
        let (cv, rv) = diff_simplex(&format!("c2D count=2 #{i}"), &s2, |a, p| unsafe {
            (a.c2D)(p)
        });
        assert_bits_eq(&format!("c2D count=2 #{i} {vs:?} det={det}"), &cv, &rv);
    }
    // Exactly-zero determinant (a.p, b.p and the origin collinear) -> c2CCW90.
    let mut rng = Rng::new(0x34);
    for i in 0..N {
        let mut vs = [rng.sv(), rng.sv(), rng.sv(), rng.sv()];
        let d = rng.v();
        vs[0].p = d;
        vs[1].p = c2v { x: d.x * 2.0, y: d.y * 2.0 }; // collinear with origin
        let s = simplex(2, rng.coord(), &vs);
        let (cv, rv) = diff_simplex(&format!("c2D det=0 #{i}"), &s, |a, p| unsafe {
            (a.c2D)(p)
        });
        assert_bits_eq(&format!("c2D det=0 #{i}"), &cv, &rv);
    }
    assert!(
        skew > 100 && ccw > 100,
        "c2D orientation arms under-exercised: skew={skew} ccw={ccw}"
    );
    println!("c2D count=2 arms: c2Skew={skew} c2CCW90={ccw}");
}

// ---------------------------------------------------------------------------
// Rows 37..41 — c2Witness and c2L
// ---------------------------------------------------------------------------

/// `div` values that matter: 1, random, exact 0 (den = inf), tiny, huge, and
/// the non-finite classes.
fn div_values(rng: &mut Rng) -> Vec<f32> {
    vec![
        1.0,
        rng.coord(),
        rng.coord(),
        0.0,
        -0.0,
        f32::from_bits(1),
        FLT_MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        rng.nan(),
        3.0,
        -2.5,
    ]
}

#[test]
fn cfg_witness_counts() {
    let (c, r) = both();
    let mut rng = Rng::new(37);
    for count in [1i32, 2, 3] {
        for i in 0..N {
            let mut vs = [rng.sv(), rng.sv(), rng.sv(), rng.sv()];
            if rng.below(6) == 0 {
                let k = rng.below(3) as usize;
                vs[k].u = rng.any_f32();
                vs[k].sA = rng.any_v();
                vs[k].sB = rng.any_v();
            }
            for div in div_values(&mut rng) {
                let s = simplex(count, div, &vs);
                let mut cs = s;
                let mut rs = s;
                let mut ca = c2v { x: 1.5, y: -2.5 };
                let mut cb = c2v { x: 3.5, y: -4.5 };
                let mut ra = ca;
                let mut rb = cb;
                unsafe { (c.c2Witness)(&mut cs, &mut ca, &mut cb) };
                unsafe { (r.c2Witness)(&mut rs, &mut ra, &mut rb) };
                let ctx = format!("c2Witness count={count} div={div:?} #{i}");
                assert_bits_eq(&format!("{ctx} / a"), &ca, &ra);
                assert_bits_eq(&format!("{ctx} / b"), &cb, &rb);
                assert_bits_eq(&format!("{ctx} / simplex"), &cs, &rs);
            }
        }
    }
}

#[test]
fn cfg_c2l_counts() {
    let mut rng = Rng::new(40);
    for count in [1i32, 2] {
        for i in 0..N {
            let mut vs = [rng.sv(), rng.sv(), rng.sv(), rng.sv()];
            if rng.below(6) == 0 {
                let k = rng.below(2) as usize;
                vs[k].u = rng.any_f32();
                vs[k].p = rng.any_v();
            }
            for div in div_values(&mut rng) {
                let s = simplex(count, div, &vs);
                let (cv, rv) = diff_simplex(
                    &format!("c2L count={count} div={div:?} #{i}"),
                    &s,
                    |a, p| unsafe { (a.c2L)(p) },
                );
                assert_bits_eq(
                    &format!("c2L count={count} div={div:?} #{i} {vs:?}"),
                    &cv,
                    &rv,
                );
            }
        }
    }
}
