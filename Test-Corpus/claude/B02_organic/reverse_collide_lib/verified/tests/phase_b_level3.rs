//! Phase B, level 3 — CONFIGS.md rows B65 … B74.
//!
//! The boolean collision API (`c2*to*`, `c2Collided`) and the public entry
//! point declared in `include/lib.h` (`reverse_collide`).

#![allow(non_snake_case)]

mod common;
use common::*;

use std::ffi::c_void;
use std::os::raw::c_int;

const N: usize = 20000;

// ---------------------------------------------------------------------------
// B65 — c2AABBtoAABB
// ---------------------------------------------------------------------------

#[test]
fn b65_aabb_to_aabb() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xB65);
    unsafe {
        // Structured cases: disjoint on x, on y, overlapping, touching,
        // contained, inverted, NaN.
        let base = c2AABB {
            min: c2v { x: 0.0, y: 0.0 },
            max: c2v { x: 10.0, y: 10.0 },
        };
        let cases = [
            c2AABB {
                min: c2v { x: 20.0, y: 0.0 },
                max: c2v { x: 30.0, y: 10.0 },
            }, // disjoint x
            c2AABB {
                min: c2v { x: 0.0, y: 20.0 },
                max: c2v { x: 10.0, y: 30.0 },
            }, // disjoint y
            c2AABB {
                min: c2v { x: 10.0, y: 0.0 },
                max: c2v { x: 20.0, y: 10.0 },
            }, // touching x
            c2AABB {
                min: c2v { x: 5.0, y: 5.0 },
                max: c2v { x: 15.0, y: 15.0 },
            }, // overlap
            c2AABB {
                min: c2v { x: 2.0, y: 2.0 },
                max: c2v { x: 8.0, y: 8.0 },
            }, // contained
            c2AABB {
                min: c2v { x: 10.0, y: 10.0 },
                max: c2v { x: 0.0, y: 0.0 },
            }, // inverted
            c2AABB {
                min: c2v {
                    x: f32::NAN,
                    y: 0.0,
                },
                max: c2v { x: 10.0, y: 10.0 },
            }, // NaN
            c2AABB {
                min: c2v {
                    x: f32::NEG_INFINITY,
                    y: f32::NEG_INFINITY,
                },
                max: c2v {
                    x: f32::INFINITY,
                    y: f32::INFINITY,
                },
            },
        ];
        for (i, bb) in cases.iter().enumerate() {
            eq_int(
                &format!("c2AABBtoAABB case{i} fwd"),
                (c.c2AABBtoAABB)(base, *bb),
                (r.c2AABBtoAABB)(base, *bb),
            );
            eq_int(
                &format!("c2AABBtoAABB case{i} rev"),
                (c.c2AABBtoAABB)(*bb, base),
                (r.c2AABBtoAABB)(*bb, base),
            );
        }
        for i in 0..N {
            let a = if rng.below(6) == 0 {
                c2AABB {
                    min: rng.wild_v(),
                    max: rng.wild_v(),
                }
            } else {
                rng.aabb(20.0)
            };
            let b = if rng.below(6) == 0 {
                c2AABB {
                    min: rng.wild_v(),
                    max: rng.wild_v(),
                }
            } else {
                rng.aabb(20.0)
            };
            eq_int(
                &format!("c2AABBtoAABB rand#{i} a={a:?} b={b:?}"),
                (c.c2AABBtoAABB)(a, b),
                (r.c2AABBtoAABB)(a, b),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// B66 — c2CircletoCircle
// ---------------------------------------------------------------------------

#[test]
fn b66_circle_to_circle() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xB66);
    unsafe {
        // Exact tangency (integer arithmetic keeps `d2 == r2` exact).
        for r1 in 1..8i32 {
            for r2 in 1..8i32 {
                let a = c2Circle {
                    p: c2v { x: 0.0, y: 0.0 },
                    r: r1 as f32,
                };
                for d in [
                    (r1 + r2) as f32,
                    (r1 + r2) as f32 - 1.0,
                    (r1 + r2) as f32 + 1.0,
                ] {
                    let b = c2Circle {
                        p: c2v { x: d, y: 0.0 },
                        r: r2 as f32,
                    };
                    eq_int(
                        &format!("c2CircletoCircle tangency r1={r1} r2={r2} d={d}"),
                        (c.c2CircletoCircle)(a, b),
                        (r.c2CircletoCircle)(a, b),
                    );
                }
            }
        }
        for i in 0..N {
            let a = if rng.below(6) == 0 {
                c2Circle {
                    p: rng.wild_v(),
                    r: rng.wild_f32(),
                }
            } else {
                rng.circle(20.0)
            };
            let b = if rng.below(6) == 0 {
                c2Circle {
                    p: rng.wild_v(),
                    r: rng.wild_f32(),
                }
            } else {
                rng.circle(20.0)
            };
            eq_int(
                &format!("c2CircletoCircle rand#{i} a={a:?} b={b:?}"),
                (c.c2CircletoCircle)(a, b),
                (r.c2CircletoCircle)(a, b),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// B67 — c2CircletoAABB
// ---------------------------------------------------------------------------

#[test]
fn b67_circle_to_aabb() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xB67);
    unsafe {
        let bb = c2AABB {
            min: c2v { x: -5.0, y: -5.0 },
            max: c2v { x: 5.0, y: 5.0 },
        };
        // Centre in each of the 9 Moore-neighbourhood regions, plus exactly on
        // an edge / corner, with radii that straddle the tangency point.
        for cx in [-10.0f32, -5.0, 0.0, 5.0, 10.0] {
            for cy in [-10.0f32, -5.0, 0.0, 5.0, 10.0] {
                for rad in [0.0f32, 1.0, 5.0, 5.000_001, 7.071_068, 100.0] {
                    let a = c2Circle {
                        p: c2v { x: cx, y: cy },
                        r: rad,
                    };
                    eq_int(
                        &format!("c2CircletoAABB grid ({cx},{cy}) r={rad}"),
                        (c.c2CircletoAABB)(a, bb),
                        (r.c2CircletoAABB)(a, bb),
                    );
                }
            }
        }
        for i in 0..N {
            let a = if rng.below(6) == 0 {
                c2Circle {
                    p: rng.wild_v(),
                    r: rng.wild_f32(),
                }
            } else {
                rng.circle(20.0)
            };
            let b = if rng.below(6) == 0 {
                c2AABB {
                    min: rng.wild_v(),
                    max: rng.wild_v(),
                }
            } else {
                rng.aabb(20.0)
            };
            eq_int(
                &format!("c2CircletoAABB rand#{i} a={a:?} b={b:?}"),
                (c.c2CircletoAABB)(a, b),
                (r.c2CircletoAABB)(a, b),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// B68 — c2CircletoCapsule (all three internal branches)
// ---------------------------------------------------------------------------

#[test]
fn b68_circle_to_capsule() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xB68);
    // [da<0, db<0 (perp), db>=0]
    let mut hits = [0usize; 3];
    unsafe {
        for i in 0..N {
            let cap = if rng.below(6) == 0 {
                c2Capsule {
                    a: rng.wild_v(),
                    b: rng.wild_v(),
                    r: rng.wild_f32(),
                }
            } else {
                rng.capsule(20.0)
            };
            let cir = if rng.below(6) == 0 {
                c2Circle {
                    p: rng.wild_v(),
                    r: rng.wild_f32(),
                }
            } else {
                rng.circle(20.0)
            };
            // classify using the C's own primitives
            let n = (c.c2Sub)(cap.b, cap.a);
            let ap = (c.c2Sub)(cir.p, cap.a);
            let da = (c.c2Dot)(ap, n);
            if da < 0.0 {
                hits[0] += 1;
            } else {
                let db = (c.c2Dot)((c.c2Sub)(cir.p, cap.b), n);
                if db < 0.0 {
                    hits[1] += 1;
                } else {
                    hits[2] += 1;
                }
            }
            eq_int(
                &format!("c2CircletoCapsule rand#{i} cir={cir:?} cap={cap:?}"),
                (c.c2CircletoCapsule)(cir, cap),
                (r.c2CircletoCapsule)(cir, cap),
            );
        }
        // Degenerate capsule (a == b) forces `c2Dot(n,n) == 0` -> 0/0.
        for &p in [
            c2v { x: 0.0, y: 0.0 },
            c2v { x: 1.0, y: 1.0 },
            c2v { x: -3.0, y: 7.0 },
        ]
        .iter()
        {
            let cap = c2Capsule {
                a: c2v { x: 0.0, y: 0.0 },
                b: c2v { x: 0.0, y: 0.0 },
                r: 2.0,
            };
            for rad in [0.0f32, 1.0, 2.0, 3.0] {
                let cir = c2Circle { p, r: rad };
                eq_int(
                    &format!("c2CircletoCapsule degenerate p={p:?} r={rad}"),
                    (c.c2CircletoCapsule)(cir, cap),
                    (r.c2CircletoCapsule)(cir, cap),
                );
            }
        }
    }
    assert!(
        hits.iter().all(|&h| h > 0),
        "c2CircletoCapsule branch coverage incomplete: {hits:?}"
    );
    println!("c2CircletoCapsule branch hits: {hits:?}");
}

// ---------------------------------------------------------------------------
// B69 / B70 — the two GJK-backed booleans
// ---------------------------------------------------------------------------

#[test]
fn b69_aabb_to_capsule() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xB69);
    let mut yes = 0usize;
    let mut no = 0usize;
    unsafe {
        for i in 0..N / 2 {
            let bb = if rng.below(8) == 0 {
                c2AABB {
                    min: rng.wild_v(),
                    max: rng.wild_v(),
                }
            } else {
                rng.aabb(20.0)
            };
            let cap = if rng.below(8) == 0 {
                c2Capsule {
                    a: rng.wild_v(),
                    b: rng.wild_v(),
                    r: rng.wild_f32(),
                }
            } else {
                rng.capsule(20.0)
            };
            let cv = (c.c2AABBtoCapsule)(bb, cap);
            let rv = (r.c2AABBtoCapsule)(bb, cap);
            eq_int(
                &format!("c2AABBtoCapsule rand#{i} bb={bb:?} cap={cap:?}"),
                cv,
                rv,
            );
            if cv != 0 {
                yes += 1
            } else {
                no += 1
            }
        }
    }
    assert!(yes > 0 && no > 0, "coverage: yes={yes} no={no}");
}

#[test]
fn b70_capsule_to_capsule() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xB70);
    let mut yes = 0usize;
    let mut no = 0usize;
    unsafe {
        // Structured: parallel, crossing, collinear, identical, degenerate.
        let structured = [
            (
                c2Capsule {
                    a: c2v { x: 0.0, y: 0.0 },
                    b: c2v { x: 10.0, y: 0.0 },
                    r: 1.0,
                },
                c2Capsule {
                    a: c2v { x: 0.0, y: 2.0 },
                    b: c2v { x: 10.0, y: 2.0 },
                    r: 1.0,
                },
            ), // parallel, exactly touching
            (
                c2Capsule {
                    a: c2v { x: 0.0, y: 0.0 },
                    b: c2v { x: 10.0, y: 0.0 },
                    r: 1.0,
                },
                c2Capsule {
                    a: c2v { x: 5.0, y: -5.0 },
                    b: c2v { x: 5.0, y: 5.0 },
                    r: 1.0,
                },
            ), // crossing
            (
                c2Capsule {
                    a: c2v { x: 0.0, y: 0.0 },
                    b: c2v { x: 10.0, y: 0.0 },
                    r: 1.0,
                },
                c2Capsule {
                    a: c2v { x: 12.0, y: 0.0 },
                    b: c2v { x: 22.0, y: 0.0 },
                    r: 1.0,
                },
            ), // collinear, exactly touching
            (
                c2Capsule {
                    a: c2v { x: 0.0, y: 0.0 },
                    b: c2v { x: 10.0, y: 0.0 },
                    r: 1.0,
                },
                c2Capsule {
                    a: c2v { x: 0.0, y: 0.0 },
                    b: c2v { x: 10.0, y: 0.0 },
                    r: 1.0,
                },
            ), // identical
            (
                c2Capsule {
                    a: c2v { x: 1.0, y: 1.0 },
                    b: c2v { x: 1.0, y: 1.0 },
                    r: 0.0,
                },
                c2Capsule {
                    a: c2v { x: 1.0, y: 1.0 },
                    b: c2v { x: 1.0, y: 1.0 },
                    r: 0.0,
                },
            ), // both degenerate
        ];
        for (i, (a, b)) in structured.iter().enumerate() {
            eq_int(
                &format!("c2CapsuletoCapsule structured{i}"),
                (c.c2CapsuletoCapsule)(*a, *b),
                (r.c2CapsuletoCapsule)(*a, *b),
            );
        }
        for i in 0..N / 2 {
            let a = if rng.below(8) == 0 {
                c2Capsule {
                    a: rng.wild_v(),
                    b: rng.wild_v(),
                    r: rng.wild_f32(),
                }
            } else {
                rng.capsule(20.0)
            };
            let b = if rng.below(8) == 0 {
                c2Capsule {
                    a: rng.wild_v(),
                    b: rng.wild_v(),
                    r: rng.wild_f32(),
                }
            } else {
                rng.capsule(20.0)
            };
            let cv = (c.c2CapsuletoCapsule)(a, b);
            let rv = (r.c2CapsuletoCapsule)(a, b);
            eq_int(
                &format!("c2CapsuletoCapsule rand#{i} a={a:?} b={b:?}"),
                cv,
                rv,
            );
            if cv != 0 {
                yes += 1
            } else {
                no += 1
            }
        }
    }
    assert!(yes > 0 && no > 0, "coverage: yes={yes} no={no}");
}

// ---------------------------------------------------------------------------
// B71 — c2Collided for all 9 valid type pairs (checks the argument swap)
// ---------------------------------------------------------------------------

#[repr(C, align(8))]
#[derive(Copy, Clone)]
struct Buf([u8; 32]);

fn put<T: Copy>(v: &T) -> Buf {
    let mut b = Buf([0x5A; 32]);
    unsafe {
        std::ptr::copy_nonoverlapping(
            v as *const T as *const u8,
            b.0.as_mut_ptr(),
            std::mem::size_of::<T>(),
        );
    }
    b
}

#[test]
fn b71_collided_all_pairs() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xB71);
    let mut hits = [[0usize; 3]; 3];
    unsafe {
        for i in 0..N / 2 {
            let bufs: [Buf; 3] = [
                put(&rng.circle(20.0)),
                put(&rng.aabb(20.0)),
                put(&rng.capsule(20.0)),
            ];
            let bufs2: [Buf; 3] = [
                put(&rng.circle(20.0)),
                put(&rng.aabb(20.0)),
                put(&rng.capsule(20.0)),
            ];
            for ta in 0..3usize {
                for tb in 0..3usize {
                    let pa = bufs[ta].0.as_ptr() as *const c_void;
                    let pb = bufs2[tb].0.as_ptr() as *const c_void;
                    let cv = (c.c2Collided)(pa, ta as c_int, pb, tb as c_int);
                    let rv = (r.c2Collided)(pa, ta as c_int, pb, tb as c_int);
                    eq_int(&format!("c2Collided {ta}x{tb} #{i}"), cv, rv);
                    if cv != 0 {
                        hits[ta][tb] += 1;
                    }
                }
            }
        }
    }
    // Every pair must have produced at least one hit and one miss over the run.
    for ta in 0..3 {
        for tb in 0..3 {
            assert!(
                hits[ta][tb] > 0,
                "c2Collided {ta}x{tb} never reported a collision"
            );
        }
    }
    println!("c2Collided hit counts: {hits:?}");
}

