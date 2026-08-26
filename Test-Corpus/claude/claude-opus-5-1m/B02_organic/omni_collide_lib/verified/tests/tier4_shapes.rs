//! Phase B, `CONFIGS.md` rows 65–71: the boolean shape-vs-shape predicates and
//! `c2Collided`.
//!
//! These are the by-value aggregate entry points, so they also exercise the
//! System V AMD64 classification of `c2Circle` (12 B, SSE+SSE), `c2AABB`
//! (16 B, SSE+SSE) and `c2Capsule` (20 B, **MEMORY** class — passed on the
//! stack).

#![allow(non_snake_case)]
#![allow(clippy::useless_format, clippy::manual_range_patterns, clippy::needless_late_init, clippy::excessive_precision, clippy::too_many_arguments, clippy::needless_range_loop)]

#[macro_use]
mod common;

use common::*;
use std::os::raw::{c_int, c_void};

const N: usize = 30_000;

#[derive(Default)]
struct Tally {
    yes: usize,
    no: usize,
}

impl Tally {
    fn note(&mut self, v: c_int) {
        if v != 0 {
            self.yes += 1
        } else {
            self.no += 1
        }
    }
    #[track_caller]
    fn require_both(&self, name: &str) {
        eprintln!("[coverage] {name}: hit={} miss={}", self.yes, self.no);
        assert!(
            self.yes > 10 && self.no > 10,
            "{name} never produced both outcomes: {} / {}",
            self.yes,
            self.no
        );
    }
}

// ---------------------------------------------------------------------------
// row 65 — c2AABBtoAABB
// ---------------------------------------------------------------------------

#[test]
fn row65_c2AABBtoAABB() {
    let (c, r) = fnpair!("c2AABBtoAABB", FnAABBtoAABB);
    let mut rng = Rng::new(SEED ^ 65);
    let mut t = Tally::default();

    let go = |a: c2AABB, b: c2AABB, ctx: String, t: &mut Tally| {
        let (cv, rv) = (c(a, b), r(a, b));
        eq_int(&format!("c2AABBtoAABB {ctx} A={a:?} B={b:?}"), cv, rv);
        t.note(cv);
    };

    for i in 0..N {
        // random (incl. inverted / degenerate via Rng::aabb)
        let a = rng.aabb();
        let b = rng.aabb();
        go(a, b, format!("rand #{i}"), &mut t);

        // integer grid: exact touching edges / corners
        let ax = (rng.below(9) as f32) - 4.0;
        let ay = (rng.below(9) as f32) - 4.0;
        let bx = (rng.below(9) as f32) - 4.0;
        let by = (rng.below(9) as f32) - 4.0;
        let a = c2AABB {
            min: c2v { x: ax, y: ay },
            max: c2v { x: ax + 2.0, y: ay + 2.0 },
        };
        let b = c2AABB {
            min: c2v { x: bx, y: by },
            max: c2v { x: bx + 2.0, y: by + 2.0 },
        };
        go(a, b, format!("grid #{i}"), &mut t);

        // nested
        let a = c2AABB {
            min: c2v { x: -10.0, y: -10.0 },
            max: c2v { x: 10.0, y: 10.0 },
        };
        let b = c2AABB {
            min: c2v { x: ax, y: ay },
            max: c2v { x: ax + 1.0, y: ay + 1.0 },
        };
        go(a, b, format!("nested #{i}"), &mut t);
    }

    // every special value in every one of the 8 float slots
    let base = c2AABB {
        min: c2v { x: 0.0, y: 0.0 },
        max: c2v { x: 1.0, y: 1.0 },
    };
    for &s in SPECIALS.iter() {
        for slot in 0..8 {
            let (mut a, mut b) = (base, base);
            let f: &mut f32 = match slot {
                0 => &mut a.min.x,
                1 => &mut a.min.y,
                2 => &mut a.max.x,
                3 => &mut a.max.y,
                4 => &mut b.min.x,
                5 => &mut b.min.y,
                6 => &mut b.max.x,
                _ => &mut b.max.y,
            };
            *f = s;
            go(a, b, format!("special slot={slot} s={s:?}"), &mut t);
        }
    }
    t.require_both("c2AABBtoAABB");
}

