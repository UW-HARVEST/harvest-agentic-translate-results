//! Phase B — CONFIGS.md rows 67..89 (`c2RaytoPoly`).

#![allow(non_snake_case)]

mod common;
use common::*;

use std::mem::{align_of, size_of};

const N: usize = 4096;

fn ray(px: f32, py: f32, dx: f32, dy: f32, t: f32) -> C2Ray {
    C2Ray {
        p: v(px, py),
        d: v(dx, dy),
        t,
    }
}

fn cmp(A: C2Ray, poly: &C2Poly, bx: Option<C2x>) {
    let (c, r) = (c(), rs());
    let bxp: *const C2x = match &bx {
        Some(x) => x as *const C2x,
        None => std::ptr::null(),
    };
    for seed in [0x0000_0000u32, 0xffff_ffff, 0x5555_5555] {
        let mut oc = poison(seed);
        let mut orr = poison(seed);
        let pc: *const C2Poly = poly;
        let rc = unsafe { (c.c2RaytoPoly)(A, pc, bxp, &mut oc) };
        let rr = unsafe { (r.c2RaytoPoly)(A, pc, bxp, &mut orr) };
        assert_eq!(
            rc, rr,
            "c2RaytoPoly return: C={rc} RUST={rr}\n  ray p={} d={} t={}\n  count={} bx={:?}",
            vshow(A.p),
            vshow(A.d),
            fshow(A.t),
            poly.count,
            bx
        );
        assert!(
            rceq(oc, orr),
            "c2RaytoPoly out: C={} RUST={}\n  ray p={} d={} t={}\n  count={} verts={:?} norms={:?} bx={:?} (poison 0x{seed:08x})",
            rcshow(oc),
            rcshow(orr),
            vshow(A.p),
            vshow(A.d),
            fshow(A.t),
            poly.count,
            &poly.verts[..poly.count.clamp(0, 8) as usize],
            &poly.norms[..poly.count.clamp(0, 8) as usize],
            bx
        );
    }
}

fn ccw90(a: C2v) -> C2v {
    v(a.y, -a.x)
}

fn nrm(a: C2v) -> C2v {
    let l = (a.x * a.x + a.y * a.y).sqrt();
    v(a.x / l, a.y / l)
}

/// Regular CCW n-gon, normals computed the way `c2MakePoly` does:
/// `norms[i] = c2Norm(c2CCW90(verts[i+1] - verts[i]))`.
fn ngon(count: i32, radius: f32, phase: f32) -> C2Poly {
    let mut p = C2Poly::default();
    p.count = count;
    let n = count.clamp(0, 8);
    for i in 0..n {
        let a = phase + (i as f32) * std::f32::consts::TAU / (n.max(1) as f32);
        p.verts[i as usize] = v(radius * a.cos(), radius * a.sin());
    }
    for i in 0..n {
        let j = (i + 1) % n.max(1);
        let e = v(
            p.verts[j as usize].x - p.verts[i as usize].x,
            p.verts[j as usize].y - p.verts[i as usize].y,
        );
        p.norms[i as usize] = nrm(ccw90(e));
    }
    p
}

/// Axis-aligned box polygon (the same shape `poly_ray` builds).
fn boxpoly(hw: f32, hh: f32) -> C2Poly {
    let mut p = C2Poly::default();
    p.count = 4;
    p.verts[0] = v(hw, -hh);
    p.verts[1] = v(hw, hh);
    p.verts[2] = v(-hw, hh);
    p.verts[3] = v(-hw, -hh);
    p.norms[0] = v(1.0, 0.0);
    p.norms[1] = v(0.0, 1.0);
    p.norms[2] = v(-1.0, 0.0);
    p.norms[3] = v(0.0, -1.0);
    p
}

/// The exact polygon from `poly_ray`.
fn polyray_shape() -> C2Poly {
    boxpoly(0.875, 11.5)
}

fn rot(ang: f32) -> C2r {
    C2r {
        c: ang.cos(),
        s: ang.sin(),
    }
}

fn xf(px: f32, py: f32, ang: f32) -> C2x {
    C2x {
        p: v(px, py),
        r: rot(ang),
    }
}

const IDENT: C2x = C2x {
    p: C2v { x: 0.0, y: 0.0 },
    r: C2r { c: 1.0, s: 0.0 },
};

