//! Phase B — CONFIGS.md rows 78..91: the boolean shape predicates, the
//! `c2Collided` dispatcher (including the C's argument swapping), and the sole
//! header-declared entry point `aabb`.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_int, c_void};

const N: usize = 4000;

// ---------------------------------------------------------------------------
// row 78: c2AABBtoAABB
// ---------------------------------------------------------------------------

#[test]
fn row78_c2AABBtoAABB() {
    let a = api();
    let mut d = Diff::new("row 78: c2AABBtoAABB (disjoint-x/-y, overlap, touch, contain, identical, inverted)");
    let r = Rng::new(78);
    let mut yes = 0usize;
    let mut no = 0usize;

    let fixed: &[(c2AABB, c2AABB)] = &[
        // exactly touching on x
        (c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 1.0, y: 1.0 } },
         c2AABB { min: c2v { x: 1.0, y: 0.0 }, max: c2v { x: 2.0, y: 1.0 } }),
        // exactly touching on y
        (c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 1.0, y: 1.0 } },
         c2AABB { min: c2v { x: 0.0, y: 1.0 }, max: c2v { x: 1.0, y: 2.0 } }),
        // identical
        (c2AABB { min: c2v { x: -3.0, y: -3.0 }, max: c2v { x: 3.0, y: 3.0 } },
         c2AABB { min: c2v { x: -3.0, y: -3.0 }, max: c2v { x: 3.0, y: 3.0 } }),
        // contained
        (c2AABB { min: c2v { x: -9.0, y: -9.0 }, max: c2v { x: 9.0, y: 9.0 } },
         c2AABB { min: c2v { x: -1.0, y: -1.0 }, max: c2v { x: 1.0, y: 1.0 } }),
        // inverted A
        (c2AABB { min: c2v { x: 5.0, y: 5.0 }, max: c2v { x: -5.0, y: -5.0 } },
         c2AABB { min: c2v { x: -1.0, y: -1.0 }, max: c2v { x: 1.0, y: 1.0 } }),
        // NaN coordinate (C: all four `<` false -> reports a hit)
        (c2AABB { min: c2v { x: f32::NAN, y: 0.0 }, max: c2v { x: 1.0, y: 1.0 } },
         c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 1.0, y: 1.0 } }),
        // ±0.0 boundary
        (c2AABB { min: c2v { x: -0.0, y: -0.0 }, max: c2v { x: 0.0, y: 0.0 } },
         c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: -0.0, y: -0.0 } }),
    ];
    for &(A, B) in fixed {
        let (c, s) = ((a.c2AABBtoAABB.0)(A, B), (a.c2AABBtoAABB.1)(A, B));
        d.check(c == s, || format!("c2AABBtoAABB({A:?}, {B:?}) C={c} R={s}"));
    }

    for i in 0..N * 3 {
        let A = r.aabb();
        let B = match i % 5 {
            0 => A,                                                     // identical
            1 => c2AABB { min: A.min, max: c2v { x: A.min.x + 0.5, y: A.min.y + 0.5 } }, // contained
            2 => c2AABB { min: A.max, max: c2v { x: A.max.x + 5.0, y: A.max.y + 5.0 } }, // corner-touch
            3 => c2AABB { min: c2v { x: A.max.x, y: A.min.y }, max: c2v { x: A.max.x + 3.0, y: A.max.y } }, // face-touch
            _ => r.aabb(),
        };
        let (c, s) = ((a.c2AABBtoAABB.0)(A, B), (a.c2AABBtoAABB.1)(A, B));
        d.check(c == s, || format!("c2AABBtoAABB({A:?}, {B:?}) C={c} R={s}"));
        if c != 0 { yes += 1 } else { no += 1 }
    }
    assert!(yes > 0 && no > 0, "coverage: yes={yes} no={no}");
    d.finish();
}

// ---------------------------------------------------------------------------
// row 79: c2CircletoCircle
// ---------------------------------------------------------------------------

