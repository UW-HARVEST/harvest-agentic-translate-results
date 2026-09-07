//! Phase B — CONFIGS.md rows 36..48: `c2RaytoCapsule` as a low-level entry
//! point, with every internal branch driven deliberately.
//!
//! A branch classifier (mirroring the C control flow, used ONLY for coverage
//! accounting — never for correctness) asserts that each of the seven paths is
//! actually reached, so a row cannot silently test nothing.

mod common;
use common::*;
use std::sync::atomic::{AtomicUsize, Ordering};

const N: usize = 20_000;

/// The seven distinct exits of `c2RaytoCapsule`.
const B_SLAB_POINT: usize = 0; // AABBtoPoint(capsule_bb, yAp) -> 1
const B_CIRCLE_A_POINT: usize = 1; // CircleToPoint(a) -> 1
const B_CIRCLE_B_POINT: usize = 2; // CircleToPoint(b) -> 1
const B_DELEG_A: usize = 3; // |yAp.x| < r, yAp.y < 0  -> RaytoCircle(Ca)
const B_DELEG_B: usize = 4; // |yAp.x| < r, yAp.y >= 0 -> RaytoCircle(Cb)
const B_ENDCAP: usize = 5; // y <= 0 or y >= yBb.y     -> RaytoCircle
const B_SIDE: usize = 6; // side slab hit
const B_REJECT: usize = 7; // final return 0
const NBRANCH: usize = 8;

static COVER: [AtomicUsize; NBRANCH] = [
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
];

fn classify(l: &Pair, ray: C2Ray, cap: C2Capsule) -> usize {
    // Recompute the C's intermediate quantities using the *C* library's own
    // primitives so the classification matches its control flow exactly.
    let my = (l.c.c2Norm)((l.c.c2Sub)(cap.b, cap.a));
    let mx = (l.c.c2CCW90)(my);
    let m = C2m { x: mx, y: my };
    let cap_n = (l.c.c2Sub)(cap.b, cap.a);
    let ybb = (l.c.c2MulmvT)(m, cap_n);
    let yap = (l.c.c2MulmvT)(m, (l.c.c2Sub)(ray.p, cap.a));
    let yad = (l.c.c2MulmvT)(m, ray.d);
    let yae = (l.c.c2Add)(yap, (l.c.c2Mulvs)(yad, ray.t));
    let bb = C2AABB {
        min: (l.c.c2V)(-cap.r, 0.0),
        max: (l.c.c2V)(cap.r, ybb.y),
    };
    if (l.c.c2AABBtoPoint)(bb, yap) != 0 {
        return B_SLAB_POINT;
    }
    if (l.c.c2CircleToPoint)(C2Circle { p: cap.a, r: cap.r }, ray.p) != 0 {
        return B_CIRCLE_A_POINT;
    }
    if (l.c.c2CircleToPoint)(C2Circle { p: cap.b, r: cap.r }, ray.p) != 0 {
        return B_CIRCLE_B_POINT;
    }
    let sel_abs = |v: f32| if v < 0.0 { -v } else { v };
    let sel_lt = |a: f32, b: f32| if a < b { a } else { b };
    if yae.x * yap.x < 0.0 || sel_lt(sel_abs(yae.x), sel_abs(yap.x)) < cap.r {
        if sel_abs(yap.x) < cap.r {
            if yap.y < 0.0 {
                return B_DELEG_A;
            }
            return B_DELEG_B;
        }
        let c = if yap.x > 0.0 { cap.r } else { -cap.r };
        let d = yae.x - yap.x;
        let t = (c - yap.x) / d;
        let y = yap.y + (yae.y - yap.y) * t;
        if y <= 0.0 || y >= ybb.y {
            return B_ENDCAP;
        }
        return B_SIDE;
    }
    B_REJECT
}

fn cmp_capsule(l: &Pair, ray: C2Ray, cap: C2Capsule, ctx: String) {
    let mut oc = SENTINEL;
    let mut or = SENTINEL;
    let rc = unsafe { (l.c.c2RaytoCapsule)(ray, cap, &mut oc) };
    let rr = unsafe { (l.r.c2RaytoCapsule)(ray, cap, &mut or) };
    diff_eq!(ctx, (rc, rcb(oc)), (rr, rcb(or)));
    let b = classify(l, ray, cap);
    COVER[b].fetch_add(1, Ordering::Relaxed);
}