// --- row 67: randomized shotgun -----------------------------------------
#[test]
fn row67_random_shotgun() {
    let mut rng = Rng::new(0x6767);
    for _ in 0..N {
        let n = 1 + rng.below(8) as i32;
        let p = ngon(n, 0.5 + rng.unit(6.0).abs(), rng.unit(3.2));
        cmp(
            ray(rng.geom(), rng.geom(), rng.geom(), rng.geom(), rng.geom()),
            &p,
            None,
        );
    }
    // completely arbitrary (possibly non-convex, un-normalised) polygons
    for _ in 0..N {
        let mut p = C2Poly::default();
        p.count = rng.below(9) as i32;
        for i in 0..8 {
            p.verts[i] = rng.geom_v();
            p.norms[i] = rng.geom_v();
        }
        cmp(
            ray(rng.geom(), rng.geom(), rng.geom(), rng.geom(), rng.geom()),
            &p,
            None,
        );
    }
    // wild floats everywhere
    for _ in 0..N {
        let mut p = C2Poly::default();
        p.count = rng.below(9) as i32;
        for i in 0..8 {
            p.verts[i] = rng.wild_v();
            p.norms[i] = rng.wild_v();
        }
        cmp(
            C2Ray {
                p: rng.wild_v(),
                d: rng.wild_v(),
                t: rng.wild(),
            },
            &p,
            None,
        );
    }
}

// --- rows 68..74: every `count` value ----------------------------------
#[test]
fn row68_to_row74_count_sweep() {
    let mut rng = Rng::new(0x6874);
    let counts: [i32; 14] = [
        i32::MIN,
        -1000,
        -8,
        -2,
        -1,
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
    ];
    for &n in counts.iter() {
        // n-gon geometry with the count overridden
        let mut p = ngon(n.clamp(1, 8), 3.0, 0.0);
        p.count = n;
        for k in 0..48 {
            let ang = (k as f32) * std::f32::consts::TAU / 48.0;
            let start = 9.0;
            cmp(
                ray(
                    ang.cos() * start,
                    ang.sin() * start,
                    -ang.cos(),
                    -ang.sin(),
                    start * 2.0,
                ),
                &p,
                None,
            );
        }
        cmp(ray(0.0, 0.0, 1.0, 0.0, 10.0), &p, None);
        cmp(ray(0.0, 0.0, 0.0, 0.0, 10.0), &p, None);
        for _ in 0..256 {
            cmp(
                ray(rng.geom(), rng.geom(), rng.geom(), rng.geom(), rng.geom()),
                &p,
                None,
            );
        }
    }
}

// --- row 75: count > 8 -> out-of-bounds reads in a padded buffer ---------
#[test]
fn row75_count_gt_8_oob() {
    let (c, r) = (c(), rs());
    let words = (size_of::<C2Poly>() + 512) / 4;
    assert_eq!(align_of::<C2Poly>(), 4);
    let mut rng = Rng::new(0x7575);

    for count in 9i32..=24 {
        for trial in 0..24 {
            // A 4-byte-aligned, fully-initialised backing buffer so that both
            // libraries read exactly the same bytes past `verts[8]`/`norms[8]`.
            let mut buf: Vec<u32> = (0..words)
                .map(|i| rng.next_u32() ^ (i as u32).wrapping_mul(0x9e37_79b9))
                .collect();
            let mut base = ngon(8, 3.0, trial as f32 * 0.1);
            base.count = count;
            unsafe {
                std::ptr::copy_nonoverlapping(
                    (&base as *const C2Poly) as *const u8,
                    buf.as_mut_ptr() as *mut u8,
                    size_of::<C2Poly>(),
                );
            }
            let pc = buf.as_ptr() as *const C2Poly;
            for k in 0..8 {
                let ang = (k as f32) * std::f32::consts::TAU / 8.0;
                let a = ray(
                    ang.cos() * 9.0,
                    ang.sin() * 9.0,
                    -ang.cos(),
                    -ang.sin(),
                    18.0,
                );
                for seed in [0u32, 0xffff_ffff] {
                    let mut oc = poison(seed);
                    let mut orr = poison(seed);
                    let rc =
                        unsafe { (c.c2RaytoPoly)(a, pc, std::ptr::null(), &mut oc) };
                    let rr =
                        unsafe { (r.c2RaytoPoly)(a, pc, std::ptr::null(), &mut orr) };
                    assert_eq!(
                        rc, rr,
                        "count={count} trial={trial} k={k}: C={rc} RUST={rr}"
                    );
                    assert!(
                        rceq(oc, orr),
                        "count={count} trial={trial} k={k}: out C={} RUST={}",
                        rcshow(oc),
                        rcshow(orr)
                    );
                }
            }
        }
    }
}

