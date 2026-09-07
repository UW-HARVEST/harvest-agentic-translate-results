//! Phase B — rows 47..76: the full `c2GJK` pipeline driven directly through
//! both `.so`s, across the cross-product of
//! `typeA × typeB × transform-kind × use_radius × cache-state × input shape`.
//!
//! `c2GJK` is the lowest-level composed entry point; the boolean helpers only
//! see a truncated view of its output, so it is exercised here on its own with
//! every out-parameter captured (`dist`, `outA`, `outB`, `*iterations`, and the
//! whole write-back `c2GJKCache`).

#![allow(non_snake_case)]

mod common;

use common::*;
use std::ffi::c_int;

const N: usize = 400;

#[derive(Copy, Clone, Debug, PartialEq)]
enum XKind {
    Null,
    Identity,
    Translate,
    Rotate,
    RotTrans,
    NonUnit,
}

fn make_x(rng: &mut Rng, k: XKind) -> Option<c2x> {
    match k {
        XKind::Null => None,
        XKind::Identity => Some(c2x {
            p: c2v { x: 0.0, y: 0.0 },
            r: c2r { c: 1.0, s: 0.0 },
        }),
        XKind::Translate => Some(c2x {
            p: rng.vec(),
            r: c2r { c: 1.0, s: 0.0 },
        }),
        XKind::Rotate => Some(c2x {
            p: c2v { x: 0.0, y: 0.0 },
            r: rng.rot(),
        }),
        XKind::RotTrans => Some(rng.xform()),
        XKind::NonUnit => Some(c2x {
            p: rng.vec(),
            r: c2r {
                c: rng.range(-3.0, 3.0),
                s: rng.range(-3.0, 3.0),
            },
        }),
    }
}

/// One differential `c2GJK` call with every observable output compared.
#[track_caller]
fn diff_gjk(
    cf: &FnGJK,
    rf: &FnGJK,
    label: &str,
    a: &Shape,
    ax: Option<&c2x>,
    b: &Shape,
    bx: Option<&c2x>,
    use_radius: c_int,
    cache: Option<c2GJKCache>,
) {
    let cr = unsafe {
        call_gjk(cf, a.ptr(), a.ty(), ax, b.ptr(), b.ty(), bx, use_radius, cache)
    };
    let rr = unsafe {
        call_gjk(rf, a.ptr(), a.ty(), ax, b.ptr(), b.ty(), bx, use_radius, cache)
    };
    if !same(&cr, &rr) {
        eprintln!("shapes: A={a:?} ax={ax:?}  B={b:?} bx={bx:?} use_radius={use_radius} cache_in={cache:?}");
    }
    assert_same(label, cr, rr);
}

// ---- rows 47..54: every type pair, NULL transforms, both use_radius -------

#[test]
fn cfg47_54_type_pairs_null_transforms() {
    let (cf, rf) = pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(0x4747);
    for (ta, tb) in TYPE_PAIRS {
        for _ in 0..N {
            let a = rand_shape(&mut rng, ta);
            let b = rand_shape(&mut rng, tb);
            for ur in [0, 1] {
                diff_gjk(
                    &cf,
                    &rf,
                    &format!("c2GJK({ta},{tb}) null xf ur={ur}"),
                    &a,
                    None,
                    &b,
                    None,
                    ur,
                    None,
                );
            }
        }
    }
}

// ---- rows 55..59: transform kinds ---------------------------------------