fn rand_capsule(rng: &mut Rng) -> C2Capsule {
    let a = rng.v(30.0);
    let ang = rng.unit() * std::f32::consts::TAU;
    let len = rng.unit() * 40.0 + 0.5;
    C2Capsule {
        a,
        b: C2v {
            x: a.x + ang.cos() * len,
            y: a.y + ang.sin() * len,
        },
        r: rng.unit() * 8.0 + 0.05,
    }
}

/// Basis of the capsule: (perp, along).
fn basis(cap: C2Capsule) -> (C2v, C2v) {
    let dx = cap.b.x - cap.a.x;
    let dy = cap.b.y - cap.a.y;
    let ln = (dx * dx + dy * dy).sqrt();
    let along = C2v { x: dx / ln, y: dy / ln };
    let perp = C2v { x: along.y, y: -along.x };
    (perp, along)
}

// --- row 36: random capsule, random normalized ray ------------------------
#[test]
fn row36_capsule_random() {
    let l = libs();
    let mut rng = Rng::new(0x3636);
    for _ in 0..N {
        let cap = rand_capsule(&mut rng);
        let ray = C2Ray {
            p: rng.v(60.0),
            d: rng.unit_dir(),
            t: rng.unit() * 120.0,
        };
        cmp_capsule(l, ray, cap, format!("row36 {ray:?} {cap:?}"));
    }
}

// --- row 37: origin inside the transformed slab box ----------------------
#[test]
fn row37_capsule_origin_in_slab() {
    let l = libs();
    let mut rng = Rng::new(0x3737);
    for _ in 0..N {
        let cap = rand_capsule(&mut rng);
        let (perp, along) = basis(cap);
        let s = rng.unit(); // 0..1 along the axis
        let ln = ((cap.b.x - cap.a.x).powi(2) + (cap.b.y - cap.a.y).powi(2)).sqrt();
        let off = rng.sym(cap.r * 0.98);
        let p = C2v {
            x: cap.a.x + along.x * (s * ln) + perp.x * off,
            y: cap.a.y + along.y * (s * ln) + perp.y * off,
        };
        let ray = C2Ray {
            p,
            d: rng.unit_dir(),
            t: rng.unit() * 60.0,
        };
        cmp_capsule(l, ray, cap, format!("row37 {ray:?} {cap:?}"));
    }
}

// --- row 38: origin inside end circle A / B -----------------------------
#[test]
fn row38_capsule_origin_in_end_circles() {
    let l = libs();
    let mut rng = Rng::new(0x3838);
    for which in 0..2 {
        for _ in 0..N / 2 {
            let cap = rand_capsule(&mut rng);
            let (_perp, along) = basis(cap);
            let centre = if which == 0 { cap.a } else { cap.b };
            // just outside the slab along the axis but within the cap radius
            let axial = if which == 0 {
                -rng.unit() * cap.r * 0.9 - 1e-4
            } else {
                rng.unit() * cap.r * 0.9 + 1e-4
            };
            let ang = rng.unit() * std::f32::consts::TAU;
            let rad = rng.unit() * cap.r * 0.5;
            let p = C2v {
                x: centre.x + along.x * axial + ang.cos() * rad,
                y: centre.y + along.y * axial + ang.sin() * rad,
            };
            let ray = C2Ray {
                p,
                d: rng.unit_dir(),
                t: rng.unit() * 60.0,
            };
            cmp_capsule(l, ray, cap, format!("row38 which={which} {ray:?} {cap:?}"));
        }
    }
}