// --- row 76: NULL bx must behave exactly like c2xIdentity() -------------
#[test]
fn row76_null_bx_equals_identity() {
    let (c, r) = (c(), rs());
    let mut rng = Rng::new(0x7676);
    for _ in 0..N {
        let n = 1 + rng.below(8) as i32;
        let p = ngon(n, 0.5 + rng.unit(5.0).abs(), rng.unit(3.2));
        let a = ray(rng.geom(), rng.geom(), rng.geom(), rng.geom(), rng.geom());
        cmp(a, &p, None);
        cmp(a, &p, Some(IDENT));
        // and cross-check: NULL and identity must give the same answer in
        // *both* libraries.
        let pc: *const C2Poly = &p;
        for api in [c, r] {
            let mut o1 = poison(7);
            let mut o2 = poison(7);
            let r1 = unsafe { (api.c2RaytoPoly)(a, pc, std::ptr::null(), &mut o1) };
            let r2 = unsafe { (api.c2RaytoPoly)(a, pc, &IDENT, &mut o2) };
            assert_eq!(r1, r2, "{}: NULL bx != identity bx", api.name);
            assert!(rceq(o1, o2), "{}: NULL bx != identity bx (out)", api.name);
        }
    }
}

// --- rows 77..80: bx variants ------------------------------------------
#[test]
fn row77_to_row80_bx_variants() {
    let mut rng = Rng::new(0x7780);
    let p4 = boxpoly(2.0, 1.0);
    let p6 = ngon(6, 2.5, 0.0);
    for i in 0..64 {
        let ang = (i as f32) * std::f32::consts::TAU / 64.0;
        let bxs = [
            // pure translation
            xf(3.0 - i as f32 * 0.1, -2.0 + i as f32 * 0.05, 0.0),
            // pure rotation
            xf(0.0, 0.0, ang),
            // rotation + translation
            xf(i as f32 * 0.25 - 8.0, 4.0 - i as f32 * 0.125, ang),
            // non-unit rotation (c*c + s*s != 1)
            C2x {
                p: v(1.0, -1.0),
                r: C2r {
                    c: 2.0 + i as f32 * 0.1,
                    s: -1.5,
                },
            },
            // zero rotation struct
            C2x {
                p: v(0.5, 0.5),
                r: C2r { c: 0.0, s: 0.0 },
            },
        ];
        for &bx in bxs.iter() {
            for k in 0..16 {
                let a2 = (k as f32) * std::f32::consts::TAU / 16.0;
                let s = 12.0;
                let a = ray(a2.cos() * s, a2.sin() * s, -a2.cos(), -a2.sin(), s * 2.0);
                cmp(a, &p4, Some(bx));
                cmp(a, &p6, Some(bx));
            }
        }
    }
    for _ in 0..N {
        let bx = C2x {
            p: rng.geom_v(),
            r: C2r {
                c: rng.geom(),
                s: rng.geom(),
            },
        };
        let n = 1 + rng.below(8) as i32;
        let p = ngon(n, 0.5 + rng.unit(5.0).abs(), rng.unit(3.2));
        cmp(
            ray(rng.geom(), rng.geom(), rng.geom(), rng.geom(), rng.geom()),
            &p,
            Some(bx),
        );
    }
    for _ in 0..N {
        let bx = C2x {
            p: rng.wild_v(),
            r: C2r {
                c: rng.wild(),
                s: rng.wild(),
            },
        };
        let p = ngon(4, 2.0, 0.0);
        cmp(
            C2Ray {
                p: rng.wild_v(),
                d: rng.wild_v(),
                t: rng.wild(),
            },
            &p,
            Some(bx),
        );
    }
}

// --- row 81: ray origin inside the polygon ----------------------------
#[test]
fn row81_origin_inside() {
    let mut rng = Rng::new(0x8181);
    let shapes = [boxpoly(2.0, 3.0), ngon(3, 4.0, 0.0), ngon(8, 4.0, 0.3), polyray_shape()];
    for p in shapes.iter() {
        for _ in 0..512 {
            let ang = rng.unit(std::f32::consts::PI);
            cmp(
                C2Ray {
                    p: v(rng.unit(0.5), rng.unit(0.5)),
                    d: v(ang.cos(), ang.sin()),
                    t: 100.0,
                },
                p,
                None,
            );
        }
        cmp(ray(0.0, 0.0, 1.0, 0.0, 100.0), p, None);
        cmp(ray(0.0, 0.0, 0.0, 1.0, 100.0), p, None);
        cmp(ray(0.0, 0.0, 0.0, 0.0, 100.0), p, None);
    }
}

