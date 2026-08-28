//! Level 2: shape-vs-shape predicates and the `f2` dispatcher.

mod harness;

use harness::*;
use std::ffi::c_void;

fn f32_pool() -> Vec<f32> {
    let mut v: Vec<f32> = EDGE_F32.to_vec();
    let mut rng = Rng::new(0xC2_1000);
    for _ in 0..60 {
        v.push(rng.next_f32_in(100.0));
    }
    for _ in 0..40 {
        v.push(rng.next_f32_bits());
    }
    v
}

fn circles() -> Vec<C2Circle> {
    let pool = f32_pool();
    let mut rng = Rng::new(0xC2_1001);
    let mut out = Vec::new();
    // Structured: sweep each field over the edge set.
    for &x in EDGE_F32 {
        out.push(C2Circle {
            p: C2v { x, y: 1.0 },
            r: 2.0,
        });
        out.push(C2Circle {
            p: C2v { x: 1.0, y: x },
            r: 2.0,
        });
        out.push(C2Circle {
            p: C2v { x: 1.0, y: 2.0 },
            r: x,
        });
    }
    for _ in 0..600 {
        out.push(C2Circle {
            p: C2v {
                x: pool[(rng.next_u32() as usize) % pool.len()],
                y: pool[(rng.next_u32() as usize) % pool.len()],
            },
            r: pool[(rng.next_u32() as usize) % pool.len()],
        });
    }
    out
}

fn aabbs() -> Vec<C2Aabb> {
    let pool = f32_pool();
    let mut rng = Rng::new(0xC2_1002);
    let mut out = Vec::new();
    for &x in EDGE_F32 {
        out.push(C2Aabb {
            min: C2v { x, y: -1.0 },
            max: C2v { x: 1.0, y: 1.0 },
        });
        out.push(C2Aabb {
            min: C2v { x: -1.0, y: -1.0 },
            max: C2v { x, y: 1.0 },
        });
        out.push(C2Aabb {
            min: C2v { x: -1.0, y: x },
            max: C2v { x: 1.0, y: 1.0 },
        });
        out.push(C2Aabb {
            min: C2v { x: -1.0, y: -1.0 },
            max: C2v { x: 1.0, y: x },
        });
    }
    for _ in 0..600 {
        out.push(C2Aabb {
            min: C2v {
                x: pool[(rng.next_u32() as usize) % pool.len()],
                y: pool[(rng.next_u32() as usize) % pool.len()],
            },
            max: C2v {
                x: pool[(rng.next_u32() as usize) % pool.len()],
                y: pool[(rng.next_u32() as usize) % pool.len()],
            },
        });
    }
    out
}

#[test]
fn circle_to_circle_matches() {
    let i = impls();
    let (c, r) = i.sym::<FnCirCir>("c2CircletoCircle");
    let cs = circles();
    let mut rng = Rng::new(1);
    let n = cs.len().min(120);
    for a in 0..n {
        for b in 0..n {
            let (a, b) = (cs[a], cs[b]);
            assert_eq!(
                unsafe { c(a, b) },
                unsafe { r(a, b) },
                "c2CircletoCircle({a:?},{b:?})"
            );
        }
    }
    for _ in 0..300_000 {
        let a = cs[(rng.next_u32() as usize) % cs.len()];
        let b = cs[(rng.next_u32() as usize) % cs.len()];
        assert_eq!(
            unsafe { c(a, b) },
            unsafe { r(a, b) },
            "c2CircletoCircle({a:?},{b:?})"
        );
    }
}

#[test]
fn circle_to_aabb_matches() {
    let i = impls();
    let (c, r) = i.sym::<FnCirAabb>("c2CircletoAABB");
    let cs = circles();
    let bs = aabbs();
    let mut rng = Rng::new(2);
    let n = cs.len().min(100);
    let m = bs.len().min(100);
    for a in 0..n {
        for b in 0..m {
            let (a, b) = (cs[a], bs[b]);
            assert_eq!(
                unsafe { c(a, b) },
                unsafe { r(a, b) },
                "c2CircletoAABB({a:?},{b:?})"
            );
        }
    }
    for _ in 0..300_000 {
        let a = cs[(rng.next_u32() as usize) % cs.len()];
        let b = bs[(rng.next_u32() as usize) % bs.len()];
        assert_eq!(
            unsafe { c(a, b) },
            unsafe { r(a, b) },
            "c2CircletoAABB({a:?},{b:?})"
        );
    }
}

#[test]
fn aabb_to_aabb_matches() {
    let i = impls();
    let (c, r) = i.sym::<FnAabbAabb>("c2AABBtoAABB");
    let bs = aabbs();
    let mut rng = Rng::new(3);
    let n = bs.len().min(120);
    for a in 0..n {
        for b in 0..n {
            let (a, b) = (bs[a], bs[b]);
            assert_eq!(
                unsafe { c(a, b) },
                unsafe { r(a, b) },
                "c2AABBtoAABB({a:?},{b:?})"
            );
        }
    }
    for _ in 0..300_000 {
        let a = bs[(rng.next_u32() as usize) % bs.len()];
        let b = bs[(rng.next_u32() as usize) % bs.len()];
        assert_eq!(
            unsafe { c(a, b) },
            unsafe { r(a, b) },
            "c2AABBtoAABB({a:?},{b:?})"
        );
    }
}

/// `f2` dispatches on the two type tags; exercise valid tags *and* the
/// out-of-range tags that fall into the `default:` arms.
#[test]
fn f2_dispatch_matches() {
    let i = impls();
    let (c, r) = i.sym::<FnF2>("f2");
    let cs = circles();
    let bs = aabbs();
    let mut rng = Rng::new(4);

    // A 16-byte buffer can hold either shape; feed the same bytes to both.
    let tags: [i32; 6] = [0, 1, 2, -1, 7, i32::MAX];

    for _ in 0..300_000 {
        let ci = cs[(rng.next_u32() as usize) % cs.len()];
        let bi = bs[(rng.next_u32() as usize) % bs.len()];
        let ta = tags[(rng.next_u32() as usize) % tags.len()];
        let tb = tags[(rng.next_u32() as usize) % tags.len()];

        // Buffers big enough for the largest shape so every cast is in-bounds.
        let mut abuf = [0f32; 4];
        let mut bbuf = [0f32; 4];
        // Fill A as a circle, and (overlapping) as an AABB-compatible 4 floats.
        abuf[0] = ci.p.x;
        abuf[1] = ci.p.y;
        abuf[2] = ci.r;
        abuf[3] = bi.max.y;
        bbuf[0] = bi.min.x;
        bbuf[1] = bi.min.y;
        bbuf[2] = bi.max.x;
        bbuf[3] = bi.max.y;

        let ap = abuf.as_ptr() as *const c_void;
        let bp = bbuf.as_ptr() as *const c_void;
        let x = unsafe { c(ap, ta, bp, tb) };
        let y = unsafe { r(ap, ta, bp, tb) };
        assert_eq!(x, y, "f2({abuf:?},{ta},{bbuf:?},{tb})");
    }

    // Deterministic pass over all tag pairs with fixed payloads.
    for &ta in tags.iter() {
        for &tb in tags.iter() {
            let abuf = [1.0f32, 2.0, 3.0, 4.0];
            let bbuf = [-1.0f32, -2.0, 5.0, 6.0];
            let ap = abuf.as_ptr() as *const c_void;
            let bp = bbuf.as_ptr() as *const c_void;
            assert_eq!(
                unsafe { c(ap, ta, bp, tb) },
                unsafe { r(ap, ta, bp, tb) },
                "f2 tags ({ta},{tb})"
            );
        }
    }
}
