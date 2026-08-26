//! Phase B — valid-path differential tests, rows 36..47 of `CONFIGS.md`
//! (`c2RaytoCapsule`, driven directly through the `.so` exports).
//!
//! Rays are built in the capsule's local frame (`M.x` across, `M.y` along the
//! axis) so that each of the ten distinct control-flow branches of
//! `c2RaytoCapsule` can be targeted; every case is then classified with
//! `classify_capsule`, which re-walks the C control flow using the C library's
//! own exported primitives, and each row asserts that its intended branch was
//! actually reached.

#![allow(non_snake_case)]

mod common;
use common::*;

const N: usize = 2000;

fn rand_capsule(rng: &mut Rng) -> C2Capsule {
    let a = v(rng.range(-40.0, 40.0), rng.range(-40.0, 40.0));
    let dir = rng.dir();
    let len = rng.range(0.5, 40.0);
    C2Capsule {
        a,
        b: vadd(a, vscale(dir, len)),
        r: rng.range(0.05, 10.0),
    }
}

struct CapCase {
    ray: C2Ray,
    cap: C2Capsule,
}

/// Build a ray from capsule-local origin `(lx, ly)` and local direction
/// `(dx, dy)` with sweep length `t`.
fn local_ray(cap: &C2Capsule, lx: f32, ly: f32, dx: f32, dy: f32, t: f32) -> C2Ray {
    C2Ray {
        p: cap_local_point(cap, lx, ly),
        d: cap_local_dir(cap, dx, dy),
        t,
    }
}

fn run_cap_row(row: &str, seed: u64, want: &[CapBranch], min_each: usize, mk: fn(&mut Rng) -> CapCase) {
    let (c, r) = apis();
    let mut rng = Rng::new(seed);
    let mut d = Diff::new(row);
    for _ in 0..N {
        let CapCase { ray, cap } = mk(&mut rng);
        let br = classify_capsule(c, ray, cap);
        d.tag(br.name());
        d.check_call(
            || format!("{} {} [{}]", rayshow(&ray), capshow(&cap), br.name()),
            call_raytocapsule(c, ray, cap),
            call_raytocapsule(r, ray, cap),
        );
    }
    for w in want {
        d.require_tag(w.name(), min_each);
    }
    d.finish();
}

/* ------------------------------------------------------------------ row 36 - */

#[test]
fn row36_capsule_origin_in_slab_box() {
    run_cap_row(
        "36: c2RaytoCapsule origin inside slab bbox",
        0x36_0001,
        &[CapBranch::InSlabBox],
        N * 8 / 10,
        |rng| {
            let cap = rand_capsule(rng);
            let len = cap_len(&cap);
            let ray = local_ray(
                &cap,
                rng.range(-0.9, 0.9) * cap.r,
                rng.range(0.05, 0.95) * len,
                rng.range(-1.0, 1.0),
                rng.range(-1.0, 1.0),
                rng.range(0.0, 50.0),
            );
            CapCase { ray, cap }
        },
    );
}

/* --------------------------------------------------------------- rows 37/38 */

#[test]
fn row37_capsule_origin_in_cap_a() {
    run_cap_row(
        "37: c2RaytoCapsule origin inside cap circle a",
        0x37_0001,
        &[CapBranch::InCapA],
        N * 7 / 10,
        |rng| {
            let cap = rand_capsule(rng);
            let ray = local_ray(
                &cap,
                rng.range(-0.6, 0.6) * cap.r,
                -rng.range(0.02, 0.6) * cap.r,
                rng.range(-1.0, 1.0),
                rng.range(-1.0, 1.0),
                rng.range(0.0, 50.0),
            );
            CapCase { ray, cap }
        },
    );
}

#[test]
fn row38_capsule_origin_in_cap_b() {
    run_cap_row(
        "38: c2RaytoCapsule origin inside cap circle b",
        0x38_0001,
        &[CapBranch::InCapB],
        N * 7 / 10,
        |rng| {
            let cap = rand_capsule(rng);
            let len = cap_len(&cap);
            let ray = local_ray(
                &cap,
                rng.range(-0.6, 0.6) * cap.r,
                len + rng.range(0.02, 0.6) * cap.r,
                rng.range(-1.0, 1.0),
                rng.range(-1.0, 1.0),
                rng.range(0.0, 50.0),
            );
            CapCase { ray, cap }
        },
    );
}

