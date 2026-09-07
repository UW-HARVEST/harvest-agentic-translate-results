//! Phase B — CONFIGS.md rows 15-20: proxy construction.
//!
//! `c2MakeProxy` / `c2BBVerts` write through an out-pointer. Buffers are
//! pre-zeroed identically on both sides so the out-of-range-type case (where
//! the C `switch` has no `default:` and writes nothing) is deterministic and
//! comparable instead of reading indeterminate stack memory.

mod common;
use common::*;

const N: usize = 5_000;

/// Rows 15-16 — `c2BBVerts` on normal, degenerate and inverted AABBs.
#[test]
fn row15_16_bbverts() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 15);

    let run = |ctx: &str, bb: c2AABB| {
        let mut co = [c2v::default(); 8];
        let mut ro = [c2v::default(); 8];
        let mut bb_c = bb;
        let mut bb_r = bb;
        unsafe { (c.c2BBVerts)(co.as_mut_ptr(), &mut bb_c) };
        unsafe { (r.c2BBVerts)(ro.as_mut_ptr(), &mut bb_r) };
        for i in 0..8 {
            eq_v(&format!("{ctx} out[{i}]"), co[i], ro[i]);
        }
        // The input must not be modified by either side.
        eq_bytes(&format!("{ctx} input untouched"), &bb_c, &bb_r);
    };

    // Row 15: normal boxes.
    for i in 0..N {
        let a = g.v();
        let b = g.v();
        let bb = c2AABB {
            min: c2v {
                x: a.x.min(b.x),
                y: a.y.min(b.y),
            },
            max: c2v {
                x: a.x.max(b.x),
                y: a.y.max(b.y),
            },
        };
        run(&format!("row15 normal #{i}"), bb);
    }

    // Row 16: degenerate (min == max), inverted (min > max), and wild.
    for i in 0..N {
        let p = g.v();
        run(&format!("row16 degenerate #{i}"), c2AABB { min: p, max: p });
        let a = g.v();
        let b = g.v();
        run(
            &format!("row16 inverted #{i}"),
            c2AABB {
                min: c2v {
                    x: a.x.max(b.x),
                    y: a.y.max(b.y),
                },
                max: c2v {
                    x: a.x.min(b.x),
                    y: a.y.min(b.y),
                },
            },
        );
        run(
            &format!("row16 wild #{i}"),
            c2AABB {
                min: g.wild_v(),
                max: g.wild_v(),
            },
        );
    }
}

/// Row 17 — `c2MakeProxy` with `C2_TYPE_CIRCLE`: `radius=r`, `count=1`.
#[test]
fn row17_makeproxy_circle() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 17);
    for i in 0..N {
        let shape = match g.below(4) {
            0 => c2Circle {
                p: g.wild_v(),
                r: g.wild(),
            },
            _ => g.circle(),
        };
        let buf = ShapeBuf::from_circle(shape);
        let mut cp = c2Proxy::default();
        let mut rp = c2Proxy::default();
        unsafe { (c.c2MakeProxy)(buf.as_ptr(), C2_TYPE_CIRCLE, &mut cp) };
        unsafe { (r.c2MakeProxy)(buf.as_ptr(), C2_TYPE_CIRCLE, &mut rp) };
        eq_proxy(&format!("row17 #{i} {shape:?}"), &cp, &rp);
        eq_int("row17 count is 1", cp.count, 1);
    }
}

/// Row 18 — `c2MakeProxy` with `C2_TYPE_CAPSULE`: `radius=r`, `count=2`,
/// including degenerate point capsules.
#[test]
fn row18_makeproxy_capsule() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 18);
    for i in 0..N {
        let shape = match g.below(4) {
            0 => c2Capsule {
                a: g.wild_v(),
                b: g.wild_v(),
                r: g.wild(),
            },
            _ => g.capsule(),
        };
        let buf = ShapeBuf::from_capsule(shape);
        let mut cp = c2Proxy::default();
        let mut rp = c2Proxy::default();
        unsafe { (c.c2MakeProxy)(buf.as_ptr(), C2_TYPE_CAPSULE, &mut cp) };
        unsafe { (r.c2MakeProxy)(buf.as_ptr(), C2_TYPE_CAPSULE, &mut rp) };
        eq_proxy(&format!("row18 #{i} {shape:?}"), &cp, &rp);
        eq_int("row18 count is 2", cp.count, 2);
    }
}

/// Row 19 — `c2MakeProxy` with `C2_TYPE_AABB`: `radius=0`, `count=4`, verts
/// filled by `c2BBVerts`.
#[test]
fn row19_makeproxy_aabb() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 19);
    for i in 0..N {
        let shape = match g.below(4) {
            0 => c2AABB {
                min: g.wild_v(),
                max: g.wild_v(),
            },
            _ => g.aabb(),
        };
        let buf = ShapeBuf::from_aabb(shape);
        let mut cp = c2Proxy::default();
        let mut rp = c2Proxy::default();
        unsafe { (c.c2MakeProxy)(buf.as_ptr(), C2_TYPE_AABB, &mut cp) };
        unsafe { (r.c2MakeProxy)(buf.as_ptr(), C2_TYPE_AABB, &mut rp) };
        eq_proxy(&format!("row19 #{i} {shape:?}"), &cp, &rp);
        eq_int("row19 count is 4", cp.count, 4);
        eq_f32_bits("row19 radius is 0", cp.radius, 0.0);
    }
}

/// Row 20 (ERRORS.md row 1) — out-of-range `type`: the C `switch` has no
/// `default:`, so `*p` is left completely untouched. With a pre-zeroed
/// destination both sides must therefore still be all-zero.
#[test]
fn row20_makeproxy_invalid_type() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 20);
    for &bad in &BAD_TYPES {
        for i in 0..64 {
            let buf = ShapeBuf::from_capsule(g.capsule());
            let mut cp = c2Proxy::default();
            let mut rp = c2Proxy::default();
            unsafe { (c.c2MakeProxy)(buf.as_ptr(), bad, &mut cp) };
            unsafe { (r.c2MakeProxy)(buf.as_ptr(), bad, &mut rp) };
            eq_proxy(&format!("row20 type={bad} #{i}"), &cp, &rp);
            eq_bytes(&format!("row20 type={bad} #{i} untouched"), &cp, &rp);
            // Neither side wrote anything.
            assert_eq!(cp.count, 0, "row20: C wrote count for type={bad}");
            assert_eq!(rp.count, 0, "row20: Rust wrote count for type={bad}");
        }
    }

    // Also: a pre-*filled* destination must survive unchanged on both sides.
    for &bad in &BAD_TYPES {
        let mut cp = c2Proxy {
            radius: 7.5,
            count: 3,
            verts: [c2v { x: 1.0, y: 2.0 }; 8],
        };
        let mut rp = cp;
        let buf = ShapeBuf::from_circle(g.circle());
        unsafe { (c.c2MakeProxy)(buf.as_ptr(), bad, &mut cp) };
        unsafe { (r.c2MakeProxy)(buf.as_ptr(), bad, &mut rp) };
        eq_proxy(&format!("row20 prefilled type={bad}"), &cp, &rp);
        eq_f32_bits("row20 prefilled radius kept", cp.radius, 7.5);
        eq_int("row20 prefilled count kept", cp.count, 3);
    }
}