// --- rows 82 & 83: ray parallel to a face (den == 0) ------------------
#[test]
fn row82_row83_parallel_to_face() {
    let p = boxpoly(2.0, 1.0);
    // norms[0] = (1,0): den == 0 when A.d.x == 0
    for x in [-6.0f32, -2.5, -2.0, -1.0, 0.0, 1.0, 2.0, 2.5, 6.0] {
        for y in [-6.0f32, -1.0, 0.0, 1.0, 6.0] {
            for t in [0.0f32, 1.0, 10.0, 100.0] {
                cmp(ray(x, y, 0.0, 1.0, t), &p, None); // parallel to +/-x faces
                cmp(ray(x, y, 0.0, -1.0, t), &p, None);
                cmp(ray(x, y, 1.0, 0.0, t), &p, None); // parallel to +/-y faces
                cmp(ray(x, y, -1.0, 0.0, t), &p, None);
                cmp(ray(x, y, 0.0, 0.0, t), &p, None); // every den == 0
            }
        }
    }
    // exactly along a face plane
    cmp(ray(-6.0, 1.0, 1.0, 0.0, 20.0), &p, None);
    cmp(ray(-6.0, -1.0, 1.0, 0.0, 20.0), &p, None);
    cmp(ray(2.0, -6.0, 0.0, 1.0, 20.0), &p, None);
    cmp(ray(-2.0, -6.0, 0.0, 1.0, 20.0), &p, None);
}

// --- row 84: hit each face in turn ------------------------------------
#[test]
fn row84_each_face_index() {
    for n in 1i32..=8 {
        let p = ngon(n, 3.0, 0.0);
        // aim at the midpoint of every face from outside along -normal
        for i in 0..n {
            let nx = p.norms[i as usize].x;
            let ny = p.norms[i as usize].y;
            for dist in [4.0f32, 6.0, 10.0, 100.0] {
                cmp(
                    ray(nx * dist, ny * dist, -nx, -ny, dist * 2.0),
                    &p,
                    None,
                );
                // graze the face from an angle
                cmp(
                    ray(nx * dist + ny, ny * dist - nx, -nx, -ny, dist * 2.0),
                    &p,
                    None,
                );
            }
        }
        // hit every vertex exactly
        for i in 0..n.min(8) {
            let vx = p.verts[i as usize].x;
            let vy = p.verts[i as usize].y;
            let l = (vx * vx + vy * vy).sqrt();
            cmp(
                ray(vx / l * 9.0, vy / l * 9.0, -vx / l, -vy / l, 20.0),
                &p,
                None,
            );
        }
    }
}

// --- row 85: A.t shapes ----------------------------------------------
#[test]
fn row85_t_shapes() {
    let p = polyray_shape();
    let ts = [
        0.0f32,
        -0.0,
        -1.0,
        -1.0e30,
        1.0e-30,
        0.5,
        1.0,
        4.0,
        1.0e30,
        f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ];
    for &t in ts.iter() {
        for px in [-4.0f32, -0.875, 0.0, 0.875, 4.0] {
            for py in [-13.0f32, -11.5, 0.0, 11.5, 13.0693407] {
                cmp(ray(px, py, 1.0, 0.0, t), &p, None);
                cmp(ray(px, py, 0.0, -1.0, t), &p, None);
                cmp(ray(px, py, -1.0, 1.0, t), &p, None);
                cmp(ray(px, py, 0.0, 0.0, t), &p, None);
            }
        }
    }
}

// --- row 86: A.d == (0,0) -------------------------------------------
#[test]
fn row86_zero_direction() {
    let mut rng = Rng::new(0x8686);
    for _ in 0..N {
        let n = 1 + rng.below(8) as i32;
        let p = ngon(n, 0.5 + rng.unit(5.0).abs(), rng.unit(3.2));
        for d in [v(0.0, 0.0), v(-0.0, 0.0), v(0.0, -0.0), v(-0.0, -0.0)] {
            cmp(
                C2Ray {
                    p: rng.geom_v(),
                    d,
                    t: rng.geom(),
                },
                &p,
                None,
            );
        }
    }
}

