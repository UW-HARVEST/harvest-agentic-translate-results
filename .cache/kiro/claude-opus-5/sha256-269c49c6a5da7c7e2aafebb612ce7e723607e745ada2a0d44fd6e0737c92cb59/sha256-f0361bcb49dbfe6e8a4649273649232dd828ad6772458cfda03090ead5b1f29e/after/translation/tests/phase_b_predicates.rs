//! Phase B — CONFIGS.md rows 14..19: the boolean predicates.

mod common;
use common::*;

const N: usize = 20_000;

fn boxes_overlapping(rng: &mut Rng) -> (C2AABB, C2AABB) {
    let mk = |rng: &mut Rng| {
        let c = rng.v(50.0);
        let h = C2v {
            x: rng.unit() * 20.0,
            y: rng.unit() * 20.0,
        };
        C2AABB {
            min: C2v {
                x: c.x - h.x,
                y: c.y - h.y,
            },
            max: C2v {
                x: c.x + h.x,
                y: c.y + h.y,
            },
        }
    };
    (mk(rng), mk(rng))
}

// --- rows 14,15,16,17: c2AABBtoAABB ---------------------------------------
#[test]
fn rows14_17_aabb_to_aabb() {
    let l = libs();
    let mut rng = Rng::new(0x1417);

    // row 14: random, mostly overlapping
    for _ in 0..N {
        let (a, b) = boxes_overlapping(&mut rng);
        diff_eq!(
            format!("c2AABBtoAABB rnd {a:?} {b:?}"),
            (l.c.c2AABBtoAABB)(a, b),
            (l.r.c2AABBtoAABB)(a, b)
        );
    }

    // row 15: each separation direction forced
    for dir in 0..4 {
        for _ in 0..2_000 {
            let a = C2AABB {
                min: C2v { x: 0.0, y: 0.0 },
                max: C2v { x: 10.0, y: 10.0 },
            };
            let off = 11.0 + rng.unit() * 100.0;
            let b = match dir {
                0 => C2AABB {
                    min: C2v { x: -off - 10.0, y: 0.0 },
                    max: C2v { x: -off, y: 10.0 },
                },
                1 => C2AABB {
                    min: C2v { x: off, y: 0.0 },
                    max: C2v { x: off + 10.0, y: 10.0 },
                },
                2 => C2AABB {
                    min: C2v { x: 0.0, y: -off - 10.0 },
                    max: C2v { x: 10.0, y: -off },
                },
                _ => C2AABB {
                    min: C2v { x: 0.0, y: off },
                    max: C2v { x: 10.0, y: off + 10.0 },
                },
            };
            diff_eq!(
                format!("c2AABBtoAABB sep{dir} {a:?} {b:?}"),
                (l.c.c2AABBtoAABB)(a, b),
                (l.r.c2AABBtoAABB)(a, b)
            );
        }
    }

    // row 16: exact touching / flat / inverted
    let s = specials_nan_payloads();
    let coords: Vec<f32> = s.iter().copied().chain([10.0, -10.0, 0.0].into_iter()).collect();
    for &v in &coords {
        for &w in &coords {
            let a = C2AABB {
                min: C2v { x: 0.0, y: 0.0 },
                max: C2v { x: v, y: 10.0 },
            };
            let b = C2AABB {
                min: C2v { x: v, y: w },
                max: C2v { x: 20.0, y: 10.0 },
            };
            diff_eq!(
                format!("c2AABBtoAABB touch {a:?} {b:?}"),
                (l.c.c2AABBtoAABB)(a, b),
                (l.r.c2AABBtoAABB)(a, b)
            );
            // inverted
            let a2 = C2AABB { min: a.max, max: a.min };
            let b2 = C2AABB { min: b.max, max: b.min };
            diff_eq!(
                format!("c2AABBtoAABB inverted {a2:?} {b2:?}"),
                (l.c.c2AABBtoAABB)(a2, b2),
                (l.r.c2AABBtoAABB)(a2, b2)
            );
        }
    }

    // row 17: full-bit-space fuzz including NaN
    for _ in 0..N {
        let a = C2AABB {
            min: rng.v_special(1e6),
            max: rng.v_special(1e6),
        };
        let b = C2AABB {
            min: rng.v_special(1e6),
            max: rng.v_special(1e6),
        };
        diff_eq!(
            format!("c2AABBtoAABB special {a:?} {b:?}"),
            (l.c.c2AABBtoAABB)(a, b),
            (l.r.c2AABBtoAABB)(a, b)
        );
    }
}

