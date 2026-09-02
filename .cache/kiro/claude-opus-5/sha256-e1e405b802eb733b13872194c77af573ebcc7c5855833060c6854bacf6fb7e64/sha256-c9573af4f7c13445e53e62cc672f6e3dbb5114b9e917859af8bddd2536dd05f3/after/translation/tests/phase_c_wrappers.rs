//! Phase C part 3: `ERRORS.md` rows 49-70 — the boolean helpers, the
//! `c2Collided` dispatcher's out-of-range enum handling, `aabb`, and the
//! ternary-select `NaN` asymmetry of `c2Maxv`/`c2Minv`/`c2Clampv`.
//!
//! The enum rows are the important ones: a C enum accepts any `int`, so a value
//! with no valid variant is a real input crossing the FFI boundary and the Rust
//! must reject it exactly as the C does.

mod common;

use common::*;
use std::ffi::{c_int, c_void};

const BAD_TYPES: [c_int; 14] = [
    3,
    4,
    5,
    7,
    8,
    255,
    256,
    -1,
    -2,
    -1000,
    1 << 20,
    i32::MAX,
    i32::MIN,
    0x7FFF_FFFE,
];

const VALID_TYPES: [c_int; 3] = [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE];

#[repr(C)]
#[derive(Copy, Clone)]
union ShapeU {
    circle: c2Circle,
    aabb: c2AABB,
    capsule: c2Capsule,
}

// ---------------------------------------------------------------------------
// ERRORS rows 49-50 — c2AABBtoAABB has no normalisation and reports NaN as hit
// ---------------------------------------------------------------------------

#[test]
fn err49_err50_aabbtoaabb_inverted_and_nan() {
    let l = libs();
    let (c, r) =
        (l.c.sym::<FnAABBtoAABB>("c2AABBtoAABB"), l.rs.sym::<FnAABBtoAABB>("c2AABBtoAABB"));
    let mut g = Rng::new(0xD49);
    let mut rep = Report::new();

    let mut probe = |rep: &mut Report, A: c2AABB, B: c2AABB, want: Option<c_int>, tag: &str| {
        let (x, y) = (c(A, B), r(A, B));
        rep.check(x == y, || {
            format!(
                "c2AABBtoAABB[{tag}] A(min={} max={}) B(min={} max={}): C={x} Rust={y}",
                show_v(A.min),
                show_v(A.max),
                show_v(B.min),
                show_v(B.max)
            )
        });
        if let Some(w) = want {
            rep.check(x == w, || format!("c2AABBtoAABB[{tag}] C returned {x}, expected {w}"));
        }
    };

    // Row 49: fully inverted boxes. The C evaluates four plain `<` tests with no
    // min/max swap, so an inverted box is NOT normalised.
    for _ in 0..3000 {
        let p = g.finite_v();
        let q = g.finite_v();
        let proper = c2AABB {
            min: c2v { x: p.x.min(q.x), y: p.y.min(q.y) },
            max: c2v { x: p.x.max(q.x), y: p.y.max(q.y) },
        };
        let inverted = c2AABB { min: proper.max, max: proper.min };
        probe(rep_mut(&mut rep), proper, inverted, None, "row49 proper vs inverted");
        probe(rep_mut(&mut rep), inverted, proper, None, "row49 inverted vs proper");
        probe(rep_mut(&mut rep), inverted, inverted, None, "row49 both inverted");
    }
    // A fully inverted box against itself: all four `<` are true when the box has
    // non-zero extent, so d0|d1|d2|d3 != 0 and the result is 0.
    probe(
        &mut rep,
        c2AABB { min: c2v { x: 1.0, y: 1.0 }, max: c2v { x: -1.0, y: -1.0 } },
        c2AABB { min: c2v { x: 1.0, y: 1.0 }, max: c2v { x: -1.0, y: -1.0 } },
        Some(0),
        "row49 inverted self",
    );

    // Row 50: a NaN coordinate makes every `<` false, so the C reports 1.
    let unit = c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 1.0, y: 1.0 } };
    let far = c2AABB { min: c2v { x: 500.0, y: 500.0 }, max: c2v { x: 501.0, y: 501.0 } };
    let nan = f32::NAN;
    for (A, B, want, tag) in [
        (
            c2AABB { min: c2v { x: nan, y: nan }, max: c2v { x: nan, y: nan } },
            far,
            // All four coordinates NaN -> all four `<` false -> reports overlap.
            Some(1),
            "row50 all NaN vs far",
        ),
        (
            c2AABB { min: c2v { x: nan, y: 0.0 }, max: c2v { x: 1.0, y: 1.0 } },
            far,
            // Only min.x is NaN, so `d1 = A.max.x < B.min.x` (1 < 500) is still
            // true -> 0. A single NaN is NOT enough to force the overlap report.
            Some(0),
            "row50 NaN min.x only",
        ),
        (
            unit,
            c2AABB { min: c2v { x: nan, y: nan }, max: c2v { x: nan, y: nan } },
            Some(1),
            "row50 B all NaN",
        ),
    ] {
        probe(&mut rep, A, B, want, tag);
    }
    // One NaN per position, against a far-away box: each must still agree.
    for i in 0..4 {
        let mut b = far;
        match i {
            0 => b.min.x = nan,
            1 => b.min.y = nan,
            2 => b.max.x = nan,
            _ => b.max.y = nan,
        }
        probe(&mut rep, unit, b, None, "row50 single NaN");
        probe(&mut rep, b, unit, None, "row50 single NaN rev");
    }
    // Infinities and signed zeros.
    for (A, B) in [
        (
            c2AABB {
                min: c2v { x: f32::NEG_INFINITY, y: f32::NEG_INFINITY },
                max: c2v { x: f32::INFINITY, y: f32::INFINITY },
            },
            far,
        ),
        (
            c2AABB {
                min: c2v { x: f32::INFINITY, y: f32::INFINITY },
                max: c2v { x: f32::NEG_INFINITY, y: f32::NEG_INFINITY },
            },
            far,
        ),
        (
            c2AABB { min: c2v { x: -0.0, y: -0.0 }, max: c2v { x: 0.0, y: 0.0 } },
            c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: -0.0, y: -0.0 } },
        ),
    ] {
        probe(&mut rep, A, B, None, "row49/50 inf and signed zero");
        probe(&mut rep, B, A, None, "row49/50 inf and signed zero rev");
    }
    rep.finish("err49_err50_aabbtoaabb_inverted_and_nan");
}