// ---------------------------------------------------------------------------
// row 66 — c2CircletoCircle
// ---------------------------------------------------------------------------

#[test]
fn row66_c2CircletoCircle() {
    let (c, r) = fnpair!("c2CircletoCircle", FnCircletoCircle);
    let mut rng = Rng::new(SEED ^ 66);
    let mut t = Tally::default();

    let go = |a: c2Circle, b: c2Circle, ctx: String, t: &mut Tally| {
        let (cv, rv) = (c(a, b), r(a, b));
        eq_int(&format!("c2CircletoCircle {ctx} A={a:?} B={b:?}"), cv, rv);
        t.note(cv);
    };

    for i in 0..N {
        go(rng.circle(), rng.circle(), format!("rand #{i}"), &mut t);
        // exactly touching: distance == rA + rB
        let ra = (rng.below(8) as f32) * 0.5;
        let rb = (rng.below(8) as f32) * 0.5;
        let p = c2v {
            x: (rng.below(9) as f32) - 4.0,
            y: (rng.below(9) as f32) - 4.0,
        };
        for dd in [-1.0f32, -0.5, 0.0, 0.5, 1.0] {
            go(
                c2Circle { p, r: ra },
                c2Circle {
                    p: c2v {
                        x: p.x + ra + rb + dd,
                        y: p.y,
                    },
                    r: rb,
                },
                format!("touch #{i} d={dd}"),
                &mut t,
            );
        }
        // concentric
        go(
            c2Circle { p, r: ra },
            c2Circle { p, r: rb },
            format!("concentric #{i}"),
            &mut t,
        );
        // negative radii (r2 = (rA+rB)^2 is still >= 0)
        go(
            c2Circle { p, r: -ra },
            c2Circle { p, r: -rb },
            format!("negrad #{i}"),
            &mut t,
        );
    }
    for &s in SPECIALS.iter() {
        for slot in 0..6 {
            let mut a = c2Circle {
                p: c2v { x: 0.0, y: 0.0 },
                r: 1.0,
            };
            let mut b = c2Circle {
                p: c2v { x: 1.5, y: 0.0 },
                r: 1.0,
            };
            match slot {
                0 => a.p.x = s,
                1 => a.p.y = s,
                2 => a.r = s,
                3 => b.p.x = s,
                4 => b.p.y = s,
                _ => b.r = s,
            }
            go(a, b, format!("special slot={slot} s={s:?}"), &mut t);
        }
    }
    t.require_both("c2CircletoCircle");
}

// ---------------------------------------------------------------------------
// row 67 — c2CircletoAABB
// ---------------------------------------------------------------------------

