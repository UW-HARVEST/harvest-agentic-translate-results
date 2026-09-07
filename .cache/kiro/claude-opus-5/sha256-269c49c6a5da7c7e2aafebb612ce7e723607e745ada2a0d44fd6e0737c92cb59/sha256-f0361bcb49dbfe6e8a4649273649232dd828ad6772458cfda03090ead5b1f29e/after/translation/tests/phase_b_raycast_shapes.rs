//! Phase B — CONFIGS.md rows 20..35: `c2RaytoCircle` and `c2RaytoAABB`,
//! called directly as low-level entry points (not via `gen_ray`).

mod common;
use common::*;

const N: usize = 20_000;

fn cmp_circle(l: &Pair, ray: C2Ray, cir: C2Circle, ctx: String) {
    let mut oc = SENTINEL;
    let mut or = SENTINEL;
    let rc = unsafe { (l.c.c2RaytoCircle)(ray, cir, &mut oc) };
    let rr = unsafe { (l.r.c2RaytoCircle)(ray, cir, &mut or) };
    diff_eq!(ctx, (rc, rcb(oc)), (rr, rcb(or)));
}

fn cmp_aabb(l: &Pair, ray: C2Ray, b: C2AABB, ctx: String) {
    let mut oc = SENTINEL;
    let mut or = SENTINEL;
    let rc = unsafe { (l.c.c2RaytoAABB)(ray, b, &mut oc) };
    let rr = unsafe { (l.r.c2RaytoAABB)(ray, b, &mut or) };
    diff_eq!(ctx, (rc, rcb(oc)), (rr, rcb(or)));
}

// ===========================================================================
// c2RaytoCircle
// ===========================================================================

// --- row 20: normalized ray, random circle --------------------------------
#[test]
fn row20_circle_normalized_random() {
    let l = libs();
    let mut rng = Rng::new(0x2020);
    for _ in 0..N {
        let ray = C2Ray {
            p: rng.v(50.0),
            d: rng.unit_dir(),
            t: rng.unit() * 100.0,
        };
        let cir = C2Circle {
            p: rng.v(50.0),
            r: rng.unit() * 20.0,
        };
        cmp_circle(l, ray, cir, format!("row20 {ray:?} {cir:?}"));
    }
}

// --- row 21: ray direction NOT normalized ---------------------------------
#[test]
fn row21_circle_unnormalized_dir() {
    let l = libs();
    let mut rng = Rng::new(0x2121);
    for _ in 0..N {
        let d = rng.unit_dir();
        let mag = 10f32.powf(rng.sym(3.0));
        let ray = C2Ray {
            p: rng.v(50.0),
            d: C2v {
                x: d.x * mag,
                y: d.y * mag,
            },
            t: rng.unit() * 100.0,
        };
        let cir = C2Circle {
            p: rng.v(50.0),
            r: rng.unit() * 20.0,
        };
        cmp_circle(l, ray, cir, format!("row21 mag={mag} {ray:?} {cir:?}"));
    }
}

// --- row 22: origin strictly inside the circle ----------------------------
#[test]
fn row22_circle_origin_inside() {
    let l = libs();
    let mut rng = Rng::new(0x2222);
    for _ in 0..N {
        let cir = C2Circle {
            p: rng.v(50.0),
            r: rng.unit() * 20.0 + 0.5,
        };
        let d = rng.unit_dir();
        let f = rng.unit() * 0.999;
        let ray = C2Ray {
            p: C2v {
                x: cir.p.x + d.x * cir.r * f,
                y: cir.p.y + d.y * cir.r * f,
            },
            d: rng.unit_dir(),
            t: rng.unit() * 100.0,
        };
        cmp_circle(l, ray, cir, format!("row22 {ray:?} {cir:?}"));
    }
}