/* --------------------------------------------------------------- rows 39/40 */

#[test]
fn row39_capsule_flat_side_positive_c() {
    run_cap_row(
        "39: c2RaytoCapsule flat side, c > 0 (out->n = M.x)",
        0x39_0001,
        &[CapBranch::SidePos],
        N / 2,
        |rng| {
            let cap = rand_capsule(rng);
            let len = cap_len(&cap);
            let gap = rng.range(0.1, 2.0) * cap.r;
            let lx = cap.r + gap;
            let ly = rng.range(0.25, 0.75) * len;
            let dy = rng.range(-0.15, 0.15);
            let ray = local_ray(&cap, lx, ly, -1.0, dy, 2.0 * lx + rng.range(0.0, 5.0));
            CapCase { ray, cap }
        },
    );
}

#[test]
fn row40_capsule_flat_side_negative_c() {
    run_cap_row(
        "40: c2RaytoCapsule flat side, c <= 0 (out->n = c2Skew(M.y))",
        0x40_0001,
        &[CapBranch::SideNeg],
        N / 2,
        |rng| {
            let cap = rand_capsule(rng);
            let len = cap_len(&cap);
            let gap = rng.range(0.1, 2.0) * cap.r;
            let lx = -(cap.r + gap);
            let ly = rng.range(0.25, 0.75) * len;
            let dy = rng.range(-0.15, 0.15);
            let ray = local_ray(&cap, lx, ly, 1.0, dy, 2.0 * -lx + rng.range(0.0, 5.0));
            CapCase { ray, cap }
        },
    );
}

/* --------------------------------------------------------------- rows 41/42 */

#[test]
fn row41_capsule_cross_delegates_to_cap_a() {
    run_cap_row(
        "41: c2RaytoCapsule slab crossing, y <= 0 => c2RaytoCircle(Ca)",
        0x41_0001,
        &[CapBranch::CrossCa],
        N / 4,
        |rng| {
            let cap = rand_capsule(rng);
            let len = cap_len(&cap);
            let gap = rng.range(0.1, 1.5) * cap.r;
            let lx = if rng.chance(2) { cap.r + gap } else { -(cap.r + gap) };
            let ly = rng.range(0.02, 0.35) * len;
            let dx = if lx > 0.0 { -1.0 } else { 1.0 };
            let dy = -rng.range(0.5, 6.0);
            let ray = local_ray(&cap, lx, ly, dx, dy, rng.range(1.0, 4.0) * (cap.r + gap));
            CapCase { ray, cap }
        },
    );
}

#[test]
fn row42_capsule_cross_delegates_to_cap_b() {
    run_cap_row(
        "42: c2RaytoCapsule slab crossing, y >= yBb.y => c2RaytoCircle(Cb)",
        0x42_0001,
        &[CapBranch::CrossCb],
        N / 4,
        |rng| {
            let cap = rand_capsule(rng);
            let len = cap_len(&cap);
            let gap = rng.range(0.1, 1.5) * cap.r;
            let lx = if rng.chance(2) { cap.r + gap } else { -(cap.r + gap) };
            let ly = rng.range(0.65, 0.98) * len;
            let dx = if lx > 0.0 { -1.0 } else { 1.0 };
            let dy = rng.range(0.5, 6.0);
            let ray = local_ray(&cap, lx, ly, dx, dy, rng.range(1.0, 4.0) * (cap.r + gap));
            CapCase { ray, cap }
        },
    );
}

/* ------------------------------------------------------------------ row 43 - */

