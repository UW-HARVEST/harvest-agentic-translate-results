//! Phase B — valid-path differential tests, CONFIGS.md rows 27..=46.
//!
//! The L2 collision kernels called DIRECTLY through their exported symbols
//! (not via the `c2Collided` dispatcher or the `circle_collide` wrapper), so
//! the composed pipeline `c2Clampv`→`c2Minv`/`c2Maxv`→`c2Sub`→`c2Dot` is
//! exercised end to end.

mod common;
use common::*;

const SEED: u64 = 0x243F_6A88_85A3_08D3;
const N: usize = 20_000;

// ===========================================================================
// c2CircletoCircle — rows 27..=31
// ===========================================================================

/// Row 27 — random finite circles with positive radii, centres clustered so
/// that both hits and misses occur (asserted).
#[test]
fn cfg27_ctc_random_finite() {
    let mut rng = Rng::new(SEED ^ 27);
    let mut hits = 0usize;
    for _ in 0..N {
        let a = C2Circle {
            p: C2v { x: rng.range(-50.0, 50.0), y: rng.range(-50.0, 50.0) },
            r: rng.range(0.0, 30.0),
        };
        let b = C2Circle {
            p: C2v { x: rng.range(-50.0, 50.0), y: rng.range(-50.0, 50.0) },
            r: rng.range(0.0, 30.0),
        };
        let got = diff(
            || format!("c2CircletoCircle({a:?}, {b:?})"),
            |api| (api.c2CircletoCircle)(a, b),
        );
        hits += (got != 0) as usize;
    }
    assert!(hits > N / 20 && hits < N - N / 20, "unbalanced hit rate: {hits}/{N}");
}

/// Row 28 — exact-touch boundary of the strict `<`: distance == `A.r + B.r`,
/// plus one ULP on either side of the distance and of the radii.
#[test]
fn cfg28_ctc_exact_touch() {
    let mut rng = Rng::new(SEED ^ 28);
    for _ in 0..N {
        // powers of two keep the arithmetic exact
        let ra = (2.0f32).powi(rng.below(20) as i32 - 8);
        let rb = (2.0f32).powi(rng.below(20) as i32 - 8);
        let d = ra + rb;
        let cx = rng.range(-8.0, 8.0).round();
        let cy = rng.range(-8.0, 8.0).round();
        let a = C2Circle { p: C2v { x: cx, y: cy }, r: ra };
        for dd in [d, ulp_down(d), ulp_up(d), d * 0.5, d * 2.0] {
            for (bx, by) in [(cx + dd, cy), (cx, cy + dd), (cx - dd, cy), (cx, cy - dd)] {
                let b = C2Circle { p: C2v { x: bx, y: by }, r: rb };
                diff(
                    || format!("c2CircletoCircle({a:?}, {b:?}) [touch dd={dd}]"),
                    |api| (api.c2CircletoCircle)(a, b),
                );
            }
        }
    }
}

/// Row 29 — concentric circles, zero radii, one zero radius.
#[test]
fn cfg29_ctc_concentric_and_zero_radii() {
    let mut rng = Rng::new(SEED ^ 29);
    for _ in 0..N {
        let p = rng.v_finite();
        let radii = [0.0f32, -0.0, rng.radius(), rng.radius()];
        for &ra in &radii {
            for &rb in &radii {
                let a = C2Circle { p, r: ra };
                let b = C2Circle { p, r: rb };
                diff(
                    || format!("c2CircletoCircle({a:?}, {b:?}) [concentric]"),
                    |api| (api.c2CircletoCircle)(a, b),
                );
            }
        }
    }
}

/// Row 30 — negative radii and radii summing to zero (no validation in the C).
#[test]
fn cfg30_ctc_negative_radii() {
    let mut rng = Rng::new(SEED ^ 30);
    for _ in 0..N {
        let r = rng.range(0.0, 40.0);
        let pa = C2v { x: rng.range(-40.0, 40.0), y: rng.range(-40.0, 40.0) };
        let pb = C2v { x: rng.range(-40.0, 40.0), y: rng.range(-40.0, 40.0) };
        for (ra, rb) in [
            (-r, -r),
            (-r, r),      // sums to exactly 0
            (r, -r),
            (-r, r * 2.0),
            (r * 2.0, -r),
            (-r, 0.0),
        ] {
            let a = C2Circle { p: pa, r: ra };
            let b = C2Circle { p: pb, r: rb };
            diff(
                || format!("c2CircletoCircle({a:?}, {b:?}) [neg radii]"),
                |api| (api.c2CircletoCircle)(a, b),
            );
        }
    }
}