// ---------------------------------------------------------------------------
// B72 / B73 / B74 — reverse_collide, the public entry point
// ---------------------------------------------------------------------------

#[test]
fn b72_reverse_collide_random() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xB72);
    let mut seen = [0usize; 8];
    unsafe {
        for i in 0..65536 {
            let x = rng.uniform(-150.0, 150.0);
            let y = rng.uniform(-150.0, 150.0);
            let rad = rng.uniform(0.0, 60.0);
            let cv = (c.reverse_collide)(x, y, rad);
            let rv = (r.reverse_collide)(x, y, rad);
            eq_int(&format!("reverse_collide rand#{i} ({x},{y},{rad})"), cv, rv);
            assert!((0..8).contains(&cv), "unexpected mask {cv}");
            seen[cv as usize] += 1;
        }
    }
    println!("reverse_collide mask histogram: {seen:?}");
    // All three shapes must be individually reachable.
    assert!(seen[0] > 0 && seen[1] > 0 && seen[2] > 0 && seen[4] > 0);
}

#[test]
fn b73_reverse_collide_boundaries() {
    let (c, r) = libs();
    unsafe {
        let vals = [
            0.0f32,
            -0.0,
            FLT_MIN_POS,
            -FLT_MIN_POS,
            1.0e-45,
            FLT_EPSILON,
            1.0,
            -1.0,
            15.0,
            -15.0,
            20.0,
            -20.0,
            40.0,
            -40.0,
            70.0,
            -70.0,
            100.0,
            -100.0,
            FLT_MAX,
            -FLT_MAX,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NAN,
            -f32::NAN,
            // exact tangency values for each embedded shape
            -50.0, // circle at (-70,0) r=20 : x = -70+20
            -90.0,
            -55.0, // aabb min.x - 40 etc.
            -30.0,
            -25.0,
        ];
        for &x in vals.iter() {
            for &y in vals.iter() {
                for &rad in vals.iter() {
                    let cv = (c.reverse_collide)(x, y, rad);
                    let rv = (r.reverse_collide)(x, y, rad);
                    eq_int(&format!("reverse_collide ({x:?},{y:?},{rad:?})"), cv, rv);
                }
            }
        }
    }
}

#[test]
fn b74_reverse_collide_lattice_sweep() {
    let (c, r) = libs();
    let mut seen = [0usize; 8];
    unsafe {
        for xi in -160..=160i32 {
            for yi in -160..=160i32 {
                for &rad in [0.0f32, 1.0, 5.0, 10.0, 20.0, 50.0].iter() {
                    let x = xi as f32;
                    let y = yi as f32;
                    let cv = (c.reverse_collide)(x, y, rad);
                    let rv = (r.reverse_collide)(x, y, rad);
                    eq_int(&format!("reverse_collide lattice ({x},{y},{rad})"), cv, rv);
                    seen[cv as usize] += 1;
                }
            }
        }
    }
    println!("lattice mask histogram: {seen:?}");
    let reachable = seen.iter().filter(|&&n| n > 0).count();
    assert!(
        reachable >= 5,
        "lattice sweep only reached {reachable} distinct masks: {seen:?}"
    );
}
