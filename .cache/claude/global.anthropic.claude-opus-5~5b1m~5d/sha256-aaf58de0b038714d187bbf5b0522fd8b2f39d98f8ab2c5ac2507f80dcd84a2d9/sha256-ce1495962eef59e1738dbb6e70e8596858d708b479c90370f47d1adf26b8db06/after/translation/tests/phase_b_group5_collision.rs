//! Phase B, Group 5 — CONFIGS.md rows C78..C87 (boolean collision routines and
//! the `capsule()` entry point declared in `include/lib.h`).

#![allow(non_snake_case)]

mod common;

use common::*;
use std::ffi::{c_int, c_void};

const N: usize = 4000;

#[repr(C, align(16))]
#[derive(Copy, Clone)]
struct ShapeBuf {
    bytes: [u8; 32],
}

impl ShapeBuf {
    fn of<T: Copy>(v: &T) -> ShapeBuf {
        let mut b = ShapeBuf { bytes: [0; 32] };
        let src = raw(v);
        b.bytes[..src.len()].copy_from_slice(&src);
        b
    }
    fn ptr(&self) -> *const c_void {
        self.bytes.as_ptr() as *const c_void
    }
}

// ---------------------------------------------------------------------------
// C78 — c2AABBtoAABB
// ---------------------------------------------------------------------------

#[test]
fn c78_c2AABBtoAABB() {
    let p = load();
    let c: FnAABBtoAABB = p.c.sym("c2AABBtoAABB");
    let r: FnAABBtoAABB = p.rs.sym("c2AABBtoAABB");
    let mut rng = Rng::new(0x78);
    let mut hits = [0usize; 2];
    unsafe {
        for _ in 0..(N * 4) {
            let (a, b) = (rng.aabb(), rng.aabb());
            let (cv, rv) = (c(a, b), r(a, b));
            assert_eq!(
                cv, rv,
                "c2AABBtoAABB A[{} {}] B[{} {}] C={cv} Rust={rv}",
                v_hex(&a.min),
                v_hex(&a.max),
                v_hex(&b.min),
                v_hex(&b.max)
            );
            hits[(cv != 0) as usize] += 1;
        }
        // separated on x only / y only / both / touching / nested
        let base = c2AABB {
            min: c2v { x: 0.0, y: 0.0 },
            max: c2v { x: 10.0, y: 10.0 },
        };
        for (dx, dy) in [
            (0.0f32, 0.0f32),
            (10.0, 0.0),   // touching on x
            (10.0, 10.0),  // touching on both
            (10.001, 0.0), // separated on x
            (0.0, 10.001), // separated on y
            (-20.0, 0.0),
            (0.0, -20.0),
            (-20.0, -20.0),
            (5.0, 5.0),
        ] {
            let b = c2AABB {
                min: c2v { x: dx, y: dy },
                max: c2v { x: dx + 10.0, y: dy + 10.0 },
            };
            assert_eq!(c(base, b), r(base, b), "c2AABBtoAABB shifted ({dx},{dy})");
            assert_eq!(c(b, base), r(b, base), "c2AABBtoAABB shifted swapped");
            // nested
            let inner = c2AABB {
                min: c2v { x: dx + 2.0, y: dy + 2.0 },
                max: c2v { x: dx + 3.0, y: dy + 3.0 },
            };
            assert_eq!(c(base, inner), r(base, inner), "c2AABBtoAABB nested");
        }
        // full special-value sweep on one corner at a time
        for sv in specials() {
            for slot in 0..8 {
                let mut a = base;
                let mut b = c2AABB {
                    min: c2v { x: 5.0, y: 5.0 },
                    max: c2v { x: 15.0, y: 15.0 },
                };
                match slot {
                    0 => a.min.x = sv,
                    1 => a.min.y = sv,
                    2 => a.max.x = sv,
                    3 => a.max.y = sv,
                    4 => b.min.x = sv,
                    5 => b.min.y = sv,
                    6 => b.max.x = sv,
                    _ => b.max.y = sv,
                }
                assert_eq!(
                    c(a, b),
                    r(a, b),
                    "c2AABBtoAABB special slot={slot} v={}",
                    f32_hex(sv)
                );
            }
        }
    }
    assert!(hits[0] > 0 && hits[1] > 0, "only one outcome seen: {hits:?}");
}

// ---------------------------------------------------------------------------
// C79 — c2CircletoCircle
// ---------------------------------------------------------------------------

