//! Level 2: `c2Clampv` (built on Minv/Maxv) and the three collision
//! primitives `c2CircletoCircle`, `c2CircletoAABB`, `c2AABBtoAABB`.

mod common;

use common::*;

#[test]
fn c2clampv_interesting() {
    let (c, r) = both();
    let vals = interesting_f32();
    for (i, &ax) in vals.iter().enumerate() {
        for (j, &lox) in vals.iter().enumerate() {
            let hix = vals[(i + j + 5) % vals.len()];
            let a = c2v {
                x: ax,
                y: vals[(i + 2 * j) % vals.len()],
            };
            let lo = c2v {
                x: lox,
                y: vals[(j + 7) % vals.len()],
            };
            let hi = c2v {
                x: hix,
                y: vals[(i * 5 + j) % vals.len()],
            };
            assert_c2v_eq(
                "c2Clampv",
                unsafe { (c.c2Clampv)(a, lo, hi) },
                unsafe { (r.c2Clampv)(a, lo, hi) },
                &format!("a={a:?} lo={lo:?} hi={hi:?}"),
            );
        }
    }
}

#[test]
fn c2clampv_random_bits() {
    let (c, r) = both();
    let mut rng = Rng::new(0xABCD_1234);
    for _ in 0..200_000 {
        let a = c2v {
            x: rng.any_f32(),
            y: rng.any_f32(),
        };
        let lo = c2v {
            x: rng.any_f32(),
            y: rng.any_f32(),
        };
        let hi = c2v {
            x: rng.any_f32(),
            y: rng.any_f32(),
        };
        assert_c2v_eq(
            "c2Clampv",
            unsafe { (c.c2Clampv)(a, lo, hi) },
            unsafe { (r.c2Clampv)(a, lo, hi) },
            &format!("a={a:?} lo={lo:?} hi={hi:?}"),
        );
    }
}

/// Inverted bounds (lo > hi) are not rejected by the C code; make sure the
/// resulting clamp order is reproduced.
#[test]
fn c2clampv_inverted_bounds() {
    let (c, r) = both();
    let mut rng = Rng::new(7);
    for _ in 0..100_000 {
        let a = c2v {
            x: rng.coord(),
            y: rng.coord(),
        };
        let lo = c2v {
            x: rng.coord() + 50.0,
            y: rng.coord() + 50.0,
        };
        let hi = c2v {
            x: rng.coord() - 50.0,
            y: rng.coord() - 50.0,
        };
        assert_c2v_eq(
            "c2Clampv",
            unsafe { (c.c2Clampv)(a, lo, hi) },
            unsafe { (r.c2Clampv)(a, lo, hi) },
            &format!("a={a:?} lo={lo:?} hi={hi:?}"),
        );
    }
}

fn circles() -> Vec<c2Circle> {
    let mut v = Vec::new();
    let coords = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        3.0,
        -3.0,
        1e30,
        -1e30,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ];
    let radii = [
        0.0f32,
        -0.0,
        -1.0,
        1.0,
        2.0,
        1e-30,
        1e30,
        f32::MAX,
        f32::INFINITY,
        f32::NAN,
    ];
    for &x in &coords {
        for &y in &coords {
            for &r in &radii {
                v.push(c2Circle { p: c2v { x, y }, r });
            }
        }
    }
    v
}

#[test]
fn circle_to_circle_touching_and_edges() {
    let (c, r) = both();
    // Exact-touch, just-inside and just-outside cases around r1+r2.
    let cases: Vec<(c2Circle, c2Circle)> = vec![
        (
            c2Circle {
                p: c2v { x: 0.0, y: 0.0 },
                r: 1.0,
            },
            c2Circle {
                p: c2v { x: 2.0, y: 0.0 },
                r: 1.0,
            },
        ),
        (
            c2Circle {
                p: c2v { x: 0.0, y: 0.0 },
                r: 1.0,
            },
            c2Circle {
                p: c2v {
                    x: 1.9999999,
                    y: 0.0,
                },
                r: 1.0,
            },
        ),
        (
            c2Circle {
                p: c2v { x: 0.0, y: 0.0 },
                r: 1.0,
            },
            c2Circle {
                p: c2v {
                    x: 2.0000002,
                    y: 0.0,
                },
                r: 1.0,
            },
        ),
        (
            c2Circle {
                p: c2v { x: 0.0, y: 0.0 },
                r: 0.0,
            },
            c2Circle {
                p: c2v { x: 0.0, y: 0.0 },
                r: 0.0,
            },
        ),
        // Negative radii: r2 = (rA+rB)^2 makes the sign vanish.
        (
            c2Circle {
                p: c2v { x: 0.0, y: 0.0 },
                r: -1.0,
            },
            c2Circle {
                p: c2v { x: 1.0, y: 0.0 },
                r: -1.0,
            },
        ),
        // Overflow of the squared radius.
        (
            c2Circle {
                p: c2v { x: 0.0, y: 0.0 },
                r: f32::MAX,
            },
            c2Circle {
                p: c2v { x: 0.0, y: 0.0 },
                r: f32::MAX,
            },
        ),
    ];
    for (a, b) in cases {
        assert_eq!(
            unsafe { (c.c2CircletoCircle)(a, b) },
            unsafe { (r.c2CircletoCircle)(a, b) },
            "c2CircletoCircle mismatch for A={a:?} B={b:?}"
        );
    }
}