/// Reborrow helper so the closure above can be called inside a loop.
fn rep_mut(r: &mut Report) -> &mut Report {
    r
}

// ---------------------------------------------------------------------------
// ERRORS rows 51-53 — the GJK-backed booleans treat any non-zero float as "no"
// ---------------------------------------------------------------------------

#[test]
fn err51_to_err53_gjk_backed_booleans() {
    let l = libs();
    let atc = (
        l.c.sym::<FnAABBtoCapsule>("c2AABBtoCapsule"),
        l.rs.sym::<FnAABBtoCapsule>("c2AABBtoCapsule"),
    );
    let ctc = (
        l.c.sym::<FnCapsuletoCapsule>("c2CapsuletoCapsule"),
        l.rs.sym::<FnCapsuletoCapsule>("c2CapsuletoCapsule"),
    );
    let gjk = (l.c.sym::<FnGJK>("c2GJK"), l.rs.sym::<FnGJK>("c2GJK"));
    let mut g = Rng::new(0xD51);
    let mut rep = Report::new();

    // Rows 51/52/53: the result must be exactly `dist == 0.0 ? 1 : 0`, where
    // `dist` is what `c2GJK(..., use_radius=1, ...)` returns. A NaN `dist` is
    // truthy in C, so it must map to 0.
    let mut saw_nan = 0usize;
    for _ in 0..6000 {
        let bb = g.aabb();
        let capA = g.capsule();
        let capB = g.capsule();

        let u = ShapeU { aabb: bb };
        let v = ShapeU { capsule: capA };
        let dist = unsafe {
            gjk.0(
                &raw const u as *const c_void,
                C2_TYPE_AABB,
                std::ptr::null(),
                &raw const v as *const c_void,
                C2_TYPE_CAPSULE,
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                1,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if dist.is_nan() {
            saw_nan += 1;
        }
        let want = if dist == 0.0 { 1 } else { 0 };
        let (x, y) = (atc.0(bb, capA), atc.1(bb, capA));
        rep.check(x == y && x == want, || {
            format!(
                "rows51/52 c2AABBtoCapsule: C={x} Rust={y} want={want} (gjk dist={})",
                show_f32(dist)
            )
        });

        let u = ShapeU { capsule: capA };
        let v = ShapeU { capsule: capB };
        let dist = unsafe {
            gjk.0(
                &raw const u as *const c_void,
                C2_TYPE_CAPSULE,
                std::ptr::null(),
                &raw const v as *const c_void,
                C2_TYPE_CAPSULE,
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                1,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if dist.is_nan() {
            saw_nan += 1;
        }
        let want = if dist == 0.0 { 1 } else { 0 };
        let (x, y) = (ctc.0(capA, capB), ctc.1(capA, capB));
        rep.check(x == y && x == want, || {
            format!(
                "row53 c2CapsuletoCapsule: C={x} Rust={y} want={want} (gjk dist={})",
                show_f32(dist)
            )
        });
    }
    // Deliberately construct NaN-producing inputs so row 51's NaN case is real.
    for cap in [
        c2Capsule { a: c2v { x: f32::NAN, y: 0.0 }, b: c2v { x: 1.0, y: 0.0 }, r: 1.0 },
        c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 1.0, y: 0.0 }, r: f32::NAN },
        c2Capsule {
            a: c2v { x: f32::INFINITY, y: 0.0 },
            b: c2v { x: f32::NEG_INFINITY, y: 0.0 },
            r: 1.0,
        },
        c2Capsule { a: c2v { x: 1.0e38, y: 0.0 }, b: c2v { x: -1.0e38, y: 0.0 }, r: 1.0e38 },
    ] {
        for bb in [
            c2AABB { min: c2v { x: -1.0, y: -1.0 }, max: c2v { x: 1.0, y: 1.0 } },
            c2AABB { min: c2v { x: f32::NAN, y: 0.0 }, max: c2v { x: 1.0, y: 1.0 } },
        ] {
            let (x, y) = (atc.0(bb, cap), atc.1(bb, cap));
            rep.check(x == y, || format!("row51 NaN c2AABBtoCapsule: C={x} Rust={y}"));
            let (x, y) = (ctc.0(cap, cap), ctc.1(cap, cap));
            rep.check(x == y, || format!("row51 NaN c2CapsuletoCapsule: C={x} Rust={y}"));
        }
    }
    eprintln!("err51-53: {saw_nan} NaN distances observed");
    rep.finish("err51_to_err53_gjk_backed_booleans");
}

// ---------------------------------------------------------------------------
// ERRORS rows 54-57 — the strict `<` boundaries and the squared-radius sign loss
// ---------------------------------------------------------------------------

#[test]
fn err54_to_err57_circle_boundaries() {
    let l = libs();
    let cc = (
        l.c.sym::<FnCircletoCircle>("c2CircletoCircle"),
        l.rs.sym::<FnCircletoCircle>("c2CircletoCircle"),
    );
    let ca = (
        l.c.sym::<FnCircletoAABB>("c2CircletoAABB"),
        l.rs.sym::<FnCircletoAABB>("c2CircletoAABB"),
    );
    let mut rep = Report::new();

    // Row 54: `r2 = (A.r + B.r)^2` squares away the sign, so a radius sum of -5
    // behaves exactly like +5.
    for (rA, rB) in [(-1.0f32, -4.0f32), (1.0, 4.0), (-3.0, 8.0), (3.0, -8.0), (-5.0, 0.0), (5.0, 0.0)] {
        let mag = (rA + rB).abs();
        for scale in [0.0f32, 0.5, 0.999, 1.0, 1.001, 2.0] {
            let d = mag * scale;
            let A = c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: rA };
            let B = c2Circle { p: c2v { x: d, y: 0.0 }, r: rB };
            let (x, y) = (cc.0(A, B), cc.1(A, B));
            rep.check(x == y, || {
                format!(
                    "row54 c2CircletoCircle(rA={}, rB={}, d={}): C={x} Rust={y}",
                    show_f32(rA),
                    show_f32(rB),
                    show_f32(d)
                )
            });
            // Sign loss: the negated pair must give the same answer.
            let A2 = c2Circle { p: A.p, r: -rA };
            let B2 = c2Circle { p: B.p, r: -rB };
            let (x2, y2) = (cc.0(A2, B2), cc.1(A2, B2));
            rep.check(x2 == y2 && x2 == x, || {
                format!("row54: negating both radii changed the answer: {x} vs {x2}")
            });
        }
    }
    // Row 55: exact touch (d2 == r2) must give 0 because the test is strict `<`.
    // Use a 3-4-5 triangle so d2 is computed exactly.
    for (rA, rB) in [(2.0f32, 3.0f32), (0.0, 5.0), (2.5, 2.5), (1.0, 4.0)] {
        let s = rA + rB;
        let A = c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: rA };
        for p in [
            c2v { x: s, y: 0.0 },
            c2v { x: 0.0, y: s },
            c2v { x: s * 0.6, y: s * 0.8 },
            c2v { x: -s * 0.8, y: s * 0.6 },
        ] {
            let B = c2Circle { p, r: rB };
            let (x, y) = (cc.0(A, B), cc.1(A, B));
            rep.check(x == y && x == 0, || {
                format!("row55 exact touch must be 0: C={x} Rust={y} (p={})", show_v(p))
            });
        }
    }
    // Row 56: c2Clampv with inverted bounds returns `lo`, so no normalisation.
    let inverted = c2AABB { min: c2v { x: 5.0, y: 5.0 }, max: c2v { x: -5.0, y: -5.0 } };
    for p in [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: 5.0, y: 5.0 },
        c2v { x: -5.0, y: -5.0 },
        c2v { x: 100.0, y: -100.0 },
    ] {
        for rad in [0.0f32, 1.0, 10.0, 200.0, -3.0] {
            let A = c2Circle { p, r: rad };
            let (x, y) = (ca.0(A, inverted), ca.1(A, inverted));
            rep.check(x == y, || {
                format!(
                    "row56 c2CircletoAABB(inverted box, p={}, r={}): C={x} Rust={y}",
                    show_v(p),
                    show_f32(rad)
                )
            });
            // The C clamps to `lo` = min = (5,5); check against that directly.
            let dx = p.x - 5.0;
            let dy = p.y - 5.0;
            let want = ((dx * dx + dy * dy) < rad * rad) as c_int;
            rep.check(x == want, || {
                format!("row56 expected clamp-to-lo semantics ({want}), got {x}")
            });
        }
    }
    // Row 57: r == 0 or d2 == r2 -> strict `<` -> 0.
    let bb = c2AABB { min: c2v { x: -1.0, y: -1.0 }, max: c2v { x: 1.0, y: 1.0 } };
    for p in [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: 1.0, y: 1.0 },
        c2v { x: 2.0, y: 0.0 },
        c2v { x: -1.0, y: 0.5 },
    ] {
        let A = c2Circle { p, r: 0.0 };
        let (x, y) = (ca.0(A, bb), ca.1(A, bb));
        rep.check(x == y && x == 0, || {
            format!("row57 r==0 must be 0: C={x} Rust={y} (p={})", show_v(p))
        });
    }
    // Exactly `rad` past a face -> d2 == r2 -> 0.
    for rad in [0.5f32, 1.0, 2.0, 4.0] {
        for p in [
            c2v { x: 1.0 + rad, y: 0.0 },
            c2v { x: -1.0 - rad, y: 0.0 },
            c2v { x: 0.0, y: 1.0 + rad },
            c2v { x: 0.0, y: -1.0 - rad },
        ] {
            let A = c2Circle { p, r: rad };
            let (x, y) = (ca.0(A, bb), ca.1(A, bb));
            rep.check(x == y && x == 0, || {
                format!("row57 exact face touch must be 0: C={x} Rust={y} (p={}, r={})", show_v(p), show_f32(rad))
            });
        }
    }
    rep.finish("err54_to_err57_circle_boundaries");
}