// --- row 18: c2AABBtoPoint ------------------------------------------------
#[test]
fn row18_aabb_to_point() {
    let l = libs();
    let mut rng = Rng::new(0x1818);
    let bx = C2AABB {
        min: C2v { x: -1.0, y: -2.0 },
        max: C2v { x: 3.0, y: 4.0 },
    };
    // edges, corners, inside, outside on each side
    let interesting: Vec<f32> = specials_nan_payloads()
        .into_iter()
        .chain([-1.0, 3.0, -2.0, 4.0, 0.0, 5.0, -5.0].into_iter())
        .collect();
    for &px in &interesting {
        for &py in &interesting {
            let p = C2v { x: px, y: py };
            diff_eq!(
                format!("c2AABBtoPoint {bx:?} {p:?}"),
                (l.c.c2AABBtoPoint)(bx, p),
                (l.r.c2AABBtoPoint)(bx, p)
            );
        }
    }
    // flat + inverted boxes
    for b in [
        C2AABB {
            min: C2v { x: 1.0, y: 1.0 },
            max: C2v { x: 1.0, y: 5.0 },
        },
        C2AABB {
            min: C2v { x: 1.0, y: 1.0 },
            max: C2v { x: 5.0, y: 1.0 },
        },
        C2AABB {
            min: C2v { x: 5.0, y: 5.0 },
            max: C2v { x: 1.0, y: 1.0 },
        },
    ] {
        for _ in 0..5_000 {
            let p = if rng.next_u32() & 1 == 0 {
                rng.v(8.0)
            } else {
                rng.v_special(8.0)
            };
            diff_eq!(
                format!("c2AABBtoPoint flat {b:?} {p:?}"),
                (l.c.c2AABBtoPoint)(b, p),
                (l.r.c2AABBtoPoint)(b, p)
            );
        }
    }
    for _ in 0..N {
        let b = C2AABB {
            min: rng.v_special(1e5),
            max: rng.v_special(1e5),
        };
        let p = rng.v_special(1e5);
        diff_eq!(
            format!("c2AABBtoPoint rnd {b:?} {p:?}"),
            (l.c.c2AABBtoPoint)(b, p),
            (l.r.c2AABBtoPoint)(b, p)
        );
    }
}

// --- row 19: c2CircleToPoint ----------------------------------------------
#[test]
fn row19_circle_to_point() {
    let l = libs();
    let mut rng = Rng::new(0x1919);

    // exactly on the rim (rejected because the test is `<`)
    for _ in 0..5_000 {
        let c = C2Circle {
            p: rng.v(20.0),
            r: rng.unit() * 10.0 + 0.001,
        };
        let d = rng.unit_dir();
        let p = C2v {
            x: c.p.x + d.x * c.r,
            y: c.p.y + d.y * c.r,
        };
        diff_eq!(
            format!("c2CircleToPoint rim {c:?} {p:?}"),
            (l.c.c2CircleToPoint)(c, p),
            (l.r.c2CircleToPoint)(c, p)
        );
    }
    // inside / outside
    for _ in 0..N {
        let c = C2Circle {
            p: rng.v(20.0),
            r: rng.unit() * 10.0,
        };
        let p = rng.v(30.0);
        diff_eq!(
            format!("c2CircleToPoint rnd {c:?} {p:?}"),
            (l.c.c2CircleToPoint)(c, p),
            (l.r.c2CircleToPoint)(c, p)
        );
    }
    // r == 0, r < 0, r huge (r*r overflows), NaN
    let s = specials_nan_payloads();
    for &r in &s {
        for &px in &s {
            for &py in &s {
                let c = C2Circle {
                    p: C2v { x: 1.0, y: -2.0 },
                    r,
                };
                let p = C2v { x: px, y: py };
                diff_eq!(
                    format!("c2CircleToPoint special r={:#x} p=({:#x},{:#x})", fb(r), fb(px), fb(py)),
                    (l.c.c2CircleToPoint)(c, p),
                    (l.r.c2CircleToPoint)(c, p)
                );
            }
        }
    }
    for _ in 0..N {
        let c = C2Circle {
            p: rng.v_special(1e6),
            r: rng.special(1e6),
        };
        let p = rng.v_special(1e6);
        diff_eq!(
            format!("c2CircleToPoint fuzz {c:?} {p:?}"),
            (l.c.c2CircleToPoint)(c, p),
            (l.r.c2CircleToPoint)(c, p)
        );
    }
}