// --- row 23: tangent / grazing (disc within a few ULP of 0) ---------------
#[test]
fn row23_circle_tangent() {
    let l = libs();
    let mut rng = Rng::new(0x2323);
    for _ in 0..N {
        let cir = C2Circle {
            p: rng.v(20.0),
            r: rng.unit() * 10.0 + 0.1,
        };
        let d = rng.unit_dir();
        let perp = C2v { x: -d.y, y: d.x };
        // origin at distance `r` perpendicular from the centre, pushed back
        // along -d so the tangent point is at t = dist.
        let back = rng.unit() * 50.0 + 1.0;
        let mut ray = C2Ray {
            p: C2v {
                x: cir.p.x + perp.x * cir.r - d.x * back,
                y: cir.p.y + perp.y * cir.r - d.y * back,
            },
            d,
            t: back * 2.0,
        };
        cmp_circle(l, ray, cir, format!("row23 tangent {ray:?} {cir:?}"));
        // nudge the offset by a few ULP either way
        for k in [-3i32, -1, 1, 3] {
            let nr = f32::from_bits((cir.r.to_bits() as i32 + k) as u32);
            ray.p = C2v {
                x: cir.p.x + perp.x * nr - d.x * back,
                y: cir.p.y + perp.y * nr - d.y * back,
            };
            cmp_circle(l, ray, cir, format!("row23 ulp{k} {ray:?} {cir:?}"));
        }
    }
}

// --- row 24: hit exactly at t == A.t and at t == 0 ------------------------
#[test]
fn row24_circle_boundary_t() {
    let l = libs();
    let mut rng = Rng::new(0x2424);
    for _ in 0..N {
        let cir = C2Circle {
            p: rng.v(20.0),
            r: rng.unit() * 10.0 + 0.1,
        };
        let d = rng.unit_dir();
        // ray starts exactly on the circle boundary, aimed inward -> t == 0
        let p_on = C2v {
            x: cir.p.x - d.x * cir.r,
            y: cir.p.y - d.y * cir.r,
        };
        cmp_circle(
            l,
            C2Ray {
                p: p_on,
                d,
                t: rng.unit() * 10.0,
            },
            cir,
            format!("row24 t0 {cir:?}"),
        );
        // ray whose length ends exactly at the entry point
        let back = rng.unit() * 50.0 + 1.0;
        let p_far = C2v {
            x: p_on.x - d.x * back,
            y: p_on.y - d.y * back,
        };
        for tmul in [back, back * (1.0 - f32::EPSILON), back * (1.0 + f32::EPSILON)] {
            cmp_circle(
                l,
                C2Ray {
                    p: p_far,
                    d,
                    t: tmul,
                },
                cir,
                format!("row24 tend={tmul} {cir:?}"),
            );
        }
    }
}

// --- row 25: A.t == 0, < 0, inf ------------------------------------------
#[test]
fn row25_circle_degenerate_t() {
    let l = libs();
    let mut rng = Rng::new(0x2525);
    let ts = [
        0.0f32,
        -0.0,
        -1.0,
        -1e30,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::MAX,
        f32::MIN_POSITIVE,
    ];
    for &t in &ts {
        for _ in 0..3_000 {
            let ray = C2Ray {
                p: rng.v(20.0),
                d: rng.unit_dir(),
                t,
            };
            let cir = C2Circle {
                p: rng.v(20.0),
                r: rng.unit() * 10.0,
            };
            cmp_circle(l, ray, cir, format!("row25 t={t} {ray:?} {cir:?}"));
        }
    }
}

// --- row 26: r == 0 / < 0 / inf and special components -------------------
#[test]
fn row26_circle_special_radius_and_fuzz() {
    let l = libs();
    let mut rng = Rng::new(0x2626);
    for &r in &specials_nan_payloads() {
        for _ in 0..1_500 {
            let ray = C2Ray {
                p: rng.v(20.0),
                d: rng.unit_dir(),
                t: rng.unit() * 60.0,
            };
            let cir = C2Circle { p: rng.v(20.0), r };
            cmp_circle(l, ray, cir, format!("row26 r={:#x} {ray:?} {cir:?}", fb(r)));
        }
    }
    // full-bit-space fuzz over every field
    for _ in 0..N {
        let ray = C2Ray {
            p: rng.v_special(1e6),
            d: rng.v_special(1e6),
            t: rng.special(1e6),
        };
        let cir = C2Circle {
            p: rng.v_special(1e6),
            r: rng.special(1e6),
        };
        cmp_circle(l, ray, cir, format!("row26 fuzz {ray:?} {cir:?}"));
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
        let cir = C2Circle {
            p: C2v {
                x: rng.any_bits(),
                y: rng.any_bits(),
            },
            r: rng.any_bits(),
        };
        cmp_circle(l, ray, cir, format!("row26 bits {ray:?} {cir:?}"));
    }
}