#[test]
fn cfg55_59_transform_kinds() {
    let (cf, rf) = pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(0x5555);
    let combos: [(XKind, XKind); 8] = [
        (XKind::Identity, XKind::Identity), // row 55
        (XKind::Translate, XKind::Null),    // row 56
        (XKind::Null, XKind::Rotate),       // row 57
        (XKind::RotTrans, XKind::RotTrans), // row 58
        (XKind::NonUnit, XKind::NonUnit),   // row 59
        (XKind::Identity, XKind::Null),
        (XKind::Rotate, XKind::Translate),
        (XKind::NonUnit, XKind::RotTrans),
    ];
    for (ta, tb) in TYPE_PAIRS {
        for (ka, kb) in combos {
            for _ in 0..(N / 4).max(40) {
                let a = rand_shape(&mut rng, ta);
                let b = rand_shape(&mut rng, tb);
                let ax = make_x(&mut rng, ka);
                let bx = make_x(&mut rng, kb);
                for ur in [0, 1] {
                    diff_gjk(
                        &cf,
                        &rf,
                        &format!("c2GJK({ta},{tb}) xf={ka:?}/{kb:?} ur={ur}"),
                        &a,
                        ax.as_ref(),
                        &b,
                        bx.as_ref(),
                        ur,
                        None,
                    );
                }
            }
        }
    }
}

// ---- rows 60..63: separation regimes ------------------------------------

/// Place `b` at a controlled offset from `a` to force overlap / touch / far.
fn shifted(s: &Shape, dx: f32, dy: f32) -> Shape {
    let mv = |v: c2v| c2v { x: v.x + dx, y: v.y + dy };
    match *s {
        Shape::Circle(c) => Shape::Circle(c2Circle { p: mv(c.p), r: c.r }),
        Shape::Aabb(bb) => Shape::Aabb(c2AABB {
            min: mv(bb.min),
            max: mv(bb.max),
        }),
        Shape::Capsule(c) => Shape::Capsule(c2Capsule {
            a: mv(c.a),
            b: mv(c.b),
            r: c.r,
        }),
    }
}

#[test]
fn cfg60_63_separation_regimes() {
    let (cf, rf) = pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(0x6060);
    for (ta, tb) in TYPE_PAIRS {
        for _ in 0..N {
            let a = rand_shape(&mut rng, ta);
            // row 60 — deep overlap (drives the hit path, s.count == 3)
            let overlap = shifted(&a, rng.range(-1.0, 1.0), rng.range(-1.0, 1.0));
            let overlap = match tb {
                C2_TYPE_CIRCLE => Shape::Circle(c2Circle {
                    p: match overlap {
                        Shape::Circle(c) => c.p,
                        Shape::Aabb(bb) => bb.min,
                        Shape::Capsule(c) => c.a,
                    },
                    r: rng.radius(),
                }),
                C2_TYPE_AABB => {
                    let p = match overlap {
                        Shape::Circle(c) => c.p,
                        Shape::Aabb(bb) => bb.min,
                        Shape::Capsule(c) => c.a,
                    };
                    Shape::Aabb(c2AABB {
                        min: c2v { x: p.x - 5.0, y: p.y - 5.0 },
                        max: c2v { x: p.x + 5.0, y: p.y + 5.0 },
                    })
                }
                _ => {
                    let p = match overlap {
                        Shape::Circle(c) => c.p,
                        Shape::Aabb(bb) => bb.min,
                        Shape::Capsule(c) => c.a,
                    };
                    Shape::Capsule(c2Capsule {
                        a: c2v { x: p.x - 3.0, y: p.y },
                        b: c2v { x: p.x + 3.0, y: p.y },
                        r: rng.radius(),
                    })
                }
            };
            for ur in [0, 1] {
                diff_gjk(&cf, &rf, "c2GJK overlap", &a, None, &overlap, None, ur, None);
            }

            // row 62 — far separation
            let far = shifted(&overlap, 5000.0, -7000.0);
            for ur in [0, 1] {
                diff_gjk(&cf, &rf, "c2GJK far", &a, None, &far, None, ur, None);
            }

            // row 63 — coincident (A and B identical)
            for ur in [0, 1] {
                diff_gjk(&cf, &rf, "c2GJK coincident", &a, None, &a, None, ur, None);
            }
        }
    }

    // row 61 — exact touching: dist == rA + rB
    for _ in 0..(N * 4) {
        let ra = rng.radius();
        let rb = rng.radius();
        let a = Shape::Circle(c2Circle {
            p: c2v { x: 0.0, y: 0.0 },
            r: ra,
        });
        let b = Shape::Circle(c2Circle {
            p: c2v { x: ra + rb, y: 0.0 },
            r: rb,
        });
        for ur in [0, 1] {
            diff_gjk(&cf, &rf, "c2GJK touching circles", &a, None, &b, None, ur, None);
        }
        let ca = Shape::Capsule(c2Capsule {
            a: c2v { x: -10.0, y: 0.0 },
            b: c2v { x: 10.0, y: 0.0 },
            r: ra,
        });
        let cb = Shape::Capsule(c2Capsule {
            a: c2v { x: -10.0, y: ra + rb },
            b: c2v { x: 10.0, y: ra + rb },
            r: rb,
        });
        for ur in [0, 1] {
            diff_gjk(&cf, &rf, "c2GJK touching capsules", &ca, None, &cb, None, ur, None);
        }
    }
}

