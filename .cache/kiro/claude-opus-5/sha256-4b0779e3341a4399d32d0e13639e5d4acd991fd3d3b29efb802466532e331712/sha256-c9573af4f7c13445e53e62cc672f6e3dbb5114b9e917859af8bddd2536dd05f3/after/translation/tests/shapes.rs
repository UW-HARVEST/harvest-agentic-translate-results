//! Phase B — CONFIGS.md rows 68..88: the six shape-pair predicates, the
//! `c2Collided` dispatcher (including the three pairs where the C **swaps** its
//! arguments), and the `reverse_collide` public entry point.

mod common;
use common::*;
use std::ffi::c_void;

const N: usize = 30_000;

type FnAABBtoAABB = unsafe extern "C" fn(C2AABB, C2AABB) -> i32;
type FnAABBtoCapsule = unsafe extern "C" fn(C2AABB, C2Capsule) -> i32;
type FnCapsuletoCapsule = unsafe extern "C" fn(C2Capsule, C2Capsule) -> i32;
type FnCircletoCircle = unsafe extern "C" fn(C2Circle, C2Circle) -> i32;
type FnCircletoAABB = unsafe extern "C" fn(C2Circle, C2AABB) -> i32;
type FnCircletoCapsule = unsafe extern "C" fn(C2Circle, C2Capsule) -> i32;
type FnCollided = unsafe extern "C" fn(*const c_void, i32, *const c_void, i32) -> i32;
type FnReverse = unsafe extern "C" fn(f32, f32, f32) -> i32;

/// row 68 — c2AABBtoAABB: separated on each side, overlapping, nested,
/// edge-touching, zero-extent, inverted, NaN.
#[test]
fn row68_aabb_to_aabb() {
    let l = libs();
    let (c, r) = l.pair::<FnAABBtoAABB>("c2AABBtoAABB");
    let mut rng = Rng::new(0x5A_0068);
    let mut yes = 0usize;
    let mut no = 0usize;

    let mut cases: Vec<(C2AABB, C2AABB)> = Vec::new();
    for a in degenerate_aabbs() {
        for b in degenerate_aabbs() {
            cases.push((a, b));
        }
    }
    // Deterministic separated-on-each-side and edge-touching cases.
    let base = C2AABB { min: C2v { x: 0.0, y: 0.0 }, max: C2v { x: 10.0, y: 10.0 } };
    for &(dx, dy) in &[
        (20.0f32, 0.0f32), (-20.0, 0.0), (0.0, 20.0), (0.0, -20.0),
        (10.0, 0.0), (-10.0, 0.0), (0.0, 10.0), (0.0, -10.0), // exactly touching
        (5.0, 5.0), (0.0, 0.0), (2.0, 2.0),
    ] {
        cases.push((
            base,
            C2AABB {
                min: C2v { x: dx, y: dy },
                max: C2v { x: dx + 10.0, y: dy + 10.0 },
            },
        ));
    }
    // Nested.
    cases.push((base, C2AABB { min: C2v { x: 2.0, y: 2.0 }, max: C2v { x: 3.0, y: 3.0 } }));
    for i in 0..N {
        let a = if i % 4 == 0 {
            C2AABB { min: rng.v_mixed(), max: rng.v_mixed() }
        } else {
            rng.aabb()
        };
        let b = if i % 5 == 0 {
            C2AABB { min: rng.v_any(), max: rng.v_any() }
        } else {
            rng.aabb()
        };
        cases.push((a, b));
    }

    for (a, b) in cases {
        let (cv, rv) = unsafe { (c(a, b), r(a, b)) };
        same_i32("c2AABBtoAABB", &(a, b), cv, rv);
        if cv != 0 { yes += 1 } else { no += 1 }
    }
    assert!(yes > 0 && no > 0, "one-sided result: yes={yes} no={no}");
}