#[test]
fn row79_c2CircletoCircle() {
    let a = api();
    let mut d = Diff::new("row 79: c2CircletoCircle (disjoint/touching/overlap/concentric/zero-r/negative-r)");
    let r = Rng::new(79);
    let mut yes = 0usize;
    let mut no = 0usize;

    // exact touching: distance == r1 + r2 (so `d2 < r2` must be false)
    let fixed: &[(c2Circle, c2Circle)] = &[
        (c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 2.0 }, c2Circle { p: c2v { x: 5.0, y: 0.0 }, r: 3.0 }),
        (c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 0.0 }, c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 0.0 }),
        (c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 0.0 }, c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 1.0 }),
        (c2Circle { p: c2v { x: 1.0, y: 1.0 }, r: -3.0 }, c2Circle { p: c2v { x: 1.5, y: 1.0 }, r: -1.0 }),
        (c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 4.0 }, c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 1.0 }),
        (c2Circle { p: c2v { x: f32::NAN, y: 0.0 }, r: 4.0 }, c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 1.0 }),
    ];
    for &(A, B) in fixed {
        let (c, s) = ((a.c2CircletoCircle.0)(A, B), (a.c2CircletoCircle.1)(A, B));
        d.check(c == s, || format!("c2CircletoCircle({A:?}, {B:?}) C={c} R={s}"));
    }

    for i in 0..N * 3 {
        let A = r.circle();
        let B = match i % 6 {
            0 => c2Circle { p: A.p, r: r.radius() },                                   // concentric
            1 => c2Circle { p: c2v { x: A.p.x + A.r, y: A.p.y }, r: 0.0 },              // on the rim, r=0
            2 => c2Circle { p: c2v { x: A.p.x + A.r + 1.0, y: A.p.y }, r: 1.0 },        // exact touch
            3 => c2Circle { p: r.geo_v(), r: -r.radius() },                             // negative radius
            _ => r.circle(),
        };
        let (c, s) = ((a.c2CircletoCircle.0)(A, B), (a.c2CircletoCircle.1)(A, B));
        d.check(c == s, || format!("c2CircletoCircle({A:?}, {B:?}) C={c} R={s}"));
        if c != 0 { yes += 1 } else { no += 1 }
    }
    assert!(yes > 0 && no > 0, "coverage: yes={yes} no={no}");
    d.finish();
}

// ---------------------------------------------------------------------------
// row 80: c2CircletoAABB
// ---------------------------------------------------------------------------

#[test]
fn row80_c2CircletoAABB() {
    let a = api();
    let mut d = Diff::new("row 80: c2CircletoAABB (inside / face / corner / boundary / degenerate)");
    let r = Rng::new(80);
    let mut yes = 0usize;
    let mut no = 0usize;
    for i in 0..N * 4 {
        let B = r.aabb();
        let ctr = c2v {
            x: (B.min.x + B.max.x) * 0.5,
            y: (B.min.y + B.max.y) * 0.5,
        };
        let A = match i % 6 {
            0 => c2Circle { p: ctr, r: r.radius() },                                        // inside
            1 => c2Circle { p: c2v { x: B.max.x, y: ctr.y }, r: 0.0 },                       // on a face, r=0
            2 => c2Circle { p: B.max, r: r.radius() },                                       // at a corner
            3 => c2Circle { p: c2v { x: B.max.x + 3.0, y: ctr.y }, r: 3.0 },                 // exact face touch
            4 => c2Circle { p: r.geo_v(), r: -r.radius() },                                  // negative radius
            _ => r.circle(),
        };
        let (c, s) = ((a.c2CircletoAABB.0)(A, B), (a.c2CircletoAABB.1)(A, B));
        d.check(c == s, || format!("c2CircletoAABB({A:?}, {B:?}) C={c} R={s}"));
        if c != 0 { yes += 1 } else { no += 1 }
    }
    assert!(yes > 0 && no > 0, "coverage: yes={yes} no={no}");
    d.finish();
}