#[test]
fn circle_to_circle_grid() {
    let (c, r) = both();
    let cs = circles();
    for (i, &a) in cs.iter().enumerate() {
        for &b in cs.iter().skip(i % 7).step_by(11) {
            assert_eq!(
                unsafe { (c.c2CircletoCircle)(a, b) },
                unsafe { (r.c2CircletoCircle)(a, b) },
                "c2CircletoCircle mismatch for A={a:?} B={b:?}"
            );
        }
    }
}

#[test]
fn circle_to_circle_random() {
    let (c, r) = both();
    let mut rng = Rng::new(0x5EED);
    for _ in 0..200_000 {
        let a = c2Circle {
            p: c2v {
                x: rng.coord(),
                y: rng.coord(),
            },
            r: rng.coord(),
        };
        let b = c2Circle {
            p: c2v {
                x: rng.coord(),
                y: rng.coord(),
            },
            r: rng.coord(),
        };
        assert_eq!(
            unsafe { (c.c2CircletoCircle)(a, b) },
            unsafe { (r.c2CircletoCircle)(a, b) },
            "c2CircletoCircle mismatch for A={a:?} B={b:?}"
        );
    }
    for _ in 0..100_000 {
        let a = c2Circle {
            p: c2v {
                x: rng.any_f32(),
                y: rng.any_f32(),
            },
            r: rng.any_f32(),
        };
        let b = c2Circle {
            p: c2v {
                x: rng.any_f32(),
                y: rng.any_f32(),
            },
            r: rng.any_f32(),
        };
        assert_eq!(
            unsafe { (c.c2CircletoCircle)(a, b) },
            unsafe { (r.c2CircletoCircle)(a, b) },
            "c2CircletoCircle mismatch for A={a:?} B={b:?}"
        );
    }
}

fn aabbs() -> Vec<c2AABB> {
    let mut v = Vec::new();
    let vals = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        2.0,
        -2.0,
        1e30,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ];
    for &a in &vals {
        for &b in &vals {
            for &cc in &vals {
                for &d in &vals {
                    v.push(c2AABB {
                        min: c2v { x: a, y: b },
                        max: c2v { x: cc, y: d },
                    });
                }
            }
        }
    }
    v
}

#[test]
fn aabb_to_aabb_grid() {
    let (c, r) = both();
    let bs = aabbs();
    for (i, &a) in bs.iter().enumerate() {
        for &b in bs.iter().skip(i % 13).step_by(29) {
            assert_eq!(
                unsafe { (c.c2AABBtoAABB)(a, b) },
                unsafe { (r.c2AABBtoAABB)(a, b) },
                "c2AABBtoAABB mismatch for A={a:?} B={b:?}"
            );
        }
    }
}

#[test]
fn aabb_to_aabb_random() {
    let (c, r) = both();
    let mut rng = Rng::new(0x1DEA);
    for _ in 0..200_000 {
        let a = c2AABB {
            min: c2v {
                x: rng.coord(),
                y: rng.coord(),
            },
            max: c2v {
                x: rng.coord(),
                y: rng.coord(),
            },
        };
        let b = c2AABB {
            min: c2v {
                x: rng.coord(),
                y: rng.coord(),
            },
            max: c2v {
                x: rng.coord(),
                y: rng.coord(),
            },
        };
        assert_eq!(
            unsafe { (c.c2AABBtoAABB)(a, b) },
            unsafe { (r.c2AABBtoAABB)(a, b) },
            "c2AABBtoAABB mismatch for A={a:?} B={b:?}"
        );
    }
    for _ in 0..100_000 {
        let a = c2AABB {
            min: c2v {
                x: rng.any_f32(),
                y: rng.any_f32(),
            },
            max: c2v {
                x: rng.any_f32(),
                y: rng.any_f32(),
            },
        };
        let b = c2AABB {
            min: c2v {
                x: rng.any_f32(),
                y: rng.any_f32(),
            },
            max: c2v {
                x: rng.any_f32(),
                y: rng.any_f32(),
            },
        };
        assert_eq!(
            unsafe { (c.c2AABBtoAABB)(a, b) },
            unsafe { (r.c2AABBtoAABB)(a, b) },
            "c2AABBtoAABB mismatch for A={a:?} B={b:?}"
        );
    }
}