#[test]
fn row67_c2CircletoAABB() {
    let (c, r) = fnpair!("c2CircletoAABB", FnCircletoAABB);
    let mut rng = Rng::new(SEED ^ 67);
    let mut t = Tally::default();

    let go = |a: c2Circle, b: c2AABB, ctx: String, t: &mut Tally| {
        let (cv, rv) = (c(a, b), r(a, b));
        eq_int(&format!("c2CircletoAABB {ctx} A={a:?} B={b:?}"), cv, rv);
        t.note(cv);
    };

    let unit = c2AABB {
        min: c2v { x: -1.0, y: -1.0 },
        max: c2v { x: 1.0, y: 1.0 },
    };
    for i in 0..N {
        go(rng.circle(), rng.aabb(), format!("rand #{i}"), &mut t);
        // centre inside / on a face / on a corner / just outside
        let rr = (rng.below(6) as f32) * 0.5;
        for p in [
            c2v { x: 0.0, y: 0.0 },
            c2v { x: 1.0, y: 0.0 },
            c2v { x: 1.0, y: 1.0 },
            c2v { x: 1.0 + rr, y: 0.0 },
            c2v { x: 1.0 + rr, y: 1.0 + rr },
            c2v { x: -1.0 - rr, y: 0.5 },
            c2v {
                x: rng.range(-3.0, 3.0),
                y: rng.range(-3.0, 3.0),
            },
        ] {
            go(
                c2Circle { p, r: rr },
                unit,
                format!("place #{i} p={p:?} r={rr}"),
                &mut t,
            );
            // negative radius: r2 = r*r > 0
            go(
                c2Circle { p, r: -rr },
                unit,
                format!("negrad #{i} p={p:?} r={rr}"),
                &mut t,
            );
        }
        // inverted AABB (no validation in the C)
        let q = rng.v();
        let w = rng.v();
        go(
            rng.circle(),
            c2AABB { min: w, max: q },
            format!("inverted #{i}"),
            &mut t,
        );
    }
    for &s in SPECIALS.iter() {
        for slot in 0..7 {
            let mut a = c2Circle {
                p: c2v { x: 0.5, y: 0.5 },
                r: 1.0,
            };
            let mut b = unit;
            match slot {
                0 => a.p.x = s,
                1 => a.p.y = s,
                2 => a.r = s,
                3 => b.min.x = s,
                4 => b.min.y = s,
                5 => b.max.x = s,
                _ => b.max.y = s,
            }
            go(a, b, format!("special slot={slot} s={s:?}"), &mut t);
        }
    }
    t.require_both("c2CircletoAABB");
}

// ---------------------------------------------------------------------------
// row 68 — c2CircletoCapsule (all three da/db branches)
// ---------------------------------------------------------------------------

#[test]
fn row68_c2CircletoCapsule() {
    let (c, r) = fnpair!("c2CircletoCapsule", FnCircletoCapsule);
    let (dot, _) = fnpair!("c2Dot", FnFvv);
    let (sub, _) = fnpair!("c2Sub", FnVvv);
    let mut rng = Rng::new(SEED ^ 68);
    let mut t = Tally::default();
    let mut branch = [0usize; 3];

    let go = |a: c2Circle, b: c2Capsule, ctx: String, t: &mut Tally, branch: &mut [usize; 3]| {
        // classify with the C library's own arithmetic
        let n = sub(b.b, b.a);
        let ap = sub(a.p, b.a);
        let da = dot(ap, n);
        if da < 0.0 {
            branch[0] += 1;
        } else {
            let db = dot(sub(a.p, b.b), n);
            if db < 0.0 {
                branch[1] += 1;
            } else {
                branch[2] += 1;
            }
        }
        let (cv, rv) = (c(a, b), r(a, b));
        eq_int(&format!("c2CircletoCapsule {ctx} A={a:?} B={b:?}"), cv, rv);
        t.note(cv);
    };

    for i in 0..N {
        go(
            rng.circle(),
            rng.capsule(),
            format!("rand #{i}"),
            &mut t,
            &mut branch,
        );
        // segment along +x from (0,0) to (4,0): behind / on / past
        let cap = c2Capsule {
            a: c2v { x: 0.0, y: 0.0 },
            b: c2v { x: 4.0, y: 0.0 },
            r: (rng.below(5) as f32) * 0.5,
        };
        for p in [
            c2v { x: -2.0, y: 0.0 },   // da < 0
            c2v { x: 0.0, y: 1.0 },    // da == 0 -> else branch
            c2v { x: 2.0, y: 1.0 },    // middle -> db < 0
            c2v { x: 4.0, y: 1.0 },    // db == 0 -> bp branch
            c2v { x: 6.0, y: 0.0 },    // past
            c2v {
                x: rng.range(-6.0, 10.0),
                y: rng.range(-3.0, 3.0),
            },
        ] {
            go(
                c2Circle {
                    p,
                    r: (rng.below(5) as f32) * 0.5,
                },
                cap,
                format!("seg #{i} p={p:?}"),
                &mut t,
                &mut branch,
            );
        }
        // degenerate capsule a == b: n == 0, da == 0, db == 0 -> bp branch,
        // and the /c2Dot(n,n) division is NOT reached.
        let pp = rng.v();
        go(
            rng.circle(),
            c2Capsule {
                a: pp,
                b: pp,
                r: rng.radius(),
            },
            format!("degen #{i}"),
            &mut t,
            &mut branch,
        );
    }
    for &s in SPECIALS.iter() {
        for slot in 0..8 {
            let mut a = c2Circle {
                p: c2v { x: 1.0, y: 1.0 },
                r: 1.0,
            };
            let mut b = c2Capsule {
                a: c2v { x: 0.0, y: 0.0 },
                b: c2v { x: 3.0, y: 0.0 },
                r: 0.5,
            };
            match slot {
                0 => a.p.x = s,
                1 => a.p.y = s,
                2 => a.r = s,
                3 => b.a.x = s,
                4 => b.a.y = s,
                5 => b.b.x = s,
                6 => b.b.y = s,
                _ => b.r = s,
            }
            go(
                a,
                b,
                format!("special slot={slot} s={s:?}"),
                &mut t,
                &mut branch,
            );
        }
    }
    eprintln!("[coverage] c2CircletoCapsule da/db branches = {branch:?}");
    assert!(
        branch.iter().all(|&x| x > 100),
        "not all three branches reached: {branch:?}"
    );
    t.require_both("c2CircletoCapsule");
}

