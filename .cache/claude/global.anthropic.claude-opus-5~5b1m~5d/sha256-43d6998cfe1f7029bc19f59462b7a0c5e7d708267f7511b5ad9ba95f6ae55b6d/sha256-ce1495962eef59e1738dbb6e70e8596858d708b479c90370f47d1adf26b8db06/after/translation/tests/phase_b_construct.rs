//! Phase B — Group 3: AABB / poly construction, proxies, support.
//! CONFIGS.md rows 22..36.

mod common;
use common::*;
use std::os::raw::{c_int, c_void};

const N: usize = 2048;

/// Sentinel-filled proxy so that "untouched" fields are observable.
fn sentinel_proxy() -> c2Proxy {
    let mut p = c2Proxy {
        radius: -98765.5,
        count: -4242,
        verts: [c2v::default(); 8],
    };
    for (i, v) in p.verts.iter_mut().enumerate() {
        *v = c2v {
            x: 100.0 + i as f32,
            y: -100.0 - i as f32,
        };
    }
    p
}

// --- rows 22, 23 ---------------------------------------------------------
#[test]
fn cfg_bbverts() {
    let mut a = DiffAccum::new("cfg_bbverts");
    let mut rng = Rng::new(0x6eed_0001);
    for i in 0..N {
        let bb = rng.aabb();
        a.check(format!("{i} random {bb:?}"), |s| {
            let mut bb = bb;
            let mut out = [c2v { x: 7.0, y: 7.0 }; 6];
            c2BBVerts(s, out.as_mut_ptr(), &mut bb);
            (out.to_vec(), bb)
        });
    }
    // degenerate / inverted / non-finite
    for i in 0..N {
        let bb = c2AABB {
            min: rng.special_vec(),
            max: rng.special_vec(),
        };
        a.check(format!("{i} special {bb:?}"), |s| {
            let mut bb = bb;
            let mut out = [c2v { x: 7.0, y: 7.0 }; 6];
            c2BBVerts(s, out.as_mut_ptr(), &mut bb);
            (out.to_vec(), bb)
        });
    }
    for &(mn, mx) in &[
        (c2v { x: 0.0, y: 0.0 }, c2v { x: 0.0, y: 0.0 }),
        (c2v { x: 1.0, y: 1.0 }, c2v { x: -1.0, y: -1.0 }),
        (c2v { x: -0.0, y: -0.0 }, c2v { x: 0.0, y: 0.0 }),
    ] {
        let bb = c2AABB { min: mn, max: mx };
        a.check(format!("edge {bb:?}"), |s| {
            let mut bb = bb;
            let mut out = [c2v { x: 7.0, y: 7.0 }; 6];
            c2BBVerts(s, out.as_mut_ptr(), &mut bb);
            (out.to_vec(), bb)
        });
    }
    a.finish();
}

// --- rows 24..30 ---------------------------------------------------------
#[test]
fn cfg_norms() {
    let mut a = DiffAccum::new("cfg_norms");
    let mut rng = Rng::new(0x6eed_0002);

    // counts 1..8, random convex CCW polys
    for count in 1..=8usize {
        for i in 0..N {
            let verts = rng.convex_poly_verts(count);
            a.check(format!("ccw count={count} #{i}"), |s| {
                let mut v = verts;
                let mut n = [c2v { x: 5.0, y: -5.0 }; 8];
                c2Norms(s, v.as_mut_ptr(), n.as_mut_ptr(), count as c_int);
                (v.to_vec(), n.to_vec())
            });
            // reversed (CW) winding — row 29
            let mut rev = verts;
            rev[..count].reverse();
            a.check(format!("cw count={count} #{i}"), |s| {
                let mut v = rev;
                let mut n = [c2v { x: 5.0, y: -5.0 }; 8];
                c2Norms(s, v.as_mut_ptr(), n.as_mut_ptr(), count as c_int);
                (v.to_vec(), n.to_vec())
            });
        }
    }

    // row 27: box from c2BBVerts
    for i in 0..N {
        let bb = rng.aabb();
        a.check(format!("box #{i} {bb:?}"), |s| {
            let mut bb = bb;
            let mut v = [c2v::default(); 8];
            c2BBVerts(s, v.as_mut_ptr(), &mut bb);
            let mut n = [c2v { x: 5.0, y: -5.0 }; 8];
            c2Norms(s, v.as_mut_ptr(), n.as_mut_ptr(), 4);
            (v.to_vec(), n.to_vec())
        });
    }

    // row 30: duplicate consecutive verts ⇒ NaN normals
    for i in 0..N {
        let mut verts = rng.convex_poly_verts(4);
        let k = rng.below(4) as usize;
        verts[(k + 1) % 4] = verts[k];
        a.check(format!("dup #{i}"), |s| {
            let mut v = verts;
            let mut n = [c2v { x: 5.0, y: -5.0 }; 8];
            c2Norms(s, v.as_mut_ptr(), n.as_mut_ptr(), 4);
            (v.to_vec(), n.to_vec())
        });
    }

    // non-finite verts
    for i in 0..N {
        let mut verts = [c2v::default(); 8];
        for v in verts.iter_mut() {
            *v = rng.special_vec();
        }
        let count = 1 + rng.below(8) as c_int;
        a.check(format!("special #{i} count={count}"), |s| {
            let mut v = verts;
            let mut n = [c2v { x: 5.0, y: -5.0 }; 8];
            c2Norms(s, v.as_mut_ptr(), n.as_mut_ptr(), count);
            (v.to_vec(), n.to_vec())
        });
    }
    a.finish();
}