// ---- rows 64..68: degenerate / extreme geometry --------------------------

#[test]
fn cfg64_68_degenerate_and_extreme() {
    let (cf, rf) = pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(0x6464);

    // row 64 — zero-size shapes
    for _ in 0..(N * 2) {
        let p = rng.vec();
        let q = rng.vec();
        let zc = Shape::Circle(c2Circle { p, r: 0.0 });
        let zb = Shape::Aabb(c2AABB { min: q, max: q });
        let zk = Shape::Capsule(c2Capsule { a: p, b: p, r: 0.0 });
        for a in [&zc, &zb, &zk] {
            for b in [&zc, &zb, &zk] {
                for ur in [0, 1] {
                    diff_gjk(&cf, &rf, "c2GJK zero-size", a, None, b, None, ur, None);
                }
            }
        }
    }

    // row 65 — inverted AABB
    for _ in 0..(N * 2) {
        let bb = rng.aabb();
        let inv = Shape::Aabb(c2AABB {
            min: bb.max,
            max: bb.min,
        });
        for (_, tb) in TYPE_PAIRS {
            let b = rand_shape(&mut rng, tb);
            for ur in [0, 1] {
                diff_gjk(&cf, &rf, "c2GJK inverted A", &inv, None, &b, None, ur, None);
                diff_gjk(&cf, &rf, "c2GJK inverted B", &b, None, &inv, None, ur, None);
            }
        }
    }

    // row 66 — negative radii
    for _ in 0..(N * 2) {
        let nc = Shape::Circle(c2Circle {
            p: rng.vec(),
            r: -rng.radius(),
        });
        let nk = Shape::Capsule(c2Capsule {
            a: rng.vec(),
            b: rng.vec(),
            r: -rng.radius(),
        });
        let bb = Shape::Aabb(rng.aabb());
        for a in [&nc, &nk] {
            for b in [&nc, &nk, &bb] {
                for ur in [0, 1] {
                    diff_gjk(&cf, &rf, "c2GJK negative radius", a, None, b, None, ur, None);
                }
            }
        }
    }

    // rows 67 & 68 — subnormal and huge scales
    for scale in [1e-30f32, 1e-20, 1e20, 1e30] {
        for _ in 0..(N * 2) {
            let sc = |v: c2v| c2v {
                x: v.x * scale,
                y: v.y * scale,
            };
            let a = Shape::Circle(c2Circle {
                p: sc(rng.vec()),
                r: rng.radius() * scale,
            });
            let b = Shape::Aabb(c2AABB {
                min: sc(rng.aabb().min),
                max: sc(rng.aabb().max),
            });
            let k = Shape::Capsule(c2Capsule {
                a: sc(rng.vec()),
                b: sc(rng.vec()),
                r: rng.radius() * scale,
            });
            for x in [&a, &b, &k] {
                for y in [&a, &b, &k] {
                    for ur in [0, 1] {
                        diff_gjk(&cf, &rf, "c2GJK scaled", x, None, y, None, ur, None);
                    }
                }
            }
        }
    }
}