/// row 69 — c2CircletoCircle
#[test]
fn row69_circle_to_circle() {
    let l = libs();
    let (c, r) = l.pair::<FnCircletoCircle>("c2CircletoCircle");
    let mut rng = Rng::new(0x5A_0069);
    let mut yes = 0usize;
    let mut no = 0usize;

    let mut cases: Vec<(C2Circle, C2Circle)> = Vec::new();
    for a in degenerate_circles() {
        for b in degenerate_circles() {
            cases.push((a, b));
        }
    }
    // Exactly touching, just inside, just outside.
    for &(d, r1, r2) in &[
        (4.0f32, 2.0f32, 2.0f32), // exactly touching -> d2 == r2, `<` is false
        (3.9, 2.0, 2.0),
        (4.1, 2.0, 2.0),
        (0.0, 2.0, 2.0),
        (0.0, 0.0, 0.0),
        (1.0, -5.0, 2.0), // negative radius
        (1.0, -1.0, -1.0),
    ] {
        cases.push((
            C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: r1 },
            C2Circle { p: C2v { x: d, y: 0.0 }, r: r2 },
        ));
    }
    for i in 0..N {
        let a = if i % 5 == 0 {
            C2Circle { p: rng.v_mixed(), r: rng.f32_mixed() }
        } else {
            rng.circle()
        };
        let b = if i % 7 == 0 {
            C2Circle { p: rng.v_any(), r: rng.any_f32() }
        } else {
            rng.circle()
        };
        cases.push((a, b));
    }

    for (a, b) in cases {
        let (cv, rv) = unsafe { (c(a, b), r(a, b)) };
        same_i32("c2CircletoCircle", &(a, b), cv, rv);
        if cv != 0 { yes += 1 } else { no += 1 }
    }
    assert!(yes > 0 && no > 0, "one-sided result: yes={yes} no={no}");
}

/// row 70 — c2CircletoAABB: inside, each face, each corner, on the boundary
#[test]
fn row70_circle_to_aabb() {
    let l = libs();
    let (c, r) = l.pair::<FnCircletoAABB>("c2CircletoAABB");
    let mut rng = Rng::new(0x5A_0070);
    let mut yes = 0usize;
    let mut no = 0usize;

    let mut cases: Vec<(C2Circle, C2AABB)> = Vec::new();
    for a in degenerate_circles() {
        for b in degenerate_aabbs() {
            cases.push((a, b));
        }
    }
    let bb = C2AABB { min: C2v { x: -10.0, y: -10.0 }, max: C2v { x: 10.0, y: 10.0 } };
    for &(x, y) in &[
        (0.0f32, 0.0f32),   // centre inside
        (15.0, 0.0), (-15.0, 0.0), (0.0, 15.0), (0.0, -15.0),  // faces
        (15.0, 15.0), (-15.0, 15.0), (15.0, -15.0), (-15.0, -15.0), // corners
        (10.0, 10.0), (-10.0, -10.0), // exactly on a corner -> d2 == 0
        (13.0, 0.0), // exactly r away from the face -> d2 == r2, `<` false
    ] {
        for &rad in &[0.0f32, 1.0, 3.0, 5.0, -3.0] {
            cases.push((C2Circle { p: C2v { x, y }, r: rad }, bb));
        }
    }
    for i in 0..N {
        let a = if i % 5 == 0 {
            C2Circle { p: rng.v_mixed(), r: rng.f32_mixed() }
        } else {
            rng.circle()
        };
        let b = if i % 6 == 0 {
            C2AABB { min: rng.v_mixed(), max: rng.v_mixed() }
        } else {
            rng.aabb()
        };
        cases.push((a, b));
    }

    for (a, b) in cases {
        let (cv, rv) = unsafe { (c(a, b), r(a, b)) };
        same_i32("c2CircletoAABB", &(a, b), cv, rv);
        if cv != 0 { yes += 1 } else { no += 1 }
    }
    assert!(yes > 0 && no > 0, "one-sided result: yes={yes} no={no}");
}