// --- row 31 --------------------------------------------------------------
#[test]
fn cfg_planeat() {
    let mut a = DiffAccum::new("cfg_planeat");
    let mut rng = Rng::new(0x6eed_0003);
    for count in 1..=8usize {
        for i in 0..N / 4 {
            let verts = rng.convex_poly_verts(count);
            let poly = make_poly(&verts, count as c_int);
            for idx in 0..count as c_int {
                a.check(format!("count={count} #{i} i={idx}"), |s| {
                    c2PlaneAt(s, &poly, idx)
                });
            }
        }
    }
    // non-finite polys
    for i in 0..N {
        let mut poly = c2Poly {
            count: 8,
            verts: [c2v::default(); 8],
            norms: [c2v::default(); 8],
        };
        for k in 0..8 {
            poly.verts[k] = rng.special_vec();
            poly.norms[k] = rng.special_vec();
        }
        for idx in 0..8 {
            a.check(format!("special #{i} i={idx}"), |s| c2PlaneAt(s, &poly, idx));
        }
    }
    a.finish();
}

// --- rows 32..34 ---------------------------------------------------------
#[test]
fn cfg_makeproxy() {
    let mut a = DiffAccum::new("cfg_makeproxy");
    let mut rng = Rng::new(0x6eed_0004);
    for i in 0..N {
        let c = rng.circle();
        a.check(format!("circle #{i} {c:?}"), |s| {
            let mut p = sentinel_proxy();
            let c = c;
            c2MakeProxy(s, &c as *const c2Circle as *const c_void, C2_TYPE_CIRCLE, &mut p);
            p
        });
        let bb = rng.aabb();
        a.check(format!("aabb #{i} {bb:?}"), |s| {
            let mut p = sentinel_proxy();
            let bb = bb;
            c2MakeProxy(s, &bb as *const c2AABB as *const c_void, C2_TYPE_AABB, &mut p);
            p
        });
        let cap = rng.capsule();
        a.check(format!("capsule #{i} {cap:?}"), |s| {
            let mut p = sentinel_proxy();
            let cap = cap;
            c2MakeProxy(
                s,
                &cap as *const c2Capsule as *const c_void,
                C2_TYPE_CAPSULE,
                &mut p,
            );
            p
        });
    }
    // non-finite shape data
    for i in 0..N {
        let c = c2Circle {
            p: rng.special_vec(),
            r: rng.special(),
        };
        a.check(format!("circle special #{i}"), |s| {
            let mut p = sentinel_proxy();
            let c = c;
            c2MakeProxy(s, &c as *const c2Circle as *const c_void, C2_TYPE_CIRCLE, &mut p);
            p
        });
        let bb = c2AABB {
            min: rng.special_vec(),
            max: rng.special_vec(),
        };
        a.check(format!("aabb special #{i}"), |s| {
            let mut p = sentinel_proxy();
            let bb = bb;
            c2MakeProxy(s, &bb as *const c2AABB as *const c_void, C2_TYPE_AABB, &mut p);
            p
        });
        let cap = c2Capsule {
            a: rng.special_vec(),
            b: rng.special_vec(),
            r: rng.special(),
        };
        a.check(format!("capsule special #{i}"), |s| {
            let mut p = sentinel_proxy();
            let cap = cap;
            c2MakeProxy(
                s,
                &cap as *const c2Capsule as *const c_void,
                C2_TYPE_CAPSULE,
                &mut p,
            );
            p
        });
    }
    a.finish();
}

// --- rows 35, 36 ---------------------------------------------------------
#[test]
fn cfg_support() {
    let mut a = DiffAccum::new("cfg_support");
    let mut rng = Rng::new(0x6eed_0005);
    for &count in &[1i32, 2, 3, 4, 5, 6, 7, 8] {
        for i in 0..N / 2 {
            let mut verts = [c2v::default(); 8];
            for v in verts.iter_mut() {
                *v = rng.vec();
            }
            let d = rng.vec();
            a.check(format!("count={count} #{i}"), |s| {
                c2Support(s, verts.as_ptr(), count, d)
            });
        }
    }
    // ties: all verts identical ⇒ strict `>` keeps index 0
    for i in 0..N {
        let v0 = rng.vec();
        let verts = [v0; 8];
        let d = rng.vec();
        a.check(format!("tie #{i}"), |s| c2Support(s, verts.as_ptr(), 8, d));
    }
    // non-finite verts / directions
    for i in 0..N {
        let mut verts = [c2v::default(); 8];
        for v in verts.iter_mut() {
            *v = rng.special_vec();
        }
        let d = rng.special_vec();
        let count = 1 + rng.below(8) as c_int;
        a.check(format!("special #{i} count={count}"), |s| {
            c2Support(s, verts.as_ptr(), count, d)
        });
    }
    a.finish();
}