// ---- rows 69..73: cache states ------------------------------------------

#[test]
fn cfg69_73_cache_states() {
    let (cf, rf) = pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(0x6969);

    // row 69 — NULL cache vs. zeroed cache; the whole 36-byte write-back is
    // compared by diff_gjk.
    for (ta, tb) in TYPE_PAIRS {
        for _ in 0..N {
            let a = rand_shape(&mut rng, ta);
            let b = rand_shape(&mut rng, tb);
            for ur in [0, 1] {
                diff_gjk(&cf, &rf, "c2GJK cache=NULL", &a, None, &b, None, ur, None);
                diff_gjk(
                    &cf,
                    &rf,
                    "c2GJK cache=zeroed",
                    &a,
                    None,
                    &b,
                    None,
                    ur,
                    Some(c2GJKCache::default()),
                );
            }
        }
    }

    // rows 70 & 71 — a real cached sequence: call repeatedly through the SAME
    // library, carrying the cache forward, and compare the cache and every
    // output at each step. Shapes are re-used verbatim (row 70) and moved
    // between steps (row 71).
    let run_sequence = |f: &FnGJK, a0: &Shape, b0: &Shape, ur: c_int, moving: bool| {
        let mut cache = c2GJKCache::default();
        let mut trace: Vec<GjkResult> = Vec::new();
        for step in 0..6 {
            let (a, b) = if moving {
                (
                    shifted(a0, step as f32 * 3.0, 0.0),
                    shifted(b0, 0.0, step as f32 * -2.0),
                )
            } else {
                (*a0, *b0)
            };
            let r = unsafe {
                call_gjk(
                    f,
                    a.ptr(),
                    a.ty(),
                    None,
                    b.ptr(),
                    b.ty(),
                    None,
                    ur,
                    Some(cache),
                )
            };
            cache = r.cache;
            trace.push(r);
        }
        trace
    };

    for (ta, tb) in TYPE_PAIRS {
        for _ in 0..(N / 2).max(40) {
            let a = rand_shape(&mut rng, ta);
            let b = rand_shape(&mut rng, tb);
            for ur in [0, 1] {
                for moving in [false, true] {
                    let ct = run_sequence(&cf, &a, &b, ur, moving);
                    let rt = run_sequence(&rf, &a, &b, ur, moving);
                    assert_eq!(ct.len(), rt.len());
                    for (i, (cv, rv)) in ct.iter().zip(rt.iter()).enumerate() {
                        if !same(cv, rv) {
                            eprintln!("cached seq step {i}: A={a:?} B={b:?} ur={ur} moving={moving}");
                        }
                        assert_same(&format!("c2GJK cached step {i}"), *cv, *rv);
                    }
                }
            }
        }
    }

    // row 72 — hand-built warm caches with every count and index permutation
    // that is in range for the corresponding proxy (circle 1, capsule 2,
    // AABB 4 vertices).
    let vert_count = |t: c_int| match t {
        C2_TYPE_CIRCLE => 1,
        C2_TYPE_CAPSULE => 2,
        _ => 4,
    };
    for (ta, tb) in TYPE_PAIRS {
        let (na, nb) = (vert_count(ta), vert_count(tb));
        for _ in 0..(N / 4).max(20) {
            let a = rand_shape(&mut rng, ta);
            let b = rand_shape(&mut rng, tb);
            for count in 1..=3 {
                for _ in 0..8 {
                    let mut cache = c2GJKCache {
                        metric: rng.range(-50.0, 50.0),
                        count,
                        iA: [0; 3],
                        iB: [0; 3],
                        div: rng.range(0.1, 5.0),
                    };
                    for k in 0..3 {
                        cache.iA[k] = (rng.next_u32() % na) as c_int;
                        cache.iB[k] = (rng.next_u32() % nb) as c_int;
                    }
                    for ur in [0, 1] {
                        diff_gjk(
                            &cf,
                            &rf,
                            &format!("c2GJK warm cache count={count}"),
                            &a,
                            None,
                            &b,
                            None,
                            ur,
                            Some(cache),
                        );
                    }
                }
            }
        }
    }

    // row 73 — warm cache + non-NULL transforms + use_radius, all together.
    for (ta, tb) in TYPE_PAIRS {
        let (na, nb) = (vert_count(ta), vert_count(tb));
        for _ in 0..(N / 4).max(20) {
            let a = rand_shape(&mut rng, ta);
            let b = rand_shape(&mut rng, tb);
            let ax = rng.xform();
            let bx = rng.xform();
            let mut cache = c2GJKCache {
                metric: rng.range(-1e6, 1e6),
                count: 1 + (rng.next_u32() % 3) as c_int,
                iA: [0; 3],
                iB: [0; 3],
                div: rng.range(-5.0, 5.0),
            };
            for k in 0..3 {
                cache.iA[k] = (rng.next_u32() % na) as c_int;
                cache.iB[k] = (rng.next_u32() % nb) as c_int;
            }
            diff_gjk(
                &cf,
                &rf,
                "c2GJK warm+xform+radius",
                &a,
                Some(&ax),
                &b,
                Some(&bx),
                1,
                Some(cache),
            );
            diff_gjk(
                &cf,
                &rf,
                "c2GJK warm+xform+noradius",
                &a,
                Some(&ax),
                &b,
                Some(&bx),
                0,
                Some(cache),
            );
        }
    }
}