// ===========================================================================
// c2RaytoAABB
// ===========================================================================

fn rand_box(rng: &mut Rng) -> C2AABB {
    let c = rng.v(40.0);
    let h = C2v {
        x: rng.unit() * 20.0 + 0.01,
        y: rng.unit() * 20.0 + 0.01,
    };
    C2AABB {
        min: C2v {
            x: c.x - h.x,
            y: c.y - h.y,
        },
        max: C2v {
            x: c.x + h.x,
            y: c.y + h.y,
        },
    }
}

// --- row 27: random normalized ray vs random box -------------------------
#[test]
fn row27_aabb_random() {
    let l = libs();
    let mut rng = Rng::new(0x2727);
    for _ in 0..N {
        let b = rand_box(&mut rng);
        let ray = C2Ray {
            p: rng.v(80.0),
            d: rng.unit_dir(),
            t: rng.unit() * 160.0,
        };
        cmp_aabb(l, ray, b, format!("row27 {ray:?} {b:?}"));
    }
    // aimed-at-the-box variant so the hit branches are hit often
    for _ in 0..N {
        let b = rand_box(&mut rng);
        let target = C2v {
            x: b.min.x + (b.max.x - b.min.x) * rng.unit(),
            y: b.min.y + (b.max.y - b.min.y) * rng.unit(),
        };
        let p = rng.v(80.0);
        let dx = target.x - p.x;
        let dy = target.y - p.y;
        let len = (dx * dx + dy * dy).sqrt();
        let ray = C2Ray {
            p,
            d: C2v {
                x: dx / len,
                y: dy / len,
            },
            t: len * (0.5 + rng.unit() * 1.5),
        };
        cmp_aabb(l, ray, b, format!("row27 aimed {ray:?} {b:?}"));
    }
}

// --- row 28: unnormalized direction --------------------------------------
#[test]
fn row28_aabb_unnormalized_dir() {
    let l = libs();
    let mut rng = Rng::new(0x2828);
    for _ in 0..N {
        let b = rand_box(&mut rng);
        let d = rng.unit_dir();
        let mag = 10f32.powf(rng.sym(3.0));
        let ray = C2Ray {
            p: rng.v(80.0),
            d: C2v {
                x: d.x * mag,
                y: d.y * mag,
            },
            t: rng.unit() * 100.0,
        };
        cmp_aabb(l, ray, b, format!("row28 mag={mag} {ray:?} {b:?}"));
    }
}

// --- row 29: each of the four face normals wins --------------------------
#[test]
fn row29_aabb_each_face() {
    let l = libs();
    let mut rng = Rng::new(0x2929);
    let b = C2AABB {
        min: C2v { x: -2.0, y: -3.0 },
        max: C2v { x: 5.0, y: 7.0 },
    };
    for side in 0..4 {
        for _ in 0..5_000 {
            let (p, d) = match side {
                0 => (
                    C2v {
                        x: -20.0,
                        y: rng.sym(3.0),
                    },
                    C2v { x: 1.0, y: 0.0 },
                ),
                1 => (
                    C2v {
                        x: 30.0,
                        y: rng.sym(3.0),
                    },
                    C2v { x: -1.0, y: 0.0 },
                ),
                2 => (
                    C2v {
                        x: rng.sym(2.0),
                        y: -25.0,
                    },
                    C2v { x: 0.0, y: 1.0 },
                ),
                _ => (
                    C2v {
                        x: rng.sym(2.0),
                        y: 40.0,
                    },
                    C2v { x: 0.0, y: -1.0 },
                ),
            };
            let ray = C2Ray {
                p,
                d,
                t: 10.0 + rng.unit() * 100.0,
            };
            cmp_aabb(l, ray, b, format!("row29 side{side} {ray:?}"));
        }
    }
    // diagonal rays from each quadrant
    for _ in 0..N {
        let ang = rng.unit() * std::f32::consts::TAU;
        let dist = 20.0 + rng.unit() * 50.0;
        let p = C2v {
            x: 1.5 + ang.cos() * dist,
            y: 2.0 + ang.sin() * dist,
        };
        let ray = C2Ray {
            p,
            d: C2v {
                x: -ang.cos(),
                y: -ang.sin(),
            },
            t: dist * (0.8 + rng.unit() * 0.6),
        };
        cmp_aabb(l, ray, b, format!("row29 diag {ray:?}"));
    }
}