/// rows 71,72,73,74 — c2CircletoCapsule with all three arms proven to be hit
#[test]
fn row71_74_circle_to_capsule() {
    let l = libs();
    let (c, r) = l.pair::<FnCircletoCapsule>("c2CircletoCapsule");
    let mut rng = Rng::new(0x5A_0071);
    let mut arms = [0usize; 3];
    let mut yes = 0usize;
    let mut no = 0usize;

    // Mirror of the C's branch structure (lib.c c2CircletoCapsule).
    fn arm(a: C2Circle, b: C2Capsule) -> usize {
        let n = C2v { x: b.b.x - b.a.x, y: b.b.y - b.a.y };
        let ap = C2v { x: a.p.x - b.a.x, y: a.p.y - b.a.y };
        let da = ap.x * n.x + ap.y * n.y;
        if da < 0.0 {
            0
        } else {
            let d = C2v { x: a.p.x - b.b.x, y: a.p.y - b.b.y };
            if d.x * n.x + d.y * n.y < 0.0 { 1 } else { 2 }
        }
    }

    let mut cases: Vec<(C2Circle, C2Capsule)> = Vec::new();
    for a in degenerate_circles() {
        for b in degenerate_capsules() {
            cases.push((a, b));
        }
    }
    // Deterministic arm coverage along a horizontal capsule from (0,0)-(10,0).
    let cap = C2Capsule { a: C2v { x: 0.0, y: 0.0 }, b: C2v { x: 10.0, y: 0.0 }, r: 2.0 };
    for &(x, y) in &[
        (-5.0f32, 0.0f32), (-1.0, 3.0),  // before a  -> arm 0
        (5.0, 0.0), (5.0, 1.9), (5.0, 2.0), (5.0, 2.1), // between -> arm 1
        (15.0, 0.0), (11.0, 3.0),        // beyond b  -> arm 2
        (0.0, 0.0), (10.0, 0.0),         // exactly on the endpoints
    ] {
        for &rad in &[0.0f32, 1.0, 2.0, -2.0] {
            cases.push((C2Circle { p: C2v { x, y }, r: rad }, C2Capsule { r: rad.abs(), ..cap }));
        }
    }
    // Degenerate capsule a == b: n == (0,0) so da == 0 and db == 0; the C takes
    // arm 2 and never divides by c2Dot(n,n).
    for &rad in &[0.0f32, 3.0] {
        cases.push((
            C2Circle { p: C2v { x: 1.0, y: 1.0 }, r: 5.0 },
            C2Capsule { a: C2v { x: 4.0, y: 4.0 }, b: C2v { x: 4.0, y: 4.0 }, r: rad },
        ));
    }
    for i in 0..N {
        let a = if i % 5 == 0 {
            C2Circle { p: rng.v_mixed(), r: rng.f32_mixed() }
        } else {
            rng.circle()
        };
        let b = match i % 8 {
            0 => C2Capsule { a: rng.v_mixed(), b: rng.v_mixed(), r: rng.f32_mixed() },
            1 => {
                let p = rng.v_tame();
                C2Capsule { a: p, b: p, r: rng.range(0.0, 30.0) } // degenerate
            }
            2 => C2Capsule { a: rng.v_any(), b: rng.v_any(), r: rng.any_f32() },
            _ => rng.capsule(),
        };
        cases.push((a, b));
    }

    for (a, b) in cases {
        arms[arm(a, b)] += 1;
        let (cv, rv) = unsafe { (c(a, b), r(a, b)) };
        same_i32("c2CircletoCapsule", &(a, b), cv, rv);
        if cv != 0 { yes += 1 } else { no += 1 }
    }
    assert!(arms.iter().all(|&x| x > 0), "arm coverage gap: {arms:?}");
    assert!(yes > 0 && no > 0, "one-sided result: yes={yes} no={no}");
    eprintln!("c2CircletoCapsule arms: before-a={} middle={} beyond-b={}", arms[0], arms[1], arms[2]);
}