// ---------------------------------------------------------------------------
// row 81: c2CircletoCapsule (all three internal branches)
// ---------------------------------------------------------------------------

#[test]
fn row81_c2CircletoCapsule() {
    let a = api();
    let mut d = Diff::new("row 81: c2CircletoCapsule (da<0 / db<0 / else branch, point capsule)");
    let r = Rng::new(81);
    let mut branch = [0usize; 3];
    let mut yes = 0usize;
    let mut no = 0usize;
    for i in 0..N * 4 {
        let B = match i % 8 {
            0 => {
                let p = r.geo_v();
                c2Capsule { a: p, b: p, r: r.radius() } // point capsule (n == (0,0))
            }
            _ => r.capsule(),
        };
        // Place the circle so that all three branches are reached.
        let n = c2v { x: B.b.x - B.a.x, y: B.b.y - B.a.y };
        let A = match i % 4 {
            0 => c2Circle { p: c2v { x: B.a.x - n.x * 0.5, y: B.a.y - n.y * 0.5 }, r: r.radius() },
            1 => c2Circle { p: c2v { x: B.a.x + n.x * 0.5 - n.y * 0.3, y: B.a.y + n.y * 0.5 + n.x * 0.3 }, r: r.radius() },
            2 => c2Circle { p: c2v { x: B.b.x + n.x * 0.5, y: B.b.y + n.y * 0.5 }, r: r.radius() },
            _ => r.circle(),
        };
        // classify (mirrors the C control flow)
        let dot = |u: c2v, v: c2v| u.x * v.x + u.y * v.y;
        let ap = c2v { x: A.p.x - B.a.x, y: A.p.y - B.a.y };
        let da = dot(ap, n);
        if da < 0.0 {
            branch[0] += 1;
        } else {
            let db = dot(c2v { x: A.p.x - B.b.x, y: A.p.y - B.b.y }, n);
            if db < 0.0 { branch[1] += 1 } else { branch[2] += 1 }
        }
        let (c, s) = ((a.c2CircletoCapsule.0)(A, B), (a.c2CircletoCapsule.1)(A, B));
        d.check(c == s, || format!("c2CircletoCapsule({A:?}, {B:?}) C={c} R={s}"));
        if c != 0 { yes += 1 } else { no += 1 }
    }
    assert!(branch.iter().all(|&b| b > 0), "branch coverage: {branch:?}");
    assert!(yes > 0 && no > 0, "coverage: yes={yes} no={no}");
    eprintln!("c2CircletoCapsule branch hits: {branch:?}");
    d.finish();
}

// ---------------------------------------------------------------------------
// row 82: c2AABBtoCapsule
// ---------------------------------------------------------------------------

#[test]
fn row82_c2AABBtoCapsule() {
    let a = api();
    let mut d = Diff::new("row 82: c2AABBtoCapsule (disjoint/overlap/touch/inside/point/zero-r)");
    let r = Rng::new(82);
    let mut yes = 0usize;
    let mut no = 0usize;
    for i in 0..N * 3 {
        let A = r.aabb();
        let ctr = c2v { x: (A.min.x + A.max.x) * 0.5, y: (A.min.y + A.max.y) * 0.5 };
        let B = match i % 6 {
            0 => c2Capsule { a: ctr, b: ctr, r: 0.0 },                                   // point at the centre
            1 => c2Capsule { a: A.min, b: A.max, r: r.radius() },                        // diagonal, inside
            2 => c2Capsule { a: c2v { x: A.max.x + 5.0, y: ctr.y }, b: c2v { x: A.max.x + 15.0, y: ctr.y }, r: 5.0 }, // exact touch
            3 => c2Capsule { a: A.max, b: c2v { x: A.max.x + 10.0, y: A.max.y + 10.0 }, r: 0.0 }, // corner touch
            _ => r.capsule(),
        };
        let (c, s) = ((a.c2AABBtoCapsule.0)(A, B), (a.c2AABBtoCapsule.1)(A, B));
        d.check(c == s, || format!("c2AABBtoCapsule({A:?}, {B:?}) C={c} R={s}"));
        if c != 0 { yes += 1 } else { no += 1 }
    }
    assert!(yes > 0 && no > 0, "coverage: yes={yes} no={no}");
    d.finish();
}