// --- row 87: zero / NaN / inf normals and verts ---------------------
#[test]
fn row87_degenerate_normals_and_verts() {
    let mut rng = Rng::new(0x8787);
    for &s in SPECIALS.iter() {
        for slot in 0..4 {
            let mut p = boxpoly(2.0, 1.0);
            match slot {
                0 => p.norms[0].x = s,
                1 => p.norms[1].y = s,
                2 => p.verts[0].x = s,
                _ => p.verts[2].y = s,
            }
            for k in 0..8 {
                let ang = (k as f32) * std::f32::consts::TAU / 8.0;
                cmp(
                    ray(ang.cos() * 6.0, ang.sin() * 6.0, -ang.cos(), -ang.sin(), 12.0),
                    &p,
                    None,
                );
            }
        }
    }
    // all-zero normals => den == 0 and num == 0 for every face
    let mut zeroed = boxpoly(2.0, 1.0);
    for i in 0..8 {
        zeroed.norms[i] = v(0.0, 0.0);
    }
    for k in 0..16 {
        let ang = (k as f32) * std::f32::consts::TAU / 16.0;
        cmp(
            ray(ang.cos() * 6.0, ang.sin() * 6.0, -ang.cos(), -ang.sin(), 12.0),
            &zeroed,
            None,
        );
    }
    // inward-facing (negated) normals
    let mut flipped = boxpoly(2.0, 1.0);
    for i in 0..8 {
        flipped.norms[i] = v(-flipped.norms[i].x, -flipped.norms[i].y);
    }
    for k in 0..16 {
        let ang = (k as f32) * std::f32::consts::TAU / 16.0;
        cmp(
            ray(ang.cos() * 6.0, ang.sin() * 6.0, -ang.cos(), -ang.sin(), 12.0),
            &flipped,
            None,
        );
    }
    for _ in 0..N {
        let mut p = C2Poly::default();
        p.count = 1 + rng.below(8) as i32;
        for i in 0..8 {
            p.verts[i] = rng.wild_v();
            p.norms[i] = rng.wild_v();
        }
        cmp(
            C2Ray {
                p: rng.wild_v(),
                d: rng.wild_v(),
                t: rng.wild(),
            },
            &p,
            None,
        );
    }
}

// --- row 88: grazing along a face plane -----------------------------
#[test]
fn row88_grazing() {
    let p = polyray_shape();
    // the exact rays that `poly_ray` casts, plus perturbations
    for dy in [0.0f32, -0.5, 0.5, -11.5, 11.5, 13.0693407, -13.0693407] {
        for dxs in [1.0f32, -1.0, 0.0] {
            cmp(ray(-3.869416, dy, dxs, 0.0, 4.0), &p, None);
            cmp(ray(-3.869416, dy, 0.0, -1.0, 4.0), &p, None);
            cmp(ray(0.875, dy, 1.0, 0.0, 4.0), &p, None);
            cmp(ray(-0.875, dy, -1.0, 0.0, 4.0), &p, None);
        }
    }
    // rays exactly along the +x face plane x == 0.875
    for t in [0.0f32, 1.0, 4.0, 40.0] {
        cmp(ray(0.875, -20.0, 0.0, 1.0, t), &p, None);
        cmp(ray(0.875, 20.0, 0.0, -1.0, t), &p, None);
        cmp(ray(-0.875, -20.0, 0.0, 1.0, t), &p, None);
        cmp(ray(-20.0, 11.5, 1.0, 0.0, t), &p, None);
        cmp(ray(-20.0, -11.5, 1.0, 0.0, t), &p, None);
    }
}

// --- row 89: out sentinel preserved on the miss paths ---------------
#[test]
fn row89_out_untouched_on_miss() {
    let (c, r) = (c(), rs());
    let p = boxpoly(2.0, 1.0);
    let mut empty = C2Poly::default();
    empty.count = 0;
    let cases: [(&str, C2Ray, &C2Poly); 4] = [
        // count == 0 -> loop never runs, index stays -1
        ("count 0", ray(-5.0, 0.0, 1.0, 0.0, 20.0), &empty),
        // parallel and outside -> den == 0 && num < 0
        ("parallel outside", ray(-5.0, 5.0, 1.0, 0.0, 20.0), &p),
        // origin inside -> index stays -1
        ("origin inside", ray(0.0, 0.0, 1.0, 0.0, 20.0), &p),
        // hi < lo -> interval collapse
        ("miss", ray(-5.0, 5.0, 1.0, -0.125, 20.0), &p),
    ];
    for (name, a, poly) in cases {
        for seed in [0u32, 1, 0xdead_beef, 0xffff_ffff] {
            let mut oc = poison(seed);
            let mut orr = poison(seed);
            let pc: *const C2Poly = poly;
            let rc = unsafe { (c.c2RaytoPoly)(a, pc, std::ptr::null(), &mut oc) };
            let rr = unsafe { (r.c2RaytoPoly)(a, pc, std::ptr::null(), &mut orr) };
            assert_eq!(rc, rr, "{name}: C={rc} RUST={rr}");
            assert_eq!(rc, 0, "{name}: expected the C library to miss");
            assert!(rceq(oc, poison(seed)), "{name}: C wrote to *out on a miss");
            assert!(
                rceq(orr, poison(seed)),
                "{name}: RUST wrote to *out on a miss: {}",
                rcshow(orr)
            );
        }
    }
}
