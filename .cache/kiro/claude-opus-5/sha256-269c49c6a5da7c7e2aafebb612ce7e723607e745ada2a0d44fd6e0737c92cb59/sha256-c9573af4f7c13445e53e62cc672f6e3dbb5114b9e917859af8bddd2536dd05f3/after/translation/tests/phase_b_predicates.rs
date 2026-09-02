//! Phase B — CONFIGS.md rows 13–15: the boolean predicates.
//! Also contains the harness self-check (negative control) proving the two
//! libraries really are two distinct shared objects and that `Diff` can fail.

#![allow(non_snake_case)]

mod common;
use common::*;

const SEED: u64 = 0x5EED_C2A1;
const N: usize = 20_000;

/// Negative control: without this, a harness bug that loaded the same `.so`
/// twice, or a `Diff` that never reports, would make every other test vacuous.
#[test]
fn harness_self_check() {
    let l = libs();
    assert_ne!(
        l.c.path.canonicalize().unwrap(),
        l.r.path.canonicalize().unwrap(),
        "C and Rust .so must be different files"
    );
    assert!(
        l.c.path.to_string_lossy().contains("c_src"),
        "C library must come from c_src/build, got {}",
        l.c.path.display()
    );
    assert!(
        l.r.path.to_string_lossy().contains("gen_ray_lib"),
        "Rust library must be libgen_ray_lib.so, got {}",
        l.r.path.display()
    );

    // `Diff` must actually detect a divergence.
    let mut d = Diff::new("control");
    d.check_f(1.0, 2.0, || "deliberate".into());
    assert_eq!(d.fails.len(), 1, "Diff failed to record a divergence");
    let caught = std::panic::catch_unwind(move || d.finish()).is_err();
    assert!(caught, "Diff::finish must panic on a recorded divergence");

    // Bit comparison must reject same-value-different-bits.
    assert!(!bits_eq(0.0, -0.0));
    assert!(!bits_eq(
        f32::from_bits(0x7fc0_0000),
        f32::from_bits(0xffc0_0000)
    ));
    assert!(bits_eq(f32::NAN, f32::NAN));

    eprintln!(
        "  harness ok: C={} Rust={}",
        l.c.path.display(),
        l.r.path.display()
    );
}

fn aabb(minx: f32, miny: f32, maxx: f32, maxy: f32) -> c2AABB {
    c2AABB {
        min: c2v { x: minx, y: miny },
        max: c2v { x: maxx, y: maxy },
    }
}

fn fmt_box(b: c2AABB) -> String {
    format!("[min {} max {}]", fmt_v(b.min), fmt_v(b.max))
}

/// Row 13 — `c2AABBtoAABB`: overlap, touch, contain, all 4 separations,
/// inverted boxes, NaN coordinates.
#[test]
fn cfg_13_aabb_to_aabb() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 13);
    let mut d = Diff::new("row13 c2AABBtoAABB");

    let base = aabb(-1.0, -1.0, 1.0, 1.0);
    let mut cases: Vec<(c2AABB, c2AABB)> = vec![
        (base, base),                              // identical
        (base, aabb(0.5, 0.5, 2.0, 2.0)),          // overlapping
        (base, aabb(1.0, -1.0, 3.0, 1.0)),         // touching edge (max==min)
        (base, aabb(1.0, 1.0, 3.0, 3.0)),          // touching corner
        (base, aabb(-0.5, -0.5, 0.5, 0.5)),        // fully contained
        (base, aabb(2.0, -1.0, 3.0, 1.0)),         // separated d1
        (base, aabb(-3.0, -1.0, -2.0, 1.0)),       // separated d0
        (base, aabb(-1.0, 2.0, 1.0, 3.0)),         // separated d3
        (base, aabb(-1.0, -3.0, 1.0, -2.0)),       // separated d2
        (aabb(1.0, 1.0, -1.0, -1.0), base),        // inverted A
        (base, aabb(1.0, 1.0, -1.0, -1.0)),        // inverted B
        (aabb(0.0, 0.0, 0.0, 0.0), base),          // degenerate point box
        (base, aabb(f32::NAN, 0.0, 1.0, 1.0)),     // NaN min.x
        (aabb(f32::NAN, f32::NAN, f32::NAN, f32::NAN), base),
        (
            aabb(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::INFINITY, f32::INFINITY),
            base,
        ),
        (base, aabb(-0.0, -0.0, 0.0, 0.0)),
    ];
    // Exhaustive one-field-special sweep on both boxes.
    for &s in SPECIALS {
        for i in 0..4 {
            let mut b = base;
            match i {
                0 => b.min.x = s,
                1 => b.min.y = s,
                2 => b.max.x = s,
                _ => b.max.y = s,
            }
            cases.push((base, b));
            cases.push((b, base));
        }
    }
    // Randomised: boxes drawn from a range that makes overlap ~50% likely.
    for _ in 0..N {
        let ax = rng.range(-10.0, 10.0);
        let ay = rng.range(-10.0, 10.0);
        let bx = rng.range(-10.0, 10.0);
        let by = rng.range(-10.0, 10.0);
        cases.push((
            aabb(ax, ay, ax + rng.range(0.0, 8.0), ay + rng.range(0.0, 8.0)),
            aabb(bx, by, bx + rng.range(0.0, 8.0), by + rng.range(0.0, 8.0)),
        ));
    }
    // Randomised with arbitrary (possibly inverted / special) corners.
    for _ in 0..N {
        cases.push((
            c2AABB {
                min: rng.vec_spicy(),
                max: rng.vec_spicy(),
            },
            c2AABB {
                min: rng.vec_spicy(),
                max: rng.vec_spicy(),
            },
        ));
    }

    let (mut n_true, mut n_false) = (0usize, 0usize);
    for (a, b) in cases {
        let cr = unsafe { (l.c.c2AABBtoAABB)(a, b) };
        let rr = unsafe { (l.r.c2AABBtoAABB)(a, b) };
        if cr != 0 {
            n_true += 1
        } else {
            n_false += 1
        }
        d.check_i(cr, rr, || {
            format!("c2AABBtoAABB({}, {})", fmt_box(a), fmt_box(b))
        });
    }
    assert!(n_true > 100 && n_false > 100, "poor branch coverage: {n_true}/{n_false}");
    d.finish();
}