#[test]
fn c79_c2CircletoCircle() {
    let p = load();
    let c: FnCircletoCircle = p.c.sym("c2CircletoCircle");
    let r: FnCircletoCircle = p.rs.sym("c2CircletoCircle");
    let mut rng = Rng::new(0x79);
    let mut hits = [0usize; 2];
    unsafe {
        for _ in 0..(N * 4) {
            let (a, b) = (rng.circle(), rng.circle());
            let (cv, rv) = (c(a, b), r(a, b));
            assert_eq!(cv, rv, "c2CircletoCircle random");
            hits[(cv != 0) as usize] += 1;
        }
        // exact touching: |d| == rA + rB
        for _ in 0..N {
            let rA = (rng.below(20) + 1) as f32;
            let rB = (rng.below(20) + 1) as f32;
            let y = rng.coord();
            let a = c2Circle { p: c2v { x: 0.0, y }, r: rA };
            for dx in [rA + rB, rA + rB - 0.001, rA + rB + 0.001, 0.0] {
                let b = c2Circle { p: c2v { x: dx, y }, r: rB };
                assert_eq!(c(a, b), r(a, b), "c2CircletoCircle dx={dx}");
            }
        }
        // r == 0 / r < 0 / concentric / specials
        for sv in specials() {
            for slot in 0..6 {
                let mut a = c2Circle { p: c2v { x: 1.0, y: 2.0 }, r: 3.0 };
                let mut b = c2Circle { p: c2v { x: 4.0, y: 6.0 }, r: 2.0 };
                match slot {
                    0 => a.p.x = sv,
                    1 => a.p.y = sv,
                    2 => a.r = sv,
                    3 => b.p.x = sv,
                    4 => b.p.y = sv,
                    _ => b.r = sv,
                }
                assert_eq!(
                    c(a, b),
                    r(a, b),
                    "c2CircletoCircle special slot={slot} v={}",
                    f32_hex(sv)
                );
            }
        }
    }
    assert!(hits[0] > 0 && hits[1] > 0, "only one outcome seen: {hits:?}");
}

// ---------------------------------------------------------------------------
// C80 — c2CircletoAABB (all Voronoi regions)
// ---------------------------------------------------------------------------

#[test]
fn c80_c2CircletoAABB() {
    let p = load();
    let c: FnCircletoAABB = p.c.sym("c2CircletoAABB");
    let r: FnCircletoAABB = p.rs.sym("c2CircletoAABB");
    let mut rng = Rng::new(0x80);
    let mut hits = [0usize; 2];
    unsafe {
        for _ in 0..(N * 4) {
            let (a, b) = (rng.circle(), rng.aabb());
            let (cv, rv) = (c(a, b), r(a, b));
            assert_eq!(cv, rv, "c2CircletoAABB random");
            hits[(cv != 0) as usize] += 1;
        }
        // 3x3 grid of Voronoi regions around a fixed box (centre + 4 edges + 4 corners)
        let bb = c2AABB {
            min: c2v { x: -10.0, y: -10.0 },
            max: c2v { x: 10.0, y: 10.0 },
        };
        for gy in -1i32..=1 {
            for gx in -1i32..=1 {
                for d in [0.0f32, 4.9999, 5.0, 5.0001, 20.0] {
                    let px = 10.0 * gx as f32 + d * gx as f32;
                    let py = 10.0 * gy as f32 + d * gy as f32;
                    for rad in [0.0f32, -0.0, -3.0, 5.0, 5.0001, 4.9999] {
                        let a = c2Circle { p: c2v { x: px, y: py }, r: rad };
                        assert_eq!(
                            c(a, bb),
                            r(a, bb),
                            "c2CircletoAABB region({gx},{gy}) d={d} r={rad}"
                        );
                    }
                }
            }
        }
        // exactly on an edge / corner
        for pt in [
            c2v { x: -10.0, y: 0.0 },
            c2v { x: 10.0, y: 0.0 },
            c2v { x: 0.0, y: -10.0 },
            c2v { x: 0.0, y: 10.0 },
            c2v { x: -10.0, y: -10.0 },
            c2v { x: 10.0, y: 10.0 },
        ] {
            for rad in [0.0f32, 0.0001, 1.0] {
                let a = c2Circle { p: pt, r: rad };
                assert_eq!(c(a, bb), r(a, bb), "c2CircletoAABB on boundary");
            }
        }
        // zero-area and inverted AABB
        for bb2 in [
            c2AABB { min: c2v { x: 3.0, y: 3.0 }, max: c2v { x: 3.0, y: 3.0 } },
            c2AABB { min: c2v { x: 10.0, y: 10.0 }, max: c2v { x: -10.0, y: -10.0 } },
        ] {
            for _ in 0..N {
                let a = rng.circle();
                assert_eq!(c(a, bb2), r(a, bb2), "c2CircletoAABB degenerate box");
            }
        }
        // full special sweep
        for sv in specials() {
            for slot in 0..7 {
                let mut a = c2Circle { p: c2v { x: 1.0, y: 2.0 }, r: 3.0 };
                let mut b = bb;
                match slot {
                    0 => a.p.x = sv,
                    1 => a.p.y = sv,
                    2 => a.r = sv,
                    3 => b.min.x = sv,
                    4 => b.min.y = sv,
                    5 => b.max.x = sv,
                    _ => b.max.y = sv,
                }
                assert_eq!(
                    c(a, b),
                    r(a, b),
                    "c2CircletoAABB special slot={slot} v={}",
                    f32_hex(sv)
                );
            }
        }
    }
    assert!(hits[0] > 0 && hits[1] > 0, "only one outcome seen: {hits:?}");
}