// ---------------------------------------------------------------------------
// row 83: c2CapsuletoCapsule
// ---------------------------------------------------------------------------

#[test]
fn row83_c2CapsuletoCapsule() {
    let a = api();
    let mut d = Diff::new("row 83: c2CapsuletoCapsule (parallel/perp/crossing/collinear/disjoint/identical/point)");
    let r = Rng::new(83);
    let mut yes = 0usize;
    let mut no = 0usize;
    for i in 0..N * 3 {
        let A = r.capsule();
        let n = c2v { x: A.b.x - A.a.x, y: A.b.y - A.a.y };
        let B = match i % 8 {
            0 => A,                                                                              // identical
            1 => c2Capsule { a: c2v { x: A.a.x, y: A.a.y + 20.0 }, b: c2v { x: A.b.x, y: A.b.y + 20.0 }, r: A.r }, // parallel
            2 => c2Capsule { a: c2v { x: A.a.x - n.y * 0.5, y: A.a.y + n.x * 0.5 }, b: c2v { x: A.a.x + n.y * 0.5, y: A.a.y - n.x * 0.5 }, r: A.r }, // perpendicular
            3 => {
                // crossing X through the midpoint
                let m = c2v { x: (A.a.x + A.b.x) * 0.5, y: (A.a.y + A.b.y) * 0.5 };
                c2Capsule { a: c2v { x: m.x - n.y * 0.5, y: m.y + n.x * 0.5 }, b: c2v { x: m.x + n.y * 0.5, y: m.y - n.x * 0.5 }, r: A.r }
            }
            4 => c2Capsule { a: A.b, b: c2v { x: A.b.x + n.x, y: A.b.y + n.y }, r: A.r },        // collinear, end to end
            5 => {
                let p = r.geo_v();
                c2Capsule { a: p, b: p, r: 0.0 }                                                 // degenerate point
            }
            _ => r.capsule(),
        };
        let (c, s) = ((a.c2CapsuletoCapsule.0)(A, B), (a.c2CapsuletoCapsule.1)(A, B));
        d.check(c == s, || format!("c2CapsuletoCapsule({A:?}, {B:?}) C={c} R={s}"));
        if c != 0 { yes += 1 } else { no += 1 }
    }
    assert!(yes > 0 && no > 0, "coverage: yes={yes} no={no}");
    d.finish();
}

// ---------------------------------------------------------------------------
// rows 84-85: c2Collided dispatcher over the whole 3x3 valid grid
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
enum Shp {
    C(c2Circle),
    A(c2AABB),
    P(c2Capsule),
}

impl Shp {
    #[allow(dead_code)]
    fn ty(&self) -> c_int {
        match self {
            Shp::C(_) => C2_TYPE_CIRCLE,
            Shp::A(_) => C2_TYPE_AABB,
            Shp::P(_) => C2_TYPE_CAPSULE,
        }
    }
    fn ptr(&self) -> *const c_void {
        match self {
            Shp::C(x) => x as *const _ as *const c_void,
            Shp::A(x) => x as *const _ as *const c_void,
            Shp::P(x) => x as *const _ as *const c_void,
        }
    }
}