// --- row 30: origin inside the box ---------------------------------------
#[test]
fn row30_aabb_origin_inside() {
    let l = libs();
    let mut rng = Rng::new(0x3030);
    for _ in 0..N {
        let b = rand_box(&mut rng);
        let p = C2v {
            x: b.min.x + (b.max.x - b.min.x) * rng.unit(),
            y: b.min.y + (b.max.y - b.min.y) * rng.unit(),
        };
        let ray = C2Ray {
            p,
            d: rng.unit_dir(),
            t: rng.unit() * 100.0,
        };
        cmp_aabb(l, ray, b, format!("row30 {ray:?} {b:?}"));
    }
}

// --- row 31: ray parallel to a face / along an edge ----------------------
#[test]
fn row31_aabb_parallel_and_edge() {
    let l = libs();
    let mut rng = Rng::new(0x3131);
    let b = C2AABB {
        min: C2v { x: -2.0, y: -3.0 },
        max: C2v { x: 5.0, y: 7.0 },
    };
    let lines = [
        b.min.y, b.max.y, b.min.x, b.max.x, 0.0, -3.0, 7.0, -2.0, 5.0,
    ];
    for &v in &lines {
        for horiz in [true, false] {
            for _ in 0..2_000 {
                let start = -30.0 + rng.unit() * 5.0;
                let ray = if horiz {
                    C2Ray {
                        p: C2v { x: start, y: v },
                        d: C2v { x: 1.0, y: 0.0 },
                        t: 60.0 + rng.unit() * 20.0,
                    }
                } else {
                    C2Ray {
                        p: C2v { x: v, y: start },
                        d: C2v { x: 0.0, y: 1.0 },
                        t: 60.0 + rng.unit() * 20.0,
                    }
                };
                cmp_aabb(l, ray, b, format!("row31 v={v} horiz={horiz} {ray:?}"));
            }
        }
    }
    // exactly-zero direction => p1 == p0, da == db on both axes
    for _ in 0..5_000 {
        let ray = C2Ray {
            p: rng.v(10.0),
            d: C2v { x: 0.0, y: 0.0 },
            t: rng.unit() * 50.0,
        };
        cmp_aabb(l, ray, b, format!("row31 zerodir {ray:?}"));
    }
}

// --- row 32: A.t == 0, < 0, huge ----------------------------------------
#[test]
fn row32_aabb_degenerate_t() {
    let l = libs();
    let mut rng = Rng::new(0x3232);
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
        f32::MIN_POSITIVE,
    ];
    for &t in &ts {
        for _ in 0..3_000 {
            let b = rand_box(&mut rng);
            let ray = C2Ray {
                p: rng.v(40.0),
                d: rng.unit_dir(),
                t,
            };
            cmp_aabb(l, ray, b, format!("row32 t={t} {ray:?} {b:?}"));
        }
    }
}

// --- row 33: flat / zero-area / inverted boxes --------------------------
#[test]
fn row33_aabb_flat_and_inverted() {
    let l = libs();
    let mut rng = Rng::new(0x3333);
    let boxes = [
        C2AABB {
            min: C2v { x: 1.0, y: -3.0 },
            max: C2v { x: 1.0, y: 7.0 },
        },
        C2AABB {
            min: C2v { x: -2.0, y: 2.0 },
            max: C2v { x: 5.0, y: 2.0 },
        },
        C2AABB {
            min: C2v { x: 3.0, y: 3.0 },
            max: C2v { x: 3.0, y: 3.0 },
        },
        C2AABB {
            min: C2v { x: 5.0, y: 7.0 },
            max: C2v { x: -2.0, y: -3.0 },
        },
        C2AABB {
            min: C2v { x: 0.0, y: 0.0 },
            max: C2v { x: -0.0, y: -0.0 },
        },
    ];
    for (i, &b) in boxes.iter().enumerate() {
        for _ in 0..5_000 {
            let ray = C2Ray {
                p: rng.v(20.0),
                d: rng.unit_dir(),
                t: rng.unit() * 60.0,
            };
            cmp_aabb(l, ray, b, format!("row33 box{i} {ray:?} {b:?}"));
        }
        // aimed straight at the degenerate box
        for _ in 0..5_000 {
            let p = rng.v(20.0);
            let dx = b.min.x - p.x;
            let dy = b.min.y - p.y;
            let len = (dx * dx + dy * dy).sqrt();
            let ray = C2Ray {
                p,
                d: C2v {
                    x: dx / len,
                    y: dy / len,
                },
                t: len * (0.9 + rng.unit() * 0.4),
            };
            cmp_aabb(l, ray, b, format!("row33 aimed box{i} {ray:?} {b:?}"));
        }
    }
}