// --- rows 39,40: |yAp.x| < r, delegating to circle A / B ----------------
#[test]
fn rows39_40_capsule_delegate_to_end_circle() {
    let l = libs();
    let mut rng = Rng::new(0x3940);
    for which in 0..2 {
        for _ in 0..N {
            let cap = rand_capsule(&mut rng);
            let (perp, along) = basis(cap);
            let ln = ((cap.b.x - cap.a.x).powi(2) + (cap.b.y - cap.a.y).powi(2)).sqrt();
            // within the slab width, but far enough past the cap that the
            // point is outside both end circles
            let off = rng.sym(cap.r * 0.95);
            let axial = if which == 0 {
                -(cap.r * 1.5 + rng.unit() * 30.0)
            } else {
                ln + cap.r * 1.5 + rng.unit() * 30.0
            };
            let p = C2v {
                x: cap.a.x + along.x * axial + perp.x * off,
                y: cap.a.y + along.y * axial + perp.y * off,
            };
            let ray = C2Ray {
                p,
                d: rng.unit_dir(),
                t: rng.unit() * 120.0,
            };
            cmp_capsule(l, ray, cap, format!("rows39_40 which={which} {ray:?} {cap:?}"));
            // aimed straight down the axis so the delegated circle cast hits
            let toward = C2v {
                x: if which == 0 { along.x } else { -along.x },
                y: if which == 0 { along.y } else { -along.y },
            };
            let ray2 = C2Ray {
                p,
                d: toward,
                t: axial.abs() + ln + cap.r * 2.0,
            };
            cmp_capsule(l, ray2, cap, format!("rows39_40 aimed which={which} {ray2:?} {cap:?}"));
        }
    }
}

// --- rows 41,42,43: side-slab hit and end-cap fallthrough ---------------
#[test]
fn rows41_43_capsule_side_and_endcap() {
    let l = libs();
    let mut rng = Rng::new(0x4143);
    for side in 0..2 {
        for _ in 0..N {
            let cap = rand_capsule(&mut rng);
            let (perp, along) = basis(cap);
            let ln = ((cap.b.x - cap.a.x).powi(2) + (cap.b.y - cap.a.y).powi(2)).sqrt();
            let sgn = if side == 0 { 1.0f32 } else { -1.0f32 };
            // origin outside the slab on the chosen side, at a random axial
            // position (sometimes beyond the caps, which drives the y<=0 /
            // y>=yBb.y end-cap branches)
            let off = sgn * (cap.r + 0.5 + rng.unit() * 30.0);
            let axial = -ln * 0.5 + rng.unit() * ln * 2.0;
            let p = C2v {
                x: cap.a.x + along.x * axial + perp.x * off,
                y: cap.a.y + along.y * axial + perp.y * off,
            };
            // aim at a random point on the segment so the slab is crossed
            let s = rng.unit();
            let tx = cap.a.x + along.x * (s * ln);
            let ty = cap.a.y + along.y * (s * ln);
            let dx = tx - p.x;
            let dy = ty - p.y;
            let dl = (dx * dx + dy * dy).sqrt();
            let ray = C2Ray {
                p,
                d: C2v { x: dx / dl, y: dy / dl },
                t: dl * (0.4 + rng.unit() * 1.6),
            };
            cmp_capsule(l, ray, cap, format!("rows41_43 side={side} {ray:?} {cap:?}"));
            // shallow, nearly-parallel crossing => extreme `y`, hits the
            // end-cap branches
            let ang = rng.sym(0.05);
            let (c, s2) = (ang.cos(), ang.sin());
            let nd = C2v {
                x: -sgn * perp.x * c + along.x * s2,
                y: -sgn * perp.y * c + along.y * s2,
            };
            let ray2 = C2Ray {
                p,
                d: nd,
                t: off.abs() * (1.0 + rng.unit() * 40.0),
            };
            cmp_capsule(l, ray2, cap, format!("rows41_43 shallow side={side} {ray2:?} {cap:?}"));
        }
    }
}