/// Row 31 — arbitrary bit patterns for all six `f32` fields.
#[test]
fn cfg31_ctc_any_bits() {
    let mut rng = Rng::new(SEED ^ 31);
    for i in 0..N {
        let (pa, ra, pb, rb) = if i % 2 == 0 {
            (rng.v_any(), rng.any_f32(), rng.v_any(), rng.any_f32())
        } else {
            (rng.v_path(), rng.pathological_f32(), rng.v_path(), rng.pathological_f32())
        };
        let a = C2Circle { p: pa, r: ra };
        let b = C2Circle { p: pb, r: rb };
        diff(
            || format!("c2CircletoCircle({a:?}, {b:?})"),
            |api| (api.c2CircletoCircle)(a, b),
        );
    }
    // full special grid on the radii, random-but-fixed centres
    let mut rng = Rng::new(SEED ^ 0x3131);
    for &ra in special_values() {
        for &rb in special_values() {
            let a = C2Circle { p: rng.v_finite(), r: ra };
            let b = C2Circle { p: rng.v_finite(), r: rb };
            diff(
                || format!("c2CircletoCircle({a:?}, {b:?})"),
                |api| (api.c2CircletoCircle)(a, b),
            );
        }
    }
}

// ===========================================================================
// c2CircletoAABB — rows 32..=37
// ===========================================================================

fn rand_box(rng: &mut Rng) -> C2Aabb {
    let x0 = rng.range(-50.0, 50.0);
    let y0 = rng.range(-50.0, 50.0);
    let w = rng.range(0.0, 60.0);
    let h = rng.range(0.0, 60.0);
    C2Aabb {
        min: C2v { x: x0, y: y0 },
        max: C2v { x: x0 + w, y: y0 + h },
    }
}

/// Row 32 — random finite circle vs well-formed box, with the centre steered
/// inside / on a face / on a corner / outside. Asserts each placement class is
/// reached and that hits and misses both occur.
#[test]
fn cfg32_ctaabb_random_finite_placements() {
    let mut rng = Rng::new(SEED ^ 32);
    let mut hits = 0usize;
    let mut classes = [0usize; 4];
    for i in 0..N {
        let bx = rand_box(&mut rng);
        let class = i % 4;
        classes[class] += 1;
        let p = match class {
            // inside
            0 => C2v {
                x: rng.range(bx.min.x, bx.max.x),
                y: rng.range(bx.min.y, bx.max.y),
            },
            // on a face
            1 => match rng.below(4) {
                0 => C2v { x: bx.min.x, y: rng.range(bx.min.y, bx.max.y) },
                1 => C2v { x: bx.max.x, y: rng.range(bx.min.y, bx.max.y) },
                2 => C2v { x: rng.range(bx.min.x, bx.max.x), y: bx.min.y },
                _ => C2v { x: rng.range(bx.min.x, bx.max.x), y: bx.max.y },
            },
            // on a corner
            2 => match rng.below(4) {
                0 => bx.min,
                1 => bx.max,
                2 => C2v { x: bx.min.x, y: bx.max.y },
                _ => C2v { x: bx.max.x, y: bx.min.y },
            },
            // outside
            _ => C2v {
                x: rng.range(bx.min.x - 40.0, bx.max.x + 40.0),
                y: rng.range(bx.min.y - 40.0, bx.max.y + 40.0),
            },
        };
        let a = C2Circle { p, r: rng.range(0.0, 25.0) };
        let got = diff(
            || format!("c2CircletoAABB({a:?}, {bx:?}) [class {class}]"),
            |api| (api.c2CircletoAABB)(a, bx),
        );
        hits += (got != 0) as usize;
    }
    assert!(classes.iter().all(|c| *c > 0));
    assert!(hits > N / 20 && hits < N - N / 20, "unbalanced hit rate: {hits}/{N}");
}