// --- row 34: tie-breaking (equal tN, corner hits) -----------------------
#[test]
fn row34_aabb_tie_breaking() {
    let l = libs();
    let mut rng = Rng::new(0x3434);
    let b = C2AABB {
        min: C2v { x: -1.0, y: -1.0 },
        max: C2v { x: 1.0, y: 1.0 },
    };
    // 45-degree rays hitting exactly a corner => t values tie
    let inv = 1.0f32 / 2f32.sqrt();
    for (sx, sy) in [(1.0f32, 1.0f32), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
        for _ in 0..5_000 {
            let dist = 5.0 + rng.unit() * 20.0;
            let ray = C2Ray {
                p: C2v {
                    x: sx * dist,
                    y: sy * dist,
                },
                d: C2v {
                    x: -sx * inv,
                    y: -sy * inv,
                },
                t: dist * 2.0,
            };
            cmp_aabb(l, ray, b, format!("row34 corner({sx},{sy}) {ray:?}"));
        }
    }
    // square box, ray through the exact centre along a diagonal
    for _ in 0..N {
        let s = rng.unit() * 10.0 + 0.5;
        let bb = C2AABB {
            min: C2v { x: -s, y: -s },
            max: C2v { x: s, y: s },
        };
        let ray = C2Ray {
            p: C2v { x: -3.0 * s, y: -3.0 * s },
            d: C2v { x: inv, y: inv },
            t: 10.0 * s,
        };
        cmp_aabb(l, ray, bb, format!("row34 sym s={s}"));
    }
}

// --- row 35: full special / bit-space fuzz -----------------------------
#[test]
fn row35_aabb_fuzz() {
    let l = libs();
    let mut rng = Rng::new(0x3535);
    for _ in 0..N {
        let ray = C2Ray {
            p: rng.v_special(1e6),
            d: rng.v_special(1e6),
            t: rng.special(1e6),
        };
        let b = C2AABB {
            min: rng.v_special(1e6),
            max: rng.v_special(1e6),
        };
        cmp_aabb(l, ray, b, format!("row35 special {ray:?} {b:?}"));
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
        let b = C2AABB {
            min: C2v {
                x: rng.any_bits(),
                y: rng.any_bits(),
            },
            max: C2v {
                x: rng.any_bits(),
                y: rng.any_bits(),
            },
        };
        cmp_aabb(l, ray, b, format!("row35 bits {ray:?} {b:?}"));
    }
    // one special field at a time on an otherwise sane hit configuration
    for &s in &specials_nan_payloads() {
        for slot in 0..9 {
            for _ in 0..300 {
                let mut ray = C2Ray {
                    p: C2v { x: -10.0, y: 0.5 },
                    d: C2v { x: 1.0, y: 0.0 },
                    t: 40.0,
                };
                let mut b = C2AABB {
                    min: C2v { x: -1.0, y: -1.0 },
                    max: C2v { x: 2.0, y: 3.0 },
                };
                match slot {
                    0 => ray.p.x = s,
                    1 => ray.p.y = s,
                    2 => ray.d.x = s,
                    3 => ray.d.y = s,
                    4 => ray.t = s,
                    5 => b.min.x = s,
                    6 => b.min.y = s,
                    7 => b.max.x = s,
                    _ => b.max.y = s,
                }
                let _ = rng.next_u32();
                cmp_aabb(l, ray, b, format!("row35 slot{slot}={:#x}", fb(s)));
            }
        }
    }
}