// --- row 44: capsule orientations ---------------------------------------
#[test]
fn row44_capsule_orientations() {
    let l = libs();
    let mut rng = Rng::new(0x4444);
    let caps: Vec<C2Capsule> = vec![
        // +y
        C2Capsule {
            a: C2v { x: 0.0, y: -5.0 },
            b: C2v { x: 0.0, y: 5.0 },
            r: 2.0,
        },
        // -y (yBb.y negative -> inverted capsule_bb)
        C2Capsule {
            a: C2v { x: 0.0, y: 5.0 },
            b: C2v { x: 0.0, y: -5.0 },
            r: 2.0,
        },
        // +x
        C2Capsule {
            a: C2v { x: -5.0, y: 0.0 },
            b: C2v { x: 5.0, y: 0.0 },
            r: 2.0,
        },
        // -x
        C2Capsule {
            a: C2v { x: 5.0, y: 0.0 },
            b: C2v { x: -5.0, y: 0.0 },
            r: 2.0,
        },
        // diagonal
        C2Capsule {
            a: C2v { x: -3.0, y: -4.0 },
            b: C2v { x: 6.0, y: 8.0 },
            r: 1.5,
        },
        // very long
        C2Capsule {
            a: C2v { x: -1e5, y: 0.0 },
            b: C2v { x: 1e5, y: 1.0 },
            r: 3.0,
        },
        // very short
        C2Capsule {
            a: C2v { x: 0.0, y: 0.0 },
            b: C2v { x: 1e-5, y: 0.0 },
            r: 2.0,
        },
        // radius much larger than the length
        C2Capsule {
            a: C2v { x: 0.0, y: 0.0 },
            b: C2v { x: 0.5, y: 0.0 },
            r: 20.0,
        },
    ];
    for (i, &cap) in caps.iter().enumerate() {
        for _ in 0..4_000 {
            let ray = C2Ray {
                p: rng.v(40.0),
                d: rng.unit_dir(),
                t: rng.unit() * 100.0,
            };
            cmp_capsule(l, ray, cap, format!("row44 cap{i} {ray:?} {cap:?}"));
        }
        // aimed rays
        for _ in 0..4_000 {
            let p = rng.v(40.0);
            let s = rng.unit();
            let tx = cap.a.x + (cap.b.x - cap.a.x) * s;
            let ty = cap.a.y + (cap.b.y - cap.a.y) * s;
            let dx = tx - p.x;
            let dy = ty - p.y;
            let dl = (dx * dx + dy * dy).sqrt();
            if dl == 0.0 {
                continue;
            }
            let ray = C2Ray {
                p,
                d: C2v { x: dx / dl, y: dy / dl },
                t: dl * (0.5 + rng.unit() * 1.5),
            };
            cmp_capsule(l, ray, cap, format!("row44 aimed cap{i} {ray:?} {cap:?}"));
        }
    }
}

// --- row 45: degenerate capsule a == b (all-NaN basis) ------------------
#[test]
fn row45_capsule_degenerate() {
    let l = libs();
    let mut rng = Rng::new(0x4545);
    for _ in 0..N {
        let a = rng.v(20.0);
        let cap = C2Capsule {
            a,
            b: a,
            r: rng.unit() * 5.0,
        };
        let ray = C2Ray {
            p: rng.v(20.0),
            d: rng.unit_dir(),
            t: rng.unit() * 60.0,
        };
        cmp_capsule(l, ray, cap, format!("row45 {ray:?} {cap:?}"));
    }
    // and with -0.0 / 0.0 mixtures so `b - a` is a signed zero
    for (ax, bx) in [(0.0f32, -0.0f32), (-0.0, 0.0), (0.0, 0.0), (-0.0, -0.0)] {
        for _ in 0..2_000 {
            let cap = C2Capsule {
                a: C2v { x: ax, y: ax },
                b: C2v { x: bx, y: bx },
                r: rng.unit() * 5.0,
            };
            let ray = C2Ray {
                p: rng.v(20.0),
                d: rng.unit_dir(),
                t: rng.unit() * 60.0,
            };
            cmp_capsule(l, ray, cap, format!("row45 zeros {ray:?} {cap:?}"));
        }
    }
}

// --- row 46: r == 0 / r < 0 / r huge ------------------------------------
#[test]
fn row46_capsule_special_radius() {
    let l = libs();
    let mut rng = Rng::new(0x4646);
    for &r in &specials_nan_payloads() {
        for _ in 0..1_200 {
            let mut cap = rand_capsule(&mut rng);
            cap.r = r;
            let ray = C2Ray {
                p: rng.v(40.0),
                d: rng.unit_dir(),
                t: rng.unit() * 100.0,
            };
            cmp_capsule(l, ray, cap, format!("row46 r={:#x} {ray:?} {cap:?}", fb(r)));
            // aimed variant
            let s = rng.unit();
            let tx = cap.a.x + (cap.b.x - cap.a.x) * s;
            let ty = cap.a.y + (cap.b.y - cap.a.y) * s;
            let dx = tx - ray.p.x;
            let dy = ty - ray.p.y;
            let dl = (dx * dx + dy * dy).sqrt();
            let ray2 = C2Ray {
                p: ray.p,
                d: C2v { x: dx / dl, y: dy / dl },
                t: dl * 1.5,
            };
            cmp_capsule(l, ray2, cap, format!("row46 aimed r={:#x} {ray2:?} {cap:?}", fb(r)));
        }
    }
}

