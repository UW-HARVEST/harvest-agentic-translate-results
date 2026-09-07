//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every call goes through `dlopen`'d symbols on BOTH shared objects; nothing
//! in the Rust crate is invoked directly.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::c_void;

/// Iterations per randomized row. Override with `DIFF_ITERS=<n>`.
fn iters() -> u32 {
    std::env::var("DIFF_ITERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(4000)
}


// ===========================================================================
// Level 0 — vector math (rows 1–16)
// ===========================================================================

#[test]
fn row01_c2V_mixed() {
    let p = pair();
    let mut g = Rng::new(0x0001);
    for i in 0..iters() {
        let (x, y) = (g.mixed_f32(), g.mixed_f32());
        unsafe {
            diff_eq!(1, i, vb((p.c.c2V)(x, y)), vb((p.r.c2V)(x, y)), format!("{x:e},{y:e}"));
        }
    }
}

#[test]
fn row02_c2Dot_normal() {
    let p = pair();
    let mut g = Rng::new(0x0002);
    for i in 0..iters() {
        let (a, b) = (g.normal_v(), g.normal_v());
        unsafe {
            diff_eq!(2, i, fb((p.c.c2Dot)(a, b)), fb((p.r.c2Dot)(a, b)), format!("{a:?} {b:?}"));
        }
    }
}

#[test]
fn row03_c2Dot_mixed() {
    let p = pair();
    let mut g = Rng::new(0x0003);
    for i in 0..iters() {
        let (a, b) = (g.mixed_v(), g.mixed_v());
        unsafe {
            diff_eq!(3, i, fb((p.c.c2Dot)(a, b)), fb((p.r.c2Dot)(a, b)), format!("{a:?} {b:?}"));
        }
    }
}

#[test]
fn row04_c2Len_normal() {
    let p = pair();
    let mut g = Rng::new(0x0004);
    for i in 0..iters() {
        let a = g.normal_v();
        unsafe {
            diff_eq!(4, i, fb((p.c.c2Len)(a)), fb((p.r.c2Len)(a)), format!("{a:?}"));
        }
    }
}

#[test]
fn row05_c2Len_mixed() {
    let p = pair();
    let mut g = Rng::new(0x0005);
    for i in 0..iters() {
        let a = g.mixed_v();
        unsafe {
            diff_eq!(5, i, fb((p.c.c2Len)(a)), fb((p.r.c2Len)(a)), format!("{a:?}"));
        }
    }
}

#[test]
fn row06_c2Add() {
    let p = pair();
    let mut g = Rng::new(0x0006);
    for i in 0..iters() {
        let (a, b) = if i % 2 == 0 {
            (g.normal_v(), g.normal_v())
        } else {
            (g.mixed_v(), g.mixed_v())
        };
        unsafe {
            diff_eq!(6, i, vb((p.c.c2Add)(a, b)), vb((p.r.c2Add)(a, b)), format!("{a:?} {b:?}"));
        }
    }
}

#[test]
fn row07_c2Sub() {
    let p = pair();
    let mut g = Rng::new(0x0007);
    for i in 0..iters() {
        let (a, b) = if i % 2 == 0 {
            (g.normal_v(), g.normal_v())
        } else {
            (g.mixed_v(), g.mixed_v())
        };
        unsafe {
            diff_eq!(7, i, vb((p.c.c2Sub)(a, b)), vb((p.r.c2Sub)(a, b)), format!("{a:?} {b:?}"));
        }
    }
}

#[test]
fn row08_c2Mulvs() {
    let p = pair();
    let mut g = Rng::new(0x0008);
    for i in 0..iters() {
        let a = g.mixed_v();
        let s = g.mixed_f32();
        unsafe {
            diff_eq!(8, i, vb((p.c.c2Mulvs)(a, s)), vb((p.r.c2Mulvs)(a, s)), format!("{a:?} {s:e}"));
        }
    }
}

#[test]
fn row09_c2Div_nonzero() {
    let p = pair();
    let mut g = Rng::new(0x0009);
    for i in 0..iters() {
        let a = g.normal_v();
        let mut s = g.normal_f32();
        if s == 0.0 {
            s = 3.25;
        }
        unsafe {
            diff_eq!(9, i, vb((p.c.c2Div)(a, s)), vb((p.r.c2Div)(a, s)), format!("{a:?} {s:e}"));
        }
    }
}

#[test]
fn row10_c2Div_mixed() {
    let p = pair();
    let mut g = Rng::new(0x000A);
    for i in 0..iters() {
        let a = g.mixed_v();
        let s = g.mixed_f32();
        unsafe {
            diff_eq!(10, i, vb((p.c.c2Div)(a, s)), vb((p.r.c2Div)(a, s)), format!("{a:?} {s:e}"));
        }
    }
}

#[test]
fn row11_c2Norm_normal() {
    let p = pair();
    let mut g = Rng::new(0x000B);
    for i in 0..iters() {
        let a = g.normal_v();
        unsafe {
            diff_eq!(11, i, vb((p.c.c2Norm)(a)), vb((p.r.c2Norm)(a)), format!("{a:?}"));
        }
    }
}

#[test]
fn row12_c2Norm_mixed() {
    let p = pair();
    let mut g = Rng::new(0x000C);
    for i in 0..iters() {
        let a = g.mixed_v();
        unsafe {
            diff_eq!(12, i, vb((p.c.c2Norm)(a)), vb((p.r.c2Norm)(a)), format!("{a:?}"));
        }
    }
}

#[test]
fn row13_minv_maxv_normal() {
    let p = pair();
    let mut g = Rng::new(0x000D);
    for i in 0..iters() {
        let (a, b) = (g.normal_v(), g.normal_v());
        unsafe {
            diff_eq!(13, i, vb((p.c.c2Minv)(a, b)), vb((p.r.c2Minv)(a, b)), format!("min {a:?} {b:?}"));
            diff_eq!(13, i, vb((p.c.c2Maxv)(a, b)), vb((p.r.c2Maxv)(a, b)), format!("max {a:?} {b:?}"));
        }
    }
}

#[test]
fn row14_minv_maxv_nan_and_signed_zero() {
    let p = pair();
    // Exhaustive over the interesting ternary-vs-fminf cases.
    let vals = [
        0.0f32,
        -0.0f32,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        1.0,
        -1.0,
    ];
    let mut i = 0u32;
    for &ax in &vals {
        for &ay in &vals {
            for &bx in &vals {
                for &by in &vals {
                    let a = C2v { x: ax, y: ay };
                    let b = C2v { x: bx, y: by };
                    unsafe {
                        diff_eq!(14, i, vb((p.c.c2Minv)(a, b)), vb((p.r.c2Minv)(a, b)), format!("min {a:?} {b:?}"));
                        diff_eq!(14, i, vb((p.c.c2Maxv)(a, b)), vb((p.r.c2Maxv)(a, b)), format!("max {a:?} {b:?}"));
                    }
                    i += 1;
                }
            }
        }
    }
    // Plus randomized mixed floats.
    let mut g = Rng::new(0x000E);
    for i in 0..iters() {
        let (a, b) = (g.mixed_v(), g.mixed_v());
        unsafe {
            diff_eq!(14, i, vb((p.c.c2Minv)(a, b)), vb((p.r.c2Minv)(a, b)), format!("min {a:?} {b:?}"));
            diff_eq!(14, i, vb((p.c.c2Maxv)(a, b)), vb((p.r.c2Maxv)(a, b)), format!("max {a:?} {b:?}"));
        }
    }
}

#[test]
fn row15_skew_ccw90() {
    let p = pair();
    let mut g = Rng::new(0x000F);
    for i in 0..iters() {
        let a = g.mixed_v();
        unsafe {
            diff_eq!(15, i, vb((p.c.c2Skew)(a)), vb((p.r.c2Skew)(a)), format!("skew {a:?}"));
            diff_eq!(15, i, vb((p.c.c2CCW90)(a)), vb((p.r.c2CCW90)(a)), format!("ccw90 {a:?}"));
        }
    }
}

#[test]
fn row16_absv() {
    let p = pair();
    let fixed = [
        C2v { x: -0.0, y: 0.0 },
        C2v { x: 0.0, y: -0.0 },
        C2v { x: f32::NAN, y: -f32::NAN },
        C2v { x: f32::INFINITY, y: f32::NEG_INFINITY },
    ];
    for (i, a) in fixed.iter().enumerate() {
        unsafe {
            diff_eq!(16, i, vb((p.c.c2Absv)(*a)), vb((p.r.c2Absv)(*a)), format!("{a:?}"));
        }
    }
    let mut g = Rng::new(0x0010);
    for i in 0..iters() {
        let a = g.mixed_v();
        unsafe {
            diff_eq!(16, i, vb((p.c.c2Absv)(a)), vb((p.r.c2Absv)(a)), format!("{a:?}"));
        }
    }
}

// ===========================================================================
// Level 1 — rotations / transforms (rows 17–25)
// ===========================================================================

#[test]
fn row17_identities() {
    let p = pair();
    unsafe {
        diff_eq!(17, 0, rb((p.c.c2RotIdentity)()), rb((p.r.c2RotIdentity)()), "c2RotIdentity");
        diff_eq!(17, 1, xb((p.c.c2xIdentity)()), xb((p.r.c2xIdentity)()), "c2xIdentity");
    }
}

#[test]
fn row18_mulrv_identity() {
    let p = pair();
    let mut g = Rng::new(0x0012);
    let r = C2r { c: 1.0, s: 0.0 };
    for i in 0..iters() {
        let v = g.mixed_v();
        unsafe {
            diff_eq!(18, i, vb((p.c.c2Mulrv)(r, v)), vb((p.r.c2Mulrv)(r, v)), format!("{v:?}"));
            diff_eq!(18, i, vb((p.c.c2MulrvT)(r, v)), vb((p.r.c2MulrvT)(r, v)), format!("T {v:?}"));
        }
    }
}

#[test]
fn row19_mulrv_unit() {
    let p = pair();
    let mut g = Rng::new(0x0013);
    for i in 0..iters() {
        let r = g.unit_r();
        let v = g.normal_v();
        unsafe {
            diff_eq!(19, i, vb((p.c.c2Mulrv)(r, v)), vb((p.r.c2Mulrv)(r, v)), format!("{r:?} {v:?}"));
            diff_eq!(19, i, vb((p.c.c2MulrvT)(r, v)), vb((p.r.c2MulrvT)(r, v)), format!("T {r:?} {v:?}"));
        }
    }
}

#[test]
fn row20_mulrv_nonunit() {
    let p = pair();
    let mut g = Rng::new(0x0014);
    for i in 0..iters() {
        let r = g.mixed_r();
        let v = g.mixed_v();
        unsafe {
            diff_eq!(20, i, vb((p.c.c2Mulrv)(r, v)), vb((p.r.c2Mulrv)(r, v)), format!("{r:?} {v:?}"));
            diff_eq!(20, i, vb((p.c.c2MulrvT)(r, v)), vb((p.r.c2MulrvT)(r, v)), format!("T {r:?} {v:?}"));
        }
    }
}

fn mulxvT_row(row: u32, seed: u64, mk: fn(&mut Rng) -> C2x, mkv: fn(&mut Rng) -> C2v) {
    let p = pair();
    let mut g = Rng::new(seed);
    for i in 0..iters() {
        let x = mk(&mut g);
        let v = mkv(&mut g);
        unsafe {
            diff_eq!(row, i, vb((p.c.c2MulxvT)(x, v)), vb((p.r.c2MulxvT)(x, v)), format!("{x:?} {v:?}"));
        }
    }
}

#[test]
fn row21_mulxvT_identity() {
    mulxvT_row(
        21,
        0x0015,
        |_g| C2x {
            p: C2v { x: 0.0, y: 0.0 },
            r: C2r { c: 1.0, s: 0.0 },
        },
        |g| g.mixed_v(),
    );
}

#[test]
fn row22_mulxvT_translation() {
    mulxvT_row(
        22,
        0x0016,
        |g| C2x {
            p: g.normal_v(),
            r: C2r { c: 1.0, s: 0.0 },
        },
        |g| g.normal_v(),
    );
}

#[test]
fn row23_mulxvT_rotation() {
    mulxvT_row(
        23,
        0x0017,
        |g| C2x {
            p: C2v { x: 0.0, y: 0.0 },
            r: g.unit_r(),
        },
        |g| g.normal_v(),
    );
}

#[test]
fn row24_mulxvT_rot_trans() {
    mulxvT_row(
        24,
        0x0018,
        |g| C2x {
            p: g.normal_v(),
            r: g.unit_r(),
        },
        |g| g.normal_v(),
    );
    // and fully adversarial
    mulxvT_row(24, 0x1018, |g| g.mixed_x(), |g| g.mixed_v());
}

#[test]
fn row25_mulmvT() {
    let p = pair();
    let mut g = Rng::new(0x0019);
    for i in 0..iters() {
        // Half random, half the {CCW90(y), y} shape c2RaytoCapsule builds.
        let m = if i % 2 == 0 {
            C2m {
                x: g.mixed_v(),
                y: g.mixed_v(),
            }
        } else {
            let y = unsafe { (p.c.c2Norm)(g.normal_v()) };
            let x = unsafe { (p.c.c2CCW90)(y) };
            C2m { x, y }
        };
        let v = if i % 3 == 0 { g.mixed_v() } else { g.normal_v() };
        unsafe {
            diff_eq!(25, i, vb((p.c.c2MulmvT)(m, v)), vb((p.r.c2MulmvT)(m, v)), format!("{m:?} {v:?}"));
        }
    }
}

// ===========================================================================
// Level 2 — predicates (rows 26–33)
// ===========================================================================

#[test]
fn row26_aabbtoaabb_normal() {
    let p = pair();
    let mut g = Rng::new(0x001A);
    for i in 0..iters() {
        let (a, b) = (g.normal_aabb(), g.normal_aabb());
        unsafe {
            diff_eq!(26, i, (p.c.c2AABBtoAABB)(a, b), (p.r.c2AABBtoAABB)(a, b), format!("{a:?} {b:?}"));
        }
    }
}

#[test]
fn row27_aabbtoaabb_degenerate() {
    let p = pair();
    let mut g = Rng::new(0x001B);
    for i in 0..iters() {
        let pt = g.normal_v();
        let a = C2AABB { min: pt, max: pt };
        let b = if g.bool() {
            let q = g.normal_v();
            C2AABB { min: q, max: q }
        } else {
            g.normal_aabb()
        };
        unsafe {
            diff_eq!(27, i, (p.c.c2AABBtoAABB)(a, b), (p.r.c2AABBtoAABB)(a, b), format!("{a:?} {b:?}"));
            diff_eq!(27, i, (p.c.c2AABBtoAABB)(b, a), (p.r.c2AABBtoAABB)(b, a), format!("swap {b:?} {a:?}"));
        }
    }
}

#[test]
fn row28_aabbtoaabb_inverted() {
    let p = pair();
    let mut g = Rng::new(0x001C);
    for i in 0..iters() {
        let n = g.normal_aabb();
        let inv = C2AABB {
            min: n.max,
            max: n.min,
        };
        let b = g.normal_aabb();
        unsafe {
            diff_eq!(28, i, (p.c.c2AABBtoAABB)(inv, b), (p.r.c2AABBtoAABB)(inv, b), format!("{inv:?} {b:?}"));
            diff_eq!(28, i, (p.c.c2AABBtoAABB)(inv, inv), (p.r.c2AABBtoAABB)(inv, inv), format!("both {inv:?}"));
        }
    }
}

#[test]
fn row29_aabbtoaabb_mixed() {
    let p = pair();
    let mut g = Rng::new(0x001D);
    for i in 0..iters() {
        let (a, b) = (g.mixed_aabb(), g.mixed_aabb());
        unsafe {
            diff_eq!(29, i, (p.c.c2AABBtoAABB)(a, b), (p.r.c2AABBtoAABB)(a, b), format!("{a:?} {b:?}"));
        }
    }
}

#[test]
fn row30_aabbtopoint_normal() {
    let p = pair();
    let mut g = Rng::new(0x001E);
    for i in 0..iters() {
        let a = g.normal_aabb();
        // Include exact edge/corner points, not just random interiors.
        let pt = match i % 6 {
            0 => a.min,
            1 => a.max,
            2 => C2v { x: a.min.x, y: a.max.y },
            3 => C2v { x: a.max.x, y: a.min.y },
            4 => C2v {
                x: (a.min.x + a.max.x) * 0.5,
                y: a.min.y,
            },
            _ => g.normal_v(),
        };
        unsafe {
            diff_eq!(30, i, (p.c.c2AABBtoPoint)(a, pt), (p.r.c2AABBtoPoint)(a, pt), format!("{a:?} {pt:?}"));
        }
    }
}

#[test]
fn row31_aabbtopoint_degenerate_mixed() {
    let p = pair();
    let mut g = Rng::new(0x001F);
    for i in 0..iters() {
        let n = g.normal_aabb();
        let a = match i % 3 {
            0 => C2AABB { min: n.min, max: n.min },
            1 => C2AABB { min: n.max, max: n.min },
            _ => g.mixed_aabb(),
        };
        let pt = if i % 2 == 0 { g.mixed_v() } else { g.normal_v() };
        unsafe {
            diff_eq!(31, i, (p.c.c2AABBtoPoint)(a, pt), (p.r.c2AABBtoPoint)(a, pt), format!("{a:?} {pt:?}"));
        }
    }
}

#[test]
fn row32_circletopoint_normal() {
    let p = pair();
    let mut g = Rng::new(0x0020);
    for i in 0..iters() {
        let c = g.normal_circle();
        let pt = match i % 4 {
            0 => c.p,                                       // dead centre
            1 => C2v { x: c.p.x + c.r, y: c.p.y },          // exactly on rim
            2 => C2v { x: c.p.x, y: c.p.y + c.r },          // exactly on rim
            _ => g.normal_v(),
        };
        unsafe {
            diff_eq!(32, i, (p.c.c2CircleToPoint)(c, pt), (p.r.c2CircleToPoint)(c, pt), format!("{c:?} {pt:?}"));
        }
    }
}

#[test]
fn row33_circletopoint_degenerate_mixed() {
    let p = pair();
    let mut g = Rng::new(0x0021);
    for i in 0..iters() {
        let base = g.normal_circle();
        let c = match i % 4 {
            0 => C2Circle { p: base.p, r: 0.0 },
            1 => C2Circle { p: base.p, r: -base.r },
            2 => g.mixed_circle(),
            _ => base,
        };
        let pt = if i % 2 == 0 { g.mixed_v() } else { c.p };
        unsafe {
            diff_eq!(33, i, (p.c.c2CircleToPoint)(c, pt), (p.r.c2CircleToPoint)(c, pt), format!("{c:?} {pt:?}"));
        }
    }
}

// ===========================================================================
// Level 3a — c2RaytoCircle (rows 34–40)
// ===========================================================================

fn circle_row(row: u32, seed: u64, mk: fn(&mut Rng, u32) -> (C2Ray, C2Circle)) {
    let p = pair();
    let mut g = Rng::new(seed);
    for i in 0..iters() {
        let (ray, ci) = mk(&mut g, i);
        let rc = ray_call(|o| unsafe { (p.c.c2RaytoCircle)(ray, ci, o) });
        let rr = ray_call(|o| unsafe { (p.r.c2RaytoCircle)(ray, ci, o) });
        diff_eq!(row, i, rc, rr, format!("{ray:?} {ci:?}"));
    }
}

#[test]
fn row34_raytocircle_axis() {
    circle_row(34, 0x0022, |g, i| {
        let c = g.normal_circle();
        // Aim from outside along an axis, mostly toward the circle.
        let mut r = g.axis_ray(i);
        if i % 3 == 0 {
            r.p = match i % 4 {
                0 => C2v { x: c.p.x - c.r - 3.0, y: c.p.y },
                1 => C2v { x: c.p.x + c.r + 3.0, y: c.p.y },
                2 => C2v { x: c.p.x, y: c.p.y - c.r - 3.0 },
                _ => C2v { x: c.p.x, y: c.p.y + c.r + 3.0 },
            };
        }
        r.t = 100.0;
        (r, c)
    });
}

#[test]
fn row35_raytocircle_diagonal() {
    circle_row(35, 0x0023, |g, _i| {
        let c = g.normal_circle();
        let mut r = g.normal_ray();
        // Half the time aim exactly at the centre so hits are common.
        if g.bool() {
            let dx = c.p.x - r.p.x;
            let dy = c.p.y - r.p.y;
            let l = (dx * dx + dy * dy).sqrt();
            r.d = C2v { x: dx / l, y: dy / l };
            r.t = l + g.range(-5.0, 20.0);
        }
        (r, c)
    });
}

#[test]
fn row36_raytocircle_nonnormalized_dir() {
    circle_row(36, 0x0024, |g, _i| {
        let c = g.normal_circle();
        let mut r = g.normal_ray();
        let s = g.range(0.05, 8.0);
        r.d = C2v { x: r.d.x * s, y: r.d.y * s };
        (r, c)
    });
}

#[test]
fn row37_raytocircle_origin_inside() {
    circle_row(37, 0x0025, |g, _i| {
        let c = C2Circle {
            p: g.normal_v(),
            r: g.range(1.0, 15.0),
        };
        let th = g.range(-7.0, 7.0);
        let rad = g.range(0.0, 0.95) * c.r;
        let r = C2Ray {
            p: C2v {
                x: c.p.x + rad * th.cos(),
                y: c.p.y + rad * th.sin(),
            },
            d: C2v {
                x: th.cos(),
                y: th.sin(),
            },
            t: g.range(0.0, 50.0),
        };
        (r, c)
    });
}

#[test]
fn row38_raytocircle_origin_on_boundary() {
    circle_row(38, 0x0026, |g, _i| {
        let c = C2Circle {
            p: g.normal_v(),
            r: g.range(0.5, 15.0),
        };
        let th = g.range(-7.0, 7.0);
        let r = C2Ray {
            p: C2v {
                x: c.p.x + c.r * th.cos(),
                y: c.p.y + c.r * th.sin(),
            },
            d: {
                let phi = g.range(-7.0, 7.0);
                C2v { x: phi.cos(), y: phi.sin() }
            },
            t: g.range(0.0, 50.0),
        };
        (r, c)
    });
}

#[test]
fn row39_raytocircle_t_sweep() {
    circle_row(39, 0x0027, |g, i| {
        let c = g.normal_circle();
        let mut r = g.normal_ray();
        // Aim at the centre, then sweep t including exactly the hit distance.
        let dx = c.p.x - r.p.x;
        let dy = c.p.y - r.p.y;
        let l = (dx * dx + dy * dy).sqrt();
        r.d = C2v { x: dx / l, y: dy / l };
        r.t = if i % 9 == 8 { l - c.r } else { g.t_sweep(i) };
        (r, c)
    });
}

#[test]
fn row40_raytocircle_degenerate() {
    circle_row(40, 0x0028, |g, i| {
        let base = g.normal_circle();
        let c = match i % 4 {
            0 => C2Circle { p: base.p, r: 0.0 },
            1 => C2Circle { p: base.p, r: 1e30 },
            2 => g.mixed_circle(),
            _ => base,
        };
        let r = match i % 3 {
            0 => C2Ray {
                p: g.normal_v(),
                d: C2v { x: 0.0, y: 0.0 },
                t: g.t_sweep(i),
            },
            1 => g.mixed_ray(),
            _ => g.normal_ray(),
        };
        (r, c)
    });
}

// ===========================================================================
// Level 3b — c2RaytoAABB (rows 41–50)
// ===========================================================================

fn aabb_row(row: u32, seed: u64, mk: fn(&mut Rng, u32) -> (C2Ray, C2AABB)) {
    let p = pair();
    let mut g = Rng::new(seed);
    for i in 0..iters() {
        let (ray, b) = mk(&mut g, i);
        let rc = ray_call(|o| unsafe { (p.c.c2RaytoAABB)(ray, b, o) });
        let rr = ray_call(|o| unsafe { (p.r.c2RaytoAABB)(ray, b, o) });
        diff_eq!(row, i, rc, rr, format!("{ray:?} {b:?}"));
    }
}

/// Ray starting outside `b` and travelling along `axis` through its centre.
fn through_box(g: &mut Rng, b: C2AABB, axis: u32) -> C2Ray {
    let cx = (b.min.x + b.max.x) * 0.5;
    let cy = (b.min.y + b.max.y) * 0.5;
    let ex = (b.max.x - b.min.x) * 0.5 + g.range(0.5, 10.0);
    let ey = (b.max.y - b.min.y) * 0.5 + g.range(0.5, 10.0);
    let (o, d) = match axis & 3 {
        0 => (C2v { x: cx - ex, y: cy }, C2v { x: 1.0, y: 0.0 }),
        1 => (C2v { x: cx + ex, y: cy }, C2v { x: -1.0, y: 0.0 }),
        2 => (C2v { x: cx, y: cy - ey }, C2v { x: 0.0, y: 1.0 }),
        _ => (C2v { x: cx, y: cy + ey }, C2v { x: 0.0, y: -1.0 }),
    };
    C2Ray {
        p: o,
        d,
        t: g.range(0.0, 80.0),
    }
}

#[test]
fn row41_raytoaabb_plus_x() {
    aabb_row(41, 0x0029, |g, _i| {
        let b = g.normal_aabb();
        (through_box(g, b, 0), b)
    });
}

#[test]
fn row42_raytoaabb_minus_x() {
    aabb_row(42, 0x002A, |g, _i| {
        let b = g.normal_aabb();
        (through_box(g, b, 1), b)
    });
}

#[test]
fn row43_raytoaabb_plus_y() {
    aabb_row(43, 0x002B, |g, _i| {
        let b = g.normal_aabb();
        (through_box(g, b, 2), b)
    });
}

#[test]
fn row44_raytoaabb_minus_y() {
    aabb_row(44, 0x002C, |g, _i| {
        let b = g.normal_aabb();
        (through_box(g, b, 3), b)
    });
}

#[test]
fn row45_raytoaabb_diagonal() {
    aabb_row(45, 0x002D, |g, _i| {
        let b = g.normal_aabb();
        let cx = (b.min.x + b.max.x) * 0.5;
        let cy = (b.min.y + b.max.y) * 0.5;
        let th = g.range(-7.0, 7.0);
        let dist = g.range(1.0, 40.0);
        let o = C2v {
            x: cx - dist * th.cos(),
            y: cy - dist * th.sin(),
        };
        (
            C2Ray {
                p: o,
                d: C2v { x: th.cos(), y: th.sin() },
                t: g.range(0.0, 80.0),
            },
            b,
        )
    });
}

#[test]
fn row46_raytoaabb_origin_inside() {
    aabb_row(46, 0x002E, |g, _i| {
        let b = g.normal_aabb();
        let o = C2v {
            x: b.min.x + (b.max.x - b.min.x) * g.unit(),
            y: b.min.y + (b.max.y - b.min.y) * g.unit(),
        };
        let th = g.range(-7.0, 7.0);
        (
            C2Ray {
                p: o,
                d: C2v { x: th.cos(), y: th.sin() },
                t: g.range(0.0, 40.0),
            },
            b,
        )
    });
}

#[test]
fn row47_raytoaabb_origin_on_face_or_corner() {
    aabb_row(47, 0x002F, |g, i| {
        let b = g.normal_aabb();
        let o = match i % 8 {
            0 => b.min,
            1 => b.max,
            2 => C2v { x: b.min.x, y: b.max.y },
            3 => C2v { x: b.max.x, y: b.min.y },
            4 => C2v { x: b.min.x, y: (b.min.y + b.max.y) * 0.5 },
            5 => C2v { x: b.max.x, y: (b.min.y + b.max.y) * 0.5 },
            6 => C2v { x: (b.min.x + b.max.x) * 0.5, y: b.min.y },
            _ => C2v { x: (b.min.x + b.max.x) * 0.5, y: b.max.y },
        };
        let th = g.range(-7.0, 7.0);
        (
            C2Ray {
                p: o,
                d: C2v { x: th.cos(), y: th.sin() },
                t: g.range(0.0, 40.0),
            },
            b,
        )
    });
}

#[test]
fn row48_raytoaabb_t_sweep() {
    aabb_row(48, 0x0030, |g, i| {
        let b = g.normal_aabb();
        let mut r = through_box(g, b, i);
        r.t = g.t_sweep(i);
        (r, b)
    });
}

#[test]
fn row49_raytoaabb_degenerate_boxes() {
    aabb_row(49, 0x0031, |g, i| {
        let n = g.normal_aabb();
        let b = match i % 4 {
            0 => C2AABB { min: n.min, max: n.min },
            1 => C2AABB { min: n.max, max: n.min },
            2 => C2AABB {
                min: C2v { x: -1e30, y: -1e30 },
                max: C2v { x: 1e30, y: 1e30 },
            },
            _ => n,
        };
        (through_box(g, n, i), b)
    });
}

#[test]
fn row50_raytoaabb_fuzz() {
    aabb_row(50, 0x0032, |g, i| {
        let b = if i % 3 == 0 { g.mixed_aabb() } else { g.normal_aabb() };
        let r = if i % 2 == 0 { g.mixed_ray() } else { g.normal_ray() };
        (r, b)
    });
}

// ===========================================================================
// Level 3c — c2RaytoCapsule (rows 51–62)
// ===========================================================================

fn capsule_row(row: u32, seed: u64, mk: fn(&mut Rng, u32) -> (C2Ray, C2Capsule)) {
    let p = pair();
    let mut g = Rng::new(seed);
    for i in 0..iters() {
        let (ray, cap) = mk(&mut g, i);
        let rc = ray_call(|o| unsafe { (p.c.c2RaytoCapsule)(ray, cap, o) });
        let rr = ray_call(|o| unsafe { (p.r.c2RaytoCapsule)(ray, cap, o) });
        diff_eq!(row, i, rc, rr, format!("{ray:?} {cap:?}"));
    }
}

fn vertical_capsule(g: &mut Rng) -> C2Capsule {
    let x = g.range(-20.0, 20.0);
    let y0 = g.range(-20.0, 20.0);
    let h = g.range(0.5, 25.0);
    C2Capsule {
        a: C2v { x, y: y0 },
        b: C2v { x, y: y0 + h },
        r: g.range(0.25, 6.0),
    }
}

#[test]
fn row51_raytocapsule_side_hit_positive_x() {
    capsule_row(51, 0x0033, |g, _i| {
        let cap = vertical_capsule(g);
        // Start well to the +x side, aim back at the axis mid-height.
        let mid = C2v {
            x: cap.a.x,
            y: (cap.a.y + cap.b.y) * 0.5,
        };
        let o = C2v {
            x: mid.x + cap.r + g.range(1.0, 20.0),
            y: mid.y + g.range(-1.0, 1.0),
        };
        let dx = mid.x - o.x;
        let dy = mid.y - o.y;
        let l = (dx * dx + dy * dy).sqrt();
        (
            C2Ray {
                p: o,
                d: C2v { x: dx / l, y: dy / l },
                t: l + g.range(-1.0, 10.0),
            },
            cap,
        )
    });
}

#[test]
fn row52_raytocapsule_side_hit_negative_x() {
    capsule_row(52, 0x0034, |g, _i| {
        let cap = vertical_capsule(g);
        let mid = C2v {
            x: cap.a.x,
            y: (cap.a.y + cap.b.y) * 0.5,
        };
        let o = C2v {
            x: mid.x - cap.r - g.range(1.0, 20.0),
            y: mid.y + g.range(-1.0, 1.0),
        };
        let dx = mid.x - o.x;
        let dy = mid.y - o.y;
        let l = (dx * dx + dy * dy).sqrt();
        (
            C2Ray {
                p: o,
                d: C2v { x: dx / l, y: dy / l },
                t: l + g.range(-1.0, 10.0),
            },
            cap,
        )
    });
}

#[test]
fn row53_raytocapsule_origin_in_bb() {
    capsule_row(53, 0x0035, |g, _i| {
        let cap = vertical_capsule(g);
        // Inside the |x| <= r, 0 <= y <= len slab.
        let o = C2v {
            x: cap.a.x + g.range(-1.0, 1.0) * cap.r * 0.9,
            y: cap.a.y + (cap.b.y - cap.a.y) * g.unit(),
        };
        let th = g.range(-7.0, 7.0);
        (
            C2Ray {
                p: o,
                d: C2v { x: th.cos(), y: th.sin() },
                t: g.range(0.0, 30.0),
            },
            cap,
        )
    });
}

#[test]
fn row54_raytocapsule_origin_in_cap_a() {
    capsule_row(54, 0x0036, |g, _i| {
        let cap = g.normal_capsule();
        let cap = C2Capsule { r: cap.r.max(0.5), ..cap };
        let th = g.range(-7.0, 7.0);
        let rad = g.range(0.0, 0.9) * cap.r;
        let o = C2v {
            x: cap.a.x + rad * th.cos(),
            y: cap.a.y + rad * th.sin(),
        };
        let phi = g.range(-7.0, 7.0);
        (
            C2Ray {
                p: o,
                d: C2v { x: phi.cos(), y: phi.sin() },
                t: g.range(0.0, 30.0),
            },
            cap,
        )
    });
}

#[test]
fn row55_raytocapsule_origin_in_cap_b() {
    capsule_row(55, 0x0037, |g, _i| {
        let cap = g.normal_capsule();
        let cap = C2Capsule { r: cap.r.max(0.5), ..cap };
        let th = g.range(-7.0, 7.0);
        let rad = g.range(0.0, 0.9) * cap.r;
        let o = C2v {
            x: cap.b.x + rad * th.cos(),
            y: cap.b.y + rad * th.sin(),
        };
        let phi = g.range(-7.0, 7.0);
        (
            C2Ray {
                p: o,
                d: C2v { x: phi.cos(), y: phi.sin() },
                t: g.range(0.0, 30.0),
            },
            cap,
        )
    });
}

/// Origin inside the infinite slab `|yAp.x| < r` but outside the bb (yAp.y
/// outside [0, len]) — drives the `c2RaytoCircle(Ca/Cb)` delegation.
fn slab_origin_capsule(g: &mut Rng, below: bool) -> (C2Ray, C2Capsule) {
    let cap = vertical_capsule(g);
    let off = g.range(0.0, 0.95) * cap.r;
    let y = if below {
        cap.a.y - g.range(0.1, 20.0)
    } else {
        cap.b.y + g.range(0.1, 20.0)
    };
    let o = C2v {
        x: cap.a.x + if g.bool() { off } else { -off },
        y,
    };
    let th = g.range(-7.0, 7.0);
    (
        C2Ray {
            p: o,
            d: C2v { x: th.cos(), y: th.sin() },
            t: g.range(0.0, 60.0),
        },
        cap,
    )
}

#[test]
fn row56_raytocapsule_slab_below() {
    capsule_row(56, 0x0038, |g, _i| slab_origin_capsule(g, true));
}

#[test]
fn row57_raytocapsule_slab_above() {
    capsule_row(57, 0x0039, |g, _i| slab_origin_capsule(g, false));
}

/// Origin outside the slab, crossing it below the `a` cap → `y <= 0` branch.
#[test]
fn row58_raytocapsule_cross_below_a() {
    capsule_row(58, 0x003A, |g, _i| {
        let cap = vertical_capsule(g);
        let o = C2v {
            x: cap.a.x + cap.r + g.range(0.5, 20.0),
            y: cap.a.y - g.range(0.5, 20.0),
        };
        // Aim at a point on the far side, below cap.a.y so the crossing y <= 0.
        let target = C2v {
            x: cap.a.x - cap.r - g.range(0.5, 5.0),
            y: cap.a.y - g.range(0.5, 20.0),
        };
        let dx = target.x - o.x;
        let dy = target.y - o.y;
        let l = (dx * dx + dy * dy).sqrt();
        (
            C2Ray {
                p: o,
                d: C2v { x: dx / l, y: dy / l },
                t: l + g.range(-2.0, 20.0),
            },
            cap,
        )
    });
}

/// Origin outside the slab, crossing it above the `b` cap → `y >= yBb.y`.
#[test]
fn row59_raytocapsule_cross_above_b() {
    capsule_row(59, 0x003B, |g, _i| {
        let cap = vertical_capsule(g);
        let o = C2v {
            x: cap.a.x + cap.r + g.range(0.5, 20.0),
            y: cap.b.y + g.range(0.5, 20.0),
        };
        let target = C2v {
            x: cap.a.x - cap.r - g.range(0.5, 5.0),
            y: cap.b.y + g.range(0.5, 20.0),
        };
        let dx = target.x - o.x;
        let dy = target.y - o.y;
        let l = (dx * dx + dy * dy).sqrt();
        (
            C2Ray {
                p: o,
                d: C2v { x: dx / l, y: dy / l },
                t: l + g.range(-2.0, 20.0),
            },
            cap,
        )
    });
}

#[test]
fn row60_raytocapsule_diagonal_and_reversed_axis() {
    capsule_row(60, 0x003C, |g, i| {
        let mut cap = g.normal_capsule();
        cap.r = cap.r.max(0.25);
        if i % 2 == 0 {
            // force a "downward" axis so yBb.y is negative
            if cap.a.y < cap.b.y {
                std::mem::swap(&mut cap.a, &mut cap.b);
            }
        }
        let mid = C2v {
            x: (cap.a.x + cap.b.x) * 0.5,
            y: (cap.a.y + cap.b.y) * 0.5,
        };
        let th = g.range(-7.0, 7.0);
        let dist = g.range(0.0, 40.0);
        let o = C2v {
            x: mid.x - dist * th.cos(),
            y: mid.y - dist * th.sin(),
        };
        (
            C2Ray {
                p: o,
                d: C2v { x: th.cos(), y: th.sin() },
                t: g.range(0.0, 80.0),
            },
            cap,
        )
    });
}

#[test]
fn row61_raytocapsule_t_sweep_and_degenerate_dir() {
    capsule_row(61, 0x003D, |g, i| {
        let cap = vertical_capsule(g);
        let mid = C2v {
            x: cap.a.x,
            y: (cap.a.y + cap.b.y) * 0.5,
        };
        let o = C2v {
            x: mid.x + cap.r + g.range(1.0, 20.0),
            y: mid.y,
        };
        let d = match i % 5 {
            0 => C2v { x: 0.0, y: 0.0 },
            1 => C2v { x: -3.7, y: 0.0 },
            2 => C2v { x: -1.0, y: 0.0 },
            3 => C2v { x: 0.0, y: 1.0 },
            _ => {
                let th = g.range(-7.0, 7.0);
                C2v { x: th.cos(), y: th.sin() }
            }
        };
        (
            C2Ray {
                p: o,
                d,
                t: g.t_sweep(i),
            },
            cap,
        )
    });
}

#[test]
fn row62_raytocapsule_fuzz() {
    capsule_row(62, 0x003E, |g, i| {
        let cap = match i % 5 {
            0 => {
                let a = g.normal_v();
                C2Capsule { a, b: a, r: g.range(0.0, 5.0) } // degenerate axis
            }
            1 => {
                let c = g.normal_capsule();
                C2Capsule { r: 0.0, ..c }
            }
            2 => {
                let c = g.normal_capsule();
                C2Capsule { r: -c.r, ..c }
            }
            3 => g.mixed_capsule(),
            _ => g.normal_capsule(),
        };
        let r = if i % 2 == 0 { g.mixed_ray() } else { g.normal_ray() };
        (r, cap)
    });
}

// ===========================================================================
// Level 3d — c2RaytoPoly (rows 63–78)
// ===========================================================================

fn poly_row(row: u32, seed: u64, mk: fn(&mut Rng, u32) -> (C2Ray, C2Poly, Option<C2x>)) {
    let p = pair();
    let mut g = Rng::new(seed);
    for i in 0..iters() {
        let (ray, poly, bx) = mk(&mut g, i);
        let bxp: *const C2x = match &bx {
            Some(x) => x as *const C2x,
            None => std::ptr::null(),
        };
        let rc = ray_call(|o| unsafe { (p.c.c2RaytoPoly)(ray, &poly as *const C2Poly, bxp, o) });
        let rr = ray_call(|o| unsafe { (p.r.c2RaytoPoly)(ray, &poly as *const C2Poly, bxp, o) });
        diff_eq!(row, i, rc, rr, format!("{ray:?} count={} {bx:?}", poly.count));
    }
}

#[test]
fn row63_raytopoly_null_bx_box_plus_x() {
    poly_row(63, 0x003F, |g, _i| {
        let poly = poly_ray_poly();
        let o = C2v {
            x: -g.range(1.0, 30.0),
            y: g.range(-15.0, 15.0),
        };
        (
            C2Ray {
                p: o,
                d: C2v { x: 1.0, y: 0.0 },
                t: g.range(0.0, 40.0),
            },
            poly,
            None,
        )
    });
}

#[test]
fn row64_raytopoly_null_bx_box_minus_y() {
    poly_row(64, 0x0040, |g, _i| {
        let poly = poly_ray_poly();
        let o = C2v {
            x: g.range(-6.0, 6.0),
            y: g.range(11.0, 30.0),
        };
        (
            C2Ray {
                p: o,
                d: C2v { x: 0.0, y: -1.0 },
                t: g.range(0.0, 40.0),
            },
            poly,
            None,
        )
    });
}

#[test]
fn row65_raytopoly_triangle() {
    poly_row(65, 0x0041, |g, _i| {
        let poly = ngon(3, g.normal_v(), g.range(0.5, 15.0), g.range(-7.0, 7.0));
        (g.normal_ray(), poly, None)
    });
}

#[test]
fn row66_raytopoly_ngons_5_to_8() {
    poly_row(66, 0x0042, |g, i| {
        let n = 5 + (i % 4) as usize;
        let poly = ngon(n, g.normal_v(), g.range(0.5, 15.0), g.range(-7.0, 7.0));
        let mut r = g.normal_ray();
        // Half the rays aimed at the polygon centre so hits are common.
        if g.bool() {
            let c = C2v {
                x: (poly.verts[0].x + poly.verts[n / 2].x) * 0.5,
                y: (poly.verts[0].y + poly.verts[n / 2].y) * 0.5,
            };
            let dx = c.x - r.p.x;
            let dy = c.y - r.p.y;
            let l = (dx * dx + dy * dy).sqrt();
            r.d = C2v { x: dx / l, y: dy / l };
            r.t = l + g.range(-3.0, 20.0);
        }
        (r, poly, None)
    });
}

#[test]
fn row67_raytopoly_count_1_and_2() {
    poly_row(67, 0x0043, |g, i| {
        let mut poly = C2Poly::default();
        if i % 2 == 0 {
            // single half-plane
            poly.count = 1;
            poly.verts[0] = g.normal_v();
            let th = g.range(-7.0, 7.0);
            poly.norms[0] = C2v { x: th.cos(), y: th.sin() };
        } else {
            // two opposing planes = a slab
            poly.count = 2;
            let c = g.normal_v();
            let h = g.range(0.5, 10.0);
            poly.verts[0] = C2v { x: c.x + h, y: c.y };
            poly.norms[0] = C2v { x: 1.0, y: 0.0 };
            poly.verts[1] = C2v { x: c.x - h, y: c.y };
            poly.norms[1] = C2v { x: -1.0, y: 0.0 };
        }
        (g.normal_ray(), poly, None)
    });
}

#[test]
fn row68_raytopoly_explicit_identity_bx() {
    // Must produce the same result as the NULL-bx path (row 63 inputs).
    let p = pair();
    let mut g = Rng::new(0x003F); // same seed as row 63
    let ident = unsafe { (p.c.c2xIdentity)() };
    for i in 0..iters() {
        let poly = poly_ray_poly();
        let o = C2v {
            x: -g.range(1.0, 30.0),
            y: g.range(-15.0, 15.0),
        };
        let ray = C2Ray {
            p: o,
            d: C2v { x: 1.0, y: 0.0 },
            t: g.range(0.0, 40.0),
        };
        let rc = ray_call(|out| unsafe {
            (p.c.c2RaytoPoly)(ray, &poly as *const C2Poly, &ident as *const C2x, out)
        });
        let rr = ray_call(|out| unsafe {
            (p.r.c2RaytoPoly)(ray, &poly as *const C2Poly, &ident as *const C2x, out)
        });
        diff_eq!(68, i, rc, rr, format!("{ray:?}"));
        // and identity bx must equal the NULL bx result on both sides
        let rc_null =
            ray_call(|out| unsafe { (p.c.c2RaytoPoly)(ray, &poly as *const C2Poly, std::ptr::null(), out) });
        diff_eq!(68, i, rc, rc_null, format!("identity-vs-null {ray:?}"));
    }
}

#[test]
fn row69_raytopoly_bx_translation() {
    poly_row(69, 0x0044, |g, i| {
        let n = 4 + (i % 5) as usize;
        let poly = ngon(n.min(8), C2v { x: 0.0, y: 0.0 }, g.range(0.5, 12.0), g.range(-7.0, 7.0));
        let bx = C2x {
            p: g.normal_v(),
            r: C2r { c: 1.0, s: 0.0 },
        };
        (g.normal_ray(), poly, Some(bx))
    });
}

#[test]
fn row70_raytopoly_bx_rotation() {
    poly_row(70, 0x0045, |g, i| {
        let n = 4 + (i % 5) as usize;
        let poly = ngon(n.min(8), C2v { x: 0.0, y: 0.0 }, g.range(0.5, 12.0), g.range(-7.0, 7.0));
        let bx = C2x {
            p: C2v { x: 0.0, y: 0.0 },
            r: g.unit_r(),
        };
        (g.normal_ray(), poly, Some(bx))
    });
}

#[test]
fn row71_raytopoly_bx_rot_trans() {
    poly_row(71, 0x0046, |g, i| {
        let n = 3 + (i % 6) as usize;
        let poly = ngon(n.min(8), C2v { x: 0.0, y: 0.0 }, g.range(0.5, 12.0), g.range(-7.0, 7.0));
        let bx = C2x {
            p: g.normal_v(),
            r: g.unit_r(),
        };
        let mut r = g.normal_ray();
        if g.bool() {
            // aim at the transformed origin so we hit often
            let dx = bx.p.x - r.p.x;
            let dy = bx.p.y - r.p.y;
            let l = (dx * dx + dy * dy).sqrt();
            r.d = C2v { x: dx / l, y: dy / l };
            r.t = l + g.range(-3.0, 20.0);
        }
        (r, poly, Some(bx))
    });
}

#[test]
fn row72_raytopoly_bx_nonunit() {
    poly_row(72, 0x0047, |g, _i| {
        let poly = poly_ray_poly();
        let bx = C2x {
            p: g.normal_v(),
            r: C2r {
                c: g.range(-3.0, 3.0),
                s: g.range(-3.0, 3.0),
            },
        };
        (g.normal_ray(), poly, Some(bx))
    });
}

#[test]
fn row73_raytopoly_origin_inside() {
    poly_row(73, 0x0048, |g, i| {
        let n = 3 + (i % 6) as usize;
        let c = g.normal_v();
        let rad = g.range(1.0, 12.0);
        let poly = ngon(n.min(8), c, rad, g.range(-7.0, 7.0));
        let th = g.range(-7.0, 7.0);
        let inner = g.range(0.0, 0.5) * rad;
        let phi = g.range(-7.0, 7.0);
        (
            C2Ray {
                p: C2v {
                    x: c.x + inner * phi.cos(),
                    y: c.y + inner * phi.sin(),
                },
                d: C2v { x: th.cos(), y: th.sin() },
                t: g.range(0.0, 40.0),
            },
            poly,
            None,
        )
    });
}

#[test]
fn row74_raytopoly_dir_parallel_to_plane() {
    poly_row(74, 0x0049, |g, i| {
        let poly = poly_ray_poly(); // axis-aligned normals
        // Pure ±x or ±y directions make den == 0 for two of the four planes.
        let d = match i % 4 {
            0 => C2v { x: 1.0, y: 0.0 },
            1 => C2v { x: -1.0, y: 0.0 },
            2 => C2v { x: 0.0, y: 1.0 },
            _ => C2v { x: 0.0, y: -1.0 },
        };
        // Sweep the origin across, on and outside each slab boundary.
        let o = match i % 6 {
            0 => C2v { x: 0.875, y: g.range(-30.0, 30.0) },
            1 => C2v { x: -0.875, y: g.range(-30.0, 30.0) },
            2 => C2v { x: g.range(-30.0, 30.0), y: 11.5 },
            3 => C2v { x: g.range(-30.0, 30.0), y: -11.5 },
            _ => C2v { x: g.range(-30.0, 30.0), y: g.range(-30.0, 30.0) },
        };
        (
            C2Ray {
                p: o,
                d,
                t: g.range(0.0, 80.0),
            },
            poly,
            None,
        )
    });
}

#[test]
fn row75_raytopoly_t_sweep_and_nonnormalized_dir() {
    poly_row(75, 0x004A, |g, i| {
        let poly = ngon(4 + (i % 5).min(4) as usize, g.normal_v(), g.range(0.5, 12.0), g.range(-7.0, 7.0));
        let th = g.range(-7.0, 7.0);
        let s = if i % 3 == 0 { g.range(0.05, 9.0) } else { 1.0 };
        (
            C2Ray {
                p: g.normal_v(),
                d: C2v { x: th.cos() * s, y: th.sin() * s },
                t: g.t_sweep(i),
            },
            poly,
            None,
        )
    });
}

#[test]
fn row76_raytopoly_random_verts_and_norms() {
    poly_row(76, 0x004B, |g, _i| {
        let mut poly = C2Poly::default();
        poly.count = 8;
        for k in 0..8 {
            poly.verts[k] = g.normal_v();
            poly.norms[k] = g.normal_v(); // deliberately not unit, not convex
        }
        (g.normal_ray(), poly, None)
    });
}

/// `count > 8` overruns `verts[8]`/`norms[8]`. Both libraries must read the
/// SAME trailing bytes, so the polygon is embedded in an over-allocated
/// `PolyBuf` whose slack is fully initialized.
#[test]
fn row77_raytopoly_count_overrun() {
    let p = pair();
    let mut g = Rng::new(0x004C);
    for i in 0..iters() {
        let mut buf = PolyBuf {
            poly: ngon(8, C2v { x: 0.0, y: 0.0 }, g.range(1.0, 12.0), g.range(-7.0, 7.0)),
            slack: [C2v { x: 0.0, y: 0.0 }; 32],
        };
        for k in 0..32 {
            buf.slack[k] = g.normal_v();
        }
        buf.poly.count = 9 + (i % 8) as i32; // 9..16
        let ray = g.normal_ray();
        let pp = &buf.poly as *const C2Poly;
        let rc = ray_call(|o| unsafe { (p.c.c2RaytoPoly)(ray, pp, std::ptr::null(), o) });
        let rr = ray_call(|o| unsafe { (p.r.c2RaytoPoly)(ray, pp, std::ptr::null(), o) });
        diff_eq!(77, i, rc, rr, format!("count={} {ray:?}", buf.poly.count));
    }
}

#[test]
fn row78_raytopoly_fuzz() {
    poly_row(78, 0x004D, |g, i| {
        let mut poly = C2Poly::default();
        poly.count = (i % 9) as i32; // 0..8
        for k in 0..8 {
            poly.verts[k] = if i % 3 == 0 { g.mixed_v() } else { g.normal_v() };
            poly.norms[k] = if i % 3 == 0 { g.mixed_v() } else { g.normal_v() };
        }
        let bx = match i % 4 {
            0 => None,
            1 => Some(g.mixed_x()),
            2 => Some(C2x { p: g.normal_v(), r: g.unit_r() }),
            _ => Some(g.mixed_x()),
        };
        let r = if i % 2 == 0 { g.mixed_ray() } else { g.normal_ray() };
        (r, poly, bx)
    });
}

// ===========================================================================
// Level 4 — c2CastRay dispatch (rows 79–86)
// ===========================================================================

#[test]
fn row79_castray_circle_null_bx() {
    let p = pair();
    let mut g = Rng::new(0x004E);
    for i in 0..iters() {
        let ray = if i % 3 == 0 { g.mixed_ray() } else { g.normal_ray() };
        let ci = if i % 4 == 0 { g.mixed_circle() } else { g.normal_circle() };
        let b = &ci as *const C2Circle as *const c_void;
        let rc = ray_call(|o| unsafe { (p.c.c2CastRay)(ray, b, std::ptr::null(), C2_TYPE_CIRCLE, o) });
        let rr = ray_call(|o| unsafe { (p.r.c2CastRay)(ray, b, std::ptr::null(), C2_TYPE_CIRCLE, o) });
        diff_eq!(79, i, rc, rr, format!("{ray:?} {ci:?}"));
        // must equal calling c2RaytoCircle directly
        let direct = ray_call(|o| unsafe { (p.c.c2RaytoCircle)(ray, ci, o) });
        diff_eq!(79, i, rc, direct, format!("dispatch-vs-direct {ray:?} {ci:?}"));
    }
}

#[test]
fn row80_castray_circle_nonnull_bx_ignored() {
    let p = pair();
    let mut g = Rng::new(0x004F);
    for i in 0..iters() {
        let ray = g.normal_ray();
        let ci = g.normal_circle();
        let bx = g.mixed_x();
        let b = &ci as *const C2Circle as *const c_void;
        let rc = ray_call(|o| unsafe { (p.c.c2CastRay)(ray, b, &bx, C2_TYPE_CIRCLE, o) });
        let rr = ray_call(|o| unsafe { (p.r.c2CastRay)(ray, b, &bx, C2_TYPE_CIRCLE, o) });
        diff_eq!(80, i, rc, rr, format!("{ray:?} {ci:?} {bx:?}"));
    }
}

#[test]
fn row81_castray_aabb_null_bx() {
    let p = pair();
    let mut g = Rng::new(0x0050);
    for i in 0..iters() {
        let bb = if i % 4 == 0 { g.mixed_aabb() } else { g.normal_aabb() };
        let ray = if i % 3 == 0 {
            g.mixed_ray()
        } else {
            through_box(&mut g, bb, i)
        };
        let b = &bb as *const C2AABB as *const c_void;
        let rc = ray_call(|o| unsafe { (p.c.c2CastRay)(ray, b, std::ptr::null(), C2_TYPE_AABB, o) });
        let rr = ray_call(|o| unsafe { (p.r.c2CastRay)(ray, b, std::ptr::null(), C2_TYPE_AABB, o) });
        diff_eq!(81, i, rc, rr, format!("{ray:?} {bb:?}"));
        let direct = ray_call(|o| unsafe { (p.c.c2RaytoAABB)(ray, bb, o) });
        diff_eq!(81, i, rc, direct, format!("dispatch-vs-direct {ray:?} {bb:?}"));
    }
}

#[test]
fn row82_castray_aabb_nonnull_bx_ignored() {
    let p = pair();
    let mut g = Rng::new(0x0051);
    for i in 0..iters() {
        let bb = g.normal_aabb();
        let ray = through_box(&mut g, bb, i);
        let bx = g.mixed_x();
        let b = &bb as *const C2AABB as *const c_void;
        let rc = ray_call(|o| unsafe { (p.c.c2CastRay)(ray, b, &bx, C2_TYPE_AABB, o) });
        let rr = ray_call(|o| unsafe { (p.r.c2CastRay)(ray, b, &bx, C2_TYPE_AABB, o) });
        diff_eq!(82, i, rc, rr, format!("{ray:?} {bb:?} {bx:?}"));
    }
}

#[test]
fn row83_castray_capsule_null_bx() {
    let p = pair();
    let mut g = Rng::new(0x0052);
    for i in 0..iters() {
        let cap = if i % 4 == 0 { g.mixed_capsule() } else { g.normal_capsule() };
        let ray = if i % 3 == 0 { g.mixed_ray() } else { g.normal_ray() };
        let b = &cap as *const C2Capsule as *const c_void;
        let rc = ray_call(|o| unsafe { (p.c.c2CastRay)(ray, b, std::ptr::null(), C2_TYPE_CAPSULE, o) });
        let rr = ray_call(|o| unsafe { (p.r.c2CastRay)(ray, b, std::ptr::null(), C2_TYPE_CAPSULE, o) });
        diff_eq!(83, i, rc, rr, format!("{ray:?} {cap:?}"));
        let direct = ray_call(|o| unsafe { (p.c.c2RaytoCapsule)(ray, cap, o) });
        diff_eq!(83, i, rc, direct, format!("dispatch-vs-direct {ray:?} {cap:?}"));
    }
}

#[test]
fn row84_castray_capsule_nonnull_bx_ignored() {
    let p = pair();
    let mut g = Rng::new(0x0053);
    for i in 0..iters() {
        let cap = g.normal_capsule();
        let ray = g.normal_ray();
        let bx = g.mixed_x();
        let b = &cap as *const C2Capsule as *const c_void;
        let rc = ray_call(|o| unsafe { (p.c.c2CastRay)(ray, b, &bx, C2_TYPE_CAPSULE, o) });
        let rr = ray_call(|o| unsafe { (p.r.c2CastRay)(ray, b, &bx, C2_TYPE_CAPSULE, o) });
        diff_eq!(84, i, rc, rr, format!("{ray:?} {cap:?} {bx:?}"));
    }
}

#[test]
fn row85_castray_poly_null_bx() {
    let p = pair();
    let mut g = Rng::new(0x0054);
    for i in 0..iters() {
        let n = 3 + (i % 6).min(5) as usize;
        let poly = ngon(n, g.normal_v(), g.range(0.5, 12.0), g.range(-7.0, 7.0));
        let ray = g.normal_ray();
        let b = &poly as *const C2Poly as *const c_void;
        let rc = ray_call(|o| unsafe { (p.c.c2CastRay)(ray, b, std::ptr::null(), C2_TYPE_POLY, o) });
        let rr = ray_call(|o| unsafe { (p.r.c2CastRay)(ray, b, std::ptr::null(), C2_TYPE_POLY, o) });
        diff_eq!(85, i, rc, rr, format!("{ray:?} count={n}"));
        let direct =
            ray_call(|o| unsafe { (p.c.c2RaytoPoly)(ray, &poly as *const C2Poly, std::ptr::null(), o) });
        diff_eq!(85, i, rc, direct, format!("dispatch-vs-direct {ray:?} count={n}"));
    }
}

#[test]
fn row86_castray_poly_with_bx() {
    let p = pair();
    let mut g = Rng::new(0x0055);
    for i in 0..iters() {
        let n = 3 + (i % 6).min(5) as usize;
        let poly = ngon(n, C2v { x: 0.0, y: 0.0 }, g.range(0.5, 12.0), g.range(-7.0, 7.0));
        let bx = C2x {
            p: g.normal_v(),
            r: g.unit_r(),
        };
        let ray = g.normal_ray();
        let b = &poly as *const C2Poly as *const c_void;
        let rc = ray_call(|o| unsafe { (p.c.c2CastRay)(ray, b, &bx, C2_TYPE_POLY, o) });
        let rr = ray_call(|o| unsafe { (p.r.c2CastRay)(ray, b, &bx, C2_TYPE_POLY, o) });
        diff_eq!(86, i, rc, rr, format!("{ray:?} count={n} {bx:?}"));
    }
}

// ===========================================================================
// Level 5 — poly_ray driver (rows 87–88)
// ===========================================================================

#[test]
fn row87_poly_ray_fixed() {
    let p = pair();
    let mut c1 = C2Raycast::default();
    let mut c2 = C2Raycast::default();
    let cret = unsafe { (p.c.poly_ray)(&mut c1, &mut c2) };
    let mut r1 = C2Raycast::default();
    let mut r2 = C2Raycast::default();
    let rret = unsafe { (p.r.poly_ray)(&mut r1, &mut r2) };
    diff_eq!(87, 0, cret, rret, "poly_ray return code");
    diff_eq!(87, 0, cb(c1), cb(r1), "poly_ray cast1");
    diff_eq!(87, 0, cb(c2), cb(r2), "poly_ray cast2");
    eprintln!("poly_ray => ret={cret} cast1={c1:?} cast2={c2:?}");
}

#[test]
fn row88_poly_ray_dirty_out_buffers() {
    let p = pair();
    for i in 0..64 {
        let mut c1 = DIRTY;
        let mut c2 = DIRTY;
        let cret = unsafe { (p.c.poly_ray)(&mut c1, &mut c2) };
        let mut r1 = DIRTY;
        let mut r2 = DIRTY;
        let rret = unsafe { (p.r.poly_ray)(&mut r1, &mut r2) };
        diff_eq!(88, i, cret, rret, "return code");
        diff_eq!(88, i, cb(c1), cb(r1), "cast1 (dirty-prefilled)");
        diff_eq!(88, i, cb(c2), cb(r2), "cast2 (dirty-prefilled)");
    }
}