fn shp(r: &Rng, ty: c_int, at: c2v, sc: f32) -> Shp {
    match ty {
        C2_TYPE_CIRCLE => Shp::C(c2Circle { p: at, r: r.unit() * sc }),
        C2_TYPE_AABB => {
            let (hw, hh) = (r.unit() * sc, r.unit() * sc);
            Shp::A(c2AABB { min: c2v { x: at.x - hw, y: at.y - hh }, max: c2v { x: at.x + hw, y: at.y + hh } })
        }
        _ => Shp::P(c2Capsule {
            a: c2v { x: at.x - r.sym(sc), y: at.y - r.sym(sc) },
            b: c2v { x: at.x + r.sym(sc), y: at.y + r.sym(sc) },
            r: r.unit() * sc * 0.5,
        }),
    }
}

#[test]
fn row84_85_c2Collided_grid() {
    let a = api();
    let mut d = Diff::new("rows 84-85: c2Collided full 3x3 grid x disjoint/touch/overlap/contained");
    let r = Rng::new(8485);
    let types = [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE];
    let mut grid_yes = [[0usize; 3]; 3];
    let mut grid_no = [[0usize; 3]; 3];
    for (i, &ta) in types.iter().enumerate() {
        for (j, &tb) in types.iter().enumerate() {
            for k in 0..1200 {
                let ca = c2v { x: r.sym(50.0), y: r.sym(50.0) };
                let (A, B) = match k % 4 {
                    0 => (shp(&r, ta, ca, 25.0), shp(&r, tb, ca, 25.0)),                    // overlap
                    1 => (shp(&r, ta, ca, 60.0), shp(&r, tb, ca, 1.0)),                     // contained
                    2 => (shp(&r, ta, ca, 5.0), shp(&r, tb, c2v { x: ca.x + 300.0, y: ca.y }, 5.0)), // far apart
                    _ => (shp(&r, ta, ca, 20.0), shp(&r, tb, c2v { x: ca.x + r.sym(40.0), y: ca.y + r.sym(40.0) }, 20.0)),
                };
                let (c, s) = unsafe {
                    ((a.c2Collided.0)(A.ptr(), ta, B.ptr(), tb), (a.c2Collided.1)(A.ptr(), ta, B.ptr(), tb))
                };
                d.check(c == s, || format!("c2Collided(typeA={ta}, typeB={tb}) A={A:?} B={B:?} C={c} R={s}"));
                if c != 0 { grid_yes[i][j] += 1 } else { grid_no[i][j] += 1 }
            }
        }
    }
    for i in 0..3 {
        for j in 0..3 {
            assert!(
                grid_yes[i][j] > 0 && grid_no[i][j] > 0,
                "type pair ({i},{j}) never produced both outcomes: yes={} no={}",
                grid_yes[i][j], grid_no[i][j]
            );
        }
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// rows 86-91: the header entry point `aabb`
// ---------------------------------------------------------------------------

#[test]
fn row86_91_aabb_entry_point() {
    let a = api();
    let mut d = Diff::new("rows 86-91: aabb() randomized + targeted bit patterns + degenerate boxes");
    let r = Rng::new(8691);
    let mut seen = [0usize; 8];

    // Row 87-90: hand-picked boxes that isolate each result bit.
    // hard-coded shapes in the C: circle @(-70,0) r=20,
    // aabb [(-40,-40),(-15,-15)], capsule (-40,40)->(-20,100) r=10.
    let targeted: &[(f32, f32, f32, f32)] = &[
        (-100.0, -10.0, -80.0, 10.0),  // circle only
        (-30.0, -30.0, -20.0, -20.0),  // aabb only
        (-35.0, 50.0, -25.0, 60.0),    // capsule only
        (500.0, 500.0, 600.0, 600.0),  // nothing
        (-200.0, -200.0, 200.0, 200.0),// everything
        (-90.0, -50.0, -10.0, 10.0),   // circle + aabb
        (-90.0, -10.0, -10.0, 60.0),   // circle + capsule
        (-45.0, -45.0, -18.0, 55.0),   // aabb + capsule
        (0.0, 0.0, 0.0, 0.0),
        (-0.0, -0.0, -0.0, -0.0),
        (10.0, 10.0, -10.0, -10.0),    // inverted
        (f32::MIN, f32::MIN, f32::MAX, f32::MAX),
        (f32::NAN, 0.0, 1.0, 1.0),
        (f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY, f32::INFINITY),
        (1.0e-40, 1.0e-40, 2.0e-40, 2.0e-40), // denormals
    ];
    for &(a0, a1, a2, a3) in targeted {
        let (c, s) = ((a.aabb.0)(a0, a1, a2, a3), (a.aabb.1)(a0, a1, a2, a3));
        d.check(c == s, || format!("aabb({a0}, {a1}, {a2}, {a3}) C={c} R={s}"));
        if (0..8).contains(&c) {
            seen[c as usize] += 1;
        }
    }

    // Row 86 / 91: randomized sweep.
    for i in 0..N * 8 {
        let (x0, y0, x1, y1) = match i % 4 {
            0 => (r.sym(200.0), r.sym(200.0), r.sym(200.0), r.sym(200.0)),
            1 => {
                let (x, y) = (r.sym(150.0), r.sym(150.0));
                (x, y, x + r.unit() * 80.0, y + r.unit() * 80.0)
            }
            2 => (r.wild(), r.wild(), r.wild(), r.wild()),
            _ => {
                let (x, y) = (r.sym(120.0), r.sym(120.0));
                (x, y, x, y) // degenerate
            }
        };
        let (c, s) = ((a.aabb.0)(x0, y0, x1, y1), (a.aabb.1)(x0, y0, x1, y1));
        d.check(c == s, || format!("aabb({x0}, {y0}, {x1}, {y1}) C={c} R={s}"));
        if (0..8).contains(&c) {
            seen[c as usize] += 1;
        }
    }

    eprintln!("aabb() result histogram: {seen:?}");
    // Every individual bit must have been observed both set and clear.
    for bit in 0..3 {
        let set: usize = (0..8).filter(|v| v & (1 << bit) != 0).map(|v| seen[v]).sum();
        let clr: usize = (0..8).filter(|v| v & (1 << bit) == 0).map(|v| seen[v]).sum();
        assert!(set > 0 && clr > 0, "aabb() bit {bit} never varied (set={set}, clear={clr})");
    }
    d.finish();
}

/// `aabb` is the only symbol in `include/lib.h`; there is no driver binary in
/// this project (`CMakeLists.txt` builds `add_library(... SHARED)` only, and the
/// Rust crate is `crate-type = ["cdylib"]`).  This test stands in for the
/// "compare the two binaries' stdout" check by formatting `aabb`'s output for a
/// deterministic input grid exactly as a driver would, from BOTH `.so`s, and
/// comparing the resulting text byte-for-byte.
#[test]
fn driver_equivalent_stdout_compare() {
    let a = api();
    let mut c_out = String::new();
    let mut r_out = String::new();
    let mut n = 0;
    let mut v = -210.0f32;
    while v <= 210.0 {
        let mut w = -210.0f32;
        while w <= 210.0 {
            let (x0, y0, x1, y1) = (v, w, v + 37.5, w + 22.25);
            c_out.push_str(&format!("{:.4} {:.4} {:.4} {:.4} -> {}\n", x0, y0, x1, y1, (a.aabb.0)(x0, y0, x1, y1)));
            r_out.push_str(&format!("{:.4} {:.4} {:.4} {:.4} -> {}\n", x0, y0, x1, y1, (a.aabb.1)(x0, y0, x1, y1)));
            n += 1;
            w += 7.0;
        }
        v += 7.0;
    }
    assert_eq!(n, 61 * 61);
    assert_eq!(
        c_out.as_bytes(),
        r_out.as_bytes(),
        "driver-equivalent stdout differs between the C and the Rust .so"
    );
    eprintln!("[driver stdout] ok ({n} lines, {} bytes identical)", c_out.len());
}