// ---------------------------------------------------------------------------
// ERRORS rows 58-61 — c2CircletoCapsule's three regions and the a == b case
// ---------------------------------------------------------------------------

#[test]
fn err58_to_err61_circletocapsule_regions() {
    let l = libs();
    let (c, r) = (
        l.c.sym::<FnCircletoCapsule>("c2CircletoCapsule"),
        l.rs.sym::<FnCircletoCapsule>("c2CircletoCapsule"),
    );
    let mut g = Rng::new(0xD58);
    let mut rep = Report::new();
    let mut regions = [0usize; 3];

    let cap = c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 10.0, y: 0.0 }, r: 2.0 };
    // Classify which region the C takes: da<0 (row58), db<0 (row59), else (row61).
    let classify = |p: c2v, k: &c2Capsule| -> usize {
        let n = c2v { x: k.b.x - k.a.x, y: k.b.y - k.a.y };
        let ap = c2v { x: p.x - k.a.x, y: p.y - k.a.y };
        let da = ap.x * n.x + ap.y * n.y;
        if da < 0.0 {
            return 0;
        }
        let bp = c2v { x: p.x - k.b.x, y: p.y - k.b.y };
        let db = bp.x * n.x + bp.y * n.y;
        if db < 0.0 {
            1
        } else {
            2
        }
    };

    for ti in -40i32..=60 {
        let t = ti as f32 * 0.35;
        for off in [0.0f32, 1.0, 2.0, 2.0000005, 3.0, -2.5, 10.0] {
            let p = c2v { x: t, y: off };
            regions[classify(p, &cap)] += 1;
            for rad in [0.0f32, 0.5, 2.0, -1.0, 100.0] {
                let A = c2Circle { p, r: rad };
                let (x, y) = (c(A, cap), c(A, cap));
                let (x2, y2) = (x, {
                    let _ = y;
                    r(A, cap)
                });
                rep.check(x2 == y2, || {
                    format!(
                        "rows58-61 c2CircletoCapsule(p={}, r={}): C={x2} Rust={y2}",
                        show_v(p),
                        show_f32(rad)
                    )
                });
            }
        }
    }
    for i in 0..3 {
        assert!(regions[i] > 0, "c2CircletoCapsule region {i} never exercised: {regions:?}");
    }
    eprintln!("err58-61: region counts (da<0, db<0, else) = {regions:?}");

    // Row 60: the degenerate capsule a == b makes n == (0,0), so da == 0 and
    // db == 0. Both are >= 0, so the `else` branch runs and the 0/0 division in
    // the middle branch is NEVER reached. Assert that specific outcome: the
    // result must equal a plain circle-vs-circle test against endpoint b.
    let ccirc = (
        l.c.sym::<FnCircletoCircle>("c2CircletoCircle"),
        l.rs.sym::<FnCircletoCircle>("c2CircletoCircle"),
    );
    for _ in 0..3000 {
        let a = g.finite_v();
        let capr = g.radius();
        let deg = c2Capsule { a, b: a, r: capr };
        let A = c2Circle { p: g.finite_v(), r: g.radius() };
        let (x, y) = (c(A, deg), r(A, deg));
        rep.check(x == y, || {
            format!(
                "row60 degenerate capsule: C={x} Rust={y} (circle p={} r={}, cap at {} r={})",
                show_v(A.p),
                show_f32(A.r),
                show_v(a),
                show_f32(capr)
            )
        });
        // Equivalent to circle-vs-circle at b with radius capr, because the
        // `else` branch measures to `B.b` and the test is `d2 < (rA+rB)^2`.
        let want = ccirc.0(A, c2Circle { p: a, r: capr });
        rep.check(x == want, || {
            format!("row60: expected circle-vs-circle-at-b semantics ({want}), got {x}")
        });
        // No NaN may leak from a 0/0 division.
        rep.check(x == 0 || x == 1, || format!("row60 produced a non-boolean {x}"));
    }
    // Row 59: the perpendicular branch divides by dot(n,n). Drive it with a
    // near-zero-length (but non-degenerate) capsule so the divisor is tiny.
    for e in [1.0e-3f32, 1.0e-10, 1.0e-20, f32::MIN_POSITIVE, f32::from_bits(1)] {
        let k = c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: e, y: 0.0 }, r: 1.0 };
        for p in [
            c2v { x: e * 0.5, y: 0.5 },
            c2v { x: e * 0.5, y: 0.0 },
            c2v { x: e * 0.5, y: 1.0e10 },
            c2v { x: 0.0, y: 0.0 },
        ] {
            for rad in [0.0f32, 1.0, 1.0e10] {
                let A = c2Circle { p, r: rad };
                let (x, y) = (c(A, k), r(A, k));
                rep.check(x == y, || {
                    format!(
                        "row59 tiny capsule (e={}): C={x} Rust={y} (p={}, r={})",
                        show_f32(e),
                        show_v(p),
                        show_f32(rad)
                    )
                });
            }
        }
    }
    rep.finish("err58_to_err61_circletocapsule_regions");
}