#[test]
fn row43_capsule_near_axis_delegation() {
    run_cap_row(
        "43: c2RaytoCapsule |yAp.x| < r => c2RaytoCircle(Ca/Cb)",
        0x43_0001,
        &[CapBranch::NearAxisCa, CapBranch::NearAxisCb],
        N / 4,
        |rng| {
            let cap = rand_capsule(rng);
            let len = cap_len(&cap);
            let lx = rng.range(-0.85, 0.85) * cap.r;
            let below = rng.chance(2);
            let ly = if below {
                -(cap.r + rng.range(0.5, 20.0))
            } else {
                len + cap.r + rng.range(0.5, 20.0)
            };
            // aim roughly back at the capsule so the delegated circle cast can hit
            let dy = if below { 1.0 } else { -1.0 };
            let ray = local_ray(
                &cap,
                lx,
                ly,
                rng.range(-0.3, 0.3),
                dy,
                rng.range(0.0, 60.0),
            );
            CapCase { ray, cap }
        },
    );
}

/* ------------------------------------------------------------------ row 44 - */

#[test]
fn row44_capsule_outside_slab_returns_zero_after_writing_out() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x44_0001);
    let mut d = Diff::new("44: c2RaytoCapsule outside slab (return 0 after writing *out)");
    let mut checked_write = 0;
    for _ in 0..N {
        let cap = rand_capsule(&mut rng);
        let len = cap_len(&cap);
        let gap = rng.range(0.05, 4.0) * cap.r;
        let lx = if rng.chance(2) { cap.r + gap } else { -(cap.r + gap) };
        let ly = rng.range(-1.0, 2.0) * len;
        // direction parallel to the capsule axis: yAe.x == yAp.x, same sign
        let dy = if rng.chance(2) { 1.0 } else { -1.0 };
        let ray = local_ray(&cap, lx, ly, 0.0, dy, rng.range(0.0, 50.0));
        let br = classify_capsule(c, ray, cap);
        d.tag(br.name());
        let (rc_c, out_c) = call_raytocapsule(c, ray, cap);
        let (rc_r, out_r) = call_raytocapsule(r, ray, cap);
        if br == CapBranch::Outside {
            // the C writes out->n / out->t *before* the early-outs, so a miss
            // must still have replaced the sentinel
            assert_eq!(rc_c, 0, "Outside branch must return 0");
            assert_ne!(
                out_c.t.to_bits(),
                SENT_T,
                "C must have overwritten out->t before returning 0"
            );
            assert_eq!(out_c.t.to_bits(), 0, "out->t must be +0.0");
            checked_write += 1;
        }
        d.check_call(
            || format!("{} {} [{}]", rayshow(&ray), capshow(&cap), br.name()),
            (rc_c, out_c),
            (rc_r, out_r),
        );
    }
    d.require_tag("outside", N / 2);
    assert!(checked_write > 100, "too few Outside cases: {checked_write}");
    d.finish();
}

/* ------------------------------------------------------------------ row 45 - */

#[test]
fn row45_capsule_axis_aligned_and_reversed() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x45_0001);
    let mut d = Diff::new("45: c2RaytoCapsule axis-aligned / b below a");
    for i in 0..N {
        let a = v(rng.range(-30.0, 30.0), rng.range(-30.0, 30.0));
        let len = rng.range(0.5, 30.0);
        let cap = C2Capsule {
            a,
            b: match i % 4 {
                0 => v(a.x + len, a.y),
                1 => v(a.x - len, a.y),
                2 => v(a.x, a.y + len),
                _ => v(a.x, a.y - len),
            },
            r: rng.range(0.05, 8.0),
        };
        let center = v((cap.a.x + cap.b.x) * 0.5, (cap.a.y + cap.b.y) * 0.5);
        let u = rng.dir();
        let ray = C2Ray {
            p: vadd(center, vscale(u, -rng.range(0.0, 60.0))),
            d: u,
            t: rng.range(0.0, 120.0),
        };
        let br = classify_capsule(c, ray, cap);
        d.tag(br.name());
        d.check_call(
            || format!("{} {} [{}]", rayshow(&ray), capshow(&cap), br.name()),
            call_raytocapsule(c, ray, cap),
            call_raytocapsule(r, ray, cap),
        );
    }
    d.finish();
}

/* ------------------------------------------------------------------ row 46 - */