/// Row 33 — degenerate box (`min == max`, i.e. a point).
#[test]
fn cfg33_ctaabb_degenerate_box() {
    let mut rng = Rng::new(SEED ^ 33);
    for _ in 0..N {
        let m = rng.v_finite();
        let bx = C2Aabb { min: m, max: m };
        let a = C2Circle { p: rng.v_finite(), r: rng.radius() };
        diff(
            || format!("c2CircletoAABB({a:?}, {bx:?}) [point box]"),
            |api| (api.c2CircletoAABB)(a, bx),
        );
        // and a box degenerate in one axis only
        let bx2 = C2Aabb {
            min: m,
            max: C2v { x: m.x, y: m.y + rng.range(0.0, 20.0) },
        };
        let bx3 = C2Aabb {
            min: m,
            max: C2v { x: m.x + rng.range(0.0, 20.0), y: m.y },
        };
        for b in [bx2, bx3] {
            diff(
                || format!("c2CircletoAABB({a:?}, {b:?}) [degenerate axis]"),
                |api| (api.c2CircletoAABB)(a, b),
            );
        }
    }
}

/// Row 34 — inverted box (`min > max`) in one or both axes; never validated.
#[test]
fn cfg34_ctaabb_inverted_box() {
    let mut rng = Rng::new(SEED ^ 34);
    for _ in 0..N {
        let lo = C2v { x: rng.range(-50.0, 50.0), y: rng.range(-50.0, 50.0) };
        let hi = C2v { x: lo.x + rng.range(0.0, 40.0), y: lo.y + rng.range(0.0, 40.0) };
        let a = C2Circle { p: rng.v_finite(), r: rng.radius() };
        let variants = [
            C2Aabb { min: hi, max: lo },                                     // both axes
            C2Aabb { min: C2v { x: hi.x, y: lo.y }, max: C2v { x: lo.x, y: hi.y } }, // x only
            C2Aabb { min: C2v { x: lo.x, y: hi.y }, max: C2v { x: hi.x, y: lo.y } }, // y only
        ];
        for b in variants {
            diff(
                || format!("c2CircletoAABB({a:?}, {b:?}) [inverted]"),
                |api| (api.c2CircletoAABB)(a, b),
            );
        }
    }
}

/// Row 35 — exact-touch: centre exactly `A.r` away from the nearest face /
/// corner, plus one ULP either side.
#[test]
fn cfg35_ctaabb_exact_touch() {
    let mut rng = Rng::new(SEED ^ 35);
    for _ in 0..N {
        let r = (2.0f32).powi(rng.below(16) as i32 - 6);
        let x0 = rng.range(-8.0, 8.0).round();
        let y0 = rng.range(-8.0, 8.0).round();
        let w = rng.range(1.0, 16.0).round();
        let h = rng.range(1.0, 16.0).round();
        let bx = C2Aabb {
            min: C2v { x: x0, y: y0 },
            max: C2v { x: x0 + w, y: y0 + h },
        };
        for d in [r, ulp_down(r), ulp_up(r)] {
            // face touches (4)
            let faces = [
                C2v { x: bx.max.x + d, y: y0 + h * 0.5 },
                C2v { x: bx.min.x - d, y: y0 + h * 0.5 },
                C2v { x: x0 + w * 0.5, y: bx.max.y + d },
                C2v { x: x0 + w * 0.5, y: bx.min.y - d },
            ];
            for p in faces {
                let a = C2Circle { p, r };
                diff(
                    || format!("c2CircletoAABB({a:?}, {bx:?}) [face touch d={d}]"),
                    |api| (api.c2CircletoAABB)(a, bx),
                );
            }
            // corner touches (4), offset along the diagonal by d/sqrt(2)
            let dd = d * std::f32::consts::FRAC_1_SQRT_2;
            let corners = [
                C2v { x: bx.max.x + dd, y: bx.max.y + dd },
                C2v { x: bx.min.x - dd, y: bx.max.y + dd },
                C2v { x: bx.max.x + dd, y: bx.min.y - dd },
                C2v { x: bx.min.x - dd, y: bx.min.y - dd },
            ];
            for p in corners {
                let a = C2Circle { p, r };
                diff(
                    || format!("c2CircletoAABB({a:?}, {bx:?}) [corner touch d={d}]"),
                    |api| (api.c2CircletoAABB)(a, bx),
                );
            }
        }
    }
}