// ---------------------------------------------------------------------------
// ERRORS rows 62-67 — out-of-range C2_TYPE across the FFI boundary
// ---------------------------------------------------------------------------

#[test]
fn err62_to_err65_collided_invalid_enums() {
    let l = libs();
    let (c, r) = (l.c.sym::<FnCollided>("c2Collided"), l.rs.sym::<FnCollided>("c2Collided"));
    let mut g = Rng::new(0xD62);
    let mut rep = Report::new();

    // Buffers big enough for any shape, filled with overlapping geometry so a
    // missing `default:` would show up as a 1 rather than a 0.
    let overlapping_circle = c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 100.0 };
    let overlapping_box =
        c2AABB { min: c2v { x: -100.0, y: -100.0 }, max: c2v { x: 100.0, y: 100.0 } };
    let overlapping_cap =
        c2Capsule { a: c2v { x: -50.0, y: 0.0 }, b: c2v { x: 50.0, y: 0.0 }, r: 100.0 };

    let bufs = [
        ShapeU { circle: overlapping_circle },
        ShapeU { aabb: overlapping_box },
        ShapeU { capsule: overlapping_cap },
    ];

    let mut probe = |rep: &mut Report, a: &ShapeU, ta: c_int, b: &ShapeU, tb: c_int, tag: &str| {
        let (x, y) = unsafe {
            (
                c(&raw const *a as *const c_void, ta, &raw const *b as *const c_void, tb),
                r(&raw const *a as *const c_void, ta, &raw const *b as *const c_void, tb),
            )
        };
        rep.check(x == y, || format!("c2Collided[{tag}](typeA={ta}, typeB={tb}): C={x} Rust={y}"));
        // Every invalid-enum path returns exactly 0, even though the shapes
        // overlap massively.
        rep.check(x == 0, || {
            format!("c2Collided[{tag}](typeA={ta}, typeB={tb}) must be 0, C returned {x}")
        });
    };

    // Rows 62/63/64: valid typeA, invalid typeB -> the inner `default:` -> 0.
    for (i, ta) in VALID_TYPES.iter().enumerate() {
        for tb in BAD_TYPES {
            for (j, b) in bufs.iter().enumerate() {
                let _ = j;
                probe(&mut rep, &bufs[i], *ta, b, tb, "rows62-64 inner default");
            }
        }
    }
    // Row 65: invalid typeA -> the outer `default:` -> 0, and B is never read.
    for ta in BAD_TYPES {
        for tb in VALID_TYPES.iter().copied().chain(BAD_TYPES) {
            for a in &bufs {
                for b in &bufs {
                    probe(&mut rep, a, ta, b, tb, "row65 outer default");
                }
            }
        }
        // With an invalid typeA the C never dereferences B, so a NULL B must be
        // safe on both sides.
        let (x, y) = unsafe {
            (
                c(bufs[0].circle.p.x.to_bits() as usize as *const c_void, ta, std::ptr::null(), 0),
                r(bufs[0].circle.p.x.to_bits() as usize as *const c_void, ta, std::ptr::null(), 0),
            )
        };
        rep.check(x == y && x == 0, || {
            format!("row65 c2Collided(bogus A, typeA={ta}, NULL B): C={x} Rust={y}")
        });
    }
    // Randomized shapes with invalid enums too, so the result is not accidentally
    // 0 because of the specific geometry.
    for _ in 0..800 {
        let a = ShapeU { capsule: g.capsule() };
        let b = ShapeU { aabb: g.aabb() };
        for ta in BAD_TYPES {
            probe(&mut rep, &a, ta, &b, C2_TYPE_AABB, "row65 random");
        }
        for tb in BAD_TYPES {
            probe(&mut rep, &a, C2_TYPE_CAPSULE, &b, tb, "rows62-64 random");
        }
    }
    rep.finish("err62_to_err65_collided_invalid_enums");
}