/// Row 14 — `c2AABBtoPoint`: inside, exactly on each edge, outside each side.
#[test]
fn cfg_14_aabb_to_point() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 14);
    let mut d = Diff::new("row14 c2AABBtoPoint");

    let b = aabb(-2.0, -3.0, 4.0, 5.0);
    let mut cases: Vec<(c2AABB, c2v)> = vec![
        (b, c2v { x: 0.0, y: 0.0 }),   // inside
        (b, c2v { x: -2.0, y: 0.0 }),  // exactly on min.x  (`<` false -> inside)
        (b, c2v { x: 0.0, y: -3.0 }),  // exactly on min.y
        (b, c2v { x: 4.0, y: 0.0 }),   // exactly on max.x  (`>` false -> inside)
        (b, c2v { x: 0.0, y: 5.0 }),   // exactly on max.y
        (b, c2v { x: -2.000001, y: 0.0 }),
        (b, c2v { x: 0.0, y: -3.000001 }),
        (b, c2v { x: 4.000001, y: 0.0 }),
        (b, c2v { x: 0.0, y: 5.000001 }),
        (b, c2v { x: -2.0, y: -3.0 }), // exact min corner
        (b, c2v { x: 4.0, y: 5.0 }),   // exact max corner
        (b, c2v { x: f32::NAN, y: 0.0 }),
        (b, c2v { x: 0.0, y: f32::NAN }),
        (aabb(4.0, 5.0, -2.0, -3.0), c2v { x: 0.0, y: 0.0 }), // inverted box
        (aabb(0.0, 0.0, 0.0, 0.0), c2v { x: 0.0, y: 0.0 }),   // point box, exact hit
        (aabb(0.0, 0.0, 0.0, 0.0), c2v { x: -0.0, y: -0.0 }),
    ];
    // One ULP either side of every edge, both directions.
    for &(edge, comp) in &[(-2.0f32, 0usize), (-3.0, 1), (4.0, 2), (5.0, 3)] {
        for delta in [-1i32, 0, 1] {
            let v = if delta == 0 {
                edge
            } else if delta < 0 {
                f32::from_bits(edge.to_bits().wrapping_sub(1))
            } else {
                f32::from_bits(edge.to_bits().wrapping_add(1))
            };
            let p = match comp {
                0 | 2 => c2v { x: v, y: 0.0 },
                _ => c2v { x: 0.0, y: v },
            };
            cases.push((b, p));
        }
    }
    for &s in SPECIALS {
        cases.push((b, c2v { x: s, y: 0.0 }));
        cases.push((b, c2v { x: 0.0, y: s }));
        cases.push((b, c2v { x: s, y: s }));
        let mut bb = b;
        bb.min.x = s;
        cases.push((bb, c2v { x: 0.0, y: 0.0 }));
        let mut bb2 = b;
        bb2.max.y = s;
        cases.push((bb2, c2v { x: 0.0, y: 0.0 }));
    }
    for _ in 0..N {
        cases.push((b, c2v { x: rng.range(-6.0, 8.0), y: rng.range(-7.0, 9.0) }));
    }
    for _ in 0..N {
        cases.push((
            c2AABB { min: rng.vec_spicy(), max: rng.vec_spicy() },
            rng.vec_spicy(),
        ));
    }

    let (mut t, mut f) = (0usize, 0usize);
    for (bx, p) in cases {
        let cr = unsafe { (l.c.c2AABBtoPoint)(bx, p) };
        let rr = unsafe { (l.r.c2AABBtoPoint)(bx, p) };
        if cr != 0 { t += 1 } else { f += 1 }
        d.check_i(cr, rr, || {
            format!("c2AABBtoPoint({}, {})", fmt_box(bx), fmt_v(p))
        });
    }
    assert!(t > 100 && f > 100, "poor branch coverage: {t}/{f}");
    d.finish();
}