/// Row 36 — `A.r` = 0 / negative / inf / NaN / denormal against random boxes.
#[test]
fn cfg36_ctaabb_radius_classes() {
    let mut rng = Rng::new(SEED ^ 36);
    let radii = [
        0.0f32,
        -0.0,
        -1.0,
        -1e30,
        f32::INFINITY,
        f32::NEG_INFINITY,
        qnan(1, false),
        qnan(2, true),
        snan(3, false),
        f32::from_bits(1),
        f32::MAX,
        f32::MIN,
    ];
    for _ in 0..(N / 4) {
        let bx = rand_box(&mut rng);
        let p = rng.v_finite();
        for &r in &radii {
            let a = C2Circle { p, r };
            diff(
                || format!("c2CircletoAABB({a:?}, {bx:?}) [radius class]"),
                |api| (api.c2CircletoAABB)(a, bx),
            );
        }
    }
}

/// Row 37 — arbitrary bit patterns for all six `f32` fields.
#[test]
fn cfg37_ctaabb_any_bits() {
    let mut rng = Rng::new(SEED ^ 37);
    for i in 0..N {
        let (p, r, mn, mx) = if i % 2 == 0 {
            (rng.v_any(), rng.any_f32(), rng.v_any(), rng.v_any())
        } else {
            (rng.v_path(), rng.pathological_f32(), rng.v_path(), rng.v_path())
        };
        let a = C2Circle { p, r };
        let b = C2Aabb { min: mn, max: mx };
        diff(
            || format!("c2CircletoAABB({a:?}, {b:?})"),
            |api| (api.c2CircletoAABB)(a, b),
        );
    }
    // special grid on the box bounds
    let mut rng = Rng::new(SEED ^ 0x3737);
    for &lo in special_values() {
        for &hi in special_values() {
            let a = C2Circle { p: rng.v_finite(), r: rng.radius() };
            let b = C2Aabb {
                min: C2v { x: lo, y: hi },
                max: C2v { x: hi, y: lo },
            };
            diff(
                || format!("c2CircletoAABB({a:?}, {b:?})"),
                |api| (api.c2CircletoAABB)(a, b),
            );
        }
    }
}

// ===========================================================================
// c2CircletoCapsule — rows 38..=46
// ===========================================================================

fn rand_capsule(rng: &mut Rng) -> C2Capsule {
    let ax = rng.range(-50.0, 50.0);
    let ay = rng.range(-50.0, 50.0);
    // keep the axis non-degenerate
    let mut nx = rng.range(-60.0, 60.0);
    let mut ny = rng.range(-60.0, 60.0);
    if nx == 0.0 && ny == 0.0 {
        nx = 1.0;
        ny = 2.0;
    }
    C2Capsule {
        a: C2v { x: ax, y: ay },
        b: C2v { x: ax + nx, y: ay + ny },
        r: rng.range(0.0, 25.0),
    }
}

/// Rows 38/39/40 helper: drive `t` into a target region and check coverage.
fn capsule_region_row(row: u32, seed: u64, t_lo: f32, t_hi: f32, want: u8) {
    let mut rng = Rng::new(seed);
    let mut in_region = 0usize;
    let mut hits = 0usize;
    for _ in 0..N {
        let cap = rand_capsule(&mut rng);
        let t = rng.range(t_lo, t_hi);
        let s = rng.range(-0.6, 0.6);
        let p = point_on_capsule_axis(cap, t, s);
        let a = C2Circle { p, r: rng.range(0.0, 25.0) };
        if capsule_region(p, cap) == want {
            in_region += 1;
        }
        let got = diff(
            || format!("[row {row}] c2CircletoCapsule({a:?}, {cap:?}) t={t} s={s}"),
            |api| (api.c2CircletoCapsule)(a, cap),
        );
        hits += (got != 0) as usize;
    }
    assert!(
        in_region > N / 2,
        "row {row}: only {in_region}/{N} samples landed in region {want}"
    );
    assert!(hits > 0, "row {row}: no collision ever reported");
}

/// Row 38 — branch `da < 0`: the before-`B.a` cap.
#[test]
fn cfg38_ctcapsule_branch_before_a() {
    capsule_region_row(38, SEED ^ 38, -3.0, -0.02, 0);
}

/// Row 39 — branch `da >= 0 && db < 0`: the shaft (the only branch that runs
/// the unguarded `da / c2Dot(n,n)` division and `c2Mulvs`).
#[test]
fn cfg39_ctcapsule_branch_shaft() {
    capsule_region_row(39, SEED ^ 39, 0.02, 0.98, 1);
}