#[test]
fn row46_capsule_degenerate_and_radius_shapes() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x46_0001);
    let mut d = Diff::new("46: c2RaytoCapsule a == b / r = 0, negative, huge, denormal");
    for i in 0..N {
        let base = rand_capsule(&mut rng);
        let cap = match i % 6 {
            0 => C2Capsule { b: base.a, ..base },
            1 => C2Capsule { r: 0.0, ..base },
            2 => C2Capsule { r: -base.r, ..base },
            3 => C2Capsule {
                r: rng.range(1e30, 3e38),
                ..base
            },
            4 => C2Capsule {
                r: f32::from_bits(rng.below(0x0080_0000)),
                ..base
            },
            _ => C2Capsule {
                r: rng.special(),
                ..base
            },
        };
        let center = v((cap.a.x + cap.b.x) * 0.5, (cap.a.y + cap.b.y) * 0.5);
        let u = rng.dir();
        let ray = C2Ray {
            p: vadd(center, vscale(u, -rng.range(0.0, 60.0))),
            d: u,
            t: rng.range(0.0, 120.0),
        };
        let br = classify_capsule(c, ray, cap);
        d.tag(br.name());
        d.check_call(
            || format!("{} {} [{}]", rayshow(&ray), capshow(&cap), br.name()),
            call_raytocapsule(c, ray, cap),
            call_raytocapsule(r, ray, cap),
        );
    }
    d.finish();
}

/* ------------------------------------------------------------------ row 47 - */

#[test]
fn row47_capsule_zero_and_infinite_sweeps() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x47_0001);
    let mut d = Diff::new("47: c2RaytoCapsule A.t = 0/inf, zero direction");
    for i in 0..N {
        let cap = rand_capsule(&mut rng);
        let len = cap_len(&cap);
        let gap = rng.range(0.1, 2.0) * cap.r;
        let base = local_ray(
            &cap,
            cap.r + gap,
            rng.range(0.1, 0.9) * len,
            -1.0,
            rng.range(-0.2, 0.2),
            2.0 * (cap.r + gap),
        );
        let ray = match i % 6 {
            0 => C2Ray { t: 0.0, ..base },
            1 => C2Ray { t: -0.0, ..base },
            2 => C2Ray {
                t: f32::INFINITY,
                ..base
            },
            3 => C2Ray {
                d: v(0.0, 0.0),
                ..base
            },
            4 => C2Ray {
                t: -rng.range(0.0, 20.0),
                ..base
            },
            _ => C2Ray {
                d: vscale(base.d, 1e30),
                t: 1e30,
                ..base
            },
        };
        let br = classify_capsule(c, ray, cap);
        d.tag(br.name());
        d.check_call(
            || format!("{} {} [{}]", rayshow(&ray), capshow(&cap), br.name()),
            call_raytocapsule(c, ray, cap),
            call_raytocapsule(r, ray, cap),
        );
    }
    d.finish();
}

/* ------------------------- extra: full-noise fuzz ------------------------- */

#[test]
fn capsule_full_noise_fuzz() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x4F_0001);
    let mut d = Diff::new("47b: c2RaytoCapsule arbitrary bit patterns");
    for i in 0..N * 4 {
        let (ray, cap) = if i % 2 == 0 {
            (
                C2Ray {
                    p: rng.v_mixed(),
                    d: rng.v_mixed(),
                    t: rng.mixed(),
                },
                C2Capsule {
                    a: rng.v_mixed(),
                    b: rng.v_mixed(),
                    r: rng.mixed(),
                },
            )
        } else {
            (
                C2Ray {
                    p: rng.v_special(),
                    d: rng.v_special(),
                    t: rng.special(),
                },
                C2Capsule {
                    a: rng.v_special(),
                    b: rng.v_special(),
                    r: rng.special(),
                },
            )
        };
        d.check_call(
            || format!("{} {}", rayshow(&ray), capshow(&cap)),
            call_raytocapsule(c, ray, cap),
            call_raytocapsule(r, ray, cap),
        );
    }
    d.finish();
}