/// Row 15 — `c2CircleToPoint`: strict `<` means the boundary is OUTSIDE.
#[test]
fn cfg_15_circle_to_point() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 15);
    let mut d = Diff::new("row15 c2CircleToPoint");

    let mk = |px: f32, py: f32, r: f32| c2Circle {
        p: c2v { x: px, y: py },
        r,
    };
    let mut cases: Vec<(c2Circle, c2v)> = vec![
        (mk(0.0, 0.0, 2.0), c2v { x: 0.0, y: 0.0 }),  // centre
        (mk(0.0, 0.0, 2.0), c2v { x: 2.0, y: 0.0 }),  // exactly on boundary -> 0
        (mk(0.0, 0.0, 2.0), c2v { x: 1.999999, y: 0.0 }),
        (mk(0.0, 0.0, 2.0), c2v { x: 2.000001, y: 0.0 }),
        (mk(0.0, 0.0, 0.0), c2v { x: 0.0, y: 0.0 }),  // r == 0 -> always 0
        (mk(0.0, 0.0, -2.0), c2v { x: 1.0, y: 0.0 }), // negative r behaves as +r
        (mk(0.0, 0.0, 2.0), c2v { x: f32::NAN, y: 0.0 }),
        (mk(f32::NAN, 0.0, 2.0), c2v { x: 0.0, y: 0.0 }),
        (mk(0.0, 0.0, f32::NAN), c2v { x: 0.0, y: 0.0 }),
        (mk(0.0, 0.0, f32::INFINITY), c2v { x: 1e30, y: 1e30 }),
        (mk(0.0, 0.0, 1e30), c2v { x: 0.0, y: 0.0 }), // r*r overflows to inf
        (mk(0.0, 0.0, 1e-30), c2v { x: 0.0, y: 0.0 }), // r*r underflows to 0
    ];
    // Exactly-on-boundary for many radii (the `<` vs `<=` distinction).
    for i in 1..200u32 {
        let r = i as f32 * 0.37;
        cases.push((mk(0.0, 0.0, r), c2v { x: r, y: 0.0 }));
        cases.push((mk(0.0, 0.0, r), c2v { x: -r, y: 0.0 }));
        cases.push((mk(0.0, 0.0, r), c2v { x: 0.0, y: r }));
    }
    for &s in SPECIALS {
        cases.push((mk(0.0, 0.0, s), c2v { x: 1.0, y: 1.0 }));
        cases.push((mk(s, s, 2.0), c2v { x: 1.0, y: 1.0 }));
        cases.push((mk(0.0, 0.0, 2.0), c2v { x: s, y: s }));
    }
    for _ in 0..N {
        let c = mk(rng.coord(), rng.coord(), rng.radius());
        cases.push((
            c,
            c2v {
                x: c.p.x + rng.range(-c.r * 1.5, c.r * 1.5),
                y: c.p.y + rng.range(-c.r * 1.5, c.r * 1.5),
            },
        ));
    }
    for _ in 0..N {
        cases.push((
            c2Circle { p: rng.vec_spicy(), r: rng.spicy() },
            rng.vec_spicy(),
        ));
    }

    let (mut t, mut f) = (0usize, 0usize);
    for (c, p) in cases {
        let cr = unsafe { (l.c.c2CircleToPoint)(c, p) };
        let rr = unsafe { (l.r.c2CircleToPoint)(c, p) };
        if cr != 0 { t += 1 } else { f += 1 }
        d.check_i(cr, rr, || {
            format!(
                "c2CircleToPoint(p={} r={}, {})",
                fmt_v(c.p),
                fmt_f(c.r),
                fmt_v(p)
            )
        });
    }
    assert!(t > 100 && f > 100, "poor branch coverage: {t}/{f}");
    d.finish();
}