/// Row 40 — branch `da >= 0 && db >= 0`: the after-`B.b` cap.
#[test]
fn cfg40_ctcapsule_branch_after_b() {
    capsule_region_row(40, SEED ^ 40, 1.02, 4.0, 2);
}

/// Row 41 — the branch boundaries themselves: `da == 0` and `db == 0` exactly.
#[test]
fn cfg41_ctcapsule_branch_boundaries() {
    let mut rng = Rng::new(SEED ^ 41);
    for _ in 0..N {
        // axis-aligned capsule with power-of-two length so `da`/`db` are exact
        let l = (2.0f32).powi(rng.below(12) as i32 - 3);
        let ax = rng.range(-8.0, 8.0).round();
        let ay = rng.range(-8.0, 8.0).round();
        let cap = C2Capsule {
            a: C2v { x: ax, y: ay },
            b: C2v { x: ax + l, y: ay },
            r: (2.0f32).powi(rng.below(10) as i32 - 3),
        };
        let s = rng.range(-20.0, 20.0).round();
        // da == 0 exactly: p.x == cap.a.x
        // db == 0 exactly: p.x == cap.b.x
        for p in [
            C2v { x: ax, y: ay + s },
            C2v { x: ax + l, y: ay + s },
            C2v { x: ulp_down(ax), y: ay + s },
            C2v { x: ulp_up(ax + l), y: ay + s },
            C2v { x: ax, y: ay },
            C2v { x: ax + l, y: ay },
        ] {
            let a = C2Circle { p, r: rng.range(0.0, 25.0) };
            diff(
                || format!("c2CircletoCapsule({a:?}, {cap:?}) [boundary]"),
                |api| (api.c2CircletoCapsule)(a, cap),
            );
        }
    }
}

/// Row 42 — degenerate capsule `B.a == B.b`, so `n == (0,0)` and the C divides
/// `da` by `+0.0` with no guard.
#[test]
fn cfg42_ctcapsule_degenerate_axis() {
    let mut rng = Rng::new(SEED ^ 42);
    for _ in 0..N {
        let m = C2v { x: rng.range(-50.0, 50.0), y: rng.range(-50.0, 50.0) };
        let cap = C2Capsule { a: m, b: m, r: rng.range(0.0, 25.0) };
        let ps = [
            m,                                                     // da == 0 -> 0/0
            C2v { x: m.x + rng.range(-40.0, 40.0), y: m.y },
            C2v { x: m.x, y: m.y + rng.range(-40.0, 40.0) },
            rng.v_finite(),
        ];
        for p in ps {
            let a = C2Circle { p, r: rng.range(0.0, 25.0) };
            diff(
                || format!("c2CircletoCapsule({a:?}, {cap:?}) [degenerate axis]"),
                |api| (api.c2CircletoCapsule)(a, cap),
            );
        }
        // near-degenerate: axis one ULP long
        let cap2 = C2Capsule {
            a: m,
            b: C2v { x: ulp_up(m.x), y: m.y },
            r: rng.range(0.0, 25.0),
        };
        let a = C2Circle { p: rng.v_finite(), r: rng.radius() };
        diff(
            || format!("c2CircletoCapsule({a:?}, {cap2:?}) [1-ulp axis]"),
            |api| (api.c2CircletoCapsule)(a, cap2),
        );
    }
}

/// Row 43 — axis-aligned (`n.y == 0`, then `n.x == 0`) and diagonal capsules.
#[test]
fn cfg43_ctcapsule_axis_orientations() {
    let mut rng = Rng::new(SEED ^ 43);
    for _ in 0..N {
        let ax = rng.range(-50.0, 50.0);
        let ay = rng.range(-50.0, 50.0);
        let l = rng.range(-60.0, 60.0);
        let r = rng.range(0.0, 25.0);
        let caps = [
            // horizontal
            C2Capsule { a: C2v { x: ax, y: ay }, b: C2v { x: ax + l, y: ay }, r },
            // vertical
            C2Capsule { a: C2v { x: ax, y: ay }, b: C2v { x: ax, y: ay + l }, r },
            // 45-degree diagonal
            C2Capsule { a: C2v { x: ax, y: ay }, b: C2v { x: ax + l, y: ay + l }, r },
            // anti-diagonal
            C2Capsule { a: C2v { x: ax, y: ay }, b: C2v { x: ax + l, y: ay - l }, r },
        ];
        let p = C2v { x: rng.range(-90.0, 90.0), y: rng.range(-90.0, 90.0) };
        let a = C2Circle { p, r: rng.range(0.0, 25.0) };
        for cap in caps {
            diff(
                || format!("c2CircletoCapsule({a:?}, {cap:?}) [orientation]"),
                |api| (api.c2CircletoCapsule)(a, cap),
            );
        }
    }
}