#[test]
fn err66_err67_collided_argument_swaps() {
    // The three mixed-type branches reinterpret the pointers the OTHER way
    // round. Asserted against the primitives so a forgotten swap cannot hide
    // behind a symmetric test.
    let l = libs();
    let coll = (l.c.sym::<FnCollided>("c2Collided"), l.rs.sym::<FnCollided>("c2Collided"));
    let cta =
        (l.c.sym::<FnCircletoAABB>("c2CircletoAABB"), l.rs.sym::<FnCircletoAABB>("c2CircletoAABB"));
    let ctc = (
        l.c.sym::<FnCircletoCapsule>("c2CircletoCapsule"),
        l.rs.sym::<FnCircletoCapsule>("c2CircletoCapsule"),
    );
    let atc = (
        l.c.sym::<FnAABBtoCapsule>("c2AABBtoCapsule"),
        l.rs.sym::<FnAABBtoCapsule>("c2AABBtoCapsule"),
    );
    let mut g = Rng::new(0xD66);
    let mut rep = Report::new();

    for _ in 0..4000 {
        let circle = g.circle();
        let bb = g.aabb();
        let cap = g.capsule();

        let cases: [(ShapeU, c_int, ShapeU, c_int, c_int, c_int, &str); 3] = [
            // Row 66: (AABB, CIRCLE) -> c2CircletoAABB(*(c2Circle*)B, *(c2AABB*)A)
            (
                ShapeU { aabb: bb },
                C2_TYPE_AABB,
                ShapeU { circle },
                C2_TYPE_CIRCLE,
                cta.0(circle, bb),
                cta.1(circle, bb),
                "row66 AABB/CIRCLE",
            ),
            // Row 67: (CAPSULE, CIRCLE) -> c2CircletoCapsule(*(c2Circle*)B, *(c2Capsule*)A)
            (
                ShapeU { capsule: cap },
                C2_TYPE_CAPSULE,
                ShapeU { circle },
                C2_TYPE_CIRCLE,
                ctc.0(circle, cap),
                ctc.1(circle, cap),
                "row67 CAPSULE/CIRCLE",
            ),
            // Row 67: (CAPSULE, AABB) -> c2AABBtoCapsule(*(c2AABB*)B, *(c2Capsule*)A)
            (
                ShapeU { capsule: cap },
                C2_TYPE_CAPSULE,
                ShapeU { aabb: bb },
                C2_TYPE_AABB,
                atc.0(bb, cap),
                atc.1(bb, cap),
                "row67 CAPSULE/AABB",
            ),
        ];

        for (a, ta, b, tb, want_c, want_r, tag) in cases {
            let (x, y) = unsafe {
                (
                    coll.0(&raw const a as *const c_void, ta, &raw const b as *const c_void, tb),
                    coll.1(&raw const a as *const c_void, ta, &raw const b as *const c_void, tb),
                )
            };
            rep.check(x == y, || format!("{tag}: C={x} Rust={y}"));
            rep.check(x == want_c && want_c == want_r, || {
                format!("{tag}: dispatch gave {x} but the primitive gives {want_c}/{want_r}")
            });
        }
    }
    rep.finish("err66_err67_collided_argument_swaps");
}