// ---------------------------------------------------------------------------
// row 69 — c2AABBtoCapsule (goes through c2GJK)
// ---------------------------------------------------------------------------

#[test]
fn row69_c2AABBtoCapsule() {
    let (c, r) = fnpair!("c2AABBtoCapsule", FnAABBtoCapsule);
    let mut rng = Rng::new(SEED ^ 69);
    let mut t = Tally::default();

    let go = |a: c2AABB, b: c2Capsule, ctx: String, t: &mut Tally| {
        let (cv, rv) = (c(a, b), r(a, b));
        eq_int(&format!("c2AABBtoCapsule {ctx} A={a:?} B={b:?}"), cv, rv);
        t.note(cv);
    };

    let unit = c2AABB {
        min: c2v { x: -1.0, y: -1.0 },
        max: c2v { x: 1.0, y: 1.0 },
    };
    for i in 0..N {
        go(rng.aabb(), rng.capsule(), format!("rand #{i}"), &mut t);
        let rr = (rng.below(5) as f32) * 0.5;
        // capsule crossing the box, alongside it, far away, degenerate
        for cap in [
            c2Capsule {
                a: c2v { x: -3.0, y: 0.0 },
                b: c2v { x: 3.0, y: 0.0 },
                r: rr,
            },
            c2Capsule {
                a: c2v { x: -3.0, y: 1.0 + rr },
                b: c2v { x: 3.0, y: 1.0 + rr },
                r: rr,
            },
            c2Capsule {
                a: c2v { x: 5.0, y: 5.0 },
                b: c2v { x: 6.0, y: 6.0 },
                r: rr,
            },
            c2Capsule {
                a: c2v { x: 0.0, y: 0.0 },
                b: c2v { x: 0.0, y: 0.0 },
                r: rr,
            },
            c2Capsule {
                a: rng.v(),
                b: rng.v(),
                r: rr,
            },
        ] {
            go(unit, cap, format!("place #{i}"), &mut t);
        }
    }
    for &s in SPECIALS.iter() {
        for slot in 0..9 {
            let mut a = unit;
            let mut b = c2Capsule {
                a: c2v { x: -3.0, y: 0.0 },
                b: c2v { x: 3.0, y: 0.0 },
                r: 0.5,
            };
            match slot {
                0 => a.min.x = s,
                1 => a.min.y = s,
                2 => a.max.x = s,
                3 => a.max.y = s,
                4 => b.a.x = s,
                5 => b.a.y = s,
                6 => b.b.x = s,
                7 => b.b.y = s,
                _ => b.r = s,
            }
            go(a, b, format!("special slot={slot} s={s:?}"), &mut t);
        }
    }
    t.require_both("c2AABBtoCapsule");
}