// ---------------------------------------------------------------------------
// C81 — c2CircletoCapsule (all three da/db arms)
// ---------------------------------------------------------------------------

#[test]
fn c81_c2CircletoCapsule() {
    let p = load();
    let c: FnCircletoCapsule = p.c.sym("c2CircletoCapsule");
    let r: FnCircletoCapsule = p.rs.sym("c2CircletoCapsule");
    let mut rng = Rng::new(0x81);
    let mut hits = [0usize; 2];
    let mut arms = [0usize; 3];
    unsafe {
        for _ in 0..(N * 4) {
            let (a, b) = (rng.circle(), rng.capsule());
            let (cv, rv) = (c(a, b), r(a, b));
            assert_eq!(cv, rv, "c2CircletoCapsule random");
            hits[(cv != 0) as usize] += 1;
            // arm accounting mirroring the C's da/db tests
            let n = c2v { x: b.b.x - b.a.x, y: b.b.y - b.a.y };
            let ap = c2v { x: a.p.x - b.a.x, y: a.p.y - b.a.y };
            let da = ap.y * n.y + ap.x * n.x;
            if da < 0.0 {
                arms[0] += 1;
            } else {
                let bp = c2v { x: a.p.x - b.b.x, y: a.p.y - b.b.y };
                let db = bp.y * n.y + bp.x * n.x;
                if db < 0.0 {
                    arms[1] += 1;
                } else {
                    arms[2] += 1;
                }
            }
        }
        // deliberately drive each arm along a fixed segment
        let seg = c2Capsule {
            a: c2v { x: 0.0, y: 0.0 },
            b: c2v { x: 20.0, y: 0.0 },
            r: 3.0,
        };
        for t in [-5.0f32, -0.0001, 0.0, 0.0001, 10.0, 19.9999, 20.0, 20.0001, 25.0] {
            for off in [0.0f32, 2.9999, 3.0, 3.0001, -3.0] {
                for rad in [0.0f32, -0.0, -1.0, 1.0] {
                    let a = c2Circle { p: c2v { x: t, y: off }, r: rad };
                    assert_eq!(c(a, seg), r(a, seg), "c2CircletoCapsule t={t} off={off} r={rad}");
                }
            }
        }
        // C81/E57: zero-length capsule (a == b) -> n == {0,0}
        for _ in 0..N {
            let z = rng.v();
            let deg = c2Capsule { a: z, b: z, r: rng.radius() };
            let a = rng.circle();
            assert_eq!(c(a, deg), r(a, deg), "c2CircletoCapsule a==b");
        }
        // full special sweep
        for sv in specials() {
            for slot in 0..8 {
                let mut a = c2Circle { p: c2v { x: 5.0, y: 1.0 }, r: 2.0 };
                let mut b = seg;
                match slot {
                    0 => a.p.x = sv,
                    1 => a.p.y = sv,
                    2 => a.r = sv,
                    3 => b.a.x = sv,
                    4 => b.a.y = sv,
                    5 => b.b.x = sv,
                    6 => b.b.y = sv,
                    _ => b.r = sv,
                }
                assert_eq!(
                    c(a, b),
                    r(a, b),
                    "c2CircletoCapsule special slot={slot} v={}",
                    f32_hex(sv)
                );
            }
        }
    }
    assert!(hits[0] > 0 && hits[1] > 0, "only one outcome seen: {hits:?}");
    assert!(arms.iter().all(|&n| n > 0), "arm coverage incomplete: {arms:?}");
    eprintln!("c2CircletoCapsule arms (da<0, db<0, else) = {arms:?}");
}