/// Shared edges: `<` (not `<=`) means touching boxes count as colliding.
#[test]
fn aabb_to_aabb_shared_edges() {
    let (c, r) = both();
    let a = c2AABB {
        min: c2v { x: 0.0, y: 0.0 },
        max: c2v { x: 1.0, y: 1.0 },
    };
    let deltas = [-2.0f32, -1.0, -0.5, 0.0, 0.5, 1.0, 2.0];
    for &dx in &deltas {
        for &dy in &deltas {
            let b = c2AABB {
                min: c2v { x: dx, y: dy },
                max: c2v { x: dx + 1.0, y: dy + 1.0 },
            };
            assert_eq!(
                unsafe { (c.c2AABBtoAABB)(a, b) },
                unsafe { (r.c2AABBtoAABB)(a, b) },
                "c2AABBtoAABB mismatch for A={a:?} B={b:?}"
            );
        }
    }
}

#[test]
fn circle_to_aabb_grid() {
    let (c, r) = both();
    let cs = circles();
    let bs = aabbs();
    for (i, &a) in cs.iter().enumerate() {
        for &b in bs.iter().skip(i % 17).step_by(37) {
            assert_eq!(
                unsafe { (c.c2CircletoAABB)(a, b) },
                unsafe { (r.c2CircletoAABB)(a, b) },
                "c2CircletoAABB mismatch for A={a:?} B={b:?}"
            );
        }
    }
}

#[test]
fn circle_to_aabb_random() {
    let (c, r) = both();
    let mut rng = Rng::new(0xFACE);
    for _ in 0..200_000 {
        let a = c2Circle {
            p: c2v {
                x: rng.coord(),
                y: rng.coord(),
            },
            r: rng.coord(),
        };
        let b = c2AABB {
            min: c2v {
                x: rng.coord(),
                y: rng.coord(),
            },
            max: c2v {
                x: rng.coord(),
                y: rng.coord(),
            },
        };
        assert_eq!(
            unsafe { (c.c2CircletoAABB)(a, b) },
            unsafe { (r.c2CircletoAABB)(a, b) },
            "c2CircletoAABB mismatch for A={a:?} B={b:?}"
        );
    }
    for _ in 0..100_000 {
        let a = c2Circle {
            p: c2v {
                x: rng.any_f32(),
                y: rng.any_f32(),
            },
            r: rng.any_f32(),
        };
        let b = c2AABB {
            min: c2v {
                x: rng.any_f32(),
                y: rng.any_f32(),
            },
            max: c2v {
                x: rng.any_f32(),
                y: rng.any_f32(),
            },
        };
        assert_eq!(
            unsafe { (c.c2CircletoAABB)(a, b) },
            unsafe { (r.c2CircletoAABB)(a, b) },
            "c2CircletoAABB mismatch for A={a:?} B={b:?}"
        );
    }
}

/// Circle centred on / just outside a box boundary, where the clamp result
/// equals the centre and `d2 == 0`.
#[test]
fn circle_to_aabb_boundary() {
    let (c, r) = both();
    let b = c2AABB {
        min: c2v { x: -1.0, y: -1.0 },
        max: c2v { x: 1.0, y: 1.0 },
    };
    let pts = [
        -1.0000001f32,
        -1.0,
        -0.9999999,
        0.0,
        0.9999999,
        1.0,
        1.0000001,
        2.0,
    ];
    let radii = [0.0f32, -0.0, 1e-30, 0.5, 1.0, 1.0000001, -1.0];
    for &x in &pts {
        for &y in &pts {
            for &rad in &radii {
                let a = c2Circle {
                    p: c2v { x, y },
                    r: rad,
                };
                assert_eq!(
                    unsafe { (c.c2CircletoAABB)(a, b) },
                    unsafe { (r.c2CircletoAABB)(a, b) },
                    "c2CircletoAABB mismatch for A={a:?} B={b:?}"
                );
            }
        }
    }
}