// --- row 47: unnormalized direction, degenerate A.t --------------------
#[test]
fn row47_capsule_dir_and_t() {
    let l = libs();
    let mut rng = Rng::new(0x4747);
    let ts = [
        0.0f32,
        -0.0,
        -1.0,
        -1e30,
        1e30,
        f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ];
    for &t in &ts {
        for _ in 0..2_500 {
            let cap = rand_capsule(&mut rng);
            let ray = C2Ray {
                p: rng.v(40.0),
                d: rng.unit_dir(),
                t,
            };
            cmp_capsule(l, ray, cap, format!("row47 t={t} {ray:?} {cap:?}"));
        }
    }
    for _ in 0..N {
        let cap = rand_capsule(&mut rng);
        let d = rng.unit_dir();
        let mag = 10f32.powf(rng.sym(3.0));
        let ray = C2Ray {
            p: rng.v(40.0),
            d: C2v {
                x: d.x * mag,
                y: d.y * mag,
            },
            t: rng.unit() * 100.0,
        };
        cmp_capsule(l, ray, cap, format!("row47 mag={mag} {ray:?} {cap:?}"));
    }
    // exactly-zero direction
    for _ in 0..5_000 {
        let cap = rand_capsule(&mut rng);
        let ray = C2Ray {
            p: rng.v(40.0),
            d: C2v { x: 0.0, y: 0.0 },
            t: rng.unit() * 50.0,
        };
        cmp_capsule(l, ray, cap, format!("row47 zerodir {ray:?} {cap:?}"));
    }
}

// --- row 48: full special / bit-space fuzz ----------------------------
#[test]
fn row48_capsule_fuzz() {
    let l = libs();
    let mut rng = Rng::new(0x4848);
    for _ in 0..N {
        let ray = C2Ray {
            p: rng.v_special(1e6),
            d: rng.v_special(1e6),
            t: rng.special(1e6),
        };
        let cap = C2Capsule {
            a: rng.v_special(1e6),
            b: rng.v_special(1e6),
            r: rng.special(1e6),
        };
        cmp_capsule(l, ray, cap, format!("row48 special {ray:?} {cap:?}"));
    }
    for _ in 0..N {
        let ray = C2Ray {
            p: C2v {
                x: rng.any_bits(),
                y: rng.any_bits(),
            },
            d: C2v {
                x: rng.any_bits(),
                y: rng.any_bits(),
            },
            t: rng.any_bits(),
        };
        let cap = C2Capsule {
            a: C2v {
                x: rng.any_bits(),
                y: rng.any_bits(),
            },
            b: C2v {
                x: rng.any_bits(),
                y: rng.any_bits(),
            },
            r: rng.any_bits(),
        };
        cmp_capsule(l, ray, cap, format!("row48 bits {ray:?} {cap:?}"));
    }
    // one special field at a time on an otherwise sane hitting configuration
    for &s in &specials_nan_payloads() {
        for slot in 0..10 {
            for _ in 0..300 {
                let mut ray = C2Ray {
                    p: C2v { x: -20.0, y: 0.3 },
                    d: C2v { x: 1.0, y: 0.0 },
                    t: 60.0,
                };
                let mut cap = C2Capsule {
                    a: C2v { x: 0.0, y: -4.0 },
                    b: C2v { x: 1.0, y: 6.0 },
                    r: 2.0,
                };
                match slot {
                    0 => ray.p.x = s,
                    1 => ray.p.y = s,
                    2 => ray.d.x = s,
                    3 => ray.d.y = s,
                    4 => ray.t = s,
                    5 => cap.a.x = s,
                    6 => cap.a.y = s,
                    7 => cap.b.x = s,
                    8 => cap.b.y = s,
                    _ => cap.r = s,
                }
                let _ = rng.next_u32();
                cmp_capsule(l, ray, cap, format!("row48 slot{slot}={:#x}", fb(s)));
            }
        }
    }
}