// ---------------------------------------------------------------------------
// row 70 — c2CapsuletoCapsule (goes through c2GJK; both args MEMORY class)
// ---------------------------------------------------------------------------

#[test]
fn row70_c2CapsuletoCapsule() {
    let (c, r) = fnpair!("c2CapsuletoCapsule", FnCapsuletoCapsule);
    let mut rng = Rng::new(SEED ^ 70);
    let mut t = Tally::default();

    let go = |a: c2Capsule, b: c2Capsule, ctx: String, t: &mut Tally| {
        let (cv, rv) = (c(a, b), r(a, b));
        eq_int(&format!("c2CapsuletoCapsule {ctx} A={a:?} B={b:?}"), cv, rv);
        t.note(cv);
    };

    for i in 0..N {
        go(rng.capsule(), rng.capsule(), format!("rand #{i}"), &mut t);
        let ra = (rng.below(5) as f32) * 0.5;
        let rb = (rng.below(5) as f32) * 0.5;
        let base = c2Capsule {
            a: c2v { x: 0.0, y: 0.0 },
            b: c2v { x: 4.0, y: 0.0 },
            r: ra,
        };
        let off = (rng.below(9) as f32) * 0.25;
        for other in [
            // parallel
            c2Capsule {
                a: c2v { x: 0.0, y: off },
                b: c2v { x: 4.0, y: off },
                r: rb,
            },
            // crossing
            c2Capsule {
                a: c2v { x: 2.0, y: -2.0 },
                b: c2v { x: 2.0, y: 2.0 },
                r: rb,
            },
            // collinear, overlapping
            c2Capsule {
                a: c2v { x: 2.0, y: 0.0 },
                b: c2v { x: 6.0, y: 0.0 },
                r: rb,
            },
            // collinear, disjoint
            c2Capsule {
                a: c2v { x: 10.0, y: 0.0 },
                b: c2v { x: 14.0, y: 0.0 },
                r: rb,
            },
            // identical
            base,
            // degenerate
            c2Capsule {
                a: c2v { x: 2.0, y: off },
                b: c2v { x: 2.0, y: off },
                r: rb,
            },
        ] {
            go(base, other, format!("cfg #{i} off={off}"), &mut t);
        }
    }
    for &s in SPECIALS.iter() {
        for slot in 0..10 {
            let mut a = c2Capsule {
                a: c2v { x: 0.0, y: 0.0 },
                b: c2v { x: 4.0, y: 0.0 },
                r: 0.5,
            };
            let mut b = c2Capsule {
                a: c2v { x: 2.0, y: -2.0 },
                b: c2v { x: 2.0, y: 2.0 },
                r: 0.5,
            };
            match slot {
                0 => a.a.x = s,
                1 => a.a.y = s,
                2 => a.b.x = s,
                3 => a.b.y = s,
                4 => a.r = s,
                5 => b.a.x = s,
                6 => b.a.y = s,
                7 => b.b.x = s,
                8 => b.b.y = s,
                _ => b.r = s,
            }
            go(a, b, format!("special slot={slot} s={s:?}"), &mut t);
        }
    }
    t.require_both("c2CapsuletoCapsule");
}

// ---------------------------------------------------------------------------
// row 71 — c2Collided over all 9 correctly-typed pairs
// ---------------------------------------------------------------------------