// ---------------------------------------------------------------------------
// C82 — c2AABBtoCapsule
// ---------------------------------------------------------------------------

#[test]
fn c82_c2AABBtoCapsule() {
    let p = load();
    let c: FnAABBtoCapsule = p.c.sym("c2AABBtoCapsule");
    let r: FnAABBtoCapsule = p.rs.sym("c2AABBtoCapsule");
    let mut rng = Rng::new(0x82);
    let mut hits = [0usize; 2];
    unsafe {
        for _ in 0..(N * 4) {
            let (a, b) = (rng.aabb(), rng.capsule());
            let (cv, rv) = (c(a, b), r(a, b));
            assert_eq!(cv, rv, "c2AABBtoCapsule random");
            hits[(cv != 0) as usize] += 1;
        }
        let bb = c2AABB {
            min: c2v { x: -10.0, y: -10.0 },
            max: c2v { x: 10.0, y: 10.0 },
        };
        for dx in [-40.0f32, -13.0, -10.0, -5.0, 0.0, 5.0, 10.0, 13.0, 40.0] {
            for rad in [0.0f32, -0.0, -2.0, 3.0] {
                let k = c2Capsule {
                    a: c2v { x: dx, y: -20.0 },
                    b: c2v { x: dx, y: 20.0 },
                    r: rad,
                };
                assert_eq!(c(bb, k), r(bb, k), "c2AABBtoCapsule vertical dx={dx} r={rad}");
                // zero-length
                let z = c2Capsule { a: k.a, b: k.a, r: rad };
                assert_eq!(c(bb, z), r(bb, z), "c2AABBtoCapsule zero-length dx={dx}");
            }
        }
        // zero-area AABB
        for _ in 0..N {
            let q = rng.v();
            let flat = c2AABB { min: q, max: q };
            let k = rng.capsule();
            assert_eq!(c(flat, k), r(flat, k), "c2AABBtoCapsule flat box");
        }
        // full special sweep
        for sv in specials() {
            for slot in 0..9 {
                let mut a = bb;
                let mut b = c2Capsule {
                    a: c2v { x: 2.0, y: 3.0 },
                    b: c2v { x: 30.0, y: 4.0 },
                    r: 1.0,
                };
                match slot {
                    0 => a.min.x = sv,
                    1 => a.min.y = sv,
                    2 => a.max.x = sv,
                    3 => a.max.y = sv,
                    4 => b.a.x = sv,
                    5 => b.a.y = sv,
                    6 => b.b.x = sv,
                    7 => b.b.y = sv,
                    _ => b.r = sv,
                }
                assert_eq!(
                    c(a, b),
                    r(a, b),
                    "c2AABBtoCapsule special slot={slot} v={}",
                    f32_hex(sv)
                );
            }
        }
    }
    assert!(hits[0] > 0 && hits[1] > 0, "only one outcome seen: {hits:?}");
}

// ---------------------------------------------------------------------------
// C83 — c2CapsuletoCapsule
// ---------------------------------------------------------------------------