// ---------------------------------------------------------------------------
// ERRORS row 68 — aabb() is a 3-bit mask
// ---------------------------------------------------------------------------

#[test]
fn err68_aabb_result_range() {
    let l = libs();
    let (c, r) = (l.c.sym::<FnAabb>("aabb"), l.rs.sym::<FnAabb>("aabb"));
    let mut g = Rng::new(0xD68);
    let mut rep = Report::new();
    for _ in 0..20000 {
        for q in [
            [g.nasty_f32(), g.nasty_f32(), g.nasty_f32(), g.nasty_f32()],
            [g.finite_f32(), g.finite_f32(), g.finite_f32(), g.finite_f32()],
        ] {
            let (x, y) = (c(q[0], q[1], q[2], q[3]), r(q[0], q[1], q[2], q[3]));
            rep.check(x == y, || {
                format!(
                    "row68 aabb({}, {}, {}, {}): C={x} Rust={y}",
                    show_f32(q[0]),
                    show_f32(q[1]),
                    show_f32(q[2]),
                    show_f32(q[3])
                )
            });
            rep.check((0..=7).contains(&x), || {
                format!("row68 aabb returned {x}, outside the 3-bit range [0,7]")
            });
        }
    }
    // All-NaN and all-inf inputs, both orders.
    for q in [
        [f32::NAN; 4],
        [f32::INFINITY; 4],
        [f32::NEG_INFINITY; 4],
        [f32::NEG_INFINITY, f32::NEG_INFINITY, f32::INFINITY, f32::INFINITY],
        [f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY],
        [0.0, -0.0, -0.0, 0.0],
        [f32::MAX, f32::MIN, f32::MIN, f32::MAX],
    ] {
        let (x, y) = (c(q[0], q[1], q[2], q[3]), r(q[0], q[1], q[2], q[3]));
        rep.check(x == y && (0..=7).contains(&x), || {
            format!("row68 aabb(special): C={x} Rust={y}")
        });
    }
    rep.finish("err68_aabb_result_range");
}