// ---- rows 74..76: out-parameters and radius combinations ----------------

#[test]
fn cfg74_76_outparams_and_radius_combos() {
    let (cf, rf) = pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(0x7474);

    // rows 74 & 75 are covered by every diff_gjk call (iterations, outA, outB
    // are all captured and compared); here they are additionally checked with
    // a histogram over the iteration counts so we know the loop really varies.
    let mut iter_hist = [0usize; 22];
    for (ta, tb) in TYPE_PAIRS {
        for _ in 0..(N * 2) {
            let a = rand_shape(&mut rng, ta);
            let b = rand_shape(&mut rng, tb);
            let ur = (rng.next_u32() % 2) as c_int;
            let cr = unsafe {
                call_gjk(&cf, a.ptr(), a.ty(), None, b.ptr(), b.ty(), None, ur, None)
            };
            let rr = unsafe {
                call_gjk(&rf, a.ptr(), a.ty(), None, b.ptr(), b.ty(), None, ur, None)
            };
            assert_same("c2GJK outparams", cr, rr);
            if (0..=21).contains(&cr.iterations) {
                iter_hist[cr.iterations as usize] += 1;
            }
        }
    }
    eprintln!("c2GJK iteration histogram: {iter_hist:?}");
    assert!(
        iter_hist.iter().filter(|&&h| h > 0).count() >= 3,
        "iteration counts did not vary: {iter_hist:?}"
    );

    // row 76 — the four (rA, rB) zero/non-zero combinations under use_radius.
    for _ in 0..(N * 4) {
        for (ra, rb) in [(0.0f32, 0.0f32), (0.0, 13.0), (17.0, 0.0), (11.0, 7.0)] {
            let a = Shape::Circle(c2Circle { p: rng.vec(), r: ra });
            let b = Shape::Capsule(c2Capsule {
                a: rng.vec(),
                b: rng.vec(),
                r: rb,
            });
            for ur in [0, 1] {
                diff_gjk(&cf, &rf, "c2GJK radius combo", &a, None, &b, None, ur, None);
            }
        }
    }
}