#[test]
fn c83_c2CapsuletoCapsule() {
    let p = load();
    let c: FnCapsuletoCapsule = p.c.sym("c2CapsuletoCapsule");
    let r: FnCapsuletoCapsule = p.rs.sym("c2CapsuletoCapsule");
    let mut rng = Rng::new(0x83);
    let mut hits = [0usize; 2];
    unsafe {
        for _ in 0..(N * 6) {
            let (a, b) = (rng.capsule(), rng.capsule());
            let (cv, rv) = (c(a, b), r(a, b));
            assert_eq!(cv, rv, "c2CapsuletoCapsule random");
            hits[(cv != 0) as usize] += 1;
        }
        let base = c2Capsule {
            a: c2v { x: -10.0, y: 0.0 },
            b: c2v { x: 10.0, y: 0.0 },
            r: 2.0,
        };
        // crossing / parallel / collinear / touching / nested
        for dy in [0.0f32, 3.9999, 4.0, 4.0001, 20.0] {
            let parallel = c2Capsule {
                a: c2v { x: -10.0, y: dy },
                b: c2v { x: 10.0, y: dy },
                r: 2.0,
            };
            assert_eq!(c(base, parallel), r(base, parallel), "parallel dy={dy}");
            let crossing = c2Capsule {
                a: c2v { x: 0.0, y: dy - 10.0 },
                b: c2v { x: 0.0, y: dy + 10.0 },
                r: 2.0,
            };
            assert_eq!(c(base, crossing), r(base, crossing), "crossing dy={dy}");
        }
        for dx in [0.0f32, 20.0, 23.9999, 24.0, 24.0001, 40.0] {
            let collinear = c2Capsule {
                a: c2v { x: dx - 10.0, y: 0.0 },
                b: c2v { x: dx + 10.0, y: 0.0 },
                r: 2.0,
            };
            assert_eq!(c(base, collinear), r(base, collinear), "collinear dx={dx}");
        }
        // zero-length on either / both sides
        for _ in 0..N {
            let z = rng.v();
            let deg = c2Capsule { a: z, b: z, r: rng.radius() };
            let k = rng.capsule();
            assert_eq!(c(deg, k), r(deg, k), "A zero-length");
            assert_eq!(c(k, deg), r(k, deg), "B zero-length");
            let z2 = rng.v();
            let deg2 = c2Capsule { a: z2, b: z2, r: rng.radius() };
            assert_eq!(c(deg, deg2), r(deg, deg2), "both zero-length");
        }
        // full special sweep
        for sv in specials() {
            for slot in 0..10 {
                let mut a = base;
                let mut b = c2Capsule {
                    a: c2v { x: 1.0, y: 5.0 },
                    b: c2v { x: 6.0, y: -5.0 },
                    r: 1.0,
                };
                match slot {
                    0 => a.a.x = sv,
                    1 => a.a.y = sv,
                    2 => a.b.x = sv,
                    3 => a.b.y = sv,
                    4 => a.r = sv,
                    5 => b.a.x = sv,
                    6 => b.a.y = sv,
                    7 => b.b.x = sv,
                    8 => b.b.y = sv,
                    _ => b.r = sv,
                }
                assert_eq!(
                    c(a, b),
                    r(a, b),
                    "c2CapsuletoCapsule special slot={slot} v={}",
                    f32_hex(sv)
                );
            }
        }
    }
    assert!(hits[0] > 0 && hits[1] > 0, "only one outcome seen: {hits:?}");
}

// ---------------------------------------------------------------------------
// C84 — c2Collided over the full valid 3x3 type cross-product
// ---------------------------------------------------------------------------