// ---------------------------------------------------------------------------
// ERRORS rows 69-70 — ternary-select NaN asymmetry and un-swapped clamp bounds
// ---------------------------------------------------------------------------

#[test]
fn err69_err70_minmax_nan_and_inverted_clamp() {
    let l = libs();
    let mx = (l.c.sym::<FnVVV>("c2Maxv"), l.rs.sym::<FnVVV>("c2Maxv"));
    let mn = (l.c.sym::<FnVVV>("c2Minv"), l.rs.sym::<FnVVV>("c2Minv"));
    let cl = (l.c.sym::<FnVVVV>("c2Clampv"), l.rs.sym::<FnVVVV>("c2Clampv"));
    let mut g = Rng::new(0xD69);
    let mut rep = Report::new();

    // Row 69: `((a.x) > (b.x) ? (a.x) : (b.x))` is a ternary select, NOT fmaxf.
    // With a NaN operand the comparison is false, so the SECOND operand wins --
    // for both c2Maxv and c2Minv. Assert that exact asymmetry.
    let nan = f32::NAN;
    for other in [0.0f32, -0.0, 1.0, -1.0, f32::INFINITY, f32::NEG_INFINITY, f32::MAX] {
        // NaN in `a`: `NaN > b` is false -> b; `NaN < b` is false -> b.
        let a = c2v { x: nan, y: nan };
        let b = c2v { x: other, y: other };
        let (x, y) = (mx.0(a, b), mx.1(a, b));
        rep.check(same_v(x, y), || format!("row69 c2Maxv(NaN, {}): C={} Rust={}", show_f32(other), show_v(x), show_v(y)));
        rep.check(same_f32(x.x, other) && same_f32(x.y, other), || {
            format!("row69 c2Maxv(NaN, {}) must return the SECOND operand, got {}", show_f32(other), show_v(x))
        });
        let (x, y) = (mn.0(a, b), mn.1(a, b));
        rep.check(same_v(x, y), || format!("row69 c2Minv(NaN, {}): C={} Rust={}", show_f32(other), show_v(x), show_v(y)));
        rep.check(same_f32(x.x, other) && same_f32(x.y, other), || {
            format!("row69 c2Minv(NaN, {}) must return the SECOND operand, got {}", show_f32(other), show_v(x))
        });
        // NaN in `b`: `a > NaN` is false -> b == NaN. So NaN wins here.
        let a = c2v { x: other, y: other };
        let b = c2v { x: nan, y: nan };
        let (x, y) = (mx.0(a, b), mx.1(a, b));
        rep.check(same_v(x, y), || format!("row69 c2Maxv({}, NaN): C={} Rust={}", show_f32(other), show_v(x), show_v(y)));
        rep.check(x.x.is_nan() && x.y.is_nan(), || {
            format!("row69 c2Maxv({}, NaN) must return NaN, got {}", show_f32(other), show_v(x))
        });
        let (x, y) = (mn.0(a, b), mn.1(a, b));
        rep.check(same_v(x, y), || format!("row69 c2Minv({}, NaN): C={} Rust={}", show_f32(other), show_v(x), show_v(y)));
        rep.check(x.x.is_nan() && x.y.is_nan(), || {
            format!("row69 c2Minv({}, NaN) must return NaN, got {}", show_f32(other), show_v(x))
        });
    }
    // Signed zeros: `0.0 > -0.0` is false, so c2Maxv(+0,-0) returns -0.
    let pz = c2v { x: 0.0, y: 0.0 };
    let nz = c2v { x: -0.0, y: -0.0 };
    for (a, b) in [(pz, nz), (nz, pz)] {
        let (x, y) = (mx.0(a, b), mx.1(a, b));
        rep.check(same_v(x, y) && same_v(x, b), || {
            format!("row69 c2Maxv signed zero must return the second operand: C={}", show_v(x))
        });
        let (x, y) = (mn.0(a, b), mn.1(a, b));
        rep.check(same_v(x, y) && same_v(x, b), || {
            format!("row69 c2Minv signed zero must return the second operand: C={}", show_v(x))
        });
    }

    // Row 70: `c2Clampv(a, lo, hi) = c2Maxv(lo, c2Minv(a, hi))`. With lo > hi the
    // bounds are NOT swapped, and the result is always exactly `lo`.
    for _ in 0..4000 {
        let a = g.nasty_v();
        let p = g.finite_v();
        let q = g.finite_v();
        let lo = c2v { x: p.x.max(q.x), y: p.y.max(q.y) };
        let hi = c2v { x: p.x.min(q.x), y: p.y.min(q.y) };
        if lo.x <= hi.x || lo.y <= hi.y || a.x.is_nan() || a.y.is_nan() {
            // Not a strictly inverted range, or NaN input; only compare.
            let (x, y) = (cl.0(a, lo, hi), cl.1(a, lo, hi));
            rep.check(same_v(x, y), || {
                format!("row70 c2Clampv({}, {}, {}): C={} Rust={}", show_v(a), show_v(lo), show_v(hi), show_v(x), show_v(y))
            });
            continue;
        }
        let (x, y) = (cl.0(a, lo, hi), cl.1(a, lo, hi));
        rep.check(same_v(x, y), || {
            format!("row70 c2Clampv({}, {}, {}): C={} Rust={}", show_v(a), show_v(lo), show_v(hi), show_v(x), show_v(y))
        });
        rep.check(same_v(x, lo), || {
            format!("row70 inverted clamp must return lo={}, got {}", show_v(lo), show_v(x))
        });
    }
    // lo == hi collapses to that value.
    for _ in 0..500 {
        let a = g.nasty_v();
        let v = g.finite_v();
        let (x, y) = (cl.0(a, v, v), cl.1(a, v, v));
        rep.check(same_v(x, y), || {
            format!("row70 c2Clampv(lo == hi): C={} Rust={}", show_v(x), show_v(y))
        });
    }
    // NaN in lo / hi.
    for (lo, hi) in [
        (c2v { x: nan, y: nan }, c2v { x: 1.0, y: 1.0 }),
        (c2v { x: 0.0, y: 0.0 }, c2v { x: nan, y: nan }),
        (c2v { x: nan, y: nan }, c2v { x: nan, y: nan }),
    ] {
        for a in [c2v { x: 0.5, y: 0.5 }, c2v { x: nan, y: 0.0 }, c2v { x: -5.0, y: 5.0 }] {
            let (x, y) = (cl.0(a, lo, hi), cl.1(a, lo, hi));
            rep.check(same_v(x, y), || {
                format!("row70 c2Clampv NaN bounds: C={} Rust={}", show_v(x), show_v(y))
            });
        }
    }
    rep.finish("err69_err70_minmax_nan_and_inverted_clamp");
}