#[test]
fn row71_c2Collided_all_pairs() {
    let (c, r) = fnpair!("c2Collided", FnCollided);
    let mut rng = Rng::new(SEED ^ 71);
    let mut tallies = [[Tally::default(), Tally::default(), Tally::default()],
                       [Tally::default(), Tally::default(), Tally::default()],
                       [Tally::default(), Tally::default(), Tally::default()]];

    for i in 0..N {
        for (ia, &ta) in ALL_TYPES.iter().enumerate() {
            for (ib, &tb) in ALL_TYPES.iter().enumerate() {
                // half "near" so both outcomes occur, half fully random
                let (ab, bb): (Vec<u8>, Vec<u8>) = if rng.bool() {
                    (
                        match ta {
                            C2_TYPE_CIRCLE => raw(&c2Circle {
                                p: c2v {
                                    x: rng.range(-2.0, 2.0),
                                    y: rng.range(-2.0, 2.0),
                                },
                                r: rng.range(0.0, 1.5),
                            })
                            .to_vec(),
                            C2_TYPE_AABB => raw(&c2AABB {
                                min: c2v { x: -1.0, y: -1.0 },
                                max: c2v { x: 1.0, y: 1.0 },
                            })
                            .to_vec(),
                            _ => raw(&c2Capsule {
                                a: c2v { x: -1.0, y: 0.0 },
                                b: c2v { x: 1.0, y: 0.0 },
                                r: rng.range(0.0, 1.0),
                            })
                            .to_vec(),
                        },
                        match tb {
                            C2_TYPE_CIRCLE => raw(&c2Circle {
                                p: c2v {
                                    x: rng.range(-3.0, 3.0),
                                    y: rng.range(-3.0, 3.0),
                                },
                                r: rng.range(0.0, 1.5),
                            })
                            .to_vec(),
                            C2_TYPE_AABB => raw(&c2AABB {
                                min: c2v {
                                    x: rng.range(-3.0, 1.0),
                                    y: rng.range(-3.0, 1.0),
                                },
                                max: c2v {
                                    x: rng.range(1.0, 3.0),
                                    y: rng.range(1.0, 3.0),
                                },
                            })
                            .to_vec(),
                            _ => raw(&c2Capsule {
                                a: c2v {
                                    x: rng.range(-3.0, 3.0),
                                    y: rng.range(-3.0, 3.0),
                                },
                                b: c2v {
                                    x: rng.range(-3.0, 3.0),
                                    y: rng.range(-3.0, 3.0),
                                },
                                r: rng.range(0.0, 1.0),
                            })
                            .to_vec(),
                        },
                    )
                } else {
                    (
                        random_shape_bytes(&mut rng, ta),
                        random_shape_bytes(&mut rng, tb),
                    )
                };
                let (cv, rv) = unsafe {
                    (
                        c(ab.as_ptr() as *const c_void, ta, bb.as_ptr() as *const c_void, tb),
                        r(ab.as_ptr() as *const c_void, ta, bb.as_ptr() as *const c_void, tb),
                    )
                };
                eq_int(
                    &format!("c2Collided #{i} ta={ta} tb={tb} A={ab:02x?} B={bb:02x?}"),
                    cv,
                    rv,
                );
                tallies[ia][ib].note(cv);
            }
        }
    }
    for (ia, &ta) in ALL_TYPES.iter().enumerate() {
        for (ib, &tb) in ALL_TYPES.iter().enumerate() {
            tallies[ia][ib].require_both(&format!("c2Collided {ta}x{tb}"));
        }
    }
}

fn random_shape_bytes(rng: &mut Rng, ty: C2_TYPE) -> Vec<u8> {
    match ty {
        C2_TYPE_CIRCLE => raw(&rng.circle()).to_vec(),
        C2_TYPE_AABB => raw(&rng.aabb()).to_vec(),
        _ => raw(&rng.capsule()).to_vec(),
    }
}