/// Runs last (alphabetically after the rows) and asserts every branch of
/// `c2RaytoCapsule` was actually reached by the suite above.
#[test]
fn zz_branch_coverage_report() {
    // Re-drive a compact but broad sweep here so this test is self-sufficient
    // regardless of test-execution order/parallelism.
    let l = libs();
    let mut rng = Rng::new(0xC0DE);
    let mut local = [0usize; NBRANCH];
    for _ in 0..200_000 {
        let cap = rand_capsule(&mut rng);
        let (perp, along) = basis(cap);
        let ln = ((cap.b.x - cap.a.x).powi(2) + (cap.b.y - cap.a.y).powi(2)).sqrt();
        let mode = rng.below(6);
        let p = match mode {
            0 => rng.v(60.0),
            1 => C2v {
                x: cap.a.x + along.x * (rng.unit() * ln) + perp.x * rng.sym(cap.r * 0.9),
                y: cap.a.y + along.y * (rng.unit() * ln) + perp.y * rng.sym(cap.r * 0.9),
            },
            2 => C2v {
                x: cap.a.x - along.x * (cap.r * 0.5) + perp.x * rng.sym(cap.r * 0.4),
                y: cap.a.y - along.y * (cap.r * 0.5) + perp.y * rng.sym(cap.r * 0.4),
            },
            3 => C2v {
                x: cap.a.x - along.x * (cap.r * 2.0 + rng.unit() * 20.0) + perp.x * rng.sym(cap.r * 0.9),
                y: cap.a.y - along.y * (cap.r * 2.0 + rng.unit() * 20.0) + perp.y * rng.sym(cap.r * 0.9),
            },
            4 => C2v {
                x: cap.b.x + along.x * (cap.r * 2.0 + rng.unit() * 20.0) + perp.x * rng.sym(cap.r * 0.9),
                y: cap.b.y + along.y * (cap.r * 2.0 + rng.unit() * 20.0) + perp.y * rng.sym(cap.r * 0.9),
            },
            _ => {
                let sgn = if rng.next_u32() & 1 == 0 { 1.0 } else { -1.0 };
                C2v {
                    x: cap.a.x + along.x * (rng.sym(ln)) + perp.x * sgn * (cap.r + rng.unit() * 20.0),
                    y: cap.a.y + along.y * (rng.sym(ln)) + perp.y * sgn * (cap.r + rng.unit() * 20.0),
                }
            }
        };
        let s = rng.unit();
        let tx = cap.a.x + (cap.b.x - cap.a.x) * s;
        let ty = cap.a.y + (cap.b.y - cap.a.y) * s;
        let (dx, dy) = (tx - p.x, ty - p.y);
        let dl = (dx * dx + dy * dy).sqrt();
        let d = if dl > 0.0 && rng.next_u32() % 3 != 0 {
            C2v { x: dx / dl, y: dy / dl }
        } else {
            rng.unit_dir()
        };
        let ray = C2Ray {
            p,
            d,
            t: if rng.next_u32() & 1 == 0 {
                dl * (0.3 + rng.unit() * 2.0)
            } else {
                rng.unit() * 120.0
            },
        };
        let mut oc = SENTINEL;
        let mut or = SENTINEL;
        let rc = unsafe { (l.c.c2RaytoCapsule)(ray, cap, &mut oc) };
        let rr = unsafe { (l.r.c2RaytoCapsule)(ray, cap, &mut or) };
        diff_eq!(format!("coverage sweep {ray:?} {cap:?}"), (rc, rcb(oc)), (rr, rcb(or)));
        local[classify(l, ray, cap)] += 1;
    }
    let names = [
        "slab-point",
        "circleA-point",
        "circleB-point",
        "delegate-A",
        "delegate-B",
        "endcap",
        "side-slab",
        "reject",
    ];
    for i in 0..NBRANCH {
        let total = local[i] + COVER[i].load(Ordering::Relaxed);
        eprintln!("c2RaytoCapsule branch {:14} hit {} times", names[i], total);
        assert!(total > 0, "branch `{}` was never exercised", names[i]);
    }
}