#[test]
fn c84_c2Collided_valid_cross_product() {
    let p = load();
    let c: FnCollided = p.c.sym("c2Collided");
    let r: FnCollided = p.rs.sym("c2Collided");
    let mut rng = Rng::new(0x84);
    let types = [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE];
    let mk = |rng: &mut Rng, t: c_int| -> ShapeBuf {
        match t {
            C2_TYPE_CIRCLE => ShapeBuf::of(&rng.circle()),
            C2_TYPE_AABB => ShapeBuf::of(&rng.aabb()),
            _ => ShapeBuf::of(&rng.capsule()),
        }
    };
    let mut hits = [[0usize; 2]; 9];
    unsafe {
        for (idx, (&ta, &tb)) in types
            .iter()
            .flat_map(|a| types.iter().map(move |b| (a, b)))
            .enumerate()
        {
            for _ in 0..(N * 4) {
                let a = mk(&mut rng, ta);
                let b = mk(&mut rng, tb);
                let cv = c(a.ptr(), ta, b.ptr(), tb);
                let rv = r(a.ptr(), ta, b.ptr(), tb);
                assert_eq!(
                    cv, rv,
                    "c2Collided(typeA={ta}, typeB={tb}) A={:02x?} B={:02x?} C={cv} Rust={rv}",
                    &a.bytes[..20],
                    &b.bytes[..20]
                );
                hits[idx][(cv != 0) as usize] += 1;
            }
        }
    }
    for (i, h) in hits.iter().enumerate() {
        assert!(
            h[0] > 0 && h[1] > 0,
            "c2Collided pair #{i} only ever produced one outcome: {h:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// C85..C87 — the public `capsule()` entry point
// ---------------------------------------------------------------------------

#[test]
fn c85_capsule_random() {
    let p = load();
    let c: FnCapsule = p.c.sym("capsule");
    let r: FnCapsule = p.rs.sym("capsule");
    let mut rng = Rng::new(0x85);
    let mut seen = [0usize; 8];
    unsafe {
        for _ in 0..(N * 20) {
            // Range chosen to straddle all three hard-coded shapes in the C.
            let min_x = rng.uniform(-120.0, 40.0);
            let min_y = rng.uniform(-80.0, 120.0);
            let max_x = rng.uniform(-120.0, 40.0);
            let max_y = rng.uniform(-80.0, 120.0);
            let rad = rng.uniform(0.0, 40.0);
            let cv = c(min_x, min_y, max_x, max_y, rad);
            let rv = r(min_x, min_y, max_x, max_y, rad);
            assert_eq!(
                cv, rv,
                "capsule({}, {}, {}, {}, {}) C={cv} Rust={rv}",
                f32_hex(min_x),
                f32_hex(min_y),
                f32_hex(max_x),
                f32_hex(max_y),
                f32_hex(rad)
            );
            if (0..8).contains(&cv) {
                seen[cv as usize] += 1;
            }
        }
    }
    eprintln!("capsule() result histogram = {seen:?}");
    let distinct = seen.iter().filter(|&&n| n > 0).count();
    assert!(
        distinct >= 5,
        "capsule() only produced {distinct} distinct results: {seen:?}"
    );
}

#[test]
fn c86_capsule_boundary_and_special_values() {
    let p = load();
    let c: FnCapsule = p.c.sym("capsule");
    let r: FnCapsule = p.rs.sym("capsule");
    unsafe {
        // one special value in each of the 5 argument slots
        for sv in specials() {
            for slot in 0..5 {
                let mut args = [-40.0f32, 40.0, -20.0, 100.0, 10.0];
                args[slot] = sv;
                let cv = c(args[0], args[1], args[2], args[3], args[4]);
                let rv = r(args[0], args[1], args[2], args[3], args[4]);
                assert_eq!(
                    cv, rv,
                    "capsule special slot={slot} v={} -> C={cv} Rust={rv}",
                    f32_hex(sv)
                );
            }
        }
        // all-special (5^3 over a reduced set to keep it quick), plus every
        // pairwise combination of specials in the first two slots
        for a in specials() {
            for b in specials() {
                let cv = c(a, b, a, b, 10.0);
                let rv = r(a, b, a, b, 10.0);
                assert_eq!(cv, rv, "capsule pair a={} b={}", f32_hex(a), f32_hex(b));
                let cv2 = c(-40.0, 40.0, a, b, b);
                let rv2 = r(-40.0, 40.0, a, b, b);
                assert_eq!(cv2, rv2, "capsule pair2 a={} b={}", f32_hex(a), f32_hex(b));
            }
        }
        // min == max (zero-length), r == 0, r < 0
        for (mx, my) in [(-40.0f32, 40.0f32), (-70.0, 0.0), (-25.0, -25.0), (0.0, 0.0)] {
            for rad in [0.0f32, -0.0, -1.0, -100.0, 1.0e30, f32::MIN_POSITIVE] {
                let cv = c(mx, my, mx, my, rad);
                let rv = r(mx, my, mx, my, rad);
                assert_eq!(cv, rv, "capsule degenerate ({mx},{my}) r={rad}");
            }
        }
    }
}

/// C87 — dense grid over the region occupied by the three hard-coded shapes, so
/// each of the three result bits is observed both set and clear.
#[test]
fn c87_capsule_dense_grid() {
    let p = load();
    let c: FnCapsule = p.c.sym("capsule");
    let r: FnCapsule = p.rs.sym("capsule");
    let mut bit_seen = [[false; 2]; 3];
    unsafe {
        let mut i = 0u32;
        for ax in -13..=4 {
            for ay in -9..=12 {
                for bx in -13..=4 {
                    for by in -9..=12 {
                        // keep the grid coarse but complete enough
                        i += 1;
                        if i % 7 != 0 {
                            continue;
                        }
                        let (min_x, min_y) = (ax as f32 * 10.0, ay as f32 * 10.0);
                        let (max_x, max_y) = (bx as f32 * 10.0, by as f32 * 10.0);
                        for rad in [0.0f32, 5.0, 15.0] {
                            let cv = c(min_x, min_y, max_x, max_y, rad);
                            let rv = r(min_x, min_y, max_x, max_y, rad);
                            assert_eq!(
                                cv, rv,
                                "capsule grid ({min_x},{min_y})-({max_x},{max_y}) r={rad}"
                            );
                            for bit in 0..3 {
                                bit_seen[bit][((cv >> bit) & 1) as usize] = true;
                            }
                        }
                    }
                }
            }
        }
    }
    for (bit, s) in bit_seen.iter().enumerate() {
        assert!(
            s[0] && s[1],
            "capsule() result bit {bit} never flipped both ways: {s:?}"
        );
    }
}