/// Row 44 — exact-touch on the shaft and on each cap, plus one ULP either side.
#[test]
fn cfg44_ctcapsule_exact_touch() {
    let mut rng = Rng::new(SEED ^ 44);
    for _ in 0..N {
        // powers of two everywhere so the projection arithmetic is exact
        let l = (2.0f32).powi(rng.below(10) as i32);
        let ra = (2.0f32).powi(rng.below(10) as i32 - 4);
        let rb = (2.0f32).powi(rng.below(10) as i32 - 4);
        let rt = ra + rb;
        let ax = rng.range(-8.0, 8.0).round();
        let ay = rng.range(-8.0, 8.0).round();
        let cap = C2Capsule {
            a: C2v { x: ax, y: ay },
            b: C2v { x: ax + l, y: ay },
            r: rb,
        };
        for d in [rt, ulp_down(rt), ulp_up(rt)] {
            // shaft: perpendicular offset of exactly d at t = 1/2
            let p_shaft = C2v { x: ax + l * 0.5, y: ay + d };
            // caps: along the axis, exactly d beyond each endpoint
            let p_cap_a = C2v { x: ax - d, y: ay };
            let p_cap_b = C2v { x: ax + l + d, y: ay };
            for p in [p_shaft, p_cap_a, p_cap_b] {
                let a = C2Circle { p, r: ra };
                diff(
                    || format!("c2CircletoCapsule({a:?}, {cap:?}) [touch d={d}]"),
                    |api| (api.c2CircletoCapsule)(a, cap),
                );
            }
        }
    }
}

/// Row 45 — radius classes: 0, negative, inf, NaN, denormal, sums overflowing.
#[test]
fn cfg45_ctcapsule_radius_classes() {
    let mut rng = Rng::new(SEED ^ 45);
    let radii = [
        0.0f32,
        -0.0,
        -1.0,
        -1e30,
        f32::INFINITY,
        f32::NEG_INFINITY,
        qnan(7, false),
        qnan(8, true),
        snan(9, true),
        f32::from_bits(1),
        f32::MAX,
        f32::MIN,
    ];
    for _ in 0..(N / 16) {
        let mut cap = rand_capsule(&mut rng);
        let p = rng.v_finite();
        for &ra in &radii {
            for &rb in &radii {
                cap.r = rb;
                let a = C2Circle { p, r: ra };
                diff(
                    || format!("c2CircletoCapsule({a:?}, {cap:?}) [radius classes]"),
                    |api| (api.c2CircletoCapsule)(a, cap),
                );
            }
        }
    }
}

/// Row 46 — arbitrary bit patterns for all seven `f32` fields.
#[test]
fn cfg46_ctcapsule_any_bits() {
    let mut rng = Rng::new(SEED ^ 46);
    for i in 0..N {
        let (p, r, ca, cb, cr) = if i % 2 == 0 {
            (rng.v_any(), rng.any_f32(), rng.v_any(), rng.v_any(), rng.any_f32())
        } else {
            (
                rng.v_path(),
                rng.pathological_f32(),
                rng.v_path(),
                rng.v_path(),
                rng.pathological_f32(),
            )
        };
        let a = C2Circle { p, r };
        let cap = C2Capsule { a: ca, b: cb, r: cr };
        diff(
            || format!("c2CircletoCapsule({a:?}, {cap:?})"),
            |api| (api.c2CircletoCapsule)(a, cap),
        );
    }
    // special grid across the axis endpoints (drives NaN/inf into `n`)
    let mut rng = Rng::new(SEED ^ 0x4646);
    for &u in special_values() {
        for &v in special_values() {
            let cap = C2Capsule {
                a: C2v { x: u, y: v },
                b: C2v { x: v, y: u },
                r: rng.radius(),
            };
            let a = C2Circle { p: rng.v_finite(), r: rng.radius() };
            diff(
                || format!("c2CircletoCapsule({a:?}, {cap:?})"),
                |api| (api.c2CircletoCapsule)(a, cap),
            );
        }
    }
}