/// row 75 — c2AABBtoCapsule (goes through c2GJK with use_radius = 1)
#[test]
fn row75_aabb_to_capsule() {
    let l = libs();
    let (c, r) = l.pair::<FnAABBtoCapsule>("c2AABBtoCapsule");
    let mut rng = Rng::new(0x5A_0075);
    let mut yes = 0usize;
    let mut no = 0usize;

    let mut cases: Vec<(C2AABB, C2Capsule)> = Vec::new();
    for a in degenerate_aabbs() {
        for b in degenerate_capsules() {
            cases.push((a, b));
        }
    }
    let bb = C2AABB { min: C2v { x: -10.0, y: -10.0 }, max: C2v { x: 10.0, y: 10.0 } };
    for &(ax, ay, bx, by, rad) in &[
        (-30.0f32, 0.0f32, -20.0f32, 0.0f32, 2.0f32),  // separated
        (-30.0, 0.0, 30.0, 0.0, 1.0),                  // crossing
        (0.0, 0.0, 5.0, 5.0, 1.0),                     // inside
        (-12.0, 0.0, -11.0, 0.0, 2.0),                 // exactly touching
        (0.0, 0.0, 0.0, 0.0, 0.0),                     // degenerate point
        (-30.0, 0.0, -20.0, 0.0, 0.0),
    ] {
        cases.push((
            bb,
            C2Capsule { a: C2v { x: ax, y: ay }, b: C2v { x: bx, y: by }, r: rad },
        ));
    }
    for i in 0..N / 10 {
        let a = if i % 5 == 0 {
            C2AABB { min: rng.v_mixed(), max: rng.v_mixed() }
        } else {
            rng.aabb()
        };
        let b = match i % 6 {
            0 => C2Capsule { a: rng.v_mixed(), b: rng.v_mixed(), r: rng.f32_mixed() },
            1 => {
                let p = rng.v_tame();
                C2Capsule { a: p, b: p, r: rng.range(0.0, 30.0) }
            }
            _ => rng.capsule(),
        };
        cases.push((a, b));
    }

    for (a, b) in cases {
        let (cv, rv) = unsafe { (c(a, b), r(a, b)) };
        same_i32("c2AABBtoCapsule", &(a, b), cv, rv);
        if cv != 0 { yes += 1 } else { no += 1 }
    }
    assert!(yes > 0 && no > 0, "one-sided result: yes={yes} no={no}");
}

/// row 76 — c2CapsuletoCapsule
#[test]
fn row76_capsule_to_capsule() {
    let l = libs();
    let (c, r) = l.pair::<FnCapsuletoCapsule>("c2CapsuletoCapsule");
    let mut rng = Rng::new(0x5A_0076);
    let mut yes = 0usize;
    let mut no = 0usize;

    let mut cases: Vec<(C2Capsule, C2Capsule)> = Vec::new();
    for a in degenerate_capsules() {
        for b in degenerate_capsules() {
            cases.push((a, b));
        }
    }
    let base = C2Capsule { a: C2v { x: 0.0, y: 0.0 }, b: C2v { x: 10.0, y: 0.0 }, r: 2.0 };
    for &(ax, ay, bx, by, rad) in &[
        (0.0f32, 10.0f32, 10.0f32, 10.0f32, 2.0f32),  // parallel, touching
        (0.0, 9.0, 10.0, 9.0, 2.0),                   // parallel, overlapping
        (0.0, 20.0, 10.0, 20.0, 2.0),                 // parallel, separated
        (5.0, -10.0, 5.0, 10.0, 1.0),                 // crossing
        (20.0, 0.0, 30.0, 0.0, 2.0),                  // collinear, separated
        (0.0, 0.0, 10.0, 0.0, 2.0),                   // coincident
        (5.0, 0.0, 5.0, 0.0, 0.0),                    // degenerate point inside
    ] {
        cases.push((
            base,
            C2Capsule { a: C2v { x: ax, y: ay }, b: C2v { x: bx, y: by }, r: rad },
        ));
    }
    for i in 0..N / 10 {
        let mk = |rng: &mut Rng, k: usize| match k {
            0 => C2Capsule { a: rng.v_mixed(), b: rng.v_mixed(), r: rng.f32_mixed() },
            1 => {
                let p = rng.v_tame();
                C2Capsule { a: p, b: p, r: rng.range(0.0, 30.0) }
            }
            _ => rng.capsule(),
        };
        let a = mk(&mut rng, i % 6);
        let b = mk(&mut rng, (i / 6) % 6);
        cases.push((a, b));
    }

    for (a, b) in cases {
        let (cv, rv) = unsafe { (c(a, b), r(a, b)) };
        same_i32("c2CapsuletoCapsule", &(a, b), cv, rv);
        if cv != 0 { yes += 1 } else { no += 1 }
    }
    assert!(yes > 0 && no > 0, "one-sided result: yes={yes} no={no}");
}

/// rows 77..85 — c2Collided over all 9 type pairs.
///
/// Three of the nine arms SWAP the arguments before delegating
/// (`AABB x CIRCLE`, `CAPSULE x CIRCLE`, `CAPSULE x AABB`); the test also
/// asserts the dispatcher agrees with the underlying predicate, which is what
/// catches a mis-transcribed swap.
#[test]
fn row77_85_collided() {
    let l = libs();
    let (c, r) = l.pair::<FnCollided>("c2Collided");
    let (cca, _) = l.pair::<FnCircletoAABB>("c2CircletoAABB");
    let (ccc, _) = l.pair::<FnCircletoCapsule>("c2CircletoCapsule");
    let (cac, _) = l.pair::<FnAABBtoCapsule>("c2AABBtoCapsule");
    let mut rng = Rng::new(0x5A_0077);
    let mut yes = [0usize; 9];
    let mut no = [0usize; 9];

    for (idx, (ta, tb)) in TYPE_PAIRS.iter().copied().enumerate() {
        for i in 0..N / 8 {
            // Bias towards nearby shapes so both outcomes occur often.
            let at = rng.v_tame();
            let (a, b) = if i % 3 == 0 {
                (ShapeBlob::random(&mut rng, ta), ShapeBlob::random(&mut rng, tb))
            } else if i % 7 == 0 {
                (ShapeBlob::degenerate(ta, i), ShapeBlob::degenerate(tb, i / 6))
            } else {
                let s = rng.range(1.0, 25.0);
                let j = C2v { x: at.x + rng.range(-2.0 * s, 2.0 * s), y: at.y + rng.range(-2.0 * s, 2.0 * s) };
                (
                    ShapeBlob::near(&mut rng, ta, at, s),
                    ShapeBlob::near(&mut rng, tb, j, s),
                )
            };
            let (cv, rv) = unsafe { (c(a.ptr(), ta, b.ptr(), tb), r(a.ptr(), ta, b.ptr(), tb)) };
            same_i32("c2Collided", &(type_name(ta), type_name(tb), a, b), cv, rv);
            if cv != 0 { yes[idx] += 1 } else { no[idx] += 1 }

            // Cross-check the swap-arms against the predicate the C delegates to.
            let expect = unsafe {
                match (ta, tb) {
                    (C2_TYPE_AABB, C2_TYPE_CIRCLE) => {
                        Some(cca(read_circle(&b), read_aabb(&a)))
                    }
                    (C2_TYPE_CAPSULE, C2_TYPE_CIRCLE) => {
                        Some(ccc(read_circle(&b), read_capsule(&a)))
                    }
                    (C2_TYPE_CAPSULE, C2_TYPE_AABB) => {
                        Some(cac(read_aabb(&b), read_capsule(&a)))
                    }
                    _ => None,
                }
            };
            if let Some(e) = expect {
                same_i32(
                    "c2Collided swap-arm delegation",
                    &(type_name(ta), type_name(tb), a, b),
                    e,
                    cv,
                );
            }
        }
    }
    for i in 0..9 {
        assert!(
            yes[i] > 0 && no[i] > 0,
            "pair {} x {}: one-sided (yes={} no={})",
            type_name(TYPE_PAIRS[i].0),
            type_name(TYPE_PAIRS[i].1),
            yes[i],
            no[i]
        );
    }
}

/// rows 86,87,88 — reverse_collide, the only symbol declared in lib.h
#[test]
fn row86_88_reverse_collide() {
    let l = libs();
    let (c, r) = l.pair::<FnReverse>("reverse_collide");
    let mut rng = Rng::new(0x5A_0086);
    let mut seen = [0usize; 8];

    // row 87 — boundary sweep against the three fixed shapes in the C:
    //   circle  centre (-70,   0)  r 20
    //   aabb    (-40,-40)..(-15,-15)
    //   capsule (-40, 40)-(-20,100) r 10
    let mut cases: Vec<(f32, f32, f32)> = Vec::new();
    for &r0 in &[
        0.0f32, -0.0, 1.0, -1.0, 20.0, 19.999998, 20.000002, 10.0, 1000.0,
        -1000.0, f32::MIN_POSITIVE, f32::from_bits(1), f32::MAX, f32::MIN,
        f32::INFINITY, f32::NEG_INFINITY, f32::NAN, FLT_EPSILON,
    ] {
        for &(x, y) in &[
            (-70.0f32, 0.0f32),  // circle centre
            (-50.0, 0.0), (-90.0, 0.0), (-70.0, 20.0), (-70.0, -20.0),
            (-27.5, -27.5),      // aabb centre
            (-40.0, -40.0), (-15.0, -15.0), (-15.0, -40.0), (-40.0, -15.0),
            (-41.0, -41.0), (-14.0, -14.0),
            (-40.0, 40.0), (-20.0, 100.0), (-30.0, 70.0), // capsule axis
            (-30.0, 80.0), (-50.0, 40.0), (-10.0, 100.0),
            (0.0, 0.0), (-0.0, -0.0), (1.0e30, -1.0e30),
        ] {
            cases.push((x, y, r0));
        }
    }
    // row 86 — randomized over the whole play area (all three bits reachable).
    for _ in 0..N {
        cases.push((rng.range(-120.0, 40.0), rng.range(-80.0, 140.0), rng.range(-5.0, 80.0)));
    }
    // row 88 — mixed float pool including NaN / inf / denormals.
    for _ in 0..N {
        cases.push((rng.f32_mixed(), rng.f32_mixed(), rng.f32_mixed()));
    }
    for _ in 0..N / 4 {
        cases.push((rng.any_f32(), rng.any_f32(), rng.any_f32()));
    }
    for &x in SPECIAL {
        for &y in SPECIAL {
            for &r0 in SPECIAL {
                cases.push((x, y, r0));
            }
        }
    }

    for (x, y, r0) in cases {
        let (cv, rv) = unsafe { (c(x, y, r0), r(x, y, r0)) };
        same_i32("reverse_collide", &(x, y, r0), cv, rv);
        assert!((0..8).contains(&cv), "reverse_collide out of range: {cv}");
        seen[cv as usize] += 1;
    }
    // Every bit and every combination of bits must have been produced.
    assert!(
        seen.iter().all(|&s| s > 0),
        "reverse_collide result coverage gap: {seen:?}"
    );
    eprintln!("reverse_collide result histogram: {seen:?}");
}

// --- blob readers ----------------------------------------------------------

fn read_circle(b: &ShapeBlob) -> C2Circle {
    unsafe { std::ptr::read_unaligned(b.bytes.as_ptr() as *const C2Circle) }
}
fn read_aabb(b: &ShapeBlob) -> C2AABB {
    unsafe { std::ptr::read_unaligned(b.bytes.as_ptr() as *const C2AABB) }
}
fn read_capsule(b: &ShapeBlob) -> C2Capsule {
    unsafe { std::ptr::read_unaligned(b.bytes.as_ptr() as *const C2Capsule) }
}
